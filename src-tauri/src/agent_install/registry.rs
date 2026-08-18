//! Bundled first-wave install-source registry.

use serde::Deserialize;

use super::error::AgentInstallError;
use super::source::SourceLayerState;
use super::types::{AgentId, InstallMode, LayerState, LicenseScope, PackageSourceKind};

const FIRST_WAVE_JSON: &str = include_str!("registry/first_wave_v1.json");

#[derive(Debug, Deserialize)]
struct RegistryFile {
    checked_at: String,
    agents: Vec<RegistryAgent>,
}

#[derive(Debug, Deserialize)]
struct RegistryAgent {
    id: AgentId,
    official_landing_url: String,
    legal_entity: String,
    license_url: String,
    license_scope: LicenseScope,
    package_source_kind: PackageSourceKind,
    cache_allowed: bool,
    redistribution_allowed: Option<bool>,
    install_mode: InstallMode,
    evidence_url: String,
    written_permission_needed: bool,
    source_state: LayerState,
}

fn parsed_registry() -> &'static RegistryFile {
    use std::sync::OnceLock;
    static REGISTRY: OnceLock<RegistryFile> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        serde_json::from_str(FIRST_WAVE_JSON).unwrap_or_else(|error| {
            panic!("first-wave registry must parse: {error}");
        })
    })
}

pub fn first_wave_ids() -> [AgentId; 6] {
    AgentId::ALL
}

pub fn registry() -> impl Iterator<Item = SourceLayerState> {
    parsed_registry()
        .agents
        .iter()
        .map(|agent| source_layer_for_record(agent, &parsed_registry().checked_at))
}

pub fn source_layer_for(agent_id: AgentId) -> Result<SourceLayerState, AgentInstallError> {
    let file = parsed_registry();
    let agent = file
        .agents
        .iter()
        .find(|agent| agent.id == agent_id)
        .ok_or_else(AgentInstallError::unknown_agent)?;
    Ok(source_layer_for_record(agent, &file.checked_at))
}

fn source_layer_for_record(agent: &RegistryAgent, checked_at: &str) -> SourceLayerState {
    SourceLayerState {
        agent_id: agent.id,
        source_state: agent.source_state,
        official_landing_url: Some(agent.official_landing_url.clone()),
        legal_entity: Some(agent.legal_entity.clone()),
        license_url: Some(agent.license_url.clone()),
        license_scope: agent.license_scope,
        package_source_kind: agent.package_source_kind,
        cache_allowed: agent.cache_allowed,
        redistribution_allowed: agent.redistribution_allowed,
        install_mode: agent.install_mode,
        evidence_url: Some(agent.evidence_url.clone()),
        checked_at: checked_at.to_owned(),
        written_permission_needed: agent.written_permission_needed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_install::gate::package_install_allowed;
    use crate::agent_install::integrity::IntegrityLayerState;
    use crate::agent_install::plan::PlanLayerState;
    use crate::agent_install::preflight::PreflightLayerState;
    use crate::agent_install::types::LayerState;

    #[test]
    fn registry_contains_exactly_six_first_wave_ids() {
        let ids: Vec<_> = first_wave_ids().into_iter().map(AgentId::as_str).collect();
        assert_eq!(
            ids,
            vec![
                "qoderwork-cn",
                "dingtalk-wukong",
                "workbuddy",
                "trae-work",
                "codex-cli",
                "claude-code",
            ]
        );
        assert_eq!(parsed_registry().agents.len(), 6);
    }

    #[test]
    fn codex_cli_is_only_redistribution_allowed_true() {
        let allowed: Vec<_> = parsed_registry()
            .agents
            .iter()
            .filter(|agent| agent.redistribution_allowed == Some(true))
            .map(|agent| agent.id)
            .collect();
        assert_eq!(allowed, vec![AgentId::CodexCli]);
    }

    #[test]
    fn unconfirmed_or_false_redistribution_fails_hosted_install() {
        let blocked = source_layer_for(AgentId::Workbuddy).expect("workbuddy exists");
        assert_eq!(blocked.distribution_gate(), LayerState::Fail);
        let mut unconfirmed = blocked.clone();
        unconfirmed.redistribution_allowed = None;
        assert_eq!(unconfirmed.distribution_gate(), LayerState::Fail);
    }

    #[test]
    fn official_guide_remains_available_when_redistribution_blocked() {
        let source = source_layer_for(AgentId::TraeWork).expect("trae exists");
        assert!(source.guide_allowed());
        assert!(source.package_install_blocked());
    }

    #[test]
    fn unknown_source_state_blocks_package_install() {
        let mut source = source_layer_for(AgentId::CodexCli).expect("codex exists");
        source.source_state = LayerState::Unknown;
        source.install_mode = InstallMode::PackageManager;
        source.redistribution_allowed = Some(true);
        assert!(source.package_install_blocked());
    }

    #[test]
    fn renderer_dto_omits_raw_download_url_for_blocked_agents() {
        for id in [
            AgentId::QoderworkCn,
            AgentId::DingtalkWukong,
            AgentId::Workbuddy,
            AgentId::TraeWork,
        ] {
            let source = source_layer_for(id).expect("guide agent exists");
            assert_eq!(source.renderer_download_url(), None);
        }
    }

    #[test]
    fn registry_rejects_unknown_agent_id() {
        let error = AgentId::parse("not-an-agent").expect_err("unknown id");
        assert_eq!(
            error.code,
            crate::agent_install::AgentInstallErrorCode::AgentUnknown
        );
    }

    #[test]
    fn four_layers_unknown_do_not_merge_into_installable() {
        let source = source_layer_for(AgentId::ClaudeCode).expect("claude exists");
        let allowed = package_install_allowed(
            &source,
            &IntegrityLayerState::unknown("2026-08-18T00:00:00Z"),
            &PreflightLayerState::unknown("2026-08-18T00:00:00Z"),
            &PlanLayerState::absent("2026-08-18T00:00:00Z"),
        );
        assert!(!allowed);
    }
}
