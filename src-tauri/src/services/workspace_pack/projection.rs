//! Shared projection engine for `.fyagent-pack` and agentless SSH config push.
//!
//! Converts live local configuration & transcripts into bounded, portable,
//! text-only projections (zero raw database dumps, zero tool-output blobs,
//! zero OAuth session tokens).

use std::collections::HashSet;
use std::fs;
use std::path::{Component, Path};

use zeroize::Zeroizing;

use super::schema::{
    sha256_hex, ProjectedDialogueTurn, ProjectedModelConfig, ProjectedPromptEntry,
    ProjectedSessionEntry, ProjectedSessionKind, ProjectedSkillEntry, ProjectedSkillFile,
    SessionProjectionMode, MAX_SKILL_FILE_BYTES, SKILLS_UNCOMPRESSED_BUDGET_BYTES,
};
use crate::{
    app_config::{AppType, McpServer},
    prompt::Prompt,
    services::{
        mcp::McpService,
        prompt::PromptService,
        provider::{
            batch_setup::BatchSetupTargetId,
            custom_templates::{list_custom_gateway_templates, CustomGatewayTemplate},
            model_overview::{inspect_target, ModelCredentialSource, ALL_CONFIGURABLE_TARGETS},
        },
        skill::SkillService,
    },
    session_manager::{
        self,
        migrate::export::preview_session,
        model::{SessionBlock, SessionMessage},
    },
    store::AppState,
};

const IGNORED_SKILL_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    "dist",
    "build",
    "__pycache__",
    ".venv",
    ".cache",
];

/// Conservative upper bound on raw uncompressed JSON bytes allocated to sessions
/// before zstd compression (~4x–8x on dialogue JSON). Final compressed bytes are
/// still hard-checked against `HARD_LIMIT_BYTES` (100 MiB) with deterministic
/// oldest-session pruning.
pub const SESSION_UNCOMPRESSED_SOFT_CAP_BYTES: u64 = 200 * 1024 * 1024;
const MAX_COMPACT_TRAILING_TURNS: usize = 6;
const MAX_SESSION_MESSAGE_CHARS: usize = 64 * 1024;

pub struct CollectedModels {
    pub models: Vec<ProjectedModelConfig>,
    pub secrets: Vec<(BatchSetupTargetId, Zeroizing<String>)>,
    pub warnings: Vec<String>,
}

pub struct CollectedSkills {
    pub skills: Vec<ProjectedSkillEntry>,
    pub warnings: Vec<String>,
}

pub struct SessionProjectionOptions {
    pub mode: SessionProjectionMode,
    pub max_age_days: Option<u32>,
    pub max_sessions: usize,
    pub uncompressed_budget_bytes: u64,
}

pub struct CollectedSessions {
    pub sessions: Vec<ProjectedSessionEntry>,
    pub excluded_count: usize,
    pub warnings: Vec<String>,
}

fn target_display_name(target: BatchSetupTargetId) -> &'static str {
    match target {
        BatchSetupTargetId::Workbuddy => "WorkBuddy",
        BatchSetupTargetId::Codex => "Codex",
        BatchSetupTargetId::Claude => "Claude Code",
        BatchSetupTargetId::Opencode => "OpenCode",
        BatchSetupTargetId::Omp => "Oh My Pi (omp)",
        BatchSetupTargetId::Grokbuild => "Grok Build",
    }
}

pub fn collect_model_projections(state: &AppState, include_api_keys: bool) -> CollectedModels {
    let mut models = Vec::new();
    let mut secrets = Vec::new();
    let mut warnings = Vec::new();

    for target in ALL_CONFIGURABLE_TARGETS {
        let details = inspect_target(state, target);
        if !details.snapshot.configured {
            continue;
        }
        if details.snapshot.credential_source == ModelCredentialSource::Oauth {
            warnings.push(format!(
                "{} 当前为官方订阅登录，出于安全原因不导出登录态，请在新设备重新登录。",
                target_display_name(target)
            ));
            continue;
        }
        let Some(base_url) = details
            .snapshot
            .base_url
            .clone()
            .filter(|s| !s.trim().is_empty())
        else {
            continue;
        };
        let Some(model_id) = details
            .snapshot
            .model_id
            .clone()
            .filter(|s| !s.trim().is_empty())
        else {
            continue;
        };

        let provider_name = details
            .snapshot
            .provider_name
            .clone()
            .or_else(|| details.snapshot.endpoint_host.clone())
            .unwrap_or_else(|| "Custom Provider".to_string());
        let provider_id = details
            .snapshot
            .provider_id
            .clone()
            .unwrap_or_else(|| format!("fyagent-{}", target.as_str()));

        let has_secret_in_pack = include_api_keys && details.api_key.is_some();
        if include_api_keys {
            if let Some(key) = details.api_key {
                secrets.push((target, key));
            } else {
                warnings.push(format!(
                    "{} 使用环境变量或命令密钥，未读取到静态 API Key，导入后需自行补齐环境变量。",
                    target_display_name(target)
                ));
            }
        }

        models.push(ProjectedModelConfig {
            target,
            provider_id,
            provider_name,
            base_url,
            model_id,
            extra_model_ids: details.extra_model_ids,
            protocol: details.snapshot.protocol,
            has_secret_in_pack,
        });
    }

    CollectedModels {
        models,
        secrets,
        warnings,
    }
}

