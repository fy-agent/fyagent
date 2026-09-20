use super::*;
use crate::{
    app_config::{InstalledSkill, McpServer},
    prompt::Prompt,
    provider::Provider,
};
use rusqlite::Connection;

fn version(db: &Database, kind: ResourceKind, app: &str) -> i64 {
    db.project_resource_version(kind, app, "source")
        .unwrap()
        .unwrap()
        .strip_prefix("db:")
        .unwrap()
        .parse()
        .unwrap()
}

#[test]
fn projects_resource_generations_owner_updates_aba_delete_reinsert_and_scope() {
    let db = Database::memory().unwrap();
    let mut provider = Provider::with_id(
        "source".into(),
        "Provider".into(),
        serde_json::json!({"apiKey":"fixture-A"}),
        None,
    );
    let mut prompt = Prompt {
        id: "source".into(),
        name: "Prompt".into(),
        content: "fixture-A".into(),
        description: None,
        enabled: false,
        created_at: None,
        updated_at: None,
    };
    let mut mcp: McpServer = serde_json::from_value(serde_json::json!({"id":"source","name":"MCP","server":{"url":"https://a.example"},"apps":{}})).unwrap();
    let mut skill: InstalledSkill = serde_json::from_value(serde_json::json!({"id":"source","name":"Skill A","directory":"fixture","apps":{},"installedAt":1})).unwrap();
    db.save_provider("codex", &provider).unwrap();
    db.save_provider("claude", &provider).unwrap();
    db.save_prompt("codex", &prompt).unwrap();
    db.save_prompt("claude", &prompt).unwrap();
    db.save_mcp_server(&mcp).unwrap();
    db.save_skill(&skill).unwrap();
    let versions = || {
        [
            ResourceKind::Provider,
            ResourceKind::Prompt,
            ResourceKind::Mcp,
            ResourceKind::Skill,
        ]
        .map(|kind| version(&db, kind, "codex"))
    };
    let initial = versions();
    for value in ["fixture-B", "fixture-A"] {
        provider.settings_config = serde_json::json!({"apiKey":value});
        prompt.content = value.into();
        mcp.server = serde_json::json!({"url":value});
        skill.name = value.into();
        let before = versions();
        db.save_provider("codex", &provider).unwrap();
        db.save_prompt("codex", &prompt).unwrap();
        db.save_mcp_server(&mcp).unwrap();
        assert!(db.update_skill_metadata(&skill).unwrap());
        assert!(versions()
            .into_iter()
            .zip(before)
            .all(|(after, before)| after > before));
    }
    assert!(versions()
        .into_iter()
        .zip(initial)
        .all(|(after, before)| after > before));
    assert_eq!(version(&db, ResourceKind::Provider, "claude"), 1);
    assert_eq!(version(&db, ResourceKind::Prompt, "claude"), 1);
    assert_eq!(
        version(&db, ResourceKind::Mcp, "claude"),
        version(&db, ResourceKind::Mcp, "codex")
    );
    assert_eq!(
        version(&db, ResourceKind::Skill, "claude"),
        version(&db, ResourceKind::Skill, "codex")
    );
    let before_delete = versions();
    db.delete_provider("codex", "source").unwrap();
    db.delete_prompt("codex", "source").unwrap();
    db.delete_mcp_server("source").unwrap();
    db.delete_skill("source").unwrap();
    let tombstones = versions();
    assert!(tombstones
        .into_iter()
        .zip(before_delete)
        .all(|(after, before)| after > before));
    db.save_provider("codex", &provider).unwrap();
    db.save_prompt("codex", &prompt).unwrap();
    db.save_mcp_server(&mcp).unwrap();
    db.save_skill(&skill).unwrap();
    assert!(versions()
        .into_iter()
        .zip(tombstones)
        .all(|(after, before)| after > before));
}

#[test]
fn projects_resource_generations_provider_custom_endpoints_follow_owner() {
    let db = Database::memory().unwrap();
    let provider = Provider::with_id(
        "source".into(),
        "Provider".into(),
        serde_json::json!({}),
        None,
    );
    db.save_provider("codex", &provider).unwrap();
    let original = version(&db, ResourceKind::Provider, "codex");
    db.add_custom_endpoint("codex", "source", "https://fixture.example")
        .unwrap();
    let added = version(&db, ResourceKind::Provider, "codex");
    assert!(added > original);
    db.remove_custom_endpoint("codex", "source", "https://fixture.example")
        .unwrap();
    assert!(version(&db, ResourceKind::Provider, "codex") > added);
}

