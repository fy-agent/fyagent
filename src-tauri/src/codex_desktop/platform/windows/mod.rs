//! Windows x64 and ARM64 current-user MSIX adapter.
//!
//! The normal adapter has no install scope, no arbitrary URL/path input, and
//! no elevation capability. It accepts only core-owned `PreparedInstallPackage`
//! evidence, delegates current-user deployment to the installed unelevated
//! helper, then relies on the common service to re-query the registered package.

mod deployment;
#[cfg(target_os = "windows")]
mod helper;
#[cfg(target_os = "windows")]
mod package_bridge;

use std::{path::Path, sync::Arc, time::Duration};

use futures::future::BoxFuture;

use self::deployment::{
    deployment_error, launch_error, verify_context_evidence, WindowsPackageManager,
    WindowsPackageRecord,
};

#[cfg(test)]
use self::deployment::{
    WindowsPackageInventory, WindowsUserContextEvidence, WindowsUserOperationReceipt,
};

#[cfg(test)]
use self::deployment::WindowsNativeError;
#[cfg(target_os = "windows")]
mod runtime;
use super::{
    installed_application_has_operational_shape, CodexDesktopPlatform, PlatformInstallPlan,
    PlatformProgressSink, PreparedInstallPackage, RestartCandidateInspection,
    RestartInstallationScope, RuntimeInspection, TrustedInstallationCandidate,
    TrustedRuntimeInstance, WINDOWS_CODEX_STABLE_IDENTITY,
};
use crate::codex_desktop::{
    download::DownloadedArtifact,
    error::{InstallerError, InstallerErrorCode},
    types::{
        CpuArchitecture, DesktopPlatform, InstalledApplication, InstalledApplicationSummary,
        LaunchTarget, LocalInstallStatus, PlatformVersion, ReleaseDescriptor, UnsupportedReason,
    },
};
use crate::windows_runtime::InteractiveUserContext;

#[cfg(target_os = "windows")]
#[cfg_attr(test, allow(unused_imports))]
pub use deployment::SystemWindowsDiskSpaceProbe;
#[cfg(target_os = "windows")]
pub use deployment::SystemWindowsPackageManager;

trait WindowsVerifiedFilePin: Send {
    fn recheck(&self) -> Result<(), InstallerError>;
    fn identity(&self) -> WindowsPackageFileIdentity;
    fn expected_size(&self) -> u64;
    fn expected_sha256(&self) -> &str;
    fn duplicate_source_file(&self) -> Result<std::fs::File, InstallerError>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct WindowsPackageFileIdentity {
    volume_serial: u64,
    file_index: u64,
    size: u64,
}

trait WindowsFilePinFactory: Send + Sync {
    fn open(
        &self,
        package: &PreparedInstallPackage,
    ) -> Result<Box<dyn WindowsVerifiedFilePin>, InstallerError>;
}

trait WindowsUserHelperRunner: Send + Sync {
    fn run(
        &self,
        context: &InteractiveUserContext,
        job_id: &str,
        pin: Box<dyn WindowsVerifiedFilePin>,
        progress: PlatformProgressSink,
        deadlines: WindowsHelperDeadlines,
    ) -> Result<(), InstallerError>;
}

trait WindowsContextRevalidator: Send + Sync {
    fn is_current(&self, context: &InteractiveUserContext) -> bool;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct WindowsHelperDeadlines {
    connect: Duration,
    operation: Duration,
    terminal_close: Duration,
}

impl WindowsHelperDeadlines {
    const PRODUCTION: Self = Self {
        connect: Duration::from_secs(30),
        operation: Duration::from_secs(10 * 60),
        terminal_close: Duration::from_secs(5),
    };
}

#[derive(Clone)]
struct WindowsInstallDependencies {
    context_revalidator: Arc<dyn WindowsContextRevalidator>,
    pin_factory: Arc<dyn WindowsFilePinFactory>,
    helper_runner: Arc<dyn WindowsUserHelperRunner>,
    deadlines: WindowsHelperDeadlines,
}

/// Host facts are injected for fake-based tests. Free-space admission covers
/// both the job staging volume and the ProgramData bridge volume discovered
/// from the Windows known-folder API; no system-drive letter is guessed.
#[derive(Debug, Clone)]
pub struct WindowsHost {
    architecture: CpuArchitecture,
}

impl WindowsHost {
    pub fn new(architecture: CpuArchitecture, _os_version: &str) -> Result<Self, InstallerError> {
        Ok(Self { architecture })
    }

    #[cfg(target_os = "windows")]
    pub fn for_current_host() -> Result<Self, InstallerError> {
        let version = windows_version::OsVersion::current();
        let revision = windows_version::revision();
        let version_text = format!(
            "{}.{}.{}.{}",
            version.major, version.minor, version.build, revision
        );
        Self::new(native_host::architecture(), &version_text)
    }

    pub(crate) fn architecture(&self) -> CpuArchitecture {
        self.architecture
    }
}

/// Windows installer adapter with injectable PackageManager facts. The public
/// construction boundary is side-effect-free, so tests never query, deploy,
/// or activate a real system package. The production facade calls
/// `revalidate_interactive_user_context` before and after native operations;
/// this adapter independently verifies every returned context stamp.
pub(crate) struct WindowsPlatformAdapter {
    package_manager: Arc<dyn WindowsPackageManager>,
    user_context: Arc<InteractiveUserContext>,
    host: WindowsHost,
    install_dependencies: WindowsInstallDependencies,
}

impl WindowsPlatformAdapter {
    fn new(
        package_manager: Arc<dyn WindowsPackageManager>,
        user_context: Arc<InteractiveUserContext>,
        host: WindowsHost,
        install_dependencies: WindowsInstallDependencies,
    ) -> Self {
        Self {
            package_manager,
            user_context,
            host,
            install_dependencies,
        }
    }

    #[cfg(target_os = "windows")]
    pub(crate) fn for_current_host(
        user_context: Arc<InteractiveUserContext>,
    ) -> Result<Self, InstallerError> {
        Ok(Self::new(
            Arc::new(SystemWindowsPackageManager),
            user_context,
            WindowsHost::for_current_host()?,
            WindowsInstallDependencies {
                context_revalidator: Arc::new(helper::SystemWindowsContextRevalidator),
                pin_factory: Arc::new(helper::SystemWindowsFilePinFactory),
                helper_runner: Arc::new(helper::SystemWindowsUserHelperRunner),
                deadlines: WindowsHelperDeadlines::PRODUCTION,
            },
        ))
    }

    fn host_support_error(&self) -> Option<InstallerError> {
        match self.host.architecture() {
            CpuArchitecture::X86_64 | CpuArchitecture::Aarch64 => None,
            architecture => Some(
                InstallerError::new(InstallerErrorCode::ArchitectureUnsupported)
                    .with_context("architecture", architecture.as_str())
                    .with_diagnostic_message("Windows V1 supports x64 and ARM64 only"),
            ),
        }
    }
}

impl CodexDesktopPlatform for WindowsPlatformAdapter {
    fn platform(&self) -> Option<DesktopPlatform> {
        Some(DesktopPlatform::Windows)
    }

    fn architecture(&self) -> CpuArchitecture {
        self.host.architecture()
    }

    fn inspect_local(&self) -> BoxFuture<'_, Result<LocalInstallStatus, InstallerError>> {
        let package_manager = self.package_manager.clone();
        let user_context = self.user_context.clone();
        let host = self.host.clone();
        let host_error = self.host_support_error();
        Box::pin(async move {
            if host.architecture() != CpuArchitecture::X86_64
                && host.architecture() != CpuArchitecture::Aarch64
            {
                return Ok(LocalInstallStatus::Unsupported {
                    reason: UnsupportedReason::Architecture,
                });
            }
            if let Some(error) = host_error {
                return Err(error);
            }
            run_blocking(move || inspect_local(package_manager.as_ref(), &user_context, &host))
                .await
        })
    }

    fn inspect_restart_candidates(
        &self,
    ) -> BoxFuture<'_, Result<RestartCandidateInspection, InstallerError>> {
        let package_manager = self.package_manager.clone();
        let user_context = self.user_context.clone();
        let host = self.host.clone();
        let host_error = self.host_support_error();
        Box::pin(async move {
            if host.architecture() != CpuArchitecture::X86_64
                && host.architecture() != CpuArchitecture::Aarch64
            {
                return Ok(RestartCandidateInspection::Unsupported(
                    UnsupportedReason::Architecture,
                ));
            }
            if let Some(error) = host_error {
                return Err(error);
            }
            run_blocking(move || {
                inspect_restart_candidates(package_manager.as_ref(), &user_context, &host)
            })
            .await
        })
    }

