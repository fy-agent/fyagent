//! One closed read-only Agent health command. No path, shell or action input.
use super::managed_auth::ManagedAuthState;
use crate::{
    services::{
        external_agents::AgentCatalogId,
        health::{self, AgentHealthSnapshot},
    },
    store::AppState,
};
use tauri::State;

#[tauri::command(rename_all = "camelCase")]
pub async fn get_agent_health(
    agent_id: AgentCatalogId,
    state: State<'_, AppState>,
    managed_auth: State<'_, ManagedAuthState>,
) -> Result<AgentHealthSnapshot, &'static str> {
    health::read(agent_id, state.inner().clone(), managed_auth.0.clone()).await
}
