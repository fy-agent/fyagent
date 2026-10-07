use super::*;
use crate::config::{get_home_dir, path_is_within, write_json_file};
use crate::model_capabilities::{image_input_capability_from_modalities, ImageInputCapability};
use once_cell::sync::OnceCell;
use std::collections::HashSet;
use std::process::{Command, Stdio};

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

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[cfg(not(test))]
static CODEX_MODEL_CATALOG_TEMPLATE_CACHE: OnceCell<Value> = OnceCell::new();

pub(crate) const CODEX_WEB_SEARCH_FIELD: &str = "web_search";
pub(crate) const CODEX_WEB_SEARCH_DISABLED: &str = "disabled";

const CODEX_WEB_SEARCH_REJECT_HOSTS: &[&str] = &[
    "xiaomimimo.com", // Xiaomi MiMo (api.xiaomimimo.com, token-plan-cn.xiaomimimo.com)
    "longcat.chat",   // Meituan LongCat (api.longcat.chat)
    "minimax.io",     // MiniMax global (api.minimax.io)
    "minimax.cn",     // MiniMax CN (current official endpoint)
    "minimaxi.com",   // MiniMax CN (legacy endpoint)
    // StepFun Responses API currently supports only `function` tools:
    // platform.stepfun.com/docs/zh/api-reference/responses/responses-create
    "stepfun.com",
    "stepfun.ai",
    // Conservative (unverified, not a confirmed reject): Baidu Qianfan's
    // pay-as-you-go Responses guide documents only `function` / `mcp` tools
    // (cloud.baidu.com/doc/qianfan-docs/s/4mi400l1m). Host-exact; Qianfan's
    // Chat plans on the same domain are ProxyChat and never consult this list.
    "qianfan.baidubce.com",
    // Conservative (unverified): iFlytek Astron Coding Plan fronts third-party
    // models behind one Responses gateway with no documented hosted-tool
    // support (www.xfyun.cn/doc/spark/CodingPlan.html).
    "xf-yun.com",
    // Zhipu GLM CN / global (open.bigmodel.cn, api.z.ai): the native Responses
    // gateway's tool-type enum is `function | web_search_preview |
    // code_interpreter | mcp` (verbatim from the #6944 400 body) — Codex's
    // `web_search` hosted tool is not in it. Matched on host labels (see
    // `codex_url_host_matches_any`), so `xyz.ai` never collides with `z.ai`.
    "bigmodel.cn",
    "z.ai",
];

const CODEX_WEB_SEARCH_REJECT_MODEL_PREFIXES: &[&str] =
    &["mimo", "longcat", "minimax", "qwen3-coder", "glm"];

pub(crate) fn codex_top_level_model(config_text: &str) -> Option<String> {
    let doc = config_text.parse::<toml::Value>().ok()?;
    doc.get("model")
        .and_then(|value| value.as_str())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub(crate) fn codex_native_gateway_rejects_web_search(config_text: &str) -> bool {
    if let Some(base_url) = extract_codex_base_url(config_text) {
        if codex_url_host_matches_any(&base_url, CODEX_WEB_SEARCH_REJECT_HOSTS) {
            return true;
        }
    }
    codex_top_level_model(config_text).is_some_and(|model| codex_model_rejects_web_search(&model))
}

const CODEX_MODEL_CATALOG_TEMPLATE_SLUG: &str = "gpt-5.5";

pub(super) fn parse_codex_positive_u64(value: Option<&Value>) -> Option<u64> {
    match value {
        Some(Value::Number(n)) => n.as_u64().filter(|v| *v > 0),
        Some(Value::String(s)) => s.trim().parse::<u64>().ok().filter(|v| *v > 0),
        _ => None,
    }
}

pub(super) fn extract_codex_top_level_u64(config_text: &str, field: &str) -> Option<u64> {
    let doc = config_text.parse::<toml::Value>().ok()?;
    doc.get(field)
        .and_then(|value| value.as_integer())
        .and_then(|value| u64::try_from(value).ok())
        .filter(|value| *value > 0)
}

pub(super) fn codex_catalog_input_modalities(
    model: &str,
    declared_modalities: Option<&[String]>,
) -> Vec<String> {
    let modalities = match image_input_capability_from_modalities(model, declared_modalities) {
        ImageInputCapability::Unsupported => &["text"][..],
        ImageInputCapability::Supported | ImageInputCapability::Unknown => &["text", "image"][..],
    };
    modalities.iter().map(|item| (*item).to_string()).collect()
}

/// Canonical reasoning effort levels Codex understands, with the same
/// descriptions the official gpt-5.5 template uses. `none` disables thinking.
const CODEX_REASONING_LEVEL_DESCRIPTIONS: &[(&str, &str)] = &[
    ("none", "Disable Thinking"),
    ("minimal", "Minimal reasoning"),
    ("low", "Fast responses with lighter reasoning"),
    (
        "medium",
        "Balances speed and reasoning depth for everyday tasks",
    ),
    ("high", "Greater reasoning depth for complex problems"),
    ("xhigh", "Extra high reasoning depth for complex problems"),
    ("max", "Maximum reasoning depth for the hardest problems"),
    ("ultra", "Ultra reasoning depth"),
];

pub(super) fn codex_reasoning_level_description(effort: &str) -> Option<&'static str> {
    CODEX_REASONING_LEVEL_DESCRIPTIONS
        .iter()
        .find(|(candidate, _)| *candidate == effort)
        .map(|(_, description)| *description)
}

/// User-declared levels reduced to the canonical efforts Codex understands,
/// in canonical (lowest → highest) order regardless of declaration order.
/// Unknown efforts are dropped so a typo can never produce an entry Codex
/// would reject.
pub(super) fn codex_canonical_efforts(levels: &[String]) -> Vec<&str> {
    CODEX_REASONING_LEVEL_DESCRIPTIONS
        .iter()
        .filter(|(effort, _)| levels.iter().any(|candidate| candidate == effort))
        .map(|(effort, _)| *effort)
        .collect()
}

/// Build a `supported_reasoning_levels` array from user-declared effort values.
pub(super) fn codex_supported_reasoning_levels(levels: &[String]) -> Value {
    let entries: Vec<Value> = codex_canonical_efforts(levels)
        .into_iter()
        .map(|effort| {
            let description = codex_reasoning_level_description(effort)
                .expect("canonical effort always has a description");
            json!({ "effort": effort, "description": description })
        })
        .collect();
    json!(entries)
}

/// Apply a per-model reasoning-level override onto a catalog entry. Returns
/// true when the override was applied (so callers can skip further work).
/// `template_default` is the base entry's `default_reasoning_level` (from the
/// profile template or an official vendor entry) used as the fallback when the
/// user did not declare one explicitly.
pub(super) fn apply_codex_reasoning_level_override(
    entry_obj: &mut serde_json::Map<String, Value>,
    template_default: Option<&str>,
    spec: &CodexCatalogModelSpec,
) -> bool {
    let Some(levels) = spec.reasoning_levels.as_deref() else {
        return false;
    };
    let canonical = codex_canonical_efforts(levels);
    if canonical.is_empty() {
        return false;
    }
    let supported = codex_supported_reasoning_levels(levels);
    entry_obj.insert("supported_reasoning_levels".to_string(), supported);

    // Default: explicit user value wins; otherwise keep the base default when
    // it is still supported; otherwise fall back to the highest supported
    // level in canonical order. All candidates are validated against the
    // canonical set so the default can never reference a dropped effort.
    let default_level = spec
        .default_reasoning_level
        .as_deref()
        .filter(|level| canonical.contains(level))
        .or_else(|| template_default.filter(|level| canonical.contains(level)))
        .or_else(|| canonical.last().copied());
    if let Some(default_level) = default_level {
        entry_obj.insert("default_reasoning_level".to_string(), json!(default_level));
    }
    true
}

