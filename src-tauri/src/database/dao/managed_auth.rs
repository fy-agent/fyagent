use crate::database::{lock_conn, Database};
use crate::error::AppError;
use crate::services::managed_auth::{
    ConnectionRecord, ConnectionStatus, CredentialPurpose, CredentialRecord, CredentialStatus,
    CredentialWithIdentity, IdentityRecord, ManagedAuthConsumer, ManagedAuthCoreError,
    ManagedAuthProvider, ManagedAuthRequestMode, MigrationRecord, MigrationStatus, NewCredential,
    RefreshOwner,
};
use crate::services::secret::{SecretHandle, SecretRef, SecretVersion};
use rusqlite::{params, Connection, OptionalExtension, Row};

fn database_error(context: &str, error: impl std::fmt::Display) -> AppError {
    AppError::Database(format!("{context}: {error}"))
}

fn invalid_row(column: usize) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        column,
        rusqlite::types::Type::Text,
        Box::new(ManagedAuthCoreError::InvalidData),
    )
}

fn parse_provider(value: String, column: usize) -> rusqlite::Result<ManagedAuthProvider> {
    ManagedAuthProvider::parse(&value).map_err(|_| invalid_row(column))
}

fn parse_purpose(value: String, column: usize) -> rusqlite::Result<CredentialPurpose> {
    CredentialPurpose::parse(&value).map_err(|_| invalid_row(column))
}

fn parse_consumer(value: String, column: usize) -> rusqlite::Result<Option<ManagedAuthConsumer>> {
    ManagedAuthConsumer::parse_optional(&value).map_err(|_| invalid_row(column))
}

fn parse_owner(value: String, column: usize) -> rusqlite::Result<RefreshOwner> {
    RefreshOwner::parse(&value).map_err(|_| invalid_row(column))
}

fn parse_status(value: String, column: usize) -> rusqlite::Result<CredentialStatus> {
    CredentialStatus::parse(&value).map_err(|_| invalid_row(column))
}

fn parse_migration_status(value: String, column: usize) -> rusqlite::Result<MigrationStatus> {
    MigrationStatus::parse(&value).map_err(|_| invalid_row(column))
}

fn parse_connection_status(value: String, column: usize) -> rusqlite::Result<ConnectionStatus> {
    ConnectionStatus::parse(&value).map_err(|_| invalid_row(column))
}

fn parse_request_mode(value: String, column: usize) -> rusqlite::Result<ManagedAuthRequestMode> {
    ManagedAuthRequestMode::parse(&value).map_err(|_| invalid_row(column))
}

fn parse_connection(row: &Row<'_>) -> rusqlite::Result<ConnectionRecord> {
    let official: Option<i64> = row.get(10)?;
    let pending: i64 = row.get(11)?;
    Ok(ConnectionRecord {
        connection_id: row.get(0)?,
        consumer: parse_consumer(row.get(1)?, 1)?.ok_or_else(|| invalid_row(1))?,
        target_id: row.get(2)?,
        provider_slot: row.get(3)?,
        credential_id: row.get(4)?,
        desired_revision: row.get(5)?,
        observed_revision: row.get(6)?,
        status: parse_connection_status(row.get(7)?, 7)?,
        request_mode: parse_request_mode(row.get(8)?, 8)?,
        request_provider_label: row.get(9)?,
        official_session_preserved: official.map(|value| value != 0),
        pending_restart: pending != 0,
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
    })
}

fn parse_identity(row: &Row<'_>, offset: usize) -> rusqlite::Result<IdentityRecord> {
    Ok(IdentityRecord {
        identity_id: row.get(offset)?,
        provider: parse_provider(row.get(offset + 1)?, offset + 1)?,
        provider_subject: row.get(offset + 2)?,
        provider_tenant: row.get(offset + 3)?,
        login: row.get(offset + 4)?,
        display_name: row.get(offset + 5)?,
        avatar_url: row.get(offset + 6)?,
        created_at: row.get(offset + 7)?,
        updated_at: row.get(offset + 8)?,
    })
}

