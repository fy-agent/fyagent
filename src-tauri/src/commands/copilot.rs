//! GitHub Copilot Tauri Commands
//!
//! 提供 Copilot OAuth 认证相关的 Tauri 命令，支持多账号管理。

use crate::proxy::providers::copilot_auth::{
    CopilotAuthManager, CopilotAuthStatus, CopilotModel, CopilotUsageResponse, GitHubAccount,
    GitHubDeviceCodeResponse,
};
use std::sync::Arc;
use tauri::State;
use tokio::sync::RwLock;

/// Copilot 认证状态
pub struct CopilotAuthState(pub Arc<RwLock<CopilotAuthManager>>);

/// Renderer IPC must never receive Copilot access tokens. Proxy refresh stays
/// on the native manager path; leftover `copilot_get_token*` commands stay
/// registered only so old clients fail closed instead of leaking.
pub(crate) const COPILOT_TOKEN_IPC_DENIED: &str = "copilot_token_not_exposed";

fn deny_copilot_token_ipc() -> Result<String, String> {
    Err(COPILOT_TOKEN_IPC_DENIED.to_string())
}

/// Leftover Copilot login/remove IPC stays registered so old clients fail
/// closed instead of starting a second Device Code owner.
const LEGACY_COPILOT_MUTATION_DISABLED: &str = "legacy_auth_mutation_disabled";

fn deny_legacy_copilot_mutation<T>() -> Result<T, String> {
    Err(LEGACY_COPILOT_MUTATION_DISABLED.to_string())
}

// ==================== 设备码流程 ====================

/// Leftover Device Code entry. Login owner is `managed_auth_start_login`.
#[tauri::command]
pub async fn copilot_start_device_flow(
    _github_domain: Option<String>,
) -> Result<GitHubDeviceCodeResponse, String> {
    deny_legacy_copilot_mutation()
}

/// Leftover poll entry. Login owner is Managed Auth.
#[tauri::command(rename_all = "camelCase")]
pub async fn copilot_poll_for_auth(
    _device_code: String,
    _github_domain: Option<String>,
) -> Result<bool, String> {
    deny_legacy_copilot_mutation()
}

/// Leftover multi-account poll entry. Login owner is Managed Auth.
#[tauri::command(rename_all = "camelCase")]
pub async fn copilot_poll_for_account(
    _device_code: String,
    _github_domain: Option<String>,
) -> Result<Option<GitHubAccount>, String> {
    deny_legacy_copilot_mutation()
}

// ==================== 多账号管理 ====================

/// 列出所有已认证的账号
#[tauri::command]
pub async fn copilot_list_accounts(
    state: State<'_, CopilotAuthState>,
) -> Result<Vec<GitHubAccount>, String> {
    let auth_manager = state.0.read().await;
    Ok(auth_manager.list_accounts().await)
}

/// Leftover remove entry. Destructive account removal is V2 `/auth`.
#[tauri::command(rename_all = "camelCase")]
pub async fn copilot_remove_account(_account_id: String) -> Result<(), String> {
    deny_legacy_copilot_mutation()
}

/// Leftover default-account entry. Defaults are owned by Managed Auth.
#[tauri::command(rename_all = "camelCase")]
pub async fn copilot_set_default_account(_account_id: String) -> Result<(), String> {
    deny_legacy_copilot_mutation()
}

// ==================== 状态查询 ====================

/// 获取认证状态（包含所有账号）
#[tauri::command]
pub async fn copilot_get_auth_status(
    state: State<'_, CopilotAuthState>,
) -> Result<CopilotAuthStatus, String> {
    let auth_manager = state.0.read().await;
    Ok(auth_manager.get_status().await)
}

/// 检查是否已认证（有任意账号）
#[tauri::command]
pub async fn copilot_is_authenticated(state: State<'_, CopilotAuthState>) -> Result<bool, String> {
    let auth_manager = state.0.read().await;
    Ok(auth_manager.is_authenticated().await)
}

