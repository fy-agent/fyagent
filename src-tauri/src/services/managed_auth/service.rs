use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use chrono::SecondsFormat;
use tauri::Manager;
use zeroize::Zeroizing;

use crate::database::Database;
use crate::proxy::providers::copilot_auth::exchange_github_token_for_copilot;
use crate::proxy::providers::xai_oauth_auth::XaiOAuthManager;
use crate::services::managed_auth::login::LoginHooks;
use crate::services::managed_auth::login_sessions::LoginSessionStore;
use crate::services::managed_auth::providers::openai;
use crate::services::managed_auth::providers::xai::XaiLoginHooks;
use crate::services::secret::{
    DecodeSecret, SecretAvailability, SecretBackend, SecretErrorCode, SecretHandle, SecretPresence,
    SecretPurpose, SecretRef, SecretService, SecretVersion,
};

use super::consumers::opencode::{self, ProjectionEntry};
use super::migration::{
    self, LegacyCredentialInput, CODEX_MIGRATION_ID, COPILOT_MIGRATION_ID, XAI_MIGRATION_ID,
};
use super::{
    now_timestamp, stable_connection_id, stable_credential_id, stable_identity_id, stable_revision,
    ConnectionRecord, ConnectionStatus, CredentialPurpose, CredentialRecord, CredentialStatus,
    CredentialWithIdentity, IdentityRecord, ManagedAuthAccountAction,
    ManagedAuthAccountRemovalImpact, ManagedAuthAccountRemovalPreview, ManagedAuthAccountSummary,
    ManagedAuthConnectionAction, ManagedAuthConnectionActionRequest, ManagedAuthConnectionState,
    ManagedAuthConnectionSummary, ManagedAuthConsumer, ManagedAuthCoreError,
    ManagedAuthCredentialManager, ManagedAuthErrorDto, ManagedAuthHealth, ManagedAuthLoginMethod,
    ManagedAuthLoginStage, ManagedAuthMutationOutcome, ManagedAuthMutationResult,
    ManagedAuthOverview, ManagedAuthProvider, ManagedAuthProviderSummary, ManagedAuthReasonCode,
    ManagedAuthRepository, ManagedAuthRequestMode, ManagedAuthSecretBundle,
    ManagedAuthSecretBundleParts, MigrationStatus, NewCredential, RefreshOwner,
    MANAGED_AUTH_CONTRACT_VERSION,
};

pub(crate) type NativeManagedAuthService =
    ManagedAuthService<crate::services::secret::NativeSecretBackend>;

pub(crate) struct AccessMaterial {
    access_token: Zeroizing<String>,
    routing_subject: Option<String>,
}

impl AccessMaterial {
    pub(crate) fn access_token(&self) -> &str {
        self.access_token.as_str()
    }

    pub(crate) fn routing_subject(&self) -> Option<&str> {
        self.routing_subject.as_deref()
    }
}

struct RefreshedGrant {
    access_token: String,
    refresh_token: Option<String>,
    id_token: Option<String>,
    expires_in: Option<i64>,
}

impl std::fmt::Debug for AccessMaterial {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AccessMaterial")
            .field("access_token", &"<redacted>")
            .field("routing_subject", &self.routing_subject)
            .finish()
    }
}

#[derive(Clone, Debug)]
pub(crate) struct CompatibilityAccount {
    pub(crate) id: String,
    pub(crate) login: String,
    pub(crate) avatar_url: Option<String>,
    pub(crate) authenticated_at: i64,
    pub(crate) is_default: bool,
    pub(crate) requires_reauth: bool,
    pub(crate) chatgpt_account_id: Option<String>,
    pub(crate) github_domain: String,
}

pub(crate) struct FailClosedState {
    pub(crate) secret_unavailable: bool,
    pub(crate) migration_blocked: bool,
}

struct RefreshCoordinator {
    locks: Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>,
}

impl RefreshCoordinator {
    fn new() -> Self {
        Self {
            locks: Mutex::new(HashMap::new()),
        }
    }

    fn lock_for(&self, credential_id: &str) -> Arc<tokio::sync::Mutex<()>> {
        let mut locks = self.locks.lock().unwrap_or_else(|error| error.into_inner());
        locks
            .entry(credential_id.to_string())
            .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
            .clone()
    }
}

pub(crate) struct ManagedAuthService<B>
where
    B: SecretBackend,
{
    pub(crate) repository: ManagedAuthRepository,
    secrets: SecretService<B>,
    config_dir: PathBuf,
    refresh: RefreshCoordinator,
    fail_closed: Mutex<FailClosedState>,
    pub(crate) login_sessions: LoginSessionStore,
    pub(crate) login_hooks: Mutex<LoginHooks>,
    pub(crate) xai_hooks: Mutex<XaiLoginHooks>,
    app_handle: Mutex<Option<tauri::AppHandle>>,
}

