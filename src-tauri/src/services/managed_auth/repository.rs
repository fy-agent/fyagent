use std::sync::Arc;

use crate::database::Database;

use super::{
    ConnectionRecord, CredentialPurpose, CredentialRecord, CredentialStatus,
    CredentialWithIdentity, ManagedAuthConsumer, ManagedAuthCoreError, ManagedAuthProvider,
    MigrationRecord, NewCredential, RefreshOwner,
};
use crate::services::secret::SecretVersion;

#[derive(Clone)]
pub(crate) struct ManagedAuthRepository {
    db: Arc<Database>,
}

impl ManagedAuthRepository {
    pub(crate) fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub(crate) fn begin_provisioning(
        &self,
        input: &NewCredential,
    ) -> Result<CredentialRecord, ManagedAuthCoreError> {
        self.db
            .managed_auth_begin_provisioning(input)
            .map_err(ManagedAuthCoreError::from)
    }

    pub(crate) fn mark_ready(
        &self,
        credential_id: &str,
        expected_generation: u64,
        secret_version: &SecretVersion,
        status: CredentialStatus,
        updated_at: i64,
    ) -> Result<bool, ManagedAuthCoreError> {
        self.db
            .managed_auth_mark_ready(
                credential_id,
                expected_generation,
                secret_version,
                status,
                updated_at,
            )
            .map_err(ManagedAuthCoreError::from)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn update_secret_cas(
        &self,
        credential_id: &str,
        expected_generation: u64,
        expected_owner: RefreshOwner,
        next_generation: u64,
        secret_version: &SecretVersion,
        access_expires_at: Option<i64>,
        status: CredentialStatus,
        refreshed_at: i64,
    ) -> Result<bool, ManagedAuthCoreError> {
        self.db
            .managed_auth_update_secret_cas(
                credential_id,
                expected_generation,
                expected_owner,
                next_generation,
                secret_version,
                access_expires_at,
                status,
                refreshed_at,
            )
            .map_err(ManagedAuthCoreError::from)
    }

    #[allow(dead_code)]
    pub(crate) fn reconcile_secret(
        &self,
        credential_id: &str,
        expected_generation: u64,
        observed_generation: u64,
        secret_version: &SecretVersion,
        status: CredentialStatus,
        updated_at: i64,
    ) -> Result<bool, ManagedAuthCoreError> {
        self.db
            .managed_auth_reconcile_secret(
                credential_id,
                expected_generation,
                observed_generation,
                secret_version,
                status,
                updated_at,
            )
            .map_err(ManagedAuthCoreError::from)
    }

    pub(crate) fn set_status(
        &self,
        credential_id: &str,
        status: CredentialStatus,
        updated_at: i64,
    ) -> Result<(), ManagedAuthCoreError> {
        self.db
            .managed_auth_set_status(credential_id, status, updated_at)
            .map_err(ManagedAuthCoreError::from)
    }

    pub(crate) fn get_credential(
        &self,
        credential_id: &str,
    ) -> Result<Option<CredentialRecord>, ManagedAuthCoreError> {
        self.db
            .managed_auth_get_credential(credential_id)
            .map_err(ManagedAuthCoreError::from)
    }

    pub(crate) fn get_credential_by_legacy(
        &self,
        provider: ManagedAuthProvider,
        purpose: CredentialPurpose,
        consumer: Option<ManagedAuthConsumer>,
        legacy_account_id: &str,
    ) -> Result<Option<CredentialRecord>, ManagedAuthCoreError> {
        self.db
            .managed_auth_get_credential_by_legacy(provider, purpose, consumer, legacy_account_id)
            .map_err(ManagedAuthCoreError::from)
    }

    pub(crate) fn list_credentials(
        &self,
        provider: ManagedAuthProvider,
        purpose: CredentialPurpose,
        consumer: Option<ManagedAuthConsumer>,
    ) -> Result<Vec<CredentialWithIdentity>, ManagedAuthCoreError> {
        self.db
            .managed_auth_list_credentials(provider, purpose, consumer)
            .map_err(ManagedAuthCoreError::from)
    }

    pub(crate) fn list_all_credentials(
        &self,
    ) -> Result<Vec<CredentialWithIdentity>, ManagedAuthCoreError> {
        self.db
            .managed_auth_list_all_credentials()
            .map_err(ManagedAuthCoreError::from)
    }

    pub(crate) fn set_default(
        &self,
        provider: ManagedAuthProvider,
        purpose: CredentialPurpose,
        consumer: Option<ManagedAuthConsumer>,
        credential_id: &str,
        updated_at: i64,
    ) -> Result<bool, ManagedAuthCoreError> {
        self.db
            .managed_auth_set_default(provider, purpose, consumer, credential_id, updated_at)
            .map_err(ManagedAuthCoreError::from)
    }

    pub(crate) fn get_default(
        &self,
        provider: ManagedAuthProvider,
        purpose: CredentialPurpose,
        consumer: Option<ManagedAuthConsumer>,
    ) -> Result<Option<String>, ManagedAuthCoreError> {
        self.db
            .managed_auth_get_default(provider, purpose, consumer)
            .map_err(ManagedAuthCoreError::from)
    }

    pub(crate) fn remove_credential(
        &self,
        credential_id: &str,
    ) -> Result<Option<CredentialRecord>, ManagedAuthCoreError> {
        self.db
            .managed_auth_remove_credential(credential_id)
            .map_err(ManagedAuthCoreError::from)
    }

    pub(crate) fn upsert_migration(
        &self,
        migration: &MigrationRecord,
    ) -> Result<(), ManagedAuthCoreError> {
        self.db
            .managed_auth_upsert_migration(migration)
            .map_err(ManagedAuthCoreError::from)
    }

    pub(crate) fn get_migration(
        &self,
        migration_id: &str,
    ) -> Result<Option<MigrationRecord>, ManagedAuthCoreError> {
        self.db
            .managed_auth_get_migration(migration_id)
            .map_err(ManagedAuthCoreError::from)
    }

    pub(crate) fn list_recoverable_credentials(
        &self,
    ) -> Result<Vec<CredentialRecord>, ManagedAuthCoreError> {
        self.db
            .managed_auth_list_provisioning()
            .map_err(ManagedAuthCoreError::from)
    }

    pub(crate) fn list_credentials_by_migration(
        &self,
        migration_id: &str,
    ) -> Result<Vec<CredentialRecord>, ManagedAuthCoreError> {
        self.db
            .managed_auth_list_credentials_by_migration(migration_id)
            .map_err(ManagedAuthCoreError::from)
    }

    #[allow(dead_code)]
    pub(crate) fn list_migrations(&self) -> Result<Vec<MigrationRecord>, ManagedAuthCoreError> {
        self.db
            .managed_auth_list_migrations()
            .map_err(ManagedAuthCoreError::from)
    }

    pub(crate) fn upsert_connection(
        &self,
        connection: &ConnectionRecord,
    ) -> Result<(), ManagedAuthCoreError> {
        self.db
            .managed_auth_upsert_connection(connection)
            .map_err(ManagedAuthCoreError::from)
    }

    pub(crate) fn list_connections(&self) -> Result<Vec<ConnectionRecord>, ManagedAuthCoreError> {
        self.db
            .managed_auth_list_connections()
            .map_err(ManagedAuthCoreError::from)
    }

    pub(crate) fn delete_connections_for_credential(
        &self,
        credential_id: &str,
    ) -> Result<(), ManagedAuthCoreError> {
        self.db
            .managed_auth_delete_connections_for_credential(credential_id)
            .map_err(ManagedAuthCoreError::from)
    }
}