pub(super) fn codex_catalog_model_entry(
    template: &Value,
    spec: &CodexCatalogModelSpec,
    priority: usize,
    profile: CodexCatalogToolProfile,
    default_context_window: u64,
) -> Value {
    let mut entry = template.clone();
    let Some(entry_obj) = entry.as_object_mut() else {
        return json!({});
    };

    let display_name = spec.display_name.as_deref().unwrap_or(&spec.model);
    let context_window = spec.context_window.unwrap_or(default_context_window);
    entry_obj.insert("slug".to_string(), json!(spec.model));
    entry_obj.insert("display_name".to_string(), json!(display_name));
    entry_obj.insert("description".to_string(), json!(display_name));
    entry_obj.insert("context_window".to_string(), json!(context_window));
    entry_obj.insert("max_context_window".to_string(), json!(context_window));
    entry_obj.insert("priority".to_string(), json!(1000 + priority));
    entry_obj.insert("additional_speed_tiers".to_string(), json!([]));
    entry_obj.insert("service_tiers".to_string(), json!([]));
    entry_obj.insert("availability_nux".to_string(), Value::Null);
    entry_obj.insert("upgrade".to_string(), Value::Null);

    // Image support is a model capability, not a tool-profile capability.
    // Trust hidden preset metadata first, then the confirmed text-only registry;
    // every unknown model fails open so GPT/relay aliases are never declared
    // text-only merely because a template had a conservative default.
    entry_obj.insert(
        "input_modalities".to_string(),
        json!(codex_catalog_input_modalities(
            &spec.model,
            spec.input_modalities.as_deref(),
        )),
    );

    if profile != CodexCatalogToolProfile::ProxyChat {
        // Native `/responses` and Anthropic gateways reject / drop Codex's freeform
        // `apply_patch` (type=="custom") tool. Strip any key that would make Codex
        // emit a custom/freeform tool, and rely on shell_type="shell_command" for
        // edits. Defensive even though the native template is already clean
        // (guards against template drift / an accidental gpt-5.5 clone).
        //
        // NOTE: `base_instructions` is NOT stripped — Codex's catalog parser
        // treats it as a REQUIRED field and refuses to load the file without
        // it ("missing field `base_instructions`"). The template carries a
        // neutral identity default; per-vendor official text overrides below.
        for key in [
            "apply_patch_tool_type",
            "web_search_tool_type",
            "tools",
            "model_messages",
        ] {
            entry_obj.remove(key);
        }
        entry_obj.insert("shell_type".to_string(), json!("shell_command"));

        if let Some(base_instructions) = spec
            .base_instructions
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            entry_obj.insert("base_instructions".to_string(), json!(base_instructions));
        }
        if let Some(parallel) = spec.supports_parallel_tool_calls {
            entry_obj.insert("supports_parallel_tool_calls".to_string(), json!(parallel));
        }
    }

    if profile == CodexCatalogToolProfile::ProxyChat {
        // Codex's `original` image detail (full-resolution) is rejected by
        // strict Chat gateways with `400 invalid_request_error`, param
        // `messages.N.content`. Never advertise the capability on the
        // ProxyChat contract so Codex keeps to auto/high.
        entry_obj.insert("supports_image_detail_original".to_string(), json!(false));
    }

    // Per-model reasoning levels override the template's conservative
    // none/high default (e.g. a LiteLLM gateway serving a model that accepts
    // low/medium/high/xhigh/max). Applies to every profile.
    let template_default = template
        .get("default_reasoning_level")
        .and_then(|value| value.as_str());
    apply_codex_reasoning_level_override(entry_obj, template_default, spec);

    entry
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct CodexCatalogModelSpec {
    pub(super) model: String,
    /// Explicit user value only. Entries fall back to the model id — except
    /// official vendor catalog entries, which keep the vendor's display name.
    pub(super) display_name: Option<String>,
    /// Explicit user value only. Entries fall back to the config's
    /// `model_context_window` (or 128k) — except official vendor catalog
    /// entries, which keep the vendor's declared window.
    pub(super) context_window: Option<u64>,
    /// Per-row override for the native template's `supports_parallel_tool_calls`
    /// (e.g. MiniMax=true, MiMo=false). Only consulted for `NativeResponses`.
    pub(super) supports_parallel_tool_calls: Option<bool>,
    /// Hidden per-row capability declaration from built-in provider metadata.
    /// When omitted, all catalog profiles consult the shared text-only model
    /// registry and otherwise default to `["text", "image"]`.
    pub(super) input_modalities: Option<Vec<String>>,
    /// Per-row override for the native template's `base_instructions` (the
    /// model identity / system preamble). Carries each vendor's OFFICIAL value
    /// (e.g. MiMo "developed by Xiaomi", MiniMax "based on MiniMax-M3"); falls
    /// back to the template default when absent. Only consulted for
    /// `NativeResponses`.
    pub(super) base_instructions: Option<String>,
    /// Per-row override for the generated catalog's `supported_reasoning_levels`
    /// (e.g. ["none", "low", "medium", "high", "xhigh", "max"]). When omitted
    /// the template's conservative default (none/high) is kept. Consulted for
    /// every profile; the vendor-catalog path applies it on top of the
    /// official entry.
    pub(super) reasoning_levels: Option<Vec<String>>,
    /// Per-row override for the generated catalog's `default_reasoning_level`.
    /// Only meaningful together with `reasoning_levels`; when absent the
    /// template default is kept if it is still in the list, otherwise the last
    /// (highest) declared level wins.
    pub(super) default_reasoning_level: Option<String>,
}

pub(super) fn codex_catalog_model_specs(settings: &Value) -> Vec<CodexCatalogModelSpec> {
    let Some(models) = settings
        .get("modelCatalog")
        .and_then(|catalog| catalog.get("models"))
        .and_then(|models| models.as_array())
    else {
        return Vec::new();
    };

    let mut seen = std::collections::HashSet::new();
    let mut specs = Vec::new();

    for model_config in models {
        let Some(model) = model_config
            .get("model")
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|model| !model.is_empty())
        else {
            continue;
        };

        if !seen.insert(model.to_string()) {
            continue;
        }

        let display_name = model_config
            .get("displayName")
            .or_else(|| model_config.get("display_name"))
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(str::to_string);
        let context_window = parse_codex_positive_u64(
            model_config
                .get("contextWindow")
                .or_else(|| model_config.get("context_window")),
        );

        let supports_parallel_tool_calls = model_config
            .get("supportsParallelToolCalls")
            .or_else(|| model_config.get("supports_parallel_tool_calls"))
            .and_then(|value| value.as_bool());
        let input_modalities = model_config
            .get("inputModalities")
            .or_else(|| model_config.get("input_modalities"))
            .and_then(|value| value.as_array())
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.as_str())
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .filter(|items| !items.is_empty());

        let base_instructions = model_config
            .get("baseInstructions")
            .or_else(|| model_config.get("base_instructions"))
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(str::to_string);

        let reasoning_levels = model_config
            .get("reasoningLevels")
            .or_else(|| model_config.get("reasoning_levels"))
            .and_then(|value| value.as_array())
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.as_str())
                    .map(str::trim)
                    .filter(|level| !level.is_empty())
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .filter(|levels| !levels.is_empty());
        let default_reasoning_level = model_config
            .get("defaultReasoningLevel")
            .or_else(|| model_config.get("default_reasoning_level"))
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|level| !level.is_empty())
            .map(str::to_string);

        specs.push(CodexCatalogModelSpec {
            model: model.to_string(),
            display_name,
            context_window,
            supports_parallel_tool_calls,
            input_modalities,
            base_instructions,
            reasoning_levels,
            default_reasoning_level,
        });
    }

    specs
}

pub(super) fn find_codex_model_template(catalog: &Value) -> Option<Value> {
    catalog
        .get("models")
        .and_then(|models| models.as_array())
        .and_then(|models| {
            models.iter().find(|model| {
                model.get("slug").and_then(|slug| slug.as_str())
                    == Some(CODEX_MODEL_CATALOG_TEMPLATE_SLUG)
            })
        })
        .cloned()
}

pub(super) fn load_codex_model_template_from_cache() -> Result<Option<Value>, AppError> {
    let path = get_codex_config_dir().join("models_cache.json");
    if !path.exists() {
        return Ok(None);
    }

    let text = fs::read_to_string(&path).map_err(|e| AppError::io(&path, e))?;
    let catalog: Value = serde_json::from_str(&text).map_err(|e| AppError::json(&path, e))?;
    Ok(find_codex_model_template(&catalog))
}