impl<B> ManagedAuthService<B>
where
    B: SecretBackend,
{
    pub(crate) fn new(db: Arc<Database>, secrets: SecretService<B>, config_dir: PathBuf) -> Self {
        Self {
            repository: ManagedAuthRepository::new(db),
            secrets,
            config_dir,
            refresh: RefreshCoordinator::new(),
            fail_closed: Mutex::new(FailClosedState {
                secret_unavailable: false,
                migration_blocked: false,
            }),
            login_sessions: LoginSessionStore::default(),
            login_hooks: Mutex::new(LoginHooks::default()),
            xai_hooks: Mutex::new(XaiLoginHooks::default()),
            app_handle: Mutex::new(None),
        }
    }

    pub(crate) fn attach_app_handle(&self, handle: tauri::AppHandle) {
        *self
            .app_handle
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = Some(handle);
    }

    pub(crate) fn with_app_state<R>(
        &self,
        callback: impl FnOnce(&crate::store::AppState) -> R,
    ) -> Option<R> {
        let guard = self
            .app_handle
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let handle = guard.as_ref()?;
        let state = handle.try_state::<crate::store::AppState>()?;
        Some(callback(&state))
    }

    pub(crate) fn repository(&self) -> &ManagedAuthRepository {
        &self.repository
    }

    #[cfg(test)]
    pub(crate) fn set_login_hooks(&self, hooks: LoginHooks) {
        *self
            .login_hooks
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = hooks;
    }

    #[cfg(test)]
    pub(crate) fn set_xai_login_hooks(&self, hooks: XaiLoginHooks) {
        *self
            .xai_hooks
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = hooks;
    }

    pub(crate) fn startup(&self) -> Result<(), ManagedAuthCoreError> {
        if !self.secret_backend_ready() {
            // Vault is not usable. Leave plaintext JSON as the live store;
            // do not seal or rename it, and do not claim a migration ran.
            self.set_fail_closed(true, false);
            return Ok(());
        }
        if let Err(error) = self.recover_credentials() {
            log::warn!("[ManagedAuth] credential recovery failed: {error}");
            self.set_fail_closed(false, true);
        }
        match migration::prepare_legacy_stores(self, &self.config_dir) {
            Ok(()) => {}
            Err(ManagedAuthCoreError::SecretUnavailable) => {
                self.set_fail_closed(true, true);
            }
            Err(error) => {
                log::warn!("[ManagedAuth] legacy store migration blocked: {error}");
                self.set_fail_closed(false, true);
            }
        }
        if let Err(error) = self.finalize_prepared_sources() {
            log::warn!("[ManagedAuth] legacy store finalize deferred: {error}");
            if matches!(
                error,
                ManagedAuthCoreError::SecretUnavailable | ManagedAuthCoreError::SecretMissing
            ) {
                self.set_fail_closed(true, true);
            }
        }
        if let Err(error) = self.upsert_proxy_connections() {
            log::warn!("[ManagedAuth] proxy connection projection failed: {error}");
        }
        Ok(())
    }

    pub(crate) fn legacy_store_sealed(&self, migration_id: &str) -> bool {
        // Seal the plaintext JSON only after this source is in the vault.
        // A blocked Copilot v1 file or a locked keychain must not freeze
        // unrelated stores or disable the still-authoritative JSON path.
        match self.repository.get_migration(migration_id) {
            Ok(Some(record)) => matches!(
                record.status,
                MigrationStatus::Prepared | MigrationStatus::Completed
            ),
            Ok(None) | Err(_) => false,
        }
    }

    pub(crate) fn provision_legacy_credential(
        &self,
        input: LegacyCredentialInput,
    ) -> Result<CredentialRecord, ManagedAuthCoreError> {
        let lock = self.refresh.lock_for(&stable_credential_id(
            input.provider,
            input.purpose,
            input.consumer,
            &input.legacy_account_id,
        ));
        let _guard = lock.blocking_lock();
        self.provision_legacy_credential_locked(input)
    }

    pub(crate) fn recover_credentials(&self) -> Result<(), ManagedAuthCoreError> {
        let recoverable = self.repository.list_recoverable_credentials()?;
        for credential in recoverable {
            let lock = self.refresh.lock_for(&credential.credential_id);
            let _guard = lock.blocking_lock();
            self.recover_one(&credential)?;
        }
        Ok(())
    }

    pub(crate) fn overview(&self) -> ManagedAuthOverview {
        match self.overview_inner() {
            Ok(overview) => overview,
            Err(error) => {
                log::warn!("[ManagedAuth] overview failed: {error}");
                let mut overview = ManagedAuthOverview::unavailable();
                overview.reason_codes = vec![error.reason_code()];
                overview
            }
        }
    }

    pub(crate) async fn resolve_access_material(
        &self,
        provider: ManagedAuthProvider,
        legacy_account_id: Option<&str>,
    ) -> Result<AccessMaterial, ManagedAuthCoreError> {
        let (purpose, consumer) = purpose_for_provider(provider);
        let credential = match legacy_account_id {
            Some(legacy_id) => self
                .repository
                .get_credential_by_legacy(provider, purpose, consumer, legacy_id)?
                .ok_or(ManagedAuthCoreError::NotFound)?,
            None => {
                let default_id = self
                    .repository
                    .get_default(provider, purpose, consumer)?
                    .ok_or(ManagedAuthCoreError::NotFound)?;
                self.repository
                    .get_credential(&default_id)?
                    .ok_or(ManagedAuthCoreError::NotFound)?
            }
        };
        self.resolve_credential_access(credential).await
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn replace_bundle_cas(
        &self,
        credential_id: &str,
        expected_generation: u64,
        expected_owner: RefreshOwner,
        bundle: ManagedAuthSecretBundle,
    ) -> Result<bool, ManagedAuthCoreError> {
        let lock = self.refresh.lock_for(credential_id);
        let _guard = lock.blocking_lock();
        self.replace_bundle_cas_locked(credential_id, expected_generation, expected_owner, bundle)
    }

    pub(crate) fn set_default_account(
        &self,
        account_id: &str,
        expected_revision: &str,
    ) -> Result<ManagedAuthMutationResult, ManagedAuthCoreError> {
        let rows = self.credentials_for_account(account_id)?;
        let Some(selected) = rows
            .iter()
            .find(|row| row.credential.status == CredentialStatus::Ready)
        else {
            return Err(ManagedAuthCoreError::NotFound);
        };
        if account_revision(selected) != expected_revision {
            return Err(ManagedAuthCoreError::Stale);
        }
        let now = chrono::Utc::now().timestamp();
        if !self.repository.set_default(
            selected.credential.provider,
            selected.credential.purpose,
            selected.credential.consumer,
            &selected.credential.credential_id,
            now,
        )? {
            return Err(ManagedAuthCoreError::Conflict);
        }
        Ok(self.mutation_result(ManagedAuthMutationOutcome::Completed, None))
    }

    pub(crate) fn preview_account_removal(
        &self,
        account_id: &str,
        expected_revision: &str,
    ) -> Result<ManagedAuthAccountRemovalPreview, ManagedAuthCoreError> {
        let rows = self.credentials_for_account(account_id)?;
        let Some(selected) = rows.first() else {
            return Err(ManagedAuthCoreError::NotFound);
        };
        let actual_revision = account_revision(selected);
        if actual_revision != expected_revision {
            return Err(ManagedAuthCoreError::Stale);
        }
        let connections = self.repository.list_connections()?;
        let credential_ids: Vec<&str> = rows
            .iter()
            .map(|row| row.credential.credential_id.as_str())
            .collect();
        let disconnects = connections
            .iter()
            .filter(|connection| {
                connection
                    .credential_id
                    .as_deref()
                    .is_some_and(|id| credential_ids.contains(&id))
            })
            .map(|connection| ManagedAuthAccountRemovalImpact {
                consumer: connection.consumer,
                target_label: connection.request_provider_label.clone(),
                request_mode: connection.request_mode,
            })
            .collect::<Vec<_>>();
        Ok(ManagedAuthAccountRemovalPreview {
            contract_version: MANAGED_AUTH_CONTRACT_VERSION,
            preview_id: removal_preview_id(account_id, expected_revision),
            account_id: account_id.to_string(),
            expected_revision: expected_revision.to_string(),
            disconnects,
            preserved: Vec::new(),
            can_apply: true,
            reason_codes: Vec::new(),
        })
    }

    pub(crate) fn remove_account(
        &self,
        preview_id: &str,
        account_id: &str,
        expected_revision: &str,
    ) -> Result<ManagedAuthMutationResult, ManagedAuthCoreError> {
        let preview = self.preview_account_removal(account_id, expected_revision)?;
        if preview.preview_id != preview_id {
            return Err(ManagedAuthCoreError::Stale);
        }
        let rows = self.credentials_for_account(account_id)?;
        for row in rows {
            self.remove_credential_record(&row.credential)?;
        }
        Ok(self.mutation_result(ManagedAuthMutationOutcome::Completed, None))
    }

    pub(crate) fn apply_opencode_connection_action(
        &self,
        request: &ManagedAuthConnectionActionRequest,
    ) -> Result<ManagedAuthMutationResult, ManagedAuthErrorDto> {
        let provider = opencode::slot_for_connection_id(&request.connection_id)
            .ok_or_else(ManagedAuthErrorDto::unavailable)?;
        match request.action {
            ManagedAuthConnectionAction::Refresh => {
                Ok(self.mutation_result(ManagedAuthMutationOutcome::Completed, None))
            }
            ManagedAuthConnectionAction::Disconnect => {
                self.opencode_disconnect(provider, &request.expected_revision)
            }
            ManagedAuthConnectionAction::ConnectAccount
            | ManagedAuthConnectionAction::SwitchAccount => {
                self.opencode_connect(provider, request)
            }
            ManagedAuthConnectionAction::Restart => {
                self.opencode_ack_restart(provider, &request.expected_revision)
            }
            _ => Err(ManagedAuthErrorDto::unavailable()),
        }
    }

    pub(crate) fn compatibility_accounts(
        &self,
        provider: ManagedAuthProvider,
    ) -> Result<Vec<CompatibilityAccount>, ManagedAuthCoreError> {
        let (purpose, consumer) = purpose_for_provider(provider);
        let rows = self
            .repository
            .list_credentials(provider, purpose, consumer)?;
        Ok(rows
            .into_iter()
            .filter(|row| {
                matches!(
                    row.credential.status,
                    CredentialStatus::Ready | CredentialStatus::RequiresReauth
                )
            })
            .map(|row| CompatibilityAccount {
                id: row.credential.legacy_account_id.clone(),
                login: row.identity.login.clone(),
                avatar_url: row.identity.avatar_url.clone(),
                authenticated_at: row.credential.authenticated_at,
                is_default: row.is_default,
                requires_reauth: row.credential.status == CredentialStatus::RequiresReauth,
                chatgpt_account_id: (provider == ManagedAuthProvider::Openai)
                    .then(|| row.identity.provider_subject.clone()),
                github_domain: if provider == ManagedAuthProvider::GithubCopilot {
                    row.identity.provider_tenant.clone()
                } else {
                    "github.com".to_string()
                },
            })
            .collect())
    }

    pub(crate) fn has_legacy_credential(
        &self,
        provider: ManagedAuthProvider,
        legacy_account_id: &str,
    ) -> bool {
        let (purpose, consumer) = purpose_for_provider(provider);
        self.repository
            .get_credential_by_legacy(provider, purpose, consumer, legacy_account_id)
            .ok()
            .flatten()
            .is_some_and(|credential| {
                matches!(
                    credential.status,
                    CredentialStatus::Ready | CredentialStatus::RequiresReauth
                )
            })
    }

    fn provision_legacy_credential_locked(
        &self,
        input: LegacyCredentialInput,
    ) -> Result<CredentialRecord, ManagedAuthCoreError> {
        let now = chrono::Utc::now().timestamp();
        let identity_id = stable_identity_id(
            input.provider,
            &input.provider_subject,
            &input.provider_tenant,
        );
        let credential_id = stable_credential_id(
            input.provider,
            input.purpose,
            input.consumer,
            &input.legacy_account_id,
        );
        let reserved = self.secrets.reserve();
        let desired_status = input.desired_status;
        let make_default = input.make_default;
        let new_credential = NewCredential {
            identity: IdentityRecord {
                identity_id: identity_id.clone(),
                provider: input.provider,
                provider_subject: input.provider_subject.clone(),
                provider_tenant: input.provider_tenant.clone(),
                login: input.login.clone(),
                display_name: input.display_name.clone(),
                avatar_url: input.avatar_url.clone(),
                created_at: input.authenticated_at,
                updated_at: now,
            },
            credential: CredentialRecord {
                credential_id: credential_id.clone(),
                identity_id,
                provider: input.provider,
                purpose: input.purpose,
                consumer: input.consumer,
                legacy_account_id: input.legacy_account_id.clone(),
                secret_handle: reserved.clone(),
                refresh_owner: input.refresh_owner,
                generation: 1,
                access_expires_at: None,
                status: CredentialStatus::Provisioning,
                authenticated_at: input.authenticated_at,
                refreshed_at: None,
                migration_id: input.migration_id.map(str::to_string),
                created_at: now,
                updated_at: now,
            },
        };
        let stored_res = self.repository.begin_provisioning(&new_credential);
        let stored = stored_res?;
        let handle = stored.secret_handle.clone();
        let bundle_res = ManagedAuthSecretBundle::new(ManagedAuthSecretBundleParts {
            credential_id: stored.credential_id.clone(),
            provider: stored.provider,
            generation: stored.generation,
            access_token: input.access_token.map(|value| value.to_string()),
            refresh_token: input.refresh_token.map(|value| value.to_string()),
            id_token: input.id_token.map(|value| value.to_string()),
            token_type: None,
            granted_scopes: Vec::new(),
            issued_at: Some(input.authenticated_at),
            expires_at: None,
        });
        let bundle = bundle_res?;
        let encoded_res = bundle.encode();
        let create_res = self.secrets.create_reserved(
            &handle,
            encoded_res?,
            SecretPurpose::ManagedOAuthCredential,
        );
        match create_res {
            Ok(_) => {}
            Err(error) if error.code() == SecretErrorCode::AlreadyExists => {}
            Err(error) if error.code() == SecretErrorCode::Missing => {
                self.repository.set_status(
                    &stored.credential_id,
                    CredentialStatus::SecretMissing,
                    now,
                )?;
                return Err(error.into());
            }
            Err(error)
                if matches!(
                    error.code(),
                    SecretErrorCode::BackendUnavailable
                        | SecretErrorCode::Locked
                        | SecretErrorCode::PermissionDenied
                ) =>
            {
                // create_reserved never wrote an item. Do not mark
                // migration_blocked: that health hid Remove and trapped
                // login leftovers.
                self.repository.set_status(
                    &stored.credential_id,
                    CredentialStatus::SecretMissing,
                    now,
                )?;
                return Err(ManagedAuthCoreError::SecretUnavailable);
            }
            Err(error) => {
                return Err(error.into());
            }
        }
        let readback_res = self.readback_bundle(&handle);
        readback_res?;
        let marked_res = self.repository.mark_ready(
            &stored.credential_id,
            stored.generation,
            handle.version(),
            desired_status,
            now,
        );
        let marked = marked_res?;
        if !marked {
            return Err(ManagedAuthCoreError::Stale);
        }
        if make_default && desired_status == CredentialStatus::Ready {
            let _ = self.repository.set_default(
                stored.provider,
                stored.purpose,
                stored.consumer,
                &stored.credential_id,
                now,
            )?;
        }
        self.repository
            .get_credential(&stored.credential_id)?
            .ok_or(ManagedAuthCoreError::NotFound)
    }

    fn recover_one(&self, credential: &CredentialRecord) -> Result<(), ManagedAuthCoreError> {
        let now = chrono::Utc::now().timestamp();
        match self.secrets.probe(
            &credential.secret_handle,
            SecretPurpose::ManagedOAuthCredential,
        ) {
            Ok(summary)
                if summary.presence() == SecretPresence::Present
                    && summary.availability() == SecretAvailability::Ready =>
            {
                match self.readback_bundle(&credential.secret_handle) {
                    Ok(bundle) => {
                        if bundle.credential_id() != credential.credential_id
                            || bundle.provider() != credential.provider
                        {
                            return Err(ManagedAuthCoreError::InvalidData);
                        }
                        let desired = if credential.status == CredentialStatus::RequiresReauth {
                            CredentialStatus::RequiresReauth
                        } else {
                            CredentialStatus::Ready
                        };
                        let _ = self.repository.mark_ready(
                            &credential.credential_id,
                            credential.generation,
                            credential.secret_handle.version(),
                            desired,
                            now,
                        )?;
                    }
                    Err(_) => {
                        // Native item exists but cannot be decoded. Keep the
                        // SecretRef; do not delete it.
                    }
                }
            }
            Ok(summary) if summary.presence() == SecretPresence::Missing => {
                self.repository.set_status(
                    &credential.credential_id,
                    CredentialStatus::SecretMissing,
                    now,
                )?;
            }
            Ok(_) | Err(_) => {
                self.repository.set_status(
                    &credential.credential_id,
                    CredentialStatus::MigrationBlocked,
                    now,
                )?;
                self.set_fail_closed(true, true);
            }
        }
        Ok(())
    }

    pub(super) fn readback_bundle(
        &self,
        handle: &SecretHandle,
    ) -> Result<ManagedAuthSecretBundle, ManagedAuthCoreError> {
        self.secrets
            .with_material(
                handle,
                SecretPurpose::ManagedOAuthCredential,
                DecodeSecret::new(ManagedAuthSecretBundle::decode),
            )?
            .map_err(|_| ManagedAuthCoreError::InvalidData)
    }

    pub(crate) fn replace_bundle_cas_locked(
        &self,
        credential_id: &str,
        expected_generation: u64,
        expected_owner: RefreshOwner,
        bundle: ManagedAuthSecretBundle,
    ) -> Result<bool, ManagedAuthCoreError> {
        let current = self
            .repository
            .get_credential(credential_id)?
            .ok_or(ManagedAuthCoreError::NotFound)?;
        if current.generation != expected_generation || current.refresh_owner != expected_owner {
            return Ok(false);
        }
        if bundle.generation() != expected_generation.saturating_add(1) {
            return Err(ManagedAuthCoreError::InvalidData);
        }
        let next_handle = current.secret_handle.rotate();
        let next_generation = bundle.generation();
        let access_expires_at = bundle.expires_at();
        self.secrets.replace_reserved(
            &current.secret_handle,
            &next_handle,
            bundle.encode()?,
            SecretPurpose::ManagedOAuthCredential,
        )?;
        let updated = self.repository.update_secret_cas(
            credential_id,
            expected_generation,
            expected_owner,
            next_generation,
            next_handle.version(),
            access_expires_at,
            CredentialStatus::Ready,
            chrono::Utc::now().timestamp(),
        )?;
        if !updated {
            let _ = self.recover_one(
                &self
                    .repository
                    .get_credential(credential_id)?
                    .ok_or(ManagedAuthCoreError::NotFound)?,
            );
            return Ok(false);
        }
        Ok(true)
    }

    pub(crate) async fn resolve_credential_access(
        &self,
        credential: CredentialRecord,
    ) -> Result<AccessMaterial, ManagedAuthCoreError> {
        if !matches!(
            credential.purpose,
            CredentialPurpose::ProxyUpstream | CredentialPurpose::Copilot
        ) {
            return Err(ManagedAuthCoreError::Conflict);
        }
        if credential.refresh_owner != RefreshOwner::Fyagent {
            return Err(ManagedAuthCoreError::Conflict);
        }
        let identity = self
            .repository
            .list_all_credentials()?
            .into_iter()
            .find(|row| row.credential.credential_id == credential.credential_id)
            .map(|row| row.identity)
            .ok_or(ManagedAuthCoreError::NotFound)?;
        let lock = self.refresh.lock_for(&credential.credential_id);
        let _guard = lock.lock().await;
        let current = self
            .repository
            .get_credential(&credential.credential_id)?
            .ok_or(ManagedAuthCoreError::NotFound)?;
        if current.refresh_owner != RefreshOwner::Fyagent {
            return Err(ManagedAuthCoreError::Conflict);
        }
        let bundle = self.readback_bundle(&current.secret_handle)?;
        if current.provider == ManagedAuthProvider::GithubCopilot {
            let github_token = bundle
                .access_token()
                .ok_or(ManagedAuthCoreError::SecretMissing)?;
            let domain = if identity.provider_tenant.is_empty() {
                "github.com"
            } else {
                identity.provider_tenant.as_str()
            };
            let exchanged = exchange_github_token_for_copilot(github_token, domain)
                .await
                .map_err(|_| ManagedAuthCoreError::InvalidData)?;
            return Ok(AccessMaterial {
                access_token: Zeroizing::new(exchanged.token),
                routing_subject: None,
            });
        }
        if let Some(access) = bundle.access_token() {
            if !access_expired(current.access_expires_at) {
                return Ok(AccessMaterial {
                    access_token: Zeroizing::new(access.to_string()),
                    routing_subject: Some(identity.provider_subject.clone()),
                });
            }
        }
        let refresh_token = bundle
            .refresh_token()
            .ok_or(ManagedAuthCoreError::SecretMissing)?
            .to_string();
        let refreshed = match current.provider {
            ManagedAuthProvider::Openai => {
                let token = openai::refresh_oauth_grant(&refresh_token)
                    .await
                    .map_err(map_refresh_error)?;
                RefreshedGrant {
                    access_token: token.access_token,
                    refresh_token: token.refresh_token,
                    id_token: token.id_token,
                    expires_in: token.expires_in,
                }
            }
            ManagedAuthProvider::Xai => {
                let token = XaiOAuthManager::refresh_oauth_grant(&refresh_token)
                    .await
                    .map_err(map_xai_refresh_error)?;
                RefreshedGrant {
                    access_token: token.access_token,
                    refresh_token: token.refresh_token,
                    id_token: token.id_token,
                    expires_in: token.expires_in,
                }
            }
            ManagedAuthProvider::GithubCopilot => {
                return Err(ManagedAuthCoreError::InvalidData);
            }
        };
        let latest = self
            .repository
            .get_credential(&current.credential_id)?
            .ok_or(ManagedAuthCoreError::NotFound)?;
        if latest.generation != current.generation {
            let latest_bundle = self.readback_bundle(&latest.secret_handle)?;
            let access = latest_bundle
                .access_token()
                .ok_or(ManagedAuthCoreError::SecretMissing)?;
            return Ok(AccessMaterial {
                access_token: Zeroizing::new(access.to_string()),
                routing_subject: Some(identity.provider_subject.clone()),
            });
        }
        let next_generation = current.generation.saturating_add(1);
        let next_refresh = refreshed.refresh_token.clone().unwrap_or(refresh_token);
        let expires_at = refreshed.expires_in.map(|seconds| {
            chrono::Utc::now()
                .timestamp()
                .saturating_add(seconds.max(0))
        });
        let next_bundle = ManagedAuthSecretBundle::new(ManagedAuthSecretBundleParts {
            credential_id: current.credential_id.clone(),
            provider: current.provider,
            generation: next_generation,
            access_token: Some(refreshed.access_token.clone()),
            refresh_token: Some(next_refresh),
            id_token: refreshed.id_token.clone(),
            token_type: Some("Bearer".to_string()),
            granted_scopes: Vec::new(),
            issued_at: Some(chrono::Utc::now().timestamp()),
            expires_at,
        })?;
        if !self.replace_bundle_cas_locked(
            &current.credential_id,
            current.generation,
            RefreshOwner::Fyagent,
            next_bundle,
        )? {
            let recovered = self
                .repository
                .get_credential(&current.credential_id)?
                .ok_or(ManagedAuthCoreError::NotFound)?;
            let recovered_bundle = self.readback_bundle(&recovered.secret_handle)?;
            let access = recovered_bundle
                .access_token()
                .ok_or(ManagedAuthCoreError::SecretMissing)?;
            return Ok(AccessMaterial {
                access_token: Zeroizing::new(access.to_string()),
                routing_subject: Some(identity.provider_subject.clone()),
            });
        }
        Ok(AccessMaterial {
            access_token: Zeroizing::new(refreshed.access_token),
            routing_subject: Some(identity.provider_subject),
        })
    }

    fn finalize_prepared_sources(&self) -> Result<(), ManagedAuthCoreError> {
        let mut blocked = false;
        for source in [CODEX_MIGRATION_ID, XAI_MIGRATION_ID, COPILOT_MIGRATION_ID] {
            if let Some(record) = self.repository.get_migration(source)? {
                if record.status == MigrationStatus::Prepared
                    || (record.status == MigrationStatus::Completed
                        && self.config_dir.join(source_filename(source)).exists())
                {
                    match migration::finalize_legacy_store(self, &self.config_dir, source) {
                        Ok(()) => {}
                        Err(
                            ManagedAuthCoreError::SecretUnavailable
                            | ManagedAuthCoreError::SecretMissing,
                        ) => {
                            return Err(ManagedAuthCoreError::SecretUnavailable);
                        }
                        Err(_) => blocked = true,
                    }
                }
            }
        }
        if blocked {
            Err(ManagedAuthCoreError::MigrationBlocked)
        } else {
            Ok(())
        }
    }

    pub(crate) fn upsert_proxy_connections(&self) -> Result<(), ManagedAuthCoreError> {
        let now = chrono::Utc::now().timestamp();
        let rows = self.repository.list_all_credentials()?;
        for provider in [
            ManagedAuthProvider::Openai,
            ManagedAuthProvider::Xai,
            ManagedAuthProvider::GithubCopilot,
        ] {
            let (purpose, consumer) = purpose_for_provider(provider);
            let selected = rows.iter().find(|row| {
                row.credential.provider == provider
                    && row.credential.purpose == purpose
                    && row.credential.consumer == consumer
                    && row.is_default
                    && row.credential.status == CredentialStatus::Ready
            });
            let Some(selected) = selected else {
                continue;
            };
            let connection_id =
                stable_connection_id(ManagedAuthConsumer::FyagentProxy, "", provider.as_str());
            self.repository.upsert_connection(&ConnectionRecord {
                connection_id: connection_id.clone(),
                consumer: ManagedAuthConsumer::FyagentProxy,
                target_id: String::new(),
                provider_slot: provider.as_str().to_string(),
                credential_id: Some(selected.credential.credential_id.clone()),
                desired_revision: stable_revision(&[&connection_id, "proxy"]),
                observed_revision: Some(stable_revision(&[&connection_id, "proxy"])),
                status: ConnectionStatus::Connected,
                request_mode: ManagedAuthRequestMode::OfficialSubscription,
                request_provider_label: Some(provider.as_str().to_string()),
                official_session_preserved: Some(true),
                pending_restart: false,
                created_at: now,
                updated_at: now,
            })?;
        }
        Ok(())
    }

    fn overview_inner(&self) -> Result<ManagedAuthOverview, ManagedAuthCoreError> {
        let fail = self.fail_closed_snapshot();
        let rows = self.repository.list_all_credentials()?;
        let connections = self.repository.list_connections()?;
        let mut accounts = Vec::new();
        let mut seen = Vec::new();
        for row in &rows {
            if seen.contains(&row.identity.identity_id) {
                continue;
            }
            seen.push(row.identity.identity_id.clone());
            accounts.push(account_summary(row, &rows, &connections, &fail));
        }
        let mut reason_codes = Vec::new();
        if fail.migration_blocked {
            reason_codes.push(ManagedAuthReasonCode::MigrationBlocked);
        }
        if fail.secret_unavailable {
            reason_codes.push(ManagedAuthReasonCode::SecretUnavailable);
        }
        let connection_summaries = merge_consumer_connections(
            &self.codex_home(),
            &self.opencode_auth_path(),
            &rows,
            &connections,
        );
        for account in &mut accounts {
            account.connected_consumer_count = connection_summaries
                .iter()
                .filter(|connection| {
                    connection.account_id.as_deref() == Some(account.account_id.as_str())
                })
                .map(|connection| connection.consumer)
                .collect::<std::collections::HashSet<_>>()
                .len();
        }
        Ok(ManagedAuthOverview {
            contract_version: MANAGED_AUTH_CONTRACT_VERSION,
            checked_at: now_timestamp(),
            providers: vec![
                provider_summary(ManagedAuthProvider::Openai, &fail),
                provider_summary(ManagedAuthProvider::Xai, &fail),
                provider_summary(ManagedAuthProvider::GithubCopilot, &fail),
            ],
            accounts,
            connections: connection_summaries,
            active_sessions: self.login_sessions.active_snapshots(),
            reason_codes,
        })
    }

    pub(crate) fn credentials_for_account(
        &self,
        account_id: &str,
    ) -> Result<Vec<CredentialWithIdentity>, ManagedAuthCoreError> {
        let rows = self
            .repository
            .list_all_credentials()?
            .into_iter()
            .filter(|row| row.identity.identity_id == account_id)
            .collect::<Vec<_>>();
        if rows.is_empty() {
            Err(ManagedAuthCoreError::NotFound)
        } else {
            Ok(rows)
        }
    }

    fn remove_credential_record(
        &self,
        credential: &CredentialRecord,
    ) -> Result<(), ManagedAuthCoreError> {
        // Delete OS vault material before SQLite rows. Identity ON DELETE
        // CASCADE only removes metadata; it cannot touch SecretRef items.
        match self.secrets.delete(&credential.secret_handle) {
            Ok(_) => {}
            Err(error) if error.code() == SecretErrorCode::Missing => {}
            Err(_error)
                if matches!(
                    credential.status,
                    CredentialStatus::SecretMissing
                        | CredentialStatus::Provisioning
                        | CredentialStatus::Revoked
                        | CredentialStatus::MigrationBlocked
                ) =>
            {
                // Vault is already unusable. errSecMissingEntitlement (-34018)
                // must not block SQLite cleanup, and must not rewrite status
                // (that would change revision and poison the preview).
                // Login leftovers marked migration_blocked never stored an item.
            }
            Err(_error) => {
                self.repository.set_status(
                    &credential.credential_id,
                    CredentialStatus::SecretMissing,
                    chrono::Utc::now().timestamp(),
                )?;
                return Err(ManagedAuthCoreError::SecretUnavailable);
            }
        }
        self.repository
            .delete_connections_for_credential(&credential.credential_id)?;
        self.repository
            .remove_credential(&credential.credential_id)?;
        Ok(())
    }

    pub(crate) fn mutation_result(
        &self,
        outcome: ManagedAuthMutationOutcome,
        reason_code: Option<ManagedAuthReasonCode>,
    ) -> ManagedAuthMutationResult {
        self.mutation_result_with(outcome, reason_code, Vec::new())
    }

    fn mutation_result_with(
        &self,
        outcome: ManagedAuthMutationOutcome,
        reason_code: Option<ManagedAuthReasonCode>,
        pending_restart_consumers: Vec<ManagedAuthConsumer>,
    ) -> ManagedAuthMutationResult {
        ManagedAuthMutationResult {
            contract_version: MANAGED_AUTH_CONTRACT_VERSION,
            operation_id: uuid::Uuid::new_v4().to_string(),
            outcome,
            overview: self.overview(),
            pending_restart_consumers,
            reason_code,
        }
    }

    pub(crate) fn codex_home(&self) -> PathBuf {
        #[cfg(test)]
        {
            self.config_dir.join("codex-home")
        }
        #[cfg(not(test))]
        {
            crate::codex_config::get_codex_config_dir()
        }
    }

    pub(crate) fn materialize_codex_document_for(
        &self,
        selected: &CredentialWithIdentity,
    ) -> Result<
        crate::services::managed_auth::consumers::codex::CodexChatGptAuthDocument,
        ManagedAuthReasonCode,
    > {
        use crate::services::managed_auth::consumers::codex::{
            materialize_from_bundle, CodexChatGptAuthDocument,
        };
        let lock = self.refresh.lock_for(&selected.credential.credential_id);
        let _guard = lock.blocking_lock();
        let current = self
            .repository
            .get_credential(&selected.credential.credential_id)
            .map_err(|_| ManagedAuthReasonCode::SecretUnavailable)?
            .ok_or(ManagedAuthReasonCode::SecretUnavailable)?;
        let bundle = self
            .readback_bundle(&current.secret_handle)
            .map_err(|_| ManagedAuthReasonCode::SecretUnavailable)?;
        match materialize_from_bundle(&bundle, &selected.identity.provider_subject) {
            Ok(document) => Ok(document),
            Err(ManagedAuthReasonCode::RequiresReauth) => {
                let refresh = bundle
                    .refresh_token()
                    .ok_or(ManagedAuthReasonCode::RequiresReauth)?
                    .to_string();
                let refreshed = tauri::async_runtime::block_on(async {
                    openai::refresh_oauth_grant(&refresh).await
                })
                .map_err(|_| ManagedAuthReasonCode::RequiresReauth)?;
                let id_token = refreshed
                    .id_token
                    .clone()
                    .or_else(|| bundle.id_token().map(str::to_string))
                    .ok_or(ManagedAuthReasonCode::RequiresReauth)?;
                let access = refreshed.access_token.clone();
                let new_refresh = refreshed.refresh_token.clone().unwrap_or(refresh);
                if let Ok(next_bundle) =
                    ManagedAuthSecretBundle::new(ManagedAuthSecretBundleParts {
                        credential_id: current.credential_id.clone(),
                        provider: current.provider,
                        generation: current.generation.saturating_add(1),
                        access_token: Some(access.clone()),
                        refresh_token: Some(new_refresh.clone()),
                        id_token: Some(id_token.clone()),
                        token_type: None,
                        granted_scopes: Vec::new(),
                        issued_at: Some(chrono::Utc::now().timestamp()),
                        expires_at: refreshed
                            .expires_in
                            .map(|secs| chrono::Utc::now().timestamp() + secs),
                    })
                {
                    let _ = self.replace_bundle_cas_locked(
                        &current.credential_id,
                        current.generation,
                        current.refresh_owner,
                        next_bundle,
                    );
                }
                CodexChatGptAuthDocument::from_tokens(
                    &id_token,
                    &access,
                    &new_refresh,
                    Some(selected.identity.provider_subject.as_str()),
                    Some(chrono::Utc::now().timestamp()),
                )
                .ok_or(ManagedAuthReasonCode::RequiresReauth)
            }
            Err(other) => Err(other),
        }
    }

    /// When live Codex auth is a managed ChatGPT account we are about to leave,
    /// absorb Codex-rotated tokens into SecretRef and return refresh ownership
    /// to FyAgent. Unknown / non-unique identities are left untouched.
    pub(crate) fn reconcile_outgoing_codex_live_tokens(
        &self,
        codex_home: &std::path::Path,
        target_provider_subject: &str,
    ) -> Result<(), ManagedAuthReasonCode> {
        use crate::services::managed_auth::consumers::codex::{
            auth_path_in, capture_auth_preimage, live_chatgpt_account_id, observe_codex_home,
            CodexChatGptAuthDocument,
        };

        let live = observe_codex_home(codex_home);
        let Some(live_account) = live_chatgpt_account_id(&live.auth_state) else {
            return Ok(());
        };
        if live_account == target_provider_subject {
            return Ok(());
        }
        let rows = self
            .repository
            .list_all_credentials()
            .map_err(|_| ManagedAuthReasonCode::PartialCompletion)?;
        let matches: Vec<_> = rows
            .iter()
            .filter(|row| {
                row.credential.purpose == CredentialPurpose::CodexNative
                    && row.identity.provider_subject == live_account
            })
            .collect();
        if matches.len() != 1 {
            return Ok(());
        }
        let row = matches[0];
        if row.credential.refresh_owner != RefreshOwner::CodexNative {
            return Ok(());
        }
        let auth_path = auth_path_in(codex_home);
        let Some(bytes) = capture_auth_preimage(&auth_path)
            .map_err(|_| ManagedAuthReasonCode::PartialCompletion)?
        else {
            return Ok(());
        };
        let Some(document) = CodexChatGptAuthDocument::try_from_live_bytes(&bytes) else {
            // Live identity matched but material is incomplete — refuse overwrite.
            return Err(ManagedAuthReasonCode::RequiresReauth);
        };
        if !document.identity_matches(live_account) {
            return Err(ManagedAuthReasonCode::IdentityMismatch);
        }

        let lock = self.refresh.lock_for(&row.credential.credential_id);
        let _guard = tokio::task::block_in_place(|| lock.blocking_lock());
        let current = self
            .repository
            .get_credential(&row.credential.credential_id)
            .map_err(|_| ManagedAuthReasonCode::SecretUnavailable)?
            .ok_or(ManagedAuthReasonCode::SecretUnavailable)?;
        if current.generation != row.credential.generation
            || current.refresh_owner != RefreshOwner::CodexNative
        {
            return Ok(());
        }
        let previous = self
            .readback_bundle(&current.secret_handle)
            .map_err(|_| ManagedAuthReasonCode::SecretUnavailable)?;
        let next_generation = current.generation.saturating_add(1);
        let next_bundle = ManagedAuthSecretBundle::new(ManagedAuthSecretBundleParts {
            credential_id: current.credential_id.clone(),
            provider: current.provider,
            generation: next_generation,
            access_token: Some(document.access_token().to_string()),
            refresh_token: Some(document.refresh_token().to_string()),
            id_token: Some(document.id_token().to_string()),
            token_type: Some("Bearer".to_string()),
            granted_scopes: Vec::new(),
            issued_at: Some(chrono::Utc::now().timestamp()),
            expires_at: previous.expires_at(),
        })
        .map_err(|_| ManagedAuthReasonCode::RequiresReauth)?;
        let replaced = self
            .replace_bundle_cas_locked(
                &current.credential_id,
                current.generation,
                RefreshOwner::CodexNative,
                next_bundle,
            )
            .map_err(|_| ManagedAuthReasonCode::PartialCompletion)?;
        if !replaced {
            return Ok(());
        }
        let _ = self.repository.transfer_refresh_owner(
            &current.credential_id,
            next_generation,
            RefreshOwner::CodexNative,
            RefreshOwner::Fyagent,
            chrono::Utc::now().timestamp(),
        );
        Ok(())
    }

    fn opencode_auth_path(&self) -> PathBuf {
        #[cfg(test)]
        {
            self.config_dir.join("opencode-data").join("auth.json")
        }
        #[cfg(not(test))]
        {
            opencode::default_auth_json_path()
        }
    }

    fn opencode_disconnect(
        &self,
        provider: ManagedAuthProvider,
        expected_revision: &str,
    ) -> Result<ManagedAuthMutationResult, ManagedAuthErrorDto> {
        let path = self.opencode_auth_path();
        let receipt = opencode::remove_file_key(
            &path,
            opencode::file_key_for(provider),
            Some(expected_revision),
        )
        .map_err(map_opencode_error)?;
        self.upsert_opencode_connection(provider, None, &receipt.revision, receipt.pending_restart)
            .map_err(ManagedAuthErrorDto::from_core)?;
        Ok(self.opencode_write_result(receipt.pending_restart))
    }

    fn opencode_connect(
        &self,
        provider: ManagedAuthProvider,
        request: &ManagedAuthConnectionActionRequest,
    ) -> Result<ManagedAuthMutationResult, ManagedAuthErrorDto> {
        let account_id = request
            .account_id
            .as_deref()
            .ok_or_else(ManagedAuthErrorDto::invalid_request)?;
        let rows = self
            .credentials_for_account(account_id)
            .map_err(ManagedAuthErrorDto::from_core)?;
        let independent = rows.iter().find(|row| {
            row.credential.provider == provider
                && row.credential.purpose == CredentialPurpose::OpencodeProvider
                && row.credential.status == CredentialStatus::Ready
        });
        let Some(selected) = independent else {
            let copied_lineage = rows.iter().any(|row| {
                row.credential.provider == provider
                    && matches!(
                        row.credential.purpose,
                        CredentialPurpose::ProxyUpstream
                            | CredentialPurpose::CodexNative
                            | CredentialPurpose::GrokNative
                            | CredentialPurpose::Copilot
                    )
            });
            return Err(ManagedAuthErrorDto::with_reason(if copied_lineage {
                ManagedAuthReasonCode::ProviderNotSupported
            } else {
                ManagedAuthReasonCode::NativeProjectionUnavailable
            }));
        };
        if matches!(
            selected.credential.refresh_owner,
            RefreshOwner::CodexNative | RefreshOwner::GrokNative
        ) {
            return Err(ManagedAuthErrorDto::with_reason(
                ManagedAuthReasonCode::ProviderNotSupported,
            ));
        }
        let bundle = self
            .readback_bundle(&selected.credential.secret_handle)
            .map_err(ManagedAuthErrorDto::from_core)?;
        let entry = projection_from_bundle(provider, &bundle).ok_or_else(|| {
            ManagedAuthErrorDto::with_reason(ManagedAuthReasonCode::NativeProjectionUnavailable)
        })?;
        let path = self.opencode_auth_path();
        let receipt = opencode::upsert_projection(
            &path,
            provider,
            &entry,
            Some(request.expected_revision.as_str()),
        )
        .map_err(map_opencode_error)?;
        let transferred = if selected.credential.refresh_owner != RefreshOwner::Opencode {
            self.repository
                .transfer_refresh_owner(
                    &selected.credential.credential_id,
                    selected.credential.generation,
                    selected.credential.refresh_owner,
                    RefreshOwner::Opencode,
                    chrono::Utc::now().timestamp(),
                )
                .map_err(ManagedAuthErrorDto::from_core)?
        } else {
            true
        };
        self.upsert_opencode_connection(
            provider,
            Some(selected.credential.credential_id.clone()),
            &receipt.revision,
            receipt.pending_restart,
        )
        .map_err(ManagedAuthErrorDto::from_core)?;
        if !transferred {
            return Ok(self.mutation_result_with(
                ManagedAuthMutationOutcome::Partial,
                Some(ManagedAuthReasonCode::PartialCompletion),
                vec![ManagedAuthConsumer::Opencode],
            ));
        }
        Ok(self.opencode_write_result(receipt.pending_restart))
    }

    pub(crate) fn finish_opencode_connect_after_login(
        &self,
        provider: ManagedAuthProvider,
        account_id: &str,
    ) -> (
        Option<String>,
        ManagedAuthLoginStage,
        Option<ManagedAuthReasonCode>,
    ) {
        match self.project_saved_opencode_session(provider, account_id) {
            Ok(result) => {
                let connection_id = Some(opencode::slot_connection_id(provider));
                if result.outcome == ManagedAuthMutationOutcome::Partial {
                    (
                        connection_id,
                        ManagedAuthLoginStage::Partial,
                        result
                            .reason_code
                            .or(Some(ManagedAuthReasonCode::PartialCompletion)),
                    )
                } else if result
                    .pending_restart_consumers
                    .contains(&ManagedAuthConsumer::Opencode)
                    || result.reason_code == Some(ManagedAuthReasonCode::PendingRestart)
                {
                    (
                        connection_id,
                        ManagedAuthLoginStage::Completed,
                        Some(ManagedAuthReasonCode::PendingRestart),
                    )
                } else {
                    (connection_id, ManagedAuthLoginStage::Completed, None)
                }
            }
            Err(error) => (
                None,
                ManagedAuthLoginStage::Partial,
                Some(error.reason_code),
            ),
        }
    }

    fn project_saved_opencode_session(
        &self,
        provider: ManagedAuthProvider,
        account_id: &str,
    ) -> Result<ManagedAuthMutationResult, ManagedAuthErrorDto> {
        let observed = opencode::observe_auth_store(&self.opencode_auth_path());
        self.opencode_connect(
            provider,
            &ManagedAuthConnectionActionRequest {
                connection_id: opencode::slot_connection_id(provider),
                expected_revision: observed.revision,
                action: ManagedAuthConnectionAction::ConnectAccount,
                account_id: Some(account_id.to_string()),
            },
        )
    }

    fn opencode_write_result(&self, pending_restart: bool) -> ManagedAuthMutationResult {
        if pending_restart {
            self.mutation_result_with(
                ManagedAuthMutationOutcome::Completed,
                Some(ManagedAuthReasonCode::PendingRestart),
                vec![ManagedAuthConsumer::Opencode],
            )
        } else {
            self.mutation_result(ManagedAuthMutationOutcome::Completed, None)
        }
    }

    fn opencode_ack_restart(
        &self,
        provider: ManagedAuthProvider,
        expected_revision: &str,
    ) -> Result<ManagedAuthMutationResult, ManagedAuthErrorDto> {
        let path = self.opencode_auth_path();
        let observed = opencode::observe_auth_store(&path);
        if observed.revision != expected_revision {
            return Err(ManagedAuthErrorDto::from_core(ManagedAuthCoreError::Stale));
        }
        if observed.closed_kind(provider).is_none() {
            return Err(ManagedAuthErrorDto::from_core(
                ManagedAuthCoreError::NotFound,
            ));
        }
        let stored = self
            .repository
            .list_connections()
            .map_err(ManagedAuthErrorDto::from_core)?
            .into_iter()
            .find(|row| {
                row.consumer == ManagedAuthConsumer::Opencode
                    && row.provider_slot == provider.as_str()
            });
        self.upsert_opencode_connection(
            provider,
            stored.and_then(|row| row.credential_id),
            &observed.revision,
            false,
        )
        .map_err(ManagedAuthErrorDto::from_core)?;
        Ok(self.mutation_result(ManagedAuthMutationOutcome::Completed, None))
    }

    fn upsert_opencode_connection(
        &self,
        provider: ManagedAuthProvider,
        credential_id: Option<String>,
        revision: &str,
        pending_restart: bool,
    ) -> Result<(), ManagedAuthCoreError> {
        let now = chrono::Utc::now().timestamp();
        let connection_id = opencode::slot_connection_id(provider);
        let status = if pending_restart {
            ConnectionStatus::PendingRestart
        } else if credential_id.is_some() {
            ConnectionStatus::Connected
        } else {
            ConnectionStatus::Disconnected
        };
        self.repository.upsert_connection(&ConnectionRecord {
            connection_id: connection_id.clone(),
            consumer: ManagedAuthConsumer::Opencode,
            target_id: String::new(),
            provider_slot: provider.as_str().to_string(),
            credential_id,
            desired_revision: revision.to_string(),
            observed_revision: Some(revision.to_string()),
            status,
            request_mode: ManagedAuthRequestMode::ProviderConnections,
            request_provider_label: Some(provider.as_str().to_string()),
            official_session_preserved: Some(true),
            pending_restart,
            created_at: now,
            updated_at: now,
        })
    }

    fn secret_backend_ready(&self) -> bool {
        let probe = SecretHandle::new(SecretRef::generate(), SecretVersion::generate());
        match self
            .secrets
            .probe(&probe, SecretPurpose::ManagedOAuthCredential)
        {
            Ok(summary) => {
                matches!(
                    summary.availability(),
                    SecretAvailability::Missing | SecretAvailability::Ready
                ) && summary.presence() != SecretPresence::Unknown
            }
            Err(_) => false,
        }
    }

    fn set_fail_closed(&self, secret_unavailable: bool, migration_blocked: bool) {
        let mut state = self
            .fail_closed
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        state.secret_unavailable |= secret_unavailable;
        state.migration_blocked |= migration_blocked;
    }

    pub(crate) fn fail_closed_snapshot(&self) -> FailClosedState {
        let state = self
            .fail_closed
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        FailClosedState {
            secret_unavailable: state.secret_unavailable,
            migration_blocked: state.migration_blocked,
        }
    }
}

fn purpose_for_provider(
    provider: ManagedAuthProvider,
) -> (CredentialPurpose, Option<ManagedAuthConsumer>) {
    match provider {
        ManagedAuthProvider::GithubCopilot => (
            CredentialPurpose::Copilot,
            Some(ManagedAuthConsumer::FyagentProxy),
        ),
        _ => (
            CredentialPurpose::ProxyUpstream,
            Some(ManagedAuthConsumer::FyagentProxy),
        ),
    }
}

fn source_filename(migration_id: &str) -> &'static str {
    match migration_id {
        CODEX_MIGRATION_ID => "codex_oauth_auth.json",
        XAI_MIGRATION_ID => "xai_oauth_auth.json",
        COPILOT_MIGRATION_ID => "copilot_auth.json",
        _ => "unknown.json",
    }
}

