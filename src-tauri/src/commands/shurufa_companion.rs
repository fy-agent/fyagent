use crate::services::shurufa_companion::{
    CompanionApplyDeviceConfig, CompanionDeviceSettings, CompanionNetwork, CompanionProfile,
    CompanionRuntime, CompanionSnapshot, CompanionState, CompanionTarget,
};

#[tauri::command]
pub fn shurufa_companion_list_ports() -> Result<Vec<String>, String> {
    CompanionState::list_ports()
}

#[tauri::command]
pub fn shurufa_companion_capture_target(
    state: tauri::State<'_, CompanionState>,
) -> Result<CompanionTarget, String> {
    state.capture_target()
}

#[tauri::command]
pub fn shurufa_companion_get_snapshot(
    state: tauri::State<'_, CompanionState>,
) -> Result<CompanionSnapshot, String> {
    state.snapshot()
}

#[tauri::command]
pub fn shurufa_companion_save_profile(
    state: tauri::State<'_, CompanionState>,
    draft: CompanionProfile,
) -> Result<CompanionProfile, String> {
    state.save_profile(draft)
}

#[tauri::command]
pub fn shurufa_companion_start_dry_run(
    state: tauri::State<'_, CompanionState>,
) -> Result<CompanionRuntime, String> {
    state.start_dry_run()
}

#[tauri::command]
pub fn shurufa_companion_enable_live(
    state: tauri::State<'_, CompanionState>,
) -> Result<CompanionRuntime, String> {
    state.enable_live()
}

#[tauri::command]
pub fn shurufa_companion_stop(
    state: tauri::State<'_, CompanionState>,
) -> Result<CompanionRuntime, String> {
    state.stop()
}

#[tauri::command]
pub fn shurufa_companion_save_device_settings(
    _state: tauri::State<'_, CompanionState>,
    draft: CompanionDeviceSettings,
) -> Result<CompanionDeviceSettings, String> {
    CompanionState::save_device_settings(draft)
}

#[tauri::command]
pub fn shurufa_companion_apply_device_config(
    state: tauri::State<'_, CompanionState>,
    request: CompanionApplyDeviceConfig,
) -> Result<CompanionNetwork, String> {
    state.apply_device_config(request)
}
