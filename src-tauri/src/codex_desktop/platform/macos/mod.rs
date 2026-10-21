//! macOS Apple-Silicon installer adapter.
//!
//! This module deliberately uses only a small command and filesystem boundary.
//! The adapter can therefore be tested with fakes on every host without
//! mounting a disk image or touching a real Applications directory. Platform
//! root code chooses whether this adapter is available for the current target.

pub(crate) mod bundle;
pub(crate) mod dmg;
mod signature;

use std::{
    collections::HashMap,
    ffi::OsString,
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use futures::future::BoxFuture;

use super::{
    CodexDesktopPlatform, PlatformInstallPlan, PlatformProgressSink, PreparedInstallPackage,
    RestartCandidateInspection, RuntimeInspection, TrustedRuntimeInstance,
    MACOS_CODEX_STABLE_IDENTITY,
};
use crate::codex_desktop::{
    download::DownloadedArtifact,
    error::{InstallerError, InstallerErrorCode},
    types::{
        CpuArchitecture, DesktopPlatform, InstalledApplication, LocalInstallStatus,
        ReleaseDescriptor, UnsupportedReason,
    },
    verify::{DiskSpaceProbe, DiskSpaceProbeError, VolumeKey},
};

const COMMAND_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_COMMAND_OUTPUT_BYTES: usize = 16 * 1024;
const MACOS_MINIMUM_MAJOR_VERSION: u16 = 14;

/// Fixed invocation record passed to the narrow system-command boundary.
/// Programs are static literals and arguments are independent OS strings; no
/// adapter path ever constructs a shell command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandInvocation {
    program: &'static str,
    arguments: Vec<OsString>,
    timeout: Duration,
}

impl CommandInvocation {
    pub fn new(
        program: &'static str,
        arguments: impl IntoIterator<Item = OsString>,
        timeout: Duration,
    ) -> Self {
        assert!(
            program.starts_with('/'),
            "macOS commands require absolute paths"
        );
        Self {
            program,
            arguments: arguments.into_iter().collect(),
            timeout,
        }
    }

    pub fn program(&self) -> &'static str {
        self.program
    }

    pub fn arguments(&self) -> &[OsString] {
        &self.arguments
    }

    pub fn timeout(&self) -> Duration {
        self.timeout
    }
}

/// Bounded command result. Callers interpret a nonzero status in the context
/// of the operation instead of exposing command stderr as a user-facing error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutput {
    status_code: Option<i32>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    stdout_truncated: bool,
    stderr_truncated: bool,
}

impl CommandOutput {
    pub fn success(stdout: impl Into<Vec<u8>>) -> Self {
        Self::new(Some(0), stdout.into(), Vec::new())
    }

    pub fn failure(status_code: Option<i32>, stderr: impl Into<Vec<u8>>) -> Self {
        Self::new(status_code, Vec::new(), stderr.into())
    }

    fn new(status_code: Option<i32>, stdout: Vec<u8>, stderr: Vec<u8>) -> Self {
        Self {
            status_code,
            stdout_truncated: stdout.len() > MAX_COMMAND_OUTPUT_BYTES,
            stderr_truncated: stderr.len() > MAX_COMMAND_OUTPUT_BYTES,
            stdout: bounded_output(stdout),
            stderr: bounded_output(stderr),
        }
    }

    pub fn stdout_truncated(&self) -> bool {
        self.stdout_truncated
    }

    pub fn stderr_truncated(&self) -> bool {
        self.stderr_truncated
    }

    pub fn is_success(&self) -> bool {
        self.status_code == Some(0)
    }

    pub fn stdout(&self) -> &[u8] {
        &self.stdout
    }