/// Fixed candidates for locating the `codex` CLI when it is not on the process
/// PATH (common in GUI apps launched outside a terminal).
#[cfg(target_os = "macos")]
const CODEX_CLI_FIXED_CANDIDATES: &[&str] = &[
    "codex",                   // PATH (all platforms)
    "/opt/homebrew/bin/codex", // macOS Apple Silicon Homebrew
    "/usr/local/bin/codex",    // macOS Intel Homebrew
];

#[cfg(windows)]
const CODEX_CLI_FIXED_CANDIDATES: &[&str] = &[];

pub(super) fn push_codex_cli_candidate(
    candidates: &mut Vec<PathBuf>,
    seen: &mut HashSet<String>,
    candidate: PathBuf,
) {
    let key = candidate.to_string_lossy().into_owned();
    if seen.insert(key) {
        candidates.push(candidate);
    }
}

pub(super) fn push_existing_codex_cli_candidate(
    candidates: &mut Vec<PathBuf>,
    seen: &mut HashSet<String>,
    candidate: PathBuf,
) {
    if candidate.exists() {
        push_codex_cli_candidate(candidates, seen, candidate);
    }
}

pub(super) fn push_codex_cli_candidates_from_version_dirs(
    candidates: &mut Vec<PathBuf>,
    seen: &mut HashSet<String>,
    versions_dir: PathBuf,
    suffix: &[&str],
) {
    let Ok(entries) = fs::read_dir(versions_dir) else {
        return;
    };

    let mut discovered = entries
        .filter_map(Result::ok)
        .map(|entry| {
            let mut candidate = entry.path();
            for component in suffix {
                candidate.push(component);
            }
            candidate
        })
        .filter(|candidate| candidate.exists())
        .collect::<Vec<_>>();

    // Prefer newer-looking version directories before older global installs.
    discovered.sort_by(|a, b| b.cmp(a));
    for candidate in discovered {
        push_codex_cli_candidate(candidates, seen, candidate);
    }
}

pub(super) fn push_home_codex_cli_candidates(
    candidates: &mut Vec<PathBuf>,
    seen: &mut HashSet<String>,
    home: &Path,
) {
    for relative in [
        ".nvm/current/bin/codex",
        ".volta/bin/codex",
        ".asdf/shims/codex",
        ".local/share/mise/shims/codex",
        ".config/mise/shims/codex",
        ".local/bin/codex",
        ".npm-global/bin/codex",
        ".npm-packages/bin/codex",
        ".local/share/pnpm/codex",
        "Library/pnpm/codex",
    ] {
        push_existing_codex_cli_candidate(candidates, seen, home.join(relative));
    }
    push_codex_cli_candidates_from_version_dirs(
        candidates,
        seen,
        home.join(".nvm/versions/node"),
        &["bin", "codex"],
    );
    push_codex_cli_candidates_from_version_dirs(
        candidates,
        seen,
        home.join(".local/share/fnm/node-versions"),
        &["installation", "bin", "codex"],
    );
    push_codex_cli_candidates_from_version_dirs(
        candidates,
        seen,
        home.join("Library/Application Support/fnm/node-versions"),
        &["installation", "bin", "codex"],
    );
}

#[cfg(target_os = "macos")]
pub(super) fn push_env_codex_cli_candidates(
    candidates: &mut Vec<PathBuf>,
    seen: &mut HashSet<String>,
) {
    for (env_key, suffix) in [
        ("NPM_CONFIG_PREFIX", &["bin", "codex"][..]),
        ("VOLTA_HOME", &["bin", "codex"][..]),
        ("ASDF_DATA_DIR", &["shims", "codex"][..]),
        ("MISE_DATA_DIR", &["shims", "codex"][..]),
        ("PNPM_HOME", &["codex"][..]),
    ] {
        let Some(prefix) = std::env::var_os(env_key) else {
            continue;
        };
        let mut candidate = PathBuf::from(prefix);
        for component in suffix {
            candidate.push(component);
        }
        push_existing_codex_cli_candidate(candidates, seen, candidate);
    }
    if let Some(nvm_dir) = std::env::var_os("NVM_DIR") {
        push_codex_cli_candidates_from_version_dirs(
            candidates,
            seen,
            PathBuf::from(nvm_dir).join("versions/node"),
            &["bin", "codex"],
        );
    }
    if let Some(fnm_dir) = std::env::var_os("FNM_DIR") {
        push_codex_cli_candidates_from_version_dirs(
            candidates,
            seen,
            PathBuf::from(fnm_dir).join("node-versions"),
            &["installation", "bin", "codex"],
        );
    }
}

#[cfg(windows)]
pub(super) fn push_env_codex_cli_candidates(
    candidates: &mut Vec<PathBuf>,
    seen: &mut HashSet<String>,
) {
    for directory in crate::windows_runtime::safe_command_search_paths() {
        for name in ["codex.cmd", "codex.exe", "codex"] {
            push_existing_codex_cli_candidate(candidates, seen, directory.join(name));
        }
    }
}

pub(super) fn codex_cli_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    let mut seen = HashSet::new();
    for candidate in CODEX_CLI_FIXED_CANDIDATES {
        push_codex_cli_candidate(&mut candidates, &mut seen, PathBuf::from(candidate));
    }
    push_env_codex_cli_candidates(&mut candidates, &mut seen);
    let home = get_home_dir();
    #[cfg(windows)]
    if crate::windows_runtime::is_local_command_path(&home) {
        push_home_codex_cli_candidates(&mut candidates, &mut seen, &home);
    }
    #[cfg(target_os = "macos")]
    push_home_codex_cli_candidates(&mut candidates, &mut seen, &home);
    candidates
}

pub(super) fn codex_bundled_models_command(
    candidate: &Path,
) -> Result<Command, crate::windows_runtime::WindowsStartupErrorCode> {
    let mut command = Command::new(candidate);
    command
        .args(["debug", "models", "--bundled"])
        .stdin(Stdio::null());
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        crate::windows_runtime::configure_shell_user_command(&mut command, candidate.parent())?;
        command
            .current_dir(crate::windows_runtime::require_interactive_user_context().user_profile());
        command.creation_flags(CREATE_NO_WINDOW);
    }
    Ok(command)
}

pub(super) fn load_codex_model_template_from_bundled() -> Result<Option<Value>, AppError> {
    first_bundled_catalog(find_codex_model_template)
}

/// 本机 Codex 自带的完整模型列表（`codex debug models --bundled`）。和账号无关，版本和
/// 那个二进制一致。不能用不带 `--bundled` 的版本：那会读到 CC Switch 自己写的目录。
pub(crate) fn load_codex_bundled_models() -> Option<Vec<Value>> {
    first_bundled_catalog(|catalog| {
        catalog
            .get("models")
            .and_then(Value::as_array)
            .filter(|models| !models.is_empty())
            .cloned()
    })
    .ok()
    .flatten()
}

/// 依次跑各个候选的 `codex debug models --bundled`，返回第一份 `pick` 取得出东西的结果。
pub(super) fn first_bundled_catalog<T>(
    pick: impl Fn(&Value) -> Option<T>,
) -> Result<Option<T>, AppError> {
    if !codex_bundled_cli_allowed(
        cfg!(target_os = "windows"),
        crate::windows_runtime::formal_windows_build(),
    ) {
        return Ok(None);
    }
    for candidate in codex_cli_candidates() {
        let mut command = match codex_bundled_models_command(&candidate) {
            Ok(command) => command,
            Err(error) => {
                log::debug!("failed to configure Codex CLI environment: {error}");
                continue;
            }
        };
        let candidate_label = candidate.to_string_lossy();
        let output = match command.output() {
            Ok(output) => output,
            Err(err) => {
                log::debug!("failed to run `{candidate_label} debug models --bundled`: {err}");
                continue;
            }
        };

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            log::debug!("`{candidate_label} debug models --bundled` failed: {stderr}");
            continue;
        }

        let catalog: Value = match serde_json::from_slice(&output.stdout) {
            Ok(catalog) => catalog,
            Err(e) => {
                log::debug!(
                    "Failed to parse `{candidate_label} debug models --bundled` output: {e}"
                );
                continue;
            }
        };
        if let Some(found) = pick(&catalog) {
            return Ok(Some(found));
        }
    }

    Ok(None)
}

