//! Frozen Windows Shell-user authority and untrusted activation boundaries.
//!
//! Windows release processes are elevated, so process-scoped directory and
//! registry APIs identify the administrator that approved UAC instead of the
//! Explorer user who launched FyAgent.  The native adapter resolves Explorer's
//! token once, before Tauri or any user data is initialized, and this module
//! exposes only immutable projections of that result.

#![cfg_attr(target_os = "macos", allow(dead_code))]

use serde::Serialize;
use std::{
    path::{Path, PathBuf},
    sync::OnceLock,
};

#[cfg(target_os = "windows")]
mod native;
mod registry;

#[cfg(target_os = "windows")]
pub(crate) use registry::{
    create_or_open_shell_user_environment_update, open_shell_user_environment_read,
    open_shell_user_environment_update, open_shell_user_run_update,
};

pub(crate) const MAX_SINGLE_INSTANCE_ARGUMENTS: usize = 8;
pub(crate) const MAX_SINGLE_INSTANCE_ARGUMENT_BYTES: usize = 64 * 1024;
pub(crate) const MAX_SINGLE_INSTANCE_JSON_BYTES: usize = 73_712;

/// Only safe process telemetry is exposed to the renderer.  The Shell SID and
/// paths deliberately remain crate-private.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimePrivilegeStatus {
    pub platform: RuntimePrivilegePlatform,
    pub supported: bool,
    pub elevated: bool,
    pub local_administrator: bool,
    pub interactive_user_match: InteractiveUserMatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RuntimePrivilegePlatform {
    Windows,
    Macos,
    #[allow(dead_code)]
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum InteractiveUserMatch {
    Match,
    Mismatch,
    Unavailable,
}

impl RuntimePrivilegeStatus {
    #[cfg(target_os = "macos")]
    const fn macos() -> Self {
        Self {
            platform: RuntimePrivilegePlatform::Macos,
            supported: false,
            elevated: false,
            local_administrator: false,
            interactive_user_match: InteractiveUserMatch::Unavailable,
        }
    }
}

#[cfg(test)]
mod platform_contract_tests {
    use super::RuntimePrivilegePlatform;

    #[test]
    fn privilege_platform_serialization_is_an_explicit_allowlist() {
        assert_eq!(
            serde_json::to_string(&RuntimePrivilegePlatform::Windows).unwrap(),
            "\"windows\""
        );
        assert_eq!(
            serde_json::to_string(&RuntimePrivilegePlatform::Macos).unwrap(),
            "\"macos\""
        );
        assert_eq!(
            serde_json::to_string(&RuntimePrivilegePlatform::Unknown).unwrap(),
            "\"unknown\""
        );
    }
}

/// Stable pre-logger failures contain no account, SID, path, token, or native
/// error details.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowsStartupErrorCode {
    InteractiveUserUnavailable,
    InteractiveUserProfileUnavailable,
    InteractiveUserLocalAppDataUnavailable,
    InteractiveUserRoamingAppDataUnavailable,
    InteractiveUserEnvironmentUnavailable,
}

impl WindowsStartupErrorCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InteractiveUserUnavailable => "WIN_INTERACTIVE_USER_UNAVAILABLE",
            Self::InteractiveUserProfileUnavailable => "WIN_INTERACTIVE_PROFILE_UNAVAILABLE",
            Self::InteractiveUserLocalAppDataUnavailable => {
                "WIN_INTERACTIVE_LOCAL_APP_DATA_UNAVAILABLE"
            }
            Self::InteractiveUserRoamingAppDataUnavailable => {
                "WIN_INTERACTIVE_ROAMING_APP_DATA_UNAVAILABLE"
            }
            Self::InteractiveUserEnvironmentUnavailable => {
                "WIN_INTERACTIVE_ENVIRONMENT_UNAVAILABLE"
            }
        }
    }
}

