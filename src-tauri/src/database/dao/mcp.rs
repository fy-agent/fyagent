//! MCP 服务器数据访问对象
//!
//! 提供 MCP 服务器的 CRUD 操作。

use crate::app_config::{AppType, McpApps, McpServer, McpTargetId};
use crate::database::{lock_conn, Database};
use crate::error::AppError;
use indexmap::IndexMap;
use rusqlite::{params, Connection, OptionalExtension, Row};

const MCP_SERVER_SELECT: &str =
    "SELECT id, name, server_config, description, homepage, docs, tags, enabled_claude, enabled_codex, enabled_gemini, enabled_grokbuild, enabled_opencode, enabled_hermes, enabled_workbuddy, enabled_qoderwork, enabled_trae_work FROM mcp_servers";
const MCP_SERVER_UPSERT: &str = "INSERT OR REPLACE INTO mcp_servers (
    id, name, server_config, description, homepage, docs, tags,
    enabled_claude, enabled_codex, enabled_gemini, enabled_grokbuild, enabled_opencode, enabled_hermes, enabled_workbuddy, enabled_qoderwork, enabled_trae_work
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)";

fn mcp_target_column(target: &McpTargetId) -> &'static str {
    match target {
        McpTargetId::Claude => "enabled_claude",
        McpTargetId::Codex => "enabled_codex",
        McpTargetId::Gemini => "enabled_gemini",
        McpTargetId::GrokBuild => "enabled_grokbuild",
        McpTargetId::OpenCode => "enabled_opencode",
        McpTargetId::Hermes => "enabled_hermes",
        McpTargetId::WorkBuddy => "enabled_workbuddy",
        McpTargetId::QoderWork => "enabled_qoderwork",
        McpTargetId::TraeWork => "enabled_trae_work",
    }
}

