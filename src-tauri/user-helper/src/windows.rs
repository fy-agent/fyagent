use std::{
    ffi::{OsStr, OsString},
    fs::File,
    mem::{offset_of, size_of},
    os::windows::{
        ffi::{OsStrExt, OsStringExt},
        io::{AsRawHandle, FromRawHandle, OwnedHandle},
    },
    path::{Component, Path, PathBuf, Prefix},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

use windows::{
    core::{BOOL, HRESULT, HSTRING, PCWSTR, PWSTR},
    Foundation::Uri,
    Management::Deployment::{
        AddPackageOptions, DeploymentProgress, DeploymentResult, PackageManager,
    },
    Wdk::{
        Foundation::OBJECT_ATTRIBUTES,
        Storage::FileSystem::{
            NtCreateFile, FILE_DIRECTORY_FILE, FILE_NON_DIRECTORY_FILE, FILE_OPEN,
            FILE_OPEN_REPARSE_POINT, FILE_SYNCHRONOUS_IO_NONALERT,
        },
    },
    Win32::{
        Foundation::{
            ERROR_IO_PENDING, HANDLE, HLOCAL, INVALID_HANDLE_VALUE, OBJ_CASE_INSENSITIVE,
            OBJ_DONT_REPARSE, UNICODE_STRING, WAIT_FAILED, WAIT_OBJECT_0, WAIT_TIMEOUT,
        },
        Security::{
            AccessCheck, AclSizeInformation,
            Authorization::{GetSecurityInfo, SE_FILE_OBJECT, SE_KERNEL_OBJECT},
            CheckTokenMembership, CopySid, CreateWellKnownSid, DuplicateToken, EqualSid, GetAce,
            GetAclInformation, GetLengthSid, GetSecurityDescriptorControl, GetTokenInformation,
            IsValidSid, IsWellKnownSid, SecurityImpersonation, TokenUser, WinAuthenticatedUserSid,
            WinBuiltinAdministratorsSid, WinLocalSystemSid, ACCESS_ALLOWED_ACE, ACL_REVISION,
            ACL_SIZE_INFORMATION, DACL_SECURITY_INFORMATION, GENERIC_MAPPING,
            GROUP_SECURITY_INFORMATION, OWNER_SECURITY_INFORMATION, PRIVILEGE_SET,
            PSECURITY_DESCRIPTOR, PSID, SECURITY_MAX_SID_SIZE, SE_DACL_AUTO_INHERITED,
            SE_DACL_AUTO_INHERIT_REQ, SE_DACL_DEFAULTED, SE_DACL_PRESENT, SE_DACL_PROTECTED,
            SE_GROUP_DEFAULTED, SE_OWNER_DEFAULTED, TOKEN_DUPLICATE, TOKEN_QUERY, TOKEN_USER,
        },
        Storage::FileSystem::{
            CreateFileW, FileAttributeTagInfo, FileStandardInfo, GetDriveTypeW,
            GetFileInformationByHandle, GetFileInformationByHandleEx, GetVolumeInformationW,
            GetVolumePathNameW, ReadFile, WriteFile, BY_HANDLE_FILE_INFORMATION, DELETE,
            FILE_ACCESS_RIGHTS, FILE_ADD_FILE, FILE_ADD_SUBDIRECTORY, FILE_ALL_ACCESS,
            FILE_APPEND_DATA, FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_NORMAL,
            FILE_ATTRIBUTE_OFFLINE, FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS,
            FILE_ATTRIBUTE_RECALL_ON_OPEN, FILE_ATTRIBUTE_REPARSE_POINT, FILE_ATTRIBUTE_TAG_INFO,
            FILE_DELETE_CHILD, FILE_FLAGS_AND_ATTRIBUTES, FILE_FLAG_BACKUP_SEMANTICS,
            FILE_FLAG_OPEN_REPARSE_POINT, FILE_FLAG_OVERLAPPED, FILE_GENERIC_EXECUTE,
            FILE_GENERIC_READ, FILE_GENERIC_WRITE, FILE_SHARE_DELETE, FILE_SHARE_MODE,
            FILE_SHARE_READ, FILE_SHARE_WRITE, FILE_STANDARD_INFO, FILE_WRITE_ATTRIBUTES,
            FILE_WRITE_DATA, FILE_WRITE_EA, OPEN_EXISTING, SECURITY_EFFECTIVE_ONLY,
            SECURITY_IDENTIFICATION, SECURITY_SQOS_PRESENT, WRITE_DAC, WRITE_OWNER,
        },
        System::{
            Com::CoTaskMemFree,
            SystemServices::{ACCESS_ALLOWED_ACE_TYPE, FILE_PERSISTENT_ACLS, MAXIMUM_ALLOWED},
            Threading::{
                CreateEventW, GetCurrentProcess, OpenEventW, OpenProcessToken, SetEvent,
                WaitForMultipleObjects, WaitForSingleObject, SYNCHRONIZATION_ACCESS_RIGHTS,
            },
            WinRT::{RoInitialize, RoUninitialize, RO_INIT_MULTITHREADED},
            WindowsProgramming::DRIVE_FIXED,
            IO::{
                CancelIoEx, GetOverlappedResult, GetOverlappedResultEx, IO_STATUS_BLOCK, OVERLAPPED,
            },
        },
        UI::Shell::{
            FOLDERID_ProgramData, PathCreateFromUrlW, SHGetKnownFolderPath, UrlCreateFromPathW,
            KF_FLAG_DEFAULT,
        },
    },
};
use windows_future::{
    AsyncOperationProgressHandler, AsyncOperationWithProgressCompletedHandler, AsyncStatus,
    IAsyncOperationWithProgress,
};

use fyagent_user_helper::{
    admission_event_name, cancel_event_name, encode_frame,
    helper_error_code_for_deployment_hresult,
    layout::{
        pipe_name, USER_HELPER_CONTROL_EVENT_ACCESS_MASK, USER_HELPER_EXECUTABLE_FILE_NAME,
        USER_HELPER_PIPE_CLIENT_ACCESS_MASK,
    },
    BridgeOperationId, HelperErrorCode, HelperMessage, InstallRequest, PackageBridgeControl,
    PinnedPackageIdentity, BRIDGE_CONTROL_BYTES, INSTALLER_FILE_NAME,
    PACKAGE_BRIDGE_ROOT_DIRECTORY, PACKAGE_BRIDGE_VERSION_DIRECTORY,
};

// Covers the parent's 30-second Explorer COM launch wait, pipe connection,
// raw-first-frame identity admission, and a bounded authentication margin.
const ADMISSION_TIMEOUT: Duration = Duration::from_secs(75);
const DEPLOYMENT_TIMEOUT: Duration = Duration::from_secs(9 * 60);
const WAIT_SLICE: Duration = Duration::from_millis(250);
const PIPE_IO_TIMEOUT: Duration = Duration::from_secs(5);
const BA_FULL_MASK: u32 = 0x001f_01ff;
const DIRECTORY_READ_MASK: u32 = 0x0012_00a9;
const DIRECTORY_TRAVERSE_MASK: u32 = 0x0012_00a0;
const FILE_READ_MASK: u32 = 0x0012_0089;
const ANCESTOR_DANGEROUS_ACCESS: u32 = DELETE.0
    | FILE_DELETE_CHILD.0
    | FILE_WRITE_EA.0
    | FILE_WRITE_ATTRIBUTES.0
    | WRITE_DAC.0
    | WRITE_OWNER.0;
const BRIDGE_DIRECTORY_DANGEROUS_ACCESS: u32 = ANCESTOR_DANGEROUS_ACCESS
    | FILE_ADD_FILE.0
    | FILE_ADD_SUBDIRECTORY.0
    | FILE_WRITE_EA.0
    | FILE_WRITE_ATTRIBUTES.0;
const BRIDGE_FILE_DANGEROUS_ACCESS: u32 = DELETE.0
    | FILE_WRITE_DATA.0
    | FILE_APPEND_DATA.0
    | FILE_WRITE_EA.0
    | FILE_WRITE_ATTRIBUTES.0
    | WRITE_DAC.0
    | WRITE_OWNER.0;
const MAX_DOS_PATH_U16: usize = 32_768;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HelperRunError {
    PipeUnavailable,
    PipeWriteFailed,
    OperationFailed(HelperErrorCode),
}

impl std::fmt::Display for HelperRunError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::PipeUnavailable => "the one-shot helper pipe is unavailable",
            Self::PipeWriteFailed => "the helper pipe closed before the operation completed",
            Self::OperationFailed(_) => "the current-user package installation failed",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for HelperRunError {}

pub(crate) fn run_install(request: &InstallRequest) -> Result<(), HelperRunError> {
    let executable = std::env::current_exe()
        .map_err(|_| HelperRunError::OperationFailed(HelperErrorCode::InstallLayoutInvalid))?;
    if executable.file_name().is_none_or(|name| {
        !name
            .to_string_lossy()
            .eq_ignore_ascii_case(USER_HELPER_EXECUTABLE_FILE_NAME)
    }) {
        return Err(HelperRunError::OperationFailed(
            HelperErrorCode::InstallLayoutInvalid,
        ));
    }
    // Open both parent-created controls before the pipe. If the parent tears
    // down a timed-out launch it closes the pipe before either event; a helper
    // holding a replacement event can therefore never cross the pipe boundary,
    // while a helper that connected already holds the original objects.
    let controls = ParentControls::open(request)?;
    let channel = PipeChannel::connect(&pipe_name(request.pipe_nonce()))?;
    let channel = Arc::new(channel);
    channel.send_hello()?;

    let bridge_control = match channel.read_bridge_control(ADMISSION_TIMEOUT) {
        Ok(control) => control,
        Err(error) => {
            let _ = channel.send_prestart_error(HelperErrorCode::ParentAdmissionFailed);
            return Err(error);
        }
    };
    let package_pin = match PinnedPackageFile::open(bridge_control) {
        Ok(package_pin) => package_pin,
        Err(error) => {
            let code = match error {
                HelperRunError::OperationFailed(code) => code,
                _ => HelperErrorCode::PackageInvalid,
            };
            channel.send_prestart_error(code)?;
            return Err(error);
        }
    };
    if let Err(error) = package_pin.recheck_for_helper() {
        channel.send_prestart_error(HelperErrorCode::PackageInvalid)?;
        return Err(error);
    }
    channel.send_started(package_pin.protocol_identity())?;

    if let Err(code) = controls.wait_for_admission(ADMISSION_TIMEOUT) {
        channel.send_terminal(HelperMessage::error(code))?;
        return Err(HelperRunError::OperationFailed(code));
    }
    channel.mark_admitted()?;
    channel.send_progress(0)?;

    match deploy_fixed_package(&package_pin, &channel, &controls) {
        Ok(()) => {
            channel.send_progress(100)?;
            channel.send_terminal(HelperMessage::Success)
        }
        Err(DeploymentFailure::Pipe) => Err(HelperRunError::PipeWriteFailed),
        Err(DeploymentFailure::Operation(code)) => {
            channel.send_terminal(HelperMessage::error(code))?;
            Err(HelperRunError::OperationFailed(code))
        }
    }
}

fn deploy_fixed_package(
    package_pin: &PinnedPackageFile,
    channel: &Arc<PipeChannel>,
    controls: &ParentControls,
) -> Result<(), DeploymentFailure> {
    let _apartment = WinRtApartment::initialize()?;
    let uri = package_pin.package_uri()?;
    let package_manager = PackageManager::new()
        .map_err(|_| DeploymentFailure::Operation(HelperErrorCode::PackageManagerUnavailable))?;
    // Defaults deliberately retain Windows signature enforcement and leave
    // force-shutdown/developer options disabled.
    let options = AddPackageOptions::new()
        .map_err(|_| DeploymentFailure::Operation(HelperErrorCode::PackageManagerUnavailable))?;
    package_pin.recheck()?;
    let operation = match package_manager.AddPackageByUriAsync(&uri, &options) {
        Ok(operation) => operation,
        // The call has crossed the PackageManager service boundary. A
        // synchronous HRESULT does not prove that the service rejected the
        // request before accepting it, and there is no operation object whose
        // Status we can observe or Cancel. Keep the complete package ancestry
        // and leaf pin alive; the parent will likewise quarantine its pin
        // because no authenticated terminal frame can be sent.
        Err(_) => hold_ambiguous_submission(package_pin),
    };

    let completion = match CompletionSignal::new() {
        Ok(completion) => completion,
        Err(()) => return settle_after_failure(&operation, None),
    };
    let completion_callback_signal = completion.signal.clone();
    let completion_status = completion.status.clone();
    if operation
        .SetCompleted(&AsyncOperationWithProgressCompletedHandler::<
            DeploymentResult,
            DeploymentProgress,
        >::new(move |_, status| {
            if status != AsyncStatus::Started {
                *completion_status
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(status);
            }
            unsafe { SetEvent(completion_callback_signal.raw()) }
        }))
        .is_err()
    {
        return settle_after_failure(&operation, None);
    }

    let progress_failure = match LocalSignal::new() {
        Ok(signal) => signal,
        Err(()) => return settle_after_failure(&operation, Some(&completion)),
    };
    let progress_channel = channel.clone();
    let progress_callback_failure = progress_failure.clone();
    if operation
        .SetProgress(&AsyncOperationProgressHandler::<
            DeploymentResult,
            DeploymentProgress,
        >::new(move |_, progress| {
            if progress_channel
                .send_progress(progress.percentage.min(100) as u8)
                .is_err()
            {
                unsafe { SetEvent(progress_callback_failure.raw()) }?;
            }
            Ok(())
        }))
        .is_err()
    {
        return settle_after_failure(&operation, Some(&completion));
    }

    let terminal = match wait_for_operation(
        &operation,
        controls,
        channel,
        &completion,
        &progress_failure,
        DEPLOYMENT_TIMEOUT,
    ) {
        Ok(status) => status,
        Err(_) => {
            return settle_after_failure(&operation, Some(&completion));
        }
    };

    finish_terminal(&operation, terminal)
}

fn validate_deployment_result(result: DeploymentResult) -> Result<(), DeploymentFailure> {
    let extended_error = result
        .ExtendedErrorCode()
        .map_err(|_| DeploymentFailure::Operation(HelperErrorCode::DeploymentResultInvalid))?;
    if extended_error.0 != 0 {
        return Err(DeploymentFailure::Operation(
            helper_error_code_for_deployment_hresult(extended_error.0),
        ));
    }
    if !result
        .IsRegistered()
        .map_err(|_| DeploymentFailure::Operation(HelperErrorCode::DeploymentResultInvalid))?
    {
        return Err(DeploymentFailure::Operation(
            HelperErrorCode::DeploymentResultInvalid,
        ));
    }
    Ok(())
}

type PackageOperation = IAsyncOperationWithProgress<DeploymentResult, DeploymentProgress>;

fn wait_for_operation(
    operation: &PackageOperation,
    controls: &ParentControls,
    channel: &PipeChannel,
    completion: &CompletionSignal,
    progress_failure: &LocalSignal,
    timeout: Duration,
) -> Result<AsyncStatus, DeploymentFailure> {
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(status) = terminal_status(operation, Some(completion)) {
            return Ok(status);
        }
        if channel.write_failed() {
            return Err(DeploymentFailure::Pipe);
        }
        let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
            return Err(DeploymentFailure::Operation(
                HelperErrorCode::DeploymentTimedOut,
            ));
        };
        let wait = remaining.min(WAIT_SLICE);
        let result = unsafe {
            WaitForMultipleObjects(
                &[
                    controls.cancel.raw(),
                    progress_failure.raw(),
                    completion.raw(),
                ],
                false,
                duration_millis(wait),
            )
        };
        if result == WAIT_OBJECT_0 {
            return Err(DeploymentFailure::Operation(
                HelperErrorCode::ParentCancelled,
            ));
        }
        if result.0 == WAIT_OBJECT_0.0 + 1 {
            return Err(DeploymentFailure::Pipe);
        }
        if result == WAIT_FAILED {
            return Err(DeploymentFailure::Operation(
                HelperErrorCode::DeploymentFailed,
            ));
        }
        if result != WAIT_TIMEOUT && result.0 != WAIT_OBJECT_0.0 + 2 {
            return Err(DeploymentFailure::Operation(
                HelperErrorCode::DeploymentFailed,
            ));
        }
    }
}

