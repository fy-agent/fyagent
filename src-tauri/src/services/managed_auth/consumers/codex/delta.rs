//! Pure delta planner for Codex Managed Auth projection.

use crate::services::managed_auth::ManagedAuthReasonCode;

use super::auth_document::CodexNativeAuthState;
use super::observation::{auth_matches_account, CodexManagedAuthObservation, CodexProviderRoute};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CodexManagedAuthDelta {
    Noop,
    AuthOnly,
    ProviderOnly,
    AuthThenProvider,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CodexDeltaError {
    StoreUnsupported,
    AuthUnreadable,
    AuthInvalid,
    AuthOversized,
    UnmanagedAccount,
    UnsupportedAuthMode,
    RouteInvalid,
}

impl CodexDeltaError {
    pub(crate) fn reason_code(self) -> ManagedAuthReasonCode {
        match self {
            Self::StoreUnsupported => ManagedAuthReasonCode::NativeProjectionUnavailable,
            Self::AuthUnreadable | Self::AuthInvalid | Self::AuthOversized => {
                ManagedAuthReasonCode::ExternalChangeDetected
            }
            Self::UnmanagedAccount | Self::UnsupportedAuthMode => {
                ManagedAuthReasonCode::ExternalChangeDetected
            }
            Self::RouteInvalid => ManagedAuthReasonCode::ConnectionUnavailable,
        }
    }
}

/// Plan the minimum write set for projecting `target_provider_subject`.
pub(crate) fn plan_codex_managed_auth_delta(
    live: &CodexManagedAuthObservation,
    target_provider_subject: &str,
) -> Result<CodexManagedAuthDelta, CodexDeltaError> {
    if !live.effective_store.allows_native_file_projection() {
        return Err(CodexDeltaError::StoreUnsupported);
    }
    match live.provider_route {
        CodexProviderRoute::Invalid => return Err(CodexDeltaError::RouteInvalid),
        CodexProviderRoute::Unknown => return Err(CodexDeltaError::RouteInvalid),
        CodexProviderRoute::Official | CodexProviderRoute::ThirdParty => {}
    }
    match &live.auth_state {
        CodexNativeAuthState::Unreadable => return Err(CodexDeltaError::AuthUnreadable),
        CodexNativeAuthState::Oversized => return Err(CodexDeltaError::AuthOversized),
        CodexNativeAuthState::Invalid { .. } => return Err(CodexDeltaError::AuthInvalid),
        CodexNativeAuthState::ChatGptUnmanaged { .. } => {
            return Err(CodexDeltaError::UnmanagedAccount);
        }
        CodexNativeAuthState::PersonalAccessToken { .. }
        | CodexNativeAuthState::AgentIdentityOnly { .. }
        | CodexNativeAuthState::Bedrock { .. }
        | CodexNativeAuthState::Unsupported { .. } => {
            return Err(CodexDeltaError::UnsupportedAuthMode);
        }
        CodexNativeAuthState::Missing
        | CodexNativeAuthState::ChatGptKnown { .. }
        | CodexNativeAuthState::ThirdPartyApiKeyOnly { .. } => {}
    }

    let account_matches = auth_matches_account(&live.auth_state, target_provider_subject);
    let route_official = live.provider_route.is_official();

    Ok(match (account_matches, route_official) {
        (true, true) => CodexManagedAuthDelta::Noop,
        (false, true) => CodexManagedAuthDelta::AuthOnly,
        (true, false) => CodexManagedAuthDelta::ProviderOnly,
        (false, false) => CodexManagedAuthDelta::AuthThenProvider,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codex_config::CodexCredentialStore;
    use crate::services::managed_auth::ManagedAuthRequestMode;

    fn live(account: Option<&str>, official: bool) -> CodexManagedAuthObservation {
        CodexManagedAuthObservation {
            config_revision: "mr1:cfg".into(),
            auth_revision: Some("mr1:auth".into()),
            effective_store: CodexCredentialStore::File,
            provider_route: if official {
                CodexProviderRoute::Official
            } else {
                CodexProviderRoute::ThirdParty
            },
            request_mode: if official {
                ManagedAuthRequestMode::OfficialSubscription
            } else {
                ManagedAuthRequestMode::ThirdPartyApi
            },
            request_provider_label: Some(if official {
                "openai".into()
            } else {
                "deepseek".into()
            }),
            auth_state: match account {
                None => CodexNativeAuthState::Missing,
                Some(id) => CodexNativeAuthState::ChatGptKnown {
                    account_id: id.to_string(),
                    revision: "mr1:auth".into(),
                },
            },
            may_need_restart: false,
        }
    }

    #[test]
    fn delta_matrix_covers_four_branches() {
        assert_eq!(
            plan_codex_managed_auth_delta(&live(Some("A"), true), "A").unwrap(),
            CodexManagedAuthDelta::Noop
        );
        assert_eq!(
            plan_codex_managed_auth_delta(&live(Some("A"), true), "B").unwrap(),
            CodexManagedAuthDelta::AuthOnly
        );
        assert_eq!(
            plan_codex_managed_auth_delta(&live(Some("A"), false), "A").unwrap(),
            CodexManagedAuthDelta::ProviderOnly
        );
        assert_eq!(
            plan_codex_managed_auth_delta(&live(Some("A"), false), "B").unwrap(),
            CodexManagedAuthDelta::AuthThenProvider
        );
        assert_eq!(
            plan_codex_managed_auth_delta(&live(None, false), "B").unwrap(),
            CodexManagedAuthDelta::AuthThenProvider
        );
    }

    #[test]
    fn unsupported_store_fails_closed() {
        let mut observation = live(Some("A"), true);
        observation.effective_store = CodexCredentialStore::Keyring;
        assert_eq!(
            plan_codex_managed_auth_delta(&observation, "A"),
            Err(CodexDeltaError::StoreUnsupported)
        );
    }
}
