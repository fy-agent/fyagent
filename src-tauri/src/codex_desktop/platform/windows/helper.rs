//! Parent-side boundary for the unelevated current-user package helper.
//!
//! This module owns the verified-file pin, protected one-shot ProgramData
//! bridge, duplex control pipe, client identity validation, and bounded
//! protocol consumer. The helper executable owns only the current-user
//! PackageManager call. The install call remains disconnected until the
//! downloader's verified handle and parent-owned bridge are retained for the
//! full helper operation.

use std::{
    ffi::{OsStr, OsString},
    fs::File,
    os::windows::{
        ffi::{OsStrExt, OsStringExt},
        io::{AsRawHandle, FromRawHandle, OwnedHandle},
    },
    path::{Path, PathBuf},
    sync::{Arc, Condvar, Mutex, OnceLock},
    time::{Duration, Instant},
};

use fyagent_user_helper::{
    admission_event_name, cancel_event_name, decode_frame, encode_plan_control, layout::pipe_name,
    AgentInstallerProduct, CanonicalJobId, GrokNpmInstallPlan, GrokOwner, GrokToolAction,
    HelperErrorCode, HelperMessage, HelperProtocolAction, HelperProtocolSequence,
    HelperProtocolTerminal, PackageBridgeArtifactKind, PackageBridgeControl, PinnedPackageIdentity,
    PipeNonce, ToolOperationResult, UserHelperAction, BRIDGE_CONTROL_BYTES, MAX_FRAME_BYTES,
    TOOL_OPERATION_STARTED_IDENTITY,
};
use windows::{
    core::{HRESULT, PCWSTR, PWSTR},
    Win32::{
        Foundation::{
            GetLastError, ERROR_ALREADY_EXISTS, ERROR_BROKEN_PIPE, ERROR_IO_PENDING, ERROR_NO_DATA,
            ERROR_PIPE_CONNECTED, GENERIC_READ, HANDLE, HLOCAL,
        },
        Security::{
            Authorization::{
                ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW,
                SDDL_REVISION_1,
            },
            GetTokenInformation, RevertToSelf, TokenSessionId, TokenUser, PSECURITY_DESCRIPTOR,
            PSID, SECURITY_ATTRIBUTES, TOKEN_QUERY, TOKEN_USER,
        },
        Storage::FileSystem::{
            CreateFileW, GetFileInformationByHandle, GetFinalPathNameByHandleW, ReadFile,
            WriteFile, BY_HANDLE_FILE_INFORMATION, FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_NORMAL,
            FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_FIRST_PIPE_INSTANCE,
            FILE_FLAG_OPEN_REPARSE_POINT, FILE_FLAG_OVERLAPPED, FILE_NAME_NORMALIZED,
            FILE_SHARE_READ, OPEN_EXISTING, PIPE_ACCESS_DUPLEX,
        },
        System::{
            Pipes::{
                ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe,
                GetNamedPipeClientProcessId, GetNamedPipeClientSessionId,
                ImpersonateNamedPipeClient, PIPE_READMODE_MESSAGE, PIPE_REJECT_REMOTE_CLIENTS,
                PIPE_TYPE_MESSAGE, PIPE_WAIT,
            },
            Threading::{
                CreateEventW, GetCurrentThread, OpenProcess, OpenThreadToken,
                QueryFullProcessImageNameW, SetEvent, PROCESS_NAME_WIN32,
                PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE,
            },
            IO::{CancelIoEx, GetOverlappedResult, GetOverlappedResultEx, OVERLAPPED},
        },
    },
};

use super::{
    package_bridge::ProtectedPackageBridge, PlatformProgressSink, PreparedInstallPackage,
    WindowsContextRevalidator, WindowsFilePinFactory, WindowsHelperDeadlines,
    WindowsPackageFileIdentity, WindowsUserHelperRunner, WindowsVerifiedFilePin,
};
use crate::{
    codex_desktop::{
        download::DownloadedArtifact,
        error::{InstallerError, InstallerErrorCode},
        types::{JobProgress, ProgressPhase},
    },
    platform::process_launch::{
        fixed_user_helper_path, launch_fyagent_user_helper_as_user, ProcessLaunchError,
        UserHelperLaunchOutcome,
    },
    windows_runtime::InteractiveUserContext,
};

const PIPE_DEFAULT_TIMEOUT_MS: u32 = 30_000;
/// Longest time a helper request waits for an active helper lifetime to
/// finish. Parallel readiness reads queue here instead of failing at once.
const HELPER_GATE_WAIT: Duration = Duration::from_secs(30);

pub(super) struct SystemWindowsContextRevalidator;

impl WindowsContextRevalidator for SystemWindowsContextRevalidator {
    fn is_current(&self, context: &InteractiveUserContext) -> bool {
        crate::windows_runtime::revalidate_interactive_user_context(context)
    }
}

pub(super) struct SystemWindowsFilePinFactory;

impl WindowsFilePinFactory for SystemWindowsFilePinFactory {
    fn open(
        &self,
        package: &PreparedInstallPackage,
    ) -> Result<Box<dyn WindowsVerifiedFilePin>, InstallerError> {
        VerifiedFilePin::open(package).map(|pin| Box::new(pin) as Box<dyn WindowsVerifiedFilePin>)
    }
}

pub(super) struct SystemWindowsUserHelperRunner;

impl WindowsUserHelperRunner for SystemWindowsUserHelperRunner {
    fn run(
        &self,
        context: &InteractiveUserContext,
        job_id: &str,
        pin: Box<dyn WindowsVerifiedFilePin>,
        progress: PlatformProgressSink,
        deadlines: WindowsHelperDeadlines,
    ) -> Result<(), InstallerError> {
        let job_id = CanonicalJobId::parse(job_id).map_err(|_| helper_identity_error())?;
        run_pinned_user_helper(
            context,
            UserHelperAction::CodexMsixInstall,
            &job_id,
            pin,
            progress,
            deadlines,
        )
    }
}

struct VerifiedFilePin {
    file: Mutex<File>,
    identity: FileIdentity,
    expected_size: u64,
    expected_sha256: String,
}

impl VerifiedFilePin {
    fn open(package: &PreparedInstallPackage) -> Result<Self, InstallerError> {
        let file = package.open_artifact_for_pinning()?;
        let identity = checked_file_identity(HANDLE(file.as_raw_handle()), package.actual_size())?;
        Ok(Self {
            file: Mutex::new(file),
            identity,
            expected_size: package.actual_size(),
            expected_sha256: package.local_sha256().to_owned(),
        })
    }

    fn open_downloaded_artifact(artifact: &DownloadedArtifact) -> Result<Self, InstallerError> {
        let file = artifact.open_for_read()?;
        let identity = checked_file_identity(HANDLE(file.as_raw_handle()), artifact.actual_size())?;
        Ok(Self {
            file: Mutex::new(file),
            identity,
            expected_size: artifact.actual_size(),
            expected_sha256: artifact.local_sha256().to_owned(),
        })
    }
}

pub(crate) fn run_verified_agent_exe_installer(
    context: &InteractiveUserContext,
    product: AgentInstallerProduct,
    job_id: &str,
    artifact: &DownloadedArtifact,
    progress: PlatformProgressSink,
) -> Result<(), InstallerError> {
    let job_id = CanonicalJobId::parse(job_id).map_err(|_| helper_identity_error())?;
    artifact.revalidate()?;
    let pin = VerifiedFilePin::open_downloaded_artifact(artifact)?;
    pin.recheck()?;
    run_pinned_user_helper(
        context,
        UserHelperAction::AgentExeInstall(product),
        &job_id,
        Box::new(pin),
        progress,
        WindowsHelperDeadlines::AGENT_EXE_INSTALL,
    )
}

pub(crate) fn run_grok_tool_operation(
    context: &InteractiveUserContext,
    action: GrokToolAction,
    expected_owner: Option<GrokOwner>,
    npm_plan: Option<GrokNpmInstallPlan>,
) -> Result<ToolOperationResult, InstallerError> {
    let job_id = CanonicalJobId::parse(&uuid::Uuid::new_v4().to_string())
        .map_err(|_| helper_identity_error())?;
    run_unpinned_tool_helper(
        context,
        UserHelperAction::GrokTool {
            action,
            expected_owner,
        },
        &job_id,
        npm_plan,
        Arc::new(|_: JobProgress| {}),
        WindowsHelperDeadlines::GROK_TOOL,
    )
}

pub(crate) fn run_claude_tool_operation(
    context: &InteractiveUserContext,
    action: GrokToolAction,
    npm_plan: Option<GrokNpmInstallPlan>,
) -> Result<ToolOperationResult, InstallerError> {
    let job_id = CanonicalJobId::parse(&uuid::Uuid::new_v4().to_string())
        .map_err(|_| helper_identity_error())?;
    run_unpinned_tool_helper(
        context,
        UserHelperAction::ClaudeTool { action },
        &job_id,
        npm_plan,
        Arc::new(|_: JobProgress| {}),
        WindowsHelperDeadlines::GROK_TOOL,
    )
}

fn run_unpinned_tool_helper(
    context: &InteractiveUserContext,
    action: UserHelperAction,
    job_id: &CanonicalJobId,
    npm_plan: Option<GrokNpmInstallPlan>,
    progress: PlatformProgressSink,
    deadlines: WindowsHelperDeadlines,
) -> Result<ToolOperationResult, InstallerError> {
    if action.requires_package_bridge() {
        return Err(helper_pipe_error(
            "the user-helper tool action required a package bridge",
        ));
    }
    let gate = HelperGateLease::acquire()?;
    let setup = (|| {
        let nonce = generate_nonce()?;
        let controls = ParentControlEvents::create(context.canonical_sid(), &nonce)?;
        let server = OneShotPipeServer::create(context.canonical_sid(), &nonce)?;
        let helper_path = fixed_user_helper_path().map_err(|_| helper_launch_error())?;
        let helper_image = PinnedHelperImage::open(&helper_path)?;
        Ok::<_, InstallerError>((nonce, controls, server, helper_image))
    })();
    let (nonce, controls, server, helper_image) = match setup {
        Ok(setup) => setup,
        Err(error) => {
            gate.finish();
            return Err(error);
        }
    };
    let mut lifetime = HelperLifetime::without_package(helper_image, controls, server);

    match launch_fyagent_user_helper_as_user(action, job_id, &nonce) {
        UserHelperLaunchOutcome::Confirmed => {}
        UserHelperLaunchOutcome::MayHaveLaunched => {
            return fail_before_admission(gate, lifetime, helper_launch_pending_error())
                .map(|_| unreachable!());
        }
        UserHelperLaunchOutcome::NotInvoked(reason) => {
            return fail_before_admission(gate, lifetime, helper_not_invoked_error(reason))
                .map(|_| unreachable!());
        }
    }
    if lifetime.server().connect(deadlines.connect).is_err() {
        return fail_before_admission(
            gate,
            lifetime,
            helper_pipe_error("the user-helper did not connect before its deadline"),
        )
        .map(|_| unreachable!());
    }
    let operation_deadline = Instant::now() + deadlines.operation;
    let first_frame_timeout = match remaining_until(operation_deadline) {
        Ok(remaining) => remaining.min(deadlines.connect),
        Err(error) => return fail_before_admission(gate, lifetime, error).map(|_| unreachable!()),
    };
    let first_frame = match lifetime.server().read_frame(first_frame_timeout) {
        Err(error) => return fail_before_admission(gate, lifetime, error).map(|_| unreachable!()),
        Ok(PipeFrameRead::Frame(frame)) => frame,
        Ok(PipeFrameRead::Closed) => {
            return fail_before_admission(
                gate,
                lifetime,
                helper_pipe_error("the user-helper pipe closed before its identity was admitted"),
            )
            .map(|_| unreachable!())
        }
    };
    let process = match lifetime.server().validate_client(
        context,
        lifetime
            .helper_image
            .as_ref()
            .expect("helper lifetime always owns helper image"),
    ) {
        Ok(process) => process,
        Err(error) => return fail_before_admission(gate, lifetime, error).map(|_| unreachable!()),
    };
    lifetime.set_process(process);
    let first_message = match decode_protocol_frame(&first_frame) {
        Ok(message) => message,
        Err(error) => return fail_before_admission(gate, lifetime, error).map(|_| unreachable!()),
    };
    let mut sequence = HelperProtocolSequence::default();
    match sequence.accept(first_message) {
        Ok(HelperProtocolAction::Hello(received_action)) if received_action == action => {}
        _ => {
            return fail_before_admission(
                gate,
                lifetime,
                helper_pipe_error("the user-helper action did not match the admitted request"),
            )
            .map(|_| unreachable!())
        }
    }
    if !crate::windows_runtime::revalidate_interactive_user_context(context) {
        return fail_before_admission(gate, lifetime, helper_context_error())
            .map(|_| unreachable!());
    }
    if sequence.mark_control_sent().is_err() {
        return fail_before_admission(
            gate,
            lifetime,
            helper_pipe_error("the helper tool control transition was invalid"),
        )
        .map(|_| unreachable!());
    }
    let grok_plan_timeout = match remaining_until(operation_deadline) {
        Ok(remaining) => remaining.min(deadlines.connect),
        Err(error) => return fail_before_admission(gate, lifetime, error).map(|_| unreachable!()),
    };
    let control = encode_plan_control(npm_plan.as_ref());
    if lifetime
        .server()
        .write_control_bytes(&control, grok_plan_timeout)
        .is_err()
    {
        return fail_before_admission(
            gate,
            lifetime,
            helper_pipe_error("the grok npm plan could not be sent to the helper"),
        )
        .map(|_| unreachable!());
    }

    let started_timeout = match remaining_until(operation_deadline) {
        Ok(remaining) => remaining.min(deadlines.connect),
        Err(error) => return fail_before_admission(gate, lifetime, error).map(|_| unreachable!()),
    };
    let started_message = match lifetime.server().read_message(started_timeout) {
        Ok(PipeMessageRead::Message(message)) => message,
        Ok(PipeMessageRead::Closed) => {
            return fail_before_admission(
                gate,
                lifetime,
                helper_pipe_error("the user-helper pipe closed before tool admission"),
            )
            .map(|_| unreachable!())
        }
        Err(error) => return fail_before_admission(gate, lifetime, error).map(|_| unreachable!()),
    };
    let helper_identity = match sequence.accept(started_message) {
        Ok(HelperProtocolAction::Started(identity)) => identity,
        Ok(HelperProtocolAction::Failure(code)) => {
            let terminal = HelperProtocolTerminal::Failure(code);
            if let Err(error) = wait_for_clean_terminal_close(
                lifetime.server(),
                &mut sequence,
                operation_deadline,
                deadlines.terminal_close,
            ) {
                return fail_before_admission(gate, lifetime, error).map(|_| unreachable!());
            }
            return finish_settled_tool(gate, lifetime, terminal);
        }
        _ => {
            return fail_before_admission(
                gate,
                lifetime,
                helper_pipe_error("the user-helper did not confirm the tool operation"),
            )
            .map(|_| unreachable!())
        }
    };
    if helper_identity != TOOL_OPERATION_STARTED_IDENTITY {
        return fail_before_admission(gate, lifetime, package_pin_error()).map(|_| unreachable!());
    }
    if !crate::windows_runtime::revalidate_interactive_user_context(context) {
        return fail_before_admission(gate, lifetime, helper_context_error())
            .map(|_| unreachable!());
    }
    if lifetime.controls().admit().is_err() {
        return fail_before_admission(
            gate,
            lifetime,
            helper_pipe_error("the helper admission event could not be signaled"),
        )
        .map(|_| unreachable!());
    }
    lifetime.mark_admitted();
    if sequence.mark_admitted().is_err() {
        gate.quarantine(
            lifetime,
            helper_pipe_error("the helper admission transition was invalid"),
        );
    }

    match consume_protocol(
        lifetime.server(),
        &mut sequence,
        progress,
        operation_deadline,
        deadlines.terminal_close,
    ) {
        Ok(terminal) => finish_settled_tool(gate, lifetime, terminal),
        Err(error) => cancel_and_quarantine(gate, lifetime, error),
    }
}

