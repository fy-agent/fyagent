//! Portable Workspace Migration Pack (`.fyagent-pack`, <= 100 MiB).
//!
//! Projection-first packaging of models, custom gateway templates, MCP servers,
//! global prompts, text-only skills, and lightweight dialogue sessions into a
//! single zstd-compressed, SHA-256 verified file with optional PBKDF2+AES-256-GCM
//! passphrase encryption for API keys.

pub mod crypto;
#[cfg(test)]
mod e2e_tests;
pub mod projection;
pub mod schema;

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use zeroize::Zeroizing;

use self::{
    crypto::{decrypt_secrets_json, encrypt_secrets_json},
    projection::{
        collect_custom_templates, collect_mcp_projections, collect_model_projections,
        collect_prompt_projections, collect_session_projections, collect_skill_projections,
        sanitize_relative_skill_path, split_mcp_secrets, SessionProjectionOptions,
        SESSION_UNCOMPRESSED_SOFT_CAP_BYTES,
    },
    schema::{
        decode_pack_container, encode_pack_container_from_json, serialize_payload_with_sha256,
        sha256_hex, McpSecretEntry, ModelSecretEntry, PackSectionCounts, ProjectedSessionKind,
        SessionProjectionMode, WorkspacePackManifest, WorkspacePackPayload, WorkspacePackSecrets,
        HARD_LIMIT_BYTES, PACK_SCHEMA, SOFT_WARNING_BYTES, ZSTD_COMPRESSION_LEVEL,
    },
};
use crate::{
    app_config::{AppType, SkillTargetId},
    config::{atomic_write, get_app_config_dir},
    services::{
        mcp::McpService,
        prompt::PromptService,
        provider::{
            batch_setup::{
                apply_batch_provider_setup, BatchProviderApplyRequest, BatchSetupTargetId,
                BatchSetupTargetSpec,
            },
            custom_templates::{save_custom_gateway_template, SaveCustomGatewayTemplateRequest},
        },
        provider_api::ApiProtocol,
        skill::SkillService,
    },
    session_manager::migrate::{
        model::{limits, MigratableSession},
        package::{build_package, write_package},
    },
    store::AppState,
};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspacePackExportOptions {
    #[serde(default = "default_true")]
    pub include_models: bool,
    #[serde(default = "default_true")]
    pub include_custom_templates: bool,
    #[serde(default = "default_true")]
    pub include_mcp: bool,
    #[serde(default = "default_true")]
    pub include_prompts: bool,
    #[serde(default = "default_true")]
    pub include_skills: bool,
    #[serde(default = "default_true")]
    pub include_sessions: bool,
    #[serde(default)]
    pub include_api_keys: bool,
    #[serde(default)]
    pub session_mode: SessionProjectionMode,
    #[serde(default)]
    pub session_max_age_days: Option<u32>,
    #[serde(default)]
    pub session_max_count: Option<usize>,
}

