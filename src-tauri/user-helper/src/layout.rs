use std::{
    fmt,
    path::{Component, Path, PathBuf},
};

use crate::CanonicalJobId;

pub const CACHE_DIRECTORY: &str = "cache";
pub const CODEX_INSTALLER_DIRECTORY: &str = "codex-installer";
pub const INSTALLER_FILE_NAME: &str = "installer.msix";
pub const PACKAGE_BRIDGE_PART_FILE_NAME: &str = "installer.msix.part";
pub const PACKAGE_BRIDGE_ROOT_DIRECTORY: &str =
    "FyAgent.PackageBridge-{96F39D37-0F42-486F-8C86-3631C12171C5}";
pub const PACKAGE_BRIDGE_VERSION_DIRECTORY: &str = "v1";
pub const FYAGENT_MAIN_EXECUTABLE_FILE_NAME: &str = "fyagent.exe";
pub const USER_HELPER_EXECUTABLE_FILE_NAME: &str = "fyagent-user-helper.exe";
pub const USER_HELPER_PIPE_PREFIX: &str = r"\\.\pipe\LOCAL\FyAgent.UserHelper.v2.";
pub const USER_HELPER_ADMISSION_EVENT_PREFIX: &str = r"Local\FyAgent.UserHelper.Admit.v2.";
pub const USER_HELPER_CANCEL_EVENT_PREFIX: &str = r"Local\FyAgent.UserHelper.Cancel.v2.";
/// `READ_CONTROL | SYNCHRONIZE`; lets the helper wait and verify BA ownership
/// without granting it `EVENT_MODIFY_STATE`.
pub const USER_HELPER_CONTROL_EVENT_ACCESS_MASK: u32 = 0x0012_0000;
/// `FILE_GENERIC_READ | FILE_WRITE_DATA`; shared with the parent pipe DACL.
/// Named-pipe connect checks `FILE_READ_ATTRIBUTES` even when the client does
/// not request it, and `FILE_GENERIC_READ` also covers `FILE_READ_EA`.
/// `READ_CONTROL` lets the helper verify the server object's BA owner.
///
/// `FILE_GENERIC_WRITE` is intentionally not used because its append-data bit
/// aliases `FILE_CREATE_PIPE_INSTANCE` for named pipes.
pub const USER_HELPER_PIPE_CLIENT_ACCESS_MASK: u32 = 0x0012_008B;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstallLayout {
    install_root: PathBuf,
    installer_path: PathBuf,
}

impl InstallLayout {
    pub fn install_root(&self) -> &Path {
        &self.install_root
    }