impl WindowsVerifiedFilePin for VerifiedFilePin {
    fn recheck(&self) -> Result<(), InstallerError> {
        let file = self.file.lock().map_err(|_| package_pin_error())?;
        if checked_file_identity(HANDLE(file.as_raw_handle()), self.expected_size)? != self.identity
        {
            return Err(package_pin_error());
        }
        Ok(())
    }

    fn identity(&self) -> WindowsPackageFileIdentity {
        WindowsPackageFileIdentity {
            volume_serial: u64::from(self.identity.volume_serial_number),
            file_index: self.identity.file_index,
            size: self.identity.size,
        }
    }

    fn expected_size(&self) -> u64 {
        self.expected_size
    }

    fn expected_sha256(&self) -> &str {
        &self.expected_sha256
    }

    fn duplicate_source_file(&self) -> Result<File, InstallerError> {
        self.file
            .lock()
            .map_err(|_| package_pin_error())?
            .try_clone()
            .map_err(|_| package_pin_error())
    }
}

/// Runs the fixed helper while retaining the verified source, sealed bridge,
/// and helper-image handles until PackageManager has a proven terminal outcome.
fn run_pinned_user_helper(
    context: &InteractiveUserContext,
    action: UserHelperAction,
    job_id: &CanonicalJobId,
    pin: Box<dyn WindowsVerifiedFilePin>,
    progress: PlatformProgressSink,
    deadlines: WindowsHelperDeadlines,
) -> Result<(), InstallerError> {
    let gate = HelperGateLease::acquire()?;
    pin.recheck()?;
    let expected_size = pin.expected_size();
    let source_identity = pin.identity();
    if expected_size == 0 || source_identity.size != expected_size {
        return Err(package_pin_error());
    }
    let mut source_file = pin.duplicate_source_file()?;
    let cloned_identity =
        checked_file_identity(HANDLE(source_file.as_raw_handle()), expected_size)?;
    if u64::from(cloned_identity.volume_serial_number) != source_identity.volume_serial
        || cloned_identity.file_index != source_identity.file_index
        || cloned_identity.size != source_identity.size
    {
        return Err(package_pin_error());
    }
    let bridge = ProtectedPackageBridge::create(
        context.canonical_sid(),
        &mut source_file,
        expected_size,
        pin.expected_sha256(),
        action.artifact_kind(),
    );
    drop(source_file);
    let bridge = bridge?;

    let setup = (|| {
        let nonce = generate_nonce()?;
        let controls = ParentControlEvents::create(context.canonical_sid(), &nonce)?;
        let server = OneShotPipeServer::create(context.canonical_sid(), &nonce)?;
        let helper_path = fixed_user_helper_path().map_err(|_| helper_launch_error())?;
        let helper_image = PinnedHelperImage::open(&helper_path)?;
        Ok::<_, InstallerError>((nonce, controls, server, helper_image))
    })();
    let (nonce, controls, server, helper_image) = match setup {
        Ok(setup) => setup,
        Err(error) => {
            let _ = bridge.cleanup();
            gate.finish();
            return Err(error);
        }
    };
    let mut lifetime = HelperLifetime::new(pin, bridge, helper_image, controls, server);

    match launch_fyagent_user_helper_as_user(action, job_id, &nonce) {
        UserHelperLaunchOutcome::Confirmed => {}
        UserHelperLaunchOutcome::MayHaveLaunched => {
            return fail_before_admission(gate, lifetime, helper_launch_pending_error());
        }
        UserHelperLaunchOutcome::NotInvoked(reason) => {
            return fail_before_admission(gate, lifetime, helper_not_invoked_error(reason));
        }
    }
    if lifetime.server().connect(deadlines.connect).is_err() {
        return fail_before_admission(
            gate,
            lifetime,
            helper_pipe_error("the user-helper did not connect before its deadline"),
        );
    }
    let operation_deadline = Instant::now() + deadlines.operation;
    // ImpersonateNamedPipeClient binds to the last message read. Read one
    // bounded frame without decoding or accepting it, authenticate that
    // connection, and only then admit the frame into the protocol state.
    let first_frame_timeout = match remaining_until(operation_deadline) {
        Ok(remaining) => remaining.min(deadlines.connect),
        Err(error) => return fail_before_admission(gate, lifetime, error),
    };
    let first_frame = match lifetime.server().read_frame(first_frame_timeout) {
        Err(error) => return fail_before_admission(gate, lifetime, error),
        Ok(PipeFrameRead::Frame(frame)) => frame,
        Ok(PipeFrameRead::Closed) => {
            return fail_before_admission(
                gate,
                lifetime,
                helper_pipe_error("the user-helper pipe closed before its identity was admitted"),
            )
        }
    };
    let process = match lifetime.server().validate_client(
        context,
        lifetime
            .helper_image
            .as_ref()
            .expect("helper lifetime always owns helper image"),
    ) {
        Ok(process) => process,
        Err(error) => return fail_before_admission(gate, lifetime, error),
    };
    lifetime.set_process(process);
    let first_message = match decode_protocol_frame(&first_frame) {
        Ok(message) => message,
        Err(error) => return fail_before_admission(gate, lifetime, error),
    };
    let mut sequence = HelperProtocolSequence::default();
    match sequence.accept(first_message) {
        Ok(HelperProtocolAction::Hello(received_action)) if received_action == action => {}
        _ => {
            return fail_before_admission(
                gate,
                lifetime,
                helper_pipe_error("the user-helper action did not match the admitted request"),
            )
        }
    }
    if !crate::windows_runtime::revalidate_interactive_user_context(context) {
        return fail_before_admission(gate, lifetime, helper_context_error());
    }
    if let Err(error) = lifetime.bridge().recheck() {
        return fail_before_admission(gate, lifetime, error);
    }
    let bridge_control_timeout = match remaining_until(operation_deadline) {
        Ok(remaining) => remaining.min(deadlines.connect),
        Err(error) => return fail_before_admission(gate, lifetime, error),
    };
    let bridge_control = lifetime.bridge().control();
    if lifetime
        .server()
        .send_bridge_control(bridge_control, bridge_control_timeout)
        .is_err()
    {
        return fail_before_admission(
            gate,
            lifetime,
            helper_pipe_error("the protected package bridge could not be sent to the helper"),
        );
    }
    if sequence.mark_control_sent().is_err() {
        return fail_before_admission(
            gate,
            lifetime,
            helper_pipe_error("the helper bridge control transition was invalid"),
        );
    }

    let started_timeout = match remaining_until(operation_deadline) {
        Ok(remaining) => remaining.min(deadlines.connect),
        Err(error) => return fail_before_admission(gate, lifetime, error),
    };
    let started_message = match lifetime.server().read_message(started_timeout) {
        Ok(PipeMessageRead::Message(message)) => message,
        Ok(PipeMessageRead::Closed) => {
            return fail_before_admission(
                gate,
                lifetime,
                helper_pipe_error("the user-helper pipe closed before bridge admission"),
            )
        }
        Err(error) => return fail_before_admission(gate, lifetime, error),
    };
    let helper_package_identity = match sequence.accept(started_message) {
        Ok(HelperProtocolAction::Started(identity)) => identity,
        Ok(HelperProtocolAction::Failure(code)) => {
            let terminal = HelperProtocolTerminal::Failure(code);
            if let Err(error) = wait_for_clean_terminal_close(
                lifetime.server(),
                &mut sequence,
                operation_deadline,
                deadlines.terminal_close,
            ) {
                return fail_before_admission(gate, lifetime, error);
            }
            return finish_settled(gate, lifetime, terminal);
        }
        _ => {
            return fail_before_admission(
                gate,
                lifetime,
                helper_pipe_error("the user-helper did not confirm the protected bridge"),
            )
        }
    };
    if !bridge_identity_matches(helper_package_identity, lifetime.bridge().identity()) {
        return fail_before_admission(gate, lifetime, package_pin_error());
    }
    if !crate::windows_runtime::revalidate_interactive_user_context(context) {
        return fail_before_admission(gate, lifetime, helper_context_error());
    }
    if let Err(error) = lifetime.bridge().recheck() {
        return fail_before_admission(gate, lifetime, error);
    }
    if lifetime.controls().admit().is_err() {
        return fail_before_admission(
            gate,
            lifetime,
            helper_pipe_error("the helper admission event could not be signaled"),
        );
    }
    lifetime.mark_admitted();
    if sequence.mark_admitted().is_err() {
        gate.quarantine(
            lifetime,
            helper_pipe_error("the helper admission transition was invalid"),
        );
    }

    match consume_protocol(
        lifetime.server(),
        &mut sequence,
        progress,
        operation_deadline,
        deadlines.terminal_close,
    ) {
        Ok(terminal) => finish_settled(gate, lifetime, terminal),
        // Any post-admission protocol, timeout, or transport failure destroys
        // the authenticated settlement transcript. A later terminal frame
        // must not wash that failure away and release the package lifetime.
        Err(error) => cancel_and_quarantine(gate, lifetime, error),
    }
}

fn generate_nonce() -> Result<PipeNonce, InstallerError> {
    let random = generate_random_256("the user-helper pipe nonce could not be generated")?;
    let mut encoded = String::with_capacity(64);
    for byte in random {
        use std::fmt::Write as _;
        write!(&mut encoded, "{byte:02x}")
            .map_err(|_| helper_pipe_error("the user-helper pipe nonce could not be encoded"))?;
    }
    PipeNonce::parse(&encoded)
        .map_err(|_| helper_pipe_error("the user-helper pipe nonce was invalid"))
}

fn generate_random_256(error_message: &'static str) -> Result<[u8; 32], InstallerError> {
    use windows::Win32::Security::Cryptography::{
        BCryptGenRandom, BCRYPT_USE_SYSTEM_PREFERRED_RNG,
    };

    let mut random = [0_u8; 32];
    let status = unsafe { BCryptGenRandom(None, &mut random, BCRYPT_USE_SYSTEM_PREFERRED_RNG) };
    if status.0 < 0 {
        return Err(helper_pipe_error(error_message));
    }
    Ok(random)
}

struct ParentControlEvents {
    admission: ParentControlEvent,
    cancel: ParentControlEvent,
}

