#[cfg(target_os = "windows")]
use std::time::{Duration, Instant};

#[cfg(target_os = "windows")]
use super::target::process_paths_match;
use super::target::{ForegroundRestoreOutcome, ForegroundTargetRestorer, Target};

pub struct WindowsForegroundTargetRestorer;

impl ForegroundTargetRestorer for WindowsForegroundTargetRestorer {
    fn restore_saved_target(&self, target: &Target) -> ForegroundRestoreOutcome {
        restore_saved_target(target)
    }
}

#[cfg(target_os = "windows")]
fn restore_saved_target(target: &Target) -> ForegroundRestoreOutcome {
    let Some(expected_path) = target.process_path() else {
        return ForegroundRestoreOutcome::Unchanged;
    };
    let Ok(matches) = matching_windows(expected_path) else {
        return ForegroundRestoreOutcome::Rejected;
    };
    let Some(window) = matches.into_iter().next() else {
        return ForegroundRestoreOutcome::Missing;
    };
    focus_window(window)
}

#[cfg(not(target_os = "windows"))]
fn restore_saved_target(_target: &Target) -> ForegroundRestoreOutcome {
    ForegroundRestoreOutcome::Unchanged
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub struct RestoreWindowHint {
    pub visible: bool,
    pub owned: bool,
    pub tool_window: bool,
}

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub fn is_restore_candidate(hint: RestoreWindowHint) -> bool {
    hint.visible && !hint.owned && !hint.tool_window
}

#[cfg(target_os = "windows")]
fn matching_windows(
    expected_path: &std::path::Path,
) -> Result<Vec<windows_sys::Win32::Foundation::HWND>, ()> {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use std::path::PathBuf;

    use windows_sys::Win32::Foundation::{HWND, LPARAM};
    use windows_sys::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindow, GetWindowLongPtrW, GetWindowThreadProcessId, IsWindowVisible,
        GWL_EXSTYLE, GW_OWNER, WS_EX_TOOLWINDOW,
    };

    struct Search<'a> {
        expected_path: &'a std::path::Path,
        matches: Vec<HWND>,
    }

    unsafe extern "system" fn callback(hwnd: HWND, lparam: LPARAM) -> windows_sys::core::BOOL {
        let search = unsafe { &mut *(lparam as *mut Search<'static>) };
        let visible = unsafe { IsWindowVisible(hwnd) } != 0;
        let owned = !unsafe { GetWindow(hwnd, GW_OWNER) }.is_null();
        let tool_window =
            unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) } as u32 & WS_EX_TOOLWINDOW != 0;
        if !is_restore_candidate(RestoreWindowHint {
            visible,
            owned,
            tool_window,
        }) {
            return 1;
        }

        let mut process_id = 0;
        let thread_id = unsafe { GetWindowThreadProcessId(hwnd, &mut process_id) };
        if thread_id == 0 || process_id == 0 {
            return 1;
        }
        let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, process_id) };
        if process.is_null() {
            return 1;
        }
        let mut buffer = vec![0u16; 32_768];
        let mut size = u32::try_from(buffer.len()).expect("fixed restore path buffer fits in u32");
        let read =
            unsafe { QueryFullProcessImageNameW(process, 0, buffer.as_mut_ptr(), &mut size) };
        let _ = unsafe { windows_sys::Win32::Foundation::CloseHandle(process) };
        if read == 0 {
            return 1;
        }
        let path = PathBuf::from(OsString::from_wide(
            &buffer[..usize::try_from(size).expect("u32 path length fits usize")],
        ));
        if process_paths_match(search.expected_path, &path) {
            search.matches.push(hwnd);
        }
        1
    }

    let mut search = Search {
        expected_path,
        matches: Vec::new(),
    };
    // The callback does not outlive this call. The lifetime erasure is only
    // needed to satisfy the Win32 callback ABI and is restored immediately.
    let callback_arg = (&mut search as *mut Search<'_>) as isize;
    if unsafe { EnumWindows(Some(callback), callback_arg) } == 0 {
        return Err(());
    }
    Ok(search.matches)
}

#[cfg(target_os = "windows")]
fn focus_window(target: windows_sys::Win32::Foundation::HWND) -> ForegroundRestoreOutcome {
    use windows_sys::Win32::System::Threading::GetCurrentThreadId;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        BringWindowToTop, GetForegroundWindow, GetWindowThreadProcessId, IsIconic,
        SetForegroundWindow, ShowWindowAsync, SW_RESTORE,
    };

    if unsafe { IsIconic(target) } != 0 {
        unsafe {
            let _ = ShowWindowAsync(target, SW_RESTORE);
        }
    }

    let mut accepted = unsafe { SetForegroundWindow(target) } != 0;
    if !accepted {
        let foreground = unsafe { GetForegroundWindow() };
        let current_thread = unsafe { GetCurrentThreadId() };
        let foreground_thread = if foreground.is_null() {
            0
        } else {
            unsafe { GetWindowThreadProcessId(foreground, std::ptr::null_mut()) }
        };
        if foreground_thread != 0 && foreground_thread != current_thread {
            // UART is not a Windows input event, so SetForegroundWindow is often
            // rejected. Share the current foreground input queue only for this
            // bounded attempt; Drop detaches before the verified wait loop.
            if let Some(_attached) = AttachedInput::join(current_thread, foreground_thread) {
                unsafe {
                    let _ = BringWindowToTop(target);
                    accepted = SetForegroundWindow(target) != 0;
                }
            }
        }
    }
    if !accepted {
        return ForegroundRestoreOutcome::Rejected;
    }

    let deadline = Instant::now() + Duration::from_millis(250);
    loop {
        if unsafe { GetForegroundWindow() } == target {
            return ForegroundRestoreOutcome::Restored;
        }
        if Instant::now() >= deadline {
            return ForegroundRestoreOutcome::Rejected;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[cfg(target_os = "windows")]
struct AttachedInput {
    current_thread: u32,
    foreground_thread: u32,
}

#[cfg(target_os = "windows")]
impl AttachedInput {
    fn join(current_thread: u32, foreground_thread: u32) -> Option<Self> {
        use windows_sys::Win32::System::Threading::AttachThreadInput;
        if unsafe { AttachThreadInput(current_thread, foreground_thread, 1) } == 0 {
            return None;
        }
        Some(Self {
            current_thread,
            foreground_thread,
        })
    }
}

#[cfg(target_os = "windows")]
impl Drop for AttachedInput {
    fn drop(&mut self) {
        use windows_sys::Win32::System::Threading::AttachThreadInput;
        unsafe {
            let _ = AttachThreadInput(self.current_thread, self.foreground_thread, 0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{is_restore_candidate, RestoreWindowHint};

    #[test]
    fn restore_candidate_skips_hidden_owned_and_tool_windows() {
        assert!(is_restore_candidate(RestoreWindowHint {
            visible: true,
            owned: false,
            tool_window: false,
        }));
        assert!(!is_restore_candidate(RestoreWindowHint {
            visible: false,
            owned: false,
            tool_window: false,
        }));
        assert!(!is_restore_candidate(RestoreWindowHint {
            visible: true,
            owned: true,
            tool_window: false,
        }));
        assert!(!is_restore_candidate(RestoreWindowHint {
            visible: true,
            owned: false,
            tool_window: true,
        }));
    }
}
