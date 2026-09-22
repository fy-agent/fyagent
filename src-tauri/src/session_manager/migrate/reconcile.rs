//! Resolving writes whose outcome is unknown.
//!
//! A row is unresolved when the process died inside the crash window, or the
//! native call timed out, or the target answered in a way that rules nothing
//! out. Until it is resolved the snapshot may not be written again, because
//! the alternative is a duplicated conversation the user has to clean up by
//! hand.
//!
//! Resolution only ever uses two sources of evidence: the target's own read
//! channel, and the nonce the writer planted. Content similarity is
//! deliberately not one of them — "a session that looks like this one" is not
//! the same claim as "the session this attempt wrote".
//!
//! The reasoning here depends on no write being in flight. A row in
//! `nativeWritePending` whose writer is still running would be read as a
//! crashed attempt and could be retried into a duplicate, so this whole pass
//! runs under [`restore::serialize_migration`]. Combined with the
//! application's single-instance lock, a pending row seen here belongs to a
//! process that is gone.

use crate::database::Database;

use super::model::{MigrationError, MigrationResult, RestoreAttempt, RestoreStage};
use super::native::{self, NativeSessionWriter, ReconciliationModel};
use super::receipt::{now_ms, ReceiptStore};
use super::restore::{self, expectation_from_receipt, serialize_migration};

/// Reconcile every unresolved attempt on this machine.
///
/// Returns the rows as they stand afterwards, including the ones that stayed
/// unresolved: a row nobody can resolve must remain visible rather than being
/// quietly dropped from the list.
pub fn reconcile_all(db: &Database) -> MigrationResult<Vec<RestoreAttempt>> {
    let _serialized = serialize_migration();
    let store = ReceiptStore::new(db);
    let unresolved = store.unresolved()?;
    let mut out = Vec::with_capacity(unresolved.len());
    for attempt in unresolved {
        let attempt_id = attempt.attempt_id.clone();
        reconcile_one(&store, &attempt)?;
        out.push(store.get(&attempt_id)?);
    }
    Ok(out)
}

fn reconcile_one(store: &ReceiptStore<'_>, attempt: &RestoreAttempt) -> MigrationResult<()> {
    let Some(writer) = native::writer_for(&attempt.target_provider_id) else {
        // The build that wrote this row had a writer for the provider and this
        // one does not. Nothing here can be proven, so the row stays
        // unresolved with the reason recorded.
        return store.set_stage(
            &attempt.attempt_id,
            RestoreStage::NeedsReconciliation,
            Some(&MigrationError::ProviderVersionUnsupported {
                provider_id: attempt.target_provider_id.clone(),
                detected: None,
                verified: Vec::new(),
            }),
            now_ms(),
        );
    };

    // Every conclusion below is about one store instance on this machine. If
    // the provider was reinstalled or its data directory replaced, neither the
    // recorded ID nor the "no ID means no side effect" inference says anything
    // about what is there now.
    if let Err(error) = restore::require_verified_target(writer, attempt) {
        return store.set_stage(
            &attempt.attempt_id,
            RestoreStage::NeedsReconciliation,
            Some(&error),
            now_ms(),
        );
    }

    match attempt.target_native_id.as_deref() {
        Some(native_id) => resolve_known_id(store, writer, attempt, native_id),
        None => resolve_missing_id(store, writer, attempt),
    }
}

