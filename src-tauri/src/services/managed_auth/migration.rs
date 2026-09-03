use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

use super::{
    CredentialPurpose, CredentialStatus, ManagedAuthConsumer, ManagedAuthCoreError,
    ManagedAuthProvider, ManagedAuthService, ManagedSecretKind, MigrationRecord, MigrationStatus,
    RefreshOwner,
};

const MAX_LEGACY_STORE_BYTES: u64 = 1024 * 1024;
const MAX_IDENTITY_TEXT: usize = 512;
const MAX_LOGIN_TEXT: usize = 320;
const MAX_TOKEN_TEXT: usize = 2_200;

const CODEX_MIGRATION_ID: &str = "legacy-codex-oauth-v2";
const XAI_MIGRATION_ID: &str = "legacy-xai-oauth-v1";
const COPILOT_MIGRATION_ID: &str = "legacy-copilot-auth-v3";

pub(super) struct LegacyCredentialInput {
    pub(super) migration_id: &'static str,
    pub(super) provider: ManagedAuthProvider,
    pub(super) purpose: CredentialPurpose,
    pub(super) consumer: Option<ManagedAuthConsumer>,
    pub(super) legacy_account_id: String,
    pub(super) provider_subject: String,
    pub(super) provider_tenant: String,
    pub(super) login: String,
    pub(super) display_name: Option<String>,
    pub(super) avatar_url: Option<String>,
    pub(super) secret_kind: ManagedSecretKind,
    pub(super) token: Zeroizing<String>,
    pub(super) status: CredentialStatus,
    pub(super) refresh_owner: RefreshOwner,
    pub(super) authenticated_at: i64,
    pub(super) make_default: bool,
}

struct LegacySource {
    migration_id: &'static str,
    source_kind: &'static str,
    filename: &'static str,
    parse: fn(&[u8]) -> Result<Vec<LegacyCredentialInput>, ManagedAuthCoreError>,
}

const SOURCES: &[LegacySource] = &[
    LegacySource {
        migration_id: CODEX_MIGRATION_ID,
        source_kind: "codex_oauth_v2",
        filename: "codex_oauth_auth.json",
        parse: parse_codex_store,
    },
    LegacySource {
        migration_id: XAI_MIGRATION_ID,
        source_kind: "xai_oauth_v1",
        filename: "xai_oauth_auth.json",
        parse: parse_xai_store,
    },
    LegacySource {
        migration_id: COPILOT_MIGRATION_ID,
        source_kind: "copilot_auth_v3",
        filename: "copilot_auth.json",
        parse: parse_copilot_store,
    },
];

pub(super) fn prepare_legacy_stores(
    service: &ManagedAuthService,
    config_dir: &Path,
) -> Result<(), ManagedAuthCoreError> {
    for source in SOURCES {
        let path = config_dir.join(source.filename);
        if !path.exists() {
            continue;
        }
        prepare_source(service, source, &path)?;
    }
    Ok(())
}

fn prepare_source(
    service: &ManagedAuthService,
    source: &LegacySource,
    path: &Path,
) -> Result<(), ManagedAuthCoreError> {
    let bytes = read_bounded(path)?;
    let source_hash = sha256_hex(&bytes);
    if let Some(existing) = service.repository().get_migration(source.migration_id)? {
        if existing.status == MigrationStatus::Completed {
            if existing.source_hash == source_hash {
                return Ok(());
            }
            return Err(ManagedAuthCoreError::MigrationBlocked);
        }
        if existing.status == MigrationStatus::Prepared && existing.source_hash == source_hash {
            service.recover_credentials()?;
            return Ok(());
        }
    }

    let now = chrono::Utc::now().timestamp();
    service.repository().upsert_migration(&MigrationRecord {
        migration_id: source.migration_id.to_string(),
        source_kind: source.source_kind.to_string(),
        source_hash: source_hash.clone(),
        status: MigrationStatus::Copying,
        reason_code: None,
        backup_name: None,
        created_at: now,
        updated_at: now,
        completed_at: None,
    })?;

    let result = (source.parse)(&bytes).and_then(|credentials| {
        for credential in credentials {
            service.provision_legacy_credential(credential)?;
        }
        Ok(())
    });

    match result {
        Ok(()) => {
            let now = chrono::Utc::now().timestamp();
            service.repository().upsert_migration(&MigrationRecord {
                migration_id: source.migration_id.to_string(),
                source_kind: source.source_kind.to_string(),
                source_hash,
                status: MigrationStatus::Prepared,
                reason_code: None,
                backup_name: None,
                created_at: now,
                updated_at: now,
                completed_at: None,
            })?;
            Ok(())
        }
        Err(error) => {
            let now = chrono::Utc::now().timestamp();
            let _ = service.repository().upsert_migration(&MigrationRecord {
                migration_id: source.migration_id.to_string(),
                source_kind: source.source_kind.to_string(),
                source_hash,
                status: MigrationStatus::Blocked,
                reason_code: Some("migration_blocked".to_string()),
                backup_name: None,
                created_at: now,
                updated_at: now,
                completed_at: None,
            });
            Err(error)
        }
    }
}