    fn preflight<'a>(
        &'a self,
        release: &'a ReleaseDescriptor,
        temp_root: &'a Path,
    ) -> BoxFuture<'a, Result<PlatformInstallPlan, InstallerError>> {
        let host = self.host.clone();
        let release = release.clone();
        let temp_root = temp_root.to_path_buf();
        let host_error = self.host_support_error();
        Box::pin(async move {
            if let Some(error) = host_error {
                return Err(error);
            }
            run_blocking(move || preflight(&host, &release, &temp_root)).await
        })
    }

    fn prepare_install_package<'a>(
        &'a self,
        release: &'a ReleaseDescriptor,
        artifact: &'a DownloadedArtifact,
    ) -> BoxFuture<'a, Result<PreparedInstallPackage, InstallerError>> {
        let host = self.host.clone();
        let release = release.clone();
        let artifact = artifact.clone();
        let host_error = self.host_support_error();
        Box::pin(async move {
            if let Some(error) = host_error {
                return Err(error);
            }
            run_blocking(move || {
                validate_release_for_host(&host, &release)?;
                PreparedInstallPackage::from_prepared_artifact(&release, artifact)
            })
            .await
        })
    }

    fn install_current_user<'a>(
        &'a self,
        package: &'a PreparedInstallPackage,
        progress: PlatformProgressSink,
    ) -> BoxFuture<'a, Result<Option<InstalledApplication>, InstallerError>> {
        let package_manager = self.package_manager.clone();
        let install_dependencies = self.install_dependencies.clone();
        let user_context = self.user_context.clone();
        let host = self.host.clone();
        let package = package.clone();
        let host_error = self.host_support_error();
        Box::pin(async move {
            if let Some(error) = host_error {
                return Err(error);
            }
            run_blocking(move || {
                install_current_user(
                    package_manager.as_ref(),
                    &install_dependencies,
                    &user_context,
                    &host,
                    &package,
                    progress,
                )
            })
            .await
        })
    }

    fn launch<'a>(
        &'a self,
        installed: &'a InstalledApplication,
    ) -> BoxFuture<'a, Result<(), InstallerError>> {
        let package_manager = self.package_manager.clone();
        let user_context = self.user_context.clone();
        let host = self.host.clone();
        let installed = installed.clone();
        let host_error = self.host_support_error();
        Box::pin(async move {
            if let Some(error) = host_error {
                return Err(error);
            }
            run_blocking(move || launch(package_manager.as_ref(), &user_context, &host, &installed))
                .await
        })
    }

    fn inspect_runtime<'a>(
        &'a self,
        installed: &'a InstalledApplication,
    ) -> BoxFuture<'a, Result<RuntimeInspection, InstallerError>> {
        let user_context = self.user_context.clone();
        let installed = installed.clone();
        let host_error = self.host_support_error();
        Box::pin(async move {
            if let Some(error) = host_error {
                return Err(error);
            }
            run_blocking(move || runtime::inspect(&user_context, &installed)).await
        })
    }

    fn force_shutdown<'a>(
        &'a self,
        installed: &'a InstalledApplication,
        instances: &'a [TrustedRuntimeInstance],
    ) -> BoxFuture<'a, Result<(), InstallerError>> {
        let user_context = self.user_context.clone();
        let installed = installed.clone();
        let instances = instances.to_vec();
        let host_error = self.host_support_error();
        Box::pin(async move {
            if let Some(error) = host_error {
                return Err(error);
            }
            run_blocking(move || runtime::force_shutdown(&user_context, &installed, &instances))
                .await
        })
    }

    fn is_runtime_instance_running<'a>(
        &'a self,
        installed: &'a InstalledApplication,
        instances: &'a [TrustedRuntimeInstance],
    ) -> BoxFuture<'a, Result<bool, InstallerError>> {
        let user_context = self.user_context.clone();
        let installed = installed.clone();
        let instances = instances.to_vec();
        let host_error = self.host_support_error();
        Box::pin(async move {
            if let Some(error) = host_error {
                return Err(error);
            }
            run_blocking(move || {
                runtime::is_instance_running(&user_context, &installed, &instances)
            })
            .await
        })
    }
}

fn inspect_local(
    package_manager: &dyn WindowsPackageManager,
    user_context: &InteractiveUserContext,
    host: &WindowsHost,
) -> Result<LocalInstallStatus, InstallerError> {
    let records = inventory_records(package_manager, user_context)?;
    let stable_records = records
        .iter()
        .filter(|record| record.identity_name == WINDOWS_CODEX_STABLE_IDENTITY)
        .collect::<Vec<_>>();
    if stable_records.is_empty() {
        return Ok(LocalInstallStatus::NotInstalled {
            platform: DesktopPlatform::Windows,
            architecture: host.architecture(),
        });
    }

    let applications = stable_records
        .into_iter()
        .map(|record| installed_application_from_record(record, host))
        .collect::<Result<Vec<_>, _>>()?;
    match applications.as_slice() {
        [application] => Ok(LocalInstallStatus::Installed {
            application: application.clone(),
        }),
        _ => Ok(LocalInstallStatus::Ambiguous {
            candidates: applications
                .iter()
                .map(InstalledApplicationSummary::from)
                .collect(),
            error: InstallerError::new(InstallerErrorCode::MultipleInstallations)
                .with_diagnostic_message(
                    "multiple Stable Windows packages prevent a safe update or launch",
                )
                .to_dto(),
        }),
    }
}

/// Produces the one current-user exact PFN-bound installation candidate for
/// the restart planner, or explicit ambiguity when more than one survives.
/// `family_name` is obtained from PackageManager and validated while forming
/// the verified AUMID; display name, executable name, window title, and package
/// path never participate in candidate discovery or ordering.
fn inspect_restart_candidates(
    package_manager: &dyn WindowsPackageManager,
    user_context: &InteractiveUserContext,
    host: &WindowsHost,
) -> Result<RestartCandidateInspection, InstallerError> {
    let records = inventory_records(package_manager, user_context)?;
    let stable_records = records
        .iter()
        .filter(|record| record.identity_name == WINDOWS_CODEX_STABLE_IDENTITY)
        .collect::<Vec<_>>();
    if stable_records.is_empty() {
        return Ok(RestartCandidateInspection::NotInstalled);
    }

    let candidates = stable_records
        .into_iter()
        .map(|record| {
            let application = installed_application_from_record(record, host)?;
            Ok(TrustedInstallationCandidate {
                // The Package Family Name is the exact Windows lifecycle
                // identity. It stays private to the planner/token record and
                // never crosses IPC or appears in ordinary diagnostics.
                stable_key: format!("windows-pfn:{}", record.family_name),
                application,
                scope: RestartInstallationScope::CurrentUser,
            })
        })
        .collect::<Result<Vec<_>, InstallerError>>()?;
    match candidates.as_slice() {
        [candidate] => Ok(RestartCandidateInspection::Trusted(vec![candidate.clone()])),
        _ => Ok(RestartCandidateInspection::AmbiguousInstallations),
    }
}

fn inventory_records(
    package_manager: &dyn WindowsPackageManager,
    user_context: &InteractiveUserContext,
) -> Result<Vec<WindowsPackageRecord>, InstallerError> {
    let inventory = package_manager
        .packages_for_user(user_context)
        .map_err(deployment_error)?;
    verify_context_evidence(user_context, inventory.context_evidence())?;
    Ok(inventory.records().to_vec())
}

