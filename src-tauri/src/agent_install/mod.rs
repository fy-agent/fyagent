//! Agent install/action façade. Canonical catalog IDs only; renderer input is
//! `agentId + action` plus an optional opaque backend-generated release ID.

mod auth_actions;
mod auth_sessions;
mod cli;
mod desktop;
mod fetch;
mod inventory;
mod jobs;
mod lifecycle_policy;
mod macos;
mod sources;
mod types;
mod windows;

use std::sync::atomic::AtomicBool;
use std::sync::Arc;
#[cfg(target_os = "windows")]
use std::time::Duration;

pub use auth_sessions::{
    auth_observation_for, get_active_agent_auth_session, get_agent_auth_session,
    start_agent_auth_session, stop_waiting_for_agent_auth, AgentAuthSessionStore,
};
pub use inventory::{inventory_for, AgentInstallationInventoryStore};
pub use jobs::AgentActionJobStore;
pub use types::{
    resolve_requested_surface, validate_opaque_release_id, AgentActionErrorDto, AgentActionId,
    AgentActionJobSnapshot, AgentActionJobStage, AgentActionResult, AgentAuthErrorDto,
    AgentAuthObservationDto, AgentAuthOwnership, AgentAuthSessionSnapshot, AgentAuthState,
    AgentInstallReadinessDto, AgentInstallState, AgentInstallationInventoryDto, AgentReasonCode,
    AgentSourceKind, AgentSurface, AgentUpdateState, InstallationInventoryState,
    StartAgentActionRequest, StartAgentAuthSessionRequest, AGENT_ACTION_CONTRACT_VERSION,
    AGENT_INSTALL_READINESS_CONTRACT_VERSION, AGENT_INSTALL_READINESS_REVIEWED_AT,
};

use auth_actions::observe_auth_state;
use cli::{observe_cli, run_cli_lifecycle};
#[cfg(target_os = "windows")]
use desktop::{
    capture_desktop_installation_baseline, download_windows_exe_to_job, verify_windows_deployment,
    verify_windows_exe_source, WindowsDeploymentExpectation,
};
use desktop::{
    launch_desktop_installation, readiness_source_codes, resolve_desktop_source, source_reason,
};
use fetch::download_macos_dmg_to_job;
use inventory::{
    inventory_readiness_projection, validate_action_target, InventoryReadinessProjection,
    ValidatedActionTarget,
};
use jobs::download_progress_sink;
use lifecycle_policy::{admit_action, should_resolve_desktop_source, AgentLifecyclePolicy};
use macos::deploy_macos_dmg;
use sources::PackageFormat;

#[cfg(any(target_os = "windows", test))]
use crate::codex_desktop::error::InstallerErrorCode;
#[cfg(target_os = "windows")]
use crate::codex_desktop::platform::{
    windows::run_verified_agent_exe_installer, PlatformProgressSink,
};
use crate::codex_desktop::temp::JobTempRoot;
use crate::codex_desktop::types::LocalInstallStatus;
use crate::services::external_agents::AgentCatalogId;
use crate::store::AppState;
#[cfg(target_os = "windows")]
use fyagent_user_helper::AgentInstallerProduct;

fn desktop_versions_equivalent(local: &str, remote: &str) -> bool {
    if local == remote {
        return true;
    }
    let local_parts: Vec<&str> = local.split('.').collect();
    let remote_parts: Vec<&str> = remote.split('.').collect();
    let shared = local_parts.len().min(remote_parts.len());
    shared > 0
        && local_parts[..shared] == remote_parts[..shared]
        && local_parts.len() != remote_parts.len()
}

pub async fn readiness_for(agent_id: AgentCatalogId, state: &AppState) -> AgentInstallReadinessDto {
    let inventory = inventory_readiness_projection(agent_id, state).await;
    let mut readiness = match agent_id {
        AgentCatalogId::Codex => codex_readiness(state).await,
        AgentCatalogId::GrokBuild => cli_readiness(agent_id).await,
        AgentCatalogId::ClaudeCode
        | AgentCatalogId::OpenCode
        | AgentCatalogId::QoderWork
        | AgentCatalogId::TraeWork
        | AgentCatalogId::WorkBuddy => desktop_readiness(agent_id, &inventory).await,
    };
    apply_inventory_overlay(&mut readiness, &inventory);
    readiness
}

fn apply_inventory_overlay(
    readiness: &mut AgentInstallReadinessDto,
    inventory: &InventoryReadinessProjection,
) {
    readiness.inventory_state = inventory.state;
    readiness.requires_target_selection =
        matches!(inventory.state, InstallationInventoryState::Multiple);
    if readiness.source_kind != AgentSourceKind::ManagedDesktop
        && matches!(
            inventory.state,
            InstallationInventoryState::Multiple | InstallationInventoryState::Unknown
        )
        && matches!(
            readiness.install_state,
            AgentInstallState::Installed | AgentInstallState::InstalledNotRunnable
        )
    {
        readiness.install_state = AgentInstallState::Unknown;
        readiness.local_version = None;
        readiness.update_state = AgentUpdateState::Unknown;
    }
    for reason in &inventory.reason_codes {
        if !readiness.reason_codes.contains(reason) {
            readiness.reason_codes.push(*reason);
        }
    }
    if readiness.requires_target_selection
        && !readiness
            .reason_codes
            .contains(&AgentReasonCode::TargetSelectionRequired)
    {
        readiness
            .reason_codes
            .push(AgentReasonCode::TargetSelectionRequired);
    }
}

async fn cli_readiness(agent_id: AgentCatalogId) -> AgentInstallReadinessDto {
    let observation = observe_cli(agent_id).await;
    let (install_state, local_version, remote_version, unavailable) = match &observation {
        Some(value) if value.unavailable => (AgentInstallState::Unavailable, None, None, true),
        Some(value) if value.runnable => (
            AgentInstallState::Installed,
            value.local_version.clone(),
            value.latest_version.clone(),
            false,
        ),
        Some(value) if value.detected => (
            AgentInstallState::InstalledNotRunnable,
            value.local_version.clone(),
            value.latest_version.clone(),
            false,
        ),
        Some(value) => (
            AgentInstallState::NotInstalled,
            None,
            value.latest_version.clone(),
            false,
        ),
        None => (AgentInstallState::Unknown, None, None, false),
    };
    let update_state = cli_update_state(
        install_state,
        local_version.as_deref(),
        remote_version.as_deref(),
    );
    let auth_ownership = if agent_id == AgentCatalogId::OpenCode {
        AgentAuthOwnership::ProviderOwned
    } else {
        AgentAuthOwnership::AgentOwned
    };
    let auth_state = observe_auth_state(
        agent_id,
        observation.as_ref().is_some_and(|value| value.detected),
        unavailable,
    );
    let mut reason_codes = Vec::new();
    let mut allowed_actions = Vec::new();
    if unavailable {
        reason_codes.push(AgentReasonCode::InteractiveUserUnavailable);
    } else {
        if install_state == AgentInstallState::NotInstalled
            || install_state == AgentInstallState::Unknown
        {
            allowed_actions.push(AgentActionId::Install);
        }
        if matches!(
            update_state,
            AgentUpdateState::UpdateAvailable | AgentUpdateState::LatestUnknown
        ) && install_state != AgentInstallState::NotInstalled
        {
            allowed_actions.push(AgentActionId::Update);
        }
        if install_state == AgentInstallState::NotInstalled
            && update_state == AgentUpdateState::UpdateAvailable
        {
            // first install uses Install, not Update
        }
    }
    if install_state == AgentInstallState::InstalledNotRunnable {
        reason_codes.push(AgentReasonCode::InstalledNotRunnable);
    }
    if auth_state == AgentAuthState::Unknown {
        reason_codes.push(AgentReasonCode::AuthStateUnknown);
    }
    if auth_state == AgentAuthState::ProviderConnectionRequired {
        reason_codes.push(AgentReasonCode::ProviderConnectionRequired);
    }
    AgentInstallReadinessDto {
        contract_version: AGENT_INSTALL_READINESS_CONTRACT_VERSION,
        agent_id,
        reviewed_at: AGENT_INSTALL_READINESS_REVIEWED_AT,
        install_state,
        inventory_state: InstallationInventoryState::Unknown,
        requires_target_selection: false,
        update_state,
        release_id: None,
        local_version,
        remote_version,
        auth_ownership,
        auth_state,
        source_kind: AgentSourceKind::CliTooling,
        allowed_actions,
        reason_codes,
        surfaces: Vec::new(),
    }
}

