//! Agentless target config file renderer for remote SSH push.
//!
//! Renders ready-to-use client configuration files locally from FyAgent's
//! projection engine so the remote host needs NO FyAgent installation at all.
//! Raw API keys stay strictly inside `Zeroizing<Vec<u8>>` file payloads and
//! never appear in preview DTOs.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use zeroize::Zeroizing;

use crate::{
    app_config::McpServer,
    services::{
        provider::batch_setup::BatchSetupTargetId,
        provider_api::ApiProtocol,
        workspace_pack::{
            projection::{
                collect_mcp_projections, collect_model_projections, collect_prompt_projections,
                collect_skill_projections, sanitize_relative_skill_path,
            },
            schema::ProjectedModelConfig,
        },
    },
    store::AppState,
};

/// Upper bound on total payload bytes pushed in a single SSH transaction (20 MiB).
pub const MAX_REMOTE_PUSH_TOTAL_BYTES: usize = 20 * 1024 * 1024;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemotePushSelection {
    #[serde(default = "default_true")]
    pub include_models: bool,
    #[serde(default = "default_true")]
    pub include_mcp: bool,
    #[serde(default = "default_true")]
    pub include_prompts: bool,
    #[serde(default = "default_true")]
    pub include_skills: bool,
    /// Optional subset of model clients to render (defaults to claude, codex, opencode, omp).
    #[serde(default)]
    pub model_targets: Vec<BatchSetupTargetId>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RemoteFileCategory {
    ModelConfig,
    McpConfig,
    GlobalPrompt,
    SkillFile,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemotePathMapping {
    /// Optional target root directory override (empty means remote `$HOME`).
    /// Accepts `~/relative/path` or an absolute POSIX `/path` without `..`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub home_root: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claude_dir: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub codex_dir: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opencode_dir: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub omp_dir: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedPathMapping {
    pub home_root: Option<String>,
    pub display_prefix: String,
    pub claude_dir: String,
    pub codex_dir: String,
    pub opencode_dir: String,
    pub omp_dir: String,
}

impl ValidatedPathMapping {
    pub fn into_normalized_mapping(self) -> RemotePathMapping {
        RemotePathMapping {
            home_root: self.home_root,
            claude_dir: (self.claude_dir != ".claude").then_some(self.claude_dir),
            codex_dir: (self.codex_dir != ".codex").then_some(self.codex_dir),
            opencode_dir: (self.opencode_dir != ".config/opencode").then_some(self.opencode_dir),
            omp_dir: (self.omp_dir != ".omp/agent").then_some(self.omp_dir),
        }
    }
}

fn is_safe_posix_path_segment(segment: &str) -> bool {
    !segment.is_empty()
        && segment != "."
        && segment != ".."
        && !segment.starts_with('-')
        && segment
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
}

fn validate_relative_posix_dir(
    raw: Option<&str>,
    default_dir: &str,
    label: &str,
) -> Result<String, String> {
    let Some(trimmed) = raw.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(default_dir.to_string());
    };
    let stripped = trimmed.trim_start_matches("./").trim_end_matches('/');
    if stripped.is_empty() || stripped.starts_with('/') || stripped.starts_with('~') {
        return Err(format!(
            "{label}请填写相对目标根目录的子路径（例如 {default_dir}）"
        ));
    }
    if stripped.len() > 160 || stripped.contains('\\') {
        return Err(format!("{label}路径过长或包含非法反斜杠"));
    }
    let segments: Vec<&str> = stripped.split('/').collect();
    if segments.iter().any(|seg| !is_safe_posix_path_segment(seg)) {
        return Err(format!("{label}包含不支持的字符或 '..' 越界路径"));
    }
    Ok(segments.join("/"))
}

fn validate_home_root(raw: Option<&str>) -> Result<(Option<String>, String), String> {
    let Some(trimmed) = raw.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok((None, "~/".to_string()));
    };
    if trimmed == "~" || trimmed == "~/" {
        return Ok((None, "~/".to_string()));
    }
    if trimmed.len() > 200 || trimmed.contains('\\') {
        return Err("目标根目录过长或包含非法反斜杠".to_string());
    }
    if let Some(rest) = trimmed.strip_prefix("~/") {
        let cleaned = rest.trim_end_matches('/');
        if cleaned.is_empty() {
            return Ok((None, "~/".to_string()));
        }
        let segments: Vec<&str> = cleaned.split('/').collect();
        if segments.iter().any(|seg| !is_safe_posix_path_segment(seg)) {
            return Err("目标根目录包含非法字符或 '..' 越界路径".to_string());
        }
        let normalized = format!("~/{}", segments.join("/"));
        let prefix = format!("{normalized}/");
        return Ok((Some(normalized), prefix));
    }
    if let Some(rest) = trimmed.strip_prefix('/') {
        let cleaned = rest.trim_end_matches('/');
        if cleaned.is_empty() {
            return Err("目标根目录不能直接指向系统根路径 '/'".to_string());
        }
        let segments: Vec<&str> = cleaned.split('/').collect();
        if segments.iter().any(|seg| !is_safe_posix_path_segment(seg)) {
            return Err("目标根目录包含非法字符或 '..' 越界路径".to_string());
        }
        let normalized = format!("/{}", segments.join("/"));
        let prefix = format!("{normalized}/");
        return Ok((Some(normalized), prefix));
    }
    Err("目标根目录请使用 '~/子目录' 或绝对 POSIX 路径 '/path/to/dir'".to_string())
}