    pub fn installer_path(&self) -> &Path {
        &self.installer_path
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayoutError {
    ExecutablePathNotAbsolute,
    ExecutablePathNotNormalized,
    ExecutablePathHasNoFileName,
    ExecutablePathHasNoParent,
}

impl fmt::Display for LayoutError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::ExecutablePathNotAbsolute => "helper executable path is not absolute",
            Self::ExecutablePathNotNormalized => "helper executable path is not normalized",
            Self::ExecutablePathHasNoFileName => "helper executable path has no file name",
            Self::ExecutablePathHasNoParent => "helper executable path has no installation root",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for LayoutError {}

pub fn derive_install_layout(
    current_executable: &Path,
    job_id: &CanonicalJobId,
) -> Result<InstallLayout, LayoutError> {
    if !current_executable.is_absolute() {
        return Err(LayoutError::ExecutablePathNotAbsolute);
    }
    if current_executable
        .components()
        .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
    {
        return Err(LayoutError::ExecutablePathNotNormalized);
    }
    if current_executable.file_name().is_none() {
        return Err(LayoutError::ExecutablePathHasNoFileName);
    }

    let install_root = current_executable
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .ok_or(LayoutError::ExecutablePathHasNoParent)?
        .to_path_buf();
    let installer_path = install_root
        .join(CACHE_DIRECTORY)
        .join(CODEX_INSTALLER_DIRECTORY)
        .join(job_id.as_str())
        .join(INSTALLER_FILE_NAME);

    Ok(InstallLayout {
        install_root,
        installer_path,
    })
}

pub fn pipe_name(nonce: &crate::PipeNonce) -> String {
    nonce_scoped_name(USER_HELPER_PIPE_PREFIX, nonce)
}

pub fn admission_event_name(nonce: &crate::PipeNonce) -> String {
    nonce_scoped_name(USER_HELPER_ADMISSION_EVENT_PREFIX, nonce)
}

pub fn cancel_event_name(nonce: &crate::PipeNonce) -> String {
    nonce_scoped_name(USER_HELPER_CANCEL_EVENT_PREFIX, nonce)
}

fn nonce_scoped_name(prefix: &str, nonce: &crate::PipeNonce) -> String {
    let mut name = String::with_capacity(prefix.len() + nonce.as_str().len());
    name.push_str(prefix);
    name.push_str(nonce.as_str());
    name
}

#[cfg(test)]
mod tests {
    use std::{ffi::OsStr, path::Path};

    use crate::{CanonicalJobId, PipeNonce};

    use super::*;

    const JOB_ID: &str = "123e4567-e89b-12d3-a456-426614174000";
    const NONCE: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    fn job_id() -> CanonicalJobId {
        CanonicalJobId::parse(JOB_ID).expect("canonical UUID")
    }

    fn assert_single_normal_component(value: &str) {
        assert!(!value.contains('/'));
        assert!(!value.contains('\\'));
        assert_ne!(value, ".");
        assert_ne!(value, "..");

        let mut components = Path::new(value).components();
        assert!(matches!(
            components.next(),
            Some(Component::Normal(component)) if component == OsStr::new(value)
        ));
        assert!(components.next().is_none());
    }

    #[cfg(target_os = "windows")]
    const ABSOLUTE_HELPER: &str = r"C:\opt\FyAgent\fyagent-user-helper.exe";
    #[cfg(target_os = "macos")]
    const ABSOLUTE_HELPER: &str = "/opt/FyAgent/fyagent-user-helper.exe";
    #[cfg(target_os = "windows")]
    const INSTALL_ROOT: &str = r"C:\opt\FyAgent";
    #[cfg(target_os = "macos")]
    const INSTALL_ROOT: &str = "/opt/FyAgent";
    #[cfg(target_os = "windows")]
    const ROOT: &str = r"C:\";
    #[cfg(target_os = "macos")]
    const ROOT: &str = "/";
    #[cfg(target_os = "windows")]
    const SPACED_HELPER: &str = r"C:\install root\FyAgent\fyagent-user-helper.exe";
    #[cfg(target_os = "macos")]
    const SPACED_HELPER: &str = "/opt/install root/FyAgent/fyagent-user-helper.exe";
    #[cfg(target_os = "windows")]
    const SPACED_ROOT: &str = r"C:\install root\FyAgent";
    #[cfg(target_os = "macos")]
    const SPACED_ROOT: &str = "/opt/install root/FyAgent";
    #[cfg(target_os = "windows")]
    const TRAVERSAL_HELPER: &str = r"C:\opt\FyAgent\..\other\fyagent-user-helper.exe";
    #[cfg(target_os = "macos")]
    const TRAVERSAL_HELPER: &str = "/opt/FyAgent/../other/fyagent-user-helper.exe";

    #[test]
    fn package_bridge_components_are_exact_fixed_single_names() {
        assert_eq!(
            PACKAGE_BRIDGE_ROOT_DIRECTORY,
            "FyAgent.PackageBridge-{96F39D37-0F42-486F-8C86-3631C12171C5}"
        );
        assert_eq!(PACKAGE_BRIDGE_VERSION_DIRECTORY, "v1");
        assert_eq!(PACKAGE_BRIDGE_PART_FILE_NAME, "installer.msix.part");
        assert_single_normal_component(PACKAGE_BRIDGE_ROOT_DIRECTORY);
        assert_single_normal_component(PACKAGE_BRIDGE_VERSION_DIRECTORY);
        assert_single_normal_component(INSTALLER_FILE_NAME);
        assert_single_normal_component(PACKAGE_BRIDGE_PART_FILE_NAME);
    }

    #[test]
    fn derives_only_the_fixed_direct_child_installer_path() {
        let layout = derive_install_layout(Path::new(ABSOLUTE_HELPER), &job_id())
            .expect("absolute helper path");

        assert_eq!(layout.install_root(), Path::new(INSTALL_ROOT));
        assert_eq!(
            layout.installer_path(),
            Path::new(INSTALL_ROOT)
                .join("cache")
                .join("codex-installer")
                .join(JOB_ID)
                .join("installer.msix")
        );
        assert_eq!(
            layout
                .installer_path()
                .strip_prefix(layout.install_root())
                .expect("installer must remain under its derived root")
                .components()
                .count(),
            4
        );
    }

    #[test]
    fn preserves_install_roots_with_spaces_without_using_the_working_directory() {
        let layout = derive_install_layout(Path::new(SPACED_HELPER), &job_id())
            .expect("absolute helper path");
        assert_eq!(layout.install_root(), Path::new(SPACED_ROOT));
        assert!(layout.installer_path().ends_with(INSTALLER_FILE_NAME));
    }

    #[test]
    fn rejects_relative_or_parentless_executable_paths() {
        assert_eq!(
            derive_install_layout(Path::new("fyagent-user-helper.exe"), &job_id()).unwrap_err(),
            LayoutError::ExecutablePathNotAbsolute
        );
        assert_eq!(
            derive_install_layout(Path::new(ROOT), &job_id()).unwrap_err(),
            LayoutError::ExecutablePathHasNoFileName
        );
    }

    #[test]
    fn rejects_parent_traversal_in_the_executable_path() {
        assert_eq!(
            derive_install_layout(Path::new(TRAVERSAL_HELPER), &job_id()).unwrap_err(),
            LayoutError::ExecutablePathNotNormalized
        );
    }

    #[test]
    fn pipe_name_is_exactly_the_fixed_local_prefix_and_nonce() {
        assert_eq!(
            USER_HELPER_PIPE_PREFIX,
            r"\\.\pipe\LOCAL\FyAgent.UserHelper.v2."
        );
        let nonce = PipeNonce::parse(NONCE).expect("valid nonce");
        let name = pipe_name(&nonce);
        assert_eq!(name, format!("{USER_HELPER_PIPE_PREFIX}{NONCE}"));
        assert_eq!(name.len(), USER_HELPER_PIPE_PREFIX.len() + 64);
        assert!(!name.contains(JOB_ID));
    }

    #[test]
    fn admission_and_cancel_events_are_distinct_fixed_nonce_scoped_names() {
        assert_eq!(
            USER_HELPER_ADMISSION_EVENT_PREFIX,
            r"Local\FyAgent.UserHelper.Admit.v2."
        );
        assert_eq!(
            USER_HELPER_CANCEL_EVENT_PREFIX,
            r"Local\FyAgent.UserHelper.Cancel.v2."
        );
        let nonce = PipeNonce::parse(NONCE).expect("valid nonce");
        let admission = admission_event_name(&nonce);
        let cancel = cancel_event_name(&nonce);

        assert_eq!(
            admission,
            format!("{USER_HELPER_ADMISSION_EVENT_PREFIX}{NONCE}")
        );
        assert_eq!(cancel, format!("{USER_HELPER_CANCEL_EVENT_PREFIX}{NONCE}"));
        assert_ne!(admission, cancel);
        assert!(!admission.contains(JOB_ID));
        assert!(!cancel.contains(JOB_ID));
    }

    #[test]
    fn pipe_client_access_is_generic_read_plus_write_data_without_create_instance() {
        const FILE_READ_DATA: u32 = 0x0000_0001;
        const FILE_WRITE_DATA: u32 = 0x0000_0002;
        const FILE_APPEND_DATA_OR_CREATE_PIPE_INSTANCE: u32 = 0x0000_0004;
        const FILE_READ_EA: u32 = 0x0000_0008;
        const FILE_READ_ATTRIBUTES: u32 = 0x0000_0080;
        const READ_CONTROL: u32 = 0x0002_0000;
        const SYNCHRONIZE: u32 = 0x0010_0000;

        assert_eq!(
            USER_HELPER_PIPE_CLIENT_ACCESS_MASK,
            FILE_READ_DATA
                | FILE_WRITE_DATA
                | FILE_READ_EA
                | FILE_READ_ATTRIBUTES
                | READ_CONTROL
                | SYNCHRONIZE
        );
        assert_eq!(
            USER_HELPER_PIPE_CLIENT_ACCESS_MASK & FILE_APPEND_DATA_OR_CREATE_PIPE_INSTANCE,
            0
        );
    }

    #[test]
    fn control_event_access_is_read_control_plus_synchronize_only() {
        const READ_CONTROL: u32 = 0x0002_0000;
        const SYNCHRONIZE: u32 = 0x0010_0000;
        const EVENT_MODIFY_STATE: u32 = 0x0000_0002;

        assert_eq!(
            USER_HELPER_CONTROL_EVENT_ACCESS_MASK,
            READ_CONTROL | SYNCHRONIZE
        );
        assert_eq!(
            USER_HELPER_CONTROL_EVENT_ACCESS_MASK & EVENT_MODIFY_STATE,
            0
        );
    }
}