impl std::fmt::Display for WindowsStartupErrorCode {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Immutable Shell-user authority selected at process startup.
///
/// This type is intentionally not serializable.  Paths and the canonical SID
/// are capabilities for trusted host code, not renderer diagnostics.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct WindowsInteractiveUserContext {
    process_session_id: u32,
    shell_session_id: u32,
    canonical_sid: String,
    user_profile: PathBuf,
    user_local_app_data: PathBuf,
    user_roaming_app_data: PathBuf,
    shell_command_paths: Vec<PathBuf>,
}

/// Transitional name retained for the current-user Codex adapter.  The alias
/// can disappear when that adapter is moved to the Shell-user helper.
pub(crate) type InteractiveUserContext = WindowsInteractiveUserContext;

impl std::fmt::Debug for WindowsInteractiveUserContext {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("WindowsInteractiveUserContext")
            .field("process_session_id", &self.process_session_id)
            .field("shell_session_id", &self.shell_session_id)
            .field("canonical_sid", &"<redacted>")
            .field("user_profile", &"<redacted>")
            .field("user_local_app_data", &"<redacted>")
            .field("user_roaming_app_data", &"<redacted>")
            .field("shell_command_paths", &"<redacted>")
            .finish()
    }
}

impl WindowsInteractiveUserContext {
    pub(crate) const fn process_session_id(&self) -> u32 {
        self.process_session_id
    }

    pub(crate) const fn shell_session_id(&self) -> u32 {
        self.shell_session_id
    }

    pub(crate) fn canonical_sid(&self) -> &str {
        &self.canonical_sid
    }

    pub(crate) fn user_profile(&self) -> &Path {
        &self.user_profile
    }

    pub(crate) fn user_local_app_data(&self) -> &Path {
        &self.user_local_app_data
    }

    pub(crate) fn user_roaming_app_data(&self) -> &Path {
        &self.user_roaming_app_data
    }

    pub(crate) fn shell_command_paths(&self) -> &[PathBuf] {
        &self.shell_command_paths
    }

    #[cfg(test)]
    pub(crate) fn for_test(canonical_sid: &str, session_id: u32) -> Self {
        let test_profile = std::env::temp_dir().join("fyagent-test-shell-user");
        build_interactive_user_context(InteractiveUserObservation {
            process_session_id: Some(session_id),
            process_sid: Some(canonical_sid),
            shell_session_id: Some(session_id),
            shell_sid: Some(canonical_sid),
            user_profile: Some(test_profile.clone()),
            user_local_app_data: Some(test_profile.join("AppData").join("Local")),
            user_roaming_app_data: Some(test_profile.join("AppData").join("Roaming")),
            shell_command_paths: vec![PathBuf::from(r"C:\FyAgentTest\bin")],
        })
        .expect("test interactive-user context must be complete")
    }
}

#[derive(Debug)]
pub(super) struct InteractiveUserObservation<'a> {
    pub process_session_id: Option<u32>,
    pub process_sid: Option<&'a str>,
    pub shell_session_id: Option<u32>,
    pub shell_sid: Option<&'a str>,
    pub user_profile: Option<PathBuf>,
    pub user_local_app_data: Option<PathBuf>,
    pub user_roaming_app_data: Option<PathBuf>,
    pub shell_command_paths: Vec<PathBuf>,
}

