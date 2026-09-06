//! Closed user-configuration recovery facade. Paths come only from the
//! existing native configuration owners, never from renderer input.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::ConfigService;
use crate::app_config::AppType;
use crate::config::{file_recovery, FileRecovery, FileWriteTarget};
use crate::store::AppState;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigFileRecoveryTarget {
    ClaudeSettings,
    ClaudeMcp,
    CodexAuth,
    CodexConfig,
    CodexCatalog,
    GrokConfig,
    OpencodeConfig,
    OpencodeAuth,
}

impl ConfigFileRecoveryTarget {
    fn path(self) -> PathBuf {
        match self {
            Self::ClaudeSettings => crate::config::get_claude_settings_path(),
            Self::ClaudeMcp => crate::config::get_claude_mcp_path(),
            Self::CodexAuth => crate::codex_config::get_codex_auth_path(),
            Self::CodexConfig => crate::codex_config::get_codex_config_path(),
            Self::CodexCatalog => crate::codex_config::get_codex_model_catalog_path(),
            Self::GrokConfig => crate::grok_config::get_grok_config_path(),
            Self::OpencodeConfig => crate::opencode_config::get_opencode_config_path(),
            Self::OpencodeAuth => crate::opencode_config::get_opencode_auth_json_path(),
        }
    }