fn installed_application_from_record(
    record: &WindowsPackageRecord,
    host: &WindowsHost,
) -> Result<InstalledApplication, InstallerError> {
    if record.identity_name != WINDOWS_CODEX_STABLE_IDENTITY {
        return Err(
            InstallerError::new(InstallerErrorCode::PackageIdentityMismatch)
                .with_diagnostic_message("PackageManager record does not have the Stable identity"),
        );
    }
    if record.architecture != host.architecture() {
        return Err(
            InstallerError::new(InstallerErrorCode::PackageArchitectureMismatch)
                .with_context("architecture", record.architecture.as_str())
                .with_diagnostic_message(
                    "installed Stable package architecture does not match this host",
                ),
        );
    }
    if !matches!(&record.version, PlatformVersion::WindowsMsix { .. }) {
        return Err(InstallerError::new(InstallerErrorCode::PackageParseFailed)
            .with_diagnostic_message("PackageManager returned a non-Windows package version"));
    }
    let application_id = single_application_id(record)?;
    let aumid = verified_aumid(&record.family_name, application_id)?;
    Ok(InstalledApplication {
        stable_identity: WINDOWS_CODEX_STABLE_IDENTITY.to_owned(),
        display_name: record.display_name.clone(),
        display_version: Some(windows_version_text(&record.version)?),
        platform_version: record.version.clone(),
        architecture: record.architecture,
        location: None,
        launch_target: LaunchTarget::WindowsAumid(aumid),
    })
}

fn installed_application_from_dynamic_record(
    record: &WindowsPackageRecord,
) -> Result<InstalledApplication, InstallerError> {
    if record.identity_name.is_empty() || record.publisher.is_empty() {
        return Err(
            InstallerError::new(InstallerErrorCode::InstallationVerifyFailed)
                .with_diagnostic_message("installed package result has no operational identity"),
        );
    }
    if !matches!(&record.version, PlatformVersion::WindowsMsix { .. }) {
        return Err(
            InstallerError::new(InstallerErrorCode::InstallationVerifyFailed)
                .with_diagnostic_message("installed package result has no Windows version"),
        );
    }
    let application_id = single_application_id(record)?;
    let aumid = verified_aumid(&record.family_name, application_id)?;
    Ok(InstalledApplication {
        stable_identity: record.identity_name.clone(),
        display_name: record.display_name.clone(),
        display_version: Some(windows_version_text(&record.version)?),
        platform_version: record.version.clone(),
        architecture: record.architecture,
        location: None,
        launch_target: LaunchTarget::WindowsAumid(aumid),
    })
}

fn preflight(
    host: &WindowsHost,
    release: &ReleaseDescriptor,
    temp_root: &Path,
) -> Result<PlatformInstallPlan, InstallerError> {
    validate_release_for_host(host, release)?;
    if !temp_root.is_dir() {
        return Err(InstallerError::new(InstallerErrorCode::InternalError)
            .with_diagnostic_message("installer temporary root is not an available directory"));
    }
    #[cfg(target_os = "windows")]
    {
        Ok(PlatformInstallPlan::new(vec![
            package_bridge::program_data_bridge_probe_path()?,
        ]))
    }

    #[cfg(target_os = "macos")]
    Ok(PlatformInstallPlan::default())
}

fn install_current_user(
    package_manager: &dyn WindowsPackageManager,
    install_dependencies: &WindowsInstallDependencies,
    user_context: &InteractiveUserContext,
    host: &WindowsHost,
    package: &PreparedInstallPackage,
    progress: PlatformProgressSink,
) -> Result<Option<InstalledApplication>, InstallerError> {
    if package.platform() != DesktopPlatform::Windows
        || package.architecture() != host.architecture()
    {
        return Err(InstallerError::new(InstallerErrorCode::InternalError)
            .with_diagnostic_message(
                "non-Windows prepared package reached the Windows installer",
            ));
    }
    let job_id = package.job_id().ok_or_else(|| {
        InstallerError::new(InstallerErrorCode::InternalError)
            .with_diagnostic_message("prepared Windows package has no canonical job identity")
    })?;
    let parsed_job_id = uuid::Uuid::parse_str(job_id).map_err(|_| {
        InstallerError::new(InstallerErrorCode::InternalError)
            .with_diagnostic_message("prepared Windows package job identity is invalid")
    })?;
    if parsed_job_id.hyphenated().to_string() != job_id {
        return Err(InstallerError::new(InstallerErrorCode::InternalError)
            .with_diagnostic_message("prepared Windows package job identity is not canonical"));
    }

    let context_revalidator = install_dependencies.context_revalidator.as_ref();
    require_current_context(context_revalidator, user_context)?;
    let before_records = inventory_records(package_manager, user_context)?;
    package.revalidate_artifact()?;
    let pin = install_dependencies.pin_factory.open(package)?;
    pin.recheck()?;
    require_current_context(context_revalidator, user_context)?;

    let helper_result = install_dependencies.helper_runner.run(
        user_context,
        job_id,
        pin,
        progress,
        install_dependencies.deadlines,
    );
    require_current_context(context_revalidator, user_context)?;
    helper_result?;

    let records = inventory_records(package_manager, user_context)?;
    require_current_context(context_revalidator, user_context)?;
    let changed_records = records
        .iter()
        .filter(|record| !before_records.contains(record))
        .collect::<Vec<_>>();
    let compatible_stable = records
        .iter()
        .filter(|record| record.identity_name == WINDOWS_CODEX_STABLE_IDENTITY)
        .collect::<Vec<_>>();
    let record = match changed_records.as_slice() {
        [record] => *record,
        [] if compatible_stable.len() == 1 => compatible_stable[0],
        [] => {
            return Err(
                InstallerError::new(InstallerErrorCode::InstallationVerifyFailed)
                    .with_diagnostic_message(
                        "the helper completed without one uniquely discoverable package result for the interactive user",
                    ),
            );
        }
        _ => {
            return Err(
                InstallerError::new(InstallerErrorCode::MultipleInstallations)
                    .with_diagnostic_message(
                        "multiple changed Windows packages prevent post-install result selection",
                    ),
            );
        }
    };
    let installed = installed_application_from_dynamic_record(record)?;
    if !installed_application_has_operational_shape(&installed, package.locked_release())? {
        return Err(
            InstallerError::new(InstallerErrorCode::InstallationVerifyFailed)
                .with_diagnostic_message(
                    "the current-user package does not have a usable post-install platform shape",
                ),
        );
    }
    Ok(Some(installed))
}

fn require_current_context(
    revalidator: &dyn WindowsContextRevalidator,
    context: &InteractiveUserContext,
) -> Result<(), InstallerError> {
    revalidator
        .is_current(context)
        .then_some(())
        .ok_or_else(deployment::interactive_context_error)
}

fn launch(
    package_manager: &dyn WindowsPackageManager,
    user_context: &InteractiveUserContext,
    host: &WindowsHost,
    installed: &InstalledApplication,
) -> Result<(), InstallerError> {
    if installed.stable_identity != WINDOWS_CODEX_STABLE_IDENTITY
        || installed.architecture != host.architecture()
        || !matches!(
            &installed.platform_version,
            PlatformVersion::WindowsMsix { .. }
        )
    {
        return Err(
            InstallerError::new(InstallerErrorCode::LaunchFailed).with_diagnostic_message(
                "launch request does not contain a verified Stable Windows app",
            ),
        );
    }
    let LaunchTarget::WindowsAumid(aumid) = &installed.launch_target else {
        return Err(InstallerError::new(InstallerErrorCode::LaunchFailed)
            .with_diagnostic_message("launch request does not contain a Windows AUMID"));
    };
    if !is_valid_aumid(aumid) {
        return Err(InstallerError::new(InstallerErrorCode::LaunchFailed)
            .with_diagnostic_message("launch request contains an invalid Windows AUMID"));
    }

    // A previously selected application is not itself a launch capability.
    // Re-enumerate the frozen SID/Main inventory immediately before Explorer
    // activation and require the one trusted result to be byte-for-byte the
    // same domain record.
    let records = inventory_records(package_manager, user_context)?;
    let stable_records = records
        .iter()
        .filter(|record| record.identity_name == WINDOWS_CODEX_STABLE_IDENTITY)
        .collect::<Vec<_>>();
    let record = match stable_records.as_slice() {
        [record] => *record,
        [] => {
            return Err(InstallerError::new(InstallerErrorCode::LaunchFailed)
                .with_diagnostic_message(
                    "launch requires one exact Stable package for the interactive user",
                ));
        }
        _ => {
            return Err(
                InstallerError::new(InstallerErrorCode::MultipleInstallations)
                    .with_diagnostic_message(
                        "multiple Stable Windows packages prevent a safe launch",
                    ),
            );
        }
    };
    let current = installed_application_from_record(record, host)?;
    if &current != installed {
        return Err(
            InstallerError::new(InstallerErrorCode::LaunchFailed).with_diagnostic_message(
                "the selected Stable Windows application changed before launch",
            ),
        );
    }

    let receipt = package_manager
        .launch_aumid(user_context, aumid)
        .map_err(launch_error)?;
    verify_context_evidence(user_context, receipt.context_evidence())
}