fn settle_after_failure(
    operation: &PackageOperation,
    completion: Option<&CompletionSignal>,
) -> Result<(), DeploymentFailure> {
    // Cancel is a request, not a terminal result. Call it at most once and
    // keep the helper alive until Status is definitively non-Started. If the
    // status API itself remains unavailable this loop intentionally does not
    // let the process exit; the parent will quarantine the verified-file pin.
    if terminal_status(operation, completion).is_none() {
        let _ = operation.Cancel();
    }
    let status = wait_for_true_terminal(operation, completion);
    finish_terminal(operation, status)
}

fn wait_for_true_terminal(
    operation: &PackageOperation,
    completion: Option<&CompletionSignal>,
) -> AsyncStatus {
    loop {
        if let Some(status) = terminal_status(operation, completion) {
            return status;
        }
        match completion {
            Some(completion) => {
                let _ =
                    unsafe { WaitForSingleObject(completion.raw(), duration_millis(WAIT_SLICE)) };
            }
            None => std::thread::sleep(WAIT_SLICE),
        }
    }
}

fn terminal_status(
    operation: &PackageOperation,
    completion: Option<&CompletionSignal>,
) -> Option<AsyncStatus> {
    completion
        .and_then(CompletionSignal::terminal_status)
        .or_else(|| {
            operation
                .Status()
                .ok()
                .filter(|status| *status != AsyncStatus::Started)
        })
}

