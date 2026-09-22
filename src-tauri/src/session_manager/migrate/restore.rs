//! Ordering around one native write.
//!
//! The native write is a subprocess or an external protocol call, so it cannot
//! join the local SQLite transaction and there is no cross-process
//! exactly-once. What this module can guarantee is that the crash window is
//! always resolvable:
//!
//! 1. the receipt row exists, with a pre-allocated nonce, before anything is
//!    written;
//! 2. the row moves to `nativeWritePending` and no second write may start
//!    while it stays unresolved;
//! 3. an ID the target hands back is persisted before the content side effect,
//!    through [`NativeWriteContext::record_native_id`].
//!
//! A crash at any point therefore leaves either "no side effect" or "a locatable
//! candidate", never "something happened somewhere".
//!
//! The stage a row ends in is decided by evidence the system obtained. A
//! successful `import` exit code means `nativeWritten` and nothing more;
//! `nativeReadbackVerified` requires the target's own read channel to show the
//! same final-only projection back.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use crate::database::dao::session_restore::AttemptClaim;
use crate::database::Database;

use super::capability;
use super::identity;
use super::model::{
    MigratableSession, MigrationError, MigrationResult, PackageReadResult, RestoreAttempt,
    RestoreRequest, RestoreRequestKind, RestoreStage,
};
use super::native::{
    self, NativeRestoreInput, NativeSessionWriter, NativeWriteContext, NativeWriteOutcome,
    ReadbackVerdict,
};
use super::package;
use super::receipt::{now_ms, ReceiptStore};

/// Serializes restoring, verification and reconciliation.
///
/// Reconciliation reasons about rows in `nativeWritePending`, and that
/// reasoning is only valid if no write is in flight: a row belonging to a
/// writer that is still running would otherwise be read as a crashed attempt
/// and retried, producing the duplicate this whole design exists to prevent.
///
/// One process-wide lock is sufficient because the application is
/// single-instance (`tauri_plugin_single_instance` in `lib.rs`), so a pending
/// row that outlives this lock belongs to a process that no longer exists.
static MIGRATION_LOCK: Mutex<()> = Mutex::new(());

/// Hold the serialization lock for the duration of one public operation.
///
/// Taken only at public entry points; the helpers below assume it is already
/// held and never re-acquire it.
pub(crate) fn serialize_migration() -> MutexGuard<'static, ()> {
    // A panic inside a previous holder tells us nothing about the receipt
    // table, which is guarded by SQLite, so the poison flag is not a reason
    // to refuse every later migration.
    MIGRATION_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Read a package and attach the receipts this machine already has for it.
///
/// The receipts are what let the import screen say "this conversation is
/// already here" from recorded evidence instead of inferring it from the
/// transcript.
pub fn read_for_restore(db: &Database, path: &str) -> MigrationResult<PackageReadResult> {
    let package = package::read_package(path)?;
    let snapshot_ids: Vec<String> = package
        .sessions
        .iter()
        .map(|session| session.snapshot_id.clone())
        .collect();
    let attempts = ReceiptStore::new(db).for_snapshots(&snapshot_ids)?;
    Ok(PackageReadResult { package, attempts })
}

/// Restore the requested snapshots from a package file.
///
/// Returns one receipt per requested snapshot. A snapshot that cannot be
/// written does not fail the call: its row carries the stage and the error, so
/// a mixed batch reports per-session truth instead of collapsing to one
/// message. Call-level problems — unreadable package, closed capability gate,
/// missing target directory — do fail the call, because no row can be trusted
/// under them.
pub fn restore_package(
    db: &Database,
    request: &RestoreRequest,
) -> MigrationResult<Vec<RestoreAttempt>> {
    let _serialized = serialize_migration();
    capability::require_write_gate(&request.target_provider_id)?;
    let writer = require_writer(&request.target_provider_id)?;
    restore_with_writer(db, request, writer)
}

/// Same orchestration with the writer supplied by the caller.
///
/// The capability gate above is about this machine's installed CLI; the
/// ordering rules below are not, which is why a test can exercise them with a
/// fake writer and still be testing the real code path.
pub(crate) fn restore_with_writer(
    db: &Database,
    request: &RestoreRequest,
    writer: &dyn NativeSessionWriter,
) -> MigrationResult<Vec<RestoreAttempt>> {
    let workspace = resolve_workspace(&request.target_workspace)?;
    let target_store_id = writer.resolve_target_store_id()?;

    let package = package::read_package(&request.package_path)?;
    let selected = select_sessions(&package.sessions, &request.snapshot_ids)?;

    // Whole-batch pre-flight. Every reason to refuse the request is checked
    // before the first native write, so a mismatched second session cannot
    // abort a call that has already written the first one.
    identity::validate_identity_field("requestId", &request.request_id)?;
    for session in &selected {
        if session.origin.provider_id != request.target_provider_id {
            return Err(MigrationError::ProviderMismatch {
                source_provider_id: session.origin.provider_id.clone(),
                target_provider_id: request.target_provider_id.clone(),
            });
        }
    }

    let store = ReceiptStore::new(db);
    let device_binding = store.binding_for(&target_store_id)?;
    let selection: Vec<String> = selected
        .iter()
        .map(|session| session.snapshot_id.clone())
        .collect();
    let proposed: Vec<RestoreAttempt> = selected
        .iter()
        .map(|session| {
            build_proposed_attempt(
                request,
                session,
                &target_store_id,
                &device_binding,
                &workspace,
            )
        })
        .collect();

    // The whole action is reserved in one transaction. Claiming row by row
    // would let the third session's identity conflict surface as a call-level
    // error after the first two had already been written into the target,
    // which is the one failure mode a per-row result cannot describe.
    let claims = store.claim_batch(proposed, &selection)?;

    let mut attempts = Vec::with_capacity(claims.len());
    for (claim, session) in claims.into_iter().zip(selected) {
        attempts.push(execute_claim(
            &store,
            writer,
            claim,
            session,
            &target_store_id,
            &workspace,
        )?);
    }
    Ok(attempts)
}

fn require_writer(provider_id: &str) -> MigrationResult<&'static dyn NativeSessionWriter> {
    native::writer_for(provider_id).ok_or_else(|| MigrationError::ProviderVersionUnsupported {
        provider_id: provider_id.to_string(),
        detected: None,
        verified: Vec::new(),
    })
}

/// Everything that has to still be true before acting on an old receipt: the
/// installed version is one we verified, and the store is the same instance.
///
/// Both are re-checked rather than trusted from the row. A receipt is a
/// statement about a moment; the provider can be upgraded or reinstalled
/// between the write and the next time anyone looks.
pub(crate) fn require_verified_target(
    writer: &dyn NativeSessionWriter,
    attempt: &RestoreAttempt,
) -> MigrationResult<()> {
    capability::require_write_gate(&attempt.target_provider_id)?;
    require_same_store(writer, attempt)
}

