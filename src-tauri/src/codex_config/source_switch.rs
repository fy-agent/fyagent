//! Lossless request-source changes. The selected provider owns its routing
//! table; all other provider tables, MCP entries, permissions, profiles and
//! credential-store choices stay with the live document.

use super::*;

fn invalid(message: &str) -> AppError {
    AppError::Config(message.to_string())
}

pub(crate) fn validate_source(
    category: Option<&str>,
    auth: &Value,
    desired_config: &str,
) -> Result<(), AppError> {
    let desired = desired_config
        .parse::<DocumentMut>()
        .map_err(|_| invalid("Codex 模型配置格式无效，未修改任何文件"))?;
    for key in ["model_provider", "model"] {
        if desired
            .get(key)
            .is_some_and(|value| value.as_str().is_none_or(|text| text.trim().is_empty()))
        {
            return Err(invalid("Codex 模型及来源必须是非空文本，未修改任何文件"));
        }
    }
    if category == Some("official") {
        if let Some(id) = active_codex_model_provider_id(&desired) {
            let native_auth = id == "openai"
                || active_codex_provider_table(&desired).is_some_and(|(_, table)| {
                    table.get("requires_openai_auth").and_then(Item::as_bool) == Some(true)
                });
            if !native_auth {
                return Err(invalid("官方模型来源包含不兼容的认证配置，请先检查此配置"));
            }
        }
        return Ok(());
    }
    let Some(id) = active_codex_model_provider_id(&desired) else {
        // A saved API-key configuration may intentionally use Codex's built-in
        // default provider. An empty/missing third-party setup is not that case.
        return if !desired_config.trim().is_empty()
            && extract_codex_api_key(Some(auth), Some(desired_config)).is_some()
        {
            Ok(())
        } else {
            Err(invalid("请先配置模型来源及认证 auth，未修改任何文件"))
        };
    };
    if !is_custom_codex_model_provider_id(&id) {
        // Built-in local/cloud providers own their own authentication methods;
        // do not invent a bearer key requirement for these reserved routes.
        return Ok(());
    }
    let (_, table) = active_codex_provider_table(&desired)
        .ok_or_else(|| invalid("第三方模型来源缺少提供商配置，未修改任何文件"))?;
    let has_auth = extract_codex_api_key(Some(auth), Some(desired_config)).is_some()
        || table.get("requires_openai_auth").and_then(Item::as_bool) == Some(true)
        || table
            .get("env_key")
            .and_then(Item::as_str)
            .is_some_and(|key| !key.trim().is_empty())
        || table.get("auth").and_then(Item::as_table_like).is_some();
    if !has_auth {
        return Err(invalid(
            "第三方模型来源缺少认证配置，请先保存 API Key 或配置认证方式",
        ));
    }
    Ok(())
}

pub(crate) fn patch_source(
    current_config: &str,
    category: Option<&str>,
    auth: &Value,
    desired_config: &str,
    config_dir: &Path,
    unify_sessions: bool,
) -> Result<String, AppError> {
    validate_source(category, auth, desired_config)?;
    let mut current = current_config
        .parse::<DocumentMut>()
        .map_err(|_| invalid("现有 Codex 配置格式无效，请先检查原文件"))?;
    let desired_text = if category == Some("official") {
        desired_config.to_string()
    } else {
        prepare_codex_provider_live_config(auth, desired_config)?
    };
    let desired = desired_text
        .parse::<DocumentMut>()
        .map_err(|_| invalid("Codex 模型配置格式无效"))?;
    let mut retained_comments = String::new();

    for key in [
        "model_provider",
        "model",
        "experimental_bearer_token",
        "base_url",
        "wire_api",
    ] {
        patch_owned_value(&mut current, key, desired.get(key), &mut retained_comments);
    }
    // These are model-specific saved choices. Absence is not permission to
    // reset a user's global preferences or other configuration tables.
    for key in [
        "review_model",
        "model_reasoning_effort",
        "model_reasoning_summary",
        "model_verbosity",
        "model_context_window",
        "model_auto_compact_token_limit",
        "web_search",
    ] {
        if let Some(value) = desired.get(key) {
            patch_owned_value(&mut current, key, Some(value), &mut retained_comments);
        }
    }
    if let Some(value) = desired.get("model_catalog_json") {
        current["model_catalog_json"] = value.clone();
    } else if resolve_fyagent_catalog_path(current_config, config_dir).is_some() {
        current.as_table_mut().remove("model_catalog_json");
    }

    if let Some(id) = active_codex_model_provider_id(&desired) {
        if is_custom_codex_model_provider_id(&id) {
            let selected = desired
                .get("model_providers")
                .and_then(Item::as_table_like)
                .and_then(|providers| providers.get(&id))
                .filter(|item| item.as_table_like().is_some())
                .ok_or_else(|| invalid("所选模型来源缺少提供商配置"))?;
            if current.get("model_providers").is_none() {
                current["model_providers"] = toml_edit::table();
            }
            let providers = current
                .get_mut("model_providers")
                .and_then(Item::as_table_like_mut)
                .ok_or_else(|| invalid("现有 Codex model_providers 不是可编辑的表"))?;
            providers.insert(&id, selected.clone());
        }
    }
    let patched = format!("{retained_comments}{current}");
    if category == Some("official") && unify_sessions {
        // Check the real live tables before injecting `custom`; an existing
        // unrelated custom provider must never be repurposed as official.
        inject_codex_unified_session_bucket(&patched)
    } else {
        Ok(patched)
    }
}