fn build_interactive_user_context(
    observation: InteractiveUserObservation<'_>,
) -> Result<WindowsInteractiveUserContext, WindowsStartupErrorCode> {
    let process_session_id = observation
        .process_session_id
        .ok_or(WindowsStartupErrorCode::InteractiveUserUnavailable)?;
    let shell_session_id = observation
        .shell_session_id
        .ok_or(WindowsStartupErrorCode::InteractiveUserUnavailable)?;
    if process_session_id != shell_session_id {
        return Err(WindowsStartupErrorCode::InteractiveUserUnavailable);
    }
    let shell_sid = observation
        .shell_sid
        .filter(|sid| is_canonical_sid(sid))
        .ok_or(WindowsStartupErrorCode::InteractiveUserUnavailable)?;

    // The process SID is telemetry only.  In the supported UAC scenario the
    // elevated process can be Bob while Explorer, user state, and PackageManager
    // authority belong to Alice.
    let _process_matches_shell = observation
        .process_sid
        .filter(|sid| is_canonical_sid(sid))
        .is_some_and(|sid| sid == shell_sid);

    let user_profile = required_absolute_path(
        observation.user_profile,
        WindowsStartupErrorCode::InteractiveUserProfileUnavailable,
    )?;
    let user_local_app_data = required_absolute_path(
        observation.user_local_app_data,
        WindowsStartupErrorCode::InteractiveUserLocalAppDataUnavailable,
    )?;
    let user_roaming_app_data = required_absolute_path(
        observation.user_roaming_app_data,
        WindowsStartupErrorCode::InteractiveUserRoamingAppDataUnavailable,
    )?;
    let shell_command_paths = normalize_windows_command_paths(observation.shell_command_paths);
    #[cfg(target_os = "windows")]
    let shell_command_paths = shell_command_paths
        .into_iter()
        .filter(|path| native::is_local_fixed_drive_path(path))
        .collect::<Vec<_>>();
    if shell_command_paths.is_empty() {
        return Err(WindowsStartupErrorCode::InteractiveUserEnvironmentUnavailable);
    }

    Ok(WindowsInteractiveUserContext {
        process_session_id,
        shell_session_id,
        canonical_sid: shell_sid.to_owned(),
        user_profile,
        user_local_app_data,
        user_roaming_app_data,
        shell_command_paths,
    })
}

fn windows_path_key(path: &Path) -> String {
    path.to_string_lossy()
        .replace('/', "\\")
        .trim_end_matches('\\')
        .to_ascii_lowercase()
}

fn push_unique_windows_path(paths: &mut Vec<PathBuf>, path: PathBuf) {
    if !paths
        .iter()
        .any(|existing| windows_path_key(existing) == windows_path_key(&path))
    {
        paths.push(path);
    }
}

fn is_absolute_windows_path(value: &str) -> bool {
    if value.starts_with(r"\\?\") || value.starts_with(r"\\.\") || value.starts_with(r"\??\") {
        return false;
    }
    if value
        .chars()
        .any(|character| matches!(character, '<' | '>' | '"' | '|' | '?' | '*'))
    {
        return false;
    }

    let bytes = value.as_bytes();
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'\\' | b'/')
        && !value[2..].contains(':')
}

fn normalize_windows_command_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut normalized = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for path in paths {
        let Some(rendered) = path.to_str() else {
            continue;
        };
        let rendered = rendered.trim();
        let rendered = match (rendered.strip_prefix('"'), rendered.strip_suffix('"')) {
            (Some(without_prefix), Some(_)) if rendered.len() >= 2 => {
                &without_prefix[..without_prefix.len().saturating_sub(1)]
            }
            (None, None) => rendered,
            _ => continue,
        };
        if rendered.is_empty()
            || rendered.contains('"')
            || rendered.chars().any(char::is_control)
            || !is_absolute_windows_path(rendered)
            || rendered
                .replace('/', "\\")
                .trim_end_matches('\\')
                .to_ascii_lowercase()
                .ends_with("\\microsoft\\windowsapps")
        {
            continue;
        }
        let path = PathBuf::from(rendered);
        if seen.insert(windows_path_key(&path)) {
            normalized.push(path);
        }
    }
    normalized
}

pub(crate) fn parse_windows_command_path(value: &str) -> Vec<PathBuf> {
    normalize_windows_command_paths(value.split(';').map(PathBuf::from).collect())
}

fn required_absolute_path(
    path: Option<PathBuf>,
    error: WindowsStartupErrorCode,
) -> Result<PathBuf, WindowsStartupErrorCode> {
    path.filter(|path| path.is_absolute() && !path.as_os_str().is_empty())
        .ok_or(error)
}

pub(crate) fn is_canonical_sid(value: &str) -> bool {
    let Some(components) = value.strip_prefix("S-") else {
        return false;
    };
    let components = components.split('-').collect::<Vec<_>>();

    value.len() <= 184
        && components.len() >= 3
        && components.iter().all(|component| {
            !component.is_empty() && component.bytes().all(|byte| byte.is_ascii_digit())
        })
}

