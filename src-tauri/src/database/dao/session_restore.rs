//! Session migration receipts.
//!
//! One local table holding identity, target mapping and stage. Message bodies
//! are never stored here: the provider's own native store stays the record of
//! truth for transcripts, and a second copy would be a second thing to keep
//! consistent.
//!
//! `device_binding` is stored per row and participates in both uniqueness
//! rules. Validating it is the caller's job, because the binding is derived
//! from this installation plus the row's own target store; see
//! `session_manager::migrate::receipt`, which drops every row that does not
//! belong to this machine before anything else looks at it.
//!
//! What SQLite buys here is the atomic single-row update and the uniqueness
//! gates. What it cannot buy is cross-process exactly-once: the native write
//! is a subprocess or an external protocol call and is never inside these
//! transactions.

use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::database::{lock_conn, Database};
use crate::error::AppError;
use crate::session_manager::migrate::model::{
    MigrationError, MigrationResult, OriginIdentity, RestoreAttempt, RestoreRequestKind,
    RestoreStage, UserAttestation,
};

/// A receipt row plus the binding that scopes it to one installation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestoreAttemptRecord {
    pub attempt: RestoreAttempt,
    pub device_binding: String,
    pub installation_id: String,
    pub request_fingerprint: String,
}

/// Result of claiming a receipt row for one restore request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttemptClaim {
    /// A new row was created; this caller owns the write.
    Created(RestoreAttemptRecord),
    /// The same request already has a row. A retry of one user action must
    /// reuse it rather than producing a second copy.
    ExistingRequest(RestoreAttemptRecord),
    /// The default-import slot is held by another attempt for the same
    /// snapshot, target store and device.
    SlotTaken(RestoreAttemptRecord),
}

/// Stages a row may be in when a native write is allowed to start.
///
/// `failed` is included because it means "proven side-effect free"; every
/// other unresolved stage must be reconciled first.
const WRITABLE_STAGES: &str = "'packageVerified', 'failed'";

impl Database {
    /// Insert the pre-write receipt, or return the row that already owns this
    /// request or slot.
    ///
    /// Conflicts are detected by the unique indexes rather than by reading
    /// first and writing after, so two concurrent commands cannot both decide
    /// they are the first.
    #[cfg(test)]
    pub(crate) fn claim_restore_attempt(
        &self,
        record: &RestoreAttemptRecord,
    ) -> MigrationResult<AttemptClaim> {
        self.claim_restore_attempts(std::slice::from_ref(record))?
            .pop()
            .ok_or_else(|| claim_error("empty receipt claim"))
    }

    /// Reserve a whole action before the first native side effect. A conflict
    /// in a later selection rolls back every newly reserved row.
    pub(crate) fn claim_restore_attempts(
        &self,
        records: &[RestoreAttemptRecord],
    ) -> MigrationResult<Vec<AttemptClaim>> {
        let mut conn = self.conn.lock().map_err(claim_error)?;
        let tx = conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(claim_error)?;
        let mut claims = Vec::with_capacity(records.len());
        for record in records {
            claims.push(claim_on_connection(&tx, record)?);
        }
        tx.commit().map_err(claim_error)?;
        Ok(claims)
    }

    /// Move a row into `nativeWritePending` and take ownership of the write.
    ///
    /// Returns `false` when the row is not in a writable stage, which is how
    /// an unresolved attempt is prevented from being written a second time.
    pub(crate) fn begin_restore_native_write(
        &self,
        attempt_id: &str,
        now: i64,
    ) -> Result<bool, AppError> {
        let conn = lock_conn!(self.conn);
        let updated = conn
            .execute(
                &format!(
                    "UPDATE session_restore_attempts
                     SET stage = 'nativeWritePending',
                         attempt_count = attempt_count + 1,
                         last_error_code = NULL,
                         last_error_detail = NULL,
                         updated_at = ?2
                     WHERE attempt_id = ?1 AND stage IN ({WRITABLE_STAGES})"
                ),
                params![attempt_id, now],
            )
            .map_err(|error| AppError::Database(format!("receipt claim failed: {error}")))?;
        Ok(updated == 1)
    }

