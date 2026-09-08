//! Fixed C ABI for the in-process Swift privileged client.
//!
//! Ordinary checks/tests remain independent of the Swift image. The extern
//! symbol exists only when the macOS feature and one reviewed signed runtime
//! mode are both selected by `build.rs`.

use crate::agent_install::AgentReasonCode;

use super::types::{AuthorizedSystemCommit, ABI_VERSION, PROTOCOL_VERSION};

pub const OPERATION_STATUS: u32 = 1;
pub const OPERATION_ENSURE_HELPER: u32 = 2;
pub const OPERATION_COMMIT: u32 = 3;
pub const OPERATION_REMOVE_HELPER: u32 = 4;

pub const OUTCOME_COMMITTED: u32 = 1;
pub const OUTCOME_ROLLBACK_RESTORED: u32 = 2;
pub const OUTCOME_RECOVERY_REQUIRED: u32 = 3;
pub const OUTCOME_READY: u32 = 4;
pub const OUTCOME_FAILED: u32 = 5;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FyAgentPrivilegedRequest {
    pub abi_version: u32,
    pub size: u32,
    pub protocol_version: u32,
    pub operation: u32,
    pub action: u32,
    pub product: u32,
    pub target_slot: u32,
    pub reserved0: u32,
    pub operation_id: [u8; 16],
    pub expected_source_revision: [u8; 32],
    pub expected_target_revision: [u8; 32],
    pub source_directory_fd: i32,
    pub reserved1: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FyAgentPrivilegedReply {
    pub abi_version: u32,
    pub size: u32,
    pub protocol_version: u32,
    pub outcome: u32,
    pub reason: u32,
    pub helper_state: u32,
    pub reserved0: u32,
    pub operation_id: [u8; 16],
}

impl FyAgentPrivilegedRequest {
    fn operation(operation: u32, operation_id: [u8; 16]) -> Self {
        Self {
            abi_version: ABI_VERSION,
            size: std::mem::size_of::<Self>() as u32,
            protocol_version: PROTOCOL_VERSION,
            operation,
            action: 0,
            product: 0,
            target_slot: 0,
            reserved0: 0,
            operation_id,
            expected_source_revision: [0; 32],
            expected_target_revision: [0; 32],
            source_directory_fd: -1,
            reserved1: 0,
        }
    }

    pub fn status_query() -> Self {
        Self::operation(OPERATION_STATUS, [0; 16])
    }

    pub fn ensure_helper(operation_id: [u8; 16]) -> Self {
        Self::operation(OPERATION_ENSURE_HELPER, operation_id)
    }

    pub fn remove_helper(operation_id: [u8; 16]) -> Self {
        Self::operation(OPERATION_REMOVE_HELPER, operation_id)
    }

    pub fn commit(commit: &AuthorizedSystemCommit) -> Self {
        let mut request = Self::operation(OPERATION_COMMIT, commit.operation_id());
        request.action = commit.action() as u32;
        request.product = commit.product().as_u32();
        request.target_slot = commit.target_slot();
        request.expected_source_revision = commit.expected_source_revision();
        request.expected_target_revision = commit.expected_target_revision();
        request.source_directory_fd = commit.source_directory_fd();
        request
    }
}

#[cfg(all(
    target_os = "macos",
    feature = "macos-privileged-client",
    any(
        fyagent_macos_system_commit_mode = "development",
        fyagent_macos_system_commit_mode = "formal"
    )
))]
unsafe extern "C" {
    fn fyagent_privileged_invoke(
        request: *const FyAgentPrivilegedRequest,
        reply: *mut FyAgentPrivilegedReply,
    ) -> i32;
}

pub const fn linked() -> bool {
    cfg!(all(
        target_os = "macos",
        feature = "macos-privileged-client",
        any(
            fyagent_macos_system_commit_mode = "development",
            fyagent_macos_system_commit_mode = "formal"
        )
    ))
}

/// Invoke the Swift bridge and validate the complete fixed-width envelope.
#[cfg(all(
    target_os = "macos",
    feature = "macos-privileged-client",
    any(
        fyagent_macos_system_commit_mode = "development",
        fyagent_macos_system_commit_mode = "formal"
    )
))]
pub fn invoke(
    request: &FyAgentPrivilegedRequest,
) -> Result<FyAgentPrivilegedReply, AgentReasonCode> {
    let mut reply = FyAgentPrivilegedReply {
        abi_version: 0,
        size: 0,
        protocol_version: 0,
        outcome: OUTCOME_FAILED,
        reason: 0,
        helper_state: 0,
        reserved0: 0,
        operation_id: [0; 16],
    };
    let status = unsafe { fyagent_privileged_invoke(request, &mut reply) };
    if status != 0 {
        return Err(AgentReasonCode::HelperProtocolIncompatible);
    }
    if reply.abi_version != ABI_VERSION
        || reply.size != std::mem::size_of::<FyAgentPrivilegedReply>() as u32
        || reply.protocol_version != PROTOCOL_VERSION
        || reply.reserved0 != 0
        || reply.operation_id != request.operation_id
    {
        return Err(AgentReasonCode::HelperProtocolIncompatible);
    }
    Ok(reply)
}