fn default_true() -> bool {
    true
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspacePackExportRequest {
    pub target_path: String,
    pub options: WorkspacePackExportOptions,
    #[serde(default)]
    pub passphrase: Option<String>,
}

impl fmt::Debug for WorkspacePackExportRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WorkspacePackExportRequest")
            .field("target_path", &self.target_path)
            .field("options", &self.options)
            .field("has_passphrase", &self.passphrase.is_some())
            .finish()
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspacePackBudgetBreakdown {
    pub models_bytes: u64,
    pub templates_bytes: u64,
    pub mcp_bytes: u64,
    pub prompts_bytes: u64,
    pub skills_bytes: u64,
    pub sessions_bytes: u64,
    pub total_uncompressed_bytes: u64,
    pub estimated_compressed_bytes: u64,
    pub soft_warning_bytes: u64,
    pub hard_limit_bytes: u64,
    pub within_hard_limit: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspacePackExportPreview {
    pub counts: PackSectionCounts,
    pub api_key_count: usize,
    pub breakdown: WorkspacePackBudgetBreakdown,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspacePackExportOutcome {
    pub path: String,
    pub pack_id: String,
    pub compressed_bytes: u64,
    pub uncompressed_bytes: u64,
    pub file_sha256: String,
    pub has_encrypted_secrets: bool,
    pub counts: PackSectionCounts,
    pub warnings: Vec<String>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspacePackInspectRequest {
    pub path: String,
    #[serde(default)]
    pub passphrase: Option<String>,
}

impl fmt::Debug for WorkspacePackInspectRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WorkspacePackInspectRequest")
            .field("path", &self.path)
            .field("has_passphrase", &self.passphrase.is_some())
            .finish()
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspacePackImportPreview {
    pub path: String,
    pub manifest: WorkspacePackManifest,
    pub secrets_locked: bool,
    pub decrypted_api_key_targets: Vec<BatchSetupTargetId>,
    pub model_targets: Vec<BatchSetupTargetId>,
    pub strict_session_count: usize,
    pub portable_session_count: usize,
    pub warnings: Vec<String>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspacePackImportRequest {
    pub path: String,
    #[serde(default)]
    pub passphrase: Option<String>,
    #[serde(default = "default_true")]
    pub import_models: bool,
    #[serde(default = "default_true")]
    pub import_custom_templates: bool,
    #[serde(default = "default_true")]
    pub import_mcp: bool,
    #[serde(default = "default_true")]
    pub import_prompts: bool,
    #[serde(default = "default_true")]
    pub import_skills: bool,
    #[serde(default = "default_true")]
    pub import_sessions: bool,
}

impl fmt::Debug for WorkspacePackImportRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WorkspacePackImportRequest")
            .field("path", &self.path)
            .field("has_passphrase", &self.passphrase.is_some())
            .finish()
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspacePackImportOutcome {
    pub models_applied: usize,
    pub models_skipped: usize,
    pub templates_imported: usize,
    pub mcp_imported: usize,
    pub prompts_imported: usize,
    pub skills_imported: usize,
    pub sessions_archived: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_archive_dir: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub native_session_package_path: Option<String>,
    pub warnings: Vec<String>,
}

fn current_platform() -> String {
    crate::session_manager::migrate::package::exporter_info().platform
}

fn json_len<T: Serialize>(value: &T) -> u64 {
    serde_json::to_vec(value)
        .map(|b| b.len() as u64)
        .unwrap_or(0)
}

struct BuiltPack {
    payload: WorkspacePackPayload,
    secrets: WorkspacePackSecrets,
    counts: PackSectionCounts,
    warnings: Vec<String>,
}

fn build_pack_payload(state: &AppState, options: &WorkspacePackExportOptions) -> BuiltPack {
    let mut payload = WorkspacePackPayload::default();
    let mut secrets = WorkspacePackSecrets::default();
    let mut warnings = Vec::new();
    let mut sessions_excluded = 0usize;

    if options.include_models {
        let collected = collect_model_projections(state, options.include_api_keys);
        payload.models = collected.models;
        secrets.models = collected
            .secrets
            .into_iter()
            .map(|(target, key)| ModelSecretEntry {
                target,
                api_key: key.as_str().to_string(),
            })
            .collect();
        warnings.extend(collected.warnings);
    }

    if options.include_custom_templates {
        let (templates, w) = collect_custom_templates(state);
        payload.custom_templates = templates;
        warnings.extend(w);
    }

    if options.include_mcp {
        let (servers, w) = collect_mcp_projections(state);
        warnings.extend(w);
        let (sanitized_servers, mcp_secrets, split_warnings) =
            split_mcp_secrets(servers, options.include_api_keys);
        payload.mcp_servers = sanitized_servers;
        secrets.mcp_secrets = mcp_secrets;
        warnings.extend(split_warnings);
    }

    if options.include_prompts {
        let (prompts, w) = collect_prompt_projections(state);
        payload.prompts = prompts;
        warnings.extend(w);
    }

    if options.include_skills {
        let collected = collect_skill_projections(state);
        payload.skills = collected.skills;
        warnings.extend(collected.warnings);
    }

    if options.include_sessions {
        let collected = collect_session_projections(
            state,
            &SessionProjectionOptions {
                mode: options.session_mode,
                max_age_days: options.session_max_age_days,
                max_sessions: options.session_max_count.unwrap_or(200),
                uncompressed_budget_bytes: SESSION_UNCOMPRESSED_SOFT_CAP_BYTES,
            },
        );
        payload.sessions = collected.sessions;
        sessions_excluded = collected.excluded_count;
        warnings.extend(collected.warnings);
    }

    let counts = PackSectionCounts {
        models: payload.models.len(),
        custom_templates: payload.custom_templates.len(),
        mcp_servers: payload.mcp_servers.len(),
        prompts: payload.prompts.len(),
        skills: payload.skills.len(),
        sessions_included: payload.sessions.len(),
        sessions_excluded,
    };

    BuiltPack {
        payload,
        secrets,
        counts,
        warnings,
    }
}

/// Deterministically trims oldest sessions if zstd-compressed payload would exceed
/// the 100 MiB hard ceiling. Never silently truncates inside a session.
fn enforce_hard_budget_by_pruning_oldest_sessions(
    payload: &mut WorkspacePackPayload,
    counts: &mut PackSectionCounts,
    warnings: &mut Vec<String>,
) -> Result<u64, String> {
    const HEADROOM_BYTES: u64 = 1_572_864;
    let target_compressed = HARD_LIMIT_BYTES.saturating_sub(HEADROOM_BYTES);
    let mut trimmed = 0usize;
    loop {
        let raw = serde_json::to_vec(payload).map_err(|e| format!("估算体积失败: {e}"))?;
        let compressed = zstd::bulk::compress(&raw, ZSTD_COMPRESSION_LEVEL)
            .map_err(|e| format!("估算压缩体积失败: {e}"))?;
        if (compressed.len() as u64) <= target_compressed {
            if trimmed > 0 {
                counts.sessions_included = payload.sessions.len();
                counts.sessions_excluded += trimmed;
                warnings.push(format!(
                    "为严格满足 100 MiB 硬上限，已自动裁剪最旧的 {trimmed} 条历史会话投影。"
                ));
            }
            return Ok(compressed.len() as u64);
        }
        if payload.sessions.is_empty() {
            return Err(
                "非会话资产压缩后已超出 100 MiB 硬上限，请精简 Skills 或 Prompts 后重试"
                    .to_string(),
            );
        }
        // Proportional jump converges in 1-2 compression passes even for huge histories.
        let ratio = (target_compressed as f64 / compressed.len().max(1) as f64).clamp(0.1, 0.9);
        let proportional_keep = ((payload.sessions.len() as f64) * ratio * 0.90).floor() as usize;
        let new_len = proportional_keep.min(payload.sessions.len().saturating_sub(1));
        trimmed += payload.sessions.len() - new_len;
        payload.sessions.truncate(new_len);
    }
}

pub fn preview_workspace_pack_export(
    state: &AppState,
    options: WorkspacePackExportOptions,
) -> Result<WorkspacePackExportPreview, String> {
    let mut built = build_pack_payload(state, &options);
    let est_compressed = enforce_hard_budget_by_pruning_oldest_sessions(
        &mut built.payload,
        &mut built.counts,
        &mut built.warnings,
    )?;

    let models_bytes = json_len(&built.payload.models);
    let templates_bytes = json_len(&built.payload.custom_templates);
    let mcp_bytes = json_len(&built.payload.mcp_servers);
    let prompts_bytes = json_len(&built.payload.prompts);
    let skills_bytes = json_len(&built.payload.skills);
    let sessions_bytes = json_len(&built.payload.sessions);
    let total_uncompressed_bytes =
        models_bytes + templates_bytes + mcp_bytes + prompts_bytes + skills_bytes + sessions_bytes;

    Ok(WorkspacePackExportPreview {
        counts: built.counts,
        api_key_count: built.secrets.models.len(),
        breakdown: WorkspacePackBudgetBreakdown {
            models_bytes,
            templates_bytes,
            mcp_bytes,
            prompts_bytes,
            skills_bytes,
            sessions_bytes,
            total_uncompressed_bytes,
            estimated_compressed_bytes: est_compressed + 1024,
            soft_warning_bytes: SOFT_WARNING_BYTES,
            hard_limit_bytes: HARD_LIMIT_BYTES,
            within_hard_limit: est_compressed + 1024 <= HARD_LIMIT_BYTES,
        },
        warnings: built.warnings,
    })
}

fn read_pack_file_bounded(raw_path: &str) -> Result<Vec<u8>, String> {
    let file = fs::File::open(raw_path).map_err(|e| format!("无法读取迁移包文件: {e}"))?;
    let mut bytes = Vec::new();
    file.take(HARD_LIMIT_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("读取迁移包失败: {e}"))?;
    if bytes.len() as u64 > HARD_LIMIT_BYTES {
        return Err("迁移包文件超出 100 MiB 硬上限，拒绝读取".to_string());
    }
    Ok(bytes)
}

/// Atomic file write for skill files without leaving `.fyagent.undo.json` or
/// `.fyagent.backup` sidecar files inside skill directories.
fn write_skill_file_clean(dest: &Path, data: &[u8]) -> Result<(), String> {
    let parent = dest
        .parent()
        .ok_or_else(|| "技能目标路径缺少父目录".to_string())?;
    fs::create_dir_all(parent).map_err(|e| format!("创建技能目录失败: {e}"))?;
    let tmp_name = format!(
        ".{}.fyagent-tmp-{}",
        dest.file_name().and_then(|n| n.to_str()).unwrap_or("skill"),
        Uuid::new_v4().simple()
    );
    let tmp_path = parent.join(tmp_name);
    fs::write(&tmp_path, data).map_err(|e| format!("写入技能临时文件失败: {e}"))?;
    if let Err(err) = fs::rename(&tmp_path, dest) {
        let _ = fs::remove_file(&tmp_path);
        return Err(format!("替换技能文件失败: {err}"));
    }
    Ok(())
}

pub fn export_workspace_pack(
    state: &AppState,
    request: WorkspacePackExportRequest,
) -> Result<WorkspacePackExportOutcome, String> {
    let target_path = request.target_path.trim();
    if target_path.is_empty() {
        return Err("请先选择迁移包保存路径".to_string());
    }

    let mut built = build_pack_payload(state, &request.options);
    enforce_hard_budget_by_pruning_oldest_sessions(
        &mut built.payload,
        &mut built.counts,
        &mut built.warnings,
    )?;

    let (payload_json, payload_sha256) = serialize_payload_with_sha256(&built.payload)?;

    let encrypted_secrets = if request.options.include_api_keys && !built.secrets.is_empty() {
        let passphrase = request
            .passphrase
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                "包含 API Key 或 MCP 凭证导出时必须设置加密口令（至少 6 位）".to_string()
            })?;
        let plaintext = Zeroizing::new(
            serde_json::to_vec(&built.secrets)
                .map_err(|e| format!("序列化加密凭证数据失败: {e}"))?,
        );
        Some(encrypt_secrets_json(
            &plaintext,
            passphrase,
            &payload_sha256,
        )?)
    } else {
        None
    };

    let pack_id = format!("fypk-{}", Uuid::new_v4().simple());
    let mut manifest = WorkspacePackManifest {
        schema: PACK_SCHEMA.to_string(),
        pack_id: pack_id.clone(),
        exported_at: chrono::Utc::now().timestamp(),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        platform: current_platform(),
        has_encrypted_secrets: encrypted_secrets.is_some(),
        session_projection_mode: request.options.session_mode,
        payload_sha256: String::new(),
        uncompressed_bytes: 0,
        compressed_bytes: 0,
        budget_limit_bytes: HARD_LIMIT_BYTES,
        counts: built.counts.clone(),
        warnings: built.warnings.clone(),
    };

    let bytes = encode_pack_container_from_json(
        &mut manifest,
        &payload_json,
        &payload_sha256,
        encrypted_secrets.as_ref(),
    )?;
    let file_sha256 = sha256_hex(&bytes);
    atomic_write(Path::new(target_path), &bytes).map_err(|e| format!("写入迁移包文件失败: {e}"))?;

    Ok(WorkspacePackExportOutcome {
        path: target_path.to_string(),
        pack_id,
        compressed_bytes: bytes.len() as u64,
        uncompressed_bytes: manifest.uncompressed_bytes,
        file_sha256,
        has_encrypted_secrets: manifest.has_encrypted_secrets,
        counts: manifest.counts,
        warnings: manifest.warnings,
    })
}

pub fn inspect_workspace_pack(
    request: WorkspacePackInspectRequest,
) -> Result<WorkspacePackImportPreview, String> {
    let raw_path = request.path.trim();
    if raw_path.is_empty() {
        return Err("请选择要导入的 .fyagent-pack 迁移包文件".to_string());
    }
    let bytes = read_pack_file_bounded(raw_path)?;
    let decoded = decode_pack_container(&bytes)?;

    let mut warnings = decoded.manifest.warnings.clone();
    let mut secrets_locked = decoded.secrets.is_some();
    let mut decrypted_api_key_targets = Vec::new();

    if let Some(envelope) = decoded.secrets.as_ref() {
        if let Some(pass) = request.passphrase.as_deref().filter(|s| !s.is_empty()) {
            let plain = decrypt_secrets_json(envelope, pass, &decoded.manifest.payload_sha256)?;
            let parsed: WorkspacePackSecrets =
                serde_json::from_slice(&plain).map_err(|e| format!("解析密钥区失败: {e}"))?;
            decrypted_api_key_targets = parsed.models.iter().map(|m| m.target).collect();
            secrets_locked = false;
        }
    }

    if secrets_locked {
        warnings.push("该迁移包包含加密的 API Key 与 MCP 凭证，输入口令后可一并导入；不输口令则自动转为自定义网关模板。".to_string());
    }

    let model_targets = decoded.payload.models.iter().map(|m| m.target).collect();
    let strict_session_count = decoded
        .payload
        .sessions
        .iter()
        .filter(|s| s.kind == ProjectedSessionKind::StrictNative && s.strict_session.is_some())
        .count();
    let portable_session_count = decoded.payload.sessions.len() - strict_session_count;

    Ok(WorkspacePackImportPreview {
        path: raw_path.to_string(),
        manifest: decoded.manifest,
        secrets_locked,
        decrypted_api_key_targets,
        model_targets,
        strict_session_count,
        portable_session_count,
        warnings,
    })
}

fn sanitize_single_segment(raw: &str, fallback: &str) -> String {
    let cleaned: String = raw
        .trim()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '-'
            }
        })
        .collect();
    let trimmed = cleaned.trim_matches(['-', '.']);
    if trimmed.is_empty() || trimmed.contains("..") {
        fallback.to_string()
    } else {
        trimmed.chars().take(64).collect()
    }
}

