use tauri::State;

use crate::commands::codex_oauth::CodexOAuthState;
use crate::commands::copilot::CopilotAuthState;
use crate::commands::managed_auth::ManagedAuthState;
use crate::commands::xai_oauth::XaiOAuthState;
use crate::proxy::providers::copilot_auth::GitHubAccount;
use crate::proxy::providers::xai_oauth_auth::XaiOAuthAccount;
use crate::services::managed_auth::{
    CompatibilityAccount, ManagedAuthProvider, CODEX_MIGRATION_ID, COPILOT_MIGRATION_ID,
    XAI_MIGRATION_ID,
};

/// Leftover `auth_*` login/remove IPC stays registered so old clients fail
/// closed instead of starting a second Device Code or JSON-store owner.
pub(crate) const LEGACY_AUTH_MUTATION_DISABLED: &str = "legacy_auth_mutation_disabled";

fn deny_legacy_auth_mutation<T>() -> Result<T, String> {
    Err(LEGACY_AUTH_MUTATION_DISABLED.to_string())
}

const AUTH_PROVIDER_GITHUB_COPILOT: &str = "github_copilot";
const AUTH_PROVIDER_CODEX_OAUTH: &str = "codex_oauth";
const AUTH_PROVIDER_XAI_OAUTH: &str = "xai_oauth";

#[derive(Debug, Clone, serde::Serialize)]
pub struct ManagedAuthAccount {
    pub id: String,
    pub provider: String,
    pub login: String,
    pub avatar_url: Option<String>,
    pub authenticated_at: i64,
    pub is_default: bool,
    pub github_domain: String,
    pub requires_reauth: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chatgpt_account_id: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ManagedAuthStatus {
    pub provider: String,
    pub authenticated: bool,
    pub default_account_id: Option<String>,
    pub migration_error: Option<String>,
    pub accounts: Vec<ManagedAuthAccount>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub native_projection_available: Option<bool>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ManagedAuthDeviceCodeResponse {
    pub provider: String,
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
}

fn ensure_auth_provider(auth_provider: &str) -> Result<&'static str, String> {
    match auth_provider {
        AUTH_PROVIDER_GITHUB_COPILOT => Ok(AUTH_PROVIDER_GITHUB_COPILOT),
        AUTH_PROVIDER_CODEX_OAUTH => Ok(AUTH_PROVIDER_CODEX_OAUTH),
        AUTH_PROVIDER_XAI_OAUTH => Ok(AUTH_PROVIDER_XAI_OAUTH),
        _ => Err(format!("Unsupported auth provider: {auth_provider}")),
    }
}

fn map_account(
    provider: &str,
    account: GitHubAccount,
    default_account_id: Option<&str>,
) -> ManagedAuthAccount {
    ManagedAuthAccount {
        is_default: default_account_id == Some(account.id.as_str()),
        id: account.id,
        provider: provider.to_string(),
        login: account.login,
        avatar_url: account.avatar_url,
        authenticated_at: account.authenticated_at,
        github_domain: account.github_domain,
        requires_reauth: false,
        chatgpt_account_id: account.chatgpt_account_id,
    }
}

fn map_xai_account(
    account: XaiOAuthAccount,
    default_account_id: Option<&str>,
) -> ManagedAuthAccount {
    ManagedAuthAccount {
        is_default: default_account_id == Some(account.id.as_str()),
        id: account.id,
        provider: AUTH_PROVIDER_XAI_OAUTH.to_string(),
        login: account.login,
        avatar_url: account.avatar_url,
        authenticated_at: account.authenticated_at,
        github_domain: account.github_domain,
        requires_reauth: account.requires_reauth,
        chatgpt_account_id: None,
    }
}

fn vault_provider(auth_provider: &str) -> Option<ManagedAuthProvider> {
    match auth_provider {
        AUTH_PROVIDER_GITHUB_COPILOT => Some(ManagedAuthProvider::GithubCopilot),
        AUTH_PROVIDER_CODEX_OAUTH => Some(ManagedAuthProvider::Openai),
        AUTH_PROVIDER_XAI_OAUTH => Some(ManagedAuthProvider::Xai),
        _ => None,
    }
}

fn vault_migration_id(provider: ManagedAuthProvider) -> &'static str {
    match provider {
        ManagedAuthProvider::Openai => CODEX_MIGRATION_ID,
        ManagedAuthProvider::Xai => XAI_MIGRATION_ID,
        ManagedAuthProvider::GithubCopilot => COPILOT_MIGRATION_ID,
    }
}

