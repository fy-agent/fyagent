//! xAI Device Code login for Managed Auth.
//!
//! HTTP, discovery allowlist, slow_down, and refresh rotation stay in
//! `XaiOAuthManager` / `xai_oauth_auth`. This adapter owns session dispatch
//! and vault admission. It does not persist tokens to `xai_oauth_auth.json`.

use std::sync::Arc;
use std::time::Duration;

use chrono::{SecondsFormat, Utc};
use zeroize::Zeroizing;

use crate::proxy::providers::xai_oauth_auth::{
    extract_xai_identity, next_xai_poll_interval, poll_xai_device_token, request_xai_device_code,
    request_xai_device_code_at, required_refresh_token, XaiDeviceTokenPoll, XaiOAuthEndpoints,
    XaiOAuthError, XAI_OFFICIAL_HOST,
};
use crate::services::managed_auth::core::{
    stable_connection_id, stable_revision, ConnectionRecord, ConnectionStatus,
};
use crate::services::managed_auth::login_sessions::{map_xai_reason, LoginSessionHandle};
use crate::services::managed_auth::migration::LegacyCredentialInput;
use crate::services::managed_auth::service::ManagedAuthService;
use crate::services::managed_auth::{
    CredentialPurpose, CredentialStatus, CredentialWithIdentity, ManagedAuthConsumer,
    ManagedAuthCoreError, ManagedAuthErrorDto, ManagedAuthLoginMethod, ManagedAuthLoginPurpose,
    ManagedAuthLoginSessionSnapshot, ManagedAuthLoginStage, ManagedAuthMutationOutcome,
    ManagedAuthMutationResult, ManagedAuthProvider, ManagedAuthReasonCode, ManagedAuthRequestMode,
    RefreshOwner, StartManagedAuthLoginRequest,
};
use crate::services::secret::SecretBackend;

const XAI_INTERACTIVE_SOURCE: &str = "interactive-xai-device";

#[derive(Clone, Default)]
pub(crate) struct XaiLoginHooks {
    pub endpoints: Option<XaiOAuthEndpoints>,
}

