//! IPC owner for bounded Agent-auth observation and session monitoring.

use tauri::State;

use crate::{
    agent_install::{
        auth_observation_for, get_agent_auth_session as session_snapshot,
        start_agent_auth_session as start_session, stop_waiting_for_agent_auth as stop_waiting,
        AgentAuthErrorDto, AgentAuthObservationDto, AgentAuthSessionSnapshot,
        StartAgentAuthSessionRequest,
    },
    services::external_agents::AgentCatalogId,
    store::AppState,
};

#[tauri::command(rename_all = "camelCase")]
pub async fn get_agent_auth_observation(
    agent_id: AgentCatalogId,
) -> Result<AgentAuthObservationDto, String> {
    Ok(auth_observation_for(agent_id).await)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn start_agent_auth_session(
    request: StartAgentAuthSessionRequest,
    state: State<'_, AppState>,
) -> Result<AgentAuthSessionSnapshot, AgentAuthErrorDto> {
    start_session(request, &state)
        .await
        .map_err(AgentAuthErrorDto::from)
}

#[tauri::command(rename_all = "camelCase")]
pub fn get_agent_auth_session(
    session_id: String,
    state: State<'_, AppState>,
) -> Result<AgentAuthSessionSnapshot, AgentAuthErrorDto> {
    session_snapshot(&session_id, &state).map_err(AgentAuthErrorDto::from)
}

#[tauri::command(rename_all = "camelCase")]
pub fn stop_waiting_for_agent_auth(
    session_id: String,
    state: State<'_, AppState>,
) -> Result<AgentAuthSessionSnapshot, AgentAuthErrorDto> {
    stop_waiting(&session_id, &state).map_err(AgentAuthErrorDto::from)
}
