//! One native owner for Codex API-key persistence. The public Database save
//! method is a compatibility facade; its SQL implementation stays in the DAO.
//! No SQLite lock is held while calling the credential backend.
use std::collections::HashSet;

use serde_json::{json, Value};
use subtle::ConstantTimeEq;
use zeroize::Zeroizing;

use crate::database::{Database, ProviderCredentialRecord};
use crate::error::AppError;
use crate::provider::{Provider, UsageScript};
use crate::services::secret::{
    DecodeSecret, SecretErrorCode, SecretMaterial, SecretPurpose, SecretServiceError,
};

mod material;
use material::CredentialMaterial;

const REF_KEY: &str = "credentialRef";
const PURPOSE: SecretPurpose = SecretPurpose::CodexApiKey;

pub(crate) struct ProviderCredentials;

pub(crate) struct AdmittedUsageTest {
    pub api_key: String,
    pub base_url: String,
    pub access_token: Option<String>,
    pub user_id: Option<String>,
    pub template_type: Option<String>,
}

pub(crate) struct UsageTestInput<'a> {
    pub script_code: &'a str,
    pub api_key: Option<&'a str>,
    pub base_url: Option<&'a str>,
    pub access_token: Option<&'a str>,
    pub user_id: Option<&'a str>,
    pub template_type: Option<&'a str>,
}

fn secret_error(error: SecretServiceError) -> AppError {
    let code = match error.code() {
        SecretErrorCode::Locked => "provider_secret_locked",
        SecretErrorCode::PermissionDenied => "provider_secret_denied",
        SecretErrorCode::Missing => "provider_secret_missing",
        SecretErrorCode::BackendUnavailable => "provider_secret_unavailable",
        _ => "provider_secret_operation_failed",
    };
    AppError::Message(code.into())
}

fn invalid() -> AppError {
    AppError::Message("provider_secret_invalid".into())
}

fn binding(provider: &Provider) -> Option<&str> {
    provider
        .settings_config
        .get(REF_KEY)
        .and_then(Value::as_str)
}

