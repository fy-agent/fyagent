//! Closed compatibility for leftover customer-project storage.
//!
//! This is not a product DAO. It does not read or write domain records at
//! runtime and it does not revive the retired module. Backup restore uses the
//! frozen trigger text so a binary snapshot can be accepted or rejected
//! without a substring match.

/// Retired customer-project tables. New dumps omit them so an import cannot
/// revive the removed module. Leftover rows in an upgraded live database are
/// archived before a replace that would drop them.
pub(crate) const RETIRED_MODULE_TABLES: &[&str] = &[
    "fde_customers",
    "fde_projects",
    "fde_project_kit_intents",
    "fde_project_context_versions",
    "fde_resource_generations",
    "verification_revocations",
    "verification_evidence",
    "verification_handoff",
];

/// Durable archive directory under the app config dir. Ordinary rotating
/// `backups/` retention must not delete these files.
pub(crate) const RETIRED_ARCHIVE_DIRNAME: &str = "retired-customer-projects";

/// Historical `CREATE TRIGGER IF NOT EXISTS` text from baseline
/// `c0b2ec21` `Database::project_resource_trigger_sql()`.
pub(crate) fn retired_fde_resource_trigger_sql() -> Vec<String> {
    let mut triggers = Vec::new();
    for (kind, table, scoped) in [
        ("provider", "providers", true),
        ("prompt", "prompts", true),
        ("mcp", "mcp_servers", false),
        ("skill", "skills", false),
    ] {
        let bump = |row: &str| {
            let app = if scoped {
                format!("{row}.app_type")
            } else {
                "''".into()
            };
            format!("INSERT INTO fde_resource_generations(kind,app_type,resource_id,generation) VALUES('{kind}',{app},{row}.id,1) ON CONFLICT(kind,app_type,resource_id) DO UPDATE SET generation=generation+1;")
        };
        for (event, row) in [("INSERT", "NEW"), ("DELETE", "OLD")] {
            triggers.push(format!(
                "CREATE TRIGGER IF NOT EXISTS fde_resource_{kind}_{} AFTER {event} ON {table} BEGIN {} END",
                event.to_lowercase(),
                bump(row)
            ));
        }
        let same_identity = if scoped {
            "OLD.id IS NEW.id AND OLD.app_type IS NEW.app_type"
        } else {
            "OLD.id IS NEW.id"
        };
        triggers.push(format!(
            "CREATE TRIGGER IF NOT EXISTS fde_resource_{kind}_update AFTER UPDATE ON {table} BEGIN {} END",
            bump("OLD")
        ));
        triggers.push(format!(
            "CREATE TRIGGER IF NOT EXISTS fde_resource_{kind}_rekey AFTER UPDATE ON {table} WHEN NOT ({same_identity}) BEGIN {} END",
            bump("NEW")
        ));
    }
    let endpoint_bump = |row: &str| {
        format!("INSERT INTO fde_resource_generations(kind,app_type,resource_id,generation) VALUES('provider',{row}.app_type,{row}.provider_id,1) ON CONFLICT(kind,app_type,resource_id) DO UPDATE SET generation=generation+1;")
    };
    for (event, row) in [("INSERT", "NEW"), ("DELETE", "OLD"), ("UPDATE", "OLD")] {
        triggers.push(format!(
            "CREATE TRIGGER IF NOT EXISTS fde_resource_provider_endpoint_{} AFTER {event} ON provider_endpoints BEGIN {} END",
            event.to_lowercase(),
            endpoint_bump(row)
        ));
    }
    triggers.push(format!(
        "CREATE TRIGGER IF NOT EXISTS fde_resource_provider_endpoint_rekey AFTER UPDATE ON provider_endpoints WHEN NOT (OLD.provider_id IS NEW.provider_id AND OLD.app_type IS NEW.app_type) BEGIN {} END",
        endpoint_bump("NEW")
    ));
    triggers
}

pub(crate) fn normalize_trigger_sql(sql: &str) -> String {
    let trimmed = sql.trim().trim_end_matches(';').trim();
    const PREFIX: &str = "CREATE TRIGGER IF NOT EXISTS";
    if trimmed
        .get(..PREFIX.len())
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(PREFIX))
    {
        format!("CREATE TRIGGER{}", &trimmed[PREFIX.len()..])
    } else {
        trimmed.to_string()
    }
}

pub(crate) fn is_known_retired_fde_trigger(sql: &str) -> bool {
    let normalized = normalize_trigger_sql(sql);
    retired_fde_resource_trigger_sql()
        .iter()
        .any(|known| normalize_trigger_sql(known) == normalized)
}

#[cfg(test)]
mod tests {
    use super::{is_known_retired_fde_trigger, retired_fde_resource_trigger_sql};
    use rusqlite::Connection;

    #[test]
    fn known_retired_trigger_count_matches_baseline() {
        assert_eq!(retired_fde_resource_trigger_sql().len(), 20);
    }

    #[test]
    fn sqlite_stored_genuine_trigger_sql_is_accepted() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE providers (id TEXT, app_type TEXT);
             CREATE TABLE provider_endpoints (provider_id TEXT, app_type TEXT);
             CREATE TABLE prompts (id TEXT, app_type TEXT);
             CREATE TABLE mcp_servers (id TEXT);
             CREATE TABLE skills (id TEXT);
             CREATE TABLE fde_resource_generations (
                kind TEXT, app_type TEXT, resource_id TEXT, generation INTEGER,
                PRIMARY KEY(kind,app_type,resource_id));",
        )
        .unwrap();
        for sql in retired_fde_resource_trigger_sql() {
            conn.execute_batch(&sql).unwrap();
        }
        let stored = conn
            .prepare("SELECT sql FROM sqlite_schema WHERE type='trigger' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(stored.len(), 20);
        for sql in stored {
            assert!(
                is_known_retired_fde_trigger(&sql),
                "stored trigger must match the closed definition: {sql}"
            );
        }
    }

    #[test]
    fn keyword_in_comment_or_literal_is_not_a_known_trigger() {
        let comment = "CREATE TRIGGER innocent AFTER INSERT ON providers BEGIN\n  -- fde_resource_provider_update\n  SELECT 1;\nEND";
        let literal = "CREATE TRIGGER innocent AFTER INSERT ON providers BEGIN INSERT INTO t VALUES('fde_resource_x'); END";
        assert!(!is_known_retired_fde_trigger(comment));
        assert!(!is_known_retired_fde_trigger(literal));
    }

    #[test]
    fn prefix_name_with_forged_body_is_not_a_known_trigger() {
        let forged = "CREATE TRIGGER IF NOT EXISTS fde_resource_provider_update AFTER UPDATE ON providers BEGIN INSERT INTO stolen(value) VALUES (NEW.settings_config); END";
        assert!(!is_known_retired_fde_trigger(forged));
        assert!(is_known_retired_fde_trigger(
            &retired_fde_resource_trigger_sql()
                .into_iter()
                .find(|sql| sql.contains("fde_resource_provider_update AFTER UPDATE"))
                .unwrap()
        ));
    }

    #[test]
    fn unicode_trigger_name_is_rejected_without_panicking() {
        let sql = "CREATE TRIGGER 客户项目备份 AFTER INSERT ON providers BEGIN SELECT 1; END";
        assert!(!is_known_retired_fde_trigger(sql));
    }
}
