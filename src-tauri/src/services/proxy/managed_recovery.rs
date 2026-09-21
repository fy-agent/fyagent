//! Managed-only restore proof. Legacy API-key backups keep their existing shape.

use super::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const MARKER: &str = "fyagentManagedRestore";

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ManagedRestoreProof {
    version: u8,
    config: Value,
    preimages: Vec<Option<Vec<u8>>>,
    // None is an interrupted projection. Never guess which bytes it published.
    files: Option<Vec<Option<String>>>,
}

fn paths(app: &AppType) -> Result<Vec<std::path::PathBuf>, String> {
    Ok(match app {
        AppType::Claude => vec![get_claude_settings_path()],
        AppType::Codex => vec![
            crate::codex_config::get_codex_config_path(),
            crate::codex_config::get_codex_model_catalog_path(),
        ],
        AppType::GrokBuild => vec![crate::grok_config::get_grok_config_path()],
        _ => return Err("Managed restore target unavailable".into()),
    })
}

fn read_files(app: &AppType) -> Result<Vec<Option<Vec<u8>>>, String> {
    paths(app)?
        .into_iter()
        .map(|path| match std::fs::read(path) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(_) => Err("Managed restore file unavailable".into()),
        })
        .collect()
}

fn hashes(files: &[Option<Vec<u8>>]) -> Vec<Option<String>> {
    files
        .iter()
        .map(|bytes| {
            bytes
                .as_ref()
                .map(|bytes| format!("{:x}", Sha256::digest(bytes)))
        })
        .collect()
}

fn fingerprints(app: &AppType) -> Result<Vec<Option<String>>, String> {
    Ok(hashes(&read_files(app)?))
}

fn decode(config: &Value) -> Result<Option<ManagedRestoreProof>, String> {
    if config.as_object().is_some_and(|object| object.len() == 1) {
        if let Some(proof) = config.get(MARKER) {
            return serde_json::from_value::<ManagedRestoreProof>(proof.clone())
                .map(Some)
                .map_err(|_| "Managed restore proof unavailable".into());
        }
    }
    Ok(None)
}

impl ProxyService {
    pub(super) fn validate_existing_managed_backup(
        &self,
        app: &AppType,
        encoded: &str,
    ) -> Result<(), String> {
        let raw: Value =
            serde_json::from_str(encoded).map_err(|_| "Managed restore backup unavailable")?;
        if !self.verify_and_unwrap_managed_restore(app, raw)?.1 {
            return Err(
                "Restore the previous proxy configuration before subscription binding".into(),
            );
        }
        Ok(())
    }

    pub(super) fn require_managed_restore_proof(
        &self,
        app: &AppType,
        has_proof: bool,
    ) -> Result<(), String> {
        if !has_proof
            && self
                .get_current_provider_for_app(app)?
                .is_some_and(|provider| provider.uses_subscription_proxy())
        {
            return Err(
                "Managed subscription recovery proof is missing; recovery requires review".into(),
            );
        }
        Ok(())
    }

    pub(super) fn unwrap_managed_backup(config: Value) -> Result<Value, String> {
        Ok(match decode(&config)? {
            Some(proof) => proof.config,
            None => config,
        })
    }

    pub(super) fn managed_restore_files(app: &AppType) -> Result<Vec<Option<String>>, String> {
        fingerprints(app)
    }

    pub(super) fn managed_written_files(
        app: &AppType,
        before: Vec<Option<String>>,
    ) -> Result<Vec<Option<String>>, String> {
        paths(app)?
            .into_iter()
            .zip(before)
            .map(|(path, unchanged)| {
                crate::config::file_mutation_expected_hash(&path)
                    .map(|written| written.unwrap_or(unchanged))
                    .map_err(|_| "Managed restore publication proof unavailable".into())
            })
            .collect()
    }

