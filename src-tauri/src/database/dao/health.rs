//! Read-only request evidence for the health page; no usage imports or backfill.

use crate::database::{lock_conn, Database};
use crate::error::AppError;
use rusqlite::{params, OptionalExtension};

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct HealthRequestEvidence {
    pub status_code: u16,
    pub created_at: i64,
}

impl Database {
    /// Missing per-Agent proxy configuration stays unknown. In particular this
    /// must not use the legacy getter, which seeds missing rows while reading.
    pub(crate) fn health_proxy_enabled(&self, app_type: &str) -> Result<Option<bool>, AppError> {
        if !matches!(app_type, "claude" | "codex" | "gemini" | "grokbuild") {
            return Err(AppError::Database(
                "health_proxy_unsupported_app".to_string(),
            ));
        }
        let conn = lock_conn!(self.conn);
        let enabled = conn
            .query_row(
                "SELECT enabled FROM proxy_config WHERE app_type = ?1",
                [app_type],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map_err(|_| AppError::Database("health_proxy_read_failed".to_string()))?;
        match enabled {
            None => Ok(None),
            Some(0) => Ok(Some(false)),
            Some(1) => Ok(Some(true)),
            Some(_) => Err(AppError::Database(
                "health_proxy_invalid_record".to_string(),
            )),
        }
    }

    /// An exact Agent/source match is required: session imports and other
    /// Agents cannot establish that this Agent made a real proxy request.
    pub(crate) fn latest_health_request(
        &self,
        app_type: &str,
        provider_id: Option<&str>,
    ) -> Result<Option<HealthRequestEvidence>, AppError> {
        let conn = lock_conn!(self.conn);
        let row = conn
            .query_row(
                "SELECT status_code, created_at FROM proxy_request_logs
                 WHERE app_type = ?1 AND data_source = 'proxy'
                   AND (?2 IS NULL OR provider_id = ?2)
                 ORDER BY created_at DESC, request_id DESC LIMIT 1",
                params![app_type, provider_id],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
            )
            .optional()
            .map_err(|_| AppError::Database("health_request_read_failed".to_string()))?;
        let Some((status_code, created_at)) = row else {
            return Ok(None);
        };
        // The schema has INTEGER affinity but no HTTP/time CHECK constraints.
        // Reject a corrupt newest row instead of falling back to older success.
        if !(100..=599).contains(&status_code)
            || created_at <= 0
            || created_at > chrono::Utc::now().timestamp()
        {
            return Err(AppError::Database(
                "health_request_invalid_record".to_string(),
            ));
        }
        Ok(Some(HealthRequestEvidence {
            status_code: status_code as u16,
            created_at,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn total_changes(db: &Database) -> i64 {
        db.conn
            .lock()
            .unwrap()
            .query_row("SELECT total_changes()", [], |row| row.get(0))
            .unwrap()
    }

    #[test]
    fn proxy_health_is_exact_and_never_initializes_missing_rows() {
        let db = Database::memory().unwrap();
        {
            let conn = db.conn.lock().unwrap();
            conn.execute(
                "UPDATE proxy_config SET enabled = 1 WHERE app_type = 'claude'",
                [],
            )
            .unwrap();
            conn.execute(
                "UPDATE proxy_config SET enabled = 0 WHERE app_type = 'codex'",
                [],
            )
            .unwrap();
            conn.execute(
                "UPDATE proxy_config SET enabled = 2 WHERE app_type = 'gemini'",
                [],
            )
            .unwrap();
            conn.execute("DELETE FROM proxy_config WHERE app_type = 'grokbuild'", [])
                .unwrap();
        }
        let before = total_changes(&db);
        assert_eq!(db.health_proxy_enabled("claude").unwrap(), Some(true));
        assert_eq!(db.health_proxy_enabled("codex").unwrap(), Some(false));
        assert_eq!(db.health_proxy_enabled("grokbuild").unwrap(), None);
        assert!(db.health_proxy_enabled("gemini").is_err());
        for unsupported in [
            "CLAUDE",
            "claude-desktop",
            " codex",
            "grok-build",
            "' OR 1=1 --",
        ] {
            assert!(db.health_proxy_enabled(unsupported).is_err());
        }
        assert_eq!(total_changes(&db), before);
    }

    fn insert(
        db: &Database,
        id: &str,
        app: &str,
        provider: &str,
        source: &str,
        status: i64,
        time: i64,
    ) {
        let conn = db.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO proxy_request_logs
             (request_id, provider_id, app_type, model, latency_ms, status_code, created_at, data_source, error_message)
             VALUES (?1, ?2, ?3, 'fixture-private-model', 1, ?4, ?5, ?6, 'fixture-private-error')",
            params![id, provider, app, status, time, source],
        )
        .unwrap();
    }

    #[test]
    fn request_evidence_is_exactly_scoped_and_does_not_write() {
        let db = Database::memory().unwrap();
        insert(
            &db,
            "claude-cli",
            "claude",
            "same-provider",
            "proxy",
            401,
            1_700_000_001,
        );
        insert(
            &db,
            "claude-desktop",
            "claude-desktop",
            "same-provider",
            "proxy",
            200,
            1_700_000_005,
        );
        insert(
            &db,
            "codex",
            "codex",
            "same-provider",
            "proxy",
            502,
            1_700_000_003,
        );
        insert(
            &db,
            "codex-other",
            "codex",
            "other-provider",
            "proxy",
            201,
            1_700_000_004,
        );
        for (index, source) in [
            "session_log",
            "codex_session",
            "gemini_session",
            "opencode_session",
        ]
        .iter()
        .enumerate()
        {
            insert(
                &db,
                &format!("session-{index}"),
                "codex",
                "same-provider",
                source,
                200,
                1_700_000_010 + index as i64,
            );
        }
        let before = total_changes(&db);
        assert_eq!(
            db.latest_health_request("claude", None).unwrap(),
            Some(HealthRequestEvidence {
                status_code: 401,
                created_at: 1_700_000_001
            })
        );
        assert_eq!(
            db.latest_health_request("claude-desktop", None).unwrap(),
            Some(HealthRequestEvidence {
                status_code: 200,
                created_at: 1_700_000_005
            })
        );
        assert_eq!(
            db.latest_health_request("codex", Some("same-provider"))
                .unwrap(),
            Some(HealthRequestEvidence {
                status_code: 502,
                created_at: 1_700_000_003
            })
        );
        assert_eq!(
            db.latest_health_request("codex", None).unwrap(),
            Some(HealthRequestEvidence {
                status_code: 201,
                created_at: 1_700_000_004
            })
        );
        assert_eq!(
            db.latest_health_request("codex", Some("missing")).unwrap(),
            None
        );
        assert_eq!(db.latest_health_request("CODEX", None).unwrap(), None);
        assert_eq!(db.latest_health_request("grokbuild", None).unwrap(), None);
        assert_eq!(db.latest_health_request("' OR 1=1 --", None).unwrap(), None);
        assert_eq!(total_changes(&db), before);
        let debug = format!("{:?}", db.latest_health_request("codex", None).unwrap());
        assert!(!debug.contains("fixture-private"));
        assert!(!debug.contains("provider"));
    }

    #[test]
    fn empty_and_import_only_databases_have_no_proxy_evidence() {
        let db = Database::memory().unwrap();
        assert_eq!(db.latest_health_request("codex", None).unwrap(), None);
        insert(
            &db,
            "session",
            "codex",
            "provider",
            "codex_session",
            200,
            1_700_000_000,
        );
        let before = total_changes(&db);
        assert_eq!(db.latest_health_request("codex", None).unwrap(), None);
        assert_eq!(total_changes(&db), before);
    }

    #[test]
    fn tied_timestamps_have_a_deterministic_latest_result() {
        let db = Database::memory().unwrap();
        insert(
            &db,
            "request-a",
            "codex",
            "provider",
            "proxy",
            200,
            1_700_000_000,
        );
        insert(
            &db,
            "request-z",
            "codex",
            "provider",
            "proxy",
            429,
            1_700_000_000,
        );
        assert_eq!(
            db.latest_health_request("codex", None)
                .unwrap()
                .unwrap()
                .status_code,
            429
        );
    }

    #[test]
    fn invalid_newest_status_or_time_is_an_error_without_writing() {
        for (status, time) in [
            (0, 1_700_000_000),
            (65_736, 1_700_000_000),
            (-1, 1_700_000_000),
            (600, 1_700_000_000),
            (200, 0),
            (200, -1),
            (200, i64::MAX),
        ] {
            let db = Database::memory().unwrap();
            insert(&db, "invalid", "codex", "provider", "proxy", status, time);
            let before = total_changes(&db);
            let error = db
                .latest_health_request("codex", None)
                .unwrap_err()
                .to_string();
            assert!(error.contains("health_request_invalid_record"));
            assert!(!error.contains("fixture-private"));
            assert_eq!(total_changes(&db), before);
        }
    }
}
