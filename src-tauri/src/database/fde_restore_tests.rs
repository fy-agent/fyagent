use super::{AppError, Connection, Database, TestHomeGuard};
use serial_test::serial;

const RETIRED_MODULE_TABLES: [&str; 8] = [
    "verification_revocations",
    "verification_evidence",
    "verification_handoff",
    "fde_project_context_versions",
    "fde_project_kit_intents",
    "fde_projects",
    "fde_customers",
    "fde_resource_generations",
];

fn retired_table_count(conn: &Connection) -> Result<i64, AppError> {
    let mut count = 0;
    for table in RETIRED_MODULE_TABLES {
        if Database::table_exists(conn, table)? {
            count += 1;
        }
    }
    Ok(count)
}

fn retired_trigger_count(conn: &Connection) -> Result<i64, rusqlite::Error> {
    conn.query_row(
        "SELECT count(*) FROM sqlite_schema WHERE type='trigger' AND name LIKE 'fde_resource_%'",
        [],
        |row| row.get(0),
    )
}

fn create_historical_fde_tables(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS fde_customers (
            customer_id TEXT PRIMARY KEY, name TEXT NOT NULL, revision INTEGER NOT NULL, archived INTEGER NOT NULL);
         CREATE TABLE IF NOT EXISTS fde_projects (
            project_id TEXT PRIMARY KEY, customer_id TEXT NOT NULL, project_revision INTEGER NOT NULL,
            archived INTEGER NOT NULL, document TEXT NOT NULL);
         CREATE TABLE IF NOT EXISTS fde_project_kit_intents (
            intent_id TEXT PRIMARY KEY, project_id TEXT NOT NULL, request TEXT NOT NULL, result TEXT NOT NULL);
         CREATE TABLE IF NOT EXISTS fde_project_context_versions (
            project_id TEXT NOT NULL, generation TEXT NOT NULL, digest TEXT NOT NULL,
            PRIMARY KEY(project_id, generation));
         CREATE TABLE IF NOT EXISTS fde_resource_generations (
            kind TEXT NOT NULL, app_type TEXT NOT NULL, resource_id TEXT NOT NULL, generation INTEGER NOT NULL,
            PRIMARY KEY(kind, app_type, resource_id));
         CREATE TABLE IF NOT EXISTS verification_evidence (
            id TEXT PRIMARY KEY, project_id TEXT NOT NULL, recorded_at TEXT NOT NULL, payload TEXT NOT NULL);
         CREATE TABLE IF NOT EXISTS verification_revocations (
            evidence_id TEXT PRIMARY KEY, revoked_at TEXT NOT NULL);
         CREATE TABLE IF NOT EXISTS verification_handoff (
            project_id TEXT PRIMARY KEY, revision INTEGER NOT NULL, payload TEXT NOT NULL);",
    )
    .map_err(|e| AppError::Database(e.to_string()))
}

fn install_genuine_retired_triggers(conn: &Connection) -> Result<(), AppError> {
    for sql in crate::database::retired_customer_projects::retired_fde_resource_trigger_sql() {
        conn.execute_batch(&sql)
            .map_err(|e| AppError::Database(e.to_string()))?;
    }
    Ok(())
}

fn retired_archive_dir() -> std::path::PathBuf {
    crate::config::get_app_config_dir()
        .join(crate::database::retired_customer_projects::RETIRED_ARCHIVE_DIRNAME)
}

fn list_retired_archives() -> Vec<std::path::PathBuf> {
    let dir = retired_archive_dir();
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut files = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "db"))
        .collect::<Vec<_>>();
    files.sort();
    files
}

fn seed_live_historical_customer_project(db: &Database) -> Result<(), AppError> {
    let conn = crate::database::lock_conn!(db.conn);
    create_historical_fde_tables(&conn)?;
    conn.execute_batch(
        "INSERT INTO providers(id,app_type,name,settings_config,meta)
         VALUES('live-provider','claude','Live Provider','{}','{}');
         INSERT INTO fde_customers VALUES('customer','Live customer',3,0);
         INSERT INTO fde_projects VALUES('project','customer',7,0,'{\"fixture\":\"live-project\"}');
         INSERT INTO fde_resource_generations VALUES('provider','claude','live-provider',41);",
    )?;
    Ok(())
}

#[derive(Clone, Copy, Debug)]
enum IntegrationPredecessor {
    Legacy21,
    Fde22,
    Subscription22,
}

