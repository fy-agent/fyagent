//! Live Codex home observation for Managed Auth projection.

use std::io::Read;
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::codex_config::is_custom_codex_model_provider_id;
use crate::codex_config::{parse_cli_auth_credentials_store, CodexCredentialStore};
use crate::services::managed_auth::{
    providers::openai, ManagedAuthRequestMode, ManagedAuthSecretBundle,
};

use super::auth_document::{
    classify_auth_bytes, CodexChatGptAuthDocument, CodexNativeAuthState, MAX_AUTH_JSON_BYTES,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CodexProviderRoute {
    Official,
    ThirdParty,
    Unknown,
    Invalid,
}

impl CodexProviderRoute {
    pub(crate) const fn is_official(self) -> bool {
        matches!(self, Self::Official)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct CodexManagedAuthObservation {
    #[allow(dead_code)]
    pub config_revision: String,
    pub auth_revision: Option<String>,
    pub effective_store: CodexCredentialStore,
    pub provider_route: CodexProviderRoute,
    pub request_mode: ManagedAuthRequestMode,
    pub request_provider_label: Option<String>,
    pub auth_state: CodexNativeAuthState,
    /// Conservative: true when Desktop hot-reload is unproven after a write.
    #[allow(dead_code)]
    pub may_need_restart: bool,
}

impl Default for CodexManagedAuthObservation {
    fn default() -> Self {
        Self {
            config_revision: revision_for_bytes(b""),
            auth_revision: None,
            effective_store: CodexCredentialStore::Unset,
            provider_route: CodexProviderRoute::Official,
            request_mode: ManagedAuthRequestMode::OfficialSubscription,
            request_provider_label: Some("openai".to_string()),
            auth_state: CodexNativeAuthState::Missing,
            may_need_restart: false,
        }
    }
}

/// Single-shot observation of Codex config + auth used by overview, delta, and
/// projection. Missing `model_provider` is treated as official openai.
pub(crate) fn observe_managed_auth(codex_home: &Path) -> CodexManagedAuthObservation {
    let config_path = codex_home.join("config.toml");
    let auth_path = codex_home.join("auth.json");

    let (config_text, config_ok) = match std::fs::read_to_string(&config_path) {
        Ok(text) => (text, true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => (String::new(), false),
        Err(_) => {
            return CodexManagedAuthObservation {
                config_revision: revision_for_bytes(b"unreadable-config"),
                auth_revision: None,
                effective_store: CodexCredentialStore::Unknown,
                provider_route: CodexProviderRoute::Invalid,
                request_mode: ManagedAuthRequestMode::Unknown,
                request_provider_label: None,
                auth_state: observe_auth_file(&auth_path).1,
                may_need_restart: false,
            };
        }
    };

    let effective_store = if !config_ok && config_text.is_empty() {
        CodexCredentialStore::Unset
    } else {
        parse_cli_auth_credentials_store(&config_text).unwrap_or(CodexCredentialStore::Unknown)
    };

    let (provider_route, request_mode, request_provider_label) =
        classify_provider_route(&config_text, config_ok);

    let (auth_revision, auth_state) = observe_auth_file(&auth_path);

    CodexManagedAuthObservation {
        config_revision: revision_for_bytes(config_text.as_bytes()),
        auth_revision,
        effective_store,
        provider_route,
        request_mode,
        request_provider_label,
        auth_state,
        may_need_restart: false,
    }
}

fn classify_provider_route(
    config_toml: &str,
    config_present: bool,
) -> (CodexProviderRoute, ManagedAuthRequestMode, Option<String>) {
    if !config_present && config_toml.is_empty() {
        return (
            CodexProviderRoute::Official,
            ManagedAuthRequestMode::OfficialSubscription,
            Some("openai".to_string()),
        );
    }
    let Ok(document) = config_toml.parse::<toml_edit::DocumentMut>() else {
        return (
            CodexProviderRoute::Invalid,
            ManagedAuthRequestMode::Unknown,
            None,
        );
    };
    let Some(item) = document.get("model_provider") else {
        return (
            CodexProviderRoute::Official,
            ManagedAuthRequestMode::OfficialSubscription,
            Some("openai".to_string()),
        );
    };
    let Some(id) = item.as_str().filter(|value| !value.is_empty()) else {
        return (
            CodexProviderRoute::Unknown,
            ManagedAuthRequestMode::Unknown,
            None,
        );
    };
    if !is_custom_codex_model_provider_id(id) {
        (
            CodexProviderRoute::Official,
            ManagedAuthRequestMode::OfficialSubscription,
            Some(id.to_string()),
        )
    } else {
        (
            CodexProviderRoute::ThirdParty,
            ManagedAuthRequestMode::ThirdPartyApi,
            Some(id.to_string()),
        )
    }
}

fn observe_auth_file(path: &Path) -> (Option<String>, CodexNativeAuthState) {
    match read_bounded_auth_bytes(path) {
        Ok(None) => (None, CodexNativeAuthState::Missing),
        Ok(Some(bytes)) => {
            let revision = revision_for_bytes(&bytes);
            (
                Some(revision.clone()),
                classify_auth_bytes(&bytes, revision),
            )
        }
        Err(AuthReadError::Oversized) => (None, CodexNativeAuthState::Oversized),
        Err(AuthReadError::Io) => (None, CodexNativeAuthState::Unreadable),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AuthReadError {
    Io,
    Oversized,
}

fn read_bounded_auth_bytes(path: &Path) -> Result<Option<Vec<u8>>, AuthReadError> {
    let file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(AuthReadError::Io),
    };
    let mut bytes = Vec::new();
    let limit = u64::try_from(MAX_AUTH_JSON_BYTES.saturating_add(1)).expect("auth limit");
    file.take(limit)
        .read_to_end(&mut bytes)
        .map_err(|_| AuthReadError::Io)?;
    if bytes.len() > MAX_AUTH_JSON_BYTES {
        return Err(AuthReadError::Oversized);
    }
    Ok(Some(bytes))
}

pub(crate) fn revision_for_bytes(bytes: &[u8]) -> String {
    use crate::services::managed_auth::stable_revision;
    let digest = Sha256::digest(bytes);
    let hex: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
    stable_revision(&["codex-auth-json", &hex])
}

pub(crate) fn live_chatgpt_account_id(state: &CodexNativeAuthState) -> Option<&str> {
    match state {
        CodexNativeAuthState::ChatGptKnown { account_id, .. } => Some(account_id.as_str()),
        _ => None,
    }
}

pub(crate) fn auth_matches_account(state: &CodexNativeAuthState, provider_subject: &str) -> bool {
    live_chatgpt_account_id(state) == Some(provider_subject)
}

/// Build a ChatGPT auth document from a SecretRef bundle when tokens are complete.
pub(crate) fn document_from_bundle(
    bundle: &ManagedAuthSecretBundle,
    expected_subject: &str,
) -> Option<CodexChatGptAuthDocument> {
    let access = bundle.access_token()?;
    let refresh = bundle.refresh_token()?;
    let id_token = bundle.id_token()?;
    let grant = openai::OpenAiTokenGrant {
        access_token: access.to_string(),
        refresh_token: Some(refresh.to_string()),
        id_token: Some(id_token.to_string()),
        expires_in: None,
    };
    let identity = openai::extract_identity(&grant).ok()?;
    if identity.subject != expected_subject {
        return None;
    }
    CodexChatGptAuthDocument::from_tokens(
        id_token,
        access,
        refresh,
        Some(identity.subject.as_str()),
        Some(chrono::Utc::now().timestamp()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn missing_model_provider_is_official_openai() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("config.toml"), "model = \"gpt-5\"\n").unwrap();
        let observed = observe_managed_auth(dir.path());
        assert_eq!(observed.provider_route, CodexProviderRoute::Official);
        assert_eq!(
            observed.request_mode,
            ManagedAuthRequestMode::OfficialSubscription
        );
        assert_eq!(observed.request_provider_label.as_deref(), Some("openai"));
        assert!(observed.effective_store.allows_native_file_projection());
    }

    #[test]
    fn missing_config_defaults_to_official_file_store() {
        let dir = tempdir().unwrap();
        let observed = observe_managed_auth(dir.path());
        assert_eq!(observed.provider_route, CodexProviderRoute::Official);
        assert_eq!(observed.effective_store, CodexCredentialStore::Unset);
        assert!(observed.effective_store.allows_native_file_projection());
        assert_eq!(observed.auth_state, CodexNativeAuthState::Missing);
    }

    #[test]
    fn third_party_route_is_detected() {
        let dir = tempdir().unwrap();
        std::fs::write(
            dir.path().join("config.toml"),
            "model_provider = \"deepseek\"\ncli_auth_credentials_store = \"file\"\n",
        )
        .unwrap();
        let observed = observe_managed_auth(dir.path());
        assert_eq!(observed.provider_route, CodexProviderRoute::ThirdParty);
        assert_eq!(observed.request_mode, ManagedAuthRequestMode::ThirdPartyApi);
    }
}
