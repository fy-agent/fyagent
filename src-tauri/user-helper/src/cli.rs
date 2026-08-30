use std::{ffi::OsString, fmt};

pub const INSTALL_ACTION: &str = "codex-msix-install";
pub const AGENT_EXE_INSTALL_ACTION: &str = "agent-exe-install";
const PRODUCT_FLAG: &str = "--product";
const JOB_ID_FLAG: &str = "--job-id";
const PIPE_FLAG: &str = "--pipe";
const JOB_ID_BYTES: usize = 36;
const PIPE_NONCE_BYTES: usize = 64;

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
}

impl UserHelperAction {
    pub const fn wire_code(self) -> u8 {
        match self {
            Self::CodexMsixInstall => 1,
            Self::AgentExeInstall(AgentInstallerProduct::QoderWork) => 2,
            Self::AgentExeInstall(AgentInstallerProduct::TraeWork) => 3,
            Self::AgentExeInstall(AgentInstallerProduct::WorkBuddy) => 4,
        }
    }

    pub const fn from_wire(value: u8) -> Option<Self> {
        match value {
            1 => Some(Self::CodexMsixInstall),
            2 => Some(Self::AgentExeInstall(AgentInstallerProduct::QoderWork)),
            3 => Some(Self::AgentExeInstall(AgentInstallerProduct::TraeWork)),
            4 => Some(Self::AgentExeInstall(AgentInstallerProduct::WorkBuddy)),
            _ => None,
        }
    }

    pub const fn artifact_kind(self) -> crate::layout::PackageBridgeArtifactKind {
        match self {
            Self::CodexMsixInstall => crate::layout::PackageBridgeArtifactKind::Msix,
            Self::AgentExeInstall(_) => crate::layout::PackageBridgeArtifactKind::Exe,
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
        }
    }
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
        Some(INSTALL_ACTION | AGENT_EXE_INSTALL_ACTION) => {
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
}