fn cli_update_state(
    install_state: AgentInstallState,
    local: Option<&str>,
    remote: Option<&str>,
) -> AgentUpdateState {
    match (install_state, local, remote) {
        (AgentInstallState::Unavailable, _, _) => AgentUpdateState::Unavailable,
        (_, _, None) => AgentUpdateState::Unknown,
        (
            AgentInstallState::Installed | AgentInstallState::InstalledNotRunnable,
            Some(local),
            Some(remote),
        ) if local == remote => AgentUpdateState::UpToDate,
        (_, _, Some(_)) => AgentUpdateState::UpdateAvailable,
    }
}

async fn desktop_readiness(
    agent_id: AgentCatalogId,
    inventory: &InventoryReadinessProjection,
) -> AgentInstallReadinessDto {
    let mut reason_codes = Vec::new();
    let mut allowed_actions = Vec::new();
    let install_state = desktop_install_state_from_inventory(inventory);
    let policy = match lifecycle_policy::lifecycle_policy(agent_id, AgentSurface::Desktop) {
        Ok(policy) => policy,
        Err(reason) => {
            reason_codes.push(reason);
            return AgentInstallReadinessDto {
                contract_version: AGENT_INSTALL_READINESS_CONTRACT_VERSION,
                agent_id,
                reviewed_at: AGENT_INSTALL_READINESS_REVIEWED_AT,
                install_state: AgentInstallState::Unavailable,
                inventory_state: InstallationInventoryState::Unknown,
                requires_target_selection: false,
                update_state: AgentUpdateState::Unavailable,
                release_id: None,
                local_version: None,
                remote_version: None,
                auth_ownership: desktop_auth_ownership(agent_id),
                auth_state: AgentAuthState::Unavailable,
                source_kind: AgentSourceKind::ManagedDesktop,
                allowed_actions,
                reason_codes,
                surfaces: Vec::new(),
            };
        }
    };
    let (release_id, remote_version, source_ok, package_installable) =
        if should_resolve_desktop_source(policy, install_state) {
            match resolve_desktop_source(agent_id).await {
                Ok(resolved) => {
                    let installable = (cfg!(target_os = "macos")
                        && resolved.format == PackageFormat::Dmg
                        && resolved.platform == sources::AgentPlatform::Macos)
                        || (cfg!(target_os = "windows")
                            && resolved.format == PackageFormat::Exe
                            && resolved.platform == sources::AgentPlatform::Windows);
                    (
                        Some(resolved.release_id.clone()),
                        resolved.display_version.clone(),
                        true,
                        installable,
                    )
                }
                Err(error) => {
                    reason_codes.extend(readiness_source_codes(error));
                    (None, None, false, false)
                }
            }
        } else {
            (None, None, false, false)
        };
    let local_version = inventory.single_local_version.clone();
    let update_state = if should_resolve_desktop_source(policy, install_state) {
        desktop_update_state(
            source_ok,
            install_state,
            local_version.as_deref(),
            remote_version.as_deref(),
        )
    } else {
        skipped_desktop_source_update_state(policy, install_state)
    };
    allowed_actions = desktop_allowed_actions(policy, inventory, update_state, package_installable);
    if install_state == AgentInstallState::InstalledNotRunnable {
        reason_codes.push(AgentReasonCode::InstalledNotRunnable);
    }
    let auth_state = observe_auth_state(agent_id, false, false);
    if auth_state == AgentAuthState::Unknown {
        reason_codes.push(AgentReasonCode::AuthStateUnknown);
    }
    AgentInstallReadinessDto {
        contract_version: AGENT_INSTALL_READINESS_CONTRACT_VERSION,
        agent_id,
        reviewed_at: AGENT_INSTALL_READINESS_REVIEWED_AT,
        install_state,
        inventory_state: InstallationInventoryState::Unknown,
        requires_target_selection: false,
        update_state,
        release_id,
        local_version,
        remote_version,
        auth_ownership: desktop_auth_ownership(agent_id),
        auth_state,
        source_kind: AgentSourceKind::ManagedDesktop,
        allowed_actions,
        reason_codes,
        surfaces: Vec::new(),
    }
}

fn desktop_allowed_actions(
    policy: &AgentLifecyclePolicy,
    inventory: &InventoryReadinessProjection,
    update_state: AgentUpdateState,
    package_installable: bool,
) -> Vec<AgentActionId> {
    let mut allowed_actions = Vec::new();
    if package_installable {
        match inventory.state {
            InstallationInventoryState::NotObserved if policy.install => {
                allowed_actions.push(AgentActionId::Install)
            }
            InstallationInventoryState::Single
                if policy.update
                    && inventory.single_update_eligible
                    && update_state != AgentUpdateState::UpToDate =>
            {
                allowed_actions.push(AgentActionId::Update)
            }
            _ => {}
        }
    }
    if inventory.state == InstallationInventoryState::Single
        && inventory.single_launch_eligible
        && policy.launch
    {
        allowed_actions.push(AgentActionId::Launch);
    }
    allowed_actions
}

fn desktop_auth_ownership(agent_id: AgentCatalogId) -> AgentAuthOwnership {
    if agent_id == AgentCatalogId::OpenCode {
        AgentAuthOwnership::ProviderOwned
    } else {
        AgentAuthOwnership::AgentOwned
    }
}

fn skipped_desktop_source_update_state(
    policy: &AgentLifecyclePolicy,
    install_state: AgentInstallState,
) -> AgentUpdateState {
    let install_only_skip = matches!(
        install_state,
        AgentInstallState::Installed | AgentInstallState::InstalledNotRunnable
    ) && !policy.update;
    if install_only_skip || install_state == AgentInstallState::Unavailable {
        AgentUpdateState::Unavailable
    } else {
        AgentUpdateState::Unknown
    }
}