    pub fn stderr(&self) -> &[u8] {
        &self.stderr
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandRunnerErrorKind {
    Spawn,
    Timeout,
    Io,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandRunnerError {
    kind: CommandRunnerErrorKind,
}

impl CommandRunnerError {
    pub const fn kind(self) -> CommandRunnerErrorKind {
        self.kind
    }
}

/// All macOS system commands pass through this interface. Test runners return
/// queued, bounded outcomes and record arguments without invoking a shell.
pub trait CommandRunner: Send + Sync {
    fn run(&self, invocation: &CommandInvocation) -> Result<CommandOutput, CommandRunnerError>;
}

/// Production implementation with a bounded wall-clock wait. It is not
/// constructed or called by tests; the adapter invokes it only from a blocking
/// task so service orchestration remains asynchronous.
#[derive(Debug, Default)]
pub struct SystemCommandRunner;

impl CommandRunner for SystemCommandRunner {
    fn run(&self, invocation: &CommandInvocation) -> Result<CommandOutput, CommandRunnerError> {
        if !invocation.program.starts_with('/') {
            return Err(CommandRunnerError {
                kind: CommandRunnerErrorKind::Io,
            });
        }
        let mut child = Command::new(invocation.program)
            .args(&invocation.arguments)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(command_runner_error_from_io)?;
        // Keep only the bounded prefix while draining both pipes to EOF so a
        // verbose child cannot exhaust memory or block on a full pipe.
        let stdout_handle = child
            .stdout
            .take()
            .map(|pipe| thread::spawn(move || read_command_output(pipe)));
        let stderr_handle = child
            .stderr
            .take()
            .map(|pipe| thread::spawn(move || read_command_output(pipe)));
        let deadline = Instant::now() + invocation.timeout;
        let status_code = loop {
            match child.try_wait().map_err(command_runner_error_from_io)? {
                Some(status) => break status.code(),
                None if Instant::now() >= deadline => {
                    let _ = child.kill();
                    let _ = child.wait();
                    drop(stdout_handle);
                    drop(stderr_handle);
                    return Err(CommandRunnerError {
                        kind: CommandRunnerErrorKind::Timeout,
                    });
                }
                None => thread::sleep(Duration::from_millis(10)),
            }
        };
        type OutputReader = thread::JoinHandle<std::io::Result<(Vec<u8>, bool)>>;
        let join_output = |handle: Option<OutputReader>| {
            handle
                .ok_or(CommandRunnerError {
                    kind: CommandRunnerErrorKind::Io,
                })?
                .join()
                .map_err(|_| CommandRunnerError {
                    kind: CommandRunnerErrorKind::Io,
                })?
                .map_err(|_| CommandRunnerError {
                    kind: CommandRunnerErrorKind::Io,
                })
        };
        // Join both readers even when one reports an error.
        let stdout = join_output(stdout_handle);
        let stderr = join_output(stderr_handle);
        let (stdout, stdout_truncated) = stdout?;
        let (stderr, stderr_truncated) = stderr?;
        Ok(CommandOutput {
            status_code,
            stdout,
            stderr,
            stdout_truncated,
            stderr_truncated,
        })
    }
}

fn read_command_output<R: Read>(mut reader: R) -> std::io::Result<(Vec<u8>, bool)> {
    let mut output = Vec::with_capacity(MAX_COMMAND_OUTPUT_BYTES);
    let mut buffer = [0u8; 4096];
    let mut truncated = false;
    loop {
        let count = match reader.read(&mut buffer) {
            Ok(0) => return Ok((output, truncated)),
            Ok(count) => count,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        };
        let keep = count.min(MAX_COMMAND_OUTPUT_BYTES - output.len());
        output.extend_from_slice(&buffer[..keep]);
        truncated |= keep < count;
    }
}

fn bounded_output(mut value: Vec<u8>) -> Vec<u8> {
    value.truncate(MAX_COMMAND_OUTPUT_BYTES);
    value
}

fn command_runner_error_from_io(error: std::io::Error) -> CommandRunnerError {
    let kind = if error.kind() == std::io::ErrorKind::NotFound {
        CommandRunnerErrorKind::Spawn
    } else {
        CommandRunnerErrorKind::Io
    };
    CommandRunnerError { kind }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MacosFileKind {
    File,
    Directory,
    Symlink,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MacosFilesystemErrorKind {
    NotFound,
    PermissionDenied,
    AlreadyExists,
    Unsupported,
    Invalid,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MacosFilesystemError {
    kind: MacosFilesystemErrorKind,
}

impl MacosFilesystemError {
    pub const fn kind(self) -> MacosFilesystemErrorKind {
        self.kind
    }
}

/// Filesystem operations required by the macOS adapter. The implementation
/// deliberately has no generic arbitrary-path deletion API: all removal is
/// guarded by the DMG transaction code before it reaches this boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FileIdentity {
    dev: u64,
    inode: u64,
}

pub trait MacosFilesystem: Send + Sync {
    fn write_record_atomic(&self, path: &Path, data: &[u8]) -> Result<(), MacosFilesystemError>;
    fn remove_record(&self, path: &Path) -> Result<(), MacosFilesystemError>;

    fn identity(&self, path: &Path) -> Result<FileIdentity, MacosFilesystemError>;

    fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>, MacosFilesystemError>;

    fn read_file_bounded(
        &self,
        path: &Path,
        max_bytes: u64,
    ) -> Result<Vec<u8>, MacosFilesystemError>;

    fn canonicalize(&self, path: &Path) -> Result<PathBuf, MacosFilesystemError>;

    fn file_kind(&self, path: &Path) -> Result<MacosFileKind, MacosFilesystemError>;

    fn directory_writable(&self, path: &Path) -> bool;

    fn create_dir_all(&self, path: &Path) -> Result<(), MacosFilesystemError>;

    fn rename(&self, source: &Path, destination: &Path) -> Result<(), MacosFilesystemError>;

    /// Recovery must never replace a target created while verification runs.
    fn rename_exclusive(
        &self,
        source: &Path,
        destination: &Path,
    ) -> Result<(), MacosFilesystemError>;

    /// Atomically exchange two existing same-volume directories. Only an
    /// explicit Unsupported result permits the recoverable rename fallback.
    fn swap(&self, source: &Path, destination: &Path) -> Result<(), MacosFilesystemError>;

    fn remove_dir_all(&self, path: &Path) -> Result<(), MacosFilesystemError>;
}

/// Production filesystem adapter. It contains no macOS framework imports and
/// is usable only through the constrained path checks in `bundle` and `dmg`.
#[derive(Debug, Default)]
pub struct StdMacosFilesystem;

impl MacosFilesystem for StdMacosFilesystem {
    fn write_record_atomic(&self, path: &Path, data: &[u8]) -> Result<(), MacosFilesystemError> {
        use std::io::Write;
        let parent = path.parent().filter(|p| !p.as_os_str().is_empty());
        let Some(parent) = parent.filter(|_| path.file_name().is_some()) else {
            return Err(MacosFilesystemError {
                kind: MacosFilesystemErrorKind::Invalid,
            });
        };
        let temporary = path.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
        let result = (|| {
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)?;
            file.write_all(data)?;
            file.sync_all()?;
            fs::rename(&temporary, path)?;
            fs::File::open(parent)?.sync_all()
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result.map_err(macos_filesystem_error_from_io)
    }
    fn remove_record(&self, path: &Path) -> Result<(), MacosFilesystemError> {
        fs::remove_file(path).map_err(macos_filesystem_error_from_io)
    }

    fn identity(&self, path: &Path) -> Result<FileIdentity, MacosFilesystemError> {
        #[cfg(target_os = "macos")]
        {
            use std::os::unix::fs::MetadataExt;
            let metadata = fs::symlink_metadata(path).map_err(macos_filesystem_error_from_io)?;
            if !metadata.is_dir() || metadata.file_type().is_symlink() {
                return Err(MacosFilesystemError {
                    kind: MacosFilesystemErrorKind::Invalid,
                });
            }
            return Ok(FileIdentity {
                dev: metadata.dev(),
                inode: metadata.ino(),
            });
        }
        #[allow(unreachable_code)]
        {
            let _ = path;
            Err(MacosFilesystemError {
                kind: MacosFilesystemErrorKind::Unsupported,
            })
        }
    }

    fn directory_writable(&self, path: &Path) -> bool {
        #[cfg(target_os = "macos")]
        {
            use std::{ffi::CString, os::unix::ffi::OsStrExt};
            CString::new(path.as_os_str().as_bytes()).is_ok_and(|path| unsafe {
                libc::access(path.as_ptr(), libc::W_OK | libc::X_OK) == 0
            })
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = path;
            false
        }
    }
    fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>, MacosFilesystemError> {
        fs::read_dir(path)
            .map_err(macos_filesystem_error_from_io)?
            .map(|entry| {
                entry
                    .map(|entry| entry.path())
                    .map_err(macos_filesystem_error_from_io)
            })
            .collect()
    }

    fn read_file_bounded(
        &self,
        path: &Path,
        max_bytes: u64,
    ) -> Result<Vec<u8>, MacosFilesystemError> {
        let metadata = fs::symlink_metadata(path).map_err(macos_filesystem_error_from_io)?;
        if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() > max_bytes {
            return Err(MacosFilesystemError {
                kind: MacosFilesystemErrorKind::Invalid,
            });
        }
        fs::read(path).map_err(macos_filesystem_error_from_io)
    }

    fn canonicalize(&self, path: &Path) -> Result<PathBuf, MacosFilesystemError> {
        fs::canonicalize(path).map_err(macos_filesystem_error_from_io)
    }

    fn file_kind(&self, path: &Path) -> Result<MacosFileKind, MacosFilesystemError> {
        let metadata = fs::symlink_metadata(path).map_err(macos_filesystem_error_from_io)?;
        let file_type = metadata.file_type();
        Ok(if file_type.is_symlink() {
            MacosFileKind::Symlink
        } else if file_type.is_dir() {
            MacosFileKind::Directory
        } else if file_type.is_file() {
            MacosFileKind::File
        } else {
            MacosFileKind::Other
        })
    }

    fn create_dir_all(&self, path: &Path) -> Result<(), MacosFilesystemError> {
        fs::create_dir_all(path).map_err(macos_filesystem_error_from_io)
    }

    fn rename(&self, source: &Path, destination: &Path) -> Result<(), MacosFilesystemError> {
        fs::rename(source, destination).map_err(macos_filesystem_error_from_io)
    }

    fn rename_exclusive(
        &self,
        source: &Path,
        destination: &Path,
    ) -> Result<(), MacosFilesystemError> {
        #[cfg(target_os = "macos")]
        {
            use std::{ffi::CString, os::unix::ffi::OsStrExt};
            let path = |p: &Path| {
                CString::new(p.as_os_str().as_bytes()).map_err(|_| MacosFilesystemError {
                    kind: MacosFilesystemErrorKind::Invalid,
                })
            };
            let source = path(source)?;
            let destination = path(destination)?;
            return if unsafe {
                libc::renamex_np(source.as_ptr(), destination.as_ptr(), libc::RENAME_EXCL)
            } == 0
            {
                Ok(())
            } else {
                Err(macos_filesystem_error_from_io(
                    std::io::Error::last_os_error(),
                ))
            };
        }
        #[allow(unreachable_code)]
        {
            let _ = (source, destination);
            Err(MacosFilesystemError {
                kind: MacosFilesystemErrorKind::Unsupported,
            })
        }
    }

    fn swap(&self, source: &Path, destination: &Path) -> Result<(), MacosFilesystemError> {
        #[cfg(target_os = "macos")]
        {
            use std::{ffi::CString, os::unix::ffi::OsStrExt};
            let path = |p: &Path| {
                CString::new(p.as_os_str().as_bytes()).map_err(|_| MacosFilesystemError {
                    kind: MacosFilesystemErrorKind::Invalid,
                })
            };
            let source = path(source)?;
            let destination = path(destination)?;
            // Both C strings remain alive for the call; RENAME_SWAP never
            // exposes an interval in which the target name is missing.
            if unsafe { libc::renamex_np(source.as_ptr(), destination.as_ptr(), libc::RENAME_SWAP) }
                == 0
            {
                return Ok(());
            }
            let error = std::io::Error::last_os_error();
            if matches!(
                error.raw_os_error(),
                Some(libc::ENOTSUP | libc::EOPNOTSUPP | libc::EINVAL | libc::ENOSYS)
            ) {
                return Err(MacosFilesystemError {
                    kind: MacosFilesystemErrorKind::Unsupported,
                });
            }
            return Err(macos_filesystem_error_from_io(error));
        }
        #[allow(unreachable_code)]
        {
            let _ = (source, destination);
            Err(MacosFilesystemError {
                kind: MacosFilesystemErrorKind::Unsupported,
            })
        }
    }

    fn remove_dir_all(&self, path: &Path) -> Result<(), MacosFilesystemError> {
        fs::remove_dir_all(path).map_err(macos_filesystem_error_from_io)
    }
}

fn macos_filesystem_error_from_io(error: std::io::Error) -> MacosFilesystemError {
    let kind = match error.kind() {
        std::io::ErrorKind::NotFound => MacosFilesystemErrorKind::NotFound,
        std::io::ErrorKind::PermissionDenied => MacosFilesystemErrorKind::PermissionDenied,
        std::io::ErrorKind::AlreadyExists => MacosFilesystemErrorKind::AlreadyExists,
        std::io::ErrorKind::InvalidInput | std::io::ErrorKind::InvalidData => {
            MacosFilesystemErrorKind::Invalid
        }
        _ => MacosFilesystemErrorKind::Other,
    };
    MacosFilesystemError { kind }
}

/// macOS target-volume probe backed by the fixed `df -Pk` command. The volume
/// key is an opaque filesystem token used only for one service preflight; raw
/// mount paths and command text never leave this adapter.
pub struct MacosDiskSpaceProbe {
    runner: Arc<dyn CommandRunner>,
    snapshots: Mutex<HashMap<VolumeKey, u64>>,
}

impl MacosDiskSpaceProbe {
    pub fn new(runner: Arc<dyn CommandRunner>) -> Self {
        Self {
            runner,
            snapshots: Mutex::new(HashMap::new()),
        }
    }

    #[cfg(target_os = "macos")]
    pub fn for_current_host() -> Self {
        Self::new(Arc::new(SystemCommandRunner))
    }
}

impl DiskSpaceProbe for MacosDiskSpaceProbe {
    fn volume_key(&self, path: &Path) -> Result<VolumeKey, DiskSpaceProbeError> {
        let output = self
            .runner
            .run(&command(
                "/bin/df",
                vec![OsString::from("-Pk"), path.to_path_buf().into_os_string()],
            ))
            .map_err(|_| DiskSpaceProbeError::Unavailable)?;
        if !output.is_success() {
            return Err(DiskSpaceProbeError::Unavailable);
        }
        let (filesystem, available_blocks) =
            parse_df_snapshot(output.stdout()).ok_or(DiskSpaceProbeError::Unavailable)?;
        let available_bytes = available_blocks
            .checked_mul(1024)
            .ok_or(DiskSpaceProbeError::Unavailable)?;
        let volume = VolumeKey::new(format!("macos-df:{filesystem}"))
            .map_err(|_| DiskSpaceProbeError::InvalidVolumeKey)?;
        self.snapshots
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(volume.clone(), available_bytes);
        Ok(volume)
    }

    fn available_bytes(&self, volume: &VolumeKey) -> Result<u64, DiskSpaceProbeError> {
        self.snapshots
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(volume)
            .copied()
            .ok_or(DiskSpaceProbeError::Unavailable)
    }
}

fn parse_df_snapshot(bytes: &[u8]) -> Option<(&str, u64)> {
    let text = std::str::from_utf8(bytes).ok()?;
    let line = text.lines().skip(1).find(|line| !line.trim().is_empty())?;
    let fields = line.split_whitespace().collect::<Vec<_>>();
    if fields.len() < 6 {
        return None;
    }
    Some((fields[0], fields[3].parse().ok()?))
}

/// Parsed macOS version with numeric comparison semantics. It intentionally
/// does not use a display-version or lexical comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct MacosVersion {
    major: u16,
    minor: u16,
    patch: u16,
}

impl MacosVersion {
    pub(crate) fn parse(value: &str) -> Result<Self, InstallerError> {
        let mut parts = value.split('.');
        let parse_part = |part: Option<&str>| -> Result<u16, InstallerError> {
            let part = part.ok_or_else(|| {
                InstallerError::new(InstallerErrorCode::OsVersionUnsupported)
                    .with_diagnostic_message("macOS version is not a numeric version")
            })?;
            if part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_digit()) {
                return Err(
                    InstallerError::new(InstallerErrorCode::OsVersionUnsupported)
                        .with_diagnostic_message("macOS version is not a numeric version"),
                );
            }
            part.parse::<u16>().map_err(|_| {
                InstallerError::new(InstallerErrorCode::OsVersionUnsupported)
                    .with_diagnostic_message("macOS version is outside the supported range")
            })
        };

        let major = parse_part(parts.next())?;
        let minor = match parts.next() {
            Some(value) => parse_part(Some(value))?,
            None => 0,
        };
        let patch = match parts.next() {
            Some(value) => parse_part(Some(value))?,
            None => 0,
        };
        if parts.next().is_some() {
            return Err(
                InstallerError::new(InstallerErrorCode::OsVersionUnsupported)
                    .with_diagnostic_message("macOS version has too many components"),
            );
        }

        Ok(Self {
            major,
            minor,
            patch,
        })
    }
}

/// Host facts are injected so unit tests never need to inspect the actual
/// workstation. The Applications roots are trusted local configuration, not
/// renderer input or mirror metadata.
#[derive(Debug, Clone)]
pub struct MacosHost {
    architecture: CpuArchitecture,
    os_version: MacosVersion,
    applications_dir: PathBuf,
    user_applications_dir: PathBuf,
}

impl MacosHost {
    pub fn new(
        architecture: CpuArchitecture,
        os_version: &str,
        applications_dir: PathBuf,
        user_applications_dir: PathBuf,
    ) -> Result<Self, InstallerError> {
        Ok(Self {
            architecture,
            os_version: MacosVersion::parse(os_version)?,
            applications_dir,
            user_applications_dir,
        })
    }

    #[cfg(target_os = "macos")]
    pub fn for_current_host() -> Result<Self, InstallerError> {
        let output = SystemCommandRunner
            .run(&command(
                "/usr/bin/sw_vers",
                vec![OsString::from("-productVersion")],
            ))
            .map_err(|_| {
                InstallerError::new(InstallerErrorCode::PlatformUnsupported)
                    .with_diagnostic_message("macOS version could not be detected")
            })?;
        if !output.is_success() {
            return Err(InstallerError::new(InstallerErrorCode::PlatformUnsupported)
                .with_diagnostic_message("macOS version could not be detected"));
        }
        let version = std::str::from_utf8(output.stdout()).map_err(|_| {
            InstallerError::new(InstallerErrorCode::PlatformUnsupported)
                .with_diagnostic_message("macOS version could not be decoded")
        })?;
        let home = std::env::var_os("HOME").ok_or_else(|| {
            InstallerError::new(InstallerErrorCode::PlatformUnsupported)
                .with_diagnostic_message("macOS home directory could not be determined")
        })?;
        let architecture = match std::env::consts::ARCH {
            "aarch64" => CpuArchitecture::Aarch64,
            "x86_64" => CpuArchitecture::X86_64UnsupportedMac,
            _ => CpuArchitecture::Unsupported,
        };
        Self::new(
            architecture,
            version.trim(),
            PathBuf::from("/Applications"),
            PathBuf::from(home).join("Applications"),
        )
    }

    pub(crate) fn architecture(&self) -> CpuArchitecture {
        self.architecture
    }

    pub(crate) fn os_version(&self) -> MacosVersion {
        self.os_version
    }

    pub(crate) fn applications_dir(&self) -> &Path {
        &self.applications_dir
    }

    pub(crate) fn user_applications_dir(&self) -> &Path {
        &self.user_applications_dir
    }
}

/// Apple-Silicon platform adapter. Its public constructor accepts injectable
/// host facts, command execution, and filesystem capabilities; this keeps all
/// macOS tests entirely fake and gives platform root a side-effect-free object
/// to construct during application setup.
pub(crate) struct MacosPlatformAdapter {
    runner: Arc<dyn CommandRunner>,
    filesystem: Arc<dyn MacosFilesystem>,
    host: MacosHost,
}

impl MacosPlatformAdapter {
    pub(crate) fn new(
        runner: Arc<dyn CommandRunner>,
        filesystem: Arc<dyn MacosFilesystem>,
        host: MacosHost,
    ) -> Self {
        Self {
            runner,
            filesystem,
            host,
        }
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn for_current_host() -> Result<Self, InstallerError> {
        Ok(Self::new(
            Arc::new(SystemCommandRunner),
            Arc::new(StdMacosFilesystem),
            MacosHost::for_current_host()?,
        ))
    }

    fn host_support_error(&self) -> Option<InstallerError> {
        match self.host.architecture() {
            CpuArchitecture::Aarch64
                if self.host.os_version().major >= MACOS_MINIMUM_MAJOR_VERSION =>
            {
                None
            }
            CpuArchitecture::Aarch64 => Some(
                InstallerError::new(InstallerErrorCode::OsVersionUnsupported)
                    .with_diagnostic_message("macOS 14 or later is required for the installer"),
            ),
            architecture => Some(
                InstallerError::new(InstallerErrorCode::ArchitectureUnsupported)
                    .with_context("architecture", architecture.as_str())
                    .with_diagnostic_message("macOS V1 supports Apple Silicon only"),
            ),
        }
    }
}

impl CodexDesktopPlatform for MacosPlatformAdapter {
    fn platform(&self) -> Option<DesktopPlatform> {
        Some(DesktopPlatform::Macos)
    }

    fn architecture(&self) -> CpuArchitecture {
        self.host.architecture()
    }

    fn inspect_local(&self) -> BoxFuture<'_, Result<LocalInstallStatus, InstallerError>> {
        let runner = self.runner.clone();
        let filesystem = self.filesystem.clone();
        let host = self.host.clone();
        Box::pin(async move {
            if host.architecture() != CpuArchitecture::Aarch64 {
                return Ok(LocalInstallStatus::Unsupported {
                    reason: UnsupportedReason::Architecture,
                });
            }
            if host.os_version().major < MACOS_MINIMUM_MAJOR_VERSION {
                return Ok(LocalInstallStatus::Unsupported {
                    reason: UnsupportedReason::OsVersion,
                });
            }
            run_blocking(move || bundle::inspect_local(runner.as_ref(), filesystem.as_ref(), &host))
                .await
        })
    }

    fn inspect_restart_candidates(
        &self,
    ) -> BoxFuture<'_, Result<RestartCandidateInspection, InstallerError>> {
        // v1.0.2 deliberately does not reuse the legacy macOS bundle-path
        // inspection as lifecycle authority. The target Bundle ID has not
        // received the required independent production evidence, so any
        // restart request must fail closed before it can enumerate a process,
        // send a close message, or launch an app. This also prevents a path,
        // display-name, or title fallback from creeping into the adapter.
        Box::pin(async { Ok(RestartCandidateInspection::UntrustedTarget) })
    }

    fn preflight<'a>(
        &'a self,
        release: &'a ReleaseDescriptor,
        temp_root: &'a Path,
    ) -> BoxFuture<'a, Result<PlatformInstallPlan, InstallerError>> {
        let runner = self.runner.clone();
        let filesystem = self.filesystem.clone();
        let host = self.host.clone();
        let release = release.clone();
        let temp_root = temp_root.to_path_buf();
        let host_error = self.host_support_error();
        Box::pin(async move {
            if let Some(error) = host_error {
                return Err(error);
            }
            run_blocking(move || {
                dmg::preflight(
                    runner.as_ref(),
                    filesystem.as_ref(),
                    &host,
                    &release,
                    &temp_root,
                )
            })
            .await
        })
    }

    fn prepare_install_package<'a>(
        &'a self,
        release: &'a ReleaseDescriptor,
        artifact: &'a DownloadedArtifact,
    ) -> BoxFuture<'a, Result<PreparedInstallPackage, InstallerError>> {
        let runner = self.runner.clone();
        let filesystem = self.filesystem.clone();
        let host = self.host.clone();
        let release = release.clone();
        let artifact = artifact.clone();
        let host_error = self.host_support_error();
        Box::pin(async move {
            if let Some(error) = host_error {
                return Err(error);
            }
            run_blocking(move || {
                dmg::prepare_install_package(
                    runner.as_ref(),
                    filesystem.as_ref(),
                    &host,
                    &release,
                    artifact,
                )
            })
            .await
        })
    }

    fn install_current_user<'a>(
        &'a self,
        package: &'a PreparedInstallPackage,
        progress: PlatformProgressSink,
    ) -> BoxFuture<'a, Result<Option<InstalledApplication>, InstallerError>> {
        let runner = self.runner.clone();
        let filesystem = self.filesystem.clone();
        let host = self.host.clone();
        let package = package.clone();
        let host_error = self.host_support_error();
        Box::pin(async move {
            if let Some(error) = host_error {
                return Err(error);
            }
            run_blocking(move || {
                let installed = dmg::install_current_user(
                    runner.as_ref(),
                    filesystem.as_ref(),
                    &host,
                    &package,
                    progress,
                )?;
                Ok(Some(installed))
            })
            .await
        })
    }

    fn launch<'a>(
        &'a self,
        installed: &'a InstalledApplication,
    ) -> BoxFuture<'a, Result<(), InstallerError>> {
        let runner = self.runner.clone();
        let filesystem = self.filesystem.clone();
        let host = self.host.clone();
        let installed = installed.clone();
        let host_error = self.host_support_error();
        Box::pin(async move {
            if let Some(error) = host_error {
                return Err(error);
            }
            run_blocking(move || {
                bundle::launch_verified(runner.as_ref(), filesystem.as_ref(), &host, &installed)
            })
            .await
        })
    }

    fn inspect_runtime<'a>(
        &'a self,
        installed: &'a InstalledApplication,
    ) -> BoxFuture<'a, Result<RuntimeInspection, InstallerError>> {
        let runner = self.runner.clone();
        let filesystem = self.filesystem.clone();
        let installed = installed.clone();
        let host_error = self.host_support_error();
        Box::pin(async move {
            if let Some(error) = host_error {
                return Err(error);
            }
            run_blocking(move || {
                bundle::inspect_runtime(runner.as_ref(), filesystem.as_ref(), &installed)
            })
            .await
        })
    }

    fn force_shutdown<'a>(
        &'a self,
        installed: &'a InstalledApplication,
        instances: &'a [TrustedRuntimeInstance],
    ) -> BoxFuture<'a, Result<(), InstallerError>> {
        let runner = self.runner.clone();
        let filesystem = self.filesystem.clone();
        let installed = installed.clone();
        let instances = instances.to_vec();
        let host_error = self.host_support_error();
        Box::pin(async move {
            if let Some(error) = host_error {
                return Err(error);
            }
            run_blocking(move || {
                bundle::force_shutdown(runner.as_ref(), filesystem.as_ref(), &installed, &instances)
            })
            .await
        })
    }

    fn is_runtime_instance_running<'a>(
        &'a self,
        installed: &'a InstalledApplication,
        instances: &'a [TrustedRuntimeInstance],
    ) -> BoxFuture<'a, Result<bool, InstallerError>> {
        let runner = self.runner.clone();
        let filesystem = self.filesystem.clone();
        let installed = installed.clone();
        let instances = instances.to_vec();
        let host_error = self.host_support_error();
        Box::pin(async move {
            if let Some(error) = host_error {
                return Err(error);
            }
            run_blocking(move || {
                bundle::is_runtime_instance_running(
                    runner.as_ref(),
                    filesystem.as_ref(),
                    &installed,
                    &instances,
                )
            })
            .await
        })
    }
}

