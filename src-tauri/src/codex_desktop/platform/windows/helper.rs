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
    io::{Seek, SeekFrom},
    os::windows::{
        ffi::{OsStrExt, OsStringExt},
        io::{AsRawHandle, FromRawHandle, OwnedHandle},
    },
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};

use fyagent_user_helper::{
    admission_event_name, cancel_event_name, decode_frame, layout::pipe_name, CanonicalJobId,
    HelperErrorCode, HelperMessage, HelperProtocolAction, HelperProtocolSequence,
    HelperProtocolTerminal, PackageBridgeControl, PinnedPackageIdentity, PipeNonce,
    BRIDGE_CONTROL_BYTES, MAX_FRAME_BYTES,
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
        error::{InstallerError, InstallerErrorCode},
        types::{JobProgress, ProgressPhase},
        verify::verify_reader,
    },
    platform::process_launch::{
        fixed_user_helper_path, launch_fyagent_user_helper_as_user, UserHelperLaunchOutcome,
    },
    windows_runtime::InteractiveUserContext,
};

const PIPE_DEFAULT_TIMEOUT_MS: u32 = 30_000;

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
        run_pinned_user_helper(context, &job_id, pin, progress, deadlines)
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
        let mut file = package.open_artifact_for_pinning()?;
        let identity = checked_file_identity(HANDLE(file.as_raw_handle()), package.actual_size())?;
        verify_reader(&mut file, package.actual_size(), package.local_sha256())?;
        if checked_file_identity(HANDLE(file.as_raw_handle()), identity.size)? != identity {
            return Err(package_pin_error());
        }
        Ok(Self {
            file: Mutex::new(file),
            identity,
            expected_size: package.actual_size(),
            expected_sha256: package.local_sha256().to_owned(),
        })
    }
}

impl WindowsVerifiedFilePin for VerifiedFilePin {
    fn recheck(&self) -> Result<(), InstallerError> {
        let mut file = self.file.lock().map_err(|_| package_pin_error())?;
        if checked_file_identity(HANDLE(file.as_raw_handle()), self.expected_size)? != self.identity
        {
            return Err(package_pin_error());
        }
        file.seek(SeekFrom::Start(0))
            .map_err(|_| package_pin_error())?;
        verify_reader(&mut *file, self.expected_size, &self.expected_sha256)?;
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

    match launch_fyagent_user_helper_as_user(job_id, &nonce) {
        UserHelperLaunchOutcome::Confirmed => {}
        UserHelperLaunchOutcome::MayHaveLaunched => {
            return fail_before_admission(gate, lifetime, helper_launch_pending_error());
        }
        UserHelperLaunchOutcome::NotInvoked(_) => {
            return fail_before_admission(gate, lifetime, helper_launch_error());
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
        Ok(HelperProtocolAction::Hello) => {}
        _ => {
            return fail_before_admission(
                gate,
                lifetime,
                helper_pipe_error("the user-helper did not begin with Hello"),
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

enum HelperGateState {
    Idle,
    Active,
    Quarantined { _lifetime: Box<HelperLifetime> },
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
}

struct HelperGateLease {
    active: bool,
}

impl HelperGateLease {
    fn acquire() -> Result<Self, InstallerError> {
        let mut state = HELPER_GATE
            .get_or_init(|| Mutex::new(HelperGateState::Idle))
            .lock()
            .map_err(|_| helper_quarantine_error())?;
        match &*state {
            HelperGateState::Idle => {
                *state = HelperGateState::Active;
                Ok(Self { active: true })
            }
            HelperGateState::Active | HelperGateState::Quarantined { .. } => {
                Err(helper_quarantine_error())
            }
        }
    }

    fn finish(mut self) {
        if let Ok(mut state) = HELPER_GATE
            .get_or_init(|| Mutex::new(HelperGateState::Idle))
            .lock()
        {
            if matches!(*state, HelperGateState::Active) {
                *state = HelperGateState::Idle;
            }
        }
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
        if let Ok(mut state) = HELPER_GATE
            .get_or_init(|| Mutex::new(HelperGateState::Idle))
            .lock()
        {
            if matches!(*state, HelperGateState::Active) {
                *state = HelperGateState::Idle;
            }
        }
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
        let bytes = control.encode();
        let event = OwnedEvent::new()?;
        let mut overlapped = OVERLAPPED {
            hEvent: event.raw(),
            ..Default::default()
        };
        let mut transferred = 0_u32;
        match unsafe {
            WriteFile(
                self.raw(),
                Some(&bytes),
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
            HelperProtocolAction::Hello | HelperProtocolAction::Started(_) => {
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

fn finish_settled(
    gate: HelperGateLease,
    mut lifetime: HelperLifetime,
    terminal: HelperProtocolTerminal,
) -> Result<(), InstallerError> {
    lifetime.mark_settled();
    let result = protocol_terminal_result(terminal);
    lifetime.cleanup_bridge();
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
    let _ = lifetime.controls().cancel();
    lifetime.cleanup_bridge();
    gate.finish();
    Err(error)
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

fn protocol_terminal_result(terminal: HelperProtocolTerminal) -> Result<(), InstallerError> {
    match terminal {
        HelperProtocolTerminal::Success => Ok(()),
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
    use super::*;

    const BRIDGE_IDENTITY: PinnedPackageIdentity = PinnedPackageIdentity::new(7, 11, 13);

    fn production_source() -> &'static str {
        include_str!("helper.rs")
            .split_once("#[cfg(test)]\nmod tests {")
            .expect("helper production source must precede the test module")
            .0
    }

    fn started(identity: PinnedPackageIdentity) -> HelperMessage {
        HelperMessage::Started { package: identity }
    }

    #[test]
    fn parent_sequence_requires_hello_control_started_admission_and_terminal() {
        let mut sequence = HelperProtocolSequence::default();
        assert!(matches!(
            sequence.accept(HelperMessage::Hello).unwrap(),
            HelperProtocolAction::Hello
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
        let source = production_source();
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
}