    pub(super) fn mark_managed_restore_proof(
        &self,
        app: &AppType,
        files: Option<Vec<Option<String>>>,
    ) -> Result<(), String> {
        let backup = self
            .db
            .get_live_backup_sync(app.as_str())
            .map_err(|_| "Managed restore backup unavailable")?
            .ok_or("Managed restore backup missing")?;
        let raw: Value = serde_json::from_str(&backup.original_config)
            .map_err(|_| "Managed restore backup unavailable")?;
        // Rebinding may not adopt edits made after the previous owned write.
        if files.is_none() {
            if decode(&raw)?.is_some() || self.detect_takeover_in_live_config_for_app(app) {
                self.verify_and_unwrap_managed_restore(app, raw.clone())?;
            }
            if decode(&raw)?.is_none() && self.detect_takeover_in_live_config_for_app(app) {
                // A legacy takeover has no byte preimage/ownership receipt in
                // its DB backup. Restore it first instead of adopting proxy
                // placeholders as this subscription's original native files.
                return Err(
                    "Restore the current proxy configuration before subscription binding".into(),
                );
            }
        } else if files.as_ref() != Some(&fingerprints(app)?) {
            return Err("Managed subscription configuration changed before proof commit".into());
        }
        let (config, preimages) = match decode(&raw)? {
            Some(proof) => (proof.config, proof.preimages),
            None => (raw, read_files(app)?),
        };
        self.save_managed_restore_proof(
            app,
            &ManagedRestoreProof {
                version: 1,
                config,
                preimages,
                files,
            },
        )
    }

    fn save_managed_restore_proof(
        &self,
        app: &AppType,
        proof: &ManagedRestoreProof,
    ) -> Result<(), String> {
        self.db
            .save_live_backup_sync(
                app.as_str(),
                &serde_json::to_string(&json!({ MARKER: proof }))
                    .map_err(|_| "Managed restore proof unavailable")?,
            )
            .map_err(|_| "Managed restore proof write failed".into())
    }

    pub(super) fn closed_restore_preview_paths(
        app: &AppType,
    ) -> Result<Vec<std::path::PathBuf>, String> {
        paths(app)
    }

    fn restore_verified_managed_proof(
        &self,
        app: &AppType,
        config: Value,
    ) -> Result<ManagedRestoreProof, String> {
        let proof = match decode(&config)? {
            Some(proof) => proof,
            None => self
                .compute_legacy_managed_restore_proof(
                    app,
                    &config,
                    self.get_current_provider_for_app(app)?,
                )?
                .ok_or("Managed restore proof missing")?,
        };
        verify_proof(app, &proof, true)?;
        Ok(proof)
    }

    pub(super) fn restore_verified_managed_config(&self, app: &AppType) -> Result<(), String> {
        let backup = self
            .db
            .get_live_backup_sync(app.as_str())
            .map_err(|_| "Managed restore backup unavailable")?
            .ok_or("Managed restore backup missing")?;
        let raw: Value = serde_json::from_str(&backup.original_config)
            .map_err(|_| "Managed restore backup unavailable")?;
        let proof = self.restore_verified_managed_proof(app, raw)?;
        // Persist the just-verified proof before any live write. A crash after
        // the first file is restored can then decode original preimages and
        // owned hashes; recomputing a pre-MARKER proof would require the live
        // proxy endpoint and reject already-restored originals.
        self.save_managed_restore_proof(app, &proof)?;
        let expected = hashes(&proof.preimages);
        let paths = paths(app)?;
        if paths.len() != proof.preimages.len() {
            return Err("Managed restore proof unavailable".into());
        }
        let owned = proof
            .files
            .clone()
            .ok_or("Managed restore proof unavailable")?;
        for (index, ((path, bytes), hash)) in paths
            .into_iter()
            .zip(proof.preimages)
            .zip(owned)
            .enumerate()
        {
            crate::config::restore_file_preimage_if_owned(&path, bytes.as_deref(), hash.as_deref())
                .map_err(|_| "Managed restore configuration changed")?;
            #[cfg(test)]
            if index == 0 {
                self.observe_managed_activation(app, "restored_first_file")?;
            }
            #[cfg(not(test))]
            let _ = index;
        }
        self.mark_managed_restore_proof(app, Some(expected))
    }

