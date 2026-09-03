//! Process-local Managed Auth login sessions.
//!
//! Dialog close and hidden routes do not cancel. App restart drops this store
//! and never restores device codes, verifiers, or authorization codes.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use tokio::sync::watch;
use uuid::Uuid;

use super::{
    ManagedAuthConsumer, ManagedAuthErrorDto, ManagedAuthLoginMethod, ManagedAuthLoginPurpose,
    ManagedAuthLoginSessionSnapshot, ManagedAuthLoginStage, ManagedAuthProvider,
    ManagedAuthReasonCode, MANAGED_AUTH_CONTRACT_VERSION,
};
use crate::proxy::providers::xai_oauth_auth::XAI_OFFICIAL_HOST;

const MAX_SESSIONS: usize = 8;

#[derive(Clone)]
pub(crate) struct LoginSessionRecord {
    pub snapshot: ManagedAuthLoginSessionSnapshot,
    pub generation: u64,
    pub cancel: watch::Sender<bool>,
}

pub(crate) struct LoginSessionStore {
    inner: Mutex<HashMap<String, LoginSessionRecord>>,
    next_generation: AtomicU64,
}

impl Default for LoginSessionStore {
    fn default() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
            next_generation: AtomicU64::new(1),
        }
    }
}

impl LoginSessionStore {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn create(
        &self,
        request: &super::StartManagedAuthLoginRequest,
        method: ManagedAuthLoginMethod,
        stage: ManagedAuthLoginStage,
        reason_code: Option<ManagedAuthReasonCode>,
        user_code: Option<String>,
        verification_uri: Option<String>,
        expires_at: Option<String>,
    ) -> Result<(ManagedAuthLoginSessionSnapshot, LoginSessionHandle), ManagedAuthErrorDto> {
        let mut sessions = self.lock();
        if sessions
            .values()
            .any(|record| !record.snapshot.terminal && record.snapshot.provider == request.provider)
        {
            return Err(ManagedAuthErrorDto::from_reason(
                ManagedAuthReasonCode::OperationConflict,
            ));
        }
        if sessions.len() >= MAX_SESSIONS {
            sessions.retain(|_, record| !record.snapshot.terminal);
        }
        let session_id = Uuid::new_v4().hyphenated().to_string();
        let generation = self.next_generation.fetch_add(1, Ordering::SeqCst);
        let (cancel, cancel_rx) = watch::channel(false);
        let snapshot = build_snapshot(
            session_id.clone(),
            request.provider,
            request.purpose,
            request.consumer,
            method,
            stage,
            user_code,
            verification_uri,
            expires_at,
            request.account_id.clone(),
            None,
            reason_code,
        );
        sessions.insert(
            session_id.clone(),
            LoginSessionRecord {
                snapshot: snapshot.clone(),
                generation,
                cancel,
            },
        );
        Ok((
            snapshot,
            LoginSessionHandle {
                session_id,
                generation,
                cancel: cancel_rx,
                expected_generation: Arc::new(AtomicU64::new(generation)),
            },
        ))
    }

    pub(crate) fn get(
        &self,
        session_id: &str,
    ) -> Result<ManagedAuthLoginSessionSnapshot, ManagedAuthErrorDto> {
        self.lock()
            .get(session_id)
            .map(|record| record.snapshot.clone())
            .ok_or_else(ManagedAuthErrorDto::invalid_request)
    }

    pub(crate) fn active_snapshots(&self) -> Vec<ManagedAuthLoginSessionSnapshot> {
        self.lock()
            .values()
            .filter(|record| !record.snapshot.terminal)
            .map(|record| record.snapshot.clone())
            .collect()
    }

    pub(crate) fn cancel(
        &self,
        session_id: &str,
    ) -> Result<ManagedAuthLoginSessionSnapshot, ManagedAuthErrorDto> {
        let mut sessions = self.lock();
        let record = sessions
            .get_mut(session_id)
            .ok_or_else(ManagedAuthErrorDto::invalid_request)?;
        if record.snapshot.terminal {
            return Ok(record.snapshot.clone());
        }
        let _ = record.cancel.send(true);
        record.generation = record.generation.saturating_add(1);
        record.snapshot = apply_stage(
            &record.snapshot,
            ManagedAuthLoginStage::Cancelled,
            Some(ManagedAuthReasonCode::Cancelled),
            record.snapshot.account_id.clone(),
            record.snapshot.connection_id.clone(),
        );
        Ok(record.snapshot.clone())
    }

    pub(crate) fn update(
        &self,
        session_id: &str,
        expected_generation: u64,
        mut apply: impl FnMut(&mut ManagedAuthLoginSessionSnapshot),
    ) -> Result<ManagedAuthLoginSessionSnapshot, ManagedAuthErrorDto> {
        let mut sessions = self.lock();
        let record = sessions
            .get_mut(session_id)
            .ok_or_else(ManagedAuthErrorDto::invalid_request)?;
        if record.generation != expected_generation {
            return Err(ManagedAuthErrorDto::from_reason(
                ManagedAuthReasonCode::OperationConflict,
            ));
        }
        apply(&mut record.snapshot);
        record.snapshot = rebuild_flags(record.snapshot.clone());
        Ok(record.snapshot.clone())
    }

