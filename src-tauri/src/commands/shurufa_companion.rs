use crate::services::shurufa_companion::{
    CompanionApplyDeviceConfig, CompanionDeviceSettings, CompanionNetwork, CompanionProfile,
    CompanionRuntime, CompanionSnapshot, CompanionState, CompanionTarget,
};

#[tauri::command]
pub async fn shurufa_companion_list_ports(
    state: tauri::State<'_, CompanionState>,
) -> Result<Vec<String>, String> {
    let io = state.io();
    tauri::async_runtime::spawn_blocking(move || io.list_ports())
        .await
        .map_err(|_| "listing serial ports was cancelled".to_owned())?
}

#[tauri::command]
pub async fn shurufa_companion_capture_target(
    state: tauri::State<'_, CompanionState>,
) -> Result<CompanionTarget, String> {
    let io = state.io();
    tauri::async_runtime::spawn_blocking(move || io.capture_target())
        .await
        .map_err(|_| "capture target was cancelled".to_owned())?
}

#[tauri::command]
pub async fn shurufa_companion_get_snapshot(
    state: tauri::State<'_, CompanionState>,
) -> Result<CompanionSnapshot, String> {
    state.snapshot()
}

#[tauri::command]
pub async fn shurufa_companion_save_profile(
    state: tauri::State<'_, CompanionState>,
    draft: CompanionProfile,
) -> Result<CompanionProfile, String> {
    let io = state.io();
    tauri::async_runtime::spawn_blocking(move || io.save_profile(draft))
        .await
        .map_err(|_| "save profile was cancelled".to_owned())?
}

#[tauri::command]
pub async fn shurufa_companion_start_dry_run(
    state: tauri::State<'_, CompanionState>,
) -> Result<CompanionRuntime, String> {
    let io = state.io();
    tauri::async_runtime::spawn_blocking(move || io.start_dry_run())
        .await
        .map_err(|_| "start dry-run was cancelled".to_owned())?
}

#[tauri::command]
pub async fn shurufa_companion_enable_live(
    state: tauri::State<'_, CompanionState>,
) -> Result<CompanionRuntime, String> {
    let io = state.io();
    tauri::async_runtime::spawn_blocking(move || io.enable_live())
        .await
        .map_err(|_| "enable live was cancelled".to_owned())?
}

#[tauri::command]
pub async fn shurufa_companion_stop(
    state: tauri::State<'_, CompanionState>,
) -> Result<CompanionRuntime, String> {
    let io = state.io();
    tauri::async_runtime::spawn_blocking(move || io.stop())
        .await
        .map_err(|_| "stop was cancelled".to_owned())?
}

#[tauri::command]
pub async fn shurufa_companion_save_device_settings(
    state: tauri::State<'_, CompanionState>,
    draft: CompanionDeviceSettings,
) -> Result<CompanionDeviceSettings, String> {
    let io = state.io();
    tauri::async_runtime::spawn_blocking(move || io.save_device_settings(draft))
        .await
        .map_err(|_| "save device settings was cancelled".to_owned())?
}

#[tauri::command]
pub async fn shurufa_companion_apply_device_config(
    state: tauri::State<'_, CompanionState>,
    request: CompanionApplyDeviceConfig,
) -> Result<CompanionNetwork, String> {
    let io = state.io();
    tauri::async_runtime::spawn_blocking(move || io.apply_device_config(request))
        .await
        .map_err(|_| "apply device config was cancelled".to_owned())?
}
