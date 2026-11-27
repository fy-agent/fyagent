//! Saved non-secret SSH push profiles (`常用 SSH 主机`).
//!
//! Stores host aliases, optional user/port, private key file path references
//! (never key contents or passwords), and optional remote path mappings inside
//! SQLite `settings` so users can reuse frequent remote servers in one click.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::database::Database;

use super::{
    render::{validate_path_mapping, RemotePathMapping},
    ssh_config::{validate_ssh_target, SshTargetSpec},
};

const SSH_PUSH_PROFILES_KEY: &str = "ssh_push_profiles_v1";
const MAX_SSH_PUSH_PROFILES: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SshPushProfile {
    pub id: String,
    pub name: String,
    pub host: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identity_file: Option<String>,
    #[serde(default)]
    pub path_mapping: RemotePathMapping,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SaveSshPushProfileRequest {
    #[serde(default)]
    pub id: Option<String>,
    pub name: String,
    pub host: String,
    #[serde(default)]
    pub user: Option<String>,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub identity_file: Option<String>,
    #[serde(default)]
    pub path_mapping: Option<RemotePathMapping>,
}

fn contains_secret_pattern(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.contains("sk-")
        || lower.contains("sk_")
        || lower.contains("bearer ")
        || lower.contains("-----begin")
        || lower.contains("password=")
        || lower.contains("api_key=")
}

fn validate_profile_id(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    if !trimmed.starts_with("ssh-profile-")
        || trimmed.len() <= "ssh-profile-".len()
        || trimmed.len() > 64
        || !trimmed
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
    {
        return Err("SSH 主机档案 ID 格式无效".to_string());
    }
    Ok(trimmed.to_string())
}

pub fn list_ssh_push_profiles(db: &Database) -> Result<Vec<SshPushProfile>, String> {
    let Some(raw) = db
        .get_setting(SSH_PUSH_PROFILES_KEY)
        .map_err(|e| format!("读取常用 SSH 主机失败: {e}"))?
    else {
        return Ok(Vec::new());
    };
    let mut items: Vec<SshPushProfile> =
        serde_json::from_str(&raw).map_err(|e| format!("解析常用 SSH 主机失败: {e}"))?;
    items.sort_by(|a, b| {
        b.updated_at
            .cmp(&a.updated_at)
            .then_with(|| a.name.cmp(&b.name))
    });
    Ok(items)
}

fn persist_profiles(db: &Database, items: &[SshPushProfile]) -> Result<(), String> {
    let json = serde_json::to_string(items).map_err(|e| format!("序列化常用 SSH 主机失败: {e}"))?;
    db.set_setting(SSH_PUSH_PROFILES_KEY, &json)
        .map_err(|e| format!("保存常用 SSH 主机失败: {e}"))
}

pub fn save_ssh_push_profile(
    db: &Database,
    request: SaveSshPushProfileRequest,
) -> Result<SshPushProfile, String> {
    let name = request.name.trim().to_string();
    if name.is_empty() || name.chars().count() > 60 {
        return Err("主机名称不能为空且不能超过 60 字".to_string());
    }
    if contains_secret_pattern(&name) || name.chars().any(char::is_control) {
        return Err("主机名称不能包含密钥文本或控制字符".to_string());
    }

    let spec = SshTargetSpec {
        host: request.host.trim().to_string(),
        user: request
            .user
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToOwned::to_owned),
        port: request.port,
        identity_file: request
            .identity_file
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToOwned::to_owned),
    };
    if let Some(id_file) = &spec.identity_file {
        if contains_secret_pattern(id_file) {
            return Err("私钥字段仅填写本机路径，请勿粘贴私钥或密钥内容".to_string());
        }
    }
    let _ = validate_ssh_target(&spec)?;

    let raw_mapping = request.path_mapping.unwrap_or_default();
    let validated_mapping = validate_path_mapping(&raw_mapping)?;
    let normalized_mapping = validated_mapping.into_normalized_mapping();

    let mut items = list_ssh_push_profiles(db)?;
    let now = chrono::Utc::now().timestamp();
    let id = match request
        .id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        Some(existing) => validate_profile_id(existing)?,
        None => format!("ssh-profile-{}", &Uuid::new_v4().simple().to_string()[..12]),
    };

    let profile = SshPushProfile {
        id: id.clone(),
        name,
        host: spec.host,
        user: spec.user,
        port: spec.port,
        identity_file: spec.identity_file,
        path_mapping: normalized_mapping,
        updated_at: now,
    };

    if let Some(slot) = items.iter_mut().find(|item| item.id == id) {
        *slot = profile.clone();
    } else {
        if items.len() >= MAX_SSH_PUSH_PROFILES {
            return Err(format!(
                "最多支持保存 {MAX_SSH_PUSH_PROFILES} 个常用 SSH 主机"
            ));
        }
        items.push(profile.clone());
    }

    persist_profiles(db, &items)?;
    Ok(profile)
}

pub fn delete_ssh_push_profile(db: &Database, id: &str) -> Result<bool, String> {
    let valid_id = validate_profile_id(id)?;
    let mut items = list_ssh_push_profiles(db)?;
    let before = items.len();
    items.retain(|item| item.id != valid_id);
    if items.len() == before {
        return Ok(false);
    }
    persist_profiles(db, &items)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ssh_push_profiles_crud_and_secret_guard() {
        let db = Database::memory().expect("in-memory db");
        assert!(list_ssh_push_profiles(&db).unwrap().is_empty());

        // Reject private key PEM pasted into identityFile
        let bad = save_ssh_push_profile(
            &db,
            SaveSshPushProfileRequest {
                id: None,
                name: "Prod".to_string(),
                host: "10.0.0.10".to_string(),
                user: Some("deploy".to_string()),
                port: Some(2222),
                identity_file: Some("-----BEGIN OPENSSH PRIVATE KEY-----".to_string()),
                path_mapping: None,
            },
        );
        assert!(bad.is_err());

        // Save valid profile with custom path mapping
        let saved = save_ssh_push_profile(
            &db,
            SaveSshPushProfileRequest {
                id: None,
                name: "GPU 开发机".to_string(),
                host: "10.0.0.12".to_string(),
                user: Some("dev".to_string()),
                port: Some(2222),
                identity_file: Some("~/.ssh/id_ed25519".to_string()),
                path_mapping: Some(RemotePathMapping {
                    home_root: Some("~/work-env".to_string()),
                    claude_dir: Some(".claude-work".to_string()),
                    codex_dir: None,
                    opencode_dir: None,
                    omp_dir: None,
                }),
            },
        )
        .unwrap();
        assert!(saved.id.starts_with("ssh-profile-"));
        assert_eq!(saved.path_mapping.home_root.as_deref(), Some("~/work-env"));
        assert_eq!(
            saved.path_mapping.claude_dir.as_deref(),
            Some(".claude-work")
        );

        let listed = list_ssh_push_profiles(&db).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].name, "GPU 开发机");

        // Update in place
        let updated = save_ssh_push_profile(
            &db,
            SaveSshPushProfileRequest {
                id: Some(saved.id.clone()),
                name: "GPU 主节点".to_string(),
                host: "10.0.0.12".to_string(),
                user: Some("dev".to_string()),
                port: Some(2200),
                identity_file: None,
                path_mapping: None,
            },
        )
        .unwrap();
        assert_eq!(updated.id, saved.id);
        assert_eq!(updated.port, Some(2200));

        assert!(delete_ssh_push_profile(&db, &saved.id).unwrap());
        assert!(list_ssh_push_profiles(&db).unwrap().is_empty());
    }
}