async fn run_blocking<T: Send + 'static>(
    operation: impl FnOnce() -> Result<T, InstallerError> + Send + 'static,
) -> Result<T, InstallerError> {
    tokio::task::spawn_blocking(operation).await.map_err(|_| {
        InstallerError::new(InstallerErrorCode::InternalError)
            .with_diagnostic_message("macOS platform worker stopped unexpectedly")
    })?
}

pub(crate) fn command(
    program: &'static str,
    arguments: impl IntoIterator<Item = OsString>,
) -> CommandInvocation {
    CommandInvocation::new(program, arguments, COMMAND_TIMEOUT)
}

pub(crate) fn stable_bundle_id() -> &'static str {
    MACOS_CODEX_STABLE_IDENTITY
}

pub(crate) fn error(code: InstallerErrorCode, message: &'static str) -> InstallerError {
    InstallerError::new(code).with_diagnostic_message(message)
}

pub(crate) fn is_not_found(error: MacosFilesystemError) -> bool {
    error.kind() == MacosFilesystemErrorKind::NotFound
}

pub(crate) fn is_permission_denied(error: MacosFilesystemError) -> bool {
    error.kind() == MacosFilesystemErrorKind::PermissionDenied
}

#[cfg(test)]
pub(super) mod test_support {
    //! In-memory command/filesystem boundaries shared by the macOS unit tests.
    //!
    //! They intentionally model only the narrow adapter traits. No test here
    //! reaches a host `hdiutil`, `ditto`, `open`, or Applications directory.

