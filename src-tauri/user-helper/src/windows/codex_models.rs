//! Ordinary-user Codex model catalog: starts the user's own `codex app-server`
//! over stdio and reads `model/list` within the portable session bounds.

use std::{
    io::{BufRead, BufReader, Write},
    process::{Child, ChildStdin, ChildStdout},
    sync::mpsc::{self, Receiver, RecvTimeoutError, SyncSender},
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
    child: Child,
    stdin: Option<ChildStdin>,
    lines: Receiver<StdoutLine>,
}

impl AppServer {
    fn spawn(program: &Path) -> Result<Self, HelperErrorCode> {
        let profile =
            known_user_folder(&FOLDERID_Profile).map_err(|_| HelperErrorCode::CodexCliMissing)?;
        let mut command = if is_windows_command_script(program) {
            let cmd =
                system_command_processor().map_err(|_| HelperErrorCode::CodexAppServerFailed)?;
            let mut command = Command::new(cmd);
            command
                .args(["/D", "/S", "/C"])
                .raw_arg(command_script_line(program, &["app-server"]));
            command
        } else {
            let mut command = Command::new(program);
            command.arg("app-server");
            command
        };
        // FyAgent attributes the catalog by reading `<profile>\.codex`; a
        // user-level CODEX_HOME must not point app-server at another login.
        command
            .env("CODEX_HOME", profile.join(".codex"))
            .current_dir(profile)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .creation_flags(CREATE_NO_WINDOW);
        let mut child = command
            .spawn()
            .map_err(|_| HelperErrorCode::CodexAppServerFailed)?;
        let (Some(stdin), Some(stdout)) = (child.stdin.take(), child.stdout.take()) else {
            let _ = child.kill();
            let _ = child.wait();
            return Err(HelperErrorCode::CodexAppServerFailed);
        };
        let (sender, lines) = mpsc::sync_channel(STDOUT_LINE_QUEUE);
        // Detached: a reader blocked on a wedged grandchild must not hold the
        // helper open after its terminal frame.
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
        let exited = unsafe {
            WaitForSingleObject(
                HANDLE(self.child.as_raw_handle()),
                duration_millis(APP_SERVER_EXIT_GRACE),
            )
        } == WAIT_OBJECT_0;
        if !exited {
            let _ = self.child.kill();
        }
        let _ = self.child.wait();
    }
}

fn read_lines(stdout: ChildStdout, sender: SyncSender<StdoutLine>) {
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