/// Confirm the store this receipt was written against is still the store on
/// this machine.
///
/// A receipt maps a snapshot into one store instance. If the provider's data
/// directory was replaced, the recorded native ID means nothing here, and
/// acting on it would verify or open someone else's session.
pub(crate) fn require_same_store(
    writer: &dyn NativeSessionWriter,
    attempt: &RestoreAttempt,
) -> MigrationResult<()> {
    let current = writer.resolve_target_store_id()?;
    if current != attempt.target_store_id {
        return Err(MigrationError::TargetStoreUnidentified {
            provider_id: attempt.target_provider_id.clone(),
        });
    }
    Ok(())
}

/// Ask the target's own read channel about an attempt and record the verdict.
///
/// Separate from [`restore_package`] because the answer can change after the
/// fact: a target that indexes asynchronously may only become able to answer
/// minutes later, and the user can ask again.
pub fn verify_native_readback(db: &Database, attempt_id: &str) -> MigrationResult<RestoreAttempt> {
    let _serialized = serialize_migration();
    let store = ReceiptStore::new(db);
    let attempt = store.get(attempt_id)?;
    let writer = require_writer(&attempt.target_provider_id)?;
    require_verified_target(writer, &attempt)?;
    let Some(native_id) = attempt.target_native_id.clone() else {
        return Err(MigrationError::ReconciliationRequired {
            attempt_id: attempt_id.to_string(),
            reason: "no native id is recorded for this attempt".to_string(),
        });
    };

    let expected = expectation_from_receipt(&attempt);
    apply_readback(&store, writer, &attempt, &native_id, &expected)?;
    store.get(attempt_id)
}

/// Launch the target CLI on a restored session.
///
/// The invocation is rebuilt here from the provider ID and the native ID the
/// target itself returned. Nothing executable ever travels inside a package.
///
/// Two things this deliberately does not do:
///
/// * It does not move the stage. Starting a terminal is a request to open,
///   not evidence that the target loaded the conversation, so `targetOpened`
///   stays reserved for evidence we do not have yet. `nativeReadbackVerified`
///   and the user's own attestation each remain what they were.
/// * It does not open a row that was never read back. `nativeWritten` means
///   an exit code, and launching a CLI on the strength of that would present
///   an unverified write as a restored conversation.
pub fn open_restored_session(db: &Database, attempt_id: &str) -> MigrationResult<bool> {
    let _serialized = serialize_migration();
    let store = ReceiptStore::new(db);
    let attempt = store.get(attempt_id)?;
    if !attempt.stage.is_restored() {
        return Err(MigrationError::ReconciliationRequired {
            attempt_id: attempt_id.to_string(),
            reason: format!(
                "stage {} has no verified native history to open",
                attempt.stage.as_str()
            ),
        });
    }
    let writer = require_writer(&attempt.target_provider_id)?;
    // Re-resolves the store and re-applies the version gate: the receipt's
    // stored hash describes the store as it was, not as it is.
    require_verified_target(writer, &attempt)?;
    let Some(native_id) = attempt.target_native_id.as_deref() else {
        return Err(MigrationError::ReconciliationRequired {
            attempt_id: attempt_id.to_string(),
            reason: "no native id is recorded for this attempt".to_string(),
        });
    };
    let command = resume_invocation(writer, native_id)?;

    let terminal = crate::settings::get_preferred_terminal().ok_or_else(|| {
        MigrationError::CapabilityProbeFailed {
            provider_id: attempt.target_provider_id.clone(),
            reason: "no supported terminal on this platform".to_string(),
        }
    })?;
    crate::session_manager::terminal::launch_terminal(
        &terminal,
        &command,
        attempt.target_workspace.as_deref(),
        None,
    )
    .map_err(|reason| MigrationError::NativeProtocolFailed {
        provider_id: attempt.target_provider_id.clone(),
        method: "launchTerminal".to_string(),
        reason,
    })?;

    Ok(true)
}

/// Resume invocation for one provider: the exact executable that was written
/// through, bound to the exact store that was written to.
///
/// A bare `codex resume <id>` would be resolved by the login shell's `PATH`
/// and would read that CLI's default store. Both can differ from what the
/// writer used — a second installation, or a FyAgent store override — and the
/// user would then be looking at a different history while the receipt claims
/// this one. Anything that cannot be pinned is refused rather than launched.
fn resume_invocation(writer: &dyn NativeSessionWriter, native_id: &str) -> MigrationResult<String> {
    let provider_id = writer.provider_id();
    let Some(argument) = writer.resume_argument(native_id) else {
        return Err(MigrationError::IdentityFieldInvalid {
            field: "targetNativeId".to_string(),
            reason: "not usable as a terminal argument".to_string(),
        });
    };
    let Some(resume_flag) = resume_flag(provider_id) else {
        // No proven non-interactive resume invocation for this provider. The
        // session is in the target's store; we simply cannot promise that a
        // command we invented would open it.
        return Err(MigrationError::NativeProtocolFailed {
            provider_id: provider_id.to_string(),
            method: "resume".to_string(),
            reason: "this provider has no verified resume invocation".to_string(),
        });
    };
    if native::user_cli_execution_blocked().is_some() {
        return Err(MigrationError::NativeProtocolFailed {
            provider_id: provider_id.to_string(),
            method: "resume".to_string(),
            reason: "user-owned CLIs cannot be launched from this build".to_string(),
        });
    }
    let executable = native::provider_cli_path(provider_id).ok_or_else(|| {
        MigrationError::ProviderNotInstalled {
            provider_id: provider_id.to_string(),
        }
    })?;

    let mut prefix = String::new();
    for (name, value) in store_environment(provider_id)? {
        prefix.push_str(&format!(
            "{name}={} ",
            shell_quote(&value.to_string_lossy())
        ));
    }
    Ok(format!(
        "{prefix}{} {resume_flag} {argument}",
        shell_quote(&executable.to_string_lossy())
    ))
}

/// The subcommand or flag each CLI uses to reopen one session by ID.
///
/// Same strings the session list already generates in
/// `session_manager::providers`. Hermes is absent there and here: the
/// installed CLI has no verified non-interactive resume-by-id entry point.
fn resume_flag(provider_id: &str) -> Option<&'static str> {
    match provider_id {
        "codex" => Some("resume"),
        "opencode" => Some("-s"),
        "gemini" => Some("--resume"),
        _ => None,
    }
}