pub(crate) fn user_sid_matches_context(
    expected: &WindowsInteractiveUserContext,
    candidate_sid: Option<&str>,
) -> bool {
    matches!(
        candidate_sid,
        Some(candidate_sid)
            if is_canonical_sid(candidate_sid)
                && candidate_sid == expected.canonical_sid()
    )
}

static USER_CONTEXT: OnceLock<Result<WindowsInteractiveUserContext, WindowsStartupErrorCode>> =
    OnceLock::new();

/// Initializes the frozen Shell-user authority. `main` calls this before the
/// panic hook, Tauri construction, or user path access.
pub fn initialize_windows_user_context() -> Result<(), WindowsStartupErrorCode> {
    #[cfg(target_os = "windows")]
    let result = USER_CONTEXT.get_or_init(native::resolve_interactive_user_context);

    #[cfg(target_os = "macos")]
    let result =
        USER_CONTEXT.get_or_init(|| Err(WindowsStartupErrorCode::InteractiveUserUnavailable));

    result.as_ref().map(|_| ()).map_err(|error| *error)
}

pub(crate) fn interactive_user_context() -> Option<&'static WindowsInteractiveUserContext> {
    USER_CONTEXT.get()?.as_ref().ok()
}

pub(crate) fn require_interactive_user_context() -> &'static WindowsInteractiveUserContext {
    interactive_user_context().expect(
        "WindowsInteractiveUserContext must be initialized before any Windows user-path access",
    )
}

pub(crate) fn user_home_dir() -> PathBuf {
    require_interactive_user_context()
        .user_profile()
        .to_path_buf()
}

pub(crate) fn user_local_app_data_dir() -> PathBuf {
    require_interactive_user_context()
        .user_local_app_data()
        .to_path_buf()
}

pub(crate) fn user_roaming_app_data_dir() -> PathBuf {
    require_interactive_user_context()
        .user_roaming_app_data()
        .to_path_buf()
}

pub(crate) fn shell_command_search_paths() -> Vec<PathBuf> {
    require_interactive_user_context()
        .shell_command_paths()
        .to_vec()
}

#[cfg(target_os = "windows")]
fn shell_command_path_value_for_context(
    context: &WindowsInteractiveUserContext,
    primary: Option<&Path>,
) -> Option<std::ffi::OsString> {
    let mut paths = Vec::new();
    if let Some(primary) = primary {
        for primary in normalize_windows_command_paths(vec![primary.to_path_buf()])
            .into_iter()
            .filter(|path| native::is_local_fixed_drive_path(path))
        {
            push_unique_windows_path(&mut paths, primary);
        }
    }
    for path in context.shell_command_paths() {
        push_unique_windows_path(&mut paths, path.clone());
    }
    std::env::join_paths(paths).ok()
}

#[cfg(target_os = "windows")]
pub(crate) fn is_local_command_path(path: &Path) -> bool {
    normalize_windows_command_paths(vec![path.to_path_buf()])
        .into_iter()
        .any(|path| native::is_local_fixed_drive_path(&path))
}

