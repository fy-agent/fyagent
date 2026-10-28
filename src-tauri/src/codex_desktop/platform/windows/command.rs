//! Windows counterpart of the macOS CommandRunner boundary: fixed system
//! executable, independent environment values, bounded output and deadline.
use std::{
    ffi::OsString,
    io::Read,
    path::PathBuf,
    process::{Command, Stdio},
    sync::mpsc,
    time::{Duration, Instant},
};

pub(super) const MAX_OUTPUT: usize = 16 * 1024;
const TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone)]
pub(super) struct CommandInvocation {
    pub program: PathBuf,
    pub arguments: Vec<OsString>,
    pub package_path: OsString,
}

pub(super) struct CommandOutput {
    pub status: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub truncated: bool,
}

pub(super) trait CommandRunner: Send + Sync {
    fn run(&self, invocation: &CommandInvocation) -> Result<CommandOutput, ()>;
}

pub(super) struct SystemCommandRunner;

pub(super) fn is_absolute_system_program(path: &std::path::Path) -> bool {
    let Some(path) = path.to_str() else {
        return false;
    };
    let bytes = path.as_bytes();
    bytes.len() > 3
        && bytes[0].is_ascii_alphabetic()
        && &bytes[1..3] == b":\\"
        && !path.contains('/')
        && !path.split('\\').any(|part| part == ".." || part == ".")
        && path.ends_with("\\WindowsPowerShell\\v1.0\\powershell.exe")
}

impl CommandRunner for SystemCommandRunner {
    fn run(&self, invocation: &CommandInvocation) -> Result<CommandOutput, ()> {
        if !is_absolute_system_program(&invocation.program) {
            return Err(());
        }
        let mut command = Command::new(&invocation.program);
        command
            .args(&invocation.arguments)
            .env("FYAGENT_CODEX_SIGNATURE_PATH", &invocation.package_path)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        }
        let mut child = command.spawn().map_err(|_| ())?;
        let (sender, receiver) = mpsc::channel();
        let stdout = child.stdout.take().ok_or(())?;
        let stderr = child.stderr.take().ok_or(())?;
        let stdout_sender = sender.clone();
        std::thread::spawn(move || {
            let _ = stdout_sender.send((true, read_output(stdout)));
        });
        std::thread::spawn(move || {
            let _ = sender.send((false, read_output(stderr)));
        });
        let deadline = Instant::now() + TIMEOUT;
        let result = (|| {
            let status = loop {
                if let Some(status) = child.try_wait().map_err(|_| ())? {
                    break status.code();
                }
                if Instant::now() >= deadline {
                    return Err(());
                }
                std::thread::sleep(Duration::from_millis(10));
            };
            let mut output = CommandOutput {
                status,
                stdout: Vec::new(),
                stderr: Vec::new(),
                truncated: false,
            };
            for _ in 0..2 {
                let remaining = deadline.checked_duration_since(Instant::now()).ok_or(())?;
                let (stdout, bytes) = receiver.recv_timeout(remaining).map_err(|_| ())?;
                let (bytes, truncated) = bytes.map_err(|_| ())?;
                output.truncated |= truncated;
                if stdout {
                    output.stdout = bytes;
                } else {
                    output.stderr = bytes;
                }
            }
            Ok(output)
        })();
        if result.is_err() {
            let _ = child.kill();
            let _ = child.wait();
        }
        result
    }
}

fn read_output(mut reader: impl Read) -> std::io::Result<(Vec<u8>, bool)> {
    let mut bytes = Vec::new();
    let mut truncated = false;
    let mut buffer = [0; 4096];
    loop {
        let count = match reader.read(&mut buffer) {
            Ok(0) => return Ok((bytes, truncated)),
            Ok(count) => count,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        };
        let keep = count.min(MAX_OUTPUT - bytes.len());
        bytes.extend_from_slice(&buffer[..keep]);
        truncated |= keep < count;
    }
}

#[cfg(target_os = "windows")]
pub(super) fn powershell_path() -> Result<PathBuf, ()> {
    use windows::Win32::System::SystemInformation::GetSystemDirectoryW;
    let mut buffer = [0u16; 32768];
    let length = unsafe { GetSystemDirectoryW(Some(&mut buffer)) } as usize;
    if length == 0 || length >= buffer.len() {
        return Err(());
    }
    use std::os::windows::ffi::OsStringExt;
    Ok(PathBuf::from(OsString::from_wide(&buffer[..length]))
        .join(r"WindowsPowerShell\v1.0\powershell.exe"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn output_reader_is_bounded_and_marks_truncation() {
        for size in [0, MAX_OUTPUT, MAX_OUTPUT + 1, MAX_OUTPUT * 4] {
            let (bytes, truncated) = read_output(std::io::Cursor::new(vec![b'x'; size])).unwrap();
            assert_eq!(bytes.len(), size.min(MAX_OUTPUT));
            assert_eq!(truncated, size > MAX_OUTPUT);
        }
    }
    #[test]
    fn runtime_rejects_non_absolute_commands_before_spawn() {
        for path in [
            "powershell.exe",
            r"C:WindowsPowerShell\v1.0\powershell.exe",
            r"\WindowsPowerShell\v1.0\powershell.exe",
            r"C:\Windows\..\WindowsPowerShell\v1.0\powershell.exe",
        ] {
            assert!(SystemCommandRunner
                .run(&CommandInvocation {
                    program: path.into(),
                    arguments: vec![],
                    package_path: "unused".into()
                })
                .is_err());
        }
    }
}