/// The mapping survived the crash, so the target can be asked directly.
fn resolve_known_id(
    store: &ReceiptStore<'_>,
    writer: &dyn NativeSessionWriter,
    attempt: &RestoreAttempt,
    native_id: &str,
) -> MigrationResult<()> {
    let expected = expectation_from_receipt(attempt);
    let verdict = match writer.verify_readback(native_id, &expected) {
        Ok(verdict) => verdict,
        Err(error) => {
            return store.set_stage(
                &attempt.attempt_id,
                RestoreStage::NeedsReconciliation,
                Some(&error),
                now_ms(),
            )
        }
    };

    let (stage, error) = match verdict {
        native::ReadbackVerdict::Visible { .. } => (RestoreStage::NativeReadbackVerified, None),
        // The target's own read channel says the session it would have created
        // is not there. That is the positive no-side-effect evidence `failed`
        // requires, and it is what makes an idempotent retry allowed.
        native::ReadbackVerdict::NotVisible => (
            RestoreStage::Failed,
            Some(MigrationError::ReconciliationRequired {
                attempt_id: attempt.attempt_id.clone(),
                reason: "the target reports no session for the recorded id".to_string(),
            }),
        ),
        // Something is there and it is not what we wrote. No automatic action:
        // both overwriting and retrying could destroy or duplicate real
        // history.
        native::ReadbackVerdict::Mismatch { observed } => (
            RestoreStage::Ambiguous,
            Some(MigrationError::NativeReadbackMismatch {
                attempt_id: attempt.attempt_id.clone(),
                expected_digest: attempt.content_digest.clone(),
                observed,
            }),
        ),
        native::ReadbackVerdict::Ambiguous { candidates } => (
            RestoreStage::Ambiguous,
            Some(MigrationError::AmbiguousNativeMatch {
                attempt_id: attempt.attempt_id.clone(),
                candidates,
            }),
        ),
        // No read channel in this version. The mapping exists but nothing can
        // confirm or deny it, so the row keeps its unresolved state with the
        // gap recorded rather than being promoted on optimism.
        native::ReadbackVerdict::Blocked { reason } => (
            RestoreStage::NeedsReconciliation,
            Some(MigrationError::ReconciliationRequired {
                attempt_id: attempt.attempt_id.clone(),
                reason: format!("no native read channel: {reason}"),
            }),
        ),
    };
    store.set_stage(&attempt.attempt_id, stage, error.as_ref(), now_ms())
}