/// 官方原生行：逐行补 Codex 解析器必需的字段（不覆盖已有值）、补旧的指令字段，再校验。
/// 原生字段一律保留（不套 profile、`comp_hash` 和工具定义不动）。有一行不合格就整份作废
/// （`None`），不只丢那一行：那样会悄悄少一个官方模型。
pub(crate) fn normalize_codex_native_rows(rows: Vec<Value>) -> Option<Vec<Value>> {
    if rows.is_empty() {
        return None;
    }
    rows.into_iter()
        .map(|mut row| {
            fill_template_fields_from_static(&mut row);
            backfill_codex_base_instructions(&mut row);
            codex_native_row_is_valid(&row).then_some(row)
        })
        .collect()
}

/// Codex 的指令有新旧两种写法：新的 `model_messages.instructions_template`，旧的顶层
/// `base_instructions`，有一个就能解析。新版 Codex 写缓存、服务端返回的都只有新的，
/// 更早的 Codex 只认旧的。缺旧字段、有 template 时原样复制（空串也照抄），和 codex-rs
/// 的序列化器一样；已有旧字段的不动，新版以 template 为准。
pub(super) fn backfill_codex_base_instructions(row: &mut Value) {
    let Some(obj) = row.as_object_mut() else {
        return;
    };
    if obj.get("base_instructions").is_some_and(Value::is_string) {
        return;
    }
    if let Some(template) = obj
        .get("model_messages")
        .and_then(|messages| messages.get("instructions_template"))
        .and_then(Value::as_str)
        .map(str::to_string)
    {
        obj.insert("base_instructions".to_string(), json!(template));
    }
}

pub(super) fn codex_native_row_is_valid(row: &Value) -> bool {
    let Some(obj) = row.as_object() else {
        return false;
    };
    let has_slug = obj
        .get("slug")
        .and_then(Value::as_str)
        .is_some_and(|slug| !slug.trim().is_empty());
    let has_required = CODEX_CATALOG_PARSER_REQUIRED_FIELDS
        .iter()
        .all(|key| obj.contains_key(*key));
    let has_instructions = obj.get("base_instructions").is_some_and(Value::is_string)
        || obj
            .get("model_messages")
            .and_then(|messages| messages.get("instructions_template"))
            .is_some_and(Value::is_string);
    has_slug && has_required && has_instructions
}

pub(super) fn load_codex_model_template_static() -> Option<Value> {
    let text = include_str!("../resources/gpt5_5_template.json");
    match serde_json::from_str(text) {
        Ok(template) => Some(template),
        Err(e) => {
            log::warn!("Failed to parse bundled gpt-5.5 template: {e}");
            None
        }
    }
}

/// Bundled clean template for native `/responses` providers. Unlike the
/// gpt-5.5 template it carries NO freeform `apply_patch` / `web_search` tool
/// declarations and no GPT-5 base_instructions, so Codex never emits a
/// `type=="custom"` tool that native gateways (MiMo/MiniMax/…) reject. Edits
/// flow through `shell_type="shell_command"` instead. We deliberately do NOT
/// fall back to `models_cache.json` here (that would reintroduce gpt-5.5's
/// freeform apply_patch).
pub(super) fn load_codex_native_responses_template() -> Value {
    let text = include_str!("../resources/codex_native_responses_template.json");
    serde_json::from_str(text).expect("bundled codex native responses template must be valid JSON")
}

/// Hosts whose native `/responses` gateway publishes an OFFICIAL Codex model
/// catalog (models.json) that cc-switch mirrors verbatim. Matched against
/// `base_url` ONLY — deliberately NOT by model brand, unlike
/// `CODEX_WEB_SEARCH_REJECT_MODEL_PREFIXES`: the official entries GRANT
/// capabilities (freeform `apply_patch`, vendor harness), and an aggregator
/// merely hosting the same model may not honor them. The safe failure
/// direction for aggregators is the neutral template (degraded but working);
/// wrongly granting freeform apply_patch would reintroduce the custom-tool
/// rejection bug.
const CODEX_DEEPSEEK_OFFICIAL_CATALOG_HOSTS: &[&str] = &["deepseek.com"];

/// Bundled copy of DeepSeek's official Codex models.json — the exact file
/// their one-click integration script writes (api-docs.deepseek.com →
/// quick_start/agent_integrations/codex): freeform apply_patch, GPT-5 harness
/// base_instructions, low/high/max reasoning levels, web_search supported,
/// 1m context. Declares `minimal_client_version` 0.144.0.
pub(super) fn load_codex_deepseek_official_catalog_models() -> Vec<Value> {
    let text = include_str!("../resources/codex_deepseek_catalog_template.json");
    let catalog: Value =
        serde_json::from_str(text).expect("bundled DeepSeek official catalog must be valid JSON");
    catalog
        .get("models")
        .and_then(|models| models.as_array())
        .cloned()
        .unwrap_or_default()
}

/// Official vendor catalog entries for the provider in `config_text`, if its
/// gateway ships one. Only the `NativeResponses` profile qualifies: ProxyChat
/// runs through cc-switch's converter (gpt-5.5 template contract) and the
/// Anthropic transform drops custom tools, so both must keep their existing
/// templates. Host-driven like the web_search blacklist, so existing providers
/// pick it up on their next switch without a re-save.
pub(super) fn codex_official_vendor_catalog_models(
    config_text: &str,
    profile: CodexCatalogToolProfile,
) -> Option<Vec<Value>> {
    if profile != CodexCatalogToolProfile::NativeResponses {
        return None;
    }
    let base_url = extract_codex_base_url(config_text)?;
    let url = url::Url::parse(&base_url).ok()?;
    if url.scheme() == "https"
        && codex_url_host_matches_any(&base_url, CODEX_DEEPSEEK_OFFICIAL_CATALOG_HOSTS)
    {
        let models = load_codex_deepseek_official_catalog_models();
        if !models.is_empty() {
            return Some(models);
        }
    }
    None
}

/// Build one catalog entry from an official vendor catalog: match the user's
/// model id against the vendor entries by slug; an unknown id clones the
/// vendor's first (flagship) entry so it keeps the gateway's capability
/// profile without impersonating the flagship. The official entry is
/// authoritative — no tool-profile stripping — but explicit per-row user
/// overrides still win.
pub(super) fn codex_vendor_catalog_model_entry(
    vendor_models: &[Value],
    spec: &CodexCatalogModelSpec,
    priority: usize,
) -> Value {
    let matched = vendor_models.iter().find(|entry| {
        entry
            .get("slug")
            .and_then(|slug| slug.as_str())
            .is_some_and(|slug| slug.eq_ignore_ascii_case(&spec.model))
    });
    let mut entry = match matched {
        Some(found) => found.clone(),
        None => vendor_models.first().cloned().unwrap_or_else(|| json!({})),
    };
    // Capture before the mutable borrow: the vendor entry's own default is the
    // fallback when the user declares reasoning levels without a default.
    let vendor_default = entry
        .get("default_reasoning_level")
        .and_then(|value| value.as_str())
        .map(str::to_string);
    let Some(entry_obj) = entry.as_object_mut() else {
        return json!({});
    };

    if matched.is_none() {
        let display_name = spec.display_name.as_deref().unwrap_or(&spec.model);
        entry_obj.insert("slug".to_string(), json!(spec.model));
        entry_obj.insert("display_name".to_string(), json!(display_name));
        entry_obj.insert("description".to_string(), json!(display_name));
        entry_obj.insert("priority".to_string(), json!(1000 + priority));
        // Unknown model: don't inherit the flagship entry's modalities —
        // resolve from the registry/fail-open logic instead, so a vision
        // variant (e.g. deepseek-v4-flash-vision-exp) is not declared
        // text-only merely because the flagship is.
        entry_obj.insert(
            "input_modalities".to_string(),
            json!(codex_catalog_input_modalities(
                &spec.model,
                spec.input_modalities.as_deref(),
            )),
        );
    }

    // Explicit user overrides win over the official entry; absent values keep
    // the vendor's declarations (context window, modalities, harness, ...).
    if let Some(display_name) = spec.display_name.as_deref() {
        entry_obj.insert("display_name".to_string(), json!(display_name));
    }
    if let Some(context_window) = spec.context_window {
        entry_obj.insert("context_window".to_string(), json!(context_window));
        entry_obj.insert("max_context_window".to_string(), json!(context_window));
    }
    if let Some(parallel) = spec.supports_parallel_tool_calls {
        entry_obj.insert("supports_parallel_tool_calls".to_string(), json!(parallel));
    }
    if let Some(modalities) = spec.input_modalities.as_deref() {
        entry_obj.insert("input_modalities".to_string(), json!(modalities));
    }
    if let Some(base_instructions) = spec
        .base_instructions
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty())
    {
        entry_obj.insert("base_instructions".to_string(), json!(base_instructions));
    }

    // Per-model reasoning levels win over the official vendor entry too.
    // The vendor file is the base (its own levels stay when no override is
    // declared); its default_reasoning_level is the fallback.
    apply_codex_reasoning_level_override(entry_obj, vendor_default.as_deref(), spec);

    // Defensive: if a future codex parser requires a field the vendor file
    // predates, backfill only whitelisted parser-required keys.
    fill_template_fields_from_static(&mut entry);
    entry
}