    use std::{
        collections::{BTreeMap, HashMap, VecDeque},
        path::{Path, PathBuf},
        sync::{Arc, Mutex},
    };

    use super::{
        CommandInvocation, CommandOutput, CommandRunner, CommandRunnerError, MacosFileKind,
        MacosFilesystem, MacosFilesystemError, MacosFilesystemErrorKind,
    };

    fn next_identity() -> super::FileIdentity {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        super::FileIdentity {
            dev: 1,
            inode: NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        }
    }

    #[derive(Clone)]
    struct FakeEntry {
        identity: super::FileIdentity,
        kind: MacosFileKind,
        canonical: PathBuf,
        contents: Vec<u8>,
    }

    #[derive(Default)]
    struct FakeFilesystemState {
        entries: BTreeMap<PathBuf, FakeEntry>,
        create_dir_failures: HashMap<PathBuf, MacosFilesystemErrorKind>,
        crash_after: Option<usize>,
        completed_operations: usize,
        operation_failures: VecDeque<(&'static str, usize, MacosFilesystemErrorKind)>,
    }

    /// An in-memory filesystem with component-aware directory moves. Tests
    /// can configure only the errors relevant to their transaction branch.
    #[derive(Clone, Default)]
    pub struct FakeFilesystem {
        state: Arc<Mutex<FakeFilesystemState>>,
    }

