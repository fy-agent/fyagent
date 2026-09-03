//! IPC facade for the unified managed-auth control plane.

use std::sync::Arc;

use tauri::State;

use crate::services::managed_auth::{
    validate_session_id, ManagedAuthAccountMutationRequest, ManagedAuthAccountRemovalPreview,
    ManagedAuthAccountRemovalRequest, ManagedAuthConnectionActionRequest, ManagedAuthErrorDto,
    ManagedAuthLoginMethod, ManagedAuthLoginSessionSnapshot, ManagedAuthMutationResult,
    ManagedAuthOverview, NativeManagedAuthService, StartManagedAuthLoginRequest,
};

pub struct ManagedAuthState(pub(crate) Arc<NativeManagedAuthService>);

#[tauri::command]
pub fn managed_auth_get_overview(state: State<'_, ManagedAuthState>) -> ManagedAuthOverview {
    state.0.overview()
}

#[tauri::command(rename_all = "camelCase")]
pub fn managed_auth_start_login(
    request: StartManagedAuthLoginRequest,
    state: State<'_, ManagedAuthState>,
) -> Result<ManagedAuthLoginSessionSnapshot, ManagedAuthErrorDto> {
    request.validate()?;
    state.0.start_login(request)
}

#[tauri::command(rename_all = "camelCase")]
pub fn managed_auth_get_login_session(
    session_id: String,
    state: State<'_, ManagedAuthState>,
) -> Result<ManagedAuthLoginSessionSnapshot, ManagedAuthErrorDto> {
    validate_session_id(&session_id)?;
    state.0.get_login_session(&session_id)
}

#[tauri::command(rename_all = "camelCase")]
pub fn managed_auth_cancel_login(
    session_id: String,
    state: State<'_, ManagedAuthState>,
) -> Result<ManagedAuthLoginSessionSnapshot, ManagedAuthErrorDto> {
    validate_session_id(&session_id)?;
    state.0.cancel_login(&session_id)
}

#[tauri::command(rename_all = "camelCase")]
pub fn managed_auth_reopen_login(
    session_id: String,
    state: State<'_, ManagedAuthState>,
) -> Result<ManagedAuthLoginSessionSnapshot, ManagedAuthErrorDto> {
    validate_session_id(&session_id)?;
    state.0.reopen_login(&session_id)
}

#[tauri::command(rename_all = "camelCase")]
pub fn managed_auth_switch_login_method(
    session_id: String,
    method: ManagedAuthLoginMethod,
    state: State<'_, ManagedAuthState>,
) -> Result<ManagedAuthLoginSessionSnapshot, ManagedAuthErrorDto> {
    validate_session_id(&session_id)?;
    state.0.switch_login_method(&session_id, method)
}

#[tauri::command(rename_all = "camelCase")]
pub fn managed_auth_set_default_account(
    request: ManagedAuthAccountMutationRequest,
    state: State<'_, ManagedAuthState>,
) -> Result<ManagedAuthMutationResult, ManagedAuthErrorDto> {
    request.validate()?;
    state
        .0
        .set_default_account(&request.account_id, &request.expected_revision)
        .map_err(ManagedAuthErrorDto::from_core)
}

#[tauri::command(rename_all = "camelCase")]
pub fn managed_auth_preview_account_removal(
    request: ManagedAuthAccountMutationRequest,
    state: State<'_, ManagedAuthState>,
) -> Result<ManagedAuthAccountRemovalPreview, ManagedAuthErrorDto> {
    request.validate()?;
    state
        .0
        .preview_account_removal(&request.account_id, &request.expected_revision)
        .map_err(ManagedAuthErrorDto::from_core)
}

#[tauri::command(rename_all = "camelCase")]
pub fn managed_auth_remove_account(
    request: ManagedAuthAccountRemovalRequest,
    state: State<'_, ManagedAuthState>,
) -> Result<ManagedAuthMutationResult, ManagedAuthErrorDto> {
    request.validate()?;
    state
        .0
        .remove_account(
            &request.preview_id,
            &request.account_id,
            &request.expected_revision,
        )
        .map_err(ManagedAuthErrorDto::from_core)
}

#[tauri::command(rename_all = "camelCase")]
pub fn managed_auth_apply_connection_action(
    request: ManagedAuthConnectionActionRequest,
    state: State<'_, ManagedAuthState>,
) -> Result<ManagedAuthMutationResult, ManagedAuthErrorDto> {
    request.validate()?;
    state.0.apply_connection_action(&request)
}