    pub(super) fn verify_and_unwrap_managed_restore(
        &self,
        app: &AppType,
        config: Value,
    ) -> Result<(Value, bool), String> {
        self.verify_managed_restore(app, config, false)
    }

    pub(super) fn verify_and_unwrap_managed_restore_for_exit(
        &self,
        app: &AppType,
        config: Value,
    ) -> Result<(Value, bool), String> {
        self.verify_managed_restore(app, config, true)
    }

    fn verify_managed_restore(
        &self,
        app: &AppType,
        config: Value,
        allow_restored: bool,
    ) -> Result<(Value, bool), String> {
        let proof = match decode(&config)? {
            Some(proof) => proof,
            None => match self.compute_legacy_managed_restore_proof(
                app,
                &config,
                self.get_current_provider_for_app(app)?,
            )? {
                Some(proof) => proof,
                None => return Ok((config, false)),
            },
        };
        verify_proof(app, &proof, allow_restored)?;
        Ok((proof.config, true))
    }

    pub(super) fn managed_restore_preview_paths(
        &self,
        app: &AppType,
        config: &Value,
    ) -> Result<Option<Vec<std::path::PathBuf>>, String> {
        let proof = match decode(config)? {
            Some(proof) => Some(proof),
            None => self.compute_legacy_managed_restore_proof(
                app,
                config,
                self.current_provider_for_restore_preview(app)?,
            )?,
        };
        let Some(proof) = proof else {
            return Ok(None);
        };
        verify_proof(app, &proof, true)?;
        Ok(Some(paths(app)?))
    }

    /// v0.4.5 stored logical backups but its atomic writers already retained
    /// path-bound receipts. A receipt alone is insufficient: a later MCP edit
    /// also has one. Admit only complete, reproducible subscription projections.
    fn compute_legacy_managed_restore_proof(
        &self,
        app: &AppType,
        original: &Value,
        provider: Option<Provider>,
    ) -> Result<Option<ManagedRestoreProof>, String> {
        let Some(provider) = provider.filter(Provider::uses_subscription_proxy) else {
            return Ok(None);
        };
        let paths = paths(app)?;
        let receipt = crate::config::verified_file_recovery(&paths[0])
            .map_err(|_| "Legacy subscription recovery receipt is invalid")?
            .ok_or("Legacy subscription recovery receipt is missing; restore the original configuration before reconnecting")?;
        let current = read_files(app)?;
        let live = current[0]
            .as_deref()
            .ok_or("Legacy subscription configuration unavailable")?;
        let listener = futures::executor::block_on(self.db.get_global_proxy_config())
            .map_err(|_| "Legacy subscription listener unavailable")?;
        let host = if listener.listen_address.contains(':') {
            format!("[{}]", listener.listen_address)
        } else {
            listener.listen_address
        };
        let expected_origin = format!("http://{host}:{}", listener.listen_port);
        // A later writer can produce another valid receipt. Its endpoint must
        // still be the saved subscription listener, not a self-declared origin.
        if listener.listen_port == 0
            || legacy_proxy_origin(app, live).as_deref() != Some(expected_origin.as_str())
        {
            return Err("Legacy subscription endpoint changed; recovery requires review".into());
        }
        let saved = self
            .db
            .get_all_providers(app.as_str())
            .map_err(|_| "Legacy subscription provider unavailable")?;
        let candidates = saved
            .values()
            .filter(|item| item.uses_subscription_proxy())
            .collect::<Vec<_>>();
        if !self.legacy_projection_matches(app, original, &provider, live, &candidates) {
            return Err("Legacy subscription configuration changed; preserve the current configuration and restore its original source before reconnecting".into());
        }
        let native = legacy_native_bytes(app, original)?;
        let preimage = if legacy_native_matches(app, original, receipt.preimage.as_deref()) {
            receipt.preimage
        } else {
            // A later model/account binding has rolled the sidecar forward.
            // Its preimage must itself be an explained managed projection.
            // The legacy DB remains the restore authority for original fields
            // masked by both projections; v0.4.5 retained no digest of them.
            let previous = receipt
                .preimage
                .as_deref()
                .ok_or("Legacy subscription original configuration is unavailable")?;
            if !candidates.iter().any(|candidate| {
                self.legacy_projection_matches(app, original, candidate, previous, &candidates)
            }) {
                return Err("Legacy subscription backup does not match its writer receipt".into());
            }
            Some(native)
        };
        // The legacy DB never retained Codex catalog bytes. Preserve its
        // current contents; recovery does not claim an older catalog preimage.
        let mut preimages = current.clone();
        preimages[0] = preimage;
        let mut expected = hashes(&current);
        expected[0] = receipt.postimage_sha256;
        if fingerprints(app)? != expected {
            return Err("Legacy subscription configuration changed during recovery".into());
        }
        let proof = ManagedRestoreProof {
            version: 1,
            config: original.clone(),
            preimages,
            files: Some(expected),
        };
        Ok(Some(proof))
    }

