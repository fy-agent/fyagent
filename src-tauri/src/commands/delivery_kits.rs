use crate::services::delivery_kits::{
    self, DemoResult, KitError, KitIdentity, KitLibrary, KitView, Preview,
};
use std::sync::Mutex;
use tauri_plugin_dialog::DialogExt;

pub(crate) struct DeliveryKitsState(pub Mutex<KitLibrary>);

#[tauri::command]
pub(crate) fn list_delivery_kits(
    state: tauri::State<'_, DeliveryKitsState>,
) -> Result<Vec<KitView>, KitError> {
    state.0.lock().map_err(|_| KitError::Busy)?.list()
}
#[tauri::command]
pub(crate) fn preview_builtin_delivery_kit(
    state: tauri::State<'_, DeliveryKitsState>,
    identity: KitIdentity,
) -> Result<Preview, KitError> {
    state
        .0
        .lock()
        .map_err(|_| KitError::Busy)?
        .preview_builtin(&identity)
}
#[tauri::command]
pub(crate) async fn pick_delivery_kit_import(
    app: tauri::AppHandle,
    state: tauri::State<'_, DeliveryKitsState>,
) -> Result<Option<Preview>, KitError> {
    let selected = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .add_filter("FyAgent 交付包", &["json"])
            .blocking_pick_file()
    })
    .await
    .map_err(|_| KitError::LibraryUnavailable)?;
    let Some(file) = selected else {
        return Ok(None);
    };
    let path = file.into_path().map_err(|_| KitError::UnsafeContent)?;
    state
        .0
        .lock()
        .map_err(|_| KitError::Busy)?
        .preview_file(&path)
        .map(Some)
}
#[tauri::command]
pub(crate) fn apply_delivery_kit_import(
    state: tauri::State<'_, DeliveryKitsState>,
    preview_id: String,
    manifest_digest: String,
) -> Result<KitView, KitError> {
    state
        .0
        .lock()
        .map_err(|_| KitError::Busy)?
        .apply(&preview_id, &manifest_digest)
}
#[tauri::command]
pub(crate) fn cancel_delivery_kit_preview(
    state: tauri::State<'_, DeliveryKitsState>,
    preview_id: String,
) -> Result<(), KitError> {
    state
        .0
        .lock()
        .map_err(|_| KitError::Busy)?
        .cancel(&preview_id);
    Ok(())
}
#[tauri::command]
pub(crate) fn preview_delivery_kit_export(
    state: tauri::State<'_, DeliveryKitsState>,
    identity: KitIdentity,
) -> Result<Preview, KitError> {
    state
        .0
        .lock()
        .map_err(|_| KitError::Busy)?
        .preview_export(&identity)
}
#[tauri::command]
pub(crate) async fn save_delivery_kit_export(
    app: tauri::AppHandle,
    state: tauri::State<'_, DeliveryKitsState>,
    preview_id: String,
    manifest_digest: String,
) -> Result<bool, KitError> {
    // Validate before opening a picker; recheck TTL/digest after it closes.
    state
        .0
        .lock()
        .map_err(|_| KitError::Busy)?
        .export_bytes(&preview_id, &manifest_digest)?;
    let selected = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .add_filter("FyAgent 交付包", &["json"])
            .set_file_name("delivery.fyagent-kit.json")
            .blocking_save_file()
    })
    .await
    .map_err(|_| KitError::LibraryUnavailable)?;
    let Some(file) = selected else {
        return Ok(false);
    };
    let path = file.into_path().map_err(|_| KitError::UnsafeContent)?;
    let mut library = state.0.lock().map_err(|_| KitError::Busy)?;
    let bytes = library.export_bytes(&preview_id, &manifest_digest)?;
    delivery_kits::export_file(&path, &bytes)?;
    library.cancel(&preview_id);
    Ok(true)
}
#[tauri::command]
pub(crate) fn run_delivery_kit_demo(
    state: tauri::State<'_, DeliveryKitsState>,
    identity: KitIdentity,
) -> Result<DemoResult, KitError> {
    state.0.lock().map_err(|_| KitError::Busy)?.run(&identity)
}