fn validate_release_for_host(
    host: &WindowsHost,
    release: &ReleaseDescriptor,
) -> Result<(), InstallerError> {
    if release.platform != DesktopPlatform::Windows
        || !matches!(
            &release.platform_version,
            PlatformVersion::WindowsMsix { .. }
        )
    {
        return Err(InstallerError::new(InstallerErrorCode::PlatformUnsupported)
            .with_diagnostic_message("Windows adapter received a non-Windows release"));
    }
    if !matches!(
        release.architecture,
        CpuArchitecture::X86_64 | CpuArchitecture::Aarch64
    ) || release.architecture != host.architecture()
    {
        return Err(
            InstallerError::new(InstallerErrorCode::ArchitectureUnsupported)
                .with_context("architecture", release.architecture.as_str())
                .with_diagnostic_message("Windows release architecture does not match this host"),
        );
    }
    Ok(())
}

fn single_application_id(record: &WindowsPackageRecord) -> Result<&str, InstallerError> {
    let [application_id] = record.application_ids.as_slice() else {
        return Err(InstallerError::new(InstallerErrorCode::PackageParseFailed)
            .with_diagnostic_message(
                "installed Stable package does not have exactly one app entry",
            ));
    };
    if !is_valid_application_id(application_id) {
        return Err(InstallerError::new(InstallerErrorCode::PackageParseFailed)
            .with_diagnostic_message("installed Stable package Application Id is invalid"));
    }
    Ok(application_id)
}

fn verified_aumid(family_name: &str, application_id: &str) -> Result<String, InstallerError> {
    if family_name.is_empty()
        || family_name.len() > 512
        || family_name.contains('!')
        || family_name.bytes().any(|byte| byte.is_ascii_control())
        || !is_valid_application_id(application_id)
    {
        return Err(InstallerError::new(InstallerErrorCode::PackageParseFailed)
            .with_diagnostic_message("installed Stable package cannot form a verified AUMID"));
    }
    Ok(format!("{family_name}!{application_id}"))
}

fn is_valid_application_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-'))
}

fn is_valid_aumid(value: &str) -> bool {
    let Some((family_name, application_id)) = value.split_once('!') else {
        return false;
    };
    !family_name.is_empty()
        && !family_name.contains('!')
        && family_name.len() <= 512
        && !family_name.bytes().any(|byte| byte.is_ascii_control())
        && is_valid_application_id(application_id)
}

fn windows_version_text(version: &PlatformVersion) -> Result<String, InstallerError> {
    let PlatformVersion::WindowsMsix {
        major,
        minor,
        build,
        revision,
    } = version
    else {
        return Err(InstallerError::new(InstallerErrorCode::PackageParseFailed)
            .with_diagnostic_message("installed package version is not a Windows MSIX version"));
    };
    Ok(format!("{major}.{minor}.{build}.{revision}"))
}

async fn run_blocking<T: Send + 'static>(
    operation: impl FnOnce() -> Result<T, InstallerError> + Send + 'static,
) -> Result<T, InstallerError> {
    tokio::task::spawn_blocking(operation).await.map_err(|_| {
        InstallerError::new(InstallerErrorCode::InternalError)
            .with_diagnostic_message("Windows platform worker stopped unexpectedly")
    })?
}

#[cfg(target_os = "windows")]
mod native_host {
    use windows::Win32::System::SystemInformation::{
        GetNativeSystemInfo, PROCESSOR_ARCHITECTURE_AMD64, PROCESSOR_ARCHITECTURE_ARM64,
        SYSTEM_INFO,
    };

    use crate::codex_desktop::types::CpuArchitecture;

