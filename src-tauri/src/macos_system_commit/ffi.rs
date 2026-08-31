//! C ABI for the in-process Swift privileged client.
//!
//! The Swift dylib is not linked in this slice. Invoke fail-closes to
//! `helper_not_packaged` instead of `#[link]`-ing an absent image (which
//! would break `cargo test`). A later slice may enable linking behind a
//! dedicated cfg once `libFyAgentPrivilegedClient.dylib` is embedded.

use crate::agent_install::AgentReasonCode;

use super::types::{ABI_VERSION, PROTOCOL_VERSION};

pub const OPERATION_STATUS: u32 = 1;
pub const OPERATION_ENSURE_HELPER: u32 = 2;
pub const OPERATION_COMMIT: u32 = 3;
pub const OPERATION_REMOVE_HELPER: u32 = 4;

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
    pub fn status_query() -> Self {
        Self {
            abi_version: ABI_VERSION,
            size: std::mem::size_of::<Self>() as u32,
            protocol_version: PROTOCOL_VERSION,
            operation: OPERATION_STATUS,
            action: 0,
            product: 0,
            target_slot: 0,
            reserved0: 0,
            operation_id: [0; 16],
            expected_source_revision: [0; 32],
            expected_target_revision: [0; 32],
            source_directory_fd: -1,
            reserved1: 0,
        }
    }
}

/// Invoke the in-process Swift bridge. Missing packaging maps to a closed
/// helper reason and never panics.
pub fn invoke(
    request: &FyAgentPrivilegedRequest,
) -> Result<FyAgentPrivilegedReply, AgentReasonCode> {
    let _ = request;
    // Linking `FyAgentPrivilegedClient` is disabled in this slice so
    // unsigned/debug `cargo test` does not require the Swift image.
    Err(AgentReasonCode::HelperNotPackaged)
}

#[cfg(test)]
mod tests {
    use super::*;

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
    }

    #[test]
    fn missing_client_fail_closes_to_helper_not_packaged() {
        let err = invoke(&FyAgentPrivilegedRequest::status_query());
        assert_eq!(err, Err(AgentReasonCode::HelperNotPackaged));
    }
}