pub fn collect_custom_templates(state: &AppState) -> (Vec<CustomGatewayTemplate>, Vec<String>) {
    match list_custom_gateway_templates(&state.db) {
        Ok(items) => (items, Vec::new()),
        Err(err) => (
            Vec::new(),
            vec![format!("读取自定义模板失败，已跳过：{err}")],
        ),
    }
}

pub fn collect_mcp_projections(state: &AppState) -> (Vec<McpServer>, Vec<String>) {
    match McpService::get_all_servers(state) {
        Ok(map) => (map.into_values().collect(), Vec::new()),
        Err(err) => (
            Vec::new(),
            vec![format!("读取 MCP 清单失败，已跳过：{err}")],
        ),
    }
}

pub fn collect_prompt_projections(state: &AppState) -> (Vec<ProjectedPromptEntry>, Vec<String>) {
    let mut entries = Vec::new();
    let mut warnings = Vec::new();
    let apps = [
        AppType::Claude,
        AppType::Codex,
        AppType::Gemini,
        AppType::OpenCode,
    ];

    for app in apps {
        match PromptService::get_prompts(state, app.clone()) {
            Ok(map) if !map.is_empty() => {
                for prompt in map.into_values() {
                    entries.push(ProjectedPromptEntry {
                        app: app.as_str().to_string(),
                        prompt,
                    });
                }
            }
            Ok(_) => {
                if let Ok(Some(content)) = PromptService::get_current_file_content(app.clone()) {
                    if !content.trim().is_empty() {
                        entries.push(ProjectedPromptEntry {
                            app: app.as_str().to_string(),
                            prompt: Prompt {
                                id: format!("live-{}", app.as_str()),
                                name: format!("{} 全局提示词", app.as_str()),
                                content,
                                description: Some("由当前生效文件自动投影".to_string()),
                                enabled: true,
                                created_at: None,
                                updated_at: None,
                            },
                        });
                    }
                }
            }
            Err(err) => {
                warnings.push(format!("读取 {} 提示词失败：{err}", app.as_str()));
            }
        }
    }

    (entries, warnings)
}

pub fn sanitize_relative_skill_path(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim().replace('\\', "/");
    if trimmed.is_empty() || trimmed.starts_with('/') {
        return Err("技能相对路径不能为空或为绝对路径".to_string());
    }
    let path = Path::new(&trimmed);
    let mut clean_parts = Vec::new();
    for comp in path.components() {
        match comp {
            Component::Normal(part) => {
                let text = part
                    .to_str()
                    .ok_or_else(|| "技能路径包含无效字符".to_string())?;
                if text.is_empty() || text == "." || text == ".." {
                    return Err("技能路径包含非法段".to_string());
                }
                clean_parts.push(text);
            }
            _ => return Err("技能路径不允许包含上级目录或前缀".to_string()),
        }
    }
    if clean_parts.is_empty() {
        return Err("技能路径无效".to_string());
    }
    Ok(clean_parts.join("/"))
}

fn collect_skill_files_recursive(
    root: &Path,
    dir: &Path,
    remaining_budget: &mut u64,
    out: &mut Vec<ProjectedSkillFile>,
    omitted_large: &mut usize,
) {
    let Ok(read_dir) = fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<_> = read_dir.flatten().collect();
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        // Never follow symlinks when bundling portable skills.
        if file_type.is_symlink() {
            continue;
        }
        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if file_type.is_dir() {
            if IGNORED_SKILL_DIRS.contains(&name_str.as_ref()) {
                continue;
            }
            collect_skill_files_recursive(root, &path, remaining_budget, out, omitted_large);
            continue;
        }
        if !file_type.is_file() {
            continue;
        }
        let Ok(meta) = entry.metadata() else {
            continue;
        };
        let len = meta.len();
        if len > MAX_SKILL_FILE_BYTES || len > *remaining_budget {
            *omitted_large += 1;
            continue;
        }
        let Ok(bytes) = fs::read(&path) else {
            continue;
        };
        // Skip binary files (NUL byte or invalid UTF-8).
        if bytes.contains(&0) {
            *omitted_large += 1;
            continue;
        }
        let Ok(content) = String::from_utf8(bytes) else {
            *omitted_large += 1;
            continue;
        };
        let Ok(rel) = path.strip_prefix(root) else {
            continue;
        };
        let Ok(clean_rel) = sanitize_relative_skill_path(&rel.to_string_lossy()) else {
            continue;
        };
        *remaining_budget = remaining_budget.saturating_sub(content.len() as u64);
        out.push(ProjectedSkillFile {
            relative_path: clean_rel,
            content,
        });
    }
}