fn access_expired(expires_at: Option<i64>) -> bool {
    match expires_at {
        Some(expires_at) => expires_at <= chrono::Utc::now().timestamp() + 60,
        None => true,
    }
}

fn map_refresh_error(error: openai::OpenAiOAuthError) -> ManagedAuthCoreError {
    match error {
        openai::OpenAiOAuthError::RefreshTokenInvalid => ManagedAuthCoreError::InvalidData,
        _ => ManagedAuthCoreError::InvalidData,
    }
}

fn map_xai_refresh_error(
    error: crate::proxy::providers::xai_oauth_auth::XaiOAuthError,
) -> ManagedAuthCoreError {
    match error {
        crate::proxy::providers::xai_oauth_auth::XaiOAuthError::RefreshTokenInvalid
        | crate::proxy::providers::xai_oauth_auth::XaiOAuthError::ReauthRequired(_) => {
            ManagedAuthCoreError::InvalidData
        }
        _ => ManagedAuthCoreError::InvalidData,
    }
}

fn account_revision(row: &CredentialWithIdentity) -> String {
    stable_revision(&[
        &row.identity.identity_id,
        row.credential.status.as_str(),
        &row.credential.generation.to_string(),
        &row.credential.updated_at.to_string(),
    ])
}

fn removal_preview_id(account_id: &str, revision: &str) -> String {
    stable_revision(&["removal-preview", account_id, revision]).replacen("mr1:", "mp1:", 1)[..36]
        .to_string()
}

