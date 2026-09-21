//! Device-scoped view over the receipt table.
//!
//! Every read revalidates the stored binding against this installation and the
//! row's own target store. A receipt that arrived through a restored SQL dump
//! or a copied database describes a conversation in some other machine's
//! provider store; acting on it would reconcile against the wrong store or
//! silently suppress a legitimate local import.
//!
//! Foreign rows are not deleted. They are filtered out and can still be shown
//! as "a receipt from another device", which is a real thing a user may want
//! to see after restoring a backup.

use crate::database::dao::session_restore::{AttemptClaim, RestoreAttemptRecord};
use crate::database::Database;
use crate::error::AppError;

use super::identity;
use super::model::{
    MigrationError, MigrationResult, RestoreAttempt, RestoreStage, UserAttestation,
};

pub struct ReceiptStore<'a> {
    db: &'a Database,
}

fn store_failed(error: AppError) -> MigrationError {
    MigrationError::ReceiptStoreFailed {
        reason: error.to_string(),
    }
}

impl<'a> ReceiptStore<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Binding for one target store on this machine.
    pub fn binding_for(&self, target_store_id: &str) -> MigrationResult<String> {
        identity::device_binding(target_store_id)
    }

    /// A row belongs here only when its stored binding still equals the
    /// binding derived from this installation plus the row's own target store.
    fn belongs_here(&self, record: &RestoreAttemptRecord) -> bool {
        match self.binding_for(&record.attempt.target_store_id) {
            Ok(expected) => expected == record.device_binding,
            Err(_) => false,
        }
    }

    fn local_only(&self, records: Vec<RestoreAttemptRecord>) -> Vec<RestoreAttempt> {
        records
            .into_iter()
            .filter(|record| self.belongs_here(record))
            .map(|record| record.attempt)
            .collect()
    }

    #[cfg(test)]
    pub fn claim(&self, attempt: RestoreAttempt) -> MigrationResult<AttemptClaim> {
        let snapshot_ids = vec![attempt.snapshot_id.clone()];
        self.claim_for_request(attempt, &snapshot_ids)
    }

    #[cfg(test)]
    pub fn claim_for_request(
        &self,
        attempt: RestoreAttempt,
        snapshot_ids: &[String],
    ) -> MigrationResult<AttemptClaim> {
        let record = self.record_for_request(attempt, snapshot_ids)?;
        self.db.claim_restore_attempt(&record)
    }

    pub fn claim_batch(
        &self,
        attempts: Vec<RestoreAttempt>,
        snapshot_ids: &[String],
    ) -> MigrationResult<Vec<AttemptClaim>> {
        let requested: std::collections::BTreeSet<_> = snapshot_ids.iter().collect();
        let actual: std::collections::BTreeSet<_> = attempts
            .iter()
            .map(|attempt| &attempt.snapshot_id)
            .collect();
        if attempts.is_empty() || actual != requested || actual.len() != attempts.len() {
            return Err(MigrationError::IdentityFieldInvalid {
                field: "snapshotIds".into(),
                reason: "batch rows must exactly match a nonempty snapshot selection".into(),
            });
        }
        let records = attempts
            .into_iter()
            .map(|attempt| self.record_for_request(attempt, snapshot_ids))
            .collect::<MigrationResult<Vec<_>>>()?;
        self.db.claim_restore_attempts(&records)
    }

    fn record_for_request(
        &self,
        attempt: RestoreAttempt,
        snapshot_ids: &[String],
    ) -> MigrationResult<RestoreAttemptRecord> {
        let mut selected = snapshot_ids.to_vec();
        selected.sort();
        selected.dedup();
        let fingerprint_bytes = serde_json::to_vec(&(
            "fyagent.restore.request.v1",
            &attempt.target_provider_id,
            &attempt.target_store_id,
            &attempt.target_workspace,
            attempt.request_kind.as_str(),
            selected,
        ))
        .map_err(|error| MigrationError::ReceiptStoreFailed {
            reason: error.to_string(),
        })?;
        Ok(RestoreAttemptRecord {
            device_binding: self.binding_for(&attempt.target_store_id)?,
            installation_id: identity::install_identity()?,
            request_fingerprint: identity::sha256_hex(&fingerprint_bytes),
            attempt,
        })
    }

    pub fn get(&self, attempt_id: &str) -> MigrationResult<RestoreAttempt> {
        let record = self
            .db
            .get_restore_attempt(attempt_id)
            .map_err(store_failed)?
            .filter(|record| self.belongs_here(record))
            .ok_or_else(|| MigrationError::AttemptNotFound {
                attempt_id: attempt_id.to_string(),
            })?;
        Ok(record.attempt)
    }

    pub fn for_native(
        &self,
        provider_id: &str,
        native_id: &str,
    ) -> MigrationResult<Vec<RestoreAttempt>> {
        let records = self
            .db
            .restore_attempts_for_native(provider_id, native_id)
            .map_err(store_failed)?;
        Ok(self.local_only(records))
    }

    pub fn list(&self) -> MigrationResult<Vec<RestoreAttempt>> {
        let records = self.db.list_restore_attempts().map_err(store_failed)?;
        Ok(self.local_only(records))
    }

    pub fn for_snapshots(&self, snapshot_ids: &[String]) -> MigrationResult<Vec<RestoreAttempt>> {
        let records = self
            .db
            .list_restore_attempts_for_snapshots(snapshot_ids)
            .map_err(store_failed)?;
        Ok(self.local_only(records))
    }

    pub fn unresolved(&self) -> MigrationResult<Vec<RestoreAttempt>> {
        let records = self
            .db
            .list_unresolved_restore_attempts()
            .map_err(store_failed)?;
        Ok(self.local_only(records))
    }

    /// Take ownership of the native write for one attempt.
    pub fn begin_native_write(&self, attempt_id: &str, now: i64) -> MigrationResult<bool> {
        // Confirm the row is ours before mutating it.
        self.get(attempt_id)?;
        self.db
            .begin_restore_native_write(attempt_id, now)
            .map_err(store_failed)
    }

    pub fn set_native_id(
        &self,
        attempt_id: &str,
        native_id: &str,
        now: i64,
    ) -> MigrationResult<()> {
        self.get(attempt_id)?;
        self.db
            .set_restore_native_id(attempt_id, native_id, now)
            .map_err(store_failed)
    }

    pub fn set_stage(
        &self,
        attempt_id: &str,
        stage: RestoreStage,
        error: Option<&MigrationError>,
        now: i64,
    ) -> MigrationResult<()> {
        self.get(attempt_id)?;
        self.db
            .set_restore_stage(attempt_id, stage, error, now)
            .map_err(store_failed)
    }

    /// Record what the user says they saw. This never touches `stage`.
    pub fn attest(
        &self,
        attempt_id: &str,
        attestation: &UserAttestation,
    ) -> MigrationResult<RestoreAttempt> {
        self.get(attempt_id)?;
        self.db
            .set_restore_user_attestation(attempt_id, attestation)
            .map_err(store_failed)?;
        self.get(attempt_id)
    }
}