pub fn collect_skill_projections(state: &AppState) -> CollectedSkills {
    let mut skills = Vec::new();
    let mut warnings = Vec::new();
    let installed = match SkillService::get_all_installed(&state.db) {
        Ok(list) => list,
        Err(err) => {
            warnings.push(format!("读取已安装 Skills 失败，已跳过：{err}"));
            return CollectedSkills { skills, warnings };
        }
    };
    let ssot_dir = SkillService::get_ssot_dir().ok();
    let mut remaining_budget = SKILLS_UNCOMPRESSED_BUDGET_BYTES;
    let mut total_omitted = 0usize;

    for meta in installed {
        let mut files = Vec::new();
        let mut omitted_large_files = 0usize;
        if let Some(root) = ssot_dir.as_ref() {
            let skill_dir = root.join(&meta.directory);
            if skill_dir.is_dir() {
                collect_skill_files_recursive(
                    &skill_dir,
                    &skill_dir,
                    &mut remaining_budget,
                    &mut files,
                    &mut omitted_large_files,
                );
            }
        }
        total_omitted += omitted_large_files;
        skills.push(ProjectedSkillEntry {
            metadata: meta,
            files,
            omitted_large_files,
        });
    }

    if total_omitted > 0 {
        warnings.push(format!(
            "Skills 投影自动跳过了 {total_omitted} 个二进制或超大文件（单文件上限 256 KiB / 总上限 15 MiB）。"
        ));
    }

    CollectedSkills { skills, warnings }
}

fn workspace_basename(project_dir: Option<&str>) -> Option<String> {
    let raw = project_dir?.trim();
    if raw.is_empty() {
        return None;
    }
    raw.rsplit(['/', '\\'])
        .find(|part| !part.is_empty())
        .map(|part| part.chars().take(120).collect())
}

fn strip_data_uris(text: &str) -> String {
    if !text.contains("data:image/") && !text.contains("base64,") {
        return text.to_string();
    }
    // Strip inline base64 blobs to guarantee zero megabyte-scale image strings in projections.
    let mut out = String::with_capacity(text.len().min(4096));
    let mut rest = text;
    while let Some(idx) = rest.find("data:") {
        out.push_str(&rest[..idx]);
        let after = &rest[idx..];
        let end = after
            .find(|c: char| c.is_whitespace() || matches!(c, ')' | '"' | '\'' | '>'))
            .unwrap_or(after.len());
        let candidate = &after[..end];
        if candidate.contains(";base64,") && candidate.len() > 256 {
            out.push_str("[omitted-inline-attachment]");
        } else {
            out.push_str(candidate);
        }
        rest = &after[end..];
    }
    out.push_str(rest);
    out
}

fn cap_message_text(raw: &str) -> String {
    let cleaned = strip_data_uris(raw);
    if cleaned.chars().count() <= MAX_SESSION_MESSAGE_CHARS {
        return cleaned;
    }
    let prefix: String = cleaned.chars().take(MAX_SESSION_MESSAGE_CHARS).collect();
    format!("{prefix}\n\n[...内容超过 64 KiB，已在轻量投影中折叠...]")
}

fn extract_portable_turns(messages: &[SessionMessage]) -> (Vec<ProjectedDialogueTurn>, usize) {
    let mut turns = Vec::new();
    let mut omitted_blocks = 0usize;

    for msg in messages {
        if msg.injected {
            omitted_blocks += 1;
            continue;
        }
        let role = match msg.role.as_str() {
            "user" => "user",
            "assistant" => "assistant",
            _ => {
                omitted_blocks += 1;
                continue;
            }
        };

        let text = if !msg.blocks.is_empty() {
            let mut parts = Vec::new();
            for block in &msg.blocks {
                match block {
                    SessionBlock::Text { text, .. } => {
                        let trimmed = text.trim();
                        if !trimmed.is_empty() {
                            parts.push(trimmed);
                        }
                    }
                    _ => {
                        omitted_blocks += 1;
                    }
                }
            }
            parts.join("\n\n")
        } else {
            msg.content.trim().to_string()
        };

        if text.is_empty() {
            continue;
        }
        turns.push(ProjectedDialogueTurn {
            role: role.to_string(),
            text: cap_message_text(&text),
            ts: msg.ts,
        });
    }

    (turns, omitted_blocks)
}

