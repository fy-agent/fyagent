//! Grok subscription bindings reuse the Provider transaction and local Proxy.

use super::*;
use crate::provider::{AuthBinding, AuthBindingSource, ProviderMeta};
use crate::services::managed_auth::{
    CredentialPurpose, CredentialStatus, IdentityRecord, ManagedAuthConsumer, ManagedAuthProvider,
    ManagedAuthRepository, ManagedAuthService, RefreshOwner,
};
use crate::services::secret::SecretBackend;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BindXaiManagedRequest {
    pub app: String,
    pub account_id: String,
    pub model_id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BindXaiManagedResult {
    pub provider_id: String,
    pub provider_name: String,
    pub app: String,
    pub already_bound: bool,
    pub activated: bool,
}

#[derive(Debug, Clone, Copy, Serialize, thiserror::Error)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum BindXaiManagedError {
    #[error("invalid_request")]
    InvalidRequest,
    #[error("account_unavailable")]
    AccountUnavailable,
    #[error("provider_conflict")]
    ProviderConflict,
    #[error("apply_failed_rolled_back")]
    ApplyFailedRolledBack,
    #[error("rollback_partial_state_unknown")]
    RollbackPartialStateUnknown,
}

impl From<QuickSetupApplyError> for BindXaiManagedError {
    fn from(error: QuickSetupApplyError) -> Self {
        match error.code {
            QuickSetupApplyFailureCode::ApplyFailedRolledBack => Self::ApplyFailedRolledBack,
            QuickSetupApplyFailureCode::RollbackPartialStateUnknown => {
                Self::RollbackPartialStateUnknown
            }
        }
    }
}

fn parse_request(request: &BindXaiManagedRequest) -> Result<AppType, BindXaiManagedError> {
    let app = match request.app.as_str() {
        "claude" => AppType::Claude,
        "codex" => AppType::Codex,
        "claude-desktop" => AppType::ClaudeDesktop,
        _ => return Err(BindXaiManagedError::InvalidRequest),
    };
    let model = request.model_id.as_bytes();
    if request.account_id.is_empty()
        || request.account_id.len() > 160
        || request.account_id.trim() != request.account_id
        || model.is_empty()
        || model.len() > 128
        || !model[0].is_ascii_alphanumeric()
        || !model
            .iter()
            .all(|c| c.is_ascii_alphanumeric() || b"-_.:".contains(c))
    {
        return Err(BindXaiManagedError::InvalidRequest);
    }
    Ok(app)
}

fn build_provider(
    app: &AppType,
    account: &str,
    identity: &IdentityRecord,
    model: &str,
) -> Provider {
    // Model-specific drafts avoid mutating an already active binding when a
    // second target/model is selected. No user-supplied text becomes an ID.
    let digest = format!(
        "{:x}",
        Sha256::digest(format!("{account}\0{model}").as_bytes())
    );
    let id = format!("fyagent-xai-{}-{}", app.as_str(), &digest[..24]);
    let base_url = crate::proxy::providers::XAI_SUBSCRIPTION_BASE_URL;
    let settings = if *app == AppType::Codex {
        let config = format!("model_provider = \"xai\"\nmodel = \"{model}\"\n\n[model_providers.xai]\nname = \"Grok subscription\"\nbase_url = \"{base_url}\"\nwire_api = \"responses\"\n");
        serde_json::json!({"auth": {}, "config": config})
    } else {
        serde_json::json!({"env": {
            "ANTHROPIC_BASE_URL": base_url,
            "ANTHROPIC_MODEL": model,
            "ANTHROPIC_DEFAULT_HAIKU_MODEL": model,
            "ANTHROPIC_DEFAULT_SONNET_MODEL": model,
            "ANTHROPIC_DEFAULT_OPUS_MODEL": model
        }})
    };
    let label: String = identity
        .display_name
        .as_deref()
        .filter(|name| !name.trim().is_empty())
        .unwrap_or(&identity.login)
        .chars()
        .filter(|c| c.is_alphanumeric() || " ._@-".contains(*c))
        .take(24)
        .collect();
    let label = if label.trim().is_empty() {
        "账号"
    } else {
        label.trim()
    };
    let account_tag = format!("{:x}", Sha256::digest(identity.identity_id.as_bytes()));
    let mut provider = Provider::with_id(
        id,
        format!("Grok · {label} ({}) · {model}", &account_tag[..6]),
        settings,
        Some("https://x.ai/grok".into()),
    );
    provider.category = Some("third_party".into());
    provider.icon = Some("xai".into());
    provider.meta = Some(ProviderMeta {
        provider_type: Some("xai_oauth".into()),
        auth_binding: Some(AuthBinding {
            source: AuthBindingSource::ManagedAccount,
            auth_provider: Some("xai_oauth".into()),
            account_id: Some(account.into()),
        }),
        ..Default::default()
    });
    if *app == AppType::ClaudeDesktop {
        let meta = provider.meta.as_mut().expect("created metadata");
        meta.claude_desktop_mode = Some(crate::provider::ClaudeDesktopMode::Proxy);
        for route in crate::claude_desktop_config::DEFAULT_PROXY_ROUTES {
            meta.claude_desktop_model_routes.insert(
                route.route_id.into(),
                crate::provider::ClaudeDesktopModelRoute {
                    model: model.into(),
                    supports_1m: Some(false),
                    label_override: None,
                },
            );
        }
    }
    provider
}