    #[cfg_attr(target_os = "windows", allow(dead_code))]
    impl FakeFilesystem {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn add_dir(&self, path: impl AsRef<Path>) {
            let mut state = self.lock();
            insert_directory_and_ancestors(&mut state.entries, path.as_ref());
        }

        pub fn add_file(&self, path: impl AsRef<Path>) {
            self.add_file_with_contents(path, Vec::new());
        }

        pub fn add_file_with_contents(&self, path: impl AsRef<Path>, contents: impl Into<Vec<u8>>) {
            let path = path.as_ref().to_path_buf();
            let mut state = self.lock();
            if let Some(parent) = path.parent() {
                insert_directory_and_ancestors(&mut state.entries, parent);
            }
            state.entries.insert(
                path.clone(),
                FakeEntry {
                    identity: next_identity(),
                    kind: MacosFileKind::File,
                    canonical: path,
                    contents: contents.into(),
                },
            );
        }

        pub fn add_symlink(&self, path: impl AsRef<Path>, canonical_target: impl AsRef<Path>) {
            let path = path.as_ref().to_path_buf();
            let mut state = self.lock();
            if let Some(parent) = path.parent() {
                insert_directory_and_ancestors(&mut state.entries, parent);
            }
            state.entries.insert(
                path,
                FakeEntry {
                    identity: next_identity(),
                    kind: MacosFileKind::Symlink,
                    canonical: canonical_target.as_ref().to_path_buf(),
                    contents: Vec::new(),
                },
            );
        }