fn apply_compact_window(turns: Vec<ProjectedDialogueTurn>) -> (Vec<ProjectedDialogueTurn>, bool) {
    if turns.len() <= MAX_COMPACT_TRAILING_TURNS + 2 {
        return (turns, false);
    }
    let mut compact = Vec::with_capacity(MAX_COMPACT_TRAILING_TURNS + 2);
    compact.push(turns[0].clone());
    if turns.len() > 1 && turns[1].role == "assistant" {
        compact.push(turns[1].clone());
    }
    let tail_start = turns.len().saturating_sub(MAX_COMPACT_TRAILING_TURNS);
    let head_len = compact.len();
    for turn in turns.into_iter().skip(tail_start.max(head_len)) {
        compact.push(turn);
    }
    (compact, true)
}

fn digest_turns(turns: &[ProjectedDialogueTurn]) -> String {
    let mut material = Vec::new();
    for turn in turns {
        material.extend_from_slice(turn.role.as_bytes());
        material.push(0);
        material.extend_from_slice(turn.text.as_bytes());
        material.push(0xff);
    }
    sha256_hex(&material)
}

fn normalize_timestamp_ms(ts: i64) -> i64 {
    if ts > 0 && ts < 10_000_000_000 {
        ts * 1000
    } else {
        ts
    }
}