fn parse_credential(row: &Row<'_>, offset: usize) -> rusqlite::Result<CredentialRecord> {
    let generation: i64 = row.get(offset + 9)?;
    if generation <= 0 {
        return Err(invalid_row(offset + 9));
    }
    let secret_ref =
        SecretRef::parse(row.get::<_, String>(offset + 6)?).map_err(|_| invalid_row(offset + 6))?;
    let secret_version = SecretVersion::parse(row.get::<_, String>(offset + 7)?)
        .map_err(|_| invalid_row(offset + 7))?;
    Ok(CredentialRecord {
        credential_id: row.get(offset)?,
        identity_id: row.get(offset + 1)?,
        provider: parse_provider(row.get(offset + 2)?, offset + 2)?,
        purpose: parse_purpose(row.get(offset + 3)?, offset + 3)?,
        consumer: parse_consumer(row.get(offset + 4)?, offset + 4)?,
        legacy_account_id: row.get(offset + 5)?,
        secret_handle: SecretHandle::new(secret_ref, secret_version),
        refresh_owner: parse_owner(row.get(offset + 8)?, offset + 8)?,
        generation: generation as u64,
        access_expires_at: row.get(offset + 10)?,
        status: parse_status(row.get(offset + 11)?, offset + 11)?,
        authenticated_at: row.get(offset + 12)?,
        refreshed_at: row.get(offset + 13)?,
        migration_id: row.get(offset + 14)?,
        created_at: row.get(offset + 15)?,
        updated_at: row.get(offset + 16)?,
    })
}

impl Database {
    pub(crate) fn managed_auth_begin_provisioning(
        &self,
        input: &NewCredential,
    ) -> Result<CredentialRecord, AppError> {
        let mut conn = lock_conn!(self.conn);
        let tx = conn
            .transaction()
            .map_err(|error| database_error("start managed auth provisioning", error))?;
        Self::managed_auth_upsert_identity_on_conn(&tx, &input.identity)?;

        let existing = Self::managed_auth_get_credential_by_legacy_on_conn(
            &tx,
            input.credential.provider,
            input.credential.purpose,
            input.credential.consumer,
            &input.credential.legacy_account_id,
        )?;
        if let Some(existing) = existing {
            if existing.credential_id != input.credential.credential_id
                || existing.identity_id != input.credential.identity_id
            {
                return Err(AppError::Database(
                    "managed auth credential identity conflict".to_string(),
                ));
            }
            tx.commit()
                .map_err(|error| database_error("commit idempotent provisioning", error))?;
            return Ok(existing);
        }

        let credential = &input.credential;
        tx.execute(
            "INSERT INTO managed_auth_credentials (
                credential_id, identity_id, provider, purpose, consumer,
                legacy_account_id, secret_ref, secret_version, refresh_owner,
                generation, access_expires_at, status, authenticated_at,
                refreshed_at, migration_id, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
            params![
                credential.credential_id,
                credential.identity_id,
                credential.provider.as_str(),
                credential.purpose.as_str(),
                credential
                    .consumer
                    .map(ManagedAuthConsumer::as_str)
                    .unwrap_or(""),
                credential.legacy_account_id,
                credential.secret_handle.secret_ref().as_str(),
                credential.secret_handle.version().as_str(),
                credential.refresh_owner.as_str(),
                i64::try_from(credential.generation).map_err(|_| AppError::Database(
                    "managed auth generation overflow".to_string()
                ))?,
                credential.access_expires_at,
                credential.status.as_str(),
                credential.authenticated_at,
                credential.refreshed_at,
                credential.migration_id,
                credential.created_at,
                credential.updated_at,
            ],
        )
        .map_err(|error| database_error("insert managed auth credential", error))?;

        tx.commit()
            .map_err(|error| database_error("commit managed auth provisioning", error))?;
        Ok(credential.clone())
    }

    fn managed_auth_upsert_identity_on_conn(
        conn: &Connection,
        identity: &IdentityRecord,
    ) -> Result<(), AppError> {
        let existing: Option<String> = conn
            .query_row(
                "SELECT identity_id FROM managed_auth_identities
                 WHERE provider = ?1 AND provider_subject = ?2 AND provider_tenant = ?3",
                params![
                    identity.provider.as_str(),
                    identity.provider_subject,
                    identity.provider_tenant,
                ],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| database_error("read managed auth identity", error))?;
        if existing
            .as_deref()
            .is_some_and(|value| value != identity.identity_id)
        {
            return Err(AppError::Database(
                "managed auth stable identity mismatch".to_string(),
            ));
        }
        conn.execute(
            "INSERT INTO managed_auth_identities (
                identity_id, provider, provider_subject, provider_tenant, login,
                display_name, avatar_url, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(identity_id) DO UPDATE SET
                login = excluded.login,
                display_name = excluded.display_name,
                avatar_url = excluded.avatar_url,
                updated_at = excluded.updated_at",
            params![
                identity.identity_id,
                identity.provider.as_str(),
                identity.provider_subject,
                identity.provider_tenant,
                identity.login,
                identity.display_name,
                identity.avatar_url,
                identity.created_at,
                identity.updated_at,
            ],
        )
        .map_err(|error| database_error("upsert managed auth identity", error))?;
        Ok(())
    }

