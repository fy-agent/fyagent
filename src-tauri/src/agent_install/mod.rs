//! Agent install/action façade. Canonical catalog IDs only; renderer input is
//! `agentId + action` plus an optional opaque backend-generated release ID.

mod auth_actions;
mod cli;
mod desktop;
mod fetch;
mod jobs;
mod sources;
mod types;

use std::sync::atomic::AtomicBool;
use std::sync::Arc;

pub use jobs::AgentActionJobStore;
pub use types::{
    validate_opaque_release_id, AgentActionErrorDto, AgentActionId, AgentActionJobSnapshot,
    AgentActionJobStage, AgentActionResult, AgentAuthOwnership, AgentAuthState,
    AgentInstallReadinessDto, AgentInstallState, AgentReasonCode, AgentSourceKind,
    AgentUpdateState, StartAgentActionRequest, AGENT_ACTION_CONTRACT_VERSION,
    AGENT_INSTALL_READINESS_CONTRACT_VERSION, AGENT_INSTALL_READINESS_REVIEWED_AT,
};

use auth_actions::{observe_auth_state, start_auth_action};
use cli::{observe_cli, run_cli_lifecycle};
use desktop::{
    download_resolved_source, finish_macos_dmg_install, install_state_from_observation,
    launch_if_present, observe_desktop, readiness_source_codes, resolve_desktop_source,
    source_reason, windows_exe_unavailable,
};
use sources::PackageFormat;

