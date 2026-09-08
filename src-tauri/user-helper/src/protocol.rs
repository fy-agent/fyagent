use std::{fmt, mem::size_of};

use crate::cli::UserHelperAction;
use crate::grok::{GrokOutcome, GrokOwner, ToolOperationResult};

pub const PROTOCOL_VERSION: u8 = 3;
pub const FRAME_LENGTH_BYTES: usize = 4;
pub const MAX_ERROR_MESSAGE_BYTES: usize = 256;
pub const MAX_PAYLOAD_BYTES: usize = 2 + 1 + 2 + MAX_ERROR_MESSAGE_BYTES;
pub const MAX_FRAME_BYTES: usize = FRAME_LENGTH_BYTES + MAX_PAYLOAD_BYTES;
pub const MAX_PROTOCOL_MESSAGES: usize = 104;

const STARTED_KIND: u8 = 1;
const PROGRESS_KIND: u8 = 2;
const SUCCESS_KIND: u8 = 3;
const ERROR_KIND: u8 = 4;
const HELLO_KIND: u8 = 5;
const TOOL_RESULT_KIND: u8 = 6;
const STARTED_IDENTITY_BYTES: usize = 3 * size_of::<u64>();
const TOOL_RESULT_FIXED_BYTES: usize = 4;
const MAX_TOOL_VERSION_BYTES: usize = crate::grok::MAX_NORMALIZED_VERSION_BYTES;

/// Opaque identity for the helper's no-follow, handle-relative MSIX pin.
///
/// The parent compares this fixed-width value with its independently hashed
/// pin before signaling admission. Paths and other user-controlled strings do
/// not cross the pipe.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PinnedPackageIdentity {
    volume_serial: u64,
    file_index: u64,
    size: u64,
}

impl PinnedPackageIdentity {
    pub const fn new(volume_serial: u64, file_index: u64, size: u64) -> Self {
        Self {
            volume_serial,
            file_index,
            size,
        }
    }

    pub const fn volume_serial(self) -> u64 {
        self.volume_serial
    }

    pub const fn file_index(self) -> u64 {
        self.file_index
    }

