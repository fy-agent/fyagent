//! Wire schema & binary container framing for `.fyagent-pack` (`FYPK0001`).
//!
//! Container layout (little-endian lengths, strictly capped at 100 MiB):
//!   [0..8]                       ASCII magic `FYPK0001`
//!   [8..12]                      u32 manifest_len (<= 256 KiB)
//!   [12..12+manifest_len]        UTF-8 JSON `WorkspacePackManifest` (auditable)
//!   [..+4]                       u32 payload_zst_len
//!   [..+payload_zst_len]         Zstd-compressed UTF-8 JSON `WorkspacePackPayload`
//!   [..+4]                       u32 secrets_len (0 if no encrypted secrets)
//!   [..+secrets_len]             UTF-8 JSON `EncryptedSecretsEnvelope`

use std::io::Read;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::crypto::EncryptedSecretsEnvelope;
use crate::{
    app_config::{InstalledSkill, McpServer},
    prompt::Prompt,
    services::{
        provider::{batch_setup::BatchSetupTargetId, custom_templates::CustomGatewayTemplate},
        provider_api::ApiProtocol,
    },
    session_manager::migrate::model::MigratableSession,
};

pub const PACK_MAGIC: &[u8; 8] = b"FYPK0001";
pub const PACK_SCHEMA: &str = "fyagent-workspace-pack/v1";

