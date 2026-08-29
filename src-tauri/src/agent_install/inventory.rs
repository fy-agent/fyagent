//! Read-only installation inventory and opaque target authority.
//!
//! Platform adapters produce evidence. This module owns normalization,
//! snapshot-scoped IDs, revisions, selection policy and stale revalidation.

use std::{
    collections::{HashMap, HashSet, VecDeque},
    path::PathBuf,
    sync::Mutex,
    time::{Duration, Instant},
};

use sha2::{Digest, Sha256};

use super::{
    cli::{observe_cli, tooling_id_for},
    desktop::discover_desktop_installations,
    types::{
        validate_opaque_inventory_id, validate_opaque_target_id, validate_opaque_target_revision,
        AgentActionId, AgentInstallationInventoryDto, AgentReasonCode, FreshInstallDestinationDto,
        InstallationCandidateDto, InstallationEvidenceCode, InstallationInventoryState,
        InstallationOwner, InstallationPackageKind, InstallationScope, StartAgentActionRequest,
        AGENT_INSTALLATION_INVENTORY_CONTRACT_VERSION,
    },
};
use crate::{
    codex_desktop::types::LocalInstallStatus, services::external_agents::AgentCatalogId,
    store::AppState,
};

const INVENTORY_TTL: Duration = Duration::from_secs(5 * 60);
const MAX_SNAPSHOTS: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum InstallationTargetCapability {
    Desktop {
        path: PathBuf,
        scope: InstallationScope,
        package_kind: InstallationPackageKind,
    },
    Cli,
    CodexDesktop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FreshDestinationCapability {
    #[cfg(target_os = "macos")]
    MacUserApplications,
    #[cfg(target_os = "macos")]
    MacSystemApplications,
    #[cfg(target_os = "windows")]
    WindowsCurrentUser,
    #[cfg(target_os = "windows")]
    WindowsAllUsers,
    #[cfg(target_os = "windows")]
    VendorInstallerChoice,
    CliPackageManager,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ValidatedActionTarget {
    Existing(InstallationTargetCapability),
    Fresh(FreshDestinationCapability),
    None,
}

impl ValidatedActionTarget {
    pub(super) fn desktop_path(&self) -> Option<&PathBuf> {
        match self {
            Self::Existing(InstallationTargetCapability::Desktop { path, .. }) => Some(path),
            _ => None,
        }
    }

    pub(super) fn fresh_destination(&self) -> Option<FreshDestinationCapability> {
        match self {
            Self::Fresh(destination) => Some(*destination),
            _ => None,
        }
    }
}

#[derive(Clone)]
struct ProbeCandidate {
    stable_key: String,
    revision: String,
    scope: InstallationScope,
    owner: InstallationOwner,
    package_kind: InstallationPackageKind,
    local_version: Option<String>,
    launch_eligible: bool,
    install_eligible: bool,
    update_eligible: bool,
    reason_codes: Vec<AgentReasonCode>,
    evidence_codes: Vec<InstallationEvidenceCode>,
    location_label: String,
    capability: InstallationTargetCapability,
}

impl ProbeCandidate {
    fn trusted(&self) -> bool {
        self.launch_eligible || self.update_eligible
    }
}

#[derive(Clone)]
struct ProbeDestination {
    stable_key: String,
    revision: String,
    scope: InstallationScope,
    owner: InstallationOwner,
    package_kind: InstallationPackageKind,
    requires_elevation: bool,
    writable: bool,
    eligible: bool,
    reason_codes: Vec<AgentReasonCode>,
    location_label: String,
    capability: FreshDestinationCapability,
}

struct InventoryProbe {
    state_override: Option<InstallationInventoryState>,
    candidates: Vec<ProbeCandidate>,
    destinations: Vec<ProbeDestination>,
    reason_codes: Vec<AgentReasonCode>,
}

impl InventoryProbe {
    fn state(&self) -> InstallationInventoryState {
        if let Some(state) = self.state_override {
            return state;
        }
        let trusted = self
            .candidates
            .iter()
            .filter(|candidate| candidate.trusted())
            .count();
        match trusted {
            0 if self.candidates.is_empty() => InstallationInventoryState::NotObserved,
            0 => InstallationInventoryState::Unknown,
            1 => InstallationInventoryState::Single,
            _ => InstallationInventoryState::Multiple,
        }
    }
}

#[derive(Clone)]
struct SnapshotCandidate {
    stable_key: String,
    revision: String,
    launch_eligible: bool,
    update_eligible: bool,
}

#[derive(Clone)]
struct SnapshotDestination {
    stable_key: String,
    revision: String,
    eligible: bool,
}

struct InventorySnapshot {
    created_at: Instant,
    agent_id: AgentCatalogId,
    candidates: HashMap<String, SnapshotCandidate>,
    destinations: HashMap<String, SnapshotDestination>,
}

#[derive(Default)]
struct InventoryCache {
    snapshots: HashMap<String, InventorySnapshot>,
    order: VecDeque<String>,
}

pub struct AgentInstallationInventoryStore {
    cache: Mutex<InventoryCache>,
}

impl AgentInstallationInventoryStore {
    pub fn new() -> Self {
        Self {
            cache: Mutex::new(InventoryCache::default()),
        }
    }

    fn insert(&self, inventory_id: String, snapshot: InventorySnapshot) {
        let mut cache = self
            .cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        prune_expired(&mut cache);
        cache.order.push_back(inventory_id.clone());
        cache.snapshots.insert(inventory_id, snapshot);
        while cache.order.len() > MAX_SNAPSHOTS {
            if let Some(oldest) = cache.order.pop_front() {
                cache.snapshots.remove(&oldest);
            }
        }
    }

    fn snapshot_target(
        &self,
        inventory_id: &str,
        agent_id: AgentCatalogId,
        target_id: &str,
        expected_revision: &str,
    ) -> Result<SnapshotTarget, AgentReasonCode> {
        let mut cache = self
            .cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        prune_expired(&mut cache);
        let snapshot = cache
            .snapshots
            .get(inventory_id)
            .ok_or(AgentReasonCode::InventoryExpired)?;
        if snapshot.agent_id != agent_id {
            return Err(AgentReasonCode::RefreshRequired);
        }
        if let Some(candidate) = snapshot.candidates.get(target_id) {
            if candidate.revision != expected_revision {
                return Err(AgentReasonCode::TargetChanged);
            }
            return Ok(SnapshotTarget::Candidate(candidate.clone()));
        }
        if let Some(destination) = snapshot.destinations.get(target_id) {
            if destination.revision != expected_revision {
                return Err(AgentReasonCode::TargetChanged);
            }
            return Ok(SnapshotTarget::Destination(destination.clone()));
        }
        Err(AgentReasonCode::TargetChanged)
    }
}

impl Default for AgentInstallationInventoryStore {
    fn default() -> Self {
        Self::new()
    }
}

enum SnapshotTarget {
    Candidate(SnapshotCandidate),
    Destination(SnapshotDestination),
}

fn prune_expired(cache: &mut InventoryCache) {
    let now = Instant::now();
    cache
        .snapshots
        .retain(|_, snapshot| now.saturating_duration_since(snapshot.created_at) <= INVENTORY_TTL);
    let live: HashSet<String> = cache.snapshots.keys().cloned().collect();
    cache
        .order
        .retain(|inventory_id| live.contains(inventory_id));
}

pub async fn inventory_for(
    agent_id: AgentCatalogId,
    state: &AppState,
) -> AgentInstallationInventoryDto {
    let probe = probe_inventory(agent_id, state).await;
    project_and_store(agent_id, probe, &state.agent_installation_inventory)
}

pub async fn inventory_summary(
    agent_id: AgentCatalogId,
    state: &AppState,
) -> InstallationInventoryState {
    probe_inventory(agent_id, state).await.state()
}

pub async fn validate_action_target(
    request: &StartAgentActionRequest,
    state: &AppState,
) -> Result<ValidatedActionTarget, AgentReasonCode> {
    let binding = match (
        request.inventory_id.as_deref(),
        request.target_id.as_deref(),
        request.expected_target_revision.as_deref(),
    ) {
        (None, None, None) => None,
        (Some(inventory_id), Some(target_id), Some(revision))
            if validate_opaque_inventory_id(inventory_id)
                && validate_opaque_target_id(target_id)
                && validate_opaque_target_revision(revision) =>
        {
            Some((inventory_id, target_id, revision))
        }
        _ => return Err(AgentReasonCode::RefreshRequired),
    };

    if matches!(
        request.action,
        AgentActionId::AuthLogout | AgentActionId::AuthConnectProvider
    ) && binding.is_some()
    {
        return Err(AgentReasonCode::RefreshRequired);
    }

    if let Some((inventory_id, target_id, revision)) = binding {
        let snapshot_target = state.agent_installation_inventory.snapshot_target(
            inventory_id,
            request.agent_id,
            target_id,
            revision,
        )?;
        let current = probe_inventory(request.agent_id, state).await;
        return validate_snapshot_target(request.action, snapshot_target, current);
    }

    match request.action {
        AgentActionId::Install | AgentActionId::Update => {
            if request.agent_id == AgentCatalogId::Codex {
                Ok(ValidatedActionTarget::None)
            } else {
                Err(AgentReasonCode::TargetSelectionRequired)
            }
        }
        AgentActionId::Launch | AgentActionId::AuthLogin
            if matches!(
                request.agent_id,
                AgentCatalogId::QoderWork | AgentCatalogId::TraeWork | AgentCatalogId::WorkBuddy
            ) =>
        {
            unique_legacy_candidate(request.agent_id, state).await
        }
        AgentActionId::Launch if request.agent_id == AgentCatalogId::Codex => {
            Ok(ValidatedActionTarget::None)
        }
        _ => Ok(ValidatedActionTarget::None),
    }
}

async fn unique_legacy_candidate(
    agent_id: AgentCatalogId,
    state: &AppState,
) -> Result<ValidatedActionTarget, AgentReasonCode> {
    let mut eligible = probe_inventory(agent_id, state)
        .await
        .candidates
        .into_iter()
        .filter(|candidate| candidate.launch_eligible);
    let first = eligible
        .next()
        .ok_or(AgentReasonCode::TargetNotExecutable)?;
    if eligible.next().is_some() {
        return Err(AgentReasonCode::TargetSelectionRequired);
    }
    Ok(ValidatedActionTarget::Existing(first.capability))
}

fn validate_snapshot_target(
    action: AgentActionId,
    snapshot: SnapshotTarget,
    current: InventoryProbe,
) -> Result<ValidatedActionTarget, AgentReasonCode> {
    match snapshot {
        SnapshotTarget::Candidate(snapshot) => {
            if action == AgentActionId::Install {
                return Err(AgentReasonCode::TargetScopeUnsupported);
            }
            if matches!(action, AgentActionId::Launch | AgentActionId::AuthLogin)
                && !snapshot.launch_eligible
            {
                return Err(AgentReasonCode::TargetNotExecutable);
            }
            if action == AgentActionId::Update && !snapshot.update_eligible {
                return Err(AgentReasonCode::TargetScopeUnsupported);
            }
            let candidate = current
                .candidates
                .into_iter()
                .find(|candidate| candidate.stable_key == snapshot.stable_key)
                .ok_or(AgentReasonCode::TargetChanged)?;
            if candidate.revision != snapshot.revision {
                return Err(AgentReasonCode::TargetChanged);
            }
            Ok(ValidatedActionTarget::Existing(candidate.capability))
        }
        SnapshotTarget::Destination(snapshot) => {
            if action != AgentActionId::Install || !snapshot.eligible {
                return Err(AgentReasonCode::TargetScopeUnsupported);
            }
            let destination = current
                .destinations
                .into_iter()
                .find(|destination| destination.stable_key == snapshot.stable_key)
                .ok_or(AgentReasonCode::TargetChanged)?;
            if destination.revision != snapshot.revision || !destination.eligible {
                return Err(AgentReasonCode::TargetChanged);
            }
            Ok(ValidatedActionTarget::Fresh(destination.capability))
        }
    }
}

fn project_and_store(
    agent_id: AgentCatalogId,
    probe: InventoryProbe,
    store: &AgentInstallationInventoryStore,
) -> AgentInstallationInventoryDto {
    let inventory_id = opaque_id("i1:");
    let state = probe.state();
    let mut snapshot_candidates = HashMap::new();
    let mut candidates = Vec::with_capacity(probe.candidates.len());
    for candidate in probe.candidates {
        let candidate_id = opaque_id("c1:");
        snapshot_candidates.insert(
            candidate_id.clone(),
            SnapshotCandidate {
                stable_key: candidate.stable_key,
                revision: candidate.revision.clone(),
                launch_eligible: candidate.launch_eligible,
                update_eligible: candidate.update_eligible,
            },
        );
        candidates.push(InstallationCandidateDto {
            candidate_id,
            candidate_revision: candidate.revision,
            agent_id,
            scope: candidate.scope,
            owner: candidate.owner,
            package_kind: candidate.package_kind,
            local_version: candidate.local_version,
            launch_eligible: candidate.launch_eligible,
            install_eligible: candidate.install_eligible,
            update_eligible: candidate.update_eligible,
            reason_codes: candidate.reason_codes,
            evidence_codes: candidate.evidence_codes,
            location_label: candidate.location_label,
        });
    }
    let mut snapshot_destinations = HashMap::new();
    let mut fresh_destinations = Vec::with_capacity(probe.destinations.len());
    for destination in probe.destinations {
        let destination_id = opaque_id("d1:");
        snapshot_destinations.insert(
            destination_id.clone(),
            SnapshotDestination {
                stable_key: destination.stable_key,
                revision: destination.revision.clone(),
                eligible: destination.eligible,
            },
        );
        fresh_destinations.push(FreshInstallDestinationDto {
            destination_id,
            destination_revision: destination.revision,
            scope: destination.scope,
            owner: destination.owner,
            package_kind: destination.package_kind,
            requires_elevation: destination.requires_elevation,
            writable: destination.writable,
            eligible: destination.eligible,
            reason_codes: destination.reason_codes,
            location_label: destination.location_label,
        });
    }
    store.insert(
        inventory_id.clone(),
        InventorySnapshot {
            created_at: Instant::now(),
            agent_id,
            candidates: snapshot_candidates,
            destinations: snapshot_destinations,
        },
    );
    AgentInstallationInventoryDto {
        contract_version: AGENT_INSTALLATION_INVENTORY_CONTRACT_VERSION,
        inventory_id,
        agent_id,
        state,
        candidates,
        fresh_destinations,
        reason_codes: probe.reason_codes,
    }
}

async fn probe_inventory(agent_id: AgentCatalogId, state: &AppState) -> InventoryProbe {
    match agent_id {
        AgentCatalogId::QoderWork | AgentCatalogId::TraeWork | AgentCatalogId::WorkBuddy => {
            probe_desktop(agent_id)
        }
        AgentCatalogId::ClaudeCode | AgentCatalogId::GrokBuild | AgentCatalogId::OpenCode => {
            probe_cli(agent_id).await
        }
        AgentCatalogId::Codex => probe_codex(state).await,
    }
}

fn probe_desktop(agent_id: AgentCatalogId) -> InventoryProbe {
    if !cfg!(any(target_os = "macos", target_os = "windows")) {
        return InventoryProbe {
            state_override: Some(InstallationInventoryState::Unsupported),
            candidates: Vec::new(),
            destinations: Vec::new(),
            reason_codes: vec![AgentReasonCode::PlatformUnsupported],
        };
    }
    let mut candidates = Vec::new();
    for evidence in discover_desktop_installations(agent_id) {
        let location_label =
            desktop_location_label(agent_id, evidence.scope, evidence.package_kind);
        let mut candidate = ProbeCandidate {
            stable_key: evidence.stable_key,
            revision: String::new(),
            scope: evidence.scope,
            owner: InstallationOwner::VendorInstaller,
            package_kind: evidence.package_kind,
            local_version: evidence.local_version,
            launch_eligible: true,
            install_eligible: false,
            update_eligible: true,
            reason_codes: Vec::new(),
            evidence_codes: match evidence.package_kind {
                InstallationPackageKind::AppBundle => {
                    vec![InstallationEvidenceCode::BundleIdentity]
                }
                InstallationPackageKind::Exe => vec![
                    InstallationEvidenceCode::KnownPath,
                    InstallationEvidenceCode::FileIdentity,
                ],
                _ => vec![InstallationEvidenceCode::FileIdentity],
            },
            location_label,
            capability: InstallationTargetCapability::Desktop {
                path: evidence.path,
                scope: evidence.scope,
                package_kind: evidence.package_kind,
            },
        };
        refresh_candidate_revision(&mut candidate);
        candidates.push(candidate);
    }
    InventoryProbe {
        state_override: None,
        candidates: normalize_candidates(candidates),
        destinations: desktop_destinations(agent_id),
        reason_codes: Vec::new(),
    }
}

fn normalize_candidates(candidates: Vec<ProbeCandidate>) -> Vec<ProbeCandidate> {
    let mut normalized: Vec<ProbeCandidate> = Vec::new();
    let mut positions: HashMap<String, usize> = HashMap::new();
    for candidate in candidates {
        let Some(index) = positions.get(&candidate.stable_key).copied() else {
            positions.insert(candidate.stable_key.clone(), normalized.len());
            normalized.push(candidate);
            continue;
        };
        let existing = &mut normalized[index];
        let identity_conflict = existing.scope != candidate.scope
            || existing.owner != candidate.owner
            || existing.package_kind != candidate.package_kind
            || existing.capability != candidate.capability
            || matches!(
                (&existing.local_version, &candidate.local_version),
                (Some(left), Some(right)) if left != right
            );

        for evidence in candidate.evidence_codes {
            if !existing.evidence_codes.contains(&evidence) {
                existing.evidence_codes.push(evidence);
            }
        }
        for reason in candidate.reason_codes {
            if !existing.reason_codes.contains(&reason) {
                existing.reason_codes.push(reason);
            }
        }
        if existing.local_version.is_none() {
            existing.local_version = candidate.local_version;
        }

        if identity_conflict {
            existing.launch_eligible = false;
            existing.install_eligible = false;
            existing.update_eligible = false;
            if !existing
                .reason_codes
                .contains(&AgentReasonCode::CandidateConflict)
            {
                existing
                    .reason_codes
                    .push(AgentReasonCode::CandidateConflict);
            }
        } else {
            existing.launch_eligible |= candidate.launch_eligible;
            existing.install_eligible |= candidate.install_eligible;
            existing.update_eligible |= candidate.update_eligible;
        }
        refresh_candidate_revision(existing);
    }
    normalized
}

fn refresh_candidate_revision(candidate: &mut ProbeCandidate) {
    let mut evidence = candidate
        .evidence_codes
        .iter()
        .map(|code| format!("{code:?}"))
        .collect::<Vec<_>>();
    evidence.sort();
    let evidence = evidence.join(",");
    let mut reasons = candidate
        .reason_codes
        .iter()
        .map(|code| format!("{code:?}"))
        .collect::<Vec<_>>();
    reasons.sort();
    let reasons = reasons.join(",");
    candidate.revision = revision_for(&[
        &candidate.stable_key,
        scope_key(candidate.scope),
        owner_key(candidate.owner),
        package_key(candidate.package_kind),
        candidate.local_version.as_deref().unwrap_or(""),
        if candidate.launch_eligible {
            "launch"
        } else {
            "no-launch"
        },
        if candidate.install_eligible {
            "install"
        } else {
            "no-install"
        },
        if candidate.update_eligible {
            "update"
        } else {
            "no-update"
        },
        &evidence,
        &reasons,
    ]);
}

async fn probe_cli(agent_id: AgentCatalogId) -> InventoryProbe {
    let Some(tool_id) = tooling_id_for(agent_id) else {
        return InventoryProbe {
            state_override: Some(InstallationInventoryState::Unsupported),
            candidates: Vec::new(),
            destinations: Vec::new(),
            reason_codes: vec![AgentReasonCode::PlatformUnsupported],
        };
    };
    let Some(observation) = observe_cli(agent_id).await else {
        return InventoryProbe {
            state_override: Some(InstallationInventoryState::Unknown),
            candidates: Vec::new(),
            destinations: Vec::new(),
            reason_codes: vec![AgentReasonCode::RefreshRequired],
        };
    };
    let mut candidates = Vec::new();
    if observation.detected {
        let stable_key = format!("cli:{tool_id}");
        let runnable = observation.runnable && !observation.unavailable;
        candidates.push(ProbeCandidate {
            revision: revision_for(&[
                &stable_key,
                observation.local_version.as_deref().unwrap_or(""),
                if runnable { "runnable" } else { "not-runnable" },
            ]),
            stable_key,
            scope: InstallationScope::CurrentUser,
            owner: InstallationOwner::PackageManager,
            package_kind: InstallationPackageKind::Unknown,
            local_version: observation.local_version,
            launch_eligible: false,
            install_eligible: false,
            update_eligible: runnable,
            reason_codes: if runnable {
                Vec::new()
            } else {
                vec![AgentReasonCode::InstalledNotRunnable]
            },
            evidence_codes: vec![InstallationEvidenceCode::PathLookup],
            location_label: "Shell PATH".to_string(),
            capability: InstallationTargetCapability::Cli,
        });
    }
    let destination = ProbeDestination {
        stable_key: format!("cli-package-manager:{tool_id}"),
        revision: revision_for(&["cli-package-manager", tool_id]),
        scope: InstallationScope::CurrentUser,
        owner: InstallationOwner::PackageManager,
        package_kind: InstallationPackageKind::Unknown,
        requires_elevation: false,
        writable: !observation.unavailable,
        eligible: !observation.unavailable,
        reason_codes: if observation.unavailable {
            vec![AgentReasonCode::InteractiveUserUnavailable]
        } else {
            Vec::new()
        },
        location_label: "CLI package manager".to_string(),
        capability: FreshDestinationCapability::CliPackageManager,
    };
    InventoryProbe {
        state_override: if observation.unavailable {
            Some(InstallationInventoryState::Unknown)
        } else {
            None
        },
        candidates,
        destinations: vec![destination],
        reason_codes: Vec::new(),
    }
}

async fn probe_codex(state: &AppState) -> InventoryProbe {
    match state.codex_desktop_service.get_local_status().await {
        Ok(LocalInstallStatus::Installed { application }) => {
            let stable_key = "codex-desktop:trusted-candidate".to_string();
            InventoryProbe {
                state_override: None,
                candidates: vec![ProbeCandidate {
                    revision: revision_for(&[
                        &stable_key,
                        application.display_version.as_deref().unwrap_or(""),
                    ]),
                    stable_key,
                    scope: InstallationScope::Unknown,
                    owner: InstallationOwner::Fyagent,
                    package_kind: InstallationPackageKind::Unknown,
                    local_version: application.display_version,
                    launch_eligible: true,
                    install_eligible: false,
                    update_eligible: false,
                    reason_codes: vec![AgentReasonCode::ManagedByCodexDesktop],
                    evidence_codes: vec![InstallationEvidenceCode::CodexDesktopAdapter],
                    location_label: "Codex Desktop managed installation".to_string(),
                    capability: InstallationTargetCapability::CodexDesktop,
                }],
                destinations: Vec::new(),
                reason_codes: vec![AgentReasonCode::ManagedByCodexDesktop],
            }
        }
        Ok(LocalInstallStatus::NotInstalled { .. }) => InventoryProbe {
            state_override: Some(InstallationInventoryState::NotObserved),
            candidates: Vec::new(),
            destinations: Vec::new(),
            reason_codes: vec![AgentReasonCode::ManagedByCodexDesktop],
        },
        Ok(LocalInstallStatus::Unsupported { .. }) => InventoryProbe {
            state_override: Some(InstallationInventoryState::Unsupported),
            candidates: Vec::new(),
            destinations: Vec::new(),
            reason_codes: vec![AgentReasonCode::PlatformUnsupported],
        },
        Ok(LocalInstallStatus::Ambiguous { .. }) => InventoryProbe {
            state_override: Some(InstallationInventoryState::Unknown),
            candidates: Vec::new(),
            destinations: Vec::new(),
            reason_codes: vec![AgentReasonCode::CandidateConflict],
        },
        Err(_) => InventoryProbe {
            state_override: Some(InstallationInventoryState::Unknown),
            candidates: Vec::new(),
            destinations: Vec::new(),
            reason_codes: vec![AgentReasonCode::RefreshRequired],
        },
    }
}

fn desktop_destinations(_agent_id: AgentCatalogId) -> Vec<ProbeDestination> {
    #[cfg(target_os = "macos")]
    {
        let user_writable = super::desktop::user_applications_writable();
        return vec![
            destination(
                "mac:user-applications",
                InstallationScope::CurrentUser,
                InstallationPackageKind::AppBundle,
                false,
                user_writable,
                "~/Applications",
                FreshDestinationCapability::MacUserApplications,
            ),
            destination(
                "mac:system-applications",
                InstallationScope::AllUsers,
                InstallationPackageKind::AppBundle,
                true,
                true,
                "/Applications",
                FreshDestinationCapability::MacSystemApplications,
            ),
        ];
    }
    #[cfg(target_os = "windows")]
    {
        let agent_id = _agent_id;
        let name = agent_display_name(agent_id);
        return vec![
            destination(
                &format!("windows:{agent_id:?}:current-user"),
                InstallationScope::CurrentUser,
                InstallationPackageKind::Exe,
                false,
                true,
                &format!("%LOCALAPPDATA%\\Programs\\{name}"),
                FreshDestinationCapability::WindowsCurrentUser,
            ),
            destination(
                &format!("windows:{agent_id:?}:all-users"),
                InstallationScope::AllUsers,
                InstallationPackageKind::Exe,
                true,
                true,
                &format!("%PROGRAMFILES%\\{name}"),
                FreshDestinationCapability::WindowsAllUsers,
            ),
            destination(
                &format!("windows:{agent_id:?}:vendor-choice"),
                InstallationScope::Unknown,
                InstallationPackageKind::Exe,
                true,
                true,
                "Vendor installer choice",
                FreshDestinationCapability::VendorInstallerChoice,
            ),
        ];
    }
    #[allow(unreachable_code)]
    Vec::new()
}

fn destination(
    stable_key: &str,
    scope: InstallationScope,
    package_kind: InstallationPackageKind,
    requires_elevation: bool,
    writable: bool,
    location_label: &str,
    capability: FreshDestinationCapability,
) -> ProbeDestination {
    ProbeDestination {
        stable_key: stable_key.to_string(),
        revision: revision_for(&[
            stable_key,
            scope_key(scope),
            package_key(package_kind),
            if requires_elevation {
                "elevated"
            } else {
                "user"
            },
            if writable { "writable" } else { "not-writable" },
        ]),
        scope,
        owner: InstallationOwner::VendorInstaller,
        package_kind,
        requires_elevation,
        writable,
        eligible: writable,
        reason_codes: if writable {
            Vec::new()
        } else {
            vec![AgentReasonCode::TargetScopeUnsupported]
        },
        location_label: location_label.to_string(),
        capability,
    }
}

fn desktop_location_label(
    agent_id: AgentCatalogId,
    scope: InstallationScope,
    package_kind: InstallationPackageKind,
) -> String {
    let name = agent_display_name(agent_id);
    match (scope, package_kind) {
        (InstallationScope::CurrentUser, InstallationPackageKind::AppBundle) => {
            format!("~/Applications/{name}.app")
        }
        (InstallationScope::AllUsers, InstallationPackageKind::AppBundle) => {
            format!("/Applications/{name}.app")
        }
        (InstallationScope::CurrentUser, InstallationPackageKind::Exe) => {
            format!("%LOCALAPPDATA%\\Programs\\{name}")
        }
        (InstallationScope::AllUsers, InstallationPackageKind::Exe) => {
            format!("%PROGRAMFILES%\\{name}")
        }
        _ => format!("Custom location ({name})"),
    }
}

fn agent_display_name(agent_id: AgentCatalogId) -> &'static str {
    match agent_id {
        AgentCatalogId::QoderWork => "QoderWork CN",
        AgentCatalogId::TraeWork => "TRAE SOLO CN",
        AgentCatalogId::WorkBuddy => "WorkBuddy",
        AgentCatalogId::GrokBuild => "Grok Build",
        AgentCatalogId::Codex => "Codex",
        AgentCatalogId::ClaudeCode => "Claude Code",
        AgentCatalogId::OpenCode => "OpenCode",
    }
}

fn scope_key(scope: InstallationScope) -> &'static str {
    match scope {
        InstallationScope::CurrentUser => "current_user",
        InstallationScope::AllUsers => "all_users",
        InstallationScope::Custom => "custom",
        InstallationScope::Unknown => "unknown",
    }
}