    fn app_type(self) -> AppType {
        match self {
            Self::ClaudeSettings | Self::ClaudeMcp => AppType::Claude,
            Self::CodexAuth | Self::CodexConfig | Self::CodexCatalog => AppType::Codex,
            Self::GrokConfig => AppType::GrokBuild,
            Self::OpencodeConfig | Self::OpencodeAuth => AppType::OpenCode,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum ConfigFileRecoveryState {
    Available,
    None,
    ManualBackup,
    Conflict,
    Unavailable,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigFileRecoverySnapshot {
    contract_version: u8,
    target: ConfigFileRecoveryTarget,
    write_target: FileWriteTarget,
    state: ConfigFileRecoveryState,
    receipt_id: Option<String>,
    restores_existing_file: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConfigFileRecoveryRequest {
    pub target: ConfigFileRecoveryTarget,
    pub receipt_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum ConfigFileRecoveryErrorCode {
    InvalidRequest,
    Unavailable,
    ExternalChange,
    RecoveryRequired,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigFileRecoveryError {
    contract_version: u8,
    code: ConfigFileRecoveryErrorCode,
}

impl ConfigFileRecoveryError {
    fn new(code: ConfigFileRecoveryErrorCode) -> Self {
        Self {
            contract_version: 1,
            code,
        }
    }

    pub(crate) fn unavailable() -> Self {
        Self::new(ConfigFileRecoveryErrorCode::Unavailable)
    }
}

impl ConfigService {
    pub(crate) fn file_recoveries(
        targets: &[ConfigFileRecoveryTarget],
    ) -> Result<Vec<ConfigFileRecoverySnapshot>, ConfigFileRecoveryError> {
        let unique = targets.iter().collect::<std::collections::HashSet<_>>();
        if targets.is_empty() || targets.len() > 8 || targets.len() != unique.len() {
            return Err(ConfigFileRecoveryError::new(
                ConfigFileRecoveryErrorCode::InvalidRequest,
            ));
        }
        targets
            .iter()
            .map(|target| snapshot_at(*target, &target.path()))
            .collect()
    }

    pub(crate) fn restore_file_recovery(
        state: &AppState,
        request: &ConfigFileRecoveryRequest,
    ) -> Result<ConfigFileRecoverySnapshot, ConfigFileRecoveryError> {
        validate_receipt(&request.receipt_id)?;
        let _guard = futures::executor::block_on(
            state
                .proxy_service
                .lock_switch_for_app(request.target.app_type().as_str()),
        );
        let path = request.target.path();
        // The shared writer serializes restore with all managed file commits.
        // A receipt is accepted only while both postimage and backup match.
        restore_at(request, &path)
    }
}

fn validate_receipt(receipt_id: &str) -> Result<(), ConfigFileRecoveryError> {
    let uuid = uuid::Uuid::parse_str(receipt_id)
        .map_err(|_| ConfigFileRecoveryError::new(ConfigFileRecoveryErrorCode::InvalidRequest))?;
    if uuid.get_version() != Some(uuid::Version::Random) || uuid.to_string() != receipt_id {
        return Err(ConfigFileRecoveryError::new(
            ConfigFileRecoveryErrorCode::InvalidRequest,
        ));
    }
    Ok(())
}

fn snapshot_at(
    target: ConfigFileRecoveryTarget,
    path: &Path,
) -> Result<ConfigFileRecoverySnapshot, ConfigFileRecoveryError> {
    let write_target = crate::config::file_write_target(path)
        .map_err(|_| ConfigFileRecoveryError::unavailable())?;
    let (state, receipt_id, restores_existing_file) = match file_recovery(path) {
        Ok(Some(FileRecovery {
            receipt_id,
            had_file,
            can_restore: true,
        })) => (
            ConfigFileRecoveryState::Available,
            Some(receipt_id),
            Some(had_file),
        ),
        Ok(Some(_)) => (ConfigFileRecoveryState::Conflict, None, None),
        Ok(None) if crate::config::rolling_backup_path(path).exists() => {
            (ConfigFileRecoveryState::ManualBackup, None, None)
        }
        Ok(None) => (ConfigFileRecoveryState::None, None, None),
        Err(_) => (ConfigFileRecoveryState::Unavailable, None, None),
    };
    Ok(ConfigFileRecoverySnapshot {
        contract_version: 1,
        target,
        write_target,
        state,
        receipt_id,
        restores_existing_file,
    })
}

fn restore_at(
    request: &ConfigFileRecoveryRequest,
    path: &Path,
) -> Result<ConfigFileRecoverySnapshot, ConfigFileRecoveryError> {
    validate_receipt(&request.receipt_id)?;
    let snapshot = snapshot_at(request.target, path)?;
    if snapshot.state != ConfigFileRecoveryState::Available
        || snapshot.receipt_id.as_deref() != Some(&request.receipt_id)
    {
        return Err(ConfigFileRecoveryError::new(
            ConfigFileRecoveryErrorCode::ExternalChange,
        ));
    }
    let result = match request.target {
        ConfigFileRecoveryTarget::CodexAuth => {
            crate::services::managed_auth::consumers::codex::restore_auth_recovery(
                path,
                &request.receipt_id,
            )
        }
        ConfigFileRecoveryTarget::OpencodeAuth => {
            crate::services::managed_auth::consumers::opencode::restore_auth_recovery(
                path,
                &request.receipt_id,
            )
        }
        _ => crate::config::restore_file_recovery(path, &request.receipt_id),
    };
    result
        .map_err(|_| ConfigFileRecoveryError::new(ConfigFileRecoveryErrorCode::RecoveryRequired))?;
    snapshot_at(request.target, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requests_admit_only_closed_targets_and_canonical_receipts() {
        let id = uuid::Uuid::new_v4().to_string();
        let good = serde_json::json!({"target":"codex_auth","receiptId":id});
        assert!(serde_json::from_value::<ConfigFileRecoveryRequest>(good.clone()).is_ok());
        let mut with_path = good;
        with_path["path"] = serde_json::json!("/arbitrary/file");
        assert!(serde_json::from_value::<ConfigFileRecoveryRequest>(with_path).is_err());
        assert!(serde_json::from_str::<ConfigFileRecoveryTarget>(r#""../../config""#).is_err());
        assert!(validate_receipt("not-a-receipt").is_err());
        assert!(ConfigService::file_recoveries(&[]).is_err());
        assert!(ConfigService::file_recoveries(&[
            ConfigFileRecoveryTarget::CodexAuth,
            ConfigFileRecoveryTarget::CodexAuth,
        ])
        .is_err());
    }

    #[test]
    fn snapshot_discloses_paths_but_never_credential_bytes_or_hashes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("auth.json");
        std::fs::write(&path, b"old-test-token").unwrap();
        crate::config::atomic_write_private(&path, b"new-test-token").unwrap();
        let snapshot = snapshot_at(ConfigFileRecoveryTarget::CodexAuth, &path).unwrap();
        assert_eq!(snapshot.state, ConfigFileRecoveryState::Available);
        let json = serde_json::to_string(&snapshot).unwrap();
        assert!(json.contains("auth.json.fyagent.backup"));
        assert!(!json.contains("test-token"));
        assert!(!json.contains("sha256"));
        let request = ConfigFileRecoveryRequest {
            target: ConfigFileRecoveryTarget::CodexAuth,
            receipt_id: snapshot.receipt_id.unwrap(),
        };
        let after = restore_at(&request, &path).unwrap();
        assert_ne!(after.state, ConfigFileRecoveryState::Available);
        assert_eq!(std::fs::read(&path).unwrap(), b"old-test-token");
    }

    #[test]
    fn recovery_conflict_is_read_only_and_creation_undo_is_explicit() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        crate::config::atomic_write(&path, b"new").unwrap();
        let snapshot = snapshot_at(ConfigFileRecoveryTarget::ClaudeSettings, &path).unwrap();
        assert_eq!(snapshot.restores_existing_file, Some(false));
        let request = ConfigFileRecoveryRequest {
            target: ConfigFileRecoveryTarget::ClaudeSettings,
            receipt_id: snapshot.receipt_id.unwrap(),
        };
        std::fs::write(&path, b"external").unwrap();
        let error = restore_at(&request, &path).unwrap_err();
        assert_eq!(error.code, ConfigFileRecoveryErrorCode::ExternalChange);
        assert_eq!(std::fs::read(&path).unwrap(), b"external");
    }
}
