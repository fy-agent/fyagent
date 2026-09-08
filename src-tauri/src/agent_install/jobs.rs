use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

use chrono::{SecondsFormat, Utc};
use uuid::Uuid;

use super::types::{
    AgentActionId, AgentActionJobSnapshot, AgentActionJobStage, AgentActionTransferPhase,
    AgentActionTransferSample, AgentActionTransferSnapshot, AgentReasonCode, AgentSurface,
    AGENT_ACTION_CONTRACT_VERSION,
};
use crate::codex_desktop::download::{DownloadProgressSink, DownloadProgressUpdate};
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
        surface: AgentSurface,
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
            transfer: None,
            surface,
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
        if is_terminal(stage)
            || matches!(
                stage,
                AgentActionJobStage::LaunchingInstaller
                    | AgentActionJobStage::AwaitingUser
                    | AgentActionJobStage::Installing
            )
        {
            job.snapshot.cancellable = false;
        }
        Ok(job.snapshot.clone())
    }

    pub fn record_transfer(
        &self,
        job_id: &str,
        sample: AgentActionTransferSample,
    ) -> Result<AgentActionJobSnapshot, AgentReasonCode> {
        let mut guard = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let job = guard.as_mut().ok_or(AgentReasonCode::OperationConflict)?;
        if job.snapshot.job_id != job_id {
            return Err(AgentReasonCode::OperationConflict);
        }
        if is_terminal(job.snapshot.stage)
            || job.snapshot.stage != AgentActionJobStage::Downloading
            || !sample.is_well_formed()
        {
            return Ok(job.snapshot.clone());
        }
        if let Some(previous) = job.snapshot.transfer.as_ref() {
            if sample.attempt < previous.attempt
                || (sample.attempt == previous.attempt
                    && sample.completed_bytes < previous.completed_bytes)
            {
                return Ok(job.snapshot.clone());
            }
        }
        let sequence = job
            .snapshot
            .transfer
            .as_ref()
            .map(|previous| previous.sequence.saturating_add(1))
            .unwrap_or(1);
        job.snapshot.transfer = Some(AgentActionTransferSnapshot {
            phase: AgentActionTransferPhase::Download,
            completed_bytes: sample.completed_bytes,
            total_bytes: sample.total_bytes,
            attempt: sample.attempt,
            max_attempts: sample.max_attempts,
            sequence,
            observed_at: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
        });
        Ok(job.snapshot.clone())
    }

    pub fn is_cancelled(&self, flag: &AtomicBool) -> bool {
        flag.load(Ordering::Acquire)
    }
}

pub(super) fn download_progress_sink(
    jobs: Arc<AgentActionJobStore>,
    job_id: impl Into<String>,
) -> impl DownloadProgressSink {
    AgentJobDownloadProgress {
        jobs,
        job_id: job_id.into(),
    }
}

struct AgentJobDownloadProgress {
    jobs: Arc<AgentActionJobStore>,
    job_id: String,
}

impl DownloadProgressSink for AgentJobDownloadProgress {
    fn emit(&self, update: DownloadProgressUpdate) {
        let _ = self.jobs.record_transfer(
            &self.job_id,
            AgentActionTransferSample::from_progress_bytes(
                update.completed_bytes,
                update.total_bytes,
                update.attempt,
                update.max_attempts,
            ),
        );
    }
}