fn map_compatibility_account(provider: &str, account: CompatibilityAccount) -> ManagedAuthAccount {
    ManagedAuthAccount {
        id: account.id,
        provider: provider.to_string(),
        login: account.login,
        avatar_url: account.avatar_url,
        authenticated_at: account.authenticated_at,
        is_default: account.is_default,
        github_domain: account.github_domain,
        requires_reauth: account.requires_reauth,
        chatgpt_account_id: account.chatgpt_account_id,
    }
}

fn vault_accounts(
    managed_auth: &ManagedAuthState,
    auth_provider: &str,
) -> Option<Vec<ManagedAuthAccount>> {
    let provider = vault_provider(auth_provider)?;
    if !managed_auth
        .0
        .legacy_store_sealed(vault_migration_id(provider))
    {
        return None;
    }
    let accounts = managed_auth
        .0
        .compatibility_accounts(provider)
        .ok()
        .filter(|accounts| !accounts.is_empty())?;
    Some(
        accounts
            .into_iter()
            .map(|account| map_compatibility_account(auth_provider, account))
            .collect(),
    )
}

#[tauri::command(rename_all = "camelCase")]
pub async fn auth_start_login(
    _auth_provider: String,
    _github_domain: Option<String>,
) -> Result<ManagedAuthDeviceCodeResponse, String> {
    deny_legacy_auth_mutation()
}

#[tauri::command(rename_all = "camelCase")]
pub async fn auth_poll_for_account(
    _auth_provider: String,
    _device_code: String,
    _github_domain: Option<String>,
) -> Result<Option<ManagedAuthAccount>, String> {
    deny_legacy_auth_mutation()
}

#[tauri::command(rename_all = "camelCase")]
pub async fn auth_list_accounts(
    auth_provider: String,
    copilot_state: State<'_, CopilotAuthState>,
    codex_state: State<'_, CodexOAuthState>,
    xai_state: State<'_, XaiOAuthState>,
    managed_auth: State<'_, ManagedAuthState>,
) -> Result<Vec<ManagedAuthAccount>, String> {
    let auth_provider = ensure_auth_provider(&auth_provider)?;
    if let Some(accounts) = vault_accounts(&managed_auth, auth_provider) {
        return Ok(accounts);
    }
    match auth_provider {
        AUTH_PROVIDER_GITHUB_COPILOT => {
            let auth_manager = copilot_state.0.read().await;
            let status = auth_manager.get_status().await;
            let default_account_id = status.default_account_id.clone();
            Ok(status
                .accounts
                .into_iter()
                .map(|account| map_account(auth_provider, account, default_account_id.as_deref()))
                .collect())
        }
        AUTH_PROVIDER_CODEX_OAUTH => {
            let auth_manager = codex_state.0.read().await;
            let status = auth_manager.get_status().await;
            let default_account_id = status.default_account_id.clone();
            Ok(status
                .accounts
                .into_iter()
                .map(|account| map_account(auth_provider, account, default_account_id.as_deref()))
                .collect())
        }
        AUTH_PROVIDER_XAI_OAUTH => {
            let auth_manager = xai_state.0.read().await;
            let status = auth_manager.get_status().await;
            let default_account_id = status.default_account_id.clone();
            Ok(status
                .accounts
                .into_iter()
                .map(|account| map_xai_account(account, default_account_id.as_deref()))
                .collect())
        }
        _ => unreachable!(),
    }
}

