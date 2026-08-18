//! Immutable install plan (#28).

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::error::{AgentInstallError, AgentInstallErrorCode};
use super::types::{InstallContract, InstallMode, LayerState};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PlanSummary {
    pub agent_id: String,
    pub version: String,
    pub source_kind: String,
    pub package_hash: String,
    pub actions: Vec<String>,
    pub signer_id: String,
    pub revocation_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PlanLayerState {
    pub plan_snapshot_id: Option<String>,
    pub plan_hash: Option<String>,
    pub plan_summary: Option<PlanSummary>,
    pub snapshot_stale: bool,
    pub drift_reasons: Vec<String>,
    pub refreshed_at: String,
}

impl PlanLayerState {
    pub fn absent(refreshed_at: impl Into<String>) -> Self {
        Self {
            plan_snapshot_id: None,
            plan_hash: None,
            plan_summary: None,
            snapshot_stale: true,
            drift_reasons: vec!["plan_absent".to_owned()],
            refreshed_at: refreshed_at.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StartAgentInstallRequest {
    pub snapshot_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanSnapshot {
    pub snapshot_id: String,
    pub plan_hash: String,
    pub summary: PlanSummary,
    pub refreshed_at: String,
}

impl PlanSnapshot {
    pub fn as_layer(&self) -> PlanLayerState {
        PlanLayerState {
            plan_snapshot_id: Some(self.snapshot_id.clone()),
            plan_hash: Some(self.plan_hash.clone()),
            plan_summary: Some(self.summary.clone()),
            snapshot_stale: false,
            drift_reasons: Vec::new(),
            refreshed_at: self.refreshed_at.clone(),
        }
    }
}

pub fn summary_from_contract(contract: &InstallContract) -> PlanSummary {
    let action = match contract.catalog.install_mode {
        InstallMode::OfficialGuide => "OpenOfficialGuide",
        InstallMode::PackageManager => "PackageManagerInstall",
        InstallMode::NativeVerified => "NativeVerifiedInstall",
    };
    PlanSummary {
        agent_id: contract.agent_id.as_str().to_owned(),
        version: "registry-v1".to_owned(),
        source_kind: format!("{:?}", contract.catalog.package_source_kind),
        package_hash: contract
            .package
            .hash
            .value
            .clone()
            .unwrap_or_else(|| "unknown".to_owned()),
        actions: vec![action.to_owned()],
        signer_id: contract
            .package
            .observed_signer
            .clone()
            .unwrap_or_else(|| "unknown".to_owned()),
        revocation_status: format!("{:?}", contract.package.revocation.state),
    }
}

pub fn plan_hash(summary: &PlanSummary) -> Result<String, AgentInstallError> {
    let encoded = serde_json::to_vec(summary)
        .map_err(|_| AgentInstallError::new(AgentInstallErrorCode::InternalError))?;
    Ok(Sha256::digest(encoded)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

pub fn create_snapshot(contract: &InstallContract, now: &str) -> PlanSnapshot {
    let summary = summary_from_contract(contract);
    let hash = plan_hash(&summary).unwrap_or_else(|_| "invalid".to_owned());
    PlanSnapshot {
        snapshot_id: format!("snap-{hash}"),
        plan_hash: hash,
        summary,
        refreshed_at: now.to_owned(),
    }
}

pub fn drift_reasons(expected: &PlanSummary, observed: &PlanSummary) -> Vec<String> {
    let mut reasons = Vec::new();
    if expected.source_kind != observed.source_kind {
        reasons.push("source_kind".to_owned());
    }
    if expected.version != observed.version {
        reasons.push("version".to_owned());
    }
    if expected.package_hash != observed.package_hash {
        reasons.push("package_hash".to_owned());
    }
    if expected.actions != observed.actions {
        reasons.push("actions".to_owned());
    }
    if expected.signer_id != observed.signer_id {
        reasons.push("signer_id".to_owned());
    }
    if expected.revocation_status != observed.revocation_status {
        reasons.push("revocation_status".to_owned());
    }
    reasons
}

pub fn reconfirm_snapshot(current: &PlanSnapshot, now: &str) -> PlanSnapshot {
    let hash = plan_hash(&current.summary).unwrap_or_else(|_| current.plan_hash.clone());
    PlanSnapshot {
        snapshot_id: format!("snap-{hash}-{now}"),
        plan_hash: hash,
        summary: current.summary.clone(),
        refreshed_at: now.to_owned(),
    }
}

pub fn start_install(
    snapshot: &PlanSnapshot,
    request: &StartAgentInstallRequest,
) -> Result<PlanLayerState, AgentInstallError> {
    if snapshot.snapshot_id != request.snapshot_id {
        return Err(AgentInstallError::snapshot_stale());
    }
    let recomputed = plan_hash(&snapshot.summary)?;
    if recomputed != snapshot.plan_hash {
        return Err(AgentInstallError::snapshot_stale());
    }
    Ok(snapshot.as_layer())
}

pub fn warn_does_not_block(integrity: LayerState, preflight: LayerState, stale: bool) -> bool {
    matches!(integrity, LayerState::Ok | LayerState::Warn)
        && matches!(preflight, LayerState::Ok | LayerState::Warn)
        && !stale
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_install::contract::default_contract;
    use crate::agent_install::types::AgentId;

    fn sample_summary(hash: &str, signer: &str) -> PlanSummary {
        PlanSummary {
            agent_id: "codex-cli".to_owned(),
            version: "registry-v1".to_owned(),
            source_kind: "PackageManager".to_owned(),
            package_hash: hash.to_owned(),
            actions: vec!["PackageManagerInstall".to_owned()],
            signer_id: signer.to_owned(),
            revocation_status: "Unknown".to_owned(),
        }
    }

    #[test]
    fn start_install_rejects_unknown_fields() {
        let parsed = serde_json::from_str::<StartAgentInstallRequest>(
            r#"{"snapshotId":"snap-1","url":"https://evil.example"}"#,
        );
        assert!(parsed.is_err());
    }

    #[test]
    fn start_install_accepts_only_snapshot_id() {
        let request: StartAgentInstallRequest =
            serde_json::from_str(r#"{"snapshotId":"snap-1"}"#).expect("snapshot only");
        assert_eq!(request.snapshot_id, "snap-1");
    }

    #[test]
    fn hash_change_marks_snapshot_stale() {
        let left = sample_summary("aaa", "unknown");
        let right = sample_summary("bbb", "unknown");
        assert_eq!(
            drift_reasons(&left, &right),
            vec!["package_hash".to_owned()]
        );
    }

    #[test]
    fn signer_change_requires_reconfirm() {
        let left = sample_summary("aaa", "old");
        let right = sample_summary("aaa", "new");
        assert_eq!(drift_reasons(&left, &right), vec!["signer_id".to_owned()]);
    }

    #[test]
    fn revocation_change_requires_reconfirm() {
        let mut left = sample_summary("aaa", "unknown");
        let mut right = left.clone();
        right.revocation_status = "Fail".to_owned();
        left.revocation_status = "Unknown".to_owned();
        assert_eq!(
            drift_reasons(&left, &right),
            vec!["revocation_status".to_owned()]
        );
    }

    #[test]
    fn action_order_change_is_drift() {
        let mut left = sample_summary("aaa", "unknown");
        let mut right = left.clone();
        left.actions = vec!["A".to_owned(), "B".to_owned()];
        right.actions = vec!["B".to_owned(), "A".to_owned()];
        assert_eq!(drift_reasons(&left, &right), vec!["actions".to_owned()]);
    }

    #[test]
    fn stale_snapshot_cannot_start_install() {
        let contract = default_contract(AgentId::CodexCli, "t0").expect("contract");
        let snapshot = create_snapshot(&contract, "t0");
        let request = StartAgentInstallRequest {
            snapshot_id: "other".to_owned(),
        };
        let error = start_install(&snapshot, &request).expect_err("stale");
        assert_eq!(error.code, AgentInstallErrorCode::SnapshotStale);
    }

    #[test]
    fn reconfirm_issues_new_snapshot_id() {
        let contract = default_contract(AgentId::CodexCli, "t0").expect("contract");
        let first = create_snapshot(&contract, "t0");
        let second = reconfirm_snapshot(&first, "t1");
        assert_ne!(first.snapshot_id, second.snapshot_id);
    }

    #[test]
    fn warn_does_not_block_when_others_ok() {
        assert!(warn_does_not_block(LayerState::Warn, LayerState::Ok, false));
        assert!(!warn_does_not_block(
            LayerState::Unknown,
            LayerState::Ok,
            false
        ));
        assert!(!warn_does_not_block(
            LayerState::Ok,
            LayerState::Fail,
            false
        ));
    }
}