    pub(crate) fn managed_auth_mark_ready(
        &self,
        credential_id: &str,
        expected_generation: u64,
        secret_version: &SecretVersion,
        status: CredentialStatus,
        updated_at: i64,
    ) -> Result<bool, AppError> {
        let generation = i64::try_from(expected_generation)
            .map_err(|_| AppError::Database("managed auth generation overflow".to_string()))?;
        let conn = lock_conn!(self.conn);
        let updated = conn
            .execute(
                "UPDATE managed_auth_credentials
                 SET secret_version = ?3, status = ?4, updated_at = ?5
                 WHERE credential_id = ?1 AND generation = ?2
                   AND status IN ('provisioning','migration_blocked','secret_missing')",
                params![
                    credential_id,
                    generation,
                    secret_version.as_str(),
                    status.as_str(),
                    updated_at,
                ],
            )
            .map_err(|error| database_error("mark managed auth credential ready", error))?;
        Ok(updated == 1)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn managed_auth_update_secret_cas(
        &self,
        credential_id: &str,
        expected_generation: u64,
        expected_owner: RefreshOwner,
        next_generation: u64,
        secret_version: &SecretVersion,
        access_expires_at: Option<i64>,
        status: CredentialStatus,
        refreshed_at: i64,
    ) -> Result<bool, AppError> {
        let expected_generation = i64::try_from(expected_generation)
            .map_err(|_| AppError::Database("managed auth generation overflow".to_string()))?;
        let next_generation = i64::try_from(next_generation)
            .map_err(|_| AppError::Database("managed auth generation overflow".to_string()))?;
        let conn = lock_conn!(self.conn);
        let updated = conn
            .execute(
                "UPDATE managed_auth_credentials
                 SET secret_version = ?4, generation = ?5, access_expires_at = ?6,
                     status = ?7, refreshed_at = ?8, updated_at = ?8
                 WHERE credential_id = ?1 AND generation = ?2 AND refresh_owner = ?3",
                params![
                    credential_id,
                    expected_generation,
                    expected_owner.as_str(),
                    secret_version.as_str(),
                    next_generation,
                    access_expires_at,
                    status.as_str(),
                    refreshed_at,
                ],
            )
            .map_err(|error| database_error("update managed auth secret metadata", error))?;
        Ok(updated == 1)
    }

    pub(crate) fn managed_auth_transfer_refresh_owner(
        &self,
        credential_id: &str,
        expected_generation: u64,
        from: RefreshOwner,
        to: RefreshOwner,
        updated_at: i64,
    ) -> Result<bool, AppError> {
        let expected_generation = i64::try_from(expected_generation)
            .map_err(|_| AppError::Database("managed auth generation overflow".to_string()))?;
        let conn = lock_conn!(self.conn);
        let updated = conn
            .execute(
                "UPDATE managed_auth_credentials
                 SET refresh_owner = ?4, updated_at = ?5
                 WHERE credential_id = ?1 AND generation = ?2 AND refresh_owner = ?3",
                params![
                    credential_id,
                    expected_generation,
                    from.as_str(),
                    to.as_str(),
                    updated_at,
                ],
            )
            .map_err(|error| database_error("transfer managed auth refresh owner", error))?;
        Ok(updated == 1)
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn managed_auth_reconcile_secret(
        &self,
        credential_id: &str,
        expected_generation: u64,
        observed_generation: u64,
        secret_version: &SecretVersion,
        status: CredentialStatus,
        updated_at: i64,
    ) -> Result<bool, AppError> {
        let expected_generation = i64::try_from(expected_generation)
            .map_err(|_| AppError::Database("managed auth generation overflow".to_string()))?;
        let observed_generation = i64::try_from(observed_generation)
            .map_err(|_| AppError::Database("managed auth generation overflow".to_string()))?;
        if observed_generation < expected_generation {
            return Ok(false);
        }
        let conn = lock_conn!(self.conn);
        // Observed generations may advance metadata, but this path never
        // changes refresh_owner. Secrets live in the OS vault, not SQLite.
        let updated = conn
            .execute(
                "UPDATE managed_auth_credentials
                 SET secret_version = ?4, generation = ?3, status = ?5,
                     updated_at = ?6
                 WHERE credential_id = ?1 AND generation = ?2",
                params![
                    credential_id,
                    expected_generation,
                    observed_generation,
                    secret_version.as_str(),
                    status.as_str(),
                    updated_at,
                ],
            )
            .map_err(|error| database_error("reconcile managed auth secret metadata", error))?;
        Ok(updated == 1)
    }

    pub(crate) fn managed_auth_set_status(
        &self,
        credential_id: &str,
        status: CredentialStatus,
        updated_at: i64,
    ) -> Result<(), AppError> {
        if matches!(status, CredentialStatus::Ready) {
            return Err(AppError::Database(
                "managed auth ready status requires generation-aware mark_ready".to_string(),
            ));
        }
        let conn = lock_conn!(self.conn);
        conn.execute(
            "UPDATE managed_auth_credentials SET status = ?2, updated_at = ?3
             WHERE credential_id = ?1",
            params![credential_id, status.as_str(), updated_at],
        )
        .map_err(|error| database_error("update managed auth credential status", error))?;
        Ok(())
    }

    pub(crate) fn managed_auth_get_credential(
        &self,
        credential_id: &str,
    ) -> Result<Option<CredentialRecord>, AppError> {
        let conn = lock_conn!(self.conn);
        Self::managed_auth_get_credential_on_conn(&conn, credential_id)
    }

    fn managed_auth_get_credential_on_conn(
        conn: &Connection,
        credential_id: &str,
    ) -> Result<Option<CredentialRecord>, AppError> {
        conn.query_row(
            "SELECT credential_id, identity_id, provider, purpose, consumer,
                    legacy_account_id, secret_ref, secret_version, refresh_owner,
                    generation, access_expires_at, status, authenticated_at,
                    refreshed_at, migration_id, created_at, updated_at
             FROM managed_auth_credentials WHERE credential_id = ?1",
            params![credential_id],
            |row| parse_credential(row, 0),
        )
        .optional()
        .map_err(|error| database_error("read managed auth credential", error))
    }

    pub(crate) fn managed_auth_get_credential_by_legacy(
        &self,
        provider: ManagedAuthProvider,
        purpose: CredentialPurpose,
        consumer: Option<ManagedAuthConsumer>,
        legacy_account_id: &str,
    ) -> Result<Option<CredentialRecord>, AppError> {
        let conn = lock_conn!(self.conn);
        Self::managed_auth_get_credential_by_legacy_on_conn(
            &conn,
            provider,
            purpose,
            consumer,
            legacy_account_id,
        )
    }

    fn managed_auth_get_credential_by_legacy_on_conn(
        conn: &Connection,
        provider: ManagedAuthProvider,
        purpose: CredentialPurpose,
        consumer: Option<ManagedAuthConsumer>,
        legacy_account_id: &str,
    ) -> Result<Option<CredentialRecord>, AppError> {
        conn.query_row(
            "SELECT credential_id, identity_id, provider, purpose, consumer,
                    legacy_account_id, secret_ref, secret_version, refresh_owner,
                    generation, access_expires_at, status, authenticated_at,
                    refreshed_at, migration_id, created_at, updated_at
             FROM managed_auth_credentials
             WHERE provider = ?1 AND purpose = ?2 AND consumer = ?3
               AND legacy_account_id = ?4",
            params![
                provider.as_str(),
                purpose.as_str(),
                consumer.map(ManagedAuthConsumer::as_str).unwrap_or(""),
                legacy_account_id,
            ],
            |row| parse_credential(row, 0),
        )
        .optional()
        .map_err(|error| database_error("read managed auth legacy credential", error))
    }

    pub(crate) fn managed_auth_list_credentials(
        &self,
        provider: ManagedAuthProvider,
        purpose: CredentialPurpose,
        consumer: Option<ManagedAuthConsumer>,
    ) -> Result<Vec<CredentialWithIdentity>, AppError> {
        let conn = lock_conn!(self.conn);
        let default_id =
            Self::managed_auth_get_default_on_conn(&conn, provider, purpose, consumer)?;
        let mut statement = conn
            .prepare(
                "SELECT
                    c.credential_id, c.identity_id, c.provider, c.purpose, c.consumer,
                    c.legacy_account_id, c.secret_ref, c.secret_version, c.refresh_owner,
                    c.generation, c.access_expires_at, c.status, c.authenticated_at,
                    c.refreshed_at, c.migration_id, c.created_at, c.updated_at,
                    i.identity_id, i.provider, i.provider_subject, i.provider_tenant,
                    i.login, i.display_name, i.avatar_url, i.created_at, i.updated_at
                 FROM managed_auth_credentials c
                 JOIN managed_auth_identities i ON i.identity_id = c.identity_id
                 WHERE c.provider = ?1 AND c.purpose = ?2 AND c.consumer = ?3
                 ORDER BY c.authenticated_at DESC, c.credential_id ASC",
            )
            .map_err(|error| database_error("prepare managed auth credential list", error))?;
        let rows = statement
            .query_map(
                params![
                    provider.as_str(),
                    purpose.as_str(),
                    consumer.map(ManagedAuthConsumer::as_str).unwrap_or(""),
                ],
                |row| {
                    let credential = parse_credential(row, 0)?;
                    let identity = parse_identity(row, 17)?;
                    Ok(CredentialWithIdentity {
                        is_default: default_id.as_deref()
                            == Some(credential.credential_id.as_str()),
                        credential,
                        identity,
                    })
                },
            )
            .map_err(|error| database_error("query managed auth credential list", error))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| database_error("decode managed auth credential list", error))
    }

    pub(crate) fn managed_auth_list_all_credentials(
        &self,
    ) -> Result<Vec<CredentialWithIdentity>, AppError> {
        let conn = lock_conn!(self.conn);
        let mut statement = conn
            .prepare(
                "SELECT
                    c.credential_id, c.identity_id, c.provider, c.purpose, c.consumer,
                    c.legacy_account_id, c.secret_ref, c.secret_version, c.refresh_owner,
                    c.generation, c.access_expires_at, c.status, c.authenticated_at,
                    c.refreshed_at, c.migration_id, c.created_at, c.updated_at,
                    i.identity_id, i.provider, i.provider_subject, i.provider_tenant,
                    i.login, i.display_name, i.avatar_url, i.created_at, i.updated_at,
                    EXISTS(
                      SELECT 1 FROM managed_auth_defaults d
                      WHERE d.credential_id = c.credential_id
                    )
                 FROM managed_auth_credentials c
                 JOIN managed_auth_identities i ON i.identity_id = c.identity_id
                 ORDER BY i.provider, i.login, c.authenticated_at DESC",
            )
            .map_err(|error| database_error("prepare managed auth overview", error))?;
        let rows = statement
            .query_map([], |row| {
                Ok(CredentialWithIdentity {
                    credential: parse_credential(row, 0)?,
                    identity: parse_identity(row, 17)?,
                    is_default: row.get::<_, i64>(26)? != 0,
                })
            })
            .map_err(|error| database_error("query managed auth overview", error))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| database_error("decode managed auth overview", error))
    }