fn account_summary(
    row: &CredentialWithIdentity,
    all: &[CredentialWithIdentity],
    connections: &[ConnectionRecord],
    fail: &FailClosedState,
) -> ManagedAuthAccountSummary {
    let siblings: Vec<&CredentialWithIdentity> = all
        .iter()
        .filter(|other| other.identity.identity_id == row.identity.identity_id)
        .collect();
    let health = account_health(&siblings, fail);
    let credential_ids: Vec<&str> = siblings
        .iter()
        .map(|item| item.credential.credential_id.as_str())
        .collect();
    let connected_consumer_count = connections
        .iter()
        .filter(|connection| {
            connection
                .credential_id
                .as_deref()
                .is_some_and(|id| credential_ids.contains(&id))
        })
        .map(|connection| connection.consumer)
        .collect::<std::collections::HashSet<_>>()
        .len();
    let mut reason_codes = Vec::new();
    if health == ManagedAuthHealth::MigrationBlocked {
        reason_codes.push(ManagedAuthReasonCode::MigrationBlocked);
    }
    if fail.secret_unavailable || health == ManagedAuthHealth::Unavailable {
        reason_codes.push(ManagedAuthReasonCode::SecretUnavailable);
    }
    if health == ManagedAuthHealth::RequiresReauth {
        reason_codes.push(ManagedAuthReasonCode::RequiresReauth);
    }
    let mut allowed_actions = Vec::new();
    if health == ManagedAuthHealth::Ready {
        allowed_actions.push(ManagedAuthAccountAction::Reauthenticate);
        allowed_actions.push(ManagedAuthAccountAction::SetDefault);
        allowed_actions.push(ManagedAuthAccountAction::Remove);
    } else {
        if health == ManagedAuthHealth::RequiresReauth {
            allowed_actions.push(ManagedAuthAccountAction::Reauthenticate);
        }
        allowed_actions.push(ManagedAuthAccountAction::Remove);
    }
    ManagedAuthAccountSummary {
        account_id: row.identity.identity_id.clone(),
        revision: account_revision(row),
        provider: row.identity.provider,
        login: row.identity.login.clone(),
        display_name: row.identity.display_name.clone(),
        health,
        is_default: siblings.iter().any(|item| item.is_default),
        last_authenticated_at: Some(
            chrono::DateTime::from_timestamp(row.credential.authenticated_at, 0)
                .map(|value| value.to_rfc3339_opts(SecondsFormat::Secs, true))
                .unwrap_or_else(now_timestamp),
        ),
        connected_consumer_count,
        plan_summary: None,
        quota_summary: None,
        allowed_actions,
        reason_codes,
    }
}