fn desktop_install_state_from_inventory(
    inventory: &InventoryReadinessProjection,
) -> AgentInstallState {
    match inventory.state {
        InstallationInventoryState::NotObserved => AgentInstallState::NotInstalled,
        InstallationInventoryState::Single if inventory.single_launch_eligible => {
            AgentInstallState::Installed
        }
        InstallationInventoryState::Single => AgentInstallState::InstalledNotRunnable,
        InstallationInventoryState::Multiple | InstallationInventoryState::Unknown => {
            AgentInstallState::Unknown
        }
        InstallationInventoryState::Unsupported => AgentInstallState::Unavailable,
    }
}

fn desktop_update_state(
    source_ok: bool,
    install_state: AgentInstallState,
    local: Option<&str>,
    remote: Option<&str>,
) -> AgentUpdateState {
    if !source_ok || install_state == AgentInstallState::Unavailable {
        return AgentUpdateState::Unavailable;
    }
    match install_state {
        AgentInstallState::Unknown => AgentUpdateState::Unknown,
        AgentInstallState::NotInstalled => {
            if remote.is_some() {
                AgentUpdateState::UpdateAvailable
            } else {
                AgentUpdateState::LatestUnknown
            }
        }
        AgentInstallState::Installed | AgentInstallState::InstalledNotRunnable => {
            match (local, remote) {
                (Some(local), Some(remote)) if desktop_versions_equivalent(local, remote) => {
                    AgentUpdateState::UpToDate
                }
                (_, Some(_)) => AgentUpdateState::UpdateAvailable,
                (_, None) => AgentUpdateState::LatestUnknown,
            }
        }
        AgentInstallState::Unavailable => AgentUpdateState::Unavailable,
    }
}

async fn codex_readiness(state: &AppState) -> AgentInstallReadinessDto {
    let local = state.codex_desktop_service.get_local_status().await.ok();
    let (install_state, local_version) = match local {
        Some(LocalInstallStatus::Installed { application }) => {
            (AgentInstallState::Installed, application.display_version)
        }
        Some(LocalInstallStatus::NotInstalled { .. }) => (AgentInstallState::NotInstalled, None),
        Some(LocalInstallStatus::Unsupported { .. }) => (AgentInstallState::Unavailable, None),
        Some(LocalInstallStatus::Ambiguous { .. }) => (AgentInstallState::Unknown, None),
        None => (AgentInstallState::Unknown, None),
    };
    let remote = state.codex_desktop_service.check_latest(false).await.ok();
    let (release_id, remote_version, update_state) = match remote {
        Some(status) => {
            let update = match (local_version.as_deref(), status.display_version.as_str()) {
                (Some(local), remote) if local == remote => AgentUpdateState::UpToDate,
                (Some(_), _) => AgentUpdateState::UpdateAvailable,
                (None, _) if install_state == AgentInstallState::Installed => {
                    AgentUpdateState::LatestUnknown
                }
                (None, _) => AgentUpdateState::UpdateAvailable,
            };
            (
                Some(status.release_id),
                Some(status.display_version),
                update,
            )
        }
        None => (None, None, AgentUpdateState::Unknown),
    };
    AgentInstallReadinessDto {
        contract_version: AGENT_INSTALL_READINESS_CONTRACT_VERSION,
        agent_id: AgentCatalogId::Codex,
        reviewed_at: AGENT_INSTALL_READINESS_REVIEWED_AT,
        install_state,
        inventory_state: InstallationInventoryState::Unknown,
        requires_target_selection: false,
        update_state,
        release_id,
        local_version,
        remote_version,
        auth_ownership: AgentAuthOwnership::FyagentManaged,
        auth_state: AgentAuthState::Unknown,
        source_kind: AgentSourceKind::CodexDesktop,
        allowed_actions: Vec::new(),
        reason_codes: vec![
            AgentReasonCode::ManagedByCodexDesktop,
            AgentReasonCode::AuthStateUnknown,
        ],
        surfaces: Vec::new(),
    }
}

pub async fn start_agent_action(
    request: StartAgentActionRequest,
    state: &AppState,
) -> Result<AgentActionResult, AgentReasonCode> {
    let surface = resolve_requested_surface(request.agent_id, request.surface)?;
    lifecycle_policy::lifecycle_policy(request.agent_id, surface)?;
    if matches!(
        request.action,
        AgentActionId::AuthLogin | AgentActionId::AuthLogout | AgentActionId::AuthConnectProvider
    ) {
        return Err(AgentReasonCode::ExecutorNotImplemented);
    }
    admit_action(request.agent_id, surface, request.action)?;
    if let Some(release_id) = request.expected_release_id.as_deref() {
        if !validate_opaque_release_id(release_id) {
            return Err(AgentReasonCode::RefreshRequired);
        }
    }
    let target = validate_action_target(&request, state).await?;
    match (request.agent_id, surface, request.action) {
        (
            AgentCatalogId::Codex,
            AgentSurface::Desktop,
            AgentActionId::Install | AgentActionId::Update,
        ) => Err(AgentReasonCode::ManagedByCodexDesktop),
        (AgentCatalogId::Codex, AgentSurface::Desktop, AgentActionId::Launch) => {
            state
                .codex_desktop_service
                .launch()
                .await
                .map_err(|_| AgentReasonCode::ApplicationLaunchFailed)?;
            Ok(immediate_result(
                request.agent_id,
                request.action,
                surface,
                AgentActionJobStage::Succeeded,
                None,
            ))
        }
        (
            AgentCatalogId::GrokBuild,
            AgentSurface::Cli,
            AgentActionId::Install | AgentActionId::Update,
        ) => {
            run_cli_lifecycle(request.agent_id, request.action).await?;
            Ok(immediate_result(
                request.agent_id,
                request.action,
                surface,
                AgentActionJobStage::Succeeded,
                None,
            ))
        }
        (
            AgentCatalogId::QoderWork
            | AgentCatalogId::TraeWork
            | AgentCatalogId::WorkBuddy
            | AgentCatalogId::OpenCode
            | AgentCatalogId::ClaudeCode,
            AgentSurface::Desktop,
            AgentActionId::Install | AgentActionId::Update,
        ) => start_desktop_job(request, surface, state, target).await,
        (
            AgentCatalogId::QoderWork
            | AgentCatalogId::TraeWork
            | AgentCatalogId::WorkBuddy
            | AgentCatalogId::OpenCode
            | AgentCatalogId::ClaudeCode,
            AgentSurface::Desktop,
            AgentActionId::Launch,
        ) => {
            let path = target
                .desktop_path()
                .ok_or(AgentReasonCode::TargetNotExecutable)?;
            launch_desktop_installation(request.agent_id, path)?;
            Ok(immediate_result(
                request.agent_id,
                request.action,
                surface,
                AgentActionJobStage::Succeeded,
                None,
            ))
        }
        _ => Err(AgentReasonCode::SurfaceNotSupported),
    }
}