    pub(crate) fn managed_auth_set_default(
        &self,
        provider: ManagedAuthProvider,
        purpose: CredentialPurpose,
        consumer: Option<ManagedAuthConsumer>,
        credential_id: &str,
        updated_at: i64,
    ) -> Result<bool, AppError> {
        let conn = lock_conn!(self.conn);
        let credential = Self::managed_auth_get_credential_on_conn(&conn, credential_id)?;
        let Some(credential) = credential else {
            return Ok(false);
        };
        if credential.provider != provider
            || credential.purpose != purpose
            || credential.consumer != consumer
            || !matches!(credential.status, CredentialStatus::Ready)
        {
            return Ok(false);
        }
        Self::managed_auth_set_default_on_conn(
            &conn,
            provider,
            purpose,
            consumer,
            credential_id,
            updated_at,
        )?;
        Ok(true)
    }

    fn managed_auth_set_default_on_conn(
        conn: &Connection,
        provider: ManagedAuthProvider,
        purpose: CredentialPurpose,
        consumer: Option<ManagedAuthConsumer>,
        credential_id: &str,
        updated_at: i64,
    ) -> Result<(), AppError> {
        conn.execute(
            "INSERT INTO managed_auth_defaults (
                provider, purpose, consumer, credential_id, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(provider, purpose, consumer) DO UPDATE SET
                credential_id = excluded.credential_id,
                updated_at = excluded.updated_at",
            params![
                provider.as_str(),
                purpose.as_str(),
                consumer.map(ManagedAuthConsumer::as_str).unwrap_or(""),
                credential_id,
                updated_at,
            ],
        )
        .map_err(|error| database_error("set managed auth default", error))?;
        Ok(())
    }