    fn legacy_projection_matches(
        &self,
        app: &AppType,
        original: &Value,
        provider: &Provider,
        live: &[u8],
        previous: &[&Provider],
    ) -> bool {
        let Some(origin) = legacy_proxy_origin(app, live) else {
            return false;
        };
        let matches = |base: &Value| {
            self.legacy_projection(app, base, provider, &origin)
                .and_then(|projected| legacy_native_bytes(app, &projected))
                .is_ok_and(|expected| expected == live)
        };
        if matches(original) {
            return true;
        }
        // Codex keeps inactive provider tables across a source change. There
        // are two historical subscription sources; rebuild the earlier slot
        // from a saved managed Provider, never from arbitrary current fields.
        *app == AppType::Codex
            && previous.iter().any(|prior| {
                self.legacy_projection(app, original, prior, &origin)
                    .is_ok_and(|projected| matches(&projected))
            })
    }

    fn legacy_projection(
        &self,
        app: &AppType,
        original: &Value,
        provider: &Provider,
        origin: &str,
    ) -> Result<Value, String> {
        if *app == AppType::Claude {
            let mut projected = original.clone();
            Self::apply_claude_takeover_fields_for_provider(&mut projected, origin, provider);
            return Ok(crate::services::provider::sanitize_claude_settings_for_live(&projected));
        }
        let mut projected =
            build_effective_settings_with_common_config(self.db.as_ref(), app, provider)
                .map_err(|_| "Legacy subscription provider unavailable")?;
        let original_text = original
            .get("config")
            .and_then(Value::as_str)
            .ok_or("Legacy subscription backup unavailable")?;
        match app {
            AppType::Codex => {
                // Catalog generation can execute native discovery. This
                // compatibility admission is strictly local and read-only.
                if projected.get("modelCatalog").is_some() {
                    return Err("Legacy subscription catalog requires review".into());
                }
                Self::preserve_toml_mcp_servers_from_existing_config(&mut projected, original)?;
                Self::apply_codex_takeover_fields_for_provider(
                    &mut projected,
                    &format!("{origin}/v1"),
                    provider,
                )?;
                let auth = projected.get("auth").unwrap_or(&Value::Null);
                let text = crate::codex_config::patch_codex_source_config(
                    original_text,
                    provider.category.as_deref(),
                    auth,
                    projected
                        .get("config")
                        .and_then(Value::as_str)
                        .ok_or("Legacy subscription provider unavailable")?,
                    &crate::codex_config::get_codex_config_dir(),
                    crate::settings::unify_codex_session_history(),
                )
                .map_err(|_| "Legacy subscription projection unavailable")?;
                projected["config"] = json!(
                    crate::codex_config::prepare_codex_provider_live_config(auth, &text)
                        .map_err(|_| "Legacy subscription projection unavailable")?
                );
            }
            AppType::GrokBuild => {
                let desired = projected
                    .get("config")
                    .and_then(Value::as_str)
                    .ok_or("Legacy subscription provider unavailable")?;
                projected["config"] =
                    json!(crate::services::provider::patch_grok_quick_setup_config(
                        original_text,
                        desired
                    )
                    .map_err(|_| "Legacy subscription projection unavailable")?);
                Self::apply_grok_takeover_fields(
                    &mut projected,
                    &format!("{origin}/grokbuild/v1"),
                )?;
            }
            _ => return Err("Legacy subscription target unavailable".into()),
        }
        Ok(projected)
    }
}

