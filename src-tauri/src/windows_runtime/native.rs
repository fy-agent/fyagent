//! Native Explorer-token resolution for the frozen Shell-user context.

use std::{ffi::OsString, os::windows::ffi::OsStringExt, path::PathBuf};

use windows::{
    core::{BOOL, PCWSTR, PWSTR},
    Win32::{
        Foundation::{CloseHandle, HANDLE, HWND},
        Security::{
            Authorization::ConvertSidToStringSidW, CheckTokenMembership, CreateWellKnownSid,
            GetTokenInformation, TokenElevation, TokenSessionId, TokenUser,
            WinBuiltinAdministratorsSid, PSID, SECURITY_MAX_SID_SIZE, TOKEN_DUPLICATE,
            TOKEN_ELEVATION, TOKEN_IMPERSONATE, TOKEN_QUERY, TOKEN_USER,
        },
        Storage::FileSystem::GetDriveTypeW,
        System::{
            Com::CoTaskMemFree,
            Environment::{CreateEnvironmentBlock, DestroyEnvironmentBlock},
            Threading::{
                GetCurrentProcess, OpenProcess, OpenProcessToken, PROCESS_QUERY_LIMITED_INFORMATION,
            },
            WindowsProgramming::DRIVE_FIXED,
        },
        UI::{
            Shell::{
                FOLDERID_LocalAppData, FOLDERID_Profile, FOLDERID_ProgramFiles,
                FOLDERID_ProgramFilesX86, FOLDERID_RoamingAppData, SHGetKnownFolderPath,
                KNOWN_FOLDER_FLAG,
            },
            WindowsAndMessaging::{GetShellWindow, GetWindowThreadProcessId},
        },
    },
};

use super::{
    build_interactive_user_context, is_canonical_sid, InteractiveUserMatch,
    InteractiveUserObservation, RuntimePrivilegePlatform, RuntimePrivilegeStatus,
    WindowsInteractiveUserContext, WindowsStartupErrorCode,
};

struct OwnedHandle(HANDLE);

impl OwnedHandle {
    fn get(&self) -> HANDLE {
        self.0
    }
}

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        if !self.0.is_invalid() {
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }
}

struct IdentityProbe {
    process_session_id: u32,
    process_sid: Option<String>,
    shell_session_id: u32,
    shell_sid: String,
    user_profile: PathBuf,
    user_local_app_data: PathBuf,
    user_roaming_app_data: PathBuf,
    shell_command_paths: Vec<PathBuf>,
}

pub(super) fn resolve_interactive_user_context(
) -> Result<WindowsInteractiveUserContext, WindowsStartupErrorCode> {
    let probe = probe_identity(true)?;
    build_interactive_user_context(InteractiveUserObservation {
        process_session_id: Some(probe.process_session_id),
        process_sid: probe.process_sid.as_deref(),
        shell_session_id: Some(probe.shell_session_id),
        shell_sid: Some(&probe.shell_sid),
        user_profile: Some(probe.user_profile),
        user_local_app_data: Some(probe.user_local_app_data),
        user_roaming_app_data: Some(probe.user_roaming_app_data),
        shell_command_paths: probe.shell_command_paths,
    })
}

pub(super) fn revalidate_interactive_user_context(
    expected: &WindowsInteractiveUserContext,
) -> bool {
    let Ok(probe) = probe_identity(false) else {
        return false;
    };

    probe.shell_session_id == expected.shell_session_id()
        && probe.shell_sid == expected.canonical_sid()
        && probe.user_profile == expected.user_profile()
        && probe.user_local_app_data == expected.user_local_app_data()
        && probe.user_roaming_app_data == expected.user_roaming_app_data()
}

pub(super) fn runtime_privilege_status(
    context: Option<&WindowsInteractiveUserContext>,
) -> RuntimePrivilegeStatus {
    let (elevated, local_administrator, process_sid) =
        process_privilege_facts().unwrap_or((false, false, None));
    let interactive_user_match = match (process_sid.as_deref(), context) {
        (Some(process_sid), Some(context)) if process_sid == context.canonical_sid() => {
            InteractiveUserMatch::Match
        }
        (Some(_), Some(_)) => InteractiveUserMatch::Mismatch,
        _ => InteractiveUserMatch::Unavailable,
    };

    RuntimePrivilegeStatus {
        platform: RuntimePrivilegePlatform::Windows,
        supported: process_sid.is_some(),
        elevated,
        local_administrator,
        interactive_user_match,
    }
}