/// Clears the elevated process environment and installs the narrow environment
/// shared by every Windows user-CLI child. All user projections and PATH come
/// from the frozen Explorer context; required system constants come from the
/// OS-resolved command processor path.
#[cfg(target_os = "windows")]
pub(crate) fn configure_shell_user_command(
    command: &mut std::process::Command,
    primary: Option<&Path>,
) -> Result<(), WindowsStartupErrorCode> {
    let context =
        interactive_user_context().ok_or(WindowsStartupErrorCode::InteractiveUserUnavailable)?;
    let path = shell_command_path_value_for_context(context, primary)
        .ok_or(WindowsStartupErrorCode::InteractiveUserEnvironmentUnavailable)?;
    let command_processor = system_command_path()
        .ok_or(WindowsStartupErrorCode::InteractiveUserEnvironmentUnavailable)?;
    let system_root = command_processor
        .parent()
        .and_then(Path::parent)
        .filter(|path| path.is_absolute())
        .ok_or(WindowsStartupErrorCode::InteractiveUserEnvironmentUnavailable)?;
    let user_temp = context.user_local_app_data().join("Temp");

    command.env_clear();
    command
        .env("PATH", path)
        .env("USERPROFILE", context.user_profile())
        .env("HOME", context.user_profile())
        .env("LOCALAPPDATA", context.user_local_app_data())
        .env("APPDATA", context.user_roaming_app_data())
        .env("TEMP", &user_temp)
        .env("TMP", &user_temp)
        .env("PATHEXT", ".COM;.EXE;.BAT;.CMD")
        .env("OS", "Windows_NT")
        .env("ComSpec", &command_processor)
        .env("SystemRoot", system_root)
        .env("WINDIR", system_root);
    Ok(())
}

pub(crate) fn tauri_user_store_path(identifier: &str, filename: &str) -> PathBuf {
    user_roaming_app_data_dir().join(identifier).join(filename)
}

pub(crate) fn tauri_window_state_path(identifier: &str) -> PathBuf {
    tauri_user_store_path(identifier, ".window-state.json")
}

pub(crate) fn webview_user_data_dir(identifier: &str) -> PathBuf {
    user_local_app_data_dir().join(identifier)
}

/// Returns frozen Alice PATH entries plus deterministic locations used only to
/// discover user-installed tools. Elevated-process PATH and user-scoped
/// process variables are excluded. Callers must use `shell_command_search_paths`
/// (or `shell_command_path_value`) when selecting the PATH default or launching
/// a discovered command.
#[cfg(target_os = "windows")]
pub(crate) fn safe_command_search_paths() -> Vec<PathBuf> {
    let home = user_home_dir();
    let local = user_local_app_data_dir();
    let roaming = user_roaming_app_data_dir();
    let mut paths = shell_command_search_paths();
    let supplemental = [
        home.join(".local").join("bin"),
        home.join(".npm-global").join("bin"),
        home.join(".npm-packages").join("bin"),
        home.join(".local").join("share").join("pnpm"),
        home.join(".volta").join("bin"),
        home.join("scoop").join("shims"),
        home.join(".bun").join("bin"),
        home.join("go").join("bin"),
        local.join("pnpm"),
        local.join("Volta").join("bin"),
        local.join("Yarn").join("bin"),
        roaming.join("npm"),
    ];
    for path in supplemental
        .into_iter()
        .filter(|path| is_local_command_path(path))
    {
        push_unique_windows_path(&mut paths, path);
    }
    for path in native::system_command_directories()
        .into_iter()
        .filter(|path| is_local_command_path(path))
    {
        push_unique_windows_path(&mut paths, path);
    }
    paths
}

#[cfg(target_os = "windows")]
pub(crate) fn system_command_path() -> Option<PathBuf> {
    native::system_executable_path("cmd.exe")
}

#[cfg(target_os = "windows")]
pub(crate) fn system_executable_path(filename: &str) -> Option<PathBuf> {
    native::system_executable_path(filename)
}

pub(crate) fn revalidate_interactive_user_context(
    expected: &WindowsInteractiveUserContext,
) -> bool {
    #[cfg(target_os = "windows")]
    {
        native::revalidate_interactive_user_context(expected)
    }

    #[cfg(target_os = "macos")]
    {
        let _ = expected;
        false
    }
}

pub fn runtime_privilege_status() -> RuntimePrivilegeStatus {
    #[cfg(target_os = "windows")]
    {
        native::runtime_privilege_status(interactive_user_context())
    }

    #[cfg(target_os = "macos")]
    {
        RuntimePrivilegeStatus::macos()
    }
}

/// Release/test manifest selection remains a compile-time fact used by the
/// elevated user-CLI boundary.  It no longer selects user identity or a
/// ProgramData runtime.
#[cfg(target_os = "windows")]
pub(crate) const fn formal_windows_build() -> bool {
    cfg!(all(target_os = "windows", fyagent_windows_release))
}