pub fn validate_path_mapping(mapping: &RemotePathMapping) -> Result<ValidatedPathMapping, String> {
    let (home_root, display_prefix) = validate_home_root(mapping.home_root.as_deref())?;
    Ok(ValidatedPathMapping {
        home_root,
        display_prefix,
        claude_dir: validate_relative_posix_dir(
            mapping.claude_dir.as_deref(),
            ".claude",
            "Claude 目录映射",
        )?,
        codex_dir: validate_relative_posix_dir(
            mapping.codex_dir.as_deref(),
            ".codex",
            "Codex 目录映射",
        )?,
        opencode_dir: validate_relative_posix_dir(
            mapping.opencode_dir.as_deref(),
            ".config/opencode",
            "OpenCode 目录映射",
        )?,
        omp_dir: validate_relative_posix_dir(
            mapping.omp_dir.as_deref(),
            ".omp/agent",
            "omp 目录映射",
        )?,
    })
}

fn looks_like_windows_local_path(value: &str) -> bool {
    let bytes = value.as_bytes();
    (bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/'))
        || value.starts_with("\\\\")
}

/// Normalizes a local MCP server definition for a remote POSIX target:
/// - Unwraps Windows `cmd /c <bin> ...` wrappers into direct POSIX `<bin> ...`.
/// - Skips stdio servers bound to host-local Windows drive paths (`C:\...`) with a warning.
pub(crate) fn normalize_mcp_server_for_posix(
    server: &McpServer,
    warnings: &mut Vec<String>,
) -> Option<McpServer> {
    let Some(obj) = server.server.as_object() else {
        return Some(server.clone());
    };
    let command = obj
        .get("command")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let Some(cmd) = command else {
        return Some(server.clone());
    };

    let mut next_obj = obj.clone();
    let lower_cmd = cmd.to_ascii_lowercase();
    if lower_cmd == "cmd" || lower_cmd == "cmd.exe" {
        if let Some(args) = obj.get("args").and_then(Value::as_array) {
            let str_args: Vec<&str> = args.iter().filter_map(Value::as_str).collect();
            if str_args.len() >= 2 && str_args[0].eq_ignore_ascii_case("/c") {
                let real_cmd = str_args[1].trim();
                if looks_like_windows_local_path(real_cmd) {
                    warnings.push(format!(
                        "MCP「{}」引用了 Windows 本机路径命令，远程 POSIX 环境不可直接执行，已跳过。",
                        server.id
                    ));
                    return None;
                }
                next_obj.insert("command".to_string(), Value::String(real_cmd.to_string()));
                let rest: Vec<Value> = str_args[2..]
                    .iter()
                    .map(|s| Value::String((*s).to_string()))
                    .collect();
                next_obj.insert("args".to_string(), Value::Array(rest));
                warnings.push(format!(
                    "MCP「{}」已自动将 Windows cmd /c 包装转换为 POSIX 直接调用 `{}`。",
                    server.id, real_cmd
                ));
            }
        }
    } else if looks_like_windows_local_path(cmd) {
        warnings.push(format!(
            "MCP「{}」使用 Windows 本机路径命令，远程 POSIX 环境不可直接执行，已跳过。",
            server.id
        ));
        return None;
    }

    if let Some(cwd) = next_obj
        .get("cwd")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        if looks_like_windows_local_path(cwd) {
            next_obj.remove("cwd");
            warnings.push(format!(
                "MCP「{}」的工作目录为 Windows 本机路径，远程渲染时已忽略 cwd。",
                server.id
            ));
        }
    }

    let mut cloned = server.clone();
    cloned.server = Value::Object(next_obj);
    Some(cloned)
}

fn summarize_endpoint_host(base_url: &str) -> String {
    reqwest::Url::parse(base_url)
        .ok()
        .and_then(|u| u.host_str().map(ToOwned::to_owned))
        .unwrap_or_else(|| "自定义网关".to_string())
}

#[derive(Debug, Clone)]
pub struct RenderedRemoteFile {
    /// Clean relative path under target root (e.g. `.codex/config.toml`).
    pub remote_rel_path: String,
    pub category: RemoteFileCategory,
    /// Octal permissions string (`"600"` for secret-bearing files, `"644"` otherwise).
    pub mode: &'static str,
    pub contains_secret: bool,
    /// Redacted human-readable content summary (NEVER contains API keys or tokens).
    pub summary: String,
    pub content: Zeroizing<Vec<u8>>,
}

