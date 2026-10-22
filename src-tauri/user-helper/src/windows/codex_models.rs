//! Ordinary-user Codex model catalog: starts the user's own `codex app-server`
//! over stdio and reads `model/list` within the portable session bounds.

use std::{
    io::{BufRead, BufReader, Write},
    sync::mpsc::{self, Receiver, RecvTimeoutError, SyncSender},
};

use windows::Win32::{
    Foundation::{SetHandleInformation, HANDLE_FLAGS, HANDLE_FLAG_INHERIT},
    Security::SECURITY_ATTRIBUTES,
    System::{
        JobObjects::{
            AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
            SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
            JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        },
        Pipes::CreatePipe,
        Threading::{
            CreateProcessW, DeleteProcThreadAttributeList, InitializeProcThreadAttributeList,
            ResumeThread, TerminateProcess, UpdateProcThreadAttribute,
            CREATE_NO_WINDOW as PROCESS_NO_WINDOW, CREATE_SUSPENDED, CREATE_UNICODE_ENVIRONMENT,
            EXTENDED_STARTUPINFO_PRESENT, LPPROC_THREAD_ATTRIBUTE_LIST, PROCESS_INFORMATION,
            PROC_THREAD_ATTRIBUTE_HANDLE_LIST, STARTF_USESTDHANDLES, STARTUPINFOEXW,
        },
    },
};

use super::*;
use fyagent_user_helper::codex_models::{
    classify_response_line, cli_version_from_initialize, codex_cli_search_directories,
    initialize_request, initialized_notification, model_list_request, model_list_request_id,
    CatalogStep, CodexCatalogBuilder, CodexCatalogError, CodexModelCatalog, RpcLine,
    CODEX_INITIALIZE_REQUEST_ID, CODEX_WINDOWS_EXECUTABLE_NAMES, MAX_CODEX_RPC_LINE_BYTES,
    MAX_CODEX_RPC_SKIPPED_LINES,
};

/// Whole app-server conversation. The parent's operation deadline is longer,
/// so a stalled server ends here with a terminal frame instead of
/// quarantining the helper gate.
const APP_SERVER_TIMEOUT: Duration = Duration::from_secs(45);
/// Time app-server gets to exit after its stdin closes.
const APP_SERVER_EXIT_GRACE: Duration = Duration::from_secs(2);
const STDOUT_LINE_QUEUE: usize = 8;

pub(super) fn execute() -> Result<CodexModelCatalog, HelperErrorCode> {
    let program = discover_cli()?;
    let mut server = AppServer::spawn(&program)?;
    server.read_catalog(Instant::now() + APP_SERVER_TIMEOUT)
}

fn discover_cli() -> Result<PathBuf, HelperErrorCode> {
    let profile =
        known_user_folder(&FOLDERID_Profile).map_err(|_| HelperErrorCode::CodexCliMissing)?;
    let path = interactive_path_directories().unwrap_or_default();
    codex_cli_search_directories(&path, &profile)
        .into_iter()
        .flat_map(|directory| {
            CODEX_WINDOWS_EXECUTABLE_NAMES
                .iter()
                .map(move |name| directory.join(name))
        })
        .find(|candidate| candidate.is_file())
        .ok_or(HelperErrorCode::CodexCliMissing)
}

enum StdoutLine {
    Line(Vec<u8>),
    TooLong,
}

struct AppServer {
    child: ProcessTree,
    stdin: Option<File>,
    lines: Receiver<StdoutLine>,
}

impl AppServer {
    fn spawn(program: &Path) -> Result<Self, HelperErrorCode> {
        let profile =
            known_user_folder(&FOLDERID_Profile).map_err(|_| HelperErrorCode::CodexCliMissing)?;
        Self::spawn_in(program, &profile)
    }

    fn spawn_in(program: &Path, directory: &Path) -> Result<Self, HelperErrorCode> {
        let (child, stdin, stdout) = ProcessTree::spawn(program, directory)?;
        let (sender, lines) = mpsc::sync_channel(STDOUT_LINE_QUEUE);
        // The per-call job closes every inherited writer on settlement.
        thread::spawn(move || read_lines(stdout, sender));
        Ok(Self {
            child,
            stdin: Some(stdin),
            lines,
        })
    }