/// Fields Codex's external-catalog parser REQUIRES (no serde default): when
/// one is missing Codex rejects the whole catalog file at startup ("missing
/// field ..."). `base_instructions` is the other known required field; the
/// templates always carry it and `codex_catalog_model_entry` handles it.
/// When Codex requires a new field, add it here AND to the static templates.
const CODEX_CATALOG_PARSER_REQUIRED_FIELDS: &[&str] = &[
    "supports_reasoning_summaries",
    // codex 0.148.0 rejects the catalog without it (#6661); a models_cache.json
    // written by an older build can lack it.
    "supports_parallel_tool_calls",
];

/// `models_cache.json` is shared by every Codex install on the machine (npm
/// CLI, desktop-bundled binary, ...), and each version serializes its own
/// `ModelInfo` shape — the cache's field set follows whichever process wrote
/// it last, so it cannot be assumed to satisfy the current external-catalog
/// schema (observed live: 0.144.5 requires `supports_reasoning_summaries`
/// while a coexisting build kept rewriting the cache without it). Backfill
/// ONLY parser-required fields from the bundled static template: optional
/// capability fields keep their missing-means-default semantics, and existing
/// values always win.
pub(super) fn fill_template_fields_from_static(template: &mut Value) {
    let Some(static_template) = load_codex_model_template_static() else {
        return;
    };
    let (Some(template_obj), Some(static_obj)) =
        (template.as_object_mut(), static_template.as_object())
    else {
        return;
    };
    for key in CODEX_CATALOG_PARSER_REQUIRED_FIELDS {
        if !template_obj.contains_key(*key) {
            if let Some(value) = static_obj.get(*key) {
                template_obj.insert((*key).to_string(), value.clone());
            }
        }
    }
}

pub(super) fn load_codex_model_catalog_template_uncached() -> Result<Value, AppError> {
    // ① models_cache.json (created by Codex when it connects to OpenAI)
    if let Some(mut template) = load_codex_model_template_from_cache()? {
        fill_template_fields_from_static(&mut template);
        return Ok(template);
    }
    // ② codex CLI (PATH + platform-specific common paths)
    if let Some(mut template) = load_codex_model_template_from_bundled()? {
        fill_template_fields_from_static(&mut template);
        return Ok(template);
    }
    // ③ Static fallback bundled at compile time
    if let Some(template) = load_codex_model_template_static() {
        return Ok(template);
    }

    Err(AppError::Message(format!(
        "Codex model catalog template `{CODEX_MODEL_CATALOG_TEMPLATE_SLUG}` not found. Please start Codex once so models_cache.json is available, or ensure the `codex` CLI is on PATH."
    )))
}

pub(super) fn get_or_load_codex_model_catalog_template<F>(
    cache: &OnceCell<Value>,
    loader: F,
) -> Result<Value, AppError>
where
    F: FnOnce() -> Result<Value, AppError>,
{
    cache.get_or_try_init(loader).cloned()
}

#[cfg(not(test))]
pub(super) fn load_codex_model_catalog_template() -> Result<Value, AppError> {
    get_or_load_codex_model_catalog_template(
        &CODEX_MODEL_CATALOG_TEMPLATE_CACHE,
        load_codex_model_catalog_template_uncached,
    )
}

#[cfg(test)]
pub(super) fn load_codex_model_catalog_template() -> Result<Value, AppError> {
    load_codex_model_catalog_template_uncached()
}

#[cfg(test)]
pub(super) fn codex_model_catalog_from_specs(
    specs: &[CodexCatalogModelSpec],
    template: &Value,
    profile: CodexCatalogToolProfile,
    default_context_window: u64,
) -> Value {
    let entries: Vec<Value> = specs
        .iter()
        .enumerate()
        .map(|(index, spec)| {
            codex_catalog_model_entry(template, spec, index, profile, default_context_window)
        })
        .collect();

    json!({ "models": entries })
}

pub(super) fn codex_model_catalog_from_settings(
    settings: &Value,
    config_text: &str,
    profile: CodexCatalogToolProfile,
) -> Result<Option<Value>, AppError> {
    let specs = codex_catalog_model_specs(settings);
    if specs.is_empty() {
        return Ok(None);
    }
    codex_catalog_from_specs_for_row(&specs, config_text, profile).map(Some)
}

/// Codex 自带的 OpenAI 官方模型列表（`codex debug models --bundled`），给第三方的 GPT
/// 行按模型名找官方条目用。只缓存成功读到的列表：Codex 升级后要重启 CC Switch 才换新。
#[cfg(not(test))]
static CODEX_OPENAI_OFFICIAL_MODELS_CACHE: OnceCell<Vec<Value>> = OnceCell::new();

#[cfg(test)]
thread_local! {
    /// 测试里的官方列表（`None` 时只有编译期内置的 gpt-5.5），不跑本机的 `codex`。
    static CODEX_OPENAI_OFFICIAL_MODELS_OVERRIDE: std::cell::RefCell<Option<Vec<Value>>> =
        const { std::cell::RefCell::new(None) };
}

/// 编译期内置的 gpt-5.5：本机没有可用的 `codex` 时唯一的官方条目。
pub(super) fn codex_static_official_models() -> Vec<Value> {
    load_codex_model_template_static()
        .and_then(|template| normalize_codex_native_rows(vec![template]))
        .unwrap_or_default()
}

/// 和账号无关的 OpenAI 官方模型列表。不用 `models_cache.json`：它是哪个账号、哪个版本
/// 写的证明不了（见 `codex_official_models`）。
#[cfg(not(test))]
pub(super) fn codex_openai_official_models() -> Vec<Value> {
    CODEX_OPENAI_OFFICIAL_MODELS_CACHE
        .get_or_try_init(|| {
            load_codex_bundled_models()
                .and_then(normalize_codex_native_rows)
                .ok_or(())
        })
        .cloned()
        .unwrap_or_else(|_| codex_static_official_models())
}

#[cfg(test)]
pub(super) fn codex_openai_official_models() -> Vec<Value> {
    CODEX_OPENAI_OFFICIAL_MODELS_OVERRIDE
        .with(|rows| rows.borrow().clone())
        .unwrap_or_else(codex_static_official_models)
}

