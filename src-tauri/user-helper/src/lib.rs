//! Minimal protocol and executable boundary for the FyAgent current-user
//! Windows package helper.
//!
//! The portable modules deliberately own all parsing, path derivation, and
//! wire validation. Native calls live only in the executable's private
//! Windows module, so a normal library dependency cannot reach deployment.

pub mod bridge_control;
pub mod cli;
pub mod grok;
pub mod grok_npm;
pub mod layout;
pub mod protocol;

pub use bridge_control::{
    BridgeControlError, BridgeOperationId, PackageBridgeControl, BRIDGE_CONTROL_BYTES,
    BRIDGE_CONTROL_VERSION, BRIDGE_OPERATION_ID_BYTES,
};
pub use cli::{
    parse_cli_args, AgentInstallerProduct, CanonicalJobId, CliError, InstallRequest, PipeNonce,
    UserHelperAction, AGENT_EXE_INSTALL_ACTION, GROK_TOOL_ACTION, INSTALL_ACTION,
};
pub use grok::{
    GrokOutcome, GrokOwner, GrokOwnerObservation, GrokPlanFailure, GrokPlanKind, GrokToolAction,
    ToolOperationResult, GROK_NPM_PACKAGE, TOOL_OPERATION_STARTED_IDENTITY,
};
pub use grok_npm::{
    decode_plan_control, encode_plan_control, npm_install_argv_or_reject, GrokNpmInstallPlan,
    GrokNpmPlanError, GrokNpmRegistry, GROK_NPM_PLAN_CONTROL_BYTES, GROK_NPM_REGISTRY_ENV,
};
pub use layout::{
    admission_event_name, cancel_event_name, derive_install_layout, InstallLayout, LayoutError,
    PackageBridgeArtifactKind, AGENT_INSTALLER_FILE_NAME, AGENT_PACKAGE_BRIDGE_PART_FILE_NAME,
    INSTALLER_FILE_NAME, PACKAGE_BRIDGE_PART_FILE_NAME, PACKAGE_BRIDGE_ROOT_DIRECTORY,
    PACKAGE_BRIDGE_VERSION_DIRECTORY,
};
pub use protocol::{
    decode_frame, decode_frame_length, encode_frame, helper_error_code_for_deployment_hresult,
    HelperErrorCode, HelperMessage, HelperProtocolAction, HelperProtocolSequence,
    HelperProtocolTerminal, PinnedPackageIdentity, ProtocolError, ProtocolSequenceError,
    FRAME_LENGTH_BYTES, MAX_ERROR_MESSAGE_BYTES, MAX_FRAME_BYTES, MAX_PAYLOAD_BYTES,
    MAX_PROTOCOL_MESSAGES, PROTOCOL_VERSION,
};

/// A helper runtime failure exits with this dedicated code only after it has
/// either not created a PackageManager operation or observed that operation's
/// true terminal state and attempted to close it. This is diagnostic only:
/// the parent releases its package pin solely after a valid terminal frame and
/// clean pipe close, never from a process exit code.
pub const SETTLED_FAILURE_EXIT_CODE: u8 = 10;
