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

#[derive(Debug, Clone)]
pub struct RenderedRemoteFile {
    /// Clean relative path under remote `$HOME` (e.g. `.codex/config.toml`).
    pub remote_rel_path: String,
    pub category: RemoteFileCategory,
    /// Octal permissions string (`"600"` for secret-bearing files, `"644"` otherwise).
    pub mode: &'static str,
    pub contains_secret: bool,
    pub content: Zeroizing<Vec<u8>>,
}

pub struct RenderedPushBundle {
    pub files: Vec<RenderedRemoteFile>,
    pub warnings: Vec<String>,
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

pub fn render_remote_push_bundle(
    state: &AppState,
    selection: &RemotePushSelection,
) -> Result<RenderedPushBundle, String> {
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

    let mcp_servers = if selection.include_mcp {
        let (servers, w) = collect_mcp_projections(state);
        warnings.extend(w);
        servers
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
                        remote_rel_path: ".claude/settings.json".to_string(),
                        category: RemoteFileCategory::ModelConfig,
                        mode: "600",
                        contains_secret: true,
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
                    codex_rendered = true;
                    files.push(RenderedRemoteFile {
                        remote_rel_path: ".codex/config.toml".to_string(),
                        category: RemoteFileCategory::ModelConfig,
                        mode: if mcp_has_secrets { "600" } else { "644" },
                        contains_secret: mcp_has_secrets,
                        content: Zeroizing::new(toml_text.into_bytes()),
                    });
                    let auth_json = serde_json::to_vec_pretty(&json!({
                        "OPENAI_API_KEY": api_key.as_str(),
                    }))
                    .map_err(|e| format!("渲染 Codex auth.json 失败: {e}"))?;
                    files.push(RenderedRemoteFile {
                        remote_rel_path: ".codex/auth.json".to_string(),
                        category: RemoteFileCategory::ModelConfig,
                        mode: "600",
                        contains_secret: true,
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
                    opencode_rendered = true;
                    files.push(RenderedRemoteFile {
                        remote_rel_path: ".config/opencode/opencode.json".to_string(),
                        category: RemoteFileCategory::ModelConfig,
                        mode: "600",
                        contains_secret: true,
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
                    files.push(RenderedRemoteFile {
                        remote_rel_path: ".omp/agent/models.yml".to_string(),
                        category: RemoteFileCategory::ModelConfig,
                        mode: "600",
                        contains_secret: true,
                        content: Zeroizing::new(models_yml.into_bytes()),
                    });
                    files.push(RenderedRemoteFile {
                        remote_rel_path: ".omp/agent/config.yml".to_string(),
                        category: RemoteFileCategory::ModelConfig,
                        mode: "644",
                        contains_secret: false,
                        content: Zeroizing::new(config_yml.into_bytes()),
                    });
                }
                BatchSetupTargetId::Workbuddy | BatchSetupTargetId::Grokbuild => {
                    warnings.push(format!(
                        "{} 为桌面端专用配置，远程 SSH 推送默认跳过。",
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
            let body = serde_json::to_vec_pretty(&json!({ "mcpServers": claude_mcp }))
                .map_err(|e| format!("渲染 Claude MCP 清单失败: {e}"))?;
            files.push(RenderedRemoteFile {
                remote_rel_path: ".claude/mcp.json".to_string(),
                category: RemoteFileCategory::McpConfig,
                mode: "600",
                contains_secret: claude_mcp_has_secret,
                content: Zeroizing::new(body),
            });
            warnings.push(
                "Claude MCP 已写入独立的 ~/.claude/mcp.json 以保护远端 ~/.claude.json 登录态，启动时可加 --mcp-config ~/.claude/mcp.json 加载。"
                    .to_string(),
            );
        }

        if !codex_rendered {
            if let Some((toml_text, has_secrets)) =
                render_codex_mcp_only_toml(&mcp_servers, &mut warnings)
            {
                files.push(RenderedRemoteFile {
                    remote_rel_path: ".codex/config.toml".to_string(),
                    category: RemoteFileCategory::McpConfig,
                    mode: if has_secrets { "600" } else { "644" },
                    contains_secret: has_secrets,
                    content: Zeroizing::new(toml_text.into_bytes()),
                });
            }
        }

        if !opencode_rendered {
            let (mcp_obj, has_secrets) = build_opencode_mcp_map(&mcp_servers, &mut warnings);
            if !mcp_obj.is_empty() {
                let body = serde_json::to_vec_pretty(&json!({
                    "$schema": "https://opencode.ai/config.json",
                    "mcp": mcp_obj,
                }))
                .map_err(|e| format!("渲染 OpenCode MCP 配置失败: {e}"))?;
                files.push(RenderedRemoteFile {
                    remote_rel_path: ".config/opencode/opencode.json".to_string(),
                    category: RemoteFileCategory::McpConfig,
                    mode: if has_secrets { "600" } else { "644" },
                    contains_secret: has_secrets,
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
        warnings.push(
            "远程推送采用整文件原子替换，覆盖前会自动将远端原文件完整备份到 ~/.fyagent-remote-backups/<时间戳>/。"
                .to_string(),
        );
    }

    if selection.include_prompts {
        let (prompts, w) = collect_prompt_projections(state);
        warnings.extend(w);
        for entry in prompts {
            if !entry.prompt.enabled || entry.prompt.content.trim().is_empty() {
                continue;
            }
            let rel_path = match entry.app.as_str() {
                "claude" => ".claude/CLAUDE.md",
                "codex" => ".codex/AGENTS.md",
                "gemini" => ".gemini/GEMINI.md",
                "opencode" => ".config/opencode/AGENTS.md",
                _ => continue,
            };
            if files.iter().any(|f| f.remote_rel_path == rel_path) {
                continue;
            }
            files.push(RenderedRemoteFile {
                remote_rel_path: rel_path.to_string(),
                category: RemoteFileCategory::GlobalPrompt,
                mode: "644",
                contains_secret: false,
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
            let mut target_roots: Vec<&str> = Vec::new();
            if skill.metadata.apps.claude {
                target_roots.push(".claude/skills");
            }
            if skill.metadata.apps.codex {
                target_roots.push(".codex/skills");
            }
            if skill.metadata.apps.opencode {
                target_roots.push(".config/opencode/skills");
            }
            if target_roots.is_empty() {
                target_roots.push(".claude/skills");
            }

            for root in target_roots {
                for file in &skill.files {
                    if let Ok(clean_rel) = sanitize_relative_skill_path(&file.relative_path) {
                        files.push(RenderedRemoteFile {
                            remote_rel_path: format!("{root}/{dir_slug}/{clean_rel}"),
                            category: RemoteFileCategory::SkillFile,
                            mode: "644",
                            contains_secret: false,
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

    Ok(RenderedPushBundle { files, warnings })
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
}
