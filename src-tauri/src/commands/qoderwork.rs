//! Thin IPC boundary for QoderWork's fixed user Hooks document.

use tauri::State;

use crate::services::qoderwork::{
    self, QoderHooksErrorDto, QoderHooksSnapshot, QoderHooksState, SaveQoderworkHooksOutcome,
    SaveQoderworkHooksRequest,
};

#[tauri::command]
pub async fn get_qoderwork_hooks(
    state: State<'_, QoderHooksState>,
) -> Result<QoderHooksSnapshot, QoderHooksErrorDto> {
    qoderwork::get_qoderwork_hooks(state.inner())
        .await
        .map_err(Into::into)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn save_qoderwork_hooks(
    state: State<'_, QoderHooksState>,
    request: SaveQoderworkHooksRequest,
) -> Result<SaveQoderworkHooksOutcome, QoderHooksErrorDto> {
    qoderwork::save_qoderwork_hooks(state.inner(), request)
        .await
        .map_err(Into::into)
}
