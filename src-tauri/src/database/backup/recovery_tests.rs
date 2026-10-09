use crate::database::backup::recovery_outcome::{
    RestorePhase, RestorePublication, RestoreResultCode, RestoreTracker,
};
use crate::database::backup::{
    BackupLifecycleTestEvent, BACKUP_LIFECYCLE_TEST_PROBE, RECOVERY_TEST_CHECKPOINT,
};
use crate::database::backup::{RecoveryTestFault, RECOVERY_TEST_FAULT};
use crate::database::{Database, SCHEMA_VERSION};
use crate::error::AppError;
use rusqlite::{backup::Backup, Connection, OpenFlags};
use serial_test::serial;
use std::path::{Path, PathBuf};

/// Every production resolver must stay inside this fixture, including scratch.
pub(super) struct IsolatedDatabaseHome {
    root: tempfile::TempDir,
    previous: Vec<(&'static str, Option<std::ffi::OsString>)>,
}

impl IsolatedDatabaseHome {
    pub(super) fn new() -> Self {
        let root = tempfile::tempdir().expect("exclusive database recovery root");
        let scratch = root.path().join("scratch");
        std::fs::create_dir(&scratch).unwrap();
        let mut previous = Vec::new();
        for (name, value) in [
            ("FYAGENT_TEST_HOME", root.path()),
            ("TMPDIR", scratch.as_path()),
            ("TEMP", scratch.as_path()),
            ("TMP", scratch.as_path()),
        ] {
            previous.push((name, std::env::var_os(name)));
            std::env::set_var(name, value);
        }
        let fixture = Self { root, previous };
        assert!(
            crate::app_store::get_app_config_dir_override().is_none(),
            "app-store override must be absent"
        );
        for path in [
            crate::config::get_home_dir(),
            crate::config::get_app_config_dir(),
            crate::config::get_user_temp_dir(),
        ] {
            assert!(
                path.starts_with(fixture.root.path()),
                "resolver escaped fixture: {}",
                path.display()
            );
        }
        std::fs::create_dir_all(crate::config::get_app_config_dir()).unwrap();
        fixture
    }

    pub(super) fn db_path(&self) -> PathBuf {
        let path = crate::config::get_app_config_dir().join("fyagent.db");
        assert!(path.starts_with(self.root.path()));
        path
    }

