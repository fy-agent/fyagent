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
        if decode(&raw)?.is_none() {
            return Err(
                "Restore the previous proxy configuration before subscription binding".into(),
            );
        }
        self.verify_and_unwrap_managed_restore(app, raw).map(|_| ())
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
            self.verify_and_unwrap_managed_restore(app, raw.clone())?;
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
        let proof = ManagedRestoreProof {
            version: 1,
            config,
            preimages,
            files,
        };
        self.db
            .save_live_backup_sync(
                app.as_str(),
                &serde_json::to_string(&json!({MARKER: proof}))
                    .map_err(|_| "Managed restore proof unavailable")?,
            )
            .map_err(|_| "Managed restore proof write failed".into())
    }

    pub(super) fn restore_verified_managed_config(&self, app: &AppType) -> Result<(), String> {
        let backup = self
            .db
            .get_live_backup_sync(app.as_str())
            .map_err(|_| "Managed restore backup unavailable")?
            .ok_or("Managed restore backup missing")?;
        let raw: Value = serde_json::from_str(&backup.original_config)
            .map_err(|_| "Managed restore backup unavailable")?;
        self.verify_and_unwrap_managed_restore(app, raw.clone())?;
        let proof = decode(&raw)?.ok_or("Managed restore proof missing")?;
        let expected = hashes(&proof.preimages);
        let paths = paths(app)?;
        if paths.len() != proof.preimages.len() {
            return Err("Managed restore proof unavailable".into());
        }
        let owned = proof.files.ok_or("Managed restore proof unavailable")?;
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
        let Some(proof) = decode(&config)? else {
            return Ok((config, false));
        };
        if proof.version != 1 || proof.files.as_ref() != Some(&fingerprints(app)?) {
            return Err(
                "Managed subscription configuration changed; recovery requires review".into(),
            );
        }
        Ok((proof.config, true))
    }
}