fn write_imported_sessions_archive(
    pack_id: &str,
    sessions: &[schema::ProjectedSessionEntry],
) -> Result<(PathBuf, Option<PathBuf>, Option<String>), String> {
    let safe_pack = sanitize_single_segment(pack_id, "imported-pack");
    let archive_dir = get_app_config_dir()
        .join("imported-sessions")
        .join(&safe_pack);
    fs::create_dir_all(&archive_dir).map_err(|e| format!("创建会话档案目录失败: {e}"))?;

    let index_json =
        serde_json::to_vec_pretty(sessions).map_err(|e| format!("序列化会话档案索引失败: {e}"))?;
    atomic_write(&archive_dir.join("sessions-projection.json"), &index_json)
        .map_err(|e| format!("写入会话投影索引失败: {e}"))?;

    // Render human-readable Markdown summary for quick browsing on the new machine.
    let mut md = String::from("# FyAgent 迁移会话投影索引\n\n");
    let mut strict_sessions: Vec<MigratableSession> = Vec::new();
    for (idx, session) in sessions.iter().enumerate() {
        let title = session.title.as_deref().unwrap_or("未命名会话");
        md.push_str(&format!(
            "## {}. [{}] {}\n",
            idx + 1,
            session.provider_id,
            title
        ));
        if let Some(ws) = session.workspace_label.as_deref() {
            md.push_str(&format!("- 工作区：`{ws}`\n"));
        }
        if let Some(strict) = session.strict_session.as_ref() {
            strict_sessions.push(strict.clone());
            md.push_str(&format!(
                "- 类型：原生无损投影（{} 条消息，可直接还原）\n\n",
                strict.messages.len()
            ));
        } else {
            md.push_str(&format!(
                "- 类型：轻量对话投影（{} 轮，已剔除 {} 个工具/推理块）\n\n",
                session.turns.len(),
                session.omitted_tool_and_reasoning_blocks
            ));
            for turn in session.turns.iter().take(20) {
                md.push_str(&format!("**{}**: {}\n\n", turn.role, turn.text));
            }
        }
    }
    atomic_write(&archive_dir.join("README.md"), md.as_bytes())
        .map_err(|e| format!("写入会话可读摘要失败: {e}"))?;

    let mut truncation_warning = None;
    let native_pkg_path = if !strict_sessions.is_empty() {
        let max_native = limits::PACKAGE_SESSIONS as usize;
        let total_strict = strict_sessions.len();
        if total_strict > max_native {
            strict_sessions.truncate(max_native);
            truncation_warning = Some(format!(
                "原生会话还原包上限为 {} 条，已优先收录最新 {} 条（其余 {} 条保留在 sessions-projection.json 中）。",
                max_native,
                max_native,
                total_strict - max_native
            ));
        }
        let pkg_path = archive_dir.join("native-restore.fyagent-session.json");
        let pkg = build_package(strict_sessions, chrono::Utc::now().timestamp_millis());
        write_package(&pkg, &pkg_path.to_string_lossy())
            .map_err(|e| format!("生成原生会话还原包失败: {e}"))?;
        Some(pkg_path)
    } else {
        None
    };

    Ok((archive_dir, native_pkg_path, truncation_warning))
}