fn finish_terminal(
    operation: &PackageOperation,
    status: AsyncStatus,
) -> Result<(), DeploymentFailure> {
    debug_assert_ne!(status, AsyncStatus::Started);
    let result = match status {
        AsyncStatus::Completed => operation.GetResults().map_err(|error| {
            let hresult = operation.ErrorCode().unwrap_or_else(|_| error.code());
            DeploymentFailure::Operation(helper_error_code_for_deployment_hresult(hresult.0))
        }),
        AsyncStatus::Canceled => Err(DeploymentFailure::Operation(
            HelperErrorCode::ParentCancelled,
        )),
        AsyncStatus::Error => {
            let code = operation
                .ErrorCode()
                .map(|value| helper_error_code_for_deployment_hresult(value.0))
                .unwrap_or(HelperErrorCode::DeploymentFailed);
            Err(DeploymentFailure::Operation(code))
        }
        AsyncStatus::Started => unreachable!("terminal status cannot be Started"),
        _ => Err(DeploymentFailure::Operation(
            HelperErrorCode::DeploymentFailed,
        )),
    };

    let outcome = result.and_then(validate_deployment_result);

    // Non-Started Status/Completed is the PackageManager terminal proof.
    // Close is subsequent cleanup and must not hide a late Completed-success
    // or the actual Canceled/Error outcome.
    if operation.Close().is_err() {
        eprintln!("fyagent-user-helper: PackageManager terminal cleanup failed");
    }
    outcome
}

fn duration_millis(duration: Duration) -> u32 {
    duration.as_millis().clamp(1, u32::MAX as u128) as u32
}

