//! Backend-owned OpenAI login workers.

use std::io::ErrorKind;
use std::sync::atomic::{AtomicU32, Ordering as AtomicOrdering};
use std::sync::Arc;
use std::time::Duration;

use chrono::{SecondsFormat, Utc};
use zeroize::Zeroizing;

use super::consumers::codex::file_projection_enabled;
use super::core::{stable_connection_id, stable_revision, ConnectionRecord, ConnectionStatus};
use super::login_sessions::{map_openai_reason, LoginSessionHandle};
use super::migration::LegacyCredentialInput;
use super::providers::openai::{
    accept_one_callback, bind_registered_loopback, build_authorize_url,
    exchange_authorization_code, extract_identity, generate_pkce, generate_state,
    loopback_redirect_uri, open_system_browser, poll_device_authorization, request_device_usercode,
    CallbackDecision, LoopbackBindOutcome, OpenAiOAuthEndpoints, OpenAiOAuthError, PkceCodes,
    LOOPBACK_FALLBACK_PORT, OPENAI_DEVICE_VERIFICATION_URL,
};
use super::{
    CredentialPurpose, CredentialStatus, CredentialWithIdentity, ManagedAuthConsumer,
    ManagedAuthCoreError, ManagedAuthErrorDto, ManagedAuthLoginMethod, ManagedAuthLoginPurpose,
    ManagedAuthLoginSessionSnapshot, ManagedAuthLoginStage, ManagedAuthMutationOutcome,
    ManagedAuthMutationResult, ManagedAuthProvider, ManagedAuthReasonCode, ManagedAuthRequestMode,
    RefreshOwner, StartManagedAuthLoginRequest,
};
use crate::services::secret::SecretBackend;

use super::service::ManagedAuthService;

#[derive(Clone)]
pub(crate) struct LoginHooks {
    pub endpoints: OpenAiOAuthEndpoints,
    pub open_browser: fn(&str) -> Result<(), ErrorKind>,
    pub occupy_preferred: bool,
    pub occupy_fallback: bool,
    pub bind_ephemeral: bool,
    pub fixed_state: Option<String>,
    pub fixed_pkce: Option<PkceCodes>,
    pub bound_port: std::sync::Arc<std::sync::Mutex<Option<u16>>>,
    pub open_count: Arc<AtomicU32>,
}

impl Default for LoginHooks {
    fn default() -> Self {
        Self {
            endpoints: OpenAiOAuthEndpoints::production(),
            open_browser: open_system_browser,
            occupy_preferred: false,
            occupy_fallback: false,
            bind_ephemeral: false,
            fixed_state: None,
            fixed_pkce: None,
            bound_port: std::sync::Arc::new(std::sync::Mutex::new(None)),
            open_count: Arc::new(AtomicU32::new(0)),
        }
    }
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
        if request.provider == ManagedAuthProvider::Xai {
            return self.start_xai_login(request);
        }
        if request.provider != ManagedAuthProvider::Openai {
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
        let mut method = request.method;
        if method == ManagedAuthLoginMethod::BrowserLoopback && self.loopback_both_busy() {
            method = ManagedAuthLoginMethod::DeviceCode;
        }
        let (snapshot, handle) = self.login_sessions.create(
            &request,
            method,
            ManagedAuthLoginStage::Preparing,
            None,
            None,
            None,
            None,
        )?;
        self.spawn_login(request, method, handle);
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
        let snapshot = self.login_sessions.get(session_id)?;
        if snapshot.terminal {
            return Ok(snapshot);
        }
        if let Some(url) = self.login_sessions.reopen_target(session_id)? {
            let hooks = self.login_hooks();
            if (hooks.open_browser)(&url).is_ok() {
                hooks.open_count.fetch_add(1, AtomicOrdering::SeqCst);
            }
        }
        self.login_sessions.get(session_id)
    }