    fn read_catalog(&mut self, deadline: Instant) -> Result<CodexModelCatalog, HelperErrorCode> {
        self.send(&initialize_request(env!("CARGO_PKG_VERSION")))?;
        let initialized = self.await_result(CODEX_INITIALIZE_REQUEST_ID, deadline)?;
        self.send(&initialized_notification())?;
        let mut builder = CodexCatalogBuilder::new(cli_version_from_initialize(&initialized));
        let mut cursor: Option<String> = None;
        loop {
            let index = builder.pages_read();
            self.send(&model_list_request(index, cursor.as_deref()))?;
            let page = self.await_result(model_list_request_id(index), deadline)?;
            match builder.accept_page(&page).map_err(catalog_error)? {
                CatalogStep::Next(next) => cursor = Some(next),
                CatalogStep::Done => return Ok(builder.finish()),
            }
        }
    }

    fn send(&mut self, line: &[u8]) -> Result<(), HelperErrorCode> {
        let stdin = self
            .stdin
            .as_mut()
            .ok_or(HelperErrorCode::CodexAppServerFailed)?;
        stdin
            .write_all(line)
            .and_then(|()| stdin.flush())
            .map_err(|_| HelperErrorCode::CodexAppServerFailed)
    }

    fn await_result(
        &self,
        expected_id: u64,
        deadline: Instant,
    ) -> Result<serde_json::Value, HelperErrorCode> {
        let mut skipped = 0_usize;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(HelperErrorCode::CodexAppServerTimedOut);
            }
            match self.lines.recv_timeout(remaining) {
                Ok(StdoutLine::Line(line)) => {
                    match classify_response_line(&line, expected_id).map_err(catalog_error)? {
                        RpcLine::Result(value) => return Ok(value),
                        RpcLine::Unrelated => {
                            skipped += 1;
                            if skipped > MAX_CODEX_RPC_SKIPPED_LINES {
                                return Err(HelperErrorCode::CodexCatalogInvalid);
                            }
                        }
                    }
                }
                Ok(StdoutLine::TooLong) => return Err(HelperErrorCode::CodexCatalogInvalid),
                Err(RecvTimeoutError::Timeout) => {
                    return Err(HelperErrorCode::CodexAppServerTimedOut)
                }
                Err(RecvTimeoutError::Disconnected) => {
                    return Err(HelperErrorCode::CodexAppServerFailed)
                }
            }
        }
    }
}

impl Drop for AppServer {
    fn drop(&mut self) {
        // app-server exits on stdin EOF. Through the npm `.cmd` shim the
        // child is cmd.exe; closing stdin still reaches the real server.
        drop(self.stdin.take());
        unsafe {
            WaitForSingleObject(
                self.child.process.raw(),
                duration_millis(APP_SERVER_EXIT_GRACE),
            );
        }
        // ProcessTree drops next, even if the outer shim has already exited.
    }
}

/// Private owner for this one closed action; never contains the helper itself.
struct ProcessTree {
    job: Option<OwnedKernelHandle>,
    process: OwnedKernelHandle,
}

