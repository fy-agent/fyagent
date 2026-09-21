//! API-key backup admission. Keep the historical DB format, but require the
//! existing path-bound writer receipt and an explained takeover projection.

use super::*;
use sha2::{Digest, Sha256};
use std::path::PathBuf;

const CONFLICT: &str =
    "无法确认代理配置仍可安全恢复；已保留恢复记录。请先退出相关软件，对比现有配置与备份，保留后续改动后再恢复。";

fn path(app: &AppType) -> Result<PathBuf, String> {
    match app {
        AppType::Claude => Ok(get_claude_settings_path()),
        AppType::Codex => Ok(crate::codex_config::get_codex_config_path()),
        AppType::Gemini => Ok(crate::gemini_config::get_gemini_env_path()),
        AppType::GrokBuild => Ok(crate::grok_config::get_grok_config_path()),
        _ => Err(CONFLICT.into()),
    }
}

fn text(bytes: &[u8]) -> Result<&str, String> {
    std::str::from_utf8(bytes).map_err(|_| CONFLICT.into())
}

fn toml_equal(left: &str, right: &str) -> bool {
    match (left.parse::<toml::Value>(), right.parse::<toml::Value>()) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}

fn codex_catalog_fields(
    settings: &Value,
    config: &str,
    profile: crate::codex_config::CodexCatalogToolProfile,
) -> Result<String, String> {
    use crate::codex_config::*;
    if settings.get("modelCatalog").is_none() {
        return Ok(config.to_owned());
    }
    // Use the catalog owner's pure field transformations. Admission must not
    // generate a catalog, discover a CLI template or write a native file.
    let has_catalog = codex_model_catalog_write_required(settings);
    let catalog_path = get_codex_model_catalog_path();
    let config =
        set_codex_model_catalog_json_field(config, has_catalog.then_some(catalog_path.as_path()))
            .map_err(|_| CONFLICT)?;
    let disable = profile == CodexCatalogToolProfile::Anthropic
        || (has_catalog
            && profile == CodexCatalogToolProfile::NativeResponses
            && codex_native_gateway_rejects_web_search(&config));
    set_codex_native_web_search_field(&config, disable).map_err(|_| CONFLICT.into())
}

fn original_matches(app: &AppType, original: &Value, bytes: &[u8]) -> bool {
    match app {
        AppType::Claude => serde_json::from_slice::<Value>(bytes).is_ok_and(|value| {
            value == crate::services::provider::sanitize_claude_settings_for_live(original)
                || (value.is_null() && original.as_object().is_some_and(|object| object.is_empty()))
        }),
        AppType::Gemini => text(bytes).is_ok_and(|text| {
            crate::gemini_config::json_to_env(original)
                .is_ok_and(|env| env == crate::gemini_config::parse_env_file(text))
        }),
        AppType::Codex => {
            let Some(config) = original.get("config").and_then(Value::as_str) else {
                return false;
            };
            let prepared = codex_catalog_fields(
                original,
                config,
                crate::codex_config::CodexCatalogToolProfile::ProxyChat,
            )
            .and_then(|config| {
                crate::codex_config::prepare_codex_provider_live_config(
                    original.get("auth").unwrap_or(&Value::Null),
                    &config,
                )
                .map_err(|_| CONFLICT.to_owned())
            });
            prepared
                .is_ok_and(|expected| text(bytes).is_ok_and(|actual| toml_equal(&expected, actual)))
        }
        AppType::GrokBuild => original
            .get("config")
            .and_then(Value::as_str)
            .is_some_and(|expected| text(bytes).is_ok_and(|actual| toml_equal(expected, actual))),
        _ => false,
    }
}

