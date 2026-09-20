//! OpenCode projection under the existing Provider transaction and config lock.
//! Recovery retains exact bytes and refuses to overwrite external edits.

use super::*;
use crate::opencode_config::{read_opencode_config_bytes, write_opencode_config_value};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct OpenCodeProxyBackup {
    version: u8,
    original: Option<Vec<u8>>,
    before: Option<Vec<u8>>,
    projected: Value,
    postimage_sha256: String,
}

fn matches_postimage(bytes: Option<&[u8]>, backup: &OpenCodeProxyBackup) -> bool {
    bytes.is_some_and(|bytes| format!("{:x}", Sha256::digest(bytes)) == backup.postimage_sha256)
}

fn read_config() -> Result<Value, String> {
    let bytes = read_opencode_config_bytes().map_err(|_| "OpenCode configuration unavailable")?;
    match bytes {
        None => Ok(json!({})),
        Some(bytes) => serde_json::from_slice::<Value>(&bytes)
            .ok()
            .filter(Value::is_object)
            .ok_or_else(|| "OpenCode configuration unavailable".into()),
    }
}

fn selected_model(provider: &Provider) -> Result<&str, String> {
    let models = provider
        .settings_config
        .get("models")
        .and_then(Value::as_object)
        .filter(|models| models.len() == 1)
        .ok_or("OpenCode subscription model unavailable")?;
    models
        .keys()
        .next()
        .map(String::as_str)
        .ok_or_else(|| "OpenCode subscription model unavailable".into())
}

fn projected_provider(provider: &Provider, proxy_url: &str) -> Result<Value, String> {
    if !provider.uses_subscription_proxy()
        || provider.settings_config.get("npm").and_then(Value::as_str) != Some("@ai-sdk/openai")
    {
        return Err("OpenCode subscription provider unavailable".into());
    }
    selected_model(provider)?;
    let mut projected = provider.settings_config.clone();
    projected["options"]["baseURL"] =
        json!(format!("{}/opencode/v1", proxy_url.trim_end_matches('/')));
    projected["options"]["apiKey"] = json!(PROXY_TOKEN_PLACEHOLDER);
    Ok(projected)
}

impl ProxyService {
    /// Lock is held by the narrow binder from revision admission to commit.
    pub(crate) fn validate_opencode_managed_projection(
        &self,
        provider: &Provider,
    ) -> Result<(), String> {
        selected_model(provider)?;
        let config = read_config()?;
        if config
            .get("provider")
            .is_some_and(|value| !value.is_object())
        {
            return Err("OpenCode provider configuration unavailable".into());
        }
        let backup = self
            .db
            .get_live_backup_sync("opencode")
            .map_err(|_| "OpenCode recovery state unavailable")?;
        if let Some(backup) = backup {
            let backup: OpenCodeProxyBackup = serde_json::from_str(&backup.original_config)
                .map_err(|_| "OpenCode recovery state unavailable")?;
            let bytes =
                read_opencode_config_bytes().map_err(|_| "OpenCode configuration unavailable")?;
            if backup.version != 1 || !matches_postimage(bytes.as_deref(), &backup) {
                return Err(
                    "OpenCode configuration changed outside the subscription operation".into(),
                );
            }
        } else if Self::is_opencode_live_taken_over(&config) {
            return Err("OpenCode subscription recovery record is missing".into());
        }
        if let Some(existing) = config
            .get("provider")
            .and_then(|providers| providers.get(&provider.id))
        {
            // Only an owned, intact projection may be replaced. Native/user
            // providers with a colliding reserved ID are never overwritten.
            let url = existing
                .pointer("/options/baseURL")
                .and_then(Value::as_str)
                .and_then(|url| url.strip_suffix("/opencode/v1"))
                .filter(|url| is_local_proxy_url(url));
            if url
                .is_none_or(|url| projected_provider(provider, url).ok().as_ref() != Some(existing))
            {
                return Err("OpenCode subscription provider conflict".into());
            }
        }
        Ok(())
    }

