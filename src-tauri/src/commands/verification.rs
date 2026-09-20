use crate::{services::verification::*, store::AppState};
use tauri::State;
use tauri_plugin_dialog::DialogExt;

#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn get_project_verification(
    project_id: String,
    state: State<'_, AppState>,
) -> VerificationResult<VerificationSnapshot> {
    let service = state.verification.clone();
    tauri::async_runtime::spawn_blocking(move || service.snapshot(&project_id))
        .await
        .map_err(|_| "verification_unavailable")?
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn run_project_verification(
    request: RunRequest,
    state: State<'_, AppState>,
) -> VerificationResult<VerificationSnapshot> {
    state.verification.run(request).await
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn record_project_verification(
    request: ManualRequest,
    state: State<'_, AppState>,
) -> VerificationResult<VerificationSnapshot> {
    let service = state.verification.clone();
    tauri::async_runtime::spawn_blocking(move || service.record_manual(request))
        .await
        .map_err(|_| "verification_unavailable")?
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn revoke_project_verification(
    request: RevokeRequest,
    state: State<'_, AppState>,
) -> VerificationResult<VerificationSnapshot> {
    let service = state.verification.clone();
    tauri::async_runtime::spawn_blocking(move || service.revoke(request))
        .await
        .map_err(|_| "verification_unavailable")?
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn save_project_handoff(
    request: SaveHandoffRequest,
    state: State<'_, AppState>,
) -> VerificationResult<VerificationSnapshot> {
    let service = state.verification.clone();
    tauri::async_runtime::spawn_blocking(move || service.save_handoff(request))
        .await
        .map_err(|_| "verification_unavailable")?
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn preview_project_handoff(
    project_id: String,
    state: State<'_, AppState>,
) -> VerificationResult<HandoffPreview> {
    let service = state.verification.clone();
    tauri::async_runtime::spawn_blocking(move || service.preview(&project_id))
        .await
        .map_err(|_| "verification_unavailable")?
}

#[derive(Clone, Copy, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum HandoffFormat {
    Json,
    Markdown,
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn export_project_handoff(
    app: tauri::AppHandle,
    project_id: String,
    format: HandoffFormat,
    state: State<'_, AppState>,
) -> VerificationResult<bool> {
    let service = state.verification.clone();
    tauri::async_runtime::spawn_blocking(move || {
        // Choose a destination, then re-read dependencies immediately before
        // export. No renderer-provided file path, payload or file-reading IPC.
        let extension = match format {
            HandoffFormat::Json => "json",
            HandoffFormat::Markdown => "md",
        };
        let selected = app
            .dialog()
            .file()
            .set_file_name(format!("fyagent-handoff.{extension}"))
            .add_filter("交接文件", &[extension])
            .blocking_save_file();
        let Some(selected) = selected else {
            return Ok(false);
        };
        let path = selected.into_path().map_err(|_| "export_failed")?;
        service.export_to(
            &project_id,
            &path,
            matches!(format, HandoffFormat::Markdown),
        )?;
        Ok(true)
    })
    .await
    .map_err(|_| "export_failed")?
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn cancel_project_verification(
    project_id: String,
    state: State<'_, AppState>,
) -> VerificationResult<bool> {
    state.verification.cancel(&project_id)
}
