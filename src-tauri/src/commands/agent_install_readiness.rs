//! Registered read-only IPC owner for the Agent install readiness projection.

use crate::{
    agent_install::{readiness_for, AgentInstallReadinessDto},
    services::external_agents::AgentCatalogId,
};

#[tauri::command]
pub fn get_agent_install_readiness(agent_id: AgentCatalogId) -> AgentInstallReadinessDto {
    readiness_for(agent_id)
}