impl ProcessTree {
    fn spawn(program: &Path, directory: &Path) -> Result<(Self, File, File), HelperErrorCode> {
        let failed = || HelperErrorCode::CodexAppServerFailed;
        validate_program_path(program)?;
        let (application, command_line) = if is_windows_command_script(program) {
            let cmd = system_command_processor().map_err(|_| failed())?;
            let line = format!(
                "{} /D /S /C {}",
                quote_windows_path(&cmd),
                command_script_line(program, &["app-server"])
            );
            (cmd, line)
        } else {
            (
                program.to_path_buf(),
                format!("{} app-server", quote_windows_path(program)),
            )
        };
        let application = wide_null(application.as_os_str()).map_err(|_| failed())?;
        let mut command_line = wide_null(OsStr::new(&command_line)).map_err(|_| failed())?;
        // Preserve stable A4's account attribution: never inherit an alternate
        // CODEX_HOME when the host observed this profile's `.codex` login.
        let environment = app_server_environment(directory)?;
        let directory = wide_null(directory.as_os_str()).map_err(|_| failed())?;
        let job = OwnedKernelHandle::new(
            unsafe { CreateJobObjectW(None, PCWSTR::null()) }.map_err(|_| failed())?,
        )
        .map_err(|_| failed())?;
        let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        unsafe {
            SetInformationJobObject(
                job.raw(),
                JobObjectExtendedLimitInformation,
                &limits as *const _ as *const _,
                size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        }
        .map_err(|_| failed())?;
        let (child_stdin, parent_stdin) = pipe_pair(false)?;
        let (parent_stdout, child_stdout) = pipe_pair(true)?;
        let security = inheritable_security();
        let nul = wide_null(OsStr::new("NUL")).map_err(|_| failed())?;
        let child_stderr = OwnedKernelHandle::new(
            unsafe {
                CreateFileW(
                    PCWSTR(nul.as_ptr()),
                    FILE_GENERIC_WRITE.0,
                    FILE_SHARE_READ | FILE_SHARE_WRITE,
                    Some(&security),
                    OPEN_EXISTING,
                    FILE_ATTRIBUTE_NORMAL,
                    None,
                )
            }
            .map_err(|_| failed())?,
        )
        .map_err(|_| failed())?;
        // Restrict inheritance to stdio. The authenticated helper/control pipe
        // and unrelated inheritable handles must never reach the user CLI.
        let handles = [child_stdin.raw(), child_stdout.raw(), child_stderr.raw()];
        let attributes = InheritedHandles::new(&handles)?;
        let mut startup = STARTUPINFOEXW::default();
        startup.StartupInfo.cb = size_of::<STARTUPINFOEXW>() as u32;
        startup.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
        startup.StartupInfo.hStdInput = handles[0];
        startup.StartupInfo.hStdOutput = handles[1];
        startup.StartupInfo.hStdError = handles[2];
        startup.lpAttributeList = attributes.raw();
        let mut info = PROCESS_INFORMATION::default();
        unsafe {
            CreateProcessW(
                PCWSTR(application.as_ptr()),
                Some(PWSTR(command_line.as_mut_ptr())),
                None,
                None,
                true,
                CREATE_SUSPENDED
                    | PROCESS_NO_WINDOW
                    | EXTENDED_STARTUPINFO_PRESENT
                    | CREATE_UNICODE_ENVIRONMENT,
                Some(environment.as_ptr().cast()),
                PCWSTR(directory.as_ptr()),
                &startup.StartupInfo,
                &mut info,
            )
        }
        .map_err(|_| failed())?;
        let process = OwnedKernelHandle::new(info.hProcess).map_err(|_| failed())?;
        let primary_thread = OwnedKernelHandle::new(info.hThread).map_err(|_| failed())?;
        let tree = Self {
            job: Some(job),
            process,
        };
        // No user code runs until assignment. A spawn-then-assign race would
        // let an npm shim create descendants outside this job.
        if unsafe { AssignProcessToJobObject(tree.job.as_ref().unwrap().raw(), tree.process.raw()) }
            .is_err()
            || unsafe { ResumeThread(primary_thread.raw()) } == u32::MAX
        {
            // Assignment can fail when the host imposes incompatible job
            // restrictions. Fail closed; never run an uncontained process.
            let _ = unsafe { TerminateProcess(tree.process.raw(), 1) };
            return Err(failed());
        }
        Ok((
            tree,
            File::from(parent_stdin.0),
            File::from(parent_stdout.0),
        ))
    }
}

impl Drop for ProcessTree {
    fn drop(&mut self) {
        // KILL_ON_JOB_CLOSE also covers a shim that exits before its children.
        // The job handle is non-inheritable and owned only by this operation.
        drop(self.job.take());
        unsafe {
            WaitForSingleObject(self.process.raw(), duration_millis(APP_SERVER_EXIT_GRACE));
        }
    }
}

fn validate_program_path(program: &Path) -> Result<(), HelperErrorCode> {
    let failed = HelperErrorCode::CodexAppServerFailed;
    validate_ordinary_dos_path(program).map_err(|_| failed)?;
    let path = program.to_str().ok_or(failed)?;
    if path.chars().any(|c| c.is_control() || c == '"')
        || (is_windows_command_script(program)
            && path
                .chars()
                .any(|c| matches!(c, '%' | '!' | '^' | '&' | '|' | '<' | '>')))
    {
        return Err(failed);
    }
    Ok(())
}

fn app_server_environment(profile: &Path) -> Result<Vec<u16>, HelperErrorCode> {
    let mut entries = std::env::vars_os()
        .filter(|(key, _)| !key.to_string_lossy().eq_ignore_ascii_case("CODEX_HOME"))
        .collect::<Vec<_>>();
    entries.push((
        OsString::from("CODEX_HOME"),
        profile.join(".codex").into_os_string(),
    ));
    entries.sort_unstable_by_key(|(key, _)| key.to_string_lossy().to_uppercase());
    let mut block = Vec::new();
    for (key, value) in entries {
        let mut entry = key;
        entry.push("=");
        entry.push(value);
        block.extend(wide_null(&entry).map_err(|_| HelperErrorCode::CodexAppServerFailed)?);
    }
    block.push(0);
    Ok(block)
}

fn inheritable_security() -> SECURITY_ATTRIBUTES {
    SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        bInheritHandle: BOOL(1),
        ..Default::default()
    }
}