fn hold_ambiguous_submission(_package_pin: &PinnedPackageFile) -> ! {
    // No PackageManager operation object exists, so neither cancellation nor
    // a true terminal Status can be proved. Deliberately keep this helper and
    // its package/ancestry handles alive until a trusted administrator or the
    // OS terminates the process. The elevated parent remains non-terminal and
    // retains its independently verified pin as well.
    loop {
        std::thread::park();
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct NativeFileIdentity {
    volume_serial: u64,
    file_index: u64,
    size: u64,
}

#[derive(Clone, Copy)]
enum NativeObjectKind {
    Directory,
    RegularFile,
}

struct PinnedPackageFile {
    directories: Vec<File>,
    package_file: File,
    uri_reopen: File,
    identity: NativeFileIdentity,
    package_uri: String,
    user: FrozenUser,
    program_data_directory_count: usize,
}

impl PinnedPackageFile {
    fn open(control: PackageBridgeControl) -> Result<Self, HelperRunError> {
        let user = FrozenUser::capture()?;
        let program_data = known_program_data_path()?;
        validate_ordinary_dos_path(&program_data)?;
        let (volume_root, volume_serial) = validate_fixed_ntfs_volume(&program_data)?;
        let root = open_volume_root_no_follow(&volume_root)?;
        require_volume(&root, NativeObjectKind::Directory, volume_serial)?;
        verify_effective_access(&root, &user, ANCESTOR_DANGEROUS_ACCESS)?;

        let relative_program_data = program_data
            .strip_prefix(&volume_root)
            .map_err(|_| package_pin_error())?;
        let program_data_components = normal_components(relative_program_data)?;
        if program_data_components.is_empty() {
            return Err(package_pin_error());
        }

        let mut directories = vec![root];
        for component in &program_data_components {
            let child = open_relative_no_follow(
                directories.last().expect("the volume root is retained"),
                component,
                NativeObjectKind::Directory,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            )?;
            require_volume(&child, NativeObjectKind::Directory, volume_serial)?;
            verify_effective_access(&child, &user, ANCESTOR_DANGEROUS_ACCESS)?;
            directories.push(child);
        }
        let program_data_directory_count = directories.len();

        for (component, acl) in [
            (
                OsString::from(PACKAGE_BRIDGE_ROOT_DIRECTORY),
                ExactBridgeAcl::StableDirectory,
            ),
            (
                OsString::from(PACKAGE_BRIDGE_VERSION_DIRECTORY),
                ExactBridgeAcl::StableDirectory,
            ),
            (
                OsString::from(control.operation_id().directory_name()),
                ExactBridgeAcl::OperationDirectory,
            ),
        ] {
            let child = open_relative_no_follow(
                directories.last().expect("the bridge ancestry is retained"),
                &component,
                NativeObjectKind::Directory,
                FILE_SHARE_READ,
            )?;
            require_volume(&child, NativeObjectKind::Directory, volume_serial)?;
            verify_exact_bridge_acl(&child, &user, acl)?;
            verify_effective_access(&child, &user, BRIDGE_DIRECTORY_DANGEROUS_ACCESS)?;
            directories.push(child);
        }

        let operation_directory = directories
            .last()
            .expect("the operation directory is retained");
        let package_file = open_relative_no_follow(
            operation_directory,
            OsStr::new(INSTALLER_FILE_NAME),
            NativeObjectKind::RegularFile,
            FILE_SHARE_READ,
        )?;
        verify_exact_bridge_acl(&package_file, &user, ExactBridgeAcl::PackageFile)?;
        verify_effective_access(&package_file, &user, BRIDGE_FILE_DANGEROUS_ACCESS)?;
        let identity = native_file_identity(&package_file, NativeObjectKind::RegularFile)?;
        if identity.size == 0 || identity != native_identity(control.package()) {
            return Err(package_pin_error());
        }

        let dos_path = bridge_path_for_program_data(&program_data, control.operation_id());
        validate_ordinary_dos_path(&dos_path)?;
        let package_uri = local_file_uri_roundtrip(&dos_path)?;
        let uri_reopen = open_relative_no_follow(
            operation_directory,
            OsStr::new(INSTALLER_FILE_NAME),
            NativeObjectKind::RegularFile,
            FILE_SHARE_READ,
        )?;
        if native_file_identity(&uri_reopen, NativeObjectKind::RegularFile)? != identity {
            return Err(package_pin_error());
        }
        verify_exact_bridge_acl(&uri_reopen, &user, ExactBridgeAcl::PackageFile)?;
        verify_effective_access(&uri_reopen, &user, BRIDGE_FILE_DANGEROUS_ACCESS)?;

        let package = Self {
            directories,
            package_file,
            uri_reopen,
            identity,
            package_uri,
            user,
            program_data_directory_count,
        };
        package.recheck_for_helper()?;
        Ok(package)
    }

    fn protocol_identity(&self) -> PinnedPackageIdentity {
        PinnedPackageIdentity::new(
            self.identity.volume_serial,
            self.identity.file_index,
            self.identity.size,
        )
    }

    fn recheck_for_helper(&self) -> Result<(), HelperRunError> {
        for directory in &self.directories[..self.program_data_directory_count] {
            native_file_identity(directory, NativeObjectKind::Directory)?;
            verify_effective_access(directory, &self.user, ANCESTOR_DANGEROUS_ACCESS)?;
        }
        for (offset, acl) in [
            ExactBridgeAcl::StableDirectory,
            ExactBridgeAcl::StableDirectory,
            ExactBridgeAcl::OperationDirectory,
        ]
        .into_iter()
        .enumerate()
        {
            let directory = &self.directories[self.program_data_directory_count + offset];
            native_file_identity(directory, NativeObjectKind::Directory)?;
            verify_exact_bridge_acl(directory, &self.user, acl)?;
            verify_effective_access(directory, &self.user, BRIDGE_DIRECTORY_DANGEROUS_ACCESS)?;
        }
        for file in [&self.package_file, &self.uri_reopen] {
            let identity = native_file_identity(file, NativeObjectKind::RegularFile)?;
            if identity != self.identity {
                return Err(package_pin_error());
            }
            verify_exact_bridge_acl(file, &self.user, ExactBridgeAcl::PackageFile)?;
            verify_effective_access(file, &self.user, BRIDGE_FILE_DANGEROUS_ACCESS)?;
        }
        Ok(())
    }

    fn recheck(&self) -> Result<(), DeploymentFailure> {
        self.recheck_for_helper()
            .map_err(|_| DeploymentFailure::Operation(HelperErrorCode::PackageInvalid))
    }

    fn package_uri(&self) -> Result<Uri, DeploymentFailure> {
        if !has_local_file_uri_shape(&self.package_uri.encode_utf16().collect::<Vec<_>>()) {
            return Err(DeploymentFailure::Operation(
                HelperErrorCode::PackageUriInvalid,
            ));
        }
        let uri = Uri::CreateUri(&HSTRING::from(self.package_uri.as_str()))
            .map_err(|_| DeploymentFailure::Operation(HelperErrorCode::PackageUriInvalid))?;
        let scheme = uri
            .SchemeName()
            .map_err(|_| DeploymentFailure::Operation(HelperErrorCode::PackageUriInvalid))?;
        let host = uri
            .Host()
            .map_err(|_| DeploymentFailure::Operation(HelperErrorCode::PackageUriInvalid))?;
        let query = uri
            .Query()
            .map_err(|_| DeploymentFailure::Operation(HelperErrorCode::PackageUriInvalid))?;
        let fragment = uri
            .Fragment()
            .map_err(|_| DeploymentFailure::Operation(HelperErrorCode::PackageUriInvalid))?;
        if scheme != "file" || !host.is_empty() || !query.is_empty() || !fragment.is_empty() {
            return Err(DeploymentFailure::Operation(
                HelperErrorCode::PackageUriInvalid,
            ));
        }
        Ok(uri)
    }
}

fn native_identity(identity: PinnedPackageIdentity) -> NativeFileIdentity {
    NativeFileIdentity {
        volume_serial: identity.volume_serial(),
        file_index: identity.file_index(),
        size: identity.size(),
    }
}

fn bridge_path_for_program_data(program_data: &Path, operation_id: BridgeOperationId) -> PathBuf {
    program_data
        .join(PACKAGE_BRIDGE_ROOT_DIRECTORY)
        .join(PACKAGE_BRIDGE_VERSION_DIRECTORY)
        .join(operation_id.directory_name())
        .join(INSTALLER_FILE_NAME)
}

struct CoTaskPath(PWSTR);

impl Drop for CoTaskPath {
    fn drop(&mut self) {
        unsafe { CoTaskMemFree(Some(self.0 .0.cast())) };
    }
}

fn known_program_data_path() -> Result<PathBuf, HelperRunError> {
    let raw = unsafe { SHGetKnownFolderPath(&FOLDERID_ProgramData, KF_FLAG_DEFAULT, None) }
        .map_err(|_| package_pin_error())?;
    if raw.0.is_null() {
        return Err(package_pin_error());
    }
    let raw = CoTaskPath(raw);
    let mut length = 0_usize;
    while length < MAX_DOS_PATH_U16 && unsafe { *raw.0 .0.add(length) } != 0 {
        length += 1;
    }
    if length == 0 || length == MAX_DOS_PATH_U16 {
        return Err(package_pin_error());
    }
    let path = unsafe { std::slice::from_raw_parts(raw.0 .0, length) };
    Ok(PathBuf::from(OsString::from_wide(path)))
}

fn validate_ordinary_dos_path(path: &Path) -> Result<(), HelperRunError> {
    let mut components = path.components();
    let Some(Component::Prefix(prefix)) = components.next() else {
        return Err(package_pin_error());
    };
    if !matches!(prefix.kind(), Prefix::Disk(letter) if letter.is_ascii_alphabetic())
        || components.next() != Some(Component::RootDir)
        || components.any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(package_pin_error());
    }
    Ok(())
}

fn normal_components(path: &Path) -> Result<Vec<OsString>, HelperRunError> {
    path.components()
        .map(|component| match component {
            Component::Normal(component) => Ok(component.to_os_string()),
            _ => Err(package_pin_error()),
        })
        .collect()
}

fn validate_fixed_ntfs_volume(path: &Path) -> Result<(PathBuf, u64), HelperRunError> {
    let path_wide = wide_null(path.as_os_str())?;
    let mut volume_buffer = [0_u16; MAX_DOS_PATH_U16];
    unsafe { GetVolumePathNameW(PCWSTR(path_wide.as_ptr()), &mut volume_buffer) }
        .map_err(|_| package_pin_error())?;
    let volume_length = volume_buffer
        .iter()
        .position(|value| *value == 0)
        .filter(|length| *length > 0)
        .ok_or_else(package_pin_error)?;
    let volume_root = PathBuf::from(OsString::from_wide(&volume_buffer[..volume_length]));
    validate_ordinary_dos_path(&volume_root)?;
    if volume_root.components().count() != 2 {
        return Err(package_pin_error());
    }
    let volume_wide = wide_null(volume_root.as_os_str())?;
    if unsafe { GetDriveTypeW(PCWSTR(volume_wide.as_ptr())) } != DRIVE_FIXED {
        return Err(package_pin_error());
    }

    let mut serial = 0_u32;
    let mut flags = 0_u32;
    let mut file_system = [0_u16; 32];
    unsafe {
        GetVolumeInformationW(
            PCWSTR(volume_wide.as_ptr()),
            None,
            Some(&mut serial),
            None,
            Some(&mut flags),
            Some(&mut file_system),
        )
    }
    .map_err(|_| package_pin_error())?;
    let file_system_length = file_system
        .iter()
        .position(|value| *value == 0)
        .ok_or_else(package_pin_error)?;
    let file_system = OsString::from_wide(&file_system[..file_system_length]);
    if !file_system
        .to_str()
        .is_some_and(|name| name.eq_ignore_ascii_case("NTFS"))
        || flags & FILE_PERSISTENT_ACLS == 0
    {
        return Err(package_pin_error());
    }
    Ok((volume_root, u64::from(serial)))
}

fn local_file_uri_roundtrip(path: &Path) -> Result<String, HelperRunError> {
    let path_wide = wide_null(path.as_os_str())?;
    let mut uri_buffer = [0_u16; MAX_DOS_PATH_U16];
    let mut uri_length = u32::try_from(uri_buffer.len()).map_err(|_| package_pin_error())?;
    unsafe {
        UrlCreateFromPathW(
            PCWSTR(path_wide.as_ptr()),
            PWSTR(uri_buffer.as_mut_ptr()),
            &mut uri_length,
            0,
        )
    }
    .map_err(|_| package_pin_error())?;
    let uri_length = usize::try_from(uri_length).map_err(|_| package_pin_error())?;
    if uri_length == 0
        || uri_length >= uri_buffer.len()
        || uri_buffer[uri_length] != 0
        || uri_buffer[..uri_length].contains(&0)
        || !has_local_file_uri_shape(&uri_buffer[..uri_length])
    {
        return Err(package_pin_error());
    }

    let mut roundtrip = [0_u16; MAX_DOS_PATH_U16];
    let mut roundtrip_length = u32::try_from(roundtrip.len()).map_err(|_| package_pin_error())?;
    unsafe {
        PathCreateFromUrlW(
            PCWSTR(uri_buffer.as_ptr()),
            PWSTR(roundtrip.as_mut_ptr()),
            &mut roundtrip_length,
            0,
        )
    }
    .map_err(|_| package_pin_error())?;
    let roundtrip_length = usize::try_from(roundtrip_length).map_err(|_| package_pin_error())?;
    let original = &path_wide[..path_wide.len() - 1];
    if roundtrip_length != original.len()
        || roundtrip_length >= roundtrip.len()
        || roundtrip[roundtrip_length] != 0
        || roundtrip[..roundtrip_length] != *original
    {
        return Err(package_pin_error());
    }
    String::from_utf16(&uri_buffer[..uri_length]).map_err(|_| package_pin_error())
}

fn has_local_file_uri_shape(uri: &[u16]) -> bool {
    const PREFIX: &[u16] = &[
        b'f' as u16,
        b'i' as u16,
        b'l' as u16,
        b'e' as u16,
        b':' as u16,
        b'/' as u16,
        b'/' as u16,
        b'/' as u16,
    ];
    uri.starts_with(PREFIX)
        && uri.len() > PREFIX.len()
        && !uri.iter().any(|value| matches!(*value, value if value == b'?' as u16 || value == b'#' as u16 || value == b'\\' as u16 || value == 0))
}

fn wide_null(value: &OsStr) -> Result<Vec<u16>, HelperRunError> {
    let mut wide = value.encode_wide().collect::<Vec<_>>();
    if wide.is_empty() || wide.contains(&0) || wide.len() >= MAX_DOS_PATH_U16 {
        return Err(package_pin_error());
    }
    wide.push(0);
    Ok(wide)
}

fn open_volume_root_no_follow(path: &Path) -> Result<File, HelperRunError> {
    let path = wide_null(path.as_os_str())?;
    let handle = unsafe {
        CreateFileW(
            PCWSTR(path.as_ptr()),
            DIRECTORY_TRAVERSE_MASK,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            None,
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
            None,
        )
    }
    .map_err(|_| package_pin_error())?;
    if handle.is_invalid() {
        return Err(package_pin_error());
    }
    let file = unsafe { File::from_raw_handle(handle.0) };
    native_file_identity(&file, NativeObjectKind::Directory)?;
    Ok(file)
}

fn open_relative_no_follow(
    root: &File,
    name: &OsStr,
    kind: NativeObjectKind,
    share_access: FILE_SHARE_MODE,
) -> Result<File, HelperRunError> {
    let mut components = Path::new(name).components();
    if !matches!(components.next(), Some(Component::Normal(component)) if component == name)
        || components.next().is_some()
    {
        return Err(package_pin_error());
    }
    let mut wide = name.encode_wide().collect::<Vec<_>>();
    if wide.is_empty() || wide.contains(&0) {
        return Err(package_pin_error());
    }
    let byte_length = wide
        .len()
        .checked_mul(size_of::<u16>())
        .and_then(|length| u16::try_from(length).ok())
        .ok_or_else(package_pin_error)?;
    let object_name = UNICODE_STRING {
        Length: byte_length,
        MaximumLength: byte_length,
        Buffer: PWSTR(wide.as_mut_ptr()),
    };
    let object_attributes = OBJECT_ATTRIBUTES {
        Length: size_of::<OBJECT_ATTRIBUTES>() as u32,
        RootDirectory: HANDLE(root.as_raw_handle()),
        ObjectName: &object_name,
        Attributes: OBJ_CASE_INSENSITIVE | OBJ_DONT_REPARSE,
        SecurityDescriptor: std::ptr::null(),
        SecurityQualityOfService: std::ptr::null(),
    };
    let (desired_access, attributes, create_options) = match kind {
        NativeObjectKind::Directory => (
            DIRECTORY_TRAVERSE_MASK,
            FILE_ATTRIBUTE_DIRECTORY.0,
            (FILE_DIRECTORY_FILE | FILE_OPEN_REPARSE_POINT | FILE_SYNCHRONOUS_IO_NONALERT).0,
        ),
        NativeObjectKind::RegularFile => (
            FILE_READ_MASK,
            FILE_ATTRIBUTE_NORMAL.0,
            (FILE_NON_DIRECTORY_FILE | FILE_OPEN_REPARSE_POINT | FILE_SYNCHRONOUS_IO_NONALERT).0,
        ),
    };
    let mut handle = HANDLE::default();
    let mut io_status = IO_STATUS_BLOCK::default();
    let status = unsafe {
        NtCreateFile(
            &mut handle,
            FILE_ACCESS_RIGHTS(desired_access),
            &object_attributes,
            &mut io_status,
            None,
            FILE_FLAGS_AND_ATTRIBUTES(attributes),
            share_access,
            FILE_OPEN,
            windows::Wdk::Storage::FileSystem::NTCREATEFILE_CREATE_OPTIONS(create_options),
            None,
            0,
        )
    };
    if status.is_err() || handle.0.is_null() || handle == INVALID_HANDLE_VALUE {
        return Err(package_pin_error());
    }
    let file = unsafe { File::from_raw_handle(handle.0) };
    native_file_identity(&file, kind)?;
    Ok(file)
}

fn require_volume(
    file: &File,
    kind: NativeObjectKind,
    expected_serial: u64,
) -> Result<(), HelperRunError> {
    if native_file_identity(file, kind)?.volume_serial != expected_serial {
        return Err(package_pin_error());
    }
    Ok(())
}

fn native_file_identity(
    file: &File,
    kind: NativeObjectKind,
) -> Result<NativeFileIdentity, HelperRunError> {
    let handle = HANDLE(file.as_raw_handle());
    let mut information = BY_HANDLE_FILE_INFORMATION::default();
    unsafe { GetFileInformationByHandle(handle, &mut information) }
        .map_err(|_| package_pin_error())?;
    let mut standard = FILE_STANDARD_INFO::default();
    unsafe {
        GetFileInformationByHandleEx(
            handle,
            FileStandardInfo,
            (&mut standard as *mut FILE_STANDARD_INFO).cast(),
            size_of::<FILE_STANDARD_INFO>() as u32,
        )
    }
    .map_err(|_| package_pin_error())?;
    let mut attributes = FILE_ATTRIBUTE_TAG_INFO::default();
    unsafe {
        GetFileInformationByHandleEx(
            handle,
            FileAttributeTagInfo,
            (&mut attributes as *mut FILE_ATTRIBUTE_TAG_INFO).cast(),
            size_of::<FILE_ATTRIBUTE_TAG_INFO>() as u32,
        )
    }
    .map_err(|_| package_pin_error())?;

    let is_directory = information.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY.0 != 0;
    let disallowed_attributes = FILE_ATTRIBUTE_REPARSE_POINT.0
        | FILE_ATTRIBUTE_OFFLINE.0
        | FILE_ATTRIBUTE_RECALL_ON_OPEN.0
        | FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS.0;
    if attributes.FileAttributes != information.dwFileAttributes
        || attributes.FileAttributes & disallowed_attributes != 0
        || standard.DeletePending
        || standard.Directory != is_directory
        || standard.NumberOfLinks != information.nNumberOfLinks
        || match kind {
            NativeObjectKind::Directory => !is_directory,
            NativeObjectKind::RegularFile => is_directory || standard.NumberOfLinks != 1,
        }
    {
        return Err(package_pin_error());
    }
    let size = (u64::from(information.nFileSizeHigh) << 32) | u64::from(information.nFileSizeLow);
    if standard.EndOfFile < 0 || standard.EndOfFile as u64 != size {
        return Err(package_pin_error());
    }
    Ok(NativeFileIdentity {
        volume_serial: u64::from(information.dwVolumeSerialNumber),
        file_index: (u64::from(information.nFileIndexHigh) << 32)
            | u64::from(information.nFileIndexLow),
        size,
    })
}

struct FrozenUser {
    token: OwnedKernelHandle,
    sid_storage: Vec<usize>,
}

impl FrozenUser {
    fn capture() -> Result<Self, HelperRunError> {
        let mut primary = HANDLE::default();
        unsafe {
            OpenProcessToken(
                GetCurrentProcess(),
                TOKEN_QUERY | TOKEN_DUPLICATE,
                &mut primary,
            )
        }
        .map_err(|_| package_pin_error())?;
        let primary = OwnedKernelHandle::new(primary).map_err(|_| package_pin_error())?;
        let mut token = HANDLE::default();
        unsafe { DuplicateToken(primary.raw(), SecurityImpersonation, &mut token) }
            .map_err(|_| package_pin_error())?;
        let token = OwnedKernelHandle::new(token).map_err(|_| package_pin_error())?;

        let mut information_length = 0_u32;
        let _ = unsafe {
            GetTokenInformation(token.raw(), TokenUser, None, 0, &mut information_length)
        };
        if information_length < size_of::<TOKEN_USER>() as u32 {
            return Err(package_pin_error());
        }
        let mut information = aligned_words(information_length as usize);
        unsafe {
            GetTokenInformation(
                token.raw(),
                TokenUser,
                Some(information.as_mut_ptr().cast()),
                information_length,
                &mut information_length,
            )
        }
        .map_err(|_| package_pin_error())?;
        let user = unsafe { &*(information.as_ptr().cast::<TOKEN_USER>()) };
        if user.User.Sid.0.is_null() || !unsafe { IsValidSid(user.User.Sid) }.as_bool() {
            return Err(package_pin_error());
        }
        let sid_length = unsafe { GetLengthSid(user.User.Sid) };
        if sid_length == 0 {
            return Err(package_pin_error());
        }
        let mut sid_storage = aligned_words(sid_length as usize);
        let sid = PSID(sid_storage.as_mut_ptr().cast());
        unsafe { CopySid(sid_length, sid, user.User.Sid) }.map_err(|_| package_pin_error())?;
        Ok(Self { token, sid_storage })
    }

    fn sid(&self) -> PSID {
        PSID(self.sid_storage.as_ptr().cast_mut().cast())
    }

    fn is_local_administrator(&self) -> Result<bool, HelperRunError> {
        let mut administrators = aligned_words(SECURITY_MAX_SID_SIZE as usize);
        let mut administrators_len = SECURITY_MAX_SID_SIZE;
        let administrators_sid = PSID(administrators.as_mut_ptr().cast());
        unsafe {
            CreateWellKnownSid(
                WinBuiltinAdministratorsSid,
                None,
                Some(administrators_sid),
                &mut administrators_len,
            )
        }
        .map_err(|_| package_pin_error())?;
        let mut member = BOOL::default();
        unsafe { CheckTokenMembership(Some(self.token.raw()), administrators_sid, &mut member) }
            .map_err(|_| package_pin_error())?;
        Ok(member.as_bool())
    }
}

fn aligned_words(byte_length: usize) -> Vec<usize> {
    vec![0_usize; byte_length.div_ceil(size_of::<usize>()).max(1)]
}

#[derive(Clone, Copy)]
enum ExactBridgeAcl {
    StableDirectory,
    OperationDirectory,
    PackageFile,
}

#[derive(Clone, Copy)]
enum ExpectedTrustee {
    Administrators,
    System,
    AuthenticatedUsers,
    Alice,
}

fn verify_exact_bridge_acl(
    file: &File,
    user: &FrozenUser,
    kind: ExactBridgeAcl,
) -> Result<(), HelperRunError> {
    let security = file_security(HANDLE(file.as_raw_handle()))?;
    if !is_well_known(security.owner, WinBuiltinAdministratorsSid)
        || !is_well_known(security.group, WinBuiltinAdministratorsSid)
    {
        return Err(package_pin_error());
    }
    let mut control = 0_u16;
    let mut revision = 0_u32;
    unsafe { GetSecurityDescriptorControl(security.descriptor.0, &mut control, &mut revision) }
        .map_err(|_| package_pin_error())?;
    let forbidden_control = SE_OWNER_DEFAULTED.0
        | SE_GROUP_DEFAULTED.0
        | SE_DACL_DEFAULTED.0
        | SE_DACL_AUTO_INHERIT_REQ.0
        | SE_DACL_AUTO_INHERITED.0;
    if control & SE_DACL_PRESENT.0 == 0
        || control & SE_DACL_PROTECTED.0 == 0
        || control & forbidden_control != 0
    {
        return Err(package_pin_error());
    }

    let expected = match kind {
        ExactBridgeAcl::StableDirectory => [
            (ExpectedTrustee::Administrators, BA_FULL_MASK),
            (ExpectedTrustee::System, DIRECTORY_READ_MASK),
            (ExpectedTrustee::AuthenticatedUsers, DIRECTORY_TRAVERSE_MASK),
        ],
        ExactBridgeAcl::OperationDirectory => [
            (ExpectedTrustee::Administrators, BA_FULL_MASK),
            (ExpectedTrustee::System, DIRECTORY_READ_MASK),
            (ExpectedTrustee::Alice, DIRECTORY_READ_MASK),
        ],
        ExactBridgeAcl::PackageFile => [
            (ExpectedTrustee::Administrators, BA_FULL_MASK),
            (ExpectedTrustee::System, FILE_READ_MASK),
            (ExpectedTrustee::Alice, FILE_READ_MASK),
        ],
    };
    let mut information = ACL_SIZE_INFORMATION::default();
    unsafe {
        GetAclInformation(
            security.dacl,
            (&mut information as *mut ACL_SIZE_INFORMATION).cast(),
            size_of::<ACL_SIZE_INFORMATION>() as u32,
            AclSizeInformation,
        )
    }
    .map_err(|_| package_pin_error())?;
    let acl = unsafe { &*security.dacl };
    if u32::from(acl.AclRevision) != ACL_REVISION.0
        || acl.Sbz1 != 0
        || acl.Sbz2 != 0
        || information.AceCount != expected.len() as u32
    {
        return Err(package_pin_error());
    }
    for (index, (trustee, expected_mask)) in expected.into_iter().enumerate() {
        let mut raw_ace = std::ptr::null_mut();
        unsafe { GetAce(security.dacl, index as u32, &mut raw_ace) }
            .map_err(|_| package_pin_error())?;
        if raw_ace.is_null() {
            return Err(package_pin_error());
        }
        let header = unsafe { &*(raw_ace.cast::<windows::Win32::Security::ACE_HEADER>()) };
        let sid_offset = offset_of!(ACCESS_ALLOWED_ACE, SidStart);
        let ace_size = usize::from(header.AceSize);
        let minimum_sid_bytes = 8_usize;
        if header.AceType != ACCESS_ALLOWED_ACE_TYPE as u8
            || header.AceFlags != 0
            || sid_offset
                .checked_add(minimum_sid_bytes)
                .is_none_or(|minimum| minimum > ace_size)
        {
            return Err(package_pin_error());
        }
        let ace = unsafe { &*(raw_ace.cast::<ACCESS_ALLOWED_ACE>()) };
        let sid = PSID(std::ptr::addr_of!(ace.SidStart).cast_mut().cast());
        let sid_header = unsafe { std::slice::from_raw_parts(sid.0.cast::<u8>(), 2) };
        let encoded_sid_length = minimum_sid_bytes
            .checked_add(usize::from(sid_header[1]).saturating_mul(size_of::<u32>()))
            .ok_or_else(package_pin_error)?;
        if sid_offset
            .checked_add(encoded_sid_length)
            .is_none_or(|length| length > ace_size)
        {
            return Err(package_pin_error());
        }
        let sid_length = if unsafe { IsValidSid(sid) }.as_bool() {
            usize::try_from(unsafe { GetLengthSid(sid) }).unwrap_or(0)
        } else {
            0
        };
        if ace.Mask != expected_mask
            || sid_length != encoded_sid_length
            || sid_offset
                .checked_add(sid_length)
                .is_none_or(|length| length != ace_size)
            || !trustee_matches(trustee, sid, user)
        {
            return Err(package_pin_error());
        }
    }
    Ok(())
}

fn trustee_matches(trustee: ExpectedTrustee, sid: PSID, user: &FrozenUser) -> bool {
    match trustee {
        ExpectedTrustee::Administrators => is_well_known(sid, WinBuiltinAdministratorsSid),
        ExpectedTrustee::System => is_well_known(sid, WinLocalSystemSid),
        ExpectedTrustee::AuthenticatedUsers => is_well_known(sid, WinAuthenticatedUserSid),
        ExpectedTrustee::Alice => unsafe { EqualSid(sid, user.sid()) }.is_ok(),
    }
}

fn is_well_known(sid: PSID, kind: windows::Win32::Security::WELL_KNOWN_SID_TYPE) -> bool {
    !sid.0.is_null() && unsafe { IsWellKnownSid(sid, kind) }.as_bool()
}

struct FileSecurity {
    descriptor: LocalSecurityDescriptor,
    owner: PSID,
    group: PSID,
    dacl: *mut windows::Win32::Security::ACL,
}

fn file_security(handle: HANDLE) -> Result<FileSecurity, HelperRunError> {
    let mut owner = PSID::default();
    let mut group = PSID::default();
    let mut dacl = std::ptr::null_mut();
    let mut descriptor = PSECURITY_DESCRIPTOR::default();
    let status = unsafe {
        GetSecurityInfo(
            handle,
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION | GROUP_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
            Some(&mut owner),
            Some(&mut group),
            Some(&mut dacl),
            None,
            Some(&mut descriptor),
        )
    };
    if status.0 != 0
        || descriptor.0.is_null()
        || owner.0.is_null()
        || group.0.is_null()
        || dacl.is_null()
    {
        return Err(package_pin_error());
    }
    Ok(FileSecurity {
        descriptor: LocalSecurityDescriptor(descriptor),
        owner,
        group,
        dacl,
    })
}

fn verify_effective_access(
    file: &File,
    user: &FrozenUser,
    forbidden: u32,
) -> Result<(), HelperRunError> {
    let security = file_security(HANDLE(file.as_raw_handle()))?;
    let mapping = GENERIC_MAPPING {
        GenericRead: FILE_GENERIC_READ.0,
        GenericWrite: FILE_GENERIC_WRITE.0,
        GenericExecute: FILE_GENERIC_EXECUTE.0,
        GenericAll: FILE_ALL_ACCESS.0,
    };
    let mut privileges = aligned_words(4096);
    let mut privilege_length = (privileges.len() * size_of::<usize>()) as u32;
    let mut granted = 0_u32;
    let mut access_status = BOOL::default();
    unsafe {
        AccessCheck(
            security.descriptor.0,
            user.token.raw(),
            MAXIMUM_ALLOWED,
            &mapping,
            Some(privileges.as_mut_ptr().cast::<PRIVILEGE_SET>()),
            &mut privilege_length,
            &mut granted,
            &mut access_status,
        )
    }
    .map_err(|_| package_pin_error())?;
    if !access_status.as_bool() {
        return Err(package_pin_error());
    }
    if forbidden_access_rejected(granted, forbidden, user.is_local_administrator()?) {
        return Err(package_pin_error());
    }
    Ok(())
}

fn forbidden_access_rejected(granted: u32, forbidden: u32, token_is_administrator: bool) -> bool {
    granted & forbidden != 0 && !token_is_administrator
}

struct LocalSecurityDescriptor(PSECURITY_DESCRIPTOR);

impl Drop for LocalSecurityDescriptor {
    fn drop(&mut self) {
        unsafe {
            let _ = windows::Win32::Foundation::LocalFree(Some(HLOCAL(self.0 .0)));
        }
    }
}

fn verify_builtin_administrators_owner(handle: HANDLE) -> Result<(), HelperRunError> {
    let mut owner = PSID::default();
    let mut descriptor = PSECURITY_DESCRIPTOR::default();
    let status = unsafe {
        GetSecurityInfo(
            handle,
            SE_KERNEL_OBJECT,
            OWNER_SECURITY_INFORMATION,
            Some(&mut owner),
            None,
            None,
            None,
            Some(&mut descriptor),
        )
    };
    if status.0 != 0 || descriptor.0.is_null() {
        return Err(parent_admission_error());
    }
    let _descriptor = LocalSecurityDescriptor(descriptor);
    if owner.0.is_null() || !unsafe { IsWellKnownSid(owner, WinBuiltinAdministratorsSid) }.as_bool()
    {
        return Err(parent_admission_error());
    }
    Ok(())
}

fn parent_admission_error() -> HelperRunError {
    HelperRunError::OperationFailed(HelperErrorCode::ParentAdmissionFailed)
}

fn package_pin_error() -> HelperRunError {
    HelperRunError::OperationFailed(HelperErrorCode::PackageInvalid)
}

struct ParentControls {
    admission: OwnedKernelHandle,
    cancel: OwnedKernelHandle,
}

impl ParentControls {
    fn open(request: &InstallRequest) -> Result<Self, HelperRunError> {
        // Open, never create. The elevated parent owns first-creation and the
        // DACL; accepting a helper-created replacement would turn the nonce
        // into an admission bypass after a launch timeout.
        let admission = open_sync_event(&admission_event_name(request.pipe_nonce()))?;
        let cancel = open_sync_event(&cancel_event_name(request.pipe_nonce()))?;
        Ok(Self { admission, cancel })
    }

    fn wait_for_admission(&self, timeout: Duration) -> Result<(), HelperErrorCode> {
        // Cancellation is deliberately first so a simultaneous admit/cancel
        // race never starts PackageManager.
        let result = unsafe {
            WaitForMultipleObjects(
                &[self.cancel.raw(), self.admission.raw()],
                false,
                duration_millis(timeout),
            )
        };
        if result == WAIT_OBJECT_0 {
            Err(HelperErrorCode::ParentCancelled)
        } else if result.0 == WAIT_OBJECT_0.0 + 1 {
            Ok(())
        } else {
            Err(HelperErrorCode::ParentAdmissionFailed)
        }
    }
}

fn open_sync_event(name: &str) -> Result<OwnedKernelHandle, HelperRunError> {
    let name: Vec<u16> = OsStr::new(name).encode_wide().chain(Some(0)).collect();
    let access = SYNCHRONIZATION_ACCESS_RIGHTS(USER_HELPER_CONTROL_EVENT_ACCESS_MASK);
    let handle = unsafe { OpenEventW(access, false, PCWSTR(name.as_ptr())) }
        .map_err(|_| HelperRunError::PipeUnavailable)?;
    let handle = OwnedKernelHandle::new(handle).map_err(|_| HelperRunError::PipeUnavailable)?;
    if let Err(error) = verify_builtin_administrators_owner(handle.raw()) {
        return Err(error);
    }
    Ok(handle)
}

#[derive(Clone)]
struct LocalSignal(Arc<OwnedKernelHandle>);

impl LocalSignal {
    fn new() -> Result<Self, ()> {
        let handle = unsafe { CreateEventW(None, true, false, PCWSTR::null()) }.map_err(|_| ())?;
        Ok(Self(Arc::new(OwnedKernelHandle::new(handle)?)))
    }

    fn raw(&self) -> HANDLE {
        self.0.raw()
    }
}

struct CompletionSignal {
    signal: LocalSignal,
    status: Arc<Mutex<Option<AsyncStatus>>>,
}

impl CompletionSignal {
    fn new() -> Result<Self, ()> {
        Ok(Self {
            signal: LocalSignal::new()?,
            status: Arc::new(Mutex::new(None)),
        })
    }

    fn raw(&self) -> HANDLE {
        self.signal.raw()
    }

    fn terminal_status(&self) -> Option<AsyncStatus> {
        *self
            .status
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

struct OwnedKernelHandle(OwnedHandle);

impl OwnedKernelHandle {
    fn new(handle: HANDLE) -> Result<Self, ()> {
        if handle.is_invalid() {
            Err(())
        } else {
            Ok(Self(unsafe { OwnedHandle::from_raw_handle(handle.0) }))
        }
    }

    fn raw(&self) -> HANDLE {
        HANDLE(self.0.as_raw_handle())
    }
}

struct WinRtApartment;

impl WinRtApartment {
    fn initialize() -> Result<Self, DeploymentFailure> {
        unsafe { RoInitialize(RO_INIT_MULTITHREADED) }.map_err(|_| {
            DeploymentFailure::Operation(HelperErrorCode::WinRtInitializationFailed)
        })?;
        Ok(Self)
    }
}

impl Drop for WinRtApartment {
    fn drop(&mut self) {
        unsafe { RoUninitialize() };
    }
}

enum DeploymentFailure {
    Pipe,
    Operation(HelperErrorCode),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ChannelState {
    Initial,
    HelloSent,
    ControlReceived,
    Started {
        admitted: bool,
        last_progress: Option<u8>,
    },
    Terminal,
}

struct PipeState {
    handle: OwnedHandle,
    state: ChannelState,
}

struct PipeChannel {
    state: Mutex<PipeState>,
    write_failed: AtomicBool,
}

impl PipeChannel {
    fn connect(name: &str) -> Result<Self, HelperRunError> {
        let wide_name: Vec<u16> = OsStr::new(name).encode_wide().chain(Some(0)).collect();
        // The parent creates the one and only server instance before launching
        // this process. A single CreateFileW attempt therefore avoids silently
        // attaching to a replacement endpoint after any failure.
        let handle = unsafe {
            CreateFileW(
                PCWSTR(wide_name.as_ptr()),
                USER_HELPER_PIPE_CLIENT_ACCESS_MASK,
                FILE_SHARE_MODE(0),
                None,
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL
                    | FILE_FLAG_OVERLAPPED
                    | SECURITY_SQOS_PRESENT
                    | SECURITY_IDENTIFICATION
                    | SECURITY_EFFECTIVE_ONLY,
                None,
            )
        }
        .map_err(|_| HelperRunError::PipeUnavailable)?;

        let owned = unsafe { OwnedHandle::from_raw_handle(handle.0) };
        if let Err(error) = verify_builtin_administrators_owner(HANDLE(owned.as_raw_handle())) {
            return Err(error);
        }
        Ok(Self {
            state: Mutex::new(PipeState {
                handle: owned,
                state: ChannelState::Initial,
            }),
            write_failed: AtomicBool::new(false),
        })
    }

    fn send_hello(&self) -> Result<(), HelperRunError> {
        let mut state = self.lock_state()?;
        if state.state != ChannelState::Initial {
            return self.fail_write();
        }
        write_message(&state.handle, &HelperMessage::Hello).map_err(|_| {
            self.write_failed.store(true, Ordering::Release);
            HelperRunError::PipeWriteFailed
        })?;
        state.state = ChannelState::HelloSent;
        Ok(())
    }

    fn read_bridge_control(
        &self,
        timeout: Duration,
    ) -> Result<PackageBridgeControl, HelperRunError> {
        let mut state = self.lock_state()?;
        if state.state != ChannelState::HelloSent {
            return self.fail_write();
        }
        let mut bytes = [0_u8; BRIDGE_CONTROL_BYTES];
        read_exact_overlapped(&state.handle, &mut bytes, timeout).map_err(|_| {
            self.write_failed.store(true, Ordering::Release);
            HelperRunError::PipeWriteFailed
        })?;
        let control = PackageBridgeControl::decode(&bytes).map_err(|_| parent_admission_error())?;
        state.state = ChannelState::ControlReceived;
        Ok(control)
    }

    fn send_started(&self, package: PinnedPackageIdentity) -> Result<(), HelperRunError> {
        let mut state = self.lock_state()?;
        if state.state != ChannelState::ControlReceived {
            return self.fail_write();
        }
        write_message(&state.handle, &HelperMessage::Started { package }).map_err(|_| {
            self.write_failed.store(true, Ordering::Release);
            HelperRunError::PipeWriteFailed
        })?;
        state.state = ChannelState::Started {
            admitted: false,
            last_progress: None,
        };
        Ok(())
    }

    fn mark_admitted(&self) -> Result<(), HelperRunError> {
        let mut state = self.lock_state()?;
        let ChannelState::Started {
            admitted: false,
            last_progress,
        } = state.state
        else {
            return self.fail_write();
        };
        state.state = ChannelState::Started {
            admitted: true,
            last_progress,
        };
        Ok(())
    }

    fn send_progress(&self, completed: u8) -> Result<(), HelperRunError> {
        let completed = completed.min(100);
        let mut state = self.lock_state()?;
        let ChannelState::Started {
            admitted: true,
            last_progress,
        } = state.state
        else {
            return self.fail_write();
        };
        if last_progress.is_some_and(|previous| completed <= previous) {
            return Ok(());
        }
        write_message(&state.handle, &HelperMessage::Progress { completed }).map_err(|_| {
            self.write_failed.store(true, Ordering::Release);
            HelperRunError::PipeWriteFailed
        })?;
        state.state = ChannelState::Started {
            admitted: true,
            last_progress: Some(completed),
        };
        Ok(())
    }

    fn send_prestart_error(&self, code: HelperErrorCode) -> Result<(), HelperRunError> {
        let mut state = self.lock_state()?;
        if !matches!(
            state.state,
            ChannelState::HelloSent | ChannelState::ControlReceived
        ) {
            return self.fail_write();
        }
        write_message(&state.handle, &HelperMessage::error(code)).map_err(|_| {
            self.write_failed.store(true, Ordering::Release);
            HelperRunError::PipeWriteFailed
        })?;
        state.state = ChannelState::Terminal;
        Ok(())
    }

    fn send_terminal(&self, message: HelperMessage) -> Result<(), HelperRunError> {
        if !matches!(
            message,
            HelperMessage::Success | HelperMessage::Error { .. }
        ) {
            return self.fail_write();
        }
        let mut state = self.lock_state()?;
        if !matches!(state.state, ChannelState::Started { .. }) {
            return self.fail_write();
        }
        write_message(&state.handle, &message).map_err(|_| {
            self.write_failed.store(true, Ordering::Release);
            HelperRunError::PipeWriteFailed
        })?;
        state.state = ChannelState::Terminal;
        Ok(())
    }

    fn lock_state(&self) -> Result<std::sync::MutexGuard<'_, PipeState>, HelperRunError> {
        self.state.lock().map_err(|_| {
            self.write_failed.store(true, Ordering::Release);
            HelperRunError::PipeWriteFailed
        })
    }

    fn fail_write<T>(&self) -> Result<T, HelperRunError> {
        self.write_failed.store(true, Ordering::Release);
        Err(HelperRunError::PipeWriteFailed)
    }

    fn write_failed(&self) -> bool {
        self.write_failed.load(Ordering::Acquire)
    }
}

fn write_message(handle: &OwnedHandle, message: &HelperMessage) -> Result<(), ()> {
    let frame = encode_frame(message).map_err(|_| ())?;
    write_exact_overlapped(handle, &frame, PIPE_IO_TIMEOUT)
}

fn write_exact_overlapped(handle: &OwnedHandle, bytes: &[u8], timeout: Duration) -> Result<(), ()> {
    let event = LocalSignal::new()?;
    let mut overlapped = OVERLAPPED {
        hEvent: event.raw(),
        ..Default::default()
    };
    let mut written = 0_u32;
    match unsafe {
        WriteFile(
            HANDLE(handle.as_raw_handle()),
            Some(bytes),
            Some(&mut written),
            Some(&mut overlapped),
        )
    } {
        Ok(()) => unsafe {
            GetOverlappedResult(
                HANDLE(handle.as_raw_handle()),
                &overlapped,
                &mut written,
                true,
            )
        }
        .map_err(|_| ())?,
        Err(error) if error.code() == hresult_from_win32(ERROR_IO_PENDING.0) => {
            written = wait_for_overlapped_io(HANDLE(handle.as_raw_handle()), &overlapped, timeout)?;
        }
        Err(_) => return Err(()),
    }
    if written as usize == bytes.len() {
        Ok(())
    } else {
        Err(())
    }
}

fn read_exact_overlapped(
    handle: &OwnedHandle,
    bytes: &mut [u8],
    timeout: Duration,
) -> Result<(), ()> {
    let event = LocalSignal::new()?;
    let mut overlapped = OVERLAPPED {
        hEvent: event.raw(),
        ..Default::default()
    };
    let mut read = 0_u32;
    match unsafe {
        ReadFile(
            HANDLE(handle.as_raw_handle()),
            Some(bytes),
            Some(&mut read),
            Some(&mut overlapped),
        )
    } {
        Ok(()) => unsafe {
            GetOverlappedResult(HANDLE(handle.as_raw_handle()), &overlapped, &mut read, true)
        }
        .map_err(|_| ())?,
        Err(error) if error.code() == hresult_from_win32(ERROR_IO_PENDING.0) => {
            read = wait_for_overlapped_io(HANDLE(handle.as_raw_handle()), &overlapped, timeout)?;
        }
        Err(_) => return Err(()),
    }
    if read as usize == bytes.len() {
        Ok(())
    } else {
        Err(())
    }
}

fn wait_for_overlapped_io(
    handle: HANDLE,
    overlapped: &OVERLAPPED,
    timeout: Duration,
) -> Result<u32, ()> {
    let mut transferred = 0_u32;
    match unsafe {
        GetOverlappedResultEx(
            handle,
            overlapped,
            &mut transferred,
            duration_millis(timeout),
            false,
        )
    } {
        Ok(()) => Ok(transferred),
        Err(_) => {
            unsafe {
                let _ = CancelIoEx(handle, Some(overlapped));
                let _ = GetOverlappedResult(handle, overlapped, &mut transferred, true);
            }
            Err(())
        }
    }
}

const fn hresult_from_win32(value: u32) -> HRESULT {
    HRESULT::from_win32(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use fyagent_user_helper::BridgeOperationId;

    fn operation_id() -> BridgeOperationId {
        BridgeOperationId::new([0xab; 32]).expect("nonzero operation ID")
    }

    #[test]
    fn bridge_path_is_fixed_and_operation_scoped() {
        let path = bridge_path_for_program_data(Path::new(r"C:\ProgramData"), operation_id());

        assert_eq!(
            path,
            Path::new(r"C:\ProgramData")
                .join(PACKAGE_BRIDGE_ROOT_DIRECTORY)
                .join(PACKAGE_BRIDGE_VERSION_DIRECTORY)
                .join("abababababababababababababababababababababababababababababababab")
                .join(INSTALLER_FILE_NAME)
        );
    }

    #[test]
    fn file_uri_shape_accepts_only_local_file_authority() {
        assert!(has_local_file_uri_shape(&wide(
            "file:///C:/ProgramData/package.msix"
        )));
        for rejected in [
            "fyagent:///C:/ProgramData/package.msix",
            "file://host/C:/ProgramData/package.msix",
            "file:///C:/ProgramData/package.msix?query",
            "file:///C:/ProgramData/package.msix#fragment",
            "file:/C:/ProgramData/package.msix",
        ] {
            assert!(!has_local_file_uri_shape(&wide(rejected)), "{rejected}");
        }
    }

    #[test]
    fn access_check_masks_separate_ancestor_replacement_from_bridge_writes() {
        assert_eq!(
            ANCESTOR_DANGEROUS_ACCESS & (FILE_ADD_FILE | FILE_ADD_SUBDIRECTORY).0,
            0
        );
        assert_ne!(ANCESTOR_DANGEROUS_ACCESS & FILE_DELETE_CHILD.0, 0);
        assert_ne!(ANCESTOR_DANGEROUS_ACCESS & FILE_WRITE_EA.0, 0);
        assert_ne!(BRIDGE_DIRECTORY_DANGEROUS_ACCESS & FILE_ADD_FILE.0, 0);
        assert_ne!(
            BRIDGE_DIRECTORY_DANGEROUS_ACCESS & FILE_ADD_SUBDIRECTORY.0,
            0
        );
        assert_ne!(BRIDGE_FILE_DANGEROUS_ACCESS & FILE_WRITE_DATA.0, 0);
        assert_ne!(BRIDGE_FILE_DANGEROUS_ACCESS & FILE_APPEND_DATA.0, 0);
    }

    #[test]
    fn privileged_helper_token_may_hold_os_owned_ancestors() {
        assert!(forbidden_access_rejected(
            ANCESTOR_DANGEROUS_ACCESS,
            ANCESTOR_DANGEROUS_ACCESS,
            false
        ));
        assert!(!forbidden_access_rejected(
            ANCESTOR_DANGEROUS_ACCESS,
            ANCESTOR_DANGEROUS_ACCESS,
            true
        ));
        assert!(!forbidden_access_rejected(
            0,
            ANCESTOR_DANGEROUS_ACCESS,
            false
        ));
    }

    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().collect()
    }
}