use crate::codex_desktop::types::LocalInstallStatus;
use crate::services::external_agents::AgentCatalogId;
use crate::store::AppState;

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
    match agent_id {
        AgentCatalogId::Codex => codex_readiness(state).await,
        AgentCatalogId::ClaudeCode | AgentCatalogId::GrokBuild | AgentCatalogId::OpenCode => {
            cli_readiness(agent_id).await
        }
        AgentCatalogId::QoderWork | AgentCatalogId::TraeWork | AgentCatalogId::WorkBuddy => {
            desktop_readiness(agent_id).await
        }
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
        if observation.as_ref().is_some_and(|value| value.detected) {
            match agent_id {
                AgentCatalogId::OpenCode => {
                    allowed_actions.push(AgentActionId::AuthConnectProvider)
                }
                AgentCatalogId::ClaudeCode | AgentCatalogId::GrokBuild => {
                    allowed_actions.push(AgentActionId::AuthLogin);
                    allowed_actions.push(AgentActionId::AuthLogout);
                }
                _ => {}
            }
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
        update_state,
        release_id: None,
        local_version,
        remote_version,
        auth_ownership,
        auth_state,
        source_kind: AgentSourceKind::CliTooling,
        allowed_actions,
        reason_codes,
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

async fn desktop_readiness(agent_id: AgentCatalogId) -> AgentInstallReadinessDto {
    let observed = observe_desktop(agent_id);
    let source = resolve_desktop_source(agent_id).await;
    let mut reason_codes = Vec::new();
    let mut allowed_actions = Vec::new();
    let (release_id, remote_version, source_ok, dmg_installable) = match &source {
        Ok(resolved) => {
            let dmg = resolved.format == PackageFormat::Dmg
                && resolved.platform == sources::AgentPlatform::Macos
                && !windows_exe_unavailable(resolved);
            (
                Some(resolved.release_id.clone()),
                resolved.display_version.clone(),
                true,
                dmg,
            )
        }
        Err(error) => {
            reason_codes.extend(readiness_source_codes(*error));
            (None, None, false, false)
        }
    };
    if source.as_ref().is_ok_and(windows_exe_unavailable) {
        reason_codes.push(AgentReasonCode::InteractiveUserUnavailable);
    }
    let install_state = install_state_from_observation(&observed);
    let update_state = if !source_ok {
        AgentUpdateState::Unavailable
    } else if remote_version.is_none() {
        AgentUpdateState::LatestUnknown
    } else if observed.installed {
        match (observed.local_version.as_deref(), remote_version.as_deref()) {
            (Some(local), Some(remote)) if desktop_versions_equivalent(local, remote) => {
                AgentUpdateState::UpToDate
            }
            _ => AgentUpdateState::UpdateAvailable,
        }
    } else {
        AgentUpdateState::UpdateAvailable
    };
    if dmg_installable {
        if install_state != AgentInstallState::Installed {
            allowed_actions.push(AgentActionId::Install);
        } else if update_state != AgentUpdateState::UpToDate {
            allowed_actions.push(AgentActionId::Update);
        }
    }
    if observed.installed {
        allowed_actions.push(AgentActionId::Launch);
        allowed_actions.push(AgentActionId::AuthLogin);
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
        update_state,
        release_id,
        local_version: observed.local_version,
        remote_version,
        auth_ownership: AgentAuthOwnership::AgentOwned,
        auth_state,
        source_kind: AgentSourceKind::ManagedDesktop,
        allowed_actions,
        reason_codes,
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
    }
}

pub async fn start_agent_action(
    request: StartAgentActionRequest,
    state: &AppState,
) -> Result<AgentActionResult, AgentReasonCode> {
    if let Some(release_id) = request.expected_release_id.as_deref() {
        if !validate_opaque_release_id(release_id) {
            return Err(AgentReasonCode::RefreshRequired);
        }
    }
    match (request.agent_id, request.action) {
        (AgentCatalogId::Codex, AgentActionId::Install | AgentActionId::Update) => {
            Err(AgentReasonCode::ManagedByCodexDesktop)
        }
        (AgentCatalogId::Codex, AgentActionId::Launch) => {
            state
                .codex_desktop_service
                .launch()
                .await
                .map_err(|_| AgentReasonCode::InteractiveUserUnavailable)?;
            Ok(immediate_result(
                request.agent_id,
                request.action,
                AgentActionJobStage::Succeeded,
                None,
            ))
        }
        (
            _,
            AgentActionId::AuthLogin
            | AgentActionId::AuthLogout
            | AgentActionId::AuthConnectProvider,
        ) => {
            start_desktop_or_cli_auth(request.agent_id, request.action)?;
            Ok(immediate_result(
                request.agent_id,
                request.action,
                AgentActionJobStage::Succeeded,
                None,
            ))
        }
        (
            AgentCatalogId::ClaudeCode | AgentCatalogId::GrokBuild | AgentCatalogId::OpenCode,
            AgentActionId::Install | AgentActionId::Update,
        ) => {
            run_cli_lifecycle(request.agent_id, request.action).await?;
            Ok(immediate_result(
                request.agent_id,
                request.action,
                AgentActionJobStage::Succeeded,
                None,
            ))
        }
        (
            AgentCatalogId::QoderWork | AgentCatalogId::TraeWork | AgentCatalogId::WorkBuddy,
            AgentActionId::Install | AgentActionId::Update,
        ) => start_desktop_job(request, state).await,
        (
            AgentCatalogId::QoderWork | AgentCatalogId::TraeWork | AgentCatalogId::WorkBuddy,
            AgentActionId::Launch,
        ) => {
            launch_if_present(request.agent_id)?;
            Ok(immediate_result(
                request.agent_id,
                request.action,
                AgentActionJobStage::Succeeded,
                None,
            ))
        }
        _ => Err(AgentReasonCode::ExecutorNotImplemented),
    }
}

fn start_desktop_or_cli_auth(
    agent_id: AgentCatalogId,
    action: AgentActionId,
) -> Result<(), AgentReasonCode> {
    match (agent_id, action) {
        (
            AgentCatalogId::QoderWork | AgentCatalogId::TraeWork | AgentCatalogId::WorkBuddy,
            AgentActionId::AuthLogin | AgentActionId::Launch,
        ) => launch_if_present(agent_id),
        _ => start_auth_action(agent_id, action),
    }
}

async fn start_desktop_job(
    request: StartAgentActionRequest,
    state: &AppState,
) -> Result<AgentActionResult, AgentReasonCode> {
    let source = resolve_desktop_source(request.agent_id)
        .await
        .map_err(source_reason)?;
    if windows_exe_unavailable(&source) {
        return Err(AgentReasonCode::InteractiveUserUnavailable);
    }
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
    let (snapshot, cancel) = state
        .agent_action_jobs
        .start(request.agent_id, request.action)?;
    let job_id = snapshot.job_id.clone();
    let jobs = Arc::clone(&state.agent_action_jobs);
    tokio::spawn(async move {
        run_desktop_install_job(jobs, job_id, source, cancel).await;
    });
    Ok(AgentActionResult {
        contract_version: AGENT_ACTION_CONTRACT_VERSION,
        agent_id: request.agent_id,
        action: request.action,
        job_id: Some(snapshot.job_id),
        stage: snapshot.stage,
        reason_code: None,
    })
}

async fn run_desktop_install_job(
    jobs: Arc<AgentActionJobStore>,
    job_id: String,
    source: sources::ResolvedDesktopSource,
    cancel: Arc<AtomicBool>,
) {
    if jobs.is_cancelled(&cancel) {
        let _ = jobs.transition(
            &job_id,
            AgentActionJobStage::Cancelled,
            Some(AgentReasonCode::Cancelled),
        );
        return;
    }
    let _ = jobs.transition(&job_id, AgentActionJobStage::Downloading, None);
    if jobs.is_cancelled(&cancel) {
        let _ = jobs.transition(
            &job_id,
            AgentActionJobStage::Cancelled,
            Some(AgentReasonCode::Cancelled),
        );
        return;
    }
    let bytes = match download_resolved_source(&source, cancel.as_ref()).await {
        Ok(bytes) => bytes,
        Err(AgentReasonCode::Cancelled) => {
            let _ = jobs.transition(
                &job_id,
                AgentActionJobStage::Cancelled,
                Some(AgentReasonCode::Cancelled),
            );
            return;
        }
        Err(reason) => {
            let _ = jobs.transition(&job_id, AgentActionJobStage::Failed, Some(reason));
            return;
        }
    };
    if jobs.is_cancelled(&cancel) {
        let _ = jobs.transition(
            &job_id,
            AgentActionJobStage::Cancelled,
            Some(AgentReasonCode::Cancelled),
        );
        return;
    }
    let _ = jobs.transition(&job_id, AgentActionJobStage::Installing, None);
    match finish_macos_dmg_install(source.product, &bytes) {
        Ok(()) => {
            let _ = jobs.transition(&job_id, AgentActionJobStage::VerifyingInstallation, None);
            if source.product == AgentCatalogId::TraeWork
                && !observe_desktop(source.product).installed
            {
                let _ = jobs.transition(
                    &job_id,
                    AgentActionJobStage::Failed,
                    Some(AgentReasonCode::InstalledNotRunnable),
                );
            } else {
                let _ = jobs.transition(&job_id, AgentActionJobStage::Succeeded, None);
            }
        }
        Err(AgentReasonCode::Cancelled) => {
            let _ = jobs.transition(
                &job_id,
                AgentActionJobStage::Cancelled,
                Some(AgentReasonCode::Cancelled),
            );
        }
        Err(reason) => {
            let _ = jobs.transition(&job_id, AgentActionJobStage::Failed, Some(reason));
        }
    }
}

fn immediate_result(
    agent_id: AgentCatalogId,
    action: AgentActionId,
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
            "localVersion",
            "reasonCodes",
            "releaseId",
            "remoteVersion",
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
        assert_eq!(value["contractVersion"], 2);
        assert!(value.get("automation").is_none());
        assert!(value.get("plan").is_none());
        assert!(value.get("integrity").is_none());
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
