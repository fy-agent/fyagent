//! IPC facade for the unified managed-auth control plane.

use crate::services::managed_auth::{
    validate_session_id, ManagedAuthAccountMutationRequest, ManagedAuthAccountRemovalPreview,
    ManagedAuthAccountRemovalRequest, ManagedAuthConnectionActionRequest, ManagedAuthErrorDto,
    ManagedAuthLoginMethod, ManagedAuthLoginSessionSnapshot, ManagedAuthMutationResult,
    ManagedAuthOverview, StartManagedAuthLoginRequest,
};

#[tauri::command]
pub fn managed_auth_get_overview() -> ManagedAuthOverview {
    ManagedAuthOverview::unavailable()
}

#[tauri::command(rename_all = "camelCase")]
pub fn managed_auth_start_login(
    request: StartManagedAuthLoginRequest,
) -> Result<ManagedAuthLoginSessionSnapshot, ManagedAuthErrorDto> {
    request.validate()?;
    Err(ManagedAuthErrorDto::unavailable())
}

#[tauri::command(rename_all = "camelCase")]
pub fn managed_auth_get_login_session(
    session_id: String,
) -> Result<ManagedAuthLoginSessionSnapshot, ManagedAuthErrorDto> {
    validate_session_id(&session_id)?;
    Err(ManagedAuthErrorDto::unavailable())
}

#[tauri::command(rename_all = "camelCase")]
pub fn managed_auth_cancel_login(
    session_id: String,
) -> Result<ManagedAuthLoginSessionSnapshot, ManagedAuthErrorDto> {
    validate_session_id(&session_id)?;
    Err(ManagedAuthErrorDto::unavailable())
}

#[tauri::command(rename_all = "camelCase")]
pub fn managed_auth_reopen_login(
    session_id: String,
) -> Result<ManagedAuthLoginSessionSnapshot, ManagedAuthErrorDto> {
    validate_session_id(&session_id)?;
    Err(ManagedAuthErrorDto::unavailable())
}

#[tauri::command(rename_all = "camelCase")]
pub fn managed_auth_switch_login_method(
    session_id: String,
    method: ManagedAuthLoginMethod,
) -> Result<ManagedAuthLoginSessionSnapshot, ManagedAuthErrorDto> {
    validate_session_id(&session_id)?;
    let _ = method;
    Err(ManagedAuthErrorDto::unavailable())
}

#[tauri::command(rename_all = "camelCase")]
pub fn managed_auth_set_default_account(
    request: ManagedAuthAccountMutationRequest,
) -> Result<ManagedAuthMutationResult, ManagedAuthErrorDto> {
    request.validate()?;
    Err(ManagedAuthErrorDto::unavailable())
}

#[tauri::command(rename_all = "camelCase")]
pub fn managed_auth_preview_account_removal(
    request: ManagedAuthAccountMutationRequest,
) -> Result<ManagedAuthAccountRemovalPreview, ManagedAuthErrorDto> {
    request.validate()?;
    Err(ManagedAuthErrorDto::unavailable())
}

#[tauri::command(rename_all = "camelCase")]
pub fn managed_auth_remove_account(
    request: ManagedAuthAccountRemovalRequest,
) -> Result<ManagedAuthMutationResult, ManagedAuthErrorDto> {
    request.validate()?;
    Err(ManagedAuthErrorDto::unavailable())
}

#[tauri::command(rename_all = "camelCase")]
pub fn managed_auth_apply_connection_action(
    request: ManagedAuthConnectionActionRequest,
) -> Result<ManagedAuthMutationResult, ManagedAuthErrorDto> {
    request.validate()?;
    Err(ManagedAuthErrorDto::unavailable())
}