impl ParentControlEvents {
    fn create(shell_sid: &str, nonce: &PipeNonce) -> Result<Self, InstallerError> {
        // Both names are first-created before Explorer sees the nonce. The
        // unelevated helper can synchronize and read the owner only; the
        // creator's existing handles are the sole EVENT_MODIFY_STATE
        // capability. Explicit BA ownership is the cross-account authority
        // proof checked by the helper before it reports Started.
        let admission = ParentControlEvent::create(
            shell_sid,
            &admission_event_name(nonce),
            "the helper admission event could not be created",
        )?;
        let cancel = ParentControlEvent::create(
            shell_sid,
            &cancel_event_name(nonce),
            "the helper cancellation event could not be created",
        )?;
        Ok(Self { admission, cancel })
    }

    fn admit(&self) -> Result<(), InstallerError> {
        self.admission.signal()
    }

    fn cancel(&self) -> Result<(), InstallerError> {
        self.cancel.signal()
    }
}

struct ParentControlEvent(OwnedWin32Handle);

impl ParentControlEvent {
    fn create(
        shell_sid: &str,
        name: &str,
        error_message: &'static str,
    ) -> Result<Self, InstallerError> {
        let security = EventSecurityDescriptor::new(shell_sid)?;
        let attributes = SECURITY_ATTRIBUTES {
            nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: security.as_ptr(),
            bInheritHandle: false.into(),
        };
        let name = wide_null(name);
        let handle = unsafe { CreateEventW(Some(&attributes), true, false, PCWSTR(name.as_ptr())) }
            .map_err(|_| helper_pipe_error(error_message))?;
        // GetLastError must be sampled immediately: CreateEventW succeeds for
        // an existing object, but accepting that handle would trust its DACL
        // and signaled state.
        let already_existed = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;
        let handle = OwnedWin32Handle::new(handle)?;
        if already_existed {
            return Err(helper_pipe_error(
                "the helper control event name was already in use",
            ));
        }
        Ok(Self(handle))
    }

    fn signal(&self) -> Result<(), InstallerError> {
        unsafe { SetEvent(self.0.raw()) }
            .map_err(|_| helper_pipe_error("the helper control event could not be signaled"))
    }
}

struct AdmittedHelperProcess {
    // Retain the authenticated process object through settlement or quarantine.
    _process_handle: OwnedWin32Handle,
    _image: PinnedHelperImage,
}

struct HelperLifetime {
    pin: Option<Box<dyn WindowsVerifiedFilePin>>,
    bridge: Option<ProtectedPackageBridge>,
    helper_image: Option<PinnedHelperImage>,
    controls: Option<ParentControlEvents>,
    server: Option<OneShotPipeServer>,
    process: Option<AdmittedHelperProcess>,
    admitted: bool,
    settled: bool,
}

impl HelperLifetime {
    fn new(
        pin: Box<dyn WindowsVerifiedFilePin>,
        bridge: ProtectedPackageBridge,
        helper_image: PinnedHelperImage,
        controls: ParentControlEvents,
        server: OneShotPipeServer,
    ) -> Self {
        Self {
            pin: Some(pin),
            bridge: Some(bridge),
            helper_image: Some(helper_image),
            controls: Some(controls),
            server: Some(server),
            process: None,
            admitted: false,
            settled: false,
        }
    }

    fn without_package(
        helper_image: PinnedHelperImage,
        controls: ParentControlEvents,
        server: OneShotPipeServer,
    ) -> Self {
        Self {
            pin: None,
            bridge: None,
            helper_image: Some(helper_image),
            controls: Some(controls),
            server: Some(server),
            process: None,
            admitted: false,
            settled: false,
        }
    }

    fn controls(&self) -> &ParentControlEvents {
        self.controls
            .as_ref()
            .expect("helper lifetime always owns controls")
    }

    fn bridge(&self) -> &ProtectedPackageBridge {
        self.bridge
            .as_ref()
            .expect("helper lifetime always owns package bridge")
    }

    fn server(&self) -> &OneShotPipeServer {
        self.server
            .as_ref()
            .expect("helper lifetime always owns pipe")
    }

    fn set_process(&mut self, process: AdmittedHelperProcess) {
        self.process = Some(process);
    }

    fn mark_admitted(&mut self) {
        self.admitted = true;
    }

    fn mark_settled(&mut self) {
        self.settled = true;
    }

    fn cleanup_bridge(mut self) {
        debug_assert!(!self.admitted || self.settled);
        // Close every helper capability before asking the bridge to delete its
        // exact known objects. A cleanup error intentionally leaves an
        // immutable diagnostic orphan and never replaces the install result.
        drop(self.server.take());
        drop(self.process.take());
        drop(self.controls.take());
        drop(self.helper_image.take());
        if let Some(bridge) = self.bridge.take() {
            let _ = bridge.cleanup();
        }
        drop(self.pin.take());
    }
}

impl Drop for HelperLifetime {
    fn drop(&mut self) {
        if self.admitted && !self.settled {
            // A panic or task unwind after the irreversible admission signal
            // must not release either pin and then let the service publish a
            // terminal job. Move every handle into the same one-slot
            // quarantine used by protocol failures, then stop unwinding.
            retain_quarantined_lifetime(Self {
                pin: self.pin.take(),
                bridge: self.bridge.take(),
                helper_image: self.helper_image.take(),
                controls: self.controls.take(),
                server: self.server.take(),
                process: self.process.take(),
                admitted: true,
                settled: false,
            });
            loop {
                std::thread::park();
            }
        }
        // A late helper must lose its pipe before names for the unsignaled
        // controls can disappear. The protected bridge and verified source
        // pin are deliberately released only after those capabilities.
        drop(self.server.take());
        drop(self.process.take());
        drop(self.controls.take());
        drop(self.helper_image.take());
        drop(self.bridge.take());
        drop(self.pin.take());
    }
}

static HELPER_GATE: OnceLock<Mutex<HelperGateState>> = OnceLock::new();
static HELPER_GATE_RELEASED: Condvar = Condvar::new();

enum HelperGateState<R = HelperLifetime> {
    Idle,
    Active,
    Quarantined { _lifetime: Box<R> },
}

/// Enters the single helper gate. An active lifetime is waited for up to
/// `wait`; a quarantined lifetime still fails at once because it has no
/// terminal proof and must never be released by a later request.
fn enter_helper_gate<R>(
    gate: &Mutex<HelperGateState<R>>,
    released: &Condvar,
    wait: Duration,
) -> Result<(), InstallerError> {
    let deadline = Instant::now() + wait;
    let mut state = gate.lock().map_err(|_| helper_quarantine_error())?;
    loop {
        match &*state {
            HelperGateState::Idle => {
                *state = HelperGateState::Active;
                return Ok(());
            }
            HelperGateState::Quarantined { .. } => return Err(helper_quarantine_error()),
            HelperGateState::Active => {
                let remaining = deadline.saturating_duration_since(Instant::now());
                if remaining.is_zero() {
                    return Err(helper_busy_error());
                }
                state = released
                    .wait_timeout(state, remaining)
                    .map_err(|_| helper_quarantine_error())?
                    .0;
            }
        }
    }
}

fn leave_helper_gate<R>(gate: &Mutex<HelperGateState<R>>, released: &Condvar) {
    if let Ok(mut state) = gate.lock() {
        if matches!(*state, HelperGateState::Active) {
            *state = HelperGateState::Idle;
        }
    }
    released.notify_all();
}

fn retain_quarantined_lifetime(lifetime: HelperLifetime) {
    match HELPER_GATE
        .get_or_init(|| Mutex::new(HelperGateState::Idle))
        .lock()
    {
        Ok(mut state) if matches!(*state, HelperGateState::Active) => {
            *state = HelperGateState::Quarantined {
                _lifetime: Box::new(lifetime),
            };
        }
        _ => {
            // A poisoned or inconsistent gate must fail closed. Leaking is
            // intentional: releasing this pin without terminal proof would
            // reopen the package-replacement race.
            Box::leak(Box::new(lifetime));
        }
    }
    // Waiters must observe the quarantine now instead of timing out later.
    HELPER_GATE_RELEASED.notify_all();
}

struct HelperGateLease {
    active: bool,
}

impl HelperGateLease {
    fn acquire() -> Result<Self, InstallerError> {
        enter_helper_gate(
            HELPER_GATE.get_or_init(|| Mutex::new(HelperGateState::Idle)),
            &HELPER_GATE_RELEASED,
            HELPER_GATE_WAIT,
        )
        .inspect_err(log_helper_failure)?;
        Ok(Self { active: true })
    }

    fn finish(mut self) {
        leave_helper_gate(
            HELPER_GATE.get_or_init(|| Mutex::new(HelperGateState::Idle)),
            &HELPER_GATE_RELEASED,
        );
        self.active = false;
    }

    fn quarantine(mut self, lifetime: HelperLifetime, _error: InstallerError) -> ! {
        retain_quarantined_lifetime(lifetime);
        self.active = false;
        // The caller is the blocking installer worker. Never returning keeps
        // its job in Installing, so the application exit/restart lifecycle
        // gate cannot release the quarantined handles through normal UI flows.
        loop {
            std::thread::park();
        }
    }
}

impl Drop for HelperGateLease {
    fn drop(&mut self) {
        if !self.active {
            return;
        }
        leave_helper_gate(
            HELPER_GATE.get_or_init(|| Mutex::new(HelperGateState::Idle)),
            &HELPER_GATE_RELEASED,
        );
    }
}

struct OneShotPipeServer {
    handle: OwnedHandle,
}

impl OneShotPipeServer {
    fn create(shell_sid: &str, nonce: &PipeNonce) -> Result<Self, InstallerError> {
        let security = PipeSecurityDescriptor::new(shell_sid)?;
        let attributes = SECURITY_ATTRIBUTES {
            nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: security.as_ptr(),
            bInheritHandle: false.into(),
        };
        let name = wide_null(&pipe_name(nonce));
        let handle = unsafe {
            CreateNamedPipeW(
                PCWSTR(name.as_ptr()),
                PIPE_ACCESS_DUPLEX | FILE_FLAG_FIRST_PIPE_INSTANCE | FILE_FLAG_OVERLAPPED,
                PIPE_TYPE_MESSAGE | PIPE_READMODE_MESSAGE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
                1,
                BRIDGE_CONTROL_BYTES as u32,
                MAX_FRAME_BYTES as u32,
                PIPE_DEFAULT_TIMEOUT_MS,
                Some(&attributes),
            )
        };
        if handle.is_invalid() {
            return Err(helper_pipe_error(
                "the one-shot user-helper pipe could not be created",
            ));
        }
        Ok(Self {
            handle: unsafe { OwnedHandle::from_raw_handle(handle.0) },
        })
    }

    fn raw(&self) -> HANDLE {
        HANDLE(self.handle.as_raw_handle())
    }

    fn connect(&self, timeout: Duration) -> Result<(), InstallerError> {
        let event = OwnedEvent::new()?;
        let mut overlapped = OVERLAPPED {
            hEvent: event.raw(),
            ..Default::default()
        };
        match unsafe { ConnectNamedPipe(self.raw(), Some(&mut overlapped)) } {
            Ok(()) => Ok(()),
            Err(error) if error.code() == hresult_from_win32(ERROR_PIPE_CONNECTED.0) => Ok(()),
            Err(error) if error.code() == hresult_from_win32(ERROR_IO_PENDING.0) => {
                wait_for_overlapped(self.raw(), &overlapped, timeout).map(|_| ())
            }
            Err(_) => Err(helper_pipe_error(
                "the user-helper did not connect to its one-shot pipe",
            )),
        }
    }

    fn validate_client(
        &self,
        context: &InteractiveUserContext,
        expected_image: &PinnedHelperImage,
    ) -> Result<AdmittedHelperProcess, InstallerError> {
        let mut process_id = 0_u32;
        let mut pipe_session_id = 0_u32;
        unsafe { GetNamedPipeClientProcessId(self.raw(), &mut process_id) }
            .map_err(|_| helper_identity_error())?;
        unsafe { GetNamedPipeClientSessionId(self.raw(), &mut pipe_session_id) }
            .map_err(|_| helper_identity_error())?;
        if process_id == 0 || pipe_session_id != context.shell_session_id() {
            return Err(helper_identity_error());
        }

        let process = unsafe {
            OpenProcess(
                PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE,
                false,
                process_id,
            )
        }
        .map_err(|_| helper_identity_error())?;
        let process = OwnedWin32Handle::new(process)?;
        let image = process_image_path(process.raw())?;
        let connected_image = PinnedHelperImage::open(&image)?;
        if connected_image.canonical_path() != expected_image.canonical_path()
            || connected_image.identity() != expected_image.identity()
        {
            return Err(helper_identity_error());
        }

        let (connection_token_sid, connection_token_session_id) =
            connected_client_token_identity(self.raw())?;

        if connection_token_sid != context.canonical_sid()
            || connection_token_session_id != context.shell_session_id()
        {
            return Err(helper_identity_error());
        }
        Ok(AdmittedHelperProcess {
            _process_handle: process,
            _image: connected_image,
        })
    }

