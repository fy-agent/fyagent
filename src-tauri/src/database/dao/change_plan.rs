use crate::change_plan::{
    enum_json, ApplyChangePlanOutcome, ChangeApplyOutcomeKind, ChangeJobSnapshot, ChangePlan,
    ChangePlanErrorCode, RestartRequirement, StoredChangePlan,
};
use crate::database::{lock_conn, Database};
use crate::error::AppError;
use rusqlite::{params, OptionalExtension};

impl Database {
    pub(crate) fn insert_change_plan(&self, plan: &StoredChangePlan) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        conn.execute(
            "INSERT INTO change_plans (
                plan_id, operation, target_provider_id, target_provider_name, plan_digest,
                baseline_digest, current_provider_id, current_provider_code,
                target_provider_code, current_definition_digest, target_definition_digest,
                live_projection_digest, contract_digest, created_at, expires_at, status
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            params![
                plan.public.plan_id,
                enum_json(plan.public.operation)?,
                plan.public.target_provider_id,
                plan.public.target_provider_name,
                plan.public.plan_digest,
                plan.public.baseline_digest,
                plan.current_provider_id,
                plan.public.current_provider_code,
                plan.public.target_provider_code,
                plan.current_definition_digest,
                plan.target_definition_digest,
                plan.live_projection_digest,
                plan.contract_digest,
                plan.public.created_at,
                plan.public.expires_at,
                enum_json(plan.public.status)?,
            ],
        )
        .map_err(|error| AppError::Database(format!("insert change plan failed: {error}")))?;
        Ok(())
    }

    pub(crate) fn get_stored_change_plan(
        &self,
        plan_id: &str,
    ) -> Result<Option<StoredChangePlan>, AppError> {
        let conn = lock_conn!(self.conn);
        conn.query_row(
            "SELECT operation, target_provider_id, target_provider_name, plan_digest,
                    baseline_digest, current_provider_id, current_provider_code,
                    target_provider_code, current_definition_digest, target_definition_digest,
                    live_projection_digest, contract_digest, created_at, expires_at, status
             FROM change_plans WHERE plan_id = ?1",
            params![plan_id],
            |row| {
                let operation = serde_json::Value::String(row.get(0)?);
                let status = serde_json::Value::String(row.get(14)?);
                Ok(StoredChangePlan {
                    public: ChangePlan {
                        plan_id: plan_id.to_string(),
                        operation: serde_json::from_value(operation)
                            .map_err(|_| rusqlite::Error::InvalidQuery)?,
                        target_provider_id: row.get(1)?,
                        target_provider_name: row.get(2)?,
                        plan_digest: row.get(3)?,
                        baseline_digest: row.get(4)?,
                        created_at: row.get(12)?,
                        expires_at: row.get(13)?,
                        status: serde_json::from_value(status)
                            .map_err(|_| rusqlite::Error::InvalidQuery)?,
                        current_provider_code: row.get(6)?,
                        target_provider_code: row.get(7)?,
                        restart_expectation: RestartRequirement::Recommended,
                        risks: vec![crate::change_plan::ChangePlanRisk {
                            code: "local_configuration_write".to_string(),
                            severity: "notice".to_string(),
                        }],
                        evidence_note: "usage_not_observed".to_string(),
                    },
                    current_provider_id: row.get(5)?,
                    current_definition_digest: row.get(8)?,
                    target_definition_digest: row.get(9)?,
                    live_projection_digest: row.get(10)?,
                    contract_digest: row.get(11)?,
                })
            },
        )
        .optional()
        .map_err(|error| AppError::Database(format!("read change plan failed: {error}")))
    }

    pub(crate) fn admit_change_plan(
        &self,
        plan_id: &str,
        plan_digest: &str,
        observed_baseline_digest: &str,
        job_id: &str,
        now: i64,
    ) -> Result<ApplyChangePlanOutcome, AppError> {
        let mut conn = lock_conn!(self.conn);
        let tx = conn
            .transaction()
            .map_err(|error| AppError::Database(format!("start admission failed: {error}")))?;
        let row = tx
            .query_row(
                "SELECT target_provider_id, plan_digest, baseline_digest, expires_at, status
                 FROM change_plans WHERE plan_id = ?1",
                params![plan_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, i64>(3)?,
                        row.get::<_, String>(4)?,
                    ))
                },
            )
            .optional()
            .map_err(|error| AppError::Database(format!("read admission plan failed: {error}")))?;

        let Some((target_id, expected_digest, baseline_digest, expires_at, status)) = row else {
            return Ok(Self::rejected_change_plan(
                ChangePlanErrorCode::PlanNotFound,
            ));
        };
        let rejection = if expected_digest != plan_digest {
            Some(ChangePlanErrorCode::InvalidDigest)
        } else if status != "ready" {
            Some(ChangePlanErrorCode::Consumed)
        } else if expires_at <= now {
            Some(ChangePlanErrorCode::Expired)
        } else if baseline_digest != observed_baseline_digest {
            Some(ChangePlanErrorCode::Stale)
        } else {
            None
        };
        if let Some(code) = rejection {
            return Ok(Self::rejected_change_plan(code));
        }

        let updated = tx
            .execute(
                "UPDATE change_plans SET status = 'consumed', consumed_at = ?2
                 WHERE plan_id = ?1 AND status = 'ready'",
                params![plan_id, now],
            )
            .map_err(|error| AppError::Database(format!("consume plan failed: {error}")))?;
        if updated != 1 {
            return Ok(Self::rejected_change_plan(ChangePlanErrorCode::Consumed));
        }

        let job =
            ChangeJobSnapshot::planned(job_id.to_string(), plan_id.to_string(), target_id, now);
        Self::insert_change_job_on_conn(&tx, &job)?;
        tx.execute(
            "INSERT INTO change_job_events (job_id, event_seq, kind, code, created_at)
             VALUES (?1, ?2, 'snapshot', 'planned', ?3)",
            params![job.job_id, job.event_seq, now],
        )
        .map_err(|error| AppError::Database(format!("insert change event failed: {error}")))?;
        tx.commit()
            .map_err(|error| AppError::Database(format!("commit admission failed: {error}")))?;
        Ok(ApplyChangePlanOutcome {
            kind: ChangeApplyOutcomeKind::Admitted,
            job: Some(job),
            error_code: None,
        })
    }

    fn rejected_change_plan(code: ChangePlanErrorCode) -> ApplyChangePlanOutcome {
        ApplyChangePlanOutcome {
            kind: ChangeApplyOutcomeKind::Rejected,
            job: None,
            error_code: Some(code),
        }
    }

    fn insert_change_job_on_conn(
        conn: &rusqlite::Connection,
        job: &ChangeJobSnapshot,
    ) -> Result<(), AppError> {
        conn.execute(
            "INSERT INTO change_jobs (
                job_id, plan_id, target_provider_id, revision, event_seq, status,
                result_code, steps_json, resources_json, restart_requirement,
                usage_evidence, recovery_state, diagnostic_code, live_config_changed,
                created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            params![
                job.job_id,
                job.plan_id,
                job.target_provider_id,
                job.revision,
                job.event_seq,
                enum_json(job.status)?,
                enum_json(job.result_code)?,
                serde_json::to_string(&job.steps).map_err(|e| AppError::Database(e.to_string()))?,
                serde_json::to_string(&job.resources)
                    .map_err(|e| AppError::Database(e.to_string()))?,
                enum_json(job.restart_requirement)?,
                enum_json(job.usage_evidence)?,
                enum_json(job.recovery_state)?,
                job.diagnostic_code,
                job.live_config_changed,
                job.created_at,
                job.updated_at,
            ],
        )
        .map_err(|error| AppError::Database(format!("insert change job failed: {error}")))?;
        Ok(())
    }

    pub(crate) fn get_change_job(
        &self,
        job_id: &str,
    ) -> Result<Option<ChangeJobSnapshot>, AppError> {
        let conn = lock_conn!(self.conn);
        Self::get_change_job_on_conn(&conn, job_id)
    }

    fn get_change_job_on_conn(
        conn: &rusqlite::Connection,
        job_id: &str,
    ) -> Result<Option<ChangeJobSnapshot>, AppError> {
        conn.query_row(
            "SELECT plan_id, target_provider_id, revision, event_seq, status, result_code,
                    steps_json, resources_json, restart_requirement, usage_evidence,
                    recovery_state, diagnostic_code, live_config_changed, created_at, updated_at
             FROM change_jobs WHERE job_id = ?1",
            params![job_id],
            |row| {
                let string_value = |text: String| serde_json::Value::String(text);
                Ok(ChangeJobSnapshot {
                    job_id: job_id.to_string(),
                    plan_id: row.get(0)?,
                    target_provider_id: row.get(1)?,
                    revision: row.get(2)?,
                    event_seq: row.get(3)?,
                    status: serde_json::from_value(string_value(row.get(4)?))
                        .map_err(|_| rusqlite::Error::InvalidQuery)?,
                    result_code: serde_json::from_value(string_value(row.get(5)?))
                        .map_err(|_| rusqlite::Error::InvalidQuery)?,
                    steps: serde_json::from_str(&row.get::<_, String>(6)?)
                        .map_err(|_| rusqlite::Error::InvalidQuery)?,
                    resources: serde_json::from_str(&row.get::<_, String>(7)?)
                        .map_err(|_| rusqlite::Error::InvalidQuery)?,
                    restart_requirement: serde_json::from_value(string_value(row.get(8)?))
                        .map_err(|_| rusqlite::Error::InvalidQuery)?,
                    usage_evidence: serde_json::from_value(string_value(row.get(9)?))
                        .map_err(|_| rusqlite::Error::InvalidQuery)?,
                    recovery_state: serde_json::from_value(string_value(row.get(10)?))
                        .map_err(|_| rusqlite::Error::InvalidQuery)?,
                    diagnostic_code: row.get(11)?,
                    live_config_changed: row.get(12)?,
                    created_at: row.get(13)?,
                    updated_at: row.get(14)?,
                })
            },
        )
        .optional()
        .map_err(|error| AppError::Database(format!("read change job failed: {error}")))
    }

    pub(crate) fn save_change_job(
        &self,
        job: &ChangeJobSnapshot,
        event_code: &str,
    ) -> Result<(), AppError> {
        let mut conn = lock_conn!(self.conn);
        let tx = conn
            .transaction()
            .map_err(|error| AppError::Database(format!("start job update failed: {error}")))?;
        tx.execute(
            "UPDATE change_jobs SET revision=?2, event_seq=?3, status=?4, result_code=?5,
                    steps_json=?6, resources_json=?7, restart_requirement=?8,
                    usage_evidence=?9, recovery_state=?10, diagnostic_code=?11,
                    live_config_changed=?12, updated_at=?13 WHERE job_id=?1",
            params![
                job.job_id,
                job.revision,
                job.event_seq,
                enum_json(job.status)?,
                enum_json(job.result_code)?,
                serde_json::to_string(&job.steps).map_err(|e| AppError::Database(e.to_string()))?,
                serde_json::to_string(&job.resources)
                    .map_err(|e| AppError::Database(e.to_string()))?,
                enum_json(job.restart_requirement)?,
                enum_json(job.usage_evidence)?,
                enum_json(job.recovery_state)?,
                job.diagnostic_code,
                job.live_config_changed,
                job.updated_at,
            ],
        )
        .map_err(|error| AppError::Database(format!("update change job failed: {error}")))?;
        tx.execute(
            "INSERT INTO change_job_events (job_id,event_seq,kind,code,created_at)
             VALUES (?1,?2,'snapshot',?3,?4)",
            params![job.job_id, job.event_seq, event_code, job.updated_at],
        )
        .map_err(|error| AppError::Database(format!("append change event failed: {error}")))?;
        tx.commit()
            .map_err(|error| AppError::Database(format!("commit job update failed: {error}")))?;
        Ok(())
    }

    pub(crate) fn list_recoverable_change_jobs(&self) -> Result<Vec<ChangeJobSnapshot>, AppError> {
        let conn = lock_conn!(self.conn);
        let mut stmt = conn
            .prepare("SELECT job_id FROM change_jobs WHERE status IN ('planned','running') ORDER BY updated_at ASC")
            .map_err(|error| AppError::Database(error.to_string()))?;
        let ids = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|error| AppError::Database(error.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| AppError::Database(error.to_string()))?;
        ids.into_iter()
            .map(|id| {
                Self::get_change_job_on_conn(&conn, &id)?.ok_or_else(|| {
                    AppError::Database("recoverable change job disappeared".to_string())
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::change_plan::{
        ChangeOperation, ChangePlanRisk, ChangePlanStatus, CHANGE_PLAN_CONTRACT_VERSION,
    };
    use crate::database::SCHEMA_VERSION;

    fn record(now: i64) -> StoredChangePlan {
        StoredChangePlan {
            public: ChangePlan {
                plan_id: "plan-1".into(),
                operation: ChangeOperation::CodexProviderSwitch,
                target_provider_id: "target".into(),
                target_provider_name: "Target".into(),
                plan_digest: "plan-digest".into(),
                baseline_digest: "baseline-digest".into(),
                created_at: now,
                expires_at: now + 900,
                status: ChangePlanStatus::Ready,
                current_provider_code: "current_configured".into(),
                target_provider_code: "existing_provider".into(),
                restart_expectation: RestartRequirement::Recommended,
                risks: vec![ChangePlanRisk {
                    code: "local_configuration_write".into(),
                    severity: "notice".into(),
                }],
                evidence_note: "usage_not_observed".into(),
            },
            current_provider_id: Some("current".into()),
            current_definition_digest: Some("current-def".into()),
            target_definition_digest: "target-def".into(),
            live_projection_digest: "live-digest".into(),
            contract_digest: CHANGE_PLAN_CONTRACT_VERSION.into(),
        }
    }

    #[test]
    fn change_plan_store_adds_tables_without_claiming_schema_v17() {
        let db = Database::memory().expect("database");
        let conn = db.conn.lock().expect("database lock");
        let version: i32 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(SCHEMA_VERSION, 16);
        assert_eq!(version, 0);
        for table in ["change_plans", "change_jobs", "change_job_events"] {
            let exists: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                    params![table],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(exists, 1, "missing {table}");
        }
    }

    #[test]
    fn change_plan_store_admission_is_atomic_and_replay_resistant() {
        let db = Database::memory().expect("database");
        db.insert_change_plan(&record(100)).unwrap();
        let admitted = db
            .admit_change_plan("plan-1", "plan-digest", "baseline-digest", "job-1", 101)
            .unwrap();
        assert_eq!(admitted.kind, ChangeApplyOutcomeKind::Admitted);
        assert_eq!(db.list_recoverable_change_jobs().unwrap().len(), 1);
        let replay = db
            .admit_change_plan("plan-1", "plan-digest", "baseline-digest", "job-2", 102)
            .unwrap();
        assert_eq!(replay.error_code, Some(ChangePlanErrorCode::Consumed));
        assert!(db.get_change_job("job-2").unwrap().is_none());
    }

    #[test]
    fn change_plan_store_rejections_create_no_job() {
        for (digest, baseline, now, expected) in [
            (
                "wrong",
                "baseline-digest",
                101,
                ChangePlanErrorCode::InvalidDigest,
            ),
            ("plan-digest", "drifted", 101, ChangePlanErrorCode::Stale),
            (
                "plan-digest",
                "baseline-digest",
                1000,
                ChangePlanErrorCode::Expired,
            ),
        ] {
            let db = Database::memory().unwrap();
            db.insert_change_plan(&record(100)).unwrap();
            let outcome = db
                .admit_change_plan("plan-1", digest, baseline, "rejected", now)
                .unwrap();
            assert_eq!(outcome.error_code, Some(expected));
            assert!(db.get_change_job("rejected").unwrap().is_none());
        }
    }
}
