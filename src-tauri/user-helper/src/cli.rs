use std::{ffi::OsString, fmt};

use crate::grok::{GrokOwner, GrokToolAction};

pub const INSTALL_ACTION: &str = "codex-msix-install";
pub const AGENT_EXE_INSTALL_ACTION: &str = "agent-exe-install";
pub const GROK_TOOL_ACTION: &str = "grok-tool";
const PRODUCT_FLAG: &str = "--product";
const ACTION_FLAG: &str = "--action";
const OWNER_FLAG: &str = "--owner";
const JOB_ID_FLAG: &str = "--job-id";
const PIPE_FLAG: &str = "--pipe";
const JOB_ID_BYTES: usize = 36;
const PIPE_NONCE_BYTES: usize = 64;
const GROK_TOOL_WIRE_BASE: u8 = 5;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonicalJobId(String);

impl CanonicalJobId {
    pub fn parse(value: &str) -> Result<Self, CliError> {
        if is_canonical_lowercase_uuid(value.as_bytes()) {
            Ok(Self(value.to_owned()))
        } else {
            Err(CliError::InvalidJobId)
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CanonicalJobId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct PipeNonce(String);

impl PipeNonce {
    pub fn parse(value: &str) -> Result<Self, CliError> {
        if value.len() == PIPE_NONCE_BYTES
            && value
                .as_bytes()
                .iter()
                .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
        {
            Ok(Self(value.to_owned()))
        } else {
            Err(CliError::InvalidPipeNonce)
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for PipeNonce {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("PipeNonce([redacted])")
    }
}

pub struct InstallRequest {
    action: UserHelperAction,
    job_id: CanonicalJobId,
    pipe_nonce: PipeNonce,
}

impl InstallRequest {
    pub const fn action(&self) -> UserHelperAction {
        self.action
    }

    pub fn job_id(&self) -> &CanonicalJobId {
        &self.job_id
    }

    pub fn pipe_nonce(&self) -> &PipeNonce {
        &self.pipe_nonce
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AgentInstallerProduct {
    QoderWork,
    TraeWork,
    WorkBuddy,
}

impl AgentInstallerProduct {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::QoderWork => "qoderwork",
            Self::TraeWork => "trae-work",
            Self::WorkBuddy => "workbuddy",
        }
    }

    fn parse(value: &str) -> Result<Self, CliError> {
        match value {
            "qoderwork" => Ok(Self::QoderWork),
            "trae-work" => Ok(Self::TraeWork),
            "workbuddy" => Ok(Self::WorkBuddy),
            _ => Err(CliError::InvalidProduct),
        }
    }
}

impl fmt::Display for AgentInstallerProduct {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UserHelperAction {
    CodexMsixInstall,
    AgentExeInstall(AgentInstallerProduct),
    GrokTool {
        action: GrokToolAction,
        expected_owner: Option<GrokOwner>,
    },
}

impl UserHelperAction {
    pub const fn wire_code(self) -> u8 {
        match self {
            Self::CodexMsixInstall => 1,
            Self::AgentExeInstall(AgentInstallerProduct::QoderWork) => 2,
            Self::AgentExeInstall(AgentInstallerProduct::TraeWork) => 3,
            Self::AgentExeInstall(AgentInstallerProduct::WorkBuddy) => 4,
            Self::GrokTool {
                action,
                expected_owner,
            } => grok_tool_wire_code(action, expected_owner),
        }
    }

    pub const fn from_wire(value: u8) -> Option<Self> {
        match value {
            1 => Some(Self::CodexMsixInstall),
            2 => Some(Self::AgentExeInstall(AgentInstallerProduct::QoderWork)),
            3 => Some(Self::AgentExeInstall(AgentInstallerProduct::TraeWork)),
            4 => Some(Self::AgentExeInstall(AgentInstallerProduct::WorkBuddy)),
            5..=13 => grok_tool_from_wire(value),
            _ => None,
        }
    }

    pub const fn requires_package_bridge(self) -> bool {
        !matches!(self, Self::GrokTool { .. })
    }

    pub const fn artifact_kind(self) -> crate::layout::PackageBridgeArtifactKind {
        match self {
            Self::CodexMsixInstall => crate::layout::PackageBridgeArtifactKind::Msix,
            Self::AgentExeInstall(_) | Self::GrokTool { .. } => {
                crate::layout::PackageBridgeArtifactKind::Exe
            }
        }
    }

    pub fn command_line(self, job_id: &CanonicalJobId, pipe_nonce: &PipeNonce) -> String {
        match self {
            Self::CodexMsixInstall => format!(
                "{INSTALL_ACTION} --job-id {job_id} --pipe {}",
                pipe_nonce.as_str()
            ),
            Self::AgentExeInstall(product) => format!(
                "{AGENT_EXE_INSTALL_ACTION} --product {product} --job-id {job_id} --pipe {}",
                pipe_nonce.as_str()
            ),
            Self::GrokTool {
                action,
                expected_owner: None,
            } => format!(
                "{GROK_TOOL_ACTION} --action {} --job-id {job_id} --pipe {}",
                action.as_str(),
                pipe_nonce.as_str()
            ),
            Self::GrokTool {
                action,
                expected_owner: Some(owner),
            } => format!(
                "{GROK_TOOL_ACTION} --action {} --owner {} --job-id {job_id} --pipe {}",
                action.as_str(),
                owner.cli_token(),
                pipe_nonce.as_str()
            ),
        }
    }
}

const fn grok_tool_wire_code(action: GrokToolAction, expected_owner: Option<GrokOwner>) -> u8 {
    let action_offset = match action {
        GrokToolAction::Observe => 0,
        GrokToolAction::Install => 3,
        GrokToolAction::Update => 6,
    };
    let owner_offset = match expected_owner {
        None => 0,
        Some(GrokOwner::Native) => 1,
        Some(GrokOwner::Npm) => 2,
    };
    GROK_TOOL_WIRE_BASE + action_offset + owner_offset
}

const fn grok_tool_from_wire(value: u8) -> Option<UserHelperAction> {
    if value < GROK_TOOL_WIRE_BASE || value > 13 {
        return None;
    }
    let offset = value - GROK_TOOL_WIRE_BASE;
    let action = match offset / 3 {
        0 => GrokToolAction::Observe,
        1 => GrokToolAction::Install,
        2 => GrokToolAction::Update,
        _ => return None,
    };
    let expected_owner = match offset % 3 {
        0 => None,
        1 => Some(GrokOwner::Native),
        2 => Some(GrokOwner::Npm),
        _ => return None,
    };
    Some(UserHelperAction::GrokTool {
        action,
        expected_owner,
    })
}

impl fmt::Debug for InstallRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("InstallRequest")
            .field("job_id", &self.job_id)
            .field("pipe_nonce", &self.pipe_nonce)
            .finish()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CliError {
    WrongArgumentCount,
    NonUnicodeArgument,
    UnknownAction,
    ExpectedProductFlag,
    InvalidProduct,
    ExpectedActionFlag,
    InvalidToolAction,
    ExpectedOwnerFlag,
    InvalidOwner,
    ExpectedJobIdFlag,
    InvalidJobId,
    ExpectedPipeFlag,
    InvalidPipeNonce,
}

impl fmt::Display for CliError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::WrongArgumentCount => {
                "the helper arguments do not match a supported action shape"
            }
            Self::NonUnicodeArgument => "helper arguments must be valid Unicode",
            Self::UnknownAction => "the helper action is not supported",
            Self::ExpectedProductFlag => "--product must immediately follow the Agent EXE action",
            Self::InvalidProduct => "the Agent installer product is not supported",
            Self::ExpectedActionFlag => "--action must immediately follow the Grok tool action",
            Self::InvalidToolAction => "the helper tool action is not supported",
            Self::ExpectedOwnerFlag => "--owner must immediately follow the Grok tool action",
            Self::InvalidOwner => "the Grok distribution owner is not supported",
            Self::ExpectedJobIdFlag => "--job-id must immediately follow the action",
            Self::InvalidJobId => "job ID must be a canonical lowercase UUID",
            Self::ExpectedPipeFlag => "--pipe must immediately follow the job ID",
            Self::InvalidPipeNonce => {
                "pipe nonce must be exactly 64 lowercase hexadecimal characters"
            }
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for CliError {}

/// Parses only the arguments after the executable name.
///
/// The order is intentionally fixed. Treating the flags as a general option
/// map would accidentally admit duplicates, reordering, or future arbitrary
/// path-like options at this privilege boundary.
pub fn parse_cli_args<I, T>(args: I) -> Result<InstallRequest, CliError>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString>,
{
    let raw = args
        .into_iter()
        .map(|argument| {
            argument
                .into()
                .into_string()
                .map_err(|_| CliError::NonUnicodeArgument)
        })
        .collect::<Result<Vec<_>, _>>()?;

    let (action, job_flag_index) = match raw.first().map(String::as_str) {
        Some(INSTALL_ACTION) if raw.len() == 5 => (UserHelperAction::CodexMsixInstall, 1),
        Some(AGENT_EXE_INSTALL_ACTION) if raw.len() == 7 => {
            if raw[1] != PRODUCT_FLAG {
                return Err(CliError::ExpectedProductFlag);
            }
            (
                UserHelperAction::AgentExeInstall(AgentInstallerProduct::parse(&raw[2])?),
                3,
            )
        }
        Some(GROK_TOOL_ACTION)
            if raw.len() == 9 && raw.get(3).map(String::as_str) != Some(OWNER_FLAG) =>
        {
            if raw[3] == JOB_ID_FLAG {
                return Err(CliError::WrongArgumentCount);
            }
            return Err(CliError::ExpectedOwnerFlag);
        }
        Some(GROK_TOOL_ACTION) if raw.len() == 7 || raw.len() == 9 => {
            if raw[1] != ACTION_FLAG {
                return Err(CliError::ExpectedActionFlag);
            }
            let grok_action =
                GrokToolAction::parse_cli(&raw[2]).ok_or(CliError::InvalidToolAction)?;
            let (expected_owner, job_index) = if raw.len() == 7 {
                (None, 3)
            } else {
                let owner = match raw[4].as_str() {
                    "none" => None,
                    other => Some(GrokOwner::parse_cli(other).ok_or(CliError::InvalidOwner)?),
                };
                (owner, 5)
            };
            (
                UserHelperAction::GrokTool {
                    action: grok_action,
                    expected_owner,
                },
                job_index,
            )
        }
        Some(INSTALL_ACTION | AGENT_EXE_INSTALL_ACTION | GROK_TOOL_ACTION) => {
            return Err(CliError::WrongArgumentCount)
        }
        Some(_) => return Err(CliError::UnknownAction),
        None => return Err(CliError::WrongArgumentCount),
    };
    if raw[job_flag_index] != JOB_ID_FLAG {
        return Err(CliError::ExpectedJobIdFlag);
    }
    let job_id = CanonicalJobId::parse(&raw[job_flag_index + 1])?;
    if raw[job_flag_index + 2] != PIPE_FLAG {
        return Err(CliError::ExpectedPipeFlag);
    }
    let pipe_nonce = PipeNonce::parse(&raw[job_flag_index + 3])?;

    Ok(InstallRequest {
        action,
        job_id,
        pipe_nonce,
    })
}

fn is_canonical_lowercase_uuid(value: &[u8]) -> bool {
    if value.len() != JOB_ID_BYTES {
        return false;
    }

    value.iter().enumerate().all(|(index, byte)| {
        if matches!(index, 8 | 13 | 18 | 23) {
            *byte == b'-'
        } else {
            byte.is_ascii_digit() || matches!(byte, b'a'..=b'f')
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const JOB_ID: &str = "123e4567-e89b-12d3-a456-426614174000";
    const NONCE: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    fn valid_args() -> [&'static str; 5] {
        [INSTALL_ACTION, JOB_ID_FLAG, JOB_ID, PIPE_FLAG, NONCE]
    }

    #[test]
    fn accepts_only_the_exact_cli_shape() {
        let request = parse_cli_args(valid_args()).expect("the exact helper CLI must parse");
        assert_eq!(request.action(), UserHelperAction::CodexMsixInstall);
        assert_eq!(request.job_id().as_str(), JOB_ID);
        assert_eq!(request.pipe_nonce().as_str(), NONCE);
    }

    #[test]
    fn accepts_only_closed_agent_exe_products() {
        for (value, product) in [
            ("qoderwork", AgentInstallerProduct::QoderWork),
            ("trae-work", AgentInstallerProduct::TraeWork),
            ("workbuddy", AgentInstallerProduct::WorkBuddy),
        ] {
            let request = parse_cli_args([
                AGENT_EXE_INSTALL_ACTION,
                PRODUCT_FLAG,
                value,
                JOB_ID_FLAG,
                JOB_ID,
                PIPE_FLAG,
                NONCE,
            ])
            .expect("closed Agent EXE action");
            assert_eq!(request.action(), UserHelperAction::AgentExeInstall(product));
        }
        assert_eq!(
            parse_cli_args([
                AGENT_EXE_INSTALL_ACTION,
                PRODUCT_FLAG,
                "unknown",
                JOB_ID_FLAG,
                JOB_ID,
                PIPE_FLAG,
                NONCE,
            ])
            .unwrap_err(),
            CliError::InvalidProduct
        );
    }

    #[test]
    fn rejects_missing_and_extra_arguments() {
        assert_eq!(
            parse_cli_args(valid_args()[..4].iter().copied()).unwrap_err(),
            CliError::WrongArgumentCount
        );

        let mut extra = valid_args().to_vec();
        extra.push("unexpected");
        assert_eq!(
            parse_cli_args(extra).unwrap_err(),
            CliError::WrongArgumentCount
        );
    }

    #[test]
    fn rejects_unknown_actions() {
        let mut args = valid_args();
        args[0] = "run";
        assert_eq!(parse_cli_args(args).unwrap_err(), CliError::UnknownAction);
    }

    #[test]
    fn rejects_unknown_path_or_command_options() {
        for option in ["--path", "--program", "--command", "--uri", "--scope"] {
            let mut args = valid_args();
            args[1] = option;
            assert_eq!(
                parse_cli_args(args).unwrap_err(),
                CliError::ExpectedJobIdFlag
            );
        }
    }

    #[test]
    fn agent_action_rejects_path_command_and_scope_fields() {
        for option in ["--path", "--program", "--command", "--uri", "--scope"] {
            let args = [
                AGENT_EXE_INSTALL_ACTION,
                option,
                "qoderwork",
                JOB_ID_FLAG,
                JOB_ID,
                PIPE_FLAG,
                NONCE,
            ];
            assert_eq!(
                parse_cli_args(args).unwrap_err(),
                CliError::ExpectedProductFlag
            );
        }
    }

    #[test]
    fn helper_action_builds_only_fixed_argument_shapes() {
        let job_id = CanonicalJobId::parse(JOB_ID).unwrap();
        let nonce = PipeNonce::parse(NONCE).unwrap();
        assert_eq!(
            UserHelperAction::CodexMsixInstall.command_line(&job_id, &nonce),
            format!("{INSTALL_ACTION} --job-id {JOB_ID} --pipe {NONCE}")
        );
        assert_eq!(
            UserHelperAction::AgentExeInstall(AgentInstallerProduct::WorkBuddy)
                .command_line(&job_id, &nonce),
            format!(
                "{AGENT_EXE_INSTALL_ACTION} --product workbuddy --job-id {JOB_ID} --pipe {NONCE}"
            )
        );
        assert_eq!(UserHelperAction::CodexMsixInstall.wire_code(), 1);
        assert_eq!(
            UserHelperAction::from_wire(4),
            Some(UserHelperAction::AgentExeInstall(
                AgentInstallerProduct::WorkBuddy
            ))
        );
        assert_eq!(UserHelperAction::from_wire(0), None);
        assert_eq!(
            UserHelperAction::from_wire(5),
            Some(UserHelperAction::GrokTool {
                action: GrokToolAction::Observe,
                expected_owner: None,
            })
        );
        assert_eq!(
            UserHelperAction::from_wire(13),
            Some(UserHelperAction::GrokTool {
                action: GrokToolAction::Update,
                expected_owner: Some(GrokOwner::Npm),
            })
        );
        assert_eq!(UserHelperAction::from_wire(14), None);
        assert_eq!(
            UserHelperAction::GrokTool {
                action: GrokToolAction::Install,
                expected_owner: Some(GrokOwner::Native),
            }
            .command_line(&job_id, &nonce),
            format!(
                "{GROK_TOOL_ACTION} --action install --owner native --job-id {JOB_ID} --pipe {NONCE}"
            )
        );
        assert!(!UserHelperAction::GrokTool {
            action: GrokToolAction::Observe,
            expected_owner: None,
        }
        .requires_package_bridge());
    }

    #[test]
    fn rejects_reordered_and_duplicate_flags() {
        let reordered = [INSTALL_ACTION, PIPE_FLAG, NONCE, JOB_ID_FLAG, JOB_ID];
        assert_eq!(
            parse_cli_args(reordered).unwrap_err(),
            CliError::ExpectedJobIdFlag
        );

        let duplicate = [INSTALL_ACTION, JOB_ID_FLAG, JOB_ID, JOB_ID_FLAG, JOB_ID];
        assert_eq!(
            parse_cli_args(duplicate).unwrap_err(),
            CliError::ExpectedPipeFlag
        );
    }

    #[test]
    fn rejects_noncanonical_job_ids() {
        for job_id in [
            "123e4567-e89b-12d3-a456-42661417400",
            "123E4567-e89b-12d3-a456-426614174000",
            "123e4567e89b12d3a456426614174000",
            "123e4567-e89b-12d3-a456-42661417400g",
            "{123e4567-e89b-12d3-a456-426614174000}",
            "../../outside/cache/installer.msix",
        ] {
            let args = [INSTALL_ACTION, JOB_ID_FLAG, job_id, PIPE_FLAG, NONCE];
            assert_eq!(
                parse_cli_args(args).unwrap_err(),
                CliError::InvalidJobId,
                "unexpectedly accepted {job_id:?}"
            );
        }
    }

    #[test]
    fn rejects_invalid_pipe_nonces() {
        let invalid = [
            "0123456789abcdef",
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcde",
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0",
            "0123456789ABCDEF0123456789abcdef0123456789abcdef0123456789abcdef",
            "g123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            "../../pipe/0123456789abcdef0123456789abcdef0123456789abcdef0123456789",
        ];
        for nonce in invalid {
            let args = [INSTALL_ACTION, JOB_ID_FLAG, JOB_ID, PIPE_FLAG, nonce];
            assert_eq!(
                parse_cli_args(args).unwrap_err(),
                CliError::InvalidPipeNonce,
                "unexpectedly accepted {nonce:?}"
            );
        }
    }

    #[test]
    fn debug_output_redacts_the_pipe_capability() {
        let request = parse_cli_args(valid_args()).expect("valid request");
        let debug = format!("{request:?}");
        assert!(debug.contains(JOB_ID));
        assert!(debug.contains("[redacted]"));
        assert!(!debug.contains(NONCE));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn rejects_non_unicode_arguments() {
        use std::os::unix::ffi::OsStringExt;

        let args = [
            OsString::from(INSTALL_ACTION),
            OsString::from(JOB_ID_FLAG),
            OsString::from_vec(vec![0xff]),
            OsString::from(PIPE_FLAG),
            OsString::from(NONCE),
        ];
        assert_eq!(
            parse_cli_args(args).unwrap_err(),
            CliError::NonUnicodeArgument
        );
    }

    #[test]
    fn accepts_closed_grok_tool_shapes_and_rejects_unknown_owner_or_action() {
        let observe = parse_cli_args([
            GROK_TOOL_ACTION,
            ACTION_FLAG,
            "observe",
            JOB_ID_FLAG,
            JOB_ID,
            PIPE_FLAG,
            NONCE,
        ])
        .expect("grok observe");
        assert_eq!(
            observe.action(),
            UserHelperAction::GrokTool {
                action: GrokToolAction::Observe,
                expected_owner: None,
            }
        );

        let update = parse_cli_args([
            GROK_TOOL_ACTION,
            ACTION_FLAG,
            "update",
            OWNER_FLAG,
            "native",
            JOB_ID_FLAG,
            JOB_ID,
            PIPE_FLAG,
            NONCE,
        ])
        .expect("grok update native");
        assert_eq!(
            update.action(),
            UserHelperAction::GrokTool {
                action: GrokToolAction::Update,
                expected_owner: Some(GrokOwner::Native),
            }
        );

        assert_eq!(
            parse_cli_args([
                GROK_TOOL_ACTION,
                ACTION_FLAG,
                "install",
                OWNER_FLAG,
                "none",
                JOB_ID_FLAG,
                JOB_ID,
                PIPE_FLAG,
                NONCE,
            ])
            .expect("explicit none owner")
            .action(),
            UserHelperAction::GrokTool {
                action: GrokToolAction::Install,
                expected_owner: None,
            }
        );

        assert_eq!(
            parse_cli_args([
                GROK_TOOL_ACTION,
                ACTION_FLAG,
                "repair",
                JOB_ID_FLAG,
                JOB_ID,
                PIPE_FLAG,
                NONCE,
            ])
            .unwrap_err(),
            CliError::InvalidToolAction
        );
        assert_eq!(
            parse_cli_args([
                GROK_TOOL_ACTION,
                ACTION_FLAG,
                "install",
                OWNER_FLAG,
                "winget",
                JOB_ID_FLAG,
                JOB_ID,
                PIPE_FLAG,
                NONCE,
            ])
            .unwrap_err(),
            CliError::InvalidOwner
        );
        assert_eq!(
            parse_cli_args([
                GROK_TOOL_ACTION,
                "--command",
                "observe",
                JOB_ID_FLAG,
                JOB_ID,
                PIPE_FLAG,
                NONCE,
            ])
            .unwrap_err(),
            CliError::ExpectedActionFlag
        );
        assert_eq!(
            parse_cli_args([
                GROK_TOOL_ACTION,
                ACTION_FLAG,
                "observe",
                "--cwd",
                JOB_ID,
                PIPE_FLAG,
                NONCE,
            ])
            .unwrap_err(),
            CliError::ExpectedJobIdFlag
        );
        let mut extra = vec![
            GROK_TOOL_ACTION,
            ACTION_FLAG,
            "observe",
            JOB_ID_FLAG,
            JOB_ID,
            PIPE_FLAG,
            NONCE,
            "unexpected",
        ];
        extra.push("/tmp/script.ps1");
        assert_eq!(
            parse_cli_args(extra).unwrap_err(),
            CliError::WrongArgumentCount
        );
    }
}