pub fn collect_session_projections(
    state: &AppState,
    options: &SessionProjectionOptions,
) -> CollectedSessions {
    let all_metas = session_manager::scan_sessions();
    let now_ms = chrono::Utc::now().timestamp_millis();
    let min_ts_ms = options
        .max_age_days
        .map(|days| now_ms - (days as i64) * 86_400_000);

    let mut sessions = Vec::new();
    let mut seen_digests = HashSet::new();
    let mut excluded_count = 0usize;
    let mut excluded_by_budget = 0usize;
    let mut excluded_by_filter = 0usize;
    let mut used_bytes = 0u64;
    let max_sessions = options.max_sessions.clamp(1, 500);

    for meta in all_metas {
        if let Some(cutoff) = min_ts_ms {
            let ts = meta
                .last_active_at
                .or(meta.created_at)
                .map(normalize_timestamp_ms);
            if let Some(ts_ms) = ts {
                if ts_ms < cutoff {
                    excluded_count += 1;
                    excluded_by_filter += 1;
                    continue;
                }
            }
        }
        if sessions.len() >= max_sessions {
            excluded_count += 1;
            excluded_by_budget += 1;
            continue;
        }
        let Some(source_path) = meta.source_path.as_deref() else {
            excluded_count += 1;
            continue;
        };

        // 1. Prefer verified strict native `MigratableSession` when in FullDialogue mode.
        let candidate = if options.mode == SessionProjectionMode::FullDialogue {
            match preview_session(&state.db, &meta.provider_id, source_path) {
                Ok(strict) if !strict.messages.is_empty() => {
                    let omitted = (strict.extraction.omitted.tool_events
                        + strict.extraction.omitted.reasoning_blocks
                        + strict.extraction.omitted.commentary_messages
                        + strict.extraction.omitted.attachments
                        + strict.extraction.omitted.runtime_injections
                        + strict.extraction.omitted.unknown_blocks)
                        as usize;
                    Some(ProjectedSessionEntry {
                        content_digest: strict.content_digest.clone(),
                        provider_id: meta.provider_id.clone(),
                        session_id: meta.session_id.clone(),
                        title: strict.title.clone().or_else(|| meta.title.clone()),
                        workspace_label: strict
                            .workspace_label
                            .clone()
                            .or_else(|| workspace_basename(meta.project_dir.as_deref())),
                        created_at: strict.created_at.or(meta.created_at),
                        last_active_at: strict.last_active_at.or(meta.last_active_at),
                        kind: ProjectedSessionKind::StrictNative,
                        omitted_tool_and_reasoning_blocks: omitted,
                        strict_session: Some(strict),
                        turns: Vec::new(),
                    })
                }
                _ => None,
            }
        } else {
            None
        };

        // 2. Fallback to structured text-only projection (covers Claude Code, Gemini, omp, compact mode).
        let entry = match candidate {
            Some(strict_entry) => strict_entry,
            None => {
                let Ok(messages) = session_manager::load_messages(&meta.provider_id, source_path)
                else {
                    excluded_count += 1;
                    continue;
                };
                let (raw_turns, omitted_blocks) = extract_portable_turns(&messages);
                if raw_turns.is_empty() {
                    excluded_count += 1;
                    continue;
                }
                let (turns, compacted) = match options.mode {
                    SessionProjectionMode::CompactContext => apply_compact_window(raw_turns),
                    SessionProjectionMode::FullDialogue => (raw_turns, false),
                };
                let digest = digest_turns(&turns);
                ProjectedSessionEntry {
                    content_digest: digest,
                    provider_id: meta.provider_id.clone(),
                    session_id: meta.session_id.clone(),
                    title: meta.title.clone(),
                    workspace_label: workspace_basename(meta.project_dir.as_deref()),
                    created_at: meta.created_at,
                    last_active_at: meta.last_active_at,
                    kind: if compacted {
                        ProjectedSessionKind::CompactContext
                    } else {
                        ProjectedSessionKind::PortableDialogue
                    },
                    omitted_tool_and_reasoning_blocks: omitted_blocks,
                    strict_session: None,
                    turns,
                }
            }
        };

        if !seen_digests.insert(entry.content_digest.clone()) {
            continue;
        }
        let est_bytes = serde_json::to_vec(&entry)
            .map(|buf| buf.len() as u64)
            .unwrap_or(0);
        if used_bytes.saturating_add(est_bytes) > options.uncompressed_budget_bytes {
            excluded_count += 1;
            excluded_by_budget += 1;
            continue;
        }
        used_bytes = used_bytes.saturating_add(est_bytes);
        sessions.push(entry);
    }

    let mut warnings = Vec::new();
    if excluded_by_filter > 0 {
        warnings.push(format!(
            "按时间筛选跳过了 {excluded_by_filter} 条更早的历史会话。"
        ));
    }
    if excluded_by_budget > 0 {
        warnings.push(format!(
            "为控制在 100 MiB 硬上限内，已优先保留最新会话并排除 {excluded_by_budget} 条较旧会话。"
        ));
    }

    CollectedSessions {
        sessions,
        excluded_count,
        warnings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_relative_skill_path_blocks_traversal() {
        assert_eq!(
            sanitize_relative_skill_path("nested\\SKILL.md").unwrap(),
            "nested/SKILL.md"
        );
        assert!(sanitize_relative_skill_path("../secret").is_err());
        assert!(sanitize_relative_skill_path("/etc/passwd").is_err());
    }

    #[test]
    fn portable_turns_strip_tool_thinking_and_base64_blobs() {
        let big_b64 = format!("see image data:image/png;base64,{}", "A".repeat(600));
        let msgs = vec![
            SessionMessage {
                role: "user".to_string(),
                injected: true,
                blocks: vec![SessionBlock::Text {
                    text: "injected AGENTS.md".to_string(),
                    full: None,
                }],
                ..Default::default()
            },
            SessionMessage {
                role: "user".to_string(),
                blocks: vec![SessionBlock::Text {
                    text: big_b64,
                    full: None,
                }],
                ..Default::default()
            },
            SessionMessage {
                role: "assistant".to_string(),
                blocks: vec![
                    SessionBlock::Thinking {
                        text: "chain of thought".to_string(),
                        summary: None,
                        redacted: false,
                        duration_ms: None,
                        full: None,
                    },
                    SessionBlock::Text {
                        text: "Final concise answer.".to_string(),
                        full: None,
                    },
                ],
                ..Default::default()
            },
        ];

        let (turns, omitted) = extract_portable_turns(&msgs);
        assert_eq!(turns.len(), 2);
        assert!(omitted >= 2);
        assert!(turns[0].text.contains("[omitted-inline-attachment]"));
        assert!(!turns[0].text.contains(&"A".repeat(200)));
        assert_eq!(turns[1].text, "Final concise answer.");
    }

    #[test]
    fn compact_window_keeps_opening_goal_and_tail_turns() {
        let turns: Vec<_> = (0..14)
            .map(|i| ProjectedDialogueTurn {
                role: if i % 2 == 0 {
                    "user".to_string()
                } else {
                    "assistant".to_string()
                },
                text: format!("turn-{i}"),
                ts: Some(i as i64),
            })
            .collect();
        let (compact, compacted) = apply_compact_window(turns);
        assert!(compacted);
        assert_eq!(compact.len(), 8);
        assert_eq!(compact[0].text, "turn-0");
        assert_eq!(compact[1].text, "turn-1");
        assert_eq!(compact.last().unwrap().text, "turn-13");
    }
}