impl ProviderService {
    pub(crate) fn xai_managed_codex_shape_is_valid(provider: &Provider) -> bool {
        let Some(auth) = provider
            .settings_config
            .get("auth")
            .and_then(Value::as_object)
        else {
            return false;
        };
        let Some(config) = provider
            .settings_config
            .get("config")
            .and_then(Value::as_str)
            .and_then(|text| text.parse::<toml::Table>().ok())
        else {
            return false;
        };
        let Some(model) = config.get("model").and_then(toml::Value::as_str) else {
            return false;
        };
        let Some(selected) = config
            .get("model_providers")
            .and_then(toml::Value::as_table)
            .and_then(|table| table.get("xai"))
            .and_then(toml::Value::as_table)
        else {
            return false;
        };
        auth.is_empty()
            && config.get("model_provider").and_then(toml::Value::as_str) == Some("xai")
            && selected.get("base_url").and_then(toml::Value::as_str)
                == Some(crate::proxy::providers::XAI_SUBSCRIPTION_BASE_URL)
            && selected.get("wire_api").and_then(toml::Value::as_str) == Some("responses")
            && selected.keys().all(|key| {
                matches!(
                    key.as_str(),
                    "name" | "base_url" | "wire_api" | "http_headers"
                )
            })
            && selected.get("http_headers").is_none_or(|headers| {
                headers.as_table().is_some_and(|headers| {
                    headers.len() == 1
                        && headers
                            .get(crate::codex_config::CODEX_IMAGE_EXTENSION_HEADER)
                            .and_then(toml::Value::as_str)
                            == Some(crate::codex_config::CODEX_IMAGE_EXTENSION_VALUE)
                })
            })
            && parse_request(&BindXaiManagedRequest {
                app: "codex".into(),
                account_id: "validation".into(),
                model_id: model.into(),
            })
            .is_ok()
    }

    pub(crate) fn xai_managed_account_is_ready(state: &AppState, provider: &Provider) -> bool {
        let Some(binding) = provider
            .meta
            .as_ref()
            .and_then(|meta| meta.auth_binding.as_ref())
        else {
            return false;
        };
        if !provider.is_xai_oauth()
            || binding.source != AuthBindingSource::ManagedAccount
            || binding.auth_provider.as_deref() != Some("xai_oauth")
        {
            return false;
        }
        let Some(account) = binding.account_id.as_deref().filter(|id| !id.is_empty()) else {
            return false;
        };
        ManagedAuthRepository::new(state.db.clone())
            .get_credential_by_legacy(
                ManagedAuthProvider::Xai,
                CredentialPurpose::ProxyUpstream,
                Some(ManagedAuthConsumer::FyagentProxy),
                account,
            )
            .ok()
            .flatten()
            .is_some_and(|credential| {
                credential.status == CredentialStatus::Ready
                    && credential.refresh_owner == RefreshOwner::Fyagent
            })
    }

