//! Narrow IPC commands for TRAE model preflight and external MCP validation.

use serde_json::Value;
use tauri::State;

use crate::services::traework::{
    self, ExternalMcpAgentId, TraeEndpointCancelResult, TraeEndpointProbeResult,
    TraeEndpointProbeState, TraeErrorDto, TraeMcpValidationResult, TraeModelConfigRequest,
};
use crate::services::traework_models::{self, TraeWorkModelIdsResult};

#[tauri::command(rename_all = "camelCase")]
pub async fn validate_traework_model_config(
    request: TraeModelConfigRequest,
) -> Result<TraeEndpointProbeResult, TraeErrorDto> {
    traework::validate_traework_model_config(request)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn test_traework_model_endpoint(
    request_id: String,
    request: TraeModelConfigRequest,
    state: State<'_, TraeEndpointProbeState>,
) -> Result<TraeEndpointProbeResult, TraeErrorDto> {
    // Registration is an RAII guard: duplicate IDs fail before validation and
    // every return, cancellation, timeout, or unwind removes the active token.
    let registration = state.register(&request_id)?;
    traework::test_traework_model_endpoint(
        registration.request_id(),
        request,
        registration.cancellation(),
    )
    .await
}

#[tauri::command(rename_all = "camelCase")]
pub async fn cancel_traework_model_endpoint(
    request_id: String,
    state: State<'_, TraeEndpointProbeState>,
) -> Result<TraeEndpointCancelResult, TraeErrorDto> {
    state.cancel(&request_id)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn get_traework_model_ids() -> Result<TraeWorkModelIdsResult, TraeErrorDto> {
    traework_models::get_traework_model_ids().await
}

#[tauri::command(rename_all = "camelCase")]
pub async fn validate_external_mcp_config(
    agent_id: ExternalMcpAgentId,
    config: Value,
) -> Result<TraeMcpValidationResult, TraeErrorDto> {
    traework::validate_external_mcp_config(agent_id, config)
}