fn key(provider: &Provider) -> Option<Zeroizing<String>> {
    let settings = &provider.settings_config;
    settings
        .pointer("/env/OPENAI_API_KEY")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(|value| value.trim().to_owned())
        .or_else(|| {
            crate::codex_config::extract_codex_api_key(
                settings.get("auth"),
                settings.get("config").and_then(Value::as_str),
            )
        })
        .or_else(|| {
            settings
                .get("apiKey")
                .or_else(|| settings.get("api_key"))
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .map(Zeroizing::new)
}

fn blank_or_masked(value: &str) -> bool {
    let value = value.trim();
    value.is_empty()
        || value == "[REDACTED]"
        || value.chars().all(|ch| matches!(ch, '*' | '•' | '●'))
}

fn inherit_usage_field(caller: Option<&str>, stored: Option<&String>) -> Option<String> {
    caller
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .or_else(|| stored.cloned())
}

fn requested_usage_script(
    stored: &Provider,
    script_code: &str,
    base_url: Option<&str>,
    user_id: Option<&str>,
    template_type: Option<&str>,
) -> UsageScript {
    let stored_script = stored
        .meta
        .as_ref()
        .and_then(|meta| meta.usage_script.as_ref());
    UsageScript {
        enabled: true,
        language: stored_script
            .map(|script| script.language.clone())
            .unwrap_or_else(|| "javascript".into()),
        code: script_code.to_owned(),
        timeout: None,
        api_key: None,
        base_url: inherit_usage_field(
            base_url,
            stored_script.and_then(|script| script.base_url.as_ref()),
        ),
        access_token: None,
        user_id: inherit_usage_field(
            user_id,
            stored_script.and_then(|script| script.user_id.as_ref()),
        ),
        template_type: inherit_usage_field(
            template_type,
            stored_script.and_then(|script| script.template_type.as_ref()),
        ),
        auto_query_interval: None,
        coding_plan_provider: stored_script.and_then(|script| script.coding_plan_provider.clone()),
        access_key_id: None,
        secret_access_key: None,
        team_organization_id: stored_script.and_then(|script| script.team_organization_id.clone()),
        team_project_id: stored_script.and_then(|script| script.team_project_id.clone()),
    }
}

fn is_api_provider(provider: &Provider, app: &str) -> bool {
    app == "codex"
        && !provider.uses_subscription_proxy()
        && (key(provider).is_some()
            || binding(provider).is_some()
            || (provider.category.as_deref() != Some("official")
                && provider.id != "codex-official"))
}

// Strip every known representation of the same key. Unrelated TOML formatting
// remains owned by toml_edit. Conflicting inline credentials fail closed rather
// than silently changing which account a legacy provider uses.
fn strip_key(provider: &mut Provider, secret: &str) -> Result<(), AppError> {
    let settings = provider
        .settings_config
        .as_object_mut()
        .ok_or_else(invalid)?;
    // Different inline accounts cannot silently collapse into one reference.
    for candidate in [
        settings.get("apiKey"),
        settings.get("api_key"),
        settings.get("auth").and_then(|v| v.get("OPENAI_API_KEY")),
        settings.get("env").and_then(|v| v.get("OPENAI_API_KEY")),
    ]
    .into_iter()
    .flatten()
    .filter_map(Value::as_str)
    {
        if !blank_or_masked(candidate) && candidate.trim() != secret.trim() {
            return Err(invalid());
        }
    }
    for name in ["apiKey", "api_key"] {
        settings.remove(name);
    }
    for (container, name) in [("auth", "OPENAI_API_KEY"), ("env", "OPENAI_API_KEY")] {
        if let Some(object) = settings.get_mut(container).and_then(Value::as_object_mut) {
            object.remove(name);
        }
    }
    settings.entry("auth").or_insert_with(|| json!({}));
    if let Some(config) = settings.get("config").and_then(Value::as_str) {
        let stripped =
            crate::codex_config::remove_codex_experimental_bearer_token_if(config, |value| {
                value == secret || blank_or_masked(value)
            })
            .map_err(|_| invalid())?;
        settings.insert("config".into(), Value::String(stripped));
    }
    fn has_material(value: &Value) -> bool {
        match value {
            Value::String(s) => !s.is_empty(),
            Value::Object(m) => m.values().any(has_material),
            Value::Array(a) => a.iter().any(has_material),
            _ => false,
        }
    }
    fn has_other_secret(value: &Value) -> bool {
        match value {
            Value::Object(m) => m.iter().any(|(name, value)| {
                // This exact built-in feature marker carries no credential.
                // Other header values still fail closed, including a changed
                // value under the same header name or an additional header.
                if name == "http_headers"
                    && value.as_object().is_some_and(|headers| {
                        headers.len() == 1
                            && headers.iter().all(|(header, value)| {
                                header.eq_ignore_ascii_case(
                                    crate::codex_config::CODEX_IMAGE_EXTENSION_HEADER,
                                ) && value.as_str()
                                    == Some(crate::codex_config::CODEX_IMAGE_EXTENSION_VALUE)
                            })
                    })
                {
                    return false;
                }
                name != REF_KEY
                    && ((crate::provider::is_sensitive_config_key(name) && has_material(value))
                        || has_other_secret(value))
            }),
            Value::Array(a) => a.iter().any(has_other_secret),
            _ => false,
        }
    }
    // Unsupported extra credentials remain legacy on migration failure. They
    // must not survive a supposedly reference-only successful save.
    if has_other_secret(&provider.settings_config) {
        return Err(invalid());
    }
    if let Some(config) = provider
        .settings_config
        .get("config")
        .and_then(Value::as_str)
    {
        let parsed = config.parse::<toml::Value>().map_err(|_| invalid())?;
        if has_other_secret(&serde_json::to_value(parsed).map_err(|_| invalid())?) {
            return Err(invalid());
        }
    }
    // Auxiliary usage credentials are captured by the same native owner before
    // stripping. They are independent of the inference key, including token plans.
    if let Some(usage) = provider
        .meta
        .as_mut()
        .and_then(|meta| meta.usage_script.as_mut())
    {
        usage.api_key = None;
        usage.access_token = None;
        usage.access_key_id = None;
        usage.secret_access_key = None;
    }
    if !secret.is_empty()
        && contains_value(
            &serde_json::to_value(&provider).map_err(|_| invalid())?,
            secret,
        )
    {
        return Err(invalid());
    }
    Ok(())
}

fn strip_material(provider: &mut Provider, material: &CredentialMaterial) -> Result<(), AppError> {
    strip_key(provider, material.key())?;
    let value = serde_json::to_value(&provider).map_err(|_| invalid())?;
    if material
        .values()
        .into_iter()
        .any(|secret| !secret.is_empty() && contains_value(&value, secret))
    {
        return Err(invalid());
    }
    material.project_usage(provider, false)
}

fn contains_value(value: &Value, secret: &str) -> bool {
    match value {
        Value::String(text) => text.contains(secret),
        Value::Array(values) => values.iter().any(|value| contains_value(value, secret)),
        Value::Object(values) => values
            .iter()
            .any(|(name, value)| name.contains(secret) || contains_value(value, secret)),
        _ => false,
    }
}

impl Database {
    /// Compatibility facade for all Provider writers, including backfill and
    /// metadata updates. The raw DAO only persists the prepared reference row.
    pub fn save_provider(&self, app: &str, provider: &Provider) -> Result<(), AppError> {
        ProviderCredentials::save(self, app, provider)
    }

    /// Backfill/recovery callers share the same credential owner as form saves.
    pub fn update_provider_settings_config(
        &self,
        app: &str,
        provider_id: &str,
        settings: &Value,
    ) -> Result<(), AppError> {
        if let Some(mut provider) = self.get_provider_by_id(provider_id, app)? {
            provider.settings_config = settings.clone();
            self.save_provider(app, &provider)?;
        }
        Ok(())
    }
}

impl ProviderCredentials {
    fn validate_native_draft(
        db: &Database,
        provider: &Provider,
        effective: &Provider,
    ) -> Result<(), AppError> {
        if let Some(draft) = provider
            .meta
            .as_ref()
            .and_then(|m| m.native_credential_draft.as_ref())
        {
            if draft.provider_id != provider.id
                || draft.target_fingerprint
                    != CredentialMaterial::draft_fingerprint(provider, effective)?
            {
                return Err(invalid());
            }
            let current = db.get_provider_by_id(&provider.id, "codex")?;
            if current.as_ref().and_then(binding) != draft.source_ref.as_deref() {
                return Err(AppError::Message("provider_secret_revoked".into()));
            }
        }
        Ok(())
    }

    /// Renderer edits need the non-secret usage configuration and retain
    /// markers. Ordinary exports continue to drop the complete script.
    pub(crate) fn renderer_projection(provider: &Provider) -> Provider {
        let mut clean = crate::provider::sanitize_provider_for_export(provider);
        if let Some(meta) = &mut clean.meta {
            meta.native_credential_draft = None;
        }
        if binding(provider).is_some() {
            if let Some(script) = provider.meta.as_ref().and_then(|m| m.usage_script.as_ref()) {
                let safe = [
                    script.api_key.as_deref(),
                    script.access_token.as_deref(),
                    script.access_key_id.as_deref(),
                    script.secret_access_key.as_deref(),
                ]
                .into_iter()
                .flatten()
                .all(|value| value == material::MASK);
                if safe {
                    clean.meta.get_or_insert_with(Default::default).usage_script =
                        Some(script.clone());
                }
            }
        }
        clean
    }

    pub(crate) fn comparison(
        db: &Database,
        expected: &Provider,
        persisted: &Provider,
    ) -> Result<Provider, AppError> {
        let mut normalized = expected.clone();
        if let Some(id) = binding(persisted) {
            let snippet = db.get_config_snippet("codex")?;
            let projected =
                super::live::build_codex_credential_projection(expected, snippet.as_deref())?;
            let persisted_projection =
                super::live::build_codex_credential_projection(persisted, snippet.as_deref())?;
            let record = Self::record(db, persisted)?;
            let previous = Self::read(db, &record)?;
            previous.validate_projection(persisted, &persisted_projection)?;
            let captured = key(expected);
            let secret = captured
                .as_deref()
                .filter(|v| !blank_or_masked(v))
                .map(String::as_str)
                .unwrap_or(previous.key());
            let material =
                CredentialMaterial::capture(expected, &projected, secret, Some(&previous))?;
            let same = Self::matches(db, &record, &material)?;
            if !same {
                return Err(secret_error(SecretServiceError::verify_failed()));
            }
            strip_material(&mut normalized, &material)?;
            normalized.settings_config[REF_KEY] = Value::String(id.to_owned());
        }
        Ok(normalized)
    }

    fn persist(
        db: &Database,
        app: &str,
        stored: &Provider,
        previous: Option<&Provider>,
    ) -> Result<(), AppError> {
        let result = db.save_provider_record(app, stored).and_then(|()| {
            let actual = db
                .get_provider_by_id(&stored.id, app)?
                .ok_or_else(invalid)?;
            if !Self::stored_contents_match(&actual, stored)? {
                return Err(invalid());
            }
            Ok(())
        });
        if result.is_err() {
            let restored = match previous {
                Some(previous) => db.save_provider_record(app, previous),
                None => db.delete_provider(app, &stored.id),
            }
            .and_then(|()| {
                let actual = db.get_provider_by_id(&stored.id, app)?;
                match (actual.as_ref(), previous) {
                    (Some(actual), Some(previous))
                        if Self::stored_contents_match(actual, previous)? =>
                    {
                        Ok(())
                    }
                    (None, None) => Ok(()),
                    _ => Err(invalid()),
                }
            });
            return Err(AppError::Message(
                if restored.is_ok() {
                    "provider_secret_persistence_failed"
                } else {
                    "provider_secret_recovery_required"
                }
                .into(),
            ));
        }
        Ok(())
    }

    fn stored_contents_match(actual: &Provider, expected: &Provider) -> Result<bool, AppError> {
        Ok(actual.settings_config == expected.settings_config
            && serde_json::to_value(actual.meta.clone().unwrap_or_default())
                .map_err(|_| invalid())?
                == serde_json::to_value(expected.meta.clone().unwrap_or_default())
                    .map_err(|_| invalid())?)
    }

    fn read(
        db: &Database,
        record: &ProviderCredentialRecord,
    ) -> Result<CredentialMaterial, AppError> {
        db.provider_secrets
            .with_material(
                &record.handle,
                PURPOSE,
                DecodeSecret::new(CredentialMaterial::decode),
            )
            .map_err(secret_error)?
    }

    fn matches(
        db: &Database,
        record: &ProviderCredentialRecord,
        material: &CredentialMaterial,
    ) -> Result<bool, AppError> {
        db.provider_secrets
            .with_material(
                &record.handle,
                PURPOSE,
                DecodeSecret::new(|bytes: &[u8]| material.matches(bytes)),
            )
            .map_err(secret_error)?
    }
    fn record(db: &Database, provider: &Provider) -> Result<ProviderCredentialRecord, AppError> {
        let id = binding(provider).ok_or_else(invalid)?;
        let current = db.get_provider_by_id(&provider.id, "codex")?;
        if current.as_ref().and_then(binding) != Some(id) {
            return Err(AppError::Message("provider_secret_revoked".into()));
        }
        let record = db
            .provider_credential(id, &provider.id)?
            .ok_or_else(|| secret_error(SecretServiceError::missing()))?;
        if record.status != "ready" {
            return Err(AppError::Message("provider_secret_revoked".into()));
        }
        Ok(record)
    }

    /// Bind the requested usage-script test target first. Saved secrets are
    /// resolved only after that destination matches this owner's material.
    pub(crate) fn admit_usage_test(
        db: &Database,
        app: &str,
        stored: &Provider,
        input: &UsageTestInput<'_>,
    ) -> Result<AdmittedUsageTest, AppError> {
        if app != "codex" {
            return Err(invalid());
        }
        let snippet = db.get_config_snippet("codex")?;
        let effective = super::live::build_codex_credential_projection(stored, snippet.as_deref())?;
        let requested = requested_usage_script(
            stored,
            input.script_code,
            input.base_url,
            input.user_id,
            input.template_type,
        );
        let fresh_key = input
            .api_key
            .map(str::trim)
            .filter(|value| !value.is_empty() && !blank_or_masked(value));
        let fresh_token = input
            .access_token
            .map(str::trim)
            .filter(|value| !value.is_empty() && !blank_or_masked(value));
        if fresh_key.is_some() || fresh_token.is_some() {
            return Ok(AdmittedUsageTest {
                api_key: fresh_key.unwrap_or_default().to_owned(),
                base_url: CredentialMaterial::destination(&effective, &requested),
                access_token: fresh_token.map(str::to_owned),
                user_id: requested.user_id.clone(),
                template_type: requested.template_type.clone(),
            });
        }
        let stored_script = stored
            .meta
            .as_ref()
            .and_then(|meta| meta.usage_script.as_ref())
            .ok_or_else(invalid)?;
        if binding(stored).is_some() {
            let record = Self::record(db, stored)?;
            let material = Self::read(db, &record)?;
            let (api_key, base_url, access_token) =
                material.admit_test(stored, &effective, stored_script, &requested)?;
            return Ok(AdmittedUsageTest {
                api_key,
                base_url,
                access_token,
                user_id: requested.user_id.clone(),
                template_type: requested.template_type.clone(),
            });
        }
        if CredentialMaterial::same_request(&effective, stored_script, &requested) {
            let api_key = effective
                .resolve_usage_credentials(&crate::app_config::AppType::Codex)
                .1;
            if !api_key.is_empty() {
                return Ok(AdmittedUsageTest {
                    api_key,
                    base_url: CredentialMaterial::destination(&effective, &requested),
                    access_token: None,
                    user_id: requested.user_id.clone(),
                    template_type: requested.template_type.clone(),
                });
            }
        }
        Err(invalid())
    }

    /// Native-only materialization. A bound provider must never fall back to an
    /// inline key, environment value, live file or another provider's reference.
    pub(crate) fn resolve(
        db: &Database,
        app: &str,
        provider: &Provider,
    ) -> Result<Provider, AppError> {
        if app != "codex" {
            return Ok(provider.clone());
        }
        let snippet = db.get_config_snippet("codex")?;
        let mut resolved =
            super::live::build_codex_credential_projection(provider, snippet.as_deref())?;
        Self::validate_native_draft(db, provider, &resolved)?;
        if binding(provider).is_none() {
            return Ok(resolved);
        }
        let record = Self::record(db, provider)?;
        let material = Self::read(db, &record)?;
        material.validate_projection(provider, &resolved)?;
        strip_material(&mut resolved, &material)?;
        material.project_usage(&mut resolved, true)?;
        if !material.key().is_empty() {
            resolved.settings_config["auth"]["OPENAI_API_KEY"] =
                Value::String(material.key().to_owned());
        }
        if let Some(meta) = &mut resolved.meta {
            meta.native_credential_draft = None;
        }
        Ok(resolved)
    }

    /// Blank/masked edits inherit only this provider's existing reference.
    /// Raw request JSON never grants authority to use another provider's key.
    pub(crate) fn merge_edit(
        db: &Database,
        app: &str,
        provider: &Provider,
    ) -> Result<Provider, AppError> {
        if !is_api_provider(provider, app) {
            return Ok(provider.clone());
        }
        let existing = db.get_provider_by_id(&provider.id, app)?;
        let (material, target_fingerprint) = Self::capture_edit(db, provider, existing.as_ref())?;
        // capture_edit has already checked any inherited context. Keep the
        // original admission snapshot across repeated native merge calls.
        let draft = provider
            .meta
            .as_ref()
            .and_then(|m| m.native_credential_draft.clone())
            .unwrap_or(crate::provider::NativeCredentialDraft {
                provider_id: provider.id.clone(),
                source_ref: existing.as_ref().and_then(binding).map(str::to_owned),
                target_fingerprint,
            });
        let mut merged = provider.clone();
        strip_material(&mut merged, &material)?;
        // A materialized request is a native draft. Keeping an old reference
        // here would make projection ignore freshly edited material.
        merged
            .settings_config
            .as_object_mut()
            .ok_or_else(invalid)?
            .remove(REF_KEY);
        material.project_usage(&mut merged, true)?;
        if !material.key().is_empty() {
            merged.settings_config["auth"]["OPENAI_API_KEY"] =
                Value::String(material.key().to_owned());
        }
        merged
            .meta
            .get_or_insert_with(Default::default)
            .native_credential_draft = Some(draft);
        Ok(merged)
    }

    fn capture_edit(
        db: &Database,
        provider: &Provider,
        existing: Option<&Provider>,
    ) -> Result<(CredentialMaterial, [u8; 32]), AppError> {
        if binding(provider).is_some() && binding(provider) != existing.and_then(binding) {
            return Err(invalid());
        }
        let snippet = db.get_config_snippet("codex")?;
        let projected =
            super::live::build_codex_credential_projection(provider, snippet.as_deref())?;
        Self::validate_native_draft(db, provider, &projected)?;
        let previous_projection = existing
            .map(|existing| {
                super::live::build_codex_credential_projection(existing, snippet.as_deref())
            })
            .transpose()?;
        let captured = key(provider);
        let fresh = captured.as_deref().filter(|value| !blank_or_masked(value));
        if fresh.is_none() {
            if let Some(existing) = existing {
                if (binding(existing).is_some() || key(existing).is_some())
                    && !material::same_inference_target(
                        existing,
                        previous_projection.as_ref().ok_or_else(invalid)?,
                        provider,
                        &projected,
                    )?
                {
                    return Err(invalid());
                }
            }
        }
        let previous = if let Some(existing) = existing.filter(|p| binding(p).is_some()) {
            let record = Self::record(db, existing)?;
            match Self::read(db, &record) {
                Ok(material) => {
                    // The current row can itself have been edited outside this
                    // owner. Validate it against the native target envelope
                    // before interpreting any input as a retain instruction.
                    let previous_projection = previous_projection.as_ref().ok_or_else(invalid)?;
                    if fresh.is_none() {
                        material.validate_inference_projection(existing, previous_projection)?;
                    }
                    if provider
                        .meta
                        .as_ref()
                        .and_then(|m| m.usage_script.as_ref())
                        .is_some_and(|usage| {
                            [
                                usage.api_key.as_deref(),
                                usage.access_token.as_deref(),
                                usage.access_key_id.as_deref(),
                                usage.secret_access_key.as_deref(),
                            ]
                            .into_iter()
                            .flatten()
                            .any(|value| !value.trim().is_empty() && blank_or_masked(value))
                        })
                    {
                        material.validate_usage_projection(existing, previous_projection)?;
                    }
                    Some(material)
                }
                Err(AppError::Message(code))
                    if code == "provider_secret_missing" && fresh.is_some() =>
                {
                    None
                }
                Err(error) => return Err(error),
            }
        } else {
            None
        };
        let legacy = existing.and_then(key);
        let secret = fresh
            .map(String::as_str)
            .or_else(|| previous.as_ref().map(CredentialMaterial::key))
            .or_else(|| legacy.as_deref().map(String::as_str))
            .unwrap_or_default();
        if secret.is_empty()
            && captured
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty())
        {
            return Err(invalid());
        }
        let material =
            CredentialMaterial::capture(provider, &projected, secret, previous.as_ref())?;
        // Common config has the same secret guards as the persisted provider;
        // it cannot silently inject a shadow key or an unowned auth header.
        strip_material(&mut projected.clone(), &material)?;
        Ok((
            material,
            CredentialMaterial::draft_fingerprint(provider, &projected)?,
        ))
    }

    fn save(db: &Database, app: &str, provider: &Provider) -> Result<(), AppError> {
        if !is_api_provider(provider, app) {
            return db.save_provider_record(app, provider);
        }
        let _guard = db.provider_secret_guard.lock().map_err(|_| invalid())?;
        let existing = db.get_provider_by_id(&provider.id, app)?;
        let (material, _) = Self::capture_edit(db, provider, existing.as_ref())?;
        let mut stored = provider.clone();
        strip_material(&mut stored, &material)?;
        if material.is_empty() {
            // Explicit credential-less drafts remain unbound. Clearing the last
            // auxiliary secret also revokes that binding at transaction settle.
            stored
                .settings_config
                .as_object_mut()
                .ok_or_else(invalid)?
                .remove(REF_KEY);
            return Self::persist(db, app, &stored, existing.as_ref());
        }
        if let Some(existing) = existing.as_ref().filter(|p| binding(p).is_some()) {
            let record = Self::record(db, existing)?;
            let same = match Self::matches(db, &record, &material) {
                Ok(same) => same,
                Err(AppError::Message(code)) if code == "provider_secret_missing" => false,
                Err(error) => return Err(error),
            };
            if same {
                stored.settings_config[REF_KEY] = Value::String(record.id);
                return Self::persist(db, app, &stored, Some(existing));
            }
        }

        // Resume an admitted create whose native write completed but whose
        // readback/DB cutover did not. Compare the freshly supplied material;
        // a handle alone never proves ownership of an uncertain native item.
        for pending in db
            .provider_credential_records()?
            .into_iter()
            .filter(|record| record.provider_id == provider.id && record.status == "pending")
        {
            match Self::matches(db, &pending, &material) {
                Ok(true) => {
                    db.set_provider_credential_status(&pending.id, "ready")?;
                    stored.settings_config[REF_KEY] = Value::String(pending.id);
                    return Self::persist(db, app, &stored, existing.as_ref());
                }
                Ok(false) => {}
                Err(AppError::Message(code)) if code == "provider_secret_missing" => {
                    db.set_provider_credential_status(&pending.id, "deleted")?
                }
                Err(error) => return Err(error),
            }
        }

        let encoded = material.encode()?;
        let native_material =
            SecretMaterial::from_native_input(encoded.to_vec(), PURPOSE).map_err(secret_error)?;
        let record = ProviderCredentialRecord {
            id: format!("pc_{}", uuid::Uuid::new_v4().simple()),
            provider_id: provider.id.clone(),
            handle: db.provider_secrets.reserve(),
            status: "pending".into(),
        };
        // Admission precedes native create, including a create whose readback
        // is uncertain. Failure leaves both this record and old config intact.
        db.admit_provider_credential(&record)?;
        db.provider_secrets
            .create_reserved(&record.handle, native_material, PURPOSE)
            .map_err(secret_error)?;
        let verified = Self::matches(db, &record, &material)?;
        if !verified {
            return Err(secret_error(SecretServiceError::verify_failed()));
        }
        db.set_provider_credential_status(&record.id, "ready")?;
        stored.settings_config[REF_KEY] = Value::String(record.id);
        Self::persist(db, app, &stored, existing.as_ref())
    }

    /// Best-effort startup migration: one failed/locked row cannot erase its
    /// legacy material or prevent other rows from migrating. No live file write.
    pub(crate) fn migrate_legacy(db: &Database) -> Result<usize, AppError> {
        let mut migrated = 0;
        for provider in db.get_all_providers("codex")?.values() {
            if is_api_provider(provider, "codex")
                && binding(provider).is_none()
                && key(provider).is_some()
            {
                match Self::save(db, "codex", provider) {
                    Ok(()) => migrated += 1,
                    Err(_) => log::warn!(
                        "Codex credential migration deferred; existing configuration retained"
                    ),
                }
            }
        }
        Ok(migrated)
    }

    /// Called only after an enclosing Provider transaction has settled. Never
    /// delete an old key while its provider can still be restored by rollback.
    /// Locked/failed deletes retain durable rows for the next startup/retry.
    pub(crate) fn cleanup(db: &Database) -> Result<(), AppError> {
        let _guard = db.provider_secret_guard.lock().map_err(|_| invalid())?;
        let bound: HashSet<String> = db
            .get_all_providers("codex")?
            .values()
            .filter_map(binding)
            .map(str::to_owned)
            .collect();
        for record in db.provider_credential_records()? {
            // A pending create may have succeeded without readback; ownership
            // must be re-proven with its captured input before deletion.
            if bound.contains(&record.id) || matches!(record.status.as_str(), "deleted" | "pending")
            {
                continue;
            }
            db.set_provider_credential_status(&record.id, "revoked")?;
            match db.provider_secrets.delete(&record.handle) {
                Ok(_) => db.set_provider_credential_status(&record.id, "deleted")?,
                Err(error) if error.code() == SecretErrorCode::Missing => {
                    db.set_provider_credential_status(&record.id, "deleted")?
                }
                Err(error) => return Err(secret_error(error)),
            }
        }
        Ok(())
    }

    pub(crate) fn settle(db: &Database) {
        if Self::cleanup(db).is_err() {
            log::warn!("Provider credential cleanup deferred; revoked handles retained for retry");
        }
    }
}

#[cfg(test)]
mod tests;
