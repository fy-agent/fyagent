//! One native owner for Codex API-key persistence. The public Database save
//! method is a compatibility facade; its SQL implementation stays in the DAO.
//! No SQLite lock is held while calling the credential backend.
use std::collections::HashSet;

use serde_json::{json, Value};
use subtle::ConstantTimeEq;
use zeroize::Zeroizing;

use crate::database::{Database, ProviderCredentialRecord};
use crate::error::AppError;
use crate::provider::Provider;
use crate::services::secret::{
    DecodeSecret, SecretErrorCode, SecretMaterial, SecretPurpose, SecretServiceError,
};

const REF_KEY: &str = "credentialRef";
const PURPOSE: SecretPurpose = SecretPurpose::CodexApiKey;

pub(crate) struct ProviderCredentials;

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
    // Duplicated usage overrides have the same semantic owner as the main key.
    if let Some(usage) = provider
        .meta
        .as_mut()
        .and_then(|meta| meta.usage_script.as_mut())
    {
        if usage.api_key.as_deref() == Some(secret) {
            usage.api_key = None;
        }
        if usage.api_key.as_deref().is_some_and(|key| !key.is_empty())
            || usage
                .access_token
                .as_deref()
                .is_some_and(|token| !token.is_empty())
        {
            return Err(invalid());
        }
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
    pub(crate) fn comparison(
        db: &Database,
        expected: &Provider,
        persisted: &Provider,
    ) -> Result<Provider, AppError> {
        let mut normalized = expected.clone();
        if let (Some(secret), Some(id)) = (key(expected), binding(persisted)) {
            let record = Self::record(db, persisted)?;
            let same = db
                .provider_secrets
                .with_material(
                    &record.handle,
                    PURPOSE,
                    DecodeSecret::new(|bytes: &[u8]| bool::from(bytes.ct_eq(secret.as_bytes()))),
                )
                .map_err(secret_error)?;
            if !same {
                return Err(secret_error(SecretServiceError::verify_failed()));
            }
            strip_key(&mut normalized, &secret)?;
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
            if actual.settings_config != stored.settings_config {
                return Err(invalid());
            }
            Ok(())
        });
        if result.is_err() {
            let restored = match previous {
                Some(previous) => db.save_provider_record(app, previous),
                None => db.delete_provider(app, &stored.id),
            };
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

    /// Native-only materialization. A bound provider must never fall back to an
    /// inline key, environment value, live file or another provider's reference.
    pub(crate) fn resolve(
        db: &Database,
        app: &str,
        provider: &Provider,
    ) -> Result<Provider, AppError> {
        if app != "codex" || binding(provider).is_none() {
            return Ok(provider.clone());
        }
        let record = Self::record(db, provider)?;
        let key = db
            .provider_secrets
            .with_material(
                &record.handle,
                PURPOSE,
                DecodeSecret::new(|bytes: &[u8]| {
                    std::str::from_utf8(bytes)
                        .map(|text| Zeroizing::new(text.to_owned()))
                        .map_err(|_| invalid())
                }),
            )
            .map_err(secret_error)??;
        let mut resolved = provider.clone();
        strip_key(&mut resolved, &key)?;
        resolved.settings_config["auth"]["OPENAI_API_KEY"] = Value::String(key.to_string());
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
        let mut merged = provider.clone();
        if key(provider)
            .as_deref()
            .is_some_and(|value| !blank_or_masked(value))
        {
            // A captured replacement is a draft; a stale edit reference must
            // not override the new material during native projection.
            if let Some(settings) = merged.settings_config.as_object_mut() {
                settings.remove(REF_KEY);
            }
        }
        if key(provider)
            .as_deref()
            .is_none_or(|key| blank_or_masked(key))
        {
            if let Some(existing) = db.get_provider_by_id(&provider.id, app)? {
                if let Some(id) = binding(&existing) {
                    merged.settings_config[REF_KEY] = Value::String(id.to_owned());
                    // A masked input is only a retain instruction.
                    let masked = key(&merged).map(|key| key.to_string()).unwrap_or_default();
                    strip_key(&mut merged, &masked)?;
                    return Self::resolve(db, app, &merged);
                }
                if let Some(previous) = key(&existing) {
                    merged.settings_config["auth"]["OPENAI_API_KEY"] =
                        Value::String(previous.to_string());
                }
            }
        }
        Ok(merged)
    }

    fn save(db: &Database, app: &str, provider: &Provider) -> Result<(), AppError> {
        if !is_api_provider(provider, app) {
            return db.save_provider_record(app, provider);
        }
        let _guard = db.provider_secret_guard.lock().map_err(|_| invalid())?;
        let existing = db.get_provider_by_id(&provider.id, app)?;
        if binding(provider).is_some() && binding(provider) != existing.as_ref().and_then(binding) {
            return Err(invalid());
        }
        let mut stored = provider.clone();
        let captured = key(provider);
        let input = if captured
            .as_deref()
            .is_none_or(|value| blank_or_masked(value))
            && existing.as_ref().and_then(binding).is_none()
        {
            // Blank edits of a not-yet-migrated row must not erase its only
            // copy. The inherited legacy key follows the same verified cutover.
            existing
                .as_ref()
                .and_then(key)
                .filter(|value| !blank_or_masked(value))
                .or(captured)
        } else {
            captured
        };
        let provided_key = input.as_deref().filter(|value| !blank_or_masked(value));

        if provided_key.is_none() {
            let existing_ref = existing.as_ref().and_then(binding);
            let requested_ref = binding(provider);
            if requested_ref.is_some() && requested_ref != existing_ref {
                return Err(invalid());
            }
            if let Some(id) = existing_ref {
                stored.settings_config[REF_KEY] = Value::String(id.to_owned());
                strip_key(
                    &mut stored,
                    input.as_deref().map(String::as_str).unwrap_or_default(),
                )?;
                Self::record(db, &stored)?;
                return Self::persist(db, app, &stored, existing.as_ref());
            }
            if input
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty())
            {
                return Err(invalid());
            }
            // Explicit credential-less drafts remain unbound.
            return Self::persist(db, app, &stored, existing.as_ref());
        }

        let material = provided_key.expect("checked key presence");
        strip_key(&mut stored, material)?;
        if let Some(existing) = existing
            .as_ref()
            .filter(|existing| binding(existing).is_some())
        {
            let record = Self::record(db, existing)?;
            let same = db.provider_secrets.with_material(
                &record.handle,
                PURPOSE,
                DecodeSecret::new(|bytes: &[u8]| bool::from(bytes.ct_eq(material.as_bytes()))),
            );
            // Freshly captured input may repair a missing native item. Locked,
            // denied or unavailable stores still fail closed before any cutover.
            let same = match same {
                Ok(same) => same,
                Err(error) if error.code() == SecretErrorCode::Missing => false,
                Err(error) => return Err(secret_error(error)),
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
            match db.provider_secrets.with_material(
                &pending.handle,
                PURPOSE,
                DecodeSecret::new(|bytes: &[u8]| bool::from(bytes.ct_eq(material.as_bytes()))),
            ) {
                Ok(true) => {
                    db.set_provider_credential_status(&pending.id, "ready")?;
                    stored.settings_config[REF_KEY] = Value::String(pending.id);
                    return Self::persist(db, app, &stored, existing.as_ref());
                }
                Ok(false) => {}
                Err(error) if error.code() == SecretErrorCode::Missing => {
                    db.set_provider_credential_status(&pending.id, "deleted")?
                }
                Err(error) => return Err(secret_error(error)),
            }
        }

        let material = SecretMaterial::from_native_input(material.as_bytes().to_vec(), PURPOSE)
            .map_err(secret_error)?;
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
            .create_reserved(&record.handle, material, PURPOSE)
            .map_err(secret_error)?;
        let verified = db
            .provider_secrets
            .with_material(
                &record.handle,
                PURPOSE,
                DecodeSecret::new(|bytes: &[u8]| {
                    bool::from(bytes.ct_eq(provided_key.expect("checked key").as_bytes()))
                }),
            )
            .map_err(secret_error)?;
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
