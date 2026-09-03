use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use chrono::SecondsFormat;
use zeroize::Zeroizing;

use crate::database::Database;
use crate::proxy::providers::codex_oauth_auth::CodexOAuthManager;
use crate::proxy::providers::copilot_auth::exchange_github_token_for_copilot;
use crate::proxy::providers::xai_oauth_auth::XaiOAuthManager;
use crate::services::managed_auth::login_sessions::LoginSessionStore;
use crate::services::managed_auth::providers::xai::XaiLoginHooks;
use crate::services::secret::{
    DecodeSecret, SecretAvailability, SecretBackend, SecretErrorCode, SecretHandle, SecretPresence,
    SecretPurpose, SecretRef, SecretService, SecretVersion,
};

use super::migration::{
    self, LegacyCredentialInput, CODEX_MIGRATION_ID, COPILOT_MIGRATION_ID, XAI_MIGRATION_ID,
};
use super::{
    now_timestamp, stable_connection_id, stable_credential_id, stable_identity_id, stable_revision,
    ConnectionRecord, ConnectionStatus, CredentialPurpose, CredentialRecord, CredentialStatus,
    CredentialWithIdentity, IdentityRecord, ManagedAuthAccountAction,
    ManagedAuthAccountRemovalImpact, ManagedAuthAccountRemovalPreview, ManagedAuthAccountSummary,
    ManagedAuthConnectionState, ManagedAuthConnectionSummary, ManagedAuthConsumer,
    ManagedAuthCoreError, ManagedAuthCredentialManager, ManagedAuthHealth, ManagedAuthLoginMethod,
    ManagedAuthMutationOutcome, ManagedAuthMutationResult, ManagedAuthOverview,
    ManagedAuthProvider, ManagedAuthProviderSummary, ManagedAuthReasonCode, ManagedAuthRepository,
    ManagedAuthRequestMode, ManagedAuthSecretBundle, ManagedAuthSecretBundleParts, MigrationStatus,
    NewCredential, RefreshOwner, MANAGED_AUTH_CONTRACT_VERSION,
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
    pub(crate) xai_hooks: Mutex<XaiLoginHooks>,
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
            xai_hooks: Mutex::new(XaiLoginHooks::default()),
        }
    }

    pub(crate) fn repository(&self) -> &ManagedAuthRepository {
        &self.repository
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
        if account_revision(selected) != expected_revision {
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

    pub(crate) fn set_compatibility_default(
        &self,
        provider: ManagedAuthProvider,
        legacy_account_id: &str,
    ) -> Result<(), ManagedAuthCoreError> {
        let (purpose, consumer) = purpose_for_provider(provider);
        let credential = self
            .repository
            .get_credential_by_legacy(provider, purpose, consumer, legacy_account_id)?
            .ok_or(ManagedAuthCoreError::NotFound)?;
        if credential.status != CredentialStatus::Ready {
            return Err(ManagedAuthCoreError::Conflict);
        }
        if !self.repository.set_default(
            provider,
            purpose,
            consumer,
            &credential.credential_id,
            chrono::Utc::now().timestamp(),
        )? {
            return Err(ManagedAuthCoreError::Conflict);
        }
        Ok(())
    }

    pub(crate) fn remove_compatibility_account(
        &self,
        provider: ManagedAuthProvider,
        legacy_account_id: &str,
    ) -> Result<(), ManagedAuthCoreError> {
        let (purpose, consumer) = purpose_for_provider(provider);
        let credential = self
            .repository
            .get_credential_by_legacy(provider, purpose, consumer, legacy_account_id)?
            .ok_or(ManagedAuthCoreError::NotFound)?;
        self.remove_credential_record(&credential)
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
                migration_id: Some(input.migration_id.to_string()),
                created_at: now,
                updated_at: now,
            },
        };
        let stored = self.repository.begin_provisioning(&new_credential)?;
        if matches!(
            stored.status,
            CredentialStatus::Ready | CredentialStatus::RequiresReauth
        ) {
            return Ok(stored);
        }
        let handle = stored.secret_handle.clone();
        let bundle = ManagedAuthSecretBundle::new(ManagedAuthSecretBundleParts {
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
        })?;
        match self.secrets.create_reserved(
            &handle,
            bundle.encode()?,
            SecretPurpose::ManagedOAuthCredential,
        ) {
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
            Err(error) => {
                if matches!(
                    error.code(),
                    SecretErrorCode::BackendUnavailable
                        | SecretErrorCode::Locked
                        | SecretErrorCode::PermissionDenied
                ) {
                    self.repository.set_status(
                        &stored.credential_id,
                        CredentialStatus::MigrationBlocked,
                        now,
                    )?;
                    return Err(ManagedAuthCoreError::SecretUnavailable);
                }
                return Err(error.into());
            }
        }
        self.readback_bundle(&handle)?;
        let marked = self.repository.mark_ready(
            &stored.credential_id,
            stored.generation,
            handle.version(),
            desired_status,
            now,
        )?;
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

    fn replace_bundle_cas_locked(
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
                let token = CodexOAuthManager::refresh_with_token(&refresh_token)
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
        Ok(ManagedAuthOverview {
            contract_version: MANAGED_AUTH_CONTRACT_VERSION,
            checked_at: now_timestamp(),
            providers: vec![
                provider_summary(ManagedAuthProvider::Openai, &fail),
                provider_summary(ManagedAuthProvider::Xai, &fail),
                provider_summary(ManagedAuthProvider::GithubCopilot, &fail),
            ],
            accounts,
            connections: merge_grok_connection(&rows, &connections),
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
            Err(_) => {
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
        ManagedAuthMutationResult {
            contract_version: MANAGED_AUTH_CONTRACT_VERSION,
            operation_id: uuid::Uuid::new_v4().to_string(),
            outcome,
            overview: self.overview(),
            pending_restart_consumers: Vec::new(),
            reason_code,
        }
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

fn map_refresh_error(
    error: crate::proxy::providers::codex_oauth_auth::CodexOAuthError,
) -> ManagedAuthCoreError {
    match error {
        crate::proxy::providers::codex_oauth_auth::CodexOAuthError::RefreshTokenInvalid => {
            ManagedAuthCoreError::InvalidData
        }
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
        allowed_actions.push(ManagedAuthAccountAction::SetDefault);
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
    let xai_ready =
        provider == ManagedAuthProvider::Xai && !fail.secret_unavailable && !fail.migration_blocked;
    if !xai_ready && reason_codes.is_empty() {
        reason_codes.push(ManagedAuthReasonCode::NativeProjectionUnavailable);
    }
    ManagedAuthProviderSummary {
        provider,
        available: xai_ready,
        login_methods: if xai_ready {
            vec![ManagedAuthLoginMethod::DeviceCode]
        } else {
            Vec::new()
        },
        consumers,
        reason_codes,
    }
}

fn merge_grok_connection(
    rows: &[CredentialWithIdentity],
    connections: &[ConnectionRecord],
) -> Vec<crate::services::managed_auth::ManagedAuthConnectionSummary> {
    let account = rows.iter().find(|row| {
        row.credential.purpose == CredentialPurpose::GrokNative
            && row.credential.status == CredentialStatus::Ready
    });
    let stored = connections
        .iter()
        .find(|connection| connection.consumer == ManagedAuthConsumer::Grokbuild);
    let mut summaries: Vec<_> = connections
        .iter()
        .filter(|connection| connection.consumer != ManagedAuthConsumer::Grokbuild)
        .map(|connection| connection_summary(connection, rows))
        .collect();
    summaries.insert(
        0,
        crate::services::managed_auth::consumers::grok::connection_summary(
            account,
            stored,
            now_timestamp(),
        ),
    );
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::secret::MemorySecretBackend;
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

    fn sample_input(legacy: &str, token: &str, make_default: bool) -> LegacyCredentialInput {
        LegacyCredentialInput {
            migration_id: CODEX_MIGRATION_ID,
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
}