    pub(crate) fn switch_login_method(
        self: &Arc<Self>,
        session_id: &str,
        method: ManagedAuthLoginMethod,
    ) -> Result<ManagedAuthLoginSessionSnapshot, ManagedAuthErrorDto> {
        if method != ManagedAuthLoginMethod::DeviceCode {
            return Err(ManagedAuthErrorDto::invalid_request());
        }
        let current = self.login_sessions.get(session_id)?;
        if current.terminal
            || current.provider != ManagedAuthProvider::Openai
            || current.method != ManagedAuthLoginMethod::BrowserLoopback
        {
            return Err(ManagedAuthErrorDto::invalid_request());
        }
        let generation = self.login_sessions.bump_generation(session_id)?;
        let snapshot = self.login_sessions.update(session_id, generation, |item| {
            item.method = ManagedAuthLoginMethod::DeviceCode;
            item.stage = ManagedAuthLoginStage::Preparing;
            item.user_code = None;
            item.verification_uri = None;
            item.expires_at = None;
            item.reason_code = None;
        })?;
        let request = StartManagedAuthLoginRequest {
            provider: current.provider,
            purpose: current.purpose,
            consumer: current.consumer,
            method: ManagedAuthLoginMethod::DeviceCode,
            account_id: current.account_id.clone(),
        };
        let handle = self.login_sessions.handle_for(session_id, generation)?;
        self.spawn_login(request, ManagedAuthLoginMethod::DeviceCode, handle);
        Ok(snapshot)
    }

