//! A connection mutation is admitted only after a matching file-impact
//! preview. This is an Auth admission boundary, not another write executor:
//! existing consumer adapters still own their locks, CAS and readback.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::config::{display_user_path, file_write_target};
use crate::services::secret::SecretBackend;

use super::{
    CredentialPurpose, CredentialStatus, ManagedAuthConnectionAction,
    ManagedAuthConnectionActionPreview, ManagedAuthConnectionActionRequest,
    ManagedAuthConnectionSummary, ManagedAuthConsumer, ManagedAuthErrorDto, ManagedAuthHealth,
    ManagedAuthMutationResult, ManagedAuthReasonCode, ManagedAuthService, RefreshOwner,
    MANAGED_AUTH_CONTRACT_VERSION,
};

const PREVIEW_LIFETIME: Duration = Duration::from_secs(5 * 60);
const MAX_PREVIEWS: usize = 32;

#[derive(Default)]
pub(super) struct ConnectionActionPreviews {
    entries: Mutex<HashMap<String, PreparedAction>>,
}

struct PreparedAction {
    request: ManagedAuthConnectionActionRequest,
    paths: Vec<PathBuf>,
    expires_at: Instant,
}

impl ConnectionActionPreviews {
    fn issue(
        &self,
        request: &ManagedAuthConnectionActionRequest,
        paths: Vec<PathBuf>,
    ) -> Result<String, ManagedAuthErrorDto> {
        let mut entries = self.entries.lock().map_err(|_| conflict())?;
        let now = Instant::now();
        entries.retain(|_, entry| {
            entry.expires_at > now && entry.request.connection_id != request.connection_id
        });
        if entries.len() >= MAX_PREVIEWS {
            return Err(conflict());
        }
        let preview_id = uuid::Uuid::new_v4().to_string();
        entries.insert(
            preview_id.clone(),
            PreparedAction {
                request: request.clone(),
                paths,
                expires_at: now + PREVIEW_LIFETIME,
            },
        );
        Ok(preview_id)
    }

    fn consume(
        &self,
        id: &str,
        request: &ManagedAuthConnectionActionRequest,
    ) -> Result<PreparedAction, ManagedAuthErrorDto> {
        let uuid = uuid::Uuid::parse_str(id).map_err(|_| stale())?;
        if uuid.get_version() != Some(uuid::Version::Random) || uuid.to_string() != id {
            return Err(stale());
        }
        let mut entries = self.entries.lock().map_err(|_| conflict())?;
        let entry = entries.remove(id).ok_or_else(stale)?;
        if entry.expires_at <= Instant::now() || entry.request != *request {
            return Err(stale());
        }
        Ok(entry)
    }
}

fn conflict() -> ManagedAuthErrorDto {
    ManagedAuthErrorDto::with_reason(ManagedAuthReasonCode::OperationConflict)
}

fn stale() -> ManagedAuthErrorDto {
    ManagedAuthErrorDto::with_reason(ManagedAuthReasonCode::TargetChanged)
}

fn requires_confirmation(action: ManagedAuthConnectionAction) -> bool {
    !matches!(
        action,
        ManagedAuthConnectionAction::Refresh | ManagedAuthConnectionAction::Restart
    )
}