/// 按 Codex 自己查模型信息的规则找官方条目（codex-rs `models-manager/src/manager.rs` 的
/// `construct_model_info_from_candidates`）：先按最长前缀，没有再去掉一层命名空间
/// （`openai/gpt-5.5`）重试；区分大小写。结果就是没有 CC Switch 的目录时 Codex 会用的那条。
pub(super) fn find_codex_official_model<'a>(
    model: &str,
    candidates: &'a [Value],
) -> Option<&'a Value> {
    fn longest_prefix<'a>(model: &str, candidates: &'a [Value]) -> Option<&'a Value> {
        candidates
            .iter()
            .filter_map(|candidate| {
                let slug = candidate.get("slug").and_then(Value::as_str)?;
                (!slug.is_empty() && model.starts_with(slug)).then_some((slug.len(), candidate))
            })
            .max_by_key(|(len, _)| *len)
            .map(|(_, candidate)| candidate)
    }

    longest_prefix(model, candidates).or_else(|| {
        let (namespace, suffix) = model.split_once('/')?;
        let simple_namespace = !namespace.is_empty()
            && namespace
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
        if suffix.contains('/') || !simple_namespace {
            return None;
        }
        longest_prefix(suffix, candidates)
    })
}

/// 第三方供应商上命中官方的 GPT 行：官方条目整条照搬（提示词、工具、档位、窗口都以官方
/// 为准，不接受行里的覆盖值；窗口不同时在 `config.toml` 里设 `model_context_window`），
/// 只改掉属于官方账号或官方后端的字段。
pub(super) fn codex_official_model_entry(
    official: &Value,
    model: &str,
    priority: usize,
    profile: CodexCatalogToolProfile,
) -> Value {
    let mut entry = official.clone();
    let Some(obj) = entry.as_object_mut() else {
        return json!({});
    };
    // 前缀、命名空间命中的别名（`gpt-5.5-high`）不冒用官方的显示名。
    if obj.get("slug").and_then(Value::as_str) != Some(model) {
        obj.insert("display_name".to_string(), json!(model));
    }
    obj.insert("slug".to_string(), json!(model));
    obj.insert("priority".to_string(), json!(1000 + priority));
    // 官方隐藏的条目用户明确写了，也要出现在选择器里。
    obj.insert("visibility".to_string(), json!("list"));
    // 速度档、升级提示归官方账号。
    obj.insert("service_tiers".to_string(), json!([]));
    obj.insert("additional_speed_tiers".to_string(), json!([]));
    obj.insert("availability_nux".to_string(), Value::Null);
    obj.insert("upgrade".to_string(), Value::Null);
    // 第三方不支持 Responses Lite 协议。
    obj.insert("use_responses_lite".to_string(), Value::Bool(false));
    if profile == CodexCatalogToolProfile::ProxyChat {
        // 同 `codex_catalog_model_entry`：严格的 Chat 网关拒收 `original` 精度的图片。
        obj.insert("supports_image_detail_original".to_string(), json!(false));
    }
    entry
}

/// 目录里的一行是不是按 `official` 原样镜像出来的：反向解析会保留的几项（显示名、窗口、
/// 模态、并行工具调用）都和重新镜像的结果相同，只还原模型名才不丢东西。
pub(super) fn is_codex_official_mirror(entry: &Value, model: &str, official: &Value) -> bool {
    let expected =
        codex_official_model_entry(official, model, 0, CodexCatalogToolProfile::NativeResponses);
    [
        "display_name",
        "context_window",
        "input_modalities",
        "supports_parallel_tool_calls",
    ]
    .iter()
    .all(|key| entry.get(*key) == expected.get(*key))
}

/// The catalog for one provider's models: its official vendor catalog when the
/// gateway ships one, otherwise the profile's template.
pub(super) fn codex_catalog_from_specs_for_row(
    specs: &[CodexCatalogModelSpec],
    config_text: &str,
    profile: CodexCatalogToolProfile,
) -> Result<Value, AppError> {
    // Vendors that publish an OFFICIAL Codex models.json for their native
    // `/responses` gateway get it mirrored verbatim instead of the neutral
    // template: its freeform apply_patch, vendor harness base_instructions and
    // reasoning levels are load-bearing (the harness tells the model to use
    // apply_patch, so catalog and harness must stay consistent).
    if let Some(vendor_models) = codex_official_vendor_catalog_models(config_text, profile) {
        let entries: Vec<Value> = specs
            .iter()
            .enumerate()
            .map(|(index, spec)| codex_vendor_catalog_model_entry(&vendor_models, spec, index))
            .collect();
        return Ok(json!({ "models": entries }));
    }

    let default_context_window =
        extract_codex_top_level_u64(config_text, "model_context_window").unwrap_or(128_000);

    // Native providers use the bundled clean template (no freeform apply_patch,
    // no cache dependency); proxy-chat providers keep cloning Codex's gpt-5.5
    // entry so the proxy can rewrite custom<->function tools as before.
    let template = match profile {
        CodexCatalogToolProfile::NativeResponses | CodexCatalogToolProfile::Anthropic => {
            load_codex_native_responses_template()
        }
        CodexCatalogToolProfile::ProxyChat => load_codex_model_catalog_template()?,
    };
    // 命中 OpenAI 官方条目的行照搬官方（写了目录之后 Codex 只认文件里的条目，通用模板
    // 会顶掉 GPT 自己的提示词），其余行照旧按模板生成。Responses→Anthropic 的转换会丢掉
    // 官方条目里的 custom 工具，这条路不照搬。
    let official = match profile {
        CodexCatalogToolProfile::Anthropic => Vec::new(),
        CodexCatalogToolProfile::NativeResponses | CodexCatalogToolProfile::ProxyChat => {
            codex_openai_official_models()
        }
    };
    let entries: Vec<Value> = specs
        .iter()
        .enumerate()
        .map(
            |(index, spec)| match find_codex_official_model(&spec.model, &official) {
                Some(found) => codex_official_model_entry(found, &spec.model, index, profile),
                None => codex_catalog_model_entry(
                    &template,
                    spec,
                    index,
                    profile,
                    default_context_window,
                ),
            },
        )
        .collect();
    Ok(json!({ "models": entries }))
}

/// 一个供应商的模型目录：没有配置模型时为 `None`，不生成、也不指向目录文件。
/// `web_search` 由 [`codex_disables_web_search`] 另算（切走时要按上一家算，不必生成目录）。
pub(crate) struct CodexCatalogPlan {
    pub catalog: Option<Value>,
}

/// 要不要关掉 web_search：Responses→Anthropic 的转换会丢掉这个内置工具，一律关；原生
/// Responses 网关按拒收名单判定（MiMo、LongCat、MiniMax 等按域名或模型品牌，Qwen3-Coder
/// 按模型）；其余保持 Codex 的默认。只在有模型目录时才看名单。
pub(crate) fn codex_disables_web_search(
    settings: &Value,
    config_text: &str,
    profile: CodexCatalogToolProfile,
) -> bool {
    match profile {
        CodexCatalogToolProfile::Anthropic => true,
        CodexCatalogToolProfile::NativeResponses => {
            !codex_catalog_model_specs(settings).is_empty()
                && codex_native_gateway_rejects_web_search(config_text)
        }
        CodexCatalogToolProfile::ProxyChat => false,
    }
}

/// 在内存里算出模型目录，不写盘。`config_text` 是归一化后的配置（选路、路由表地址、
/// 模型名、窗口），见 `CodexProjection::catalog_input_text`。
pub(crate) fn plan_codex_model_catalog(
    settings: &Value,
    config_text: &str,
    profile: CodexCatalogToolProfile,
) -> Result<CodexCatalogPlan, AppError> {
    Ok(CodexCatalogPlan {
        catalog: codex_model_catalog_from_settings(settings, config_text, profile)?,
    })
}

/// 一家第三方供应商发布的模型：行里的模型目录；没有配置目录时只有行的 `model`。
pub(super) fn codex_published_specs(
    settings: &Value,
    config_text: &str,
) -> Vec<CodexCatalogModelSpec> {
    let specs = codex_catalog_model_specs(settings);
    if !specs.is_empty() {
        return specs;
    }
    codex_top_level_model(config_text)
        .map(|model| {
            vec![CodexCatalogModelSpec {
                model,
                ..CodexCatalogModelSpec::default()
            }]
        })
        .unwrap_or_default()
}

