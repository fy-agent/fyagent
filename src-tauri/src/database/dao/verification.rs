use crate::{
    database::{lock_conn, Database},
    error::AppError,
    services::verification::{Evidence, HandoffNotes},
};
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::HashSet;

impl Database {
    /// Domain-owned additive migration; root composes this into the unique 21 -> 22 step.
    pub(crate) fn migrate_verification_v22(conn: &Connection) -> Result<(), AppError> {
        conn.execute_batch("CREATE TABLE IF NOT EXISTS verification_evidence (
            id TEXT PRIMARY KEY, project_id TEXT NOT NULL, recorded_at TEXT NOT NULL,
            payload TEXT NOT NULL CHECK(json_valid(payload)));
            CREATE INDEX IF NOT EXISTS verification_project ON verification_evidence(project_id, recorded_at);
            CREATE TABLE IF NOT EXISTS verification_revocations (
            evidence_id TEXT PRIMARY KEY REFERENCES verification_evidence(id) DEFERRABLE INITIALLY DEFERRED, revoked_at TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS verification_handoff (
            project_id TEXT PRIMARY KEY, revision INTEGER NOT NULL CHECK(revision >= 0),
            payload TEXT NOT NULL CHECK(json_valid(payload)));"
        ).map_err(|e| AppError::Database(e.to_string()))
    }

    pub(crate) fn verification_append(&self, evidence: &Evidence) -> Result<(), AppError> {
        let payload =
            serde_json::to_string(evidence).map_err(|e| AppError::Message(e.to_string()))?;
        let mut conn = lock_conn!(self.conn);
        let tx = conn
            .transaction()
            .map_err(|e| AppError::Database(e.to_string()))?;
        let count: u64 = tx
            .query_row(
                "SELECT count(*) FROM verification_evidence WHERE project_id=?1",
                [&evidence.dependencies.project_id],
                |r| r.get(0),
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        if count >= 2000 {
            return Err(AppError::Message("verification_history_full".into()));
        }
        tx.execute("INSERT INTO verification_evidence(id,project_id,recorded_at,payload) VALUES(?1,?2,?3,?4)", params![evidence.id,evidence.dependencies.project_id,evidence.recorded_at,payload]).map_err(|e| AppError::Database(e.to_string()))?;
        tx.commit().map_err(|e| AppError::Database(e.to_string()))
    }

    pub(crate) fn verification_find(&self, id: &str) -> Result<Option<Evidence>, AppError> {
        let conn = lock_conn!(self.conn);
        let payload: Option<String> = conn
            .query_row(
                "SELECT payload FROM verification_evidence WHERE id=?1",
                [id],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| AppError::Database(e.to_string()))?;
        payload
            .map(|s| {
                serde_json::from_str(&s).map_err(|_| AppError::Message("invalid_evidence".into()))
            })
            .transpose()
    }

    pub(crate) fn verification_records(
        &self,
        project: &str,
    ) -> Result<(Vec<Evidence>, HashSet<String>), AppError> {
        let conn = lock_conn!(self.conn);
        let mut query = conn.prepare("SELECT payload FROM verification_evidence WHERE project_id=?1 ORDER BY recorded_at,rowid LIMIT 2001").map_err(|e| AppError::Database(e.to_string()))?;
        let rows = query
            .query_map([project], |r| r.get::<_, String>(0))
            .map_err(|e| AppError::Database(e.to_string()))?;
        let mut result = Vec::new();
        for row in rows {
            let value = row.map_err(|e| AppError::Database(e.to_string()))?;
            result.push(
                serde_json::from_str(&value)
                    .map_err(|_| AppError::Message("invalid_evidence".into()))?,
            );
        }
        if result.len() > 2000 {
            return Err(AppError::Message("verification_history_full".into()));
        }
        let mut query = conn.prepare("SELECT r.evidence_id FROM verification_revocations r JOIN verification_evidence e ON r.evidence_id=e.id WHERE e.project_id=?1").map_err(|e| AppError::Database(e.to_string()))?;
        let revoked = query
            .query_map([project], |r| r.get::<_, String>(0))
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<HashSet<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok((result, revoked))
    }

    pub(crate) fn verification_revoke(
        &self,
        project: &str,
        id: &str,
        at: &str,
    ) -> Result<bool, AppError> {
        let conn = lock_conn!(self.conn);
        let exists = conn
            .query_row(
                "SELECT 1 FROM verification_evidence WHERE project_id=?1 AND id=?2",
                params![project, id],
                |_| Ok(()),
            )
            .optional()
            .map_err(|e| AppError::Database(e.to_string()))?
            .is_some();
        if exists {
            conn.execute(
                "INSERT OR IGNORE INTO verification_revocations VALUES(?1,?2)",
                params![id, at],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        }
        Ok(exists)
    }

    pub(crate) fn verification_handoff(
        &self,
        project: &str,
    ) -> Result<(u64, HandoffNotes), AppError> {
        let conn = lock_conn!(self.conn);
        let row: Option<(u64, String)> = conn
            .query_row(
                "SELECT revision,payload FROM verification_handoff WHERE project_id=?1",
                [project],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
            .map_err(|e| AppError::Database(e.to_string()))?;
        row.map(|(v, json)| {
            serde_json::from_str(&json)
                .map(|n| (v, n))
                .map_err(|_| AppError::Message("invalid_handoff".into()))
        })
        .unwrap_or(Ok((0, HandoffNotes::default())))
    }

    pub(crate) fn verification_save_handoff(
        &self,
        project: &str,
        revision: u64,
        notes: &HandoffNotes,
    ) -> Result<(), AppError> {
        let payload = serde_json::to_string(notes).map_err(|e| AppError::Message(e.to_string()))?;
        let mut conn = lock_conn!(self.conn);
        let tx = conn
            .transaction()
            .map_err(|e| AppError::Database(e.to_string()))?;
        tx.execute("INSERT OR IGNORE INTO verification_handoff VALUES(?1,0,'{\"items\":[],\"rollback\":null}')",[project]).map_err(|e|AppError::Database(e.to_string()))?;
        if tx.execute("UPDATE verification_handoff SET revision=revision+1,payload=?1 WHERE project_id=?2 AND revision=?3",params![payload,project,revision]).map_err(|e|AppError::Database(e.to_string()))? != 1 { return Err(AppError::Message("handoff_conflict".into())); }
        tx.commit().map_err(|e| AppError::Database(e.to_string()))
    }
}