/// Environment that binds a CLI to the store the writer actually wrote to.
///
/// These are the same overrides the writers set on their own subprocesses
/// (`native/{codex,opencode,hermes,gemini}.rs`). They are read from the same
/// configuration helpers, so a store the user redirected inside FyAgent is
/// the store the terminal opens. [`require_same_store`] has already confirmed
/// this resolves to the store on the receipt.
fn store_environment(provider_id: &str) -> MigrationResult<Vec<(&'static str, PathBuf)>> {
    let pairs = match provider_id {
        "codex" => vec![("CODEX_HOME", crate::codex_config::get_codex_config_dir())],
        "opencode" => vec![
            (
                "OPENCODE_DB",
                crate::opencode_config::get_opencode_db_path(),
            ),
            (
                "OPENCODE_CONFIG_DIR",
                crate::opencode_config::get_opencode_dir(),
            ),
        ],
        "hermes" => vec![("HERMES_HOME", crate::hermes_config::get_hermes_dir())],
        "gemini" => {
            // `GEMINI_CLI_HOME` is the home root; the CLI appends `.gemini`
            // itself, which is why the writer binds the parent directory.
            let directory = crate::gemini_config::get_gemini_dir();
            let home = directory
                .parent()
                .filter(|_| directory.file_name().and_then(|name| name.to_str()) == Some(".gemini"))
                .ok_or_else(|| MigrationError::TargetStoreUnidentified {
                    provider_id: provider_id.to_string(),
                })?;
            vec![("GEMINI_CLI_HOME", home.to_path_buf())]
        }
        _ => Vec::new(),
    };
    Ok(pairs)
}

/// Quote one value for the login shell the terminal launchers use.
///
/// Mirrors `session_manager::terminal::shell_escape`, which is private to
/// that module: single quotes disable every expansion, and the one character
/// they cannot carry is closed, escaped and reopened.
fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', r"'\''"))
}

/// The receipt row this request would create for one session.
///
/// Pure: it allocates identifiers and nothing else, so the whole batch can be
/// built and handed to the receipt table before any of it is acted on.
fn build_proposed_attempt(
    request: &RestoreRequest,
    session: &MigratableSession,
    target_store_id: &str,
    device_binding: &str,
    workspace: &Path,
) -> RestoreAttempt {
    let now = now_ms();
    let idempotency_slot = match request.request_kind {
        RestoreRequestKind::DefaultImport => Some(identity::idempotency_slot(
            &session.snapshot_id,
            &request.target_provider_id,
            target_store_id,
            device_binding,
        )),
        // An explicit extra copy takes no slot: the user asked for a second
        // conversation, and both mappings are kept.
        RestoreRequestKind::SaveAsNewCopy => None,
    };

    RestoreAttempt {
        attempt_id: identity::new_attempt_id(),
        request_id: request.request_id.clone(),
        snapshot_id: session.snapshot_id.clone(),
        request_kind: request.request_kind,
        idempotency_slot,
        content_digest: session.content_digest.clone(),
        origin: session.origin.clone(),
        target_provider_id: request.target_provider_id.clone(),
        target_store_id: target_store_id.to_string(),
        target_native_nonce: identity::new_nonce(),
        target_native_id: None,
        target_workspace: Some(workspace.to_string_lossy().to_string()),
        stage: RestoreStage::PackageVerified,
        attempt_count: 0,
        user_attestation: None,
        last_error: None,
        created_at: now,
        updated_at: now,
    }
}

/// Carry out one already-reserved row.
fn execute_claim(
    store: &ReceiptStore<'_>,
    writer: &dyn NativeSessionWriter,
    claim: AttemptClaim,
    session: &MigratableSession,
    target_store_id: &str,
    workspace: &Path,
) -> MigrationResult<RestoreAttempt> {
    let attempt = match claim {
        AttemptClaim::Created(record) => record.attempt,
        // The same user action arriving twice, or a resend after a timeout.
        // The receipt table has already checked that this request still means
        // the same thing; a changed workspace or selection never gets here.
        AttemptClaim::ExistingRequest(record) => record.attempt,
        // A different action already owns the default slot for this snapshot.
        // Returning that row is the honest answer: the conversation is
        // already here, under a receipt the user can inspect.
        AttemptClaim::SlotTaken(record) => return Ok(record.attempt),
    };

    if attempt.stage.is_restored() || attempt.stage == RestoreStage::NativeWritten {
        return Ok(attempt);
    }
    if attempt.stage.blocks_native_write() {
        // Unknown outcome. Reconciliation has to run first; writing again
        // could duplicate a conversation that is already there.
        store.set_stage(
            &attempt.attempt_id,
            attempt.stage,
            Some(&MigrationError::ReconciliationRequired {
                attempt_id: attempt.attempt_id.clone(),
                reason: "a previous write for this attempt has an unknown outcome".to_string(),
            }),
            now_ms(),
        )?;
        return store.get(&attempt.attempt_id);
    }

    if !store.begin_native_write(&attempt.attempt_id, now_ms())? {
        // Lost the race against a concurrent command; that caller owns it.
        return store.get(&attempt.attempt_id);
    }

    let input = NativeRestoreInput {
        session: session.clone(),
        target_workspace: workspace.to_path_buf(),
        target_store_id: target_store_id.to_string(),
        target_native_nonce: attempt.target_native_nonce.clone(),
        // A retry of a proven-side-effect-free attempt must land on the ID
        // the receipt already carries. Minting a second one would leave the
        // row pointing at a thread nobody wrote.
        target_native_id: attempt.target_native_id.clone(),
    };
    let context = ReceiptContext {
        store,
        attempt_id: attempt.attempt_id.clone(),
    };

    match writer.restore(&input, &context) {
        NativeWriteOutcome::ProvenNoSideEffect { error } => {
            // Positive evidence that nothing was written, so this row may be
            // retried later without risking a duplicate.
            store.set_stage(
                &attempt.attempt_id,
                RestoreStage::Failed,
                Some(&error),
                now_ms(),
            )?;
        }
        NativeWriteOutcome::Unresolved { error } => {
            store.set_stage(
                &attempt.attempt_id,
                RestoreStage::NeedsReconciliation,
                Some(&error),
                now_ms(),
            )?;
        }
        NativeWriteOutcome::Written {
            target_native_id,
            nonce_carrier,
        } => {
            store.set_stage(
                &attempt.attempt_id,
                RestoreStage::NativeWritten,
                None,
                now_ms(),
            )?;
            let native_id = match target_native_id {
                Some(id) => {
                    store.set_native_id(&attempt.attempt_id, &id, now_ms())?;
                    Some(id)
                }
                // The target returned success without an ID. The nonce is the
                // only anchor, and looking for it is meaningful only if the
                // writer actually placed it somewhere.
                None => {
                    let current = store.get(&attempt.attempt_id)?;
                    match current.target_native_id {
                        Some(id) => Some(id),
                        None => {
                            resolve_missing_id(store, writer, &attempt, nonce_carrier)?;
                            store.get(&attempt.attempt_id)?.target_native_id
                        }
                    }
                }
            };
            if let Some(native_id) = native_id {
                apply_readback(store, writer, &attempt, &native_id, session)?;
            }
        }
    }

    store.get(&attempt.attempt_id)
}