/// Stack 模型用：一家第三方供应商发布的模型名（按目录顺序）。`config_text` 是行里的
/// `config`（只读顶层 `model`）。
pub(crate) fn codex_published_models(settings: &Value, config_text: &str) -> Vec<String> {
    codex_published_specs(settings, config_text)
        .into_iter()
        .map(|spec| spec.model)
        .collect()
}

/// 合并目录里的一家第三方供应商。
pub(crate) struct CodexCatalogRow<'a> {
    pub settings: &'a Value,
    /// 这一家归一化后的配置（`CodexProjection::catalog_input_text`）：地址、模型名、窗口。
    pub config_text: &'a str,
    /// 这一家自己的工具 profile：各家的请求走各自的转换，目录要和转换对得上。
    pub profile: CodexCatalogToolProfile,
}

/// 合并目录里的一家 Stack 供应商。
pub(crate) struct CodexStackCatalogMember<'a> {
    pub key: &'a str,
    pub provider_name: &'a str,
    pub row: CodexCatalogRow<'a>,
}

/// 合并目录里路由那家的行。
pub(crate) enum CodexStackRoute<'a> {
    /// 第三方路由：按它的行生成。
    ThirdParty(CodexCatalogRow<'a>),
    /// 官方路由：官方模型列表的原生行（已补齐、已校验），原样保留；`config_text` 是
    /// 官方卡归一化后的配置，只取窗口键。
    Official {
        native: Vec<Value>,
        config_text: &'a str,
    },
}

/// 合并目录里 Stack 行统一的 `comp_hash`。Codex 在一个会话记下的值变了时会压缩一次；
/// 模板带来的值会随来源漂移（DeepSeek 官方目录是 "3000"，从 Codex 缓存克隆的 gpt-5.5
/// 跟着缓存变），固定值才稳定。路由那家的行不改：它的值要和名单为空时的目录一致，否则
/// 加进第一家、移除最后一家都会让路由上的会话恢复时被压缩一次。
const CODEX_STACK_COMP_HASH: &str = "fyagent";

