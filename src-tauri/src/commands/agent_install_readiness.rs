//! IPC owner for Agent install readiness and closed Agent actions.

use tauri::State;

use crate::{
    agent_install::{
        cancel_agent_action as cancel_job, get_agent_action_job as job_snapshot, readiness_for,
        start_agent_action as start_job, AgentActionErrorDto, AgentActionJobSnapshot,
        AgentActionResult, AgentInstallReadinessDto, AgentInstallationInventoryDto, AgentSurface,
        StartAgentActionRequest,
    },
    services::external_agents::AgentCatalogId,
    store::AppState,
};

#[tauri::command(rename_all = "camelCase")]
pub async fn get_agent_install_readiness(
    agent_id: AgentCatalogId,
    state: State<'_, AppState>,
) -> Result<AgentInstallReadinessDto, String> {
    Ok(readiness_for(agent_id, &state).await)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn get_agent_installation_inventory(
    agent_id: AgentCatalogId,
    surface: Option<AgentSurface>,
    state: State<'_, AppState>,
) -> Result<AgentInstallationInventoryDto, String> {
    Ok(crate::agent_install::inventory_for(agent_id, &state, surface).await)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn start_agent_action(
    request: StartAgentActionRequest,
    state: State<'_, AppState>,
) -> Result<AgentActionResult, AgentActionErrorDto> {
    start_job(request, &state)
        .await
        .map_err(AgentActionErrorDto::from)
}

#[tauri::command(rename_all = "camelCase")]
pub fn cancel_agent_action(
    job_id: String,
    state: State<'_, AppState>,
) -> Result<AgentActionJobSnapshot, AgentActionErrorDto> {
    cancel_job(&job_id, &state).map_err(AgentActionErrorDto::from)
}

#[tauri::command(rename_all = "camelCase")]
pub fn get_agent_action_job(
    job_id: String,
    state: State<'_, AppState>,
) -> Result<AgentActionJobSnapshot, AgentActionErrorDto> {
    job_snapshot(&job_id, &state).map_err(AgentActionErrorDto::from)
}