fn integration_predecessor(
    conn: &Connection,
    predecessor: IntegrationPredecessor,
) -> Result<(), AppError> {
    Database::create_tables_on_conn(conn)?;
    // These predecessor releases did not yet have the Session receipt table.
    conn.execute_batch("DROP TABLE session_restore_attempts")?;
    let triggers = conn
        .prepare(
            "SELECT name FROM sqlite_schema WHERE type='trigger' AND name LIKE 'fde_resource_%'",
        )?
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    for trigger in triggers {
        conn.execute_batch(&format!("DROP TRIGGER {trigger}"))?;
    }
    for table in RETIRED_MODULE_TABLES {
        conn.execute_batch(&format!("DROP TABLE IF EXISTS {table}"))?;
    }
    let mut sql: String = conn.query_row(
        "SELECT sql FROM sqlite_schema WHERE name='proxy_config'",
        [],
        |row| row.get(0),
    )?;
    if !matches!(predecessor, IntegrationPredecessor::Subscription22) {
        sql = sql.replace(",'opencode'", "");
    }
    conn.execute_batch("DROP TABLE proxy_config")?;
    conn.execute_batch(&sql)?;
    conn.execute_batch(
        r#"INSERT INTO proxy_config (app_type) VALUES ('claude'),('codex'),('gemini'),('grokbuild');
        UPDATE proxy_config SET proxy_enabled=1, listen_port=32123, enabled=1,
        auto_failover_enabled=1, max_retries=9, streaming_first_byte_timeout=73,
        streaming_idle_timeout=149, non_streaming_timeout=617, circuit_failure_threshold=8,
        circuit_success_threshold=3, circuit_timeout_seconds=99, circuit_error_rate_threshold=0.8,
        circuit_min_requests=18, default_cost_multiplier='2.75', pricing_model_source='request',
        live_takeover_active=1, created_at='original-created', updated_at='original-updated';
        INSERT INTO providers(id,app_type,name,settings_config,meta,is_current) VALUES('retained','opencode','Subscription','{}','{"authBinding":{"source":"managed_account","accountId":"opaque-identity"}}',1);
        INSERT INTO settings VALUES ('integration-sentinel','preserved-setting');
        INSERT INTO proxy_live_backup(app_type,original_config,backed_up_at) VALUES('opencode','{"version":1,"original":null}','original-time');"#,
    )?;
    if matches!(predecessor, IntegrationPredecessor::Subscription22) {
        conn.execute_batch(
            "INSERT INTO proxy_config (app_type,enabled,listen_port,default_cost_multiplier,created_at,updated_at)
            VALUES ('opencode',1,32123,'4.5','opencode-created','opencode-updated');",
        )?;
    }
    if matches!(predecessor, IntegrationPredecessor::Fde22) {
        create_historical_fde_tables(conn)?;
        conn.execute_batch(
            r#"INSERT INTO fde_customers VALUES('customer','Retained customer',3,0);
            INSERT INTO fde_projects VALUES('project','customer',7,0,'{"fixture":"retained-project"}');
            INSERT INTO fde_project_kit_intents VALUES('intent','project','request','result');
            INSERT INTO fde_project_context_versions VALUES('project','generation','aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa');
            INSERT INTO fde_resource_generations VALUES('provider','opencode','retained',7);
            INSERT INTO fde_resource_generations VALUES('provider','codex','deleted-provider',9);
            INSERT INTO verification_evidence VALUES('evidence','project','2026-09-19','{"fixture":"retained-evidence"}');
            INSERT INTO verification_revocations VALUES('evidence','2026-09-20');
            INSERT INTO verification_handoff VALUES('project',5,'{"fixture":"retained-handoff"}');"#,
        )?;
        install_genuine_retired_triggers(conn)?;
    }
    Database::set_user_version(
        conn,
        if matches!(predecessor, IntegrationPredecessor::Legacy21) {
            21
        } else {
            22
        },
    )
}

type IntegrationRow = std::collections::BTreeMap<String, rusqlite::types::Value>;