#[cfg(target_os = "macos")]
pub(crate) const fn formal_windows_build() -> bool {
    false
}

/// Validates the complete untrusted argv envelope before any deep-link,
/// lightweight-window, or focus behavior runs.
fn serialized_single_instance_envelope_size(args: &[String]) -> Option<usize> {
    #[derive(Serialize)]
    struct SingleInstanceEnvelope<'a> {
        version: u8,
        args: &'a [String],
    }

    serde_json::to_vec(&SingleInstanceEnvelope { version: 1, args })
        .ok()
        .map(|encoded| encoded.len())
}

pub(crate) fn normalize_single_instance_args(args: Vec<String>) -> Option<Vec<String>> {
    if args.len() > MAX_SINGLE_INSTANCE_ARGUMENTS {
        return None;
    }
    if args.iter().any(|argument| {
        argument.len() > MAX_SINGLE_INSTANCE_ARGUMENT_BYTES
            || argument.chars().any(char::is_control)
    }) {
        return None;
    }
    if serialized_single_instance_envelope_size(&args)? > MAX_SINGLE_INSTANCE_JSON_BYTES {
        return None;
    }
    Some(args)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALICE: &str = "S-1-5-21-100-200-300-1001";
    const BOB: &str = "S-1-5-21-100-200-300-1002";

    fn observation(process_sid: &'static str) -> InteractiveUserObservation<'static> {
        #[cfg(target_os = "windows")]
        let test_profile = PathBuf::from(r"C:\Users\Alice");
        #[cfg(target_os = "macos")]
        let test_profile = PathBuf::from("/users/alice");

        InteractiveUserObservation {
            process_session_id: Some(7),
            process_sid: Some(process_sid),
            shell_session_id: Some(7),
            shell_sid: Some(ALICE),
            user_profile: Some(test_profile.clone()),
            user_local_app_data: Some(test_profile.join("local")),
            user_roaming_app_data: Some(test_profile.join("roaming")),
            shell_command_paths: vec![PathBuf::from(r"C:\Users\Alice\bin")],
        }
    }

    #[derive(Clone, Copy)]
    enum RequiredUserPath {
        Profile,
        LocalAppData,
        RoamingAppData,
    }

    impl RequiredUserPath {
        const fn expected_error(self) -> WindowsStartupErrorCode {
            match self {
                Self::Profile => WindowsStartupErrorCode::InteractiveUserProfileUnavailable,
                Self::LocalAppData => {
                    WindowsStartupErrorCode::InteractiveUserLocalAppDataUnavailable
                }
                Self::RoamingAppData => {
                    WindowsStartupErrorCode::InteractiveUserRoamingAppDataUnavailable
                }
            }
        }

        fn set(self, observation: &mut InteractiveUserObservation<'_>, value: Option<PathBuf>) {
            match self {
                Self::Profile => observation.user_profile = value,
                Self::LocalAppData => observation.user_local_app_data = value,
                Self::RoamingAppData => observation.user_roaming_app_data = value,
            }
        }
    }

    #[test]
    fn same_user_context_uses_shell_paths() {
        let context = build_interactive_user_context(observation(ALICE)).unwrap();
        assert_eq!(context.canonical_sid(), ALICE);
        assert_eq!(
            context.user_profile(),
            observation(ALICE).user_profile.as_deref().unwrap()
        );
    }

    #[test]
    fn elevated_bob_is_allowed_while_shell_alice_remains_authority() {
        let context = build_interactive_user_context(observation(BOB)).unwrap();
        assert_eq!(context.canonical_sid(), ALICE);
        assert_eq!(context.process_session_id(), 7);
        assert_eq!(context.shell_session_id(), 7);
        assert_eq!(
            context.user_local_app_data(),
            observation(ALICE).user_local_app_data.as_deref().unwrap()
        );
    }

    #[test]
    fn unavailable_process_sid_does_not_replace_or_invalidate_shell_authority() {
        let mut unavailable_process = observation(BOB);
        unavailable_process.process_sid = None;

        let context = build_interactive_user_context(unavailable_process).unwrap();

        assert_eq!(context.canonical_sid(), ALICE);
        assert_eq!(context.process_session_id(), 7);
        assert_eq!(context.shell_session_id(), 7);
        assert_eq!(
            context.user_profile(),
            observation(ALICE).user_profile.as_deref().unwrap()
        );
    }

    #[test]
    fn missing_shell_session_or_environment_fails_closed() {
        let mut missing_process_session = observation(BOB);
        missing_process_session.process_session_id = None;
        assert_eq!(
            build_interactive_user_context(missing_process_session),
            Err(WindowsStartupErrorCode::InteractiveUserUnavailable)
        );

        let mut missing_shell_session = observation(BOB);
        missing_shell_session.shell_session_id = None;
        assert_eq!(
            build_interactive_user_context(missing_shell_session),
            Err(WindowsStartupErrorCode::InteractiveUserUnavailable)
        );

        let mut missing_shell = observation(BOB);
        missing_shell.shell_sid = None;
        assert_eq!(
            build_interactive_user_context(missing_shell),
            Err(WindowsStartupErrorCode::InteractiveUserUnavailable)
        );

        let mut different_session = observation(BOB);
        different_session.shell_session_id = Some(8);
        assert_eq!(
            build_interactive_user_context(different_session),
            Err(WindowsStartupErrorCode::InteractiveUserUnavailable)
        );

        let mut missing_environment = observation(BOB);
        missing_environment.shell_command_paths.clear();
        assert_eq!(
            build_interactive_user_context(missing_environment),
            Err(WindowsStartupErrorCode::InteractiveUserEnvironmentUnavailable)
        );
    }

    #[test]
    fn every_required_shell_user_path_rejects_missing_and_relative_values() {
        let required_paths = [
            RequiredUserPath::Profile,
            RequiredUserPath::LocalAppData,
            RequiredUserPath::RoamingAppData,
        ];

        for required_path in required_paths {
            let mut missing = observation(BOB);
            required_path.set(&mut missing, None);
            assert_eq!(
                build_interactive_user_context(missing),
                Err(required_path.expected_error())
            );

            let mut relative = observation(BOB);
            required_path.set(&mut relative, Some(PathBuf::from("relative/user/path")));
            assert_eq!(
                build_interactive_user_context(relative),
                Err(required_path.expected_error())
            );
        }
    }

    #[test]
    fn noncanonical_shell_sids_fail_closed() {
        for invalid_sid in [
            "alice",
            "s-1-5-21-100-200-300-1001",
            "S-1--5-21-100-200-300-1001",
            "S-1-5-21-100-200-300-alice",
        ] {
            let mut invalid_shell = observation(BOB);
            invalid_shell.shell_sid = Some(invalid_sid);
            assert_eq!(
                build_interactive_user_context(invalid_shell),
                Err(WindowsStartupErrorCode::InteractiveUserUnavailable),
                "invalid SID must be rejected: {invalid_sid}"
            );
        }

        let overlong_sid = format!("S-1-5-{}", "1".repeat(185));
        assert!(!is_canonical_sid(&overlong_sid));
    }

    #[test]
    fn context_debug_output_redacts_shell_authority_and_paths() {
        let context = build_interactive_user_context(observation(BOB)).unwrap();
        let debug = format!("{context:?}");
        let secrets = [
            context.canonical_sid().to_owned(),
            context.user_profile().to_string_lossy().into_owned(),
            context.user_local_app_data().to_string_lossy().into_owned(),
            context
                .user_roaming_app_data()
                .to_string_lossy()
                .into_owned(),
            context.shell_command_paths()[0]
                .to_string_lossy()
                .into_owned(),
        ];

        for secret in secrets {
            assert!(
                !debug.contains(&secret),
                "debug output leaked a Shell-user secret: {debug}"
            );
        }
        assert_eq!(debug.matches("<redacted>").count(), 5);
        assert!(debug.contains("process_session_id: 7"));
        assert!(debug.contains("shell_session_id: 7"));
    }

    #[test]
    fn startup_error_codes_have_an_exhaustive_stable_public_surface() {
        let cases = [
            (
                WindowsStartupErrorCode::InteractiveUserUnavailable,
                "WIN_INTERACTIVE_USER_UNAVAILABLE",
            ),
            (
                WindowsStartupErrorCode::InteractiveUserProfileUnavailable,
                "WIN_INTERACTIVE_PROFILE_UNAVAILABLE",
            ),
            (
                WindowsStartupErrorCode::InteractiveUserLocalAppDataUnavailable,
                "WIN_INTERACTIVE_LOCAL_APP_DATA_UNAVAILABLE",
            ),
            (
                WindowsStartupErrorCode::InteractiveUserRoamingAppDataUnavailable,
                "WIN_INTERACTIVE_ROAMING_APP_DATA_UNAVAILABLE",
            ),
            (
                WindowsStartupErrorCode::InteractiveUserEnvironmentUnavailable,
                "WIN_INTERACTIVE_ENVIRONMENT_UNAVAILABLE",
            ),
        ];

        for (code, expected) in cases {
            assert_eq!(code.as_str(), expected);
            assert_eq!(code.to_string(), expected);
        }
    }

    #[test]
    fn frozen_context_revalidation_is_read_only_and_fails_closed_for_a_synthetic_shell() {
        let context = WindowsInteractiveUserContext::for_test(ALICE, u32::MAX);
        let frozen = context.clone();

        assert!(!revalidate_interactive_user_context(&context));
        assert_eq!(context, frozen);
    }

    #[test]
    fn single_instance_args_enforce_count_item_control_and_aggregate_bounds() {
        assert!(normalize_single_instance_args(vec![
            "FyAgent.exe".to_owned(),
            "fyagent://v1/import?resource=provider".to_owned(),
        ])
        .is_some());
        assert!(normalize_single_instance_args(vec!["x".to_owned(); 9]).is_none());
        assert!(normalize_single_instance_args(vec!["x".repeat(65_537)]).is_none());
        assert!(normalize_single_instance_args(vec!["bad\nvalue".to_owned()]).is_none());
        assert!(normalize_single_instance_args(vec!["x".repeat(64 * 1024); 2]).is_none());

        let mut exact = vec![
            "x".repeat(MAX_SINGLE_INSTANCE_ARGUMENT_BYTES),
            String::new(),
        ];
        let empty_second_size = serialized_single_instance_envelope_size(&exact).unwrap();
        exact[1] = "y".repeat(MAX_SINGLE_INSTANCE_JSON_BYTES - empty_second_size);
        assert_eq!(
            serialized_single_instance_envelope_size(&exact).unwrap(),
            MAX_SINGLE_INSTANCE_JSON_BYTES
        );
        assert!(normalize_single_instance_args(exact.clone()).is_some());
        exact[1].push('y');
        assert!(normalize_single_instance_args(exact).is_none());
    }

    #[test]
    fn sid_matching_is_exact_and_canonical() {
        let context = WindowsInteractiveUserContext::for_test(ALICE, 7);
        assert!(user_sid_matches_context(&context, Some(ALICE)));
        assert!(!user_sid_matches_context(&context, Some(BOB)));
        assert!(!user_sid_matches_context(&context, Some("alice")));
    }

    #[test]
    fn windows_command_path_keeps_absolute_order_and_deduplicates_semantically() {
        let paths = parse_windows_command_path(
            r#"relative;C:\Tools;"c:/tools/";C:drive-relative;\\server\share\bin;C:\Users\Alice\AppData\Local\Microsoft\WindowsApps;D:\Vendor\bin;\\?\C:\device;"C:\broken"quote"#,
        );
        assert_eq!(
            paths,
            vec![PathBuf::from(r"C:\Tools"), PathBuf::from(r"D:\Vendor\bin"),]
        );
    }
}