pub struct RenderedPushBundle {
    pub files: Vec<RenderedRemoteFile>,
    pub warnings: Vec<String>,
    pub target_root: Option<String>,
    pub display_prefix: String,
}

fn sanitize_toml_ident(raw: &str) -> String {
    let cleaned: String = raw
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_') {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let trimmed = cleaned.trim_matches('-');
    if trimmed.is_empty() {
        "fyagent".to_string()
    } else {
        trimmed.chars().take(48).collect()
    }
}

fn format_toml_string(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "\"\"".to_string())
}

fn append_codex_mcp_toml(
    out: &mut String,
    mcp_servers: &[McpServer],
    warnings: &mut Vec<String>,
) -> bool {
    let mut used_keys: HashSet<String> = HashSet::new();
    let mut has_secrets = false;

    for server in mcp_servers {
        if !server.apps.codex {
            continue;
        }
        let Some(obj) = server.server.as_object() else {
            continue;
        };

        let command = obj
            .get("command")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty());
        let url = obj
            .get("url")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty());

        if command.is_none() && url.is_none() {
            warnings.push(format!(
                "Codex MCP「{}」缺少 command/url，已跳过远程渲染。",
                server.id
            ));
            continue;
        }

        let base_key = sanitize_toml_ident(&server.id);
        let mut mcp_key = base_key.clone();
        let mut suffix = 2usize;
        while !used_keys.insert(mcp_key.clone()) {
            mcp_key = format!("{base_key}-{suffix}");
            suffix += 1;
        }

        out.push_str(&format!("\n[mcp_servers.{mcp_key}]\n"));
        // Enforce strict stdio vs HTTP transport mutual exclusion for Codex TOML.
        if let Some(cmd) = command {
            out.push_str(&format!("command = {}\n", format_toml_string(cmd)));
            if let Some(args) = obj.get("args").and_then(Value::as_array) {
                let rendered_args: Vec<String> = args
                    .iter()
                    .filter_map(Value::as_str)
                    .map(format_toml_string)
                    .collect();
                out.push_str(&format!("args = [{}]\n", rendered_args.join(", ")));
            }
            if let Some(cwd) = obj
                .get("cwd")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                out.push_str(&format!("cwd = {}\n", format_toml_string(cwd)));
            }
            if let Some(env_map) = obj.get("env").and_then(Value::as_object) {
                if !env_map.is_empty() {
                    has_secrets = true;
                    out.push_str(&format!("\n[mcp_servers.{mcp_key}.env]\n"));
                    for (k, v) in env_map {
                        if let Some(vs) = v.as_str() {
                            out.push_str(&format!(
                                "{} = {}\n",
                                format_toml_string(k),
                                format_toml_string(vs)
                            ));
                        }
                    }
                }
            }
        } else if let Some(u) = url {
            out.push_str(&format!("url = {}\n", format_toml_string(u)));
            if let Some(env_var) = obj
                .get("bearer_token_env_var")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                has_secrets = true;
                out.push_str(&format!(
                    "bearer_token_env_var = {}\n",
                    format_toml_string(env_var)
                ));
            }
            let headers_map = obj
                .get("http_headers")
                .or_else(|| obj.get("headers"))
                .and_then(Value::as_object);
            if let Some(headers) = headers_map {
                if !headers.is_empty() {
                    has_secrets = true;
                    out.push_str(&format!("\n[mcp_servers.{mcp_key}.http_headers]\n"));
                    for (k, v) in headers {
                        if let Some(vs) = v.as_str() {
                            out.push_str(&format!(
                                "{} = {}\n",
                                format_toml_string(k),
                                format_toml_string(vs)
                            ));
                        }
                    }
                }
            }
        }
    }

    has_secrets
}

fn render_codex_config_toml(
    provider_id: &str,
    provider_name: &str,
    base_url: &str,
    model_id: &str,
    protocol: Option<ApiProtocol>,
    mcp_servers: &[McpServer],
    warnings: &mut Vec<String>,
) -> (String, bool) {
    let safe_id = sanitize_toml_ident(provider_id);
    let wire_api = match protocol {
        Some(ApiProtocol::Chat) => "chat",
        _ => "responses",
    };

    let mut out = format!(
        "# Generated by FyAgent remote config push\nmodel_provider = {}\nmodel = {}\n\n[model_providers.{}]\nname = {}\nbase_url = {}\nwire_api = {}\nrequires_openai_auth = true\n",
        format_toml_string(&safe_id),
        format_toml_string(model_id),
        safe_id,
        format_toml_string(provider_name),
        format_toml_string(base_url),
        format_toml_string(wire_api),
    );

    let has_secrets = append_codex_mcp_toml(&mut out, mcp_servers, warnings);
    (out, has_secrets)
}