const ALL_SKILL_SYNC_TARGETS: [SkillTargetId; 9] = [
    SkillTargetId::Claude,
    SkillTargetId::Codex,
    SkillTargetId::Gemini,
    SkillTargetId::GrokBuild,
    SkillTargetId::OpenCode,
    SkillTargetId::Hermes,
    SkillTargetId::QoderWork,
    SkillTargetId::TraeWork,
    SkillTargetId::WorkBuddy,
];

fn ensure_mcp_target_dirs_for_fresh_pc(servers: &[crate::app_config::McpServer]) {
    let any_claude = servers.iter().any(|s| s.apps.claude);
    let any_codex = servers.iter().any(|s| s.apps.codex);
    let any_opencode = servers.iter().any(|s| s.apps.opencode);
    let any_gemini = servers.iter().any(|s| s.apps.gemini);

    if any_claude {
        let _ = fs::create_dir_all(crate::config::get_claude_config_dir());
    }
    if any_codex {
        let _ = fs::create_dir_all(crate::codex_config::get_codex_config_dir());
    }
    if any_opencode {
        let _ = fs::create_dir_all(crate::opencode_config::get_opencode_dir());
    }
    if any_gemini {
        let _ = fs::create_dir_all(crate::gemini_config::get_gemini_dir());
    }
}

