use std::fmt::{Display, Formatter};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub process_name: String,
    pub process_path: String,
}
impl Target {
    pub fn new(process_name: String, process_path: String) -> Result<Self, TargetError> {
        if process_name.trim().is_empty() || process_path.trim().is_empty() {
            return Err(TargetError::Invalid);
        }
        Ok(Self {
            process_name: process_name.trim().to_owned(),
            process_path: normalize_path(&process_path),
        })
    }

    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    pub fn process_path(&self) -> Option<&Path> {
        Some(Path::new(&self.process_path))
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub enum ForegroundRestoreOutcome {
    Unchanged,
    Restored,
    Missing,
    Rejected,
}
pub trait ForegroundTargetRestorer {
    fn restore_saved_target(&self, target: &Target) -> ForegroundRestoreOutcome;
}
#[derive(Debug, Default)]
#[cfg(test)]
pub struct NoopForegroundRestorer;
#[cfg(test)]
impl ForegroundTargetRestorer for NoopForegroundRestorer {
    fn restore_saved_target(&self, _target: &Target) -> ForegroundRestoreOutcome {
        ForegroundRestoreOutcome::Unchanged
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForegroundIdentity {
    pub process_name: String,
    pub process_path: String,
}
pub trait ForegroundProbe {
    fn foreground_identity(&self) -> Result<Option<ForegroundIdentity>, TargetError>;
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetDecision {
    Ready,
    NoForeground,
    Unavailable,
    ProcessNameMismatch,
    ProcessPathMismatch,
}
pub fn evaluate_target(probe: &impl ForegroundProbe, target: &Target) -> TargetDecision {
    let identity = match probe.foreground_identity() {
        Ok(Some(identity)) => identity,
        Ok(None) => return TargetDecision::NoForeground,
        Err(_) => return TargetDecision::Unavailable,
    };
    if !identity
        .process_name
        .eq_ignore_ascii_case(&target.process_name)
    {
        return TargetDecision::ProcessNameMismatch;
    }
    if normalize_path(&identity.process_path) != target.process_path {
        return TargetDecision::ProcessPathMismatch;
    }
    TargetDecision::Ready
}
fn normalize_path(value: &str) -> String {
    value
        .replace('/', "\\")
        .trim_start_matches(r"\\?\")
        .to_ascii_lowercase()
}
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub fn process_paths_match(expected: &Path, actual: &Path) -> bool {
    normalize_process_path(expected).eq_ignore_ascii_case(&normalize_process_path(actual))
}
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub fn normalize_process_path(path: &Path) -> String {
    let unified = path.to_string_lossy().replace('/', "\\");
    if let Some(rest) = unified.strip_prefix(r"\\?\") {
        if let Some(unc) = rest.strip_prefix("UNC\\") {
            format!(r"\\{unc}")
        } else {
            rest.to_owned()
        }
    } else {
        unified
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetError {
    Invalid,
    Unavailable,
}
impl Display for TargetError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Invalid => "invalid target",
            Self::Unavailable => "foreground identity unavailable",
        })
    }
}
impl std::error::Error for TargetError {}

/// Reads the foreground identity only when a later live runtime event needs a
/// guard decision. Unit tests inject a ForegroundProbe fake instead.
pub struct WindowsForegroundProbe;

#[cfg(target_os = "windows")]
impl ForegroundProbe for WindowsForegroundProbe {
    fn foreground_identity(&self) -> Result<Option<ForegroundIdentity>, TargetError> {
        use std::ffi::{c_void, OsString};
        use std::os::windows::ffi::OsStringExt;

        const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
        #[link(name = "user32")]
        unsafe extern "system" {
            fn GetForegroundWindow() -> isize;
            fn GetWindowThreadProcessId(window: isize, process_id: *mut u32) -> u32;
        }
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn OpenProcess(access: u32, inherit_handle: i32, process_id: u32) -> *mut c_void;
            fn QueryFullProcessImageNameW(
                process: *mut c_void,
                flags: u32,
                executable_path: *mut u16,
                size: *mut u32,
            ) -> i32;
            fn CloseHandle(handle: *mut c_void) -> i32;
        }

        let window = unsafe { GetForegroundWindow() };
        if window == 0 {
            return Ok(None);
        }
        let mut process_id = 0_u32;
        if unsafe { GetWindowThreadProcessId(window, &mut process_id) } == 0 || process_id == 0 {
            return Err(TargetError::Unavailable);
        }
        let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, process_id) };
        if process.is_null() {
            return Err(TargetError::Unavailable);
        }
        let mut buffer = vec![0_u16; 32_768];
        let mut length = buffer.len() as u32;
        let read =
            unsafe { QueryFullProcessImageNameW(process, 0, buffer.as_mut_ptr(), &mut length) };
        let closed = unsafe { CloseHandle(process) };
        if read == 0 || closed == 0 {
            return Err(TargetError::Unavailable);
        }
        let path = OsString::from_wide(&buffer[..length as usize])
            .to_string_lossy()
            .into_owned();
        let name = std::path::Path::new(&path)
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or(TargetError::Unavailable)?
            .to_owned();
        Ok(Some(ForegroundIdentity {
            process_name: name,
            process_path: path,
        }))
    }
}

#[cfg(not(target_os = "windows"))]
impl ForegroundProbe for WindowsForegroundProbe {
    fn foreground_identity(&self) -> Result<Option<ForegroundIdentity>, TargetError> {
        Err(TargetError::Unavailable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    struct Fake(Result<Option<ForegroundIdentity>, TargetError>);
    impl ForegroundProbe for Fake {
        fn foreground_identity(&self) -> Result<Option<ForegroundIdentity>, TargetError> {
            self.0.clone()
        }
    }
    #[test]
    fn strict_target_rejects_wrong_foreground_path() {
        let target = Target::new("Codex.exe".into(), r"C:\Codex.exe".into()).unwrap();
        let probe = Fake(Ok(Some(ForegroundIdentity {
            process_name: "codex.exe".into(),
            process_path: r"C:\Other.exe".into(),
        })));
        assert_eq!(
            evaluate_target(&probe, &target),
            TargetDecision::ProcessPathMismatch
        );
        assert_eq!(
            target.process_path().map(std::path::Path::to_string_lossy),
            Some(std::borrow::Cow::Borrowed(r"c:\codex.exe"))
        );
    }
    #[test]
    fn process_paths_treat_windows_verbatim_prefix_as_the_same_strict_path() {
        assert!(process_paths_match(
            PathBuf::from(r"\\?\C:\Users\xk\AppData\Local\Programs\WorkBuddy\WorkBuddy.exe")
                .as_path(),
            PathBuf::from(r"C:\Users\xk\AppData\Local\Programs\WorkBuddy\WorkBuddy.exe").as_path(),
        ));
        assert!(process_paths_match(
            PathBuf::from(r"C:\Program Files\WorkBuddy\WorkBuddy.exe").as_path(),
            PathBuf::from(r"c:\program files\workbuddy\workbuddy.exe").as_path(),
        ));
        assert!(!process_paths_match(
            PathBuf::from(r"\\?\C:\Users\xk\AppData\Local\Programs\WorkBuddy\WorkBuddy.exe")
                .as_path(),
            PathBuf::from(r"C:\Users\xk\AppData\Local\Programs\WorkBuddy\WorkBuddy.old.exe")
                .as_path(),
        ));
        assert_eq!(
            normalize_process_path(PathBuf::from(r"\\?\UNC\server\share\WorkBuddy.exe").as_path()),
            r"\\server\share\WorkBuddy.exe",
        );
    }
}