fn probe_identity(
    capture_shell_environment: bool,
) -> Result<IdentityProbe, WindowsStartupErrorCode> {
    let process_token = current_process_token()?;
    let process_session_id = token_session_id(process_token.get())?;
    // Process identity is diagnostic only. A failure to render Bob's SID must
    // never select another user or invalidate an otherwise proven Alice Shell
    // context.
    let process_sid = token_user_sid(process_token.get()).ok();

    let shell_token = shell_process_token()?;
    let shell_session_id = token_session_id(shell_token.get())?;
    let shell_sid = token_user_sid(shell_token.get())?;
    if !is_canonical_sid(&shell_sid) {
        return Err(WindowsStartupErrorCode::InteractiveUserUnavailable);
    }

    let user_profile = known_folder_for_token(
        Some(shell_token.get()),
        &FOLDERID_Profile,
        WindowsStartupErrorCode::InteractiveUserProfileUnavailable,
    )?;
    let user_local_app_data = known_folder_for_token(
        Some(shell_token.get()),
        &FOLDERID_LocalAppData,
        WindowsStartupErrorCode::InteractiveUserLocalAppDataUnavailable,
    )?;
    let user_roaming_app_data = known_folder_for_token(
        Some(shell_token.get()),
        &FOLDERID_RoamingAppData,
        WindowsStartupErrorCode::InteractiveUserRoamingAppDataUnavailable,
    )?;
    let shell_command_paths = if capture_shell_environment {
        environment_path_for_token(shell_token.get())?
    } else {
        Vec::new()
    };

    Ok(IdentityProbe {
        process_session_id,
        process_sid,
        shell_session_id,
        shell_sid,
        user_profile,
        user_local_app_data,
        user_roaming_app_data,
        shell_command_paths,
    })
}

struct OwnedEnvironmentBlock(*mut core::ffi::c_void);

impl Drop for OwnedEnvironmentBlock {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                let _ = DestroyEnvironmentBlock(self.0);
            }
        }
    }
}

fn environment_path_for_token(token: HANDLE) -> Result<Vec<PathBuf>, WindowsStartupErrorCode> {
    let mut raw = std::ptr::null_mut();
    if unsafe { CreateEnvironmentBlock(&mut raw, Some(token), false) }.is_err() || raw.is_null() {
        return Err(WindowsStartupErrorCode::InteractiveUserEnvironmentUnavailable);
    }
    let block = OwnedEnvironmentBlock(raw);
    let base = block.0.cast::<u16>();
    let mut offset = 0_usize;
    const MAX_ENVIRONMENT_CHARS: usize = 1024 * 1024;

    while offset < MAX_ENVIRONMENT_CHARS {
        let mut length = 0_usize;
        unsafe {
            while offset + length < MAX_ENVIRONMENT_CHARS && *base.add(offset + length) != 0 {
                length += 1;
            }
        }
        if length == 0 || offset + length >= MAX_ENVIRONMENT_CHARS {
            break;
        }
        let entry =
            OsString::from_wide(unsafe { std::slice::from_raw_parts(base.add(offset), length) });
        let entry = entry.to_string_lossy();
        if let Some((name, value)) = entry.split_once('=') {
            if name.eq_ignore_ascii_case("PATH") {
                let paths = super::parse_windows_command_path(value);
                return if paths.is_empty() {
                    Err(WindowsStartupErrorCode::InteractiveUserEnvironmentUnavailable)
                } else {
                    Ok(paths)
                };
            }
        }
        offset += length + 1;
    }

    Err(WindowsStartupErrorCode::InteractiveUserEnvironmentUnavailable)
}

pub(super) fn is_local_fixed_drive_path(path: &std::path::Path) -> bool {
    let Some(value) = path.to_str() else {
        return false;
    };
    let bytes = value.as_bytes();
    if bytes.len() < 3
        || !bytes[0].is_ascii_alphabetic()
        || bytes[1] != b':'
        || !matches!(bytes[2], b'\\' | b'/')
    {
        return false;
    }

    let root = [bytes[0] as u16, b':' as u16, b'\\' as u16, 0];
    unsafe { GetDriveTypeW(PCWSTR(root.as_ptr())) == DRIVE_FIXED }
}