fn render_codex_mcp_only_toml(
    mcp_servers: &[McpServer],
    warnings: &mut Vec<String>,
) -> Option<(String, bool)> {
    if !mcp_servers.iter().any(|s| s.apps.codex) {
        return None;
    }
    let mut out = String::from("# Generated by FyAgent remote MCP config push\n");
    let before = out.len();
    let has_secrets = append_codex_mcp_toml(&mut out, mcp_servers, warnings);
    if out.len() == before {
        None
    } else {
        Some((out, has_secrets))
    }
}

fn build_opencode_mcp_map(
    mcp_servers: &[McpServer],
    warnings: &mut Vec<String>,
) -> (Map<String, Value>, bool) {
    let mut mcp_obj = Map::new();
    let mut has_secrets = false;
    for server in mcp_servers {
        if !server.apps.opencode {
            continue;
        }
        match crate::mcp::convert_to_opencode_format(&server.server) {
            Ok(converted) => {
                if converted
                    .get("environment")
                    .and_then(Value::as_object)
                    .is_some_and(|m| !m.is_empty())
                    || converted
                        .get("headers")
                        .and_then(Value::as_object)
                        .is_some_and(|m| !m.is_empty())
                {
                    has_secrets = true;
                }
                mcp_obj.insert(server.id.clone(), converted);
            }
            Err(err) => {
                warnings.push(format!(
                    "OpenCode MCP「{}」格式转换失败，已跳过：{err}",
                    server.id
                ));
            }
        }
    }
    (mcp_obj, has_secrets)
}

fn render_opencode_json(
    model: &ProjectedModelConfig,
    api_key: &str,
    mcp_servers: &[McpServer],
    warnings: &mut Vec<String>,
) -> Result<Vec<u8>, String> {
    let mut models_obj = Map::new();
    models_obj.insert(model.model_id.clone(), json!({ "name": model.model_id }));
    for extra in &model.extra_model_ids {
        if !extra.trim().is_empty() {
            models_obj.insert(extra.clone(), json!({ "name": extra }));
        }
    }

    let mut root = Map::new();
    root.insert(
        "$schema".to_string(),
        Value::String("https://opencode.ai/config.json".to_string()),
    );
    root.insert(
        "model".to_string(),
        Value::String(format!("{}/{}", model.provider_id, model.model_id)),
    );
    root.insert(
        "provider".to_string(),
        json!({
            &model.provider_id: {
                "npm": "@ai-sdk/openai-compatible",
                "name": model.provider_name,
                "options": {
                    "baseURL": model.base_url,
                    "apiKey": api_key,
                },
                "models": models_obj,
            }
        }),
    );

    let (mcp_obj, _) = build_opencode_mcp_map(mcp_servers, warnings);
    if !mcp_obj.is_empty() {
        root.insert("mcp".to_string(), Value::Object(mcp_obj));
    }

    serde_json::to_vec_pretty(&Value::Object(root))
        .map_err(|e| format!("渲染 OpenCode 配置失败: {e}"))
}

fn render_omp_files(
    provider_id: &str,
    base_url: &str,
    model_id: &str,
    api_key: &str,
    protocol: Option<ApiProtocol>,
) -> (String, String) {
    let api_kind = match protocol {
        Some(ApiProtocol::Anthropic) => "anthropic-messages",
        Some(ApiProtocol::Responses) => "openai-responses",
        _ => "openai-completions",
    };
    let safe_id = sanitize_toml_ident(provider_id);
    let models_yml = format!(
        "# Generated by FyAgent remote config push\nproviders:\n  {}:\n    baseUrl: {}\n    apiKey: {}\n    api: {}\n    models:\n      - id: {}\n",
        safe_id,
        format_toml_string(base_url),
        format_toml_string(api_key),
        format_toml_string(api_kind),
        format_toml_string(model_id),
    );
    let config_yml = format!(
        "# Generated by FyAgent remote config push\nmodelRoles:\n  default: {}\n",
        format_toml_string(&format!("{safe_id}/{model_id}")),
    );
    (models_yml, config_yml)
}

