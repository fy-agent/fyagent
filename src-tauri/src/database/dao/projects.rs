use crate::database::{lock_conn, Database};
use crate::error::AppError;
use crate::services::projects::{domain::*, project_error};
use rusqlite::{params, Connection, OptionalExtension};

fn db_error(_: rusqlite::Error) -> AppError {
    project_error("storage_unavailable")
}

impl Database {
    /// Additive domain migration; root composes this into the single 21 -> 22 step.
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
          .map_err(db_error)
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