pub fn apply_workspace_pack_import(
    state: &AppState,
    request: WorkspacePackImportRequest,
) -> Result<WorkspacePackImportOutcome, String> {
    let raw_path = request.path.trim();
    if raw_path.is_empty() {
        return Err("请选择迁移包文件".to_string());
    }
    let bytes = read_pack_file_bounded(raw_path)?;
    let decoded = decode_pack_container(&bytes)?;

    let mut warnings = Vec::new();
    let mut secret_map: HashMap<BatchSetupTargetId, Zeroizing<String>> = HashMap::new();
    let mut mcp_secret_map: HashMap<String, McpSecretEntry> = HashMap::new();
    if let Some(envelope) = decoded.secrets.as_ref() {
        if let Some(pass) = request.passphrase.as_deref().filter(|s| !s.is_empty()) {
            let plain = decrypt_secrets_json(envelope, pass, &decoded.manifest.payload_sha256)?;
            let parsed: WorkspacePackSecrets =
                serde_json::from_slice(&plain).map_err(|e| format!("解析密钥区失败: {e}"))?;
            for entry in &parsed.models {
                secret_map.insert(entry.target, Zeroizing::new(entry.api_key.clone()));
            }
            for entry in &parsed.mcp_secrets {
                mcp_secret_map.insert(entry.server_id.clone(), entry.clone());
            }
        } else if (request.import_models
            && decoded.payload.models.iter().any(|m| m.has_secret_in_pack))
            || request.import_mcp
        {
            warnings.push(
                "未提供解密口令：模型端点已自动保存为“我的模板”（补充 API Key 即可一键应用），MCP 的 env/headers 敏感字段未解密还原。"
                    .to_string(),
            );
        }
    }

    let mut models_applied = 0usize;
    let mut models_skipped = 0usize;
    let mut templates_imported = 0usize;
    if request.import_models {
        for model in &decoded.payload.models {
            let Some(api_key) = secret_map.get(&model.target) else {
                models_skipped += 1;
                // Preserve keyless model endpoints as reusable custom gateway templates so
                // users on a new PC only have to paste their API key in 1 click.
                let is_claude = model.target == BatchSetupTargetId::Claude;
                let slug: String = sanitize_single_segment(&model.provider_id, "gw")
                    .chars()
                    .map(|c| {
                        if c.is_ascii_alphanumeric() || matches!(c, '-' | '_') {
                            c
                        } else {
                            '-'
                        }
                    })
                    .take(36)
                    .collect();
                let fallback_tpl_id = format!("custom-import-{}-{}", model.target.as_str(), slug);
                let save_res = save_custom_gateway_template(
                    &state.db,
                    SaveCustomGatewayTemplateRequest {
                        id: Some(fallback_tpl_id),
                        name: format!("{} ({})", model.provider_name, model.target.as_str()),
                        summary: "从无密钥迁移包自动保存，补充 API Key 即可一键应用".to_string(),
                        base_url: if is_claude {
                            crate::services::provider::model_overview::adapt_base_url_for_target(
                                BatchSetupTargetId::Claude,
                                &model.base_url,
                                BatchSetupTargetId::Codex,
                            )
                            .0
                        } else {
                            model.base_url.clone()
                        },
                        anthropic_base_url: if is_claude {
                            Some(model.base_url.clone())
                        } else {
                            None
                        },
                        default_model_id: model.model_id.clone(),
                        codex_protocol: Some(match model.protocol {
                            Some(ApiProtocol::Chat) => ApiProtocol::Chat,
                            _ => ApiProtocol::Responses,
                        }),
                        api_key_url: None,
                    },
                );
                if save_res.is_ok() {
                    templates_imported += 1;
                    warnings.push(format!(
                        "{} 未携带 API Key，已自动保存为自定义模板「{}」，补充密钥后可一键写入。",
                        model.target.as_str(),
                        model.provider_name
                    ));
                } else {
                    warnings.push(format!(
                        "{} 未携带可用 API Key，已跳过自动写入。",
                        model.target.as_str()
                    ));
                }
                continue;
            };
            let res = apply_batch_provider_setup(
                state,
                BatchProviderApplyRequest {
                    api_key: api_key.as_str().to_string(),
                    targets: vec![BatchSetupTargetSpec {
                        target: model.target,
                        provider_id: model.provider_id.clone(),
                        provider_name: model.provider_name.clone(),
                        base_url: model.base_url.clone(),
                        model_id: model.model_id.clone(),
                        extra_model_ids: model.extra_model_ids.clone(),
                        protocol: model.protocol,
                    }],
                },
            );
            match res {
                Ok(apply_res)
                    if apply_res
                        .outcomes
                        .first()
                        .is_some_and(|o| o.status == crate::services::provider::batch_setup::BatchTargetStatus::Applied) =>
                {
                    models_applied += 1;
                }
                Ok(apply_res) => {
                    models_skipped += 1;
                    let msg = apply_res
                        .outcomes
                        .first()
                        .map(|o| o.message.as_str())
                        .unwrap_or("写入失败");
                    warnings.push(format!("{} 模型导入失败：{msg}", model.target.as_str()));
                }
                Err(err) => {
                    models_skipped += 1;
                    warnings.push(format!("{} 模型导入失败：{err}", model.target.as_str()));
                }
            }
        }
    }

    if request.import_custom_templates {
        for tpl in &decoded.payload.custom_templates {
            let save_req = SaveCustomGatewayTemplateRequest {
                id: Some(tpl.id.clone()),
                name: tpl.name.clone(),
                summary: tpl.summary.clone(),
                base_url: tpl.base_url.clone(),
                anthropic_base_url: tpl.anthropic_base_url.clone(),
                default_model_id: tpl.default_model_id.clone(),
                codex_protocol: Some(tpl.codex_protocol),
                api_key_url: tpl.api_key_url.clone(),
            };
            match save_custom_gateway_template(&state.db, save_req) {
                Ok(_) => templates_imported += 1,
                Err(err) => warnings.push(format!("自定义模板「{}」导入失败：{err}", tpl.name)),
            }
        }
    }

    let mut mcp_imported = 0usize;
    if request.import_mcp {
        ensure_mcp_target_dirs_for_fresh_pc(&decoded.payload.mcp_servers);
        for server in &decoded.payload.mcp_servers {
            let mut hydrated = server.clone();
            if let Some(sec) = mcp_secret_map.get(&hydrated.id) {
                if let Some(obj) = hydrated.server.as_object_mut() {
                    if !sec.env.is_empty() {
                        obj.insert(
                            "env".to_string(),
                            serde_json::Value::Object(sec.env.clone()),
                        );
                    }
                    if !sec.headers.is_empty() {
                        obj.insert(
                            "headers".to_string(),
                            serde_json::Value::Object(sec.headers.clone()),
                        );
                    }
                    if !sec.http_headers.is_empty() {
                        obj.insert(
                            "http_headers".to_string(),
                            serde_json::Value::Object(sec.http_headers.clone()),
                        );
                    }
                }
            }
            match McpService::upsert_server(state, hydrated) {
                Ok(()) => mcp_imported += 1,
                Err(err) => warnings.push(format!("MCP「{}」导入失败：{err}", server.name)),
            }
        }
    }

    let mut prompts_imported = 0usize;
    if request.import_prompts {
        for entry in &decoded.payload.prompts {
            let Ok(app) = <AppType as std::str::FromStr>::from_str(&entry.app) else {
                continue;
            };
            match PromptService::upsert_prompt(state, app, &entry.prompt.id, entry.prompt.clone()) {
                Ok(()) => prompts_imported += 1,
                Err(err) => {
                    warnings.push(format!("提示词「{}」导入失败：{err}", entry.prompt.name))
                }
            }
        }
    }

    let mut skills_imported = 0usize;
    if request.import_skills {
        if let Ok(ssot_dir) = SkillService::get_ssot_dir() {
            let mut used_dirs: HashSet<String> = HashSet::new();
            let mut targets_to_sync: Vec<SkillTargetId> = Vec::new();
            for skill in &decoded.payload.skills {
                if skill.files.is_empty() {
                    warnings.push(format!(
                        "跳过空文件 Skill「{}」，避免生成无效幽灵条目。",
                        skill.metadata.name
                    ));
                    continue;
                }
                let base_dir = sanitize_single_segment(&skill.metadata.directory, "imported-skill");
                let mut dir_name = base_dir.clone();
                let mut suffix = 2usize;
                while !used_dirs.insert(dir_name.clone()) {
                    dir_name = format!("{base_dir}-{suffix}");
                    suffix += 1;
                }
                let target_dir = ssot_dir.join(&dir_name);
                if target_dir.exists() {
                    let _ = fs::remove_dir_all(&target_dir);
                }
                let mut write_ok = true;
                for file in &skill.files {
                    match sanitize_relative_skill_path(&file.relative_path) {
                        Ok(clean_rel) => {
                            let dest = target_dir.join(clean_rel);
                            if let Err(err) = write_skill_file_clean(&dest, file.content.as_bytes())
                            {
                                warnings.push(format!(
                                    "写入 Skill「{}」文件失败：{err}",
                                    skill.metadata.name
                                ));
                                write_ok = false;
                                break;
                            }
                        }
                        Err(err) => {
                            warnings.push(format!(
                                "跳过 Skill「{}」中的非法路径：{err}",
                                skill.metadata.name
                            ));
                            write_ok = false;
                            break;
                        }
                    }
                }
                if write_ok {
                    let mut meta = skill.metadata.clone();
                    meta.directory = dir_name;
                    if let Err(err) = state.db.save_skill(&meta) {
                        warnings.push(format!("注册 Skill「{}」失败：{err}", meta.name));
                    } else {
                        skills_imported += 1;
                        for target in ALL_SKILL_SYNC_TARGETS {
                            if meta.apps.is_enabled_for_target(&target)
                                && !targets_to_sync.contains(&target)
                            {
                                targets_to_sync.push(target);
                            }
                        }
                    }
                }
            }
            for target in targets_to_sync {
                if let Err(err) = SkillService::sync_to_target(&state.db, &target) {
                    warnings.push(format!(
                        "同步 Skills 到 {} 目录失败：{err}",
                        target.as_str()
                    ));
                }
            }
        }
    }

    let mut sessions_archived = 0usize;
    let mut session_archive_dir = None;
    let mut native_session_package_path = None;
    if request.import_sessions && !decoded.payload.sessions.is_empty() {
        let (archive_dir, native_pkg, trunc_warn) =
            write_imported_sessions_archive(&decoded.manifest.pack_id, &decoded.payload.sessions)?;
        if let Some(w) = trunc_warn {
            warnings.push(w);
        }
        sessions_archived = decoded.payload.sessions.len();
        session_archive_dir = Some(archive_dir.to_string_lossy().to_string());
        native_session_package_path = native_pkg.map(|p| p.to_string_lossy().to_string());
    }

    Ok(WorkspacePackImportOutcome {
        models_applied,
        models_skipped,
        templates_imported,
        mcp_imported,
        prompts_imported,
        skills_imported,
        sessions_archived,
        session_archive_dir,
        native_session_package_path,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::workspace_pack::schema::{ProjectedDialogueTurn, ProjectedSessionEntry};

    #[test]
    fn container_roundtrips_with_zstd_sha256_and_encrypted_secrets() {
        let payload = WorkspacePackPayload {
            sessions: vec![ProjectedSessionEntry {
                content_digest: sha256_hex(b"hello"),
                provider_id: "claude".to_string(),
                session_id: "s-1".to_string(),
                title: Some("Refactor auth".to_string()),
                workspace_label: Some("fyagent".to_string()),
                created_at: Some(1763500000),
                last_active_at: Some(1763500100),
                kind: ProjectedSessionKind::PortableDialogue,
                omitted_tool_and_reasoning_blocks: 42,
                strict_session: None,
                turns: vec![
                    ProjectedDialogueTurn {
                        role: "user".to_string(),
                        text: "Help me refactor auth".to_string(),
                        ts: Some(1763500000),
                    },
                    ProjectedDialogueTurn {
                        role: "assistant".to_string(),
                        text: "Done! Extracted auth service.".to_string(),
                        ts: Some(1763500100),
                    },
                ],
            }],
            ..Default::default()
        };

        let (payload_json, payload_sha256) = serialize_payload_with_sha256(&payload).unwrap();
        let secrets_json = serde_json::to_vec(&WorkspacePackSecrets {
            models: vec![ModelSecretEntry {
                target: BatchSetupTargetId::Codex,
                api_key: "sk-portable-secret".to_string(),
            }],
            mcp_secrets: Vec::new(),
        })
        .unwrap();
        let envelope =
            encrypt_secrets_json(&secrets_json, "pack-pass-2026", &payload_sha256).unwrap();

        let mut manifest = WorkspacePackManifest {
            schema: PACK_SCHEMA.to_string(),
            pack_id: "fypk-test".to_string(),
            exported_at: 1763550000,
            app_version: "0.4.16".to_string(),
            platform: "macos".to_string(),
            has_encrypted_secrets: true,
            session_projection_mode: SessionProjectionMode::FullDialogue,
            payload_sha256: String::new(),
            uncompressed_bytes: 0,
            compressed_bytes: 0,
            budget_limit_bytes: HARD_LIMIT_BYTES,
            counts: PackSectionCounts {
                sessions_included: 1,
                ..Default::default()
            },
            warnings: Vec::new(),
        };

        let encoded = encode_pack_container_from_json(
            &mut manifest,
            &payload_json,
            &payload_sha256,
            Some(&envelope),
        )
        .unwrap();
        assert!(encoded.len() as u64 <= HARD_LIMIT_BYTES);
        assert!(!String::from_utf8_lossy(&encoded).contains("sk-portable-secret"));

        let decoded = decode_pack_container(&encoded).unwrap();
        assert_eq!(decoded.manifest.pack_id, "fypk-test");
        assert_eq!(decoded.payload.sessions.len(), 1);
        assert!(decoded.secrets.is_some());

        let decrypted = decrypt_secrets_json(
            decoded.secrets.as_ref().unwrap(),
            "pack-pass-2026",
            &decoded.manifest.payload_sha256,
        )
        .unwrap();
        let parsed: WorkspacePackSecrets = serde_json::from_slice(&decrypted).unwrap();
        assert_eq!(parsed.models[0].api_key, "sk-portable-secret");

        // Tampering with payload bytes must fail SHA-256 verification.
        let mut tampered = encoded.clone();
        let last_payload_idx = tampered.len() / 2;
        tampered[last_payload_idx] ^= 0x55;
        assert!(decode_pack_container(&tampered).is_err());

        // Re-signing a modified payload with a new SHA-256 MUST fail AAD decryption!
        let mut altered_payload = payload.clone();
        altered_payload.sessions[0].title = Some("Attacker Tampered".to_string());
        let (altered_json, altered_sha) = serialize_payload_with_sha256(&altered_payload).unwrap();
        let mut altered_manifest = manifest.clone();
        let re_signed = encode_pack_container_from_json(
            &mut altered_manifest,
            &altered_json,
            &altered_sha,
            Some(&envelope),
        )
        .unwrap();
        let decoded_altered = decode_pack_container(&re_signed).unwrap();
        assert!(decrypt_secrets_json(
            decoded_altered.secrets.as_ref().unwrap(),
            "pack-pass-2026",
            &decoded_altered.manifest.payload_sha256,
        )
        .is_err());
    }
}