    pub(crate) fn finish(
        &self,
        session_id: &str,
        expected_generation: u64,
        stage: ManagedAuthLoginStage,
        reason_code: Option<ManagedAuthReasonCode>,
        account_id: Option<String>,
        connection_id: Option<String>,
    ) -> Result<ManagedAuthLoginSessionSnapshot, ManagedAuthErrorDto> {
        self.update(session_id, expected_generation, |snapshot| {
            *snapshot = apply_stage(
                snapshot,
                stage,
                reason_code,
                account_id.clone(),
                connection_id.clone(),
            );
        })
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, LoginSessionRecord>> {
        self.inner.lock().unwrap_or_else(|error| error.into_inner())
    }
}

#[derive(Clone)]
pub(crate) struct LoginSessionHandle {
    pub session_id: String,
    pub generation: u64,
    pub cancel: watch::Receiver<bool>,
    pub expected_generation: Arc<AtomicU64>,
}

impl LoginSessionHandle {
    pub(crate) fn is_current(&self) -> bool {
        !*self.cancel.borrow() && self.expected_generation.load(Ordering::SeqCst) == self.generation
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn build_snapshot(
    session_id: String,
    provider: ManagedAuthProvider,
    purpose: ManagedAuthLoginPurpose,
    consumer: Option<ManagedAuthConsumer>,
    method: ManagedAuthLoginMethod,
    stage: ManagedAuthLoginStage,
    user_code: Option<String>,
    verification_uri: Option<String>,
    expires_at: Option<String>,
    account_id: Option<String>,
    connection_id: Option<String>,
    reason_code: Option<ManagedAuthReasonCode>,
) -> ManagedAuthLoginSessionSnapshot {
    rebuild_flags(ManagedAuthLoginSessionSnapshot {
        contract_version: MANAGED_AUTH_CONTRACT_VERSION,
        session_id,
        provider,
        purpose,
        consumer,
        method,
        stage,
        can_cancel: false,
        can_retry: false,
        can_switch_to_device_code: false,
        official_host: official_host(provider).to_string(),
        user_code,
        verification_uri,
        expires_at,
        account_id,
        connection_id,
        reason_code,
        terminal: false,
    })
}

fn apply_stage(
    current: &ManagedAuthLoginSessionSnapshot,
    stage: ManagedAuthLoginStage,
    reason_code: Option<ManagedAuthReasonCode>,
    account_id: Option<String>,
    connection_id: Option<String>,
) -> ManagedAuthLoginSessionSnapshot {
    let terminal = is_terminal(stage);
    let (user_code, verification_uri, expires_at) =
        if current.method == ManagedAuthLoginMethod::DeviceCode && !terminal {
            (
                current.user_code.clone(),
                current.verification_uri.clone(),
                current.expires_at.clone(),
            )
        } else {
            (None, None, None)
        };
    rebuild_flags(ManagedAuthLoginSessionSnapshot {
        contract_version: MANAGED_AUTH_CONTRACT_VERSION,
        session_id: current.session_id.clone(),
        provider: current.provider,
        purpose: current.purpose,
        consumer: current.consumer,
        method: current.method,
        stage,
        can_cancel: false,
        can_retry: false,
        can_switch_to_device_code: false,
        official_host: current.official_host.clone(),
        user_code,
        verification_uri,
        expires_at,
        account_id,
        connection_id,
        reason_code,
        terminal: false,
    })
}

fn rebuild_flags(mut snapshot: ManagedAuthLoginSessionSnapshot) -> ManagedAuthLoginSessionSnapshot {
    let terminal = is_terminal(snapshot.stage);
    snapshot.terminal = terminal;
    snapshot.can_cancel = !terminal;
    snapshot.can_retry = matches!(
        snapshot.stage,
        ManagedAuthLoginStage::Failed | ManagedAuthLoginStage::Expired
    );
    snapshot.can_switch_to_device_code = snapshot.provider == ManagedAuthProvider::Openai
        && snapshot.method == ManagedAuthLoginMethod::BrowserLoopback
        && !terminal;
    if snapshot.method == ManagedAuthLoginMethod::BrowserLoopback {
        snapshot.user_code = None;
        snapshot.verification_uri = None;
        snapshot.expires_at = None;
    }
    snapshot
}

pub(crate) fn is_terminal(stage: ManagedAuthLoginStage) -> bool {
    matches!(
        stage,
        ManagedAuthLoginStage::Completed
            | ManagedAuthLoginStage::Partial
            | ManagedAuthLoginStage::Failed
            | ManagedAuthLoginStage::Cancelled
            | ManagedAuthLoginStage::Expired
    )
}

pub(crate) fn official_host(provider: ManagedAuthProvider) -> &'static str {
    match provider {
        ManagedAuthProvider::Openai => "auth.openai.com",
        ManagedAuthProvider::Xai => XAI_OFFICIAL_HOST,
        ManagedAuthProvider::GithubCopilot => "github.com",
    }
}

pub(crate) fn map_xai_reason(
    error: crate::proxy::providers::xai_oauth_auth::XaiOAuthError,
) -> (ManagedAuthLoginStage, ManagedAuthReasonCode) {
    use crate::proxy::providers::xai_oauth_auth::XaiOAuthError;
    match error {
        XaiOAuthError::AuthorizationPending => (
            ManagedAuthLoginStage::AwaitingUser,
            ManagedAuthReasonCode::LoginFailed,
        ),
        XaiOAuthError::AccessDenied => (
            ManagedAuthLoginStage::Cancelled,
            ManagedAuthReasonCode::Cancelled,
        ),
        XaiOAuthError::ExpiredToken => (
            ManagedAuthLoginStage::Expired,
            ManagedAuthReasonCode::DeviceCodeExpired,
        ),
        XaiOAuthError::ParseError(_) => (
            ManagedAuthLoginStage::Failed,
            ManagedAuthReasonCode::IdentityMismatch,
        ),
        XaiOAuthError::RefreshTokenInvalid | XaiOAuthError::ReauthRequired(_) => (
            ManagedAuthLoginStage::Failed,
            ManagedAuthReasonCode::LoginFailed,
        ),
        XaiOAuthError::TokenFetchFailed(_)
        | XaiOAuthError::NetworkError(_)
        | XaiOAuthError::IoError(_)
        | XaiOAuthError::AccountNotFound(_) => (
            ManagedAuthLoginStage::Failed,
            ManagedAuthReasonCode::LoginFailed,
        ),
    }
}

impl ManagedAuthErrorDto {
    pub(crate) fn from_reason(reason_code: ManagedAuthReasonCode) -> Self {
        Self {
            contract_version: MANAGED_AUTH_CONTRACT_VERSION,
            reason_code,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::managed_auth::StartManagedAuthLoginRequest;

    #[test]
    fn new_store_is_empty_after_restart() {
        let store = LoginSessionStore::default();
        assert!(store.active_snapshots().is_empty());
    }

    #[test]
    fn xai_device_snapshot_keeps_user_code_and_hides_switch() {
        let snapshot = build_snapshot(
            Uuid::new_v4().hyphenated().to_string(),
            ManagedAuthProvider::Xai,
            ManagedAuthLoginPurpose::SaveOnly,
            None,
            ManagedAuthLoginMethod::DeviceCode,
            ManagedAuthLoginStage::AwaitingUser,
            Some("ABCD-EFGH".into()),
            Some("https://auth.x.ai/device".into()),
            Some(crate::services::managed_auth::now_timestamp()),
            None,
            None,
            None,
        );
        assert!(snapshot.can_cancel);
        assert!(!snapshot.can_retry);
        assert!(!snapshot.can_switch_to_device_code);
        assert!(!snapshot.terminal);
        assert_eq!(snapshot.official_host, XAI_OFFICIAL_HOST);
        assert_eq!(snapshot.user_code.as_deref(), Some("ABCD-EFGH"));
        assert_eq!(
            snapshot.verification_uri.as_deref(),
            Some("https://auth.x.ai/device")
        );
        let text = serde_json::to_string(&snapshot)
            .unwrap()
            .to_ascii_lowercase();
        assert_eq!(snapshot.method, ManagedAuthLoginMethod::DeviceCode);
        assert!(
            !text.contains("\"devicecode\""),
            "snapshot must not expose the OAuth device_code grant"
        );
        assert!(!text.contains("refresh_token"));
    }

    #[test]
    fn cancel_is_terminal_and_drops_late_generation() {
        let store = LoginSessionStore::default();
        let request = StartManagedAuthLoginRequest {
            provider: ManagedAuthProvider::Xai,
            purpose: ManagedAuthLoginPurpose::SaveOnly,
            consumer: None,
            method: ManagedAuthLoginMethod::DeviceCode,
            account_id: None,
        };
        let (snapshot, handle) = store
            .create(
                &request,
                ManagedAuthLoginMethod::DeviceCode,
                ManagedAuthLoginStage::AwaitingUser,
                None,
                Some("ABCD-EFGH".into()),
                Some("https://auth.x.ai/device".into()),
                None,
            )
            .expect("create");
        let cancelled = store.cancel(&snapshot.session_id).expect("cancel");
        assert_eq!(cancelled.stage, ManagedAuthLoginStage::Cancelled);
        assert_eq!(
            cancelled.reason_code,
            Some(ManagedAuthReasonCode::Cancelled)
        );
        assert!(cancelled.terminal);
        assert!(!handle.is_current());
        assert!(store
            .finish(
                &snapshot.session_id,
                handle.generation,
                ManagedAuthLoginStage::Completed,
                None,
                Some("ma1:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into()),
                None,
            )
            .is_err());
    }
}
