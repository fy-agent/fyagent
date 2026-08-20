use std::{ffi::OsString, fmt};

pub const INSTALL_ACTION: &str = "codex-msix-install";
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
    job_id: CanonicalJobId,
    pipe_nonce: PipeNonce,
}

impl InstallRequest {
    pub fn job_id(&self) -> &CanonicalJobId {
        &self.job_id
    }

    pub fn pipe_nonce(&self) -> &PipeNonce {
        &self.pipe_nonce
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
    ExpectedJobIdFlag,
    InvalidJobId,
    ExpectedPipeFlag,
    InvalidPipeNonce,
}

impl fmt::Display for CliError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::WrongArgumentCount => "the helper requires exactly five arguments",
            Self::NonUnicodeArgument => "helper arguments must be valid Unicode",
            Self::UnknownAction => "the helper action is not supported",
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
    let mut iterator = args.into_iter();
    let mut raw = Vec::with_capacity(5);
    for _ in 0..5 {
        raw.push(
            iterator
                .next()
                .ok_or(CliError::WrongArgumentCount)?
                .into()
                .into_string()
                .map_err(|_| CliError::NonUnicodeArgument)?,
        );
    }
    if iterator.next().is_some() {
        return Err(CliError::WrongArgumentCount);
    }

    if raw[0] != INSTALL_ACTION {
        return Err(CliError::UnknownAction);
    }
    if raw[1] != JOB_ID_FLAG {
        return Err(CliError::ExpectedJobIdFlag);
    }
    let job_id = CanonicalJobId::parse(&raw[2])?;
    if raw[3] != PIPE_FLAG {
        return Err(CliError::ExpectedPipeFlag);
    }
    let pipe_nonce = PipeNonce::parse(&raw[4])?;

    Ok(InstallRequest { job_id, pipe_nonce })
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
        assert_eq!(request.job_id().as_str(), JOB_ID);
        assert_eq!(request.pipe_nonce().as_str(), NONCE);
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