async fn start_desktop_job(
    request: StartAgentActionRequest,
    surface: AgentSurface,
    state: &AppState,
    target: ValidatedActionTarget,
) -> Result<AgentActionResult, AgentReasonCode> {
    match request.action {
        AgentActionId::Install if target.fresh_destination().is_none() => {
            return Err(AgentReasonCode::TargetSelectionRequired);
        }
        AgentActionId::Update if target.desktop_path().is_none() => {
            return Err(AgentReasonCode::TargetSelectionRequired);
        }
        _ => {}
    }
    let deployment_target = target
        .into_desktop_deployment_target()
        .ok_or(AgentReasonCode::TargetSelectionRequired)?;
    let source = resolve_desktop_source(request.agent_id)
        .await
        .map_err(source_reason)?;
    if !source.versionless_latest {
        let expected = request
            .expected_release_id
            .as_deref()
            .ok_or(AgentReasonCode::RefreshRequired)?;
        if expected != source.release_id {
            return Err(AgentReasonCode::RefreshRequired);
        }
    } else if let Some(expected) = request.expected_release_id.as_deref() {
        if expected != source.release_id {
            return Err(AgentReasonCode::RefreshRequired);
        }
    }
    let (snapshot, cancel) =
        state
            .agent_action_jobs
            .start(request.agent_id, request.action, surface)?;
    let job_id = snapshot.job_id.clone();
    let jobs = Arc::clone(&state.agent_action_jobs);
    tokio::spawn(async move {
        run_desktop_install_job(jobs, job_id, source, deployment_target, cancel).await;
    });
    Ok(AgentActionResult {
        contract_version: AGENT_ACTION_CONTRACT_VERSION,
        agent_id: request.agent_id,
        action: request.action,
        job_id: Some(snapshot.job_id),
        stage: snapshot.stage,
        reason_code: None,
        surface,
    })
}

async fn run_desktop_install_job(
    jobs: Arc<AgentActionJobStore>,
    job_id: String,
    source: sources::ResolvedDesktopSource,
    target: inventory::DesktopDeploymentTarget,
    cancel: Arc<AtomicBool>,
) {
    #[cfg(target_os = "windows")]
    if source.platform == sources::AgentPlatform::Windows {
        run_windows_desktop_install_job(jobs, job_id, source, target, cancel).await;
        return;
    }

    if jobs.is_cancelled(&cancel) {
        let _ = jobs.transition(
            &job_id,
            AgentActionJobStage::Cancelled,
            Some(AgentReasonCode::Cancelled),
        );
        return;
    }
    let job_directory = match JobTempRoot::for_current_process().create_job(&job_id) {
        Ok(directory) => directory,
        Err(_) => {
            let _ = jobs.transition(
                &job_id,
                AgentActionJobStage::Failed,
                Some(AgentReasonCode::InstallerArtifactUnavailable),
            );
            return;
        }
    };
    if jobs.is_cancelled(&cancel) {
        let _ = job_directory.cleanup();
        let _ = jobs.transition(
            &job_id,
            AgentActionJobStage::Cancelled,
            Some(AgentReasonCode::Cancelled),
        );
        return;
    }
    let _ = jobs.transition(&job_id, AgentActionJobStage::Downloading, None);
    if jobs.is_cancelled(&cancel) {
        let _ = job_directory.cleanup();
        let _ = jobs.transition(
            &job_id,
            AgentActionJobStage::Cancelled,
            Some(AgentReasonCode::Cancelled),
        );
        return;
    }
    let progress = download_progress_sink(Arc::clone(&jobs), job_id.clone());
    let artifact = match download_macos_dmg_to_job(
        &source,
        &job_directory,
        cancel.as_ref(),
        &progress,
    )
    .await
    {
        Ok(artifact) => artifact,
        Err(AgentReasonCode::Cancelled) => {
            let _ = job_directory.cleanup();
            let _ = jobs.transition(
                &job_id,
                AgentActionJobStage::Cancelled,
                Some(AgentReasonCode::Cancelled),
            );
            return;
        }
        Err(reason) => {
            let _ = job_directory.cleanup();
            let _ = jobs.transition(&job_id, AgentActionJobStage::Failed, Some(reason));
            return;
        }
    };
    if jobs.is_cancelled(&cancel) {
        let _ = job_directory.cleanup();
        let _ = jobs.transition(
            &job_id,
            AgentActionJobStage::Cancelled,
            Some(AgentReasonCode::Cancelled),
        );
        return;
    }
    let _ = jobs.transition(&job_id, AgentActionJobStage::Staging, None);
    let jobs_for_commit = Arc::clone(&jobs);
    let jobs_for_verify = Arc::clone(&jobs);
    let commit_job_id = job_id.clone();
    let verify_job_id = job_id.clone();
    let cancel_for_commit = Arc::clone(&cancel);
    let product = source.product;
    let expected_release_version = source.display_version.clone();
    let deployment = tokio::task::spawn_blocking(move || {
        deploy_macos_dmg(
            product,
            &artifact,
            target,
            expected_release_version,
            |awaiting_user| {
                if jobs_for_commit.is_cancelled(&cancel_for_commit) {
                    return Err(AgentReasonCode::Cancelled);
                }
                jobs_for_commit
                    .transition(
                        &commit_job_id,
                        if awaiting_user {
                            AgentActionJobStage::AwaitingUser
                        } else {
                            AgentActionJobStage::Installing
                        },
                        None,
                    )
                    .map(|_| ())
            },
            || {
                jobs_for_verify
                    .transition(
                        &verify_job_id,
                        AgentActionJobStage::VerifyingInstallation,
                        None,
                    )
                    .map(|_| ())
            },
        )
    })
    .await;
    let cleanup_failed = job_directory.cleanup().is_err();
    match deployment {
        Ok(Ok(_)) if cleanup_failed => {
            let _ = jobs.transition(
                &job_id,
                AgentActionJobStage::Incomplete,
                Some(AgentReasonCode::RecoveryRequired),
            );
        }
        Ok(Ok(_)) => {
            let _ = jobs.transition(&job_id, AgentActionJobStage::Succeeded, None);
        }
        Ok(Err(AgentReasonCode::Cancelled)) => {
            let _ = jobs.transition(
                &job_id,
                AgentActionJobStage::Cancelled,
                Some(AgentReasonCode::Cancelled),
            );
        }
        Ok(Err(_)) if cleanup_failed => {
            let _ = jobs.transition(
                &job_id,
                AgentActionJobStage::Failed,
                Some(AgentReasonCode::RecoveryRequired),
            );
        }
        Ok(Err(reason)) => {
            let _ = jobs.transition(&job_id, AgentActionJobStage::Failed, Some(reason));
        }
        Err(_) => {
            let _ = jobs.transition(
                &job_id,
                AgentActionJobStage::Failed,
                Some(AgentReasonCode::RecoveryRequired),
            );
        }
    }
}

#[cfg(target_os = "windows")]
enum WindowsInstallerOutcome {
    Rejected(AgentReasonCode),
    CancelledBeforeCommit,
    Invoked(Result<(), AgentReasonCode>),
}