        pub fn contains(&self, path: impl AsRef<Path>) -> bool {
            self.lock().entries.contains_key(path.as_ref())
        }

        pub fn fail_create_dir_under(
            &self,
            path: impl AsRef<Path>,
            kind: MacosFilesystemErrorKind,
        ) {
            self.lock()
                .create_dir_failures
                .insert(path.as_ref().to_path_buf(), kind);
        }

        pub fn crash_after_operation(&self, count: usize) {
            let mut state = self.lock();
            state.completed_operations = 0;
            state.crash_after = Some(count);
        }

        pub fn completed_operations(&self) -> usize {
            self.lock().completed_operations
        }

        pub fn fail_operation(&self, operation: &'static str, kind: MacosFilesystemErrorKind) {
            self.fail_operation_after(operation, 0, kind);
        }

        pub fn fail_operation_after(
            &self,
            operation: &'static str,
            skip: usize,
            kind: MacosFilesystemErrorKind,
        ) {
            self.lock()
                .operation_failures
                .push_back((operation, skip, kind));
        }

        /// Model a successful `ditto` without involving the host filesystem.
        pub fn copy_tree(
            &self,
            source: impl AsRef<Path>,
            destination: impl AsRef<Path>,
        ) -> Result<(), MacosFilesystemError> {
            let source = source.as_ref();
            let destination = destination.as_ref();
            let mut state = self.lock();
            copy_tree(&mut state.entries, source, destination)?;
            completed_operation(&mut state);
            Ok(())
        }

        fn lock(&self) -> std::sync::MutexGuard<'_, FakeFilesystemState> {
            self.state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
        }
    }

    impl MacosFilesystem for FakeFilesystem {
        fn write_record_atomic(
            &self,
            path: &Path,
            data: &[u8],
        ) -> Result<(), MacosFilesystemError> {
            {
                let mut state = self.lock();
                fail_operation(&mut state, "write_record")?;
                fail_operation(&mut state, "rename_record")?;
            }
            self.add_file_with_contents(path, data.to_vec());
            completed_operation(&mut self.lock());
            Ok(())
        }
        fn remove_record(&self, path: &Path) -> Result<(), MacosFilesystemError> {
            let mut state = self.lock();
            fail_operation(&mut state, "remove_record")?;
            state.entries.remove(path);
            completed_operation(&mut state);
            Ok(())
        }

        fn identity(&self, path: &Path) -> Result<super::FileIdentity, MacosFilesystemError> {
            let result = {
                self.lock()
                    .entries
                    .get(path)
                    .filter(|entry| entry.kind == MacosFileKind::Directory)
                    .map(|entry| entry.identity)
                    .ok_or_else(|| filesystem_error(MacosFilesystemErrorKind::Invalid))
            };
            completed_operation(&mut self.lock());
            result
        }

        fn directory_writable(&self, path: &Path) -> bool {
            let result = {
                !self
                    .lock()
                    .create_dir_failures
                    .keys()
                    .any(|root| path.starts_with(root))
            };
            completed_operation(&mut self.lock());
            result
        }
        fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>, MacosFilesystemError> {
            let result = (|| {
                let state = self.lock();
                match state.entries.get(path) {
                    Some(FakeEntry {
                        kind: MacosFileKind::Directory,
                        ..
                    }) => {}
                    Some(_) => return Err(filesystem_error(MacosFilesystemErrorKind::Invalid)),
                    None => return Err(filesystem_error(MacosFilesystemErrorKind::NotFound)),
                }
                Ok(state
                    .entries
                    .keys()
                    .filter(|candidate| candidate.parent() == Some(path))
                    .cloned()
                    .collect())
            })();
            completed_operation(&mut self.lock());
            result
        }

        fn read_file_bounded(
            &self,
            path: &Path,
            max_bytes: u64,
        ) -> Result<Vec<u8>, MacosFilesystemError> {
            let result = (|| {
                let state = self.lock();
                let entry = state
                    .entries
                    .get(path)
                    .ok_or_else(|| filesystem_error(MacosFilesystemErrorKind::NotFound))?;
                if entry.kind != MacosFileKind::File || entry.contents.len() as u64 > max_bytes {
                    return Err(filesystem_error(MacosFilesystemErrorKind::Invalid));
                }
                Ok(entry.contents.clone())
            })();
            completed_operation(&mut self.lock());
            result
        }

        fn canonicalize(&self, path: &Path) -> Result<PathBuf, MacosFilesystemError> {
            let result = {
                self.lock()
                    .entries
                    .get(path)
                    .map(|entry| entry.canonical.clone())
                    .ok_or_else(|| filesystem_error(MacosFilesystemErrorKind::NotFound))
            };
            completed_operation(&mut self.lock());
            result
        }

        fn file_kind(&self, path: &Path) -> Result<MacosFileKind, MacosFilesystemError> {
            let result = {
                self.lock()
                    .entries
                    .get(path)
                    .map(|entry| entry.kind)
                    .ok_or_else(|| filesystem_error(MacosFilesystemErrorKind::NotFound))
            };
            completed_operation(&mut self.lock());
            result
        }

        fn create_dir_all(&self, path: &Path) -> Result<(), MacosFilesystemError> {
            let mut state = self.lock();
            if let Some(kind) = state
                .create_dir_failures
                .iter()
                .find_map(|(root, kind)| path.starts_with(root).then_some(*kind))
            {
                return Err(filesystem_error(kind));
            }
            insert_directory_and_ancestors(&mut state.entries, path);
            completed_operation(&mut state);
            Ok(())
        }

        fn rename(&self, source: &Path, destination: &Path) -> Result<(), MacosFilesystemError> {
            let mut state = self.lock();
            fail_operation(&mut state, "rename")?;
            move_tree(&mut state.entries, source, destination)?;
            completed_operation(&mut state);
            Ok(())
        }

        fn rename_exclusive(
            &self,
            source: &Path,
            destination: &Path,
        ) -> Result<(), MacosFilesystemError> {
            // move_tree checks destination absence under the same mutex.
            self.rename(source, destination)
        }

        fn swap(&self, source: &Path, destination: &Path) -> Result<(), MacosFilesystemError> {
            let mut state = self.lock();
            fail_operation(&mut state, "swap")?;
            let mut entries = state.entries.clone();
            let temporary = Path::new("/__fake_swap_temporary__");
            move_tree(&mut entries, source, temporary)?;
            move_tree(&mut entries, destination, source)?;
            move_tree(&mut entries, temporary, destination)?;
            state.entries = entries;
            completed_operation(&mut state);
            Ok(())
        }

