use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodexCatalogToolProfile {
    ProxyChat,
    NativeResponses,
    Anthropic,
}

impl CodexCatalogToolProfile {
    pub fn from_api_format(api_format: Option<&str>) -> Self {
        match api_format {
            Some("anthropic") => CodexCatalogToolProfile::Anthropic,
            Some("openai_responses") => CodexCatalogToolProfile::NativeResponses,
            _ => CodexCatalogToolProfile::ProxyChat,
        }
    }
}

pub(super) const MAX_CODEX_CATALOG_BYTES: u64 = 32 * 1024 * 1024;

pub fn read_codex_model_catalog_simplified_from_live() -> Result<Option<Value>, AppError> {
    let config_text = read_codex_config_text()?;
    let config_dir = get_codex_config_dir();
    let Some(catalog_path) = resolve_fyagent_catalog_path(&config_text, &config_dir) else {
        return Ok(None);
    };
    if !catalog_path.exists() {
        return Ok(None);
    }
    let catalog_text = match read_limited_string(&catalog_path, MAX_CODEX_CATALOG_BYTES) {
        Ok(text) => text,
        Err(error) => {
            log::warn!(
                "拒绝读取越界或过大的 Codex 模型目录 {}: {error}",
                catalog_path.display()
            );
            return Ok(None);
        }
    };
    Ok(build_simplified_catalog_from_texts(
        &config_text,
        &catalog_text,
    ))
}

pub(crate) fn read_limited_string(path: &Path, max_bytes: u64) -> Result<String, AppError> {
    let metadata = fs::metadata(path).map_err(|error| AppError::io(path, error))?;
    if metadata.len() > max_bytes {
        return Err(AppError::Config(format!(
            "文件 {} 超过大小上限 {} 字节",
            path.display(),
            max_bytes
        )));
    }
    fs::read_to_string(path).map_err(|error| AppError::io(path, error))
}

pub(crate) fn read_codex_model_catalog_text(path: &Path) -> Result<String, AppError> {
    read_limited_string(path, MAX_CODEX_CATALOG_BYTES)
}

pub(crate) fn resolve_fyagent_catalog_path(config_text: &str, base_dir: &Path) -> Option<PathBuf> {
    if config_text.trim().is_empty() {
        return None;
    }
    let doc = config_text.parse::<DocumentMut>().ok()?;
    let catalog_path_str = doc
        .get("model_catalog_json")
        .and_then(|item| item.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())?;

    let referenced_path = Path::new(catalog_path_str);
    let is_fyagent_owned = referenced_path.file_name().and_then(|name| name.to_str())
        == Some(FYAGENT_CODEX_MODEL_CATALOG_FILENAME);
    if !is_fyagent_owned {
        return None;
    }

    let is_unix_absolute = catalog_path_str.starts_with('/');
    let resolved = if referenced_path.is_absolute() || is_unix_absolute {
        referenced_path.to_path_buf()
    } else {
        base_dir.join(referenced_path)
    };

    if !path_is_within(base_dir, &resolved) {
        log::warn!(
            "Codex model_catalog_json 指向配置目录外: {}（允许目录: {}）",
            resolved.display(),
            base_dir.display()
        );
        return None;
    }

    if resolved.exists() {
        let canonical = match fs::canonicalize(&resolved) {
            Ok(path) => path,
            Err(error) => {
                log::warn!(
                    "Codex model_catalog_json canonicalize 失败: {}: {error}",
                    resolved.display()
                );
                return None;
            }
        };
        let canonical_base = fs::canonicalize(base_dir).unwrap_or_else(|_| base_dir.to_path_buf());
        if !path_is_within(&canonical_base, &canonical) {
            log::warn!(
                "Codex model_catalog_json 经符号链接解析到配置目录外: {} -> {}（允许目录: {}）",
                resolved.display(),
                canonical.display(),
                canonical_base.display()
            );
            return None;
        }
        return Some(canonical);
    }

    Some(resolved)
}

pub(super) fn build_simplified_catalog_from_texts(
    config_text: &str,
    catalog_text: &str,
) -> Option<Value> {
    let catalog: Value = serde_json::from_str(catalog_text).ok()?;
    let models = catalog.get("models").and_then(|m| m.as_array())?;
    let default_context_window =
        extract_codex_top_level_u64(config_text, "model_context_window").unwrap_or(128_000);

    let mut entries = Vec::with_capacity(models.len());
    for entry in models {
        let Some(model) = entry
            .get("slug")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
        else {
            continue;
        };

        let mut obj = serde_json::Map::new();
        obj.insert("model".to_string(), json!(model));
        if let Some(display_name) = entry
            .get("display_name")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty() && *s != model)
        {
            obj.insert("displayName".to_string(), json!(display_name));
        }
        if let Some(context_window) = entry
            .get("context_window")
            .and_then(|v| v.as_u64())
            .filter(|v| *v > 0 && *v != default_context_window)
        {
            obj.insert("contextWindow".to_string(), json!(context_window));
        }
        if let Some(parallel) = entry
            .get("supports_parallel_tool_calls")
            .and_then(|v| v.as_bool())
        {
            obj.insert("supportsParallelToolCalls".to_string(), json!(parallel));
        }
        if let Some(modalities) = entry.get("input_modalities").and_then(|v| v.as_array()) {
            let mods: Vec<String> = modalities
                .iter()
                .filter_map(|m| m.as_str())
                .map(str::to_string)
                .collect();
            let inferred = codex_catalog_input_modalities(model, None);
            if !mods.is_empty() && mods != inferred {
                obj.insert("inputModalities".to_string(), json!(mods));
            }
        }
        entries.push(Value::Object(obj));
    }

    (!entries.is_empty()).then(|| json!({ "models": entries }))
}
