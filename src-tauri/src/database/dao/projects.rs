use crate::database::{lock_conn, Database};
use crate::error::AppError;
use crate::services::projects::{domain::*, project_error};
use rusqlite::{params, Connection, OptionalExtension};

fn db_error(_: rusqlite::Error) -> AppError {
    project_error("storage_unavailable")
}

impl Database {
    /// Idempotent domain schema shared by fresh creation and forward migrations.
    pub(crate) fn create_project_tables_on_conn(conn: &Connection) -> Result<(), AppError> {
        conn.execute_batch("CREATE TABLE IF NOT EXISTS fde_customers (
          customer_id TEXT PRIMARY KEY, name TEXT NOT NULL CHECK(length(name) BETWEEN 1 AND 160),
          revision INTEGER NOT NULL CHECK(revision >= 0 AND revision <= 9007199254740990),
          archived INTEGER NOT NULL CHECK(archived IN (0,1)));
        CREATE TABLE IF NOT EXISTS fde_projects (
          project_id TEXT PRIMARY KEY, customer_id TEXT NOT NULL REFERENCES fde_customers(customer_id),
          project_revision INTEGER NOT NULL CHECK(project_revision >= 0 AND project_revision <= 9007199254740990),
          archived INTEGER NOT NULL CHECK(archived IN (0,1)), document TEXT NOT NULL);
        CREATE INDEX IF NOT EXISTS idx_fde_projects_customer ON fde_projects(customer_id);
        CREATE TABLE IF NOT EXISTS fde_project_kit_intents (
          intent_id TEXT PRIMARY KEY, project_id TEXT NOT NULL REFERENCES fde_projects(project_id),
          request TEXT NOT NULL, result TEXT NOT NULL);
        CREATE TABLE IF NOT EXISTS fde_project_context_versions (
          project_id TEXT NOT NULL REFERENCES fde_projects(project_id), generation TEXT NOT NULL,
          digest TEXT NOT NULL CHECK(length(digest)=64), PRIMARY KEY(project_id,generation));")
          .map_err(db_error)?;
        Self::create_project_resource_generations_on_conn(conn)
    }

