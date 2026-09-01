//! `MacSystemCommitPort` and the single Swift-backed production adapter.
//!
//! The same port is used by signed development and an explicitly admitted
//! Developer ID HIL candidate. Ordinary checks, tests, unsigned binaries, and
//! standard Release bundles remain runtime-disabled until production HIL is
//! complete.

use uuid::Uuid;

use crate::agent_install::AgentReasonCode;

use super::ffi;
use super::types::{
    AuthorizedSystemCommit, HelperState, HelperStatus, SystemCommitOutcome, UserIntent,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum MacSystemCommitRuntimeMode {
    Disabled,
    DevelopmentSigned,
    FormalRelease,
}

pub const fn runtime_mode() -> MacSystemCommitRuntimeMode {
    #[cfg(fyagent_macos_system_commit_mode = "development")]
    {
        return MacSystemCommitRuntimeMode::DevelopmentSigned;
    }
    #[cfg(fyagent_macos_system_commit_mode = "formal")]
    {
        return MacSystemCommitRuntimeMode::FormalRelease;
    }
    #[allow(unreachable_code)]
    MacSystemCommitRuntimeMode::Disabled
}

// Production admission remains deliberately closed until a notarized
// Developer ID candidate passes the real-machine Bless/XPC/system-install HIL.
// Signed development is independently admitted so that the same helper
// implementation can be exercised without weakening the production gate.
const FORMAL_RELEASE_HIL_APPROVED: bool = false;

const fn runtime_admitted(
    mode: MacSystemCommitRuntimeMode,
    client_linked: bool,
    formal_hil_approved: bool,
) -> bool {
    match mode {
        MacSystemCommitRuntimeMode::Disabled => false,
        MacSystemCommitRuntimeMode::DevelopmentSigned => client_linked,
        MacSystemCommitRuntimeMode::FormalRelease => client_linked && formal_hil_approved,
    }
}

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

impl ProductionMacSystemCommitPort {
    fn invoke_status(
        request: &ffi::FyAgentPrivilegedRequest,
    ) -> Result<HelperStatus, AgentReasonCode> {
        let reply = ffi::invoke(request)?;
        let state = match reply.helper_state {
            1 => HelperState::Ready,
            2 => HelperState::UpdateRequired,
            3 => HelperState::Incompatible,
            4 => HelperState::RecoveryRequired,
            5 => HelperState::Missing,
            _ => return Err(AgentReasonCode::HelperProtocolIncompatible),
        };
        Ok(HelperStatus::from_parts(
            state,
            ffi::reason_code(reply.reason),
        ))
    }
}

impl MacSystemCommitPort for ProductionMacSystemCommitPort {
    fn helper_status(&self) -> HelperStatus {
        if !self.production_enabled() {
            return HelperStatus::not_packaged();
        }
        Self::invoke_status(&ffi::FyAgentPrivilegedRequest::status_query())
            .unwrap_or_else(|reason| HelperStatus::from_parts(HelperState::Missing, Some(reason)))
    }

    fn production_enabled(&self) -> bool {
        runtime_admitted(runtime_mode(), ffi::linked(), FORMAL_RELEASE_HIL_APPROVED)
    }

    fn ensure_helper_ready(&self, _intent: UserIntent) -> Result<HelperStatus, AgentReasonCode> {
        if !self.production_enabled() {
            return Err(AgentReasonCode::HelperNotPackaged);
        }
        let operation_id = *Uuid::new_v4().as_bytes();
        let request = ffi::FyAgentPrivilegedRequest::ensure_helper(operation_id);
        let reply = ffi::invoke(&request)?;
        let status = Self::invoke_status(&ffi::FyAgentPrivilegedRequest::status_query())?;
        if reply.outcome != ffi::OUTCOME_READY || !status.claims_success() {
            return Err(ffi::reason_code(reply.reason)
                .or(status.reason)
                .unwrap_or(AgentReasonCode::HelperInstallFailed));
        }
        Ok(status)
    }

    fn commit_known_application(
        &self,
        commit: AuthorizedSystemCommit,
    ) -> Result<SystemCommitOutcome, AgentReasonCode> {
        if !self.production_enabled() {
            return Err(AgentReasonCode::AuthorizationRequired);
        }
        let reply = ffi::invoke(&ffi::FyAgentPrivilegedRequest::commit(&commit))?;
        match reply.outcome {
            ffi::OUTCOME_COMMITTED if reply.reason == 0 => Ok(SystemCommitOutcome::Committed),
            ffi::OUTCOME_ROLLBACK_RESTORED => Ok(SystemCommitOutcome::RollbackRestored),
            ffi::OUTCOME_RECOVERY_REQUIRED => Ok(SystemCommitOutcome::RecoveryRequired),
            _ => Err(ffi::reason_code(reply.reason)
                .unwrap_or(AgentReasonCode::InstallationVerificationFailed)),
        }
    }

    fn remove_helper(&self, _intent: UserIntent) -> Result<(), AgentReasonCode> {
        if !self.production_enabled() {
            return Err(AgentReasonCode::HelperNotPackaged);
        }
        let request = ffi::FyAgentPrivilegedRequest::remove_helper(*Uuid::new_v4().as_bytes());
        let reply = ffi::invoke(&request)?;
        if reply.outcome == ffi::OUTCOME_READY && reply.reason == 0 {
            Ok(())
        } else {
            Err(ffi::reason_code(reply.reason).unwrap_or(AgentReasonCode::HelperRemovalFailed))
        }
    }
}

pub fn production_port() -> ProductionMacSystemCommitPort {
    ProductionMacSystemCommitPort
}

pub fn production_enabled() -> bool {
    ProductionMacSystemCommitPort.production_enabled()
}

/// Inventory and deploy adapters use this only while the signed runtime is off.
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
    fn ordinary_test_build_remains_disabled() {
        if runtime_mode() != MacSystemCommitRuntimeMode::Disabled {
            return;
        }
        assert!(!production_enabled());
        assert!(!production_port().production_enabled());
        assert_eq!(
            system_scope_rejection(),
            AgentReasonCode::AuthorizationRequired
        );
    }

    #[test]
    fn disabled_status_does_not_claim_success() {
        if runtime_mode() != MacSystemCommitRuntimeMode::Disabled {
            return;
        }
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

#[cfg(test)]
mod runtime_admission_tests {
    use super::{runtime_admitted, MacSystemCommitRuntimeMode};

    #[test]
    fn unsigned_runtime_is_always_closed() {
        assert!(!runtime_admitted(
            MacSystemCommitRuntimeMode::Disabled,
            true,
            true
        ));
    }

    #[test]
    fn signed_development_only_depends_on_the_linked_client() {
        assert!(runtime_admitted(
            MacSystemCommitRuntimeMode::DevelopmentSigned,
            true,
            false
        ));
        assert!(!runtime_admitted(
            MacSystemCommitRuntimeMode::DevelopmentSigned,
            false,
            true
        ));
    }

    #[test]
    fn formal_release_stays_closed_until_hil_is_approved() {
        assert!(!runtime_admitted(
            MacSystemCommitRuntimeMode::FormalRelease,
            true,
            false
        ));
        assert!(runtime_admitted(
            MacSystemCommitRuntimeMode::FormalRelease,
            true,
            true
        ));
    }
}
