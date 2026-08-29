use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

use uuid::Uuid;

use super::types::{
    AgentActionId, AgentActionJobSnapshot, AgentActionJobStage, AgentReasonCode,
    AGENT_ACTION_CONTRACT_VERSION,
};
use crate::services::external_agents::AgentCatalogId;

struct JobRecord {
    snapshot: AgentActionJobSnapshot,
    cancel: Arc<AtomicBool>,
}

#[derive(Default)]
pub struct AgentActionJobStore {
    inner: Mutex<Option<JobRecord>>,
}

impl AgentActionJobStore {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(None),
        }
    }

    pub fn get(&self, job_id: &str) -> Result<AgentActionJobSnapshot, AgentReasonCode> {
        let guard = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        match guard.as_ref() {
            Some(job) if job.snapshot.job_id == job_id => Ok(job.snapshot.clone()),
            _ => Err(AgentReasonCode::OperationConflict),
        }
    }

    pub fn current(&self) -> Option<AgentActionJobSnapshot> {
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .as_ref()
            .map(|job| job.snapshot.clone())
    }

    pub fn start(
        &self,
        agent_id: AgentCatalogId,
        action: AgentActionId,
    ) -> Result<(AgentActionJobSnapshot, Arc<AtomicBool>), AgentReasonCode> {
        let mut guard = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(existing) = guard.as_ref() {
            if !is_terminal(existing.snapshot.stage) {
                return Err(AgentReasonCode::OperationConflict);
            }
        }
        let cancel = Arc::new(AtomicBool::new(false));
        let snapshot = AgentActionJobSnapshot {
            contract_version: AGENT_ACTION_CONTRACT_VERSION,
            job_id: Uuid::new_v4().to_string(),
            agent_id,
            action,
            stage: AgentActionJobStage::Checking,
            cancellable: true,
            reason_code: None,
        };
        *guard = Some(JobRecord {
            snapshot: snapshot.clone(),
            cancel: Arc::clone(&cancel),
        });
        Ok((snapshot, cancel))
    }

    pub fn request_cancel(&self, job_id: &str) -> Result<AgentActionJobSnapshot, AgentReasonCode> {
        let mut guard = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let job = guard.as_mut().ok_or(AgentReasonCode::OperationConflict)?;
        if job.snapshot.job_id != job_id {
            return Err(AgentReasonCode::OperationConflict);
        }
        if !job.snapshot.cancellable || is_terminal(job.snapshot.stage) {
            return Err(AgentReasonCode::OperationConflict);
        }
        job.cancel.store(true, Ordering::Release);
        job.snapshot.cancellable = false;
        Ok(job.snapshot.clone())
    }

    pub fn transition(
        &self,
        job_id: &str,
        stage: AgentActionJobStage,
        reason_code: Option<AgentReasonCode>,
    ) -> Result<AgentActionJobSnapshot, AgentReasonCode> {
        let mut guard = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let job = guard.as_mut().ok_or(AgentReasonCode::OperationConflict)?;
        if job.snapshot.job_id != job_id {
            return Err(AgentReasonCode::OperationConflict);
        }
        job.snapshot.stage = stage;
        job.snapshot.reason_code = reason_code;
        if is_terminal(stage) || stage == AgentActionJobStage::Installing {
            job.snapshot.cancellable = false;
        }
        Ok(job.snapshot.clone())
    }

    pub fn is_cancelled(&self, flag: &AtomicBool) -> bool {
        flag.load(Ordering::Acquire)
    }
}

fn is_terminal(stage: AgentActionJobStage) -> bool {
    matches!(
        stage,
        AgentActionJobStage::Succeeded
            | AgentActionJobStage::Failed
            | AgentActionJobStage::Cancelled
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_flight_rejects_a_second_active_job() {
        let store = AgentActionJobStore::new();
        let (first, _) = store
            .start(AgentCatalogId::QoderWork, AgentActionId::Install)
            .unwrap();
        assert_eq!(
            store
                .start(AgentCatalogId::WorkBuddy, AgentActionId::Update)
                .err(),
            Some(AgentReasonCode::OperationConflict)
        );
        store
            .transition(&first.job_id, AgentActionJobStage::Succeeded, None)
            .unwrap();
        assert!(store
            .start(AgentCatalogId::WorkBuddy, AgentActionId::Update)
            .is_ok());
    }

    #[test]
    fn cancel_is_ignored_after_installing_boundary() {
        let store = AgentActionJobStore::new();
        let (job, flag) = store
            .start(AgentCatalogId::TraeWork, AgentActionId::Install)
            .unwrap();
        store
            .transition(&job.job_id, AgentActionJobStage::Installing, None)
            .unwrap();
        assert_eq!(
            store.request_cancel(&job.job_id),
            Err(AgentReasonCode::OperationConflict)
        );
        assert!(!flag.load(Ordering::Acquire));
    }

    #[test]
    fn staging_remains_cancellable_until_the_commit_boundary() {
        let store = AgentActionJobStore::new();
        let (job, flag) = store
            .start(AgentCatalogId::WorkBuddy, AgentActionId::Update)
            .unwrap();
        let staging = store
            .transition(&job.job_id, AgentActionJobStage::Staging, None)
            .unwrap();
        assert!(staging.cancellable);

        let cancelled = store.request_cancel(&job.job_id).unwrap();
        assert!(!cancelled.cancellable);
        assert!(flag.load(Ordering::Acquire));
    }
}
