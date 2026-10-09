//! Closed, feature-local truth about a binary database restore.
use serde::Serialize;
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RestorePhase {
    Precheck,
    Candidate,
    Archive,
    SafetyBackup,
    Publish,
    Readback,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RestorePublication {
    NotCommitted,
    Committed,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RestoreResultCode {
    Restored,
    PrecheckFailed,
    CandidateFailed,
    ArchiveFailed,
    SafetyBackupFailed,
    PublishFailed,
    ReadbackFailed,
    WorkerLost,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseRestoreOutcome {
    pub contract_version: u8,
    pub phase: RestorePhase,
    pub publication: RestorePublication,
    pub safety_backup_filename: Option<String>,
    pub warnings: Vec<String>,
    pub result_code: RestoreResultCode,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DatabaseReadabilityState {
    Readable,
    Unavailable,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseRecoveryReadability {
    pub contract_version: u8,
    pub state: DatabaseReadabilityState,
}

impl DatabaseRecoveryReadability {
    pub(crate) fn from_readable(readable: bool) -> Self {
        Self {
            contract_version: 1,
            state: if readable {
                DatabaseReadabilityState::Readable
            } else {
                DatabaseReadabilityState::Unavailable
            },
        }
    }
}

/// A join error cannot erase the publication evidence recorded by the worker.
#[derive(Clone)]
pub(crate) struct RestoreTracker(Arc<Mutex<DatabaseRestoreOutcome>>);

impl RestoreTracker {
    pub(crate) fn new() -> Self {
        Self(Arc::new(Mutex::new(DatabaseRestoreOutcome {
            contract_version: 1,
            phase: RestorePhase::Precheck,
            publication: RestorePublication::NotCommitted,
            safety_backup_filename: None,
            warnings: Vec::new(),
            result_code: RestoreResultCode::PrecheckFailed,
        })))
    }

    fn update(&self, update: impl FnOnce(&mut DatabaseRestoreOutcome)) {
        let mut outcome = self.0.lock().unwrap_or_else(|error| error.into_inner());
        update(&mut outcome);
    }

    pub(crate) fn phase(&self, phase: RestorePhase) {
        self.update(|outcome| outcome.phase = phase);
    }

    pub(crate) fn safety_backup(&self, filename: String) {
        self.update(|outcome| outcome.safety_backup_filename = Some(filename));
    }

    pub(crate) fn begin_publish(&self) {
        self.update(|outcome| {
            outcome.phase = RestorePhase::Publish;
            // A fallible replacement error alone cannot prove the live preimage.
            outcome.publication = RestorePublication::Unknown;
        });
    }

    pub(crate) fn committed(&self) {
        self.update(|outcome| outcome.publication = RestorePublication::Committed);
    }

    pub(crate) fn retention_warning(&self) {
        self.update(|outcome| {
            if outcome.warnings.is_empty() {
                outcome.warnings.push("retentionFailed".into());
            }
        });
    }

    pub(crate) fn finish(&self, succeeded: bool) -> DatabaseRestoreOutcome {
        self.update(|outcome| {
            outcome.result_code = if succeeded {
                RestoreResultCode::Restored
            } else {
                match outcome.phase {
                    RestorePhase::Precheck => RestoreResultCode::PrecheckFailed,
                    RestorePhase::Candidate => RestoreResultCode::CandidateFailed,
                    RestorePhase::Archive => RestoreResultCode::ArchiveFailed,
                    RestorePhase::SafetyBackup => RestoreResultCode::SafetyBackupFailed,
                    RestorePhase::Publish => RestoreResultCode::PublishFailed,
                    RestorePhase::Readback => RestoreResultCode::ReadbackFailed,
                }
            };
        });
        self.snapshot()
    }

    pub(crate) fn snapshot(&self) -> DatabaseRestoreOutcome {
        self.0
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
    }

    pub(crate) fn worker_lost(&self) -> DatabaseRestoreOutcome {
        self.update(|outcome| {
            if outcome.publication != RestorePublication::Committed {
                outcome.publication = RestorePublication::Unknown;
            }
            outcome.result_code = RestoreResultCode::WorkerLost;
        });
        self.snapshot()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lost_worker_preserves_commit_evidence_and_backup_leaf() {
        let tracker = RestoreTracker::new();
        assert_eq!(
            tracker.worker_lost().publication,
            RestorePublication::Unknown
        );
        tracker.safety_backup("db_backup_fixture.db".into());
        tracker.begin_publish();
        tracker.committed();
        tracker.phase(RestorePhase::Readback);
        let outcome = tracker.worker_lost();
        assert_eq!(outcome.publication, RestorePublication::Committed);
        assert_eq!(outcome.phase, RestorePhase::Readback);
        assert_eq!(outcome.result_code, RestoreResultCode::WorkerLost);
        assert_eq!(
            outcome.safety_backup_filename.as_deref(),
            Some("db_backup_fixture.db")
        );
    }
}
