//! IPC surface for cross-device session migration.
//!
//! These commands are a thin shell: every rule lives in
//! `session_manager::migrate`. The two things done here are moving blocking
//! work off the async runtime and turning [`MigrationError`] into the
//! `{code, detail?}` string the renderer parses.
//!
//! The renderer can never assert a verification result. It picks a package
//! path, a target directory and a request kind; every stage in a receipt comes
//! from evidence the backend obtained itself.

#![allow(non_snake_case)]

use std::sync::Arc;

use tauri::State;

use crate::database::Database;
use crate::session_manager::migrate::export::{self, ExportItem};
use crate::session_manager::migrate::model::{
    ExportOutcome, LocalProviderProbe, MigratableSession, MigrationError, MigrationResult,
    PackageReadResult, ReleaseCapability, RestoreAttempt, RestoreRequest, RestoreStage,
    UserAttestation,
};
use crate::session_manager::migrate::{capability, receipt, reconcile, restore};
use crate::store::AppState;

/// Every command reports `{code, detail?}` so the UI keys off a stable code
/// instead of parsing prose.
fn to_ipc(error: MigrationError) -> String {
    error.to_ipc_string()
}

/// Run blocking migration work off the async runtime.
async fn blocking<T, F>(work: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> MigrationResult<T> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|error| {
            to_ipc(MigrationError::ReceiptStoreFailed {
                reason: format!("migration task failed to run: {error}"),
            })
        })?
        .map_err(to_ipc)
}

/// Preview takes the database because a session that was itself restored here
/// keeps its original origin, and only the local receipt table knows that.
#[tauri::command]
pub async fn preview_session_migration(
    state: State<'_, AppState>,
    providerId: String,
    sourcePath: String,
) -> Result<MigratableSession, String> {
    let db = db_handle(&state);
    blocking(move || export::preview_session(&db, &providerId, &sourcePath)).await
}

#[tauri::command]
pub async fn export_session_package(
    state: State<'_, AppState>,
    items: Vec<ExportItem>,
    targetPath: String,
) -> Result<ExportOutcome, String> {
    let db = db_handle(&state);
    blocking(move || export::export_package(&db, &items, &targetPath)).await
}

#[tauri::command]
pub async fn read_session_package(
    state: State<'_, AppState>,
    path: String,
) -> Result<PackageReadResult, String> {
    let db = db_handle(&state);
    blocking(move || restore::read_for_restore(&db, &path)).await
}

#[tauri::command]
pub async fn probe_local_provider(providerId: String) -> Result<LocalProviderProbe, String> {
    blocking(move || capability::probe_local_provider(&providerId)).await
}

#[tauri::command]
pub async fn get_release_capability_matrix() -> Result<Vec<ReleaseCapability>, String> {
    blocking(|| Ok(capability::release_capability_matrix())).await
}

/// Restore selected snapshots into the target's own store.
///
/// One call can return rows in different stages: a snapshot already imported
/// under the default slot comes back as its existing receipt, while a fresh
/// one goes through a native write. The per-row stage is the answer, which is
/// why this returns receipts rather than a single success flag.
#[tauri::command]
pub async fn restore_session_package(
    state: State<'_, AppState>,
    request: RestoreRequest,
) -> Result<Vec<RestoreAttempt>, String> {
    let db = db_handle(&state);
    blocking(move || restore::restore_package(&db, &request)).await
}

#[tauri::command]
pub async fn verify_native_readback(
    state: State<'_, AppState>,
    attemptId: String,
) -> Result<RestoreAttempt, String> {
    let db = db_handle(&state);
    blocking(move || restore::verify_native_readback(&db, &attemptId)).await
}

#[tauri::command]
pub async fn list_restore_attempts(
    state: State<'_, AppState>,
) -> Result<Vec<RestoreAttempt>, String> {
    let db = db_handle(&state);
    blocking(move || receipt::ReceiptStore::new(&db).list()).await
}

#[tauri::command]
pub async fn reconcile_restore_attempts(
    state: State<'_, AppState>,
) -> Result<Vec<RestoreAttempt>, String> {
    let db = db_handle(&state);
    blocking(move || reconcile::reconcile_all(&db)).await
}

/// Store what the user says they saw.
///
/// Written to its own columns and never merged into `stage`: a self-report is
/// useful for tracing a problem and is not evidence that the migration worked.
#[tauri::command]
pub async fn record_user_attestation(
    state: State<'_, AppState>,
    attemptId: String,
    claimedStage: String,
    note: Option<String>,
) -> Result<RestoreAttempt, String> {
    let db = db_handle(&state);
    blocking(move || {
        let Some(claimed_stage) = RestoreStage::parse(&claimedStage) else {
            return Err(MigrationError::IdentityFieldInvalid {
                field: "claimedStage".to_string(),
                reason: format!("unknown stage `{claimedStage}`"),
            });
        };
        let attestation = UserAttestation {
            attested_at: receipt::now_ms(),
            claimed_stage,
            note,
        };
        receipt::ReceiptStore::new(&db).attest(&attemptId, &attestation)
    })
    .await
}

#[tauri::command]
pub async fn open_restored_session(
    state: State<'_, AppState>,
    attemptId: String,
) -> Result<bool, String> {
    let db = db_handle(&state);
    blocking(move || restore::open_restored_session(&db, &attemptId)).await
}

// The target directory comes from `commands::config::pick_directory`, which
// already wraps the platform dialog. The restore path then re-checks that the
// directory exists: a path typed or carried in a package is never a write
// target.

fn db_handle(state: &State<'_, AppState>) -> Arc<Database> {
    state.db.clone()
}
