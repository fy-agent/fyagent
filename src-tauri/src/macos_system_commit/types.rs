//! Closed helper-facing types. `AuthorizedSystemCommit` is crate-private and
//! is not a Tauri command argument.

use crate::agent_install::AgentReasonCode;

use super::policy::{resolve_slot, KnownSystemProduct};

pub const ABI_VERSION: u32 = 1;
pub const PROTOCOL_VERSION: u32 = 1;

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemCommitAction {
    FreshInstall = 1,
    UpdateExisting = 2,
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemCommitOutcome {
    Committed = 1,
    RollbackRestored = 2,
    RecoveryRequired = 3,
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HelperState {
    Ready = 1,
    UpdateRequired = 2,
    Incompatible = 3,
    RecoveryRequired = 4,
    Missing = 5,
}

/// Backend-attested user intent. Not serializable and not a renderer input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UserIntent {
    _private: (),
}

impl UserIntent {
    pub(crate) fn attested() -> Self {
        Self { _private: () }
    }
}

/// Inventory-validated system commit. Constructed only after the Agent/Codex
/// lifecycle owner re-enumerates the opaque target. Never `Serialize`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorizedSystemCommit {
    product: KnownSystemProduct,
    target_slot: u32,
    action: SystemCommitAction,
    operation_id: [u8; 16],
    expected_source_revision: [u8; 32],
    expected_target_revision: [u8; 32],
    source_directory_fd: i32,
}

impl AuthorizedSystemCommit {
    pub fn new(
        product: KnownSystemProduct,
        target_slot: u32,
        action: SystemCommitAction,
        operation_id: [u8; 16],
        expected_source_revision: [u8; 32],
        expected_target_revision: [u8; 32],
        source_directory_fd: i32,
    ) -> Result<Self, AgentReasonCode> {
        let slot = resolve_slot(product, target_slot)?;
        if matches!(action, SystemCommitAction::FreshInstall) && slot.existing_only {
            return Err(AgentReasonCode::TargetSlotInvalid);
        }
        Ok(Self {
            product,
            target_slot,
            action,
            operation_id,
            expected_source_revision,
            expected_target_revision,
            source_directory_fd,
        })
    }

    pub fn product(&self) -> KnownSystemProduct {
        self.product
    }

    pub fn target_slot(&self) -> u32 {
        self.target_slot
    }

    pub fn action(&self) -> SystemCommitAction {
        self.action
    }

    pub fn operation_id(&self) -> [u8; 16] {
        self.operation_id
    }

    pub fn expected_source_revision(&self) -> [u8; 32] {
        self.expected_source_revision
    }

    pub fn expected_target_revision(&self) -> [u8; 32] {
        self.expected_target_revision
    }

    pub fn source_directory_fd(&self) -> i32 {
        self.source_directory_fd
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HelperStatus {
    pub protocol_version: u32,
    pub state: HelperState,
    pub reason: Option<AgentReasonCode>,
}

impl HelperStatus {
    pub fn not_packaged() -> Self {
        Self {
            protocol_version: PROTOCOL_VERSION,
            state: HelperState::Missing,
            reason: Some(AgentReasonCode::HelperNotPackaged),
        }
    }

    pub fn claims_success(&self) -> bool {
        matches!(self.state, HelperState::Ready) && self.reason.is_none()
    }

    pub fn from_parts(state: HelperState, reason: Option<AgentReasonCode>) -> Self {
        Self {
            protocol_version: PROTOCOL_VERSION,
            state,
            reason,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_slot_and_existing_only_fresh_install_are_rejected() {
        let err = AuthorizedSystemCommit::new(
            KnownSystemProduct::QoderWork,
            9,
            SystemCommitAction::FreshInstall,
            [0; 16],
            [0; 32],
            [0; 32],
            -1,
        );
        assert_eq!(err, Err(AgentReasonCode::TargetSlotInvalid));

        let existing_only = AuthorizedSystemCommit::new(
            KnownSystemProduct::CodexDesktop,
            2,
            SystemCommitAction::FreshInstall,
            [0; 16],
            [0; 32],
            [0; 32],
            -1,
        );
        assert_eq!(existing_only, Err(AgentReasonCode::TargetSlotInvalid));

        let update = AuthorizedSystemCommit::new(
            KnownSystemProduct::CodexDesktop,
            2,
            SystemCommitAction::UpdateExisting,
            [1; 16],
            [2; 32],
            [3; 32],
            -1,
        )
        .expect("historical Codex.app slot is update-only");
        assert_eq!(update.product(), KnownSystemProduct::CodexDesktop);
        assert_eq!(update.target_slot(), 2);
        assert_eq!(update.action(), SystemCommitAction::UpdateExisting);
        assert_eq!(update.source_directory_fd(), -1);
    }

    #[test]
    fn authorized_commit_stores_closed_enums_not_paths() {
        let commit = AuthorizedSystemCommit::new(
            KnownSystemProduct::WorkBuddy,
            1,
            SystemCommitAction::FreshInstall,
            [9; 16],
            [8; 32],
            [7; 32],
            4,
        )
        .expect("workbuddy fresh slot");
        let debug = format!("{commit:?}");
        assert!(!debug.contains("/Applications"));
        assert!(!debug.contains("WorkBuddy.app"));
        assert!(!debug.contains("http"));
        assert_eq!(commit.source_directory_fd(), 4);
        assert_eq!(commit.operation_id(), [9; 16]);
        assert_eq!(commit.expected_source_revision(), [8; 32]);
        assert_eq!(commit.expected_target_revision(), [7; 32]);
    }

    #[test]
    fn helper_status_not_packaged_does_not_claim_success() {
        let status = HelperStatus::not_packaged();
        assert!(!status.claims_success());
        assert_eq!(status.state, HelperState::Missing);
        assert_eq!(status.reason, Some(AgentReasonCode::HelperNotPackaged));
        assert_eq!(HelperState::Ready as u32, 1);
        assert_eq!(HelperState::UpdateRequired as u32, 2);
        assert_eq!(HelperState::Incompatible as u32, 3);
        assert_eq!(HelperState::RecoveryRequired as u32, 4);
        assert_eq!(HelperState::Missing as u32, 5);
        assert_eq!(SystemCommitOutcome::Committed as u32, 1);
        assert_eq!(SystemCommitOutcome::RollbackRestored as u32, 2);
        assert_eq!(SystemCommitOutcome::RecoveryRequired as u32, 3);
    }
}