pub fn render_remote_push_bundle_with_mapping(
    state: &AppState,
    selection: &RemotePushSelection,
    path_mapping: Option<&RemotePathMapping>,
    backup_dir_name: &str,
) -> Result<RenderedPushBundle, String> {
    let mapping = match path_mapping {
        Some(raw) => validate_path_mapping(raw)?,
        None => validate_path_mapping(&RemotePathMapping::default())?,
    };
    let mut files: Vec<RenderedRemoteFile> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();

    let allowed_targets: Vec<BatchSetupTargetId> = if selection.model_targets.is_empty() {
        vec![
            BatchSetupTargetId::Claude,
            BatchSetupTargetId::Codex,
            BatchSetupTargetId::Opencode,
            BatchSetupTargetId::Omp,
        ]
    } else {
        selection.model_targets.clone()
    };

    let mcp_servers: Vec<McpServer> = if selection.include_mcp {
        let (servers, w) = collect_mcp_projections(state);
        warnings.extend(w);
        servers
            .iter()
            .filter_map(|server| normalize_mcp_server_for_posix(server, &mut warnings))
            .collect()
    } else {
        Vec::new()
    };

    let mut codex_rendered = false;
    let mut opencode_rendered = false;

    if selection.include_models {
        let collected = collect_model_projections(state, true);
        warnings.extend(collected.warnings);
        let key_map: HashMap<BatchSetupTargetId, Zeroizing<String>> =
            collected.secrets.into_iter().collect();

        for model in collected.models {
            if !allowed_targets.contains(&model.target) {
                continue;
            }
            let Some(api_key) = key_map.get(&model.target) else {
                warnings.push(format!(
                    "{} 缺少静态 API Key，已跳过远程配置文件生成。",
                    model.target.as_str()
                ));
                continue;
            };
            let host = summarize_endpoint_host(&model.base_url);

            match model.target {
                BatchSetupTargetId::Claude => {
                    let body = serde_json::to_vec_pretty(&json!({
                        "env": {
                            "ANTHROPIC_BASE_URL": model.base_url,
                            "ANTHROPIC_AUTH_TOKEN": api_key.as_str(),
                            "ANTHROPIC_MODEL": model.model_id,
                        }
                    }))
                    .map_err(|e| format!("渲染 Claude 配置失败: {e}"))?;
                    files.push(RenderedRemoteFile {
                        remote_rel_path: format!("{}/settings.json", mapping.claude_dir),
                        category: RemoteFileCategory::ModelConfig,
                        mode: "600",
                        contains_secret: true,
                        summary: format!(
                            "Claude Code · 模型 {} · {}（密钥已隐藏）",
                            model.model_id, host
                        ),
                        content: Zeroizing::new(body),
                    });
                }
                BatchSetupTargetId::Codex => {
                    let (toml_text, mcp_has_secrets) = render_codex_config_toml(
                        &model.provider_id,
                        &model.provider_name,
                        &model.base_url,
                        &model.model_id,
                        model.protocol,
                        &mcp_servers,
                        &mut warnings,
                    );
                    let codex_mcp_count = mcp_servers.iter().filter(|s| s.apps.codex).count();
                    codex_rendered = true;
                    files.push(RenderedRemoteFile {
                        remote_rel_path: format!("{}/config.toml", mapping.codex_dir),
                        category: RemoteFileCategory::ModelConfig,
                        mode: if mcp_has_secrets { "600" } else { "644" },
                        contains_secret: mcp_has_secrets,
                        summary: if codex_mcp_count > 0 {
                            format!(
                                "Codex · 模型 {} · {}（含 {} 项 MCP）",
                                model.model_id, host, codex_mcp_count
                            )
                        } else {
                            format!("Codex · 模型 {} · {}", model.model_id, host)
                        },
                        content: Zeroizing::new(toml_text.into_bytes()),
                    });
                    let auth_json = serde_json::to_vec_pretty(&json!({
                        "OPENAI_API_KEY": api_key.as_str(),
                    }))
                    .map_err(|e| format!("渲染 Codex auth.json 失败: {e}"))?;
                    files.push(RenderedRemoteFile {
                        remote_rel_path: format!("{}/auth.json", mapping.codex_dir),
                        category: RemoteFileCategory::ModelConfig,
                        mode: "600",
                        contains_secret: true,
                        summary: format!("Codex API 凭据 · {}（密钥已隐藏）", host),
                        content: Zeroizing::new(auth_json),
                    });
                }
                BatchSetupTargetId::Opencode => {
                    let json_bytes = render_opencode_json(
                        &model,
                        api_key.as_str(),
                        &mcp_servers,
                        &mut warnings,
                    )?;
                    let oc_mcp_count = mcp_servers.iter().filter(|s| s.apps.opencode).count();
                    opencode_rendered = true;
                    files.push(RenderedRemoteFile {
                        remote_rel_path: format!("{}/opencode.json", mapping.opencode_dir),
                        category: RemoteFileCategory::ModelConfig,
                        mode: "600",
                        contains_secret: true,
                        summary: if oc_mcp_count > 0 {
                            format!(
                                "OpenCode · 模型 {} · {}（含 {} 项 MCP，密钥已隐藏）",
                                model.model_id, host, oc_mcp_count
                            )
                        } else {
                            format!(
                                "OpenCode · 模型 {} · {}（密钥已隐藏）",
                                model.model_id, host
                            )
                        },
                        content: Zeroizing::new(json_bytes),
                    });
                }
                BatchSetupTargetId::Omp => {
                    let (models_yml, config_yml) = render_omp_files(
                        &model.provider_id,
                        &model.base_url,
                        &model.model_id,
                        api_key.as_str(),
                        model.protocol,
                    );
                    let safe_id = sanitize_toml_ident(&model.provider_id);
                    files.push(RenderedRemoteFile {
                        remote_rel_path: format!("{}/models.yml", mapping.omp_dir),
                        category: RemoteFileCategory::ModelConfig,
                        mode: "600",
                        contains_secret: true,
                        summary: format!(
                            "Oh My Pi · 模型 {} · {}（密钥已隐藏）",
                            model.model_id, host
                        ),
                        content: Zeroizing::new(models_yml.into_bytes()),
                    });
                    files.push(RenderedRemoteFile {
                        remote_rel_path: format!("{}/config.yml", mapping.omp_dir),
                        category: RemoteFileCategory::ModelConfig,
                        mode: "644",
                        contains_secret: false,
                        summary: format!("Oh My Pi 默认模型 {safe_id}/{}", model.model_id),
                        content: Zeroizing::new(config_yml.into_bytes()),
                    });
                }
                BatchSetupTargetId::Workbuddy | BatchSetupTargetId::Grokbuild => {
                    warnings.push(format!(
                        "{} 为桌面端专用配置，远程同步默认跳过。",
                        model.target.as_str()
                    ));
                }
            }
        }
    }

    // Standalone MCP rendering (supports MCP-only pushes when `include_models == false`
    // as well as Claude's separate `~/.claude/mcp.json` to preserve remote OAuth state).
    if selection.include_mcp {
        let mut claude_mcp = Map::new();
        let mut claude_mcp_has_secret = false;
        for server in &mcp_servers {
            if server.apps.claude {
                if server
                    .server
                    .get("env")
                    .and_then(Value::as_object)
                    .is_some_and(|m| !m.is_empty())
                    || server
                        .server
                        .get("headers")
                        .and_then(Value::as_object)
                        .is_some_and(|m| !m.is_empty())
                {
                    claude_mcp_has_secret = true;
                }
                claude_mcp.insert(server.id.clone(), server.server.clone());
            }
        }
        if !claude_mcp.is_empty() {
            let count = claude_mcp.len();
            let body = serde_json::to_vec_pretty(&json!({ "mcpServers": claude_mcp }))
                .map_err(|e| format!("渲染 Claude MCP 清单失败: {e}"))?;
            let claude_mcp_rel = format!("{}/mcp.json", mapping.claude_dir);
            files.push(RenderedRemoteFile {
                remote_rel_path: claude_mcp_rel.clone(),
                category: RemoteFileCategory::McpConfig,
                mode: "600",
                contains_secret: claude_mcp_has_secret,
                summary: format!("Claude Code · {count} 项 MCP 服务器"),
                content: Zeroizing::new(body),
            });
            warnings.push(format!(
                "Claude MCP 已写入独立的 {}{} 以保护远端登录态，启动时可加 --mcp-config {}{} 加载。",
                mapping.display_prefix, claude_mcp_rel, mapping.display_prefix, claude_mcp_rel
            ));
        }

        if !codex_rendered {
            if let Some((toml_text, has_secrets)) =
                render_codex_mcp_only_toml(&mcp_servers, &mut warnings)
            {
                let count = mcp_servers.iter().filter(|s| s.apps.codex).count();
                files.push(RenderedRemoteFile {
                    remote_rel_path: format!("{}/config.toml", mapping.codex_dir),
                    category: RemoteFileCategory::McpConfig,
                    mode: if has_secrets { "600" } else { "644" },
                    contains_secret: has_secrets,
                    summary: format!("Codex · {count} 项 MCP 服务器"),
                    content: Zeroizing::new(toml_text.into_bytes()),
                });
            }
        }

        if !opencode_rendered {
            let (mcp_obj, has_secrets) = build_opencode_mcp_map(&mcp_servers, &mut warnings);
            if !mcp_obj.is_empty() {
                let count = mcp_obj.len();
                let body = serde_json::to_vec_pretty(&json!({
                    "$schema": "https://opencode.ai/config.json",
                    "mcp": mcp_obj,
                }))
                .map_err(|e| format!("渲染 OpenCode MCP 配置失败: {e}"))?;
                files.push(RenderedRemoteFile {
                    remote_rel_path: format!("{}/opencode.json", mapping.opencode_dir),
                    category: RemoteFileCategory::McpConfig,
                    mode: if has_secrets { "600" } else { "644" },
                    contains_secret: has_secrets,
                    summary: format!("OpenCode · {count} 项 MCP 服务器"),
                    content: Zeroizing::new(body),
                });
            }
        }
    }

    if files.iter().any(|f| {
        matches!(
            f.category,
            RemoteFileCategory::ModelConfig | RemoteFileCategory::McpConfig
        )
    }) {
        warnings.push(format!(
            "同步采用整文件原子替换，覆盖前会自动将目标原文件完整备份到 ~/{backup_dir_name}/<时间戳>/。"
        ));
    }

    if selection.include_prompts {
        let (prompts, w) = collect_prompt_projections(state);
        warnings.extend(w);
        for entry in prompts {
            if !entry.prompt.enabled || entry.prompt.content.trim().is_empty() {
                continue;
            }
            let (rel_path, app_label) = match entry.app.as_str() {
                "claude" => (format!("{}/CLAUDE.md", mapping.claude_dir), "Claude Code"),
                "codex" => (format!("{}/AGENTS.md", mapping.codex_dir), "Codex"),
                "gemini" => (".gemini/GEMINI.md".to_string(), "Gemini"),
                "opencode" => (format!("{}/AGENTS.md", mapping.opencode_dir), "OpenCode"),
                _ => continue,
            };
            if files.iter().any(|f| f.remote_rel_path == rel_path) {
                continue;
            }
            let line_count = entry.prompt.content.lines().count();
            files.push(RenderedRemoteFile {
                remote_rel_path: rel_path,
                category: RemoteFileCategory::GlobalPrompt,
                mode: "644",
                contains_secret: false,
                summary: format!("{app_label} 全局提示词（{line_count} 行）"),
                content: Zeroizing::new(entry.prompt.content.into_bytes()),
            });
        }
    }

    if selection.include_skills {
        let collected = collect_skill_projections(state);
        warnings.extend(collected.warnings);
        let mut used_skill_slugs: HashSet<String> = HashSet::new();
        for skill in collected.skills {
            if skill.files.is_empty() {
                continue;
            }
            let base_slug = sanitize_toml_ident(&skill.metadata.directory);
            let mut dir_slug = base_slug.clone();
            let mut suffix = 2usize;
            while !used_skill_slugs.insert(dir_slug.clone()) {
                dir_slug = format!("{base_slug}-{suffix}");
                suffix += 1;
            }
            let mut target_roots: Vec<String> = Vec::new();
            if skill.metadata.apps.claude {
                target_roots.push(format!("{}/skills", mapping.claude_dir));
            }
            if skill.metadata.apps.codex {
                target_roots.push(format!("{}/skills", mapping.codex_dir));
            }
            if skill.metadata.apps.opencode {
                target_roots.push(format!("{}/skills", mapping.opencode_dir));
            }
            if target_roots.is_empty() {
                target_roots.push(format!("{}/skills", mapping.claude_dir));
            }

            for root in target_roots {
                for file in &skill.files {
                    if let Ok(clean_rel) = sanitize_relative_skill_path(&file.relative_path) {
                        files.push(RenderedRemoteFile {
                            remote_rel_path: format!("{root}/{dir_slug}/{clean_rel}"),
                            category: RemoteFileCategory::SkillFile,
                            mode: "644",
                            contains_secret: false,
                            summary: format!("Skill「{dir_slug}」"),
                            content: Zeroizing::new(file.content.as_bytes().to_vec()),
                        });
                    }
                }
            }
        }
    }

    let total_bytes: usize = files.iter().map(|f| f.content.len()).sum();
    if total_bytes > MAX_REMOTE_PUSH_TOTAL_BYTES {
        return Err(format!(
            "待推送文件总计 {:.1} MiB，超出单次远程推送 20 MiB 安全上限，请取消勾选大型 Skills 后重试",
            total_bytes as f64 / (1024.0 * 1024.0)
        ));
    }

    Ok(RenderedPushBundle {
        files,
        warnings,
        target_root: mapping.home_root,
        display_prefix: mapping.display_prefix,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_config::McpApps;

    #[test]
    fn renders_valid_codex_toml_and_opencode_and_omp_configs() {
        let mut warnings = Vec::new();
        let mcp = vec![
            McpServer {
                id: "my.mcp".to_string(),
                name: "First".to_string(),
                server: json!({
                    "command": "npx",
                    "args": ["-y", "mcp-a"],
                    "cwd": "/tmp",
                    "env": { "API_TOKEN": "tok-123" }
                }),
                apps: McpApps {
                    codex: true,
                    opencode: true,
                    ..Default::default()
                },
                description: None,
                homepage: None,
                docs: None,
                tags: Vec::new(),
            },
            McpServer {
                id: "my/mcp".to_string(),
                name: "Second colliding slug".to_string(),
                server: json!({
                    "type": "http",
                    "url": "https://mcp.example.com/sse",
                    "headers": { "Authorization": "Bearer xyz" }
                }),
                apps: McpApps {
                    codex: true,
                    opencode: true,
                    ..Default::default()
                },
                description: None,
                homepage: None,
                docs: None,
                tags: Vec::new(),
            },
        ];

        let (toml_text, has_secrets) = render_codex_config_toml(
            "deepseek",
            "DeepSeek",
            "https://api.deepseek.com/v1",
            "deepseek-chat",
            Some(ApiProtocol::Chat),
            &mcp,
            &mut warnings,
        );
        assert!(has_secrets);
        let parsed: toml::Value =
            toml::from_str(&toml_text).expect("valid deduplicated codex toml");
        assert_eq!(
            parsed.get("model").and_then(|v| v.as_str()),
            Some("deepseek-chat")
        );
        let mcp_tables = parsed
            .get("mcp_servers")
            .and_then(|v| v.as_table())
            .expect("mcp_servers table");
        assert!(mcp_tables.contains_key("my-mcp"));
        assert!(mcp_tables.contains_key("my-mcp-2"));

        let oc_model = ProjectedModelConfig {
            target: BatchSetupTargetId::Opencode,
            provider_id: "deepseek".to_string(),
            provider_name: "DeepSeek".to_string(),
            base_url: "https://api.deepseek.com/v1".to_string(),
            model_id: "deepseek-chat".to_string(),
            extra_model_ids: Vec::new(),
            protocol: Some(ApiProtocol::Chat),
            has_secret_in_pack: true,
        };
        let oc = render_opencode_json(&oc_model, "sk-test", &mcp, &mut warnings).unwrap();
        let oc_json: Value = serde_json::from_slice(&oc).expect("valid opencode json");
        assert_eq!(
            oc_json.get("model").and_then(Value::as_str),
            Some("deepseek/deepseek-chat")
        );
        // Must use OpenCode's native `type: "local"` + `command: [...]` + `environment` schema!
        assert_eq!(oc_json["mcp"]["my.mcp"]["type"].as_str(), Some("local"));
        assert_eq!(
            oc_json["mcp"]["my.mcp"]["environment"]["API_TOKEN"].as_str(),
            Some("tok-123")
        );
        assert_eq!(oc_json["mcp"]["my/mcp"]["type"].as_str(), Some("remote"));

        let (models_yml, config_yml) = render_omp_files(
            "deepseek",
            "https://api.deepseek.com/v1",
            "deepseek-chat",
            "sk-test",
            Some(ApiProtocol::Chat),
        );
        let yml_val: serde_yaml::Value =
            serde_yaml::from_str(&models_yml).expect("valid omp models yaml");
        assert!(yml_val.get("providers").is_some());
        let cfg_val: serde_yaml::Value =
            serde_yaml::from_str(&config_yml).expect("valid omp config yaml");
        assert_eq!(
            cfg_val["modelRoles"]["default"].as_str(),
            Some("deepseek/deepseek-chat")
        );
    }

    #[test]
    fn validates_custom_path_mappings_and_blocks_traversal() {
        let ok = validate_path_mapping(&RemotePathMapping {
            home_root: Some("~/work/profile".to_string()),
            claude_dir: Some(".claude-alt".to_string()),
            codex_dir: None,
            opencode_dir: Some(".config/opencode-dev".to_string()),
            omp_dir: None,
        })
        .unwrap();
        assert_eq!(ok.home_root.as_deref(), Some("~/work/profile"));
        assert_eq!(ok.display_prefix, "~/work/profile/");
        assert_eq!(ok.claude_dir, ".claude-alt");
        assert_eq!(ok.codex_dir, ".codex");
        assert_eq!(ok.opencode_dir, ".config/opencode-dev");

        assert!(validate_path_mapping(&RemotePathMapping {
            home_root: Some("~/../etc".to_string()),
            ..Default::default()
        })
        .is_err());
        assert!(validate_path_mapping(&RemotePathMapping {
            home_root: Some("/".to_string()),
            ..Default::default()
        })
        .is_err());
        assert!(validate_path_mapping(&RemotePathMapping {
            claude_dir: Some("/abs/path".to_string()),
            ..Default::default()
        })
        .is_err());
        assert!(validate_path_mapping(&RemotePathMapping {
            codex_dir: Some("foo;rm -rf /".to_string()),
            ..Default::default()
        })
        .is_err());
    }

    #[test]
    fn normalizes_windows_cmd_wrapper_and_skips_drive_paths_for_posix_targets() {
        let mut warnings = Vec::new();
        let wrapped = McpServer {
            id: "playwright".to_string(),
            name: "Playwright".to_string(),
            server: json!({
                "type": "stdio",
                "command": "cmd",
                "args": ["/c", "npx", "-y", "@playwright/mcp@latest"],
                "cwd": "C:\\Users\\demo\\project"
            }),
            apps: McpApps {
                claude: true,
                ..Default::default()
            },
            description: None,
            homepage: None,
            docs: None,
            tags: Vec::new(),
        };
        let normalized = normalize_mcp_server_for_posix(&wrapped, &mut warnings).unwrap();
        assert_eq!(normalized.server["command"], "npx");
        assert_eq!(
            normalized.server["args"],
            json!(["-y", "@playwright/mcp@latest"])
        );
        assert!(normalized.server.get("cwd").is_none());
        assert!(warnings.iter().any(|w| w.contains("cmd /c")));
        assert!(warnings.iter().any(|w| w.contains("cwd")));

        let local_exe = McpServer {
            id: "local-tool".to_string(),
            name: "Local Exe".to_string(),
            server: json!({
                "type": "stdio",
                "command": "C:\\tools\\my-mcp.exe",
                "args": []
            }),
            apps: McpApps {
                claude: true,
                ..Default::default()
            },
            description: None,
            homepage: None,
            docs: None,
            tags: Vec::new(),
        };
        assert!(normalize_mcp_server_for_posix(&local_exe, &mut warnings).is_none());
        assert!(warnings.iter().any(|w| w.contains("local-tool")));
    }
}