    pub(crate) fn bind_xai_managed<B: SecretBackend>(
        state: &AppState,
        auth: &ManagedAuthService<B>,
        request: BindXaiManagedRequest,
    ) -> Result<BindXaiManagedResult, BindXaiManagedError> {
        let app = parse_request(&request)?;
        let _guard = Self::lock_provider_mutation(state, &app);
        let credential = auth
            .xai_proxy_account(&request.account_id)
            .map_err(|_| BindXaiManagedError::AccountUnavailable)?;
        let mut provider = build_provider(
            &app,
            &credential.credential.legacy_account_id,
            &credential.identity,
            &request.model_id,
        );
        Self::normalize_provider_if_claude(&app, &mut provider);
        if app == AppType::Codex {
            crate::codex_config::prepare_codex_provider_features_for_save(&mut provider, true)
                .map_err(|_| BindXaiManagedError::InvalidRequest)?;
        }
        normalize_provider_common_config_for_storage(state.db.as_ref(), &app, &mut provider)
            .map_err(|_| BindXaiManagedError::InvalidRequest)?;
        let existing = state
            .db
            .get_provider_by_id(&provider.id, app.as_str())
            .map_err(|_| BindXaiManagedError::ApplyFailedRolledBack)?;
        if let Some(existing) = &existing {
            // Names are presentation, not binding authority. Preserve a saved
            // name when public account labels or the user's own label change.
            provider.name = existing.name.clone();
            if !Self::quick_setup_persisted_provider_matches(&provider, existing).unwrap_or(false) {
                return Err(BindXaiManagedError::ProviderConflict);
            }
        }
        let result = BindXaiManagedResult {
            provider_id: provider.id.clone(),
            provider_name: provider.name.clone(),
            app: app.as_str().into(),
            already_bound: existing.is_some(),
            activated: app == AppType::Claude,
        };
        if app == AppType::Claude {
            Self::apply_quick_setup_locked(state, app, provider)?;
        } else if existing.is_none() {
            // Codex keeps the existing Change Plan confirmation. Desktop keeps
            // its dedicated profile application; this command only saves it.
            let current_before = state
                .db
                .get_current_provider(app.as_str())
                .map_err(|_| BindXaiManagedError::ApplyFailedRolledBack)?;
            let saved = Self::add_with_initial_activation(
                state,
                app.clone(),
                provider.clone(),
                false,
                false,
            );
            let verified = saved.is_ok()
                && state
                    .db
                    .get_provider_by_id(&provider.id, app.as_str())
                    .ok()
                    .flatten()
                    .as_ref()
                    .is_some_and(|row| {
                        Self::quick_setup_persisted_provider_matches(&provider, row)
                            .unwrap_or(false)
                    })
                && state
                    .db
                    .get_current_provider(app.as_str())
                    .is_ok_and(|current| current == current_before);
            if !verified {
                state
                    .db
                    .delete_provider(app.as_str(), &provider.id)
                    .map_err(|_| BindXaiManagedError::RollbackPartialStateUnknown)?;
                match current_before.as_deref() {
                    Some(id) => state.db.set_current_provider(app.as_str(), id),
                    None => state.db.clear_current_provider(app.as_str()),
                }
                .map_err(|_| BindXaiManagedError::RollbackPartialStateUnknown)?;
                if state
                    .db
                    .get_provider_by_id(&provider.id, app.as_str())
                    .map_err(|_| BindXaiManagedError::RollbackPartialStateUnknown)?
                    .is_some()
                    || state
                        .db
                        .get_current_provider(app.as_str())
                        .map_err(|_| BindXaiManagedError::RollbackPartialStateUnknown)?
                        != current_before
                {
                    return Err(BindXaiManagedError::RollbackPartialStateUnknown);
                }
                return Err(BindXaiManagedError::ApplyFailedRolledBack);
            }
        }
        Ok(result)
    }
}