fn proxy_url(app: &AppType, bytes: &[u8]) -> Option<String> {
    let text = std::str::from_utf8(bytes).ok()?;
    let url = match app {
        AppType::Claude => serde_json::from_str::<Value>(text)
            .ok()?
            .pointer("/env/ANTHROPIC_BASE_URL")?
            .as_str()?
            .to_owned(),
        AppType::Codex => crate::codex_config::extract_codex_base_url(text)?,
        AppType::Gemini => crate::gemini_config::parse_env_file(text)
            .get("GOOGLE_GEMINI_BASE_URL")?
            .clone(),
        AppType::GrokBuild => crate::grok_config::extract_base_url(text)?,
        _ => return None,
    };
    let parsed = url::Url::parse(&url).ok()?;
    (is_local_proxy_url(&url)
        && parsed.scheme() == "http"
        && parsed.username().is_empty()
        && parsed.password().is_none()
        && parsed.query().is_none()
        && parsed.fragment().is_none())
    .then_some(url)
}

fn native_config(app: &AppType, bytes: &[u8]) -> Result<Value, String> {
    match app {
        AppType::Claude => serde_json::from_slice(bytes).map_err(|_| CONFLICT.into()),
        AppType::Gemini => Ok(crate::gemini_config::env_to_json(
            &crate::gemini_config::parse_env_file(text(bytes)?),
        )),
        AppType::Codex | AppType::GrokBuild => Ok(json!({ "config": text(bytes)?, "auth": {} })),
        _ => Err(CONFLICT.into()),
    }
}

enum LegacyRestorePlan {
    Unchanged(Vec<PathBuf>),
    Exact {
        path: PathBuf,
        preimage: Option<Vec<u8>>,
        expected_hash: Option<String>,
    },
    Source(Vec<(PathBuf, Option<String>)>),
}

impl LegacyRestorePlan {
    fn paths(&self) -> Vec<PathBuf> {
        match self {
            Self::Unchanged(paths) => paths.clone(),
            Self::Exact { path, .. } => vec![path.clone()],
            Self::Source(expected) => expected.iter().map(|(path, _)| path.clone()).collect(),
        }
    }
}

fn closed_legacy_restore_paths(app: &AppType, path: PathBuf, original: &Value) -> Vec<PathBuf> {
    let mut paths = vec![path];
    if *app == AppType::Codex && crate::codex_config::codex_model_catalog_write_required(original) {
        paths.push(crate::codex_config::get_codex_model_catalog_path());
    }
    paths
}

impl ProxyService {
    pub(super) fn legacy_restore_preview_paths(
        &self,
        app: &AppType,
        original: &Value,
    ) -> Result<Vec<PathBuf>, String> {
        Ok(self.legacy_restore_plan(app, original)?.paths())
    }

    fn legacy_restore_plan(
        &self,
        app: &AppType,
        original: &Value,
    ) -> Result<LegacyRestorePlan, String> {
        let path = path(app)?;
        let metadata = std::fs::symlink_metadata(&path).map_err(|_| CONFLICT)?;
        if !metadata.is_file()
            || metadata.file_type().is_symlink()
            || metadata.len() > 64 * 1024 * 1024
        {
            return Err(CONFLICT.into());
        }
        let current = std::fs::read(&path).map_err(|_| CONFLICT)?;
        // A previous exit may have restored this target before another target
        // failed. Do not rotate its backup or rewrite equivalent external bytes.
        if original_matches(app, original, &current) {
            return Ok(LegacyRestorePlan::Unchanged(closed_legacy_restore_paths(
                app, path, original,
            )));
        }
        let receipt = crate::config::verified_file_recovery(&path)
            .map_err(|_| CONFLICT)?
            .ok_or(CONFLICT)?;
        if receipt.postimage_sha256.as_deref()
            != Some(format!("{:x}", Sha256::digest(&current)).as_str())
        {
            return Err(CONFLICT.into());
        }
        let url = proxy_url(app, &current).ok_or(CONFLICT)?;
        let preimage_matches = receipt
            .preimage
            .as_deref()
            .is_some_and(|bytes| original_matches(app, original, bytes));
        // A later provider switch may advance the rolling receipt. It must
        // retain the previously owned endpoint; a new loopback URL cannot
        // declare itself trusted solely by appearing in the current file.
        let unchanged_projection = receipt
            .preimage
            .as_deref()
            .and_then(|bytes| native_config(app, bytes).ok())
            .is_some_and(|before| {
                self.legacy_takeover_projection_matches(app, &before, &current, &url)
                    .unwrap_or(false)
            });
        if !preimage_matches
            && !unchanged_projection
            && receipt
                .preimage
                .as_deref()
                .and_then(|bytes| proxy_url(app, bytes))
                .as_deref()
                != Some(url.as_str())
        {
            return Err(CONFLICT.into());
        }
        if !self
            .legacy_takeover_projection_matches(app, original, &current, &url)
            .map_err(|_| CONFLICT)?
        {
            return Err(CONFLICT.into());
        }
        if preimage_matches {
            return Ok(LegacyRestorePlan::Exact {
                path,
                preimage: receipt.preimage,
                expected_hash: receipt.postimage_sha256,
            });
        }
        let mut expected = vec![(path, receipt.postimage_sha256)];
        if *app == AppType::Codex
            && crate::codex_config::codex_model_catalog_write_required(original)
        {
            let catalog = crate::codex_config::get_codex_model_catalog_path();
            let receipt = crate::config::verified_file_recovery(&catalog).map_err(|_| CONFLICT)?;
            expected.push((catalog, receipt.and_then(|item| item.postimage_sha256)));
        }
        Ok(LegacyRestorePlan::Source(expected))
    }