    pub(super) fn architecture() -> CpuArchitecture {
        let mut info = SYSTEM_INFO::default();
        unsafe { GetNativeSystemInfo(&mut info) };
        let native_architecture = unsafe { info.Anonymous.Anonymous.wProcessorArchitecture };
        match native_architecture {
            PROCESSOR_ARCHITECTURE_AMD64 => CpuArchitecture::X86_64,
            PROCESSOR_ARCHITECTURE_ARM64 => CpuArchitecture::Aarch64,
            _ => CpuArchitecture::Unsupported,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        fs,
        sync::{
            atomic::{AtomicBool, AtomicUsize, Ordering},
            Arc, Mutex,
        },
    };

    use super::*;
    use crate::codex_desktop::{
        download::DownloadedArtifact,
        error::{InstallerErrorCode, SuggestedAction},
        temp::JobTempDir,
        types::{JobProgress, PlatformVersion, ProgressPhase, TrustedDownloadEndpoint},
        verify::ArtifactKind,
    };
    use uuid::Uuid;

    const PUBLISHER: &str = "CN=fixture publisher";
    const FAMILY_NAME: &str = "OpenAI.Codex_fixture";
    const USER_SID: &str = "S-1-5-21-1000";
    const OTHER_USER_SID: &str = "S-1-5-21-2000";

    #[derive(Clone)]
    enum FakeEvidence {
        Bound,
        Missing,
        Override(WindowsUserContextEvidence),
    }

    impl FakeEvidence {
        fn for_context(
            &self,
            context: &InteractiveUserContext,
        ) -> Option<WindowsUserContextEvidence> {
            match self {
                Self::Bound => Some(WindowsUserContextEvidence::for_test(context)),
                Self::Missing => None,
                Self::Override(evidence) => Some(evidence.clone()),
            }
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum FakePackageOperation {
        InventoryMain {
            canonical_sid: String,
        },
        Launch {
            canonical_sid: String,
            aumid: String,
        },
    }

    #[derive(Default)]
    struct FakeContextRevalidator {
        current: AtomicBool,
        calls: AtomicUsize,
    }

    impl FakeContextRevalidator {
        fn current() -> Self {
            Self {
                current: AtomicBool::new(true),
                calls: AtomicUsize::new(0),
            }
        }

        fn set_current(&self, current: bool) {
            self.current.store(current, Ordering::Release);
        }
    }

    impl WindowsContextRevalidator for FakeContextRevalidator {
        fn is_current(&self, _context: &InteractiveUserContext) -> bool {
            self.calls.fetch_add(1, Ordering::AcqRel);
            self.current.load(Ordering::Acquire)
        }
    }

    #[derive(Default)]
    struct FakePinState {
        opened: AtomicUsize,
        rechecked: AtomicUsize,
        dropped: AtomicUsize,
    }

    struct FakePin {
        state: Arc<FakePinState>,
    }

    impl WindowsVerifiedFilePin for FakePin {
        fn recheck(&self) -> Result<(), InstallerError> {
            self.state.rechecked.fetch_add(1, Ordering::AcqRel);
            Ok(())
        }

        fn identity(&self) -> WindowsPackageFileIdentity {
            WindowsPackageFileIdentity {
                volume_serial: 7,
                file_index: 11,
                size: 13,
            }
        }

        fn expected_size(&self) -> u64 {
            13
        }

        fn expected_sha256(&self) -> &str {
            "0000000000000000000000000000000000000000000000000000000000000000"
        }

        fn duplicate_source_file(&self) -> Result<std::fs::File, InstallerError> {
            Err(InstallerError::new(
                InstallerErrorCode::PackageIdentityMismatch,
            ))
        }
    }

    impl Drop for FakePin {
        fn drop(&mut self) {
            self.state.dropped.fetch_add(1, Ordering::AcqRel);
        }
    }

    struct FakePinFactory {
        state: Arc<FakePinState>,
    }

    impl WindowsFilePinFactory for FakePinFactory {
        fn open(
            &self,
            _package: &PreparedInstallPackage,
        ) -> Result<Box<dyn WindowsVerifiedFilePin>, InstallerError> {
            self.state.opened.fetch_add(1, Ordering::AcqRel);
            Ok(Box::new(FakePin {
                state: self.state.clone(),
            }))
        }
    }

    #[derive(Default)]
    struct FakeHelperState {
        calls: AtomicUsize,
        job_ids: Mutex<Vec<String>>,
        deadlines: Mutex<Vec<WindowsHelperDeadlines>>,
        error: Mutex<Option<InstallerErrorCode>>,
        retain_pin: AtomicBool,
        retained_pin: Mutex<Option<Box<dyn WindowsVerifiedFilePin>>>,
    }

    struct FakeHelperRunner {
        state: Arc<FakeHelperState>,
        pin_state: Arc<FakePinState>,
        context: Arc<FakeContextRevalidator>,
        drift_after_run: AtomicBool,
        after_run: Mutex<Option<Arc<dyn Fn() + Send + Sync>>>,
    }

    impl FakeHelperRunner {
        fn set_error(&self, error: Option<InstallerErrorCode>) {
            *self
                .state
                .error
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()) = error;
        }

        fn retain_pin(&self, retain: bool) {
            self.state.retain_pin.store(retain, Ordering::Release);
        }

        fn drift_after_run(&self, drift: bool) {
            self.drift_after_run.store(drift, Ordering::Release);
        }

        fn after_run(&self, callback: Arc<dyn Fn() + Send + Sync>) {
            *self
                .after_run
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(callback);
        }
    }

    impl WindowsUserHelperRunner for FakeHelperRunner {
        fn run(
            &self,
            _context: &InteractiveUserContext,
            job_id: &str,
            pin: Box<dyn WindowsVerifiedFilePin>,
            progress: PlatformProgressSink,
            deadlines: WindowsHelperDeadlines,
        ) -> Result<(), InstallerError> {
            assert_eq!(self.pin_state.rechecked.load(Ordering::Acquire), 1);
            assert_eq!(self.pin_state.dropped.load(Ordering::Acquire), 0);
            assert_eq!(pin.expected_size(), pin.identity().size);
            assert_eq!(pin.expected_sha256().len(), 64);
            assert!(pin
                .expected_sha256()
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase()));
            self.state.calls.fetch_add(1, Ordering::AcqRel);
            self.state
                .job_ids
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .push(job_id.to_owned());
            self.state
                .deadlines
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .push(deadlines);
            for completed in [0, 35, 80, 100] {
                progress.report_progress(JobProgress::new(
                    ProgressPhase::Installation,
                    Some(completed),
                    Some(100),
                ));
            }
            if self.state.retain_pin.load(Ordering::Acquire) {
                *self
                    .state
                    .retained_pin
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(pin);
            } else {
                drop(pin);
            }
            if self.drift_after_run.load(Ordering::Acquire) {
                self.context.set_current(false);
            }
            if let Some(callback) = self
                .after_run
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .as_ref()
            {
                callback();
            }
            match *self
                .state
                .error
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
            {
                Some(code) => Err(InstallerError::new(code)
                    .with_diagnostic_message("fake helper reported a bounded failure")),
                None => Ok(()),
            }
        }
    }

    struct FakeInstallHarness {
        context: Arc<FakeContextRevalidator>,
        pin_state: Arc<FakePinState>,
        helper_state: Arc<FakeHelperState>,
        helper: Arc<FakeHelperRunner>,
    }

    impl FakeInstallHarness {
        fn new() -> Self {
            let context = Arc::new(FakeContextRevalidator::current());
            let pin_state = Arc::new(FakePinState::default());
            let helper_state = Arc::new(FakeHelperState::default());
            let helper = Arc::new(FakeHelperRunner {
                state: helper_state.clone(),
                pin_state: pin_state.clone(),
                context: context.clone(),
                drift_after_run: AtomicBool::new(false),
                after_run: Mutex::new(None),
            });
            Self {
                context,
                pin_state,
                helper_state,
                helper,
            }
        }

        fn dependencies(&self) -> WindowsInstallDependencies {
            WindowsInstallDependencies {
                context_revalidator: self.context.clone(),
                pin_factory: Arc::new(FakePinFactory {
                    state: self.pin_state.clone(),
                }),
                helper_runner: self.helper.clone(),
                deadlines: WindowsHelperDeadlines::PRODUCTION,
            }
        }
    }

    struct FakePackageManager {
        records_by_sid: Mutex<HashMap<String, Vec<WindowsPackageRecord>>>,
        context_is_current: AtomicBool,
        inventory_evidence: Mutex<FakeEvidence>,
        launch_evidence: Mutex<FakeEvidence>,
        launched_aumids: Mutex<Vec<String>>,
        launch_result: Mutex<Result<(), WindowsNativeError>>,
        operations: Mutex<Vec<FakePackageOperation>>,
    }

    impl FakePackageManager {
        fn with_records(records: Vec<WindowsPackageRecord>) -> Self {
            Self::with_user_records([(USER_SID, records)])
        }

        fn with_user_records(
            records: impl IntoIterator<Item = (&'static str, Vec<WindowsPackageRecord>)>,
        ) -> Self {
            Self {
                records_by_sid: Mutex::new(
                    records
                        .into_iter()
                        .map(|(sid, records)| (sid.to_owned(), records))
                        .collect(),
                ),
                context_is_current: AtomicBool::new(true),
                inventory_evidence: Mutex::new(FakeEvidence::Bound),
                launch_evidence: Mutex::new(FakeEvidence::Bound),
                launched_aumids: Mutex::new(Vec::new()),
                launch_result: Mutex::new(Ok(())),
                operations: Mutex::new(Vec::new()),
            }
        }

        fn set_user_records(&self, sid: &str, records: Vec<WindowsPackageRecord>) {
            self.records_by_sid
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .insert(sid.to_owned(), records);
        }

        fn set_context_is_current(&self, value: bool) {
            self.context_is_current.store(value, Ordering::Release);
        }

        fn set_inventory_evidence(&self, evidence: FakeEvidence) {
            *self
                .inventory_evidence
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()) = evidence;
        }

        fn set_launch_evidence(&self, evidence: FakeEvidence) {
            *self
                .launch_evidence
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()) = evidence;
        }

        fn set_launch_result(&self, result: Result<(), WindowsNativeError>) {
            *self
                .launch_result
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()) = result;
        }

