//! Download-before checks for an inventory-bound lifecycle action. No job or
//! installer is started here; execution repeats the checks at admission.

use std::path::{Path, PathBuf};

use serde::Serialize;

use super::{
    inventory::{FreshDestinationCapability, InstallationTargetCapability, ValidatedActionTarget},
    types::{AgentActionId, AgentReasonCode, AgentSurface, StartAgentActionRequest},
};
use crate::{
    codex_desktop::{temp::JobTempRoot, verify::DiskSpaceProbe},
    store::AppState,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentInstallPreflightDto {
    pub contract_version: u16,
    pub request: StartAgentActionRequest,
    pub platform: String,
    pub architecture: String,
    pub version_or_channel: String,
    pub target_label: String,
    pub available_bytes: u64,
    pub runtime: InstallRuntime,
    pub execution: InstallExecution,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallRuntime {
    NativeInstaller,
    NodeNpm,
    ExistingCli,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallExecution {
    CurrentUser,
    SystemAuthorization,
    VendorWizard,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PreparedInstallTarget {
    pub(super) request: StartAgentActionRequest,
    pub(super) paths: Vec<PathBuf>,
    pub(super) runtime: InstallRuntime,
    pub(super) execution: InstallExecution,
    pub(super) target_label: String,
}

pub async fn preflight_for(
    request: StartAgentActionRequest,
    state: &AppState,
) -> Result<AgentInstallPreflightDto, AgentReasonCode> {
    let (summary, target) = inspect_preflight(request, state).await?;
    state.agent_installation_inventory.record_preflight(
        summary
            .request
            .inventory_id
            .as_deref()
            .ok_or(AgentReasonCode::InventoryExpired)?,
        target,
    )?;
    Ok(summary)
}

pub(super) async fn confirm_preflight(
    request: StartAgentActionRequest,
    state: &AppState,
) -> Result<(), AgentReasonCode> {
    let (summary, target) = inspect_preflight(request, state).await?;
    state.agent_installation_inventory.consume_preflight(
        summary
            .request
            .inventory_id
            .as_deref()
            .ok_or(AgentReasonCode::InventoryExpired)?,
        &target,
    )
}

async fn inspect_preflight(
    request: StartAgentActionRequest,
    state: &AppState,
) -> Result<(AgentInstallPreflightDto, PreparedInstallTarget), AgentReasonCode> {
    let surface = super::resolve_requested_surface(request.agent_id, request.surface)?;
    if !matches!(
        request.action,
        AgentActionId::Install | AgentActionId::Update
    ) {
        return Err(AgentReasonCode::ActionNotSupported);
    }
    super::lifecycle_policy::admit_action(request.agent_id, surface, request.action)?;
    let target = super::inventory::validate_action_target(&request, state).await?;
    let (platform, architecture) =
        super::sources::current_host_target().ok_or(AgentReasonCode::PlatformUnsupported)?;
    let (version_or_channel, runtime) = match surface {
        AgentSurface::Desktop => {
            let source = super::desktop::resolve_desktop_source(request.agent_id)
                .await
                .map_err(super::desktop::source_reason)?;
            if request.expected_release_id.as_deref() != Some(&source.release_id) {
                return Err(AgentReasonCode::RefreshRequired);
            }
            (
                source
                    .display_version
                    .unwrap_or_else(|| "官方最新渠道".to_string()),
                InstallRuntime::NativeInstaller,
            )
        }
        AgentSurface::Cli => (
            "官方渠道（保留当前安装方式）".to_string(),
            InstallRuntime::NodeNpm,
        ),
    };
    let agent_id = request.agent_id;
    let action = request.action;
    let (paths, target_label, execution, runtime) = match surface {
        AgentSurface::Cli => {
            let checked =
                crate::services::tooling::preflight_cli_lifecycle(agent_id, action).await?;
            (
                checked.paths,
                checked.location_label,
                InstallExecution::CurrentUser,
                if checked.requires_npm {
                    runtime
                } else {
                    InstallRuntime::ExistingCli
                },
            )
        }
        AgentSurface::Desktop => desktop_target_paths(&target)?,
    };
    let binding = PreparedInstallTarget {
        request: request.clone(),
        paths: paths.clone(),
        runtime,
        execution,
        target_label: target_label.clone(),
    };
    let available_bytes = tokio::task::spawn_blocking(move || check_storage(&paths))
        .await
        .map_err(|_| AgentReasonCode::InstallerArtifactUnavailable)??;
    Ok((
        AgentInstallPreflightDto {
            contract_version: 1,
            request,
            platform: platform.as_str().to_string(),
            architecture: architecture.as_str().to_string(),
            version_or_channel,
            target_label,
            available_bytes,
            runtime,
            execution,
        },
        binding,
    ))
}

fn desktop_target_paths(
    target: &ValidatedActionTarget,
) -> Result<(Vec<PathBuf>, String, InstallExecution, InstallRuntime), AgentReasonCode> {
    #[cfg(target_os = "macos")]
    {
        use super::types::InstallationScope;
        let (path, label, execution) = match target {
            ValidatedActionTarget::Existing(InstallationTargetCapability::Desktop {
                path,
                scope,
                ..
            }) => {
                let parent = path
                    .parent()
                    .ok_or(AgentReasonCode::TargetChanged)?
                    .to_path_buf();
                let execution = if *scope == InstallationScope::AllUsers {
                    if !crate::macos_system_commit::production_enabled() {
                        return Err(AgentReasonCode::AuthorizationRequired);
                    }
                    InstallExecution::SystemAuthorization
                } else {
                    InstallExecution::CurrentUser
                };
                (parent, display_user_path(path), execution)
            }
            ValidatedActionTarget::Fresh(FreshDestinationCapability::MacUserApplications) => (
                super::desktop::user_applications_dir()?,
                "~/Applications".to_string(),
                InstallExecution::CurrentUser,
            ),
            ValidatedActionTarget::Fresh(FreshDestinationCapability::MacSystemApplications) => {
                if !crate::macos_system_commit::production_enabled() {
                    return Err(AgentReasonCode::AuthorizationRequired);
                }
                (
                    PathBuf::from("/Applications"),
                    "/Applications".to_string(),
                    InstallExecution::SystemAuthorization,
                )
            }
            _ => return Err(AgentReasonCode::TargetScopeUnsupported),
        };
        let ancestor = existing_directory(&path)?;
        if execution == InstallExecution::CurrentUser && !directory_writable(&ancestor) {
            return Err(AgentReasonCode::PermissionDenied);
        }
        for runtime in ["/usr/bin/hdiutil", "/usr/bin/ditto", "/usr/bin/plutil"] {
            if !Path::new(runtime).is_file() {
                return Err(AgentReasonCode::NativeProjectionUnavailable);
            }
        }
        Ok((
            vec![ancestor],
            label,
            execution,
            InstallRuntime::NativeInstaller,
        ))
    }
    #[cfg(target_os = "windows")]
    {
        let context = crate::windows_runtime::require_interactive_user_context();
        if !crate::windows_runtime::revalidate_interactive_user_context(context) {
            return Err(AgentReasonCode::InteractiveUserUnavailable);
        }
        // The official wizard chooses its final directory and owns UAC. Do not
        // pretend the elevated host's access rights describe the Shell user.
        let label = match target {
            ValidatedActionTarget::Fresh(FreshDestinationCapability::WindowsCurrentUser) => {
                "当前桌面用户的应用目录；最终位置由官方安装窗口显示"
            }
            ValidatedActionTarget::Fresh(FreshDestinationCapability::VendorInstallerChoice) => {
                "由官方安装窗口选择位置；需要时由 Windows 请求管理员授权"
            }
            _ => return Err(AgentReasonCode::TargetScopeUnsupported),
        };
        Ok((
            vec![crate::windows_runtime::user_local_app_data_dir()],
            label.to_string(),
            InstallExecution::VendorWizard,
            InstallRuntime::NativeInstaller,
        ))
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = target;
        Err(AgentReasonCode::PlatformUnsupported)
    }
}

pub(crate) fn display_user_path(path: &Path) -> String {
    let home = crate::config::get_home_dir();
    if let Ok(relative) = path.strip_prefix(home) {
        format!("~/{}", relative.to_string_lossy())
    } else if path.starts_with("/Applications")
        || path.starts_with("/opt/homebrew")
        || path.starts_with("/usr/local")
    {
        path.to_string_lossy().into_owned()
    } else {
        "当前安装工具管理的位置".to_string()
    }
}

pub(crate) fn existing_directory(path: &Path) -> Result<PathBuf, AgentReasonCode> {
    for parent in path.ancestors() {
        match std::fs::symlink_metadata(parent) {
            Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {
                return Ok(parent.to_path_buf())
            }
            Ok(_) => return Err(AgentReasonCode::PermissionDenied),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(_) => return Err(AgentReasonCode::PermissionDenied),
        }
    }
    Err(AgentReasonCode::PermissionDenied)
}

#[cfg(target_os = "macos")]
pub(crate) fn directory_writable(path: &Path) -> bool {
    use std::{ffi::CString, os::unix::ffi::OsStrExt};
    CString::new(path.as_os_str().as_bytes())
        .is_ok_and(|path| unsafe { libc::access(path.as_ptr(), libc::W_OK | libc::X_OK) == 0 })
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn check_storage(paths: &[PathBuf]) -> Result<u64, AgentReasonCode> {
    let scratch = JobTempRoot::for_current_process()
        .create_job(&uuid::Uuid::new_v4().to_string())
        .map_err(|_| AgentReasonCode::InstallerArtifactUnavailable)?;
    let result = (|| {
        scratch
            .revalidate()
            .map_err(|_| AgentReasonCode::InstallerArtifactUnavailable)?;
        #[cfg(target_os = "macos")]
        let probe = crate::codex_desktop::platform::macos::MacosDiskSpaceProbe::for_current_host();
        #[cfg(target_os = "windows")]
        let probe = crate::codex_desktop::platform::windows::SystemWindowsDiskSpaceProbe::new();
        minimum_available(
            &probe,
            std::iter::once(scratch.path()).chain(paths.iter().map(PathBuf::as_path)),
        )
    })();
    scratch
        .cleanup()
        .map_err(|_| AgentReasonCode::InstallerArtifactUnavailable)?;
    result
}

fn minimum_available<'a>(
    probe: &dyn DiskSpaceProbe,
    paths: impl IntoIterator<Item = &'a Path>,
) -> Result<u64, AgentReasonCode> {
    let mut available = None;
    for path in paths {
        let ancestor = existing_directory(path)?;
        let volume = probe
            .volume_key(&ancestor)
            .map_err(|_| AgentReasonCode::DiskSpaceUnavailable)?;
        let bytes = probe
            .available_bytes(&volume)
            .map_err(|_| AgentReasonCode::DiskSpaceUnavailable)?;
        if bytes == 0 {
            return Err(AgentReasonCode::InsufficientDiskSpace);
        }
        available = Some(available.map_or(bytes, |previous: u64| previous.min(bytes)));
    }
    available.ok_or(AgentReasonCode::DiskSpaceUnavailable)
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn check_storage(_paths: &[PathBuf]) -> Result<u64, AgentReasonCode> {
    Err(AgentReasonCode::PlatformUnsupported)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codex_desktop::verify::{DiskSpaceProbeError, VolumeKey};

    struct Probe(Option<u64>);
    impl DiskSpaceProbe for Probe {
        fn volume_key(&self, _: &Path) -> Result<VolumeKey, DiskSpaceProbeError> {
            VolumeKey::new("fixture-volume")
        }
        fn available_bytes(&self, _: &VolumeKey) -> Result<u64, DiskSpaceProbeError> {
            self.0.ok_or(DiskSpaceProbeError::Unavailable)
        }
    }

    #[test]
    fn disk_preflight_distinguishes_empty_unavailable_and_available_capacity() {
        let root = tempfile::tempdir().unwrap();
        assert_eq!(
            minimum_available(&Probe(Some(0)), [root.path()]),
            Err(AgentReasonCode::InsufficientDiskSpace)
        );
        assert_eq!(
            minimum_available(&Probe(None), [root.path()]),
            Err(AgentReasonCode::DiskSpaceUnavailable)
        );
        assert_eq!(
            minimum_available(&Probe(Some(4096)), [root.path()]),
            Ok(4096)
        );
        std::fs::write(root.path().join("file"), b"fixture").unwrap();
        assert_eq!(
            minimum_available(&Probe(Some(4096)), [root.path().join("file").as_path()]),
            Err(AgentReasonCode::PermissionDenied)
        );
    }
}