impl<B> ManagedAuthService<B>
where
    B: SecretBackend + 'static,
{
    pub(crate) fn start_login(
        self: &Arc<Self>,
        request: StartManagedAuthLoginRequest,
    ) -> Result<ManagedAuthLoginSessionSnapshot, ManagedAuthErrorDto> {
        request.validate()?;
        if request.provider != ManagedAuthProvider::Xai {
            return Err(ManagedAuthErrorDto::from_reason(
                ManagedAuthReasonCode::ProviderNotSupported,
            ));
        }
        if request.method != ManagedAuthLoginMethod::DeviceCode {
            return Err(ManagedAuthErrorDto::invalid_request());
        }
        if request.purpose == ManagedAuthLoginPurpose::ConnectConsumer
            && !matches!(
                request.consumer,
                Some(ManagedAuthConsumer::Grokbuild | ManagedAuthConsumer::FyagentProxy)
            )
        {
            return Err(ManagedAuthErrorDto::from_reason(
                ManagedAuthReasonCode::ProviderNotSupported,
            ));
        }
        let fail = self.fail_closed_snapshot();
        if fail.secret_unavailable {
            return Err(ManagedAuthErrorDto::from_reason(
                ManagedAuthReasonCode::SecretUnavailable,
            ));
        }
        if fail.migration_blocked {
            return Err(ManagedAuthErrorDto::from_reason(
                ManagedAuthReasonCode::MigrationBlocked,
            ));
        }
        let (snapshot, handle) = self.login_sessions.create(
            &request,
            ManagedAuthLoginMethod::DeviceCode,
            ManagedAuthLoginStage::Preparing,
            None,
            None,
            None,
            None,
        )?;
        debug_assert_eq!(snapshot.official_host, XAI_OFFICIAL_HOST);
        self.spawn_xai_device_login(request, handle);
        Ok(snapshot)
    }

    pub(crate) fn get_login_session(
        &self,
        session_id: &str,
    ) -> Result<ManagedAuthLoginSessionSnapshot, ManagedAuthErrorDto> {
        self.login_sessions.get(session_id)
    }

    pub(crate) fn cancel_login(
        &self,
        session_id: &str,
    ) -> Result<ManagedAuthLoginSessionSnapshot, ManagedAuthErrorDto> {
        self.login_sessions.cancel(session_id)
    }

    pub(crate) fn reopen_login(
        &self,
        session_id: &str,
    ) -> Result<ManagedAuthLoginSessionSnapshot, ManagedAuthErrorDto> {
        self.login_sessions.get(session_id)
    }

    pub(crate) fn switch_login_method(
        &self,
        session_id: &str,
        _method: ManagedAuthLoginMethod,
    ) -> Result<ManagedAuthLoginSessionSnapshot, ManagedAuthErrorDto> {
        let _ = self.login_sessions.get(session_id)?;
        Err(ManagedAuthErrorDto::invalid_request())
    }

    pub(crate) fn apply_connection_action(
        &self,
        request: &crate::services::managed_auth::ManagedAuthConnectionActionRequest,
    ) -> Result<ManagedAuthMutationResult, ManagedAuthErrorDto> {
        match request.action {
            crate::services::managed_auth::ManagedAuthConnectionAction::Refresh => {
                Ok(self.mutation_result(ManagedAuthMutationOutcome::Completed, None))
            }
            crate::services::managed_auth::ManagedAuthConnectionAction::Disconnect => {
                if let Some(connection) = self
                    .repository
                    .list_connections()
                    .map_err(ManagedAuthErrorDto::from_core)?
                    .into_iter()
                    .find(|row| row.connection_id == request.connection_id)
                {
                    let mut cleared = connection;
                    cleared.credential_id = None;
                    cleared.status = ConnectionStatus::Disconnected;
                    cleared.updated_at = chrono::Utc::now().timestamp();
                    self.repository
                        .upsert_connection(&cleared)
                        .map_err(ManagedAuthErrorDto::from_core)?;
                }
                Ok(self.mutation_result(ManagedAuthMutationOutcome::Completed, None))
            }
            crate::services::managed_auth::ManagedAuthConnectionAction::ConnectAccount
            | crate::services::managed_auth::ManagedAuthConnectionAction::SwitchAccount => {
                let account_id = request
                    .account_id
                    .as_deref()
                    .ok_or_else(ManagedAuthErrorDto::invalid_request)?;
                let rows = self
                    .credentials_for_account(account_id)
                    .map_err(ManagedAuthErrorDto::from_core)?;
                let Some(selected) = rows.iter().find(|row| {
                    row.credential.purpose == CredentialPurpose::GrokNative
                        && row.credential.status == CredentialStatus::Ready
                }) else {
                    return Err(ManagedAuthErrorDto::from_reason(
                        ManagedAuthReasonCode::NativeProjectionUnavailable,
                    ));
                };
                let _ = self.upsert_grok_connection_metadata(selected, false);
                Ok(self.mutation_result(
                    ManagedAuthMutationOutcome::Partial,
                    Some(ManagedAuthReasonCode::NativeProjectionUnavailable),
                ))
            }
            _ => Err(ManagedAuthErrorDto::from_reason(
                ManagedAuthReasonCode::ProviderNotSupported,
            )),
        }
    }

    fn spawn_xai_device_login(
        self: &Arc<Self>,
        request: StartManagedAuthLoginRequest,
        handle: LoginSessionHandle,
    ) {
        let service = Arc::clone(self);
        tauri::async_runtime::spawn(async move {
            let result = service.run_xai_device_login(&request, &handle).await;
            if !handle.is_current() {
                return;
            }
            if let Err((stage, reason)) = result {
                let _ = service.login_sessions.finish(
                    &handle.session_id,
                    handle.generation,
                    stage,
                    Some(reason),
                    None,
                    None,
                );
            }
        });
    }

    async fn run_xai_device_login(
        &self,
        request: &StartManagedAuthLoginRequest,
        handle: &LoginSessionHandle,
    ) -> Result<(), (ManagedAuthLoginStage, ManagedAuthReasonCode)> {
        self.set_stage(handle, ManagedAuthLoginStage::Preparing, None)?;
        let hooks = self.xai_login_hooks();
        let grant = match hooks.endpoints.as_ref() {
            Some(endpoints) => request_xai_device_code_at(endpoints).await,
            None => request_xai_device_code().await,
        }
        .map_err(map_xai_reason)?;
        if !handle.is_current() {
            return Err((
                ManagedAuthLoginStage::Cancelled,
                ManagedAuthReasonCode::Cancelled,
            ));
        }
        let expires_at = Utc::now()
            .checked_add_signed(chrono::TimeDelta::seconds(grant.expires_in as i64))
            .unwrap_or_else(Utc::now)
            .to_rfc3339_opts(SecondsFormat::Secs, true);
        self.login_sessions
            .update(&handle.session_id, handle.generation, |snapshot| {
                snapshot.method = ManagedAuthLoginMethod::DeviceCode;
                snapshot.stage = ManagedAuthLoginStage::AwaitingUser;
                snapshot.user_code = Some(grant.user_code.clone());
                snapshot.verification_uri = Some(grant.verification_uri.clone());
                snapshot.expires_at = Some(expires_at.clone());
                snapshot.reason_code = None;
            })
            .map_err(|_| {
                (
                    ManagedAuthLoginStage::Cancelled,
                    ManagedAuthReasonCode::Cancelled,
                )
            })?;
        let deadline = Utc::now() + chrono::TimeDelta::seconds(grant.expires_in as i64);
        let mut interval = grant.interval.max(1);
        loop {
            if !handle.is_current() {
                return Err((
                    ManagedAuthLoginStage::Cancelled,
                    ManagedAuthReasonCode::Cancelled,
                ));
            }
            if Utc::now() >= deadline {
                return Err((
                    ManagedAuthLoginStage::Expired,
                    ManagedAuthReasonCode::DeviceCodeExpired,
                ));
            }
            match poll_xai_device_token(&grant.token_endpoint, &grant.device_code).await {
                Ok(XaiDeviceTokenPoll::Granted(tokens)) => {
                    if !handle.is_current() {
                        return Err((
                            ManagedAuthLoginStage::Cancelled,
                            ManagedAuthReasonCode::Cancelled,
                        ));
                    }
                    self.set_stage(handle, ManagedAuthLoginStage::ExchangingCode, None)?;
                    return self.save_xai_grant(request, handle, tokens).await;
                }
                Ok(XaiDeviceTokenPoll::Pending) => {
                    tokio::time::sleep(Duration::from_secs(interval)).await;
                }
                Ok(XaiDeviceTokenPoll::SlowDown) => {
                    interval = next_xai_poll_interval(interval);
                    tokio::time::sleep(Duration::from_secs(interval)).await;
                }
                Err(XaiOAuthError::ExpiredToken) => {
                    return Err((
                        ManagedAuthLoginStage::Expired,
                        ManagedAuthReasonCode::DeviceCodeExpired,
                    ));
                }
                Err(error) => return Err(map_xai_reason(error)),
            }
        }
    }

    async fn save_xai_grant(
        &self,
        request: &StartManagedAuthLoginRequest,
        handle: &LoginSessionHandle,
        tokens: crate::proxy::providers::xai_oauth_auth::OAuthTokenResponse,
    ) -> Result<(), (ManagedAuthLoginStage, ManagedAuthReasonCode)> {
        if !handle.is_current() {
            return Err((
                ManagedAuthLoginStage::Cancelled,
                ManagedAuthReasonCode::Cancelled,
            ));
        }
        self.set_stage(handle, ManagedAuthLoginStage::SavingAccount, None)?;
        let identity = extract_xai_identity(&tokens).map_err(map_xai_reason)?;
        let refresh_token = required_refresh_token(&tokens).map_err(map_xai_reason)?;
        let (purpose, consumer) = purpose_for_xai_login(request);
        let admitted = tokio::task::block_in_place(|| {
            self.provision_legacy_credential(LegacyCredentialInput {
                migration_id: XAI_INTERACTIVE_SOURCE,
                provider: ManagedAuthProvider::Xai,
                purpose,
                consumer,
                legacy_account_id: identity.subject.clone(),
                provider_subject: identity.subject.clone(),
                provider_tenant: identity.tenant.clone(),
                login: identity.login.clone(),
                display_name: None,
                avatar_url: None,
                access_token: Some(Zeroizing::new(tokens.access_token.clone())),
                refresh_token: Some(Zeroizing::new(refresh_token)),
                id_token: tokens.id_token.clone().map(Zeroizing::new),
                desired_status: CredentialStatus::Ready,
                refresh_owner: RefreshOwner::Fyagent,
                authenticated_at: chrono::Utc::now().timestamp(),
                make_default: true,
            })
        })
        .map_err(|error| match error {
            ManagedAuthCoreError::SecretUnavailable | ManagedAuthCoreError::SecretMissing => (
                ManagedAuthLoginStage::Failed,
                ManagedAuthReasonCode::SecretUnavailable,
            ),
            ManagedAuthCoreError::InvalidData => (
                ManagedAuthLoginStage::Failed,
                ManagedAuthReasonCode::IdentityMismatch,
            ),
            _ => (
                ManagedAuthLoginStage::Failed,
                ManagedAuthReasonCode::LoginFailed,
            ),
        })?;
        let account_id = admitted.identity_id.clone();
        let mut connection_id = None;
        let mut stage = ManagedAuthLoginStage::Completed;
        let mut reason = None;
        if purpose == CredentialPurpose::ProxyUpstream {
            let _ = self.upsert_proxy_connections();
        }
        if request.purpose == ManagedAuthLoginPurpose::ConnectConsumer
            && request.consumer == Some(ManagedAuthConsumer::Grokbuild)
        {
            self.set_stage(handle, ManagedAuthLoginStage::ConnectingConsumer, None)?;
            let row = self
                .credentials_for_account(&account_id)
                .ok()
                .and_then(|rows| {
                    rows.into_iter()
                        .find(|row| row.credential.credential_id == admitted.credential_id)
                });
            if let Some(row) = row {
                let _ = self.upsert_grok_connection_metadata(&row, false);
                connection_id = Some(stable_connection_id(
                    ManagedAuthConsumer::Grokbuild,
                    "",
                    "xai",
                ));
            }
            stage = ManagedAuthLoginStage::Partial;
            reason = Some(ManagedAuthReasonCode::NativeProjectionUnavailable);
        }
        self.set_stage(handle, ManagedAuthLoginStage::Verifying, None)?;
        self.login_sessions
            .finish(
                &handle.session_id,
                handle.generation,
                stage,
                reason,
                Some(account_id),
                connection_id,
            )
            .map_err(|_| {
                (
                    ManagedAuthLoginStage::Cancelled,
                    ManagedAuthReasonCode::Cancelled,
                )
            })?;
        Ok(())
    }

    fn set_stage(
        &self,
        handle: &LoginSessionHandle,
        stage: ManagedAuthLoginStage,
        reason: Option<ManagedAuthReasonCode>,
    ) -> Result<(), (ManagedAuthLoginStage, ManagedAuthReasonCode)> {
        if !handle.is_current() {
            return Err((
                ManagedAuthLoginStage::Cancelled,
                ManagedAuthReasonCode::Cancelled,
            ));
        }
        self.login_sessions
            .update(&handle.session_id, handle.generation, |snapshot| {
                snapshot.stage = stage;
                snapshot.reason_code = reason;
            })
            .map(|_| ())
            .map_err(|_| {
                (
                    ManagedAuthLoginStage::Cancelled,
                    ManagedAuthReasonCode::Cancelled,
                )
            })
    }

    fn xai_login_hooks(&self) -> XaiLoginHooks {
        self.xai_hooks
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
    }

    fn upsert_grok_connection_metadata(
        &self,
        row: &CredentialWithIdentity,
        pending_restart: bool,
    ) -> Result<(), ManagedAuthCoreError> {
        let now = chrono::Utc::now().timestamp();
        let connection_id = stable_connection_id(ManagedAuthConsumer::Grokbuild, "", "xai");
        self.repository.upsert_connection(&ConnectionRecord {
            connection_id: connection_id.clone(),
            consumer: ManagedAuthConsumer::Grokbuild,
            target_id: String::new(),
            provider_slot: "xai".to_string(),
            credential_id: Some(row.credential.credential_id.clone()),
            desired_revision: stable_revision(&[&connection_id, "desired"]),
            observed_revision: Some(stable_revision(&[&connection_id, "observed"])),
            status: ConnectionStatus::Unavailable,
            request_mode: ManagedAuthRequestMode::OfficialSubscription,
            request_provider_label: Some("xai".to_string()),
            official_session_preserved: Some(true),
            pending_restart,
            created_at: now,
            updated_at: now,
        })
    }
}