    pub(crate) fn project_resource_trigger_sql() -> Vec<String> {
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
                triggers.push(format!("CREATE TRIGGER IF NOT EXISTS fde_resource_{kind}_{} AFTER {event} ON {table} BEGIN {} END", event.to_lowercase(), bump(row)));
            }
            // Updating an identity invalidates both its old tombstone and its new row.
            let same_identity = if scoped {
                "OLD.id IS NEW.id AND OLD.app_type IS NEW.app_type"
            } else {
                "OLD.id IS NEW.id"
            };
            triggers.push(format!("CREATE TRIGGER IF NOT EXISTS fde_resource_{kind}_update AFTER UPDATE ON {table} BEGIN {} END", bump("OLD")));
            triggers.push(format!("CREATE TRIGGER IF NOT EXISTS fde_resource_{kind}_rekey AFTER UPDATE ON {table} WHEN NOT ({same_identity}) BEGIN {} END", bump("NEW")));
        }
        // Custom endpoints are part of the provider returned by its existing owner.
        let endpoint_bump = |row: &str| {
            format!("INSERT INTO fde_resource_generations(kind,app_type,resource_id,generation) VALUES('provider',{row}.app_type,{row}.provider_id,1) ON CONFLICT(kind,app_type,resource_id) DO UPDATE SET generation=generation+1;")
        };
        for (event, row) in [("INSERT", "NEW"), ("DELETE", "OLD"), ("UPDATE", "OLD")] {
            triggers.push(format!("CREATE TRIGGER IF NOT EXISTS fde_resource_provider_endpoint_{} AFTER {event} ON provider_endpoints BEGIN {} END", event.to_lowercase(), endpoint_bump(row)));
        }
        triggers.push(format!("CREATE TRIGGER IF NOT EXISTS fde_resource_provider_endpoint_rekey AFTER UPDATE ON provider_endpoints WHEN NOT (OLD.provider_id IS NEW.provider_id AND OLD.app_type IS NEW.app_type) BEGIN {} END", endpoint_bump("NEW")));
        triggers
    }

    fn create_project_resource_generations_on_conn(conn: &Connection) -> Result<(), AppError> {
        // Tombstones deliberately outlive their source row. No content or digest is stored.
        conn.execute_batch("CREATE TABLE IF NOT EXISTS fde_resource_generations (
            kind TEXT NOT NULL CHECK(kind IN ('provider','prompt','mcp','skill')),
            app_type TEXT NOT NULL,
            resource_id TEXT NOT NULL,
            generation INTEGER NOT NULL CHECK(typeof(generation)='integer' AND generation BETWEEN 1 AND 9007199254740990),
            CHECK(kind IN ('provider','prompt') OR app_type=''),
            PRIMARY KEY(kind,app_type,resource_id));").map_err(db_error)?;
        let triggers = Self::project_resource_trigger_sql();
        for (index, (kind, table, scoped)) in [
            ("provider", "providers", true),
            ("prompt", "prompts", true),
            ("mcp", "mcp_servers", false),
            ("skill", "skills", false),
        ]
        .into_iter()
        .enumerate()
        {
            let exists: bool = conn
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM sqlite_schema WHERE type='table' AND name=?1)",
                    [table],
                    |r| r.get(0),
                )
                .map_err(db_error)?;
            if !exists {
                continue;
            }
            // Startup creates tables before migrating legacy Skills identities.
            // The forward domain migration seeds these after the v3 rebuild.
            if kind == "skill"
                && Self::get_user_version(conn)? < 3
                && !Self::has_column(conn, table, "id")?
            {
                continue;
            }
            let app = if scoped { "app_type" } else { "''" };
            conn.execute_batch(&format!("INSERT OR IGNORE INTO fde_resource_generations(kind,app_type,resource_id,generation) SELECT '{kind}',{app},id,1 FROM {table};")).map_err(db_error)?;
            for trigger in &triggers[index * 4..index * 4 + 4] {
                conn.execute_batch(trigger).map_err(db_error)?;
            }
        }
        let endpoints_exist: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_schema WHERE type='table' AND name='provider_endpoints')", [], |r| r.get(0),
        ).map_err(db_error)?;
        if endpoints_exist {
            for trigger in &triggers[16..] {
                conn.execute_batch(trigger).map_err(db_error)?;
            }
        }
        Ok(())
    }

    /// A sync replacement cannot retain evidence based on the previous local rows.
    /// Call after restoring the local-only generations snapshot, within the import transaction.
    pub(crate) fn advance_project_resource_generations_on_conn(
        conn: &Connection,
    ) -> Result<(), AppError> {
        conn.execute(
            "UPDATE fde_resource_generations SET generation=generation+1",
            [],
        )
        .map_err(db_error)?;
        Self::create_project_resource_generations_on_conn(conn)
    }

    pub(crate) fn project_resource_version(
        &self,
        kind: ResourceKind,
        app: &str,
        id: &str,
    ) -> Result<Option<String>, AppError> {
        let (kind, app) = match kind {
            ResourceKind::Provider => ("provider", app),
            ResourceKind::Prompt => ("prompt", app),
            ResourceKind::Mcp => ("mcp", ""),
            ResourceKind::Skill => ("skill", ""),
            ResourceKind::Memory => return Ok(None),
        };
        let conn = lock_conn!(self.conn);
        conn.query_row("SELECT generation FROM fde_resource_generations WHERE kind=?1 AND app_type=?2 AND resource_id=?3", params![kind, app, id], |r| r.get::<_, i64>(0))
            .optional().map_err(db_error)
            .map(|value| value.map(|generation| format!("db:{generation}")))
    }

    pub(crate) fn projects_list_customers(&self) -> Result<Vec<Customer>, AppError> {
        let conn = lock_conn!(self.conn);
        let mut s = conn.prepare("SELECT customer_id,name,revision,archived FROM fde_customers ORDER BY name,customer_id").map_err(db_error)?;
        let result = s
            .query_map([], |r| {
                Ok(Customer {
                    customer_id: r.get(0)?,
                    name: r.get(1)?,
                    revision: r.get(2)?,
                    archived: r.get(3)?,
                })
            })
            .map_err(db_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(db_error);
        result
    }
    pub(crate) fn projects_save_customer(
        &self,
        c: &Customer,
        expected: Option<i64>,
    ) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        let count = if let Some(rev) = expected {
            // Archiving a customer with active projects is explicit, never cascades silently.
            if c.archived && conn.query_row("SELECT count(*) FROM fde_projects WHERE customer_id=?1 AND archived=0",[&c.customer_id],|r|r.get::<_,i64>(0)).map_err(db_error)? > 0 { return Err(project_error("active_projects")); }
            conn.execute("UPDATE fde_customers SET name=?2,revision=?3,archived=?4 WHERE customer_id=?1 AND revision=?5",params![c.customer_id,c.name,c.revision,c.archived,rev])
        } else { conn.execute("INSERT INTO fde_customers VALUES (?1,?2,?3,?4)",params![c.customer_id,c.name,c.revision,c.archived]) }.map_err(db_error)?;
        if count != 1 {
            return Err(project_error("revision_conflict"));
        }
        Ok(())
    }
    pub(crate) fn projects_list(&self) -> Result<Vec<Project>, AppError> {
        let conn = lock_conn!(self.conn);
        let mut s = conn
            .prepare("SELECT document FROM fde_projects ORDER BY project_id")
            .map_err(db_error)?;
        let docs = s
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(db_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(db_error)?;
        docs.into_iter()
            .map(|s| serde_json::from_str(&s).map_err(|_| project_error("storage_unavailable")))
            .collect()
    }
    pub(crate) fn projects_get(&self, id: &str) -> Result<Project, AppError> {
        let conn = lock_conn!(self.conn);
        Self::project_on_conn(&conn, id)
    }
    fn project_on_conn(conn: &Connection, id: &str) -> Result<Project, AppError> {
        let doc = conn
            .query_row(
                "SELECT document FROM fde_projects WHERE project_id=?1",
                [id],
                |r| r.get::<_, String>(0),
            )
            .optional()
            .map_err(db_error)?
            .ok_or_else(|| project_error("not_found"))?;
        serde_json::from_str(&doc).map_err(|_| project_error("storage_unavailable"))
    }
    pub(crate) fn projects_insert(&self, p: &Project) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        let active = conn
            .query_row(
                "SELECT archived=0 FROM fde_customers WHERE customer_id=?1",
                [&p.customer_id],
                |r| r.get::<_, bool>(0),
            )
            .optional()
            .map_err(db_error)?
            .unwrap_or(false);
        if !active {
            return Err(project_error("customer_unavailable"));
        }
        conn.execute(
            "INSERT INTO fde_projects VALUES (?1,?2,?3,?4,?5)",
            params![
                p.project_id,
                p.customer_id,
                p.project_revision,
                p.archived,
                serde_json::to_string(p).map_err(|_| project_error("invalid_request"))?
            ],
        )
        .map_err(db_error)?;
        Ok(())
    }
    pub(crate) fn projects_cas(&self, p: &Project, expected: i64) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        Self::project_cas_on_conn(&conn, p, expected)
    }
    fn project_cas_on_conn(conn: &Connection, p: &Project, expected: i64) -> Result<(), AppError> {
        if !(0..MAX_REVISION).contains(&expected) || p.project_revision != expected + 1 {
            return Err(project_error("revision_conflict"));
        }
        let n=conn.execute("UPDATE fde_projects SET project_revision=?2,archived=?3,document=?4 WHERE project_id=?1 AND project_revision=?5 AND archived=0",params![p.project_id,p.project_revision,p.archived,serde_json::to_string(p).map_err(|_|project_error("invalid_request"))?,expected]).map_err(db_error)?;
        if n != 1 {
            return Err(project_error("revision_conflict"));
        }
        Ok(())
    }
    pub(crate) fn projects_publish_context(
        &self,
        p: &Project,
        expected: i64,
        digest: &str,
    ) -> Result<(), AppError> {
        let mut conn = lock_conn!(self.conn);
        let tx = conn.transaction().map_err(db_error)?;
        Self::project_cas_on_conn(&tx, p, expected)?;
        tx.execute(
            "INSERT INTO fde_project_context_versions VALUES (?1,?2,?3)",
            params![p.project_id, p.context_generation, digest],
        )
        .map_err(db_error)?;
        tx.commit().map_err(db_error)
    }
    pub(crate) fn projects_context_digest(
        &self,
        id: &str,
        generation: &str,
    ) -> Result<String, AppError> {
        let conn = lock_conn!(self.conn);
        conn.query_row(
            "SELECT digest FROM fde_project_context_versions WHERE project_id=?1 AND generation=?2",
            params![id, generation],
            |r| r.get(0),
        )
        .map_err(db_error)
    }
    pub(crate) fn projects_bind_kit(&self, r: &BindKitRequest) -> Result<Project, AppError> {
        let mut conn = lock_conn!(self.conn);
        let tx = conn.transaction().map_err(db_error)?;
        let request = serde_json::to_string(r).map_err(|_| project_error("invalid_request"))?;
        let existing = tx
            .query_row(
                "SELECT request,result FROM fde_project_kit_intents WHERE intent_id=?1",
                [&r.binding_intent_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()
            .map_err(db_error)?;
        if let Some((prior, result)) = existing {
            if prior != request {
                return Err(project_error("intent_conflict"));
            }
            return serde_json::from_str(&result).map_err(|_| project_error("storage_unavailable"));
        }
        let mut p = Self::project_on_conn(&tx, &r.project_id)?;
        if p.archived || p.project_revision != r.expected_revision {
            return Err(project_error("revision_conflict"));
        }
        p.kit = Some(KitBinding {
            kit_id: r.kit_id.clone(),
            kit_version: r.kit_version.clone(),
            manifest_digest: r.manifest_digest.clone(),
        });
        p.project_revision += 1;
        p.updated_at = chrono::Utc::now().to_rfc3339();
        Self::project_cas_on_conn(&tx, &p, r.expected_revision)?;
        tx.execute(
            "INSERT INTO fde_project_kit_intents VALUES (?1,?2,?3,?4)",
            params![
                r.binding_intent_id,
                r.project_id,
                request,
                serde_json::to_string(&p).map_err(|_| project_error("invalid_request"))?
            ],
        )
        .map_err(db_error)?;
        tx.commit().map_err(db_error)?;
        Ok(p)
    }
}
