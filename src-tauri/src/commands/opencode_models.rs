//! Narrow IPC commands for OpenCode provider model snapshot, fetch, and save.

use crate::services::opencode_models::{
    self, FetchOpenCodeModelsRequest, FetchedModelList, OpenCodeModelSnapshot,
    OpenCodeModelsErrorDto, SaveOpenCodeModelsOutcome, SaveOpenCodeModelsRequest,
};

#[tauri::command(rename_all = "camelCase")]
pub fn get_opencode_model_snapshot() -> Result<OpenCodeModelSnapshot, OpenCodeModelsErrorDto> {
    opencode_models::get_opencode_model_snapshot()
}

#[tauri::command(rename_all = "camelCase")]
pub async fn fetch_opencode_provider_models(
    request: FetchOpenCodeModelsRequest,
) -> Result<FetchedModelList, OpenCodeModelsErrorDto> {
    opencode_models::fetch_opencode_provider_models(request).await
}

#[tauri::command(rename_all = "camelCase")]
pub fn save_opencode_models(
    request: SaveOpenCodeModelsRequest,
) -> Result<SaveOpenCodeModelsOutcome, OpenCodeModelsErrorDto> {
    opencode_models::save_opencode_models(request)
}