pub(super) fn finalize_legacy_store(
    service: &ManagedAuthService,
    config_dir: &Path,
    migration_id: &str,
) -> Result<(), ManagedAuthCoreError> {
    let source = SOURCES
        .iter()
        .find(|source| source.migration_id == migration_id)
        .ok_or(ManagedAuthCoreError::InvalidData)?;
    let path = config_dir.join(source.filename);
    let Some(mut migration) = service.repository().get_migration(migration_id)? else {
        return Err(ManagedAuthCoreError::NotFound);
    };
    if migration.status == MigrationStatus::Completed {
        return Ok(());
    }
    if migration.status != MigrationStatus::Prepared || !path.exists() {
        return Err(ManagedAuthCoreError::MigrationBlocked);
    }
    let bytes = read_bounded(&path)?;
    if sha256_hex(&bytes) != migration.source_hash {
        return Err(ManagedAuthCoreError::Stale);
    }
    service.recover_credentials()?;

    let backup_name = format!("{}.managed-auth-v1.bak", source.filename);
    let backup_path = config_dir.join(&backup_name);
    if backup_path.exists() {
        return Err(ManagedAuthCoreError::Conflict);
    }
    std::fs::rename(&path, &backup_path).map_err(|_| ManagedAuthCoreError::Io)?;
    migration.status = MigrationStatus::Completed;
    migration.backup_name = Some(backup_name);
    migration.updated_at = chrono::Utc::now().timestamp();
    migration.completed_at = Some(migration.updated_at);
    service.repository().upsert_migration(&migration)
}

