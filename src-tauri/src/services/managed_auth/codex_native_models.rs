//! The model catalog of the user's own Codex CLI (`codex app-server`
//! `model/list`), attributed to one saved ChatGPT account.
//!
//! The CLI reports whatever its current login can use, so the catalog is only
//! read, and only returned, when the live Codex login is the selected account
//! and the Codex files did not change during the read.

use std::path::Path;

use fyagent_user_helper::CodexModelCatalog;
use serde::Serialize;

use super::consumers::codex::{live_chatgpt_account_id, CodexNativeAuthState};
use super::{CredentialWithIdentity, ManagedAuthProvider, MANAGED_AUTH_CONTRACT_VERSION};

pub const CODEX_NATIVE_MODELS_SOURCE: &str = "codex_app_server";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CodexNativeAccountBinding {
    Matched,
    Mismatch,
    Unknown,
}

/// Why an `unknown` binding could not be decided.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CodexNativeBindingDetail {
    SignedOut,
    /// API key, access token, Bedrock or another non-ChatGPT login.
    OtherLogin,
    Unreadable,
    /// FyAgent manages a Codex home other than the CLI's `~/.codex`.
    CustomCodexHome,
    /// The active store does not establish the CLI login from `auth.json`.
    UnsupportedCredentialStore,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CodexNativeCatalogStatus {
    Ready,
    /// Not read because the binding is not `matched`.
    NotRead,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CodexNativeCatalogFailure {
    PlatformUnsupported,
    /// The ordinary-user helper could not run or did not answer.
    HelperUnavailable,
    /// The helper answered with an older protocol; reinstalling pairs it.
    HelperOutdated,
    CliMissing,
    AppServerFailed,
    TimedOut,
    ModelListUnsupported,
    InvalidResponse,
    ChangedDuringRead,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedAuthCodexNativeModel {
    pub id: String,
    pub display_name: Option<String>,
    pub is_default: bool,
    pub reasoning_efforts: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedAuthCodexNativeModels {
    pub contract_version: u8,
    pub account_id: String,
    pub source: &'static str,
    pub account_binding: CodexNativeAccountBinding,
    pub binding_detail: Option<CodexNativeBindingDetail>,
    /// The saved FyAgent account the live Codex login belongs to, if any.
    pub native_account_id: Option<String>,
    /// Codex connection revision the result was observed under.
    pub revision: String,
    pub checked_at: String,
    pub catalog_status: CodexNativeCatalogStatus,
    pub failure: Option<CodexNativeCatalogFailure>,
    pub cli_version: Option<String>,
    pub truncated: bool,
    pub models: Vec<ManagedAuthCodexNativeModel>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CodexNativeAttribution {
    pub binding: CodexNativeAccountBinding,
    pub detail: Option<CodexNativeBindingDetail>,
    pub native_account_id: Option<String>,
}

/// Compares the live Codex login with the selected account's provider subject.
/// Internal FyAgent account IDs are never compared with provider subjects.
pub(crate) fn attribute(
    selected_subject: &str,
    accounts: &[CredentialWithIdentity],
    auth_state: &CodexNativeAuthState,
    cli_reads_managed_home: bool,
    cli_uses_auth_file: bool,
) -> CodexNativeAttribution {
    let unknown = |detail| CodexNativeAttribution {
        binding: CodexNativeAccountBinding::Unknown,
        detail: Some(detail),
        native_account_id: None,
    };
    if !cli_reads_managed_home {
        return unknown(CodexNativeBindingDetail::CustomCodexHome);
    }
    if !cli_uses_auth_file {
        return unknown(CodexNativeBindingDetail::UnsupportedCredentialStore);
    }
    if let Some(live) = live_chatgpt_account_id(auth_state) {
        let native_account_id = accounts
            .iter()
            .find(|row| {
                row.identity.provider == ManagedAuthProvider::Openai
                    && row.identity.provider_subject == live
            })
            .map(|row| row.identity.identity_id.clone());
        return CodexNativeAttribution {
            binding: if live == selected_subject {
                CodexNativeAccountBinding::Matched
            } else {
                CodexNativeAccountBinding::Mismatch
            },
            detail: None,
            native_account_id,
        };
    }
    unknown(match auth_state {
        CodexNativeAuthState::Missing => CodexNativeBindingDetail::SignedOut,
        CodexNativeAuthState::ThirdPartyApiKeyOnly { .. }
        | CodexNativeAuthState::PersonalAccessToken { .. }
        | CodexNativeAuthState::AgentIdentityOnly { .. }
        | CodexNativeAuthState::Bedrock { .. }
        | CodexNativeAuthState::Unsupported { .. } => CodexNativeBindingDetail::OtherLogin,
        CodexNativeAuthState::ChatGptKnown { .. }
        | CodexNativeAuthState::ChatGptUnmanaged { .. }
        | CodexNativeAuthState::Invalid { .. }
        | CodexNativeAuthState::Unreadable
        | CodexNativeAuthState::Oversized => CodexNativeBindingDetail::Unreadable,
    })
}

/// The helper starts app-server with `CODEX_HOME=<profile>\.codex`, so the
/// catalog only describes FyAgent's Codex home when that is the one managed.
pub(crate) fn same_codex_home(managed: &Path, cli: &Path) -> bool {
    fn normalized(path: &Path) -> String {
        let text = path.to_string_lossy();
        let trimmed = text.trim_end_matches(['/', '\\']);
        #[cfg(target_os = "windows")]
        {
            trimmed.replace('/', "\\").to_lowercase()
        }
        #[cfg(target_os = "macos")]
        {
            trimmed.to_owned()
        }
    }
    normalized(managed) == normalized(cli)
}

#[cfg(target_os = "windows")]
pub(crate) async fn read_codex_catalog() -> Result<CodexModelCatalog, CodexNativeCatalogFailure> {
    tokio::task::spawn_blocking(|| {
        crate::codex_desktop::platform::windows::run_codex_models(
            crate::windows_runtime::require_interactive_user_context(),
        )
        .map_err(|error| {
            failure_from_platform_code(error.to_dto().details.platform_error_code.as_deref())
        })
    })
    .await
    .map_err(|_| CodexNativeCatalogFailure::HelperUnavailable)?
}

#[cfg(target_os = "macos")]
pub(crate) async fn read_codex_catalog() -> Result<CodexModelCatalog, CodexNativeCatalogFailure> {
    Err(CodexNativeCatalogFailure::PlatformUnsupported)
}

/// Maps the helper's redacted platform code; anything else means the helper
/// itself did not complete the exchange.
#[cfg(any(target_os = "windows", test))]
pub(crate) fn failure_from_platform_code(code: Option<&str>) -> CodexNativeCatalogFailure {
    match code {
        Some("codex_cli_missing") => CodexNativeCatalogFailure::CliMissing,
        Some("codex_app_server_failed") => CodexNativeCatalogFailure::AppServerFailed,
        Some("codex_app_server_timed_out") => CodexNativeCatalogFailure::TimedOut,
        Some("codex_model_list_unsupported") => CodexNativeCatalogFailure::ModelListUnsupported,
        Some("codex_catalog_invalid") => CodexNativeCatalogFailure::InvalidResponse,
        Some("user_helper_version_mismatch") => CodexNativeCatalogFailure::HelperOutdated,
        _ => CodexNativeCatalogFailure::HelperUnavailable,
    }
}

impl ManagedAuthCodexNativeModels {
    pub(crate) fn not_read(
        account_id: &str,
        attribution: CodexNativeAttribution,
        revision: String,
        checked_at: String,
    ) -> Self {
        Self {
            contract_version: MANAGED_AUTH_CONTRACT_VERSION,
            account_id: account_id.to_owned(),
            source: CODEX_NATIVE_MODELS_SOURCE,
            account_binding: attribution.binding,
            binding_detail: attribution.detail,
            native_account_id: attribution.native_account_id,
            revision,
            checked_at,
            catalog_status: CodexNativeCatalogStatus::NotRead,
            failure: None,
            cli_version: None,
            truncated: false,
            models: Vec::new(),
        }
    }

    pub(crate) fn settle(
        mut self,
        read: Result<CodexModelCatalog, CodexNativeCatalogFailure>,
        checked_at: String,
    ) -> Self {
        self.checked_at = checked_at;
        match read {
            Ok(catalog) => {
                self.catalog_status = CodexNativeCatalogStatus::Ready;
                self.cli_version = catalog.cli_version;
                self.truncated = catalog.truncated;
                self.models = catalog
                    .models
                    .into_iter()
                    .map(|model| ManagedAuthCodexNativeModel {
                        id: model.id,
                        display_name: model.display_name,
                        is_default: model.is_default,
                        reasoning_efforts: model.reasoning_efforts,
                    })
                    .collect();
            }
            Err(failure) => {
                self.catalog_status = CodexNativeCatalogStatus::Failed;
                self.failure = Some(failure);
            }
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helper_codes_map_to_actionable_failures() {
        for (code, failure) in [
            ("codex_cli_missing", CodexNativeCatalogFailure::CliMissing),
            (
                "codex_app_server_failed",
                CodexNativeCatalogFailure::AppServerFailed,
            ),
            (
                "codex_app_server_timed_out",
                CodexNativeCatalogFailure::TimedOut,
            ),
            (
                "codex_model_list_unsupported",
                CodexNativeCatalogFailure::ModelListUnsupported,
            ),
            (
                "codex_catalog_invalid",
                CodexNativeCatalogFailure::InvalidResponse,
            ),
            (
                "user_helper_version_mismatch",
                CodexNativeCatalogFailure::HelperOutdated,
            ),
            (
                "user_helper_launch_pending",
                CodexNativeCatalogFailure::HelperUnavailable,
            ),
        ] {
            assert_eq!(failure_from_platform_code(Some(code)), failure, "{code}");
        }
        assert_eq!(
            failure_from_platform_code(None),
            CodexNativeCatalogFailure::HelperUnavailable
        );
    }

    #[test]
    fn codex_home_comparison_ignores_trailing_separators_only() {
        let home = std::env::temp_dir().join("me").join(".codex");
        let trailing = std::path::PathBuf::from(format!("{}/", home.display()));
        assert!(same_codex_home(&home, &trailing));
        assert!(!same_codex_home(&home, &home.with_file_name(".codex-alt")));
    }
}
