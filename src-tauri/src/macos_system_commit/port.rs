//! `MacSystemCommitPort` trait and the production adapter.
//!
//! Production `production_enabled()` is false until formal signed/notarized
//! HIL. The production adapter never claims helper success.

use crate::agent_install::AgentReasonCode;

use super::ffi;
use super::types::{AuthorizedSystemCommit, HelperStatus, SystemCommitOutcome, UserIntent};

pub trait MacSystemCommitPort {
    fn helper_status(&self) -> HelperStatus;
    fn production_enabled(&self) -> bool;
    fn ensure_helper_ready(&self, intent: UserIntent) -> Result<HelperStatus, AgentReasonCode>;
    fn commit_known_application(
        &self,
        commit: AuthorizedSystemCommit,
    ) -> Result<SystemCommitOutcome, AgentReasonCode>;
    fn remove_helper(&self, intent: UserIntent) -> Result<(), AgentReasonCode>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct ProductionMacSystemCommitPort;

impl MacSystemCommitPort for ProductionMacSystemCommitPort {
    fn helper_status(&self) -> HelperStatus {
        let _ = ffi::invoke(&ffi::FyAgentPrivilegedRequest::status_query());
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
        _commit: AuthorizedSystemCommit,
    ) -> Result<SystemCommitOutcome, AgentReasonCode> {
        debug_assert!(!self.production_enabled());
        Err(AgentReasonCode::AuthorizationRequired)
    }

    fn remove_helper(&self, _intent: UserIntent) -> Result<(), AgentReasonCode> {
        Err(AgentReasonCode::HelperNotPackaged)
    }
}

pub fn production_port() -> ProductionMacSystemCommitPort {
    ProductionMacSystemCommitPort
}

pub fn production_enabled() -> bool {
    ProductionMacSystemCommitPort.production_enabled()
}

/// Inventory and deploy adapters use this until HIL enables the port.
pub fn system_scope_rejection() -> AgentReasonCode {
    debug_assert!(!production_enabled());
    AgentReasonCode::AuthorizationRequired
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::macos_system_commit::policy::KnownSystemProduct;
    use crate::macos_system_commit::types::SystemCommitAction;

    fn sample_commit() -> AuthorizedSystemCommit {
        AuthorizedSystemCommit::new(
            KnownSystemProduct::QoderWork,
            1,
            SystemCommitAction::FreshInstall,
            [0; 16],
            [0; 32],
            [0; 32],
            -1,
        )
        .expect("qoderwork fresh slot")
    }

    #[test]
    fn production_enabled_is_false() {
        assert!(!production_enabled());
        assert!(!production_port().production_enabled());
        assert_eq!(
            system_scope_rejection(),
            AgentReasonCode::AuthorizationRequired
        );
    }

    #[test]
    fn production_status_is_not_packaged_and_does_not_claim_success() {
        let status = production_port().helper_status();
        assert!(!status.claims_success());
        assert_eq!(status.reason, Some(AgentReasonCode::HelperNotPackaged));
        assert_eq!(
            production_port().ensure_helper_ready(UserIntent::attested()),
            Err(AgentReasonCode::HelperNotPackaged)
        );
        assert_eq!(
            production_port().remove_helper(UserIntent::attested()),
            Err(AgentReasonCode::HelperNotPackaged)
        );
        assert_eq!(
            production_port().commit_known_application(sample_commit()),
            Err(AgentReasonCode::AuthorizationRequired)
        );
    }
}
