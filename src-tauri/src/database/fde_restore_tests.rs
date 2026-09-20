use super::{AppError, Connection, Database, TestHomeGuard};
use serial_test::serial;

#[test]
#[serial]
fn binary_restore_generation_exhaustion_preserves_live_database() -> Result<(), AppError> {
    let home = TestHomeGuard::new();
    let backups = crate::config::get_app_config_dir().join("backups");
    std::fs::create_dir_all(&backups).expect("create isolated backups");
    let candidate = backups.join("exhausted-generation.db");
    assert!(candidate.starts_with(home.path()));
    {
        let source = Connection::open(&candidate)?;
        Database::create_tables_on_conn(&source)?;
        Database::apply_schema_migrations_on_conn(&source)?;
        source.execute_batch(
            "INSERT INTO providers(id,app_type,name,settings_config,meta)
             VALUES('candidate','codex','Candidate','{}','{}');
             UPDATE fde_resource_generations SET generation=9007199254740990
             WHERE kind='provider' AND app_type='codex' AND resource_id='candidate';",
        )?;
        Database::reject_persistent_triggers(&source)?;
        assert_eq!(
            source.query_row(
                "SELECT generation FROM fde_resource_generations
                 WHERE kind='provider' AND app_type='codex' AND resource_id='candidate'",
                [],
                |row| row.get::<_, i64>(0),
            )?,
            9_007_199_254_740_990,
        );
    }
    let candidate_bytes = std::fs::read(&candidate).expect("read candidate before restore");
    let target = Database::memory()?;
    {
        let conn = crate::database::lock_conn!(target.conn);
        conn.execute_batch(
            "INSERT INTO providers(id,app_type,name,settings_config,meta)
             VALUES('sentinel','codex','Live provider','{}','{}');",
        )?;
    }
    let error = target
        .restore_from_backup("exhausted-generation.db")
        .expect_err("generation exhaustion must reject the restore");
    assert!(
        error.to_string().contains("projects_storage_unavailable"),
        "the candidate must reach generation validation: {error}",
    );
    let conn = crate::database::lock_conn!(target.conn);
    let providers = conn
        .prepare("SELECT id FROM providers ORDER BY id")?
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    assert_eq!(
        providers,
        vec!["sentinel"],
        "failed restore must preserve live rows"
    );
    assert_eq!(
        conn.query_row(
            "SELECT generation FROM fde_resource_generations
             WHERE kind='provider' AND app_type='codex' AND resource_id='sentinel'",
            [],
            |row| row.get::<_, i64>(0),
        )?,
        1,
        "failed restore must preserve live resource generations",
    );
    assert_eq!(
        std::fs::read(&candidate).expect("read candidate after failed restore"),
        candidate_bytes,
        "preparing a restore must not mutate the backup file",
    );
    Ok(())
}
