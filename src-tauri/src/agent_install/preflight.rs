//! Download-before checks for an inventory-bound lifecycle action. No job or
//! installer is started here; execution repeats the checks at admission.

use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use serde::Serialize;

use super::{
    inventory::{FreshDestinationCapability, ValidatedActionTarget},
    types::{AgentActionId, AgentReasonCode, AgentSurface, StartAgentActionRequest},
};
use crate::{
    codex_desktop::{
        temp::JobTempRoot,
        verify::{required_free_space, DiskSpaceProbe},
    },
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
    pub download_url: Option<String>,
    pub target_label: String,
    pub available_bytes: u64,
    pub required_bytes: Option<u64>,
    pub artifact_size_bytes: Option<u64>,
    pub space_budget_basis: SpaceBudgetBasis,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SpaceBudgetBasis {
    SourceSize,
    DownloadLimit,
    PackageReserve,
    CliUnknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct StorageBudget {
    pub(super) artifact_size_bytes: Option<u64>,
    pub(super) required_bytes: Option<u64>,
    pub(super) basis: SpaceBudgetBasis,
}

pub(super) fn storage_budget(
    surface: AgentSurface,
    runtime: InstallRuntime,
    artifact_size_bytes: Option<u64>,
) -> Result<StorageBudget, AgentReasonCode> {
    let limit = super::fetch::MAX_STREAMED_ARTIFACT_BYTES;
    match (surface, runtime, artifact_size_bytes) {
        (AgentSurface::Desktop, _, Some(size)) => {
            if size == 0 || size > limit {
                return Err(AgentReasonCode::SourceNotVerified);
            }
            let required_bytes =
                required_free_space(size).map_err(|_| AgentReasonCode::SourceNotVerified)?;
            Ok(StorageBudget {
                artifact_size_bytes: Some(size),
                required_bytes: Some(required_bytes),
                basis: SpaceBudgetBasis::SourceSize,
            })
        }
        (AgentSurface::Desktop, _, None) => {
            let required_bytes =
                required_free_space(limit).map_err(|_| AgentReasonCode::SourceNotVerified)?;
            Ok(StorageBudget {
                artifact_size_bytes: None,
                required_bytes: Some(required_bytes),
                basis: SpaceBudgetBasis::DownloadLimit,
            })
        }
        (AgentSurface::Cli, InstallRuntime::NodeNpm, Some(size)) => {
            if size == 0 || size > limit {
                return Err(AgentReasonCode::SourceNotVerified);
            }
            let required_bytes =
                required_free_space(size).map_err(|_| AgentReasonCode::SourceNotVerified)?;
            Ok(StorageBudget {
                artifact_size_bytes: Some(size),
                required_bytes: Some(required_bytes),
                basis: SpaceBudgetBasis::PackageReserve,
            })
        }
        (AgentSurface::Cli, InstallRuntime::ExistingCli, None) => {
            Err(AgentReasonCode::OfficialPageOnly)
        }
        _ => Err(AgentReasonCode::SourceNotVerified),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PreparedPlanPayload {
    Desktop,
    CliNpm(crate::services::tooling::grok_npm::GrokNpmManifest),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PreparedInstallTarget {
    pub(super) request: StartAgentActionRequest,
    pub(super) paths: Vec<PathBuf>,
    pub(super) runtime: InstallRuntime,
    pub(super) execution: InstallExecution,
    pub(super) target_label: String,
    pub(super) download_url: Option<String>,
    pub(super) storage_budget: StorageBudget,
    pub(super) plan: PreparedPlanPayload,
    pub(super) npm_target: Option<fyagent_user_helper::NpmTargetBinding>,
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
) -> Result<PreparedInstallTarget, AgentReasonCode> {
    let (summary, target) = inspect_preflight(request, state).await?;
    state.agent_installation_inventory.consume_preflight(
        summary
            .request
            .inventory_id
            .as_deref()
            .ok_or(AgentReasonCode::InventoryExpired)?,
        &target,
    )?;
    Ok(target)
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
    let mut npm_target = None;
    let (
        paths,
        target_label,
        execution,
        runtime,
        version_or_channel,
        download_url,
        artifact_size_bytes,
        plan,
    ) = match surface {
        AgentSurface::Desktop => {
            let source = super::desktop::resolve_desktop_source(request.agent_id)
                .await
                .map_err(super::desktop::source_reason)?;
            if request.expected_release_id.as_deref() != Some(&source.release_id) {
                return Err(AgentReasonCode::RefreshRequired);
            }
            let (paths, target_label, execution, runtime) = desktop_target_paths(&target)?;
            (
                paths,
                target_label,
                execution,
                runtime,
                source
                    .display_version
                    .unwrap_or_else(|| "官方最新渠道".to_string()),
                Some(source.download_url.to_string()),
                source.artifact_size_bytes,
                PreparedPlanPayload::Desktop,
            )
        }
        AgentSurface::Cli => {
            let checked =
                crate::services::tooling::preflight_cli_lifecycle(request.agent_id, request.action)
                    .await?;
            let (runtime, version_or_channel, artifact_size_bytes, plan, checked) = if checked
                .requires_npm
            {
                let tool = match request.agent_id {
                    crate::services::external_agents::AgentCatalogId::ClaudeCode => {
                        fyagent_user_helper::grok_npm::OfficialNpmTool::Claude
                    }
                    crate::services::external_agents::AgentCatalogId::GrokBuild => {
                        fyagent_user_helper::grok_npm::OfficialNpmTool::Grok
                    }
                    _ => return Err(AgentReasonCode::ActionNotSupported),
                };
                let manifest = crate::services::tooling::grok_npm::resolve_published_manifest(tool)
                    .await
                    .map_err(|_| AgentReasonCode::SourceNotVerified)?;
                let total_size = manifest.total_unpacked_size();
                let checked = crate::services::tooling::bind_cli_npm_preflight(
                    request.agent_id,
                    request.action,
                    &manifest,
                )
                .await?;
                (
                    InstallRuntime::NodeNpm,
                    manifest.version().to_string(),
                    Some(total_size),
                    PreparedPlanPayload::CliNpm(manifest),
                    checked,
                )
            } else {
                return Err(AgentReasonCode::OfficialPageOnly);
            };
            npm_target = checked.npm_target.clone();
            (
                checked.paths,
                checked.location_label,
                InstallExecution::CurrentUser,
                runtime,
                version_or_channel,
                None,
                artifact_size_bytes,
                plan,
            )
        }
    };
    let budget = storage_budget(surface, runtime, artifact_size_bytes)?;
    let binding = PreparedInstallTarget {
        request: request.clone(),
        paths: paths.clone(),
        runtime,
        execution,
        target_label: target_label.clone(),
        download_url: download_url.clone(),
        storage_budget: budget,
        plan,
        npm_target,
    };
    let available_bytes =
        tokio::task::spawn_blocking(move || check_storage(&paths, budget.required_bytes))
            .await
            .map_err(|_| AgentReasonCode::InstallerArtifactUnavailable)??;
    Ok((
        AgentInstallPreflightDto {
            contract_version: 1,
            request,
            platform: platform.as_str().to_string(),
            architecture: architecture.as_str().to_string(),
            version_or_channel,
            download_url,
            target_label,
            available_bytes,
            required_bytes: budget.required_bytes,
            artifact_size_bytes: budget.artifact_size_bytes,
            space_budget_basis: budget.basis,
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
        use super::{inventory::InstallationTargetCapability, types::InstallationScope};
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
        || is_displayable_dos_destination(path)
    {
        path.to_string_lossy().into_owned()
    } else {
        "当前安装工具管理的位置".to_string()
    }
}

fn is_displayable_dos_destination(path: &Path) -> bool {
    let text = path.to_string_lossy();
    let bytes = text.as_bytes();
    bytes.len() >= 3
        && bytes.len() <= 200
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/')
        && !text.chars().any(char::is_control)
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
fn check_storage(paths: &[PathBuf], required_bytes: Option<u64>) -> Result<u64, AgentReasonCode> {
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
            required_bytes,
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
    required_bytes: Option<u64>,
) -> Result<u64, AgentReasonCode> {
    if required_bytes == Some(0) {
        return Err(AgentReasonCode::SourceNotVerified);
    }
    let mut available = None;
    let mut seen_volumes = HashSet::new();
    for path in paths {
        let ancestor = existing_directory(path)?;
        let volume = probe
            .volume_key(&ancestor)
            .map_err(|_| AgentReasonCode::DiskSpaceUnavailable)?;
        // One capacity read per physical volume, including shared temp/target.
        if !seen_volumes.insert(volume.clone()) {
            continue;
        }
        let bytes = probe
            .available_bytes(&volume)
            .map_err(|_| AgentReasonCode::DiskSpaceUnavailable)?;
        if bytes == 0 || required_bytes.is_some_and(|required| bytes < required) {
            return Err(AgentReasonCode::InsufficientDiskSpace);
        }
        available = Some(available.map_or(bytes, |previous: u64| previous.min(bytes)));
    }
    available.ok_or(AgentReasonCode::DiskSpaceUnavailable)
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn check_storage(_paths: &[PathBuf], _required_bytes: Option<u64>) -> Result<u64, AgentReasonCode> {
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
    #[serial_test::serial]
    fn display_user_path_shows_ordinary_dos_destinations() {
        struct RestoreHome(Option<std::ffi::OsString>);
        impl Drop for RestoreHome {
            fn drop(&mut self) {
                match self.0.take() {
                    Some(value) => std::env::set_var("FYAGENT_TEST_HOME", value),
                    None => std::env::remove_var("FYAGENT_TEST_HOME"),
                }
            }
        }
        let home = tempfile::tempdir().unwrap();
        let _restore = RestoreHome(std::env::var_os("FYAGENT_TEST_HOME"));
        std::env::set_var("FYAGENT_TEST_HOME", home.path());
        assert_eq!(
            display_user_path(Path::new(r"D:\npm-prefix")),
            r"D:\npm-prefix"
        );
        assert_eq!(display_user_path(&home.path().join("bin")), "~/bin");
        assert_eq!(
            display_user_path(Path::new("/tmp/not-a-known-location")),
            "当前安装工具管理的位置"
        );
    }

    #[test]
    fn disk_preflight_distinguishes_empty_unavailable_and_available_capacity() {
        let root = tempfile::tempdir().unwrap();
        for available in [0, 1024, 4095] {
            assert_eq!(
                minimum_available(&Probe(Some(available)), [root.path()], Some(4096)),
                Err(AgentReasonCode::InsufficientDiskSpace)
            );
        }
        assert_eq!(
            minimum_available(&Probe(None), [root.path()], Some(4096)),
            Err(AgentReasonCode::DiskSpaceUnavailable)
        );
        assert_eq!(
            minimum_available(&Probe(Some(4096)), [root.path()], Some(4096)),
            Ok(4096)
        );
        std::fs::write(root.path().join("file"), b"fixture").unwrap();
        assert_eq!(
            minimum_available(
                &Probe(Some(4096)),
                [root.path().join("file").as_path()],
                Some(4096)
            ),
            Err(AgentReasonCode::PermissionDenied)
        );
    }

    #[test]
    fn storage_budget_uses_exact_hint_or_explicit_conservative_estimate() {
        let known = storage_budget(
            AgentSurface::Desktop,
            InstallRuntime::NativeInstaller,
            Some(4096),
        )
        .unwrap();
        assert_eq!(known.required_bytes, Some(12288));
        assert_eq!(known.basis, SpaceBudgetBasis::SourceSize);
        let unknown =
            storage_budget(AgentSurface::Desktop, InstallRuntime::NativeInstaller, None).unwrap();
        assert_eq!(unknown.required_bytes, Some(6 * 1024 * 1024 * 1024));
        assert_eq!(unknown.artifact_size_bytes, None);
        assert_eq!(unknown.basis, SpaceBudgetBasis::DownloadLimit);
        let cli_npm =
            storage_budget(AgentSurface::Cli, InstallRuntime::NodeNpm, Some(1024)).unwrap();
        assert_eq!(cli_npm.required_bytes, Some(3072));
        assert_eq!(cli_npm.artifact_size_bytes, Some(1024));
        assert_eq!(cli_npm.basis, SpaceBudgetBasis::PackageReserve);
        let cli_existing = storage_budget(AgentSurface::Cli, InstallRuntime::ExistingCli, None);
        assert_eq!(cli_existing, Err(AgentReasonCode::OfficialPageOnly));
        for size in [
            0,
            super::super::fetch::MAX_STREAMED_ARTIFACT_BYTES + 1,
            u64::MAX,
        ] {
            assert_eq!(
                storage_budget(
                    AgentSurface::Desktop,
                    InstallRuntime::NativeInstaller,
                    Some(size)
                ),
                Err(AgentReasonCode::SourceNotVerified)
            );
            assert_eq!(
                storage_budget(AgentSurface::Cli, InstallRuntime::NodeNpm, Some(size)),
                Err(AgentReasonCode::SourceNotVerified)
            );
        }
    }

    #[test]
    fn every_volume_must_pass_and_rechecking_observes_capacity_loss() {
        use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
        struct Volumes {
            target: AtomicU64,
            reads: AtomicUsize,
        }
        impl DiskSpaceProbe for Volumes {
            fn volume_key(&self, path: &Path) -> Result<VolumeKey, DiskSpaceProbeError> {
                VolumeKey::new(if path.ends_with("target") {
                    "target"
                } else {
                    "temp"
                })
            }
            fn available_bytes(&self, volume: &VolumeKey) -> Result<u64, DiskSpaceProbeError> {
                self.reads.fetch_add(1, Ordering::Relaxed);
                Ok(if *volume == VolumeKey::new("target")? {
                    self.target.load(Ordering::Relaxed)
                } else {
                    8192
                })
            }
        }
        let root = tempfile::tempdir().unwrap();
        let temp = root.path().join("temp");
        let target = root.path().join("target");
        std::fs::create_dir(&temp).unwrap();
        std::fs::create_dir(&target).unwrap();
        let probe = Volumes {
            target: AtomicU64::new(4096),
            reads: AtomicUsize::new(0),
        };
        assert_eq!(
            minimum_available(
                &probe,
                [temp.as_path(), temp.as_path(), target.as_path()],
                Some(4096)
            ),
            Ok(4096)
        );
        assert_eq!(probe.reads.load(Ordering::Relaxed), 2);
        probe.target.store(4095, Ordering::Relaxed);
        // inspect_preflight runs this same check again before confirmation consumes
        // the prepared target; earlier success cannot authorize diminished space.
        assert_eq!(
            minimum_available(&probe, [temp.as_path(), target.as_path()], Some(4096)),
            Err(AgentReasonCode::InsufficientDiskSpace)
        );
        assert_eq!(
            minimum_available(&probe, [], Some(4096)),
            Err(AgentReasonCode::DiskSpaceUnavailable)
        );
    }
}