    /// Persist the target's native ID. Called before the content side effect,
    /// so a crash afterwards still leaves a locatable mapping.
    ///
    /// An existing mapping is never overwritten: every copy keeps its own row.
    pub(crate) fn set_restore_native_id(
        &self,
        attempt_id: &str,
        native_id: &str,
        now: i64,
    ) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        let updated = conn
            .execute(
                "UPDATE session_restore_attempts
             SET target_native_id = ?2, updated_at = ?3
             WHERE attempt_id = ?1 AND (target_native_id IS NULL OR target_native_id = ?2)",
                params![attempt_id, native_id, now],
            )
            .map_err(|error| {
                AppError::Database(format!("receipt native id write failed: {error}"))
            })?;
        if updated != 1 {
            return Err(AppError::Database(
                "receipt native ID is missing or already bound to another session".into(),
            ));
        }
        Ok(())
    }

    pub(crate) fn set_restore_stage(
        &self,
        attempt_id: &str,
        stage: RestoreStage,
        error: Option<&MigrationError>,
        now: i64,
    ) -> Result<(), AppError> {
        let (code, detail) = match error {
            Some(error) => (Some(error.code().to_string()), Some(error.to_ipc_string())),
            None => (None, None),
        };
        let conn = lock_conn!(self.conn);
        conn.execute(
            "UPDATE session_restore_attempts
             SET stage = ?2, last_error_code = ?3, last_error_detail = ?4, updated_at = ?5
             WHERE attempt_id = ?1",
            params![attempt_id, stage.as_str(), code, detail, now],
        )
        .map_err(|error| AppError::Database(format!("receipt stage write failed: {error}")))?;
        Ok(())
    }

    /// Write the user's own claim. It sits beside the stage and never moves
    /// it: a self-report is not system evidence.
    pub(crate) fn set_restore_user_attestation(
        &self,
        attempt_id: &str,
        attestation: &UserAttestation,
    ) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        let updated = conn
            .execute(
                "UPDATE session_restore_attempts
                 SET user_attested_at = ?2, user_claimed_stage = ?3, user_note = ?4,
                     updated_at = ?5
                 WHERE attempt_id = ?1",
                params![
                    attempt_id,
                    attestation.attested_at,
                    attestation.claimed_stage.as_str(),
                    attestation.note,
                    attestation.attested_at,
                ],
            )
            .map_err(|error| AppError::Database(format!("receipt attestation failed: {error}")))?;
        if updated == 0 {
            return Err(AppError::Database(format!(
                "restore attempt not found: {attempt_id}"
            )));
        }
        Ok(())
    }

    pub(crate) fn get_restore_attempt(
        &self,
        attempt_id: &str,
    ) -> Result<Option<RestoreAttemptRecord>, AppError> {
        let conn = lock_conn!(self.conn);
        query_optional(
            &conn,
            "SELECT * FROM session_restore_attempts WHERE attempt_id = ?1",
            params![attempt_id],
        )
    }

    pub(crate) fn restore_attempts_for_native(
        &self,
        provider_id: &str,
        native_id: &str,
    ) -> Result<Vec<RestoreAttemptRecord>, AppError> {
        let conn = lock_conn!(self.conn);
        query_many(&conn, "SELECT * FROM session_restore_attempts WHERE target_provider_id = ?1 AND target_native_id = ?2 ORDER BY created_at DESC", params![provider_id, native_id])
    }

    pub(crate) fn list_restore_attempts(&self) -> Result<Vec<RestoreAttemptRecord>, AppError> {
        let conn = lock_conn!(self.conn);
        query_many(
            &conn,
            "SELECT * FROM session_restore_attempts ORDER BY created_at DESC, attempt_id",
            [],
        )
    }

    /// Attempts for the snapshots inside a package, so the reader can show
    /// "this one is already imported" without guessing from the transcript.
    pub(crate) fn list_restore_attempts_for_snapshots(
        &self,
        snapshot_ids: &[String],
    ) -> Result<Vec<RestoreAttemptRecord>, AppError> {
        if snapshot_ids.is_empty() {
            return Ok(Vec::new());
        }
        let conn = lock_conn!(self.conn);
        let mut out = Vec::new();
        for snapshot_id in snapshot_ids {
            out.extend(query_many(
                &conn,
                "SELECT * FROM session_restore_attempts
                 WHERE snapshot_id = ?1 ORDER BY created_at DESC, attempt_id",
                params![snapshot_id],
            )?);
        }
        Ok(out)
    }

    /// Rows whose write outcome is unknown. These are the only candidates for
    /// reconciliation, and they may not be written again until it runs.
    pub(crate) fn list_unresolved_restore_attempts(
        &self,
    ) -> Result<Vec<RestoreAttemptRecord>, AppError> {
        let conn = lock_conn!(self.conn);
        query_many(
            &conn,
            "SELECT * FROM session_restore_attempts
             WHERE stage IN ('nativeWritePending', 'needsReconciliation', 'ambiguous')
             ORDER BY created_at",
            [],
        )
    }
}