impl<B: SecretBackend + 'static> ManagedAuthService<B> {
    pub(crate) fn preview_connection_action(
        &self,
        request: &ManagedAuthConnectionActionRequest,
    ) -> Result<ManagedAuthConnectionActionPreview, ManagedAuthErrorDto> {
        request.validate()?;
        if !requires_confirmation(request.action) {
            return Err(ManagedAuthErrorDto::invalid_request());
        }
        let connection = self.observe_connection_action(request)?;
        let (paths, preserved) = self.connection_action_paths(&connection, request.action)?;
        let write_targets = paths
            .iter()
            .map(|path| {
                file_write_target(path).map_err(|_| {
                    ManagedAuthErrorDto::with_reason(ManagedAuthReasonCode::ConnectionUnavailable)
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        // Bind all resolved paths, including explicitly preserved config paths.
        // A changed override cannot redirect an already-confirmed operation.
        let bound_paths = paths.into_iter().chain(preserved.iter().cloned()).collect();
        let preview_id = self.connection_previews.issue(request, bound_paths)?;
        Ok(ManagedAuthConnectionActionPreview {
            contract_version: MANAGED_AUTH_CONTRACT_VERSION,
            preview_id,
            connection_id: request.connection_id.clone(),
            expected_revision: request.expected_revision.clone(),
            action: request.action,
            account_id: request.account_id.clone(),
            write_targets,
            preserved_paths: preserved
                .iter()
                .map(|path| display_user_path(path))
                .collect(),
            can_apply: true,
            reason_codes: Vec::new(),
        })
    }

    pub(crate) fn apply_connection_action(
        &self,
        request: &ManagedAuthConnectionActionRequest,
        preview_id: Option<&str>,
    ) -> Result<ManagedAuthMutationResult, ManagedAuthErrorDto> {
        request.validate()?;
        if requires_confirmation(request.action) {
            let preview_id = preview_id.ok_or_else(ManagedAuthErrorDto::invalid_request)?;
            let prepared = self.connection_previews.consume(preview_id, request)?;
            let connection = self.observe_connection_action(request)?;
            let (paths, preserved) = self.connection_action_paths(&connection, request.action)?;
            if prepared.paths != paths.into_iter().chain(preserved).collect::<Vec<_>>() {
                return Err(stale());
            }
        } else {
            self.observe_connection_action(request)?;
        }
        self.apply_prepared_connection_action(request)
    }

    fn observe_connection_action(
        &self,
        request: &ManagedAuthConnectionActionRequest,
    ) -> Result<ManagedAuthConnectionSummary, ManagedAuthErrorDto> {
        let overview = self.overview();
        let connection = overview
            .connections
            .into_iter()
            .find(|connection| connection.connection_id == request.connection_id)
            .ok_or_else(|| {
                ManagedAuthErrorDto::with_reason(ManagedAuthReasonCode::ConnectionUnavailable)
            })?;
        if connection.revision != request.expected_revision {
            return Err(ManagedAuthErrorDto::with_reason(
                ManagedAuthReasonCode::ExternalChangeDetected,
            ));
        }
        if let Some(account_id) = request.account_id.as_deref() {
            let account = overview
                .accounts
                .iter()
                .find(|account| account.account_id == account_id)
                .ok_or_else(ManagedAuthErrorDto::invalid_request)?;
            if account.health != ManagedAuthHealth::Ready
                || Some(account.provider) != connection.provider
            {
                return Err(ManagedAuthErrorDto::with_reason(
                    ManagedAuthReasonCode::RequiresReauth,
                ));
            }
            let purpose = match connection.consumer {
                ManagedAuthConsumer::Codex => CredentialPurpose::CodexNative,
                ManagedAuthConsumer::Grokbuild => CredentialPurpose::GrokNative,
                ManagedAuthConsumer::Opencode => CredentialPurpose::OpencodeProvider,
                ManagedAuthConsumer::FyagentProxy => {
                    return Err(ManagedAuthErrorDto::with_reason(
                        ManagedAuthReasonCode::ProviderNotSupported,
                    ));
                }
            };
            let rows = self
                .credentials_for_account(account_id)
                .map_err(ManagedAuthErrorDto::from_core)?;
            if !rows.iter().any(|row| {
                row.credential.purpose == purpose
                    && row.credential.status == CredentialStatus::Ready
                    && Some(row.credential.provider) == connection.provider
                    && (connection.consumer != ManagedAuthConsumer::Opencode
                        || !matches!(
                            row.credential.refresh_owner,
                            RefreshOwner::CodexNative | RefreshOwner::GrokNative
                        ))
            }) {
                return Err(ManagedAuthErrorDto::with_reason(
                    ManagedAuthReasonCode::ProviderNotSupported,
                ));
            }
        }
        if !connection.allowed_actions.contains(&request.action) {
            return Err(ManagedAuthErrorDto::with_reason(
                ManagedAuthReasonCode::NativeProjectionUnavailable,
            ));
        }
        Ok(connection)
    }

    fn connection_action_paths(
        &self,
        connection: &ManagedAuthConnectionSummary,
        action: ManagedAuthConnectionAction,
    ) -> Result<(Vec<PathBuf>, Vec<PathBuf>), ManagedAuthErrorDto> {
        match (connection.consumer, action) {
            (
                ManagedAuthConsumer::Codex,
                ManagedAuthConnectionAction::ConnectAccount
                | ManagedAuthConnectionAction::SwitchAccount,
            ) => {
                let home = self.codex_home();
                Ok((vec![home.join("auth.json")], vec![home.join("config.toml")]))
            }
            (ManagedAuthConsumer::Codex, ManagedAuthConnectionAction::Disconnect) => {
                let home = self.codex_home();
                Ok((
                    Vec::new(),
                    vec![home.join("auth.json"), home.join("config.toml")],
                ))
            }
            (
                ManagedAuthConsumer::Opencode,
                ManagedAuthConnectionAction::ConnectAccount
                | ManagedAuthConnectionAction::SwitchAccount
                | ManagedAuthConnectionAction::Disconnect,
            ) => Ok((
                vec![self.opencode_auth_path()],
                vec![crate::opencode_config::get_opencode_config_path()],
            )),
            (ManagedAuthConsumer::Grokbuild, ManagedAuthConnectionAction::Disconnect) => Ok((
                Vec::new(),
                vec![
                    super::consumers::grok::default_grok_home().join("auth.json"),
                    crate::grok_config::get_grok_config_path(),
                ],
            )),
            (ManagedAuthConsumer::FyagentProxy, ManagedAuthConnectionAction::Disconnect) => {
                Ok((Vec::new(), Vec::new()))
            }
            // Grok's native projection remains closed by its owning evidence
            // gate; never advertise an empty write set for an unknown adapter.
            _ => Err(ManagedAuthErrorDto::with_reason(
                ManagedAuthReasonCode::NativeProjectionUnavailable,
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> ManagedAuthConnectionActionRequest {
        ManagedAuthConnectionActionRequest {
            connection_id: format!("mc1:{}", "a".repeat(32)),
            expected_revision: format!("mr1:{}", "b".repeat(64)),
            action: ManagedAuthConnectionAction::ConnectAccount,
            account_id: Some(format!("ma1:{}", "c".repeat(32))),
        }
    }

    #[test]
    fn preview_is_request_bound_expiring_and_single_use() {
        let store = ConnectionActionPreviews::default();
        let request = request();
        let id = store
            .issue(&request, vec![PathBuf::from("/fixture/auth.json")])
            .unwrap();
        assert!(store.consume(&id, &request).is_ok());
        assert!(store.consume(&id, &request).is_err());
        let id = store.issue(&request, Vec::new()).unwrap();
        let mut changed = request.clone();
        changed.account_id = Some(format!("ma1:{}", "d".repeat(32)));
        assert!(store.consume(&id, &changed).is_err());
        assert!(store.consume(&id, &request).is_err());
        for changed in [
            ManagedAuthConnectionActionRequest {
                action: ManagedAuthConnectionAction::SwitchAccount,
                ..request.clone()
            },
            ManagedAuthConnectionActionRequest {
                expected_revision: format!("mr1:{}", "e".repeat(64)),
                ..request.clone()
            },
        ] {
            let id = store.issue(&request, Vec::new()).unwrap();
            assert!(store.consume(&id, &changed).is_err());
            assert!(store.consume(&id, &request).is_err());
        }
        let id = store.issue(&request, Vec::new()).unwrap();
        store
            .entries
            .lock()
            .unwrap()
            .get_mut(&id)
            .unwrap()
            .expires_at = Instant::now() - Duration::from_secs(1);
        assert!(store.consume(&id, &request).is_err());
    }

    #[test]
    fn fresh_preview_supersedes_same_connection_without_unbounded_growth() {
        let store = ConnectionActionPreviews::default();
        let request = request();
        let old = store.issue(&request, Vec::new()).unwrap();
        for _ in 0..100 {
            store.issue(&request, Vec::new()).unwrap();
        }
        assert_eq!(store.entries.lock().unwrap().len(), 1);
        assert!(store.consume(&old, &request).is_err());
        assert!(requires_confirmation(
            ManagedAuthConnectionAction::Disconnect
        ));
        assert!(!requires_confirmation(ManagedAuthConnectionAction::Refresh));
    }
}