fn current_process_token() -> Result<OwnedHandle, WindowsStartupErrorCode> {
    let mut token = HANDLE::default();
    unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) }
        .map_err(|_| WindowsStartupErrorCode::InteractiveUserUnavailable)?;
    if token.is_invalid() {
        return Err(WindowsStartupErrorCode::InteractiveUserUnavailable);
    }
    Ok(OwnedHandle(token))
}

fn shell_process_token() -> Result<OwnedHandle, WindowsStartupErrorCode> {
    let shell_window = unsafe { GetShellWindow() };
    if shell_window == HWND::default() {
        return Err(WindowsStartupErrorCode::InteractiveUserUnavailable);
    }

    let mut shell_pid = 0_u32;
    unsafe { GetWindowThreadProcessId(shell_window, Some(&mut shell_pid)) };
    if shell_pid == 0 {
        return Err(WindowsStartupErrorCode::InteractiveUserUnavailable);
    }

    let shell_process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, shell_pid) }
        .map_err(|_| WindowsStartupErrorCode::InteractiveUserUnavailable)?;
    if shell_process.is_invalid() {
        return Err(WindowsStartupErrorCode::InteractiveUserUnavailable);
    }
    let shell_process = OwnedHandle(shell_process);

    let mut shell_token = HANDLE::default();
    // SHGetKnownFolderPath requires TOKEN_QUERY | TOKEN_IMPERSONATE, while
    // CreateEnvironmentBlock additionally requires TOKEN_DUPLICATE. Explorer's
    // profile hive is already mounted, so no ambient process-user lookup or
    // profile loading is needed.
    unsafe {
        OpenProcessToken(
            shell_process.get(),
            TOKEN_QUERY | TOKEN_DUPLICATE | TOKEN_IMPERSONATE,
            &mut shell_token,
        )
    }
    .map_err(|_| WindowsStartupErrorCode::InteractiveUserUnavailable)?;
    if shell_token.is_invalid() {
        return Err(WindowsStartupErrorCode::InteractiveUserUnavailable);
    }
    Ok(OwnedHandle(shell_token))
}

fn token_session_id(token: HANDLE) -> Result<u32, WindowsStartupErrorCode> {
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
    .map_err(|_| WindowsStartupErrorCode::InteractiveUserUnavailable)?;
    if returned < std::mem::size_of::<u32>() as u32 {
        return Err(WindowsStartupErrorCode::InteractiveUserUnavailable);
    }
    Ok(session_id)
}

fn token_user_sid(token: HANDLE) -> Result<String, WindowsStartupErrorCode> {
    let mut required = 0_u32;
    let _ = unsafe { GetTokenInformation(token, TokenUser, None, 0, &mut required) };
    if required == 0 {
        return Err(WindowsStartupErrorCode::InteractiveUserUnavailable);
    }

    let word = std::mem::size_of::<usize>();
    let words = (required as usize).div_ceil(word);
    let mut aligned = vec![0_usize; words];
    unsafe {
        GetTokenInformation(
            token,
            TokenUser,
            Some(aligned.as_mut_ptr().cast()),
            required,
            &mut required,
        )
    }
    .map_err(|_| WindowsStartupErrorCode::InteractiveUserUnavailable)?;

    let token_user = unsafe { &*aligned.as_ptr().cast::<TOKEN_USER>() };
    sid_to_string(token_user.User.Sid)
}

fn sid_to_string(sid: PSID) -> Result<String, WindowsStartupErrorCode> {
    let mut string_sid = PWSTR::null();
    unsafe { ConvertSidToStringSidW(sid, &mut string_sid) }
        .map_err(|_| WindowsStartupErrorCode::InteractiveUserUnavailable)?;
    if string_sid.is_null() {
        return Err(WindowsStartupErrorCode::InteractiveUserUnavailable);
    }

    let rendered = unsafe { PCWSTR(string_sid.0).to_string() }
        .map_err(|_| WindowsStartupErrorCode::InteractiveUserUnavailable);
    unsafe {
        let _ = windows::Win32::Foundation::LocalFree(Some(windows::Win32::Foundation::HLOCAL(
            string_sid.0.cast(),
        )));
    }
    rendered
}