/// No mapping survived. What that proves depends on the writer's model.
fn resolve_missing_id(
    store: &ReceiptStore<'_>,
    writer: &dyn NativeSessionWriter,
    attempt: &RestoreAttempt,
) -> MigrationResult<()> {
    match writer.reconciliation_model() {
        ReconciliationModel::IdBeforeContent => store.set_stage(
            &attempt.attempt_id,
            RestoreStage::Failed,
            Some(&MigrationError::ReconciliationRequired {
                attempt_id: attempt.attempt_id.clone(),
                reason: "this writer records the id before writing content, so an attempt \
                         without one produced no side effect"
                    .to_string(),
            }),
            now_ms(),
        ),
        ReconciliationModel::NonceLookup => {
            match writer.locate_by_nonce(&attempt.target_native_nonce) {
                Ok(candidates) if candidates.is_empty() => store.set_stage(
                    &attempt.attempt_id,
                    RestoreStage::Failed,
                    Some(&MigrationError::ReconciliationRequired {
                        attempt_id: attempt.attempt_id.clone(),
                        reason: "the nonce is absent from the target store".to_string(),
                    }),
                    now_ms(),
                ),
                Ok(candidates) if candidates.len() == 1 => {
                    store.set_native_id(&attempt.attempt_id, &candidates[0], now_ms())?;
                    let refreshed = store.get(&attempt.attempt_id)?;
                    match refreshed.target_native_id.as_deref() {
                        Some(native_id) => resolve_known_id(store, writer, &refreshed, native_id),
                        // Another attempt already owns that mapping, so this
                        // row cannot claim it.
                        None => store.set_stage(
                            &attempt.attempt_id,
                            RestoreStage::Ambiguous,
                            Some(&MigrationError::AmbiguousNativeMatch {
                                attempt_id: attempt.attempt_id.clone(),
                                candidates,
                            }),
                            now_ms(),
                        ),
                    }
                }
                Ok(candidates) => store.set_stage(
                    &attempt.attempt_id,
                    RestoreStage::Ambiguous,
                    Some(&MigrationError::AmbiguousNativeMatch {
                        attempt_id: attempt.attempt_id.clone(),
                        candidates,
                    }),
                    now_ms(),
                ),
                Err(error) => store.set_stage(
                    &attempt.attempt_id,
                    RestoreStage::NeedsReconciliation,
                    Some(&error),
                    now_ms(),
                ),
            }
        }
        ReconciliationModel::Unresolvable => store.set_stage(
            &attempt.attempt_id,
            RestoreStage::Ambiguous,
            Some(&MigrationError::ReconciliationRequired {
                attempt_id: attempt.attempt_id.clone(),
                reason: "this writer cannot prove what a crashed write did".to_string(),
            }),
            now_ms(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session_manager::migrate::identity;
    use crate::session_manager::migrate::model::{OriginIdentity, RestoreRequestKind};

    fn attempt(stage: RestoreStage, provider: &str, native_id: Option<&str>) -> RestoreAttempt {
        RestoreAttempt {
            attempt_id: format!("a-{}-{}", provider, stage.as_str()),
            request_id: "req-1".to_string(),
            snapshot_id: "fys1:snap".to_string(),
            request_kind: RestoreRequestKind::SaveAsNewCopy,
            idempotency_slot: None,
            content_digest: "fyc1:digest".to_string(),
            origin: OriginIdentity {
                origin_id: "fyo1:origin".to_string(),
                provider_id: provider.to_string(),
                session_id: None,
                cli_version: None,
                store_fingerprint: None,
            },
            target_provider_id: provider.to_string(),
            target_store_id: "store-1".to_string(),
            target_native_nonce: "fynonce1-abc".to_string(),
            target_native_id: native_id.map(str::to_string),
            target_workspace: None,
            stage,
            attempt_count: 1,
            user_attestation: None,
            last_error: None,
            created_at: 1_000,
            updated_at: 1_000,
        }
    }

    #[test]
    fn a_provider_without_a_writer_stays_unresolved() {
        let state = tempfile::tempdir().expect("state");
        identity::with_local_state_dir(state.path(), || {
            let db = Database::memory().expect("db");
            let store = ReceiptStore::new(&db);
            let row = attempt(RestoreStage::NativeWritePending, "claude", None);
            store.claim(row.clone()).expect("claim");

            reconcile_one(&store, &row).expect("reconcile");
            let after = store.get(&row.attempt_id).expect("get");
            assert_eq!(after.stage, RestoreStage::NeedsReconciliation);
            assert_eq!(
                after.last_error.map(|error| error.code()),
                Some("providerVersionUnsupported")
            );
        });
    }

    #[test]
    fn an_id_first_writer_without_a_mapping_is_proven_side_effect_free() {
        // Codex allocates and records the ID before publishing anything, so a
        // row that has no ID cannot have produced a visible session. Proving
        // that is what lets the user retry without risking a duplicate.
        let state = tempfile::tempdir().expect("state");
        identity::with_local_state_dir(state.path(), || {
            let db = Database::memory().expect("db");
            let store = ReceiptStore::new(&db);
            let row = attempt(RestoreStage::NativeWritePending, "codex", None);
            store.claim(row.clone()).expect("claim");

            let writer = native::writer_for("codex").expect("codex writer");
            assert_eq!(
                writer.reconciliation_model(),
                ReconciliationModel::IdBeforeContent
            );
            resolve_missing_id(&store, writer, &row).expect("resolve");

            let after = store.get(&row.attempt_id).expect("get");
            assert_eq!(after.stage, RestoreStage::Failed);
            assert!(!after.stage.blocks_native_write());
        });
    }

    #[test]
    fn every_registered_writer_makes_its_crash_window_resolvable() {
        // All four preallocate the native ID and persist it before producing
        // content, which is what makes "no ID on the row" a proof rather
        // than a guess. A writer that could not do that would have to say
        // so, and reconciliation would stop at ambiguous instead of
        // releasing the row for retry.
        for provider in ["codex", "opencode", "hermes", "gemini"] {
            let writer = native::writer_for(provider).expect("writer");
            assert_eq!(
                writer.reconciliation_model(),
                ReconciliationModel::IdBeforeContent,
                "{provider}"
            );
        }
    }

    #[test]
    fn reconciling_touches_only_unresolved_rows() {
        let state = tempfile::tempdir().expect("state");
        identity::with_local_state_dir(state.path(), || {
            let db = Database::memory().expect("db");
            let store = ReceiptStore::new(&db);

            // `claude` has no writer, so reconciliation stops before
            // touching any installed CLI. What this test is about is which
            // rows are visited at all.
            let mut settled = attempt(RestoreStage::PackageVerified, "claude", None);
            settled.attempt_id = "settled".to_string();
            settled.request_id = "req-settled".to_string();
            store.claim(settled.clone()).expect("claim settled");

            let mut pending = attempt(RestoreStage::NativeWritePending, "claude", None);
            pending.attempt_id = "pending".to_string();
            pending.request_id = "req-pending".to_string();
            store.claim(pending.clone()).expect("claim pending");
            store
                .set_stage("pending", RestoreStage::NativeWritePending, None, now_ms())
                .expect("stage");

            let reconciled = reconcile_all(&db).expect("reconcile all");
            assert_eq!(reconciled.len(), 1);
            assert_eq!(reconciled[0].attempt_id, "pending");
            assert_eq!(
                store.get("settled").expect("settled row").stage,
                RestoreStage::PackageVerified
            );
        });
    }
}