    pub(crate) fn apply_connection_action(
        &self,
        request: &super::ManagedAuthConnectionActionRequest,
    ) -> Result<ManagedAuthMutationResult, ManagedAuthErrorDto> {
        let grok_slot = stable_connection_id(ManagedAuthConsumer::Grokbuild, "", "xai");
        if super::consumers::opencode::slot_for_connection_id(&request.connection_id).is_some() {
            return self.apply_opencode_connection_action(request);
        }
        if request.connection_id == grok_slot
            || self
                .repository
                .list_connections()
                .ok()
                .into_iter()
                .flatten()
                .any(|row| {
                    row.connection_id == request.connection_id
                        && row.consumer == ManagedAuthConsumer::Grokbuild
                })
        {
            return self.apply_grok_connection_action(request);
        }
        match request.action {
            super::ManagedAuthConnectionAction::Refresh => {
                Ok(self.mutation_result(ManagedAuthMutationOutcome::Completed, None))
            }
            super::ManagedAuthConnectionAction::Disconnect => {
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
            super::ManagedAuthConnectionAction::ConnectAccount
            | super::ManagedAuthConnectionAction::SwitchAccount => {
                let account_id = request
                    .account_id
                    .as_deref()
                    .ok_or_else(ManagedAuthErrorDto::invalid_request)?;
                let rows = self
                    .credentials_for_account(account_id)
                    .map_err(ManagedAuthErrorDto::from_core)?;
                let Some(selected) = rows.iter().find(|row| {
                    row.credential.purpose == CredentialPurpose::CodexNative
                        && row.credential.status == CredentialStatus::Ready
                }) else {
                    return Err(ManagedAuthErrorDto::from_reason(
                        ManagedAuthReasonCode::NativeProjectionUnavailable,
                    ));
                };
                if !file_projection_enabled() {
                    self.upsert_codex_connection_metadata(selected, false)
                        .map_err(ManagedAuthErrorDto::from_core)?;
                    return Ok(self.mutation_result(
                        ManagedAuthMutationOutcome::Partial,
                        Some(ManagedAuthReasonCode::NativeProjectionUnavailable),
                    ));
                }
                Err(ManagedAuthErrorDto::from_reason(
                    ManagedAuthReasonCode::NativeProjectionUnavailable,
                ))
            }
            _ => Err(ManagedAuthErrorDto::from_reason(
                ManagedAuthReasonCode::ProviderNotSupported,
            )),
        }
    }

    fn spawn_login(
        self: &Arc<Self>,
        request: StartManagedAuthLoginRequest,
        method: ManagedAuthLoginMethod,
        handle: LoginSessionHandle,
    ) {
        let service = Arc::clone(self);
        tauri::async_runtime::spawn(async move {
            let result = match method {
                ManagedAuthLoginMethod::BrowserLoopback => {
                    service.run_browser_login(&request, &handle).await
                }
                ManagedAuthLoginMethod::DeviceCode => {
                    service.run_device_login(&request, &handle).await
                }
            };
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

    async fn run_browser_login(
        &self,
        request: &StartManagedAuthLoginRequest,
        handle: &LoginSessionHandle,
    ) -> Result<(), (ManagedAuthLoginStage, ManagedAuthReasonCode)> {
        let hooks = self.login_hooks();
        self.set_stage(handle, ManagedAuthLoginStage::Preparing, None)?;
        let listener = match self.bind_loopback(&hooks) {
            Ok(listener) => listener,
            Err(LoopbackBindOutcome::BothBusy) => {
                return self.run_device_login(request, handle).await;
            }
        };
        let port = listener.port;
        *hooks
            .bound_port
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = Some(port);
        let pkce = hooks.fixed_pkce.clone().unwrap_or_else(generate_pkce);
        let state = hooks.fixed_state.clone().unwrap_or_else(generate_state);
        let redirect_uri = loopback_redirect_uri(port);
        let authorize_url = build_authorize_url(&hooks.endpoints, &redirect_uri, &pkce, &state);
        self.login_sessions
            .set_reopen_target(&handle.session_id, handle.generation, authorize_url.clone())
            .map_err(|_| {
                (
                    ManagedAuthLoginStage::Cancelled,
                    ManagedAuthReasonCode::Cancelled,
                )
            })?;
        self.set_stage(handle, ManagedAuthLoginStage::OpeningBrowser, None)?;
        if (hooks.open_browser)(&authorize_url).is_ok() {
            hooks.open_count.fetch_add(1, AtomicOrdering::SeqCst);
        }
        self.set_stage(handle, ManagedAuthLoginStage::AwaitingUser, None)?;
        let decision = accept_one_callback(
            listener,
            state,
            handle.generation,
            handle.expected_generation.clone(),
            handle.cancel.clone(),
        )
        .await
        .map_err(map_openai_reason)?;
        if !handle.is_current() {
            return Err((
                ManagedAuthLoginStage::Cancelled,
                ManagedAuthReasonCode::Cancelled,
            ));
        }
        let code = match decision {
            CallbackDecision::Authorized { code } => code,
            CallbackDecision::Denied => {
                return Err((
                    ManagedAuthLoginStage::Cancelled,
                    ManagedAuthReasonCode::Cancelled,
                ));
            }
            CallbackDecision::Invalid => {
                return Err((
                    ManagedAuthLoginStage::Failed,
                    ManagedAuthReasonCode::CallbackUnavailable,
                ));
            }
        };
        self.set_stage(handle, ManagedAuthLoginStage::ExchangingCode, None)?;
        let grant = exchange_authorization_code(
            &hooks.endpoints,
            &code,
            &pkce.code_verifier,
            &redirect_uri,
        )
        .await
        .map_err(map_openai_reason)?;
        self.save_grant(request, handle, grant).await
    }

    async fn run_device_login(
        &self,
        request: &StartManagedAuthLoginRequest,
        handle: &LoginSessionHandle,
    ) -> Result<(), (ManagedAuthLoginStage, ManagedAuthReasonCode)> {
        let hooks = self.login_hooks();
        self.set_stage(handle, ManagedAuthLoginStage::Preparing, None)?;
        let grant = request_device_usercode(&hooks.endpoints)
            .await
            .map_err(map_openai_reason)?;
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
                snapshot.verification_uri = Some(OPENAI_DEVICE_VERIFICATION_URL.to_string());
                snapshot.expires_at = Some(expires_at.clone());
                snapshot.reason_code = None;
            })
            .map_err(|_| {
                (
                    ManagedAuthLoginStage::Cancelled,
                    ManagedAuthReasonCode::Cancelled,
                )
            })?;
        let _ = self.login_sessions.set_reopen_target(
            &handle.session_id,
            handle.generation,
            OPENAI_DEVICE_VERIFICATION_URL.to_string(),
        );
        let deadline = Utc::now() + chrono::TimeDelta::seconds(grant.expires_in as i64);
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
            match poll_device_authorization(
                &hooks.endpoints,
                &grant.device_auth_id,
                &grant.user_code,
            )
            .await
            {
                Ok((code, verifier)) => {
                    if !handle.is_current() {
                        return Err((
                            ManagedAuthLoginStage::Cancelled,
                            ManagedAuthReasonCode::Cancelled,
                        ));
                    }
                    self.set_stage(handle, ManagedAuthLoginStage::ExchangingCode, None)?;
                    let tokens = exchange_authorization_code(
                        &hooks.endpoints,
                        &code,
                        &verifier,
                        &hooks.endpoints.device_redirect_uri,
                    )
                    .await
                    .map_err(map_openai_reason)?;
                    return self.save_grant(request, handle, tokens).await;
                }
                Err(OpenAiOAuthError::AuthorizationPending) => {
                    tokio::time::sleep(Duration::from_secs(grant.interval.max(1))).await;
                }
                Err(OpenAiOAuthError::ExpiredToken) => {
                    return Err((
                        ManagedAuthLoginStage::Expired,
                        ManagedAuthReasonCode::DeviceCodeExpired,
                    ));
                }
                Err(error) => return Err(map_openai_reason(error)),
            }
        }
    }

    async fn save_grant(
        &self,
        request: &StartManagedAuthLoginRequest,
        handle: &LoginSessionHandle,
        grant: super::providers::openai::OpenAiTokenGrant,
    ) -> Result<(), (ManagedAuthLoginStage, ManagedAuthReasonCode)> {
        if !handle.is_current() {
            return Err((
                ManagedAuthLoginStage::Cancelled,
                ManagedAuthReasonCode::Cancelled,
            ));
        }
        self.set_stage(handle, ManagedAuthLoginStage::SavingAccount, None)?;
        let identity = extract_identity(&grant).map_err(map_openai_reason)?;
        let (purpose, consumer) = purpose_for_login(request);
        let admitted = tokio::task::block_in_place(|| {
            self.provision_legacy_credential(LegacyCredentialInput {
                migration_id: None,
                provider: ManagedAuthProvider::Openai,
                purpose,
                consumer,
                legacy_account_id: format!("{}:{}", purpose.as_str(), identity.subject),
                provider_subject: identity.subject.clone(),
                provider_tenant: identity.tenant.clone(),
                login: identity.login.clone(),
                display_name: None,
                avatar_url: None,
                access_token: Some(Zeroizing::new(grant.access_token.clone())),
                refresh_token: grant.refresh_token.clone().map(Zeroizing::new),
                id_token: grant.id_token.clone().map(Zeroizing::new),
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
        if request.purpose == ManagedAuthLoginPurpose::ConnectConsumer
            && request.consumer == Some(ManagedAuthConsumer::Codex)
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
                let _ = self.upsert_codex_connection_metadata(&row, false);
                connection_id = Some(stable_connection_id(
                    ManagedAuthConsumer::Codex,
                    "",
                    "openai",
                ));
            }
            if !file_projection_enabled() {
                stage = ManagedAuthLoginStage::Partial;
                reason = Some(ManagedAuthReasonCode::NativeProjectionUnavailable);
            }
        }
        if request.purpose == ManagedAuthLoginPurpose::ConnectConsumer
            && request.consumer == Some(ManagedAuthConsumer::Opencode)
        {
            self.set_stage(handle, ManagedAuthLoginStage::ConnectingConsumer, None)?;
            let (next_connection, next_stage, next_reason) =
                self.finish_opencode_connect_after_login(ManagedAuthProvider::Openai, &account_id);
            connection_id = next_connection;
            stage = next_stage;
            reason = next_reason;
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

    pub(crate) fn set_stage(
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

    fn loopback_both_busy(&self) -> bool {
        let hooks = self.login_hooks();
        if hooks.occupy_preferred && hooks.occupy_fallback {
            return true;
        }
        bind_registered_loopback().is_err()
    }

    fn bind_loopback(
        &self,
        hooks: &LoginHooks,
    ) -> Result<super::providers::openai::LoopbackListener, LoopbackBindOutcome> {
        if hooks.occupy_preferred && hooks.occupy_fallback {
            return Err(LoopbackBindOutcome::BothBusy);
        }
        if hooks.bind_ephemeral {
            return bind_port(0).map_err(|_| LoopbackBindOutcome::BothBusy);
        }
        if hooks.occupy_preferred {
            return bind_port(LOOPBACK_FALLBACK_PORT).map_err(|_| LoopbackBindOutcome::BothBusy);
        }
        bind_registered_loopback()
    }

    fn login_hooks(&self) -> LoginHooks {
        self.login_hooks
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
    }

    fn upsert_codex_connection_metadata(
        &self,
        row: &CredentialWithIdentity,
        pending_restart: bool,
    ) -> Result<(), ManagedAuthCoreError> {
        let now = chrono::Utc::now().timestamp();
        let connection_id = stable_connection_id(ManagedAuthConsumer::Codex, "", "openai");
        self.repository.upsert_connection(&ConnectionRecord {
            connection_id: connection_id.clone(),
            consumer: ManagedAuthConsumer::Codex,
            target_id: String::new(),
            provider_slot: "openai".to_string(),
            credential_id: Some(row.credential.credential_id.clone()),
            desired_revision: stable_revision(&[&connection_id, "desired"]),
            observed_revision: Some(stable_revision(&[&connection_id, "observed"])),
            status: if pending_restart {
                ConnectionStatus::PendingRestart
            } else {
                ConnectionStatus::Connected
            },
            request_mode: ManagedAuthRequestMode::Unknown,
            request_provider_label: None,
            official_session_preserved: Some(true),
            pending_restart,
            created_at: now,
            updated_at: now,
        })
    }
}

fn purpose_for_login(
    request: &StartManagedAuthLoginRequest,
) -> (CredentialPurpose, Option<ManagedAuthConsumer>) {
    match request.purpose {
        ManagedAuthLoginPurpose::ConnectConsumer
            if request.consumer == Some(ManagedAuthConsumer::Codex) =>
        {
            (
                CredentialPurpose::CodexNative,
                Some(ManagedAuthConsumer::Codex),
            )
        }
        ManagedAuthLoginPurpose::ConnectConsumer
            if request.consumer == Some(ManagedAuthConsumer::Opencode) =>
        {
            (
                CredentialPurpose::OpencodeProvider,
                Some(ManagedAuthConsumer::Opencode),
            )
        }
        ManagedAuthLoginPurpose::Reauthenticate => match request.consumer {
            Some(ManagedAuthConsumer::Codex) => (
                CredentialPurpose::CodexNative,
                Some(ManagedAuthConsumer::Codex),
            ),
            Some(ManagedAuthConsumer::Opencode) => (
                CredentialPurpose::OpencodeProvider,
                Some(ManagedAuthConsumer::Opencode),
            ),
            _ => (
                CredentialPurpose::ProxyUpstream,
                Some(ManagedAuthConsumer::FyagentProxy),
            ),
        },
        _ => (
            CredentialPurpose::ProxyUpstream,
            Some(ManagedAuthConsumer::FyagentProxy),
        ),
    }
}

fn bind_port(port: u16) -> std::io::Result<super::providers::openai::LoopbackListener> {
    super::providers::openai::bind_loopback_port(port)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;
    use crate::services::managed_auth::ManagedAuthConnectionState;
    use crate::services::secret::{MemorySecretBackend, SecretService};
    use axum::extract::Form;
    use axum::http::StatusCode;
    use axum::routing::post;
    use axum::{Json, Router};
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
    use std::collections::HashMap;
    use std::sync::atomic::Ordering;
    use tempfile::tempdir;

    fn jwt(payload: &str) -> String {
        let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"none"}"#);
        let body = URL_SAFE_NO_PAD.encode(payload.as_bytes());
        format!("{header}.{body}.sig")
    }

    fn token_json() -> serde_json::Value {
        let access = jwt(
            r#"{"chatgpt_account_id":"acct-login-1","email":"person@example.com","organizations":[{"id":"ws-1"}]}"#,
        );
        serde_json::json!({
            "access_token": access,
            "refresh_token": "refresh-login",
            "id_token": access,
            "expires_in": 3600
        })
    }

    async fn spawn_issuer(pending_polls: u32) -> String {
        let remaining = Arc::new(AtomicU32::new(pending_polls));
        let app = Router::new()
            .route(
                "/api/accounts/deviceauth/usercode",
                post(|| async {
                    Json(serde_json::json!({
                        "device_auth_id": "device-auth-1",
                        "user_code": "ABCD-EFGH",
                        "expires_in": 120,
                        "interval": 1
                    }))
                }),
            )
            .route(
                "/api/accounts/deviceauth/token",
                post({
                    let remaining = Arc::clone(&remaining);
                    move || {
                        let remaining = Arc::clone(&remaining);
                        async move {
                            if remaining.fetch_sub(1, Ordering::SeqCst) > 0 {
                                return (
                                    StatusCode::FORBIDDEN,
                                    Json(serde_json::json!({"error": "authorization_pending"})),
                                );
                            }
                            (
                                StatusCode::OK,
                                Json(serde_json::json!({
                                    "authorization_code": "one-time-code",
                                    "code_verifier": "device-verifier"
                                })),
                            )
                        }
                    }
                }),
            )
            .route(
                "/oauth/token",
                post(|Form(_form): Form<HashMap<String, String>>| async { Json(token_json()) }),
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

    fn silent_browser(_: &str) -> Result<(), ErrorKind> {
        Ok(())
    }

    async fn wait_terminal(
        service: &ManagedAuthService<MemorySecretBackend>,
        session_id: &str,
    ) -> ManagedAuthLoginSessionSnapshot {
        for _ in 0..80 {
            let snapshot = service.get_login_session(session_id).expect("session");
            if snapshot.terminal {
                return snapshot;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        panic!("login session did not terminate");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn device_code_saves_account_without_leaking_device_auth_id() {
        let issuer = spawn_issuer(0).await;
        let (service, _dir) = service();
        service.set_login_hooks(LoginHooks {
            endpoints: OpenAiOAuthEndpoints::for_issuer(&issuer),
            open_browser: silent_browser,
            occupy_preferred: true,
            occupy_fallback: true,
            bind_ephemeral: false,
            fixed_state: None,
            fixed_pkce: None,
            bound_port: Arc::new(std::sync::Mutex::new(None)),
            open_count: Arc::new(AtomicU32::new(0)),
        });
        let snapshot = service
            .start_login(StartManagedAuthLoginRequest {
                provider: ManagedAuthProvider::Openai,
                purpose: ManagedAuthLoginPurpose::SaveOnly,
                consumer: None,
                method: ManagedAuthLoginMethod::BrowserLoopback,
                account_id: None,
            })
            .expect("start");
        assert_eq!(snapshot.method, ManagedAuthLoginMethod::DeviceCode);
        assert!(snapshot.can_cancel);
        assert!(!snapshot.can_switch_to_device_code);
        let finished = wait_terminal(&service, &snapshot.session_id).await;
        assert_eq!(finished.stage, ManagedAuthLoginStage::Completed);
        assert!(finished.account_id.is_some());
        assert!(finished.reason_code.is_none());
        let json = serde_json::to_string(&finished)
            .unwrap()
            .to_ascii_lowercase();
        for forbidden in [
            "device-auth-1",
            "device_auth_id",
            "refresh-login",
            "one-time-code",
            "device-verifier",
            "secretref",
            "access_token",
        ] {
            assert!(!json.contains(forbidden), "leaked {forbidden}");
        }
        let overview = serde_json::to_value(service.overview()).unwrap();
        assert_eq!(overview["providers"][0]["available"], true);
        assert_eq!(overview["accounts"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn purpose_for_login_isolates_opencode_from_proxy_and_codex() {
        let request = StartManagedAuthLoginRequest {
            provider: ManagedAuthProvider::Openai,
            purpose: ManagedAuthLoginPurpose::ConnectConsumer,
            consumer: Some(ManagedAuthConsumer::Opencode),
            method: ManagedAuthLoginMethod::DeviceCode,
            account_id: None,
        };
        assert_eq!(
            purpose_for_login(&request),
            (
                CredentialPurpose::OpencodeProvider,
                Some(ManagedAuthConsumer::Opencode)
            )
        );
        let proxy = StartManagedAuthLoginRequest {
            consumer: None,
            purpose: ManagedAuthLoginPurpose::SaveOnly,
            ..request
        };
        assert_eq!(
            purpose_for_login(&proxy),
            (
                CredentialPurpose::ProxyUpstream,
                Some(ManagedAuthConsumer::FyagentProxy)
            )
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn device_code_connects_opencode_with_independent_session_and_pending_restart() {
        let issuer = spawn_issuer(0).await;
        let (service, dir) = service();
        service.set_login_hooks(LoginHooks {
            endpoints: OpenAiOAuthEndpoints::for_issuer(&issuer),
            open_browser: silent_browser,
            occupy_preferred: true,
            occupy_fallback: true,
            bind_ephemeral: false,
            fixed_state: None,
            fixed_pkce: None,
            bound_port: Arc::new(std::sync::Mutex::new(None)),
            open_count: Arc::new(AtomicU32::new(0)),
        });
        let snapshot = service
            .start_login(StartManagedAuthLoginRequest {
                provider: ManagedAuthProvider::Openai,
                purpose: ManagedAuthLoginPurpose::ConnectConsumer,
                consumer: Some(ManagedAuthConsumer::Opencode),
                method: ManagedAuthLoginMethod::BrowserLoopback,
                account_id: None,
            })
            .expect("start");
        let finished = wait_terminal(&service, &snapshot.session_id).await;
        assert_eq!(finished.stage, ManagedAuthLoginStage::Completed);
        assert_eq!(
            finished.reason_code,
            Some(ManagedAuthReasonCode::PendingRestart)
        );
        let rows = service.repository.list_all_credentials().expect("rows");
        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows[0].credential.purpose,
            CredentialPurpose::OpencodeProvider
        );
        assert_eq!(
            rows[0].credential.consumer,
            Some(ManagedAuthConsumer::Opencode)
        );
        assert_eq!(rows[0].credential.refresh_owner, RefreshOwner::Opencode);
        let raw: serde_json::Value = serde_json::from_slice(
            &std::fs::read(dir.path().join("opencode-data").join("auth.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(raw["openai"]["type"], "oauth");
        assert_eq!(raw["openai"]["refresh"], "refresh-login");
        let openai = service
            .overview()
            .connections
            .into_iter()
            .find(|row| {
                row.consumer == ManagedAuthConsumer::Opencode
                    && row.provider == Some(ManagedAuthProvider::Openai)
            })
            .expect("slot");
        assert_eq!(
            openai.auth_status,
            ManagedAuthConnectionState::PendingRestart
        );
        assert!(openai.pending_restart);
        assert!(openai
            .reason_codes
            .contains(&ManagedAuthReasonCode::PendingRestart));
        assert!(!openai
            .reason_codes
            .contains(&ManagedAuthReasonCode::NativeProjectionUnavailable));
        let json = serde_json::to_string(&finished)
            .unwrap()
            .to_ascii_lowercase();
        for forbidden in [
            "refresh-login",
            "device-auth-1",
            "secretref",
            "access_token",
        ] {
            assert!(!json.contains(forbidden), "leaked {forbidden}");
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn browser_callback_saves_separate_codex_purpose_as_partial() {
        let issuer = spawn_issuer(0).await;
        let (service, _dir) = service();
        let state = "fixed-state-value-32bytes-aaaa".to_string();
        let bound_port = Arc::new(std::sync::Mutex::new(None));
        service.set_login_hooks(LoginHooks {
            endpoints: OpenAiOAuthEndpoints::for_issuer(&issuer),
            open_browser: silent_browser,
            occupy_preferred: false,
            occupy_fallback: false,
            bind_ephemeral: true,
            fixed_state: Some(state.clone()),
            fixed_pkce: Some(generate_pkce()),
            bound_port: Arc::clone(&bound_port),
            open_count: Arc::new(AtomicU32::new(0)),
        });
        let snapshot = service
            .start_login(StartManagedAuthLoginRequest {
                provider: ManagedAuthProvider::Openai,
                purpose: ManagedAuthLoginPurpose::ConnectConsumer,
                consumer: Some(ManagedAuthConsumer::Codex),
                method: ManagedAuthLoginMethod::BrowserLoopback,
                account_id: None,
            })
            .expect("start");
        let mut port = None;
        for _ in 0..40 {
            port = *bound_port.lock().unwrap();
            if port.is_some() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        let port = port.expect("loopback bound");
        let url = format!("http://127.0.0.1:{port}/auth/callback?code=browser-code&state={state}");
        reqwest::get(url).await.expect("callback");
        let finished = wait_terminal(&service, &snapshot.session_id).await;
        assert_eq!(finished.stage, ManagedAuthLoginStage::Partial);
        assert_eq!(
            finished.reason_code,
            Some(ManagedAuthReasonCode::NativeProjectionUnavailable)
        );
        assert!(finished.account_id.is_some());
        let rows = service.repository.list_all_credentials().expect("rows");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].credential.purpose, CredentialPurpose::CodexNative);
        let json = serde_json::to_string(&finished)
            .unwrap()
            .to_ascii_lowercase();
        for forbidden in [
            "browser-code",
            "fixed-state-value-32bytes-aaaa",
            "code_verifier",
            "secretref",
            "access_token",
        ] {
            assert!(!json.contains(forbidden), "leaked {forbidden}");
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn reopen_browser_login_opens_official_page_again_without_leaking_url() {
        let issuer = spawn_issuer(0).await;
        let (service, _dir) = service();
        let state = "fixed-state-value-32bytes-bbbb".to_string();
        let bound_port = Arc::new(std::sync::Mutex::new(None));
        let open_count = Arc::new(AtomicU32::new(0));
        service.set_login_hooks(LoginHooks {
            endpoints: OpenAiOAuthEndpoints::for_issuer(&issuer),
            open_browser: silent_browser,
            occupy_preferred: false,
            occupy_fallback: false,
            bind_ephemeral: true,
            fixed_state: Some(state.clone()),
            fixed_pkce: Some(generate_pkce()),
            bound_port: Arc::clone(&bound_port),
            open_count: Arc::clone(&open_count),
        });
        let snapshot = service
            .start_login(StartManagedAuthLoginRequest {
                provider: ManagedAuthProvider::Openai,
                purpose: ManagedAuthLoginPurpose::SaveOnly,
                consumer: None,
                method: ManagedAuthLoginMethod::BrowserLoopback,
                account_id: None,
            })
            .expect("start");
        for _ in 0..40 {
            if bound_port.lock().unwrap().is_some() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        assert!(bound_port.lock().unwrap().is_some(), "loopback bound");
        for _ in 0..40 {
            if open_count.load(Ordering::SeqCst) >= 1 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        assert!(open_count.load(Ordering::SeqCst) >= 1);
        let reopened = service.reopen_login(&snapshot.session_id).expect("reopen");
        assert!(!reopened.terminal);
        assert_eq!(open_count.load(Ordering::SeqCst), 2);
        let json = serde_json::to_string(&reopened)
            .unwrap()
            .to_ascii_lowercase();
        assert!(!json.contains("/oauth/authorize"));
        assert!(!json.contains("code_challenge"));
        assert!(!json.contains(&state.to_ascii_lowercase()));
        service.cancel_login(&snapshot.session_id).expect("cancel");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn cancel_drops_late_device_result() {
        let issuer = spawn_issuer(8).await;
        let (service, _dir) = service();
        service.set_login_hooks(LoginHooks {
            endpoints: OpenAiOAuthEndpoints::for_issuer(&issuer),
            open_browser: silent_browser,
            occupy_preferred: true,
            occupy_fallback: true,
            bind_ephemeral: false,
            fixed_state: None,
            fixed_pkce: None,
            bound_port: Arc::new(std::sync::Mutex::new(None)),
            open_count: Arc::new(AtomicU32::new(0)),
        });
        let snapshot = service
            .start_login(StartManagedAuthLoginRequest {
                provider: ManagedAuthProvider::Openai,
                purpose: ManagedAuthLoginPurpose::SaveOnly,
                consumer: None,
                method: ManagedAuthLoginMethod::DeviceCode,
                account_id: None,
            })
            .expect("start");
        tokio::time::sleep(Duration::from_millis(30)).await;
        let cancelled = service.cancel_login(&snapshot.session_id).expect("cancel");
        assert_eq!(cancelled.stage, ManagedAuthLoginStage::Cancelled);
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert!(service
            .repository
            .list_all_credentials()
            .unwrap()
            .is_empty());
        let again = service.get_login_session(&snapshot.session_id).unwrap();
        assert_eq!(again.stage, ManagedAuthLoginStage::Cancelled);
    }

    #[test]
    fn same_identity_can_hold_proxy_and_codex_sessions() {
        let (service, _dir) = service();
        let proxy = service
            .provision_legacy_credential(LegacyCredentialInput {
                migration_id: None,
                provider: ManagedAuthProvider::Openai,
                purpose: CredentialPurpose::ProxyUpstream,
                consumer: Some(ManagedAuthConsumer::FyagentProxy),
                legacy_account_id: "proxy_upstream:acct-1".into(),
                provider_subject: "acct-1".into(),
                provider_tenant: "ws-1".into(),
                login: "person@example.com".into(),
                display_name: None,
                avatar_url: None,
                access_token: None,
                refresh_token: Some(Zeroizing::new("proxy-refresh".into())),
                id_token: None,
                desired_status: CredentialStatus::Ready,
                refresh_owner: RefreshOwner::Fyagent,
                authenticated_at: 1_700_000_000,
                make_default: true,
            })
            .expect("proxy");
        let codex = service
            .provision_legacy_credential(LegacyCredentialInput {
                migration_id: None,
                provider: ManagedAuthProvider::Openai,
                purpose: CredentialPurpose::CodexNative,
                consumer: Some(ManagedAuthConsumer::Codex),
                legacy_account_id: "codex_native:acct-1".into(),
                provider_subject: "acct-1".into(),
                provider_tenant: "ws-1".into(),
                login: "person@example.com".into(),
                display_name: None,
                avatar_url: None,
                access_token: None,
                refresh_token: Some(Zeroizing::new("codex-refresh".into())),
                id_token: None,
                desired_status: CredentialStatus::Ready,
                refresh_owner: RefreshOwner::Fyagent,
                authenticated_at: 1_700_000_000,
                make_default: true,
            })
            .expect("codex");
        assert_eq!(proxy.identity_id, codex.identity_id);
        assert_ne!(proxy.credential_id, codex.credential_id);
        let overview = service.overview();
        assert_eq!(overview.accounts.len(), 1);
        assert_eq!(overview.accounts[0].connected_consumer_count, 1);
    }
}