#[cfg(target_os = "windows")]
async fn run_windows_desktop_install_job(
    jobs: Arc<AgentActionJobStore>,
    job_id: String,
    source: sources::ResolvedDesktopSource,
    target: inventory::DesktopDeploymentTarget,
    cancel: Arc<AtomicBool>,
) {
    let expectation = match windows_deployment_expectation(target) {
        Ok(expectation) => expectation,
        Err(reason) => {
            let _ = jobs.transition(&job_id, AgentActionJobStage::Failed, Some(reason));
            return;
        }
    };
    let baseline = capture_desktop_installation_baseline(source.product);
    if !baseline.complete() {
        let _ = jobs.transition(
            &job_id,
            AgentActionJobStage::Failed,
            Some(AgentReasonCode::NativeProjectionUnavailable),
        );
        return;
    }
    let job_directory = match JobTempRoot::for_current_process().create_job(&job_id) {
        Ok(directory) => directory,
        Err(_) => {
            let _ = jobs.transition(
                &job_id,
                AgentActionJobStage::Failed,
                Some(AgentReasonCode::InstallerArtifactUnavailable),
            );
            return;
        }
    };

    if jobs.is_cancelled(&cancel) {
        let _ = job_directory.cleanup();
        let _ = jobs.transition(
            &job_id,
            AgentActionJobStage::Cancelled,
            Some(AgentReasonCode::Cancelled),
        );
        return;
    }
    let _ = jobs.transition(&job_id, AgentActionJobStage::Downloading, None);
    let progress = download_progress_sink(Arc::clone(&jobs), job_id.clone());
    let artifact = match download_windows_exe_to_job(
        &source,
        &job_directory,
        cancel.as_ref(),
        &progress,
    )
    .await
    {
        Ok(artifact) => artifact,
        Err(AgentReasonCode::Cancelled) => {
            let _ = job_directory.cleanup();
            let _ = jobs.transition(
                &job_id,
                AgentActionJobStage::Cancelled,
                Some(AgentReasonCode::Cancelled),
            );
            return;
        }
        Err(reason) => {
            let _ = job_directory.cleanup();
            let _ = jobs.transition(&job_id, AgentActionJobStage::Failed, Some(reason));
            return;
        }
    };

    let _ = jobs.transition(&job_id, AgentActionJobStage::Staging, None);
    let helper_product = match agent_helper_product(source.product) {
        Ok(product) => product,
        Err(reason) => {
            drop(artifact);
            let _ = job_directory.cleanup();
            let _ = jobs.transition(&job_id, AgentActionJobStage::Failed, Some(reason));
            return;
        }
    };
    let jobs_for_install = Arc::clone(&jobs);
    let install_job_id = job_id.clone();
    let source_for_install = source.clone();
    let cancel_for_install = Arc::clone(&cancel);
    let outcome = tokio::task::spawn_blocking(move || {
        if let Err(reason) = verify_windows_exe_source(&source_for_install, &artifact) {
            return WindowsInstallerOutcome::Rejected(reason);
        }
        if jobs_for_install.is_cancelled(&cancel_for_install) {
            return WindowsInstallerOutcome::CancelledBeforeCommit;
        }
        if jobs_for_install
            .transition(
                &install_job_id,
                AgentActionJobStage::LaunchingInstaller,
                None,
            )
            .is_err()
        {
            return WindowsInstallerOutcome::Rejected(AgentReasonCode::OperationConflict);
        }
        let Some(context) = crate::windows_runtime::interactive_user_context() else {
            return WindowsInstallerOutcome::Rejected(AgentReasonCode::InteractiveUserUnavailable);
        };
        let progress_jobs = Arc::clone(&jobs_for_install);
        let progress_job_id = install_job_id.clone();
        let progress: PlatformProgressSink = Arc::new(move |_| {
            let _ =
                progress_jobs.transition(&progress_job_id, AgentActionJobStage::AwaitingUser, None);
        });
        WindowsInstallerOutcome::Invoked(
            run_verified_agent_exe_installer(
                context,
                helper_product,
                &install_job_id,
                &artifact,
                progress,
            )
            .map_err(map_windows_installer_error),
        )
    })
    .await;

    let outcome = match outcome {
        Ok(outcome) => outcome,
        Err(_) => {
            let _ = job_directory.cleanup();
            let _ = jobs.transition(
                &job_id,
                AgentActionJobStage::Incomplete,
                Some(AgentReasonCode::RecoveryRequired),
            );
            return;
        }
    };
    if job_directory.cleanup().is_err() {
        let stage = if matches!(&outcome, WindowsInstallerOutcome::Invoked(_)) {
            AgentActionJobStage::Incomplete
        } else {
            AgentActionJobStage::Failed
        };
        let _ = jobs.transition(&job_id, stage, Some(AgentReasonCode::RecoveryRequired));
        return;
    }
    match outcome {
        WindowsInstallerOutcome::Rejected(reason) => {
            let _ = jobs.transition(&job_id, AgentActionJobStage::Failed, Some(reason));
        }
        WindowsInstallerOutcome::CancelledBeforeCommit => {
            let _ = jobs.transition(
                &job_id,
                AgentActionJobStage::Cancelled,
                Some(AgentReasonCode::Cancelled),
            );
        }
        WindowsInstallerOutcome::Invoked(helper_result) => {
            let _ = jobs.transition(&job_id, AgentActionJobStage::VerifyingInstallation, None);
            let verified = wait_for_windows_deployment(
                source.product,
                &baseline,
                &expectation,
                source.display_version.as_deref(),
            )
            .await;
            if verified.is_ok() {
                let _ = jobs.transition(&job_id, AgentActionJobStage::Succeeded, None);
                return;
            }
            let verification_reason = verified
                .err()
                .unwrap_or(AgentReasonCode::InstallationVerificationFailed);
            match helper_result {
                Err(AgentReasonCode::InstallerUserCancelled) => {
                    let _ = jobs.transition(
                        &job_id,
                        AgentActionJobStage::Cancelled,
                        Some(AgentReasonCode::InstallerUserCancelled),
                    );
                }
                Err(
                    reason @ (AgentReasonCode::InstallerProcessUnobservable
                    | AgentReasonCode::InstallerTimedOut),
                ) => {
                    let _ = jobs.transition(&job_id, AgentActionJobStage::Incomplete, Some(reason));
                }
                Err(reason) => {
                    let _ = jobs.transition(&job_id, AgentActionJobStage::Failed, Some(reason));
                }
                Ok(()) => {
                    let _ = jobs.transition(
                        &job_id,
                        AgentActionJobStage::Incomplete,
                        Some(verification_reason),
                    );
                }
            }
        }
    }
}

#[cfg(target_os = "windows")]
fn windows_deployment_expectation(
    target: inventory::DesktopDeploymentTarget,
) -> Result<WindowsDeploymentExpectation, AgentReasonCode> {
    match target {
        inventory::DesktopDeploymentTarget::Existing {
            path,
            scope,
            package_kind: types::InstallationPackageKind::Exe,
        } => Ok(WindowsDeploymentExpectation::Existing { path, scope }),
        inventory::DesktopDeploymentTarget::Fresh(
            inventory::FreshDestinationCapability::WindowsCurrentUser,
        ) => Ok(WindowsDeploymentExpectation::FreshCurrentUser),
        inventory::DesktopDeploymentTarget::Fresh(
            inventory::FreshDestinationCapability::VendorInstallerChoice,
        ) => Ok(WindowsDeploymentExpectation::FreshVendorChoice),
        _ => Err(AgentReasonCode::TargetScopeUnsupported),
    }
}

#[cfg(target_os = "windows")]
fn agent_helper_product(
    agent_id: AgentCatalogId,
) -> Result<AgentInstallerProduct, AgentReasonCode> {
    match agent_id {
        AgentCatalogId::QoderWork => Ok(AgentInstallerProduct::QoderWork),
        AgentCatalogId::TraeWork => Ok(AgentInstallerProduct::TraeWork),
        AgentCatalogId::WorkBuddy => Ok(AgentInstallerProduct::WorkBuddy),
        _ => Err(AgentReasonCode::ExecutorNotImplemented),
    }
}