/// Leftover logout-all entry. Account removal is V2 `/auth`.
#[tauri::command]
pub async fn copilot_logout() -> Result<(), String> {
    deny_legacy_copilot_mutation()
}

// ==================== Token 获取 ====================

/// 获取有效的 Copilot Token（向后兼容：使用第一个账号）
///
/// 已对 renderer 永久关闭。V2 账号页不得依赖此命令；不要新增向
/// renderer 返回 token 的命令。
#[tauri::command]
pub async fn copilot_get_token(_state: State<'_, CopilotAuthState>) -> Result<String, String> {
    deny_copilot_token_ipc()
}

/// 获取指定账号的有效 Copilot Token
#[tauri::command(rename_all = "camelCase")]
pub async fn copilot_get_token_for_account(
    _account_id: String,
    _state: State<'_, CopilotAuthState>,
) -> Result<String, String> {
    deny_copilot_token_ipc()
}

// ==================== 模型和使用量 ====================

/// 获取 Copilot 可用模型列表（向后兼容：使用第一个账号）
#[tauri::command]
pub async fn copilot_get_models(
    state: State<'_, CopilotAuthState>,
) -> Result<Vec<CopilotModel>, String> {
    let auth_manager = state.0.read().await;
    auth_manager.fetch_models().await.map_err(|e| e.to_string())
}

/// 获取指定账号的 Copilot 可用模型列表
#[tauri::command(rename_all = "camelCase")]
pub async fn copilot_get_models_for_account(
    account_id: String,
    state: State<'_, CopilotAuthState>,
) -> Result<Vec<CopilotModel>, String> {
    let auth_manager = state.0.read().await;
    auth_manager
        .fetch_models_for_account(&account_id)
        .await
        .map_err(|e| e.to_string())
}

/// 获取 Copilot 使用量信息（向后兼容：使用第一个账号）
#[tauri::command]
pub async fn copilot_get_usage(
    state: State<'_, CopilotAuthState>,
) -> Result<CopilotUsageResponse, String> {
    let auth_manager = state.0.read().await;
    auth_manager.fetch_usage().await.map_err(|e| e.to_string())
}

/// 获取指定账号的 Copilot 使用量信息
#[tauri::command(rename_all = "camelCase")]
pub async fn copilot_get_usage_for_account(
    account_id: String,
    state: State<'_, CopilotAuthState>,
) -> Result<CopilotUsageResponse, String> {
    let auth_manager = state.0.read().await;
    auth_manager
        .fetch_usage_for_account(&account_id)
        .await
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::deny_copilot_token_ipc;

    #[test]
    fn copilot_token_ipc_is_fail_closed() {
        let error = deny_copilot_token_ipc().expect_err("renderer token ipc");
        assert_eq!(error, super::COPILOT_TOKEN_IPC_DENIED);
        assert!(!error.to_ascii_lowercase().contains("gho_"));
        assert!(!error.to_ascii_lowercase().contains("token="));
    }

    #[tokio::test]
    async fn leftover_copilot_login_ipc_is_fail_closed() {
        let start = super::copilot_start_device_flow(None).await;
        let poll_auth = super::copilot_poll_for_auth("device-code".into(), None).await;
        let poll_account = super::copilot_poll_for_account("device-code".into(), None).await;
        let remove = super::copilot_remove_account("account-1".into()).await;
        let set_default = super::copilot_set_default_account("account-1".into()).await;
        let logout = super::copilot_logout().await;

        for error in [
            start.unwrap_err(),
            poll_auth.unwrap_err(),
            poll_account.unwrap_err(),
            remove.unwrap_err(),
            set_default.unwrap_err(),
            logout.unwrap_err(),
        ] {
            assert_eq!(error, super::LEGACY_COPILOT_MUTATION_DISABLED);
            let lower = error.to_ascii_lowercase();
            assert!(!lower.contains("gho_"));
            assert!(!lower.contains("token="));
            assert!(!lower.contains("device_code"));
        }
    }
}