fn patch_owned_value(
    document: &mut DocumentMut,
    key: &str,
    desired: Option<&Item>,
    retained_comments: &mut String,
) {
    if let Some(desired) = desired {
        let mut replacement = desired.clone();
        if let (Some(previous), Some(next)) = (
            document.get(key).and_then(Item::as_value),
            replacement.as_value_mut(),
        ) {
            *next.decor_mut() = previous.decor().clone();
        }
        document[key] = replacement;
    } else if let Some((removed_key, removed_value)) = document.as_table_mut().remove_entry(key) {
        // toml_edit attaches leading comments to the key being removed. Keep
        // those comments (and an inline note) rather than deleting user prose.
        for raw in [
            removed_key.leaf_decor().prefix(),
            removed_value
                .as_value()
                .and_then(|value| value.decor().suffix()),
        ] {
            if let Some(text) = raw
                .and_then(|raw| raw.as_str())
                .filter(|text| text.contains('#'))
            {
                retained_comments.push_str(text);
                if !text.ends_with('\n') {
                    retained_comments.push('\n');
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CURRENT: &str = "# User header\nmodel_provider = 'custom'\nmodel = 'old-model'\ncli_auth_credentials_store = 'keyring'\n[features]\nmulti_agent = true\n[mcp_servers.user]\ncommand = 'keep-user-mcp'\n[model_providers.custom]\nname = 'User custom'\nbase_url = 'https://old.example/v1'\nexperimental_bearer_token = 'old-fixture'\n[model_providers.unrelated]\nname = 'Unrelated'\nbase_url = 'https://untouched.example/v1'\n[profiles.personal]\nmodel = 'personal-model'\n";

    #[test]
    fn empty_official_template_preserves_unowned_configuration_and_custom_table() {
        let dir = tempfile::tempdir().unwrap();
        let result =
            patch_source(CURRENT, Some("official"), &json!({}), "", dir.path(), true).unwrap();
        let before = CURRENT.parse::<toml::Value>().unwrap();
        let after = result.parse::<toml::Value>().unwrap();
        assert!(result.contains("# User header"));
        for key in [
            "features",
            "mcp_servers",
            "model_providers",
            "profiles",
            "cli_auth_credentials_store",
        ] {
            assert_eq!(after.get(key), before.get(key), "{key}");
        }
        assert!(after.get("model_provider").is_none());
        assert!(after.get("model").is_none());
    }

    #[test]
    fn third_party_source_changes_only_the_selected_provider_and_model_fields() {
        let dir = tempfile::tempdir().unwrap();
        let desired = "model_provider = 'new-api'\nmodel = 'new-model'\n[model_providers.new-api]\nname = 'New API'\nbase_url = 'https://new.example/v1'\nwire_api = 'responses'\nrequires_openai_auth = false\n";
        let result = patch_source(
            CURRENT,
            None,
            &json!({"OPENAI_API_KEY":"new-fixture"}),
            desired,
            dir.path(),
            false,
        )
        .unwrap();
        let before = CURRENT.parse::<toml::Value>().unwrap();
        let after = result.parse::<toml::Value>().unwrap();
        assert!(result.contains("# User header"));
        for key in [
            "features",
            "mcp_servers",
            "profiles",
            "cli_auth_credentials_store",
        ] {
            assert_eq!(after.get(key), before.get(key), "{key}");
        }
        for id in ["custom", "unrelated"] {
            assert_eq!(after["model_providers"][id], before["model_providers"][id]);
        }
        assert_eq!(after["model"].as_str(), Some("new-model"));
        assert_eq!(
            after["model_providers"]["new-api"]["experimental_bearer_token"].as_str(),
            Some("new-fixture")
        );
    }

    #[test]
    fn missing_third_party_config_or_credentials_are_rejected_before_projection() {
        assert!(validate_source(None, &json!({}), "").is_err());
        assert!(validate_source(None, &json!({}), "model_provider = 'absent'").is_err());
        let no_key = "model_provider = 'api'\n[model_providers.api]\nname = 'API'\nbase_url = 'https://example.test/v1'\n";
        assert!(validate_source(None, &json!({}), no_key).is_err());
        assert!(validate_source(
            None,
            &json!({}),
            &format!("{no_key}env_key = 'USER_MANAGED_KEY'\n")
        )
        .is_ok());
    }
}
