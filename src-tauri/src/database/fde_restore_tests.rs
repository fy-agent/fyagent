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

#[derive(Clone, Copy, Debug)]
enum IntegrationPredecessor {
    Legacy21,
    Fde22,
    Subscription22,
}

const INTEGRATION_FDE_TABLES: [&str; 8] = [
    "verification_revocations",
    "verification_evidence",
    "verification_handoff",
    "fde_project_context_versions",
    "fde_project_kit_intents",
    "fde_projects",
    "fde_customers",
    "fde_resource_generations",
];

fn integration_predecessor(
    conn: &Connection,
    predecessor: IntegrationPredecessor,
) -> Result<(), AppError> {
    Database::create_tables_on_conn(conn)?;
    let triggers = conn
        .prepare("SELECT name FROM sqlite_schema WHERE type='trigger' AND name LIKE 'fde_resource_%'")?
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    for trigger in triggers {
        conn.execute_batch(&format!("DROP TRIGGER {trigger}"))?;
    }
    for table in INTEGRATION_FDE_TABLES {
        conn.execute_batch(&format!("DROP TABLE {table}"))?;
    }
    // These are the two immutable branch shapes: FDE v22 retained the four-
    // target CHECK; subscription v22 had five targets and no FDE tables.
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
    conn.execute_batch(r#"INSERT INTO proxy_config (app_type) VALUES ('claude'),('codex'),('gemini'),('grokbuild');
        UPDATE proxy_config SET proxy_enabled=1, listen_port=32123, enabled=1,
        auto_failover_enabled=1, max_retries=9, streaming_first_byte_timeout=73,
        streaming_idle_timeout=149, non_streaming_timeout=617, circuit_failure_threshold=8,
        circuit_success_threshold=3, circuit_timeout_seconds=99, circuit_error_rate_threshold=0.8,
        circuit_min_requests=18, default_cost_multiplier='2.75', pricing_model_source='request',
        live_takeover_active=1, created_at='original-created', updated_at='original-updated';
        INSERT INTO providers(id,app_type,name,settings_config,meta,is_current) VALUES('retained','opencode','Subscription','{}','{"authBinding":{"source":"managed_account","accountId":"opaque-identity"}}',1);
        INSERT INTO settings VALUES ('integration-sentinel','preserved-setting');
        INSERT INTO proxy_live_backup(app_type,original_config,backed_up_at) VALUES('opencode','{"version":1,"original":null}','original-time');"#)?;
    if matches!(predecessor, IntegrationPredecessor::Subscription22) {
        conn.execute_batch("INSERT INTO proxy_config (app_type,enabled,listen_port,default_cost_multiplier,created_at,updated_at)
            VALUES ('opencode',1,32123,'4.5','opencode-created','opencode-updated');")?;
    }
    if matches!(predecessor, IntegrationPredecessor::Fde22) {
        Database::create_project_tables_on_conn(conn)?;
        Database::migrate_verification_v22(conn)?;
        conn.execute_batch(r#"INSERT INTO fde_customers VALUES('customer','Retained customer',3,0);
            INSERT INTO fde_projects VALUES('project','customer',7,0,'{"fixture":"retained-project"}');
            INSERT INTO fde_project_kit_intents VALUES('intent','project','request','result');
            INSERT INTO fde_project_context_versions VALUES('project','generation','aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa');
            UPDATE fde_resource_generations SET generation=7 WHERE resource_id='retained';
            INSERT INTO fde_resource_generations VALUES('provider','codex','deleted-provider',9);
            INSERT INTO verification_evidence VALUES('evidence','project','2026-09-19','{"fixture":"retained-evidence"}');
            INSERT INTO verification_revocations VALUES('evidence','2026-09-20');
            INSERT INTO verification_handoff VALUES('project',5,'{"fixture":"retained-handoff"}');"#)?;
    }
    Database::set_user_version(
        conn,
        if matches!(predecessor, IntegrationPredecessor::Legacy21) { 21 } else { 22 },
    )
}

type IntegrationRow = std::collections::BTreeMap<String, rusqlite::types::Value>;

fn integration_rows(conn: &Connection, table: &str) -> Result<Vec<IntegrationRow>, AppError> {
    let mut query = conn.prepare(&format!("SELECT * FROM {table} ORDER BY 1,2"))?;
    let names = query.column_names().into_iter().map(str::to_owned).collect::<Vec<_>>();
    let rows = query
        .query_map([], |row| {
            names.iter().enumerate()
                .map(|(index, name)| Ok((name.clone(), row.get(index)?)))
                .collect()
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

fn integration_assert_current(conn: &Connection) -> Result<(), AppError> {
    assert_eq!(Database::get_user_version(conn)?, crate::database::SCHEMA_VERSION);
    for table in INTEGRATION_FDE_TABLES {
        assert!(Database::table_exists(conn, table)?, "{table}");
    }
    assert_eq!(conn.query_row(
        "SELECT count(*) FROM proxy_config WHERE app_type='opencode'", [],
        |row| row.get::<_, i64>(0),
    )?, 1);
    assert!(conn.execute("INSERT INTO proxy_config(app_type) VALUES ('unknown')", []).is_err());
    assert_eq!(conn.query_row(
        "SELECT count(*) FROM sqlite_schema WHERE type='trigger' AND name LIKE 'fde_resource_%'", [],
        |row| row.get::<_, i64>(0),
    )?, 20);
    Ok(())
}

#[test]
fn release_integration_migration_completes_both_v22_variants_legacy_and_fresh() -> Result<(), AppError> {
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
            tables.extend(INTEGRATION_FDE_TABLES);
        }
        let before = tables.iter()
            .map(|table| Ok((*table, integration_rows(&conn, table)?)))
            .collect::<Result<Vec<_>, AppError>>()?;
        Database::apply_schema_migrations_on_conn(&conn)?;
        integration_assert_current(&conn)?;
        let after_proxy = integration_rows(&conn, "proxy_config")?;
        for row in proxy {
            assert!(after_proxy.contains(&row), "{predecessor:?} lost proxy settings");
        }
        for (table, rows) in before {
            assert_eq!(integration_rows(&conn, table)?, rows, "{predecessor:?}: {table}");
        }
        let stable = integration_rows(&conn, "fde_resource_generations")?;
        Database::apply_schema_migrations_on_conn(&conn)?;
        assert_eq!(integration_rows(&conn, "proxy_config")?, after_proxy);
        assert_eq!(integration_rows(&conn, "fde_resource_generations")?, stable);
        let generation: i64 = conn.query_row(
            "SELECT generation FROM fde_resource_generations WHERE kind='provider' AND resource_id='retained'", [],
            |row| row.get(0),
        )?;
        conn.execute("UPDATE providers SET name='changed' WHERE id='retained'", [])?;
        assert_eq!(conn.query_row(
            "SELECT generation FROM fde_resource_generations WHERE kind='provider' AND resource_id='retained'", [],
            |row| row.get::<_, i64>(0),
        )?, generation + 1);
    }
    let fresh = Database::memory()?;
    let conn = crate::database::lock_conn!(fresh.conn);
    Database::apply_schema_migrations_on_conn(&conn)?;
    integration_assert_current(&conn)?;
    Ok(())
}

#[test]
fn release_integration_late_version_failure_rolls_back_both_predecessor_shapes() -> Result<(), AppError> {
    use rusqlite::hooks::{AuthAction, AuthContext, Authorization};
    for predecessor in [IntegrationPredecessor::Fde22, IntegrationPredecessor::Subscription22] {
        let conn = Connection::open_in_memory()?;
        integration_predecessor(&conn, predecessor)?;
        let before_schema = integration_rows(&conn,"sqlite_schema")?;
        let before_proxy = integration_rows(&conn,"proxy_config")?;
        let before_generations = matches!(predecessor,IntegrationPredecessor::Fde22).then(||integration_rows(&conn,"fde_resource_generations")).transpose()?;
        let target = crate::database::SCHEMA_VERSION.to_string();
        conn.authorizer(Some(move |context: AuthContext<'_>| match context.action {
            AuthAction::Pragma { pragma_name, pragma_value } if pragma_name == "user_version" && pragma_value == Some(target.as_str()) => Authorization::Deny,
            _ => Authorization::Allow,
        }));
        assert!(Database::apply_schema_migrations_on_conn(&conn).is_err());
        conn.authorizer(None::<fn(AuthContext<'_>) -> Authorization>);
        assert_eq!(Database::get_user_version(&conn)?,22);
        assert_eq!(integration_rows(&conn,"sqlite_schema")?,before_schema);
        assert_eq!(integration_rows(&conn,"proxy_config")?,before_proxy);
        if let Some(before) = before_generations { assert_eq!(integration_rows(&conn,"fde_resource_generations")?,before); }
    }
    Ok(())
}

#[test]
#[serial]
fn release_integration_binary_restore_upgrades_both_v22_variants_without_changing_backup() -> Result<(), AppError> {
    let _home = TestHomeGuard::new();
    let backups = crate::config::get_app_config_dir().join("backups");
    std::fs::create_dir_all(&backups).unwrap();
    for (name, predecessor) in [("fde22.db",IntegrationPredecessor::Fde22),("subscription22.db",IntegrationPredecessor::Subscription22)] {
        let path = backups.join(name);
        {
            let source = Connection::open(&path)?;
            integration_predecessor(&source,predecessor)?;
        }
        let bytes = std::fs::read(&path).unwrap();
        let target = Database::memory()?;
        target.restore_from_backup(name)?;
        let conn = crate::database::lock_conn!(target.conn);
        integration_assert_current(&conn)?;
        assert_eq!(conn.query_row("SELECT value FROM settings WHERE key='integration-sentinel'",[],|row|row.get::<_,String>(0))?,"preserved-setting");
        assert_eq!(conn.query_row("SELECT original_config FROM proxy_live_backup WHERE app_type='opencode'",[],|row|row.get::<_,String>(0))?,"{\"version\":1,\"original\":null}");
        if matches!(predecessor,IntegrationPredecessor::Fde22) {
            assert_eq!(conn.query_row("SELECT generation FROM fde_resource_generations WHERE resource_id='retained'",[],|row|row.get::<_,i64>(0))?,8);
            assert_eq!(conn.query_row("SELECT payload FROM verification_evidence WHERE id='evidence'",[],|row|row.get::<_,String>(0))?,"{\"fixture\":\"retained-evidence\"}");
        } else {
            assert_eq!(conn.query_row("SELECT enabled,default_cost_multiplier FROM proxy_config WHERE app_type='opencode'",[],|row|Ok((row.get::<_,bool>(0)?,row.get::<_,String>(1)?)))?,(true,"4.5".into()));
        }
        assert_eq!(std::fs::read(path).unwrap(),bytes);
    }
    Ok(())
}