fn is_terminal(stage: AgentActionJobStage) -> bool {
    matches!(
        stage,
        AgentActionJobStage::Succeeded
            | AgentActionJobStage::Failed
            | AgentActionJobStage::Cancelled
            | AgentActionJobStage::Incomplete
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_flight_rejects_a_second_active_job() {
        let store = AgentActionJobStore::new();
        let (first, _) = store
            .start(
                AgentCatalogId::QoderWork,
                AgentActionId::Install,
                AgentSurface::Desktop,
            )
            .unwrap();
        assert_eq!(
            store
                .start(
                    AgentCatalogId::WorkBuddy,
                    AgentActionId::Update,
                    AgentSurface::Desktop
                )
                .err(),
            Some(AgentReasonCode::OperationConflict)
        );
        store
            .transition(&first.job_id, AgentActionJobStage::Succeeded, None)
            .unwrap();
        assert!(store
            .start(
                AgentCatalogId::WorkBuddy,
                AgentActionId::Update,
                AgentSurface::Desktop
            )
            .is_ok());
    }

    #[test]
    fn cancel_is_ignored_after_installing_boundary() {
        let store = AgentActionJobStore::new();
        let (job, flag) = store
            .start(
                AgentCatalogId::TraeWork,
                AgentActionId::Install,
                AgentSurface::Desktop,
            )
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
    fn external_installer_launch_is_the_non_cancellable_boundary() {
        for stage in [
            AgentActionJobStage::LaunchingInstaller,
            AgentActionJobStage::AwaitingUser,
        ] {
            let store = AgentActionJobStore::new();
            let (job, flag) = store
                .start(
                    AgentCatalogId::WorkBuddy,
                    AgentActionId::Install,
                    AgentSurface::Desktop,
                )
                .unwrap();
            let snapshot = store.transition(&job.job_id, stage, None).unwrap();
            assert!(!snapshot.cancellable);
            assert_eq!(
                store.request_cancel(&job.job_id),
                Err(AgentReasonCode::OperationConflict)
            );
            assert!(!flag.load(Ordering::Acquire));
        }
    }

    #[test]
    fn incomplete_is_terminal_and_allows_a_new_job() {
        let store = AgentActionJobStore::new();
        let (job, _) = store
            .start(
                AgentCatalogId::TraeWork,
                AgentActionId::Install,
                AgentSurface::Desktop,
            )
            .unwrap();
        store
            .transition(
                &job.job_id,
                AgentActionJobStage::Incomplete,
                Some(AgentReasonCode::InstallerProcessUnobservable),
            )
            .unwrap();
        assert!(store
            .start(
                AgentCatalogId::QoderWork,
                AgentActionId::Install,
                AgentSurface::Desktop
            )
            .is_ok());
    }

    #[test]
    fn staging_remains_cancellable_until_the_commit_boundary() {
        let store = AgentActionJobStore::new();
        let (job, flag) = store
            .start(
                AgentCatalogId::WorkBuddy,
                AgentActionId::Update,
                AgentSurface::Desktop,
            )
            .unwrap();
        let staging = store
            .transition(&job.job_id, AgentActionJobStage::Staging, None)
            .unwrap();
        assert!(staging.cancellable);

        let cancelled = store.request_cancel(&job.job_id).unwrap();
        assert!(!cancelled.cancellable);
        assert!(flag.load(Ordering::Acquire));
    }

    fn sample(completed: u64, total: Option<u64>, attempt: u8) -> AgentActionTransferSample {
        AgentActionTransferSample {
            completed_bytes: completed,
            total_bytes: total,
            attempt,
            max_attempts: 3,
        }
    }

    #[test]
    fn transfer_samples_are_monotonic_per_attempt_and_reset_on_retry() {
        let store = AgentActionJobStore::new();
        let (job, _) = store
            .start(
                AgentCatalogId::QoderWork,
                AgentActionId::Install,
                AgentSurface::Desktop,
            )
            .unwrap();
        assert!(job.transfer.is_none());
        store
            .transition(&job.job_id, AgentActionJobStage::Downloading, None)
            .unwrap();

        let first = store
            .record_transfer(&job.job_id, sample(100, Some(400), 1))
            .unwrap();
        let transfer = first.transfer.expect("first sample");
        assert_eq!(transfer.completed_bytes, 100);
        assert_eq!(transfer.total_bytes, Some(400));
        assert_eq!(transfer.sequence, 1);
        assert_eq!(transfer.phase, AgentActionTransferPhase::Download);
        assert!(transfer.observed_at.ends_with('Z'));
        assert!(
            transfer.observed_at.contains('.'),
            "download speed samples need sub-second timestamps: {}",
            transfer.observed_at
        );

        let ignored = store
            .record_transfer(&job.job_id, sample(50, Some(400), 1))
            .unwrap();
        assert_eq!(ignored.transfer.as_ref().unwrap().completed_bytes, 100);
        assert_eq!(ignored.transfer.as_ref().unwrap().sequence, 1);

        let advanced = store
            .record_transfer(&job.job_id, sample(200, Some(400), 1))
            .unwrap();
        assert_eq!(advanced.transfer.as_ref().unwrap().completed_bytes, 200);
        assert_eq!(advanced.transfer.as_ref().unwrap().sequence, 2);

        let retried = store
            .record_transfer(&job.job_id, sample(0, Some(400), 2))
            .unwrap();
        let retried = retried.transfer.expect("retry sample");
        assert_eq!(retried.attempt, 2);
        assert_eq!(retried.completed_bytes, 0);
        assert_eq!(retried.sequence, 3);

        let unknown_total = store
            .record_transfer(&job.job_id, sample(64, None, 2))
            .unwrap();
        assert_eq!(unknown_total.transfer.as_ref().unwrap().total_bytes, None);

        store
            .transition(&job.job_id, AgentActionJobStage::Succeeded, None)
            .unwrap();
        let terminal = store.get(&job.job_id).unwrap();
        assert_eq!(terminal.transfer.as_ref().unwrap().completed_bytes, 64);
        let after_terminal = store
            .record_transfer(&job.job_id, sample(128, Some(400), 2))
            .unwrap();
        assert_eq!(
            after_terminal.transfer.as_ref().unwrap().completed_bytes,
            64
        );
    }

    #[test]
    fn malformed_or_pre_download_samples_do_not_create_transfer() {
        let store = AgentActionJobStore::new();
        let (job, _) = store
            .start(
                AgentCatalogId::WorkBuddy,
                AgentActionId::Install,
                AgentSurface::Desktop,
            )
            .unwrap();
        let checking = store
            .record_transfer(&job.job_id, sample(10, Some(20), 1))
            .unwrap();
        assert!(checking.transfer.is_none());
        store
            .transition(&job.job_id, AgentActionJobStage::Downloading, None)
            .unwrap();
        let invalid = AgentActionTransferSample {
            completed_bytes: 1,
            total_bytes: Some(2),
            attempt: 0,
            max_attempts: 1,
        };
        let ignored = store.record_transfer(&job.job_id, invalid).unwrap();
        assert!(ignored.transfer.is_none());
    }
}