/// Hard upper bound for the finished `.fyagent-pack` file (100 MiB).
pub const HARD_LIMIT_BYTES: u64 = 100 * 1024 * 1024;
/// Soft warning threshold shown in the budget meter (80 MiB).
pub const SOFT_WARNING_BYTES: u64 = 80 * 1024 * 1024;
/// Upper bound for the plaintext auditable manifest header (256 KiB).
pub const MAX_MANIFEST_BYTES: usize = 256 * 1024;
/// Upper bound for encrypted secrets envelope (1 MiB).
pub const MAX_SECRETS_BYTES: usize = 1024 * 1024;
/// Decompression bomb guard for zstd-inflated JSON payload (256 MiB).
pub const MAX_DECOMPRESSED_PAYLOAD_BYTES: usize = 256 * 1024 * 1024;
/// Maximum uncompressed bytes allocated to bundled text skills (15 MiB).
pub const SKILLS_UNCOMPRESSED_BUDGET_BYTES: u64 = 15 * 1024 * 1024;
/// Maximum size of one bundled skill text file (256 KiB).
pub const MAX_SKILL_FILE_BYTES: u64 = 256 * 1024;
/// Zstd compression level (good balance of speed and ~6x-8x JSON compression).
pub const ZSTD_COMPRESSION_LEVEL: i32 = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SessionProjectionMode {
    /// Keep all user prompts and final assistant answers (tool logs/attachments stripped).
    #[default]
    FullDialogue,
    /// Low-capacity hardware mode: opening goal turn + last 6 dialogue turns.
    CompactContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspacePackManifest {
    pub schema: String,
    pub pack_id: String,
    pub exported_at: i64,
    pub app_version: String,
    pub platform: String,
    pub has_encrypted_secrets: bool,
    pub session_projection_mode: SessionProjectionMode,
    pub payload_sha256: String,
    pub uncompressed_bytes: u64,
    pub compressed_bytes: u64,
    pub budget_limit_bytes: u64,
    pub counts: PackSectionCounts,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackSectionCounts {
    pub models: usize,
    pub custom_templates: usize,
    pub mcp_servers: usize,
    pub prompts: usize,
    pub skills: usize,
    pub sessions_included: usize,
    pub sessions_excluded: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectedModelConfig {
    pub target: BatchSetupTargetId,
    pub provider_id: String,
    pub provider_name: String,
    pub base_url: String,
    pub model_id: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extra_model_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protocol: Option<ApiProtocol>,
    pub has_secret_in_pack: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectedPromptEntry {
    pub app: String,
    pub prompt: Prompt,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectedSkillFile {
    /// Clean forward-slash relative path inside the skill directory.
    pub relative_path: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectedSkillEntry {
    pub metadata: InstalledSkill,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<ProjectedSkillFile>,
    pub omitted_large_files: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectedSessionKind {
    /// Strict lossless `MigratableSession` ready for native CLI restore.
    StrictNative,
    /// Structured text-only dialogue projection (userText + assistantFinal, tool logs stripped).
    PortableDialogue,
    /// Compact context projection (opening goal + trailing turns).
    CompactContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectedDialogueTurn {
    pub role: String,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ts: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectedSessionEntry {
    pub content_digest: String,
    pub provider_id: String,
    pub session_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_active_at: Option<i64>,
    pub kind: ProjectedSessionKind,
    pub omitted_tool_and_reasoning_blocks: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strict_session: Option<MigratableSession>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub turns: Vec<ProjectedDialogueTurn>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspacePackPayload {
    pub models: Vec<ProjectedModelConfig>,
    pub custom_templates: Vec<CustomGatewayTemplate>,
    pub mcp_servers: Vec<McpServer>,
    pub prompts: Vec<ProjectedPromptEntry>,
    pub skills: Vec<ProjectedSkillEntry>,
    pub sessions: Vec<ProjectedSessionEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelSecretEntry {
    pub target: BatchSetupTargetId,
    pub api_key: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspacePackSecrets {
    pub models: Vec<ModelSecretEntry>,
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

pub fn encode_pack_container(
    manifest: &mut WorkspacePackManifest,
    payload: &WorkspacePackPayload,
    secrets: Option<&EncryptedSecretsEnvelope>,
) -> Result<Vec<u8>, String> {
    let payload_json =
        serde_json::to_vec(payload).map_err(|e| format!("序列化迁移包内容失败: {e}"))?;
    let payload_zst = zstd::bulk::compress(&payload_json, ZSTD_COMPRESSION_LEVEL)
        .map_err(|e| format!("压缩迁移包数据失败: {e}"))?;
    let secrets_json = match secrets {
        Some(env) => serde_json::to_vec(env).map_err(|e| format!("序列化加密密钥块失败: {e}"))?,
        None => Vec::new(),
    };
    if secrets_json.len() > MAX_SECRETS_BYTES {
        return Err("加密密钥区超出大小上限".to_string());
    }

    manifest.payload_sha256 = sha256_hex(&payload_json);
    manifest.uncompressed_bytes = payload_json.len() as u64;
    manifest.has_encrypted_secrets = !secrets_json.is_empty();
    manifest.budget_limit_bytes = HARD_LIMIT_BYTES;

    // Two-pass manifest sizing so `compressed_bytes` records the exact final container length.
    manifest.compressed_bytes = 0;
    let first_manifest =
        serde_json::to_vec(manifest).map_err(|e| format!("序列化迁移包清单失败: {e}"))?;
    let approx_total =
        8 + 4 + first_manifest.len() + 4 + payload_zst.len() + 4 + secrets_json.len();
    manifest.compressed_bytes = approx_total as u64;
    let manifest_json =
        serde_json::to_vec(manifest).map_err(|e| format!("序列化迁移包清单失败: {e}"))?;
    let exact_total = 8 + 4 + manifest_json.len() + 4 + payload_zst.len() + 4 + secrets_json.len();
    manifest.compressed_bytes = exact_total as u64;
    let manifest_json =
        serde_json::to_vec(manifest).map_err(|e| format!("序列化迁移包清单失败: {e}"))?;

    if manifest_json.len() > MAX_MANIFEST_BYTES {
        return Err("迁移包清单头超出大小上限".to_string());
    }
    let final_total = 8 + 4 + manifest_json.len() + 4 + payload_zst.len() + 4 + secrets_json.len();
    if final_total as u64 > HARD_LIMIT_BYTES {
        return Err(format!(
            "迁移包压缩后体积为 {:.1} MiB，超出 100 MiB 硬上限，请改用精简会话投影或缩小时间范围",
            final_total as f64 / (1024.0 * 1024.0)
        ));
    }

    let mut out = Vec::with_capacity(final_total);
    out.extend_from_slice(PACK_MAGIC);
    out.extend_from_slice(&(manifest_json.len() as u32).to_le_bytes());
    out.extend_from_slice(&manifest_json);
    out.extend_from_slice(&(payload_zst.len() as u32).to_le_bytes());
    out.extend_from_slice(&payload_zst);
    out.extend_from_slice(&(secrets_json.len() as u32).to_le_bytes());
    out.extend_from_slice(&secrets_json);
    Ok(out)
}

pub struct DecodedPackContainer {
    pub manifest: WorkspacePackManifest,
    pub payload: WorkspacePackPayload,
    pub secrets: Option<EncryptedSecretsEnvelope>,
}

fn read_u32_le(bytes: &[u8], cursor: &mut usize) -> Result<usize, String> {
    if bytes.len().saturating_sub(*cursor) < 4 {
        return Err("迁移包文件不完整或已截断".to_string());
    }
    let arr: [u8; 4] = bytes[*cursor..*cursor + 4]
        .try_into()
        .map_err(|_| "迁移包长度字段损坏".to_string())?;
    *cursor += 4;
    Ok(u32::from_le_bytes(arr) as usize)
}

fn read_slice<'a>(bytes: &'a [u8], cursor: &mut usize, len: usize) -> Result<&'a [u8], String> {
    if bytes.len().saturating_sub(*cursor) < len {
        return Err("迁移包文件数据段不完整".to_string());
    }
    let slice = &bytes[*cursor..*cursor + len];
    *cursor += len;
    Ok(slice)
}

pub fn decode_pack_container(bytes: &[u8]) -> Result<DecodedPackContainer, String> {
    if bytes.len() as u64 > HARD_LIMIT_BYTES {
        return Err("迁移包文件超出 100 MiB 硬上限，拒绝加载".to_string());
    }
    if bytes.len() < PACK_MAGIC.len() + 12 || &bytes[0..8] != PACK_MAGIC {
        return Err("不是有效的 FyAgent 迁移包（.fyagent-pack）文件".to_string());
    }
    let mut cursor = 8usize;

    let manifest_len = read_u32_le(bytes, &mut cursor)?;
    if manifest_len == 0 || manifest_len > MAX_MANIFEST_BYTES {
        return Err("迁移包清单头长度无效".to_string());
    }
    let manifest_bytes = read_slice(bytes, &mut cursor, manifest_len)?;
    let manifest: WorkspacePackManifest =
        serde_json::from_slice(manifest_bytes).map_err(|e| format!("解析迁移包清单失败: {e}"))?;
    if manifest.schema != PACK_SCHEMA {
        return Err(format!("不支持的迁移包版本: {}", manifest.schema));
    }

    let payload_len = read_u32_le(bytes, &mut cursor)?;
    let payload_zst = read_slice(bytes, &mut cursor, payload_len)?;

    let secrets_len = read_u32_le(bytes, &mut cursor)?;
    if secrets_len > MAX_SECRETS_BYTES {
        return Err("迁移包加密密钥区超出大小限制".to_string());
    }
    let secrets_bytes = read_slice(bytes, &mut cursor, secrets_len)?;
    if cursor != bytes.len() {
        return Err("迁移包尾部存在多余未识别数据".to_string());
    }

    // Stream-decompress with hard cap to defend against decompression bombs.
    let decoder = zstd::stream::read::Decoder::new(payload_zst)
        .map_err(|e| format!("解压迁移包失败: {e}"))?;
    let mut payload_json = Vec::new();
    decoder
        .take((MAX_DECOMPRESSED_PAYLOAD_BYTES + 1) as u64)
        .read_to_end(&mut payload_json)
        .map_err(|e| format!("读取解压后的迁移包内容失败: {e}"))?;
    if payload_json.len() > MAX_DECOMPRESSED_PAYLOAD_BYTES {
        return Err("迁移包解压后体积超出安全上限，已中止导入".to_string());
    }

    let actual_sha = sha256_hex(&payload_json);
    if actual_sha != manifest.payload_sha256 {
        return Err("迁移包完整性校验失败（SHA-256 不匹配）".to_string());
    }

    let payload: WorkspacePackPayload =
        serde_json::from_slice(&payload_json).map_err(|e| format!("解析迁移包内容失败: {e}"))?;
    let secrets = if secrets_bytes.is_empty() {
        None
    } else {
        Some(
            serde_json::from_slice::<EncryptedSecretsEnvelope>(secrets_bytes)
                .map_err(|e| format!("解析迁移包加密密钥区失败: {e}"))?,
        )
    };

    Ok(DecodedPackContainer {
        manifest,
        payload,
        secrets,
    })
}