    pub(super) fn project_opencode_managed_locked(
        &self,
        provider: &Provider,
        proxy_url: &str,
    ) -> Result<(), String> {
        self.validate_opencode_managed_projection(provider)?;
        let before =
            read_opencode_config_bytes().map_err(|_| "OpenCode configuration unavailable")?;
        let mut config = read_config()?;
        let projected = projected_provider(provider, proxy_url)?;
        let model = selected_model(provider)?;
        if config.get("provider").is_none() {
            config["provider"] = json!({});
        }
        config["provider"][&provider.id] = projected;
        config["model"] = json!(format!("{}/{model}", provider.id));
        let previous_backup = self
            .db
            .get_live_backup_sync("opencode")
            .map_err(|_| "OpenCode recovery state unavailable")?;
        let original = match previous_backup {
            Some(backup) => {
                let backup: OpenCodeProxyBackup = serde_json::from_str(&backup.original_config)
                    .map_err(|_| "OpenCode recovery state unavailable")?;
                if let Some((old_id, _)) = backup
                    .projected
                    .get("model")
                    .and_then(Value::as_str)
                    .and_then(|model| model.split_once('/'))
                {
                    if old_id != provider.id {
                        if let Some(providers) =
                            config.get_mut("provider").and_then(Value::as_object_mut)
                        {
                            providers.remove(old_id);
                        }
                    }
                }
                backup.original
            }
            None => before.clone(),
        };
        let backup = OpenCodeProxyBackup {
            version: 1,
            original,
            before: before.clone(),
            projected: config.clone(),
            postimage_sha256: format!(
                "{:x}",
                Sha256::digest(
                    crate::config::json_file_contents(&config)
                        .map_err(|_| "OpenCode projection unavailable")?
                )
            ),
        };
        // Persist the recovery pre/postimage before any file publication, so
        // recovery covers a crash on either side of atomic replacement.
        self.db
            .save_live_backup_sync(
                "opencode",
                &serde_json::to_string(&backup)
                    .map_err(|_| "OpenCode recovery state unavailable")?,
            )
            .map_err(|_| "OpenCode recovery backup failed")?;
        if let Some(bytes) = &before {
            crate::config::atomic_write_private(
                &crate::opencode_config::get_opencode_dir().join("opencode.json.backup"),
                bytes,
            )
            .map_err(|_| "OpenCode backup failed")?;
        }
        if read_opencode_config_bytes().map_err(|_| "OpenCode configuration unavailable")? != before
        {
            return Err("OpenCode configuration changed before publication".into());
        }
        let written = write_opencode_config_value(&config).map_err(|_| "OpenCode write failed")?;
        if read_opencode_config_bytes()
            .map_err(|_| "OpenCode readback failed")?
            .as_deref()
            != Some(written.as_slice())
            || read_config()? != config
        {
            return Err("OpenCode subscription configuration readback failed".into());
        }
        Ok(())
    }

    pub(super) fn restore_opencode_managed(&self) -> Result<(), String> {
        let _config = crate::opencode_config::lock_opencode_config();
        let backup = self
            .db
            .get_live_backup_sync("opencode")
            .map_err(|_| "OpenCode recovery state unavailable")?;
        let Some(backup) = backup else {
            return if Self::is_opencode_live_taken_over(&read_config()?) {
                Err("OpenCode subscription recovery record is missing".into())
            } else {
                Ok(())
            };
        };
        let backup: OpenCodeProxyBackup = serde_json::from_str(&backup.original_config)
            .map_err(|_| "OpenCode recovery state unavailable")?;
        let current =
            read_opencode_config_bytes().map_err(|_| "OpenCode configuration unavailable")?;
        if backup.version != 1
            || (current != backup.original
                && current != backup.before
                && !matches_postimage(current.as_deref(), &backup))
        {
            return Err("OpenCode configuration changed; recovery requires review".into());
        }
        if current != backup.original {
            let owned_hash = current
                .as_deref()
                .map(|bytes| format!("{:x}", Sha256::digest(bytes)));
            crate::config::restore_file_preimage_if_owned(
                &crate::opencode_config::get_opencode_config_path(),
                backup.original.as_deref(),
                owned_hash.as_deref(),
            )
            .map_err(|_| "OpenCode restore failed")?;
        }
        if read_opencode_config_bytes().map_err(|_| "OpenCode restore readback failed")?
            != backup.original
        {
            return Err("OpenCode restore readback failed".into());
        }
        Ok(())
    }

    pub(super) fn is_opencode_live_taken_over(config: &Value) -> bool {
        config
            .get("provider")
            .and_then(Value::as_object)
            .is_some_and(|providers| {
                providers.iter().any(|(id, provider)| {
                    id.starts_with("fyagent-")
                        && provider.pointer("/options/apiKey").and_then(Value::as_str)
                            == Some(PROXY_TOKEN_PLACEHOLDER)
                        && provider
                            .pointer("/options/baseURL")
                            .and_then(Value::as_str)
                            .is_some_and(|url| {
                                url.ends_with("/opencode/v1") && is_local_proxy_url(url)
                            })
                })
            })
    }

    pub(super) fn opencode_matches_provider(
        &self,
        provider: &Provider,
        proxy_url: &str,
    ) -> Result<bool, String> {
        let config = read_config()?;
        let expected = projected_provider(provider, proxy_url)?;
        let model = selected_model(provider)?;
        Ok(config
            .get("provider")
            .and_then(|providers| providers.get(&provider.id))
            == Some(&expected)
            && config.get("model").and_then(Value::as_str)
                == Some(format!("{}/{model}", provider.id).as_str()))
    }
}