/// Ask the target's own read channel and translate the verdict into a stage.
fn apply_readback(
    store: &ReceiptStore<'_>,
    writer: &dyn NativeSessionWriter,
    attempt: &RestoreAttempt,
    native_id: &str,
    session: &MigratableSession,
) -> MigrationResult<()> {
    // Read the row back rather than trusting the caller's copy: a retry may
    // have advanced it since.
    let current = store.get(&attempt.attempt_id)?.stage;
    let verdict = match writer.verify_readback(native_id, session) {
        Ok(verdict) => verdict,
        // A failing read channel is not evidence about the write. The row
        // keeps whatever it had proved and records why verification could not
        // run.
        Err(error) => {
            store.set_stage(&attempt.attempt_id, current, Some(&error), now_ms())?;
            return Ok(());
        }
    };

    let (stage, error) = match verdict {
        // Never regress: a row that was already opened or continued keeps the
        // stronger evidence it earned.
        ReadbackVerdict::Visible { .. } if current.is_restored() => (current, None),
        ReadbackVerdict::Visible { .. } => (RestoreStage::NativeReadbackVerified, None),
        // Fresh contradicting evidence. It replaces an earlier positive
        // verdict on purpose: the current state of the target store is what
        // the user will see.
        ReadbackVerdict::Mismatch { observed } => (
            RestoreStage::Ambiguous,
            Some(MigrationError::NativeReadbackMismatch {
                attempt_id: attempt.attempt_id.clone(),
                expected_digest: session.content_digest.clone(),
                observed,
            }),
        ),
        // The write reported success and the target says the session is not
        // there. Contradictory evidence is never resolved by guessing.
        ReadbackVerdict::NotVisible => (
            RestoreStage::NeedsReconciliation,
            Some(MigrationError::ReconciliationRequired {
                attempt_id: attempt.attempt_id.clone(),
                reason: "the target reports the written session is not visible".to_string(),
            }),
        ),
        // No read channel exists in this version. Absence proves nothing, so
        // the row keeps whatever it already had. This is the release-level cap
        // for such a provider, not a transient state.
        ReadbackVerdict::Blocked { reason } => {
            log::info!(
                "{}: native readback unavailable ({reason})",
                writer.provider_id()
            );
            (current, None)
        }
        ReadbackVerdict::Ambiguous { candidates } => (
            RestoreStage::Ambiguous,
            Some(MigrationError::AmbiguousNativeMatch {
                attempt_id: attempt.attempt_id.clone(),
                candidates,
            }),
        ),
    };
    store.set_stage(&attempt.attempt_id, stage, error.as_ref(), now_ms())
}

/// Find the native ID for a write that returned success without one.
fn resolve_missing_id(
    store: &ReceiptStore<'_>,
    writer: &dyn NativeSessionWriter,
    attempt: &RestoreAttempt,
    nonce_carrier: native::NonceCarrier,
) -> MigrationResult<()> {
    if nonce_carrier == native::NonceCarrier::NotUsed {
        // Nothing was planted, so there is nothing to search for. Saying
        // "needs reconciliation" is the truthful outcome; inventing an ID or
        // matching on content would be a guess about the user's history.
        return store.set_stage(
            &attempt.attempt_id,
            RestoreStage::NeedsReconciliation,
            Some(&MigrationError::ReconciliationRequired {
                attempt_id: attempt.attempt_id.clone(),
                reason: "the target returned no id and carried no nonce".to_string(),
            }),
            now_ms(),
        );
    }

    match writer.locate_by_nonce(&attempt.target_native_nonce) {
        Ok(candidates) if candidates.len() == 1 => {
            store.set_native_id(&attempt.attempt_id, &candidates[0], now_ms())
        }
        Ok(candidates) if candidates.is_empty() => store.set_stage(
            &attempt.attempt_id,
            RestoreStage::NeedsReconciliation,
            Some(&MigrationError::ReconciliationRequired {
                attempt_id: attempt.attempt_id.clone(),
                reason: "the nonce is not present in the target store".to_string(),
            }),
            now_ms(),
        ),
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

/// What a receipt can still assert about the transcript it was created from.
///
/// Verification may be re-run days later, when the package file is gone. The
/// receipt keeps identity and the content digest but no message bodies, so
/// this is the complete expectation and the reason writers compare digests
/// rather than text. The empty `messages` here is the honest representation of
/// "the bodies are in the provider's store, not in our table"; it never
/// reaches a package, where zero messages is invalid.
pub(crate) fn expectation_from_receipt(attempt: &RestoreAttempt) -> MigratableSession {
    use super::model::{ExtractionReport, OmittedCounts, PathFamily};
    MigratableSession {
        snapshot_id: attempt.snapshot_id.clone(),
        content_digest: attempt.content_digest.clone(),
        origin: attempt.origin.clone(),
        title: None,
        created_at: None,
        last_active_at: None,
        workspace_label: None,
        messages: Vec::new(),
        extraction: ExtractionReport {
            rule_id: String::new(),
            rule_verified_versions: Vec::new(),
            open_user_messages: Vec::new(),
            omitted: OmittedCounts::default(),
            source_path_family: PathFamily::Unknown,
        },
    }
}

/// The target directory must exist on this machine right now.
fn resolve_workspace(raw: &str) -> MigrationResult<PathBuf> {
    let path = PathBuf::from(raw);
    if !path.is_absolute() {
        return Err(MigrationError::TargetDirectoryNotFound {
            path: raw.to_string(),
        });
    }
    let canonical = path
        .canonicalize()
        .map_err(|_| MigrationError::TargetDirectoryNotFound {
            path: raw.to_string(),
        })?;
    if !canonical.is_dir() {
        return Err(MigrationError::TargetDirectoryNotFound {
            path: raw.to_string(),
        });
    }
    Ok(canonical)
}

/// Pick the requested snapshots, in package order.
///
/// Selection is explicit: an empty set or a missing snapshot fails before
/// claiming a receipt or writing anything. “All” callers enumerate the IDs.
fn select_sessions<'a>(
    sessions: &'a [MigratableSession],
    snapshot_ids: &[String],
) -> MigrationResult<Vec<&'a MigratableSession>> {
    if snapshot_ids.is_empty() {
        return Err(MigrationError::PackageMalformed {
            reason: "select at least one session to restore".into(),
        });
    }
    let mut selected = Vec::with_capacity(snapshot_ids.len());
    for wanted in snapshot_ids {
        let found = sessions
            .iter()
            .find(|session| &session.snapshot_id == wanted)
            .ok_or_else(|| MigrationError::PackageMalformed {
                reason: format!("requested snapshot {wanted} is not in this package"),
            })?;
        selected.push(found);
    }
    Ok(selected)
}

struct ReceiptContext<'a> {
    store: &'a ReceiptStore<'a>,
    attempt_id: String,
}