/// Stack 名单非空时的模型目录：路由那家的行在前，各 Stack 供应商的行按名单顺序在后，
/// `priority` 统一重新编号。
///
/// 窗口类全局键（`model_context_window`、`model_auto_compact_token_limit`）这时不写进
/// `config.toml`（Codex 会拿它覆盖所有行），改由各家写进自己的行，见 [`sink_row_windows`]。
pub(crate) fn plan_codex_stack_catalog(
    route: CodexStackRoute<'_>,
    stack: &[CodexStackCatalogMember<'_>],
) -> Result<Value, AppError> {
    let mut entries = match route {
        CodexStackRoute::ThirdParty(row) => codex_stack_third_party_rows(&row)?,
        CodexStackRoute::Official {
            mut native,
            config_text,
        } => {
            // 按官方的 priority 排好再重新编号，模型选择器里的顺序和默认模型都不变。
            native.sort_by_key(|entry| {
                entry
                    .get("priority")
                    .and_then(Value::as_i64)
                    .unwrap_or(i64::MAX)
            });
            let windows = RowWindows::of(config_text);
            for entry in &mut native {
                sink_row_windows(entry, &windows, false);
            }
            native
        }
    };
    for member in stack {
        for mut entry in codex_stack_third_party_rows(&member.row)? {
            let Some(obj) = entry.as_object_mut() else {
                continue;
            };
            obj.insert("comp_hash".to_string(), json!(CODEX_STACK_COMP_HASH));
            let Some(model) = obj.get("slug").and_then(Value::as_str).map(str::to_string) else {
                continue;
            };
            let display = obj
                .get("display_name")
                .and_then(Value::as_str)
                .map(str::to_string)
                .unwrap_or_else(|| model.clone());
            obj.insert(
                "slug".to_string(),
                json!(crate::mode::stack::encode(
                    &crate::app_config::AppType::Codex,
                    member.key,
                    &model,
                    false,
                )),
            );
            obj.insert(
                "display_name".to_string(),
                json!(crate::mode::stack::display_name(
                    &display,
                    member.provider_name
                )),
            );
            let window = obj
                .get("context_window")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            obj.insert(
                "description".to_string(),
                json!(crate::mode::stack::model_description(&model, window)),
            );
            // 第三方不支持 Responses Lite 协议。
            if obj.get("use_responses_lite") == Some(&Value::Bool(true)) {
                obj.insert("use_responses_lite".to_string(), Value::Bool(false));
            }
            entries.push(entry);
        }
    }
    for (index, entry) in entries.iter_mut().enumerate() {
        if let Some(obj) = entry.as_object_mut() {
            obj.insert("priority".to_string(), json!(index + 1));
        }
    }
    Ok(json!({ "models": entries }))
}

/// 一家第三方供应商在合并目录里的行（`comp_hash` 保持模板的值）。
pub(super) fn codex_stack_third_party_rows(
    row: &CodexCatalogRow<'_>,
) -> Result<Vec<Value>, AppError> {
    let specs = codex_published_specs(row.settings, row.config_text);
    if specs.is_empty() {
        return Ok(Vec::new());
    }
    let catalog = codex_catalog_from_specs_for_row(&specs, row.config_text, row.profile)?;
    let mut entries = match catalog {
        Value::Object(mut obj) => match obj.remove("models") {
            Some(Value::Array(entries)) => entries,
            _ => Vec::new(),
        },
        _ => Vec::new(),
    };
    let windows = RowWindows::of(row.config_text);
    for entry in &mut entries {
        sink_row_windows(entry, &windows, true);
    }
    Ok(entries)
}

/// 一家行里配置的窗口类全局键（见 [`sink_row_windows`]）。
struct RowWindows {
    window: Option<u64>,
    limit: Option<u64>,
}

impl RowWindows {
    fn of(config_text: &str) -> Self {
        Self {
            window: extract_codex_top_level_u64(config_text, "model_context_window"),
            limit: extract_codex_top_level_u64(config_text, "model_auto_compact_token_limit"),
        }
    }
}

/// 把一家行里的窗口类全局键写进它自己的行：`model_context_window` 写成行的窗口，
/// `model_auto_compact_token_limit` 写成行的压缩点。第三方行没有压缩点时写窗口的 90%
/// （Codex 自己的默认也是 90%，写出来是为了不受别的来源影响）；官方原生行只写行里
/// 明确配置的值，其余保持原样。
fn sink_row_windows(entry: &mut Value, windows: &RowWindows, third_party: bool) {
    let Some(obj) = entry.as_object_mut() else {
        return;
    };
    if let Some(window) = windows.window {
        obj.insert("context_window".to_string(), json!(window));
        obj.insert("max_context_window".to_string(), json!(window));
    }
    let limit = windows.limit.or_else(|| {
        third_party
            .then(|| obj.get("context_window").and_then(Value::as_u64))
            .flatten()
            .filter(|window| *window > 0)
            .map(|window| window * 9 / 10)
    });
    if let Some(limit) = limit {
        obj.insert("auto_compact_token_limit".to_string(), json!(limit));
    }
}

/// Reverse of `prepare_codex_config_text_with_model_catalog`: read the
/// cc-switch–maintained catalog file referenced by `~/.codex/config.toml` and
/// convert it back into the simplified shape the frontend table uses:
/// `{ "models": [{ "model", "displayName"?, "contextWindow"?, hidden overrides... }, ...] }`.
///
/// We only reverse-parse catalogs whose `model_catalog_json` path is the
/// cc-switch–generated file (identified by filename
/// `cc-switch-model-catalog.json`). A user-managed external catalog file is
/// left alone — surfacing its richer structure as the simplified table would
/// be a downgrade we can't safely round-trip.
///
/// `displayName`, `contextWindow`, and `inputModalities` are omitted from the
/// returned entry when the on-disk value matches the fallback that
/// `codex_model_catalog_from_settings` injects for unset inputs (slug for
/// display_name, `model_context_window` or 128_000 for context_window, and the
/// shared confirmed-text-only inference for input modalities). This preserves
/// the "user left it blank" intent across round-trip; an unavoidable edge case
/// is that a user-typed value that happens to equal the fallback also collapses
/// to blank, but the next save writes the same fallback so behavior is stable.
///
/// All failure modes (missing file, parse error, no `model_catalog_json`,
/// entries without `slug`) collapse to `Ok(None)` so callers can treat this
/// as best-effort enrichment without making `read_live_settings` brittle.
/// 模型目录文件读取上限（32 MiB）。目录 JSON 正常只有几百 KiB；超过则视为异常，
/// 避免指向外部大文件时耗尽内存。
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

/// 安全地读取文件为字符串，并在超过字节上限时返回错误。
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

/// Read the cc-switch Codex model catalog file with a size cap.
pub(crate) fn read_codex_model_catalog_text(path: &Path) -> Result<String, AppError> {
    read_limited_string(path, MAX_CODEX_CATALOG_BYTES)
}

/// Given `config.toml` text, resolve the on-disk path of the cc-switch–owned
/// catalog file (returns `None` if `model_catalog_json` is absent or points at
/// a file we don't own). Relative paths are resolved under `base_dir`;
/// absolute paths must still be inside `base_dir`.
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
    let is_cc_switch_owned = referenced_path.file_name().and_then(|name| name.to_str())
        == Some(FYAGENT_CODEX_MODEL_CATALOG_FILENAME);
    if !is_cc_switch_owned {
        return None;
    }

    // 注意（有意的行为变更）：Windows 上 `/…` 形式的异平台绝对路径也会
    // 被视为绝对路径，从而在下方的包含性校验中失败——此前这类路径会因无法匹配
    // 生成文件名而回退为按文件名解析、碰巧能工作。可接受：下一次切换供应商时
    // 写入侧会重新落一个裸文件名，配置自愈（见
    // `set_catalog_json_none_removes_cc_switch_owned_by_filename` 的场景注释）。
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

    // 词法包含不等于运行时包含：配置目录内的符号链接（如 ~/.codex/link ->
    // /etc）能让 `link/cc-switch-model-catalog.json` 通过上面的检查，读取却
    // 落到目录外。文件存在时把真实路径 canonicalize 出来再校验一次，并把
    // canonical 路径返回给调用方——后续读取不再经过 symlink 组件。
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
        // base 同样 canonicalize，保证两侧前缀一致（Windows \\?\、
        // macOS /tmp -> /private/tmp）；base 失败时退回词法 base——
        // 词法 base 与 canonical 路径比较只会误拒（退化为不读），不会误放。
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

/// Pure reverse-parsing core: convert Codex catalog JSON text back into the
/// frontend's simplified model-mapping shape. Returns `None` when the catalog
/// is unparseable, has no `models` array, or yields zero valid entries.
pub(super) fn build_simplified_catalog_from_texts(
    config_text: &str,
    catalog_text: &str,
) -> Option<Value> {
    let catalog: Value = serde_json::from_str(catalog_text).ok()?;
    let models = catalog.get("models").and_then(|m| m.as_array())?;

    let default_context_window =
        extract_codex_top_level_u64(config_text, "model_context_window").unwrap_or(128_000);

    let mut official: Option<Vec<Value>> = None;
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
        // Stack 模型的行（保留前缀）不属于路由那家，不能进它的编辑表单、再被保存回库里。
        if !matches!(
            crate::mode::stack::decode(&crate::app_config::AppType::Codex, model),
            crate::mode::stack::Decoded::Plain
        ) {
            continue;
        }
        // 照搬官方的行只还原模型名，不能把官方值当成用户填的存回库里。通用模板没有
        // `model_messages`（走 Anthropic 的行不照搬）；旧版 ProxyChat 克隆的 gpt-5.5 模板
        // 有，但带着用户填的显示名、窗口，下面逐项比对不上，照旧还原。
        if entry
            .get("model_messages")
            .and_then(|messages| messages.get("instructions_template"))
            .is_some()
        {
            let official = official.get_or_insert_with(codex_openai_official_models);
            if find_codex_official_model(model, official)
                .is_some_and(|found| is_codex_official_mirror(entry, model, found))
            {
                entries.push(json!({ "model": model }));
                continue;
            }
        }

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

        // Preserve native-profile per-row overrides so a DB-SSOT-missing
        // fallback round-trip doesn't silently drop them.
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

    if entries.is_empty() {
        return None;
    }

    Some(json!({ "models": entries }))
}

pub(super) fn codex_bundled_cli_allowed(
    target_is_windows: bool,
    formal_windows_build: bool,
) -> bool {
    !target_is_windows || !formal_windows_build
}

pub(crate) fn codex_model_catalog_write_required(settings: &Value) -> bool {
    !codex_catalog_model_specs(settings).is_empty()
}

pub(crate) fn set_codex_model_catalog_json_field(
    config_text: &str,
    catalog_path: Option<&Path>,
) -> Result<String, AppError> {
    let mut doc = config_text
        .parse::<DocumentMut>()
        .map_err(|e| AppError::Message(format!("Invalid Codex config.toml: {e}")))?;
    match catalog_path {
        Some(_) => {
            // A configured user catalog is not owned by the generated catalog writer.
            let owned = doc
                .get("model_catalog_json")
                .and_then(Item::as_str)
                .map(|path| {
                    Path::new(path).file_name().and_then(|name| name.to_str())
                        == Some(FYAGENT_CODEX_MODEL_CATALOG_FILENAME)
                })
                .unwrap_or(true);
            if owned {
                doc["model_catalog_json"] = toml_edit::value(FYAGENT_CODEX_MODEL_CATALOG_FILENAME);
            }
        }
        None => {
            let should_remove = doc
                .get("model_catalog_json")
                .and_then(|item| item.as_str())
                .map(|path| {
                    Path::new(path).file_name().and_then(|name| name.to_str())
                        == Some(FYAGENT_CODEX_MODEL_CATALOG_FILENAME)
                })
                .unwrap_or(false);
            if should_remove {
                doc.as_table_mut().remove("model_catalog_json");
            }
        }
    }
    Ok(doc.to_string())
}

pub(crate) fn set_codex_native_web_search_field(
    config_text: &str,
    disable: bool,
) -> Result<String, AppError> {
    let mut doc = config_text
        .parse::<DocumentMut>()
        .map_err(|e| AppError::Message(format!("Invalid Codex config.toml: {e}")))?;
    if disable {
        doc[CODEX_WEB_SEARCH_FIELD] = toml_edit::value(CODEX_WEB_SEARCH_DISABLED);
    } else {
        let owned = doc
            .get(CODEX_WEB_SEARCH_FIELD)
            .and_then(|item| item.as_str())
            == Some(CODEX_WEB_SEARCH_DISABLED);
        if owned {
            doc.as_table_mut().remove(CODEX_WEB_SEARCH_FIELD);
        }
    }
    Ok(doc.to_string())
}

pub fn prepare_codex_config_text_with_model_catalog(
    settings: &Value,
    config_text: &str,
    profile: CodexCatalogToolProfile,
) -> Result<String, AppError> {
    if let Some(catalog) = codex_model_catalog_from_settings(settings, config_text, profile)? {
        let catalog_path = get_codex_model_catalog_path();
        let config_text = set_codex_model_catalog_json_field(config_text, Some(&catalog_path))?;
        let disable_web_search = match profile {
            CodexCatalogToolProfile::Anthropic => true,
            CodexCatalogToolProfile::NativeResponses => {
                codex_native_gateway_rejects_web_search(&config_text)
            }
            CodexCatalogToolProfile::ProxyChat => false,
        };
        let config_text = set_codex_native_web_search_field(&config_text, disable_web_search)?;
        write_json_file(&catalog_path, &catalog)?;
        Ok(config_text)
    } else {
        let config_text = set_codex_model_catalog_json_field(config_text, None)?;
        let disable_web_search = profile == CodexCatalogToolProfile::Anthropic;
        set_codex_native_web_search_field(&config_text, disable_web_search)
    }
}

pub(crate) fn codex_model_rejects_web_search(model: &str) -> bool {
    let model = model.trim().to_ascii_lowercase();
    let model = model.rsplit('/').next().unwrap_or(model.as_str());
    CODEX_WEB_SEARCH_REJECT_MODEL_PREFIXES
        .iter()
        .any(|prefix| model.starts_with(prefix))
}