    pub const fn size(self) -> u64 {
        self.size
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum HelperErrorCode {
    InstallLayoutInvalid = 1,
    WinRtInitializationFailed = 2,
    PackageUriInvalid = 3,
    PackageManagerUnavailable = 4,
    PackageInUse = 5,
    DeploymentBlocked = 6,
    DependencyMissing = 7,
    SignatureInvalid = 8,
    PackageInvalid = 9,
    DeploymentFailed = 10,
    DeploymentResultInvalid = 11,
    ParentAdmissionFailed = 12,
    ParentCancelled = 13,
    DeploymentTimedOut = 14,
    PackageDowngrade = 15,
    InstallerLaunchFailed = 16,
    InstallerCancelled = 17,
    InstallerTimedOut = 18,
    InstallerProcessUnobservable = 19,
    InstallerExitedNonzero = 20,
    ToolHostMissing = 21,
    ToolTimedOut = 22,
    ToolOutputLimit = 23,
    ToolOwnerMismatch = 24,
    ToolNotDetected = 25,
    ToolExecutionFailed = 26,
}

impl HelperErrorCode {
    pub const ALL: [Self; 26] = [
        Self::InstallLayoutInvalid,
        Self::WinRtInitializationFailed,
        Self::PackageUriInvalid,
        Self::PackageManagerUnavailable,
        Self::PackageInUse,
        Self::DeploymentBlocked,
        Self::DependencyMissing,
        Self::SignatureInvalid,
        Self::PackageInvalid,
        Self::DeploymentFailed,
        Self::DeploymentResultInvalid,
        Self::ParentAdmissionFailed,
        Self::ParentCancelled,
        Self::DeploymentTimedOut,
        Self::PackageDowngrade,
        Self::InstallerLaunchFailed,
        Self::InstallerCancelled,
        Self::InstallerTimedOut,
        Self::InstallerProcessUnobservable,
        Self::InstallerExitedNonzero,
        Self::ToolHostMissing,
        Self::ToolTimedOut,
        Self::ToolOutputLimit,
        Self::ToolOwnerMismatch,
        Self::ToolNotDetected,
        Self::ToolExecutionFailed,
    ];

    pub const fn wire_code(self) -> u8 {
        self as u8
    }

    pub const fn redacted_message(self) -> &'static str {
        match self {
            Self::InstallLayoutInvalid => "The installed helper layout is invalid",
            Self::WinRtInitializationFailed => "Windows package services could not initialize",
            Self::PackageUriInvalid => "The fixed package source URI is invalid",
            Self::PackageManagerUnavailable => "Windows PackageManager is unavailable",
            Self::PackageInUse => "Codex is in use and must be closed before installation",
            Self::DeploymentBlocked => "Windows policy blocked the current-user installation",
            Self::DependencyMissing => "A required Windows package dependency is missing",
            Self::SignatureInvalid => "Windows rejected the package signature",
            Self::PackageInvalid => "Windows rejected the package contents",
            Self::DeploymentFailed => "Windows PackageManager deployment failed",
            Self::DeploymentResultInvalid => {
                "Windows did not register the package for the current user"
            }
            Self::ParentAdmissionFailed => "FyAgent did not admit the package helper",
            Self::ParentCancelled => "FyAgent cancelled the package installation",
            Self::DeploymentTimedOut => "Windows package installation timed out",
            Self::PackageDowngrade => "Windows rejected an older package version",
            Self::InstallerLaunchFailed => "Windows could not start the verified installer",
            Self::InstallerCancelled => "The user cancelled the Windows installer launch",
            Self::InstallerTimedOut => "The Windows installer did not finish before the deadline",
            Self::InstallerProcessUnobservable => {
                "Windows did not return an observable installer process"
            }
            Self::InstallerExitedNonzero => "The Windows installer exited with a failure status",
            Self::ToolHostMissing => "The official Grok Build host is unavailable",
            Self::ToolTimedOut => "The Grok Build operation did not finish before the deadline",
            Self::ToolOutputLimit => "The Grok Build operation exceeded its output limit",
            Self::ToolOwnerMismatch => "The Grok Build installation owner does not match",
            Self::ToolNotDetected => "Grok Build is not installed for the current user",
            Self::ToolExecutionFailed => "The Grok Build operation failed",
        }
    }

    fn from_wire(value: u8) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|candidate| candidate.wire_code() == value)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HelperMessage {
    Hello {
        action: UserHelperAction,
    },
    Started {
        package: PinnedPackageIdentity,
    },
    Progress {
        completed: u8,
    },
    Success,
    ToolResult(ToolOperationResult),
    Error {
        code: HelperErrorCode,
        message: String,
    },
}

impl HelperMessage {
    pub fn error(code: HelperErrorCode) -> Self {
        Self::Error {
            code,
            message: code.redacted_message().to_owned(),
        }
    }
}

/// Maps only stable Windows deployment categories into the bounded helper
/// error enum. Raw HRESULT values never cross the pipe.
pub fn helper_error_code_for_deployment_hresult(value: i32) -> HelperErrorCode {
    match value as u32 {
        // ERROR_PACKAGES_IN_USE is retryable after closing the target app.
        0x8007_3D02 => HelperErrorCode::PackageInUse,
        // ERROR_INSTALL_PACKAGE_DOWNGRADE must remain distinct so the parent
        // can refresh release metadata instead of asking the user to close the
        // app or retry the same older package.
        0x8007_3D06 => HelperErrorCode::PackageDowngrade,
        0x8007_3CFF | 0x8007_3D01 | 0x8007_3D19 | 0x8007_3D21 | 0x8007_3D22 | 0x8007_3D23
        | 0x8007_0005 => HelperErrorCode::DeploymentBlocked,
        0x8007_3CF3 | 0x8007_3CFD => HelperErrorCode::DependencyMissing,
        0x800B_0100 | 0x800B_0109 | 0x800B_010A | 0x800B_0004 => HelperErrorCode::SignatureInvalid,
        0x8008_0204..=0x8008_0207 => HelperErrorCode::PackageInvalid,
        _ => HelperErrorCode::DeploymentFailed,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProtocolError {
    FrameTooShort,
    PayloadTooLarge,
    TruncatedFrame,
    TrailingBytes,
    UnsupportedVersion,
    UnknownMessageKind,
    InvalidMessageLength,
    InvalidProgress,
    UnknownErrorCode,
    ErrorMessageTooLong,
    EmptyErrorMessage,
    ErrorMessageContainsControl,
    InvalidUtf8,
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::FrameTooShort => "helper protocol frame is shorter than its length prefix",
            Self::PayloadTooLarge => "helper protocol payload exceeds its absolute limit",
            Self::TruncatedFrame => "helper protocol frame is truncated",
            Self::TrailingBytes => "helper protocol frame has trailing bytes",
            Self::UnsupportedVersion => "helper protocol version is unsupported",
            Self::UnknownMessageKind => "helper protocol message kind is unknown",
            Self::InvalidMessageLength => "helper protocol message has an invalid length",
            Self::InvalidProgress => "helper protocol progress is outside 0..=100",
            Self::UnknownErrorCode => "helper protocol error code is unknown",
            Self::ErrorMessageTooLong => "helper protocol error message is too long",
            Self::EmptyErrorMessage => "helper protocol error message is empty",
            Self::ErrorMessageContainsControl => {
                "helper protocol error message contains a control character"
            }
            Self::InvalidUtf8 => "helper protocol error message is not valid UTF-8",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for ProtocolError {}

pub fn encode_frame(message: &HelperMessage) -> Result<Vec<u8>, ProtocolError> {
    let mut payload = Vec::with_capacity(MAX_PAYLOAD_BYTES);
    payload.push(PROTOCOL_VERSION);

    match message {
        HelperMessage::Hello { action } => {
            payload.push(HELLO_KIND);
            payload.push(action.wire_code());
        }
        HelperMessage::Started { package } => {
            payload.push(STARTED_KIND);
            payload.extend_from_slice(&package.volume_serial().to_le_bytes());
            payload.extend_from_slice(&package.file_index().to_le_bytes());
            payload.extend_from_slice(&package.size().to_le_bytes());
        }
        HelperMessage::Progress { completed } => {
            if *completed > 100 {
                return Err(ProtocolError::InvalidProgress);
            }
            payload.push(PROGRESS_KIND);
            payload.push(*completed);
        }
        HelperMessage::Success => payload.push(SUCCESS_KIND),
        HelperMessage::ToolResult(result) => {
            encode_tool_result(&mut payload, result)?;
        }
        HelperMessage::Error { code, message } => {
            validate_error_message(message)?;
            payload.push(ERROR_KIND);
            payload.push(code.wire_code());
            payload.extend_from_slice(&(message.len() as u16).to_le_bytes());
            payload.extend_from_slice(message.as_bytes());
        }
    }

    if payload.len() > MAX_PAYLOAD_BYTES {
        return Err(ProtocolError::PayloadTooLarge);
    }
    let mut frame = Vec::with_capacity(FRAME_LENGTH_BYTES + payload.len());
    frame.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    frame.extend_from_slice(&payload);
    Ok(frame)
}

pub fn decode_frame_length(prefix: [u8; FRAME_LENGTH_BYTES]) -> Result<usize, ProtocolError> {
    let length = u32::from_le_bytes(prefix) as usize;
    if length > MAX_PAYLOAD_BYTES {
        Err(ProtocolError::PayloadTooLarge)
    } else {
        Ok(length)
    }
}

pub fn decode_frame(frame: &[u8]) -> Result<HelperMessage, ProtocolError> {
    if frame.len() < FRAME_LENGTH_BYTES {
        return Err(ProtocolError::FrameTooShort);
    }
    let mut prefix = [0_u8; FRAME_LENGTH_BYTES];
    prefix.copy_from_slice(&frame[..FRAME_LENGTH_BYTES]);
    let payload_length = decode_frame_length(prefix)?;
    let expected_length = FRAME_LENGTH_BYTES + payload_length;
    if frame.len() < expected_length {
        return Err(ProtocolError::TruncatedFrame);
    }
    if frame.len() > expected_length {
        return Err(ProtocolError::TrailingBytes);
    }
    if payload_length < 2 {
        return Err(ProtocolError::InvalidMessageLength);
    }

    let payload = &frame[FRAME_LENGTH_BYTES..];
    if payload[0] != PROTOCOL_VERSION {
        return Err(ProtocolError::UnsupportedVersion);
    }

    match payload[1] {
        HELLO_KIND if payload.len() == 3 => {
            let action =
                UserHelperAction::from_wire(payload[2]).ok_or(ProtocolError::UnknownMessageKind)?;
            Ok(HelperMessage::Hello { action })
        }
        STARTED_KIND if payload.len() == 2 + STARTED_IDENTITY_BYTES => {
            let mut field = [0_u8; size_of::<u64>()];
            field.copy_from_slice(&payload[2..10]);
            let volume_serial = u64::from_le_bytes(field);
            field.copy_from_slice(&payload[10..18]);
            let file_index = u64::from_le_bytes(field);
            field.copy_from_slice(&payload[18..26]);
            let size = u64::from_le_bytes(field);
            Ok(HelperMessage::Started {
                package: PinnedPackageIdentity::new(volume_serial, file_index, size),
            })
        }
        PROGRESS_KIND if payload.len() == 3 => {
            let completed = payload[2];
            if completed > 100 {
                Err(ProtocolError::InvalidProgress)
            } else {
                Ok(HelperMessage::Progress { completed })
            }
        }
        SUCCESS_KIND if payload.len() == 2 => Ok(HelperMessage::Success),
        TOOL_RESULT_KIND => decode_tool_result_payload(payload),
        ERROR_KIND => decode_error_payload(payload),
        HELLO_KIND | STARTED_KIND | PROGRESS_KIND | SUCCESS_KIND => {
            Err(ProtocolError::InvalidMessageLength)
        }
        _ => Err(ProtocolError::UnknownMessageKind),
    }
}

fn encode_tool_result(
    payload: &mut Vec<u8>,
    result: &ToolOperationResult,
) -> Result<(), ProtocolError> {
    let version = result.normalized_version.as_deref().unwrap_or("");
    if version.len() > MAX_TOOL_VERSION_BYTES {
        return Err(ProtocolError::ErrorMessageTooLong);
    }
    if !version
        .bytes()
        .all(|byte| byte.is_ascii() && !byte.is_ascii_control())
    {
        return Err(ProtocolError::ErrorMessageContainsControl);
    }
    payload.push(TOOL_RESULT_KIND);
    payload.push(u8::from(result.detected));
    payload.push(result.owner.map(GrokOwner::wire).unwrap_or(0));
    payload.push(result.outcome.wire());
    payload.push(version.len() as u8);
    payload.extend_from_slice(version.as_bytes());
    Ok(())
}

fn decode_tool_result_payload(payload: &[u8]) -> Result<HelperMessage, ProtocolError> {
    if payload.len() < 2 + TOOL_RESULT_FIXED_BYTES {
        return Err(ProtocolError::InvalidMessageLength);
    }
    let version_len = payload[5] as usize;
    if version_len > MAX_TOOL_VERSION_BYTES {
        return Err(ProtocolError::ErrorMessageTooLong);
    }
    if payload.len() != 2 + TOOL_RESULT_FIXED_BYTES + version_len {
        return Err(ProtocolError::InvalidMessageLength);
    }
    let detected = match payload[2] {
        0 => false,
        1 => true,
        _ => return Err(ProtocolError::InvalidMessageLength),
    };
    let owner = match payload[3] {
        0 => None,
        value => Some(GrokOwner::from_wire(value).ok_or(ProtocolError::UnknownMessageKind)?),
    };
    let outcome = GrokOutcome::from_wire(payload[4]).ok_or(ProtocolError::UnknownMessageKind)?;
    let version = if version_len == 0 {
        None
    } else {
        let text = std::str::from_utf8(&payload[6..]).map_err(|_| ProtocolError::InvalidUtf8)?;
        if text.chars().any(char::is_control) {
            return Err(ProtocolError::ErrorMessageContainsControl);
        }
        Some(text.to_owned())
    };
    Ok(HelperMessage::ToolResult(ToolOperationResult {
        detected,
        normalized_version: version,
        owner,
        outcome,
    }))
}

fn decode_error_payload(payload: &[u8]) -> Result<HelperMessage, ProtocolError> {
    if payload.len() < 5 {
        return Err(ProtocolError::InvalidMessageLength);
    }
    let code = HelperErrorCode::from_wire(payload[2]).ok_or(ProtocolError::UnknownErrorCode)?;
    let message_length = u16::from_le_bytes([payload[3], payload[4]]) as usize;
    if message_length > MAX_ERROR_MESSAGE_BYTES {
        return Err(ProtocolError::ErrorMessageTooLong);
    }
    if payload.len() != 5 + message_length {
        return Err(ProtocolError::InvalidMessageLength);
    }
    let message = std::str::from_utf8(&payload[5..]).map_err(|_| ProtocolError::InvalidUtf8)?;
    validate_error_message(message)?;
    Ok(HelperMessage::Error {
        code,
        message: message.to_owned(),
    })
}

fn validate_error_message(message: &str) -> Result<(), ProtocolError> {
    if message.len() > MAX_ERROR_MESSAGE_BYTES {
        return Err(ProtocolError::ErrorMessageTooLong);
    }
    if message.trim().is_empty() {
        return Err(ProtocolError::EmptyErrorMessage);
    }
    if message.chars().any(char::is_control) {
        return Err(ProtocolError::ErrorMessageContainsControl);
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HelperProtocolTerminal {
    Success,
    ToolSuccess(ToolOperationResult),
    Failure(HelperErrorCode),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HelperProtocolAction {
    Hello(UserHelperAction),
    Started(PinnedPackageIdentity),
    Progress(u8),
    Success,
    ToolResult(ToolOperationResult),
    Failure(HelperErrorCode),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProtocolSequenceError {
    MessageLimitExceeded,
    UnexpectedMessage,
    NonCanonicalError,
    ProgressRegression,
    ControlOutOfOrder,
    AdmissionOutOfOrder,
}

impl fmt::Display for ProtocolSequenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::MessageLimitExceeded => "helper protocol exceeded its bounded message count",
            Self::UnexpectedMessage => "helper protocol message arrived out of order",
            Self::NonCanonicalError => "helper protocol error text was not canonical",
            Self::ProgressRegression => "helper protocol progress did not strictly increase",
            Self::ControlOutOfOrder => "helper bridge control was sent out of order",
            Self::AdmissionOutOfOrder => "helper admission was signaled out of order",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for ProtocolSequenceError {}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum ProtocolPhase {
    #[default]
    AwaitingHello,
    AwaitingControl,
    AwaitingStarted,
    AwaitingAdmission,
    Running,
    Terminal,
}

/// Portable parent-side ordering gate for the one-shot helper protocol.
///
/// The caller authenticates the raw `Hello` frame before accepting it, marks
/// the exact bridge-control write, validates `Started` against its own bridge
/// pin, and finally marks the BA-owned admission signal. Keeping those native
/// transitions explicit prevents a valid frame from skipping an external
/// security check.
#[derive(Debug, Default)]
pub struct HelperProtocolSequence {
    message_count: usize,
    phase: ProtocolPhase,
    last_progress: Option<u8>,
    terminal: Option<HelperProtocolTerminal>,
}

impl HelperProtocolSequence {
    pub fn accept(
        &mut self,
        message: HelperMessage,
    ) -> Result<HelperProtocolAction, ProtocolSequenceError> {
        if self.message_count == MAX_PROTOCOL_MESSAGES {
            return Err(ProtocolSequenceError::MessageLimitExceeded);
        }
        self.message_count += 1;

        match (self.phase, message) {
            (ProtocolPhase::AwaitingHello, HelperMessage::Hello { action }) => {
                self.phase = ProtocolPhase::AwaitingControl;
                Ok(HelperProtocolAction::Hello(action))
            }
            (ProtocolPhase::AwaitingStarted, HelperMessage::Started { package }) => {
                self.phase = ProtocolPhase::AwaitingAdmission;
                Ok(HelperProtocolAction::Started(package))
            }
            (ProtocolPhase::AwaitingStarted, HelperMessage::Error { code, message })
            | (ProtocolPhase::AwaitingAdmission, HelperMessage::Error { code, message })
            | (ProtocolPhase::Running, HelperMessage::Error { code, message }) => {
                if message != code.redacted_message() {
                    return Err(ProtocolSequenceError::NonCanonicalError);
                }
                self.phase = ProtocolPhase::Terminal;
                self.terminal = Some(HelperProtocolTerminal::Failure(code));
                Ok(HelperProtocolAction::Failure(code))
            }
            (ProtocolPhase::Running, HelperMessage::Progress { completed }) => {
                if self
                    .last_progress
                    .is_some_and(|previous| completed <= previous)
                {
                    return Err(ProtocolSequenceError::ProgressRegression);
                }
                self.last_progress = Some(completed);
                Ok(HelperProtocolAction::Progress(completed))
            }
            (ProtocolPhase::Running, HelperMessage::Success) => {
                self.phase = ProtocolPhase::Terminal;
                self.terminal = Some(HelperProtocolTerminal::Success);
                Ok(HelperProtocolAction::Success)
            }
            (ProtocolPhase::Running, HelperMessage::ToolResult(result)) => {
                self.phase = ProtocolPhase::Terminal;
                self.terminal = Some(HelperProtocolTerminal::ToolSuccess(result.clone()));
                Ok(HelperProtocolAction::ToolResult(result))
            }
            _ => Err(ProtocolSequenceError::UnexpectedMessage),
        }
    }

    pub fn mark_control_sent(&mut self) -> Result<(), ProtocolSequenceError> {
        if self.phase != ProtocolPhase::AwaitingControl {
            return Err(ProtocolSequenceError::ControlOutOfOrder);
        }
        self.phase = ProtocolPhase::AwaitingStarted;
        Ok(())
    }

    pub fn mark_admitted(&mut self) -> Result<(), ProtocolSequenceError> {
        if self.phase != ProtocolPhase::AwaitingAdmission {
            return Err(ProtocolSequenceError::AdmissionOutOfOrder);
        }
        self.phase = ProtocolPhase::Running;
        Ok(())
    }

    pub fn terminal(&self) -> Option<HelperProtocolTerminal> {
        self.terminal.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HELLO_ACTION: UserHelperAction = UserHelperAction::CodexMsixInstall;
    const PINNED_PACKAGE: PinnedPackageIdentity = PinnedPackageIdentity::new(
        0x0102_0304_0506_0708,
        0x1112_1314_1516_1718,
        0x2122_2324_2526_2728,
    );

    #[test]
    fn exact_wire_codes_are_stable_and_unique() {
        assert_eq!(PROTOCOL_VERSION, 3);
        assert_eq!(
            HelperErrorCode::ALL.map(HelperErrorCode::wire_code),
            [
                1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
                24, 25, 26,
            ]
        );
        assert_eq!(encode_frame(&hello()).unwrap(), [3, 0, 0, 0, 3, 5, 1]);
        assert_eq!(
            encode_frame(&HelperMessage::Started {
                package: PINNED_PACKAGE,
            })
            .unwrap(),
            [
                26, 0, 0, 0, 3, 1, 8, 7, 6, 5, 4, 3, 2, 1, 24, 23, 22, 21, 20, 19, 18, 17, 40, 39,
                38, 37, 36, 35, 34, 33,
            ]
        );
        assert_eq!(
            encode_frame(&HelperMessage::Progress { completed: 42 }).unwrap(),
            [3, 0, 0, 0, 3, 2, 42]
        );
        assert_eq!(
            encode_frame(&HelperMessage::Success).unwrap(),
            [2, 0, 0, 0, 3, 3]
        );
    }

    #[test]
    fn every_message_and_error_enum_round_trips() {
        let mut messages = vec![
            hello(),
            HelperMessage::Hello {
                action: UserHelperAction::AgentExeInstall(
                    crate::cli::AgentInstallerProduct::QoderWork,
                ),
            },
            HelperMessage::Hello {
                action: UserHelperAction::AgentExeInstall(
                    crate::cli::AgentInstallerProduct::TraeWork,
                ),
            },
            HelperMessage::Hello {
                action: UserHelperAction::AgentExeInstall(
                    crate::cli::AgentInstallerProduct::WorkBuddy,
                ),
            },
            HelperMessage::Hello {
                action: UserHelperAction::AgentExeInstall(
                    crate::cli::AgentInstallerProduct::OpenCode,
                ),
            },
            HelperMessage::Started {
                package: PINNED_PACKAGE,
            },
            HelperMessage::Progress { completed: 0 },
            HelperMessage::Progress { completed: 50 },
            HelperMessage::Progress { completed: 100 },
            HelperMessage::Success,
            HelperMessage::Hello {
                action: UserHelperAction::GrokTool {
                    action: crate::grok::GrokToolAction::Observe,
                    expected_owner: None,
                },
            },
            HelperMessage::ToolResult(ToolOperationResult {
                detected: true,
                normalized_version: Some("1.2.3".to_owned()),
                owner: Some(crate::grok::GrokOwner::Native),
                outcome: crate::grok::GrokOutcome::Observed,
            }),
        ];
        messages.extend(HelperErrorCode::ALL.map(HelperMessage::error));

        for message in messages {
            let encoded = encode_frame(&message).expect("known message must encode");
            assert!(encoded.len() <= MAX_FRAME_BYTES);
            assert_eq!(decode_frame(&encoded).unwrap(), message);
        }
    }

    #[test]
    fn maximum_error_message_fits_the_absolute_frame_bound() {
        let message = HelperMessage::Error {
            code: HelperErrorCode::DeploymentFailed,
            message: "x".repeat(MAX_ERROR_MESSAGE_BYTES),
        };
        let encoded = encode_frame(&message).expect("maximum message must fit");
        assert_eq!(encoded.len(), MAX_FRAME_BYTES);
        assert_eq!(decode_frame(&encoded).unwrap(), message);

        let oversized = HelperMessage::Error {
            code: HelperErrorCode::DeploymentFailed,
            message: "x".repeat(MAX_ERROR_MESSAGE_BYTES + 1),
        };
        assert_eq!(
            encode_frame(&oversized).unwrap_err(),
            ProtocolError::ErrorMessageTooLong
        );
    }

    #[test]
    fn rejects_invalid_progress_on_encode_and_decode() {
        assert_eq!(
            encode_frame(&HelperMessage::Progress { completed: 101 }).unwrap_err(),
            ProtocolError::InvalidProgress
        );
        let invalid = [3, 0, 0, 0, PROTOCOL_VERSION, PROGRESS_KIND, 101];
        assert_eq!(
            decode_frame(&invalid).unwrap_err(),
            ProtocolError::InvalidProgress
        );
    }

    #[test]
    fn rejects_unknown_versions_variants_and_error_codes() {
        assert_eq!(
            decode_frame(&[
                26,
                0,
                0,
                0,
                1,
                STARTED_KIND,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
            ])
            .unwrap_err(),
            ProtocolError::UnsupportedVersion
        );
        assert_eq!(
            decode_frame(&[2, 0, 0, 0, PROTOCOL_VERSION, 99]).unwrap_err(),
            ProtocolError::UnknownMessageKind
        );
        assert_eq!(
            decode_frame(&[6, 0, 0, 0, PROTOCOL_VERSION, ERROR_KIND, 99, 1, 0, b'x']).unwrap_err(),
            ProtocolError::UnknownErrorCode
        );
    }

    #[test]
    fn rejects_short_truncated_trailing_and_oversized_frames() {
        assert_eq!(
            decode_frame(&[0, 0, 0]).unwrap_err(),
            ProtocolError::FrameTooShort
        );
        assert_eq!(
            decode_frame(&[2, 0, 0, 0, PROTOCOL_VERSION]).unwrap_err(),
            ProtocolError::TruncatedFrame
        );
        assert_eq!(
            decode_frame(&[
                26,
                0,
                0,
                0,
                PROTOCOL_VERSION,
                STARTED_KIND,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
            ])
            .unwrap_err(),
            ProtocolError::TrailingBytes
        );
        assert_eq!(
            decode_frame_length(((MAX_PAYLOAD_BYTES + 1) as u32).to_le_bytes()).unwrap_err(),
            ProtocolError::PayloadTooLarge
        );
    }

    #[test]
    fn rejects_wrong_lengths_and_malformed_utf8() {
        assert_eq!(
            decode_frame(&[2, 0, 0, 0, PROTOCOL_VERSION, HELLO_KIND]).unwrap_err(),
            ProtocolError::InvalidMessageLength
        );
        assert_eq!(
            decode_frame(&[3, 0, 0, 0, PROTOCOL_VERSION, HELLO_KIND, 0]).unwrap_err(),
            ProtocolError::UnknownMessageKind
        );
        assert_eq!(
            decode_frame(&[3, 0, 0, 0, PROTOCOL_VERSION, STARTED_KIND, 0]).unwrap_err(),
            ProtocolError::InvalidMessageLength
        );
        let malformed = [6, 0, 0, 0, PROTOCOL_VERSION, ERROR_KIND, 1, 1, 0, 0xff];
        assert_eq!(
            decode_frame(&malformed).unwrap_err(),
            ProtocolError::InvalidUtf8
        );
    }

    #[test]
    fn started_identity_requires_exact_fixed_width_little_endian_fields() {
        let message = HelperMessage::Started {
            package: PINNED_PACKAGE,
        };
        assert_eq!(
            decode_frame(&encode_frame(&message).unwrap()).unwrap(),
            message
        );

        let truncated = [
            25,
            0,
            0,
            0,
            PROTOCOL_VERSION,
            STARTED_KIND,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
        ];
        assert_eq!(
            decode_frame(&truncated).unwrap_err(),
            ProtocolError::InvalidMessageLength
        );

        let encoded = encode_frame(&message).unwrap();
        assert_eq!(
            &encoded[6..14],
            &PINNED_PACKAGE.volume_serial().to_le_bytes()
        );
        assert_eq!(&encoded[14..22], &PINNED_PACKAGE.file_index().to_le_bytes());
        assert_eq!(&encoded[22..30], &PINNED_PACKAGE.size().to_le_bytes());
    }

    #[test]
    fn rejects_empty_or_control_character_error_text() {
        for message in ["", "   ", "line one\nline two", "prefix\0suffix"] {
            assert!(encode_frame(&HelperMessage::Error {
                code: HelperErrorCode::DeploymentFailed,
                message: message.to_owned(),
            })
            .is_err());
        }
    }

    #[test]
    fn deployment_hresult_mapping_keeps_package_in_use_and_downgrade_distinct() {
        let cases = [
            (0x8007_3D02_u32 as i32, HelperErrorCode::PackageInUse),
            (0x8007_3D06_u32 as i32, HelperErrorCode::PackageDowngrade),
            (0x8007_3CFF_u32 as i32, HelperErrorCode::DeploymentBlocked),
            (0x8007_3CF3_u32 as i32, HelperErrorCode::DependencyMissing),
            (0x800B_0100_u32 as i32, HelperErrorCode::SignatureInvalid),
            (0x8007_3CF0_u32 as i32, HelperErrorCode::DeploymentFailed),
            (0x8008_0205_u32 as i32, HelperErrorCode::PackageInvalid),
            (0x8123_4567_u32 as i32, HelperErrorCode::DeploymentFailed),
        ];
        for (hresult, expected) in cases {
            assert_eq!(helper_error_code_for_deployment_hresult(hresult), expected);
        }
    }

    fn started() -> HelperMessage {
        HelperMessage::Started {
            package: PINNED_PACKAGE,
        }
    }

    fn hello() -> HelperMessage {
        HelperMessage::Hello {
            action: HELLO_ACTION,
        }
    }

    fn admit(sequence: &mut HelperProtocolSequence) {
        assert_eq!(
            sequence.accept(hello()),
            Ok(HelperProtocolAction::Hello(HELLO_ACTION))
        );
        sequence
            .mark_control_sent()
            .expect("control follows authenticated hello");
        assert_eq!(
            sequence.accept(started()),
            Ok(HelperProtocolAction::Started(PINNED_PACKAGE))
        );
        sequence
            .mark_admitted()
            .expect("admission follows exact bridge identity proof");
    }

    #[test]
    fn sequence_requires_hello_control_started_and_admission_in_order() {
        let mut sequence = HelperProtocolSequence::default();
        assert_eq!(
            sequence.accept(started()),
            Err(ProtocolSequenceError::UnexpectedMessage)
        );

        let mut sequence = HelperProtocolSequence::default();
        assert_eq!(
            sequence.mark_control_sent(),
            Err(ProtocolSequenceError::ControlOutOfOrder)
        );
        assert_eq!(
            sequence.mark_admitted(),
            Err(ProtocolSequenceError::AdmissionOutOfOrder)
        );
        assert_eq!(
            sequence.accept(hello()),
            Ok(HelperProtocolAction::Hello(HELLO_ACTION))
        );
        assert_eq!(
            sequence.accept(hello()),
            Err(ProtocolSequenceError::UnexpectedMessage)
        );
        assert_eq!(
            sequence.accept(started()),
            Err(ProtocolSequenceError::UnexpectedMessage)
        );
        sequence.mark_control_sent().unwrap();
        assert_eq!(
            sequence.accept(started()),
            Ok(HelperProtocolAction::Started(PINNED_PACKAGE))
        );
        assert_eq!(
            sequence.accept(HelperMessage::Progress { completed: 0 }),
            Err(ProtocolSequenceError::UnexpectedMessage)
        );
        sequence.mark_admitted().unwrap();
        assert_eq!(
            sequence.accept(HelperMessage::Progress { completed: 0 }),
            Ok(HelperProtocolAction::Progress(0))
        );
    }

    #[test]
    fn sequence_allows_only_canonical_failure_before_started_or_admission() {
        for after_started in [false, true] {
            let mut sequence = HelperProtocolSequence::default();
            sequence.accept(hello()).unwrap();
            sequence.mark_control_sent().unwrap();
            if after_started {
                sequence.accept(started()).unwrap();
            }
            assert_eq!(
                sequence.accept(HelperMessage::error(HelperErrorCode::PackageInvalid)),
                Ok(HelperProtocolAction::Failure(
                    HelperErrorCode::PackageInvalid
                ))
            );
            assert_eq!(
                sequence.terminal(),
                Some(HelperProtocolTerminal::Failure(
                    HelperErrorCode::PackageInvalid
                ))
            );
        }

        let mut before_control = HelperProtocolSequence::default();
        before_control.accept(hello()).unwrap();
        assert_eq!(
            before_control.accept(HelperMessage::error(HelperErrorCode::PackageInvalid)),
            Err(ProtocolSequenceError::UnexpectedMessage)
        );

        let mut noncanonical = HelperProtocolSequence::default();
        noncanonical.accept(hello()).unwrap();
        noncanonical.mark_control_sent().unwrap();
        assert_eq!(
            noncanonical.accept(HelperMessage::Error {
                code: HelperErrorCode::PackageInvalid,
                message: "untrusted detail".to_owned(),
            }),
            Err(ProtocolSequenceError::NonCanonicalError)
        );
    }

    #[test]
    fn sequence_requires_strict_progress_and_one_terminal() {
        let mut sequence = HelperProtocolSequence::default();
        admit(&mut sequence);
        assert_eq!(
            sequence.accept(HelperMessage::Progress { completed: 0 }),
            Ok(HelperProtocolAction::Progress(0))
        );
        assert_eq!(
            sequence.accept(HelperMessage::Progress { completed: 0 }),
            Err(ProtocolSequenceError::ProgressRegression)
        );
        assert_eq!(
            sequence.accept(HelperMessage::Progress { completed: 50 }),
            Ok(HelperProtocolAction::Progress(50))
        );
        assert_eq!(
            sequence.accept(HelperMessage::Success),
            Ok(HelperProtocolAction::Success)
        );
        assert_eq!(sequence.terminal(), Some(HelperProtocolTerminal::Success));
        assert_eq!(
            sequence.accept(HelperMessage::Success),
            Err(ProtocolSequenceError::UnexpectedMessage)
        );
    }

    #[test]
    fn grok_tool_result_is_a_distinct_terminal_from_empty_success() {
        let mut sequence = HelperProtocolSequence::default();
        sequence
            .accept(HelperMessage::Hello {
                action: UserHelperAction::GrokTool {
                    action: crate::grok::GrokToolAction::Observe,
                    expected_owner: None,
                },
            })
            .unwrap();
        sequence.mark_control_sent().unwrap();
        let started = sequence
            .accept(HelperMessage::Started {
                package: crate::grok::TOOL_OPERATION_STARTED_IDENTITY,
            })
            .unwrap();
        assert!(matches!(started, HelperProtocolAction::Started(_)));
        sequence.mark_admitted().unwrap();
        let result = ToolOperationResult {
            detected: false,
            normalized_version: None,
            owner: None,
            outcome: crate::grok::GrokOutcome::Observed,
        };
        assert_eq!(
            sequence.accept(HelperMessage::ToolResult(result.clone())),
            Ok(HelperProtocolAction::ToolResult(result.clone()))
        );
        assert_eq!(
            sequence.terminal(),
            Some(HelperProtocolTerminal::ToolSuccess(result))
        );
        assert_eq!(
            sequence.accept(HelperMessage::Success),
            Err(ProtocolSequenceError::UnexpectedMessage)
        );
    }

    #[test]
    fn maximum_legal_sequence_is_exactly_104_messages() {
        let mut sequence = HelperProtocolSequence::default();
        admit(&mut sequence);
        for completed in 0..=100 {
            assert_eq!(
                sequence.accept(HelperMessage::Progress { completed }),
                Ok(HelperProtocolAction::Progress(completed))
            );
        }
        assert_eq!(
            sequence.accept(HelperMessage::Success),
            Ok(HelperProtocolAction::Success)
        );
        assert_eq!(MAX_PROTOCOL_MESSAGES, 104);
        assert_eq!(
            sequence.accept(HelperMessage::Success),
            Err(ProtocolSequenceError::MessageLimitExceeded)
        );
    }
}