    pub(super) fn restore_legacy_config_if_owned(
        &self,
        app: &AppType,
        original: &Value,
    ) -> Result<(), String> {
        let plan = self.legacy_restore_plan(app, original)?;
        if matches!(plan, LegacyRestorePlan::Unchanged(_)) {
            return Ok(());
        }
        #[cfg(test)]
        self.observe_managed_activation(app, "legacy_restore_admitted")?;
        let expected = match plan {
            LegacyRestorePlan::Unchanged(_) => return Ok(()),
            LegacyRestorePlan::Exact {
                path,
                preimage,
                expected_hash,
            } => {
                return crate::config::restore_file_preimage_if_owned(
                    &path,
                    preimage.as_deref(),
                    expected_hash.as_deref(),
                )
                .map_err(|_| CONFLICT.into());
            }
            LegacyRestorePlan::Source(expected) => expected,
        };
        let _guard = crate::config::file_restore_scope(expected).map_err(|_| CONFLICT)?;
        let _operation = crate::config::file_mutation_scope();
        if *app == AppType::Codex {
            let mut original = original.clone();
            if original
                .get("auth")
                .is_some_and(crate::codex_config::codex_auth_has_oauth_login_material)
            {
                original.as_object_mut().ok_or(CONFLICT)?.remove("auth");
            }
            self.write_codex_live_verbatim(&original)
                .map_err(|_| CONFLICT.to_owned())
        } else {
            self.write_live_config_for_app(app, original)
                .map_err(|_| CONFLICT.to_owned())
        }
    }

