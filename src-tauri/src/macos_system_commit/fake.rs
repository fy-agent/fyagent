//! Portable fake `MacSystemCommitPort` for tests. Does not link Swift or
//! mutate the real `/Applications` tree.

use std::sync::Mutex;

use crate::agent_install::AgentReasonCode;

use super::port::MacSystemCommitPort;
use super::types::{AuthorizedSystemCommit, HelperStatus, SystemCommitOutcome, UserIntent};

#[derive(Debug)]
struct FakeInner {
    mutations: u32,
    commit_script: Vec<Result<SystemCommitOutcome, AgentReasonCode>>,
}

pub struct FakeMacSystemCommitPort {
    inner: Mutex<FakeInner>,
}

impl FakeMacSystemCommitPort {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(FakeInner {
                mutations: 0,
                commit_script: Vec::new(),
            }),
        }
    }

    pub fn script_commit(&self, result: Result<SystemCommitOutcome, AgentReasonCode>) {
        self.inner
            .lock()
            .expect("fake port mutex")
            .commit_script
            .push(result);
    }

    pub fn mutations(&self) -> u32 {
        self.inner.lock().expect("fake port mutex").mutations
    }
}

impl Default for FakeMacSystemCommitPort {
    fn default() -> Self {
        Self::new()
    }
}

impl MacSystemCommitPort for FakeMacSystemCommitPort {
    fn helper_status(&self) -> HelperStatus {
        HelperStatus::not_packaged()
    }

    fn production_enabled(&self) -> bool {
        false
    }

    fn ensure_helper_ready(&self, _intent: UserIntent) -> Result<HelperStatus, AgentReasonCode> {
        Err(AgentReasonCode::HelperNotPackaged)
    }

    fn commit_known_application(
        &self,
        commit: AuthorizedSystemCommit,
    ) -> Result<SystemCommitOutcome, AgentReasonCode> {
        let _ = commit;
        let mut inner = self.inner.lock().expect("fake port mutex");
        let result = if inner.commit_script.is_empty() {
            Ok(SystemCommitOutcome::Committed)
        } else {
            inner.commit_script.remove(0)
        };
        if result.is_ok() {
            inner.mutations = inner.mutations.saturating_add(1);
        }
        result
    }

    fn remove_helper(&self, _intent: UserIntent) -> Result<(), AgentReasonCode> {
        Err(AgentReasonCode::HelperNotPackaged)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::macos_system_commit::policy::KnownSystemProduct;
    use crate::macos_system_commit::types::SystemCommitAction;

    fn commit() -> AuthorizedSystemCommit {
        AuthorizedSystemCommit::new(
            KnownSystemProduct::OpenCodeDesktop,
            1,
            SystemCommitAction::FreshInstall,
            [4; 16],
            [5; 32],
            [6; 32],
            -1,
        )
        .expect("opencode fresh slot")
    }

    #[test]
    fn fresh_commit_records_a_mutation() {
        let port = FakeMacSystemCommitPort::new();
        assert!(!port.production_enabled());
        assert_eq!(
            port.commit_known_application(commit()),
            Ok(SystemCommitOutcome::Committed)
        );
        assert_eq!(port.mutations(), 1);
    }

    #[test]
    fn rollback_restored_and_recovery_required_are_terminal_outcomes() {
        let port = FakeMacSystemCommitPort::new();
        port.script_commit(Ok(SystemCommitOutcome::RollbackRestored));
        port.script_commit(Ok(SystemCommitOutcome::RecoveryRequired));
        assert_eq!(
            port.commit_known_application(commit()),
            Ok(SystemCommitOutcome::RollbackRestored)
        );
        assert_eq!(
            port.commit_known_application(commit()),
            Ok(SystemCommitOutcome::RecoveryRequired)
        );
        assert_eq!(port.mutations(), 2);
    }

    #[test]
    fn authorization_cancelled_does_zero_mutation() {
        let port = FakeMacSystemCommitPort::new();
        port.script_commit(Err(AgentReasonCode::OperationAuthorizationCancelled));
        port.script_commit(Err(AgentReasonCode::HelperInstallAuthorizationCancelled));
        assert_eq!(
            port.commit_known_application(commit()),
            Err(AgentReasonCode::OperationAuthorizationCancelled)
        );
        assert_eq!(
            port.commit_known_application(commit()),
            Err(AgentReasonCode::HelperInstallAuthorizationCancelled)
        );
        assert_eq!(port.mutations(), 0);
    }
}