        fn remove_dir_all(&self, path: &Path) -> Result<(), MacosFilesystemError> {
            let mut state = self.lock();
            fail_operation(&mut state, "remove")?;
            if state.entries.get(path).map(|entry| entry.kind) != Some(MacosFileKind::Directory) {
                return Err(filesystem_error(MacosFilesystemErrorKind::NotFound));
            }
            let paths = state
                .entries
                .keys()
                .filter(|candidate| candidate.starts_with(path))
                .cloned()
                .collect::<Vec<_>>();
            for path in paths {
                state.entries.remove(&path);
            }
            completed_operation(&mut state);
            Ok(())
        }
    }

    fn completed_operation(state: &mut FakeFilesystemState) {
        state.completed_operations += 1;
        if state.crash_after == Some(state.completed_operations) {
            state.crash_after = None;
            std::panic::panic_any("simulated filesystem crash");
        }
    }

    fn fail_operation(
        state: &mut FakeFilesystemState,
        operation: &str,
    ) -> Result<(), MacosFilesystemError> {
        if let Some((name, skip, _)) = state.operation_failures.front_mut() {
            if *name == operation {
                if *skip > 0 {
                    *skip -= 1;
                } else {
                    return Err(filesystem_error(
                        state.operation_failures.pop_front().unwrap().2,
                    ));
                }
            }
        }
        Ok(())
    }

    fn insert_directory_and_ancestors(entries: &mut BTreeMap<PathBuf, FakeEntry>, path: &Path) {
        let mut ancestors = path.ancestors().collect::<Vec<_>>();
        ancestors.reverse();
        for ancestor in ancestors {
            if ancestor.as_os_str().is_empty() {
                continue;
            }
            entries
                .entry(ancestor.to_path_buf())
                .or_insert_with(|| FakeEntry {
                    identity: next_identity(),
                    kind: MacosFileKind::Directory,
                    canonical: ancestor.to_path_buf(),
                    contents: Vec::new(),
                });
        }
    }

    #[cfg_attr(target_os = "windows", allow(dead_code))]
    fn copy_tree(
        entries: &mut BTreeMap<PathBuf, FakeEntry>,
        source: &Path,
        destination: &Path,
    ) -> Result<(), MacosFilesystemError> {
        if entries.get(source).map(|entry| entry.kind) != Some(MacosFileKind::Directory) {
            return Err(filesystem_error(MacosFilesystemErrorKind::NotFound));
        }
        if entries.contains_key(destination) {
            return Err(filesystem_error(MacosFilesystemErrorKind::AlreadyExists));
        }
        let copied = entries
            .iter()
            .filter(|(path, _)| path.starts_with(source))
            .map(|(path, entry)| (path.clone(), entry.clone()))
            .collect::<Vec<_>>();
        for (path, mut entry) in copied {
            entry.identity = next_identity();
            let suffix = path
                .strip_prefix(source)
                .expect("copied source path remains below source root");
            let destination_path = destination.join(suffix);
            if entry.canonical.starts_with(source) {
                let canonical_suffix = entry
                    .canonical
                    .strip_prefix(source)
                    .expect("canonical copied path remains below source root");
                entry.canonical = destination.join(canonical_suffix);
            }
            entries.insert(destination_path, entry);
        }
        Ok(())
    }

    fn move_tree(
        entries: &mut BTreeMap<PathBuf, FakeEntry>,
        source: &Path,
        destination: &Path,
    ) -> Result<(), MacosFilesystemError> {
        if entries.get(source).map(|entry| entry.kind) != Some(MacosFileKind::Directory) {
            return Err(filesystem_error(MacosFilesystemErrorKind::NotFound));
        }
        if entries.contains_key(destination) {
            return Err(filesystem_error(MacosFilesystemErrorKind::AlreadyExists));
        }
        let moved = entries
            .iter()
            .filter(|(path, _)| path.starts_with(source))
            .map(|(path, entry)| (path.clone(), entry.clone()))
            .collect::<Vec<_>>();
        for (path, _) in &moved {
            entries.remove(path);
        }
        for (path, mut entry) in moved {
            let suffix = path
                .strip_prefix(source)
                .expect("moved source path remains below source root");
            let destination_path = destination.join(suffix);
            if entry.canonical.starts_with(source) {
                let canonical_suffix = entry
                    .canonical
                    .strip_prefix(source)
                    .expect("canonical moved path remains below source root");
                entry.canonical = destination.join(canonical_suffix);
            }
            entries.insert(destination_path, entry);
        }
        Ok(())
    }

    fn filesystem_error(kind: MacosFilesystemErrorKind) -> MacosFilesystemError {
        MacosFilesystemError { kind }
    }

    #[derive(Clone)]
    struct QueuedCommand {
        program: &'static str,
        result: CommandOutput,
    }

    type CommandHook = Arc<dyn Fn(&CommandInvocation) + Send + Sync>;

    /// Queue-based fake runner. It asserts the exact command order while
    /// keeping command arguments available for security assertions.
    #[derive(Default)]
    pub struct FakeRunner {
        queued: Mutex<VecDeque<QueuedCommand>>,
        invocations: Mutex<Vec<CommandInvocation>>,
        hook: Mutex<Option<CommandHook>>,
    }

    impl FakeRunner {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn queue_success(&self, program: &'static str, stdout: impl Into<Vec<u8>>) {
            self.queue(program, CommandOutput::success(stdout));
        }

        pub fn queue_success_stderr(&self, program: &'static str, stderr: impl Into<Vec<u8>>) {
            self.queue(
                program,
                CommandOutput::new(Some(0), Vec::new(), stderr.into()),
            );
        }

        pub fn queue_failure(
            &self,
            program: &'static str,
            status_code: Option<i32>,
            stderr: impl Into<Vec<u8>>,
        ) {
            self.queue(program, CommandOutput::failure(status_code, stderr));
        }

        #[cfg_attr(target_os = "windows", allow(dead_code))]
        pub fn set_hook(&self, hook: CommandHook) {
            *self
                .hook
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(hook);
        }

        pub fn invocations(&self) -> Vec<CommandInvocation> {
            self.invocations
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .clone()
        }

        pub fn assert_drained(&self) {
            let queued = self
                .queued
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            assert!(queued.is_empty(), "unused fake command responses remain");
        }

        fn queue(&self, program: &'static str, result: CommandOutput) {
            self.queued
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .push_back(QueuedCommand { program, result });
        }
    }

