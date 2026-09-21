use crate::services::config_pack::{
    self, Candidates, ConfigPackService, ExportPreview, ImportChoice, ImportPreview, ImportResult,
    PackError,
};
use crate::store::AppState;
use serde::Deserialize;
use std::sync::Mutex;
use tauri_plugin_dialog::DialogExt;

#[derive(Default)]
pub(crate) struct ConfigPackState(pub Mutex<ConfigPackService>);
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ImportRequest {
    text: String,
    choices: Vec<ImportChoice>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ConfirmRequest {
    preview_id: String,
    digest: String,
}

#[tauri::command]
pub(crate) fn list_config_pack_candidates(
    state: tauri::State<'_, AppState>,
) -> Result<Candidates, PackError> {
    ConfigPackService::candidates(&state.db)
}
#[tauri::command]
pub(crate) fn preview_config_pack_export(
    state: tauri::State<'_, AppState>,
    packs: tauri::State<'_, ConfigPackState>,
    selection: Vec<String>,
) -> Result<ExportPreview, PackError> {
    packs
        .0
        .lock()
        .map_err(|_| PackError::Busy)?
        .preview_export(&state.db, selection)
}
#[tauri::command]
pub(crate) fn preview_config_pack_import(
    state: tauri::State<'_, AppState>,
    packs: tauri::State<'_, ConfigPackState>,
    request: ImportRequest,
) -> Result<ImportPreview, PackError> {
    packs
        .0
        .lock()
        .map_err(|_| PackError::Busy)?
        .preview_for_state(&state, &request.text, request.choices)
}
#[tauri::command]
pub(crate) fn apply_config_pack_import(
    state: tauri::State<'_, AppState>,
    packs: tauri::State<'_, ConfigPackState>,
    request: ConfirmRequest,
) -> Result<ImportResult, PackError> {
    packs
        .0
        .lock()
        .map_err(|_| PackError::Busy)?
        .apply_for_state(&state, &request.preview_id, &request.digest)
}
#[tauri::command]
pub(crate) fn cancel_config_pack_preview(
    packs: tauri::State<'_, ConfigPackState>,
    preview_id: String,
) -> Result<(), PackError> {
    packs
        .0
        .lock()
        .map_err(|_| PackError::Busy)?
        .cancel(&preview_id);
    Ok(())
}
#[tauri::command]
pub(crate) async fn pick_config_pack_file(
    app: tauri::AppHandle,
) -> Result<Option<String>, PackError> {
    let file = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .add_filter("FyAgent 连接配置", &["json"])
            .blocking_pick_file()
    })
    .await
    .map_err(|_| PackError::FileUnavailable)?;
    file.map(|f| {
        let path = f.into_path().map_err(|_| PackError::UnsafeContent)?;
        config_pack::read_file(&path)
    })
    .transpose()
}
#[tauri::command]
pub(crate) async fn save_config_pack_export(
    app: tauri::AppHandle,
    packs: tauri::State<'_, ConfigPackState>,
    request: ConfirmRequest,
) -> Result<bool, PackError> {
    packs
        .0
        .lock()
        .map_err(|_| PackError::Busy)?
        .export_bytes(&request.preview_id, &request.digest)?;
    let file = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .add_filter("FyAgent 连接配置", &["json"])
            .set_file_name("connections.fyagent-config.json")
            .blocking_save_file()
    })
    .await
    .map_err(|_| PackError::FileUnavailable)?;
    let Some(file) = file else {
        return Ok(false);
    };
    let path = file.into_path().map_err(|_| PackError::UnsafeContent)?;
    let mut service = packs.0.lock().map_err(|_| PackError::Busy)?;
    let bytes = service.export_bytes(&request.preview_id, &request.digest)?;
    config_pack::export_file(&path, &bytes)?;
    service.cancel(&request.preview_id);
    Ok(true)
}