        fn operations(&self) -> Vec<FakePackageOperation> {
            self.operations
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .clone()
        }
    }

    impl Default for FakePackageManager {
        fn default() -> Self {
            Self::with_records(Vec::new())
        }
    }

    impl WindowsPackageManager for FakePackageManager {
        fn packages_for_user(
            &self,
            context: &InteractiveUserContext,
        ) -> Result<WindowsPackageInventory, WindowsNativeError> {
            self.operations
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .push(FakePackageOperation::InventoryMain {
                    canonical_sid: context.canonical_sid().to_owned(),
                });
            if !self.context_is_current.load(Ordering::Acquire) {
                return Err(WindowsNativeError::context_mismatch());
            }
            let records = self
                .records_by_sid
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .get(context.canonical_sid())
                .cloned()
                .unwrap_or_default();
            let evidence = self
                .inventory_evidence
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .for_context(context);
            Ok(WindowsPackageInventory::for_test(evidence, records))
        }

        fn launch_aumid(
            &self,
            context: &InteractiveUserContext,
            aumid: &str,
        ) -> Result<WindowsUserOperationReceipt, WindowsNativeError> {
            if !self.context_is_current.load(Ordering::Acquire) {
                return Err(WindowsNativeError::context_mismatch());
            }
            self.launched_aumids
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .push(aumid.to_owned());
            self.operations
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .push(FakePackageOperation::Launch {
                    canonical_sid: context.canonical_sid().to_owned(),
                    aumid: aumid.to_owned(),
                });
            (*self
                .launch_result
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()))?;
            let evidence = self
                .launch_evidence
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .for_context(context);
            Ok(WindowsUserOperationReceipt::for_test(evidence))
        }
    }

    fn host(architecture: CpuArchitecture, version: &str) -> WindowsHost {
        WindowsHost::new(architecture, version).unwrap()
    }

    fn user_context(sid: &str) -> Arc<InteractiveUserContext> {
        Arc::new(InteractiveUserContext::for_test(sid, 1))
    }

    fn release(
        architecture: CpuArchitecture,
        _minimum_os_version: Option<&str>,
    ) -> ReleaseDescriptor {
        ReleaseDescriptor::new(
            DesktopPlatform::Windows,
            architecture,
            "1.2.3.4",
            PlatformVersion::parse_windows_msix("1.2.3.4").unwrap(),
            Some(1024),
            match architecture {
                CpuArchitecture::X86_64 => TrustedDownloadEndpoint::WinX64,
                CpuArchitecture::Aarch64 => TrustedDownloadEndpoint::WinArm64,
                _ => panic!("fixture release architecture must be supported"),
            },
        )
        .unwrap()
    }

    fn record(
        identity_name: &str,
        publisher: &str,
        architecture: CpuArchitecture,
        application_ids: Vec<&str>,
    ) -> WindowsPackageRecord {
        WindowsPackageRecord::new(
            identity_name,
            publisher,
            FAMILY_NAME,
            PlatformVersion::parse_windows_msix("1.2.3.4").unwrap(),
            architecture,
            Some("Codex".to_owned()),
            application_ids.into_iter().map(str::to_owned).collect(),
        )
    }

    fn adapter(manager: Arc<dyn WindowsPackageManager>) -> WindowsPlatformAdapter {
        let harness = FakeInstallHarness::new();
        adapter_with_harness(manager, &harness)
    }

    fn adapter_with_harness(
        manager: Arc<dyn WindowsPackageManager>,
        harness: &FakeInstallHarness,
    ) -> WindowsPlatformAdapter {
        WindowsPlatformAdapter::new(
            manager,
            user_context(USER_SID),
            host(CpuArchitecture::X86_64, "10.0.22631.0"),
            harness.dependencies(),
        )
    }

    fn release_for_artifact(bytes: &[u8]) -> ReleaseDescriptor {
        ReleaseDescriptor::new(
            DesktopPlatform::Windows,
            CpuArchitecture::X86_64,
            "1.2.3.4",
            PlatformVersion::parse_windows_msix("1.2.3.4").unwrap(),
            Some(bytes.len() as u64),
            TrustedDownloadEndpoint::WinX64,
        )
        .unwrap()
    }

    fn downloaded_artifact_for(
        release: &ReleaseDescriptor,
        bytes: &[u8],
    ) -> (tempfile::TempDir, DownloadedArtifact) {
        let root = tempfile::tempdir().unwrap();
        let directory =
            JobTempDir::create(root.path(), &Uuid::new_v4().hyphenated().to_string()).unwrap();
        fs::write(directory.final_path(ArtifactKind::Msix), bytes).unwrap();
        let artifact = DownloadedArtifact::from_test_file(&directory, release).unwrap();
        (root, artifact)
    }

    fn verified_msix_artifact() -> (tempfile::TempDir, ReleaseDescriptor, DownloadedArtifact) {
        let root = tempfile::tempdir().unwrap();
        let directory =
            JobTempDir::create(root.path(), &Uuid::new_v4().hyphenated().to_string()).unwrap();
        let path = directory.final_path(ArtifactKind::Msix);
        fs::write(&path, b"opaque msix bytes for local handoff").unwrap();

        let bytes = fs::read(&path).unwrap();
        let release = ReleaseDescriptor::new(
            DesktopPlatform::Windows,
            CpuArchitecture::X86_64,
            "26.721.4979",
            PlatformVersion::parse_windows_msix("26.721.4979.0").unwrap(),
            Some(bytes.len() as u64),
            TrustedDownloadEndpoint::WinX64,
        )
        .unwrap();
        let artifact = DownloadedArtifact::from_test_file(&directory, &release).unwrap();
        (root, release, artifact)
    }

    #[tokio::test]
    async fn current_user_inventory_uses_exact_identity_publisher_architecture_and_aumid() {
        let manager = Arc::new(FakePackageManager::with_records(vec![
            record(
                "OpenAI.CodexBeta",
                PUBLISHER,
                CpuArchitecture::X86_64,
                vec!["Beta"],
            ),
            record(
                WINDOWS_CODEX_STABLE_IDENTITY,
                PUBLISHER,
                CpuArchitecture::X86_64,
                vec!["CodexApp"],
            ),
        ]));
        let status = adapter(manager.clone()).inspect_local().await.unwrap();
        let LocalInstallStatus::Installed { application } = status else {
            panic!("exact Stable record should be installed")
        };
        assert_eq!(application.stable_identity, WINDOWS_CODEX_STABLE_IDENTITY);
        assert_eq!(application.display_version.as_deref(), Some("1.2.3.4"));
        assert_eq!(
            application.launch_target,
            LaunchTarget::WindowsAumid(format!("{FAMILY_NAME}!CodexApp"))
        );
        assert_eq!(application.location, None);
        assert_eq!(
            manager.operations(),
            vec![FakePackageOperation::InventoryMain {
                canonical_sid: USER_SID.to_owned(),
            }]
        );
    }

    #[tokio::test]
    async fn explicit_sid_main_inventory_ignores_other_users() {
        let manager = Arc::new(FakePackageManager::with_user_records([
            (
                USER_SID,
                vec![record(
                    WINDOWS_CODEX_STABLE_IDENTITY,
                    PUBLISHER,
                    CpuArchitecture::X86_64,
                    vec!["CodexApp"],
                )],
            ),
            (
                OTHER_USER_SID,
                vec![
                    record(
                        WINDOWS_CODEX_STABLE_IDENTITY,
                        PUBLISHER,
                        CpuArchitecture::X86_64,
                        vec!["OtherOne"],
                    ),
                    record(
                        WINDOWS_CODEX_STABLE_IDENTITY,
                        PUBLISHER,
                        CpuArchitecture::X86_64,
                        vec!["OtherTwo"],
                    ),
                ],
            ),
        ]));

        let status = adapter(manager.clone()).inspect_local().await.unwrap();
        let LocalInstallStatus::Installed { application } = status else {
            panic!("the one same-SID Stable Main package must be selected")
        };
        assert_eq!(
            application.launch_target,
            LaunchTarget::WindowsAumid(format!("{FAMILY_NAME}!CodexApp"))
        );
        assert_eq!(
            manager.operations(),
            vec![FakePackageOperation::InventoryMain {
                canonical_sid: USER_SID.to_owned(),
            }]
        );
    }

    #[tokio::test]
    async fn other_user_packages_do_not_change_same_user_absence() {
        let manager = Arc::new(FakePackageManager::with_user_records([(
            OTHER_USER_SID,
            vec![record(
                WINDOWS_CODEX_STABLE_IDENTITY,
                PUBLISHER,
                CpuArchitecture::X86_64,
                vec!["OtherCodex"],
            )],
        )]));
        let adapter = adapter(manager.clone());

        assert_eq!(
            adapter.inspect_local().await.unwrap(),
            LocalInstallStatus::NotInstalled {
                platform: DesktopPlatform::Windows,
                architecture: CpuArchitecture::X86_64,
            }
        );
        assert_eq!(
            adapter.inspect_restart_candidates().await.unwrap(),
            RestartCandidateInspection::NotInstalled
        );
        assert!(manager.operations().iter().all(|operation| matches!(
            operation,
            FakePackageOperation::InventoryMain { canonical_sid }
                if canonical_sid == USER_SID
        )));
    }

    #[tokio::test]
    async fn multiple_same_sid_stable_main_packages_are_ambiguous_for_discovery_and_restart() {
        let manager = Arc::new(FakePackageManager::with_records(vec![
            record(
                WINDOWS_CODEX_STABLE_IDENTITY,
                PUBLISHER,
                CpuArchitecture::X86_64,
                vec!["CodexOne"],
            ),
            record(
                WINDOWS_CODEX_STABLE_IDENTITY,
                PUBLISHER,
                CpuArchitecture::X86_64,
                vec!["CodexTwo"],
            ),
        ]));
        let adapter = adapter(manager.clone());

        let LocalInstallStatus::Ambiguous { candidates, .. } =
            adapter.inspect_local().await.unwrap()
        else {
            panic!("same-user duplicate Stable Main packages must be ambiguous")
        };
        assert_eq!(candidates.len(), 2);
        assert_eq!(
            adapter.inspect_restart_candidates().await.unwrap(),
            RestartCandidateInspection::AmbiguousInstallations
        );
        assert!(manager.operations().iter().all(|operation| matches!(
            operation,
            FakePackageOperation::InventoryMain { canonical_sid }
                if canonical_sid == USER_SID
        )));
    }

    #[tokio::test]
    async fn missing_or_wrong_context_inventory_evidence_fails_closed() {
        let manager = Arc::new(FakePackageManager::with_records(vec![record(
            WINDOWS_CODEX_STABLE_IDENTITY,
            PUBLISHER,
            CpuArchitecture::X86_64,
            vec!["CodexApp"],
        )]));
        let adapter = adapter(manager.clone());

        manager.set_inventory_evidence(FakeEvidence::Missing);
        let missing = adapter.inspect_local().await.unwrap_err();
        assert_eq!(missing.code(), InstallerErrorCode::PackageIdentityMismatch);

        let other_context = InteractiveUserContext::for_test(OTHER_USER_SID, 1);
        manager.set_inventory_evidence(FakeEvidence::Override(
            WindowsUserContextEvidence::for_test(&other_context),
        ));
        let wrong_owner = adapter.inspect_local().await.unwrap_err();
        assert_eq!(
            wrong_owner.code(),
            InstallerErrorCode::PackageIdentityMismatch
        );
    }

    #[tokio::test]
    async fn local_inventory_ignores_publisher_but_keeps_operational_shape_checks() {
        let publisher_drift = adapter(Arc::new(FakePackageManager::with_records(vec![record(
            WINDOWS_CODEX_STABLE_IDENTITY,
            "CN=changed-upstream",
            CpuArchitecture::X86_64,
            vec!["CodexApp"],
        )])))
        .inspect_local()
        .await
        .unwrap();
        assert!(matches!(
            publisher_drift,
            LocalInstallStatus::Installed { .. }
        ));

        for record in [
            record(
                WINDOWS_CODEX_STABLE_IDENTITY,
                PUBLISHER,
                CpuArchitecture::Aarch64,
                vec!["CodexApp"],
            ),
            record(
                WINDOWS_CODEX_STABLE_IDENTITY,
                PUBLISHER,
                CpuArchitecture::X86_64,
                vec!["One", "Two"],
            ),
        ] {
            let error = adapter(Arc::new(FakePackageManager::with_records(vec![record])))
                .inspect_local()
                .await
                .unwrap_err();
            assert!(matches!(
                error.code(),
                InstallerErrorCode::PackageIdentityMismatch
                    | InstallerErrorCode::PackageArchitectureMismatch
                    | InstallerErrorCode::PackageParseFailed
            ));
        }
    }

    #[tokio::test]
    async fn preflight_rejects_architecture_but_ignores_upstream_minimum_os() {
        let adapter = adapter(Arc::new(FakePackageManager::default()));
        let temporary = tempfile::tempdir().unwrap();
        let plan = adapter
            .preflight(&release(CpuArchitecture::X86_64, None), temporary.path())
            .await
            .unwrap();
        #[cfg(target_os = "windows")]
        {
            let bridge_probe = package_bridge::program_data_bridge_probe_path().unwrap();
            assert_eq!(
                plan.additional_disk_paths(),
                std::slice::from_ref(&bridge_probe)
            );
        }
        #[cfg(target_os = "macos")]
        assert!(plan.additional_disk_paths().is_empty());

        let architecture_error = adapter
            .preflight(&release(CpuArchitecture::Aarch64, None), temporary.path())
            .await
            .unwrap_err();
        assert_eq!(
            architecture_error.code(),
            InstallerErrorCode::ArchitectureUnsupported
        );

        adapter
            .preflight(
                &release(CpuArchitecture::X86_64, Some("10.0.65535.0")),
                temporary.path(),
            )
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn common_install_orchestration_holds_pin_reports_progress_and_consumes_job_id() {
        let manager = Arc::new(FakePackageManager::with_records(vec![record(
            WINDOWS_CODEX_STABLE_IDENTITY,
            PUBLISHER,
            CpuArchitecture::X86_64,
            vec!["CodexApp"],
        )]));
        let harness = FakeInstallHarness::new();
        let adapter = adapter_with_harness(manager.clone(), &harness);
        let trusted_bytes = b"fixture";
        let release = release_for_artifact(trusted_bytes);
        let (_root, artifact) = downloaded_artifact_for(&release, trusted_bytes);
        let package = PreparedInstallPackage::from_prepared_artifact(&release, artifact).unwrap();
        let expected_job_id = package.job_id().unwrap().to_owned();
        let reported = Arc::new(Mutex::new(Vec::<u64>::new()));
        let reported_for_sink = reported.clone();
        let progress: PlatformProgressSink = Arc::new(move |progress: JobProgress| {
            reported_for_sink
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .push(progress.completed_bytes.unwrap());
        });
        adapter
            .install_current_user(&package, progress)
            .await
            .unwrap();
        assert_eq!(
            *reported
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
            vec![0, 35, 80, 100]
        );
        assert_eq!(harness.pin_state.opened.load(Ordering::Acquire), 1);
        assert_eq!(harness.pin_state.rechecked.load(Ordering::Acquire), 1);
        assert_eq!(harness.pin_state.dropped.load(Ordering::Acquire), 1);
        assert_eq!(harness.helper_state.calls.load(Ordering::Acquire), 1);
        assert_eq!(harness.context.calls.load(Ordering::Acquire), 4);
        assert_eq!(
            *harness
                .helper_state
                .job_ids
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
            vec![expected_job_id]
        );
        assert_eq!(
            *harness
                .helper_state
                .deadlines
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
            vec![WindowsHelperDeadlines::PRODUCTION]
        );
        assert_eq!(
            manager.operations(),
            vec![
                FakePackageOperation::InventoryMain {
                    canonical_sid: USER_SID.to_owned(),
                },
                FakePackageOperation::InventoryMain {
                    canonical_sid: USER_SID.to_owned(),
                },
            ]
        );
    }

    #[tokio::test]
    async fn install_uses_the_unique_inventory_delta_and_rejects_an_ambiguous_delta() {
        let release = release_for_artifact(b"fixture");

        let unique_manager = Arc::new(FakePackageManager::default());
        let unique_harness = FakeInstallHarness::new();
        let manager_for_callback = unique_manager.clone();
        unique_harness.helper.after_run(Arc::new(move || {
            manager_for_callback.set_user_records(
                USER_SID,
                vec![record(
                    "OpenAI.FutureProduct",
                    "CN=changed-upstream",
                    CpuArchitecture::X86_64,
                    vec!["FutureApp"],
                )],
            );
        }));
        let (_root, artifact) = downloaded_artifact_for(&release, b"fixture");
        let package = PreparedInstallPackage::from_prepared_artifact(&release, artifact).unwrap();
        let installed = adapter_with_harness(unique_manager, &unique_harness)
            .install_current_user(&package, Arc::new(|_| {}))
            .await
            .unwrap()
            .expect("Windows install returns the current-job inventory result");
        assert_eq!(installed.stable_identity, "OpenAI.FutureProduct");

        let ambiguous_manager = Arc::new(FakePackageManager::default());
        let ambiguous_harness = FakeInstallHarness::new();
        let manager_for_callback = ambiguous_manager.clone();
        ambiguous_harness.helper.after_run(Arc::new(move || {
            manager_for_callback.set_user_records(
                USER_SID,
                vec![
                    record(
                        "OpenAI.FutureProduct",
                        "CN=one",
                        CpuArchitecture::X86_64,
                        vec!["FutureApp"],
                    ),
                    record(
                        "OpenAI.OtherProduct",
                        "CN=two",
                        CpuArchitecture::X86_64,
                        vec!["OtherApp"],
                    ),
                ],
            );
        }));
        let (_root, artifact) = downloaded_artifact_for(&release, b"fixture");
        let package = PreparedInstallPackage::from_prepared_artifact(&release, artifact).unwrap();
        let error = adapter_with_harness(ambiguous_manager, &ambiguous_harness)
            .install_current_user(&package, Arc::new(|_| {}))
            .await
            .unwrap_err();
        assert_eq!(error.code(), InstallerErrorCode::MultipleInstallations);
    }

    #[tokio::test]
    async fn helper_failure_is_bounded_and_a_retained_pin_is_not_dropped() {
        let manager = Arc::new(FakePackageManager::with_records(vec![record(
            WINDOWS_CODEX_STABLE_IDENTITY,
            PUBLISHER,
            CpuArchitecture::X86_64,
            vec!["CodexApp"],
        )]));
        let harness = FakeInstallHarness::new();
        harness
            .helper
            .set_error(Some(InstallerErrorCode::WindowsPackageInUse));
        harness.helper.retain_pin(true);
        let adapter = adapter_with_harness(manager.clone(), &harness);
        let trusted_bytes = b"fixture";
        let release = release_for_artifact(trusted_bytes);
        let (_root, artifact) = downloaded_artifact_for(&release, trusted_bytes);
        let package = PreparedInstallPackage::from_prepared_artifact(&release, artifact).unwrap();

        let error = adapter
            .install_current_user(&package, Arc::new(|_| {}))
            .await
            .unwrap_err();
        assert_eq!(error.code(), InstallerErrorCode::WindowsPackageInUse);
        assert_eq!(harness.pin_state.dropped.load(Ordering::Acquire), 0);
        assert!(harness
            .helper_state
            .retained_pin
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .is_some());
        assert_eq!(manager.operations().len(), 1);
    }

    #[tokio::test]
    async fn install_revalidates_context_before_and_after_the_helper_side_effect() {
        let trusted_bytes = b"fixture";
        let release = release_for_artifact(trusted_bytes);
        let (_root, artifact) = downloaded_artifact_for(&release, trusted_bytes);
        let package = PreparedInstallPackage::from_prepared_artifact(&release, artifact).unwrap();

        let manager = Arc::new(FakePackageManager::default());
        let before = FakeInstallHarness::new();
        before.context.set_current(false);
        let error = adapter_with_harness(manager.clone(), &before)
            .install_current_user(&package, Arc::new(|_| {}))
            .await
            .unwrap_err();
        assert_eq!(error.code(), InstallerErrorCode::PackageIdentityMismatch);
        assert_eq!(before.pin_state.opened.load(Ordering::Acquire), 0);
        assert_eq!(before.helper_state.calls.load(Ordering::Acquire), 0);

        let after = FakeInstallHarness::new();
        after.helper.drift_after_run(true);
        let error = adapter_with_harness(manager.clone(), &after)
            .install_current_user(&package, Arc::new(|_| {}))
            .await
            .unwrap_err();
        assert_eq!(error.code(), InstallerErrorCode::PackageIdentityMismatch);
        assert_eq!(after.helper_state.calls.load(Ordering::Acquire), 1);
        assert_eq!(manager.operations().len(), 1);
    }

    #[tokio::test]
    async fn replacement_after_platform_verification_never_reaches_current_user_deployment() {
        let manager = Arc::new(FakePackageManager::default());
        let harness = FakeInstallHarness::new();
        let adapter = WindowsPlatformAdapter::new(
            manager.clone(),
            user_context(USER_SID),
            host(CpuArchitecture::X86_64, "10.0.22631.0"),
            harness.dependencies(),
        );
        let (_root, release, artifact) = verified_msix_artifact();
        let package = adapter
            .prepare_install_package(&release, &artifact)
            .await
            .unwrap();
        let mut replacement = fs::read(package.artifact_path()).unwrap();
        replacement[0] ^= 0x01;
        fs::write(package.artifact_path(), replacement).unwrap();

        let error = adapter
            .install_current_user(&package, Arc::new(|_| {}))
            .await
            .expect_err("a post-verification replacement must not reach PackageManager");

        assert_eq!(error.code(), InstallerErrorCode::ChecksumMismatch);
        assert_eq!(harness.pin_state.opened.load(Ordering::Acquire), 0);
        assert_eq!(harness.helper_state.calls.load(Ordering::Acquire), 0);
        assert_eq!(manager.operations().len(), 1);
    }

    #[tokio::test]
    async fn launch_accepts_only_verified_aumid_and_preserves_a_stable_error() {
        let manager = Arc::new(FakePackageManager::with_records(vec![record(
            WINDOWS_CODEX_STABLE_IDENTITY,
            PUBLISHER,
            CpuArchitecture::X86_64,
            vec!["CodexApp"],
        )]));
        let adapter = adapter(manager.clone());
        let installed = InstalledApplication {
            stable_identity: WINDOWS_CODEX_STABLE_IDENTITY.to_owned(),
            display_name: Some("Codex".to_owned()),
            display_version: Some("1.2.3.4".to_owned()),
            platform_version: PlatformVersion::parse_windows_msix("1.2.3.4").unwrap(),
            architecture: CpuArchitecture::X86_64,
            location: None,
            launch_target: LaunchTarget::WindowsAumid(format!("{FAMILY_NAME}!CodexApp")),
        };
        adapter.launch(&installed).await.unwrap();
        assert_eq!(
            *manager
                .launched_aumids
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
            vec![format!("{FAMILY_NAME}!CodexApp")]
        );
        assert_eq!(
            &manager.operations()[..2],
            &[
                FakePackageOperation::InventoryMain {
                    canonical_sid: USER_SID.to_owned(),
                },
                FakePackageOperation::Launch {
                    canonical_sid: USER_SID.to_owned(),
                    aumid: format!("{FAMILY_NAME}!CodexApp"),
                },
            ]
        );

        manager.set_launch_result(Err(WindowsNativeError::from_hresult(
            0x8000_4005_u32 as i32,
        )));
        let error = adapter.launch(&installed).await.unwrap_err();
        assert_eq!(error.code(), InstallerErrorCode::LaunchFailed);

        let invalid = InstalledApplication {
            launch_target: LaunchTarget::WindowsAumid("not-an-aumid".to_owned()),
            ..installed
        };
        let error = adapter.launch(&invalid).await.unwrap_err();
        assert_eq!(error.code(), InstallerErrorCode::LaunchFailed);
    }

    #[tokio::test]
    async fn launch_requeries_unique_same_context_installation_before_activation() {
        let manager = Arc::new(FakePackageManager::with_records(vec![record(
            WINDOWS_CODEX_STABLE_IDENTITY,
            PUBLISHER,
            CpuArchitecture::X86_64,
            vec!["CodexApp"],
        )]));
        let adapter = adapter(manager.clone());
        let LocalInstallStatus::Installed { application } = adapter.inspect_local().await.unwrap()
        else {
            panic!("fixture must select one installed application")
        };
        manager
            .operations
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clear();

        let mut replacement = record(
            WINDOWS_CODEX_STABLE_IDENTITY,
            PUBLISHER,
            CpuArchitecture::X86_64,
            vec!["CodexApp"],
        );
        replacement.family_name = "OpenAI.Codex_replacement".to_owned();
        manager.set_user_records(USER_SID, vec![replacement]);
        let error = adapter.launch(&application).await.unwrap_err();
        assert_eq!(error.code(), InstallerErrorCode::LaunchFailed);
        assert!(manager
            .launched_aumids
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .is_empty());

        manager.set_user_records(
            USER_SID,
            vec![
                record(
                    WINDOWS_CODEX_STABLE_IDENTITY,
                    PUBLISHER,
                    CpuArchitecture::X86_64,
                    vec!["CodexApp"],
                ),
                record(
                    WINDOWS_CODEX_STABLE_IDENTITY,
                    PUBLISHER,
                    CpuArchitecture::X86_64,
                    vec!["SecondApp"],
                ),
            ],
        );
        let error = adapter.launch(&application).await.unwrap_err();
        assert_eq!(error.code(), InstallerErrorCode::MultipleInstallations);
        let error = error.to_dto();
        assert!(!error.retryable);
        assert_eq!(error.suggested_action, SuggestedAction::ResolvePathConflict);
        assert!(manager
            .launched_aumids
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .is_empty());
    }

    #[tokio::test]
    async fn launch_blocks_context_drift_and_rejects_a_wrong_context_receipt() {
        let installed_record = record(
            WINDOWS_CODEX_STABLE_IDENTITY,
            PUBLISHER,
            CpuArchitecture::X86_64,
            vec!["CodexApp"],
        );
        let installed = installed_application_from_record(
            &installed_record,
            &host(CpuArchitecture::X86_64, "10.0.22631.0"),
        )
        .unwrap();

        let drifted = Arc::new(FakePackageManager::with_records(vec![
            installed_record.clone()
        ]));
        drifted.set_context_is_current(false);
        let error = adapter(drifted.clone())
            .launch(&installed)
            .await
            .unwrap_err();
        assert_eq!(error.code(), InstallerErrorCode::PackageIdentityMismatch);
        assert!(drifted
            .launched_aumids
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .is_empty());

        let wrong_receipt = Arc::new(FakePackageManager::with_records(vec![installed_record]));
        let other_context = InteractiveUserContext::for_test(OTHER_USER_SID, 1);
        wrong_receipt.set_launch_evidence(FakeEvidence::Override(
            WindowsUserContextEvidence::for_test(&other_context),
        ));
        let error = adapter(wrong_receipt.clone())
            .launch(&installed)
            .await
            .unwrap_err();
        assert_eq!(error.code(), InstallerErrorCode::PackageIdentityMismatch);
        assert_eq!(
            wrong_receipt
                .launched_aumids
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .len(),
            1
        );
    }
}
