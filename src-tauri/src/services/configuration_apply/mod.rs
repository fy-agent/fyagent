//! Process-local configuration-apply coordinator with a fake Provider adapter.
//!
//! Does not persist jobs, resolve #35 secrets, or wire production commands.

mod backup;
mod provider;
mod runtime;

pub use backup::{BackupError, BackupReceipt, BackupResourceKind, FakeBackupStore};
pub use provider::{
    FakeProviderAdapter, FakeWriterMode, ProviderHttpSpy, ReadbackError, ReadbackMatch, WriteError,
    WriteReceipt,
};
pub use runtime::{ApplyRuntime, CancellationHandle, WorkerAcquireError};

/// Backend job terminal/non-terminal status. `not_started` is renderer-only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyJobStatus {
    Planned,
    Running,
    Succeeded,
    Warning,
    Failed,
    Cancelled,
}

/// Observed local effect after the run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyEffect {
    None,
    Applied,
    Partial,
    Unknown,
}

/// Recovery actions the fake coordinator may surface. Closed set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyRecoveryAction {
    RetryReadback,
    RestoreBackup,
    RetryRefresh,
    RegeneratePlan,
}

/// Binary observables for the process-local fake run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyRunOutcome {
    pub status: ApplyJobStatus,
    pub effect: ApplyEffect,
    pub writer_count: u64,
    pub outbound_provider_http_count: u64,
    pub recovery_actions: Vec<ApplyRecoveryAction>,
}

/// Fake coordinator: backup → writer once → readback → effect.
pub struct FakeApplyCoordinator {
    runtime: ApplyRuntime,
    adapter: FakeProviderAdapter,
    backup: FakeBackupStore,
}

impl FakeApplyCoordinator {
    pub fn succeeding() -> Self {
        Self {
            runtime: ApplyRuntime::new(),
            adapter: FakeProviderAdapter::succeeding(),
            backup: FakeBackupStore::new(),
        }
    }

    pub fn failing_writer() -> Self {
        Self {
            runtime: ApplyRuntime::new(),
            adapter: FakeProviderAdapter::failing(),
            backup: FakeBackupStore::new(),
        }
    }

    pub fn request_cancel(&self) {
        self.runtime.request_cancel();
    }

    pub fn run(&self) -> ApplyRunOutcome {
        let _worker = match self.runtime.try_acquire_worker() {
            Ok(guard) => guard,
            Err(WorkerAcquireError::AlreadyActive | WorkerAcquireError::Poisoned) => {
                return self.finish(
                    ApplyJobStatus::Failed,
                    ApplyEffect::None,
                    vec![ApplyRecoveryAction::RegeneratePlan],
                );
            }
        };

        if self.runtime.cancel_requested() {
            return self.finish(ApplyJobStatus::Cancelled, ApplyEffect::None, Vec::new());
        }

        let declared = [BackupResourceKind::CodexLiveConfig];
        if self
            .backup
            .create_from_declared_resources(&declared)
            .is_err()
        {
            return self.finish(
                ApplyJobStatus::Failed,
                ApplyEffect::None,
                vec![ApplyRecoveryAction::RegeneratePlan],
            );
        }

        if self.runtime.cancel_requested() {
            return self.finish(ApplyJobStatus::Cancelled, ApplyEffect::None, Vec::new());
        }

        match self.adapter.write_once() {
            Ok(_receipt) => {}
            Err(WriteError::ManagedWriteFailed | WriteError::WriterAlreadyUsed) => {
                let recovery = if self.backup.is_available() {
                    vec![ApplyRecoveryAction::RestoreBackup]
                } else {
                    vec![ApplyRecoveryAction::RegeneratePlan]
                };
                return self.finish(ApplyJobStatus::Failed, ApplyEffect::None, recovery);
            }
        }

        match self.adapter.readback() {
            Ok(_matched) => {
                self.finish(ApplyJobStatus::Succeeded, ApplyEffect::Applied, Vec::new())
            }
            Err(ReadbackError::Mismatch) => self.finish(
                ApplyJobStatus::Failed,
                ApplyEffect::Unknown,
                vec![ApplyRecoveryAction::RetryReadback],
            ),
        }
    }

    fn finish(
        &self,
        status: ApplyJobStatus,
        effect: ApplyEffect,
        recovery_actions: Vec<ApplyRecoveryAction>,
    ) -> ApplyRunOutcome {
        self.runtime.emit_terminal();
        ApplyRunOutcome {
            status,
            effect,
            writer_count: self.adapter.writer_count(),
            outbound_provider_http_count: self.adapter.outbound_http_count(),
            recovery_actions,
        }
    }
}