    fn legacy_takeover_projection_matches(
        &self,
        app: &AppType,
        original: &Value,
        current: &[u8],
        url: &str,
    ) -> Result<bool, String> {
        let mut projected = original.clone();
        // This projection is also used by preview: selection must not repair
        // preferences or turn stale state into new recovery authority.
        let provider = self.current_provider_for_restore_preview(app)?;
        match app {
            AppType::Claude => {
                if let Some(provider) = provider.as_ref() {
                    Self::apply_claude_takeover_fields_for_provider(&mut projected, url, provider);
                } else {
                    Self::apply_claude_takeover_fields_with_policy(
                        &mut projected,
                        url,
                        ClaudeTakeoverAuthPolicy::PreserveExistingOrAuthToken,
                    );
                }
                Ok(original_matches(app, &projected, current))
            }
            AppType::Gemini => {
                let env = projected
                    .as_object_mut()
                    .ok_or(CONFLICT)?
                    .entry("env")
                    .or_insert_with(|| json!({}))
                    .as_object_mut()
                    .ok_or(CONFLICT)?;
                env.insert("GOOGLE_GEMINI_BASE_URL".into(), json!(url));
                env.insert("GEMINI_API_KEY".into(), json!(PROXY_TOKEN_PLACEHOLDER));
                Ok(original_matches(app, &projected, current))
            }
            AppType::GrokBuild => {
                Self::apply_grok_takeover_fields(&mut projected, url)?;
                Ok(original_matches(app, &projected, current))
            }
            AppType::Codex => {
                Self::apply_codex_takeover_auth_placeholder(&mut projected, provider.as_ref());
                let config = projected
                    .get("config")
                    .and_then(Value::as_str)
                    .ok_or(CONFLICT)?;
                let config = Self::apply_codex_proxy_toml_config_for_provider(
                    config,
                    url,
                    provider.as_ref(),
                )?;
                Self::attach_codex_model_catalog_from_provider(&mut projected, provider.as_ref());
                let profile = provider
                    .as_ref()
                    .map(crate::proxy::providers::resolve_codex_catalog_tool_profile)
                    .unwrap_or(crate::codex_config::CodexCatalogToolProfile::ProxyChat);
                let config = codex_catalog_fields(&projected, &config, profile)?;
                let config = crate::codex_config::prepare_codex_provider_live_config(
                    projected.get("auth").unwrap_or(&Value::Null),
                    &config,
                )
                .map_err(|_| CONFLICT)?;
                self.codex_legacy_projection_equal(&config, text(current)?, url)
            }
            _ => Err(CONFLICT.into()),
        }
    }

    pub(super) fn restore_legacy_receipt_preimage(&self, app: &AppType) -> Result<(), String> {
        let receipt = crate::config::verified_file_recovery(&path(app)?)
            .map_err(|_| CONFLICT)?
            .ok_or(CONFLICT)?;
        let bytes = receipt.preimage.as_deref().ok_or(CONFLICT)?;
        let original = native_config(app, bytes)?;
        if Self::live_has_proxy_placeholder_for_app(app, &original) {
            return Err(
                "没有可验证的原连接配置，已保留当前文件和备份；请对比备份并恢复可用的模型来源。"
                    .into(),
            );
        }
        self.restore_legacy_config_if_owned(app, &original)
    }

    fn codex_legacy_projection_equal(
        &self,
        expected: &str,
        current: &str,
        url: &str,
    ) -> Result<bool, String> {
        let expected: toml::Value = expected.parse().map_err(|_| CONFLICT)?;
        let mut current: toml::Value = current.parse().map_err(|_| CONFLICT)?;
        if expected == current {
            return Ok(true);
        }
        let Some(expected_tables) = expected
            .get("model_providers")
            .and_then(toml::Value::as_table)
        else {
            return Ok(false);
        };
        let Some(current_tables) = current
            .get_mut("model_providers")
            .and_then(toml::Value::as_table_mut)
        else {
            return Ok(false);
        };
        let saved = self.db.get_all_providers("codex").map_err(|_| CONFLICT)?;
        // Source switching retains inactive provider tables. Explain each
        // extra table from a saved provider; never ignore arbitrary live TOML.
        let extras = current_tables
            .keys()
            .filter(|id| !expected_tables.contains_key(*id))
            .cloned()
            .collect::<Vec<_>>();
        for id in extras {
            let explained = saved
                .values()
                .filter(|provider| !provider.uses_subscription_proxy())
                .any(|provider| {
                    let mut settings = provider.settings_config.clone();
                    if Self::apply_codex_takeover_fields_for_provider(&mut settings, url, provider)
                        .is_err()
                    {
                        return false;
                    }
                    let Some(config) = settings.get("config").and_then(Value::as_str) else {
                        return false;
                    };
                    let Ok(config) = crate::codex_config::prepare_codex_provider_live_config(
                        settings.get("auth").unwrap_or(&Value::Null),
                        config,
                    ) else {
                        return false;
                    };
                    config.parse::<toml::Value>().is_ok_and(|config| {
                        config
                            .get("model_providers")
                            .and_then(|tables| tables.get(&id))
                            == current_tables.get(&id)
                    })
                });
            if !explained {
                return Ok(false);
            }
            current_tables.remove(&id);
        }
        Ok(current == expected)
    }
}