fn account_health(rows: &[&CredentialWithIdentity], fail: &FailClosedState) -> ManagedAuthHealth {
    if rows
        .iter()
        .any(|row| row.credential.status == CredentialStatus::MigrationBlocked)
    {
        return ManagedAuthHealth::MigrationBlocked;
    }
    if fail.secret_unavailable
        || rows.iter().any(|row| {
            matches!(
                row.credential.status,
                CredentialStatus::SecretMissing | CredentialStatus::Revoked
            )
        })
    {
        return ManagedAuthHealth::Unavailable;
    }
    if rows
        .iter()
        .any(|row| row.credential.status == CredentialStatus::RequiresReauth)
    {
        return ManagedAuthHealth::RequiresReauth;
    }
    if rows
        .iter()
        .any(|row| row.credential.status == CredentialStatus::Provisioning)
    {
        return ManagedAuthHealth::Checking;
    }
    if rows
        .iter()
        .any(|row| row.credential.status == CredentialStatus::Ready)
    {
        return ManagedAuthHealth::Ready;
    }
    ManagedAuthHealth::Unavailable
}

fn provider_summary(
    provider: ManagedAuthProvider,
    fail: &FailClosedState,
) -> ManagedAuthProviderSummary {
    let mut reason_codes = Vec::new();
    if fail.secret_unavailable {
        reason_codes.push(ManagedAuthReasonCode::SecretUnavailable);
    }
    if fail.migration_blocked {
        reason_codes.push(ManagedAuthReasonCode::MigrationBlocked);
    }
    let consumers = match provider {
        ManagedAuthProvider::Openai => vec![
            ManagedAuthConsumer::Codex,
            ManagedAuthConsumer::Opencode,
            ManagedAuthConsumer::FyagentProxy,
        ],
        ManagedAuthProvider::Xai => vec![
            ManagedAuthConsumer::Grokbuild,
            ManagedAuthConsumer::Opencode,
            ManagedAuthConsumer::FyagentProxy,
        ],
        ManagedAuthProvider::GithubCopilot => vec![
            ManagedAuthConsumer::Opencode,
            ManagedAuthConsumer::FyagentProxy,
        ],
    };
    let vault_ready = !fail.secret_unavailable && !fail.migration_blocked;
    let (available, login_methods) = match provider {
        ManagedAuthProvider::Openai if vault_ready => (
            true,
            vec![
                ManagedAuthLoginMethod::BrowserLoopback,
                ManagedAuthLoginMethod::DeviceCode,
            ],
        ),
        ManagedAuthProvider::Xai if vault_ready => (true, vec![ManagedAuthLoginMethod::DeviceCode]),
        _ => (false, Vec::new()),
    };
    if available {
        reason_codes.clear();
    } else if reason_codes.is_empty() {
        reason_codes.push(if vault_ready {
            ManagedAuthReasonCode::ProviderNotSupported
        } else if fail.secret_unavailable {
            ManagedAuthReasonCode::SecretUnavailable
        } else {
            ManagedAuthReasonCode::NativeProjectionUnavailable
        });
    }
    ManagedAuthProviderSummary {
        provider,
        available,
        login_methods,
        consumers,
        reason_codes,
    }
}