fn integration_rows(conn: &Connection, table: &str) -> Result<Vec<IntegrationRow>, AppError> {
    let mut query = conn.prepare(&format!("SELECT * FROM {table} ORDER BY 1,2"))?;
    let names = query
        .column_names()
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let rows = query
        .query_map([], |row| {
            names
                .iter()
                .enumerate()
                .map(|(index, name)| Ok((name.clone(), row.get(index)?)))
                .collect()
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

fn integration_assert_current(
    conn: &Connection,
    predecessor: IntegrationPredecessor,
) -> Result<(), AppError> {
    assert_eq!(
        Database::get_user_version(conn)?,
        crate::database::SCHEMA_VERSION
    );
    assert_eq!(
        conn.query_row(
            "SELECT count(*) FROM proxy_config WHERE app_type='opencode'",
            [],
            |row| row.get::<_, i64>(0),
        )?,
        1
    );
    assert!(conn
        .execute("INSERT INTO proxy_config(app_type) VALUES ('unknown')", [])
        .is_err());
    assert_eq!(retired_trigger_count(conn)?, 0);
    assert_receipt_schema(conn)?;
    match predecessor {
        IntegrationPredecessor::Fde22 => {
            for table in RETIRED_MODULE_TABLES {
                assert!(Database::table_exists(conn, table)?, "{table}");
            }
        }
        IntegrationPredecessor::Legacy21 | IntegrationPredecessor::Subscription22 => {
            assert_eq!(retired_table_count(conn)?, 0);
        }
    }
    Ok(())
}

#[test]
fn fresh_install_does_not_create_retired_customer_project_storage() -> Result<(), AppError> {
    let fresh = Database::memory()?;
    let conn = crate::database::lock_conn!(fresh.conn);
    Database::apply_schema_migrations_on_conn(&conn)?;
    assert_eq!(
        Database::get_user_version(&conn)?,
        crate::database::SCHEMA_VERSION
    );
    assert_eq!(retired_table_count(&conn)?, 0);
    assert_eq!(retired_trigger_count(&conn)?, 0);
    assert_receipt_schema(&conn)?;
    Ok(())
}

#[test]
fn release_integration_migration_completes_both_v22_variants_legacy_and_fresh(
) -> Result<(), AppError> {
    for predecessor in [
        IntegrationPredecessor::Legacy21,
        IntegrationPredecessor::Fde22,
        IntegrationPredecessor::Subscription22,
    ] {
        let conn = Connection::open_in_memory()?;
        integration_predecessor(&conn, predecessor)?;
        let proxy = integration_rows(&conn, "proxy_config")?;
        let mut tables = vec!["providers", "settings", "proxy_live_backup"];
        if matches!(predecessor, IntegrationPredecessor::Fde22) {
            tables.extend(RETIRED_MODULE_TABLES);
        }
        let before = tables
            .iter()
            .map(|table| Ok((*table, integration_rows(&conn, table)?)))
            .collect::<Result<Vec<_>, AppError>>()?;
        Database::apply_schema_migrations_on_conn(&conn)?;
        integration_assert_current(&conn, predecessor)?;
        let after_proxy = integration_rows(&conn, "proxy_config")?;
        for row in proxy {
            assert!(
                after_proxy.contains(&row),
                "{predecessor:?} lost proxy settings"
            );
        }
        for (table, rows) in before {
            assert_eq!(
                integration_rows(&conn, table)?,
                rows,
                "{predecessor:?}: {table}"
            );
        }
        if matches!(predecessor, IntegrationPredecessor::Fde22) {
            let generation: i64 = conn.query_row(
                "SELECT generation FROM fde_resource_generations WHERE kind='provider' AND resource_id='retained'",
                [],
                |row| row.get(0),
            )?;
            conn.execute(
                "UPDATE providers SET name='changed' WHERE id='retained'",
                [],
            )?;
            assert_eq!(
                conn.query_row(
                    "SELECT generation FROM fde_resource_generations WHERE kind='provider' AND resource_id='retained'",
                    [],
                    |row| row.get::<_, i64>(0),
                )?,
                generation,
                "retired generation triggers must not write after upgrade"
            );
        }
        Database::apply_schema_migrations_on_conn(&conn)?;
        assert_eq!(integration_rows(&conn, "proxy_config")?, after_proxy);
    }
    Ok(())
}

#[test]
fn release_integration_late_version_failure_rolls_back_both_predecessor_shapes(
) -> Result<(), AppError> {
    use rusqlite::hooks::{AuthAction, AuthContext, Authorization};
    for predecessor in [
        IntegrationPredecessor::Fde22,
        IntegrationPredecessor::Subscription22,
    ] {
        let conn = Connection::open_in_memory()?;
        integration_predecessor(&conn, predecessor)?;
        let before_schema = integration_rows(&conn, "sqlite_schema")?;
        let before_proxy = integration_rows(&conn, "proxy_config")?;
        let before_generations = matches!(predecessor, IntegrationPredecessor::Fde22)
            .then(|| integration_rows(&conn, "fde_resource_generations"))
            .transpose()?;
        let target = crate::database::SCHEMA_VERSION.to_string();
        conn.authorizer(Some(move |context: AuthContext<'_>| match context.action {
            AuthAction::Pragma {
                pragma_name,
                pragma_value,
            } if pragma_name == "user_version" && pragma_value == Some(target.as_str()) => {
                Authorization::Deny
            }
            _ => Authorization::Allow,
        }));
        assert!(Database::apply_schema_migrations_on_conn(&conn).is_err());
        conn.authorizer(None::<fn(AuthContext<'_>) -> Authorization>);
        assert_eq!(Database::get_user_version(&conn)?, 22);
        assert_eq!(integration_rows(&conn, "sqlite_schema")?, before_schema);
        assert_eq!(integration_rows(&conn, "proxy_config")?, before_proxy);
        if let Some(before) = before_generations {
            assert_eq!(integration_rows(&conn, "fde_resource_generations")?, before);
        }
    }
    Ok(())
}

#[test]
#[serial]
fn release_integration_binary_restore_upgrades_both_v22_variants_without_changing_backup(
) -> Result<(), AppError> {
    let _home = TestHomeGuard::new();
    let backups = crate::config::get_app_config_dir().join("backups");
    std::fs::create_dir_all(&backups).unwrap();
    for (name, predecessor) in [
        ("fde22.db", IntegrationPredecessor::Fde22),
        ("subscription22.db", IntegrationPredecessor::Subscription22),
    ] {
        let path = backups.join(name);
        {
            let source = Connection::open(&path)?;
            integration_predecessor(&source, predecessor)?;
        }
        let bytes = std::fs::read(&path).unwrap();
        let target = Database::memory()?;
        target.restore_from_backup(name)?;
        let conn = crate::database::lock_conn!(target.conn);
        integration_assert_current(&conn, predecessor)?;
        assert_eq!(
            conn.query_row(
                "SELECT value FROM settings WHERE key='integration-sentinel'",
                [],
                |row| row.get::<_, String>(0)
            )?,
            "preserved-setting"
        );
        assert_eq!(
            conn.query_row(
                "SELECT original_config FROM proxy_live_backup WHERE app_type='opencode'",
                [],
                |row| row.get::<_, String>(0)
            )?,
            "{\"version\":1,\"original\":null}"
        );
        if matches!(predecessor, IntegrationPredecessor::Fde22) {
            assert_eq!(
                conn.query_row(
                    "SELECT generation FROM fde_resource_generations WHERE resource_id='retained'",
                    [],
                    |row| row.get::<_, i64>(0)
                )?,
                7
            );
            assert_eq!(
                conn.query_row(
                    "SELECT payload FROM verification_evidence WHERE id='evidence'",
                    [],
                    |row| row.get::<_, String>(0)
                )?,
                "{\"fixture\":\"retained-evidence\"}"
            );
        } else {
            assert_eq!(
                conn.query_row(
                    "SELECT enabled,default_cost_multiplier FROM proxy_config WHERE app_type='opencode'",
                    [],
                    |row| Ok((row.get::<_, bool>(0)?, row.get::<_, String>(1)?))
                )?,
                (true, "4.5".into())
            );
        }
        assert_eq!(std::fs::read(path).unwrap(), bytes);
    }
    Ok(())
}

#[test]
#[serial]
fn sql_import_of_old_customer_project_dump_restores_shared_config_without_reviving_module(
) -> Result<(), AppError> {
    let _home = TestHomeGuard::new();
    let source = Database::memory()?;
    {
        let conn = crate::database::lock_conn!(source.conn);
        conn.execute_batch(
            "INSERT INTO providers(id,app_type,name,settings_config,meta)
             VALUES('shared','codex','Shared','{}','{}');
             INSERT INTO settings VALUES ('shared-sentinel','kept');",
        )?;
        create_historical_fde_tables(&conn)?;
        conn.execute_batch(
            "INSERT INTO fde_customers VALUES('customer','Imported customer',1,0);
             INSERT INTO fde_projects VALUES('project','customer',1,0,'{}');",
        )?;
        install_genuine_retired_triggers(&conn)?;
    }
    let dump = source.export_sql_string()?;
    assert!(
        !dump.contains("fde_customers"),
        "new dumps must omit retired customer-project tables"
    );
    assert!(
        !dump.contains("CREATE TRIGGER"),
        "new dumps must omit retired generation triggers"
    );

    let old_dump = format!(
        "{}\nPRAGMA foreign_keys=OFF;\nPRAGMA user_version=24;\nBEGIN TRANSACTION;\n\
         CREATE TABLE providers (id TEXT NOT NULL, app_type TEXT NOT NULL, name TEXT NOT NULL, settings_config TEXT NOT NULL, meta TEXT NOT NULL DEFAULT '{{}}', is_current BOOLEAN NOT NULL DEFAULT 0, PRIMARY KEY (id, app_type));\n\
         CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT);\n\
         CREATE TABLE fde_customers (customer_id TEXT PRIMARY KEY, name TEXT NOT NULL, revision INTEGER NOT NULL, archived INTEGER NOT NULL);\n\
         INSERT INTO providers (id, app_type, name, settings_config, meta) VALUES ('shared','codex','Shared','{{}}','{{}}');\n\
         INSERT INTO settings VALUES ('shared-sentinel','kept');\n\
         INSERT INTO fde_customers VALUES ('customer','Imported customer',1,0);\n\
         COMMIT;\n",
        super::FYAGENT_SQL_EXPORT_HEADER
    );
    let target = Database::memory()?;
    target.import_sql_string(&old_dump)?;
    let conn = crate::database::lock_conn!(target.conn);
    assert_eq!(
        conn.query_row("SELECT name FROM providers WHERE id='shared'", [], |row| {
            row.get::<_, String>(0)
        })?,
        "Shared"
    );
    assert_eq!(
        conn.query_row(
            "SELECT value FROM settings WHERE key='shared-sentinel'",
            [],
            |row| row.get::<_, String>(0)
        )?,
        "kept"
    );
    assert_eq!(retired_trigger_count(&conn)?, 0);
    Ok(())
}

fn live_provider_sentinel(conn: &Connection) -> Result<(i64, String), AppError> {
    conn.query_row("SELECT COUNT(*), MIN(id) FROM providers", [], |row| {
        Ok((row.get(0)?, row.get(1)?))
    })
    .map_err(|e| AppError::Database(e.to_string()))
}

#[test]
#[serial]
fn binary_restore_rejects_keyword_only_in_comment_or_literal_and_keeps_live_and_backup(
) -> Result<(), AppError> {
    let _home = TestHomeGuard::new();
    let backup_dir = crate::config::get_app_config_dir().join("backups");
    std::fs::create_dir_all(&backup_dir).unwrap();
    for (name, trigger_sql) in [
        (
            "keyword-comment.db",
            "CREATE TRIGGER innocent_comment AFTER INSERT ON providers BEGIN
                 -- fde_resource_provider_update
                 SELECT 1;
             END;",
        ),
        (
            "keyword-literal.db",
            "CREATE TRIGGER innocent_literal AFTER INSERT ON providers BEGIN
                 INSERT INTO providers(id,app_type,name,settings_config,meta)
                 VALUES('fde_resource_x','claude','x','{}','{}');
             END;",
        ),
    ] {
        let backup_path = backup_dir.join(name);
        {
            let source = Connection::open(&backup_path)?;
            Database::create_tables_on_conn(&source)?;
            source.execute_batch(
                "INSERT INTO providers(id,app_type,name,settings_config,meta)
                 VALUES('remote','claude','Remote','{}','{}');",
            )?;
            source.execute_batch(trigger_sql)?;
        }
        let bytes = std::fs::read(&backup_path).unwrap();
        let target = Database::memory()?;
        {
            let conn = crate::database::lock_conn!(target.conn);
            conn.execute(
                "INSERT INTO providers(id,app_type,name,settings_config,meta)
                 VALUES('sentinel','claude','Existing','{}','{}')",
                [],
            )?;
        }
        let error = target
            .restore_from_backup(name)
            .expect_err("keyword-only trigger must be rejected");
        assert!(
            error.to_string().contains("持久触发器"),
            "actual error: {error}"
        );
        let conn = crate::database::lock_conn!(target.conn);
        assert_eq!(live_provider_sentinel(&conn)?, (1, "sentinel".into()));
        assert_eq!(std::fs::read(&backup_path).unwrap(), bytes);
        assert!(!retired_archive_dir().exists());
    }
    Ok(())
}

#[test]
#[serial]
fn binary_restore_rejects_forged_prefix_trigger_body_without_mutating_live_or_backup(
) -> Result<(), AppError> {
    let _home = TestHomeGuard::new();
    let backup_dir = crate::config::get_app_config_dir().join("backups");
    std::fs::create_dir_all(&backup_dir).unwrap();
    let backup_path = backup_dir.join("forged-prefix.db");
    {
        let source = Connection::open(&backup_path)?;
        Database::create_tables_on_conn(&source)?;
        create_historical_fde_tables(&source)?;
        source.execute_batch(
            "INSERT INTO providers(id,app_type,name,settings_config,meta)
             VALUES('remote','claude','Remote','{}','{}');
             CREATE TRIGGER fde_resource_provider_update AFTER UPDATE ON providers BEGIN
                 INSERT INTO fde_resource_generations(kind,app_type,resource_id,generation)
                 VALUES('provider', NEW.app_type, NEW.id, 999)
                 ON CONFLICT(kind,app_type,resource_id) DO UPDATE SET generation=999;
             END;",
        )?;
    }
    let bytes = std::fs::read(&backup_path).unwrap();
    let target = Database::memory()?;
    {
        let conn = crate::database::lock_conn!(target.conn);
        conn.execute(
            "INSERT INTO providers(id,app_type,name,settings_config,meta)
             VALUES('sentinel','claude','Existing','{}','{}')",
            [],
        )?;
    }
    let error = target
        .restore_from_backup("forged-prefix.db")
        .expect_err("forged prefix body must be rejected");
    assert!(
        error.to_string().contains("持久触发器"),
        "actual error: {error}"
    );
    let conn = crate::database::lock_conn!(target.conn);
    assert_eq!(live_provider_sentinel(&conn)?, (1, "sentinel".into()));
    assert_eq!(std::fs::read(&backup_path).unwrap(), bytes);
    Ok(())
}

#[test]
#[serial]
fn binary_restore_accepts_genuine_retired_triggers_without_migration_side_effects(
) -> Result<(), AppError> {
    let _home = TestHomeGuard::new();
    let backup_dir = crate::config::get_app_config_dir().join("backups");
    std::fs::create_dir_all(&backup_dir).unwrap();
    let backup_path = backup_dir.join("genuine-fde.db");
    {
        let source = Connection::open(&backup_path)?;
        Database::create_tables_on_conn(&source)?;
        source.execute("DELETE FROM proxy_config WHERE app_type='opencode'", [])?;
        create_historical_fde_tables(&source)?;
        source.execute_batch(
            r#"INSERT INTO providers(id,app_type,name,settings_config,meta)
               VALUES('copilot','claude','Copilot','{}','{"usage_script":{"template_type":"copilot"}}');
               INSERT INTO fde_customers VALUES('customer','Archived customer',1,0);
               INSERT INTO fde_resource_generations VALUES('provider','claude','copilot',41);
               INSERT INTO settings VALUES('genuine-sentinel','kept');"#,
        )?;
        install_genuine_retired_triggers(&source)?;
        Database::set_user_version(&source, 5)?;
    }
    let bytes = std::fs::read(&backup_path).unwrap();
    let target = Database::memory()?;
    target.restore_from_backup("genuine-fde.db")?;
    let conn = crate::database::lock_conn!(target.conn);
    assert_eq!(
        Database::get_user_version(&conn)?,
        crate::database::SCHEMA_VERSION
    );
    assert_eq!(retired_trigger_count(&conn)?, 0);
    let all_triggers: i64 = conn.query_row(
        "SELECT count(*) FROM sqlite_schema WHERE type='trigger'",
        [],
        |row| row.get(0),
    )?;
    assert_eq!(
        all_triggers, 0,
        "restore must finish with no executable triggers"
    );
    let generation: i64 = conn.query_row(
        "SELECT generation FROM fde_resource_generations WHERE resource_id='copilot'",
        [],
        |row| row.get(0),
    )?;
    assert_eq!(
        generation, 41,
        "retired triggers must not run during create/migrate"
    );
    let meta: String =
        conn.query_row("SELECT meta FROM providers WHERE id='copilot'", [], |row| {
            row.get(0)
        })?;
    assert!(
        meta.contains("github_copilot"),
        "v5 copilot rewrite must still run: {meta}"
    );
    assert_eq!(
        conn.query_row(
            "SELECT value FROM settings WHERE key='genuine-sentinel'",
            [],
            |row| row.get::<_, String>(0)
        )?,
        "kept"
    );
    conn.execute("UPDATE providers SET name='changed' WHERE id='copilot'", [])?;
    assert_eq!(
        conn.query_row(
            "SELECT generation FROM fde_resource_generations WHERE resource_id='copilot'",
            [],
            |row| row.get::<_, i64>(0)
        )?,
        41
    );
    assert_eq!(std::fs::read(&backup_path).unwrap(), bytes);
    Ok(())
}

#[test]
#[serial]
fn sql_import_authorizer_still_denies_genuine_retired_trigger_create() -> Result<(), AppError> {
    let _home = TestHomeGuard::new();
    let trigger = crate::database::retired_customer_projects::retired_fde_resource_trigger_sql()
        .into_iter()
        .find(|sql| sql.contains("fde_resource_provider_insert"))
        .expect("insert trigger");
    let dump = format!(
        "{}\nPRAGMA foreign_keys=OFF;\nBEGIN TRANSACTION;\n\
         CREATE TABLE providers (id TEXT NOT NULL, app_type TEXT NOT NULL, name TEXT NOT NULL, settings_config TEXT NOT NULL, meta TEXT NOT NULL DEFAULT '{{}}', is_current BOOLEAN NOT NULL DEFAULT 0, PRIMARY KEY (id, app_type));\n\
         INSERT INTO providers (id, app_type, name, settings_config, meta) VALUES ('shared','codex','Shared','{{}}','{{}}');\n\
         {trigger};\nCOMMIT;\n",
        super::FYAGENT_SQL_EXPORT_HEADER
    );
    let target = Database::memory()?;
    {
        let conn = crate::database::lock_conn!(target.conn);
        conn.execute(
            "INSERT INTO providers(id,app_type,name,settings_config,meta)
             VALUES('sentinel','claude','Existing','{}','{}')",
            [],
        )?;
    }
    let error = target
        .import_sql_string(&dump)
        .expect_err("SQL import must still deny CREATE TRIGGER");
    assert!(
        error.to_string().to_ascii_lowercase().contains("authoriz"),
        "genuine retired trigger CREATE must stay authorizer-denied, actual: {error}"
    );
    let conn = crate::database::lock_conn!(target.conn);
    assert_eq!(live_provider_sentinel(&conn)?, (1, "sentinel".into()));
    Ok(())
}

#[test]
#[serial]
fn sync_and_sql_import_archive_live_history_then_replace_shared_config() -> Result<(), AppError> {
    let _home = TestHomeGuard::new();
    let remote = Database::memory()?;
    {
        let conn = crate::database::lock_conn!(remote.conn);
        conn.execute_batch(
            "INSERT INTO providers(id,app_type,name,settings_config,meta)
             VALUES('remote-provider','claude','Remote Provider','{}','{}');
             INSERT INTO settings VALUES('shared-sentinel','from-remote');",
        )?;
    }
    let remote_sql = remote.export_sql_string_for_sync()?;
    assert!(!remote_sql.contains("fde_customers"));

    let local = Database::memory()?;
    seed_live_historical_customer_project(&local)?;
    {
        let conn = crate::database::lock_conn!(local.conn);
        conn.execute_batch(
            "INSERT INTO proxy_request_logs (
                 request_id, provider_id, app_type, model,
                 input_tokens, output_tokens, total_cost_usd,
                 latency_ms, status_code, created_at
             ) VALUES ('req-1', 'live-provider', 'claude', 'claude-3', 100, 50, '0.01', 120, 200, 1000);",
        )?;
    }

    local.import_sql_string_for_sync(&remote_sql)?;
    {
        let conn = crate::database::lock_conn!(local.conn);
        let name: String = conn.query_row(
            "SELECT name FROM providers WHERE id='remote-provider'",
            [],
            |row| row.get(0),
        )?;
        assert_eq!(name, "Remote Provider");
        assert!(!Database::table_exists(&conn, "fde_customers")?);
        let logs: i64 = conn.query_row(
            "SELECT COUNT(*) FROM proxy_request_logs WHERE request_id='req-1'",
            [],
            |row| row.get(0),
        )?;
        assert_eq!(logs, 1, "sync must keep local-only logs");
    }

    let archives = list_retired_archives();
    assert_eq!(
        archives.len(),
        1,
        "one durable archive per leftover replace"
    );
    let archived = Connection::open(&archives[0])?;
    let customer: String = archived.query_row(
        "SELECT name FROM fde_customers WHERE customer_id='customer'",
        [],
        |row| row.get(0),
    )?;
    assert_eq!(customer, "Live customer");
    let generation: i64 = archived.query_row(
        "SELECT generation FROM fde_resource_generations WHERE resource_id='live-provider'",
        [],
        |row| row.get(0),
    )?;
    assert_eq!(generation, 41);

    local.import_sql_string_for_sync(&remote_sql)?;
    assert_eq!(
        list_retired_archives().len(),
        1,
        "later syncs must not duplicate archives after leftover tables are gone"
    );

    let ordinary = Database::memory()?;
    seed_live_historical_customer_project(&ordinary)?;
    ordinary.import_sql_string(&remote.export_sql_string()?)?;
    {
        let conn = crate::database::lock_conn!(ordinary.conn);
        assert!(!Database::table_exists(&conn, "fde_customers")?);
        let name: String = conn.query_row(
            "SELECT name FROM providers WHERE id='remote-provider'",
            [],
            |row| row.get(0),
        )?;
        assert_eq!(name, "Remote Provider");
    }
    assert_eq!(list_retired_archives().len(), 2);
    Ok(())
}

#[test]
#[serial]
fn binary_restore_archives_live_history_before_replace() -> Result<(), AppError> {
    let _home = TestHomeGuard::new();
    let backups = crate::config::get_app_config_dir().join("backups");
    std::fs::create_dir_all(&backups).unwrap();
    let backup_path = backups.join("subscription22.db");
    {
        let source = Connection::open(&backup_path)?;
        integration_predecessor(&source, IntegrationPredecessor::Subscription22)?;
    }
    let target = Database::memory()?;
    seed_live_historical_customer_project(&target)?;
    target.restore_from_backup("subscription22.db")?;
    {
        let conn = crate::database::lock_conn!(target.conn);
        integration_assert_current(&conn, IntegrationPredecessor::Subscription22)?;
        assert!(!Database::table_exists(&conn, "fde_customers")?);
    }
    let archives = list_retired_archives();
    assert_eq!(archives.len(), 1);
    let archived = Connection::open(&archives[0])?;
    assert_eq!(
        archived.query_row(
            "SELECT name FROM fde_customers WHERE customer_id='customer'",
            [],
            |row| row.get::<_, String>(0)
        )?,
        "Live customer"
    );
    Ok(())
}

#[test]
#[serial]
fn replace_keeps_live_db_when_retired_archive_fails() -> Result<(), AppError> {
    let _home = TestHomeGuard::new();
    let archive_path = retired_archive_dir();
    std::fs::write(&archive_path, b"not-a-directory").unwrap();

    let remote = Database::memory()?;
    {
        let conn = crate::database::lock_conn!(remote.conn);
        conn.execute_batch(
            "INSERT INTO providers(id,app_type,name,settings_config,meta)
             VALUES('remote-provider','claude','Remote Provider','{}','{}');",
        )?;
    }
    let dump = remote.export_sql_string()?;

    let target = Database::memory()?;
    seed_live_historical_customer_project(&target)?;
    let live_receipts = {
        let conn = crate::database::lock_conn!(target.conn);
        seed_restore_receipt(&conn, "unresolved-before-archive-failure", "ambiguous")?;
        integration_rows(&conn, "session_restore_attempts")?
    };
    assert!(target.import_sql_string(&dump).is_err());
    assert!(target.import_sql_string_for_sync(&dump).is_err());
    {
        let conn = crate::database::lock_conn!(target.conn);
        assert_eq!(
            integration_rows(&conn, "session_restore_attempts")?,
            live_receipts
        );
        assert_eq!(
            conn.query_row(
                "SELECT name FROM fde_customers WHERE customer_id='customer'",
                [],
                |row| row.get::<_, String>(0)
            )?,
            "Live customer"
        );
        assert_eq!(
            conn.query_row(
                "SELECT name FROM providers WHERE id='live-provider'",
                [],
                |row| row.get::<_, String>(0)
            )?,
            "Live Provider"
        );
    }

    let backups = crate::config::get_app_config_dir().join("backups");
    std::fs::create_dir_all(&backups).unwrap();
    let backup_path = backups.join("subscription22.db");
    {
        let source = Connection::open(&backup_path)?;
        integration_predecessor(&source, IntegrationPredecessor::Subscription22)?;
    }
    assert!(target.restore_from_backup("subscription22.db").is_err());
    {
        let conn = crate::database::lock_conn!(target.conn);
        assert_eq!(
            integration_rows(&conn, "session_restore_attempts")?,
            live_receipts
        );
        assert_eq!(
            conn.query_row(
                "SELECT name FROM fde_customers WHERE customer_id='customer'",
                [],
                |row| row.get::<_, String>(0)
            )?,
            "Live customer"
        );
    }
    Ok(())
}

#[test]
#[serial]
fn fresh_install_does_not_create_retired_archive_directory() -> Result<(), AppError> {
    let _home = TestHomeGuard::new();
    let fresh = Database::memory()?;
    {
        let conn = crate::database::lock_conn!(fresh.conn);
        Database::apply_schema_migrations_on_conn(&conn)?;
        assert_eq!(retired_table_count(&conn)?, 0);
    }
    assert!(
        !retired_archive_dir().exists(),
        "new installs must not create the retired archive directory"
    );
    Ok(())
}

fn assert_receipt_schema(conn: &Connection) -> Result<(), AppError> {
    assert!(Database::table_exists(conn, "session_restore_attempts")?);
    let indexes: i64 = conn.query_row(
        "SELECT count(*) FROM sqlite_schema WHERE type='index'
         AND tbl_name='session_restore_attempts'
         AND name IN ('uq_sra_request','uq_sra_slot','idx_sra_action','idx_sra_native')",
        [],
        |row| row.get(0),
    )?;
    assert_eq!(
        indexes, 4,
        "receipt constraints and lookup indexes must migrate together"
    );
    Ok(())
}

fn seed_restore_receipt(conn: &Connection, id: &str, stage: &str) -> Result<(), AppError> {
    conn.execute(
        "INSERT INTO session_restore_attempts (
            attempt_id, request_id, installation_id, request_fingerprint,
            snapshot_id, origin_id, request_kind, idempotency_slot, content_digest,
            origin_provider_id, target_provider_id, target_store_id, target_native_nonce,
            target_native_id, stage, device_binding, created_at, updated_at
         ) VALUES (?1, ?1, 'fixture-installation', ?1, ?1, ?1, 'defaultImport', ?1, ?1,
                   'codex', 'codex', 'fixture-store', ?1, ?1, ?2, 'fixture-device', 100, 101)",
        rusqlite::params![id, stage],
    )?;
    Ok(())
}

#[test]
fn retirement_then_receipts_migrate_v24_and_v25_and_survive_reopen() -> Result<(), AppError> {
    for predecessor in [24, 25] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("upgrade.db");
        let history;
        {
            let conn = Connection::open(&path)?;
            Database::create_tables_on_conn(&conn)?;
            conn.execute_batch("DROP TABLE session_restore_attempts")?;
            create_historical_fde_tables(&conn)?;
            conn.execute_batch(
                "INSERT INTO fde_customers VALUES('customer','Keep history',3,0);
                 INSERT INTO fde_resource_generations VALUES('provider','codex','retained',41);
                 INSERT INTO settings VALUES('upgrade-sentinel','keep-config');",
            )?;
            if predecessor == 24 {
                install_genuine_retired_triggers(&conn)?;
                assert_eq!(retired_trigger_count(&conn)?, 20);
            }
            Database::set_user_version(&conn, predecessor)?;
            history = integration_rows(&conn, "fde_customers")?;
            Database::apply_schema_migrations_on_conn(&conn)?;
            assert_eq!(
                Database::get_user_version(&conn)?,
                crate::database::SCHEMA_VERSION
            );
            assert_eq!(retired_trigger_count(&conn)?, 0);
            assert_receipt_schema(&conn)?;
            assert_eq!(integration_rows(&conn, "fde_customers")?, history);
            assert_eq!(
                conn.query_row(
                    "SELECT value FROM settings WHERE key='upgrade-sentinel'",
                    [],
                    |row| row.get::<_, String>(0)
                )?,
                "keep-config"
            );
            seed_restore_receipt(&conn, "after-upgrade", "ambiguous")?;
        }
        let reopened = Connection::open(&path)?;
        let receipts = integration_rows(&reopened, "session_restore_attempts")?;
        Database::create_tables_on_conn(&reopened)?;
        Database::apply_schema_migrations_on_conn(&reopened)?;
        assert_receipt_schema(&reopened)?;
        assert_eq!(retired_trigger_count(&reopened)?, 0);
        assert_eq!(integration_rows(&reopened, "fde_customers")?, history);
        assert_eq!(
            integration_rows(&reopened, "session_restore_attempts")?,
            receipts
        );
    }
    Ok(())
}

#[test]
fn receipt_migration_failure_rolls_back_retirement_and_version() -> Result<(), AppError> {
    for predecessor in [24, 25] {
        let conn = Connection::open_in_memory()?;
        Database::create_tables_on_conn(&conn)?;
        conn.execute_batch("DROP TABLE session_restore_attempts")?;
        create_historical_fde_tables(&conn)?;
        conn.execute_batch("INSERT INTO fde_customers VALUES('customer','Keep on failure',3,0)")?;
        if predecessor == 24 {
            install_genuine_retired_triggers(&conn)?;
        }
        // A same-name view makes the v26 CREATE INDEX fail after v25 dropped triggers.
        conn.execute_batch(
            "CREATE VIEW session_restore_attempts AS SELECT 'blocked' AS attempt_id",
        )?;
        Database::set_user_version(&conn, predecessor)?;
        let before_schema = integration_rows(&conn, "sqlite_schema")?;
        let history = integration_rows(&conn, "fde_customers")?;
        assert!(Database::apply_schema_migrations_on_conn(&conn).is_err());
        assert_eq!(Database::get_user_version(&conn)?, predecessor);
        assert_eq!(integration_rows(&conn, "sqlite_schema")?, before_schema);
        assert_eq!(integration_rows(&conn, "fde_customers")?, history);
        assert_eq!(
            retired_trigger_count(&conn)?,
            if predecessor == 24 { 20 } else { 0 }
        );
        conn.execute_batch("DROP VIEW session_restore_attempts")?;
        Database::apply_schema_migrations_on_conn(&conn)?;
        assert_receipt_schema(&conn)?;
        assert_eq!(
            Database::get_user_version(&conn)?,
            crate::database::SCHEMA_VERSION
        );
        assert_eq!(retired_trigger_count(&conn)?, 0);
    }
    Ok(())
}

#[test]
#[serial]
fn sql_and_sync_preserve_live_receipts_while_archiving_retired_history() -> Result<(), AppError> {
    let _home = TestHomeGuard::new();
    let remote = Database::memory()?;
    {
        let conn = crate::database::lock_conn!(remote.conn);
        seed_restore_receipt(&conn, "foreign-receipt-sentinel", "nativeReadbackVerified")?;
        conn.execute_batch(
            "INSERT INTO settings VALUES('receipt-config-sentinel','remote');
             INSERT INTO providers(id,app_type,name,settings_config,meta)
             VALUES('remote-receipt-provider','codex','Remote fixture','{}','{}')",
        )?;
    }
    for sql in [
        remote.export_sql_string()?,
        remote.export_sql_string_for_sync()?,
    ] {
        assert!(
            !sql.contains("foreign-receipt-sentinel"),
            "exports must omit receipt rows"
        );
    }
    // Legacy/untrusted dumps may contain receipt rows despite the current export policy.
    let foreign_dump = Database::dump_sql(&remote.snapshot_to_memory()?, &[])?;
    assert!(foreign_dump.contains("foreign-receipt-sentinel"));
    for sync in [false, true] {
        let local = Database::memory()?;
        seed_live_historical_customer_project(&local)?;
        let receipts = {
            let conn = crate::database::lock_conn!(local.conn);
            seed_restore_receipt(&conn, "local-complete", "nativeReadbackVerified")?;
            seed_restore_receipt(&conn, "local-unresolved", "ambiguous")?;
            integration_rows(&conn, "session_restore_attempts")?
        };
        for sql in [
            local.export_sql_string()?,
            local.export_sql_string_for_sync()?,
        ] {
            assert!(!sql.contains("local-complete"));
            for table in RETIRED_MODULE_TABLES {
                assert!(
                    !sql.contains(table),
                    "retired schema must stay out of portable dumps"
                );
            }
        }
        if sync {
            local.import_sql_string_for_sync(&foreign_dump)?;
        } else {
            local.import_sql_string(&foreign_dump)?;
        }
        let conn = crate::database::lock_conn!(local.conn);
        assert_eq!(
            integration_rows(&conn, "session_restore_attempts")?,
            receipts
        );
        assert_eq!(retired_table_count(&conn)?, 0);
        assert_eq!(
            conn.query_row(
                "SELECT value FROM settings WHERE key='receipt-config-sentinel'",
                [],
                |row| row.get::<_, String>(0)
            )?,
            "remote"
        );
    }
    let archives = list_retired_archives();
    assert_eq!(archives.len(), 2);
    for path in archives {
        let archived = Connection::open(path)?;
        assert_eq!(retired_table_count(&archived)?, 8);
        assert_eq!(
            integration_rows(&archived, "session_restore_attempts")?.len(),
            2
        );
        assert_eq!(
            archived.query_row(
                "SELECT name FROM fde_customers WHERE customer_id='customer'",
                [],
                |row| row.get::<_, String>(0)
            )?,
            "Live customer"
        );
    }
    Ok(())
}

#[test]
#[serial]
fn binary_restore_keeps_current_receipts_for_v25_and_current_backups() -> Result<(), AppError> {
    let _home = TestHomeGuard::new();
    let backups = crate::config::get_app_config_dir().join("backups");
    std::fs::create_dir_all(&backups).unwrap();
    for old_schema in [true, false] {
        let filename = if old_schema {
            "before-receipts.db"
        } else {
            "older-receipts.db"
        };
        let path = backups.join(filename);
        {
            let backup = Connection::open(&path)?;
            Database::create_tables_on_conn(&backup)?;
            backup.execute_batch("INSERT INTO settings VALUES('binary-sentinel','from-backup')")?;
            if old_schema {
                backup.execute_batch("DROP TABLE session_restore_attempts")?;
                Database::set_user_version(&backup, 25)?;
            } else {
                Database::apply_schema_migrations_on_conn(&backup)?;
                seed_restore_receipt(&backup, "local-complete", "nativeWritePending")?;
                seed_restore_receipt(&backup, "foreign-receipt", "nativeReadbackVerified")?;
            }
        }
        let original_bytes = std::fs::read(&path).unwrap();
        // Cover a device with receipts and a fresh device: foreign mappings never replay.
        for populated in [true, false] {
            let local = Database::memory()?;
            seed_live_historical_customer_project(&local)?;
            let receipts = {
                let conn = crate::database::lock_conn!(local.conn);
                if populated {
                    seed_restore_receipt(&conn, "local-complete", "nativeReadbackVerified")?;
                    seed_restore_receipt(&conn, "local-unresolved", "ambiguous")?;
                }
                integration_rows(&conn, "session_restore_attempts")?
            };
            local.restore_from_backup(filename)?;
            let conn = crate::database::lock_conn!(local.conn);
            assert_receipt_schema(&conn)?;
            assert_eq!(
                Database::get_user_version(&conn)?,
                crate::database::SCHEMA_VERSION
            );
            assert_eq!(
                integration_rows(&conn, "session_restore_attempts")?,
                receipts
            );
            assert_eq!(retired_table_count(&conn)?, 0);
            assert_eq!(
                conn.query_row(
                    "SELECT value FROM settings WHERE key='binary-sentinel'",
                    [],
                    |row| row.get::<_, String>(0)
                )?,
                "from-backup"
            );
            assert_eq!(std::fs::read(&path).unwrap(), original_bytes);
        }
    }
    assert_eq!(list_retired_archives().len(), 4);
    Ok(())
}

#[test]
fn final_publication_keeps_receipt_claims_and_updates_after_early_snapshot() -> Result<(), AppError>
{
    let local = Database::memory()?;
    {
        let conn = crate::database::lock_conn!(local.conn);
        seed_restore_receipt(&conn, "in-flight", "nativeWritePending")?;
    }
    let candidate = local.snapshot_to_memory()?;
    candidate.execute_batch("INSERT INTO settings VALUES('candidate-sentinel','ready')")?;
    let latest = {
        let conn = crate::database::lock_conn!(local.conn);
        conn.execute(
            "UPDATE session_restore_attempts SET stage='ambiguous', updated_at=102
                      WHERE attempt_id='in-flight'",
            [],
        )?;
        seed_restore_receipt(&conn, "late-claim", "nativeReadbackVerified")?;
        integration_rows(&conn, "session_restore_attempts")?
    };
    local.replace_from_candidate_preserving_receipts(&candidate)?;
    let conn = crate::database::lock_conn!(local.conn);
    assert_eq!(integration_rows(&conn, "session_restore_attempts")?, latest);
    assert_eq!(
        conn.query_row(
            "SELECT value FROM settings WHERE key='candidate-sentinel'",
            [],
            |row| row.get::<_, String>(0)
        )?,
        "ready"
    );
    Ok(())
}

#[test]
fn receipt_preservation_failure_does_not_publish_candidate() -> Result<(), AppError> {
    let local = Database::memory()?;
    {
        let conn = crate::database::lock_conn!(local.conn);
        seed_restore_receipt(&conn, "local-unresolved", "ambiguous")?;
        conn.execute_batch("INSERT INTO settings VALUES('live-sentinel','keep')")?;
    }
    let candidate = local.snapshot_to_memory()?;
    candidate.execute_batch(
        "DELETE FROM settings WHERE key='live-sentinel';
         DROP TABLE session_restore_attempts;
         CREATE TABLE session_restore_attempts(attempt_id TEXT PRIMARY KEY)",
    )?;
    let before = {
        let conn = crate::database::lock_conn!(local.conn);
        integration_rows(&conn, "session_restore_attempts")?
    };
    assert!(local
        .replace_from_candidate_preserving_receipts(&candidate)
        .is_err());
    let conn = crate::database::lock_conn!(local.conn);
    assert_eq!(integration_rows(&conn, "session_restore_attempts")?, before);
    assert_eq!(
        conn.query_row(
            "SELECT value FROM settings WHERE key='live-sentinel'",
            [],
            |row| row.get::<_, String>(0)
        )?,
        "keep"
    );
    Ok(())
}
