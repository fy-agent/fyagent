use crate::database::{lock_conn, Database};
use crate::error::AppError;
use crate::services::secret::{SecretHandle, SecretRef, SecretVersion};
use rusqlite::{params, OptionalExtension};

#[derive(Clone, Debug)]
pub(crate) struct ProviderCredentialRecord {
    pub(crate) id: String,
    pub(crate) provider_id: String,
    pub(crate) handle: SecretHandle,
    pub(crate) status: String,
}

fn decode(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProviderCredentialRecord> {
    let invalid = || rusqlite::Error::InvalidQuery;
    Ok(ProviderCredentialRecord {
        id: row.get(0)?,
        provider_id: row.get(1)?,
        handle: SecretHandle::new(
            SecretRef::parse(row.get::<_, String>(2)?).map_err(|_| invalid())?,
            SecretVersion::parse(row.get::<_, String>(3)?).map_err(|_| invalid())?,
        ),
        status: row.get(4)?,
    })
}

impl Database {
    pub(crate) fn admit_provider_credential(
        &self,
        record: &ProviderCredentialRecord,
    ) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        conn.execute(
            "INSERT INTO provider_credentials (credential_id, provider_id, secret_ref, secret_version, status) VALUES (?1, ?2, ?3, ?4, 'pending')",
            params![record.id, record.provider_id, record.handle.secret_ref().as_str(), record.handle.version().as_str()],
        ).map_err(|_| AppError::Database("provider_credential_admission_failed".into()))?;
        Ok(())
    }

    pub(crate) fn provider_credential(
        &self,
        id: &str,
        provider_id: &str,
    ) -> Result<Option<ProviderCredentialRecord>, AppError> {
        let conn = lock_conn!(self.conn);
        conn.query_row(
            "SELECT credential_id, provider_id, secret_ref, secret_version, status FROM provider_credentials WHERE credential_id = ?1 AND provider_id = ?2",
            params![id, provider_id], decode,
        ).optional().map_err(|_| AppError::Database("provider_credential_read_failed".into()))
    }

    pub(crate) fn provider_credential_records(
        &self,
    ) -> Result<Vec<ProviderCredentialRecord>, AppError> {
        let conn = lock_conn!(self.conn);
        let mut stmt = conn.prepare("SELECT credential_id, provider_id, secret_ref, secret_version, status FROM provider_credentials")
            .map_err(|_| AppError::Database("provider_credential_read_failed".into()))?;
        let records = stmt
            .query_map([], decode)
            .map_err(|_| AppError::Database("provider_credential_read_failed".into()))?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|_| AppError::Database("provider_credential_read_failed".into()))?;
        Ok(records)
    }

    pub(crate) fn set_provider_credential_status(
        &self,
        id: &str,
        status: &str,
    ) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        let changed = conn
            .execute(
                "UPDATE provider_credentials SET status = ?2 WHERE credential_id = ?1",
                params![id, status],
            )
            .map_err(|_| AppError::Database("provider_credential_write_failed".into()))?;
        if changed != 1 {
            return Err(AppError::Database("provider_credential_missing".into()));
        }
        Ok(())
    }
}