    impl CommandRunner for FakeRunner {
        fn run(&self, invocation: &CommandInvocation) -> Result<CommandOutput, CommandRunnerError> {
            let queued = self
                .queued
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .pop_front()
                .expect("unexpected macOS command invocation");
            assert_eq!(queued.program, invocation.program());
            self.invocations
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .push(invocation.clone());
            if let Some(hook) = self
                .hook
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .clone()
            {
                hook(invocation);
            }
            Ok(queued.result)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{path::Path, sync::Arc};

    use futures::executor::block_on;

    use super::{
        test_support::{FakeFilesystem, FakeRunner},
        CommandOutput, CpuArchitecture, DiskSpaceProbe, LocalInstallStatus, MacosDiskSpaceProbe,
        MacosHost, MacosPlatformAdapter, MacosVersion, UnsupportedReason, MAX_COMMAND_OUTPUT_BYTES,
    };
    use crate::codex_desktop::platform::CodexDesktopPlatform;

    #[test]
    fn output_reader_bounds_storage_but_drains_to_eof() {
        for size in [
            0,
            MAX_COMMAND_OUTPUT_BYTES,
            MAX_COMMAND_OUTPUT_BYTES + 1,
            8 * 1024 * 1024,
        ] {
            let mut reader = std::io::Cursor::new(vec![b'x'; size]);
            let (bytes, truncated) = super::read_command_output(&mut reader).unwrap();
            assert_eq!(bytes, vec![b'x'; size.min(MAX_COMMAND_OUTPUT_BYTES)]);
            assert_eq!(truncated, size > MAX_COMMAND_OUTPUT_BYTES);
            assert_eq!(reader.position(), size as u64);
            assert!(bytes.capacity() <= MAX_COMMAND_OUTPUT_BYTES);
        }
    }

    #[test]
    fn output_reader_propagates_errors_even_after_truncation() {
        struct Broken;
        impl std::io::Read for Broken {
            fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
                Err(std::io::Error::other("broken pipe"))
            }
        }
        use std::io::Read;
        let reader = std::io::Cursor::new(vec![0; MAX_COMMAND_OUTPUT_BYTES + 1]).chain(Broken);
        assert_eq!(
            super::read_command_output(reader).unwrap_err().kind(),
            std::io::ErrorKind::Other
        );
    }

    #[test]
    fn output_reader_retries_interrupted_reads() {
        struct InterruptedOnce(bool);
        impl std::io::Read for InterruptedOnce {
            fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
                if std::mem::replace(&mut self.0, false) {
                    Err(std::io::ErrorKind::Interrupted.into())
                } else {
                    Ok(0)
                }
            }
        }
        assert_eq!(
            super::read_command_output(InterruptedOnce(true)).unwrap(),
            (vec![], false)
        );
    }

    // Scan all known command-name literals, regardless of whether they appear
    // in a call, variable or constant. Runtime checks cover arbitrary names.
    fn has_bare_command(source: &str) -> bool {
        let literals = regex::Regex::new(
            r#""(?:codesign|spctl|osascript|df|sw_vers|open|ditto|hdiutil|plutil|ps|lipo|pgrep)""#,
        )
        .unwrap();
        let calls =
            regex::Regex::new(r#"(?:command|Command::new|CommandInvocation::new)\(\s*"([^"]+)""#)
                .unwrap();
        literals.is_match(source) || calls.captures_iter(source).any(|c| !c[1].starts_with('/'))
    }

    #[test]
    fn macos_command_literals_are_absolute() {
        for source in [
            include_str!("mod.rs"),
            include_str!("bundle.rs"),
            include_str!("dmg.rs"),
            include_str!("signature.rs"),
        ] {
            assert!(!has_bare_command(
                source.split("#[cfg(test)]").next().unwrap()
            ));
        }
    }

    #[test]
    fn command_scan_catches_variables_constants_comments_and_other_runners() {
        for source in [
            r#"let cmd = "codesign"; command(cmd, []);"#,
            r#"const CMD: &str = "spctl"; runner.run(CMD);"#,
            r#"run_check(runner, bundle, "codesign", args);"#,
            r#"command(/* comment */ "osascript", []);"#,
            r#"runner.run("sw_vers");"#,
            r#"Command::new("arbitrary-relative");"#,
        ] {
            assert!(has_bare_command(source), "{source}");
        }
        assert!(!has_bare_command(
            r#"let cmd = "/usr/bin/codesign"; command(cmd, []);"#
        ));
    }

    #[test]
    fn system_runner_rejects_relative_invocation_before_spawn() {
        use super::CommandRunner;
        let invocation = super::CommandInvocation {
            program: "codesign",
            arguments: vec![],
            timeout: super::COMMAND_TIMEOUT,
        };
        assert_eq!(
            super::SystemCommandRunner
                .run(&invocation)
                .unwrap_err()
                .kind(),
            super::CommandRunnerErrorKind::Io
        );
    }

    #[test]
    #[should_panic(expected = "macOS commands require absolute paths")]
    fn command_rejects_path_lookup() {
        let relative_program = "relative-program";
        super::command(relative_program, Vec::new());
    }

    #[test]
    fn version_parsing_uses_numeric_components() {
        assert!(MacosVersion::parse("14").unwrap() < MacosVersion::parse("14.1").unwrap());
        assert!(MacosVersion::parse("14.1.9").unwrap() < MacosVersion::parse("14.2").unwrap());
        for invalid in ["", "14.", "14.beta", "14.0.1.2", "65536"] {
            assert!(
                MacosVersion::parse(invalid).is_err(),
                "{invalid} must be rejected"
            );
        }
    }

    #[test]
    fn command_output_is_bounded_before_test_diagnostics_can_observe_it() {
        let output = CommandOutput::success(vec![b'x'; MAX_COMMAND_OUTPUT_BYTES + 1024]);
        assert_eq!(output.stdout().len(), MAX_COMMAND_OUTPUT_BYTES);
        assert!(output.is_success());
        assert!(output.stdout_truncated());
        assert!(!output.stderr_truncated());
        let exact = CommandOutput::success(vec![b'x'; MAX_COMMAND_OUTPUT_BYTES]);
        assert!(!exact.stdout_truncated());
        let failure = CommandOutput::failure(Some(1), vec![b'x'; MAX_COMMAND_OUTPUT_BYTES + 1]);
        assert!(failure.stderr_truncated());
        assert!(!failure.stdout_truncated());
    }

    #[test]
    fn disk_space_probe_uses_fixed_df_arguments_and_caches_the_snapshot() {
        let runner = Arc::new(FakeRunner::new());
        runner.queue_success(
            "/bin/df",
            b"Filesystem 1024-blocks Used Available Capacity Mounted on\n/dev/disk3s1 1000 1 999 1% /\n"
                .to_vec(),
        );
        let probe = MacosDiskSpaceProbe::new(runner.clone());

        let volume = probe.volume_key(Path::new("/Applications")).unwrap();
        assert_eq!(probe.available_bytes(&volume).unwrap(), 999 * 1024);
        runner.assert_drained();

        let invocation = runner.invocations().pop().unwrap();
        assert_eq!(invocation.program(), "/bin/df");
        assert_eq!(invocation.arguments()[0], "-Pk");
        assert_eq!(invocation.arguments()[1], "/Applications");
    }

    #[test]
    fn intel_host_is_explicitly_unsupported_without_touching_a_real_host() {
        let runner = Arc::new(FakeRunner::new());
        let filesystem = Arc::new(FakeFilesystem::new());
        let host = MacosHost::new(
            CpuArchitecture::X86_64UnsupportedMac,
            "14.0",
            "/Applications".into(),
            "/Users/test/Applications".into(),
        )
        .unwrap();
        let adapter = MacosPlatformAdapter::new(runner.clone(), filesystem, host);

        assert_eq!(
            block_on(adapter.inspect_local()).unwrap(),
            LocalInstallStatus::Unsupported {
                reason: UnsupportedReason::Architecture,
            }
        );
        runner.assert_drained();
    }
}