fn pipe_pair(
    parent_reads: bool,
) -> Result<(OwnedKernelHandle, OwnedKernelHandle), HelperErrorCode> {
    let failed = || HelperErrorCode::CodexAppServerFailed;
    let (mut read, mut write) = (HANDLE::default(), HANDLE::default());
    unsafe { CreatePipe(&mut read, &mut write, Some(&inheritable_security()), 0) }
        .map_err(|_| failed())?;
    let read = OwnedKernelHandle::new(read).map_err(|_| failed())?;
    let write = OwnedKernelHandle::new(write).map_err(|_| failed())?;
    let parent = if parent_reads { &read } else { &write };
    unsafe { SetHandleInformation(parent.raw(), HANDLE_FLAG_INHERIT.0, HANDLE_FLAGS(0)) }
        .map_err(|_| failed())?;
    Ok((read, write))
}

struct InheritedHandles {
    // Pointer-aligned storage; the API keeps the handle array borrowed only
    // until CreateProcessW returns, within the caller's stack frame.
    storage: Vec<usize>,
}

impl InheritedHandles {
    fn new(handles: &[HANDLE]) -> Result<Self, HelperErrorCode> {
        let failed = || HelperErrorCode::CodexAppServerFailed;
        let mut bytes = 0;
        let _ = unsafe { InitializeProcThreadAttributeList(None, 1, None, &mut bytes) };
        if bytes == 0 {
            return Err(failed());
        }
        let mut storage = vec![0_usize; bytes.div_ceil(size_of::<usize>())];
        let raw = LPPROC_THREAD_ATTRIBUTE_LIST(storage.as_mut_ptr().cast());
        unsafe { InitializeProcThreadAttributeList(Some(raw), 1, None, &mut bytes) }
            .map_err(|_| failed())?;
        let attributes = Self { storage };
        unsafe {
            UpdateProcThreadAttribute(
                attributes.raw(),
                0,
                PROC_THREAD_ATTRIBUTE_HANDLE_LIST as usize,
                Some(handles.as_ptr().cast()),
                std::mem::size_of_val(handles),
                None,
                None,
            )
        }
        .map_err(|_| failed())?;
        Ok(attributes)
    }

    fn raw(&self) -> LPPROC_THREAD_ATTRIBUTE_LIST {
        LPPROC_THREAD_ATTRIBUTE_LIST(self.storage.as_ptr() as *mut _)
    }
}

impl Drop for InheritedHandles {
    fn drop(&mut self) {
        unsafe {
            DeleteProcThreadAttributeList(self.raw());
        }
    }
}

