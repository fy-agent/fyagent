//! Read-only installation readiness for the canonical Agent Catalog.
//!
//! This module deliberately owns no registry, installer, executor, snapshot,
//! package probe, or filesystem/network access. The catalog's closed
//! `AgentCatalogId` is the only accepted selector.

use serde::Serialize;
#[cfg(test)]
use sha2::{Digest, Sha256};

use crate::services::external_agents::AgentCatalogId;

pub const AGENT_INSTALL_READINESS_CONTRACT_VERSION: u16 = 1;
pub const AGENT_INSTALL_READINESS_REVIEWED_AT: &str = "2026-08-24";
#[cfg(test)]
const PREFLIGHT_TTL_SECONDS: i64 = 15 * 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadinessLayerState {
    Ok,
    Warn,
    Fail,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AutomationState {
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AutomationReasonCode {
    OfficialGuideOnly,
    ExecutorNotImplemented,
    ManagedByCodexDesktop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallMode {
    OfficialGuide,
    ManagedPackage,
    NativeVerified,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LicenseScope {
    Unconfirmed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DistributionState {
    Unconfirmed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceReasonCode {
    SourceReviewNotRefreshed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IntegritySummaryCode {
    IntegrityNotChecked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PreflightReasonCode {
    PreflightNotRun,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PreflightCheckCode {
    OsCompatibility,
    ArchitectureCompatibility,
    Requirements,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SanitizedPreflightCheck {
    pub code: PreflightCheckCode,
    pub state: ReadinessLayerState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanReasonCode {
    PlanNotCreated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AutomationReadiness {
    pub state: AutomationState,
    pub reason_code: AutomationReasonCode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceReadiness {
    pub state: ReadinessLayerState,
    pub reason_code: SourceReasonCode,
    pub install_mode: InstallMode,
    pub license_scope: LicenseScope,
    pub distribution_state: DistributionState,
    pub checked_at: Option<()>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegrityReadiness {
    pub state: ReadinessLayerState,
    pub summary_code: IntegritySummaryCode,
    pub checked_at: Option<()>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreflightReadiness {
    pub state: ReadinessLayerState,
    pub reason_code: PreflightReasonCode,
    pub checks: Vec<SanitizedPreflightCheck>,
    pub checked_at: Option<()>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanReadiness {
    pub state: ReadinessLayerState,
    pub reason_code: PlanReasonCode,
    /// These fields are intentionally typed as null-only. Readiness never
    /// creates, caches, refreshes, or reconfirms an installation snapshot.
    pub snapshot_id: Option<()>,
    pub snapshot_stale: Option<()>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentInstallReadinessDto {
    pub contract_version: u16,
    pub agent_id: AgentCatalogId,
    pub reviewed_at: &'static str,
    pub automation: AutomationReadiness,
    pub source: SourceReadiness,
    pub integrity: IntegrityReadiness,
    pub preflight: PreflightReadiness,
    pub plan: PlanReadiness,
}

/// Rolls up independently observed factors without upgrading incomplete
/// evidence. Failure dominates; any unknown input keeps the layer unknown.
#[cfg(test)]
fn rollup_layer_states(states: &[ReadinessLayerState]) -> ReadinessLayerState {
    if states.contains(&ReadinessLayerState::Fail) {
        return ReadinessLayerState::Fail;
    }
    if states.is_empty() || states.contains(&ReadinessLayerState::Unknown) {
        return ReadinessLayerState::Unknown;
    }
    if states.contains(&ReadinessLayerState::Warn) {
        return ReadinessLayerState::Warn;
    }
    ReadinessLayerState::Ok
}

#[cfg(test)]
fn preflight_is_fresh(checked_at_epoch: Option<i64>, now_epoch: i64) -> bool {
    checked_at_epoch.is_some_and(|checked_at| {
        now_epoch >= checked_at && now_epoch - checked_at < PREFLIGHT_TTL_SECONDS
    })
}

#[cfg(test)]
#[derive(Serialize)]
struct PlanHashInput {
    agent_id: AgentCatalogId,
    automation_reason: AutomationReasonCode,
    source_state: ReadinessLayerState,
    install_mode: InstallMode,
    integrity_state: ReadinessLayerState,
    integrity_summary: IntegritySummaryCode,
    preflight_state: ReadinessLayerState,
}

#[cfg(test)]
fn plan_hash_input(readiness: &AgentInstallReadinessDto) -> PlanHashInput {
    PlanHashInput {
        agent_id: readiness.agent_id,
        automation_reason: readiness.automation.reason_code,
        source_state: readiness.source.state,
        install_mode: readiness.source.install_mode,
        integrity_state: readiness.integrity.state,
        integrity_summary: readiness.integrity.summary_code,
        preflight_state: readiness.preflight.state,
    }
}

/// Internal drift fingerprint for pure contract verification. It is never
/// serialized into readiness and never creates a plan snapshot.
#[cfg(test)]
fn stable_plan_hash(readiness: &AgentInstallReadinessDto) -> String {
    let encoded = serde_json::to_vec(&plan_hash_input(readiness))
        .expect("fixed readiness plan input must serialize");
    format!("{:x}", Sha256::digest(encoded))
}

pub fn readiness_for(agent_id: AgentCatalogId) -> AgentInstallReadinessDto {
    let (automation_reason, install_mode) = match agent_id {
        AgentCatalogId::Codex => (
            AutomationReasonCode::ManagedByCodexDesktop,
            InstallMode::ManagedPackage,
        ),
        AgentCatalogId::QoderWork
        | AgentCatalogId::TraeWork
        | AgentCatalogId::WorkBuddy
        | AgentCatalogId::GrokBuild => (
            AutomationReasonCode::OfficialGuideOnly,
            InstallMode::OfficialGuide,
        ),
        AgentCatalogId::ClaudeCode | AgentCatalogId::OpenCode => (
            AutomationReasonCode::ExecutorNotImplemented,
            InstallMode::Unsupported,
        ),
    };

    AgentInstallReadinessDto {
        contract_version: AGENT_INSTALL_READINESS_CONTRACT_VERSION,
        agent_id,
        reviewed_at: AGENT_INSTALL_READINESS_REVIEWED_AT,
        automation: AutomationReadiness {
            state: AutomationState::Unavailable,
            reason_code: automation_reason,
        },
        source: SourceReadiness {
            state: ReadinessLayerState::Unknown,
            reason_code: SourceReasonCode::SourceReviewNotRefreshed,
            install_mode,
            license_scope: LicenseScope::Unconfirmed,
            distribution_state: DistributionState::Unconfirmed,
            checked_at: None,
        },
        integrity: IntegrityReadiness {
            state: ReadinessLayerState::Unknown,
            summary_code: IntegritySummaryCode::IntegrityNotChecked,
            checked_at: None,
        },
        preflight: PreflightReadiness {
            state: ReadinessLayerState::Unknown,
            reason_code: PreflightReasonCode::PreflightNotRun,
            checks: Vec::new(),
            checked_at: None,
        },
        plan: PlanReadiness {
            state: ReadinessLayerState::Unknown,
            reason_code: PlanReasonCode::PlanNotCreated,
            snapshot_id: None,
            snapshot_stale: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

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

    fn sorted_keys(value: &Value) -> Vec<&str> {
        let mut keys = value
            .as_object()
            .expect("wire object")
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>();
        keys.sort_unstable();
        keys
    }

    #[test]
    fn readiness_accepts_exactly_the_canonical_seven_ids() {
        let ids = IDS
            .into_iter()
            .map(|id| serde_json::to_value(readiness_for(id)).unwrap()["agentId"].clone())
            .collect::<Vec<_>>();
        assert_eq!(
            ids,
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

        for invalid in [
            r#""qoderwork-cn""#,
            r#""dingtalk-wukong""#,
            r#""codex-cli""#,
            r#""claude""#,
            r#""unknown""#,
        ] {
            assert!(serde_json::from_str::<AgentCatalogId>(invalid).is_err());
        }
    }

    #[test]
    fn fail_dominates_and_unknown_never_becomes_ok() {
        assert_eq!(
            rollup_layer_states(&[ReadinessLayerState::Ok, ReadinessLayerState::Fail]),
            ReadinessLayerState::Fail
        );
        assert_eq!(
            rollup_layer_states(&[ReadinessLayerState::Ok, ReadinessLayerState::Unknown]),
            ReadinessLayerState::Unknown
        );
        assert_eq!(
            rollup_layer_states(&[ReadinessLayerState::Unknown; 3]),
            ReadinessLayerState::Unknown
        );
        assert_eq!(
            rollup_layer_states(&[ReadinessLayerState::Ok, ReadinessLayerState::Warn]),
            ReadinessLayerState::Warn
        );
    }

    #[test]
    fn readiness_wire_has_exact_keys_and_no_sensitive_fields() {
        let value = serde_json::to_value(readiness_for(AgentCatalogId::QoderWork)).unwrap();
        assert_eq!(
            sorted_keys(&value),
            [
                "agentId",
                "automation",
                "contractVersion",
                "integrity",
                "plan",
                "preflight",
                "reviewedAt",
                "source",
            ]
        );
        assert_eq!(sorted_keys(&value["automation"]), ["reasonCode", "state"]);
        assert_eq!(
            sorted_keys(&value["source"]),
            [
                "checkedAt",
                "distributionState",
                "installMode",
                "licenseScope",
                "reasonCode",
                "state",
            ]
        );
        assert_eq!(
            sorted_keys(&value["integrity"]),
            ["checkedAt", "state", "summaryCode"]
        );
        assert_eq!(
            sorted_keys(&value["preflight"]),
            ["checkedAt", "checks", "reasonCode", "state"]
        );
        assert_eq!(
            sorted_keys(&value["plan"]),
            ["reasonCode", "snapshotId", "snapshotStale", "state",]
        );

        let serialized = value.to_string().to_ascii_lowercase();
        for prohibited in [
            "url",
            "path",
            "hash",
            "script",
            "secret",
            "package",
            "signer",
            "fingerprint",
            "token",
        ] {
            assert!(!serialized.contains(prohibited), "found {prohibited}");
        }
    }

    #[test]
    fn readiness_never_creates_or_refreshes_a_plan_snapshot() {
        for agent_id in IDS {
            let dto = readiness_for(agent_id);
            assert_eq!(dto.plan.state, ReadinessLayerState::Unknown);
            assert_eq!(dto.plan.reason_code, PlanReasonCode::PlanNotCreated);
            assert_eq!(dto.plan.snapshot_id, None);
            assert_eq!(dto.plan.snapshot_stale, None);
        }
    }

    #[test]
    fn integrity_rollup_preserves_failure_and_all_unknown() {
        assert_eq!(
            rollup_layer_states(&[
                ReadinessLayerState::Ok,
                ReadinessLayerState::Fail,
                ReadinessLayerState::Unknown,
            ]),
            ReadinessLayerState::Fail
        );
        assert_eq!(
            rollup_layer_states(&[ReadinessLayerState::Unknown; 3]),
            ReadinessLayerState::Unknown
        );
    }

    #[test]
    fn preflight_ttl_expires_without_upgrading_missing_or_future_checks() {
        let now = 10_000;
        assert!(preflight_is_fresh(
            Some(now - PREFLIGHT_TTL_SECONDS + 1),
            now
        ));
        assert!(!preflight_is_fresh(Some(now - PREFLIGHT_TTL_SECONDS), now));
        assert!(!preflight_is_fresh(None, now));
        assert!(!preflight_is_fresh(Some(now + 1), now));
    }

    #[test]
    fn plan_hash_is_stable_and_critical_field_drift_invalidates_it() {
        let original = readiness_for(AgentCatalogId::OpenCode);
        assert_eq!(stable_plan_hash(&original), stable_plan_hash(&original));

        let mut drifted = original.clone();
        drifted.integrity.state = ReadinessLayerState::Fail;
        assert_ne!(stable_plan_hash(&original), stable_plan_hash(&drifted));
        assert_eq!(original.plan.snapshot_id, None);
        assert_eq!(original.plan.snapshot_stale, None);
    }

    #[test]
    fn generic_automation_is_unavailable_and_codex_stays_on_its_existing_owner() {
        for agent_id in IDS {
            let dto = readiness_for(agent_id);
            assert_eq!(dto.automation.state, AutomationState::Unavailable);
            if agent_id == AgentCatalogId::Codex {
                assert_eq!(
                    dto.automation.reason_code,
                    AutomationReasonCode::ManagedByCodexDesktop
                );
                assert_eq!(dto.source.install_mode, InstallMode::ManagedPackage);
            } else if matches!(
                agent_id,
                AgentCatalogId::QoderWork
                    | AgentCatalogId::TraeWork
                    | AgentCatalogId::WorkBuddy
                    | AgentCatalogId::GrokBuild
            ) {
                assert_eq!(
                    dto.automation.reason_code,
                    AutomationReasonCode::OfficialGuideOnly
                );
                assert_eq!(dto.source.install_mode, InstallMode::OfficialGuide);
            } else {
                assert_eq!(
                    dto.automation.reason_code,
                    AutomationReasonCode::ExecutorNotImplemented
                );
                assert_eq!(dto.source.install_mode, InstallMode::Unsupported);
            }
        }
    }
}