fn known_folder_for_token(
    token: Option<HANDLE>,
    folder: *const windows::core::GUID,
    error: WindowsStartupErrorCode,
) -> Result<PathBuf, WindowsStartupErrorCode> {
    let path =
        unsafe { SHGetKnownFolderPath(folder, KNOWN_FOLDER_FLAG(0), token) }.map_err(|_| error)?;
    if path.is_null() {
        return Err(error);
    }

    let mut len = 0_usize;
    unsafe {
        while *path.0.add(len) != 0 {
            len += 1;
        }
    }
    let value = PathBuf::from(OsString::from_wide(unsafe {
        std::slice::from_raw_parts(path.0, len)
    }));
    unsafe { CoTaskMemFree(Some(path.0.cast())) };
    if !value.is_absolute() || value.as_os_str().is_empty() {
        return Err(error);
    }
    Ok(value)
}

fn system_directory() -> Option<PathBuf> {
    use windows::Win32::System::SystemInformation::GetSystemDirectoryW;

    let mut buffer = vec![0_u16; 32_768];
    let length = unsafe { GetSystemDirectoryW(Some(&mut buffer)) } as usize;
    if length > 0 && length < buffer.len() {
        return Some(PathBuf::from(OsString::from_wide(&buffer[..length])));
    }
    None
}

pub(super) fn system_executable_path(filename: &str) -> Option<PathBuf> {
    if filename.is_empty()
        || matches!(filename, "." | "..")
        || !filename.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_')
        })
    {
        return None;
    }
    Some(system_directory()?.join(filename))
}

pub(super) fn system_command_directories() -> Vec<PathBuf> {
    let mut directories = Vec::new();
    if let Some(system) = system_directory() {
        directories.push(system);
    }

    if let Ok(program_files) = known_folder_for_token(
        None,
        &FOLDERID_ProgramFiles,
        WindowsStartupErrorCode::InteractiveUserUnavailable,
    ) {
        directories.push(program_files.join("nodejs"));
    }
    directories
}

/// Machine Program Files locations. These are OS known folders, not Alice
/// profile paths, so the query uses the process token (`None`).
pub(super) fn machine_program_files_directories() -> Vec<PathBuf> {
    let mut directories = Vec::new();
    for folder in [&FOLDERID_ProgramFiles, &FOLDERID_ProgramFilesX86] {
        if let Ok(path) = known_folder_for_token(
            None,
            folder,
            WindowsStartupErrorCode::InteractiveUserUnavailable,
        ) {
            directories.push(path);
        }
    }
    directories
}

fn process_privilege_facts() -> Option<(bool, bool, Option<String>)> {
    let token = current_process_token().ok()?;
    let process_sid = token_user_sid(token.get()).ok();

    let mut elevation = TOKEN_ELEVATION::default();
    let mut returned = 0_u32;
    let elevated = unsafe {
        GetTokenInformation(
            token.get(),
            TokenElevation,
            Some((&mut elevation as *mut TOKEN_ELEVATION).cast()),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut returned,
        )
    }
    .is_ok()
        && returned >= std::mem::size_of::<TOKEN_ELEVATION>() as u32
        && elevation.TokenIsElevated != 0;

    let mut administrators =
        vec![0_usize; (SECURITY_MAX_SID_SIZE as usize).div_ceil(std::mem::size_of::<usize>(),)];
    let mut administrators_len = SECURITY_MAX_SID_SIZE;
    let administrators_sid = PSID(administrators.as_mut_ptr().cast());
    let local_administrator = unsafe {
        CreateWellKnownSid(
            WinBuiltinAdministratorsSid,
            None,
            Some(administrators_sid),
            &mut administrators_len,
        )
    }
    .is_ok()
        && {
            let mut member = BOOL::default();
            unsafe { CheckTokenMembership(Some(token.get()), administrators_sid, &mut member) }
                .is_ok()
                && member.as_bool()
        };

    Some((elevated, local_administrator, process_sid))
}