    fn read_frame(&self, remaining: Duration) -> Result<PipeFrameRead, InstallerError> {
        let event = OwnedEvent::new()?;
        let mut overlapped = OVERLAPPED {
            hEvent: event.raw(),
            ..Default::default()
        };
        let mut frame = [0_u8; MAX_FRAME_BYTES];
        let mut transferred = 0_u32;
        match unsafe {
            ReadFile(
                self.raw(),
                Some(&mut frame),
                Some(&mut transferred),
                Some(&mut overlapped),
            )
        } {
            Ok(()) => {
                unsafe { GetOverlappedResult(self.raw(), &overlapped, &mut transferred, true) }
                    .map_err(|_| helper_pipe_error("the user-helper message could not be read"))?
            }
            Err(error) if error.code() == hresult_from_win32(ERROR_IO_PENDING.0) => {
                match wait_for_pipe_read(self.raw(), &overlapped, remaining)? {
                    PipeReadCompletion::Bytes(bytes) => transferred = bytes,
                    PipeReadCompletion::Closed => return Ok(PipeFrameRead::Closed),
                }
            }
            Err(error) if is_clean_pipe_disconnect(&error) => return Ok(PipeFrameRead::Closed),
            Err(_) => {
                return Err(helper_pipe_error(
                    "the user-helper pipe closed before a terminal message",
                ))
            }
        }

        let transferred = usize::try_from(transferred)
            .map_err(|_| helper_pipe_error("the user-helper message length was invalid"))?;
        Ok(PipeFrameRead::Frame(frame[..transferred].to_vec()))
    }

    fn read_message(&self, remaining: Duration) -> Result<PipeMessageRead, InstallerError> {
        match self.read_frame(remaining)? {
            PipeFrameRead::Frame(frame) => {
                decode_protocol_frame(&frame).map(PipeMessageRead::Message)
            }
            PipeFrameRead::Closed => Ok(PipeMessageRead::Closed),
        }
    }

    fn send_bridge_control(
        &self,
        control: PackageBridgeControl,
        timeout: Duration,
    ) -> Result<(), InstallerError> {
        self.write_control_bytes(&control.encode(), timeout)
    }

    fn write_control_bytes(&self, bytes: &[u8], timeout: Duration) -> Result<(), InstallerError> {
        let event = OwnedEvent::new()?;
        let mut overlapped = OVERLAPPED {
            hEvent: event.raw(),
            ..Default::default()
        };
        let mut transferred = 0_u32;
        match unsafe {
            WriteFile(
                self.raw(),
                Some(bytes),
                Some(&mut transferred),
                Some(&mut overlapped),
            )
        } {
            Ok(()) => {
                unsafe { GetOverlappedResult(self.raw(), &overlapped, &mut transferred, true) }
                    .map_err(|_| {
                        helper_pipe_error("the protected package bridge write was incomplete")
                    })?
            }
            Err(error) if error.code() == hresult_from_win32(ERROR_IO_PENDING.0) => {
                transferred = wait_for_overlapped(self.raw(), &overlapped, timeout)?;
            }
            Err(_) => {
                return Err(helper_pipe_error(
                    "the protected package bridge could not be written",
                ))
            }
        }
        if transferred as usize != bytes.len() {
            return Err(helper_pipe_error(
                "the protected package bridge was not written atomically",
            ));
        }
        Ok(())
    }
}

fn connected_client_token_identity(pipe: HANDLE) -> Result<(String, u32), InstallerError> {
    let raw_pipe = pipe.0 as usize;
    std::thread::Builder::new()
        .name("fyagent-helper-peer-token".to_owned())
        .spawn(move || {
            let pipe = HANDLE(raw_pipe as *mut core::ffi::c_void);
            let impersonation = PipeClientImpersonation::begin(pipe)?;
            let mut thread_token = HANDLE::default();
            unsafe { OpenThreadToken(GetCurrentThread(), TOKEN_QUERY, true, &mut thread_token) }
                .map_err(|_| helper_identity_error())?;
            let thread_token = OwnedWin32Handle::new(thread_token)?;
            let sid = token_user_sid(thread_token.raw())?;
            let session_id = token_session_id(thread_token.raw())?;
            drop(thread_token);
            impersonation.revert()?;
            Ok((sid, session_id))
        })
        .map_err(|_| helper_identity_error())?
        .join()
        .map_err(|_| helper_identity_error())?
}

enum PipeFrameRead {
    Frame(Vec<u8>),
    Closed,
}

enum PipeMessageRead {
    Message(HelperMessage),
    Closed,
}

impl Drop for OneShotPipeServer {
    fn drop(&mut self) {
        unsafe {
            let _ = DisconnectNamedPipe(self.raw());
        }
    }
}

fn consume_protocol(
    server: &OneShotPipeServer,
    sequence: &mut HelperProtocolSequence,
    progress: PlatformProgressSink,
    deadline: Instant,
    terminal_close_timeout: Duration,
) -> Result<HelperProtocolTerminal, InstallerError> {
    let terminal = loop {
        let remaining = remaining_until(deadline)?;
        let message = match server.read_message(remaining)? {
            PipeMessageRead::Message(message) => message,
            PipeMessageRead::Closed => {
                return Err(helper_pipe_error(
                    "the user-helper pipe closed before a terminal message",
                ))
            }
        };
        match accept_protocol_message(sequence, message)? {
            HelperProtocolAction::Hello(_) | HelperProtocolAction::Started(_) => {
                return Err(helper_pipe_error(
                    "the user-helper repeated its handshake after admission",
                ))
            }
            HelperProtocolAction::Progress(completed) => {
                progress.report_progress(JobProgress::new(
                    ProgressPhase::Installation,
                    Some(completed as u64),
                    Some(100),
                ));
            }
            HelperProtocolAction::Success => break HelperProtocolTerminal::Success,
            HelperProtocolAction::ToolResult(result) => {
                break HelperProtocolTerminal::ToolSuccess(result)
            }
            HelperProtocolAction::Failure(code) => break HelperProtocolTerminal::Failure(code),
        }
    };

    wait_for_clean_terminal_close(server, sequence, deadline, terminal_close_timeout)?;
    Ok(terminal)
}

fn wait_for_clean_terminal_close(
    server: &OneShotPipeServer,
    sequence: &mut HelperProtocolSequence,
    deadline: Instant,
    terminal_close_timeout: Duration,
) -> Result<(), InstallerError> {
    let remaining = remaining_until(deadline)?.min(terminal_close_timeout);
    match server.read_message(remaining)? {
        PipeMessageRead::Closed => Ok(()),
        PipeMessageRead::Message(message) => {
            let _ = sequence.accept(message);
            Err(helper_pipe_error(
                "the user-helper sent data after its terminal message",
            ))
        }
    }
}

fn accept_protocol_message(
    sequence: &mut HelperProtocolSequence,
    message: HelperMessage,
) -> Result<HelperProtocolAction, InstallerError> {
    sequence
        .accept(message)
        .map_err(|_| helper_pipe_error("the user-helper message sequence was invalid"))
}

fn retain_package_bridge_after_settlement(
    helper_ok: bool,
    artifact_kind: Option<PackageBridgeArtifactKind>,
) -> bool {
    helper_ok && matches!(artifact_kind, Some(PackageBridgeArtifactKind::Exe))
}

fn finish_settled(
    gate: HelperGateLease,
    mut lifetime: HelperLifetime,
    terminal: HelperProtocolTerminal,
) -> Result<(), InstallerError> {
    lifetime.mark_settled();
    let result = protocol_terminal_result(terminal);
    let retain_vendor_exe = retain_package_bridge_after_settlement(
        result.is_ok(),
        lifetime
            .bridge
            .as_ref()
            .map(ProtectedPackageBridge::artifact_kind),
    );
    if retain_vendor_exe {
        drop(lifetime);
    } else {
        lifetime.cleanup_bridge();
    }
    gate.finish();
    result
}

fn fail_before_admission(
    gate: HelperGateLease,
    lifetime: HelperLifetime,
    error: InstallerError,
) -> Result<(), InstallerError> {
    // Before the BA-owned admission signal, PackageManager cannot have been
    // called. Cancel and close the helper capabilities first, then attempt an
    // exact bridge cleanup. A sharing violation or validation failure leaves
    // the protected operation as an immutable orphan.
    log_helper_failure(&error);
    let _ = lifetime.controls().cancel();
    lifetime.cleanup_bridge();
    gate.finish();
    Err(error)
}

/// Writes one redacted line for a helper request that ended before
/// admission. The DTO details are already redacted diagnostic fields.
fn log_helper_failure(error: &InstallerError) {
    let dto = error.to_dto();
    log::warn!(
        "current-user helper request failed before admission: code={:?} platform_error_code={} message={}",
        dto.code,
        dto.details.platform_error_code.as_deref().unwrap_or("none"),
        dto.details.redacted_message.as_deref().unwrap_or("none"),
    );
}

fn cancel_and_quarantine(
    gate: HelperGateLease,
    lifetime: HelperLifetime,
    original_error: InstallerError,
) -> ! {
    debug_assert!(lifetime.admitted);
    let _ = lifetime.controls().cancel();
    gate.quarantine(lifetime, original_error)
}

fn remaining_until(deadline: Instant) -> Result<Duration, InstallerError> {
    deadline
        .checked_duration_since(Instant::now())
        .ok_or_else(|| helper_pipe_error("the user-helper operation timed out"))
}

fn decode_protocol_frame(frame: &[u8]) -> Result<HelperMessage, InstallerError> {
    decode_frame(frame).map_err(|_| helper_pipe_error("the user-helper message was invalid"))
}

fn bridge_identity_matches(
    helper_identity: PinnedPackageIdentity,
    bridge_identity: PinnedPackageIdentity,
) -> bool {
    helper_identity == bridge_identity
}

fn finish_settled_tool(
    gate: HelperGateLease,
    mut lifetime: HelperLifetime,
    terminal: HelperProtocolTerminal,
) -> Result<ToolOperationResult, InstallerError> {
    lifetime.mark_settled();
    let result = protocol_tool_result(terminal);
    lifetime.cleanup_bridge();
    gate.finish();
    result
}

fn protocol_terminal_result(terminal: HelperProtocolTerminal) -> Result<(), InstallerError> {
    match terminal {
        HelperProtocolTerminal::Success => Ok(()),
        HelperProtocolTerminal::ToolSuccess(_) => Err(helper_pipe_error(
            "the user-helper sent a tool result on a package action",
        )),
        HelperProtocolTerminal::Failure(code) => Err(map_helper_error(code)),
    }
}

fn protocol_tool_result(
    terminal: HelperProtocolTerminal,
) -> Result<ToolOperationResult, InstallerError> {
    match terminal {
        HelperProtocolTerminal::ToolSuccess(result) => Ok(result),
        HelperProtocolTerminal::Success => {
            Err(helper_pipe_error("the user-helper omitted the tool result"))
        }
        HelperProtocolTerminal::Failure(code) => Err(map_helper_error(code)),
    }
}

fn map_helper_error(code: HelperErrorCode) -> InstallerError {
    let installer_code = match code {
        HelperErrorCode::PackageInUse => InstallerErrorCode::WindowsPackageInUse,
        HelperErrorCode::DeploymentBlocked => InstallerErrorCode::WindowsDeploymentBlocked,
        HelperErrorCode::DependencyMissing => InstallerErrorCode::WindowsDependencyMissing,
        HelperErrorCode::SignatureInvalid => InstallerErrorCode::PackageSignatureInvalid,
        HelperErrorCode::PackageInvalid => InstallerErrorCode::PackageParseFailed,
        HelperErrorCode::PackageDowngrade => InstallerErrorCode::MetadataChanged,
        HelperErrorCode::InstallerLaunchFailed => InstallerErrorCode::LaunchFailed,
        HelperErrorCode::InstallerCancelled => {
            return InstallerError::new(InstallerErrorCode::DownloadCancelled)
                .with_platform_error_code("agent_installer_user_cancelled")
        }
        HelperErrorCode::InstallerTimedOut => {
            return InstallerError::new(InstallerErrorCode::WindowsDeploymentFailed)
                .with_platform_error_code("agent_installer_timed_out")
        }
        HelperErrorCode::InstallerProcessUnobservable => {
            return InstallerError::new(InstallerErrorCode::WindowsDeploymentFailed)
                .with_platform_error_code("agent_installer_process_unobservable")
        }
        HelperErrorCode::InstallerExitedNonzero => {
            return InstallerError::new(InstallerErrorCode::WindowsDeploymentFailed)
                .with_platform_error_code("agent_installer_exited_nonzero")
        }
        HelperErrorCode::ToolPermissionDenied => {
            return InstallerError::new(InstallerErrorCode::WindowsDeploymentFailed)
                .with_platform_error_code("tool_permission_denied")
        }
        HelperErrorCode::ToolHostMissing => {
            return InstallerError::new(InstallerErrorCode::WindowsDeploymentFailed)
                .with_platform_error_code("grok_tool_host_missing")
        }
        HelperErrorCode::ToolTimedOut => {
            return InstallerError::new(InstallerErrorCode::WindowsDeploymentFailed)
                .with_platform_error_code("grok_tool_timed_out")
        }
        HelperErrorCode::ToolOutputLimit => {
            return InstallerError::new(InstallerErrorCode::WindowsDeploymentFailed)
                .with_platform_error_code("grok_tool_output_limit")
        }
        HelperErrorCode::ToolOwnerMismatch => {
            return InstallerError::new(InstallerErrorCode::WindowsDeploymentFailed)
                .with_platform_error_code("grok_tool_owner_mismatch")
        }
        HelperErrorCode::ToolNotDetected => {
            return InstallerError::new(InstallerErrorCode::WindowsDeploymentFailed)
                .with_platform_error_code("grok_tool_not_detected")
        }
        HelperErrorCode::ToolExecutionFailed => {
            return InstallerError::new(InstallerErrorCode::WindowsDeploymentFailed)
                .with_platform_error_code("grok_tool_execution_failed")
        }
        HelperErrorCode::InsufficientDiskSpace => {
            return InstallerError::new(InstallerErrorCode::InsufficientDiskSpace)
                .with_platform_error_code("insufficient_disk_space")
        }
        HelperErrorCode::ToolCandidateConflict => {
            return InstallerError::new(InstallerErrorCode::WindowsDeploymentFailed)
                .with_platform_error_code("tool_candidate_conflict")
        }
        HelperErrorCode::ToolTargetChanged => {
            return InstallerError::new(InstallerErrorCode::WindowsDeploymentFailed)
                .with_platform_error_code("tool_target_changed")
        }
        HelperErrorCode::InstallLayoutInvalid
        | HelperErrorCode::WinRtInitializationFailed
        | HelperErrorCode::PackageUriInvalid
        | HelperErrorCode::PackageManagerUnavailable
        | HelperErrorCode::DeploymentFailed
        | HelperErrorCode::DeploymentResultInvalid
        | HelperErrorCode::ParentAdmissionFailed
        | HelperErrorCode::ParentCancelled
        | HelperErrorCode::DeploymentTimedOut => InstallerErrorCode::WindowsDeploymentFailed,
    };
    InstallerError::new(installer_code)
        .with_diagnostic_message("the current-user package helper reported a bounded failure")
}