fn read_lines(stdout: File, sender: SyncSender<StdoutLine>) {
    let mut reader = BufReader::new(stdout);
    let mut line = Vec::new();
    loop {
        let (consumed, complete) = match reader.fill_buf() {
            Ok([]) => return,
            Ok(buffer) => {
                let newline = buffer.iter().position(|byte| *byte == b'\n');
                let chunk = &buffer[..newline.unwrap_or(buffer.len())];
                if line.len() + chunk.len() > MAX_CODEX_RPC_LINE_BYTES {
                    let _ = sender.send(StdoutLine::TooLong);
                    return;
                }
                line.extend_from_slice(chunk);
                match newline {
                    Some(index) => (index + 1, true),
                    None => (buffer.len(), false),
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => return,
        };
        reader.consume(consumed);
        if complete {
            if line.last() == Some(&b'\r') {
                line.pop();
            }
            if sender
                .send(StdoutLine::Line(std::mem::take(&mut line)))
                .is_err()
            {
                return;
            }
        }
    }
}

fn catalog_error(error: CodexCatalogError) -> HelperErrorCode {
    match error {
        CodexCatalogError::MethodUnsupported => HelperErrorCode::CodexModelListUnsupported,
        CodexCatalogError::RequestFailed => HelperErrorCode::CodexAppServerFailed,
        CodexCatalogError::InvalidResponse => HelperErrorCode::CodexCatalogInvalid,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, process::Child, time::SystemTime};
    use windows::Win32::System::Threading::{
        GetProcessId, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE,
    };

    #[test]
    fn shim_path_rejects_second_expansion_and_shell_metacharacters() {
        for path in [
            r"C:\fixture\%TEAM%\codex.cmd",
            r"C:\fixture\!TEAM!\codex.cmd",
            r"C:\fixture\caret^\codex.cmd",
            r"C:\fixture\a&b\codex.cmd",
            r"C:\fixture\a|b\codex.cmd",
            r"C:\fixture\a<b\codex.cmd",
            r"C:\fixture\a>b\codex.cmd",
            "C:\\fixture\\a\"b\\codex.cmd",
            "C:\\fixture\\line\nbreak\\codex.cmd",
            r"relative\codex.cmd",
            r"C:\fixture\..\codex.cmd",
            r"\\server\share\codex.cmd",
        ] {
            assert!(validate_program_path(Path::new(path)).is_err(), "{path:?}");
        }
        assert!(validate_program_path(Path::new(r"C:\fixture with spaces\codex.cmd")).is_ok());
        assert!(validate_program_path(Path::new(r"C:\fixture\literal%name\codex.exe")).is_ok());
    }

    // These tests execute only generated local fixture scripts, never a CLI
    // discovered from PATH. All kill operations use retained owned handles.
    struct Fixture(PathBuf);

    impl Fixture {
        fn new(respond: bool) -> Self {
            let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/codex-models-tests");
            let directory = root.join(format!(
                "fixture with spaces-{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            fs::create_dir_all(&directory).unwrap();
            fs::write(directory.join("codex.cmd"), concat!(
                "@echo off\r\n",
                "\"%SystemRoot%\\System32\\WindowsPowerShell\\v1.0\\powershell.exe\" -NoLogo -NoProfile -NonInteractive -ExecutionPolicy Bypass -File \"%~dp0server.ps1\"\r\n"
            )).unwrap();
            let script = r#"
$ErrorActionPreference = 'Stop'
if ([IO.Path]::GetFullPath($env:CODEX_HOME) -cne [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '.codex'))) { throw 'fixture home mismatch' }
$ps = Join-Path $PSHOME 'powershell.exe'
$descendant = Start-Process -FilePath $ps -WindowStyle Hidden -ArgumentList @('-NoProfile', '-NonInteractive', '-Command', 'Start-Sleep -Seconds 90') -PassThru
[IO.File]::WriteAllText((Join-Path $PSScriptRoot 'owned-pids.txt'), "$PID`n$($descendant.Id)")
if (__RESPOND__) {
    while ($line = [Console]::ReadLine()) {
        $request = $line | ConvertFrom-Json
        if ($request.method -eq 'initialize') {
            [Console]::WriteLine((@{id=$request.id; result=@{userAgent='fixture/1.0'}} | ConvertTo-Json -Compress))
        } elseif ($request.method -eq 'model/list') {
            [Console]::WriteLine((@{id=$request.id; result=@{data=@(@{id='fixture-model';model='fixture-model';displayName='Fixture';isDefault=$true;hidden=$false;supportedReasoningEfforts=@()}); nextCursor=$null}} | ConvertTo-Json -Depth 8 -Compress))
            break
        }
    }
}
# Deliberately ignore stdin EOF. The job, not cooperative process exit,
# must settle both this process and its descendant on success and timeout.
Start-Sleep -Seconds 90
"#.replace("__RESPOND__", if respond { "$true" } else { "$false" });
            fs::write(directory.join("server.ps1"), script).unwrap();
            Self(directory)
        }

        fn descendants(&self) -> Vec<OwnedKernelHandle> {
            let deadline = Instant::now() + Duration::from_secs(15);
            loop {
                let pids = fs::read_to_string(self.0.join("owned-pids.txt"))
                    .unwrap_or_default()
                    .lines()
                    .filter_map(|value| value.parse::<u32>().ok())
                    .collect::<Vec<_>>();
                if pids.len() == 2 {
                    return pids
                        .into_iter()
                        .map(|pid| {
                            let handle = unsafe {
                                OpenProcess(
                                    PROCESS_SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION,
                                    false,
                                    pid,
                                )
                            }
                            .unwrap();
                            OwnedKernelHandle::new(handle).unwrap()
                        })
                        .collect();
                }
                assert!(
                    Instant::now() < deadline,
                    "fixture descendants did not become ready"
                );
                thread::sleep(Duration::from_millis(25));
            }
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/codex-models-tests");
            assert!(self.0.is_absolute() && self.0.parent() == Some(root.as_path()));
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    struct Sentinel(Child);

    impl Sentinel {
        fn new() -> Self {
            let powershell = system_command_processor()
                .unwrap()
                .parent()
                .unwrap()
                .join("WindowsPowerShell/v1.0/powershell.exe");
            Self(
                Command::new(powershell)
                    .args([
                        "-NoProfile",
                        "-NonInteractive",
                        "-Command",
                        "Start-Sleep -Seconds 90",
                    ])
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .creation_flags(CREATE_NO_WINDOW)
                    .spawn()
                    .unwrap(),
            )
        }
    }

    impl Drop for Sentinel {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }

    fn exercise_tree_cleanup(respond: bool) {
        let fixture = Fixture::new(respond);
        let mut sentinel = Sentinel::new();
        let mut server = AppServer::spawn_in(&fixture.0.join("codex.cmd"), &fixture.0).unwrap();
        let direct_pid = unsafe { GetProcessId(server.child.process.raw()) };
        let direct = OwnedKernelHandle::new(
            unsafe { OpenProcess(PROCESS_SYNCHRONIZE, false, direct_pid) }.unwrap(),
        )
        .unwrap();
        let descendants = fixture.descendants();
        for process in &descendants {
            assert_eq!(
                unsafe { WaitForSingleObject(process.raw(), 0) },
                WAIT_TIMEOUT
            );
            assert_ne!(unsafe { GetProcessId(process.raw()) }, direct_pid);
        }
        let result = server.read_catalog(
            Instant::now()
                + if respond {
                    Duration::from_secs(5)
                } else {
                    Duration::from_millis(100)
                },
        );
        if respond {
            assert_eq!(result.unwrap().models[0].id, "fixture-model");
        } else {
            assert!(matches!(
                result,
                Err(HelperErrorCode::CodexAppServerTimedOut)
            ));
        }
        drop(server);
        for process in std::iter::once(&direct).chain(descendants.iter()) {
            assert_eq!(
                unsafe { WaitForSingleObject(process.raw(), 5000) },
                WAIT_OBJECT_0,
                "owned descendant survived settlement"
            );
        }
        assert!(
            sentinel.0.try_wait().unwrap().is_none(),
            "unrelated sentinel was terminated"
        );
    }

    #[test]
    fn successful_catalog_cleans_child_and_grandchild_but_preserves_sentinel() {
        exercise_tree_cleanup(true);
    }

    #[test]
    fn timed_out_catalog_cleans_child_and_grandchild_but_preserves_sentinel() {
        exercise_tree_cleanup(false);
    }
}