fn claim_error(error: impl std::fmt::Display) -> MigrationError {
    MigrationError::ReceiptStoreFailed {
        reason: error.to_string(),
    }
}

fn claim_on_connection(
    conn: &Connection,
    record: &RestoreAttemptRecord,
) -> MigrationResult<AttemptClaim> {
    let attempt = &record.attempt;
    let existing_action = query_optional(conn,
        "SELECT * FROM session_restore_attempts WHERE installation_id = ?1 AND request_id = ?2 LIMIT 1",
        params![record.installation_id, attempt.request_id]).map_err(claim_error)?;
    if let Some(existing) = existing_action {
        if existing.request_fingerprint != record.request_fingerprint {
            return Err(MigrationError::IdentityFieldInvalid {
                field: "requestId".into(),
                reason: "this action already belongs to a different selection or target; start a new action".into(),
            });
        }
    }
    if let Some(existing) = query_optional(conn,
        "SELECT * FROM session_restore_attempts WHERE device_binding = ?1 AND request_id = ?2 AND snapshot_id = ?3",
        params![record.device_binding, attempt.request_id, attempt.snapshot_id]).map_err(claim_error)? {
        return Ok(AttemptClaim::ExistingRequest(existing));
    }
    if let Some(slot) = attempt.idempotency_slot.as_deref() {
        if let Some(existing) = query_optional(
            conn,
            "SELECT * FROM session_restore_attempts WHERE idempotency_slot = ?1",
            params![slot],
        )
        .map_err(claim_error)?
        {
            return Ok(AttemptClaim::SlotTaken(existing));
        }
    }
    if attempt.request_kind == RestoreRequestKind::DefaultImport {
        if let Some(existing) = query_optional(
            conn,
            "SELECT * FROM session_restore_attempts WHERE device_binding = ?1
             AND target_provider_id = ?2 AND target_store_id = ?3
             AND origin_id = ?4 AND snapshot_id <> ?5 LIMIT 1",
            params![
                record.device_binding,
                attempt.target_provider_id,
                attempt.target_store_id,
                attempt.origin.origin_id,
                attempt.snapshot_id
            ],
        )
        .map_err(claim_error)?
        {
            return Err(MigrationError::SourceSnapshotConflict {
                origin_id: attempt.origin.origin_id.clone(),
                existing_attempt_id: existing.attempt.attempt_id,
            });
        }
    }
    let inserted = conn
            .execute(
                "INSERT OR IGNORE INTO session_restore_attempts (
                    attempt_id, request_id, snapshot_id, origin_id, request_kind,
                    idempotency_slot, content_digest, origin_provider_id, origin_session_id,
                    origin_cli_version, origin_store_fp, target_provider_id, target_store_id,
                    target_native_nonce, target_native_id, target_workspace, stage,
                    attempt_count, device_binding, created_at, updated_at, installation_id, request_fingerprint
                 ) VALUES (
                    ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17,
                    ?18, ?19, ?20, ?21, ?22, ?23
                 )",
                params![
                    attempt.attempt_id,
                    attempt.request_id,
                    attempt.snapshot_id,
                    attempt.origin.origin_id,
                    attempt.request_kind.as_str(),
                    attempt.idempotency_slot,
                    attempt.content_digest,
                    attempt.origin.provider_id,
                    attempt.origin.session_id,
                    attempt.origin.cli_version,
                    attempt.origin.store_fingerprint,
                    attempt.target_provider_id,
                    attempt.target_store_id,
                    attempt.target_native_nonce,
                    attempt.target_native_id,
                    attempt.target_workspace,
                    attempt.stage.as_str(),
                    attempt.attempt_count,
                    record.device_binding,
                    attempt.created_at,
                    attempt.updated_at,
                    record.installation_id,
                    record.request_fingerprint,
                ],
            )
            .map_err(claim_error)?;

    if inserted == 1 {
        Ok(AttemptClaim::Created(record.clone()))
    } else {
        Err(claim_error(
            "receipt claim rejected by an unexpected identity collision",
        ))
    }
}

