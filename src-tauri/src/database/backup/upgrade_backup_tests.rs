use super::recovery_outcome_tests::{write_connection, IsolatedDatabaseHome};
use crate::database::{Database, SCHEMA_VERSION};
use crate::error::AppError;
use rusqlite::{Connection, OpenFlags};
use serial_test::serial;

fn old_database(home: &IsolatedDatabaseHome) -> Result<Vec<u8>, AppError> {
    let old = Database::memory()?;
    let conn = crate::database::lock_conn!(old.conn);
    // Genuine immediately preceding shape: no restore-receipt table yet.
    conn.execute_batch(
        "DROP TABLE session_restore_attempts;
        INSERT INTO settings(key,value) VALUES('upgrade_fixture','old-data')",
    )?;
    Database::set_user_version(&conn, SCHEMA_VERSION - 1)?;
    write_connection(&conn, &home.db_path())?;
    Ok(std::fs::read(home.db_path()).unwrap())
}

#[test]
#[serial]
fn old_schema_snapshot_precedes_first_schema_write() -> Result<(), AppError> {
    let home = IsolatedDatabaseHome::new();
    old_database(&home)?;
    let upgraded = Database::init()?;
    let entries = Database::list_backups()?;
    assert!(!entries.is_empty());
    let backup = Connection::open_with_flags(
        home.backup_dir().join(&entries[0].filename),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )?;
    assert_eq!(Database::get_user_version(&backup)?, SCHEMA_VERSION - 1);
    assert!(!Database::table_exists(
        &backup,
        "session_restore_attempts"
    )?);
    assert_eq!(
        backup.query_row::<String, _, _>(
            "SELECT value FROM settings WHERE key='upgrade_fixture'",
            [],
            |r| r.get(0)
        )?,
        "old-data"
    );
    assert_eq!(
        backup.query_row::<String, _, _>("PRAGMA integrity_check", [], |r| r.get(0))?,
        "ok"
    );
    let live = upgraded.conn.lock().unwrap();
    assert_eq!(Database::get_user_version(&live)?, SCHEMA_VERSION);
    assert!(Database::table_exists(&live, "session_restore_attempts")?);
    Ok(())
}

#[test]
#[serial]
fn backup_failure_leaves_old_bytes_and_allows_same_init_retry() -> Result<(), AppError> {
    let home = IsolatedDatabaseHome::new();
    let before = old_database(&home)?;
    std::fs::write(home.backup_dir(), b"backups-directory-blocker").unwrap();
    assert!(Database::init().is_err());
    assert_eq!(std::fs::read(home.db_path()).unwrap(), before);
    let old = Connection::open_with_flags(home.db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    assert_eq!(Database::get_user_version(&old)?, SCHEMA_VERSION - 1);
    assert!(!Database::table_exists(&old, "session_restore_attempts")?);
    assert_eq!(
        old.query_row::<String, _, _>(
            "SELECT value FROM settings WHERE key='upgrade_fixture'",
            [],
            |r| r.get(0)
        )?,
        "old-data"
    );
    drop(old);
    std::fs::remove_file(home.backup_dir()).unwrap();
    let upgraded = Database::init()?;
    assert_eq!(
        Database::get_user_version(&upgraded.conn.lock().unwrap())?,
        SCHEMA_VERSION
    );
    drop(upgraded);
    let reopened = Database::init()?;
    assert_eq!(
        Database::get_user_version(&reopened.conn.lock().unwrap())?,
        SCHEMA_VERSION
    );
    Ok(())
}

#[test]
#[serial]
fn backup_verification_failure_aborts_old_schema_writes() -> Result<(), AppError> {
    let home = IsolatedDatabaseHome::new();
    let before = old_database(&home)?;
    crate::database::backup::RECOVERY_TEST_FAULT.with(|fault| {
        fault.set(Some(
            crate::database::backup::RecoveryTestFault::SafetyVerification,
        ))
    });
    assert!(Database::init().is_err());
    crate::database::backup::RECOVERY_TEST_FAULT.with(|fault| fault.set(None));
    assert_eq!(std::fs::read(home.db_path()).unwrap(), before);
    assert!(Database::list_backups()?.is_empty());
    let upgraded = Database::init()?;
    assert_eq!(
        Database::get_user_version(&upgraded.conn.lock().unwrap())?,
        SCHEMA_VERSION
    );
    Ok(())
}

#[test]
#[serial]
fn future_schema_fails_before_create_tables_or_backup() -> Result<(), AppError> {
    let home = IsolatedDatabaseHome::new();
    let future = Connection::open(home.db_path())?;
    future.execute_batch(
        "CREATE TABLE future_only(value TEXT); INSERT INTO future_only VALUES('future-data')",
    )?;
    Database::set_user_version(&future, SCHEMA_VERSION + 1)?;
    drop(future);
    let before = std::fs::read(home.db_path()).unwrap();
    assert!(Database::init().is_err());
    assert_eq!(std::fs::read(home.db_path()).unwrap(), before);
    let future = Connection::open_with_flags(home.db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    assert_eq!(Database::get_user_version(&future)?, SCHEMA_VERSION + 1);
    assert!(!Database::table_exists(&future, "providers")?);
    assert!(!home.backup_dir().exists());
    Ok(())
}

#[test]
#[serial]
fn new_database_has_no_fabricated_upgrade_backup() -> Result<(), AppError> {
    let home = IsolatedDatabaseHome::new();
    assert!(!home.db_path().exists());
    let fresh = Database::init()?;
    let conn = fresh.conn.lock().unwrap();
    assert_eq!(Database::get_user_version(&conn)?, SCHEMA_VERSION);
    assert!(Database::table_exists(&conn, "session_restore_attempts")?);
    assert!(
        conn.query_row::<i64, _, _>("SELECT COUNT(*) FROM model_pricing", [], |r| r.get(0))? > 0
    );
    assert!(Database::list_backups()?.is_empty());
    assert!(!home.backup_dir().exists());
    Ok(())
}

#[test]
#[serial]
fn zero_version_unfinished_copy_is_not_a_valid_safety_snapshot() -> Result<(), AppError> {
    let home = IsolatedDatabaseHome::new();
    let old = Connection::open(home.db_path())?;
    old.execute_batch(
        "CREATE TABLE old_payload(value TEXT); INSERT INTO old_payload VALUES('preimage')",
    )?;
    assert_eq!(Database::get_user_version(&old)?, 0);
    drop(old);
    let before = std::fs::read(home.db_path()).unwrap();
    let mut db = Database::memory()?;
    db.conn = std::sync::Mutex::new(Connection::open(home.db_path())?);
    assert!(!db.check_recovery_readability());
    db.conn
        .lock()
        .unwrap()
        .execute_batch("BEGIN IMMEDIATE; UPDATE old_payload SET value='uncommitted'")?;
    assert!(db.create_validated_binary_backup().is_err());
    assert!(Database::list_backups()?.is_empty());
    db.conn.lock().unwrap().execute_batch("ROLLBACK")?;
    assert_eq!(std::fs::read(home.db_path()).unwrap(), before);
    assert_eq!(Database::get_user_version(&db.conn.lock().unwrap())?, 0);
    assert!(!Database::table_exists(
        &db.conn.lock().unwrap(),
        "providers"
    )?);
    Ok(())
}