#[cfg(not(all(
    target_os = "macos",
    feature = "macos-privileged-client",
    any(
        fyagent_macos_system_commit_mode = "development",
        fyagent_macos_system_commit_mode = "formal"
    )
)))]
pub fn invoke(
    _request: &FyAgentPrivilegedRequest,
) -> Result<FyAgentPrivilegedReply, AgentReasonCode> {
    Err(AgentReasonCode::HelperNotPackaged)
}

pub fn reason_code(value: u32) -> Option<AgentReasonCode> {
    match value {
        0 => None,
        1 => Some(AgentReasonCode::HelperNotPackaged),
        2 => Some(AgentReasonCode::HelperSignatureInvalid),
        3 => Some(AgentReasonCode::HelperInstallAuthorizationCancelled),
        4 => Some(AgentReasonCode::HelperInstallFailed),
        5 => Some(AgentReasonCode::HelperUpdateRequired),
        6 => Some(AgentReasonCode::HelperDowngradeRejected),
        7 => Some(AgentReasonCode::HelperProtocolIncompatible),
        8 => Some(AgentReasonCode::HelperPeerRejected),
        9 => Some(AgentReasonCode::OperationAuthorizationCancelled),
        10 => Some(AgentReasonCode::OperationAuthorizationInvalid),
        11 => Some(AgentReasonCode::SourceCapabilityInvalid),
        12 => Some(AgentReasonCode::SourceChanged),
        13 => Some(AgentReasonCode::TargetSlotInvalid),
        14 => Some(AgentReasonCode::HelperRemovalFailed),
        15 => Some(AgentReasonCode::TargetChanged),
        16 => Some(AgentReasonCode::ApplicationRunning),
        17 => Some(AgentReasonCode::PermissionDenied),
        18 => Some(AgentReasonCode::InstallationVerificationFailed),
        19 => Some(AgentReasonCode::RollbackRestored),
        20 => Some(AgentReasonCode::RecoveryRequired),
        21 | 22 => Some(AgentReasonCode::HelperProtocolIncompatible),
        _ => Some(AgentReasonCode::HelperProtocolIncompatible),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::macos_system_commit::{KnownSystemProduct, SystemCommitAction};

    #[test]
    fn request_layout_is_fixed_width_without_string_pointers() {
        assert_eq!(std::mem::size_of::<FyAgentPrivilegedRequest>(), 120);
        assert_eq!(std::mem::align_of::<FyAgentPrivilegedRequest>(), 4);
        assert_eq!(std::mem::size_of::<FyAgentPrivilegedReply>(), 44);
        let request = FyAgentPrivilegedRequest::status_query();
        assert_eq!(request.abi_version, 1);
        assert_eq!(request.protocol_version, 1);
        assert_eq!(request.reserved0, 0);
        assert_eq!(request.reserved1, 0);
        assert_eq!(request.source_directory_fd, -1);
        assert_eq!(request.size, 120);
        assert_eq!(OPERATION_STATUS, 1);
        assert_eq!(OPERATION_ENSURE_HELPER, 2);
        assert_eq!(OPERATION_COMMIT, 3);
        assert_eq!(OPERATION_REMOVE_HELPER, 4);
        assert_eq!(OUTCOME_FAILED, 5);
    }

    #[test]
    fn commit_request_contains_only_closed_capabilities() {
        let commit = AuthorizedSystemCommit::new(
            KnownSystemProduct::QoderWork,
            1,
            SystemCommitAction::FreshInstall,
            [1; 16],
            [2; 32],
            [3; 32],
            7,
        )
        .unwrap();
        let request = FyAgentPrivilegedRequest::commit(&commit);
        assert_eq!(request.operation, OPERATION_COMMIT);
        assert_eq!(request.product, KnownSystemProduct::QoderWork as u32);
        assert_eq!(request.target_slot, 1);
        assert_eq!(request.source_directory_fd, 7);
        assert_eq!(request.expected_source_revision, [2; 32]);
        assert_eq!(request.expected_target_revision, [3; 32]);
    }

    #[test]
    fn missing_client_fail_closes_to_helper_not_packaged() {
        if linked() {
            return;
        }
        let err = invoke(&FyAgentPrivilegedRequest::status_query());
        assert_eq!(err, Err(AgentReasonCode::HelperNotPackaged));
    }

    #[test]
    fn helper_reason_mapping_is_closed() {
        assert_eq!(reason_code(0), None);
        assert_eq!(
            reason_code(9),
            Some(AgentReasonCode::OperationAuthorizationCancelled)
        );
        assert_eq!(reason_code(19), Some(AgentReasonCode::RollbackRestored));
        assert_eq!(
            reason_code(999),
            Some(AgentReasonCode::HelperProtocolIncompatible)
        );
    }
}