impl NativeWriteContext for ReceiptContext<'_> {
    fn record_native_id(&self, native_id: &str) -> MigrationResult<()> {
        identity::validate_identity_field("targetNativeId", native_id)?;
        self.store
            .set_native_id(&self.attempt_id, native_id, now_ms())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_relative_or_missing_target_directory_is_refused() {
        assert_eq!(
            resolve_workspace("relative/dir").unwrap_err().code(),
            "targetDirectoryNotFound"
        );
        assert_eq!(
            resolve_workspace("/definitely/not/here/fyagent")
                .unwrap_err()
                .code(),
            "targetDirectoryNotFound"
        );
    }

    #[test]
    fn an_existing_directory_resolves_to_a_canonical_path() {
        let dir = tempfile::tempdir().expect("temp dir");
        let resolved = resolve_workspace(&dir.path().to_string_lossy()).expect("resolve");
        assert!(resolved.is_absolute());
        assert!(resolved.is_dir());
    }

    #[test]
    fn a_file_is_not_a_target_directory() {
        let dir = tempfile::tempdir().expect("temp dir");
        let file = dir.path().join("package.json");
        std::fs::write(&file, b"{}").expect("write");
        assert_eq!(
            resolve_workspace(&file.to_string_lossy())
                .unwrap_err()
                .code(),
            "targetDirectoryNotFound"
        );
    }

    #[test]
    fn a_resume_invocation_pins_the_executable_and_the_store() {
        let writer = native::writer_for("codex").expect("codex writer");
        if native::provider_cli_path("codex").is_none() {
            // Nothing installed on this machine; the refusal is the subject
            // of its own test below.
            return;
        }
        let command =
            resume_invocation(writer, "d48fbc0c-029c-4143-8794-c26891c02bf3").expect("invocation");
        let executable = native::provider_cli_path("codex").expect("executable");
        assert!(
            command.contains(&shell_quote(&executable.to_string_lossy())),
            "{command}"
        );
        assert!(command.contains("CODEX_HOME="), "{command}");
        assert!(
            command.ends_with(" resume d48fbc0c-029c-4143-8794-c26891c02bf3"),
            "{command}"
        );
        // A bare launcher name would be resolved by the login shell's PATH,
        // which is the installation this feature may never have touched.
        assert!(!command.starts_with("codex "), "{command}");
    }

    #[test]
    fn a_provider_without_a_verified_resume_is_refused_rather_than_guessed() {
        let writer = native::writer_for("hermes").expect("hermes writer");
        let error = resume_invocation(writer, "session123").expect_err("must refuse");
        assert_eq!(error.code(), "nativeProtocolFailed");
    }

    #[test]
    fn a_hostile_native_id_never_reaches_a_terminal() {
        let writer = native::writer_for("codex").expect("codex writer");
        for hostile in [
            "x; touch /tmp/fyagent-restore-injection #",
            "$(touch /tmp/fyagent-restore-substitution)",
            "--dangerously-bypass-approvals-and-sandbox",
            "",
        ] {
            let error = resume_invocation(writer, hostile).expect_err(hostile);
            assert_eq!(error.code(), "identityFieldInvalid", "{hostile}");
        }
    }

    #[test]
    fn shell_quoting_neutralizes_a_hostile_store_path() {
        assert_eq!(shell_quote("/tmp/$(id -un)"), "'/tmp/$(id -un)'");
        assert_eq!(shell_quote("/tmp/it's"), r"'/tmp/it'\''s'");
    }

    #[test]
    fn selection_is_by_snapshot_id_and_missing_ids_fail_closed() {
        let sessions = vec![sample_session("fys1:a"), sample_session("fys1:b")];
        assert!(select_sessions(&sessions, &[]).is_err());

        let one = select_sessions(&sessions, &["fys1:b".to_string()]).expect("one");
        assert_eq!(one.len(), 1);
        assert_eq!(one[0].snapshot_id, "fys1:b");

        let error = select_sessions(&sessions, &["fys1:missing".to_string()]).unwrap_err();
        assert_eq!(error.code(), "packageMalformed");
    }

    fn sample_session(snapshot_id: &str) -> MigratableSession {
        session_from("fyo1:origin", snapshot_id, "codex")
    }

    /// A session whose digest and snapshot ID are the real computed ones, so
    /// it survives `write_package`. Different `text` means a different
    /// snapshot of the same origin, which is what the conflict cases need.
    fn valid_session(origin_id: &str, text: &str, provider_id: &str) -> MigratableSession {
        let mut session = session_from(origin_id, "", provider_id);
        session.messages[0].text = text.to_string();
        session.content_digest = identity::content_digest(&session.messages);
        session.snapshot_id = identity::snapshot_id(origin_id, &session.content_digest);
        session
    }

    fn session_from(origin_id: &str, snapshot_id: &str, provider_id: &str) -> MigratableSession {
        use super::super::model::{
            ExtractionReport, MessageKind, MigratableMessage, OmittedCounts, OriginIdentity,
            PathFamily,
        };
        MigratableSession {
            snapshot_id: snapshot_id.to_string(),
            content_digest: format!("fyc1:{snapshot_id}"),
            origin: OriginIdentity {
                origin_id: origin_id.to_string(),
                provider_id: provider_id.to_string(),
                session_id: None,
                cli_version: None,
                store_fingerprint: None,
            },
            title: None,
            created_at: None,
            last_active_at: None,
            workspace_label: None,
            messages: vec![MigratableMessage {
                seq: 0,
                kind: MessageKind::UserText,
                text: "hello".to_string(),
                ts: None,
            }],
            extraction: ExtractionReport {
                rule_id: "codex.rollout.phase-v1".to_string(),
                rule_verified_versions: vec!["=0.154.0".to_string()],
                open_user_messages: vec![0],
                omitted: OmittedCounts::default(),
                source_path_family: PathFamily::Posix,
            },
        }
    }

    // —— orchestration against a fake target ————————————————————————
    //
    // These exercise the production path: real receipt table, real claim,
    // real ordering, real stage decisions. Only the native call is replaced,
    // because the questions here — does a repeated action write twice, does
    // an unknown outcome get retried — are about the orchestration and
    // cannot be answered by testing a writer.

    use std::collections::VecDeque;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex as StdMutex;

    /// What the fake target does on one `restore` call.
    #[derive(Clone, Copy)]
    enum Act {
        /// Record the ID first, then report a successful write.
        Write,
        /// Record the ID, then prove nothing was written. Retryable.
        RecordThenFail,
        /// Return without being able to rule out a partial write.
        Unresolved,
    }

    /// What its read channel answers.
    #[derive(Clone, Copy)]
    enum Answer {
        Visible,
        Mismatch,
        NotVisible,
        Unavailable,
    }

    struct FakeTarget {
        store_id: StdMutex<String>,
        acts: StdMutex<VecDeque<Act>>,
        answers: StdMutex<VecDeque<Answer>>,
        writes: AtomicUsize,
        reads: AtomicUsize,
        /// The `target_native_id` each `restore` call was handed.
        offered_ids: StdMutex<Vec<Option<String>>>,
    }

    impl FakeTarget {
        fn new(acts: &[Act], answers: &[Answer]) -> Self {
            Self {
                store_id: StdMutex::new("store-1".to_string()),
                acts: StdMutex::new(acts.iter().copied().collect()),
                answers: StdMutex::new(answers.iter().copied().collect()),
                writes: AtomicUsize::new(0),
                reads: AtomicUsize::new(0),
                offered_ids: StdMutex::new(Vec::new()),
            }
        }

        fn writes(&self) -> usize {
            self.writes.load(Ordering::SeqCst)
        }

        fn next_act(&self) -> Act {
            self.acts.lock().unwrap().pop_front().unwrap_or(Act::Write)
        }

        fn next_answer(&self) -> Answer {
            self.answers
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or(Answer::Visible)
        }
    }

    impl NativeSessionWriter for FakeTarget {
        fn provider_id(&self) -> &'static str {
            "codex"
        }

        fn write_strategy(&self) -> &'static str {
            "fake.v1"
        }

        fn verified_write_versions(&self) -> &'static [&'static str] {
            &["=0.0.0"]
        }

        fn resolve_target_store_id(&self) -> MigrationResult<String> {
            Ok(self.store_id.lock().unwrap().clone())
        }

        fn restore(
            &self,
            input: &NativeRestoreInput,
            context: &dyn NativeWriteContext,
        ) -> NativeWriteOutcome {
            self.writes.fetch_add(1, Ordering::SeqCst);
            self.offered_ids
                .lock()
                .unwrap()
                .push(input.target_native_id.clone());
            // A real writer either reuses the ID the receipt already carries
            // or mints one and persists it before touching content.
            let native_id = input
                .target_native_id
                .clone()
                .unwrap_or_else(|| format!("fake-{}", self.writes()));
            match self.next_act() {
                Act::Write => {
                    if let Err(error) = context.record_native_id(&native_id) {
                        return NativeWriteOutcome::ProvenNoSideEffect { error };
                    }
                    NativeWriteOutcome::Written {
                        target_native_id: Some(native_id),
                        nonce_carrier: native::NonceCarrier::NotUsed,
                    }
                }
                Act::RecordThenFail => {
                    if let Err(error) = context.record_native_id(&native_id) {
                        return NativeWriteOutcome::ProvenNoSideEffect { error };
                    }
                    NativeWriteOutcome::ProvenNoSideEffect {
                        error: MigrationError::NativeProtocolFailed {
                            provider_id: "codex".to_string(),
                            method: "import".to_string(),
                            reason: "refused before writing".to_string(),
                        },
                    }
                }
                Act::Unresolved => NativeWriteOutcome::Unresolved {
                    error: MigrationError::NativeProtocolFailed {
                        provider_id: "codex".to_string(),
                        method: "import".to_string(),
                        reason: "the importer was killed".to_string(),
                    },
                },
            }
        }

        fn verify_readback(
            &self,
            _native_id: &str,
            expected: &MigratableSession,
        ) -> MigrationResult<ReadbackVerdict> {
            self.reads.fetch_add(1, Ordering::SeqCst);
            match self.next_answer() {
                Answer::Visible => Ok(ReadbackVerdict::Visible {
                    observed_digest: expected.content_digest.clone(),
                }),
                Answer::Mismatch => Ok(ReadbackVerdict::Mismatch {
                    observed: "different-digest".into(),
                }),
                Answer::NotVisible => Ok(ReadbackVerdict::NotVisible),
                Answer::Unavailable => Err(MigrationError::NativeProtocolFailed {
                    provider_id: "codex".to_string(),
                    method: "export".to_string(),
                    reason: "the read channel did not answer".to_string(),
                }),
            }
        }

        fn locate_by_nonce(&self, _nonce: &str) -> MigrationResult<Vec<String>> {
            Ok(Vec::new())
        }
    }

    struct Fixture {
        _state: tempfile::TempDir,
        workspace: tempfile::TempDir,
        package_path: String,
        snapshot_ids: Vec<String>,
    }

    impl Fixture {
        fn new(sessions: Vec<MigratableSession>) -> Self {
            let state = tempfile::tempdir().expect("state dir");
            let workspace = tempfile::tempdir().expect("workspace");
            let package_path = workspace
                .path()
                .join("package.json")
                .to_string_lossy()
                .to_string();
            let snapshot_ids = sessions
                .iter()
                .map(|session| session.snapshot_id.clone())
                .collect();
            let package = package::build_package(sessions, 1_700_000_000_000);
            package::write_package(&package, &package_path).expect("write package");
            Self {
                _state: state,
                workspace,
                package_path,
                snapshot_ids,
            }
        }

        fn state(&self) -> &std::path::Path {
            self._state.path()
        }

        fn request(&self, request_id: &str, kind: RestoreRequestKind) -> RestoreRequest {
            RestoreRequest {
                package_path: self.package_path.clone(),
                request_id: request_id.to_string(),
                snapshot_ids: self.snapshot_ids.clone(),
                target_provider_id: "codex".to_string(),
                target_workspace: self.workspace.path().to_string_lossy().to_string(),
                request_kind: kind,
            }
        }
    }

    #[test]
    fn repeating_one_action_writes_once() {
        let fixture = Fixture::new(vec![valid_session("fyo1:origin", "hello", "codex")]);
        let target = FakeTarget::new(&[Act::Write], &[Answer::Visible, Answer::Visible]);
        identity::with_local_state_dir(fixture.state(), || {
            let db = Database::memory().expect("db");
            let request = fixture.request("action-1", RestoreRequestKind::DefaultImport);

            let first = restore_with_writer(&db, &request, &target).expect("first");
            let second = restore_with_writer(&db, &request, &target).expect("second");

            assert_eq!(first[0].attempt_id, second[0].attempt_id);
            assert_eq!(first[0].stage, RestoreStage::NativeReadbackVerified);
            assert_eq!(second[0].stage, RestoreStage::NativeReadbackVerified);
            // The second call found the row already written and did not go
            // near the target again.
            assert_eq!(target.writes(), 1);
            assert_eq!(ReceiptStore::new(&db).list().expect("list").len(), 1);
        });
    }

    #[test]
    fn a_second_action_on_the_same_snapshot_returns_the_existing_receipt() {
        // Different request ID, same default-import slot: the conversation is
        // already here and a second copy is not what the user asked for.
        let fixture = Fixture::new(vec![valid_session("fyo1:origin", "hello", "codex")]);
        let target = FakeTarget::new(&[Act::Write], &[Answer::Visible]);
        identity::with_local_state_dir(fixture.state(), || {
            let db = Database::memory().expect("db");
            let first = restore_with_writer(
                &db,
                &fixture.request("action-1", RestoreRequestKind::DefaultImport),
                &target,
            )
            .expect("first");
            let second = restore_with_writer(
                &db,
                &fixture.request("action-2", RestoreRequestKind::DefaultImport),
                &target,
            )
            .expect("second");

            assert_eq!(second[0].attempt_id, first[0].attempt_id);
            assert_eq!(target.writes(), 1);
        });
    }

    #[test]
    fn an_unresolved_write_is_never_retried_into_a_duplicate() {
        let fixture = Fixture::new(vec![valid_session("fyo1:origin", "hello", "codex")]);
        let target = FakeTarget::new(&[Act::Unresolved, Act::Write], &[Answer::Visible]);
        identity::with_local_state_dir(fixture.state(), || {
            let db = Database::memory().expect("db");
            let request = fixture.request("action-1", RestoreRequestKind::DefaultImport);

            let first = restore_with_writer(&db, &request, &target).expect("first");
            assert_eq!(first[0].stage, RestoreStage::NeedsReconciliation);

            // Retrying the same action must not write again: the first
            // outcome could have left a session behind.
            let second = restore_with_writer(&db, &request, &target).expect("second");
            assert_eq!(second[0].stage, RestoreStage::NeedsReconciliation);
            assert_eq!(
                second[0].last_error.as_ref().map(|error| error.code()),
                Some("reconciliationRequired")
            );
            assert_eq!(target.writes(), 1);
        });
    }

    #[test]
    fn a_proven_failure_retries_with_the_id_the_receipt_already_carries() {
        let fixture = Fixture::new(vec![valid_session("fyo1:origin", "hello", "codex")]);
        let target = FakeTarget::new(&[Act::RecordThenFail, Act::Write], &[Answer::Visible]);
        identity::with_local_state_dir(fixture.state(), || {
            let db = Database::memory().expect("db");
            let request = fixture.request("action-1", RestoreRequestKind::DefaultImport);

            let first = restore_with_writer(&db, &request, &target).expect("first");
            assert_eq!(first[0].stage, RestoreStage::Failed);
            let recorded = first[0]
                .target_native_id
                .clone()
                .expect("the id was recorded before the failure");

            let second = restore_with_writer(&db, &request, &target).expect("second");
            assert_eq!(second[0].stage, RestoreStage::NativeReadbackVerified);
            assert_eq!(second[0].target_native_id.as_ref(), Some(&recorded));
            assert_eq!(target.writes(), 2);

            // Minting a fresh ID on the retry would leave the receipt
            // pointing at the first, non-existent thread: the DAO refuses to
            // replace an ID that is already set.
            let offered = target.offered_ids.lock().unwrap().clone();
            assert_eq!(offered, vec![None, Some(recorded)]);
        });
    }

    #[test]
    fn a_changed_snapshot_of_one_source_is_refused_before_any_write() {
        // Same conversation, continued and re-exported. A default import of
        // the new snapshot would silently produce a second copy of a
        // conversation the user already has.
        let first_snapshot = valid_session("fyo1:same", "first turn", "codex");
        let second_snapshot = valid_session("fyo1:same", "first turn, then another", "codex");
        let first_fixture = Fixture::new(vec![first_snapshot]);
        let target = FakeTarget::new(&[Act::Write], &[Answer::Visible]);
        identity::with_local_state_dir(first_fixture.state(), || {
            let db = Database::memory().expect("db");
            restore_with_writer(
                &db,
                &first_fixture.request("action-1", RestoreRequestKind::DefaultImport),
                &target,
            )
            .expect("first");

            let second_fixture = Fixture::new(vec![second_snapshot]);
            let error = restore_with_writer(
                &db,
                &second_fixture.request("action-2", RestoreRequestKind::DefaultImport),
                &target,
            )
            .expect_err("the changed snapshot must be refused");
            assert_eq!(error.code(), "sourceSnapshotConflict");
            assert_eq!(target.writes(), 1);

            // Asking for an explicit extra copy is a different request and is
            // allowed, because the user said so.
            let copy = restore_with_writer(
                &db,
                &second_fixture.request("action-3", RestoreRequestKind::SaveAsNewCopy),
                &target,
            )
            .expect("explicit copy");
            assert_eq!(copy[0].stage, RestoreStage::NativeReadbackVerified);
            assert_eq!(target.writes(), 2);
        });
    }

    #[test]
    fn a_conflict_anywhere_in_a_batch_prevents_every_write_in_it() {
        // The conflict is on the second session. If claiming were per-row,
        // the first would already be in the target by the time it surfaced,
        // and the call-level error would hide that.
        let existing = valid_session("fyo1:same", "as exported before", "codex");
        let setup = Fixture::new(vec![existing]);
        let target = FakeTarget::new(&[Act::Write], &[Answer::Visible]);
        identity::with_local_state_dir(setup.state(), || {
            let db = Database::memory().expect("db");
            restore_with_writer(
                &db,
                &setup.request("action-1", RestoreRequestKind::DefaultImport),
                &target,
            )
            .expect("setup");
            assert_eq!(target.writes(), 1);

            let batch = Fixture::new(vec![
                valid_session("fyo1:other", "an unrelated conversation", "codex"),
                valid_session("fyo1:same", "as exported before, plus more", "codex"),
            ]);
            let error = restore_with_writer(
                &db,
                &batch.request("action-2", RestoreRequestKind::DefaultImport),
                &target,
            )
            .expect_err("the batch must be refused");
            assert_eq!(error.code(), "sourceSnapshotConflict");
            assert_eq!(
                target.writes(),
                1,
                "the innocent session must not be written"
            );
            assert_eq!(ReceiptStore::new(&db).list().expect("list").len(), 1);
        });
    }

    #[test]
    fn a_package_from_another_product_is_refused_before_any_write() {
        let fixture = Fixture::new(vec![
            valid_session("fyo1:a", "from codex", "codex"),
            valid_session("fyo1:b", "from opencode", "opencode"),
        ]);
        let target = FakeTarget::new(&[Act::Write], &[Answer::Visible]);
        identity::with_local_state_dir(fixture.state(), || {
            let db = Database::memory().expect("db");
            let error = restore_with_writer(
                &db,
                &fixture.request("action-1", RestoreRequestKind::DefaultImport),
                &target,
            )
            .expect_err("mixed providers must be refused");
            assert_eq!(error.code(), "providerMismatch");
            assert_eq!(target.writes(), 0);
            assert!(ReceiptStore::new(&db).list().expect("list").is_empty());
        });
    }

    #[test]
    fn two_concurrent_calls_for_one_action_write_once() {
        let fixture = Fixture::new(vec![valid_session("fyo1:origin", "hello", "codex")]);
        let target = FakeTarget::new(
            &[Act::Write, Act::Write],
            &[Answer::Visible, Answer::Visible],
        );
        let state = fixture.state().to_path_buf();
        // Materialize the install identity once, so both threads read the
        // same one rather than racing to create it.
        identity::with_local_state_dir(&state, || {
            identity::device_binding("store-1").expect("binding");
        });
        let db = Database::memory().expect("db");
        let request = fixture.request("action-1", RestoreRequestKind::DefaultImport);

        let results = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..2)
                .map(|_| {
                    let state = state.clone();
                    let db = &db;
                    let request = &request;
                    let target = &target;
                    scope.spawn(move || {
                        identity::with_local_state_dir(&state, || {
                            restore_with_writer(db, request, target)
                        })
                    })
                })
                .collect();
            handles
                .into_iter()
                .map(|handle| handle.join().expect("thread"))
                .collect::<Vec<_>>()
        });

        for result in &results {
            let attempts = result.as_ref().expect("both calls succeed");
            assert_eq!(attempts.len(), 1);
        }
        assert_eq!(target.writes(), 1);
        identity::with_local_state_dir(&state, || {
            assert_eq!(ReceiptStore::new(&db).list().expect("list").len(), 1);
        });
    }

    #[test]
    fn a_read_channel_that_does_not_answer_leaves_the_write_evidence_alone() {
        let fixture = Fixture::new(vec![valid_session("fyo1:origin", "hello", "codex")]);
        let target = FakeTarget::new(&[Act::Write], &[Answer::Unavailable]);
        identity::with_local_state_dir(fixture.state(), || {
            let db = Database::memory().expect("db");
            let attempts = restore_with_writer(
                &db,
                &fixture.request("action-1", RestoreRequestKind::DefaultImport),
                &target,
            )
            .expect("restore");

            // The write succeeded and verification could not run. Neither
            // fact may be upgraded into the other.
            assert_eq!(attempts[0].stage, RestoreStage::NativeWritten);
            assert_eq!(
                attempts[0].last_error.as_ref().map(|error| error.code()),
                Some("nativeProtocolFailed")
            );
        });
    }

    #[test]
    fn an_empty_selection_cannot_claim_or_write_an_entire_package() {
        let fixture = Fixture::new(vec![valid_session("fyo1:origin", "hello", "codex")]);
        let target = FakeTarget::new(&[Act::Write], &[Answer::Visible]);
        identity::with_local_state_dir(fixture.state(), || {
            let db = Database::memory().expect("db");
            let mut request = fixture.request("action-empty", RestoreRequestKind::DefaultImport);
            request.snapshot_ids.clear();
            assert!(restore_with_writer(&db, &request, &target).is_err());
            assert_eq!(target.writes(), 0);
            assert!(ReceiptStore::new(&db).list().unwrap().is_empty());
        });
    }

    #[test]
    fn contradictory_readback_becomes_ambiguous_and_never_repeats_a_write() {
        let fixture = Fixture::new(vec![valid_session("fyo1:origin", "hello", "codex")]);
        let target = FakeTarget::new(&[Act::Write], &[Answer::Mismatch, Answer::Mismatch]);
        identity::with_local_state_dir(fixture.state(), || {
            let db = Database::memory().expect("db");
            let request = fixture.request("action-mismatch", RestoreRequestKind::DefaultImport);
            let attempts = restore_with_writer(&db, &request, &target).unwrap();
            assert_eq!(attempts[0].stage, RestoreStage::Ambiguous);
            assert!(attempts[0].stage.blocks_native_write());
            let replay = restore_with_writer(&db, &request, &target).unwrap();
            assert_eq!(replay[0].stage, RestoreStage::Ambiguous);
            assert_eq!(target.writes(), 1);
            // Later negative evidence must also invalidate an older stronger stage.
            let store = ReceiptStore::new(&db);
            store
                .set_stage(
                    &attempts[0].attempt_id,
                    RestoreStage::NextTurnRequestVerified,
                    None,
                    now_ms(),
                )
                .unwrap();
            let current = store.get(&attempts[0].attempt_id).unwrap();
            let session = package::read_package(&fixture.package_path)
                .unwrap()
                .sessions
                .remove(0);
            apply_readback(
                &store,
                &target,
                &current,
                current.target_native_id.as_deref().unwrap(),
                &session,
            )
            .unwrap();
            assert_eq!(
                store.get(&current.attempt_id).unwrap().stage,
                RestoreStage::Ambiguous
            );
        });
    }

    #[test]
    fn a_target_that_denies_the_write_needs_reconciliation() {
        let fixture = Fixture::new(vec![valid_session("fyo1:origin", "hello", "codex")]);
        let target = FakeTarget::new(&[Act::Write], &[Answer::NotVisible]);
        identity::with_local_state_dir(fixture.state(), || {
            let db = Database::memory().expect("db");
            let attempts = restore_with_writer(
                &db,
                &fixture.request("action-1", RestoreRequestKind::DefaultImport),
                &target,
            )
            .expect("restore");
            assert_eq!(attempts[0].stage, RestoreStage::NeedsReconciliation);
            assert!(attempts[0].stage.blocks_native_write());
        });
    }

    #[test]
    fn a_write_that_was_never_read_back_cannot_be_opened() {
        let fixture = Fixture::new(vec![valid_session("fyo1:origin", "hello", "codex")]);
        let target = FakeTarget::new(&[Act::Write], &[Answer::Unavailable]);
        identity::with_local_state_dir(fixture.state(), || {
            let db = Database::memory().expect("db");
            let attempts = restore_with_writer(
                &db,
                &fixture.request("action-1", RestoreRequestKind::DefaultImport),
                &target,
            )
            .expect("restore");
            assert_eq!(attempts[0].stage, RestoreStage::NativeWritten);

            let error = open_restored_session(&db, &attempts[0].attempt_id)
                .expect_err("an unverified write must not be openable");
            assert_eq!(error.code(), "reconciliationRequired");

            // And the refusal did not invent a stage on the way out.
            let after = ReceiptStore::new(&db)
                .get(&attempts[0].attempt_id)
                .expect("row");
            assert_eq!(after.stage, RestoreStage::NativeWritten);
        });
    }

    #[test]
    fn a_replaced_store_invalidates_the_receipt() {
        let fixture = Fixture::new(vec![valid_session("fyo1:origin", "hello", "codex")]);
        let target = FakeTarget::new(&[Act::Write], &[Answer::Visible]);
        identity::with_local_state_dir(fixture.state(), || {
            let db = Database::memory().expect("db");
            let attempts = restore_with_writer(
                &db,
                &fixture.request("action-1", RestoreRequestKind::DefaultImport),
                &target,
            )
            .expect("restore");

            // The provider was reinstalled: same path, different store.
            *target.store_id.lock().unwrap() = "store-2".to_string();
            let error =
                require_same_store(&target, &attempts[0]).expect_err("the receipt is stale");
            assert_eq!(error.code(), "targetStoreUnidentified");
        });
    }
}