    pub(super) fn backup_dir(&self) -> PathBuf {
        let path = crate::config::get_app_config_dir().join("backups");
        assert!(path.starts_with(self.root.path()));
        path
    }
}

impl Drop for IsolatedDatabaseHome {
    fn drop(&mut self) {
        RECOVERY_TEST_FAULT.with(|fault| fault.set(None));
        for (name, previous) in self.previous.drain(..).rev() {
            match previous {
                Some(value) => std::env::set_var(name, value),
                None => std::env::remove_var(name),
            }
        }
    }
}

struct FaultGuard;
impl FaultGuard {
    fn at(point: RecoveryTestFault) -> Self {
        RECOVERY_TEST_FAULT.with(|fault| fault.set(Some(point)));
        Self
    }
}
impl Drop for FaultGuard {
    fn drop(&mut self) {
        RECOVERY_TEST_FAULT.with(|fault| fault.set(None));
    }
}

pub(super) fn write_connection(conn: &Connection, path: &Path) -> Result<(), AppError> {
    let mut destination = Connection::open(path)?;
    let copy = Backup::new(conn, &mut destination)?;
    copy.step(-1)?;
    Ok(())
}

fn selected_backup(home: &IsolatedDatabaseHome) -> Result<PathBuf, AppError> {
    std::fs::create_dir_all(home.backup_dir()).map_err(|e| AppError::io(home.backup_dir(), e))?;
    let selected = home.backup_dir().join("selected.db");
    let source = Database::memory()?;
    let conn = crate::database::lock_conn!(source.conn);
    Database::set_user_version(&conn, SCHEMA_VERSION)?;
    conn.execute(
        "INSERT INTO settings(key,value) VALUES('recovery_fixture','target')",
        [],
    )?;
    write_connection(&conn, &selected)?;
    Ok(selected)
}

fn live_database() -> Result<Database, AppError> {
    let db = Database::init()?;
    {
        let conn = crate::database::lock_conn!(db.conn);
        conn.execute(
            "INSERT INTO settings(key,value) VALUES('recovery_fixture','live')",
            [],
        )?;
        conn.execute_batch("INSERT INTO session_restore_attempts (
            attempt_id,request_id,installation_id,request_fingerprint,snapshot_id,origin_id,request_kind,
            content_digest,origin_provider_id,target_provider_id,target_store_id,target_native_nonce,
            stage,device_binding,created_at,updated_at)
            VALUES('live-receipt','request','installation','fingerprint','snapshot','origin','defaultImport',
            'digest','claude','codex','store','nonce','targetReady','device',1,2)")?;
    }
    Ok(db)
}

fn live_value(db: &Database) -> String {
    let conn = db.conn.lock().unwrap();
    conn.query_row(
        "SELECT value FROM settings WHERE key='recovery_fixture'",
        [],
        |r| r.get(0),
    )
    .unwrap()
}

fn restore(db: &Database) -> crate::database::backup::recovery_outcome::DatabaseRestoreOutcome {
    db.restore_from_backup_outcome("selected.db", &RestoreTracker::new())
}

fn assert_prepublication_failure(db: &Database, phase: RestorePhase, result: RestoreResultCode) {
    let outcome = restore(db);
    assert_eq!(outcome.phase, phase);
    assert_eq!(outcome.result_code, result);
    assert_eq!(outcome.publication, RestorePublication::NotCommitted);
    assert!(outcome.safety_backup_filename.is_none());
    assert_eq!(live_value(db), "live");
}

#[test]
#[serial]
fn candidate_pricing_failure_preserves_live_and_selected() -> Result<(), AppError> {
    let home = IsolatedDatabaseHome::new();
    let db = live_database()?;
    let selected = selected_backup(&home)?;
    let source = Connection::open(&selected)?;
    source.execute_batch(
        "DROP TABLE model_pricing; CREATE TABLE model_pricing(model_id TEXT PRIMARY KEY)",
    )?;
    drop(source);
    let before = std::fs::read(&selected).unwrap();
    assert_prepublication_failure(
        &db,
        RestorePhase::Candidate,
        RestoreResultCode::CandidateFailed,
    );
    assert_eq!(std::fs::read(&selected).unwrap(), before);
    assert_eq!(Database::list_backups()?.len(), 1);
    Ok(())
}

#[test]
#[serial]
fn candidate_future_version_failure_does_not_publish() -> Result<(), AppError> {
    let home = IsolatedDatabaseHome::new();
    let db = live_database()?;
    let selected = selected_backup(&home)?;
    let source = Connection::open(&selected)?;
    Database::set_user_version(&source, SCHEMA_VERSION + 1)?;
    drop(source);
    let before = std::fs::read(&selected).unwrap();
    assert_prepublication_failure(
        &db,
        RestorePhase::Candidate,
        RestoreResultCode::CandidateFailed,
    );
    assert_eq!(std::fs::read(&selected).unwrap(), before);
    Ok(())
}

#[test]
#[serial]
fn archive_failure_aborts_before_safety_or_publish() -> Result<(), AppError> {
    let home = IsolatedDatabaseHome::new();
    let db = live_database()?;
    selected_backup(&home)?;
    db.conn.lock().unwrap().execute_batch("CREATE TABLE fde_customers(customer_id TEXT); INSERT INTO fde_customers VALUES('historical')")?;
    let archive_dir = crate::config::get_app_config_dir()
        .join(crate::database::retired_customer_projects::RETIRED_ARCHIVE_DIRNAME);
    std::fs::write(&archive_dir, b"archive-blocker").unwrap();
    assert_prepublication_failure(&db, RestorePhase::Archive, RestoreResultCode::ArchiveFailed);
    assert_eq!(Database::list_backups()?.len(), 1);
    std::fs::remove_file(archive_dir).unwrap();
    assert_eq!(restore(&db).result_code, RestoreResultCode::Restored);
    assert_eq!(live_value(&db), "target");
    let archive_dir = crate::config::get_app_config_dir()
        .join(crate::database::retired_customer_projects::RETIRED_ARCHIVE_DIRNAME);
    let archive_path = std::fs::read_dir(&archive_dir)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let archive_bytes = std::fs::read(&archive_path).unwrap();
    let archive = Connection::open_with_flags(&archive_path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    assert_eq!(
        archive
            .query_row::<String, _, _>("SELECT customer_id FROM fde_customers", [], |row| row
                .get(0))?,
        "historical"
    );
    assert_eq!(std::fs::read(&archive_path).unwrap(), archive_bytes);
    Ok(())
}

#[test]
#[serial]
fn safety_create_and_verify_failures_leave_no_published_snapshot() -> Result<(), AppError> {
    let home = IsolatedDatabaseHome::new();
    let db = live_database()?;
    selected_backup(&home)?;
    for point in [
        RecoveryTestFault::SafetyCreation,
        RecoveryTestFault::SafetyVerification,
    ] {
        let _fault = FaultGuard::at(point);
        assert_prepublication_failure(
            &db,
            RestorePhase::SafetyBackup,
            RestoreResultCode::SafetyBackupFailed,
        );
        assert_eq!(Database::list_backups()?.len(), 1);
    }
    assert_eq!(restore(&db).result_code, RestoreResultCode::Restored);
    assert_eq!(live_value(&db), "target");
    Ok(())
}

#[test]
#[serial]
fn missing_disk_recovery_point_blocks_public_restore() -> Result<(), AppError> {
    let home = IsolatedDatabaseHome::new();
    let db = Database::memory()?;
    db.conn.lock().unwrap().execute(
        "INSERT INTO settings(key,value) VALUES('recovery_fixture','live')",
        [],
    )?;
    selected_backup(&home)?;
    assert!(!home.db_path().exists());
    assert_prepublication_failure(
        &db,
        RestorePhase::SafetyBackup,
        RestoreResultCode::SafetyBackupFailed,
    );
    Ok(())
}

#[test]
#[serial]
fn sqlite_publish_error_is_unknown_and_keeps_recovery_leaf() -> Result<(), AppError> {
    let home = IsolatedDatabaseHome::new();
    let db = live_database()?;
    selected_backup(&home)?;
    *db.conn.lock().unwrap() =
        Connection::open_with_flags(home.db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    assert!(db
        .conn
        .lock()
        .unwrap()
        .is_readonly(rusqlite::DatabaseName::Main)?);

    let outcome = restore(&db);
    assert_eq!(outcome.phase, RestorePhase::Publish);
    assert_eq!(outcome.result_code, RestoreResultCode::PublishFailed);
    assert_eq!(outcome.publication, RestorePublication::Unknown);
    assert_eq!(live_value(&db), "live");
    assert!(home
        .backup_dir()
        .join(outcome.safety_backup_filename.unwrap())
        .is_file());
    Ok(())
}

#[test]
#[serial]
fn sqlite_busy_publish_never_reports_committed() -> Result<(), AppError> {
    let home = IsolatedDatabaseHome::new();
    let db = live_database()?;
    selected_backup(&home)?;
    db.conn
        .lock()
        .unwrap()
        .busy_timeout(std::time::Duration::ZERO)?;
    let reader = Connection::open_with_flags(home.db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    reader.execute_batch("BEGIN")?;
    assert_eq!(
        reader.query_row::<String, _, _>(
            "SELECT value FROM settings WHERE key='recovery_fixture'",
            [],
            |row| row.get(0)
        )?,
        "live"
    );
    let outcome = restore(&db);
    assert_eq!(outcome.result_code, RestoreResultCode::PublishFailed);
    assert_eq!(outcome.phase, RestorePhase::Publish);
    assert_eq!(outcome.publication, RestorePublication::Unknown);
    assert_eq!(live_value(&db), "live");
    assert!(home
        .backup_dir()
        .join(outcome.safety_backup_filename.unwrap())
        .is_file());
    reader.execute_batch("ROLLBACK")?;
    assert_eq!(restore(&db).result_code, RestoreResultCode::Restored);
    assert_eq!(live_value(&db), "target");
    Ok(())
}

#[test]
#[serial]
fn late_readback_failure_retains_committed_truth_and_latest_receipt() -> Result<(), AppError> {
    let home = IsolatedDatabaseHome::new();
    let db = live_database()?;
    let selected = selected_backup(&home)?;
    let before = std::fs::read(&selected).unwrap();
    let outcome = {
        let _fault = FaultGuard::at(RecoveryTestFault::Readback);
        restore(&db)
    };
    assert_eq!(outcome.phase, RestorePhase::Readback);
    assert_eq!(outcome.result_code, RestoreResultCode::ReadbackFailed);
    assert_eq!(outcome.publication, RestorePublication::Committed);
    assert_eq!(live_value(&db), "target");
    assert!(db.check_recovery_readability());
    assert_eq!(std::fs::read(&selected).unwrap(), before);
    let backup = Connection::open_with_flags(
        home.backup_dir()
            .join(outcome.safety_backup_filename.unwrap()),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )?;
    assert_eq!(
        backup.query_row::<String, _, _>(
            "SELECT value FROM settings WHERE key='recovery_fixture'",
            [],
            |r| r.get(0)
        )?,
        "live"
    );
    let conn = db.conn.lock().unwrap();
    assert_eq!(
        conn.query_row::<String, _, _>(
            "SELECT attempt_id FROM session_restore_attempts",
            [],
            |r| r.get(0)
        )?,
        "live-receipt"
    );
    assert!(
        conn.query_row::<i64, _, _>("SELECT COUNT(*) FROM model_pricing", [], |r| r.get(0))? > 0
    );
    drop(conn);
    drop(db);
    let reopened = Database::init()?;
    assert_eq!(live_value(&reopened), "target");
    Ok(())
}

#[test]
#[serial]
fn retention_warning_keeps_selected_and_safety_names() -> Result<(), AppError> {
    let home = IsolatedDatabaseHome::new();
    let db = live_database()?;
    let selected = selected_backup(&home)?;
    let before = std::fs::read(&selected).unwrap();
    let outcome = {
        let _fault = FaultGuard::at(RecoveryTestFault::Retention);
        restore(&db)
    };
    assert_eq!(outcome.result_code, RestoreResultCode::Restored);
    assert_eq!(outcome.publication, RestorePublication::Committed);
    assert_eq!(outcome.warnings, ["retentionFailed"]);
    let safety = outcome.safety_backup_filename.unwrap();
    assert!(safety.ends_with(".db"));
    assert!(home.backup_dir().join(safety).is_file());
    assert_eq!(std::fs::read(selected).unwrap(), before);
    assert_eq!(live_value(&db), "target");
    Ok(())
}

#[test]
#[serial]
fn valid_backup_returns_its_leaf_when_retention_fails() -> Result<(), AppError> {
    let home = IsolatedDatabaseHome::new();
    let db = live_database()?;
    let _fault = FaultGuard::at(RecoveryTestFault::Retention);
    let backup = db
        .backup_database_file()?
        .expect("validated recovery point");
    assert_eq!(backup.parent(), Some(home.backup_dir().as_path()));
    let saved = Connection::open_with_flags(backup, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    assert_eq!(Database::get_user_version(&saved)?, SCHEMA_VERSION);
    assert_eq!(
        saved.query_row::<String, _, _>("PRAGMA integrity_check", [], |r| r.get(0))?,
        "ok"
    );
    Ok(())
}

#[test]
#[serial]
fn normal_retention_never_removes_selected_or_new_safety_backup() -> Result<(), AppError> {
    let home = IsolatedDatabaseHome::new();
    let db = live_database()?;
    let selected = selected_backup(&home)?;
    let before = std::fs::read(&selected).unwrap();
    for index in 0..crate::settings::effective_backup_retain_count() + 2 {
        std::fs::write(home.backup_dir().join(format!("older-{index}.db")), &before).unwrap();
    }
    let outcome = restore(&db);
    assert_eq!(outcome.result_code, RestoreResultCode::Restored);
    assert_eq!(std::fs::read(selected).unwrap(), before);
    assert!(home
        .backup_dir()
        .join(outcome.safety_backup_filename.unwrap())
        .is_file());
    assert!(
        Database::list_backups()?.len() <= crate::settings::effective_backup_retain_count().max(2)
    );
    Ok(())
}

#[test]
#[serial]
fn retention_protects_native_canonical_identity() -> Result<(), AppError> {
    let home = IsolatedDatabaseHome::new();
    let selected = selected_backup(&home)?;
    let before = std::fs::read(&selected).unwrap();
    std::fs::OpenOptions::new()
        .write(true)
        .open(&selected)
        .unwrap()
        .set_modified(std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(3600))
        .unwrap();
    for index in 0..crate::settings::effective_backup_retain_count() + 2 {
        std::fs::write(home.backup_dir().join(format!("newer-{index}.db")), &before).unwrap();
    }
    let alias_directory = home.backup_dir().join("alias");
    std::fs::create_dir(&alias_directory).unwrap();
    let alias = alias_directory.join("..").join("selected.db");
    Database::cleanup_db_backups_preserving(&home.backup_dir(), &[&alias])?;
    assert_eq!(std::fs::read(&selected).unwrap(), before);
    assert_eq!(
        Database::list_backups()?.len(),
        crate::settings::effective_backup_retain_count()
    );
    Ok(())
}

#[test]
#[serial]
fn unresolved_protected_identity_aborts_retention_before_deletion() -> Result<(), AppError> {
    let home = IsolatedDatabaseHome::new();
    let selected = selected_backup(&home)?;
    let before = std::fs::read(&selected).unwrap();
    for index in 0..crate::settings::effective_backup_retain_count() + 2 {
        std::fs::write(home.backup_dir().join(format!("newer-{index}.db")), &before).unwrap();
    }
    let listed_before = Database::list_backups()?;
    let missing = home.backup_dir().join("missing-protected.db");
    assert!(
        Database::cleanup_db_backups_preserving(&home.backup_dir(), &[&selected, &missing])
            .is_err()
    );
    assert_eq!(Database::list_backups()?.len(), listed_before.len());
    for entry in listed_before {
        assert_eq!(
            std::fs::read(home.backup_dir().join(entry.filename)).unwrap(),
            before
        );
    }
    Ok(())
}

#[cfg(windows)]
#[test]
#[serial]
fn windows_case_alias_restore_keeps_oldest_selected_backup() -> Result<(), AppError> {
    let home = IsolatedDatabaseHome::new();
    let db = live_database()?;
    let selected = selected_backup(&home)?;
    let selected_native = home.backup_dir().join("SELECTED.db");
    std::fs::rename(&selected, &selected_native).unwrap();
    let before = std::fs::read(&selected_native).unwrap();
    std::fs::OpenOptions::new()
        .write(true)
        .open(&selected_native)
        .unwrap()
        .set_modified(std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(3600))
        .unwrap();
    for index in 0..crate::settings::effective_backup_retain_count() + 2 {
        std::fs::write(home.backup_dir().join(format!("newer-{index}.db")), &before).unwrap();
    }
    assert_eq!(
        std::fs::canonicalize(&selected).unwrap(),
        std::fs::canonicalize(&selected_native).unwrap()
    );
    let outcome = restore(&db);
    assert_eq!(outcome.result_code, RestoreResultCode::Restored);
    assert!(outcome.warnings.is_empty());
    assert_eq!(std::fs::read(&selected_native).unwrap(), before);
    assert!(home
        .backup_dir()
        .join(outcome.safety_backup_filename.unwrap())
        .is_file());
    assert!(
        Database::list_backups()?.len() <= crate::settings::effective_backup_retain_count().max(2)
    );
    Ok(())
}

#[derive(Clone, Copy)]
enum ConcurrentDirectoryOperation {
    Backup,
    Retention,
}

fn exercise_restore_directory_lifecycle(
    operation: ConcurrentDirectoryOperation,
) -> Result<(), AppError> {
    let home = IsolatedDatabaseHome::new();
    let db = std::sync::Arc::new(live_database()?);
    let selected = selected_backup(&home)?;
    let selected_before = std::fs::read(&selected).unwrap();
    std::fs::OpenOptions::new()
        .write(true)
        .open(&selected)
        .unwrap()
        .set_modified(std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(3600))
        .unwrap();
    for index in 0..crate::settings::effective_backup_retain_count() + 2 {
        std::fs::write(
            home.backup_dir().join(format!("newer-{index}.db")),
            &selected_before,
        )
        .unwrap();
    }
    let timeout = std::time::Duration::from_secs(30);
    let tracker = RestoreTracker::new();
    let (checkpoint_tx, checkpoint_rx) = std::sync::mpsc::channel();
    let (resume_tx, resume_rx) = std::sync::mpsc::channel();
    let (probe_tx, probe_rx) = std::sync::mpsc::channel();

    std::thread::scope(|scope| {
        let restore_db = db.clone();
        let restore_tracker = tracker.clone();
        let restoring = scope.spawn(move || {
            RECOVERY_TEST_CHECKPOINT.with(|checkpoint| {
                *checkpoint.borrow_mut() = Some(Box::new(move |phase| {
                    checkpoint_tx.send(phase).unwrap();
                    resume_rx
                        .recv_timeout(timeout)
                        .expect("release restore checkpoint");
                }));
            });
            let outcome = restore_db.restore_from_backup_outcome("selected.db", &restore_tracker);
            RECOVERY_TEST_CHECKPOINT.with(|checkpoint| *checkpoint.borrow_mut() = None);
            outcome
        });

        assert_eq!(
            checkpoint_rx.recv_timeout(timeout).unwrap(),
            RestorePhase::Candidate
        );
        assert_eq!(
            tracker.snapshot().publication,
            RestorePublication::NotCommitted
        );
        assert!(tracker.snapshot().safety_backup_filename.is_none());
        assert_eq!(std::fs::read(&selected).unwrap(), selected_before);
        let competitor_db = db.clone();
        let backup_dir = home.backup_dir();
        let competing = scope.spawn(move || {
            BACKUP_LIFECYCLE_TEST_PROBE.with(|probe| *probe.borrow_mut() = Some(probe_tx));
            let result = match operation {
                ConcurrentDirectoryOperation::Backup => competitor_db.backup_database_file(),
                ConcurrentDirectoryOperation::Retention => {
                    Database::cleanup_db_backups_preserving(&backup_dir, &[]).map(|()| None)
                }
            };
            BACKUP_LIFECYCLE_TEST_PROBE.with(|probe| *probe.borrow_mut() = None);
            result
        });

        // This signal comes from a failed try_lock on the actual lifecycle
        // mutex, not from an elapsed-time or simulated worker assertion.
        assert_eq!(
            probe_rx.recv_timeout(timeout).unwrap(),
            BackupLifecycleTestEvent::Blocked
        );
        assert!(matches!(
            probe_rx.try_recv(),
            Err(std::sync::mpsc::TryRecvError::Empty)
        ));
        assert_eq!(std::fs::read(&selected).unwrap(), selected_before);
        resume_tx.send(()).unwrap();

        assert_eq!(
            checkpoint_rx.recv_timeout(timeout).unwrap(),
            RestorePhase::Publish
        );
        let publishing = tracker.snapshot();
        assert_eq!(publishing.publication, RestorePublication::Unknown);
        let safety_filename = publishing
            .safety_backup_filename
            .expect("validated safety point before publish");
        let safety_path = home.backup_dir().join(&safety_filename);
        assert_eq!(std::fs::read(&selected).unwrap(), selected_before);
        let safety = Connection::open_with_flags(&safety_path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        assert_eq!(
            safety.query_row::<String, _, _>(
                "SELECT value FROM settings WHERE key='recovery_fixture'",
                [],
                |row| row.get(0)
            )?,
            "live"
        );
        drop(safety);
        assert!(matches!(
            probe_rx.try_recv(),
            Err(std::sync::mpsc::TryRecvError::Empty)
        ));
        resume_tx.send(()).unwrap();

        let restored = restoring.join().unwrap();
        assert_eq!(restored.result_code, RestoreResultCode::Restored);
        assert_eq!(restored.publication, RestorePublication::Committed);
        assert_eq!(
            restored.safety_backup_filename.as_deref(),
            Some(safety_filename.as_str())
        );
        assert_eq!(
            probe_rx.recv_timeout(timeout).unwrap(),
            BackupLifecycleTestEvent::Acquired
        );
        let competing_result = competing.join().unwrap()?;
        assert_eq!(live_value(&db), "target");
        if let Some(path) = competing_result {
            let backup = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
            assert_eq!(
                backup.query_row::<String, _, _>(
                    "SELECT value FROM settings WHERE key='recovery_fixture'",
                    [],
                    |row| row.get(0)
                )?,
                "target"
            );
        }
        // Once restore releases its lease, ordinary retention may remove the
        // old selected point. That later policy is not stage protection.
        assert!(
            Database::list_backups()?.len() <= crate::settings::effective_backup_retain_count()
        );
        Ok(())
    })
}

#[test]
#[serial]
fn concurrent_backup_waits_through_restore_candidate_and_publish() -> Result<(), AppError> {
    exercise_restore_directory_lifecycle(ConcurrentDirectoryOperation::Backup)
}

#[test]
#[serial]
fn concurrent_retention_waits_through_restore_candidate_and_publish() -> Result<(), AppError> {
    exercise_restore_directory_lifecycle(ConcurrentDirectoryOperation::Retention)
}

#[test]
#[serial]
fn invalid_leaf_and_trigger_are_precheck_failures() -> Result<(), AppError> {
    let home = IsolatedDatabaseHome::new();
    let db = live_database()?;
    let selected = selected_backup(&home)?;
    let traversal = db.restore_from_backup_outcome("../selected.db", &RestoreTracker::new());
    assert_eq!(traversal.result_code, RestoreResultCode::PrecheckFailed);
    assert_eq!(traversal.publication, RestorePublication::NotCommitted);
    let source = Connection::open(&selected)?;
    source
        .execute_batch("CREATE TRIGGER unexpected AFTER INSERT ON settings BEGIN SELECT 1; END")?;
    drop(source);
    let before = std::fs::read(&selected).unwrap();
    assert_prepublication_failure(
        &db,
        RestorePhase::Precheck,
        RestoreResultCode::PrecheckFailed,
    );
    assert_eq!(std::fs::read(selected).unwrap(), before);
    Ok(())
}

#[test]
#[serial]
fn list_and_restore_reject_directories_and_non_owned_links() -> Result<(), AppError> {
    let home = IsolatedDatabaseHome::new();
    let db = live_database()?;
    let selected = selected_backup(&home)?;
    std::fs::create_dir(home.backup_dir().join("directory.db")).unwrap();
    assert_eq!(
        Database::list_backups()?
            .iter()
            .map(|entry| entry.filename.as_str())
            .collect::<Vec<_>>(),
        ["selected.db"]
    );
    let directory = db.restore_from_backup_outcome("directory.db", &RestoreTracker::new());
    assert_eq!(directory.result_code, RestoreResultCode::PrecheckFailed);
    assert_eq!(directory.publication, RestorePublication::NotCommitted);
    for filename in [
        "stream:backup.db",
        "line\nbreak.db",
        "NUL.db",
        ".db",
        " x.db",
        "x.db ",
    ] {
        let rejected = db.restore_from_backup_outcome(filename, &RestoreTracker::new());
        assert_eq!(rejected.result_code, RestoreResultCode::PrecheckFailed);
    }
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    {
        let link = home.backup_dir().join("linked.db");
        #[cfg(target_os = "windows")]
        let linked = std::os::windows::fs::symlink_file(&selected, &link);
        #[cfg(target_os = "macos")]
        let linked = std::os::unix::fs::symlink(&selected, &link);
        if let Err(error) = linked {
            eprintln!("symlink_subcase_unsupported: {error}");
        } else {
            assert!(!Database::list_backups()?
                .iter()
                .any(|entry| entry.filename == "linked.db"));
            let rejected = db.restore_from_backup_outcome("linked.db", &RestoreTracker::new());
            assert_eq!(rejected.result_code, RestoreResultCode::PrecheckFailed);
            assert_eq!(rejected.publication, RestorePublication::NotCommitted);
        }
    }
    assert_eq!(live_value(&db), "live");
    Ok(())
}