pub fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session_manager::migrate::model::{OriginIdentity, RestoreRequestKind};

    fn attempt(attempt_id: &str, request_id: &str, target_store_id: &str) -> RestoreAttempt {
        RestoreAttempt {
            attempt_id: attempt_id.to_string(),
            request_id: request_id.to_string(),
            snapshot_id: "fys1:snap".to_string(),
            request_kind: RestoreRequestKind::SaveAsNewCopy,
            idempotency_slot: None,
            content_digest: "fyc1:digest".to_string(),
            origin: OriginIdentity {
                origin_id: "fyo1:origin".to_string(),
                provider_id: "codex".to_string(),
                session_id: None,
                cli_version: None,
                store_fingerprint: None,
            },
            target_provider_id: "opencode".to_string(),
            target_store_id: target_store_id.to_string(),
            target_native_nonce: "fynonce1-x".to_string(),
            target_native_id: None,
            target_workspace: Some("/tmp/work".to_string()),
            stage: RestoreStage::PackageVerified,
            attempt_count: 1,
            user_attestation: None,
            last_error: None,
            created_at: 1_000,
            updated_at: 1_000,
        }
    }

    #[test]
    fn rows_written_by_this_installation_are_visible() {
        let state = tempfile::tempdir().expect("state dir");
        identity::with_local_state_dir(state.path(), || {
            let db = Database::memory().expect("db");
            let store = ReceiptStore::new(&db);
            store
                .claim(attempt("a1", "req-1", "store-1"))
                .expect("claim");

            let listed = store.list().expect("list");
            assert_eq!(listed.len(), 1);
            assert_eq!(listed[0].attempt_id, "a1");
            assert_eq!(store.get("a1").expect("get").attempt_id, "a1");
        });
    }

    #[test]
    fn a_receipt_from_another_installation_is_inert() {
        // Simulates a row that arrived through a restored backup: written
        // under one install identity, read under another.
        let db = Database::memory().expect("db");
        let writer_state = tempfile::tempdir().expect("writer state");
        identity::with_local_state_dir(writer_state.path(), || {
            ReceiptStore::new(&db)
                .claim(attempt("a1", "req-1", "store-1"))
                .expect("claim");
        });

        let reader_state = tempfile::tempdir().expect("reader state");
        identity::with_local_state_dir(reader_state.path(), || {
            let store = ReceiptStore::new(&db);
            assert!(store.list().expect("list").is_empty());
            assert!(store.unresolved().expect("unresolved").is_empty());
            assert!(store
                .for_snapshots(&["fys1:snap".to_string()])
                .expect("snapshots")
                .is_empty());
            let error = store.get("a1").expect_err("must not be readable");
            assert_eq!(error.code(), "attemptNotFound");
        });
    }

    #[test]
    fn a_foreign_row_cannot_be_mutated() {
        let db = Database::memory().expect("db");
        let writer_state = tempfile::tempdir().expect("writer state");
        identity::with_local_state_dir(writer_state.path(), || {
            ReceiptStore::new(&db)
                .claim(attempt("a1", "req-1", "store-1"))
                .expect("claim");
        });

        let reader_state = tempfile::tempdir().expect("reader state");
        identity::with_local_state_dir(reader_state.path(), || {
            let store = ReceiptStore::new(&db);
            assert_eq!(
                store
                    .begin_native_write("a1", 2_000)
                    .expect_err("must fail")
                    .code(),
                "attemptNotFound"
            );
            assert_eq!(
                store
                    .set_stage("a1", RestoreStage::Failed, None, 2_000)
                    .expect_err("must fail")
                    .code(),
                "attemptNotFound"
            );
        });

        // The original row is untouched.
        identity::with_local_state_dir(writer_state.path(), || {
            let store = ReceiptStore::new(&db);
            assert_eq!(
                store.get("a1").expect("get").stage,
                RestoreStage::PackageVerified
            );
        });
    }

    #[test]
    fn bindings_differ_per_target_store_on_the_same_machine() {
        let state = tempfile::tempdir().expect("state dir");
        identity::with_local_state_dir(state.path(), || {
            let db = Database::memory().expect("db");
            let store = ReceiptStore::new(&db);
            let first = store.binding_for("store-1").expect("binding");
            let second = store.binding_for("store-2").expect("binding");
            assert_ne!(first, second);
            assert_eq!(first, store.binding_for("store-1").expect("stable"));
        });
    }

    #[test]
    fn a_user_claim_is_returned_without_moving_the_stage() {
        let state = tempfile::tempdir().expect("state dir");
        identity::with_local_state_dir(state.path(), || {
            let db = Database::memory().expect("db");
            let store = ReceiptStore::new(&db);
            store
                .claim(attempt("a1", "req-1", "store-1"))
                .expect("claim");

            let updated = store
                .attest(
                    "a1",
                    &UserAttestation {
                        attested_at: 9_000,
                        claimed_stage: RestoreStage::TargetOpened,
                        note: None,
                    },
                )
                .expect("attest");
            assert_eq!(updated.stage, RestoreStage::PackageVerified);
            assert_eq!(
                updated.user_attestation.map(|a| a.claimed_stage),
                Some(RestoreStage::TargetOpened)
            );
        });
    }
    #[test]
    fn a_request_cannot_be_rebound_to_a_new_workspace_or_selection() {
        let state = tempfile::tempdir().unwrap();
        identity::with_local_state_dir(state.path(), || {
            let db = Database::memory().unwrap();
            let store = ReceiptStore::new(&db);
            let original = attempt("a1", "request", "store");
            store
                .claim_for_request(original.clone(), &["fys1:snap".into()])
                .unwrap();
            let mut moved = original.clone();
            moved.attempt_id = "a2".into();
            moved.target_workspace = Some("/tmp/different".into());
            assert_eq!(
                store
                    .claim_for_request(moved, &["fys1:snap".into()])
                    .unwrap_err()
                    .code(),
                "identityFieldInvalid"
            );
            assert_eq!(
                store
                    .claim_for_request(original, &["fys1:snap".into(), "fys1:extra".into()])
                    .unwrap_err()
                    .code(),
                "identityFieldInvalid"
            );
            assert_eq!(store.list().unwrap().len(), 1);
        });
    }

    #[test]
    fn a_request_cannot_be_rebound_to_another_target_store() {
        let state = tempfile::tempdir().unwrap();
        identity::with_local_state_dir(state.path(), || {
            let db = Database::memory().unwrap();
            let store = ReceiptStore::new(&db);
            store.claim(attempt("a1", "request", "store-1")).unwrap();
            assert_eq!(
                store
                    .claim(attempt("a2", "request", "store-2"))
                    .unwrap_err()
                    .code(),
                "identityFieldInvalid"
            );
            assert_eq!(store.list().unwrap().len(), 1);
        });
    }

    #[test]
    fn a_batch_selection_must_match_every_claimed_snapshot_before_inserting() {
        let state = tempfile::tempdir().unwrap();
        identity::with_local_state_dir(state.path(), || {
            let db = Database::memory().unwrap();
            let store = ReceiptStore::new(&db);
            let a = attempt("a", "request", "store");
            let mut b = a.clone();
            b.attempt_id = "b".into();
            b.snapshot_id = "fys1:other".into();
            assert_eq!(
                store
                    .claim_batch(vec![a.clone(), b], std::slice::from_ref(&a.snapshot_id))
                    .unwrap_err()
                    .code(),
                "identityFieldInvalid"
            );
            assert!(store.list().unwrap().is_empty());
            assert!(store
                .claim_batch(
                    vec![a.clone(), a.clone()],
                    std::slice::from_ref(&a.snapshot_id)
                )
                .is_err());
            assert!(store.claim_batch(vec![], &[]).is_err());
        });
    }
}