fn query_optional(
    conn: &Connection,
    sql: &str,
    params: impl rusqlite::Params,
) -> Result<Option<RestoreAttemptRecord>, AppError> {
    conn.query_row(sql, params, row_to_record)
        .optional()
        .map_err(|error| AppError::Database(format!("receipt read failed: {error}")))?
        .transpose()
}

fn query_many(
    conn: &Connection,
    sql: &str,
    params: impl rusqlite::Params,
) -> Result<Vec<RestoreAttemptRecord>, AppError> {
    let mut statement = conn
        .prepare(sql)
        .map_err(|error| AppError::Database(format!("receipt query failed: {error}")))?;
    let rows = statement
        .query_map(params, row_to_record)
        .map_err(|error| AppError::Database(format!("receipt query failed: {error}")))?;
    let mut out = Vec::new();
    for row in rows {
        let record =
            row.map_err(|error| AppError::Database(format!("receipt read failed: {error}")))?;
        out.push(record?);
    }
    Ok(out)
}

/// Decode one row.
///
/// A stored enum value this build does not recognize is a decode error, not a
/// silent default: mapping an unknown stage onto a known one would fabricate
/// evidence about what really happened.
fn row_to_record(row: &Row<'_>) -> rusqlite::Result<Result<RestoreAttemptRecord, AppError>> {
    let attempt_id: String = row.get("attempt_id")?;
    let stage_raw: String = row.get("stage")?;
    let kind_raw: String = row.get("request_kind")?;
    let claimed_stage_raw: Option<String> = row.get("user_claimed_stage")?;
    let attested_at: Option<i64> = row.get("user_attested_at")?;
    let last_error_detail: Option<String> = row.get("last_error_detail")?;

    let Some(stage) = RestoreStage::parse(&stage_raw) else {
        return Ok(Err(AppError::Database(format!(
            "unknown restore stage `{stage_raw}` on attempt {attempt_id}"
        ))));
    };
    let Some(request_kind) = RestoreRequestKind::parse(&kind_raw) else {
        return Ok(Err(AppError::Database(format!(
            "unknown restore request kind `{kind_raw}` on attempt {attempt_id}"
        ))));
    };

    let user_attestation = match (attested_at, claimed_stage_raw) {
        (Some(attested_at), Some(claimed_raw)) => {
            let Some(claimed_stage) = RestoreStage::parse(&claimed_raw) else {
                return Ok(Err(AppError::Database(format!(
                    "unknown attested stage `{claimed_raw}` on attempt {attempt_id}"
                ))));
            };
            Some(UserAttestation {
                attested_at,
                claimed_stage,
                note: row.get("user_note")?,
            })
        }
        _ => None,
    };

    Ok(Ok(RestoreAttemptRecord {
        attempt: RestoreAttempt {
            attempt_id,
            request_id: row.get("request_id")?,
            snapshot_id: row.get("snapshot_id")?,
            request_kind,
            idempotency_slot: row.get("idempotency_slot")?,
            content_digest: row.get("content_digest")?,
            origin: OriginIdentity {
                origin_id: row.get("origin_id")?,
                provider_id: row.get("origin_provider_id")?,
                session_id: row.get("origin_session_id")?,
                cli_version: row.get("origin_cli_version")?,
                store_fingerprint: row.get("origin_store_fp")?,
            },
            target_provider_id: row.get("target_provider_id")?,
            target_store_id: row.get("target_store_id")?,
            target_native_nonce: row.get("target_native_nonce")?,
            target_native_id: row.get("target_native_id")?,
            target_workspace: row.get("target_workspace")?,
            stage,
            attempt_count: row.get::<_, i64>("attempt_count")? as u32,
            user_attestation,
            last_error: last_error_detail
                .as_deref()
                .and_then(|raw| serde_json::from_str(raw).ok()),
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        },
        device_binding: row.get("device_binding")?,
        installation_id: row.get("installation_id")?,
        request_fingerprint: row.get("request_fingerprint")?,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(
        attempt_id: &str,
        request_id: &str,
        snapshot_id: &str,
        slot: Option<&str>,
        binding: &str,
    ) -> RestoreAttemptRecord {
        RestoreAttemptRecord {
            attempt: RestoreAttempt {
                attempt_id: attempt_id.to_string(),
                request_id: request_id.to_string(),
                snapshot_id: snapshot_id.to_string(),
                request_kind: if slot.is_some() {
                    RestoreRequestKind::DefaultImport
                } else {
                    RestoreRequestKind::SaveAsNewCopy
                },
                idempotency_slot: slot.map(str::to_string),
                content_digest: "fyc1:digest".to_string(),
                origin: OriginIdentity {
                    origin_id: "fyo1:origin".to_string(),
                    provider_id: "codex".to_string(),
                    session_id: Some("native-1".to_string()),
                    cli_version: None,
                    store_fingerprint: None,
                },
                target_provider_id: "opencode".to_string(),
                target_store_id: "opencode-db:abc".to_string(),
                target_native_nonce: "fynonce1-abc".to_string(),
                target_native_id: None,
                target_workspace: Some("/tmp/work".to_string()),
                stage: RestoreStage::PackageVerified,
                attempt_count: 1,
                user_attestation: None,
                last_error: None,
                created_at: 1_000,
                updated_at: 1_000,
            },
            device_binding: binding.to_string(),
            installation_id: binding.to_string(),
            request_fingerprint: format!("action:{request_id}:{}", slot.is_some()),
        }
    }

    fn db() -> Database {
        Database::memory().expect("memory database")
    }

    #[test]
    fn a_new_request_creates_exactly_one_row() {
        let db = db();
        let row = record("a1", "req-1", "snap-1", Some("slot-1"), "dev-1");
        assert!(matches!(
            db.claim_restore_attempt(&row).expect("claim"),
            AttemptClaim::Created(_)
        ));
        assert_eq!(db.list_restore_attempts().expect("list").len(), 1);
    }

    #[test]
    fn retrying_the_same_request_reuses_the_original_row() {
        // A timeout resend or a double click must not produce a second copy.
        let db = db();
        db.claim_restore_attempt(&record("a1", "req-1", "snap-1", Some("slot-1"), "dev-1"))
            .expect("first");

        let claim = db
            .claim_restore_attempt(&record("a2", "req-1", "snap-1", Some("slot-1"), "dev-1"))
            .expect("retry");
        match claim {
            AttemptClaim::ExistingRequest(existing) => {
                assert_eq!(existing.attempt.attempt_id, "a1")
            }
            other => panic!("expected the original row, got {other:?}"),
        }
        assert_eq!(db.list_restore_attempts().expect("list").len(), 1);
    }

    #[test]
    fn a_second_request_for_the_same_slot_is_reported_not_inserted() {
        let db = db();
        db.claim_restore_attempt(&record("a1", "req-1", "snap-1", Some("slot-1"), "dev-1"))
            .expect("first");

        let claim = db
            .claim_restore_attempt(&record("a2", "req-2", "snap-1", Some("slot-1"), "dev-1"))
            .expect("second");
        match claim {
            AttemptClaim::SlotTaken(existing) => assert_eq!(existing.attempt.attempt_id, "a1"),
            other => panic!("expected the slot holder, got {other:?}"),
        }
    }

    #[test]
    fn explicit_extra_copies_are_never_constrained_by_the_slot() {
        let db = db();
        db.claim_restore_attempt(&record("a1", "req-1", "snap-1", Some("slot-1"), "dev-1"))
            .expect("default import");
        // Two separate "save as new copy" actions on the same snapshot.
        assert!(matches!(
            db.claim_restore_attempt(&record("a2", "req-2", "snap-1", None, "dev-1")),
            Ok(AttemptClaim::Created(_))
        ));
        assert!(matches!(
            db.claim_restore_attempt(&record("a3", "req-3", "snap-1", None, "dev-1")),
            Ok(AttemptClaim::Created(_))
        ));
        assert_eq!(db.list_restore_attempts().expect("list").len(), 3);
    }

    #[test]
    fn a_retried_extra_copy_still_reuses_its_row() {
        let db = db();
        db.claim_restore_attempt(&record("a1", "req-1", "snap-1", None, "dev-1"))
            .expect("copy");
        assert!(matches!(
            db.claim_restore_attempt(&record("a2", "req-1", "snap-1", None, "dev-1")),
            Ok(AttemptClaim::ExistingRequest(_))
        ));
        assert_eq!(db.list_restore_attempts().expect("list").len(), 1);
    }

    #[test]
    fn another_devices_row_does_not_occupy_this_devices_slot() {
        // Slots include the device binding, so a receipt that arrived through
        // a restored backup cannot block a local import.
        let db = db();
        db.claim_restore_attempt(&record(
            "a1",
            "req-1",
            "snap-1",
            Some("slot-old"),
            "dev-old",
        ))
        .expect("foreign row");
        assert!(matches!(
            db.claim_restore_attempt(&record(
                "a2",
                "req-1",
                "snap-1",
                Some("slot-new"),
                "dev-new"
            )),
            Ok(AttemptClaim::Created(_))
        ));
    }

    #[test]
    fn only_one_caller_can_claim_the_native_write() {
        let db = db();
        db.claim_restore_attempt(&record("a1", "req-1", "snap-1", Some("slot-1"), "dev-1"))
            .expect("claim");

        assert!(db.begin_restore_native_write("a1", 2_000).expect("first"));
        // Now pending: a second caller must not start another native write.
        assert!(!db.begin_restore_native_write("a1", 3_000).expect("second"));

        let stored = db.get_restore_attempt("a1").expect("get").expect("row");
        assert_eq!(stored.attempt.stage, RestoreStage::NativeWritePending);
        assert_eq!(stored.attempt.attempt_count, 2);
    }

    #[test]
    fn an_unresolved_attempt_cannot_be_written_again() {
        let db = db();
        db.claim_restore_attempt(&record("a1", "req-1", "snap-1", Some("slot-1"), "dev-1"))
            .expect("claim");
        for stage in [
            RestoreStage::NeedsReconciliation,
            RestoreStage::Ambiguous,
            RestoreStage::NativeWritten,
        ] {
            db.set_restore_stage("a1", stage, None, 2_000)
                .expect("stage");
            assert!(
                !db.begin_restore_native_write("a1", 3_000).expect("claim"),
                "{stage:?} must block a second write"
            );
        }
    }

    #[test]
    fn a_proven_failure_allows_an_idempotent_retry_on_the_same_row() {
        let db = db();
        db.claim_restore_attempt(&record("a1", "req-1", "snap-1", Some("slot-1"), "dev-1"))
            .expect("claim");
        db.set_restore_stage("a1", RestoreStage::Failed, None, 2_000)
            .expect("stage");

        assert!(db.begin_restore_native_write("a1", 3_000).expect("retry"));
        let stored = db.get_restore_attempt("a1").expect("get").expect("row");
        assert_eq!(stored.attempt.attempt_count, 2);
        assert_eq!(db.list_restore_attempts().expect("list").len(), 1);
    }

    #[test]
    fn a_recorded_mapping_is_never_overwritten() {
        let db = db();
        db.claim_restore_attempt(&record("a1", "req-1", "snap-1", Some("slot-1"), "dev-1"))
            .expect("claim");
        db.set_restore_native_id("a1", "thread-1", 2_000)
            .expect("first id");
        db.set_restore_native_id("a1", "thread-2", 3_000)
            .expect_err("a different ID must stop publication");
        db.set_restore_native_id("a1", "thread-1", 4_000)
            .expect("same ID is idempotent");
        db.set_restore_native_id("missing", "thread-1", 4_000)
            .expect_err("missing receipt must stop publication");

        let stored = db.get_restore_attempt("a1").expect("get").expect("row");
        assert_eq!(stored.attempt.target_native_id.as_deref(), Some("thread-1"));
    }

    #[test]
    fn a_user_claim_is_stored_beside_the_stage_and_never_moves_it() {
        let db = db();
        db.claim_restore_attempt(&record("a1", "req-1", "snap-1", Some("slot-1"), "dev-1"))
            .expect("claim");
        db.set_restore_user_attestation(
            "a1",
            &UserAttestation {
                attested_at: 5_000,
                claimed_stage: RestoreStage::NextTurnReplyVerified,
                note: Some("it worked for me".to_string()),
            },
        )
        .expect("attest");

        let stored = db.get_restore_attempt("a1").expect("get").expect("row");
        assert_eq!(stored.attempt.stage, RestoreStage::PackageVerified);
        assert_eq!(
            stored
                .attempt
                .user_attestation
                .as_ref()
                .map(|attestation| attestation.claimed_stage),
            Some(RestoreStage::NextTurnReplyVerified)
        );
    }

    #[test]
    fn a_stored_error_round_trips_as_a_typed_code() {
        let db = db();
        db.claim_restore_attempt(&record("a1", "req-1", "snap-1", Some("slot-1"), "dev-1"))
            .expect("claim");
        let error = MigrationError::NativeImportFailed {
            provider_id: "opencode".to_string(),
            exit_code: Some(2),
            stderr_tail: "boom".to_string(),
        };
        db.set_restore_stage("a1", RestoreStage::NeedsReconciliation, Some(&error), 2_000)
            .expect("stage");

        let stored = db.get_restore_attempt("a1").expect("get").expect("row");
        assert_eq!(stored.attempt.last_error, Some(error));
    }

    #[test]
    fn unresolved_listing_returns_only_unknown_outcomes() {
        let db = db();
        for (index, stage) in [
            RestoreStage::PackageVerified,
            RestoreStage::NativeWritePending,
            RestoreStage::NeedsReconciliation,
            RestoreStage::Ambiguous,
            RestoreStage::NativeReadbackVerified,
            RestoreStage::Failed,
        ]
        .into_iter()
        .enumerate()
        {
            let id = format!("a{index}");
            db.claim_restore_attempt(&record(
                &id,
                &format!("req-{index}"),
                &format!("snap-{index}"),
                None,
                "dev-1",
            ))
            .expect("claim");
            db.set_restore_stage(&id, stage, None, 2_000)
                .expect("stage");
        }

        let stages: Vec<RestoreStage> = db
            .list_unresolved_restore_attempts()
            .expect("unresolved")
            .iter()
            .map(|record| record.attempt.stage)
            .collect();
        assert_eq!(
            stages,
            vec![
                RestoreStage::NativeWritePending,
                RestoreStage::NeedsReconciliation,
                RestoreStage::Ambiguous
            ]
        );
    }

    #[test]
    fn snapshot_lookup_returns_every_copy_of_that_snapshot() {
        let db = db();
        db.claim_restore_attempt(&record("a1", "req-1", "snap-1", None, "dev-1"))
            .expect("first copy");
        db.claim_restore_attempt(&record("a2", "req-2", "snap-1", None, "dev-1"))
            .expect("second copy");
        db.claim_restore_attempt(&record("a3", "req-3", "snap-2", None, "dev-1"))
            .expect("other snapshot");

        let found = db
            .list_restore_attempts_for_snapshots(&["snap-1".to_string()])
            .expect("lookup");
        assert_eq!(found.len(), 2);
        assert!(found
            .iter()
            .all(|record| record.attempt.snapshot_id == "snap-1"));
    }

    /// Raw statement against the same connection, used to force states the
    /// DAO refuses to produce.
    fn force(db: &Database, sql: &str) -> rusqlite::Result<usize> {
        let conn = db.conn.lock().expect("connection lock");
        conn.execute(sql, [])
    }

    #[test]
    fn an_unknown_stored_stage_is_an_error_not_a_silent_default() {
        let db = db();
        db.claim_restore_attempt(&record("a1", "req-1", "snap-1", None, "dev-1"))
            .expect("claim");
        force(
            &db,
            "UPDATE session_restore_attempts SET stage = 'inventedStage' WHERE attempt_id = 'a1'",
        )
        .expect("force unknown stage");

        let error = db.get_restore_attempt("a1").expect_err("must fail");
        assert!(error.to_string().contains("inventedStage"));
    }

    #[test]
    fn an_unknown_request_kind_is_rejected_by_the_schema() {
        let db = db();
        db.claim_restore_attempt(&record("a1", "req-1", "snap-1", None, "dev-1"))
            .expect("claim");
        let error = force(
            &db,
            "UPDATE session_restore_attempts SET request_kind = 'overwrite' WHERE attempt_id = 'a1'",
        )
        .expect_err("CHECK constraint must reject an unmodelled kind");
        assert!(error.to_string().contains("CHECK"));
    }
    #[test]
    fn changed_snapshot_requires_explicit_new_copy() {
        let db = db();
        db.claim_restore_attempt(&record("a1", "req1", "snap1", Some("slot1"), "device"))
            .unwrap();
        let changed = record("a2", "req2", "snap2", Some("slot2"), "device");
        assert_eq!(
            db.claim_restore_attempt(&changed).unwrap_err().code(),
            "sourceSnapshotConflict"
        );
        assert_eq!(db.list_restore_attempts().unwrap().len(), 1);
        let copy = record("a3", "req3", "snap2", None, "device");
        assert!(matches!(
            db.claim_restore_attempt(&copy).unwrap(),
            AttemptClaim::Created(_)
        ));
        assert_eq!(db.list_restore_attempts().unwrap().len(), 2);
    }

    #[test]
    fn a_late_batch_conflict_rolls_back_every_new_reservation() {
        let db = db();
        db.claim_restore_attempt(&record(
            "old",
            "old-request",
            "old-snap",
            Some("old-slot"),
            "device",
        ))
        .unwrap();
        let mut first = record("new1", "batch", "new-snap1", Some("new-slot1"), "device");
        first.attempt.origin.origin_id = "fyo1:different-origin".into();
        let second = record("new2", "batch", "new-snap2", Some("new-slot2"), "device");
        assert_eq!(
            db.claim_restore_attempts(&[first, second])
                .unwrap_err()
                .code(),
            "sourceSnapshotConflict"
        );
        let rows = db.list_restore_attempts().unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].attempt.attempt_id, "old");
    }

    #[test]
    fn same_request_cannot_change_fingerprint_even_when_store_changes() {
        let db = db();
        let original = record("a1", "request", "snap", None, "store-binding-1");
        db.claim_restore_attempt(&original).unwrap();
        let mut changed = record("a2", "request", "snap", None, "store-binding-2");
        changed.installation_id = original.installation_id;
        changed.attempt.target_store_id = "another-store".into();
        changed.request_fingerprint = "changed-selection-or-target".into();
        assert_eq!(
            db.claim_restore_attempt(&changed).unwrap_err().code(),
            "identityFieldInvalid"
        );
        assert_eq!(db.list_restore_attempts().unwrap().len(), 1);
    }

    #[test]
    fn two_snapshots_in_one_action_claim_and_retry_as_a_unit() {
        let db = db();
        let one = record("a1", "batch", "snap1", Some("slot1"), "device");
        let mut two = record("a2", "batch", "snap2", Some("slot2"), "device");
        two.attempt.origin.origin_id = "fyo1:origin2".into();
        let first = db
            .claim_restore_attempts(&[one.clone(), two.clone()])
            .unwrap();
        assert!(first
            .iter()
            .all(|claim| matches!(claim, AttemptClaim::Created(_))));
        let second = db.claim_restore_attempts(&[one, two]).unwrap();
        assert!(second
            .iter()
            .all(|claim| matches!(claim, AttemptClaim::ExistingRequest(_))));
        assert_eq!(db.list_restore_attempts().unwrap().len(), 2);
    }
}