fn owner_key(owner: InstallationOwner) -> &'static str {
    match owner {
        InstallationOwner::VendorInstaller => "vendor_installer",
        InstallationOwner::PackageManager => "package_manager",
        InstallationOwner::Fyagent => "fyagent",
        InstallationOwner::Unknown => "unknown",
    }
}

fn package_key(kind: InstallationPackageKind) -> &'static str {
    match kind {
        InstallationPackageKind::AppBundle => "app_bundle",
        InstallationPackageKind::Exe => "exe",
        InstallationPackageKind::Msi => "msi",
        InstallationPackageKind::Msix => "msix",
        InstallationPackageKind::Unknown => "unknown",
    }
}

fn revision_for(parts: &[&str]) -> String {
    let mut digest = Sha256::new();
    for part in parts {
        digest.update((part.len() as u64).to_le_bytes());
        digest.update(part.as_bytes());
    }
    format!("r1:{:x}", digest.finalize())
}

fn opaque_id(prefix: &str) -> String {
    format!("{prefix}{}", uuid::Uuid::new_v4().simple())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(stable_key: &str, evidence: InstallationEvidenceCode) -> ProbeCandidate {
        let mut candidate = ProbeCandidate {
            stable_key: stable_key.to_string(),
            revision: String::new(),
            scope: InstallationScope::CurrentUser,
            owner: InstallationOwner::VendorInstaller,
            package_kind: InstallationPackageKind::AppBundle,
            local_version: Some("1.0.0".to_string()),
            launch_eligible: true,
            install_eligible: false,
            update_eligible: true,
            reason_codes: Vec::new(),
            evidence_codes: vec![evidence],
            location_label: "~/Applications/App.app".to_string(),
            capability: InstallationTargetCapability::Desktop {
                path: PathBuf::from("/opaque-test"),
                scope: InstallationScope::CurrentUser,
                package_kind: InstallationPackageKind::AppBundle,
            },
        };
        refresh_candidate_revision(&mut candidate);
        candidate
    }

    #[test]
    fn opaque_ids_and_revisions_are_bounded() {
        assert!(validate_opaque_inventory_id(&opaque_id("i1:")));
        assert!(validate_opaque_target_id(&opaque_id("c1:")));
        assert!(validate_opaque_target_id(&opaque_id("d1:")));
        assert!(validate_opaque_target_revision(&revision_for(&["a", "b"])));
    }

    #[test]
    fn trusted_count_never_selects_the_first_of_multiple_candidates() {
        let probe = InventoryProbe {
            state_override: None,
            candidates: vec![
                candidate("one", InstallationEvidenceCode::BundleIdentity),
                candidate("two", InstallationEvidenceCode::BundleIdentity),
            ],
            destinations: Vec::new(),
            reason_codes: Vec::new(),
        };
        assert_eq!(probe.state(), InstallationInventoryState::Multiple);
    }

    #[test]
    fn duplicate_evidence_merges_without_creating_a_second_candidate() {
        let merged = normalize_candidates(vec![
            candidate("same", InstallationEvidenceCode::KnownPath),
            candidate("same", InstallationEvidenceCode::BundleIdentity),
        ]);

        assert_eq!(merged.len(), 1);
        assert_eq!(
            merged[0].evidence_codes,
            vec![
                InstallationEvidenceCode::KnownPath,
                InstallationEvidenceCode::BundleIdentity,
            ]
        );
        assert!(merged[0].launch_eligible);
        assert!(merged[0].update_eligible);
    }

    #[test]
    fn conflicting_duplicate_evidence_is_visible_but_not_executable() {
        let first = candidate("same", InstallationEvidenceCode::KnownPath);
        let mut conflicting = candidate("same", InstallationEvidenceCode::BundleIdentity);
        conflicting.scope = InstallationScope::AllUsers;
        refresh_candidate_revision(&mut conflicting);

        let merged = normalize_candidates(vec![first, conflicting]);

        assert_eq!(merged.len(), 1);
        assert!(!merged[0].launch_eligible);
        assert!(!merged[0].update_eligible);
        assert!(merged[0]
            .reason_codes
            .contains(&AgentReasonCode::CandidateConflict));
    }

    #[test]
    fn location_projection_never_contains_a_user_profile() {
        assert_eq!(
            desktop_location_label(
                AgentCatalogId::WorkBuddy,
                InstallationScope::CurrentUser,
                InstallationPackageKind::AppBundle,
            ),
            "~/Applications/WorkBuddy.app"
        );
        assert_eq!(
            desktop_location_label(
                AgentCatalogId::WorkBuddy,
                InstallationScope::CurrentUser,
                InstallationPackageKind::Exe,
            ),
            "%LOCALAPPDATA%\\Programs\\WorkBuddy"
        );
    }
}