fn merge_consumer_connections(
    codex_home: &std::path::Path,
    opencode_auth: &std::path::Path,
    rows: &[CredentialWithIdentity],
    connections: &[ConnectionRecord],
) -> Vec<crate::services::managed_auth::ManagedAuthConnectionSummary> {
    let observation =
        crate::services::managed_auth::consumers::codex::observe_codex_home(codex_home);
    let checked_at = now_timestamp();
    let mut summaries: Vec<_> = connections
        .iter()
        .filter(|connection| {
            connection.consumer != ManagedAuthConsumer::Codex
                && connection.consumer != ManagedAuthConsumer::Grokbuild
                && connection.consumer != ManagedAuthConsumer::Opencode
        })
        .map(|connection| connection_summary(connection, rows))
        .collect();
    summaries.insert(
        0,
        crate::services::managed_auth::consumers::grok::connection_summary(
            rows.iter().find(|row| {
                row.credential.purpose == CredentialPurpose::GrokNative
                    && row.credential.status == CredentialStatus::Ready
            }),
            connections
                .iter()
                .find(|connection| connection.consumer == ManagedAuthConsumer::Grokbuild),
            checked_at.clone(),
        ),
    );
    summaries.insert(
        0,
        crate::services::managed_auth::consumers::codex::connection_summary(
            &observation,
            rows.iter().find(|row| {
                row.credential.purpose == CredentialPurpose::CodexNative
                    && row.credential.status == CredentialStatus::Ready
            }),
            connections
                .iter()
                .find(|connection| connection.consumer == ManagedAuthConsumer::Codex),
            rows,
            checked_at.clone(),
        ),
    );
    summaries.extend(opencode::connection_summaries(
        &opencode::observe_auth_store(opencode_auth),
        rows,
        connections,
        checked_at,
    ));
    summaries
}

fn connection_summary(
    connection: &ConnectionRecord,
    rows: &[CredentialWithIdentity],
) -> ManagedAuthConnectionSummary {
    let account = connection
        .credential_id
        .as_ref()
        .and_then(|id| rows.iter().find(|row| row.credential.credential_id == *id));
    ManagedAuthConnectionSummary {
        connection_id: connection.connection_id.clone(),
        revision: connection
            .observed_revision
            .clone()
            .unwrap_or_else(|| connection.desired_revision.clone()),
        consumer: connection.consumer,
        target_id: (!connection.target_id.is_empty()).then(|| connection.target_id.clone()),
        target_label: None,
        provider: account.map(|row| row.identity.provider),
        account_id: account.map(|row| row.identity.identity_id.clone()),
        auth_status: match connection.status {
            ConnectionStatus::Connected => ManagedAuthConnectionState::Connected,
            ConnectionStatus::Disconnected => ManagedAuthConnectionState::Disconnected,
            ConnectionStatus::Checking => ManagedAuthConnectionState::Checking,
            ConnectionStatus::RequiresReauth => ManagedAuthConnectionState::RequiresReauth,
            ConnectionStatus::PendingRestart => ManagedAuthConnectionState::PendingRestart,
            _ => ManagedAuthConnectionState::Unavailable,
        },
        credential_manager: ManagedAuthCredentialManager::Fyagent,
        request_mode: connection.request_mode,
        request_provider_label: connection.request_provider_label.clone(),
        official_session_preserved: connection.official_session_preserved,
        pending_restart: connection.pending_restart,
        allowed_actions: Vec::new(),
        checked_at: now_timestamp(),
        reason_codes: vec![ManagedAuthReasonCode::NativeProjectionUnavailable],
    }
}

fn projection_from_bundle(
    provider: ManagedAuthProvider,
    bundle: &ManagedAuthSecretBundle,
) -> Option<ProjectionEntry> {
    match (bundle.refresh_token(), bundle.access_token()) {
        (Some(refresh), Some(access)) => Some(ProjectionEntry::Oauth {
            refresh: Zeroizing::new(refresh.to_string()),
            access: Zeroizing::new(access.to_string()),
            expires: opencode::opencode_expires_ms(bundle.expires_at()),
            account_id: None,
        }),
        (None, Some(access)) => Some(ProjectionEntry::Api {
            key: Zeroizing::new(access.to_string()),
        }),
        _ => {
            let _ = provider;
            None
        }
    }
}