fn legacy_native_bytes(app: &AppType, original: &Value) -> Result<Vec<u8>, String> {
    if *app == AppType::Claude && original.is_object() {
        crate::config::json_file_contents(
            &crate::services::provider::sanitize_claude_settings_for_live(original),
        )
        .map_err(|_| "Legacy subscription backup unavailable".into())
    } else {
        original
            .get("config")
            .and_then(Value::as_str)
            .map(|text| text.as_bytes().to_vec())
            .ok_or_else(|| "Legacy subscription backup unavailable".into())
    }
}

fn legacy_native_matches(app: &AppType, original: &Value, bytes: Option<&[u8]>) -> bool {
    match (app, bytes) {
        (AppType::Claude, Some(bytes)) => {
            serde_json::from_slice::<Value>(bytes).is_ok_and(|value| {
                &value == original
                    || (value.is_null()
                        && original.as_object().is_some_and(|object| object.is_empty()))
            })
        }
        (AppType::Claude, None) => original.as_object().is_some_and(|object| object.is_empty()),
        (_, Some(bytes)) => original
            .get("config")
            .and_then(Value::as_str)
            .is_some_and(|text| text.as_bytes() == bytes),
        (_, None) => original.get("config").and_then(Value::as_str) == Some(""),
    }
}

fn legacy_proxy_origin(app: &AppType, bytes: &[u8]) -> Option<String> {
    let text = std::str::from_utf8(bytes).ok()?;
    let url = match app {
        AppType::Claude => serde_json::from_str::<Value>(text)
            .ok()?
            .pointer("/env/ANTHROPIC_BASE_URL")?
            .as_str()?
            .to_owned(),
        AppType::Codex => crate::codex_config::extract_codex_base_url(text)?
            .strip_suffix("/v1")?
            .to_owned(),
        AppType::GrokBuild => crate::grok_config::extract_base_url(text)?
            .strip_suffix("/grokbuild/v1")?
            .to_owned(),
        _ => return None,
    };
    let parsed = url::Url::parse(&url).ok()?;
    (is_local_proxy_url(&url)
        && parsed.scheme() == "http"
        && parsed.username().is_empty()
        && parsed.password().is_none()
        && parsed.path() == "/"
        && parsed.query().is_none()
        && parsed.fragment().is_none())
    .then_some(url)
}

fn verify_proof(
    app: &AppType,
    proof: &ManagedRestoreProof,
    allow_restored: bool,
) -> Result<(), String> {
    let current = fingerprints(app)?;
    let original = hashes(&proof.preimages);
    let owned = proof
        .files
        .as_ref()
        .ok_or("Managed restore proof unavailable")?;
    if proof.version != 1
        || owned.len() != current.len()
        || original.len() != current.len()
        || !current
            .iter()
            .zip(owned)
            .zip(&original)
            .all(|((live, written), before)| live == written || (allow_restored && live == before))
    {
        return Err("Managed subscription configuration changed; recovery requires review".into());
    }
    Ok(())
}