    pub(crate) fn managed_auth_get_default(
        &self,
        provider: ManagedAuthProvider,
        purpose: CredentialPurpose,
        consumer: Option<ManagedAuthConsumer>,
    ) -> Result<Option<String>, AppError> {
        let conn = lock_conn!(self.conn);
        Self::managed_auth_get_default_on_conn(&conn, provider, purpose, consumer)
    }

    fn managed_auth_get_default_on_conn(
        conn: &Connection,
        provider: ManagedAuthProvider,
        purpose: CredentialPurpose,
        consumer: Option<ManagedAuthConsumer>,
    ) -> Result<Option<String>, AppError> {
        conn.query_row(
            "SELECT credential_id FROM managed_auth_defaults
             WHERE provider = ?1 AND purpose = ?2 AND consumer = ?3",
            params![
                provider.as_str(),
                purpose.as_str(),
                consumer.map(ManagedAuthConsumer::as_str).unwrap_or(""),
            ],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| database_error("read managed auth default", error))
    }

    pub(crate) fn managed_auth_remove_credential(
        &self,
        credential_id: &str,
    ) -> Result<Option<CredentialRecord>, AppError> {
        let mut conn = lock_conn!(self.conn);
        let tx = conn
            .transaction()
            .map_err(|error| database_error("start managed auth removal", error))?;
        let credential = Self::managed_auth_get_credential_on_conn(&tx, credential_id)?;
        let Some(credential) = credential else {
            return Ok(None);
        };
        tx.execute(
            "DELETE FROM managed_auth_credentials WHERE credential_id = ?1",
            params![credential_id],
        )
        .map_err(|error| database_error("delete managed auth credential", error))?;
        tx.execute(
            "DELETE FROM managed_auth_identities
             WHERE identity_id = ?1
               AND NOT EXISTS (
                 SELECT 1 FROM managed_auth_credentials WHERE identity_id = ?1
               )",
            params![credential.identity_id],
        )
        .map_err(|error| database_error("prune managed auth identity", error))?;
        tx.commit()
            .map_err(|error| database_error("commit managed auth removal", error))?;
        Ok(Some(credential))
    }

