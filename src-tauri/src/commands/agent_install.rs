//! Thin IPC for the six-agent install contract.

use tauri::{AppHandle, State};

use crate::agent_install::{
    plan::{PlanLayerState, StartAgentInstallRequest},
    preflight::PreflightLayerState,
    probe::HealthProbeResult,
    source::SourceLayerState,
    types::InstallContract,
    AgentInstallErrorDto,
};
use crate::store::AppState;

#[tauri::command]
pub async fn agent_install_list_catalog(
    state: State<'_, AppState>,
) -> Result<Vec<SourceLayerState>, AgentInstallErrorDto> {
    Ok(state.agent_install_service.list_catalog())
}

#[tauri::command]
pub async fn agent_install_get_contract(
    agent_id: String,
    state: State<'_, AppState>,
) -> Result<InstallContract, AgentInstallErrorDto> {
    state
        .agent_install_service
        .get_contract(&agent_id)
        .map_err(Into::into)
}

#[tauri::command]
pub async fn agent_install_refresh_preflight(
    agent_id: String,
    state: State<'_, AppState>,
) -> Result<PreflightLayerState, AgentInstallErrorDto> {
    state
        .agent_install_service
        .refresh_preflight(&agent_id)
        .map_err(Into::into)
}

#[tauri::command]
pub async fn agent_install_create_plan(
    agent_id: String,
    state: State<'_, AppState>,
) -> Result<PlanLayerState, AgentInstallErrorDto> {
    state
        .agent_install_service
        .create_plan(&agent_id)
        .map_err(Into::into)
}

#[tauri::command]
pub async fn agent_install_reconfirm_plan(
    snapshot_id: String,
    state: State<'_, AppState>,
) -> Result<PlanLayerState, AgentInstallErrorDto> {
    state
        .agent_install_service
        .reconfirm_plan(&snapshot_id)
        .map_err(Into::into)
}

#[tauri::command]
pub async fn agent_install_start_install(
    request: StartAgentInstallRequest,
    state: State<'_, AppState>,
) -> Result<PlanLayerState, AgentInstallErrorDto> {
    state
        .agent_install_service
        .start_install(request)
        .map_err(Into::into)
}

#[tauri::command]
pub async fn agent_install_get_job() -> Result<(), AgentInstallErrorDto> {
    Ok(())
}

#[tauri::command]
pub async fn agent_install_cancel_install(_job_id: String) -> Result<(), AgentInstallErrorDto> {
    Ok(())
}

#[tauri::command]
pub async fn agent_install_probe_health(
    agent_id: String,
    state: State<'_, AppState>,
) -> Result<HealthProbeResult, AgentInstallErrorDto> {
    state
        .agent_install_service
        .probe(&agent_id)
        .map_err(Into::into)
}

#[tauri::command]
pub async fn agent_install_open_official_guide(
    agent_id: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<bool, AgentInstallErrorDto> {
    let url = state
        .agent_install_service
        .official_guide_url(&agent_id)
        .map_err(AgentInstallErrorDto::from)?;
    crate::platform::process_launch::open_http_url_as_user(app, url)
        .await
        .map_err(|_| crate::agent_install::AgentInstallError::guide_unavailable().to_dto())?;
    Ok(true)
}