fn map_opencode_error(error: opencode::OpencodeAuthError) -> ManagedAuthErrorDto {
    ManagedAuthErrorDto::from_core(ManagedAuthCoreError::from(error))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::secret::{MemoryFailureMode, MemorySecretBackend};
    use tempfile::tempdir;

    fn service_with_memory() -> (ManagedAuthService<MemorySecretBackend>, tempfile::TempDir) {
        let dir = tempdir().expect("tempdir");
        let db = Arc::new(Database::memory().expect("db"));
        let service = ManagedAuthService::new(
            db,
            SecretService::new(MemorySecretBackend::new()),
            dir.path().to_path_buf(),
        );
        (service, dir)
    }

    fn service_with_shared_memory() -> (
        ManagedAuthService<MemorySecretBackend>,
        tempfile::TempDir,
        MemorySecretBackend,
    ) {
        let dir = tempdir().expect("tempdir");
        let db = Arc::new(Database::memory().expect("db"));
        let backend = MemorySecretBackend::new();
        let service = ManagedAuthService::new(
            db,
            SecretService::new(backend.clone()),
            dir.path().to_path_buf(),
        );
        (service, dir, backend)
    }

    fn sample_input(legacy: &str, token: &str, make_default: bool) -> LegacyCredentialInput {
        LegacyCredentialInput {
            migration_id: Some(CODEX_MIGRATION_ID),
            provider: ManagedAuthProvider::Openai,
            purpose: CredentialPurpose::ProxyUpstream,
            consumer: Some(ManagedAuthConsumer::FyagentProxy),
            legacy_account_id: legacy.to_string(),
            provider_subject: "workspace-subject".to_string(),
            provider_tenant: String::new(),
            login: "person@example.com".to_string(),
            display_name: None,
            avatar_url: None,
            access_token: None,
            refresh_token: Some(Zeroizing::new(token.to_string())),
            id_token: None,
            desired_status: CredentialStatus::Ready,
            refresh_owner: RefreshOwner::Fyagent,
            authenticated_at: 1_700_000_000,
            make_default,
        }
    }

    #[test]
    fn provisioning_persists_secret_ref_before_native_create_and_recovers() {
        let (service, _dir) = service_with_memory();
        let credential = service
            .provision_legacy_credential(sample_input("legacy-credential", "refresh-value", true))
            .expect("provision");
        assert_eq!(credential.status, CredentialStatus::Ready);
        assert!(service
            .repository()
            .get_credential(&credential.credential_id)
            .expect("read")
            .is_some());
        let bundle = service
            .readback_bundle(&credential.secret_handle)
            .expect("readback");
        assert_eq!(bundle.refresh_token(), Some("refresh-value"));
        assert_eq!(
            service
                .repository()
                .get_default(
                    ManagedAuthProvider::Openai,
                    CredentialPurpose::ProxyUpstream,
                    Some(ManagedAuthConsumer::FyagentProxy),
                )
                .expect("default")
                .as_deref(),
            Some(credential.credential_id.as_str())
        );
    }

    #[test]
    fn recover_after_create_without_mark_ready() {
        let (service, _dir) = service_with_memory();
        let input = sample_input("legacy-credential", "refresh-value", false);
        let reserved = service.secrets.reserve();
        let identity_id = stable_identity_id(
            input.provider,
            &input.provider_subject,
            &input.provider_tenant,
        );
        let credential_id = stable_credential_id(
            input.provider,
            input.purpose,
            input.consumer,
            &input.legacy_account_id,
        );
        let now = 1_700_000_000;
        let stored = service
            .repository
            .begin_provisioning(&NewCredential {
                identity: IdentityRecord {
                    identity_id: identity_id.clone(),
                    provider: input.provider,
                    provider_subject: input.provider_subject.clone(),
                    provider_tenant: input.provider_tenant.clone(),
                    login: input.login.clone(),
                    display_name: None,
                    avatar_url: None,
                    created_at: now,
                    updated_at: now,
                },
                credential: CredentialRecord {
                    credential_id: credential_id.clone(),
                    identity_id,
                    provider: input.provider,
                    purpose: input.purpose,
                    consumer: input.consumer,
                    legacy_account_id: input.legacy_account_id.clone(),
                    secret_handle: reserved.clone(),
                    refresh_owner: RefreshOwner::Fyagent,
                    generation: 1,
                    access_expires_at: None,
                    status: CredentialStatus::Provisioning,
                    authenticated_at: now,
                    refreshed_at: None,
                    migration_id: Some(CODEX_MIGRATION_ID.to_string()),
                    created_at: now,
                    updated_at: now,
                },
            })
            .expect("insert provisioning");
        let bundle = ManagedAuthSecretBundle::new(ManagedAuthSecretBundleParts {
            credential_id: stored.credential_id.clone(),
            provider: stored.provider,
            generation: 1,
            access_token: None,
            refresh_token: Some("refresh-value".to_string()),
            id_token: None,
            token_type: None,
            granted_scopes: Vec::new(),
            issued_at: Some(now),
            expires_at: None,
        })
        .expect("bundle");
        service
            .secrets
            .create_reserved(
                &reserved,
                bundle.encode().expect("encode"),
                SecretPurpose::ManagedOAuthCredential,
            )
            .expect("native create");
        service.recover_credentials().expect("recover");
        let recovered = service
            .repository
            .get_credential(&credential_id)
            .expect("read")
            .expect("present");
        assert_eq!(recovered.status, CredentialStatus::Ready);
    }

    #[test]
    fn cas_rejects_stale_generation() {
        let (service, _dir) = service_with_memory();
        let credential = service
            .provision_legacy_credential(sample_input("legacy-credential", "refresh-value", false))
            .expect("provision");
        let next = ManagedAuthSecretBundle::new(ManagedAuthSecretBundleParts {
            credential_id: credential.credential_id.clone(),
            provider: credential.provider,
            generation: 2,
            access_token: Some("access-2".to_string()),
            refresh_token: Some("refresh-2".to_string()),
            id_token: None,
            token_type: None,
            granted_scopes: Vec::new(),
            issued_at: None,
            expires_at: None,
        })
        .expect("bundle");
        assert!(service
            .replace_bundle_cas(&credential.credential_id, 1, RefreshOwner::Fyagent, next,)
            .expect("cas"));
        let stale = ManagedAuthSecretBundle::new(ManagedAuthSecretBundleParts {
            credential_id: credential.credential_id.clone(),
            provider: credential.provider,
            generation: 2,
            access_token: Some("access-stale".to_string()),
            refresh_token: Some("refresh-stale".to_string()),
            id_token: None,
            token_type: None,
            granted_scopes: Vec::new(),
            issued_at: None,
            expires_at: None,
        })
        .expect("stale bundle");
        assert!(!service
            .replace_bundle_cas(&credential.credential_id, 1, RefreshOwner::Fyagent, stale,)
            .expect("stale cas"));
        let current = service
            .repository
            .get_credential(&credential.credential_id)
            .expect("read")
            .expect("present");
        assert_eq!(current.generation, 2);
        let bundle = service
            .readback_bundle(&current.secret_handle)
            .expect("readback");
        assert_eq!(bundle.refresh_token(), Some("refresh-2"));
    }

    #[test]
    fn resolver_rejects_native_refresh_owner() {
        let (service, _dir) = service_with_memory();
        let mut input = sample_input("legacy-credential", "refresh-value", false);
        input.refresh_owner = RefreshOwner::CodexNative;
        let credential = service
            .provision_legacy_credential(input)
            .expect("provision");
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");
        let error = runtime
            .block_on(service.resolve_access_material(
                ManagedAuthProvider::Openai,
                Some(&credential.legacy_account_id),
            ))
            .expect_err("native owner");
        assert!(matches!(error, ManagedAuthCoreError::Conflict));
    }

    #[test]
    fn resolver_rejects_grok_native_purpose_even_with_fyagent_owner() {
        let (service, _dir) = service_with_memory();
        let mut input = sample_input("xai-user-1", "refresh-proxy", false);
        input.provider = ManagedAuthProvider::Xai;
        input.purpose = CredentialPurpose::GrokNative;
        input.consumer = Some(ManagedAuthConsumer::Grokbuild);
        input.refresh_owner = RefreshOwner::Fyagent;
        let credential = service
            .provision_legacy_credential(input)
            .expect("provision");
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");
        let error = runtime
            .block_on(service.resolve_credential_access(credential))
            .expect_err("grok native");
        assert!(matches!(error, ManagedAuthCoreError::Conflict));
        let overview = service.overview();
        let xai = overview
            .providers
            .iter()
            .find(|provider| provider.provider == ManagedAuthProvider::Xai)
            .expect("xai");
        assert!(xai.available);
        assert_eq!(
            xai.login_methods,
            vec![crate::services::managed_auth::ManagedAuthLoginMethod::DeviceCode]
        );
        let grok = overview
            .connections
            .iter()
            .find(|connection| connection.consumer == ManagedAuthConsumer::Grokbuild)
            .expect("grok");
        assert_ne!(
            grok.auth_status,
            crate::services::managed_auth::ManagedAuthConnectionState::Connected
        );
        assert!(grok
            .reason_codes
            .contains(&ManagedAuthReasonCode::NativeProjectionUnavailable));
    }

    #[test]
    fn proxy_and_grok_sessions_stay_separate_lineages() {
        let (service, _dir) = service_with_memory();
        let mut proxy = sample_input("xai-user-1", "refresh-proxy", true);
        proxy.provider = ManagedAuthProvider::Xai;
        let mut grok = sample_input("xai-user-1", "refresh-grok", false);
        grok.provider = ManagedAuthProvider::Xai;
        grok.purpose = CredentialPurpose::GrokNative;
        grok.consumer = Some(ManagedAuthConsumer::Grokbuild);
        let proxy_cred = service.provision_legacy_credential(proxy).expect("proxy");
        let grok_cred = service.provision_legacy_credential(grok).expect("grok");
        assert_ne!(proxy_cred.credential_id, grok_cred.credential_id);
        let proxy_bundle = service
            .readback_bundle(&proxy_cred.secret_handle)
            .expect("proxy bundle");
        let grok_bundle = service
            .readback_bundle(&grok_cred.secret_handle)
            .expect("grok bundle");
        assert_eq!(proxy_bundle.refresh_token(), Some("refresh-proxy"));
        assert_eq!(grok_bundle.refresh_token(), Some("refresh-grok"));
    }

    #[test]
    fn overview_has_no_secret_fields() {
        let (service, _dir) = service_with_memory();
        service
            .provision_legacy_credential(sample_input("legacy-credential", "refresh-value", true))
            .expect("provision");
        let value = serde_json::to_value(service.overview()).expect("json");
        let text = value.to_string().to_ascii_lowercase();
        for forbidden in [
            "access_token",
            "refresh_token",
            "id_token",
            "authorization_code",
            "device_auth_id",
            "\"devicecode\"",
            "secretref",
            "secret_ref",
            "verifier",
            "refresh-value",
        ] {
            assert!(!text.contains(forbidden), "leaked {forbidden}");
        }
    }

    #[test]
    fn migration_finalizes_after_readback_and_is_idempotent() {
        let (service, dir) = service_with_memory();
        let source = dir.path().join("codex_oauth_auth.json");
        std::fs::write(
            &source,
            r#"{
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
            }"#,
        )
        .expect("write source");
        service.startup().expect("startup");
        let backup = dir.path().join("codex_oauth_auth.json.managed-auth-v1.bak");
        assert!(backup.exists(), "source should be renamed after readback");
        assert!(!source.exists());
        assert!(service.legacy_store_sealed(CODEX_MIGRATION_ID));
        service.startup().expect("second startup");
        assert!(backup.exists());
        let overview = service.overview();
        assert_eq!(overview.accounts.len(), 1);
        assert_eq!(overview.accounts[0].login, "person@example.com");
    }

    #[test]
    fn migration_failure_leaves_source_file() {
        let (service, dir) = service_with_memory();
        let source = dir.path().join("copilot_auth.json");
        std::fs::write(
            &source,
            r#"{"version":1,"accounts":{},"github_token":"token"}"#,
        )
        .expect("write");
        let _ = service.startup();
        assert!(source.exists(), "blocked migration must not rename source");
        let fail = service.fail_closed_snapshot();
        assert!(fail.migration_blocked);
        assert!(!service.legacy_store_sealed(COPILOT_MIGRATION_ID));
    }

    #[test]
    fn cas_serializes_refresh_and_rejects_stale_generation_race() {
        let (service, _dir) = service_with_memory();
        let credential = service
            .provision_legacy_credential(sample_input("legacy-credential", "refresh-value", false))
            .expect("provision");
        let first = ManagedAuthSecretBundle::new(ManagedAuthSecretBundleParts {
            credential_id: credential.credential_id.clone(),
            provider: credential.provider,
            generation: 2,
            access_token: Some("access-first".to_string()),
            refresh_token: Some("refresh-first".to_string()),
            id_token: None,
            token_type: None,
            granted_scopes: Vec::new(),
            issued_at: None,
            expires_at: None,
        })
        .expect("first");
        let second = ManagedAuthSecretBundle::new(ManagedAuthSecretBundleParts {
            credential_id: credential.credential_id.clone(),
            provider: credential.provider,
            generation: 2,
            access_token: Some("access-second".to_string()),
            refresh_token: Some("refresh-second".to_string()),
            id_token: None,
            token_type: None,
            granted_scopes: Vec::new(),
            issued_at: None,
            expires_at: None,
        })
        .expect("second");
        std::thread::scope(|scope| {
            let left = scope.spawn(|| {
                service.replace_bundle_cas(
                    &credential.credential_id,
                    1,
                    RefreshOwner::Fyagent,
                    first,
                )
            });
            let right = scope.spawn(|| {
                service.replace_bundle_cas(
                    &credential.credential_id,
                    1,
                    RefreshOwner::Fyagent,
                    second,
                )
            });
            let left = left.join().expect("left").expect("cas");
            let right = right.join().expect("right").expect("cas");
            assert_ne!(left, right, "exactly one generation-1 CAS may commit");
        });
        let current = service
            .repository
            .get_credential(&credential.credential_id)
            .expect("read")
            .expect("present");
        assert_eq!(current.generation, 2);
        let bundle = service
            .readback_bundle(&current.secret_handle)
            .expect("readback");
        let refresh = bundle.refresh_token().expect("refresh");
        assert!(refresh == "refresh-first" || refresh == "refresh-second");
        assert_ne!(refresh, "refresh-value");
    }

    #[test]
    fn completed_source_hash_mismatch_is_blocked_and_leaves_new_file() {
        let (service, dir) = service_with_memory();
        let source = dir.path().join("codex_oauth_auth.json");
        std::fs::write(
            &source,
            r#"{
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
            }"#,
        )
        .expect("write source");
        service.startup().expect("startup");
        std::fs::write(
            &source,
            r#"{
              "version": 2,
              "accounts": {
                "other": {
                  "credential_id": "other",
                  "chatgpt_account_id": "other-subject",
                  "email": "other@example.com",
                  "refresh_token": "other-refresh",
                  "authenticated_at": 1700000001
                }
              }
            }"#,
        )
        .expect("write changed source");
        service.startup().expect("blocked startup still returns");
        assert!(source.exists(), "changed source must remain recoverable");
        let fail = service.fail_closed_snapshot();
        assert!(fail.migration_blocked);
        let overview = service.overview();
        assert!(overview
            .reason_codes
            .contains(&ManagedAuthReasonCode::MigrationBlocked));
    }

    #[test]
    fn unavailable_secret_backend_does_not_rename_legacy_json() {
        let dir = tempdir().expect("tempdir");
        let source = dir.path().join("codex_oauth_auth.json");
        std::fs::write(
            &source,
            r#"{
              "version": 2,
              "accounts": {
                "legacy-credential": {
                  "credential_id": "legacy-credential",
                  "chatgpt_account_id": "workspace-subject",
                  "email": "person@example.com",
                  "refresh_token": "refresh-value",
                  "authenticated_at": 1700000000
                }
              }
            }"#,
        )
        .expect("write");
        let service = ManagedAuthService::new(
            Arc::new(Database::memory().expect("db")),
            SecretService::new(crate::services::secret::UnavailableSecretBackend::new()),
            dir.path().to_path_buf(),
        );
        service.startup().expect("fail closed");
        assert!(source.exists());
        let fail = service.fail_closed_snapshot();
        assert!(fail.secret_unavailable);
        assert!(!fail.migration_blocked);
        assert!(!service.legacy_store_sealed(CODEX_MIGRATION_ID));
        assert!(service.overview().accounts.is_empty());
    }

    #[test]
    fn mutation_operation_id_matches_frontend_uuid_contract() {
        let (service, _dir) = service_with_memory();
        let credential = service
            .provision_legacy_credential(sample_input("legacy-credential", "refresh-value", true))
            .expect("provision");
        let row = service
            .repository
            .list_all_credentials()
            .expect("rows")
            .into_iter()
            .find(|row| row.credential.credential_id == credential.credential_id)
            .expect("row");
        let result = service
            .set_default_account(&row.identity.identity_id, &account_revision(&row))
            .expect("set default");
        let parsed = uuid::Uuid::parse_str(&result.operation_id).expect("uuid");
        assert_eq!(parsed.get_version_num(), 4);
        assert!(!result.operation_id.contains("mo1:"));
        let serialized = serde_json::to_value(&result).expect("json");
        assert!(serialized.get("secretRef").is_none());
        assert!(serialized.get("secret_ref").is_none());
    }

    #[test]
    fn blocked_copilot_v1_does_not_seal_or_degrade_codex_migration() {
        let (service, dir) = service_with_memory();
        let codex = dir.path().join("codex_oauth_auth.json");
        std::fs::write(
            &codex,
            r#"{
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
            }"#,
        )
        .expect("write codex");
        let copilot = dir.path().join("copilot_auth.json");
        std::fs::write(
            &copilot,
            r#"{"version":1,"accounts":{},"github_token":"token"}"#,
        )
        .expect("write copilot");
        service.startup().expect("startup");
        let backup = dir.path().join("codex_oauth_auth.json.managed-auth-v1.bak");
        assert!(
            backup.exists(),
            "successful Codex migration must still finalize"
        );
        assert!(!codex.exists());
        assert!(copilot.exists(), "blocked Copilot source must remain");
        assert!(service.legacy_store_sealed(CODEX_MIGRATION_ID));
        assert!(!service.legacy_store_sealed(COPILOT_MIGRATION_ID));
        let overview = service.overview();
        assert_eq!(overview.accounts.len(), 1);
        assert_eq!(overview.accounts[0].login, "person@example.com");
        assert_eq!(overview.accounts[0].health, ManagedAuthHealth::Ready);
        assert!(overview
            .reason_codes
            .contains(&ManagedAuthReasonCode::MigrationBlocked));
    }

    #[test]
    fn empty_legacy_store_is_left_unsealed() {
        let (service, dir) = service_with_memory();
        let source = dir.path().join("codex_oauth_auth.json");
        std::fs::write(&source, r#"{"version":2,"accounts":{}}"#).expect("write");
        service.startup().expect("startup");
        assert!(source.exists());
        assert!(!service.legacy_store_sealed(CODEX_MIGRATION_ID));
        assert!(service.overview().accounts.is_empty());
    }

    fn opencode_input(legacy: &str) -> LegacyCredentialInput {
        LegacyCredentialInput {
            migration_id: None,
            provider: ManagedAuthProvider::Openai,
            purpose: CredentialPurpose::OpencodeProvider,
            consumer: Some(ManagedAuthConsumer::Opencode),
            legacy_account_id: legacy.to_string(),
            provider_subject: "opencode-subject".to_string(),
            provider_tenant: String::new(),
            login: "opencode@example.com".to_string(),
            display_name: None,
            avatar_url: None,
            access_token: Some(Zeroizing::new("at-opencode".to_string())),
            refresh_token: Some(Zeroizing::new("rt-opencode".to_string())),
            id_token: None,
            desired_status: CredentialStatus::Ready,
            refresh_owner: RefreshOwner::Fyagent,
            authenticated_at: 1_700_000_000,
            make_default: false,
        }
    }

    #[test]
    fn opencode_overview_observes_auth_json_without_leaking_tokens() {
        let (service, dir) = service_with_memory();
        let path = dir.path().join("opencode-data").join("auth.json");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            serde_json::to_vec_pretty(&serde_json::json!({
                "openai": {
                    "type": "oauth",
                    "refresh": "rt-secret",
                    "access": "at-secret",
                    "expires": 9
                }
            }))
            .unwrap(),
        )
        .unwrap();
        let overview = service.overview();
        let opencode = overview
            .connections
            .iter()
            .filter(|row| row.consumer == ManagedAuthConsumer::Opencode)
            .collect::<Vec<_>>();
        assert_eq!(opencode.len(), 3);
        assert!(opencode.iter().any(|row| {
            row.provider == Some(ManagedAuthProvider::Openai)
                && row.auth_status == ManagedAuthConnectionState::Connected
        }));
        let text = serde_json::to_string(&overview)
            .unwrap()
            .to_ascii_lowercase();
        assert!(!text.contains("rt-secret"));
        assert!(!text.contains("at-secret"));
        assert!(!text.contains("auth.json"));
    }

    #[test]
    fn opencode_connect_projects_independent_session_and_rejects_proxy_lineage() {
        let (service, dir) = service_with_memory();
        let proxy = service
            .provision_legacy_credential(sample_input("proxy-lineage", "refresh-value", true))
            .expect("proxy");
        let independent = service
            .provision_legacy_credential(opencode_input("opencode-session"))
            .expect("opencode");
        let overview = service.overview();
        let openai = overview
            .connections
            .iter()
            .find(|row| {
                row.consumer == ManagedAuthConsumer::Opencode
                    && row.provider == Some(ManagedAuthProvider::Openai)
            })
            .expect("openai slot");
        let proxy_err = service
            .apply_connection_action(&ManagedAuthConnectionActionRequest {
                connection_id: openai.connection_id.clone(),
                expected_revision: openai.revision.clone(),
                action: ManagedAuthConnectionAction::ConnectAccount,
                account_id: Some(proxy.identity_id.clone()),
            })
            .expect_err("proxy lineage");
        assert_eq!(
            proxy_err.reason_code,
            ManagedAuthReasonCode::ProviderNotSupported
        );
        assert!(!dir.path().join("opencode-data").join("auth.json").exists());

        let mut copilot_input = sample_input("copilot-lineage", "copilot-refresh", false);
        copilot_input.migration_id = None;
        copilot_input.provider = ManagedAuthProvider::GithubCopilot;
        copilot_input.purpose = CredentialPurpose::Copilot;
        copilot_input.consumer = Some(ManagedAuthConsumer::FyagentProxy);
        copilot_input.provider_subject = "github-copilot-user".to_string();
        copilot_input.login = "copilot@example.com".to_string();
        let copilot = service
            .provision_legacy_credential(copilot_input)
            .expect("copilot");
        let github = overview
            .connections
            .iter()
            .find(|row| {
                row.consumer == ManagedAuthConsumer::Opencode
                    && row.provider == Some(ManagedAuthProvider::GithubCopilot)
            })
            .expect("github slot");
        let copilot_err = service
            .apply_connection_action(&ManagedAuthConnectionActionRequest {
                connection_id: github.connection_id.clone(),
                expected_revision: github.revision.clone(),
                action: ManagedAuthConnectionAction::ConnectAccount,
                account_id: Some(copilot.identity_id.clone()),
            })
            .expect_err("copilot lineage");
        assert_eq!(
            copilot_err.reason_code,
            ManagedAuthReasonCode::ProviderNotSupported
        );

        let result = service
            .apply_connection_action(&ManagedAuthConnectionActionRequest {
                connection_id: openai.connection_id.clone(),
                expected_revision: openai.revision.clone(),
                action: ManagedAuthConnectionAction::ConnectAccount,
                account_id: Some(independent.identity_id.clone()),
            })
            .expect("project");
        assert_eq!(result.outcome, ManagedAuthMutationOutcome::Completed);
        assert!(result
            .pending_restart_consumers
            .contains(&ManagedAuthConsumer::Opencode));
        let raw: serde_json::Value = serde_json::from_slice(
            &std::fs::read(dir.path().join("opencode-data").join("auth.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(raw["openai"]["type"], "oauth");
        assert_eq!(raw["openai"]["refresh"], "rt-opencode");
        let owner = service
            .repository
            .get_credential(&independent.credential_id)
            .unwrap()
            .unwrap()
            .refresh_owner;
        assert_eq!(owner, RefreshOwner::Opencode);
        let openai = result
            .overview
            .connections
            .iter()
            .find(|row| {
                row.consumer == ManagedAuthConsumer::Opencode
                    && row.provider == Some(ManagedAuthProvider::Openai)
            })
            .expect("projected");
        assert!(openai.pending_restart);
        assert_eq!(
            openai.auth_status,
            ManagedAuthConnectionState::PendingRestart
        );
        assert!(openai
            .reason_codes
            .contains(&ManagedAuthReasonCode::PendingRestart));
        assert!(!openai
            .reason_codes
            .contains(&ManagedAuthReasonCode::NativeProjectionUnavailable));
        let text = serde_json::to_string(&result.overview)
            .unwrap()
            .to_ascii_lowercase();
        assert!(!text.contains("rt-opencode"));
    }

    #[test]
    fn secret_missing_account_can_be_removed_when_vault_delete_is_denied() {
        let (service, _dir, backend) = service_with_shared_memory();
        service
            .provision_legacy_credential(sample_input("legacy-credential", "refresh-value", true))
            .expect("provision first");
        service
            .provision_legacy_credential(sample_input(
                "legacy-credential-2",
                "refresh-value-2",
                false,
            ))
            .expect("provision second");
        let rows = service.repository.list_all_credentials().expect("rows");
        assert_eq!(rows.len(), 2);
        let now = chrono::Utc::now().timestamp();
        for row in &rows {
            service
                .repository
                .set_status(
                    &row.credential.credential_id,
                    CredentialStatus::SecretMissing,
                    now,
                )
                .expect("mark missing");
        }
        backend.set_mode(MemoryFailureMode::Denied);
        let account = service
            .overview()
            .accounts
            .into_iter()
            .find(|account| account.login == "person@example.com")
            .expect("account");
        let preview = service
            .preview_account_removal(&account.account_id, &account.revision)
            .expect("preview");
        let result = service
            .remove_account(
                &preview.preview_id,
                &account.account_id,
                &preview.expected_revision,
            )
            .expect("remove unusable account despite vault PermissionDenied");
        assert_eq!(result.outcome, ManagedAuthMutationOutcome::Completed);
        assert!(result.overview.accounts.is_empty());
        assert!(service
            .repository
            .list_all_credentials()
            .expect("rows")
            .is_empty());
    }

    #[test]
    fn migration_blocked_account_can_be_removed_when_vault_delete_is_denied() {
        let (service, _dir, backend) = service_with_shared_memory();
        service
            .provision_legacy_credential(sample_input("legacy-credential", "refresh-value", true))
            .expect("provision");
        let rows = service.repository.list_all_credentials().expect("rows");
        let now = chrono::Utc::now().timestamp();
        service
            .repository
            .set_status(
                &rows[0].credential.credential_id,
                CredentialStatus::MigrationBlocked,
                now,
            )
            .expect("mark blocked");
        backend.set_mode(MemoryFailureMode::Denied);
        let account = service
            .overview()
            .accounts
            .into_iter()
            .next()
            .expect("account");
        assert_eq!(account.health, ManagedAuthHealth::MigrationBlocked);
        assert!(account
            .allowed_actions
            .contains(&ManagedAuthAccountAction::Remove));
        let preview = service
            .preview_account_removal(&account.account_id, &account.revision)
            .expect("preview");
        let result = service
            .remove_account(
                &preview.preview_id,
                &account.account_id,
                &preview.expected_revision,
            )
            .expect("remove leftover login admission despite vault PermissionDenied");
        assert_eq!(result.outcome, ManagedAuthMutationOutcome::Completed);
        assert!(result.overview.accounts.is_empty());
    }

    #[test]
    fn ready_account_stays_when_vault_delete_is_denied() {
        let (service, _dir, backend) = service_with_shared_memory();
        service
            .provision_legacy_credential(sample_input("legacy-credential", "refresh-value", true))
            .expect("provision");
        backend.set_mode(MemoryFailureMode::Denied);
        let account = service
            .overview()
            .accounts
            .into_iter()
            .next()
            .expect("account");
        let preview = service
            .preview_account_removal(&account.account_id, &account.revision)
            .expect("preview");
        let error = service
            .remove_account(
                &preview.preview_id,
                &account.account_id,
                &preview.expected_revision,
            )
            .expect_err("authoritative credential must stay");
        assert!(matches!(error, ManagedAuthCoreError::SecretUnavailable));
        assert_eq!(service.overview().accounts.len(), 1);
        assert_eq!(
            service
                .repository
                .list_all_credentials()
                .expect("rows")
                .len(),
            1
        );
    }
}