fn wait_for_overlapped(
    handle: HANDLE,
    overlapped: &OVERLAPPED,
    timeout: Duration,
) -> Result<u32, InstallerError> {
    let milliseconds = timeout.as_millis().min(u32::MAX as u128) as u32;
    let mut transferred = 0_u32;
    match unsafe {
        GetOverlappedResultEx(handle, overlapped, &mut transferred, milliseconds, false)
    } {
        Ok(()) => Ok(transferred),
        Err(_) => {
            unsafe {
                let _ = CancelIoEx(handle, Some(overlapped));
                let _ = GetOverlappedResult(handle, overlapped, &mut transferred, true);
            }
            Err(helper_pipe_error(
                "the user-helper operation timed out or disconnected",
            ))
        }
    }
}

enum PipeReadCompletion {
    Bytes(u32),
    Closed,
}

fn wait_for_pipe_read(
    handle: HANDLE,
    overlapped: &OVERLAPPED,
    timeout: Duration,
) -> Result<PipeReadCompletion, InstallerError> {
    let milliseconds = timeout.as_millis().min(u32::MAX as u128) as u32;
    let mut transferred = 0_u32;
    match unsafe {
        GetOverlappedResultEx(handle, overlapped, &mut transferred, milliseconds, false)
    } {
        Ok(()) => Ok(PipeReadCompletion::Bytes(transferred)),
        Err(error) if is_clean_pipe_disconnect(&error) => Ok(PipeReadCompletion::Closed),
        Err(_) => {
            unsafe {
                let _ = CancelIoEx(handle, Some(overlapped));
                // The OVERLAPPED and buffer are stack-owned, so cancellation
                // must complete before either can be dropped.
                let _ = GetOverlappedResult(handle, overlapped, &mut transferred, true);
            }
            Err(helper_pipe_error(
                "the user-helper operation timed out or disconnected",
            ))
        }
    }
}

fn is_clean_pipe_disconnect(error: &windows::core::Error) -> bool {
    error.code() == hresult_from_win32(ERROR_BROKEN_PIPE.0)
        || error.code() == hresult_from_win32(ERROR_NO_DATA.0)
}

struct PipeSecurityDescriptor(PSECURITY_DESCRIPTOR);

impl PipeSecurityDescriptor {
    fn new(shell_sid: &str) -> Result<Self, InstallerError> {
        // FILE_GENERIC_READ includes FILE_READ_ATTRIBUTES, which named-pipe
        // connect checks even when the client requests only data rights.
        // FILE_WRITE_DATA is granted separately so FILE_CREATE_PIPE_INSTANCE
        // (FILE_GENERIC_WRITE / FILE_APPEND_DATA) stays withheld.
        let sddl = format!("O:BAG:BAD:P(A;;0x0012008b;;;{shell_sid})(A;;RC;;;SY)(A;;RC;;;BA)");
        let sddl = wide_null(&sddl);
        let mut descriptor = PSECURITY_DESCRIPTOR::default();
        unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                PCWSTR(sddl.as_ptr()),
                SDDL_REVISION_1,
                &mut descriptor,
                None,
            )
        }
        .map_err(|_| helper_pipe_error("the user-helper pipe DACL could not be created"))?;
        if descriptor.0.is_null() {
            return Err(helper_pipe_error(
                "the user-helper pipe DACL was unavailable",
            ));
        }
        Ok(Self(descriptor))
    }

    fn as_ptr(&self) -> *mut core::ffi::c_void {
        self.0 .0
    }
}

impl Drop for PipeSecurityDescriptor {
    fn drop(&mut self) {
        unsafe {
            let _ = windows::Win32::Foundation::LocalFree(Some(HLOCAL(self.0 .0)));
        }
    }
}

struct EventSecurityDescriptor(PSECURITY_DESCRIPTOR);

impl EventSecurityDescriptor {
    fn new(shell_sid: &str) -> Result<Self, InstallerError> {
        let sddl = format!("O:BAG:BAD:P(A;;0x00120000;;;{shell_sid})(A;;RC;;;SY)(A;;RC;;;BA)");
        let sddl = wide_null(&sddl);
        let mut descriptor = PSECURITY_DESCRIPTOR::default();
        unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                PCWSTR(sddl.as_ptr()),
                SDDL_REVISION_1,
                &mut descriptor,
                None,
            )
        }
        .map_err(|_| helper_pipe_error("the helper control-event DACL could not be created"))?;
        if descriptor.0.is_null() {
            return Err(helper_pipe_error(
                "the helper control-event DACL was unavailable",
            ));
        }
        Ok(Self(descriptor))
    }

    fn as_ptr(&self) -> *mut core::ffi::c_void {
        self.0 .0
    }
}

