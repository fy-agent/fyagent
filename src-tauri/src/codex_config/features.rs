use super::*;

pub const CODEX_IMAGE_EXTENSION_HEADER: &str = "x-openai-actor-authorization";
pub const CODEX_IMAGE_EXTENSION_VALUE: &str = "local-image-extension";
pub(super) const CODEX_FEATURE_INVALID_TOML: &str = "CODEX_FEATURE_INVALID_TOML";
const CODEX_FEATURE_INVALID_HEADER: &str = "CODEX_FEATURE_INVALID_HEADER";
pub(super) const CODEX_FEATURE_INVALID_WEBSOCKET: &str = "CODEX_FEATURE_INVALID_WEBSOCKET";
pub const CODEX_WEBSOCKET_NON_GPT_MODEL_WARNING: &str = "CODEX_WEBSOCKET_NON_GPT_MODEL";
pub const CODEX_WEBSOCKET_PROXY_MAY_BE_UNSUPPORTED_WARNING: &str =
    "CODEX_WEBSOCKET_PROXY_MAY_BE_UNSUPPORTED";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CodexConfigDiagnostic {
    pub code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum CodexImageExtensionState {
    On,
    Off,
    LegacyPendingOn,
    Conflict { key: String },
    Invalid { code: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CodexWebsocketFeatureState {
    pub enabled: bool,
    pub compatible: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CodexProviderFeatureState {
    pub applicable: bool,
    pub image_extension: CodexImageExtensionState,
    pub websockets: CodexWebsocketFeatureState,
    pub provider_table_found: bool,
    pub diagnostics: Vec<CodexConfigDiagnostic>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CodexProviderFeatureIntent {
    pub image_extension: Option<bool>,
    pub websockets: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CodexProviderFeaturePatchResult {
    pub toml_text: String,
    pub state: CodexProviderFeatureState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_extension_configured: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub codex_native_capabilities_generated_provider: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ManagedHeaderInspection {
    Missing,
    Controlled { key: String },
    Conflict { key: String },
    Invalid,
}

fn codex_feature_error(code: &'static str, zh: &'static str, en: &'static str) -> AppError {
    AppError::localized(code, zh, en)
}

fn codex_feature_diagnostic(code: &'static str, field: &'static str) -> CodexConfigDiagnostic {
    CodexConfigDiagnostic {
        code: code.to_owned(),
        field: Some(field.to_owned()),
    }
}

pub(super) fn codex_provider_config_text(provider: &Provider) -> &str {
    provider
        .settings_config
        .get("config")
        .and_then(Value::as_str)
        .unwrap_or_default()
}

pub(super) fn active_codex_provider_table(doc: &DocumentMut) -> Option<(String, &dyn TableLike)> {
    let provider_id = active_codex_model_provider_id(doc)?;
    let table = doc
        .get("model_providers")
        .and_then(Item::as_table_like)?
        .get(&provider_id)
        .and_then(Item::as_table_like)?;
    Some((provider_id, table))
}

fn active_codex_provider_table_mut<'a>(
    doc: &'a mut DocumentMut,
    provider_id: &str,
) -> Option<&'a mut dyn TableLike> {
    doc.get_mut("model_providers")
        .and_then(Item::as_table_like_mut)?
        .get_mut(provider_id)
        .and_then(Item::as_table_like_mut)
}

fn is_fixed_official_codex_provider(provider: &Provider) -> bool {
    provider
        .id
        .eq_ignore_ascii_case(crate::database::CODEX_OFFICIAL_PROVIDER_ID)
        || provider
            .category
            .as_deref()
            .is_some_and(|category| category.eq_ignore_ascii_case("official"))
}

fn generated_official_provider_marker(provider: &Provider) -> bool {
    provider
        .meta
        .as_ref()
        .and_then(|meta| meta.codex_native_capabilities_generated_provider)
        == Some(true)
}

pub(super) fn inspect_managed_image_header(
    provider_table: &dyn TableLike,
) -> ManagedHeaderInspection {
    let Some(headers_item) = provider_table.get("http_headers") else {
        return ManagedHeaderInspection::Missing;
    };
    let Some(headers) = headers_item.as_table_like() else {
        return ManagedHeaderInspection::Invalid;
    };

    let mut matching_keys = Vec::new();
    for (key, value) in headers.iter() {
        let Some(header_value) = value.as_str() else {
            return ManagedHeaderInspection::Invalid;
        };
        if key.eq_ignore_ascii_case(CODEX_IMAGE_EXTENSION_HEADER) {
            matching_keys.push((key.to_owned(), header_value == CODEX_IMAGE_EXTENSION_VALUE));
        }
    }

    match matching_keys.as_slice() {
        [] => ManagedHeaderInspection::Missing,
        [(key, true)] => ManagedHeaderInspection::Controlled { key: key.clone() },
        [(key, false)] => ManagedHeaderInspection::Conflict { key: key.clone() },
        [(key, _), ..] => ManagedHeaderInspection::Conflict { key: key.clone() },
    }
}

fn websocket_state(provider_table: &dyn TableLike) -> (bool, Option<CodexConfigDiagnostic>) {
    let Some(item) = provider_table.get("supports_websockets") else {
        return (false, None);
    };
    match item.as_bool() {
        Some(enabled) => (enabled, None),
        None => (
            false,
            Some(codex_feature_diagnostic(
                CODEX_FEATURE_INVALID_WEBSOCKET,
                "supportsWebsockets",
            )),
        ),
    }
}

fn image_extension_marker_is_complete(provider: &Provider) -> bool {
    provider
        .meta
        .as_ref()
        .and_then(|meta| meta.image_extension_configured)
        == Some(true)
}

fn normalize_unfinished_image_extension_marker(provider: &mut Provider) {
    if let Some(meta) = provider.meta.as_mut() {
        if meta.image_extension_configured == Some(false) {
            meta.image_extension_configured = None;
        }
    }
}

fn analyze_codex_provider_features_from_document(
    provider: &Provider,
    doc: &DocumentMut,
    is_new: bool,
) -> CodexProviderFeatureState {
    let is_official = is_fixed_official_codex_provider(provider);
    let image_extension_configured = image_extension_marker_is_complete(provider);
    let missing_image_state = || {
        if is_official {
            CodexImageExtensionState::Off
        } else if is_new && !image_extension_configured {
            CodexImageExtensionState::On
        } else if !image_extension_configured {
            CodexImageExtensionState::LegacyPendingOn
        } else {
            CodexImageExtensionState::Off
        }
    };

    let Some((_provider_id, provider_table)) = active_codex_provider_table(doc) else {
        return CodexProviderFeatureState {
            applicable: true,
            image_extension: missing_image_state(),
            websockets: CodexWebsocketFeatureState {
                enabled: false,
                compatible: true,
                reason: None,
            },
            provider_table_found: false,
            diagnostics: Vec::new(),
        };
    };

    let header = inspect_managed_image_header(provider_table);
    let (websocket_enabled, websocket_diagnostic) = websocket_state(provider_table);
    let mut diagnostics = Vec::new();
    if matches!(header, ManagedHeaderInspection::Invalid) {
        diagnostics.push(codex_feature_diagnostic(
            CODEX_FEATURE_INVALID_HEADER,
            "httpHeaders",
        ));
    }
    if let Some(diagnostic) = websocket_diagnostic {
        diagnostics.push(diagnostic);
    }

    let image_extension = match header {
        ManagedHeaderInspection::Missing => missing_image_state(),
        ManagedHeaderInspection::Controlled { .. } => CodexImageExtensionState::On,
        ManagedHeaderInspection::Conflict { key } => CodexImageExtensionState::Conflict { key },
        ManagedHeaderInspection::Invalid => CodexImageExtensionState::Invalid {
            code: CODEX_FEATURE_INVALID_HEADER.to_owned(),
        },
    };

    CodexProviderFeatureState {
        applicable: true,
        image_extension,
        websockets: CodexWebsocketFeatureState {
            enabled: websocket_enabled,
            compatible: true,
            reason: None,
        },
        provider_table_found: true,
        diagnostics,
    }
}

pub fn analyze_codex_provider_features(
    provider: &Provider,
    is_new: bool,
) -> CodexProviderFeatureState {
    let config = codex_provider_config_text(provider);
    match config.parse::<DocumentMut>() {
        Ok(doc) => analyze_codex_provider_features_from_document(provider, &doc, is_new),
        Err(_) => CodexProviderFeatureState {
            applicable: false,
            image_extension: CodexImageExtensionState::Invalid {
                code: CODEX_FEATURE_INVALID_TOML.to_owned(),
            },
            websockets: CodexWebsocketFeatureState {
                enabled: false,
                compatible: false,
                reason: None,
            },
            provider_table_found: false,
            diagnostics: vec![codex_feature_diagnostic(
                CODEX_FEATURE_INVALID_TOML,
                "config",
            )],
        },
    }
}

pub(super) fn set_managed_image_header(provider_table: &mut dyn TableLike, enabled: bool) {
    if matches!(
        inspect_managed_image_header(provider_table),
        ManagedHeaderInspection::Invalid
    ) {
        provider_table.remove("http_headers");
    }

    if provider_table.get("http_headers").is_none() {
        if enabled {
            let mut headers = toml_edit::InlineTable::new();
            headers.insert(
                CODEX_IMAGE_EXTENSION_HEADER,
                CODEX_IMAGE_EXTENSION_VALUE.into(),
            );
            provider_table.insert(
                "http_headers",
                Item::Value(toml_edit::Value::InlineTable(headers)),
            );
        }
        return;
    }

    let headers_empty = {
        let Some(headers) = provider_table
            .get_mut("http_headers")
            .and_then(Item::as_table_like_mut)
        else {
            return;
        };
        let matching_keys = headers
            .iter()
            .filter(|(key, _)| key.eq_ignore_ascii_case(CODEX_IMAGE_EXTENSION_HEADER))
            .map(|(key, _)| key.to_owned())
            .collect::<Vec<_>>();
        for key in matching_keys {
            headers.remove(&key);
        }
        if enabled {
            headers.insert(
                CODEX_IMAGE_EXTENSION_HEADER,
                toml_edit::value(CODEX_IMAGE_EXTENSION_VALUE),
            );
        }
        headers.is_empty()
    };

    if headers_empty {
        provider_table.remove("http_headers");
    }
}

pub(super) fn set_provider_config_text(
    provider: &mut Provider,
    config_text: String,
) -> Result<(), AppError> {
    let settings = provider.settings_config.as_object_mut().ok_or_else(|| {
        AppError::localized(
            "provider.codex.settings.not_object",
            "Codex 配置必须是 JSON 对象",
            "Codex configuration must be a JSON object",
        )
    })?;
    settings.insert("config".to_owned(), Value::String(config_text));
    Ok(())
}

fn feature_state_error(state: &CodexProviderFeatureState) -> Option<AppError> {
    if state
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == CODEX_FEATURE_INVALID_TOML)
    {
        return Some(codex_feature_error(
            CODEX_FEATURE_INVALID_TOML,
            "Codex TOML 配置无效，无法保存原生能力设置",
            "The Codex TOML configuration is invalid and native capabilities cannot be saved",
        ));
    }
    None
}

fn feature_state_error_for_patch(
    state: &CodexProviderFeatureState,
    _intent: &CodexProviderFeatureIntent,
) -> Option<AppError> {
    feature_state_error(state)
}

fn ensure_codex_feature_provider_table(
    doc: &mut DocumentMut,
    provider: &Provider,
) -> Result<(String, bool), AppError> {
    if let Some((provider_id, _)) = active_codex_provider_table(doc) {
        return Ok((provider_id, false));
    }

    let is_official = is_fixed_official_codex_provider(provider);
    let provider_id = if is_official {
        FYAGENT_CODEX_MODEL_PROVIDER_ID.to_owned()
    } else {
        active_codex_model_provider_id(doc)
            .unwrap_or_else(|| FYAGENT_CODEX_MODEL_PROVIDER_ID.to_owned())
    };

    if doc.get("model_providers").is_none() {
        let mut providers = toml_edit::Table::new();
        providers.set_implicit(true);
        doc["model_providers"] = Item::Table(providers);
    }
    let providers = doc
        .get_mut("model_providers")
        .and_then(Item::as_table_like_mut)
        .ok_or_else(|| {
            codex_feature_error(
                CODEX_FEATURE_INVALID_TOML,
                "Codex model_providers 必须是可编辑的表",
                "Codex model_providers must be an editable table",
            )
        })?;

    let created_provider_table = providers.get(&provider_id).is_none();
    if created_provider_table {
        let mut table = toml_edit::Table::new();
        table["name"] = toml_edit::value(if is_official {
            "OpenAI"
        } else {
            provider.name.trim()
        });
        if is_official {
            table["requires_openai_auth"] = toml_edit::value(true);
        }
        table["wire_api"] = toml_edit::value("responses");
        providers.insert(&provider_id, Item::Table(table));
    }

    doc["model_provider"] = toml_edit::value(&provider_id);
    Ok((provider_id, is_official && created_provider_table))
}

fn generated_official_provider_table_is_safe_to_remove(doc: &DocumentMut) -> bool {
    if active_codex_model_provider_id(doc).as_deref() != Some(FYAGENT_CODEX_MODEL_PROVIDER_ID) {
        return false;
    }
    let Some((_, table)) = active_codex_provider_table(doc) else {
        return false;
    };
    for (key, _) in table.iter() {
        if !matches!(key, "name" | "requires_openai_auth" | "wire_api") {
            return false;
        }
    }
    table.get("name").and_then(Item::as_str) == Some("OpenAI")
        && table.get("requires_openai_auth").and_then(Item::as_bool) == Some(true)
        && table.get("wire_api").and_then(Item::as_str) == Some("responses")
}

fn remove_generated_official_provider_table(doc: &mut DocumentMut) -> bool {
    if !generated_official_provider_table_is_safe_to_remove(doc) {
        return false;
    }

    doc.as_table_mut().remove("model_provider");
    let providers_empty = doc
        .get_mut("model_providers")
        .and_then(Item::as_table_like_mut)
        .is_some_and(|providers| {
            providers.remove(FYAGENT_CODEX_MODEL_PROVIDER_ID);
            providers.is_empty()
        });
    if providers_empty {
        doc.as_table_mut().remove("model_providers");
    }
    true
}

pub fn patch_codex_provider_features(
    provider: &Provider,
    intent: &CodexProviderFeatureIntent,
    is_new: bool,
) -> Result<CodexProviderFeaturePatchResult, AppError> {
    let config = codex_provider_config_text(provider);
    let mut doc = config.parse::<DocumentMut>().map_err(|_| {
        codex_feature_error(
            CODEX_FEATURE_INVALID_TOML,
            "Codex TOML 配置无效，无法修改原生能力设置",
            "The Codex TOML configuration is invalid and native capabilities cannot be changed",
        )
    })?;
    let state = analyze_codex_provider_features_from_document(provider, &doc, is_new);
    if let Some(error) = feature_state_error_for_patch(&state, intent) {
        return Err(error);
    }

    let needs_provider_table = intent.image_extension == Some(true)
        || intent.websockets == Some(true)
        || active_codex_provider_table(&doc).is_some();
    let mut generated_provider_marker = None;
    let provider_id = if needs_provider_table {
        let (provider_id, generated_official) =
            ensure_codex_feature_provider_table(&mut doc, provider)?;
        if generated_official {
            generated_provider_marker = Some(true);
        }
        Some(provider_id)
    } else {
        None
    };

    let is_official = is_fixed_official_codex_provider(provider);
    let image_extension_configured =
        (intent.image_extension.is_some() && !is_official).then_some(true);
    if let Some(provider_id) = provider_id.as_deref() {
        let provider_table =
            active_codex_provider_table_mut(&mut doc, provider_id).ok_or_else(|| {
                codex_feature_error(
                    CODEX_FEATURE_INVALID_TOML,
                    "Codex Provider 表不可编辑",
                    "The Codex provider table is not editable",
                )
            })?;
        if let Some(enabled) = intent.image_extension {
            set_managed_image_header(provider_table, enabled);
        }
        if let Some(enabled) = intent.websockets {
            if enabled {
                provider_table.insert("supports_websockets", toml_edit::value(true));
            } else {
                provider_table.remove("supports_websockets");
            }
        }
    }

    let owns_generated_official_table = is_official
        && (generated_official_provider_marker(provider)
            || generated_provider_marker == Some(true));
    if owns_generated_official_table && remove_generated_official_provider_table(&mut doc) {
        generated_provider_marker = Some(false);
    }

    let toml_text = doc.to_string();
    let parsed = toml_text.parse::<DocumentMut>().map_err(|_| {
        codex_feature_error(
            CODEX_FEATURE_INVALID_TOML,
            "Codex TOML 配置无效，无法修改原生能力设置",
            "The Codex TOML configuration is invalid and native capabilities cannot be changed",
        )
    })?;
    let mut patched_provider = provider.clone();
    set_provider_config_text(&mut patched_provider, toml_text.clone())?;
    if image_extension_configured.is_some() {
        patched_provider
            .meta
            .get_or_insert_with(ProviderMeta::default)
            .image_extension_configured = Some(true);
    }
    if let Some(generated) = generated_provider_marker {
        patched_provider
            .meta
            .get_or_insert_with(ProviderMeta::default)
            .codex_native_capabilities_generated_provider = generated.then_some(true);
    }
    Ok(CodexProviderFeaturePatchResult {
        state: analyze_codex_provider_features_from_document(&patched_provider, &parsed, is_new),
        toml_text,
        image_extension_configured,
        codex_native_capabilities_generated_provider: generated_provider_marker,
    })
}

pub fn validate_codex_provider_features(provider: &Provider) -> Result<(), AppError> {
    let state = analyze_codex_provider_features(provider, false);
    if let Some(error) = feature_state_error(&state) {
        return Err(error);
    }
    Ok(())
}

pub fn prepare_codex_provider_features_for_save(
    provider: &mut Provider,
    is_new: bool,
) -> Result<(), AppError> {
    normalize_unfinished_image_extension_marker(provider);
    if let Some(meta) = provider.meta.as_mut() {
        if meta.codex_native_capabilities_generated_provider == Some(false) {
            meta.codex_native_capabilities_generated_provider = None;
        }
    }
    validate_codex_provider_features(provider)?;
    let state = analyze_codex_provider_features(provider, is_new);
    if !state.applicable {
        return Ok(());
    }
    if matches!(
        state.image_extension,
        CodexImageExtensionState::Conflict { .. } | CodexImageExtensionState::Invalid { .. }
    ) {
        return Ok(());
    }

    let has_image_extension_marker = image_extension_marker_is_complete(provider);
    let should_apply_default = !is_fixed_official_codex_provider(provider)
        && ((is_new && !has_image_extension_marker)
            || matches!(
                state.image_extension,
                CodexImageExtensionState::LegacyPendingOn
            ));
    if should_apply_default {
        let result = patch_codex_provider_features(
            provider,
            &CodexProviderFeatureIntent {
                image_extension: Some(true),
                websockets: None,
            },
            false,
        )?;
        set_provider_config_text(provider, result.toml_text)?;
        provider
            .meta
            .get_or_insert_with(ProviderMeta::default)
            .image_extension_configured = Some(true);
        return Ok(());
    }

    if !is_fixed_official_codex_provider(provider)
        && matches!(state.image_extension, CodexImageExtensionState::On)
        && provider
            .meta
            .as_ref()
            .and_then(|meta| meta.image_extension_configured)
            != Some(true)
    {
        provider
            .meta
            .get_or_insert_with(ProviderMeta::default)
            .image_extension_configured = Some(true);
    }
    Ok(())
}

fn codex_provider_websocket_enabled(provider: &Provider) -> bool {
    codex_provider_config_text(provider)
        .parse::<DocumentMut>()
        .ok()
        .and_then(|doc| {
            active_codex_provider_table(&doc)
                .and_then(|(_, table)| table.get("supports_websockets").and_then(Item::as_bool))
        })
        == Some(true)
}

fn is_gpt_model_id(model: &str) -> Option<bool> {
    let model = model.trim();
    if model.is_empty() {
        return None;
    }
    let basename = model.rsplit('/').next().unwrap_or_default().trim();
    Some(basename.to_ascii_lowercase().starts_with("gpt-"))
}

pub fn codex_provider_save_warning_codes(
    provider: &Provider,
    proxy_takeover_active: bool,
) -> Vec<String> {
    if !codex_provider_websocket_enabled(provider) {
        return Vec::new();
    }

    let mut has_non_gpt_model = false;
    if let Ok(doc) = codex_provider_config_text(provider).parse::<DocumentMut>() {
        has_non_gpt_model = ["model", "review_model"].into_iter().any(|field| {
            doc.get(field)
                .and_then(Item::as_str)
                .and_then(is_gpt_model_id)
                == Some(false)
        });
    }
    if !has_non_gpt_model {
        has_non_gpt_model = provider
            .settings_config
            .get("modelCatalog")
            .and_then(|catalog| catalog.get("models"))
            .and_then(Value::as_array)
            .is_some_and(|models| {
                models.iter().any(|entry| {
                    entry
                        .get("model")
                        .and_then(Value::as_str)
                        .and_then(is_gpt_model_id)
                        == Some(false)
                })
            });
    }

    let mut warnings = Vec::new();
    if has_non_gpt_model {
        warnings.push(CODEX_WEBSOCKET_NON_GPT_MODEL_WARNING.to_owned());
    }
    if proxy_takeover_active {
        warnings.push(CODEX_WEBSOCKET_PROXY_MAY_BE_UNSUPPORTED_WARNING.to_owned());
    }
    warnings
}