#[cfg(target_os = "windows")]
fn map_windows_installer_error(
    error: crate::codex_desktop::error::InstallerError,
) -> AgentReasonCode {
    let platform_error_code = error.to_dto().details.platform_error_code;
    map_windows_installer_error_parts(error.code(), platform_error_code.as_deref())
}

#[cfg(any(target_os = "windows", test))]
fn map_windows_installer_error_parts(
    code: InstallerErrorCode,
    platform_error_code: Option<&str>,
) -> AgentReasonCode {
    match platform_error_code {
        Some("agent_installer_user_cancelled") => return AgentReasonCode::InstallerUserCancelled,
        Some("agent_installer_process_unobservable") => {
            return AgentReasonCode::InstallerProcessUnobservable
        }
        Some("agent_installer_timed_out") => return AgentReasonCode::InstallerTimedOut,
        Some("agent_installer_exited_nonzero") => return AgentReasonCode::InstallerExitedNonzero,
        _ => {}
    }
    match code {
        InstallerErrorCode::DownloadCancelled => AgentReasonCode::Cancelled,
        InstallerErrorCode::DownloadFailed
        | InstallerErrorCode::DownloadTimeout
        | InstallerErrorCode::InsufficientDiskSpace
        | InstallerErrorCode::PackageParseFailed
        | InstallerErrorCode::PackageIdentityMismatch
        | InstallerErrorCode::ChecksumMismatch => AgentReasonCode::InstallerArtifactUnavailable,
        InstallerErrorCode::PackageArchitectureMismatch
        | InstallerErrorCode::ArchitectureUnsupported => AgentReasonCode::PlatformUnsupported,
        InstallerErrorCode::SourceUnavailable
        | InstallerErrorCode::ReleaseMetadataInvalid
        | InstallerErrorCode::ReleaseNotAvailable
        | InstallerErrorCode::RedirectRejected
        | InstallerErrorCode::PackageSignatureInvalid => AgentReasonCode::SourceNotVerified,
        InstallerErrorCode::MetadataChanged => AgentReasonCode::RefreshRequired,
        InstallerErrorCode::WindowsPackageInUse => AgentReasonCode::ApplicationRunning,
        InstallerErrorCode::WindowsDeploymentBlocked
        | InstallerErrorCode::WindowsDependencyMissing => AgentReasonCode::AuthorizationRequired,
        InstallerErrorCode::WindowsDeploymentFailed
        | InstallerErrorCode::LaunchFailed
        | InstallerErrorCode::InternalError => AgentReasonCode::InteractiveUserUnavailable,
        InstallerErrorCode::MultipleInstallations
        | InstallerErrorCode::InstallationVerifyFailed => {
            AgentReasonCode::InstallationVerificationFailed
        }
        InstallerErrorCode::JobAlreadyRunning | InstallerErrorCode::JobNotFound => {
            AgentReasonCode::OperationConflict
        }
        _ => AgentReasonCode::ExecutorNotImplemented,
    }
}