#[test]
fn projects_resource_generations_migrate_seed_once_and_rollback_with_owner() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("CREATE TABLE prompts(id TEXT NOT NULL,app_type TEXT NOT NULL,content TEXT NOT NULL,PRIMARY KEY(id,app_type)); INSERT INTO prompts VALUES('source','codex','private'); PRAGMA user_version=21;").unwrap();
    Database::apply_schema_migrations_on_conn(&conn).unwrap();
    let generation = || {
        conn.query_row(
            "SELECT generation FROM fde_resource_generations WHERE kind='prompt'",
            [],
            |r| r.get::<_, i64>(0),
        )
        .unwrap()
    };
    assert_eq!(generation(), 1);
    Database::create_project_tables_on_conn(&conn).unwrap();
    assert_eq!(generation(), 1);
    conn.execute_batch("SAVEPOINT fixture; UPDATE prompts SET content='temporary'; ROLLBACK TO fixture; RELEASE fixture;").unwrap();
    assert_eq!(generation(), 1);
    assert_eq!(
        conn.query_row("SELECT content FROM prompts", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "private"
    );
    conn.execute_batch("UPDATE prompts SET content='B'; UPDATE prompts SET content='private';")
        .unwrap();
    assert_eq!(generation(), 3);
    Database::advance_project_resource_generations_on_conn(&conn).unwrap();
    assert_eq!(generation(), 4);
}

#[test]
fn projects_resource_generations_read_only_snapshot_and_secret_neutral_versions() {
    let (_t, s) = fixture();
    let c = s.create_customer("Customer").unwrap();
    let p = s.create(&c.customer_id, "Project").unwrap();
    let mut prompt = Prompt {
        id: "source".into(),
        name: "Prompt".into(),
        content: "sensitive fixture A".into(),
        description: None,
        enabled: false,
        created_at: None,
        updated_at: None,
    };
    s.db.save_prompt("codex", &prompt).unwrap();
    let p = s
        .bind_resource(&request(&p), ResourceKind::Prompt, "codex", "source", None)
        .unwrap();
    let writes = || {
        s.db.conn
            .lock()
            .unwrap()
            .query_row("SELECT total_changes()", [], |r| r.get::<_, i64>(0))
            .unwrap()
    };
    let before = writes();
    let initial = s.dependency_snapshot(&p.project_id).unwrap();
    s.resource_options().unwrap();
    assert_eq!(writes(), before);
    assert_eq!(initial.resources[0].state, ObservationState::Matched);
    assert!(!serde_json::to_string(&initial)
        .unwrap()
        .contains("sensitive fixture"));
    for content in ["sensitive fixture B", "sensitive fixture A"] {
        prompt.content = content.into();
        s.db.save_prompt("codex", &prompt).unwrap();
        assert_eq!(
            s.dependency_snapshot(&p.project_id).unwrap().resources[0].state,
            ObservationState::Drifted
        );
    }
    s.db.delete_prompt("codex", "source").unwrap();
    assert_eq!(
        s.dependency_snapshot(&p.project_id).unwrap().resources[0].state,
        ObservationState::Missing
    );
    s.db.save_prompt("codex", &prompt).unwrap();
    assert_eq!(
        s.dependency_snapshot(&p.project_id).unwrap().resources[0].state,
        ObservationState::Drifted
    );
}

#[test]
fn projects_resource_generations_identity_move_and_exhaustion_fail_closed() {
    let db = Database::memory().unwrap();
    let conn = db.conn.lock().unwrap();
    conn.execute_batch("INSERT INTO prompts(id,app_type,name,content) VALUES('source','codex','Name','private'); UPDATE prompts SET app_type='claude';").unwrap();
    let counter = |app: &str| {
        conn.query_row(
            "SELECT generation FROM fde_resource_generations WHERE kind='prompt' AND app_type=?1",
            [app],
            |r| r.get::<_, i64>(0),
        )
        .unwrap()
    };
    assert_eq!(counter("codex"), 2);
    assert_eq!(counter("claude"), 1);
    conn.execute_batch("UPDATE fde_resource_generations SET generation=9007199254740990 WHERE kind='prompt' AND app_type='claude';").unwrap();
    assert!(conn
        .execute_batch("UPDATE prompts SET content='should rollback';")
        .is_err());
    assert_eq!(
        conn.query_row("SELECT content FROM prompts", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "private"
    );
    assert_eq!(counter("claude"), 9007199254740990);
}
