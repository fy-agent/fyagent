use crate::database::{lock_conn, Database};
use crate::error::AppError;
use crate::services::change_plan::{
    descriptor_for_operation, enum_json, ApplyChangePlanOutcome, ChangeApplyOutcomeKind,
    ChangeJobEvent, ChangeJobSnapshot, ChangeJobStatus, ChangeJobStep, ChangeOperation, ChangePlan,
    ChangePlanErrorCode, ChangePlanRisk, ChangeResultCode, ChangeStepKind, ChangeStepStatus,
    RestartRequirement, StoredChangePlan,
};
use rusqlite::{params, Connection, OptionalExtension};

impl Database {
    pub(crate) fn insert_change_plan(&self, plan: &StoredChangePlan) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        conn.execute(
            "INSERT INTO change_plans (
                plan_id, operation, target_provider_id, target_provider_name, plan_digest,
                baseline_digest, db_baseline_provider_id, device_baseline_provider_id,
                target_definition_digest, live_baseline_digest, target_projection_digest,
                contract_digest, secret_capability, created_at, expires_at, status
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            params![
                plan.public.plan_id,
                enum_json(plan.public.operation)?,
                plan.public.target_provider_id,
                plan.public.target_provider_name,
                plan.public.plan_digest,
                plan.public.baseline_digest,
                plan.public.db_baseline_provider_id,
                plan.public.device_baseline_provider_id,
                plan.target_definition_digest,
                plan.live_baseline_digest,
                plan.target_projection_digest,
                plan.contract_digest,
                enum_json(plan.public.secret_capability)?,
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
                    baseline_digest, db_baseline_provider_id, device_baseline_provider_id,
                    target_definition_digest, live_baseline_digest, target_projection_digest,
                    contract_digest, secret_capability, created_at, expires_at, status
             FROM change_plans WHERE plan_id = ?1",
            params![plan_id],
            |row| {
                let operation: ChangeOperation = parse_enum(row.get(0)?)?;
                Ok(StoredChangePlan {
                    public: ChangePlan {
                        plan_id: plan_id.to_string(),
                        operation,
                        target_provider_id: row.get(1)?,
                        target_provider_name: row.get(2)?,
                        plan_digest: row.get(3)?,
                        baseline_digest: row.get(4)?,
                        db_baseline_provider_id: row.get(5)?,
                        device_baseline_provider_id: row.get(6)?,
                        secret_capability: parse_enum(row.get(11)?)?,
                        created_at: row.get(12)?,
                        expires_at: row.get(13)?,
                        status: parse_enum(row.get(14)?)?,
                        adapter: descriptor_for_operation(operation),
                        current_provider_code: current_provider_code(
                            &row.get::<_, Option<String>>(5)?,
                            &row.get::<_, Option<String>>(6)?,
                        ),
                        target_provider_code: "existing_provider".to_string(),
                        restart_expectation: RestartRequirement::Recommended,
                        risks: vec![ChangePlanRisk {
                            code: "local_configuration_write".to_string(),
                            severity: "notice".to_string(),
                        }],
                        evidence_note: "usage_not_observed".to_string(),
                    },
                    target_definition_digest: row.get(7)?,
                    live_baseline_digest: row.get(8)?,
                    target_projection_digest: row.get(9)?,
                    contract_digest: row.get(10)?,
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
                "SELECT target_provider_id, plan_digest, baseline_digest, expires_at,
                        status, secret_capability
                 FROM change_plans WHERE plan_id = ?1",
                params![plan_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, i64>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                    ))
                },
            )
            .optional()
            .map_err(|error| AppError::Database(format!("read admission plan failed: {error}")))?;

        let Some((target_id, expected_digest, baseline_digest, expires_at, status, secret)) = row
        else {
            return Ok(ApplyChangePlanOutcome::rejected(
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
        } else if secret != "no_new_credential_material" {
            Some(ChangePlanErrorCode::SecretDependencyUnavailable)
        } else {
            None
        };
        if let Some(code) = rejection {
            return Ok(ApplyChangePlanOutcome::rejected(code));
        }

        let updated = tx
            .execute(
                "UPDATE change_plans SET status = 'consumed', consumed_at = ?2
                 WHERE plan_id = ?1 AND status = 'ready'",
                params![plan_id, now],
            )
            .map_err(|error| AppError::Database(format!("consume plan failed: {error}")))?;
        if updated != 1 {
            return Ok(ApplyChangePlanOutcome::rejected(
                ChangePlanErrorCode::Consumed,
            ));
        }

        let job =
            ChangeJobSnapshot::planned(job_id.to_string(), plan_id.to_string(), target_id, now);
        Self::insert_change_job_on_conn(&tx, &job)?;
        let event = job
            .events
            .first()
            .ok_or_else(|| AppError::Database("planned change event missing".to_string()))?;
        Self::insert_change_job_event_on_conn(&tx, &job.job_id, event)?;
        tx.commit()
            .map_err(|error| AppError::Database(format!("commit admission failed: {error}")))?;
        Ok(ApplyChangePlanOutcome {
            kind: ChangeApplyOutcomeKind::Admitted,
            job: Some(job),
            error_code: None,
        })
    }

    fn insert_change_job_on_conn(
        conn: &Connection,
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
                enum_json(persisted_job_status(job.status))?,
                enum_json(job.result_code)?,
                serde_json::to_string(&job.steps)
                    .map_err(|error| AppError::Database(error.to_string()))?,
                serde_json::to_string(&job.resources)
                    .map_err(|error| AppError::Database(error.to_string()))?,
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

    pub(crate) fn get_change_job_by_plan_id(
        &self,
        plan_id: &str,
    ) -> Result<Option<ChangeJobSnapshot>, AppError> {
        let conn = lock_conn!(self.conn);
        let job_id = conn
            .query_row(
                "SELECT job_id FROM change_jobs WHERE plan_id = ?1",
                params![plan_id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|error| AppError::Database(format!("read change job id failed: {error}")))?;
        job_id
            .as_deref()
            .map(|job_id| Self::get_change_job_on_conn(&conn, job_id))
            .transpose()
            .map(Option::flatten)
    }

    fn get_change_job_on_conn(
        conn: &Connection,
        job_id: &str,
    ) -> Result<Option<ChangeJobSnapshot>, AppError> {
        let job = conn
            .query_row(
                "SELECT plan_id, target_provider_id, revision, event_seq, status, result_code,
                        steps_json, resources_json, restart_requirement, usage_evidence,
                        recovery_state, diagnostic_code, live_config_changed, created_at, updated_at
                 FROM change_jobs WHERE job_id = ?1",
                params![job_id],
                |row| {
                    let result_code: ChangeResultCode = parse_enum(row.get(5)?)?;
                    let stored_status: ChangeJobStatus = parse_enum(row.get(4)?)?;
                    let steps: Vec<ChangeJobStep> = serde_json::from_str(&row.get::<_, String>(6)?)
                        .map_err(|_| rusqlite::Error::InvalidQuery)?;
                    Ok(ChangeJobSnapshot {
                        job_id: job_id.to_string(),
                        execution_id: job_id.to_string(),
                        plan_id: row.get(0)?,
                        idempotency_key: row.get(0)?,
                        target_provider_id: row.get(1)?,
                        revision: row.get(2)?,
                        event_seq: row.get(3)?,
                        status: public_job_status(stored_status, result_code),
                        result_code,
                        adapter_error_code: None,
                        steps: normalize_job_steps(steps),
                        resources: serde_json::from_str(&row.get::<_, String>(7)?)
                            .map_err(|_| rusqlite::Error::InvalidQuery)?,
                        partial_result: None,
                        events: Vec::new(),
                        restart_requirement: parse_enum(row.get(8)?)?,
                        usage_evidence: parse_enum(row.get(9)?)?,
                        recovery_state: parse_enum(row.get(10)?)?,
                        diagnostic_code: row.get(11)?,
                        live_config_changed: row.get(12)?,
                        created_at: row.get(13)?,
                        updated_at: row.get(14)?,
                    })
                },
            )
            .optional()
            .map_err(|error| AppError::Database(format!("read change job failed: {error}")))?;
        let Some(mut job) = job else {
            return Ok(None);
        };
        job.events = Self::list_change_job_events_on_conn(conn, job_id)?;
        normalize_job_events(&mut job.events);
        Ok(Some(job))
    }

    pub(crate) fn save_change_job(
        &self,
        job: &ChangeJobSnapshot,
        event: &ChangeJobEvent,
    ) -> Result<(), AppError> {
        if event.sequence != job.event_seq {
            return Err(AppError::Database(
                "change event sequence does not match snapshot".to_string(),
            ));
        }
        let mut conn = lock_conn!(self.conn);
        let tx = conn
            .transaction()
            .map_err(|error| AppError::Database(format!("start job update failed: {error}")))?;
        let updated = tx
            .execute(
                "UPDATE change_jobs SET revision=?2, event_seq=?3, status=?4, result_code=?5,
                        steps_json=?6, resources_json=?7, restart_requirement=?8,
                        usage_evidence=?9, recovery_state=?10, diagnostic_code=?11,
                        live_config_changed=?12, updated_at=?13
                 WHERE job_id=?1 AND revision < ?2 AND event_seq < ?3",
                params![
                    job.job_id,
                    job.revision,
                    job.event_seq,
                    enum_json(persisted_job_status(job.status))?,
                    enum_json(job.result_code)?,
                    serde_json::to_string(&job.steps)
                        .map_err(|error| AppError::Database(error.to_string()))?,
                    serde_json::to_string(&job.resources)
                        .map_err(|error| AppError::Database(error.to_string()))?,
                    enum_json(job.restart_requirement)?,
                    enum_json(job.usage_evidence)?,
                    enum_json(job.recovery_state)?,
                    job.diagnostic_code,
                    job.live_config_changed,
                    job.updated_at,
                ],
            )
            .map_err(|error| AppError::Database(format!("update change job failed: {error}")))?;
        if updated != 1 {
            return Err(AppError::Database(
                "stale change job snapshot rejected".to_string(),
            ));
        }
        Self::insert_change_job_event_on_conn(&tx, &job.job_id, event)?;
        tx.commit()
            .map_err(|error| AppError::Database(format!("commit job update failed: {error}")))?;
        Ok(())
    }

    fn insert_change_job_event_on_conn(
        conn: &Connection,
        job_id: &str,
        event: &ChangeJobEvent,
    ) -> Result<(), AppError> {
        conn.execute(
            "INSERT INTO change_job_events (job_id, event_seq, phase, reason_code, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                job_id,
                event.sequence,
                enum_json(event.phase)?,
                event.reason_code,
                event.created_at,
            ],
        )
        .map_err(|error| AppError::Database(format!("append change event failed: {error}")))?;
        Ok(())
    }

    fn list_change_job_events_on_conn(
        conn: &Connection,
        job_id: &str,
    ) -> Result<Vec<ChangeJobEvent>, AppError> {
        let mut stmt = conn
            .prepare(
                "SELECT event_seq, phase, reason_code, created_at
                 FROM change_job_events WHERE job_id=?1 ORDER BY event_seq ASC",
            )
            .map_err(|error| AppError::Database(error.to_string()))?;
        let events = stmt
            .query_map(params![job_id], |row| {
                Ok(ChangeJobEvent {
                    sequence: row.get(0)?,
                    phase: parse_enum(row.get(1)?)?,
                    reason_code: row.get(2)?,
                    created_at: row.get(3)?,
                })
            })
            .map_err(|error| AppError::Database(error.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| AppError::Database(error.to_string()))?;
        Ok(events)
    }

    pub(crate) fn list_recoverable_change_jobs(&self) -> Result<Vec<ChangeJobSnapshot>, AppError> {
        let conn = lock_conn!(self.conn);
        let ids = {
            let mut stmt = conn
                .prepare(
                    "SELECT job_id FROM change_jobs
                     WHERE status IN ('planned','running')
                        OR recovery_state = 'recovery_required'
                     ORDER BY updated_at ASC, job_id ASC",
                )
                .map_err(|error| AppError::Database(error.to_string()))?;
            let ids = stmt
                .query_map([], |row| row.get::<_, String>(0))
                .map_err(|error| AppError::Database(error.to_string()))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| AppError::Database(error.to_string()))?;
            ids
        };
        ids.into_iter()
            .map(|id| {
                Self::get_change_job_on_conn(&conn, &id)?.ok_or_else(|| {
                    AppError::Database("recoverable change job disappeared".to_string())
                })
            })
            .collect()
    }

    #[cfg(test)]
    pub(crate) fn create_change_plan_tables_for_tests(&self) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        Self::create_change_plan_tables_on_conn(&conn)
    }
}

fn parse_enum<T: serde::de::DeserializeOwned>(value: String) -> Result<T, rusqlite::Error> {
    serde_json::from_value(serde_json::Value::String(value))
        .map_err(|_| rusqlite::Error::InvalidQuery)
}

fn current_provider_code(db: &Option<String>, device: &Option<String>) -> String {
    match (db, device) {
        (None, None) => "current_unconfigured",
        (Some(db), Some(device)) if db == device => "current_configured",
        _ => "current_mixed",
    }
    .to_string()
}

fn persisted_job_status(status: ChangeJobStatus) -> ChangeJobStatus {
    match status {
        // Schema v20 intentionally stays immutable. Public cancellation is
        // represented durably by result_code=cancelled_before_write while the
        // coarse SQLite status remains a v20-legal terminal value.
        ChangeJobStatus::Cancelled => ChangeJobStatus::Failed,
        other => other,
    }
}

fn public_job_status(
    stored_status: ChangeJobStatus,
    result_code: ChangeResultCode,
) -> ChangeJobStatus {
    if result_code == ChangeResultCode::CancelledBeforeWrite {
        ChangeJobStatus::Cancelled
    } else {
        stored_status
    }
}

fn normalize_job_steps(steps: Vec<ChangeJobStep>) -> Vec<ChangeJobStep> {
    let mut normalized = steps
        .into_iter()
        .map(|mut step| {
            step.kind = match step.kind {
                ChangeStepKind::Apply => ChangeStepKind::ManagedWrite,
                ChangeStepKind::Reconcile => ChangeStepKind::Finalize,
                other => other,
            };
            step
        })
        .collect::<Vec<_>>();

    if !normalized
        .iter()
        .any(|step| step.kind == ChangeStepKind::Snapshot)
    {
        normalized.push(ChangeJobStep {
            kind: ChangeStepKind::Snapshot,
            status: ChangeStepStatus::Skipped,
            code: "legacy_not_recorded".to_string(),
        });
    }

    let order = [
        ChangeStepKind::Precheck,
        ChangeStepKind::Snapshot,
        ChangeStepKind::ManagedWrite,
        ChangeStepKind::Readback,
        ChangeStepKind::Finalize,
    ];
    normalized.sort_by_key(|step| {
        order
            .iter()
            .position(|kind| *kind == step.kind)
            .unwrap_or(order.len())
    });
    normalized.dedup_by_key(|step| step.kind);
    normalized
}

fn normalize_job_events(events: &mut [ChangeJobEvent]) {
    for event in events {
        event.phase = match event.phase {
            ChangeStepKind::Apply => ChangeStepKind::ManagedWrite,
            ChangeStepKind::Reconcile => ChangeStepKind::Finalize,
            other => other,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::change_plan::{
        ChangeOperation, ChangePlanStatus, SecretCapabilityResult, CHANGE_PLAN_CONTRACT_VERSION,
    };

    fn record(now: i64) -> StoredChangePlan {
        StoredChangePlan {
            public: ChangePlan {
                plan_id: "plan-1".into(),
                operation: ChangeOperation::CodexProviderSwitch,
                target_provider_id: "target".into(),
                target_provider_name: "Target".into(),
                plan_digest: "plan-digest".into(),
                baseline_digest: "baseline-digest".into(),
                db_baseline_provider_id: Some("db-current".into()),
                device_baseline_provider_id: Some("device-current".into()),
                secret_capability: SecretCapabilityResult::NoNewCredentialMaterial,
                created_at: now,
                expires_at: now + 900,
                status: ChangePlanStatus::Ready,
                adapter: descriptor_for_operation(ChangeOperation::CodexProviderSwitch),
                current_provider_code: "current_mixed".into(),
                target_provider_code: "existing_provider".into(),
                restart_expectation: RestartRequirement::Recommended,
                risks: vec![ChangePlanRisk {
                    code: "local_configuration_write".into(),
                    severity: "notice".into(),
                }],
                evidence_note: "usage_not_observed".into(),
            },
            target_definition_digest: "target-definition".into(),
            live_baseline_digest: "live-baseline".into(),
            target_projection_digest: "target-projection".into(),
            contract_digest: CHANGE_PLAN_CONTRACT_VERSION.into(),
        }
    }

    fn database() -> Database {
        let db = Database::memory().expect("database");
        db.create_change_plan_tables_for_tests().unwrap();
        db
    }

    #[test]
    fn admission_is_atomic_replay_resistant_and_keeps_separate_baselines() {
        let db = database();
        db.insert_change_plan(&record(100)).unwrap();
        let stored = db.get_stored_change_plan("plan-1").unwrap().unwrap();
        assert_eq!(
            stored.public.db_baseline_provider_id.as_deref(),
            Some("db-current")
        );
        assert_eq!(
            stored.public.device_baseline_provider_id.as_deref(),
            Some("device-current")
        );
        let admitted = db
            .admit_change_plan("plan-1", "plan-digest", "baseline-digest", "job-1", 101)
            .unwrap();
        assert_eq!(admitted.kind, ChangeApplyOutcomeKind::Admitted);
        let replay = db
            .admit_change_plan("plan-1", "plan-digest", "baseline-digest", "job-2", 102)
            .unwrap();
        assert_eq!(replay.error_code, Some(ChangePlanErrorCode::Consumed));
        assert!(db.get_change_job("job-2").unwrap().is_none());
        let job = db.get_change_job("job-1").unwrap().unwrap();
        assert_eq!(job.events.len(), 1);
        assert_eq!(job.events[0].sequence, 1);
    }

    #[test]
    fn rejected_admissions_create_no_job_or_event() {
        for (digest, baseline, now, secret, expected) in [
            (
                "wrong",
                "baseline-digest",
                101,
                "no_new_credential_material",
                ChangePlanErrorCode::InvalidDigest,
            ),
            (
                "plan-digest",
                "drifted",
                101,
                "no_new_credential_material",
                ChangePlanErrorCode::Stale,
            ),
            (
                "plan-digest",
                "baseline-digest",
                1000,
                "no_new_credential_material",
                ChangePlanErrorCode::Expired,
            ),
            (
                "plan-digest",
                "baseline-digest",
                101,
                "secret_dependency_unavailable",
                ChangePlanErrorCode::SecretDependencyUnavailable,
            ),
        ] {
            let db = database();
            db.insert_change_plan(&record(100)).unwrap();
            db.conn
                .lock()
                .unwrap()
                .execute(
                    "UPDATE change_plans SET secret_capability=?1 WHERE plan_id='plan-1'",
                    [secret],
                )
                .unwrap();
            let outcome = db
                .admit_change_plan("plan-1", digest, baseline, "rejected", now)
                .unwrap();
            assert_eq!(outcome.error_code, Some(expected));
            assert!(db.get_change_job("rejected").unwrap().is_none());
            let event_count: i64 = db
                .conn
                .lock()
                .unwrap()
                .query_row("SELECT COUNT(*) FROM change_job_events", [], |row| {
                    row.get(0)
                })
                .unwrap();
            assert_eq!(event_count, 0);
        }
    }

    #[test]
    fn legacy_v1_steps_and_events_normalize_without_schema_migration() {
        let db = database();
        db.insert_change_plan(&record(100)).unwrap();
        db.conn
            .lock()
            .unwrap()
            .execute_batch(
                r#"
                UPDATE change_plans
                   SET status='consumed', consumed_at=101
                 WHERE plan_id='plan-1';
                INSERT INTO change_jobs (
                    job_id, plan_id, target_provider_id, revision, event_seq, status,
                    result_code, steps_json, resources_json, restart_requirement,
                    usage_evidence, recovery_state, diagnostic_code, live_config_changed,
                    created_at, updated_at
                ) VALUES (
                    'legacy-job', 'plan-1', 'target', 3, 3, 'running', 'running',
                    '[{"kind":"precheck","status":"succeeded","code":"ok"},{"kind":"apply","status":"running","code":"writer_started"},{"kind":"readback","status":"pending","code":"pending"},{"kind":"reconcile","status":"pending","code":"pending"}]',
                    '[{"kind":"provider_db_current","status":"pending","code":"pending"},{"kind":"device_current","status":"pending","code":"pending"},{"kind":"target_definition","status":"pending","code":"pending"},{"kind":"codex_live_projection","status":"pending","code":"pending"}]',
                    'unknown', 'not_observed', 'not_needed', NULL, 0, 101, 103
                );
                INSERT INTO change_job_events (job_id, event_seq, phase, reason_code, created_at)
                VALUES
                    ('legacy-job', 1, 'precheck', 'planned', 101),
                    ('legacy-job', 2, 'apply', 'writer_started', 102),
                    ('legacy-job', 3, 'reconcile', 'legacy_reconcile', 103);
                "#,
            )
            .unwrap();

        let job = db.get_change_job("legacy-job").unwrap().unwrap();
        assert_eq!(
            job.steps.iter().map(|step| step.kind).collect::<Vec<_>>(),
            vec![
                ChangeStepKind::Precheck,
                ChangeStepKind::Snapshot,
                ChangeStepKind::ManagedWrite,
                ChangeStepKind::Readback,
                ChangeStepKind::Finalize,
            ]
        );
        assert_eq!(
            job.steps
                .iter()
                .find(|step| step.kind == ChangeStepKind::Snapshot)
                .unwrap()
                .code,
            "legacy_not_recorded"
        );
        assert_eq!(
            job.events
                .iter()
                .map(|event| event.phase)
                .collect::<Vec<_>>(),
            vec![
                ChangeStepKind::Precheck,
                ChangeStepKind::ManagedWrite,
                ChangeStepKind::Finalize,
            ]
        );

        let raw_steps: String = db
            .conn
            .lock()
            .unwrap()
            .query_row(
                "SELECT steps_json FROM change_jobs WHERE job_id='legacy-job'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(raw_steps.contains("\"apply\""));
        assert!(raw_steps.contains("\"reconcile\""));
    }
}