fn read_bounded(path: &Path) -> Result<Vec<u8>, ManagedAuthCoreError> {
    let metadata = std::fs::symlink_metadata(path).map_err(|_| ManagedAuthCoreError::Io)?;
    if !metadata.file_type().is_file() || metadata.len() > MAX_LEGACY_STORE_BYTES {
        return Err(ManagedAuthCoreError::InvalidData);
    }
    let mut file = File::open(path).map_err(|_| ManagedAuthCoreError::Io)?;
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.take(MAX_LEGACY_STORE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| ManagedAuthCoreError::Io)?;
    if bytes.is_empty() || bytes.len() as u64 > MAX_LEGACY_STORE_BYTES {
        return Err(ManagedAuthCoreError::InvalidData);
    }
    Ok(bytes)
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn validate_text(value: &str, max: usize) -> Result<(), ManagedAuthCoreError> {
    if value.trim().is_empty()
        || value.len() > max
        || value.chars().any(|character| character.is_control())
    {
        Err(ManagedAuthCoreError::InvalidData)
    } else {
        Ok(())
    }
}

fn validate_token(value: &str) -> Result<(), ManagedAuthCoreError> {
    if value.is_empty() || value.len() > MAX_TOKEN_TEXT || value.as_bytes().contains(&0) {
        Err(ManagedAuthCoreError::InvalidData)
    } else {
        Ok(())
    }
}

#[derive(Deserialize)]
struct CodexStore {
    #[serde(default)]
    version: u32,
    #[serde(default)]
    accounts: HashMap<String, CodexAccount>,
    #[serde(default)]
    default_account_id: Option<String>,
}

#[derive(Deserialize)]
struct CodexAccount {
    #[serde(default)]
    credential_id: Option<String>,
    #[serde(default)]
    chatgpt_account_id: Option<String>,
    #[serde(default)]
    account_id: Option<String>,
    #[serde(default)]
    email: Option<String>,
    refresh_token: String,
    authenticated_at: i64,
}

fn parse_codex_store(bytes: &[u8]) -> Result<Vec<LegacyCredentialInput>, ManagedAuthCoreError> {
    let store: CodexStore =
        serde_json::from_slice(bytes).map_err(|_| ManagedAuthCoreError::InvalidData)?;
    if store.version > 2 {
        return Err(ManagedAuthCoreError::InvalidData);
    }
    let mut result = Vec::with_capacity(store.accounts.len());
    for (map_key, account) in store.accounts {
        let legacy_account_id = account.credential_id.unwrap_or(map_key.clone());
        let subject = account
            .chatgpt_account_id
            .or(account.account_id)
            .ok_or(ManagedAuthCoreError::InvalidData)?;
        validate_text(&legacy_account_id, MAX_IDENTITY_TEXT)?;
        validate_text(&subject, MAX_IDENTITY_TEXT)?;
        validate_token(&account.refresh_token)?;
        let login = account.email.unwrap_or_else(|| {
            let suffix: String = subject.chars().take(12).collect();
            format!("ChatGPT ({suffix})")
        });
        validate_text(&login, MAX_LOGIN_TEXT)?;
        result.push(LegacyCredentialInput {
            migration_id: CODEX_MIGRATION_ID,
            provider: ManagedAuthProvider::Openai,
            purpose: CredentialPurpose::ProxyUpstream,
            consumer: Some(ManagedAuthConsumer::FyagentProxy),
            legacy_account_id: legacy_account_id.clone(),
            provider_subject: subject,
            provider_tenant: String::new(),
            login,
            display_name: None,
            avatar_url: None,
            secret_kind: ManagedSecretKind::RefreshToken,
            token: Zeroizing::new(account.refresh_token),
            status: CredentialStatus::Ready,
            refresh_owner: RefreshOwner::Fyagent,
            authenticated_at: account.authenticated_at,
            make_default: store.default_account_id.as_deref()
                == Some(legacy_account_id.as_str()),
        });
    }
    Ok(result)
}

#[derive(Deserialize)]
struct XaiStore {
    #[serde(default)]
    version: u32,
    #[serde(default)]
    accounts: HashMap<String, XaiAccount>,
    #[serde(default)]
    default_account_id: Option<String>,
}

#[derive(Deserialize)]
struct XaiAccount {
    account_id: String,
    #[serde(default)]
    login: Option<String>,
    refresh_token: String,
    authenticated_at: i64,
    #[serde(default)]
    requires_reauth: bool,
}

fn parse_xai_store(bytes: &[u8]) -> Result<Vec<LegacyCredentialInput>, ManagedAuthCoreError> {
    let store: XaiStore =
        serde_json::from_slice(bytes).map_err(|_| ManagedAuthCoreError::InvalidData)?;
    if store.version > 1 {
        return Err(ManagedAuthCoreError::InvalidData);
    }
    let mut result = Vec::with_capacity(store.accounts.len());
    for (map_key, account) in store.accounts {
        validate_text(&map_key, MAX_IDENTITY_TEXT)?;
        validate_text(&account.account_id, MAX_IDENTITY_TEXT)?;
        validate_token(&account.refresh_token)?;
        if map_key != account.account_id {
            return Err(ManagedAuthCoreError::InvalidData);
        }
        let login = account.login.unwrap_or_else(|| {
            let suffix: String = account.account_id.chars().take(12).collect();
            format!("xAI ({suffix})")
        });
        validate_text(&login, MAX_LOGIN_TEXT)?;
        result.push(LegacyCredentialInput {
            migration_id: XAI_MIGRATION_ID,
            provider: ManagedAuthProvider::Xai,
            purpose: CredentialPurpose::ProxyUpstream,
            consumer: Some(ManagedAuthConsumer::FyagentProxy),
            legacy_account_id: map_key.clone(),
            provider_subject: account.account_id,
            provider_tenant: String::new(),
            login,
            display_name: None,
            avatar_url: None,
            secret_kind: ManagedSecretKind::RefreshToken,
            token: Zeroizing::new(account.refresh_token),
            status: if account.requires_reauth {
                CredentialStatus::RequiresReauth
            } else {
                CredentialStatus::Ready
            },
            refresh_owner: RefreshOwner::Fyagent,
            authenticated_at: account.authenticated_at,
            make_default: store.default_account_id.as_deref() == Some(map_key.as_str()),
        });
    }
    Ok(result)
}

#[derive(Deserialize)]
struct CopilotStore {
    #[serde(default)]
    version: u32,
    #[serde(default)]
    accounts: HashMap<String, CopilotAccount>,
    #[serde(default)]
    default_account_id: Option<String>,
    #[serde(default)]
    github_token: Option<String>,
}

#[derive(Deserialize)]
struct CopilotAccount {
    github_token: String,
    user: CopilotUser,
    authenticated_at: i64,
    #[serde(default = "default_github_domain")]
    github_domain: String,
}

#[derive(Deserialize)]
struct CopilotUser {
    login: String,
    id: u64,
    #[serde(default)]
    avatar_url: Option<String>,
}

fn default_github_domain() -> String {
    "github.com".to_string()
}

fn parse_copilot_store(bytes: &[u8]) -> Result<Vec<LegacyCredentialInput>, ManagedAuthCoreError> {
    let store: CopilotStore =
        serde_json::from_slice(bytes).map_err(|_| ManagedAuthCoreError::InvalidData)?;
    if store.version > 3 || (store.version < 2 && store.github_token.is_some()) {
        // The v1 format has no stable offline identity. It must remain in
        // place until the user can reauthenticate or the existing online
        // migration resolves the account.
        return Err(ManagedAuthCoreError::MigrationBlocked);
    }
    let mut result = Vec::with_capacity(store.accounts.len());
    for (map_key, account) in store.accounts {
        validate_text(&map_key, MAX_IDENTITY_TEXT)?;
        validate_text(&account.user.login, MAX_LOGIN_TEXT)?;
        validate_text(&account.github_domain, MAX_IDENTITY_TEXT)?;
        validate_token(&account.github_token)?;
        result.push(LegacyCredentialInput {
            migration_id: COPILOT_MIGRATION_ID,
            provider: ManagedAuthProvider::GithubCopilot,
            purpose: CredentialPurpose::Copilot,
            consumer: Some(ManagedAuthConsumer::FyagentProxy),
            legacy_account_id: map_key.clone(),
            provider_subject: account.user.id.to_string(),
            provider_tenant: account.github_domain,
            login: account.user.login,
            display_name: None,
            avatar_url: account.user.avatar_url,
            secret_kind: ManagedSecretKind::AccessToken,
            token: Zeroizing::new(account.github_token),
            status: CredentialStatus::Ready,
            refresh_owner: RefreshOwner::Fyagent,
            authenticated_at: account.authenticated_at,
            make_default: store.default_account_id.as_deref() == Some(map_key.as_str()),
        });
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codex_v2_parser_keeps_routing_identity_separate_from_credential_alias() {
        let input = br#"{
          "version": 2,
          "accounts": {
            "legacy-credential": {
              "credential_id": "legacy-credential",
              "chatgpt_account_id": "workspace-subject",
              "email": "person@example.com",
              "refresh_token": "refresh-value",
              "authenticated_at": 1700000000
            }
          },
          "default_account_id": "legacy-credential"
        }"#;
        let parsed = parse_codex_store(input).expect("parse");
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].legacy_account_id, "legacy-credential");
        assert_eq!(parsed[0].provider_subject, "workspace-subject");
        assert!(parsed[0].make_default);
    }

    #[test]
    fn copilot_v1_without_identity_fails_closed() {
        let input = br#"{"version":1,"accounts":{},"github_token":"token"}"#;
        assert!(matches!(
            parse_copilot_store(input),
            Err(ManagedAuthCoreError::MigrationBlocked)
        ));
    }
}