    pub(crate) fn managed_auth_upsert_migration(
        &self,
        migration: &MigrationRecord,
    ) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        conn.execute(
            "INSERT INTO managed_auth_migrations (
                migration_id, source_kind, source_hash, status, reason_code,
                backup_name, created_at, updated_at, completed_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(migration_id) DO UPDATE SET
                source_hash = excluded.source_hash,
                status = excluded.status,
                reason_code = excluded.reason_code,
                backup_name = excluded.backup_name,
                updated_at = excluded.updated_at,
                completed_at = excluded.completed_at",
            params![
                migration.migration_id,
                migration.source_kind,
                migration.source_hash,
                migration.status.as_str(),
                migration.reason_code,
                migration.backup_name,
                migration.created_at,
                migration.updated_at,
                migration.completed_at,
            ],
        )
        .map_err(|error| database_error("upsert managed auth migration", error))?;
        Ok(())
    }

    pub(crate) fn managed_auth_get_migration(
        &self,
        migration_id: &str,
    ) -> Result<Option<MigrationRecord>, AppError> {
        let conn = lock_conn!(self.conn);
        conn.query_row(
            "SELECT migration_id, source_kind, source_hash, status, reason_code,
                    backup_name, created_at, updated_at, completed_at
             FROM managed_auth_migrations WHERE migration_id = ?1",
            params![migration_id],
            |row| {
                Ok(MigrationRecord {
                    migration_id: row.get(0)?,
                    source_kind: row.get(1)?,
                    source_hash: row.get(2)?,
                    status: parse_migration_status(row.get(3)?, 3)?,
                    reason_code: row.get(4)?,
                    backup_name: row.get(5)?,
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
                    completed_at: row.get(8)?,
                })
            },
        )
        .optional()
        .map_err(|error| database_error("read managed auth migration", error))
    }

    pub(crate) fn managed_auth_list_provisioning(&self) -> Result<Vec<CredentialRecord>, AppError> {
        let conn = lock_conn!(self.conn);
        let mut statement = conn
            .prepare(
                "SELECT credential_id, identity_id, provider, purpose, consumer,
                        legacy_account_id, secret_ref, secret_version, refresh_owner,
                        generation, access_expires_at, status, authenticated_at,
                        refreshed_at, migration_id, created_at, updated_at
                 FROM managed_auth_credentials
                 WHERE status IN ('provisioning','secret_missing','migration_blocked')
                 ORDER BY created_at, credential_id",
            )
            .map_err(|error| database_error("prepare managed auth recovery", error))?;
        let rows = statement
            .query_map([], |row| parse_credential(row, 0))
            .map_err(|error| database_error("query managed auth recovery", error))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| database_error("decode managed auth recovery", error))
    }

    pub(crate) fn managed_auth_list_credentials_by_migration(
        &self,
        migration_id: &str,
    ) -> Result<Vec<CredentialRecord>, AppError> {
        let conn = lock_conn!(self.conn);
        let mut statement = conn
            .prepare(
                "SELECT credential_id, identity_id, provider, purpose, consumer,
                        legacy_account_id, secret_ref, secret_version, refresh_owner,
                        generation, access_expires_at, status, authenticated_at,
                        refreshed_at, migration_id, created_at, updated_at
                 FROM managed_auth_credentials
                 WHERE migration_id = ?1
                 ORDER BY created_at, credential_id",
            )
            .map_err(|error| database_error("prepare managed auth migration credentials", error))?;
        let rows = statement
            .query_map(params![migration_id], |row| parse_credential(row, 0))
            .map_err(|error| database_error("query managed auth migration credentials", error))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| database_error("decode managed auth migration credentials", error))
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn managed_auth_list_migrations(&self) -> Result<Vec<MigrationRecord>, AppError> {
        let conn = lock_conn!(self.conn);
        let mut statement = conn
            .prepare(
                "SELECT migration_id, source_kind, source_hash, status, reason_code,
                        backup_name, created_at, updated_at, completed_at
                 FROM managed_auth_migrations
                 ORDER BY created_at, migration_id",
            )
            .map_err(|error| database_error("prepare managed auth migrations", error))?;
        let rows = statement
            .query_map([], |row| {
                Ok(MigrationRecord {
                    migration_id: row.get(0)?,
                    source_kind: row.get(1)?,
                    source_hash: row.get(2)?,
                    status: parse_migration_status(row.get(3)?, 3)?,
                    reason_code: row.get(4)?,
                    backup_name: row.get(5)?,
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
                    completed_at: row.get(8)?,
                })
            })
            .map_err(|error| database_error("query managed auth migrations", error))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| database_error("decode managed auth migrations", error))
    }

    pub(crate) fn managed_auth_upsert_connection(
        &self,
        connection: &ConnectionRecord,
    ) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        conn.execute(
            "INSERT INTO managed_auth_connections (
                connection_id, consumer, target_id, provider_slot, credential_id,
                desired_revision, observed_revision, status, request_mode,
                request_provider_label, official_session_preserved, pending_restart,
                created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
             ON CONFLICT(consumer, target_id, provider_slot) DO UPDATE SET
                connection_id = excluded.connection_id,
                credential_id = excluded.credential_id,
                desired_revision = excluded.desired_revision,
                observed_revision = excluded.observed_revision,
                status = excluded.status,
                request_mode = excluded.request_mode,
                request_provider_label = excluded.request_provider_label,
                official_session_preserved = excluded.official_session_preserved,
                pending_restart = excluded.pending_restart,
                updated_at = excluded.updated_at",
            params![
                connection.connection_id,
                connection.consumer.as_str(),
                connection.target_id,
                connection.provider_slot,
                connection.credential_id,
                connection.desired_revision,
                connection.observed_revision,
                connection.status.as_str(),
                connection.request_mode.as_str(),
                connection.request_provider_label,
                connection
                    .official_session_preserved
                    .map(|value| if value { 1 } else { 0 }),
                if connection.pending_restart { 1 } else { 0 },
                connection.created_at,
                connection.updated_at,
            ],
        )
        .map_err(|error| database_error("upsert managed auth connection", error))?;
        Ok(())
    }

    pub(crate) fn managed_auth_list_connections(&self) -> Result<Vec<ConnectionRecord>, AppError> {
        let conn = lock_conn!(self.conn);
        let mut statement = conn
            .prepare(
                "SELECT connection_id, consumer, target_id, provider_slot, credential_id,
                        desired_revision, observed_revision, status, request_mode,
                        request_provider_label, official_session_preserved, pending_restart,
                        created_at, updated_at
                 FROM managed_auth_connections
                 ORDER BY consumer, provider_slot, connection_id",
            )
            .map_err(|error| database_error("prepare managed auth connections", error))?;
        let rows = statement
            .query_map([], parse_connection)
            .map_err(|error| database_error("query managed auth connections", error))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| database_error("decode managed auth connections", error))
    }

    pub(crate) fn managed_auth_delete_connections_for_credential(
        &self,
        credential_id: &str,
    ) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        conn.execute(
            "DELETE FROM managed_auth_connections WHERE credential_id = ?1",
            params![credential_id],
        )
        .map_err(|error| database_error("delete managed auth connections", error))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::services::managed_auth::{
        stable_credential_id, stable_identity_id, ManagedAuthProvider,
    };
    use crate::services::secret::{SecretRef, SecretVersion};

    use super::*;

    fn new_credential(db: &Database) -> NewCredential {
        let now = 1_750_000_000;
        let provider = ManagedAuthProvider::Openai;
        let purpose = CredentialPurpose::ProxyUpstream;
        let consumer = Some(ManagedAuthConsumer::FyagentProxy);
        let legacy_account_id = "legacy-account".to_string();
        let identity_id = stable_identity_id(provider, "subject", "");
        let credential_id = stable_credential_id(provider, purpose, consumer, &legacy_account_id);
        let _ = db;
        NewCredential {
            identity: IdentityRecord {
                identity_id: identity_id.clone(),
                provider,
                provider_subject: "subject".to_string(),
                provider_tenant: String::new(),
                login: "person@example.com".to_string(),
                display_name: None,
                avatar_url: None,
                created_at: now,
                updated_at: now,
            },
            credential: CredentialRecord {
                credential_id,
                identity_id,
                provider,
                purpose,
                consumer,
                legacy_account_id,
                secret_handle: SecretHandle::new(SecretRef::generate(), SecretVersion::generate()),
                refresh_owner: RefreshOwner::Fyagent,
                generation: 1,
                access_expires_at: None,
                status: CredentialStatus::Provisioning,
                authenticated_at: now,
                refreshed_at: None,
                migration_id: Some("migration".to_string()),
                created_at: now,
                updated_at: now,
            },
        }
    }

    #[test]
    fn provisioning_is_idempotent_and_does_not_set_default() {
        let db = Database::memory().expect("db");
        let input = new_credential(&db);
        let first = db
            .managed_auth_begin_provisioning(&input)
            .expect("provision");
        let second = db
            .managed_auth_begin_provisioning(&input)
            .expect("idempotent");
        assert_eq!(first.credential_id, second.credential_id);
        assert_eq!(first.status, CredentialStatus::Provisioning);
        assert_eq!(
            db.managed_auth_get_default(
                input.credential.provider,
                input.credential.purpose,
                input.credential.consumer,
            )
            .expect("default"),
            None
        );
    }

    #[test]
    fn default_is_explicit_after_ready() {
        let db = Database::memory().expect("db");
        let input = new_credential(&db);
        let credential_id = input.credential.credential_id.clone();
        let version = input.credential.secret_handle.version().clone();
        db.managed_auth_begin_provisioning(&input)
            .expect("provision");
        assert!(db
            .managed_auth_mark_ready(
                &credential_id,
                1,
                &version,
                CredentialStatus::Ready,
                1_750_000_001,
            )
            .expect("ready"));
        assert!(db
            .managed_auth_set_default(
                input.credential.provider,
                input.credential.purpose,
                input.credential.consumer,
                &credential_id,
                1_750_000_002,
            )
            .expect("set default"));
        assert_eq!(
            db.managed_auth_get_default(
                input.credential.provider,
                input.credential.purpose,
                input.credential.consumer,
            )
            .expect("default")
            .as_deref(),
            Some(credential_id.as_str())
        );
    }

    #[test]
    fn reconcile_secret_rejects_stale_generation_and_keeps_refresh_owner() {
        let db = Database::memory().expect("db");
        let input = new_credential(&db);
        let credential_id = input.credential.credential_id.clone();
        let version = input.credential.secret_handle.version().clone();
        db.managed_auth_begin_provisioning(&input)
            .expect("provision");
        assert!(db
            .managed_auth_mark_ready(
                &credential_id,
                1,
                &version,
                CredentialStatus::Ready,
                1_750_000_001,
            )
            .expect("ready"));
        assert!(!db
            .managed_auth_reconcile_secret(
                &credential_id,
                2,
                1,
                &version,
                CredentialStatus::Ready,
                1_750_000_002,
            )
            .expect("stale"));
        let current = db
            .managed_auth_get_credential(&credential_id)
            .expect("read")
            .expect("present");
        assert_eq!(current.generation, 1);
        assert_eq!(current.refresh_owner, RefreshOwner::Fyagent);
    }

    #[test]
    fn list_migrations_returns_upserted_rows() {
        let db = Database::memory().expect("db");
        db.managed_auth_upsert_migration(&MigrationRecord {
            migration_id: "legacy-codex-oauth-v2".to_string(),
            source_kind: "codex_oauth_v2".to_string(),
            source_hash: "abc".to_string(),
            status: MigrationStatus::Prepared,
            reason_code: None,
            backup_name: None,
            created_at: 1,
            updated_at: 1,
            completed_at: None,
        })
        .expect("upsert");
        let rows = db.managed_auth_list_migrations().expect("list");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].migration_id, "legacy-codex-oauth-v2");
    }
}
