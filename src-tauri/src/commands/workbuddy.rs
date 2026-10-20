//! Thin IPC shell for the isolated WorkBuddy configuration domain.

use crate::services::workbuddy::{
    self,
    error::WorkBuddyErrorDto,
    types::{
        FetchWorkBuddyModelsRequest, FetchWorkBuddyModelsResult, SaveWorkBuddyModelsOutcome,
        SaveWorkBuddyModelsRequest, WorkBuddyModelIdsResult, WorkBuddyStatus,
    },
};

/// Return only the local path and a non-sensitive configuration summary.
#[tauri::command]
pub async fn get_workbuddy_status() -> Result<WorkBuddyStatus, WorkBuddyErrorDto> {
    workbuddy::get_workbuddy_status().await.map_err(Into::into)
}

/// Return the only existing-model projection the renderer may display.
/// Configuration paths, URLs, credentials, vendor metadata, and duplicate
/// entry counts remain inside the WorkBuddy service.
#[tauri::command]
pub async fn get_workbuddy_model_ids() -> Result<WorkBuddyModelIdsResult, WorkBuddyErrorDto> {
    workbuddy::get_workbuddy_model_ids()
        .await
        .map_err(Into::into)
}

/// Fetch a bounded OpenAI-compatible model list through WorkBuddy's restricted
/// fixed-endpoint transport.
#[tauri::command(rename_all = "camelCase")]
pub async fn fetch_workbuddy_models(
    request: FetchWorkBuddyModelsRequest,
) -> Result<FetchWorkBuddyModelsResult, WorkBuddyErrorDto> {
    workbuddy::fetch_workbuddy_models(request)
        .await
        .map_err(Into::into)
}

/// Upsert selected WorkBuddy models through the revision-checked config
/// transaction. This command never accepts a Provider/AppType or a filesystem
/// path from the renderer.
#[tauri::command(rename_all = "camelCase")]
pub async fn save_workbuddy_models(
    request: SaveWorkBuddyModelsRequest,
) -> Result<SaveWorkBuddyModelsOutcome, WorkBuddyErrorDto> {
    workbuddy::save_workbuddy_models(request)
        .await
        .map_err(Into::into)
}

/// Fresh native comparison of the selected binding and actual model connection.
#[tauri::command]
pub async fn get_workbuddy_subscription_source(
    app_handle: tauri::AppHandle,
) -> Result<workbuddy::types::WorkBuddySubscriptionSourceResult, String> {
    use tauri::Manager;
    tauri::async_runtime::spawn_blocking(move || {
        let state = app_handle
            .try_state::<crate::store::AppState>()
            .ok_or_else(|| "internal".to_string())?;
        let _guard = workbuddy::write_lock().blocking_lock();
        workbuddy::subscription::read_subscription_source(&state)
            .map_err(|error| error.code().as_str().to_string())
    })
    .await
    .map_err(|_| "internal".to_string())?
}