impl Drop for EventSecurityDescriptor {
    fn drop(&mut self) {
        unsafe {
            let _ = windows::Win32::Foundation::LocalFree(Some(HLOCAL(self.0 .0)));
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FileIdentity {
    volume_serial_number: u32,
    file_index: u64,
    size: u64,
}

struct PinnedHelperImage {
    _handle: OwnedWin32Handle,
    identity: FileIdentity,
    canonical_path: PathBuf,
}

fn checked_file_identity(
    handle: HANDLE,
    expected_size: u64,
) -> Result<FileIdentity, InstallerError> {
    let mut information = BY_HANDLE_FILE_INFORMATION::default();
    unsafe { GetFileInformationByHandle(handle, &mut information) }
        .map_err(|_| package_pin_error())?;
    if information.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY.0 != 0
        || information.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0
    {
        return Err(package_pin_error());
    }
    let identity = FileIdentity {
        volume_serial_number: information.dwVolumeSerialNumber,
        file_index: (u64::from(information.nFileIndexHigh) << 32)
            | u64::from(information.nFileIndexLow),
        size: (u64::from(information.nFileSizeHigh) << 32) | u64::from(information.nFileSizeLow),
    };
    if identity.size == 0 || identity.size != expected_size {
        return Err(package_pin_error());
    }
    Ok(identity)
}

impl PinnedHelperImage {
    fn open(path: &Path) -> Result<Self, InstallerError> {
        let path = wide_os_null(path.as_os_str());
        let handle = unsafe {
            CreateFileW(
                PCWSTR(path.as_ptr()),
                GENERIC_READ.0,
                FILE_SHARE_READ,
                None,
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL | FILE_FLAG_OPEN_REPARSE_POINT,
                None,
            )
        }
        .map_err(|_| helper_identity_error())?;
        let handle = OwnedWin32Handle::new(handle)?;
        let mut information = BY_HANDLE_FILE_INFORMATION::default();
        unsafe { GetFileInformationByHandle(handle.raw(), &mut information) }
            .map_err(|_| helper_identity_error())?;
        if information.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY.0 != 0
            || information.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0
        {
            return Err(helper_identity_error());
        }
        let identity = FileIdentity {
            volume_serial_number: information.dwVolumeSerialNumber,
            file_index: (u64::from(information.nFileIndexHigh) << 32)
                | u64::from(information.nFileIndexLow),
            size: (u64::from(information.nFileSizeHigh) << 32)
                | u64::from(information.nFileSizeLow),
        };
        if identity.size == 0 {
            return Err(helper_identity_error());
        }
        let canonical_path = final_path_by_handle(handle.raw())?;
        Ok(Self {
            _handle: handle,
            identity,
            canonical_path,
        })
    }

    fn identity(&self) -> &FileIdentity {
        &self.identity
    }

    fn canonical_path(&self) -> &Path {
        &self.canonical_path
    }
}

fn final_path_by_handle(handle: HANDLE) -> Result<PathBuf, InstallerError> {
    let mut buffer = vec![0_u16; 32_768];
    let length = unsafe { GetFinalPathNameByHandleW(handle, &mut buffer, FILE_NAME_NORMALIZED) };
    let length = usize::try_from(length).map_err(|_| helper_identity_error())?;
    if length == 0 || length >= buffer.len() || buffer[..length].contains(&0) {
        return Err(helper_identity_error());
    }
    Ok(PathBuf::from(OsString::from_wide(&buffer[..length])))
}

struct PipeClientImpersonation {
    active: bool,
}

impl PipeClientImpersonation {
    fn begin(pipe: HANDLE) -> Result<Self, InstallerError> {
        unsafe { ImpersonateNamedPipeClient(pipe) }.map_err(|_| helper_identity_error())?;
        Ok(Self { active: true })
    }

    fn revert(mut self) -> Result<(), InstallerError> {
        unsafe { RevertToSelf() }.map_err(|_| helper_identity_error())?;
        self.active = false;
        Ok(())
    }
}

impl Drop for PipeClientImpersonation {
    fn drop(&mut self) {
        if self.active {
            // This guard exists only on the dedicated one-shot identity
            // thread. Even if this best-effort retry fails, exiting that
            // thread releases its impersonation token instead of contaminating
            // a reusable Tauri or Tokio worker.
            unsafe {
                let _ = RevertToSelf();
            }
        }
    }
}

struct OwnedEvent(OwnedWin32Handle);

impl OwnedEvent {
    fn new() -> Result<Self, InstallerError> {
        let handle = unsafe { CreateEventW(None, true, false, PCWSTR::null()) }
            .map_err(|_| helper_pipe_error("the user-helper wait event could not be created"))?;
        Ok(Self(OwnedWin32Handle::new(handle)?))
    }

    fn raw(&self) -> HANDLE {
        self.0.raw()
    }
}

struct OwnedWin32Handle(OwnedHandle);

impl OwnedWin32Handle {
    fn new(handle: HANDLE) -> Result<Self, InstallerError> {
        if handle.is_invalid() {
            Err(helper_identity_error())
        } else {
            Ok(Self(unsafe { OwnedHandle::from_raw_handle(handle.0) }))
        }
    }

    fn raw(&self) -> HANDLE {
        HANDLE(self.0.as_raw_handle())
    }
}

fn process_image_path(process: HANDLE) -> Result<PathBuf, InstallerError> {
    let mut buffer = vec![0_u16; 32_768];
    let mut length = buffer.len() as u32;
    unsafe {
        QueryFullProcessImageNameW(
            process,
            PROCESS_NAME_WIN32,
            PWSTR(buffer.as_mut_ptr()),
            &mut length,
        )
    }
    .map_err(|_| helper_identity_error())?;
    if length == 0 || length as usize > buffer.len() {
        return Err(helper_identity_error());
    }
    Ok(PathBuf::from(OsString::from_wide(
        &buffer[..length as usize],
    )))
}

fn token_session_id(token: HANDLE) -> Result<u32, InstallerError> {
    let mut session_id = 0_u32;
    let mut returned = 0_u32;
    unsafe {
        GetTokenInformation(
            token,
            TokenSessionId,
            Some((&mut session_id as *mut u32).cast()),
            std::mem::size_of::<u32>() as u32,
            &mut returned,
        )
    }
    .map_err(|_| helper_identity_error())?;
    if returned < std::mem::size_of::<u32>() as u32 {
        return Err(helper_identity_error());
    }
    Ok(session_id)
}

fn token_user_sid(token: HANDLE) -> Result<String, InstallerError> {
    let mut required = 0_u32;
    let _ = unsafe { GetTokenInformation(token, TokenUser, None, 0, &mut required) };
    if required == 0 {
        return Err(helper_identity_error());
    }
    let word = std::mem::size_of::<usize>();
    let mut aligned = vec![0_usize; (required as usize).div_ceil(word)];
    unsafe {
        GetTokenInformation(
            token,
            TokenUser,
            Some(aligned.as_mut_ptr().cast()),
            required,
            &mut required,
        )
    }
    .map_err(|_| helper_identity_error())?;
    let token_user = unsafe { &*aligned.as_ptr().cast::<TOKEN_USER>() };
    sid_to_string(token_user.User.Sid)
}

fn sid_to_string(sid: PSID) -> Result<String, InstallerError> {
    let mut string_sid = PWSTR::null();
    unsafe { ConvertSidToStringSidW(sid, &mut string_sid) }.map_err(|_| helper_identity_error())?;
    if string_sid.is_null() {
        return Err(helper_identity_error());
    }
    let rendered = unsafe { PCWSTR(string_sid.0).to_string() }.map_err(|_| helper_identity_error());
    unsafe {
        let _ = windows::Win32::Foundation::LocalFree(Some(HLOCAL(string_sid.0.cast())));
    }
    rendered
}

fn wide_null(value: &str) -> Vec<u16> {
    OsStr::new(value).encode_wide().chain(Some(0)).collect()
}

fn wide_os_null(value: &OsStr) -> Vec<u16> {
    value.encode_wide().chain(Some(0)).collect()
}

const fn hresult_from_win32(value: u32) -> HRESULT {
    HRESULT::from_win32(value)
}

fn helper_launch_error() -> InstallerError {
    InstallerError::new(InstallerErrorCode::WindowsDeploymentFailed)
        .with_diagnostic_message("the fixed current-user package helper could not be launched")
}

/// Platform codes callers use to report an unconfirmed observation instead of
/// an unavailable product. The helper was not started, so nothing ran.
const HELPER_BUSY_PLATFORM_CODE: &str = "helper_busy";
const SHELL_DESKTOP_UNAVAILABLE_PLATFORM_CODE: &str = "shell_desktop_unavailable";
const HELPER_LAUNCH_NOT_INVOKED_PLATFORM_CODE: &str = "helper_launch_not_invoked";

fn helper_not_invoked_error(reason: ProcessLaunchError) -> InstallerError {
    match reason {
        ProcessLaunchError::ShellDesktopUnavailable => helper_launch_error()
            .with_platform_error_code(SHELL_DESKTOP_UNAVAILABLE_PLATFORM_CODE)
            .with_diagnostic_message(
                "Explorer did not expose a desktop shell view to launch the current-user helper",
            ),
        _ => {
            helper_launch_error().with_platform_error_code(HELPER_LAUNCH_NOT_INVOKED_PLATFORM_CODE)
        }
    }
}

fn helper_busy_error() -> InstallerError {
    InstallerError::new(InstallerErrorCode::WindowsDeploymentFailed)
        .with_platform_error_code(HELPER_BUSY_PLATFORM_CODE)
        .with_diagnostic_message(
            "another current-user helper operation did not finish within the bounded wait",
        )
}

fn helper_launch_pending_error() -> InstallerError {
    InstallerError::new(InstallerErrorCode::WindowsDeploymentFailed).with_diagnostic_message(
        "the current-user helper launch remains pending without a safe release proof",
    )
}

fn helper_pipe_error(message: &'static str) -> InstallerError {
    InstallerError::new(InstallerErrorCode::WindowsDeploymentFailed)
        .with_diagnostic_message(message)
}

fn helper_identity_error() -> InstallerError {
    InstallerError::new(InstallerErrorCode::PackageIdentityMismatch)
        .with_diagnostic_message("the current-user package helper identity was rejected")
}

fn package_pin_error() -> InstallerError {
    InstallerError::new(InstallerErrorCode::PackageIdentityMismatch)
        .with_diagnostic_message("the verified Windows package file pin was rejected")
}

fn helper_context_error() -> InstallerError {
    InstallerError::new(InstallerErrorCode::WindowsDeploymentFailed)
        .with_diagnostic_message("the frozen interactive-user context changed before deployment")
}

fn helper_quarantine_error() -> InstallerError {
    InstallerError::new(InstallerErrorCode::WindowsDeploymentFailed).with_diagnostic_message(
        "a prior current-user helper lifetime remains retained without terminal proof",
    )
}

#[cfg(test)]
mod tests {
    use std::os::windows::io::{AsRawHandle, FromRawHandle};

    use windows::Win32::Foundation::GENERIC_WRITE;

    use super::*;

    const BRIDGE_IDENTITY: PinnedPackageIdentity = PinnedPackageIdentity::new(7, 11, 13);

    fn production_source() -> &'static str {
        include_str!("helper.rs")
            .split_once("#[cfg(test)]\nmod tests {")
            .expect("helper production source must precede the test module")
            .0
    }

    fn pinned_runner_source() -> &'static str {
        production_source()
            .split_once("fn run_pinned_user_helper(")
            .expect("the pinned helper runner must exist")
            .1
            .split_once("\nfn generate_nonce(")
            .expect("the pinned helper runner must end before nonce generation")
            .0
    }

    fn started(identity: PinnedPackageIdentity) -> HelperMessage {
        HelperMessage::Started { package: identity }
    }

    #[test]
    fn parent_sequence_requires_hello_control_started_admission_and_terminal() {
        let mut sequence = HelperProtocolSequence::default();
        let action = UserHelperAction::CodexMsixInstall;
        assert!(matches!(
            sequence
                .accept(HelperMessage::Hello { action })
                .unwrap(),
            HelperProtocolAction::Hello(received) if received == action
        ));
        sequence.mark_control_sent().unwrap();
        assert!(matches!(
            sequence.accept(started(BRIDGE_IDENTITY)).unwrap(),
            HelperProtocolAction::Started(BRIDGE_IDENTITY)
        ));
        sequence.mark_admitted().unwrap();
        assert!(matches!(
            sequence
                .accept(HelperMessage::Progress { completed: 0 })
                .unwrap(),
            HelperProtocolAction::Progress(0)
        ));
        assert!(sequence
            .accept(HelperMessage::Progress { completed: 0 })
            .is_err());
        assert!(matches!(
            sequence.accept(HelperMessage::Success).unwrap(),
            HelperProtocolAction::Success
        ));
        assert!(sequence.accept(HelperMessage::Success).is_err());
    }

    #[test]
    fn bridge_identity_mismatch_is_never_admissible() {
        assert!(bridge_identity_matches(BRIDGE_IDENTITY, BRIDGE_IDENTITY));
        assert!(!bridge_identity_matches(
            PinnedPackageIdentity::new(7, 12, 13),
            BRIDGE_IDENTITY
        ));
        assert!(!bridge_identity_matches(
            PinnedPackageIdentity::new(7, 11, 14),
            BRIDGE_IDENTITY
        ));
    }

    #[test]
    fn vendor_exe_success_retains_package_bridge_leaf() {
        assert!(retain_package_bridge_after_settlement(
            true,
            Some(PackageBridgeArtifactKind::Exe),
        ));
        assert!(!retain_package_bridge_after_settlement(
            false,
            Some(PackageBridgeArtifactKind::Exe),
        ));
        assert!(!retain_package_bridge_after_settlement(
            true,
            Some(PackageBridgeArtifactKind::Msix),
        ));
        assert!(!retain_package_bridge_after_settlement(true, None));
        let source = production_source();
        assert!(source.contains("retain_package_bridge_after_settlement("));
        assert!(source.contains("if retain_vendor_exe"));
    }

    #[test]
    fn native_package_downgrade_result_remains_structured() {
        let error = map_helper_error(HelperErrorCode::PackageDowngrade);
        let dto = error.to_dto();
        assert_eq!(dto.code, InstallerErrorCode::MetadataChanged);
        assert_eq!(
            dto.suggested_action,
            crate::codex_desktop::error::SuggestedAction::Refresh
        );
    }

    #[test]
    fn parent_runner_orders_authenticated_hello_before_control_and_admission() {
        let source = pinned_runner_source();
        let raw_read = source.find("read_frame(first_frame_timeout)").unwrap();
        let client_validation = source.find("validate_client(").unwrap();
        let hello_acceptance = source.find("sequence.accept(first_message)").unwrap();
        let control_write = source.find(".send_bridge_control(").unwrap();
        let started_read = source.find("let started_message =").unwrap();
        let identity_check = source.find("bridge_identity_matches(").unwrap();
        let admission_signal = source.find("lifetime.controls().admit()").unwrap();
        let protocol_consumer = source.find("match consume_protocol(").unwrap();
        assert!(raw_read < client_validation);
        assert!(client_validation < hello_acceptance);
        assert!(hello_acceptance < control_write);
        assert!(control_write < started_read);
        assert!(started_read < identity_check);
        assert!(identity_check < admission_signal);
        assert!(admission_signal < protocol_consumer);
        assert!(source.contains("Err(error) => cancel_and_quarantine(gate, lifetime, error)"));
        assert!(!source.contains("drain_after_cancel"));

        for forbidden in [
            ["Parent", "Package", "Source"].concat(),
            ["Win", "sock", "Lease"].concat(),
            ["send_", "source_control"].concat(),
        ] {
            assert!(!source.contains(&forbidden));
        }
    }

    fn platform_code(error: &InstallerError) -> Option<String> {
        error.to_dto().details.platform_error_code
    }

    #[test]
    fn helper_gate_waits_for_an_active_lifetime_to_finish() {
        let gate = Mutex::new(HelperGateState::<()>::Idle);
        let released = Condvar::new();
        enter_helper_gate(&gate, &released, Duration::ZERO).unwrap();
        std::thread::scope(|scope| {
            let waiter =
                scope.spawn(|| enter_helper_gate(&gate, &released, Duration::from_secs(10)));
            std::thread::sleep(Duration::from_millis(100));
            leave_helper_gate(&gate, &released);
            waiter
                .join()
                .unwrap()
                .expect("the queued request must enter after finish");
        });
        assert!(matches!(*gate.lock().unwrap(), HelperGateState::Active));
        leave_helper_gate(&gate, &released);
        assert!(matches!(*gate.lock().unwrap(), HelperGateState::Idle));
    }

    #[test]
    fn helper_gate_wait_is_bounded_and_reports_busy() {
        let gate = Mutex::new(HelperGateState::<()>::Idle);
        let released = Condvar::new();
        enter_helper_gate(&gate, &released, Duration::ZERO).unwrap();
        let started = Instant::now();
        let error = enter_helper_gate(&gate, &released, Duration::from_millis(50)).unwrap_err();
        assert!(started.elapsed() >= Duration::from_millis(50));
        assert_eq!(
            platform_code(&error).as_deref(),
            Some(HELPER_BUSY_PLATFORM_CODE)
        );
        assert!(matches!(*gate.lock().unwrap(), HelperGateState::Active));
    }

    #[test]
    fn quarantined_helper_gate_still_fails_immediately() {
        let gate = Mutex::new(HelperGateState::Quarantined {
            _lifetime: Box::new(()),
        });
        let released = Condvar::new();
        let started = Instant::now();
        let error = enter_helper_gate(&gate, &released, Duration::from_secs(10)).unwrap_err();
        assert!(started.elapsed() < Duration::from_secs(1));
        assert_ne!(
            platform_code(&error).as_deref(),
            Some(HELPER_BUSY_PLATFORM_CODE)
        );
        assert_eq!(
            error.to_dto().code,
            InstallerErrorCode::WindowsDeploymentFailed
        );
        leave_helper_gate(&gate, &released);
        assert!(matches!(
            *gate.lock().unwrap(),
            HelperGateState::Quarantined { .. }
        ));
    }

    #[test]
    fn helper_launch_failures_carry_distinct_platform_codes() {
        assert_eq!(
            platform_code(&helper_not_invoked_error(
                ProcessLaunchError::ShellDesktopUnavailable
            ))
            .as_deref(),
            Some(SHELL_DESKTOP_UNAVAILABLE_PLATFORM_CODE)
        );
        assert_eq!(
            platform_code(&helper_not_invoked_error(
                ProcessLaunchError::InteractiveUserUnavailable
            ))
            .as_deref(),
            Some(HELPER_LAUNCH_NOT_INVOKED_PLATFORM_CODE)
        );
        assert_eq!(platform_code(&helper_launch_pending_error()), None);
        let source = production_source();
        assert!(source.contains("log_helper_failure(&error);"));
        assert!(source.contains(".inspect_err(log_helper_failure)"));
    }

    #[test]
    fn pipe_security_contract_is_local_first_instance_message_mode_and_minimal() {
        let source = production_source();
        assert!(source.contains("FILE_FLAG_FIRST_PIPE_INSTANCE"));
        assert!(source.contains("PIPE_TYPE_MESSAGE"));
        assert!(source.contains("PIPE_READMODE_MESSAGE"));
        assert!(source.contains("PIPE_REJECT_REMOTE_CLIENTS"));
        assert!(source.contains("PIPE_ACCESS_DUPLEX"));
        assert!(source.contains("BRIDGE_CONTROL_BYTES as u32"));
        assert!(source.contains("O:BAG:BAD:P(A;;0x0012008b;;;{shell_sid})(A;;RC;;;SY)(A;;RC;;;BA)"));
        assert!(source.contains("GetNamedPipeClientProcessId"));
        assert!(source.contains("ImpersonateNamedPipeClient"));
        assert!(source.contains("OpenThreadToken"));
        assert!(!source.contains("OpenProcessToken"));
        assert!(source.contains("QueryFullProcessImageNameW"));
    }

    fn between<'a>(source: &'a str, start: &str, end: &str) -> &'a str {
        source
            .split_once(start)
            .unwrap_or_else(|| panic!("missing start marker {start}"))
            .1
            .split_once(end)
            .unwrap_or_else(|| panic!("missing end marker {end}"))
            .0
    }

    fn unpinned_runner_source() -> &'static str {
        between(
            production_source(),
            "fn run_unpinned_tool_helper(",
            "\nimpl WindowsVerifiedFilePin",
        )
    }

    /// Protocol-state classification only. These inputs are illegal frames, not
    /// a closed pipe and not an exited helper process. The gate release for a
    /// real pre-admission close is
    /// `peer_half_close_before_admission_releases_the_next_request`.
    #[test]
    fn protocol_sequence_rejects_frames_before_hello_or_control() {
        let action = UserHelperAction::CodexMsixInstall;
        for message in [
            started(BRIDGE_IDENTITY),
            HelperMessage::Progress { completed: 1 },
            HelperMessage::Success,
            HelperMessage::error(HelperErrorCode::ParentCancelled),
        ] {
            let mut sequence = HelperProtocolSequence::default();
            assert!(sequence.accept(message).is_err());
            assert!(sequence.mark_control_sent().is_err());
            assert!(sequence.mark_admitted().is_err());
            assert!(sequence.terminal().is_none());
        }

        let mut after_hello = HelperProtocolSequence::default();
        assert!(matches!(
            after_hello
                .accept(HelperMessage::Hello { action })
                .unwrap(),
            HelperProtocolAction::Hello(received) if received == action
        ));
        assert!(after_hello.accept(started(BRIDGE_IDENTITY)).is_err());
        // A rejected frame must not skip the control step.
        after_hello.mark_control_sent().unwrap();
        assert!(after_hello.mark_admitted().is_err());
        assert!(after_hello.terminal().is_none());

        let mut control_without_hello = HelperProtocolSequence::default();
        assert!(control_without_hello.mark_control_sent().is_err());
        assert!(control_without_hello.terminal().is_none());
    }

    #[test]
    fn broken_pipe_and_no_data_are_clean_disconnects() {
        let broken = windows::core::Error::from_hresult(hresult_from_win32(ERROR_BROKEN_PIPE.0));
        let no_data = windows::core::Error::from_hresult(hresult_from_win32(ERROR_NO_DATA.0));
        let pending = windows::core::Error::from_hresult(hresult_from_win32(ERROR_IO_PENDING.0));
        let already_exists =
            windows::core::Error::from_hresult(hresult_from_win32(ERROR_ALREADY_EXISTS.0));
        let pipe_connected =
            windows::core::Error::from_hresult(hresult_from_win32(ERROR_PIPE_CONNECTED.0));
        assert!(is_clean_pipe_disconnect(&broken));
        assert!(is_clean_pipe_disconnect(&no_data));
        assert!(!is_clean_pipe_disconnect(&pending));
        assert!(!is_clean_pipe_disconnect(&already_exists));
        assert!(!is_clean_pipe_disconnect(&pipe_connected));
    }

    #[test]
    fn pipe_and_quarantine_errors_are_deployment_failures_without_platform_code() {
        for message in [
            "the user-helper operation timed out or disconnected",
            "the user-helper pipe closed before a terminal message",
            "the user-helper pipe closed before its identity was admitted",
            "the user-helper pipe closed before tool admission",
            "the user-helper pipe closed before bridge admission",
        ] {
            let error = helper_pipe_error(message);
            let dto = error.to_dto();
            assert_eq!(dto.code, InstallerErrorCode::WindowsDeploymentFailed);
            assert_eq!(dto.details.platform_error_code, None);
            assert_eq!(dto.details.redacted_message.as_deref(), Some(message));
            assert_ne!(
                platform_code(&error).as_deref(),
                Some(HELPER_BUSY_PLATFORM_CODE)
            );
        }

        let quarantine = helper_quarantine_error();
        let dto = quarantine.to_dto();
        assert_eq!(dto.code, InstallerErrorCode::WindowsDeploymentFailed);
        assert_eq!(platform_code(&quarantine), None);
        assert_eq!(
            dto.details.redacted_message.as_deref(),
            Some("a prior current-user helper lifetime remains retained without terminal proof")
        );
        assert_ne!(
            dto.details.redacted_message.as_deref(),
            Some("the user-helper operation timed out or disconnected")
        );
    }

    /// Re-entry after `leave_helper_gate` on a local mutex. This is a normal
    /// release, not a disconnected peer and not the process-global helper gate.
    #[test]
    fn helper_gate_accepts_the_next_request_after_normal_finish() {
        let gate = Mutex::new(HelperGateState::<()>::Idle);
        let released = Condvar::new();
        enter_helper_gate(&gate, &released, Duration::ZERO).unwrap();
        assert!(matches!(*gate.lock().unwrap(), HelperGateState::Active));
        leave_helper_gate(&gate, &released);
        assert!(matches!(*gate.lock().unwrap(), HelperGateState::Idle));

        enter_helper_gate(&gate, &released, Duration::ZERO).unwrap();
        assert!(matches!(*gate.lock().unwrap(), HelperGateState::Active));
        leave_helper_gate(&gate, &released);
        assert!(matches!(*gate.lock().unwrap(), HelperGateState::Idle));
    }

    /// The gate is constructed already `Quarantined`. This checks that state,
    /// not a crash or a closed pipe moving the gate there. The transition is
    /// `peer_half_close_after_admission_quarantines_the_next_request`.
    #[test]
    fn constructed_quarantine_rejects_the_next_request_immediately() {
        let gate = Mutex::new(HelperGateState::Quarantined {
            _lifetime: Box::new(()),
        });
        let released = Condvar::new();
        let expected = helper_quarantine_error().to_dto();
        for _ in 0..2 {
            let started = Instant::now();
            let error = enter_helper_gate(&gate, &released, Duration::from_secs(30)).unwrap_err();
            assert!(started.elapsed() < Duration::from_secs(1));
            assert_eq!(error.to_dto(), expected);
            assert_eq!(platform_code(&error), None);
            assert_ne!(
                platform_code(&error).as_deref(),
                Some(HELPER_BUSY_PLATFORM_CODE)
            );
        }
        leave_helper_gate(&gate, &released);
        assert!(matches!(
            *gate.lock().unwrap(),
            HelperGateState::Quarantined { .. }
        ));
        let error = enter_helper_gate(&gate, &released, Duration::from_secs(30)).unwrap_err();
        assert_eq!(error.to_dto(), expected);
    }

    /// Source-contract evidence, not an executed disconnect. `production_source`
    /// drops everything after `#[cfg(test)]`, then each assertion is limited to
    /// one function body. Whitespace, newlines, and function order are part of
    /// the contract. A real half-frame close is covered by the Windows pipe
    /// fixtures below. `cancel_and_quarantine` parks forever, so the
    /// post-admission fixture runs it in a child process.
    #[test]
    fn source_contract_pre_admission_close_releases_and_post_admission_close_quarantines() {
        let source = production_source();
        let read_frame = between(source, "fn read_frame(", "fn read_message(");
        assert!(read_frame
            .contains("is_clean_pipe_disconnect(&error) => return Ok(PipeFrameRead::Closed)"));
        assert!(read_frame.contains("the user-helper pipe closed before a terminal message"));

        let wait_read = between(
            source,
            "fn wait_for_pipe_read(",
            "fn is_clean_pipe_disconnect(",
        );
        assert!(wait_read
            .contains("is_clean_pipe_disconnect(&error) => Ok(PipeReadCompletion::Closed)"));
        assert!(wait_read.contains("the user-helper operation timed out or disconnected"));

        // Writes do not treat BROKEN_PIPE / NO_DATA as a clean close. Any
        // overlapped write failure becomes the same timeout/disconnect error.
        let wait_write = between(source, "fn wait_for_overlapped(", "enum PipeReadCompletion");
        assert!(!wait_write.contains("is_clean_pipe_disconnect"));
        assert!(wait_write.contains("the user-helper operation timed out or disconnected"));

        let consume = between(
            source,
            "fn consume_protocol(",
            "fn wait_for_clean_terminal_close(",
        );
        assert!(consume.contains("PipeMessageRead::Closed"));
        assert!(consume.contains("the user-helper pipe closed before a terminal message"));

        let release = between(
            source,
            "fn fail_before_admission(",
            "fn log_helper_failure(",
        );
        assert!(release.contains("gate.finish()"));
        assert!(!release.contains("quarantine"));
        assert!(!release.contains("retain_quarantined_lifetime"));

        let cancel = between(source, "fn cancel_and_quarantine(", "fn remaining_until(");
        assert!(cancel.contains("debug_assert!(lifetime.admitted)"));
        assert!(cancel.contains("gate.quarantine(lifetime, original_error)"));
        assert!(!cancel.contains("gate.finish()"));
        assert!(!cancel.contains("mark_settled"));

        let retain = between(
            source,
            "fn retain_quarantined_lifetime(",
            "struct HelperGateLease",
        );
        assert!(retain.contains("matches!(*state, HelperGateState::Active)"));
        assert!(retain.contains("HelperGateState::Quarantined"));
        assert!(retain.contains("Box::leak(Box::new(lifetime))"));
        assert!(retain.contains("HELPER_GATE_RELEASED.notify_all()"));

        let quarantine = between(source, "fn quarantine(", "impl Drop for HelperGateLease");
        assert!(quarantine.contains("retain_quarantined_lifetime(lifetime)"));
        assert!(quarantine.contains("self.active = false"));
        assert!(quarantine.contains("std::thread::park()"));
        assert!(!quarantine.contains("leave_helper_gate"));

        let lease_drop = between(
            source,
            "impl Drop for HelperGateLease",
            "struct OneShotPipeServer",
        );
        assert!(lease_drop.contains("if !self.active"));
        assert!(lease_drop.contains("leave_helper_gate"));

        let lifetime_drop = between(source, "impl Drop for HelperLifetime", "static HELPER_GATE");
        assert!(lifetime_drop.contains("if self.admitted && !self.settled"));
        assert!(lifetime_drop.contains("retain_quarantined_lifetime("));
        assert!(lifetime_drop.contains("std::thread::park()"));

        for runner in [unpinned_runner_source(), pinned_runner_source()] {
            let admitted = runner
                .find("lifetime.mark_admitted()")
                .expect("admission marker");
            let before_admission = &runner[..admitted];
            let after_admission = &runner[admitted..];
            let hello = before_admission
                .find("sequence.accept(first_message)")
                .expect("hello marker");
            let first_close = before_admission
                .find("Ok(PipeFrameRead::Closed)")
                .expect("pre-hello close");
            assert!(first_close < hello);
            assert!(before_admission
                .contains("the user-helper pipe closed before its identity was admitted"));
            assert!(before_admission.contains("fail_before_admission"));
            assert!(!before_admission.contains("cancel_and_quarantine"));
            assert!(!before_admission.contains("retain_quarantined_lifetime"));
            assert!(after_admission
                .contains("Err(error) => cancel_and_quarantine(gate, lifetime, error)"));
            assert!(!after_admission.contains("fail_before_admission"));
        }
        assert!(
            unpinned_runner_source().contains("the user-helper pipe closed before tool admission")
        );
        assert!(
            pinned_runner_source().contains("the user-helper pipe closed before bridge admission")
        );
    }

    fn post_admission_pipe_test_name() -> String {
        // Libtest filters omit the crate name. Same split as
        // `identity_process_worker` in session_manager::migrate::identity.
        let test_module = module_path!()
            .split_once("::")
            .expect("crate-qualified module path")
            .1;
        format!("{test_module}::peer_half_close_after_admission_quarantines_the_next_request")
    }
    const PIPE_CLOSED_MARK: &str = "HELPER_PIPE_CLOSED_ERROR";
    const QUARANTINE_OBSERVED_MARK: &str = "HELPER_QUARANTINE_OBSERVED";
    const QUARANTINE_TIMEOUT_MARK: &str = "HELPER_QUARANTINE_TIMEOUT";

    fn gate_mutex() -> &'static std::sync::Mutex<HelperGateState> {
        HELPER_GATE.get_or_init(|| std::sync::Mutex::new(HelperGateState::Idle))
    }

    struct FinishGateOnDrop(Option<HelperGateLease>);

    impl Drop for FinishGateOnDrop {
        fn drop(&mut self) {
            if let Some(gate) = self.0.take() {
                gate.finish();
            }
        }
    }

    struct ReleaseGlobalGate;

    impl Drop for ReleaseGlobalGate {
        fn drop(&mut self) {
            leave_helper_gate(gate_mutex(), &HELPER_GATE_RELEASED);
        }
    }

    fn half_progress_frame() -> Vec<u8> {
        let full = fyagent_user_helper::encode_frame(&HelperMessage::Progress { completed: 40 })
            .expect("progress frame");
        let split_at = full.len() / 2;
        assert!(split_at > 0 && split_at < full.len());
        full[..split_at].to_vec()
    }

    fn next_pipe_name() -> String {
        use std::sync::atomic::{AtomicU64, Ordering};

        static PIPE_SEQ: AtomicU64 = AtomicU64::new(0);
        let seq = PIPE_SEQ.fetch_add(1, Ordering::Relaxed);
        format!(
            r"\\.\pipe\fyagent-helper-crash-{}-{seq}",
            std::process::id()
        )
    }

    /// Default-DACL overlapped message pipe. `OneShotPipeServer::create` pins
    /// owner BA and grants the pipe only to a shell SID, so a test client in
    /// this process cannot connect through that constructor without changing
    /// product code. The read and protocol methods below are the production ones.
    fn open_connected_message_pipe() -> (OneShotPipeServer, OwnedHandle) {
        let name = next_pipe_name();
        let wide = wide_null(&name);
        let server_handle = unsafe {
            CreateNamedPipeW(
                PCWSTR(wide.as_ptr()),
                PIPE_ACCESS_DUPLEX | FILE_FLAG_FIRST_PIPE_INSTANCE | FILE_FLAG_OVERLAPPED,
                PIPE_TYPE_MESSAGE | PIPE_READMODE_MESSAGE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
                1,
                4_096,
                4_096,
                1_000,
                None,
            )
        };
        if server_handle.is_invalid() {
            panic!("CreateNamedPipeW failed: {:?}", unsafe { GetLastError() });
        }
        let server = OneShotPipeServer {
            handle: unsafe { OwnedHandle::from_raw_handle(server_handle.0) },
        };
        let client_handle = unsafe {
            CreateFileW(
                PCWSTR(wide.as_ptr()),
                GENERIC_READ.0 | GENERIC_WRITE.0,
                FILE_SHARE_READ,
                None,
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                None,
            )
        }
        .unwrap_or_else(|error| panic!("CreateFileW client failed: {error}"));
        if client_handle.is_invalid() {
            panic!("CreateFileW returned an invalid handle");
        }
        // Own the client before connect so a connect failure cannot leak the raw handle.
        let client = unsafe { OwnedHandle::from_raw_handle(client_handle.0) };
        server
            .connect(Duration::from_secs(2))
            .expect("ConnectNamedPipe");
        (server, client)
    }

    fn write_all_and_close(client: OwnedHandle, bytes: &[u8]) {
        let mut transferred = 0_u32;
        unsafe {
            WriteFile(
                HANDLE(client.as_raw_handle()),
                Some(bytes),
                Some(&mut transferred),
                None,
            )
        }
        .unwrap_or_else(|error| panic!("WriteFile failed: {error}"));
        assert_eq!(transferred as usize, bytes.len());
        drop(client);
    }

    fn manual_event() -> ParentControlEvent {
        let handle =
            unsafe { CreateEventW(None, true, false, PCWSTR::null()) }.expect("test control event");
        ParentControlEvent(OwnedWin32Handle::new(handle).expect("owned control event"))
    }

    fn lifetime_holding(server: OneShotPipeServer) -> HelperLifetime {
        HelperLifetime {
            pin: None,
            bridge: None,
            helper_image: None,
            controls: Some(ParentControlEvents {
                admission: manual_event(),
                cancel: manual_event(),
            }),
            server: Some(server),
            process: None,
            admitted: false,
            settled: false,
        }
    }

    fn emit_mark(line: &str) {
        let path = std::env::var_os("FYAGENT_HELPER_PIPE_RESULT").expect("result path");
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .expect("open result file");
        std::io::Write::write_all(&mut file, line.as_bytes()).expect("write result");
        std::io::Write::write_all(&mut file, b"\n").expect("write result newline");
        file.sync_all().expect("sync result file");
    }

    /// Windows CI only. This module is already compiled only for Windows.
    ///
    /// The helper runners launch the real helper and require its image, session,
    /// and token, so a test peer cannot drive `run_unpinned_tool_helper` or
    /// `run_pinned_user_helper` without a product seam. Message mode makes the
    /// short write one whole message: production `read_frame` returns those
    /// bytes, and the next `read_frame` is the peer close. The runner's
    /// pre-admission `Closed` arm then calls `fail_before_admission`; this test
    /// does that and checks the next request can enter.
    #[test]
    fn peer_half_close_before_admission_releases_the_next_request() {
        let half = half_progress_frame();
        let (server, client) = open_connected_message_pipe();
        write_all_and_close(client, &half);
        let mut held = FinishGateOnDrop(Some(
            HelperGateLease::acquire().expect("acquire helper gate"),
        ));
        let first = server
            .read_frame(Duration::from_secs(2))
            .expect("read partial frame");
        let PipeFrameRead::Frame(frame) = first else {
            panic!("partial write must arrive as its own message before the close");
        };
        assert_eq!(frame, half);
        let second = server
            .read_frame(Duration::from_secs(2))
            .expect("read close");
        assert!(matches!(second, PipeFrameRead::Closed));
        let gate = held.0.take().expect("gate still held");
        let error = fail_before_admission(
            gate,
            lifetime_holding(server),
            helper_pipe_error("the user-helper pipe closed before its identity was admitted"),
        )
        .expect_err("pre-admission close returns the pipe error");
        let dto = error.to_dto();
        assert_eq!(dto.code, InstallerErrorCode::WindowsDeploymentFailed);
        assert_eq!(dto.details.platform_error_code, None);
        assert_eq!(
            dto.details.redacted_message.as_deref(),
            Some("the user-helper pipe closed before its identity was admitted")
        );

        // `enter_helper_gate` sets Active. Drop releases it if an assertion fails.
        let _release = ReleaseGlobalGate;
        let started = Instant::now();
        enter_helper_gate(
            gate_mutex(),
            &HELPER_GATE_RELEASED,
            Duration::from_millis(500),
        )
        .expect("next request enters after pre-admission close");
        assert!(started.elapsed() < Duration::from_millis(500));
        assert!(matches!(
            *gate_mutex().lock().expect("gate lock"),
            HelperGateState::Active
        ));
        leave_helper_gate(gate_mutex(), &HELPER_GATE_RELEASED);
        assert!(matches!(
            *gate_mutex().lock().expect("gate lock"),
            HelperGateState::Idle
        ));
    }

    struct DeleteFile(std::path::PathBuf);

    impl Drop for DeleteFile {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    struct KillOnDrop(Option<std::process::Child>);

    impl KillOnDrop {
        fn spawn(mut command: std::process::Command) -> Self {
            Self(Some(
                command.spawn().expect("spawn quarantine child process"),
            ))
        }

        /// `TerminateProcess` skips destructors. Poll instead of `wait`: a
        /// child that does not die must not park this test, and dropping
        /// `Child` does not wait either.
        fn finish(&mut self) -> bool {
            let Some(mut child) = self.0.take() else {
                return true;
            };
            let _ = child.kill();
            let deadline = Instant::now() + Duration::from_secs(5);
            loop {
                match child.try_wait() {
                    Ok(Some(_)) => return true,
                    Err(_) => return false,
                    Ok(None) if Instant::now() >= deadline => return false,
                    Ok(None) => std::thread::sleep(Duration::from_millis(20)),
                }
            }
        }
    }

    impl Drop for KillOnDrop {
        fn drop(&mut self) {
            self.finish();
        }
    }

    fn complete_mark<'a>(text: &'a str, mark: &str) -> Option<&'a str> {
        text.split_inclusive('\n').find_map(|chunk| {
            let line = chunk.strip_suffix('\n')?;
            line.contains(mark).then_some(line)
        })
    }

    /// `cancel_and_quarantine` and `HelperLifetime`'s admitted drop both park
    /// forever. Run that path in a child and kill it. `TerminateProcess` does
    /// not run destructors, so the child's parked thread and admitted drop do
    /// not hang this process. The parent waits at most 20 seconds. Windows CI
    /// only: a test peer still cannot pass `validate_client`, so this starts at
    /// the production read and `consume_protocol` the runner calls after admission.
    #[test]
    fn peer_half_close_after_admission_quarantines_the_next_request() {
        if std::env::var("FYAGENT_HELPER_PIPE_CHILD").ok().as_deref() == Some("quarantine") {
            post_admission_close_quarantines_until_killed();
        }

        let result_path = std::env::temp_dir().join(format!(
            "fyagent-helper-quarantine-{}-{}.txt",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        let _delete_result = DeleteFile(result_path.clone());
        let exe = std::env::current_exe().expect("test executable");
        let mut command = std::process::Command::new(exe);
        command
            .arg("--exact")
            .arg(post_admission_pipe_test_name())
            .arg("--nocapture")
            .arg("--test-threads=1")
            .env("FYAGENT_HELPER_PIPE_CHILD", "quarantine")
            .env("FYAGENT_HELPER_PIPE_RESULT", &result_path)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped());
        let mut child = KillOnDrop::spawn(command);
        let stderr = {
            let process = child.0.as_mut().expect("child");
            process.stderr.take().expect("child stderr")
        };
        let mut stderr_reader = Some(std::thread::spawn(move || {
            let mut stderr = stderr;
            let mut buf = String::new();
            let _ = std::io::Read::read_to_string(&mut stderr, &mut buf);
            buf
        }));
        let join_stderr = |reader: Option<std::thread::JoinHandle<String>>, exited: bool| {
            if exited {
                reader
                    .map(|handle| handle.join().unwrap_or_default())
                    .unwrap_or_default()
            } else {
                String::new()
            }
        };

        let deadline = Instant::now() + Duration::from_secs(20);
        let report = loop {
            let text = std::fs::read_to_string(&result_path).unwrap_or_default();
            let observed = complete_mark(&text, QUARANTINE_OBSERVED_MARK).is_some();
            let timed_out = complete_mark(&text, QUARANTINE_TIMEOUT_MARK).is_some();
            if observed || timed_out {
                break text;
            }
            if Instant::now() >= deadline {
                let exited = child.finish();
                let stderr = join_stderr(stderr_reader.take(), exited);
                panic!("quarantine child timed out\nresult:\n{text}\nstderr:\n{stderr}");
            }
            std::thread::sleep(Duration::from_millis(20));
        };
        let exited = child.finish();
        let stderr = join_stderr(stderr_reader.take(), exited);
        let report = format!("{report}\n{stderr}");
        let observed = report
            .lines()
            .find(|line| line.contains(QUARANTINE_OBSERVED_MARK))
            .unwrap_or_else(|| panic!("child did not observe quarantine\n{report}"));
        let closed = report
            .lines()
            .find(|line| line.contains(PIPE_CLOSED_MARK))
            .unwrap_or_else(|| panic!("child did not see the pipe close\n{report}"));
        assert!(
            closed.contains("the user-helper pipe closed before a terminal message"),
            "{closed}"
        );
        let elapsed_ms: u128 = observed
            .split('\t')
            .find_map(|part| part.strip_prefix("elapsed_ms="))
            .unwrap_or_else(|| panic!("missing elapsed_ms in {observed}"))
            .parse()
            .expect("elapsed_ms");
        assert!(elapsed_ms < 1_000, "next request waited {elapsed_ms}ms");
        assert!(
            observed.contains("code=WindowsDeploymentFailed"),
            "{observed}"
        );
        assert!(observed.contains("platform=none"), "{observed}");
        assert!(
            observed.contains(concat!(
                "message=a prior current-user helper lifetime ",
                "remains retained without terminal proof"
            )),
            "{observed}"
        );
    }

    fn post_admission_close_quarantines_until_killed() -> ! {
        let half = half_progress_frame();
        let (server, client) = open_connected_message_pipe();
        write_all_and_close(client, &half);
        // The short write is one message-mode message, not a torn byte-stream
        // read. Take it with production `read_frame`, then let production
        // `consume_protocol` observe the close that follows. Feeding the short
        // message to `consume_protocol` instead would fail decode with
        // "the user-helper message was invalid" and still quarantine.
        let first = server
            .read_frame(Duration::from_secs(2))
            .expect("read partial frame");
        let PipeFrameRead::Frame(frame) = first else {
            panic!("partial write must arrive as its own message before the close");
        };
        assert_eq!(frame, half);
        let closed = consume_protocol(
            &server,
            &mut HelperProtocolSequence::default(),
            std::sync::Arc::new(|_: crate::codex_desktop::types::JobProgress| {}),
            Instant::now() + Duration::from_secs(2),
            Duration::from_secs(1),
        )
        .expect_err("close before a terminal message");
        let closed_message = closed.to_dto().details.redacted_message.unwrap_or_default();
        assert_eq!(
            closed_message,
            "the user-helper pipe closed before a terminal message"
        );
        emit_mark(&format!("{PIPE_CLOSED_MARK}\t{closed_message}"));

        let _observer = std::thread::spawn(|| {
            let deadline = Instant::now() + Duration::from_secs(10);
            loop {
                if Instant::now() > deadline {
                    emit_mark(QUARANTINE_TIMEOUT_MARK);
                    return;
                }
                let quarantined = gate_mutex()
                    .lock()
                    .map(|state| matches!(*state, HelperGateState::Quarantined { .. }))
                    .unwrap_or(false);
                if !quarantined {
                    std::thread::sleep(Duration::from_millis(20));
                    continue;
                }
                let started = Instant::now();
                let error =
                    enter_helper_gate(gate_mutex(), &HELPER_GATE_RELEASED, Duration::from_secs(5))
                        .expect_err("quarantine rejects the next request");
                let dto = error.to_dto();
                emit_mark(&format!(
                    "{QUARANTINE_OBSERVED_MARK}\telapsed_ms={}\tcode={:?}\tplatform={}\tmessage={}",
                    started.elapsed().as_millis(),
                    dto.code,
                    dto.details.platform_error_code.as_deref().unwrap_or("none"),
                    dto.details.redacted_message.as_deref().unwrap_or("none"),
                ));
                return;
            }
        });

        let gate = HelperGateLease::acquire().expect("acquire helper gate");
        let mut lifetime = lifetime_holding(server);
        lifetime.mark_admitted();
        cancel_and_quarantine(gate, lifetime, closed);
    }
}