#[tauri::command(rename_all = "camelCase")]
pub async fn auth_get_status(
    auth_provider: String,
    copilot_state: State<'_, CopilotAuthState>,
    codex_state: State<'_, CodexOAuthState>,
    xai_state: State<'_, XaiOAuthState>,
    managed_auth: State<'_, ManagedAuthState>,
) -> Result<ManagedAuthStatus, String> {
    let auth_provider = ensure_auth_provider(&auth_provider)?;
    if let Some(accounts) = vault_accounts(&managed_auth, auth_provider) {
        let default_account_id = accounts
            .iter()
            .find(|account| account.is_default)
            .map(|account| account.id.clone());
        return Ok(ManagedAuthStatus {
            provider: auth_provider.to_string(),
            authenticated: accounts.iter().any(|account| !account.requires_reauth),
            default_account_id,
            migration_error: None,
            accounts,
            native_projection_available: (auth_provider == AUTH_PROVIDER_CODEX_OAUTH)
                .then(native_codex_projection_available),
        });
    }
    match auth_provider {
        AUTH_PROVIDER_GITHUB_COPILOT => {
            let auth_manager = copilot_state.0.read().await;
            let status = auth_manager.get_status().await;
            let default_account_id = status.default_account_id.clone();
            Ok(ManagedAuthStatus {
                provider: auth_provider.to_string(),
                authenticated: status.authenticated,
                default_account_id: default_account_id.clone(),
                migration_error: status.migration_error,
                accounts: status
                    .accounts
                    .into_iter()
                    .map(|account| {
                        map_account(auth_provider, account, default_account_id.as_deref())
                    })
                    .collect(),
                native_projection_available: None,
            })
        }
        AUTH_PROVIDER_CODEX_OAUTH => {
            let auth_manager = codex_state.0.read().await;
            let status = auth_manager.get_status().await;
            let default_account_id = status.default_account_id.clone();
            Ok(ManagedAuthStatus {
                provider: auth_provider.to_string(),
                authenticated: status.authenticated,
                default_account_id: default_account_id.clone(),
                migration_error: None,
                accounts: status
                    .accounts
                    .into_iter()
                    .map(|account| {
                        map_account(auth_provider, account, default_account_id.as_deref())
                    })
                    .collect(),
                native_projection_available: Some(native_codex_projection_available()),
            })
        }
        AUTH_PROVIDER_XAI_OAUTH => {
            let auth_manager = xai_state.0.read().await;
            let status = auth_manager.get_status().await;
            let default_account_id = status.default_account_id.clone();
            Ok(ManagedAuthStatus {
                provider: auth_provider.to_string(),
                authenticated: status.authenticated,
                default_account_id: default_account_id.clone(),
                migration_error: None,
                accounts: status
                    .accounts
                    .into_iter()
                    .map(|account| map_xai_account(account, default_account_id.as_deref()))
                    .collect(),
                native_projection_available: None,
            })
        }
        _ => unreachable!(),
    }
}

#[tauri::command(rename_all = "camelCase")]
pub async fn auth_remove_account(
    _auth_provider: String,
    _account_id: String,
) -> Result<(), String> {
    deny_legacy_auth_mutation()
}

#[tauri::command(rename_all = "camelCase")]
pub async fn auth_set_default_account(
    _auth_provider: String,
    _account_id: String,
) -> Result<(), String> {
    deny_legacy_auth_mutation()
}

#[tauri::command(rename_all = "camelCase")]
pub async fn auth_logout(_auth_provider: String) -> Result<(), String> {
    deny_legacy_auth_mutation()
}

fn native_codex_projection_available() -> bool {
    let text =
        std::fs::read_to_string(crate::codex_config::get_codex_config_path()).unwrap_or_default();
    crate::codex_config::native_file_projection_allowed(&text).unwrap_or(false)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn auth_cancel_login(
    _auth_provider: String,
    _device_code: Option<String>,
) -> Result<(), String> {
    deny_legacy_auth_mutation()
}

#[cfg(test)]
mod tests {
    use super::LEGACY_AUTH_MUTATION_DISABLED;

    #[tokio::test]
    async fn leftover_legacy_auth_mutations_are_fail_closed() {
        let start = super::auth_start_login("codex_oauth".into(), None).await;
        let poll =
            super::auth_poll_for_account("codex_oauth".into(), "device-code".into(), None).await;
        let remove = super::auth_remove_account("codex_oauth".into(), "account-1".into()).await;
        let set_default =
            super::auth_set_default_account("codex_oauth".into(), "account-1".into()).await;
        let logout = super::auth_logout("codex_oauth".into()).await;
        let cancel =
            super::auth_cancel_login("codex_oauth".into(), Some("device-code".into())).await;

        for error in [
            start.unwrap_err(),
            poll.unwrap_err(),
            remove.unwrap_err(),
            set_default.unwrap_err(),
            logout.unwrap_err(),
            cancel.unwrap_err(),
        ] {
            assert_eq!(error, LEGACY_AUTH_MUTATION_DISABLED);
            let lower = error.to_ascii_lowercase();
            assert!(!lower.contains("token="));
            assert!(!lower.contains("device_code"));
            assert!(!lower.contains("refresh"));
        }
    }
}