fn purpose_for_xai_login(
    request: &StartManagedAuthLoginRequest,
) -> (CredentialPurpose, Option<ManagedAuthConsumer>) {
    match request.purpose {
        ManagedAuthLoginPurpose::ConnectConsumer
            if request.consumer == Some(ManagedAuthConsumer::Grokbuild) =>
        {
            (
                CredentialPurpose::GrokNative,
                Some(ManagedAuthConsumer::Grokbuild),
            )
        }
        _ => (
            CredentialPurpose::ProxyUpstream,
            Some(ManagedAuthConsumer::FyagentProxy),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;
    use crate::services::secret::{MemorySecretBackend, SecretService};
    use axum::extract::Form;
    use axum::http::StatusCode;
    use axum::routing::post;
    use axum::{Json, Router};
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicU32, Ordering};
    use tempfile::tempdir;

    fn jwt(payload: &str) -> String {
        let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"none"}"#);
        let body = URL_SAFE_NO_PAD.encode(payload.as_bytes());
        format!("{header}.{body}.sig")
    }

    fn token_json() -> serde_json::Value {
        let access = jwt(r#"{"sub":"xai-user-1","email":"person@x.ai"}"#);
        serde_json::json!({
            "access_token": access,
            "refresh_token": "refresh-xai-login",
            "id_token": access,
            "expires_in": 3600
        })
    }

    async fn spawn_issuer(pending_polls: u32, slow_down_once: bool) -> String {
        let remaining = Arc::new(AtomicU32::new(pending_polls));
        let slow_down = Arc::new(std::sync::atomic::AtomicBool::new(slow_down_once));
        let app = Router::new()
            .route(
                "/oauth2/device/code",
                post(|| async {
                    Json(serde_json::json!({
                        "device_code": "secret-device-code",
                        "user_code": "WXYZ-1234",
                        "verification_uri": "https://auth.x.ai/device",
                        "expires_in": 120,
                        "interval": 1
                    }))
                }),
            )
            .route(
                "/oauth2/token",
                post({
                    let remaining = Arc::clone(&remaining);
                    let slow_down = Arc::clone(&slow_down);
                    move |Form(form): Form<HashMap<String, String>>| {
                        let remaining = Arc::clone(&remaining);
                        let slow_down = Arc::clone(&slow_down);
                        async move {
                            let grant = form.get("grant_type").cloned().unwrap_or_default();
                            if grant == "refresh_token" {
                                return (StatusCode::OK, Json(token_json()));
                            }
                            if slow_down.swap(false, Ordering::SeqCst) {
                                return (
                                    StatusCode::BAD_REQUEST,
                                    Json(serde_json::json!({"error": "slow_down"})),
                                );
                            }
                            if remaining.fetch_sub(1, Ordering::SeqCst) > 0 {
                                return (
                                    StatusCode::BAD_REQUEST,
                                    Json(serde_json::json!({"error": "authorization_pending"})),
                                );
                            }
                            (StatusCode::OK, Json(token_json()))
                        }
                    }
                }),
            );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("issuer bind");
        let addr = listener.local_addr().expect("issuer addr");
        tokio::spawn(async move {
            axum::serve(listener, app).await.expect("issuer serve");
        });
        format!("http://127.0.0.1:{}", addr.port())
    }

    fn service() -> (
        Arc<ManagedAuthService<MemorySecretBackend>>,
        tempfile::TempDir,
    ) {
        let dir = tempdir().expect("tempdir");
        let db = Arc::new(Database::memory().expect("db"));
        let service = Arc::new(ManagedAuthService::new(
            db,
            SecretService::new(MemorySecretBackend::new()),
            dir.path().to_path_buf(),
        ));
        (service, dir)
    }

    async fn wait_terminal(
        service: &ManagedAuthService<MemorySecretBackend>,
        session_id: &str,
    ) -> ManagedAuthLoginSessionSnapshot {
        for _ in 0..200 {
            let snapshot = service.get_login_session(session_id).expect("session");
            if snapshot.terminal {
                return snapshot;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        panic!("login session did not terminate");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn device_code_saves_proxy_session_without_leaking_device_code() {
        let issuer = spawn_issuer(0, false).await;
        let (service, dir) = service();
        service.set_xai_login_hooks(XaiLoginHooks {
            endpoints: Some(XaiOAuthEndpoints::for_issuer(&issuer)),
        });
        let snapshot = service
            .start_login(StartManagedAuthLoginRequest {
                provider: ManagedAuthProvider::Xai,
                purpose: ManagedAuthLoginPurpose::SaveOnly,
                consumer: None,
                method: ManagedAuthLoginMethod::DeviceCode,
                account_id: None,
            })
            .expect("start");
        assert_eq!(snapshot.method, ManagedAuthLoginMethod::DeviceCode);
        assert_eq!(snapshot.official_host, XAI_OFFICIAL_HOST);
        assert!(snapshot.can_cancel);
        assert!(!snapshot.can_switch_to_device_code);
        let finished = wait_terminal(&service, &snapshot.session_id).await;
        assert_eq!(finished.stage, ManagedAuthLoginStage::Completed);
        assert!(finished.account_id.is_some());
        let json = serde_json::to_string(&finished)
            .unwrap()
            .to_ascii_lowercase();
        for forbidden in [
            "secret-device-code",
            "\"devicecode\"",
            "refresh-xai-login",
            "secretref",
            "access_token",
            "verifier",
        ] {
            assert!(!json.contains(forbidden), "leaked {forbidden}");
        }
        let rows = service.repository.list_all_credentials().expect("rows");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].credential.purpose, CredentialPurpose::ProxyUpstream);
        assert_eq!(
            rows[0].credential.consumer,
            Some(ManagedAuthConsumer::FyagentProxy)
        );
        assert_eq!(rows[0].credential.refresh_owner, RefreshOwner::Fyagent);
        assert!(!dir.path().join("auth.json").exists());
        let grok_home = dir.path().join("grok-home");
        assert!(matches!(
            crate::services::managed_auth::consumers::grok::project_grok_native(
                &grok_home,
                &Default::default(),
            ),
            Err(crate::services::managed_auth::consumers::grok::GrokStoreError::Unsupported)
        ));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn grok_connect_uses_separate_purpose_and_stays_fail_closed() {
        let issuer = spawn_issuer(0, false).await;
        let (service, dir) = service();
        service.set_xai_login_hooks(XaiLoginHooks {
            endpoints: Some(XaiOAuthEndpoints::for_issuer(&issuer)),
        });
        let snapshot = service
            .start_login(StartManagedAuthLoginRequest {
                provider: ManagedAuthProvider::Xai,
                purpose: ManagedAuthLoginPurpose::ConnectConsumer,
                consumer: Some(ManagedAuthConsumer::Grokbuild),
                method: ManagedAuthLoginMethod::DeviceCode,
                account_id: None,
            })
            .expect("start");
        let finished = wait_terminal(&service, &snapshot.session_id).await;
        assert_eq!(finished.stage, ManagedAuthLoginStage::Partial);
        assert_eq!(
            finished.reason_code,
            Some(ManagedAuthReasonCode::NativeProjectionUnavailable)
        );
        let rows = service.repository.list_all_credentials().expect("rows");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].credential.purpose, CredentialPurpose::GrokNative);
        assert_eq!(
            rows[0].credential.consumer,
            Some(ManagedAuthConsumer::Grokbuild)
        );
        assert_eq!(rows[0].credential.refresh_owner, RefreshOwner::Fyagent);
        assert!(!dir.path().join("auth.json").exists());
        let error = service
            .resolve_credential_access(rows[0].credential.clone())
            .await
            .expect_err("grok native must not resolve for proxy");
        assert!(matches!(error, ManagedAuthCoreError::Conflict));
    }

    #[test]
    fn openai_and_browser_loopback_stay_closed() {
        let (service, _dir) = service();
        assert!(service
            .start_login(StartManagedAuthLoginRequest {
                provider: ManagedAuthProvider::Openai,
                purpose: ManagedAuthLoginPurpose::SaveOnly,
                consumer: None,
                method: ManagedAuthLoginMethod::DeviceCode,
                account_id: None,
            })
            .is_err());
        assert!(StartManagedAuthLoginRequest {
            provider: ManagedAuthProvider::Xai,
            purpose: ManagedAuthLoginPurpose::SaveOnly,
            consumer: None,
            method: ManagedAuthLoginMethod::BrowserLoopback,
            account_id: None,
        }
        .validate()
        .is_err());
    }

    #[test]
    fn proxy_and_grok_purposes_do_not_share_credential_ids() {
        let proxy = crate::services::managed_auth::stable_credential_id(
            ManagedAuthProvider::Xai,
            CredentialPurpose::ProxyUpstream,
            Some(ManagedAuthConsumer::FyagentProxy),
            "xai-user-1",
        );
        let grok = crate::services::managed_auth::stable_credential_id(
            ManagedAuthProvider::Xai,
            CredentialPurpose::GrokNative,
            Some(ManagedAuthConsumer::Grokbuild),
            "xai-user-1",
        );
        assert_ne!(proxy, grok);
    }
}