#[cfg(target_os = "windows")]
async fn wait_for_windows_deployment(
    agent_id: AgentCatalogId,
    baseline: &desktop::DesktopInstallationBaseline,
    expectation: &WindowsDeploymentExpectation,
    expected_local_version: Option<&str>,
) -> Result<(), AgentReasonCode> {
    let mut last_reason = AgentReasonCode::InstallationVerificationFailed;
    for attempt in 0..=90_u8 {
        match verify_windows_deployment(agent_id, baseline, expectation, expected_local_version) {
            Ok(()) => return Ok(()),
            Err(reason) => last_reason = reason,
        }
        if attempt < 90 {
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }
    Err(last_reason)
}

fn immediate_result(
    agent_id: AgentCatalogId,
    action: AgentActionId,
    surface: AgentSurface,
    stage: AgentActionJobStage,
    reason_code: Option<AgentReasonCode>,
) -> AgentActionResult {
    AgentActionResult {
        contract_version: AGENT_ACTION_CONTRACT_VERSION,
        agent_id,
        action,
        job_id: None,
        stage,
        reason_code,
        surface,
    }
}

pub fn cancel_agent_action(
    job_id: &str,
    state: &AppState,
) -> Result<AgentActionJobSnapshot, AgentReasonCode> {
    state.agent_action_jobs.request_cancel(job_id)
}

pub fn get_agent_action_job(
    job_id: &str,
    state: &AppState,
) -> Result<AgentActionJobSnapshot, AgentReasonCode> {
    state.agent_action_jobs.get(job_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    const IDS: [AgentCatalogId; 7] = [
        AgentCatalogId::QoderWork,
        AgentCatalogId::TraeWork,
        AgentCatalogId::WorkBuddy,
        AgentCatalogId::GrokBuild,
        AgentCatalogId::Codex,
        AgentCatalogId::ClaudeCode,
        AgentCatalogId::OpenCode,
    ];

    fn dto_keys() -> &'static [&'static str] {
        &[
            "agentId",
            "allowedActions",
            "authOwnership",
            "authState",
            "contractVersion",
            "installState",
            "inventoryState",
            "localVersion",
            "reasonCodes",
            "releaseId",
            "remoteVersion",
            "requiresTargetSelection",
            "reviewedAt",
            "sourceKind",
            "updateState",
        ]
    }

    fn forbidden_substrings() -> &'static [&'static str] {
        &[
            "http://",
            "https://",
            "token",
            "secret",
            "api_key",
            "apiKey",
            "sha256",
            "script",
            "packageFormat",
            "managed_package",
        ]
    }

    #[test]
    fn workbuddy_marketing_version_matches_longer_product_version() {
        assert!(desktop_versions_equivalent("5.3.14", "5.3.14.36279234"));
        assert!(desktop_versions_equivalent("5.3.14.36279234", "5.3.14"));
        assert!(desktop_versions_equivalent("0.9.15", "0.9.15"));
        assert!(!desktop_versions_equivalent("0.9.12", "0.9.15"));
        assert!(!desktop_versions_equivalent("2.3.71801", "2.3.76122"));
        assert!(!desktop_versions_equivalent("5.3.14", "5.3.15"));
    }

    fn inventory_projection(
        state: InstallationInventoryState,
        local_version: Option<&str>,
        launch_eligible: bool,
        update_eligible: bool,
    ) -> InventoryReadinessProjection {
        InventoryReadinessProjection {
            state,
            single_local_version: local_version.map(str::to_string),
            single_launch_eligible: launch_eligible,
            single_update_eligible: update_eligible,
            reason_codes: Vec::new(),
        }
    }

    #[test]
    fn desktop_readiness_uses_inventory_as_the_only_local_install_authority() {
        let custom = inventory_projection(
            InstallationInventoryState::Single,
            Some("5.3.14"),
            true,
            true,
        );
        assert_eq!(
            desktop_install_state_from_inventory(&custom),
            AgentInstallState::Installed
        );
        assert_eq!(
            desktop_update_state(
                true,
                AgentInstallState::Installed,
                custom.single_local_version.as_deref(),
                Some("5.3.14.36279234"),
            ),
            AgentUpdateState::UpToDate
        );

        let not_runnable = inventory_projection(
            InstallationInventoryState::Single,
            Some("1.0.0"),
            false,
            false,
        );
        assert_eq!(
            desktop_install_state_from_inventory(&not_runnable),
            AgentInstallState::InstalledNotRunnable
        );

        let absent =
            inventory_projection(InstallationInventoryState::NotObserved, None, false, false);
        assert_eq!(
            desktop_install_state_from_inventory(&absent),
            AgentInstallState::NotInstalled
        );

        for state in [
            InstallationInventoryState::Multiple,
            InstallationInventoryState::Unknown,
        ] {
            let ambiguous = inventory_projection(state, Some("9.9.9"), true, true);
            assert_eq!(
                desktop_install_state_from_inventory(&ambiguous),
                AgentInstallState::Unknown
            );
            assert_eq!(
                desktop_update_state(
                    true,
                    AgentInstallState::Unknown,
                    ambiguous.single_local_version.as_deref(),
                    Some("10.0.0"),
                ),
                AgentUpdateState::Unknown
            );
        }
    }

    #[test]
    fn windows_installer_errors_never_mislabel_known_failures_as_unimplemented() {
        assert_eq!(
            map_windows_installer_error_parts(
                InstallerErrorCode::WindowsDeploymentFailed,
                Some("agent_installer_process_unobservable"),
            ),
            AgentReasonCode::InstallerProcessUnobservable
        );
        assert_eq!(
            map_windows_installer_error_parts(InstallerErrorCode::DownloadFailed, None),
            AgentReasonCode::InstallerArtifactUnavailable
        );
        assert_eq!(
            map_windows_installer_error_parts(InstallerErrorCode::InstallationVerifyFailed, None,),
            AgentReasonCode::InstallationVerificationFailed
        );
        assert_eq!(
            map_windows_installer_error_parts(InstallerErrorCode::JobAlreadyRunning, None),
            AgentReasonCode::OperationConflict
        );
    }

    #[test]
    fn catalog_ids_remain_the_canonical_seven() {
        let encoded: Vec<_> = IDS
            .into_iter()
            .map(|id| serde_json::to_value(id).unwrap())
            .collect();
        assert_eq!(
            encoded,
            [
                "qoderwork",
                "trae-work",
                "workbuddy",
                "grokbuild",
                "codex",
                "claude-code",
                "opencode",
            ]
        );
        for invalid in ["pi", "qoderwork-cn", "codex-cli", "claude"] {
            assert!(serde_json::from_str::<AgentCatalogId>(&format!("\"{invalid}\"")).is_err());
        }
    }

    #[test]
    fn start_request_rejects_path_url_and_bypass_fields() {
        assert!(
            serde_json::from_value::<StartAgentActionRequest>(serde_json::json!({
                "agentId": "qoderwork",
                "action": "install",
                "url": "https://example.invalid"
            }))
            .is_err()
        );
        assert!(
            serde_json::from_value::<StartAgentActionRequest>(serde_json::json!({
                "agentId": "claude-code",
                "action": "install",
                "path": "/tmp/installer"
            }))
            .is_err()
        );
        assert!(
            serde_json::from_value::<StartAgentActionRequest>(serde_json::json!({
                "agentId": "workbuddy",
                "action": "install",
                "bypass": true
            }))
            .is_err()
        );
        let ok = serde_json::from_value::<StartAgentActionRequest>(serde_json::json!({
            "agentId": "trae-work",
            "action": "update",
            "expectedReleaseId": "v1:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        }))
        .unwrap();
        assert_eq!(ok.agent_id, AgentCatalogId::TraeWork);
    }

    #[test]
    fn readiness_wire_omits_sensitive_and_legacy_fields() {
        let dto = AgentInstallReadinessDto {
            contract_version: AGENT_INSTALL_READINESS_CONTRACT_VERSION,
            agent_id: AgentCatalogId::QoderWork,
            reviewed_at: AGENT_INSTALL_READINESS_REVIEWED_AT,
            install_state: AgentInstallState::Unknown,
            inventory_state: InstallationInventoryState::Unknown,
            requires_target_selection: false,
            update_state: AgentUpdateState::LatestUnknown,
            release_id: Some(
                "v1:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
            ),
            local_version: None,
            remote_version: None,
            auth_ownership: AgentAuthOwnership::AgentOwned,
            auth_state: AgentAuthState::Unknown,
            source_kind: AgentSourceKind::ManagedDesktop,
            allowed_actions: vec![AgentActionId::Install],
            reason_codes: vec![AgentReasonCode::AuthStateUnknown],
            surfaces: Vec::new(),
        };
        let value = serde_json::to_value(&dto).unwrap();
        let mut keys: Vec<_> = value
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(keys, dto_keys());
        let encoded = value.to_string();
        for needle in forbidden_substrings() {
            assert!(
                !encoded
                    .to_ascii_lowercase()
                    .contains(&needle.to_ascii_lowercase())
                    || *needle == "token" && !encoded.contains("token"),
                "forbidden {needle} in {encoded}"
            );
        }
        assert_eq!(
            value["contractVersion"],
            AGENT_INSTALL_READINESS_CONTRACT_VERSION
        );
        assert!(value.get("automation").is_none());
        assert!(value.get("plan").is_none());
        assert!(value.get("integrity").is_none());
        assert!(value.get("surfaces").is_none());
    }

    #[test]
    fn opencode_readiness_wire_is_compact_desktop_only() {
        let dto = AgentInstallReadinessDto {
            contract_version: AGENT_INSTALL_READINESS_CONTRACT_VERSION,
            agent_id: AgentCatalogId::OpenCode,
            reviewed_at: AGENT_INSTALL_READINESS_REVIEWED_AT,
            install_state: AgentInstallState::NotInstalled,
            inventory_state: InstallationInventoryState::NotObserved,
            requires_target_selection: false,
            update_state: AgentUpdateState::LatestUnknown,
            release_id: Some(
                "v1:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
            ),
            local_version: None,
            remote_version: None,
            auth_ownership: AgentAuthOwnership::ProviderOwned,
            auth_state: AgentAuthState::Unknown,
            source_kind: AgentSourceKind::ManagedDesktop,
            allowed_actions: vec![AgentActionId::Install],
            reason_codes: vec![AgentReasonCode::AuthStateUnknown],
            surfaces: Vec::new(),
        };
        let value = serde_json::to_value(&dto).unwrap();
        assert!(value.get("surfaces").is_none());
        assert_eq!(value["sourceKind"], "managed_desktop");
        assert_eq!(value["authOwnership"], "provider_owned");
        assert_eq!(value["allowedActions"], serde_json::json!(["install"]));
        let encoded = value.to_string();
        assert!(!encoded.contains("https://"));
        assert!(!encoded.contains("ai.opencode.desktop"));
        assert!(!encoded.contains("cli_tooling"));
    }

    #[test]
    fn opaque_release_id_shape_matches_codex() {
        assert!(validate_opaque_release_id(
            "v1:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        ));
        assert!(!validate_opaque_release_id("v1:zzz"));
        assert!(!validate_opaque_release_id(
            "https://static.qoder.com.cn/qoder-work-cn/releases/latest/QoderWorkCN-arm64.dmg"
        ));
    }

    #[test]
    fn cli_mapping_never_includes_gemini_hermes_or_openclaw() {
        assert!(super::cli::tooling_id_for(AgentCatalogId::Codex).is_none());
        assert_eq!(
            super::cli::tooling_id_for(AgentCatalogId::ClaudeCode),
            Some("claude")
        );
        assert_eq!(
            super::cli::tooling_id_for(AgentCatalogId::GrokBuild),
            Some("grok")
        );
        assert_eq!(
            super::cli::tooling_id_for(AgentCatalogId::OpenCode),
            Some("opencode")
        );
    }

    fn start_request(
        agent_id: AgentCatalogId,
        action: AgentActionId,
        surface: Option<AgentSurface>,
    ) -> StartAgentActionRequest {
        StartAgentActionRequest {
            agent_id,
            action,
            expected_release_id: None,
            inventory_id: None,
            target_id: None,
            expected_target_revision: None,
            surface,
        }
    }

    fn test_app_state() -> AppState {
        #[cfg(target_os = "windows")]
        // AppState construction creates the production Codex service, whose log
        // root normally assumes startup already froze the Explorer-user context.
        // These tests stop before user-path I/O, so bind only the test log root;
        // do not initialize or weaken the production Windows user context.
        crate::panic_hook::init_app_config_dir(
            std::env::temp_dir()
                .join("fyagent-agent-install-tests")
                .join(".fyagent"),
        );
        let db = crate::database::Database::memory().expect("memory db");
        AppState::new(std::sync::Arc::new(db))
    }

    #[test]
    fn managed_desktop_update_projection_exposes_update_only_when_needed() {
        let installed = inventory_projection(
            InstallationInventoryState::Single,
            Some("1.0.0"),
            true,
            true,
        );
        for agent_id in [
            AgentCatalogId::QoderWork,
            AgentCatalogId::TraeWork,
            AgentCatalogId::WorkBuddy,
        ] {
            let policy = lifecycle_policy::lifecycle_policy(agent_id, AgentSurface::Desktop)
                .expect("managed desktop policy");
            let update_state = desktop_update_state(
                true,
                AgentInstallState::Installed,
                installed.single_local_version.as_deref(),
                Some("2.0.0"),
            );
            assert_eq!(update_state, AgentUpdateState::UpdateAvailable);
            assert_eq!(
                desktop_allowed_actions(policy, &installed, update_state, true),
                vec![AgentActionId::Launch]
            );
            assert_eq!(
                desktop_allowed_actions(policy, &installed, AgentUpdateState::UpToDate, true,),
                vec![AgentActionId::Launch]
            );
        }
        for agent_id in [AgentCatalogId::ClaudeCode, AgentCatalogId::OpenCode] {
            let policy = lifecycle_policy::lifecycle_policy(agent_id, AgentSurface::Desktop)
                .expect("updatable desktop policy");
            let update_state = desktop_update_state(
                true,
                AgentInstallState::Installed,
                installed.single_local_version.as_deref(),
                Some("2.0.0"),
            );
            assert_eq!(
                desktop_allowed_actions(policy, &installed, update_state, true),
                vec![AgentActionId::Update, AgentActionId::Launch]
            );
        }
    }

    #[tokio::test]
    async fn unknown_desktop_inventory_does_not_resolve_source() {
        let ambiguous = inventory_projection(
            InstallationInventoryState::Multiple,
            Some("1.0.0"),
            true,
            true,
        );
        let dto = desktop_readiness(AgentCatalogId::OpenCode, &ambiguous).await;
        assert_eq!(dto.install_state, AgentInstallState::Unknown);
        assert_eq!(dto.update_state, AgentUpdateState::Unknown);
        assert!(dto.release_id.is_none());
        assert_eq!(dto.auth_ownership, AgentAuthOwnership::ProviderOwned);
        assert!(!dto.allowed_actions.contains(&AgentActionId::Update));
        assert!(!dto.allowed_actions.contains(&AgentActionId::Install));
    }

    #[tokio::test]
    async fn managed_desktop_update_requires_a_bound_existing_target() {
        let state = test_app_state();
        for agent_id in [
            AgentCatalogId::QoderWork,
            AgentCatalogId::TraeWork,
            AgentCatalogId::WorkBuddy,
        ] {
            let result =
                start_agent_action(start_request(agent_id, AgentActionId::Update, None), &state)
                    .await;
            assert_eq!(
                result,
                Err(AgentReasonCode::ActionNotSupported),
                "{agent_id:?} must not admit FyAgent one-click update"
            );
        }
        for agent_id in [AgentCatalogId::ClaudeCode, AgentCatalogId::OpenCode] {
            let result =
                start_agent_action(start_request(agent_id, AgentActionId::Update, None), &state)
                    .await;
            assert_eq!(
                result,
                Err(AgentReasonCode::TargetSelectionRequired),
                "{agent_id:?} update must be admitted but require an inventory-bound target"
            );
        }
    }

    #[tokio::test]
    async fn claude_and_opencode_cli_requests_are_surface_not_supported() {
        let state = test_app_state();
        for agent_id in [AgentCatalogId::ClaudeCode, AgentCatalogId::OpenCode] {
            let result = start_agent_action(
                start_request(agent_id, AgentActionId::Install, Some(AgentSurface::Cli)),
                &state,
            )
            .await;
            assert_eq!(
                result,
                Err(AgentReasonCode::SurfaceNotSupported),
                "{agent_id:?} CLI must not reach a CLI executor"
            );
        }
    }

    #[tokio::test]
    async fn grok_cli_install_still_reaches_target_selection() {
        let state = test_app_state();
        let result = start_agent_action(
            start_request(
                AgentCatalogId::GrokBuild,
                AgentActionId::Install,
                Some(AgentSurface::Cli),
            ),
            &state,
        )
        .await;
        assert_eq!(result, Err(AgentReasonCode::TargetSelectionRequired));
    }

    #[tokio::test]
    async fn claude_desktop_install_without_target_is_target_selection_not_unimplemented() {
        let state = test_app_state();
        let result = start_agent_action(
            start_request(
                AgentCatalogId::ClaudeCode,
                AgentActionId::Install,
                Some(AgentSurface::Desktop),
            ),
            &state,
        )
        .await;
        assert_eq!(result, Err(AgentReasonCode::TargetSelectionRequired));
    }

    #[tokio::test]
    async fn skipped_source_update_state_is_unavailable_only_for_install_only_products() {
        let install_only = AgentLifecyclePolicy {
            surfaces: &[AgentSurface::Desktop],
            install: true,
            update: false,
            launch: true,
            managed_desktop_source: None,
        };
        assert_eq!(
            skipped_desktop_source_update_state(&install_only, AgentInstallState::Installed),
            AgentUpdateState::Unavailable
        );
        let opencode =
            lifecycle_policy::lifecycle_policy(AgentCatalogId::OpenCode, AgentSurface::Desktop)
                .unwrap();
        assert_eq!(
            skipped_desktop_source_update_state(opencode, AgentInstallState::Unknown),
            AgentUpdateState::Unknown
        );
    }

    #[test]
    fn start_request_rejects_unknown_actions_and_command_fields() {
        assert!(
            serde_json::from_value::<StartAgentActionRequest>(serde_json::json!({
                "agentId": "claude-code",
                "action": "repair"
            }))
            .is_err()
        );
        assert!(
            serde_json::from_value::<StartAgentActionRequest>(serde_json::json!({
                "agentId": "codex",
                "action": "install",
                "command": "codex install"
            }))
            .is_err()
        );
    }
}