fn save_mcp_server_on(conn: &Connection, server: &McpServer) -> Result<(), AppError> {
    let server_config = serde_json::to_string(&server.server)
        .map_err(|e| AppError::Database(format!("Failed to serialize server config: {e}")))?;
    let tags = serde_json::to_string(&server.tags)
        .map_err(|e| AppError::Database(format!("Failed to serialize tags: {e}")))?;

    conn.execute(
        MCP_SERVER_UPSERT,
        params![
            server.id,
            server.name,
            server_config,
            server.description,
            server.homepage,
            server.docs,
            tags,
            server.apps.claude,
            server.apps.codex,
            server.apps.gemini,
            server.apps.grokbuild,
            server.apps.opencode,
            server.apps.hermes,
            server.apps.workbuddy,
            server.apps.qoderwork,
            server.apps.trae_work,
        ],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;
    Ok(())
}

fn row_to_mcp_server(row: &Row<'_>) -> rusqlite::Result<(String, McpServer)> {
    let id: String = row.get(0)?;
    let name: String = row.get(1)?;
    let server_config_str: String = row.get(2)?;
    let description: Option<String> = row.get(3)?;
    let homepage: Option<String> = row.get(4)?;
    let docs: Option<String> = row.get(5)?;
    let tags_str: String = row.get(6)?;
    let enabled_claude: bool = row.get(7)?;
    let enabled_codex: bool = row.get(8)?;
    let enabled_gemini: bool = row.get(9)?;
    let enabled_grokbuild: bool = row.get(10)?;
    let enabled_opencode: bool = row.get(11)?;
    let enabled_hermes: bool = row.get(12)?;
    let enabled_workbuddy: bool = row.get(13)?;
    let enabled_qoderwork: bool = row.get(14)?;
    let enabled_trae_work: bool = row.get(15)?;

    let server = serde_json::from_str(&server_config_str).unwrap_or_default();
    let tags = serde_json::from_str(&tags_str).unwrap_or_default();

    Ok((
        id.clone(),
        McpServer {
            id,
            name,
            server,
            apps: McpApps {
                claude: enabled_claude,
                codex: enabled_codex,
                gemini: enabled_gemini,
                grokbuild: enabled_grokbuild,
                opencode: enabled_opencode,
                hermes: enabled_hermes,
                workbuddy: enabled_workbuddy,
                qoderwork: enabled_qoderwork,
                trae_work: enabled_trae_work,
            },
            description,
            homepage,
            docs,
            tags,
        },
    ))
}

impl Database {
    /// 获取所有 MCP 服务器
    pub fn get_all_mcp_servers(&self) -> Result<IndexMap<String, McpServer>, AppError> {
        let conn = lock_conn!(self.conn);
        let mut stmt = conn
            .prepare(&format!("{MCP_SERVER_SELECT} ORDER BY name ASC, id ASC"))
            .map_err(|e| AppError::Database(e.to_string()))?;

        let server_iter = stmt
            .query_map([], row_to_mcp_server)
            .map_err(|e| AppError::Database(e.to_string()))?;

        let mut servers = IndexMap::new();
        for server_res in server_iter {
            let (id, server) = server_res.map_err(|e| AppError::Database(e.to_string()))?;
            servers.insert(id, server);
        }
        Ok(servers)
    }

    /// Atomically update one application's flag and return the authoritative row.
    ///
    /// The update and read share the same connection lock, so concurrent toggles
    /// for different applications cannot overwrite one another through a stale
    /// whole-row snapshot.
    pub fn update_mcp_server_app_enabled(
        &self,
        id: &str,
        app: &AppType,
        enabled: bool,
    ) -> Result<Option<McpServer>, AppError> {
        match McpTargetId::try_from(app) {
            Ok(target) => self.update_mcp_server_target_enabled(id, &target, enabled),
            Err(_) => {
                let conn = lock_conn!(self.conn);
                conn.query_row(
                    &format!("{MCP_SERVER_SELECT} WHERE id = ?1"),
                    params![id],
                    |row| row_to_mcp_server(row).map(|(_, server)| server),
                )
                .optional()
                .map_err(|e| AppError::Database(e.to_string()))
            }
        }
    }

    pub fn update_mcp_server_target_enabled(
        &self,
        id: &str,
        target: &McpTargetId,
        enabled: bool,
    ) -> Result<Option<McpServer>, AppError> {
        let conn = lock_conn!(self.conn);
        let column = mcp_target_column(target);
        let sql = format!("UPDATE mcp_servers SET {column} = ?1 WHERE id = ?2");
        let affected = conn
            .execute(&sql, params![enabled, id])
            .map_err(|e| AppError::Database(e.to_string()))?;
        if affected == 0 {
            return Ok(None);
        }

        conn.query_row(
            &format!("{MCP_SERVER_SELECT} WHERE id = ?1"),
            params![id],
            |row| row_to_mcp_server(row).map(|(_, server)| server),
        )
        .optional()
        .map_err(|e| AppError::Database(e.to_string()))
    }

    /// 保存 MCP 服务器
    pub fn save_mcp_server(&self, server: &McpServer) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        save_mcp_server_on(&conn, server)
    }

    /// Import one application's complete MCP snapshot in one transaction.
    /// Existing equivalent rows keep their metadata and receive only the app
    /// flag; conflicting specs abort before any row is changed.
    pub fn import_mcp_servers_atomically(
        &self,
        servers: &[McpServer],
        app: &AppType,
    ) -> Result<usize, AppError> {
        let target = McpTargetId::try_from(app)
            .map_err(|_| AppError::McpValidation(format!("{} 不支持 MCP 分配", app.as_str())))?;
        self.import_mcp_servers_atomically_for_target(servers, &target)
    }

    pub fn import_mcp_servers_atomically_for_target(
        &self,
        servers: &[McpServer],
        target: &McpTargetId,
    ) -> Result<usize, AppError> {
        let column = mcp_target_column(target);
        let mut conn = lock_conn!(self.conn);
        let transaction = conn
            .transaction()
            .map_err(|e| AppError::Database(e.to_string()))?;
        let mut ordered = servers.iter().collect::<Vec<_>>();
        ordered.sort_by(|left, right| left.id.cmp(&right.id));
        let mut existing_updates = Vec::new();
        let mut new_servers = Vec::new();

        for server in ordered {
            let source_enabled = server.apps.is_enabled_for_target(target);
            let existing = transaction
                .query_row(
                    &format!("{MCP_SERVER_SELECT} WHERE id = ?1"),
                    params![server.id],
                    |row| row_to_mcp_server(row).map(|(_, server)| server),
                )
                .optional()
                .map_err(|e| AppError::Database(e.to_string()))?;

            if let Some(existing) = existing {
                if source_enabled
                    && !crate::mcp::server_specs_are_equivalent(&existing.server, &server.server)
                {
                    return Err(AppError::McpValidation(format!(
                        "MCP 服务器 '{}' 在多个应用中的配置冲突；未合并 {} 分配",
                        server.id,
                        target.as_str()
                    )));
                }
                existing_updates.push((server.id.clone(), source_enabled));
            } else if source_enabled {
                let mut imported = server.clone();
                imported.apps.set_enabled_for_target(target, true);
                new_servers.push(imported);
            }
        }

        for (id, enabled) in existing_updates {
            transaction
                .execute(
                    &format!("UPDATE mcp_servers SET {column} = ?1 WHERE id = ?2"),
                    params![enabled, id],
                )
                .map_err(|e| AppError::Database(e.to_string()))?;
        }
        for server in &new_servers {
            save_mcp_server_on(&transaction, server)?;
        }

        let new_count = new_servers.len();
        transaction
            .commit()
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(new_count)
    }

    /// 删除 MCP 服务器
    pub fn delete_mcp_server(&self, id: &str) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        conn.execute("DELETE FROM mcp_servers WHERE id = ?1", params![id])
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::{Arc, Barrier};
    use std::thread;

    fn test_server() -> McpServer {
        McpServer {
            id: "shared-server".to_string(),
            name: "Shared Server".to_string(),
            server: json!({ "command": "echo", "args": ["hello"] }),
            apps: McpApps {
                gemini: true,
                ..McpApps::default()
            },
            description: Some("description".to_string()),
            homepage: Some("https://example.com".to_string()),
            docs: None,
            tags: vec!["shared".to_string()],
        }
    }

    #[test]
    fn app_flag_updates_preserve_other_flags_and_return_authoritative_row() {
        let db = Database::memory().expect("create memory db");
        db.save_mcp_server(&test_server()).expect("seed server");

        let after_claude = db
            .update_mcp_server_app_enabled("shared-server", &AppType::Claude, true)
            .expect("enable Claude")
            .expect("server exists");
        assert!(after_claude.apps.claude);
        assert!(after_claude.apps.gemini);

        let after_codex = db
            .update_mcp_server_app_enabled("shared-server", &AppType::Codex, true)
            .expect("enable Codex")
            .expect("server exists");
        assert!(after_codex.apps.claude);
        assert!(after_codex.apps.codex);
        assert!(after_codex.apps.gemini);
        assert_eq!(after_codex.description.as_deref(), Some("description"));
        assert_eq!(after_codex.tags, vec!["shared"]);

        let stored = db
            .get_all_mcp_servers()
            .expect("read servers")
            .shift_remove("shared-server")
            .expect("stored server");
        assert_eq!(stored.apps, after_codex.apps);
    }

    #[test]
    fn concurrent_app_flag_updates_do_not_lose_each_other() {
        let db = Arc::new(Database::memory().expect("create memory db"));
        db.save_mcp_server(&test_server()).expect("seed server");
        let barrier = Arc::new(Barrier::new(3));

        let handles = [AppType::Claude, AppType::Codex].map(|app| {
            let db = Arc::clone(&db);
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                barrier.wait();
                db.update_mcp_server_app_enabled("shared-server", &app, true)
                    .expect("update app flag")
                    .expect("server exists");
            })
        });

        barrier.wait();
        for handle in handles {
            handle.join().expect("join app toggle");
        }

        let stored = db
            .get_all_mcp_servers()
            .expect("read servers")
            .shift_remove("shared-server")
            .expect("stored server");
        assert!(stored.apps.claude);
        assert!(stored.apps.codex);
        assert!(stored.apps.gemini);
    }

    #[test]
    fn app_flag_update_does_not_insert_a_missing_server() {
        let db = Database::memory().expect("create memory db");

        let updated = db
            .update_mcp_server_app_enabled("missing", &AppType::Claude, true)
            .expect("update missing server");

        assert!(updated.is_none());
        assert!(db.get_all_mcp_servers().expect("read servers").is_empty());
    }

    #[test]
    fn unsupported_mcp_apps_keep_the_existing_noop_semantics() {
        let db = Database::memory().expect("create memory db");
        let original = test_server();
        db.save_mcp_server(&original).expect("seed server");

        for app in [AppType::ClaudeDesktop, AppType::OpenClaw] {
            let returned = db
                .update_mcp_server_app_enabled("shared-server", &app, true)
                .expect("toggle unsupported app")
                .expect("server exists");
            assert_eq!(returned.apps, original.apps);
        }
    }

    #[test]
    fn imported_mcp_batch_rolls_back_app_flags_when_a_later_insert_fails() {
        let db = Database::memory().expect("create memory db");
        let existing = test_server();
        db.save_mcp_server(&existing).expect("seed server");

        {
            let conn = db.conn.lock().expect("lock database");
            conn.execute_batch(
                "CREATE TRIGGER reject_second_mcp_import
                 BEFORE INSERT ON mcp_servers
                 WHEN NEW.id = 'zeta'
                 BEGIN
                   SELECT RAISE(ABORT, 'forced MCP import failure');
                 END;",
            )
            .expect("install failure trigger");
        }

        let mut equivalent = existing.clone();
        equivalent.apps.claude = true;
        equivalent.server = json!({
            "type": "stdio",
            "command": "echo",
            "args": ["hello"],
            "env": {},
            "cwd": ""
        });
        let mut rejected = existing.clone();
        rejected.id = "zeta".to_string();
        rejected.name = "Zeta".to_string();
        rejected.apps.claude = true;

        db.import_mcp_servers_atomically(&[equivalent, rejected], &AppType::Claude)
            .expect_err("second insert must fail the complete batch");

        let stored = db.get_all_mcp_servers().expect("read servers");
        let existing = stored.get("shared-server").expect("existing row remains");
        assert!(!existing.apps.claude, "earlier app update must roll back");
        assert!(existing.apps.gemini, "unrelated app flag must remain");
        assert!(
            !stored.contains_key("zeta"),
            "failed insert must not persist"
        );
    }

    #[test]
    fn disabled_source_mcp_clears_existing_assignment_without_inserting_a_new_row() {
        let db = Database::memory().expect("create memory db");
        let mut existing = test_server();
        existing.apps.opencode = true;
        db.save_mcp_server(&existing).expect("seed existing server");

        let mut disabled_existing = existing.clone();
        disabled_existing.apps.opencode = false;
        let mut disabled_new = existing.clone();
        disabled_new.id = "disabled-new".to_string();
        disabled_new.name = "Disabled New".to_string();
        disabled_new.apps.opencode = false;

        let new_count = db
            .import_mcp_servers_atomically(&[disabled_existing, disabled_new], &AppType::OpenCode)
            .expect("import disabled source entries");

        assert_eq!(new_count, 0);
        let stored = db.get_all_mcp_servers().expect("read servers");
        assert!(
            !stored
                .get("shared-server")
                .expect("existing row remains")
                .apps
                .opencode,
            "explicit source disablement must clear the existing assignment"
        );
        assert!(
            !stored.contains_key("disabled-new"),
            "an explicitly disabled source command must not become a new managed row"
        );
    }

    #[test]
    fn workbuddy_mcp_flag_round_trips_without_an_app_type() {
        let db = Database::memory().expect("create memory db");
        let mut server = test_server();
        server.apps.workbuddy = true;
        db.save_mcp_server(&server).expect("seed server");

        let stored = db
            .get_all_mcp_servers()
            .expect("read servers")
            .shift_remove("shared-server")
            .expect("stored server");
        assert!(stored.apps.workbuddy);
        assert!(stored.apps.gemini);

        let after_disable = db
            .update_mcp_server_target_enabled("shared-server", &McpTargetId::WorkBuddy, false)
            .expect("disable workbuddy")
            .expect("server exists");
        assert!(!after_disable.apps.workbuddy);
        assert!(after_disable.apps.gemini);
    }

    #[test]
    fn qoderwork_and_trae_work_mcp_flags_round_trip_without_an_app_type() {
        let db = Database::memory().expect("create memory db");
        let mut server = test_server();
        server.apps.qoderwork = true;
        server.apps.trae_work = true;
        db.save_mcp_server(&server).expect("seed server");

        let stored = db
            .get_all_mcp_servers()
            .expect("read servers")
            .shift_remove("shared-server")
            .expect("stored server");
        assert!(stored.apps.qoderwork);
        assert!(stored.apps.trae_work);
        assert!(stored.apps.gemini);

        let after_disable = db
            .update_mcp_server_target_enabled("shared-server", &McpTargetId::QoderWork, false)
            .expect("disable qoderwork")
            .expect("server exists");
        assert!(!after_disable.apps.qoderwork);
        assert!(after_disable.apps.trae_work);
        assert!(after_disable.apps.gemini);
    }
}
