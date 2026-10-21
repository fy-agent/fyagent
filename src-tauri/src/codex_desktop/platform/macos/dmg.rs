//! DMG mount, discovery, and same-volume replacement transaction.
//!
//! The transaction never uses a remote-supplied filename for a staging or
//! backup path. It only removes generated transaction paths or a target which
//! was freshly re-verified as the exact Stable bundle identity.

use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    sync::Arc,
};

use serde::Deserialize;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::{
    bundle::{self, BundleInfo},
    command, error, is_not_found, is_permission_denied, CommandRunner, MacosFileKind,
    MacosFilesystem, MacosFilesystemErrorKind, MacosHost,
};
use crate::codex_desktop::{
    download::DownloadedArtifact,
    error::{InstallerError, InstallerErrorCode},
    platform::{PlatformInstallPlan, PlatformProgressSink, PreparedInstallPackage},
    types::{CpuArchitecture, DesktopPlatform, JobProgress, ProgressPhase, ReleaseDescriptor},
};

const STAGING_PREFIX: &str = ".fyagent-app-install-";
const BACKUP_PREFIX: &str = ".fyagent-app-backup-";
const STAGING_SUFFIX: &str = ".app";
const BACKUP_SUFFIX: &str = ".backup";
const MAX_DETACH_ATTEMPTS: usize = 3;
const MAX_MANAGED_VERSION_BYTES: u64 = 256 * 1024;

const MANAGED_RUNNING_APPLICATION_JXA: &str = r#"
ObjC.import('AppKit');
const args = ObjC.unwrap($.NSProcessInfo.processInfo.arguments);
const [expectedBundleIdentifier, expectedBundlePath] = args.slice(-2);
if (!expectedBundleIdentifier || !expectedBundlePath) {
  $.exit(2);
}
const applications = $.NSWorkspace.sharedWorkspace.runningApplications;
let result = 'not_running';
for (let index = 0; index < applications.count; index += 1) {
  const application = applications.objectAtIndex(index);
  if (!application.bundleIdentifier || !application.bundleURL) {
    continue;
  }
  const bundleIdentifier = ObjC.unwrap(application.bundleIdentifier);
  const bundlePath = ObjC.unwrap(application.bundleURL.path);
  if (bundleIdentifier === expectedBundleIdentifier && bundlePath === expectedBundlePath) {
    result = 'running';
    break;
  }
}
result;
"#;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManagedBundleVersionSource {
    InfoPlist,
    TraeProductJson,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManagedVersionEquivalence {
    Exact,
    DottedPrefix,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ManagedDmgProductPolicy {
    pub(crate) expected_bundle_id: &'static str,
    pub(crate) version_source: ManagedBundleVersionSource,
    pub(crate) version_equivalence: ManagedVersionEquivalence,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ManagedDmgInstallIntent {
    Fresh { parent: PathBuf },
    Update { target: PathBuf },
}

pub(crate) struct ManagedDmgInstallRequest<'a> {
    pub(crate) artifact_path: &'a Path,
    pub(crate) intent: ManagedDmgInstallIntent,
    pub(crate) product: &'a ManagedDmgProductPolicy,
    pub(crate) expected_release_version: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ManagedDmgInstallResult {
    pub(crate) target_path: PathBuf,
    pub(crate) local_version: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManagedDmgSystemIntent {
    Fresh,
    Update,
}

pub(crate) struct ManagedDmgSystemCommitRequest<'a> {
    pub(crate) artifact_path: &'a Path,
    pub(crate) target_path: &'a Path,
    pub(crate) intent: ManagedDmgSystemIntent,
    pub(crate) product: &'a ManagedDmgProductPolicy,
    pub(crate) expected_release_version: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ManagedDmgSystemSource {
    pub(crate) bundle_path: PathBuf,
    pub(crate) source_revision: [u8; 32],
    pub(crate) target_revision: [u8; 32],
    pub(crate) local_version: String,
}

#[derive(Debug)]
pub(crate) enum ManagedDmgSystemFailure<E> {
    Package(ManagedDmgFailureKind),
    Commit(E),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManagedDmgFailureKind {
    Cancelled,
    ApplicationRunning,
    PermissionDenied,
    SourceInvalid,
    TargetChanged,
    VerificationFailedRestored,
    RecoveryRequired,
    MountFailed,
    DetachFailed,
    Failed,
}

#[derive(Debug)]
pub(crate) struct ManagedDmgFailure {
    kind: ManagedDmgFailureKind,
}

impl ManagedDmgFailure {
    pub(crate) const fn kind(&self) -> ManagedDmgFailureKind {
        self.kind
    }
}

pub(crate) fn preflight(
    runner: &dyn CommandRunner,
    filesystem: &dyn MacosFilesystem,
    host: &MacosHost,
    release: &ReleaseDescriptor,
    temp_root: &Path,
) -> Result<PlatformInstallPlan, InstallerError> {
    validate_release(release)?;
    if filesystem.file_kind(temp_root) != Ok(MacosFileKind::Directory) {
        return Err(error(
            InstallerErrorCode::InternalError,
            "installer temporary root is not an available directory",
        ));
    }

    let stable_bundles = bundle::scan_stable_bundles(runner, filesystem, host)?;
    match stable_bundles.as_slice() {
        [] => install_preflight_target(filesystem, host.user_applications_dir()),
        [existing] => {
            bundle::ensure_not_running(runner, filesystem, existing.bundle_path())?;
            let parent = existing.bundle_path().parent().ok_or_else(|| {
                error(
                    InstallerErrorCode::InternalError,
                    "installed Stable bundle has no parent directory",
                )
            })?;
            install_preflight_target(filesystem, parent)
        }
        _ => Err(error(
            InstallerErrorCode::MacMultipleInstallations,
            "multiple Stable macOS bundles prevent a safe update",
        )),
    }
}

fn install_preflight_target(
    filesystem: &dyn MacosFilesystem,
    target: &Path,
) -> Result<PlatformInstallPlan, InstallerError> {
    for ancestor in target.ancestors() {
        match filesystem.file_kind(ancestor) {
            Ok(MacosFileKind::Directory) => {
                if !filesystem.directory_writable(ancestor) {
                    return Err(error(
                        InstallerErrorCode::MacCopyFailed,
                        "the selected installation directory is not writable",
                    )
                    .with_platform_error_code("target_permission_denied"));
                }
                return Ok(PlatformInstallPlan::new(vec![ancestor.to_path_buf()])
                    .with_confirmation_target(target.to_path_buf()));
            }
            Err(error) if is_not_found(error) => continue,
            _ => {
                return Err(error(
                    InstallerErrorCode::MacTargetPathConflict,
                    "the selected installation directory is unavailable",
                ))
            }
        }
    }
    Err(error(
        InstallerErrorCode::MacTargetPathConflict,
        "the selected installation directory is unavailable",
    ))
}

/// Bind a downloader-owned fixed DMG to its locally computed handoff evidence.
pub(crate) fn prepare_install_package(
    runner: &dyn CommandRunner,
    filesystem: &dyn MacosFilesystem,
    host: &MacosHost,
    release: &ReleaseDescriptor,
    artifact: DownloadedArtifact,
) -> Result<PreparedInstallPackage, InstallerError> {
    validate_release(release)?;
    let artifact_path = artifact.path().to_path_buf();
    validate_downloaded_dmg(filesystem, &artifact_path)?;
    let _ = (runner, host);
    PreparedInstallPackage::from_prepared_artifact(release, artifact)
}

/// Copy a prepared DMG's single app bundle into a generated staging
/// directory on the destination volume, then perform a compensating swap.
/// Existing Stable installations retain their actual location and basename.
pub(crate) fn install_current_user(
    runner: &dyn CommandRunner,
    filesystem: &dyn MacosFilesystem,
    host: &MacosHost,
    package: &PreparedInstallPackage,
    progress: PlatformProgressSink,
) -> Result<crate::codex_desktop::types::InstalledApplication, InstallerError> {
    if package.platform() != DesktopPlatform::Macos
        || package.architecture() != CpuArchitecture::Aarch64
    {
        return Err(error(
            InstallerErrorCode::InternalError,
            "non-macOS prepared package reached the macOS installer",
        ));
    }
    // Bind the downloader-owned fixed DMG immediately before `hdiutil`
    // resolves the path. This is a path/capability check, not a SHA reread.
    progress.report_progress(JobProgress::new(
        ProgressPhase::Installation,
        Some(0),
        Some(3),
    ));
    package.revalidate_artifact()?;
    validate_downloaded_dmg(filesystem, package.artifact_path())?;

    let mut mounted = mount_dmg(runner, filesystem, package.artifact_path())?;
    let result = (|| {
        let source_bundle = discover_single_bundle(runner, filesystem, mounted.mount_point())?;
        super::signature::verify_bundle_signature(runner, source_bundle.bundle_path())?;
        bundle::validate_stable_bundle(runner, filesystem, host, &source_bundle, None)?;
        let policy = BundleTransactionPolicy::Codex { host };
        let source_bundle = policy.inspect_source_info(filesystem, source_bundle)?;
        let mut targets = plan_targets(runner, filesystem, host, &source_bundle, &policy)?;
        if let Some(confirmed) = &package.confirmed_target {
            targets.retain(|target| {
                confirmed.target_root.as_ref() == Some(&target.parent)
                    && match (&confirmed.local, &target.existing) {
                        (
                            crate::codex_desktop::types::LocalInstallStatus::NotInstalled {
                                ..
                            },
                            None,
                        ) => true,
                        (
                            crate::codex_desktop::types::LocalInstallStatus::Installed {
                                application,
                            },
                            Some(existing),
                        ) => *application == bundle::installed_application(&existing.info),
                        _ => false,
                    }
            });
            if targets.len() != 1 {
                return Err(error(
                    InstallerErrorCode::MetadataChanged,
                    "the confirmed installation target changed before commit",
                ));
            }
        }
        let mut last_permission_error = None;

        for target in targets {
            match install_at_target(
                runner,
                filesystem,
                &source_bundle,
                &target,
                &policy,
                progress.clone(),
                TransactionHooks {
                    before_commit: || Ok(()),
                    post_commit_verify: |_: &VerifiedBundle| Ok(()),
                },
            ) {
                Ok(installed) => return Ok(installed),
                Err(attempt)
                    if attempt.is_permission_denied() && target.allows_permission_fallback =>
                {
                    last_permission_error = Some(attempt.into_installer_error());
                }
                Err(attempt) => return Err(attempt.into_installer_error()),
            }
        }
        Err(last_permission_error.unwrap_or_else(|| {
            error(
                InstallerErrorCode::MacTargetPathConflict,
                "no safe macOS application target path is available",
            )
        }))
    })();
    let detach_result = mounted.detach();
    match (result, detach_result) {
        (Ok(installed), detach_result) => {
            if detach_result.is_err() {
                log::warn!("Codex 安装已成功，但磁盘映像卸载失败");
            }
            progress.report_progress(JobProgress::new(
                ProgressPhase::Installation,
                Some(3),
                Some(3),
            ));
            Ok(bundle::installed_application(&installed.info))
        }
        (Err(primary_error), _) => Err(primary_error),
    }
}

/// Reuse the Codex-tested DMG mount and replacement transaction for a
/// backend-selected managed Agent target. The caller supplies only internal
/// capabilities; no renderer path reaches this function.
pub(crate) fn install_managed_exact<BeforeCommit, PostCommitVerify>(
    runner: &dyn CommandRunner,
    filesystem: &dyn MacosFilesystem,
    request: ManagedDmgInstallRequest<'_>,
    before_commit: BeforeCommit,
    mut post_commit_verify: PostCommitVerify,
) -> Result<ManagedDmgInstallResult, ManagedDmgFailure>
where
    BeforeCommit: FnMut() -> Result<(), InstallerError>,
    PostCommitVerify: FnMut(&ManagedDmgInstallResult) -> Result<(), InstallerError>,
{
    validate_downloaded_dmg(filesystem, request.artifact_path)
        .map_err(managed_failure_from_error)?;
    let mut mounted =
        mount_dmg(runner, filesystem, request.artifact_path).map_err(managed_failure_from_error)?;
    let policy = BundleTransactionPolicy::Managed(request.product);
    let result = (|| {
        let source = discover_single_bundle(runner, filesystem, mounted.mount_point())
            .and_then(|bundle| policy.inspect_source_info(filesystem, bundle))
            .map_err(managed_failure_from_error)?;
        if request
            .expected_release_version
            .is_some_and(|expected| !policy.matches_release(&source, expected))
        {
            return Err(ManagedDmgFailure {
                kind: ManagedDmgFailureKind::SourceInvalid,
            });
        }
        let recovery_parent = match &request.intent {
            ManagedDmgInstallIntent::Fresh { parent } => parent.as_path(),
            ManagedDmgInstallIntent::Update { target } => {
                target.parent().ok_or(ManagedDmgFailure {
                    kind: ManagedDmgFailureKind::TargetChanged,
                })?
            }
        };
        recover_interrupted_updates(runner, filesystem, &policy, recovery_parent)
            .map_err(managed_failure_from_error)?;
        let target = match request.intent {
            ManagedDmgInstallIntent::Fresh { parent } => TargetPlan {
                target: parent.join(source.info.bundle_name()),
                parent,
                existing: None,
                allows_permission_fallback: false,
            },
            ManagedDmgInstallIntent::Update { target } => {
                let canonical =
                    filesystem
                        .canonicalize(&target)
                        .map_err(|_| ManagedDmgFailure {
                            kind: ManagedDmgFailureKind::TargetChanged,
                        })?;
                if canonical != target
                    || filesystem.file_kind(&canonical) != Ok(MacosFileKind::Directory)
                {
                    return Err(ManagedDmgFailure {
                        kind: ManagedDmgFailureKind::TargetChanged,
                    });
                }
                let parent =
                    canonical
                        .parent()
                        .map(Path::to_path_buf)
                        .ok_or(ManagedDmgFailure {
                            kind: ManagedDmgFailureKind::TargetChanged,
                        })?;
                let existing = policy
                    .inspect_existing(runner, filesystem, &canonical)
                    .map_err(managed_failure_from_error)?;
                policy
                    .ensure_not_running(runner, filesystem, &canonical)
                    .map_err(managed_failure_from_error)?;
                TargetPlan {
                    parent,
                    target: canonical,
                    existing: Some(existing),
                    allows_permission_fallback: false,
                }
            }
        };
        let progress: PlatformProgressSink = Arc::new(|_progress: JobProgress| {});
        let installed = install_at_target(
            runner,
            filesystem,
            &source,
            &target,
            &policy,
            progress,
            TransactionHooks {
                before_commit,
                post_commit_verify: |verified: &VerifiedBundle| {
                    post_commit_verify(&ManagedDmgInstallResult {
                        target_path: verified.info.bundle_path().to_path_buf(),
                        local_version: verified.comparison_version.clone(),
                    })
                },
            },
        )
        .map_err(managed_failure_from_transaction)?;
        Ok(ManagedDmgInstallResult {
            target_path: installed.info.bundle_path().to_path_buf(),
            local_version: installed.comparison_version,
        })
    })();
    let detach = mounted.detach();
    match (result, detach) {
        (Ok(installed), Ok(())) => Ok(installed),
        (Ok(_), Err(_)) => Err(ManagedDmgFailure {
            kind: ManagedDmgFailureKind::DetachFailed,
        }),
        (Err(error), _) => Err(error),
    }
}

/// Mount and validate a managed DMG, then hand one already-verified source
/// bundle capability to the single privileged system-commit owner. This
/// function performs no privileged filesystem mutation itself.
pub(crate) fn install_managed_system_exact<Commit, CommitError>(
    runner: &dyn CommandRunner,
    filesystem: &dyn MacosFilesystem,
    request: ManagedDmgSystemCommitRequest<'_>,
    mut commit: Commit,
) -> Result<ManagedDmgInstallResult, ManagedDmgSystemFailure<CommitError>>
where
    Commit: FnMut(ManagedDmgSystemSource) -> Result<(), CommitError>,
{
    validate_downloaded_dmg(filesystem, request.artifact_path).map_err(|error| {
        ManagedDmgSystemFailure::Package(managed_failure_kind_from_error(&error))
    })?;
    let mut mounted = mount_dmg(runner, filesystem, request.artifact_path).map_err(|error| {
        ManagedDmgSystemFailure::Package(managed_failure_kind_from_error(&error))
    })?;
    let policy = BundleTransactionPolicy::Managed(request.product);
    let result = (|| {
        let source = discover_single_bundle(runner, filesystem, mounted.mount_point())
            .and_then(|bundle| policy.inspect_source_info(filesystem, bundle))
            .map_err(|error| {
                ManagedDmgSystemFailure::Package(managed_failure_kind_from_error(&error))
            })?;
        if request
            .expected_release_version
            .is_some_and(|expected| !policy.matches_release(&source, expected))
        {
            return Err(ManagedDmgSystemFailure::Package(
                ManagedDmgFailureKind::SourceInvalid,
            ));
        }

        let target_revision = match request.intent {
            ManagedDmgSystemIntent::Fresh => match filesystem.file_kind(request.target_path) {
                Err(error) if is_not_found(error) => [0; 32],
                _ => {
                    return Err(ManagedDmgSystemFailure::Package(
                        ManagedDmgFailureKind::TargetChanged,
                    ))
                }
            },
            ManagedDmgSystemIntent::Update => {
                let canonical = filesystem.canonicalize(request.target_path).map_err(|_| {
                    ManagedDmgSystemFailure::Package(ManagedDmgFailureKind::TargetChanged)
                })?;
                if canonical != request.target_path
                    || filesystem.file_kind(&canonical) != Ok(MacosFileKind::Directory)
                {
                    return Err(ManagedDmgSystemFailure::Package(
                        ManagedDmgFailureKind::TargetChanged,
                    ));
                }
                let existing = policy
                    .inspect_existing(runner, filesystem, &canonical)
                    .map_err(|error| {
                        ManagedDmgSystemFailure::Package(managed_failure_kind_from_error(&error))
                    })?;
                policy
                    .ensure_not_running(runner, filesystem, &canonical)
                    .map_err(|error| {
                        ManagedDmgSystemFailure::Package(managed_failure_kind_from_error(&error))
                    })?;
                helper_bundle_revision(&existing)
            }
        };

        let source_revision = helper_bundle_revision(&source);
        commit(ManagedDmgSystemSource {
            bundle_path: source.info.bundle_path().to_path_buf(),
            source_revision,
            target_revision,
            local_version: source.comparison_version.clone(),
        })
        .map_err(ManagedDmgSystemFailure::Commit)?;

        let canonical_target = filesystem.canonicalize(request.target_path).map_err(|_| {
            ManagedDmgSystemFailure::Package(ManagedDmgFailureKind::RecoveryRequired)
        })?;
        if canonical_target != request.target_path
            || filesystem.file_kind(&canonical_target) != Ok(MacosFileKind::Directory)
        {
            return Err(ManagedDmgSystemFailure::Package(
                ManagedDmgFailureKind::RecoveryRequired,
            ));
        }
        let installed = policy
            .inspect_existing(runner, filesystem, &canonical_target)
            .map_err(|_| {
                ManagedDmgSystemFailure::Package(ManagedDmgFailureKind::RecoveryRequired)
            })?;
        if !policy.copies_are_equivalent(&source, &installed) {
            return Err(ManagedDmgSystemFailure::Package(
                ManagedDmgFailureKind::RecoveryRequired,
            ));
        }
        Ok(ManagedDmgInstallResult {
            target_path: canonical_target,
            local_version: installed.comparison_version,
        })
    })();
    let detach = mounted.detach();
    match (result, detach) {
        (Ok(installed), Ok(())) => Ok(installed),
        (Ok(_), Err(_)) => Err(ManagedDmgSystemFailure::Package(
            ManagedDmgFailureKind::DetachFailed,
        )),
        (Err(error), _) => Err(error),
    }
}

fn helper_bundle_revision(bundle: &VerifiedBundle) -> [u8; 32] {
    let canonical = format!(
        "bundleId={}\nversion={}\nexecutable={}\n",
        bundle.info.bundle_identifier(),
        bundle.comparison_version,
        bundle.info.executable(),
    );
    Sha256::digest(canonical.as_bytes()).into()
}

fn managed_failure_from_transaction(error: TransactionFailure) -> ManagedDmgFailure {
    let kind = match error {
        TransactionFailure::PermissionDenied(_) => ManagedDmgFailureKind::PermissionDenied,
        TransactionFailure::Restored(_) => ManagedDmgFailureKind::VerificationFailedRestored,
        TransactionFailure::RecoveryRequired(_) => ManagedDmgFailureKind::RecoveryRequired,
        TransactionFailure::Cancelled(_) => ManagedDmgFailureKind::Cancelled,
        TransactionFailure::Terminal(error) => managed_failure_kind_from_error(&error),
    };
    ManagedDmgFailure { kind }
}

fn managed_failure_from_error(error: InstallerError) -> ManagedDmgFailure {
    ManagedDmgFailure {
        kind: managed_failure_kind_from_error(&error),
    }
}

fn managed_failure_kind_from_error(error: &InstallerError) -> ManagedDmgFailureKind {
    match error.code() {
        InstallerErrorCode::DownloadCancelled => ManagedDmgFailureKind::Cancelled,
        InstallerErrorCode::MacAppRunning => ManagedDmgFailureKind::ApplicationRunning,
        InstallerErrorCode::MacBundleIdMismatch
        | InstallerErrorCode::PackageParseFailed
        | InstallerErrorCode::MacAppNotFound => ManagedDmgFailureKind::SourceInvalid,
        InstallerErrorCode::MacTargetPathConflict => ManagedDmgFailureKind::TargetChanged,
        InstallerErrorCode::MacDmgMountFailed => ManagedDmgFailureKind::MountFailed,
        InstallerErrorCode::MacDmgDetachFailed => ManagedDmgFailureKind::DetachFailed,
        _ => ManagedDmgFailureKind::Failed,
    }
}

struct MountedDmg<'a> {
    runner: &'a dyn CommandRunner,
    mount_point: PathBuf,
    detach_attempted: bool,
    detached: bool,
}

impl MountedDmg<'_> {
    fn mount_point(&self) -> &Path {
        &self.mount_point
    }

    fn detach(&mut self) -> Result<(), InstallerError> {
        self.detach_attempted = true;
        for _ in 0..MAX_DETACH_ATTEMPTS {
            let output = self.runner.run(&command(
                "/usr/bin/hdiutil",
                vec![
                    OsString::from("detach"),
                    self.mount_point.clone().into_os_string(),
                ],
            ));
            if matches!(output, Ok(output) if output.is_success()) {
                self.detached = true;
                return Ok(());
            }
        }
        Err(error(
            InstallerErrorCode::MacDmgDetachFailed,
            "disk image could not be detached cleanly",
        ))
    }
}

impl Drop for MountedDmg<'_> {
    fn drop(&mut self) {
        if self.detached || self.detach_attempted {
            return;
        }
        let _ = self.runner.run(&command(
            "/usr/bin/hdiutil",
            vec![
                OsString::from("detach"),
                self.mount_point.clone().into_os_string(),
            ],
        ));
    }
}

fn mount_dmg<'a>(
    runner: &'a dyn CommandRunner,
    filesystem: &dyn MacosFilesystem,
    artifact_path: &Path,
) -> Result<MountedDmg<'a>, InstallerError> {
    let output = runner
        .run(&command(
            "/usr/bin/hdiutil",
            vec![
                OsString::from("attach"),
                artifact_path.to_path_buf().into_os_string(),
                OsString::from("-readonly"),
                OsString::from("-nobrowse"),
                OsString::from("-plist"),
            ],
        ))
        .map_err(|_| {
            error(
                InstallerErrorCode::MacDmgMountFailed,
                "disk image could not be attached",
            )
        })?;
    if !output.is_success() {
        return Err(error(
            InstallerErrorCode::MacDmgMountFailed,
            "disk image attach was rejected",
        ));
    }
    // Create the guard before touching the filesystem. Once `hdiutil attach`
    // succeeds, every later validation error must still attempt a detach.
    let raw_mount_point = parse_mount_point_plist(output.stdout())?;
    let mut mounted = MountedDmg {
        runner,
        mount_point: raw_mount_point,
        detach_attempted: false,
        detached: false,
    };
    let mount_point = filesystem
        .canonicalize(mounted.mount_point())
        .map_err(|_| {
            error(
                InstallerErrorCode::MacDmgMountFailed,
                "disk image mount point could not be canonicalized",
            )
        })?;
    if filesystem.file_kind(&mount_point) != Ok(MacosFileKind::Directory) {
        return Err(error(
            InstallerErrorCode::MacDmgMountFailed,
            "disk image mount point is not a directory",
        ));
    }
    mounted.mount_point = mount_point;
    Ok(mounted)
}

fn discover_single_bundle(
    runner: &dyn CommandRunner,
    filesystem: &dyn MacosFilesystem,
    mount_point: &Path,
) -> Result<BundleInfo, InstallerError> {
    let mut candidates = Vec::new();
    for entry in filesystem.read_dir(mount_point).map_err(|_| {
        error(
            InstallerErrorCode::MacAppNotFound,
            "disk image mount point could not be enumerated",
        )
    })? {
        if let Some(bundle) = bundle::canonical_top_level_bundle(filesystem, mount_point, &entry)? {
            candidates.push(bundle);
        }
    }
    let bundle_path = match candidates.as_slice() {
        [] => {
            return Err(error(
                InstallerErrorCode::MacAppNotFound,
                "disk image did not contain a top-level application bundle",
            ))
        }
        [bundle] => bundle,
        _ => {
            return Err(error(
                InstallerErrorCode::PackageParseFailed,
                "disk image contained multiple top-level application bundles",
            ))
        }
    };
    bundle::read_bundle_info(runner, filesystem, bundle_path)
}

#[derive(Debug, Clone)]
struct VerifiedBundle {
    info: BundleInfo,
    comparison_version: String,
    identity: super::FileIdentity,
}

fn bundle_identity(
    filesystem: &dyn MacosFilesystem,
    path: &Path,
) -> Result<super::FileIdentity, InstallerError> {
    filesystem.identity(path).map_err(|_| {
        error(
            InstallerErrorCode::MacTargetPathConflict,
            "无法读取应用文件系统身份，已保留应用",
        )
    })
}

fn identity_conflict(first: &Path, second: &Path) -> InstallerError {
    InstallerError::new(InstallerErrorCode::MacTargetPathConflict).with_diagnostic_message(format!(
        "应用身份冲突，已停止删除并保留两份应用，请人工检查：{}；{}",
        first.display(),
        second.display()
    ))
}

enum BundleTransactionPolicy<'a> {
    Codex { host: &'a MacosHost },
    Managed(&'a ManagedDmgProductPolicy),
}

impl BundleTransactionPolicy<'_> {
    fn inspect_source(
        &self,
        runner: &dyn CommandRunner,
        filesystem: &dyn MacosFilesystem,
        path: &Path,
    ) -> Result<VerifiedBundle, InstallerError> {
        let info = bundle::read_bundle_info(runner, filesystem, path)?;
        self.inspect_source_info(filesystem, info)
    }

    fn inspect_source_info(
        &self,
        filesystem: &dyn MacosFilesystem,
        info: BundleInfo,
    ) -> Result<VerifiedBundle, InstallerError> {
        match self {
            Self::Codex { .. } => Ok(VerifiedBundle {
                identity: bundle_identity(filesystem, info.bundle_path())?,
                comparison_version: info.platform_version().canonical(),
                info,
            }),
            Self::Managed(policy) => inspect_managed_bundle(filesystem, info, policy),
        }
    }

    fn inspect_existing(
        &self,
        runner: &dyn CommandRunner,
        filesystem: &dyn MacosFilesystem,
        path: &Path,
    ) -> Result<VerifiedBundle, InstallerError> {
        let verified = self.inspect_source(runner, filesystem, path)?;
        if let Self::Codex { host } = self {
            bundle::validate_stable_bundle(runner, filesystem, host, &verified.info, None)?;
        }
        Ok(verified)
    }

    fn ensure_not_running(
        &self,
        runner: &dyn CommandRunner,
        filesystem: &dyn MacosFilesystem,
        path: &Path,
    ) -> Result<(), InstallerError> {
        match self {
            Self::Codex { .. } => bundle::ensure_not_running(runner, filesystem, path),
            Self::Managed(policy) => ensure_managed_bundle_not_running(
                runner,
                filesystem,
                path,
                policy.expected_bundle_id,
            ),
        }
    }

    fn copies_are_equivalent(&self, expected: &VerifiedBundle, actual: &VerifiedBundle) -> bool {
        if expected.info.bundle_identifier() != actual.info.bundle_identifier() {
            return false;
        }
        expected.comparison_version == actual.comparison_version
            && expected.info.bundle_version() == actual.info.bundle_version()
            && expected.info.display_version() == actual.info.display_version()
    }

    fn matches_release(&self, source: &VerifiedBundle, expected_release_version: &str) -> bool {
        match self {
            Self::Codex { .. } => source.comparison_version == expected_release_version,
            Self::Managed(policy) => versions_equivalent(
                &source.comparison_version,
                expected_release_version,
                policy.version_equivalence,
            ),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TraeProductVersion {
    tron_build_version: Option<String>,
}

fn inspect_managed_bundle(
    filesystem: &dyn MacosFilesystem,
    info: BundleInfo,
    policy: &ManagedDmgProductPolicy,
) -> Result<VerifiedBundle, InstallerError> {
    if info.bundle_identifier() != policy.expected_bundle_id {
        return Err(error(
            InstallerErrorCode::MacBundleIdMismatch,
            "managed application bundle identifier did not match the product policy",
        ));
    }
    let version = match policy.version_source {
        ManagedBundleVersionSource::InfoPlist => info
            .display_version()
            .map(str::to_string)
            .unwrap_or_else(|| {
                info.platform_version()
                    .canonical()
                    .trim_start_matches("mac_bundle:")
                    .to_string()
            }),
        ManagedBundleVersionSource::TraeProductJson => {
            read_trae_product_version(filesystem, info.bundle_path())?
        }
    };
    if !valid_managed_version(&version) {
        return Err(error(
            InstallerErrorCode::PackageParseFailed,
            "managed application version was missing or malformed",
        ));
    }
    Ok(VerifiedBundle {
        identity: bundle_identity(filesystem, info.bundle_path())?,
        info,
        comparison_version: version,
    })
}

fn read_trae_product_version(
    filesystem: &dyn MacosFilesystem,
    bundle_path: &Path,
) -> Result<String, InstallerError> {
    let path = bundle_path
        .join("Contents")
        .join("Resources")
        .join("app")
        .join("product.json");
    let bytes = filesystem
        .read_file_bounded(&path, MAX_MANAGED_VERSION_BYTES)
        .map_err(|_| {
            error(
                InstallerErrorCode::PackageParseFailed,
                "TRAE product metadata could not be read",
            )
        })?;
    let parsed = serde_json::from_slice::<TraeProductVersion>(&bytes).map_err(|_| {
        error(
            InstallerErrorCode::PackageParseFailed,
            "TRAE product metadata was not valid JSON",
        )
    })?;
    parsed
        .tron_build_version
        .filter(|version| valid_managed_version(version))
        .ok_or_else(|| {
            error(
                InstallerErrorCode::PackageParseFailed,
                "TRAE product metadata omitted a valid tronBuildVersion",
            )
        })
}

fn valid_managed_version(value: &str) -> bool {
    !value.trim().is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_' | b'+'))
}

fn versions_equivalent(left: &str, right: &str, policy: ManagedVersionEquivalence) -> bool {
    match policy {
        ManagedVersionEquivalence::Exact => left == right,
        ManagedVersionEquivalence::DottedPrefix => {
            left == right
                || left
                    .strip_prefix(right)
                    .is_some_and(|suffix| suffix.starts_with('.'))
                || right
                    .strip_prefix(left)
                    .is_some_and(|suffix| suffix.starts_with('.'))
        }
    }
}

fn ensure_managed_bundle_not_running(
    runner: &dyn CommandRunner,
    filesystem: &dyn MacosFilesystem,
    bundle_path: &Path,
    expected_bundle_id: &str,
) -> Result<(), InstallerError> {
    let canonical = filesystem.canonicalize(bundle_path).map_err(|_| {
        error(
            InstallerErrorCode::MacAppRunning,
            "managed application runtime state could not be determined",
        )
    })?;
    let output = runner
        .run(&command(
            "/usr/bin/osascript",
            vec![
                OsString::from("-l"),
                OsString::from("JavaScript"),
                OsString::from("-e"),
                OsString::from(MANAGED_RUNNING_APPLICATION_JXA),
                OsString::from(expected_bundle_id),
                canonical.into_os_string(),
            ],
        ))
        .map_err(|_| {
            error(
                InstallerErrorCode::MacAppRunning,
                "managed application runtime state could not be determined",
            )
        })?;
    let stdout = std::str::from_utf8(output.stdout()).ok().map(str::trim);
    if !output.is_success() {
        return Err(error(
            InstallerErrorCode::MacAppRunning,
            "managed application runtime state could not be determined",
        ));
    }
    match stdout {
        Some("not_running") => Ok(()),
        Some("running") => Err(error(
            InstallerErrorCode::MacAppRunning,
            "the selected managed application is running",
        )),
        _ => Err(error(
            InstallerErrorCode::MacAppRunning,
            "managed application runtime state was ambiguous",
        )),
    }
}

enum TransactionFailure {
    Terminal(InstallerError),
    PermissionDenied(InstallerError),
    Restored(InstallerError),
    RecoveryRequired(InstallerError),
    Cancelled(InstallerError),
}

impl TransactionFailure {
    fn terminal(error: InstallerError) -> Self {
        Self::Terminal(error)
    }

    fn permission_denied(error: InstallerError) -> Self {
        Self::PermissionDenied(error)
    }

    fn into_installer_error(self) -> InstallerError {
        match self {
            Self::Terminal(error)
            | Self::PermissionDenied(error)
            | Self::Restored(error)
            | Self::RecoveryRequired(error)
            | Self::Cancelled(error) => error,
        }
    }

    fn is_permission_denied(&self) -> bool {
        matches!(self, Self::PermissionDenied(_))
    }
}

#[derive(Debug, Clone)]
struct TargetPlan {
    parent: PathBuf,
    target: PathBuf,
    existing: Option<VerifiedBundle>,
    allows_permission_fallback: bool,
}

struct TransactionHooks<BeforeCommit, PostCommitVerify> {
    before_commit: BeforeCommit,
    post_commit_verify: PostCommitVerify,
}

struct RestoreRequest<'a> {
    parent: &'a Path,
    target: &'a Path,
    backup: &'a Path,
    expected_replacement: &'a VerifiedBundle,
    expected_backup: &'a VerifiedBundle,
    record: &'a mut UpdateRecord,
}

pub(super) fn is_transaction_staging(path: &Path) -> bool {
    named_transaction(path).is_some_and(|(_, _, staging)| staging)
        || path
            .file_name()
            .and_then(|name| name.to_str())
            .and_then(|name| name.strip_prefix(STAGING_PREFIX))
            .and_then(|name| name.strip_suffix(STAGING_SUFFIX))
            .is_some_and(|id| {
                Uuid::parse_str(id).is_ok()
                    || id.split_once('-').is_some_and(|(time, uuid)| {
                        time.parse::<u128>().is_ok() && Uuid::parse_str(uuid).is_ok()
                    })
            })
}

fn transaction_id() -> String {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{timestamp:020}-{}", Uuid::new_v4())
}

#[cfg(test)]
fn recovery_backup_path(target: &Path) -> Result<PathBuf, InstallerError> {
    transaction_path(target, &transaction_id(), false)
}

fn transaction_path(target: &Path, id: &str, staging: bool) -> Result<PathBuf, InstallerError> {
    let name = target
        .file_name()
        .and_then(|n| n.to_str())
        .filter(|n| valid_recovery_basename(n))
        .ok_or_else(|| identity_conflict(target, target))?;
    if !target.is_absolute()
        || target
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
        || parse_transaction_id(id).is_none()
    {
        return Err(identity_conflict(target, target));
    }
    let (prefix, suffix) = if staging {
        (STAGING_PREFIX, STAGING_SUFFIX)
    } else {
        (BACKUP_PREFIX, BACKUP_SUFFIX)
    };
    Ok(target.with_file_name(format!("{prefix}{name}--{id}{suffix}")))
}

fn parse_transaction_id(id: &str) -> Option<u128> {
    let (time, uuid) = id.split_once('-')?;
    Uuid::parse_str(uuid).ok()?;
    time.parse().ok()
}

// Target and transaction identity come from the directory entry, never JSON.
fn named_transaction(path: &Path) -> Option<(&str, &str, bool)> {
    let name = path.file_name()?.to_str()?;
    let (body, staging) = if let Some(body) = name
        .strip_prefix(STAGING_PREFIX)
        .and_then(|n| n.strip_suffix(STAGING_SUFFIX))
    {
        (body, true)
    } else {
        (
            name.strip_prefix(BACKUP_PREFIX)?
                .strip_suffix(BACKUP_SUFFIX)?,
            false,
        )
    };
    let (target, id) = body.rsplit_once("--")?;
    if !valid_recovery_basename(target) {
        return None;
    }
    parse_transaction_id(id)?;
    Some((target, id, staging))
}

fn valid_recovery_basename(name: &str) -> bool {
    name.ends_with(".app") && !name.starts_with('.') && !name.contains(['/', '\\', '\0'])
}

fn recovery_candidate(path: &Path) -> Option<(&str, u128)> {
    let name = path
        .file_name()?
        .to_str()?
        .strip_prefix(BACKUP_PREFIX)?
        .strip_suffix(BACKUP_SUFFIX)?;
    if let Some((base, id)) = name.rsplit_once("--") {
        if let Some((timestamp, uuid)) = id.split_once('-') {
            if valid_recovery_basename(base) && Uuid::parse_str(uuid).is_ok() {
                if let Ok(timestamp) = timestamp.parse() {
                    return Some((base, timestamp));
                }
            }
        }
    }
    // Older deterministic backups sort before timestamped transactions.
    valid_recovery_basename(name).then_some((name, 0))
}

pub(super) fn is_transaction_backup(path: &Path) -> bool {
    recovery_candidate(path).is_some()
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct UpdateRecord {
    installer_prefix: String,
    transaction_id: String,
    target: PathBuf,
    backup: PathBuf,
    backup_identity: super::FileIdentity,
    phase: UpdatePhase,
    #[serde(skip)]
    record_file: PathBuf,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
enum UpdatePhase {
    Staged,
    Swapped,
    RollingBack,
    Committed,
    RolledBack,
}

fn fixed_record_path(target: &Path, id: &str) -> Result<PathBuf, InstallerError> {
    let backup = transaction_path(target, id, false)?;
    let name = backup
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| identity_conflict(target, &backup))?;
    Ok(backup.with_file_name(format!("{name}.transaction.json")))
}

fn record_path(backup: &Path) -> Result<PathBuf, InstallerError> {
    let (name, id, _) =
        named_transaction(backup).ok_or_else(|| identity_conflict(backup, backup))?;
    fixed_record_path(&backup.with_file_name(name), id)
}

impl UpdateRecord {
    fn new(
        target: &Path,
        backup: &Path,
        identity: super::FileIdentity,
    ) -> Result<Self, InstallerError> {
        let (_, id, staging) =
            named_transaction(backup).ok_or_else(|| identity_conflict(target, backup))?;
        if transaction_path(target, id, staging)? != backup {
            return Err(identity_conflict(target, backup));
        }
        Ok(Self {
            installer_prefix: BACKUP_PREFIX.into(),
            transaction_id: id.into(),
            target: target.into(),
            backup: backup.into(),
            backup_identity: identity,
            phase: UpdatePhase::Staged,
            record_file: fixed_record_path(target, id)?,
        })
    }

    fn binding(&self) -> Result<Vec<u8>, InstallerError> {
        // Length-delimited canonical fields bind the independent proof to the
        // name-derived id, target, backup mode and original directory object.
        // The separate binding and terminal files are witnesses outside the JSON
        // trust boundary, not authentication against an owner rewriting the whole
        // Applications directory. Missing witnesses never authorize recovery.
        let fields = serde_json::to_vec(&(
            self.installer_prefix.as_str(),
            self.transaction_id.as_str(),
            &self.target,
            &self.backup,
            self.backup_identity,
        ))
        .map_err(|_| identity_conflict(&self.target, &self.backup))?;
        Ok(Sha256::digest(fields).to_vec())
    }

    fn auxiliary_path(&self, extension: &str) -> PathBuf {
        self.record_file.with_extension(extension)
    }

    fn validate_paths(&self) -> Result<(), InstallerError> {
        let expected = Self::new(&self.target, &self.backup, self.backup_identity)?;
        if self.installer_prefix != BACKUP_PREFIX
            || self.transaction_id != expected.transaction_id
            || self.record_file != expected.record_file
        {
            return Err(identity_conflict(&self.target, &self.backup));
        }
        Ok(())
    }

    fn save(&self, fs: &dyn MacosFilesystem) -> Result<(), InstallerError> {
        self.validate_paths()?;
        if matches!(self.phase, UpdatePhase::Committed) {
            match fs.file_kind(&self.auxiliary_path("terminal")) {
                Ok(_) => {}
                Err(reason) if is_not_found(reason) => self.retire(fs)?,
                Err(_) => {
                    return Err(error(
                        InstallerErrorCode::MacCopyFailed,
                        "无法确认更新终态标记，已保留备份",
                    ))
                }
            }
        }
        let failure = || {
            error(
                InstallerErrorCode::MacCopyFailed,
                "更新事务记录写入失败，已保留应用与备份",
            )
        };
        if matches!(self.phase, UpdatePhase::Staged) {
            fs.write_record_atomic(&self.auxiliary_path("binding"), &self.binding()?)
                .map_err(|_| failure())?;
        }
        let data = serde_json::to_vec(self).map_err(|_| failure())?;
        fs.write_record_atomic(&self.record_file, &data)
            .map_err(|_| failure())
    }

    // This independent tombstone is persisted only AFTER all replacement and
    // inventory verification succeeds. It blocks Staged/Swapped recovery; an
    // explicitly persisted RollingBack phase retains bound recovery authority.
    // If persistence fails we must roll back, never report a committed install.
    fn retire(&self, fs: &dyn MacosFilesystem) -> Result<(), InstallerError> {
        self.validate_paths()?;
        fs.write_record_atomic(&self.auxiliary_path("terminal"), b"retired")
            .map_err(|_| {
                error(
                    InstallerErrorCode::MacCopyFailed,
                    "更新终态标记写入失败，必须回滚并保留未清理备份",
                )
            })
    }

    fn begin_rollback(&mut self, fs: &dyn MacosFilesystem) -> Result<(), InstallerError> {
        // Persist recovery authority before revoking any legacy terminal marker
        // or touching either application. A crash here must remain recoverable.
        self.advance(fs, UpdatePhase::RollingBack)?;
        if let Err(reason) = fs.remove_record(&self.auxiliary_path("terminal")) {
            if !is_not_found(reason) {
                return Err(error(
                    InstallerErrorCode::MacCopyFailed,
                    "无法撤销更新终态标记，已保留新版与旧版备份，未执行破坏性回滚",
                ));
            }
        }
        Ok(())
    }

    fn permits_recovery(&self, fs: &dyn MacosFilesystem) -> bool {
        matches!(self.phase, UpdatePhase::RollingBack)
            || (matches!(self.phase, UpdatePhase::Staged | UpdatePhase::Swapped)
                && fs
                    .file_kind(&self.auxiliary_path("terminal"))
                    .is_err_and(is_not_found))
    }

    fn advance(
        &mut self,
        fs: &dyn MacosFilesystem,
        phase: UpdatePhase,
    ) -> Result<(), InstallerError> {
        self.phase = phase;
        self.save(fs)
    }
    fn remove(&self, fs: &dyn MacosFilesystem) -> Result<(), InstallerError> {
        self.validate_paths()?;
        // Remove the authority first; any subsequent cleanup failure remains
        // fail-closed, including a terminal marker that cannot be removed.
        for path in [
            &self.record_file,
            &self.auxiliary_path("binding"),
            &self.auxiliary_path("terminal"),
        ] {
            if let Err(reason) = fs.remove_record(path) {
                if !is_not_found(reason) {
                    return Err(error(
                        InstallerErrorCode::MacCopyFailed,
                        "更新事务记录清理失败",
                    ));
                }
            }
        }
        Ok(())
    }
}

fn read_record(fs: &dyn MacosFilesystem, backup: &Path) -> Option<UpdateRecord> {
    let path = record_path(backup).ok()?;
    let mut record: UpdateRecord =
        serde_json::from_slice(&fs.read_file_bounded(&path, 16 * 1024).ok()?).ok()?;
    record.record_file = path;
    Some(record)
}

fn recover_interrupted_updates(
    runner: &dyn CommandRunner,
    filesystem: &dyn MacosFilesystem,
    policy: &BundleTransactionPolicy<'_>,
    parent: &Path,
) -> Result<(), InstallerError> {
    let entries = match filesystem.read_dir(parent) {
        Ok(entries) => entries,
        Err(error) if is_not_found(error) => return Ok(()),
        Err(_) => {
            return Err(error(
                InstallerErrorCode::MacCopyFailed,
                "recovery directory could not be inspected",
            ))
        }
    };
    let parent = filesystem.canonicalize(parent).map_err(|_| {
        error(
            InstallerErrorCode::MacCopyFailed,
            "recovery directory could not be canonicalized",
        )
    })?;
    let mut candidates = entries
        .into_iter()
        .filter_map(|entry| {
            if let Some((name, id, _)) = named_transaction(&entry) {
                return Some((parse_transaction_id(id)?, parent.join(name), entry));
            }
            None
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| b.2.cmp(&a.2)));
    for (_, target, entry) in candidates {
        match filesystem.file_kind(&target) {
            Err(error) if is_not_found(error) => {}
            Ok(_) => continue,
            Err(_) => return Err(identity_conflict(&target, &entry)),
        }
        let Some(mut record) = read_record(filesystem, &entry).filter(|record| {
            record.validate_paths().is_ok()
                && record.target == target
                && target.parent() == Some(parent.as_path())
                && record.backup == entry
                && record.permits_recovery(filesystem)
                && filesystem.identity(&entry).ok() == Some(record.backup_identity)
                && record.binding().ok()
                    == filesystem
                        .read_file_bounded(&record.auxiliary_path("binding"), 32)
                        .ok()
        }) else {
            log::warn!(
                "备份事务已结束、记录缺失、损坏或不一致，已保留、不恢复：{}",
                entry.display()
            );
            continue;
        };
        let verified = (|| {
            let (prefix, suffix) = if is_transaction_staging(&entry) {
                (STAGING_PREFIX, STAGING_SUFFIX)
            } else {
                (BACKUP_PREFIX, BACKUP_SUFFIX)
            };
            ensure_generated_path(filesystem, &parent, &entry, prefix, suffix)?;
            verify_transaction_bundle(runner, filesystem, policy, &entry)
        })();
        let verified = match verified {
            Ok(verified) => verified,
            Err(reason) => {
                log::warn!(
                    "中断更新备份验证失败，已保留、不恢复：{}；原因：{:?}",
                    entry.display(),
                    reason
                );
                continue;
            }
        };
        // Recheck both paths after external verification commands.
        if filesystem.file_kind(&target).is_ok() {
            continue;
        }
        target_is_available(filesystem, &target)?;
        if verified.identity != record.backup_identity
            || !record.permits_recovery(filesystem)
            || filesystem.identity(&entry).ok() != Some(verified.identity)
        {
            log::warn!("中断更新备份身份变化，已保留：{}", entry.display());
            continue;
        }
        filesystem
            .rename_exclusive(&entry, &target)
            .map_err(|_| identity_conflict(&target, &entry))?;
        record.advance(filesystem, UpdatePhase::RolledBack)?;
        record.remove(filesystem)?;
    }
    Ok(())
}

fn plan_targets(
    runner: &dyn CommandRunner,
    filesystem: &dyn MacosFilesystem,
    host: &MacosHost,
    source_bundle: &VerifiedBundle,
    policy: &BundleTransactionPolicy<'_>,
) -> Result<Vec<TargetPlan>, InstallerError> {
    for parent in [host.applications_dir(), host.user_applications_dir()] {
        recover_interrupted_updates(runner, filesystem, policy, parent)?;
    }
    let installed = bundle::scan_stable_bundles(runner, filesystem, host)?;
    match installed.as_slice() {
        [existing] => {
            policy.ensure_not_running(runner, filesystem, existing.bundle_path())?;
            let existing = policy.inspect_existing(runner, filesystem, existing.bundle_path())?;
            let parent = existing.info.bundle_path().parent().ok_or_else(|| {
                error(
                    InstallerErrorCode::InternalError,
                    "installed Stable bundle has no parent directory",
                )
            })?;
            Ok(vec![TargetPlan {
                parent: parent.to_path_buf(),
                target: existing.info.bundle_path().to_path_buf(),
                existing: Some(existing),
                allows_permission_fallback: false,
            }])
        }
        [] => {
            let mut candidates = Vec::new();
            let mut blocked = 0;
            for (parent, allows_permission_fallback) in [
                (host.applications_dir(), true),
                (host.user_applications_dir(), false),
            ] {
                let target = parent.join(source_bundle.info.bundle_name());
                match target_is_available(filesystem, &target) {
                    Ok(()) => candidates.push(TargetPlan {
                        parent: parent.to_path_buf(),
                        target,
                        existing: None,
                        allows_permission_fallback,
                    }),
                    Err(error) if error.code() == InstallerErrorCode::MacTargetPathConflict => {
                        blocked += 1;
                    }
                    Err(error) => return Err(error),
                }
            }
            if candidates.is_empty() || blocked == 2 {
                return Err(error(
                    InstallerErrorCode::MacTargetPathConflict,
                    "both standard macOS application target paths are occupied",
                ));
            }
            Ok(candidates)
        }
        _ => Err(error(
            InstallerErrorCode::MacMultipleInstallations,
            "multiple Stable macOS bundles prevent a safe update",
        )),
    }
}

fn target_is_available(
    filesystem: &dyn MacosFilesystem,
    target: &Path,
) -> Result<(), InstallerError> {
    match filesystem.file_kind(target) {
        Err(error) if is_not_found(error) => Ok(()),
        Ok(_) => Err(error(
            InstallerErrorCode::MacTargetPathConflict,
            "a standard macOS application target path is occupied",
        )),
        Err(_) => Err(error(
            InstallerErrorCode::MacTargetPathConflict,
            "a standard macOS application target path could not be inspected",
        )),
    }
}

fn install_at_target<BeforeCommit, PostCommitVerify>(
    runner: &dyn CommandRunner,
    filesystem: &dyn MacosFilesystem,
    source_bundle: &VerifiedBundle,
    target: &TargetPlan,
    policy: &BundleTransactionPolicy<'_>,
    progress: PlatformProgressSink,
    hooks: TransactionHooks<BeforeCommit, PostCommitVerify>,
) -> Result<VerifiedBundle, TransactionFailure>
where
    BeforeCommit: FnMut() -> Result<(), InstallerError>,
    PostCommitVerify: FnMut(&VerifiedBundle) -> Result<(), InstallerError>,
{
    let TransactionHooks {
        mut before_commit,
        mut post_commit_verify,
    } = hooks;
    filesystem
        .create_dir_all(&target.parent)
        .map_err(|filesystem_error| {
            let error = error(
                InstallerErrorCode::MacCopyFailed,
                "target Applications directory could not be prepared",
            );
            if is_permission_denied(filesystem_error) {
                TransactionFailure::permission_denied(error)
            } else {
                TransactionFailure::terminal(error)
            }
        })?;
    let parent = filesystem
        .canonicalize(&target.parent)
        .map_err(|filesystem_error| {
            let error = error(
                InstallerErrorCode::MacCopyFailed,
                "target Applications directory could not be canonicalized",
            );
            if is_permission_denied(filesystem_error) {
                TransactionFailure::permission_denied(error)
            } else {
                TransactionFailure::terminal(error)
            }
        })?;
    if filesystem.file_kind(&parent) != Ok(MacosFileKind::Directory) {
        return Err(TransactionFailure::terminal(error(
            InstallerErrorCode::MacCopyFailed,
            "target Applications path is not a directory",
        )));
    }
    let target_path = target
        .existing
        .as_ref()
        .map(|existing| existing.info.bundle_path().to_path_buf())
        .unwrap_or_else(|| parent.join(source_bundle.info.bundle_name()));
    if let Some(existing) = &target.existing {
        if target_path != target.target || target_path != existing.info.bundle_path() {
            return Err(TransactionFailure::terminal(error(
                InstallerErrorCode::InternalError,
                "existing Stable bundle target changed during installation planning",
            )));
        }
    } else {
        target_is_available(filesystem, &target_path).map_err(TransactionFailure::terminal)?;
    }

    let transaction_id = transaction_id();
    let staging = transaction_path(&target_path, &transaction_id, true)
        .map_err(TransactionFailure::terminal)?;
    let backup = transaction_path(&target_path, &transaction_id, false)
        .map_err(TransactionFailure::terminal)?;
    ensure_generated_path_absent(
        filesystem,
        &parent,
        &staging,
        STAGING_PREFIX,
        STAGING_SUFFIX,
    )
    .map_err(TransactionFailure::terminal)?;
    ensure_generated_path_absent(filesystem, &parent, &backup, BACKUP_PREFIX, BACKUP_SUFFIX)
        .map_err(TransactionFailure::terminal)?;

    // `create_dir_all(parent)` alone does not prove a standard Applications
    // directory is writable. Probe the generated staging location first so a
    // genuinely permission-denied fresh install can fall back to the user's
    // Applications directory without interpreting an arbitrary `ditto`
    // failure as a permission condition.
    match filesystem.create_dir_all(&staging) {
        Ok(()) => {}
        Err(filesystem_error) if is_permission_denied(filesystem_error) => {
            return Err(TransactionFailure::permission_denied(error(
                InstallerErrorCode::MacCopyFailed,
                "system Applications directory is not writable for this user",
            )));
        }
        Err(_) => {
            return Err(TransactionFailure::terminal(error(
                InstallerErrorCode::MacCopyFailed,
                "generated staging directory could not be created",
            )));
        }
    }
    remove_generated_path(
        filesystem,
        &parent,
        &staging,
        STAGING_PREFIX,
        STAGING_SUFFIX,
    )
    .map_err(|_| {
        TransactionFailure::terminal(error(
            InstallerErrorCode::MacCopyFailed,
            "generated staging directory could not be removed after write probing",
        ))
    })?;

    let copy_output = match runner.run(&command(
        "/usr/bin/ditto",
        vec![
            source_bundle
                .info
                .bundle_path()
                .to_path_buf()
                .into_os_string(),
            staging.clone().into_os_string(),
        ],
    )) {
        Ok(output) => output,
        Err(_) => {
            let _ = remove_generated_path(
                filesystem,
                &parent,
                &staging,
                STAGING_PREFIX,
                STAGING_SUFFIX,
            );
            return Err(TransactionFailure::terminal(error(
                InstallerErrorCode::MacCopyFailed,
                "application bundle copy could not be started",
            )));
        }
    };
    if !copy_output.is_success() {
        let _ = remove_generated_path(
            filesystem,
            &parent,
            &staging,
            STAGING_PREFIX,
            STAGING_SUFFIX,
        );
        return Err(TransactionFailure::terminal(error(
            InstallerErrorCode::MacCopyFailed,
            "application bundle copy failed",
        )));
    }
    let staging_bundle =
        match verify_staged_bundle(runner, filesystem, policy, &parent, &staging, source_bundle) {
            Ok(bundle) => bundle,
            Err(verify_error) => {
                let _ = remove_generated_path(
                    filesystem,
                    &parent,
                    &staging,
                    STAGING_PREFIX,
                    STAGING_SUFFIX,
                );
                return Err(TransactionFailure::terminal(verify_error));
            }
        };
    progress.report_progress(JobProgress::new(
        ProgressPhase::Installation,
        Some(1),
        Some(3),
    ));

    let before_commit_result = before_commit();
    if let Err(error) = before_commit_result {
        let _ = remove_generated_path(
            filesystem,
            &parent,
            &staging,
            STAGING_PREFIX,
            STAGING_SUFFIX,
        );
        return Err(if error.code() == InstallerErrorCode::DownloadCancelled {
            TransactionFailure::Cancelled(error)
        } else {
            TransactionFailure::Terminal(error)
        });
    }

    if let Some(expected_backup) = &target.existing {
        // Re-read and re-check both identity and running state immediately
        // before moving the old application aside. This closes the user-action
        // gap between preflight and the irreversible rename.
        if let Err(error) = policy
            .inspect_existing(runner, filesystem, &target_path)
            .and_then(|_| policy.ensure_not_running(runner, filesystem, &target_path))
        {
            let _ = remove_generated_path(
                filesystem,
                &parent,
                &staging,
                STAGING_PREFIX,
                STAGING_SUFFIX,
            );
            return Err(TransactionFailure::terminal(error));
        }
        verify_before_commit(runner, filesystem, policy, &parent, &staging)?;
        let mut record = UpdateRecord::new(&target_path, &staging, expected_backup.identity)
            .map_err(TransactionFailure::terminal)?;
        record
            .save(filesystem)
            .map_err(TransactionFailure::terminal)?;
        let swapped = match filesystem.swap(&staging, &target_path) {
            Ok(()) => true,
            Err(error) if error.kind() == MacosFilesystemErrorKind::Unsupported => false,
            Err(_) => {
                let cleaned = remove_generated_path(
                    filesystem,
                    &parent,
                    &staging,
                    STAGING_PREFIX,
                    STAGING_SUFFIX,
                );
                record
                    .advance(filesystem, UpdatePhase::RolledBack)
                    .map_err(TransactionFailure::RecoveryRequired)?;
                if cleaned.is_ok() {
                    record
                        .remove(filesystem)
                        .map_err(TransactionFailure::RecoveryRequired)?;
                }
                return Err(TransactionFailure::terminal(error(
                    InstallerErrorCode::MacCopyFailed,
                    "application bundle atomic exchange failed",
                )));
            }
        };
        if !swapped {
            record
                .remove(filesystem)
                .map_err(TransactionFailure::terminal)?;
            record = UpdateRecord::new(&target_path, &backup, expected_backup.identity)
                .map_err(TransactionFailure::terminal)?;
            record
                .save(filesystem)
                .map_err(TransactionFailure::terminal)?;
            filesystem
                .rename(&target_path, &backup)
                .map_err(|filesystem_error| {
                    let cleaned = remove_generated_path(
                        filesystem,
                        &parent,
                        &staging,
                        STAGING_PREFIX,
                        STAGING_SUFFIX,
                    );
                    if let Err(error) = record.advance(filesystem, UpdatePhase::RolledBack) {
                        return TransactionFailure::RecoveryRequired(error);
                    }
                    if cleaned.is_ok() {
                        if let Err(error) = record.remove(filesystem) {
                            return TransactionFailure::RecoveryRequired(error);
                        }
                    }
                    let error = error(
                        InstallerErrorCode::MacCopyFailed,
                        "existing Stable application could not be moved to its backup",
                    );
                    if is_permission_denied(filesystem_error) {
                        TransactionFailure::permission_denied(error)
                    } else {
                        TransactionFailure::terminal(error)
                    }
                })?;
            if let Err(filesystem_error) = filesystem.rename(&staging, &target_path) {
                let restored = restore_backup(
                    runner,
                    filesystem,
                    policy,
                    RestoreRequest {
                        parent: &parent,
                        target: &target_path,
                        backup: &backup,
                        expected_replacement: &staging_bundle,
                        expected_backup,
                        record: &mut record,
                    },
                );
                let cleaned = remove_generated_path(
                    filesystem,
                    &parent,
                    &staging,
                    STAGING_PREFIX,
                    STAGING_SUFFIX,
                );
                if restored.is_ok() && cleaned.is_ok() {
                    record
                        .remove(filesystem)
                        .map_err(TransactionFailure::RecoveryRequired)?;
                }
                let replacement_error = error(
                    InstallerErrorCode::MacCopyFailed,
                    "new Stable application could not replace the existing bundle",
                );
                return Err(if let Err(conflict) = restored {
                    TransactionFailure::RecoveryRequired(conflict)
                } else {
                    if is_permission_denied(filesystem_error) {
                        TransactionFailure::permission_denied(replacement_error)
                    } else {
                        TransactionFailure::Restored(replacement_error)
                    }
                });
            }
        }
        // After RENAME_SWAP the old application is at the original staging
        // path. Never delete it before both post-commit checks succeed.
        let backup = if swapped { staging.clone() } else { backup };
        let swapped_record_error = record.advance(filesystem, UpdatePhase::Swapped).err();
        progress.report_progress(JobProgress::new(
            ProgressPhase::Installation,
            Some(2),
            Some(3),
        ));
        let installed_bundle = match verify_installed_replacement(
            runner,
            filesystem,
            policy,
            &target_path,
            &staging_bundle,
        ) {
            Ok(installed) => installed,
            Err(_) => {
                let restored = restore_backup(
                    runner,
                    filesystem,
                    policy,
                    RestoreRequest {
                        parent: &parent,
                        target: &target_path,
                        backup: &backup,
                        expected_replacement: &staging_bundle,
                        expected_backup,
                        record: &mut record,
                    },
                );
                return Err(if let Err(conflict) = restored {
                    TransactionFailure::RecoveryRequired(conflict)
                } else {
                    TransactionFailure::Restored(error(
                        InstallerErrorCode::InstallationVerifyFailed,
                        "replacement Stable application could not be verified and was restored",
                    ))
                });
            }
        };
        let post_commit_result = post_commit_verify(&installed_bundle);
        let retirement_error = if post_commit_result.is_ok() {
            record.retire(filesystem).err()
        } else {
            None
        };
        if post_commit_result.is_err() || retirement_error.is_some() {
            let restored = restore_backup(
                runner,
                filesystem,
                policy,
                RestoreRequest {
                    parent: &parent,
                    target: &target_path,
                    backup: &backup,
                    expected_replacement: &staging_bundle,
                    expected_backup,
                    record: &mut record,
                },
            );
            return Err(if let Err(conflict) = restored {
                TransactionFailure::RecoveryRequired(conflict)
            } else {
                TransactionFailure::Restored(retirement_error.unwrap_or_else(|| error(
                    InstallerErrorCode::InstallationVerifyFailed,
                    "post-install inventory verification failed and the previous application was restored",
                )))
            });
        }
        record
            .advance(filesystem, UpdatePhase::Committed)
            .map_err(|_| {
                TransactionFailure::RecoveryRequired(error(
                    InstallerErrorCode::MacCopyFailed,
                    "新应用已验证，但提交记录写入失败；旧备份已停用自动恢复并保留",
                ))
            })?;
        let (prefix, suffix) = if swapped {
            (STAGING_PREFIX, STAGING_SUFFIX)
        } else {
            (BACKUP_PREFIX, BACKUP_SUFFIX)
        };
        // Cleanup can fail partway through a recursive removal. Do not swap a
        // possibly damaged backup over the verified new application on error.
        require_expected_identity(
            runner,
            filesystem,
            policy,
            &backup,
            &target_path,
            expected_backup,
        )
        .map_err(TransactionFailure::RecoveryRequired)?;
        remove_generated_path(filesystem, &parent, &backup, prefix, suffix)
            .map_err(TransactionFailure::RecoveryRequired)?;
        record
            .remove(filesystem)
            .map_err(TransactionFailure::RecoveryRequired)?;
        if swapped_record_error.is_some() {
            return Err(TransactionFailure::RecoveryRequired(error(
                InstallerErrorCode::MacCopyFailed,
                "交换阶段记录写入失败；新应用与安装清单已验证，旧备份已清理",
            )));
        }
        Ok(installed_bundle)
    } else {
        verify_before_commit(runner, filesystem, policy, &parent, &staging)?;
        if let Err(filesystem_error) = filesystem.rename(&staging, &target_path) {
            let _ = remove_generated_path(
                filesystem,
                &parent,
                &staging,
                STAGING_PREFIX,
                STAGING_SUFFIX,
            );
            let error = error(
                InstallerErrorCode::MacCopyFailed,
                "new Stable application could not be moved into Applications",
            );
            return Err(if is_permission_denied(filesystem_error) {
                TransactionFailure::permission_denied(error)
            } else {
                TransactionFailure::terminal(error)
            });
        }
        progress.report_progress(JobProgress::new(
            ProgressPhase::Installation,
            Some(2),
            Some(3),
        ));
        let installed_bundle =
            verify_installed_replacement(runner, filesystem, policy, &target_path, &staging_bundle)
                .map_err(|_| {
                    let removed = remove_expected_replacement(
                        runner,
                        filesystem,
                        policy,
                        &parent,
                        &target_path,
                        &staging_bundle,
                    );
                    match removed {
                        Ok(()) => TransactionFailure::Restored(error(
                            InstallerErrorCode::InstallationVerifyFailed,
                            "new application could not be verified after installation",
                        )),
                        Err(conflict) => TransactionFailure::RecoveryRequired(conflict),
                    }
                })?;
        let post_commit_result = post_commit_verify(&installed_bundle);
        if post_commit_result.is_err() {
            let removed = remove_expected_replacement(
                runner,
                filesystem,
                policy,
                &parent,
                &target_path,
                &installed_bundle,
            );
            return Err(if let Err(conflict) = removed {
                TransactionFailure::RecoveryRequired(conflict)
            } else {
                TransactionFailure::Restored(error(
                    InstallerErrorCode::InstallationVerifyFailed,
                    "post-install inventory verification failed and the new application was removed",
                ))
            });
        }
        Ok(installed_bundle)
    }
}

/// No external command or callback may run between this check and the
/// filesystem commit. Running-state inspection and the cancellation hook must
/// finish first; canonicalizing a path does not freeze the staged contents.
fn verify_before_commit(
    runner: &dyn CommandRunner,
    filesystem: &dyn MacosFilesystem,
    policy: &BundleTransactionPolicy<'_>,
    parent: &Path,
    staging: &Path,
) -> Result<(), TransactionFailure> {
    if matches!(policy, BundleTransactionPolicy::Codex { .. }) {
        if let Err(error) = super::signature::verify_bundle_signature(runner, staging) {
            let _ =
                remove_generated_path(filesystem, parent, staging, STAGING_PREFIX, STAGING_SUFFIX);
            return Err(TransactionFailure::terminal(error));
        }
    }
    Ok(())
}

fn verify_staged_bundle(
    runner: &dyn CommandRunner,
    filesystem: &dyn MacosFilesystem,
    policy: &BundleTransactionPolicy<'_>,
    parent: &Path,
    staging: &Path,
    expected_source: &VerifiedBundle,
) -> Result<VerifiedBundle, InstallerError> {
    ensure_generated_path(filesystem, parent, staging, STAGING_PREFIX, STAGING_SUFFIX)?;
    let canonical_staging = filesystem.canonicalize(staging).map_err(|_| {
        error(
            InstallerErrorCode::MacCopyFailed,
            "staging application bundle could not be canonicalized",
        )
    })?;
    if canonical_staging.parent() != Some(parent) {
        return Err(error(
            InstallerErrorCode::MacCopyFailed,
            "staging application bundle escaped its target volume",
        ));
    }
    let staged = policy.inspect_source(runner, filesystem, &canonical_staging)?;
    if !policy.copies_are_equivalent(expected_source, &staged) {
        return Err(error(
            InstallerErrorCode::InstallationVerifyFailed,
            "staged application differs from the mounted source bundle",
        ));
    }
    Ok(staged)
}

fn verify_installed_replacement(
    runner: &dyn CommandRunner,
    filesystem: &dyn MacosFilesystem,
    policy: &BundleTransactionPolicy<'_>,
    target: &Path,
    expected_source: &VerifiedBundle,
) -> Result<VerifiedBundle, InstallerError> {
    let installed = policy.inspect_source(runner, filesystem, target)?;
    if expected_source.identity != installed.identity
        || !policy.copies_are_equivalent(expected_source, &installed)
    {
        return Err(error(
            InstallerErrorCode::InstallationVerifyFailed,
            "installed application differs from the staged bundle",
        ));
    }
    Ok(installed)
}

fn restore_backup(
    runner: &dyn CommandRunner,
    filesystem: &dyn MacosFilesystem,
    policy: &BundleTransactionPolicy<'_>,
    request: RestoreRequest<'_>,
) -> Result<(), InstallerError> {
    request.record.validate_paths()?;
    if request.record.target != request.target
        || request.record.backup != request.backup
        || request.record.backup_identity != request.expected_backup.identity
    {
        return Err(identity_conflict(request.target, request.backup));
    }
    require_expected_identity(
        runner,
        filesystem,
        policy,
        request.backup,
        request.target,
        request.expected_backup,
    )?;
    if filesystem.file_kind(request.target).is_ok() {
        require_expected_identity(
            runner,
            filesystem,
            policy,
            request.target,
            request.backup,
            request.expected_replacement,
        )?;
    }
    request.record.begin_rollback(filesystem)?;
    if request
        .backup
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with(STAGING_PREFIX))
    {
        ensure_generated_path(
            filesystem,
            request.parent,
            request.backup,
            STAGING_PREFIX,
            STAGING_SUFFIX,
        )?;
        filesystem
            .swap(request.backup, request.target)
            .map_err(|_| {
                error(
                    InstallerErrorCode::InstallationVerifyFailed,
                    "atomic application rollback failed; previous application retained in staging",
                )
            })?;
        let restored = policy.inspect_existing(runner, filesystem, request.target)?;
        if request.expected_backup.identity != restored.identity
            || !policy.copies_are_equivalent(request.expected_backup, &restored)
        {
            return Err(error(
                InstallerErrorCode::InstallationVerifyFailed,
                "restored application identity changed",
            ));
        }
        request
            .record
            .advance(filesystem, UpdatePhase::RolledBack)?;
        require_expected_identity(
            runner,
            filesystem,
            policy,
            request.backup,
            request.target,
            request.expected_replacement,
        )?;
        // The failed replacement is now in our generated staging directory.
        remove_generated_path(
            filesystem,
            request.parent,
            request.backup,
            STAGING_PREFIX,
            STAGING_SUFFIX,
        )?;
        request.record.remove(filesystem)?;
        return Ok(());
    }
    match filesystem.file_kind(request.target) {
        Err(filesystem_error) if is_not_found(filesystem_error) => {}
        Ok(MacosFileKind::Directory) => {
            remove_expected_replacement(
                runner,
                filesystem,
                policy,
                request.parent,
                request.target,
                request.expected_replacement,
            )?;
        }
        _ => {
            return Err(error(
                InstallerErrorCode::InstallationVerifyFailed,
                "replacement path could not be safely removed during restore",
            ))
        }
    }
    ensure_generated_path(
        filesystem,
        request.parent,
        request.backup,
        BACKUP_PREFIX,
        BACKUP_SUFFIX,
    )?;
    filesystem
        .rename(request.backup, request.target)
        .map_err(|_| {
            error(
                InstallerErrorCode::InstallationVerifyFailed,
                "Stable application backup could not be restored",
            )
        })?;
    let restored = policy.inspect_source(runner, filesystem, request.target)?;
    if request.expected_backup.identity != restored.identity
        || !policy.copies_are_equivalent(request.expected_backup, &restored)
    {
        return Err(error(
            InstallerErrorCode::InstallationVerifyFailed,
            "restored application did not match the verified backup identity",
        ));
    }
    {
        request
            .record
            .advance(filesystem, UpdatePhase::RolledBack)?;
        // A failed second rename leaves the replacement in staging. Its caller
        // removes that directory before discarding the terminal record.
        if filesystem
            .file_kind(request.expected_replacement.info.bundle_path())
            .is_err_and(is_not_found)
        {
            request.record.remove(filesystem)?;
        }
    }
    Ok(())
}

fn verify_transaction_bundle(
    runner: &dyn CommandRunner,
    filesystem: &dyn MacosFilesystem,
    policy: &BundleTransactionPolicy<'_>,
    path: &Path,
) -> Result<VerifiedBundle, InstallerError> {
    let before = bundle_identity(filesystem, path)?;
    if filesystem.canonicalize(path).ok().as_deref() != Some(path) {
        return Err(identity_conflict(path, path));
    }
    let current = policy.inspect_source(runner, filesystem, path)?;
    if matches!(policy, BundleTransactionPolicy::Codex { .. }) {
        if current.info.bundle_identifier() != super::stable_bundle_id() {
            return Err(identity_conflict(path, path));
        }
        super::signature::verify_bundle_signature(runner, path)?;
    }
    if before != current.identity || filesystem.identity(path).ok() != Some(before) {
        return Err(identity_conflict(path, path));
    }
    Ok(current)
}

fn require_expected_identity(
    runner: &dyn CommandRunner,
    filesystem: &dyn MacosFilesystem,
    policy: &BundleTransactionPolicy<'_>,
    path: &Path,
    other: &Path,
    expected: &VerifiedBundle,
) -> Result<(), InstallerError> {
    if filesystem.identity(path).ok() != Some(expected.identity) {
        return Err(identity_conflict(path, other));
    }
    let current = verify_transaction_bundle(runner, filesystem, policy, path)
        .map_err(|_| identity_conflict(path, other))?;
    if current.identity != expected.identity || !policy.copies_are_equivalent(expected, &current) {
        return Err(identity_conflict(path, other));
    }
    Ok(())
}

fn remove_expected_replacement(
    runner: &dyn CommandRunner,
    filesystem: &dyn MacosFilesystem,
    policy: &BundleTransactionPolicy<'_>,
    parent: &Path,
    target: &Path,
    expected: &VerifiedBundle,
) -> Result<(), InstallerError> {
    require_expected_identity(
        runner,
        filesystem,
        policy,
        target,
        expected.info.bundle_path(),
        expected,
    )?;
    remove_known_child(filesystem, parent, target)
}

fn remove_known_child(
    filesystem: &dyn MacosFilesystem,
    parent: &Path,
    path: &Path,
) -> Result<(), InstallerError> {
    if path.parent() != Some(parent) || filesystem.file_kind(path) != Ok(MacosFileKind::Directory) {
        return Err(error(
            InstallerErrorCode::InstallationVerifyFailed,
            "cleanup refused a non-directory or escaped application path",
        ));
    }
    filesystem.remove_dir_all(path).map_err(|_| {
        error(
            InstallerErrorCode::InstallationVerifyFailed,
            "application cleanup could not be completed",
        )
    })
}

fn ensure_generated_path_absent(
    filesystem: &dyn MacosFilesystem,
    parent: &Path,
    path: &Path,
    prefix: &str,
    suffix: &str,
) -> Result<(), InstallerError> {
    ensure_generated_path_shape(parent, path, prefix, suffix)?;
    match filesystem.file_kind(path) {
        Err(error) if is_not_found(error) => Ok(()),
        Ok(_) => Err(error(
            InstallerErrorCode::MacCopyFailed,
            "generated macOS transaction path unexpectedly already exists",
        )),
        Err(_) => Err(error(
            InstallerErrorCode::MacCopyFailed,
            "generated macOS transaction path could not be inspected",
        )),
    }
}

fn ensure_generated_path(
    filesystem: &dyn MacosFilesystem,
    parent: &Path,
    path: &Path,
    prefix: &str,
    suffix: &str,
) -> Result<(), InstallerError> {
    ensure_generated_path_shape(parent, path, prefix, suffix)?;
    if filesystem.file_kind(path) != Ok(MacosFileKind::Directory) {
        return Err(error(
            InstallerErrorCode::InstallationVerifyFailed,
            "generated macOS transaction path is not a directory",
        ));
    }
    Ok(())
}

fn ensure_generated_path_shape(
    parent: &Path,
    path: &Path,
    prefix: &str,
    suffix: &str,
) -> Result<(), InstallerError> {
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| {
            error(
                InstallerErrorCode::InternalError,
                "generated macOS transaction path has no safe filename",
            )
        })?;
    if path.parent() != Some(parent)
        || !file_name.starts_with(prefix)
        || !file_name.ends_with(suffix)
        || file_name.contains(['/', '\\', '\0'])
    {
        return Err(error(
            InstallerErrorCode::InternalError,
            "generated macOS transaction path escaped its trusted parent",
        ));
    }
    Ok(())
}

fn remove_generated_path(
    filesystem: &dyn MacosFilesystem,
    parent: &Path,
    path: &Path,
    prefix: &str,
    suffix: &str,
) -> Result<(), InstallerError> {
    ensure_generated_path(filesystem, parent, path, prefix, suffix)?;
    filesystem.remove_dir_all(path).map_err(|_| {
        error(
            InstallerErrorCode::InstallationVerifyFailed,
            "generated macOS transaction cleanup failed",
        )
    })
}

fn validate_downloaded_dmg(
    filesystem: &dyn MacosFilesystem,
    artifact_path: &Path,
) -> Result<(), InstallerError> {
    if artifact_path.file_name().and_then(|name| name.to_str()) != Some("installer.dmg")
        || filesystem.file_kind(artifact_path) != Ok(MacosFileKind::File)
    {
        return Err(error(
            InstallerErrorCode::PackageParseFailed,
            "prepared macOS package is not the fixed installer DMG artifact",
        ));
    }
    Ok(())
}

fn validate_release(release: &ReleaseDescriptor) -> Result<(), InstallerError> {
    if release.platform != DesktopPlatform::Macos
        || release.architecture != CpuArchitecture::Aarch64
        || release.download_endpoint
            != crate::codex_desktop::types::TrustedDownloadEndpoint::MacArm64
    {
        return Err(error(
            InstallerErrorCode::ArchitectureUnsupported,
            "release descriptor is not an Apple-Silicon macOS artifact",
        ));
    }
    Ok(())
}

fn parse_mount_point_plist(bytes: &[u8]) -> Result<PathBuf, InstallerError> {
    let text = std::str::from_utf8(bytes).map_err(|_| {
        error(
            InstallerErrorCode::MacDmgMountFailed,
            "disk image attach plist was not UTF-8",
        )
    })?;
    let mut values = Vec::new();
    let mut remainder = text;
    while let Some(key_start) = remainder.find("<key>") {
        remainder = &remainder[key_start + "<key>".len()..];
        let Some(key_end) = remainder.find("</key>") else {
            return Err(error(
                InstallerErrorCode::MacDmgMountFailed,
                "disk image attach plist was malformed",
            ));
        };
        let key = remainder[..key_end].trim();
        remainder = &remainder[key_end + "</key>".len()..];
        if key != "mount-point" {
            continue;
        }
        let string_start = remainder.find("<string>").ok_or_else(|| {
            error(
                InstallerErrorCode::MacDmgMountFailed,
                "disk image attach plist omitted a mount point string",
            )
        })?;
        if remainder[..string_start].contains("<key>") {
            return Err(error(
                InstallerErrorCode::MacDmgMountFailed,
                "disk image attach plist did not pair a mount point with a string",
            ));
        }
        let string_remainder = &remainder[string_start + "<string>".len()..];
        let string_end = string_remainder.find("</string>").ok_or_else(|| {
            error(
                InstallerErrorCode::MacDmgMountFailed,
                "disk image attach plist had an unterminated mount point string",
            )
        })?;
        values.push(xml_unescape(&string_remainder[..string_end])?);
        remainder = &string_remainder[string_end + "</string>".len()..];
    }
    let mount_point = match values.as_slice() {
        [mount_point] if is_absolute_macos_path(mount_point) => mount_point,
        _ => {
            return Err(error(
                InstallerErrorCode::MacDmgMountFailed,
                "disk image attach plist did not identify exactly one absolute mount point",
            ))
        }
    };
    Ok(PathBuf::from(mount_point))
}

// The adapter's tests intentionally run on non-macOS hosts. `Path::is_absolute`
// follows the *test host* syntax there, whereas an `hdiutil` plist always
// contains a POSIX macOS mount path. Keep the validation tied to the producer
// format rather than the compilation host.
fn is_absolute_macos_path(value: &str) -> bool {
    value.starts_with('/') && !value.contains('\0')
}

fn xml_unescape(value: &str) -> Result<String, InstallerError> {
    let mut result = String::with_capacity(value.len());
    let mut remainder = value;
    while let Some(index) = remainder.find('&') {
        result.push_str(&remainder[..index]);
        remainder = &remainder[index..];
        let (entity, replacement) = [
            ("&amp;", "&"),
            ("&lt;", "<"),
            ("&gt;", ">"),
            ("&quot;", "\""),
            ("&apos;", "'"),
        ]
        .into_iter()
        .find(|(entity, _)| remainder.starts_with(entity))
        .ok_or_else(|| {
            error(
                InstallerErrorCode::MacDmgMountFailed,
                "disk image mount point contained an unsupported XML entity",
            )
        })?;
        result.push_str(replacement);
        remainder = &remainder[entity.len()..];
    }
    result.push_str(remainder);
    Ok(result)
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        sync::{Arc, Mutex},
    };

    use super::*;
    use crate::codex_desktop::{
        download::DownloadedArtifact,
        platform::macos::{
            stable_bundle_id,
            test_support::{FakeFilesystem, FakeRunner},
            MacosFilesystemErrorKind,
        },
        temp::JobTempDir,
        types::{PlatformVersion, TrustedDownloadEndpoint},
        verify::ArtifactKind,
    };
    use uuid::Uuid;

    const SYSTEM_APPLICATIONS: &str = "/Applications";
    const USER_APPLICATIONS: &str = "/Users/test/Applications";
    const MOUNT_POINT: &str = "/Volumes/FyAgent Codex";
    const ARTIFACT: &str = "/tmp/fyagent-job/installer.dmg";

    #[test]
    fn managed_running_probe_uses_jxa_completion_value_not_console_log() {
        assert!(!MANAGED_RUNNING_APPLICATION_JXA.contains("console.log"));
        assert!(MANAGED_RUNNING_APPLICATION_JXA.contains("let result = 'not_running'"));
        assert!(MANAGED_RUNNING_APPLICATION_JXA.contains("result;"));
    }

    fn host() -> MacosHost {
        MacosHost::new(
            CpuArchitecture::Aarch64,
            "14.4",
            SYSTEM_APPLICATIONS.into(),
            USER_APPLICATIONS.into(),
        )
        .unwrap()
    }

    fn release(version: &str) -> ReleaseDescriptor {
        ReleaseDescriptor::new(
            DesktopPlatform::Macos,
            CpuArchitecture::Aarch64,
            "1.0",
            PlatformVersion::parse_mac_bundle(version).unwrap(),
            Some(1024),
            TrustedDownloadEndpoint::MacArm64,
        )
        .unwrap()
    }

    fn release_for_artifact(bytes: &[u8], version: &str) -> ReleaseDescriptor {
        ReleaseDescriptor::new(
            DesktopPlatform::Macos,
            CpuArchitecture::Aarch64,
            "1.0",
            PlatformVersion::parse_mac_bundle(version).unwrap(),
            Some(bytes.len() as u64),
            TrustedDownloadEndpoint::MacArm64,
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
        fs::write(directory.final_path(ArtifactKind::Dmg), bytes).unwrap();
        let artifact = DownloadedArtifact::from_test_file(&directory, release).unwrap();
        (root, artifact)
    }

    fn plist(bundle_identifier: &str, bundle_version: &str) -> Vec<u8> {
        format!(
            "{{\"CFBundleIdentifier\":\"{bundle_identifier}\",\"CFBundleVersion\":\"{bundle_version}\",\"CFBundleShortVersionString\":\"1.0\",\"CFBundleExecutable\":\"Codex\",\"LSMinimumSystemVersion\":\"14.0\"}}"
        )
        .into_bytes()
    }

    fn managed_plist(
        bundle_identifier: &str,
        bundle_version: &str,
        short_version: &str,
    ) -> Vec<u8> {
        format!(
            "{{\"CFBundleIdentifier\":\"{bundle_identifier}\",\"CFBundleVersion\":\"{bundle_version}\",\"CFBundleShortVersionString\":\"{short_version}\",\"CFBundleExecutable\":\"Codex\"}}"
        )
        .into_bytes()
    }

    fn mount_plist(mount_point: &str) -> Vec<u8> {
        format!(
            "<?xml version=\"1.0\"?><plist><dict><key>mount-point</key><string>{mount_point}</string></dict></plist>"
        )
        .into_bytes()
    }

    fn add_bundle(filesystem: &FakeFilesystem, bundle_path: &Path) {
        filesystem.add_dir(bundle_path);
        filesystem.add_file(bundle_path.join("Contents/Info.plist"));
        filesystem.add_file(bundle_path.join("Contents/MacOS/Codex"));
    }

    fn add_trae_product_json(filesystem: &FakeFilesystem, bundle_path: &Path, version: &str) {
        filesystem.add_file_with_contents(
            bundle_path
                .join("Contents")
                .join("Resources")
                .join("app")
                .join("product.json"),
            format!(r#"{{"tronBuildVersion":"{version}"}}"#).into_bytes(),
        );
    }

    fn queue_read_and_validate(runner: &FakeRunner, version: &str) {
        queue_read_with_identity(runner, stable_bundle_id(), version);
    }

    fn queue_read_with_identity(runner: &FakeRunner, bundle_id: &str, version: &str) {
        runner.queue_success("/usr/bin/plutil", plist(bundle_id, version));
    }

    fn queue_managed_read(
        runner: &FakeRunner,
        bundle_id: &str,
        bundle_version: &str,
        short_version: &str,
    ) {
        runner.queue_success(
            "/usr/bin/plutil",
            managed_plist(bundle_id, bundle_version, short_version),
        );
    }

    fn managed_policy(
        bundle_id: &'static str,
        version_source: ManagedBundleVersionSource,
        version_equivalence: ManagedVersionEquivalence,
    ) -> ManagedDmgProductPolicy {
        ManagedDmgProductPolicy {
            expected_bundle_id: bundle_id,
            version_source,
            version_equivalence,
        }
    }

    fn install_copy_hook(runner: &Arc<FakeRunner>, filesystem: &Arc<FakeFilesystem>) {
        let filesystem_for_ditto = filesystem.clone();
        runner.set_hook(Arc::new(move |invocation| {
            if invocation.program() == "/usr/bin/ditto" {
                filesystem_for_ditto
                    .copy_tree(
                        PathBuf::from(invocation.arguments()[0].clone()),
                        PathBuf::from(invocation.arguments()[1].clone()),
                    )
                    .unwrap();
            }
        }));
    }

    fn fixture_filesystem_at(artifact_path: &Path) -> (Arc<FakeFilesystem>, PathBuf) {
        let filesystem = Arc::new(FakeFilesystem::new());
        filesystem.add_file(artifact_path);
        filesystem.add_dir(MOUNT_POINT);
        let source_bundle = Path::new(MOUNT_POINT).join("ChatGPT.app");
        add_bundle(filesystem.as_ref(), &source_bundle);
        filesystem.add_dir(SYSTEM_APPLICATIONS);
        filesystem.add_dir(USER_APPLICATIONS);
        (filesystem, source_bundle)
    }

    fn fixture_filesystem() -> (Arc<FakeFilesystem>, PathBuf) {
        fixture_filesystem_at(Path::new(ARTIFACT))
    }

    fn package(release: &ReleaseDescriptor) -> PreparedInstallPackage {
        PreparedInstallPackage::for_test_at(release, PathBuf::from(ARTIFACT))
    }

    fn queue_fresh_install(runner: &FakeRunner, version: &str) {
        queue_fresh_install_with_identity(runner, stable_bundle_id(), version);
    }

    fn queue_fresh_install_with_identity(runner: &FakeRunner, bundle_id: &str, version: &str) {
        runner.queue_success("/usr/bin/hdiutil", mount_plist(MOUNT_POINT));
        queue_read_with_identity(runner, bundle_id, version); // mounted source
        super::super::signature::tests::queue_valid_signature(runner);
        runner.queue_success("/usr/bin/ditto", Vec::<u8>::new());
        queue_read_with_identity(runner, bundle_id, version); // staging
        super::super::signature::tests::queue_valid_signature(runner);
        queue_read_with_identity(runner, bundle_id, version); // target
        runner.queue_success("/usr/bin/hdiutil", Vec::<u8>::new());
    }

    #[test]
    fn update_swap_and_fallback_failure_matrix_preserves_application_contents() {
        for case in [
            "swap_ok",
            "swap_post_bad",
            "swap_metadata_bad",
            "swap_error",
            "swap_rollback_error",
            "swap_cleanup_error",
            "swap_rollback_cleanup_error",
            "fallback_ok",
            "fallback_first_error",
            "fallback_second_error",
            "fallback_restore_error",
            "fallback_post_bad",
            "fallback_cleanup_error",
        ] {
            let (filesystem, source_path) = fixture_filesystem();
            let target_path = Path::new(USER_APPLICATIONS).join("Renamed Codex.app");
            add_bundle(&filesystem, &target_path);
            filesystem.add_file_with_contents(target_path.join("old-marker"), b"old".to_vec());
            filesystem.add_file_with_contents(source_path.join("new-marker"), b"new".to_vec());
            let runner = Arc::new(FakeRunner::new());
            install_copy_hook(&runner, &filesystem);
            let host = host();
            let policy = BundleTransactionPolicy::Codex { host: &host };
            queue_read_and_validate(&runner, "2");
            let source = policy
                .inspect_source(&*runner, &*filesystem, &source_path)
                .unwrap();
            queue_read_and_validate(&runner, "1");
            let existing = policy
                .inspect_existing(&*runner, &*filesystem, &target_path)
                .unwrap();
            let target = TargetPlan {
                parent: PathBuf::from(USER_APPLICATIONS),
                target: target_path.clone(),
                existing: Some(existing),
                allows_permission_fallback: false,
            };
            let fallback = case.starts_with("fallback");
            if fallback {
                filesystem.fail_operation("swap", MacosFilesystemErrorKind::Unsupported);
            }
            match case {
                "swap_error" => {
                    filesystem.fail_operation("swap", MacosFilesystemErrorKind::PermissionDenied)
                }
                "swap_rollback_error" => {
                    filesystem.fail_operation_after("swap", 1, MacosFilesystemErrorKind::Other)
                }
                "fallback_first_error" => {
                    filesystem.fail_operation("rename", MacosFilesystemErrorKind::PermissionDenied)
                }
                "fallback_second_error" | "fallback_restore_error" => {
                    filesystem.fail_operation_after("rename", 1, MacosFilesystemErrorKind::Other);
                    if case == "fallback_restore_error" {
                        filesystem.fail_operation("rename", MacosFilesystemErrorKind::Other);
                    }
                }
                "swap_cleanup_error" | "fallback_cleanup_error" | "swap_rollback_cleanup_error" => {
                    filesystem.fail_operation_after(
                        "remove",
                        if fallback { 0 } else { 1 },
                        MacosFilesystemErrorKind::PermissionDenied,
                    );
                }
                _ => {}
            }
            runner.queue_success("/usr/bin/ditto", vec![]);
            queue_read_and_validate(&runner, "2"); // staging metadata
            queue_read_and_validate(&runner, "1"); // old target recheck
            runner.queue_success("/usr/bin/osascript", b"[]".to_vec());
            super::super::signature::tests::queue_valid_signature(&runner);
            let committed = !matches!(
                case,
                "swap_error"
                    | "fallback_first_error"
                    | "fallback_second_error"
                    | "fallback_restore_error"
            );
            let rollback = matches!(
                case,
                "swap_post_bad"
                    | "swap_metadata_bad"
                    | "swap_rollback_error"
                    | "fallback_post_bad"
                    | "swap_rollback_cleanup_error"
            );
            if committed {
                if case == "swap_metadata_bad" {
                    runner.queue_failure("/usr/bin/plutil", Some(1), "damaged metadata");
                } else {
                    queue_read_and_validate(&runner, "2");
                }
                if rollback {
                    queue_read_and_validate(&runner, "1"); // backup identity
                    super::super::signature::tests::queue_valid_signature(&runner);
                    queue_read_and_validate(&runner, "2"); // replacement before rollback
                    super::super::signature::tests::queue_valid_signature(&runner);
                    if case != "swap_rollback_error" {
                        if fallback {
                            queue_read_and_validate(&runner, "2"); // delete replacement
                            super::super::signature::tests::queue_valid_signature(&runner);
                            queue_read_and_validate(&runner, "1"); // restored
                        } else {
                            queue_read_and_validate(&runner, "1"); // restored
                            queue_read_and_validate(&runner, "2"); // swapped out replacement
                            super::super::signature::tests::queue_valid_signature(&runner);
                        }
                    }
                } else {
                    queue_read_and_validate(&runner, "1"); // old backup cleanup
                    super::super::signature::tests::queue_valid_signature(&runner);
                }
            } else if matches!(case, "fallback_second_error" | "fallback_restore_error") {
                queue_read_and_validate(&runner, "1");
                super::super::signature::tests::queue_valid_signature(&runner);
                if case == "fallback_second_error" {
                    queue_read_and_validate(&runner, "1");
                }
            }
            let result = install_at_target(
                &*runner,
                &*filesystem,
                &source,
                &target,
                &policy,
                Arc::new(|_| {}),
                TransactionHooks {
                    before_commit: || Ok(()),
                    post_commit_verify: |_: &VerifiedBundle| {
                        if rollback {
                            Err(error(
                                InstallerErrorCode::InstallationVerifyFailed,
                                "injected readback failure",
                            ))
                        } else {
                            Ok(())
                        }
                    },
                },
            );
            let calls = runner.invocations();
            let staging = PathBuf::from(
                calls
                    .iter()
                    .find(|call| call.program() == "/usr/bin/ditto")
                    .unwrap()
                    .arguments()[1]
                    .clone(),
            );
            let backup = filesystem
                .read_dir(Path::new(USER_APPLICATIONS))
                .unwrap()
                .into_iter()
                .find(|path| is_transaction_backup(path))
                .unwrap_or_else(|| recovery_backup_path(&target_path).unwrap());
            if case.ends_with("_ok") {
                assert!(result.is_ok(), "{case}");
                assert!(
                    filesystem.contains(target_path.join("new-marker")),
                    "{case}"
                );
                assert!(!filesystem.contains(&staging), "{case}");
                assert!(!filesystem.contains(&backup), "{case}");
            } else {
                assert!(result.is_err(), "{case}");
                if matches!(
                    case,
                    "swap_rollback_error" | "swap_cleanup_error" | "fallback_cleanup_error"
                ) {
                    assert!(
                        matches!(result, Err(TransactionFailure::RecoveryRequired(_))),
                        "{case}"
                    );
                    assert!(
                        filesystem.contains(target_path.join("new-marker")),
                        "{case}"
                    );
                    let retained = if fallback { &backup } else { &staging };
                    assert!(filesystem.contains(retained.join("old-marker")), "{case}");
                } else if case == "swap_rollback_cleanup_error" {
                    assert!(matches!(
                        result,
                        Err(TransactionFailure::RecoveryRequired(_))
                    ));
                    assert!(filesystem.contains(target_path.join("old-marker")));
                    assert!(filesystem.contains(staging.join("new-marker")));
                    assert!(matches!(
                        read_record(&*filesystem, &staging).unwrap().phase,
                        UpdatePhase::RolledBack
                    ));
                } else if case == "fallback_restore_error" {
                    assert!(matches!(
                        result,
                        Err(TransactionFailure::RecoveryRequired(_))
                    ));
                    assert!(filesystem.contains(backup.join("old-marker")));
                    // Simulate the next installation after a failed/crashed rollback.
                    queue_read_and_validate(&runner, "1");
                    super::super::signature::tests::queue_valid_signature(&runner);
                    recover_interrupted_updates(
                        &*runner,
                        &*filesystem,
                        &policy,
                        Path::new(USER_APPLICATIONS),
                    )
                    .unwrap();
                    assert!(filesystem.contains(target_path.join("old-marker")));
                    assert!(!filesystem.contains(&backup));
                } else {
                    assert!(
                        filesystem.contains(target_path.join("old-marker")),
                        "{case}"
                    );
                    assert!(
                        !filesystem.contains(target_path.join("new-marker")),
                        "{case}"
                    );
                    assert!(!filesystem.contains(&staging), "{case}");
                    assert!(!filesystem.contains(&backup), "{case}");
                }
            }
            if matches!(
                case,
                "swap_cleanup_error" | "fallback_cleanup_error" | "swap_rollback_cleanup_error"
            ) {
                let retained = if fallback { &backup } else { &staging };
                let phase = read_record(&*filesystem, retained).unwrap().phase;
                assert!(matches!(
                    phase,
                    UpdatePhase::Committed | UpdatePhase::RolledBack
                ));
                filesystem.remove_dir_all(&target_path).unwrap(); // user uninstalls
                recover_interrupted_updates(
                    &*runner,
                    &*filesystem,
                    &policy,
                    Path::new(USER_APPLICATIONS),
                )
                .unwrap();
                assert!(!filesystem.contains(&target_path), "{case}");
                assert!(filesystem.contains(retained), "{case}");
            }
            // The last command before the filesystem commit must be spctl;
            // the next command can only be installed/rollback metadata readback.
            let spctl = calls
                .iter()
                .position(|call| call.program() == "/usr/sbin/spctl")
                .unwrap();
            assert_eq!(calls[spctl - 3].program(), "/usr/bin/osascript", "{case}");
            assert_eq!(calls[spctl - 2].program(), "/usr/bin/codesign", "{case}");
            assert_eq!(calls[spctl - 1].program(), "/usr/bin/codesign", "{case}");
            if let Some(next) = calls.get(spctl + 1) {
                assert_eq!(next.program(), "/usr/bin/plutil", "{case}");
            }
            runner.assert_drained();
        }
    }

    #[test]
    fn recovery_only_restores_exact_installer_backup_without_overwriting_target() {
        for scenario in [
            "missing",
            "swapped",
            "swap_missing",
            "occupied",
            "symlink",
            "invalid",
            "rename_error",
        ] {
            let filesystem = FakeFilesystem::new();
            let parent = Path::new(USER_APPLICATIONS);
            filesystem.add_dir(parent);
            let target = parent.join("Renamed Codex.app");
            let backup = if scenario == "swap_missing" {
                transaction_path(&target, &transaction_id(), true).unwrap()
            } else {
                recovery_backup_path(&target).unwrap()
            };
            add_bundle(&filesystem, &backup);
            UpdateRecord::new(&target, &backup, filesystem.identity(&backup).unwrap())
                .unwrap()
                .save(&filesystem)
                .unwrap();
            if scenario == "swapped" {
                let mut record = read_record(&filesystem, &backup).unwrap();
                record.advance(&filesystem, UpdatePhase::Swapped).unwrap();
            }
            filesystem.add_file(backup.join("old-marker"));
            // A UUID-only backup from older versions has no recoverable target
            // binding. Neither it nor another arbitrary directory may be moved.
            let legacy = parent.join(format!("{BACKUP_PREFIX}{}{BACKUP_SUFFIX}", Uuid::new_v4()));
            filesystem.add_dir(&legacy);
            let runner = FakeRunner::new();
            let host = host();
            let policy = BundleTransactionPolicy::Codex { host: &host };
            match scenario {
                "occupied" => add_bundle(&filesystem, &target),
                "symlink" => filesystem.add_symlink(&backup, "/outside"),
                "invalid" => runner.queue_failure("/usr/bin/plutil", Some(1), "broken"),
                "rename_error" => {
                    queue_read_and_validate(&runner, "1");
                    super::super::signature::tests::queue_valid_signature(&runner);
                    filesystem.fail_operation("rename", MacosFilesystemErrorKind::PermissionDenied)
                }
                _ => {
                    queue_read_and_validate(&runner, "1");
                    super::super::signature::tests::queue_valid_signature(&runner);
                }
            }
            let result = recover_interrupted_updates(&runner, &filesystem, &policy, parent);
            assert_eq!(result.is_ok(), scenario != "rename_error", "{scenario}");
            if matches!(scenario, "missing" | "swapped" | "swap_missing") {
                assert!(filesystem.contains(target.join("old-marker")));
                assert!(!filesystem.contains(&backup));
                assert!(!filesystem.contains(record_path(&backup).unwrap()));
            } else {
                assert!(filesystem.contains(&backup), "{scenario}");
            }
            assert!(filesystem.contains(&legacy));
            runner.assert_drained();
        }
    }

    #[test]
    fn local_scan_does_not_count_crash_leftover_swap_backup_as_installation() {
        let filesystem = FakeFilesystem::new();
        let staging = Path::new(USER_APPLICATIONS).join(format!(
            "{STAGING_PREFIX}{}{STAGING_SUFFIX}",
            Uuid::new_v4()
        ));
        add_bundle(&filesystem, &staging);
        let runner = FakeRunner::new();
        assert!(bundle::scan_stable_bundles(&runner, &filesystem, &host())
            .unwrap()
            .is_empty());
        assert!(filesystem.contains(&staging));
        runner.assert_drained();
    }

    #[test]
    fn plist_mount_point_requires_one_absolute_string_and_decodes_entities() {
        assert_eq!(
            parse_mount_point_plist(
                b"<plist><dict><key>mount-point</key><string>/Volumes/A&amp;B</string></dict></plist>"
            )
            .unwrap(),
            PathBuf::from("/Volumes/A&B")
        );
        assert_eq!(
            parse_mount_point_plist(
                b"<plist><dict><key>mount-point</key><string>/Volumes/A</string><key>mount-point</key><string>/Volumes/B</string></dict></plist>"
            )
            .unwrap_err()
            .code(),
            InstallerErrorCode::MacDmgMountFailed
        );
        assert_eq!(
            parse_mount_point_plist(
                b"<plist><dict><key>mount-point</key><key>other</key><string>/Volumes/A</string></dict></plist>"
            )
            .unwrap_err()
            .code(),
            InstallerErrorCode::MacDmgMountFailed
        );
    }

    #[test]
    fn attach_validation_failure_still_detaches_the_mounted_image() {
        let filesystem = FakeFilesystem::new();
        let runner = FakeRunner::new();
        runner.queue_success("/usr/bin/hdiutil", mount_plist(MOUNT_POINT));
        runner.queue_success("/usr/bin/hdiutil", Vec::<u8>::new());

        let mount_error = mount_dmg(&runner, &filesystem, Path::new(ARTIFACT))
            .err()
            .expect("an uncanonicalizable mounted path must fail");
        assert_eq!(mount_error.code(), InstallerErrorCode::MacDmgMountFailed);
        let commands = runner
            .invocations()
            .into_iter()
            .map(|invocation| invocation.program())
            .collect::<Vec<_>>();
        assert_eq!(commands, ["/usr/bin/hdiutil", "/usr/bin/hdiutil"]);
        runner.assert_drained();
    }

    #[test]
    fn preparation_only_binds_the_fixed_download_and_mount_discovery_stays_bounded() {
        let trusted_bytes = b"trusted dmg";
        let release = release_for_artifact(trusted_bytes, "5848");
        let (_root, artifact) = downloaded_artifact_for(&release, trusted_bytes);
        let (filesystem, _) = fixture_filesystem_at(artifact.path());
        let runner = FakeRunner::new();
        let prepared =
            prepare_install_package(&runner, filesystem.as_ref(), &host(), &release, artifact)
                .unwrap();
        assert_eq!(prepared.platform(), DesktopPlatform::Macos);
        assert_eq!(prepared.architecture(), CpuArchitecture::Aarch64);
        runner.assert_drained();

        let filesystem = FakeFilesystem::new();
        filesystem.add_dir(MOUNT_POINT);
        filesystem.add_dir(Path::new(MOUNT_POINT).join("One.app"));
        filesystem.add_dir(Path::new(MOUNT_POINT).join("Two.app"));
        let runner = FakeRunner::new();
        assert_eq!(
            discover_single_bundle(&runner, &filesystem, Path::new(MOUNT_POINT))
                .unwrap_err()
                .code(),
            InstallerErrorCode::PackageParseFailed
        );
        runner.assert_drained();
    }

    #[test]
    fn same_size_content_replacement_is_not_a_package_hash_admission_gate() {
        let trusted_bytes = b"trusted dmg";
        let release = release_for_artifact(trusted_bytes, "5848");
        let (_root, artifact) = downloaded_artifact_for(&release, trusted_bytes);
        let (filesystem, _) = fixture_filesystem_at(artifact.path());
        let runner = FakeRunner::new();
        let package =
            prepare_install_package(&runner, filesystem.as_ref(), &host(), &release, artifact)
                .unwrap();
        runner.assert_drained();
        let mut replacement = fs::read(package.artifact_path()).unwrap();
        replacement[0] ^= 0x01;
        fs::write(package.artifact_path(), replacement).unwrap();

        package
            .revalidate_artifact()
            .expect("same-path content drift is not a package-hash admission gate");
    }

    #[test]
    fn confirmation_preflight_uses_user_directory_and_blocks_unwritable_target() {
        let (filesystem, _) = fixture_filesystem();
        filesystem.add_dir("/tmp/fyagent-job");
        let runner = FakeRunner::new();
        let plan = preflight(
            &runner,
            filesystem.as_ref(),
            &host(),
            &release("5848"),
            Path::new("/tmp/fyagent-job"),
        )
        .unwrap();
        assert_eq!(
            plan.confirmation_target(),
            Some(Path::new(USER_APPLICATIONS))
        );
        assert!(runner.invocations().is_empty());
        filesystem.fail_create_dir_under(
            USER_APPLICATIONS,
            MacosFilesystemErrorKind::PermissionDenied,
        );
        let error = preflight(
            &runner,
            filesystem.as_ref(),
            &host(),
            &release("5848"),
            Path::new("/tmp/fyagent-job"),
        )
        .unwrap_err();
        assert_eq!(
            error.to_dto().details.platform_error_code.as_deref(),
            Some("target_permission_denied")
        );
        assert!(runner.invocations().is_empty());
    }

    #[test]
    fn confirmed_fresh_install_cannot_fall_back_to_system_applications() {
        let (filesystem, _) = fixture_filesystem();
        let runner = Arc::new(FakeRunner::new());
        install_copy_hook(&runner, &filesystem);
        queue_fresh_install(runner.as_ref(), "5848");
        let mut package = package(&release("5848"));
        package.bind_confirmed_target(crate::codex_desktop::platform::ConfirmedInstallTarget {
            local: crate::codex_desktop::types::LocalInstallStatus::NotInstalled {
                platform: DesktopPlatform::Macos,
                architecture: CpuArchitecture::Aarch64,
            },
            target_root: Some(PathBuf::from(USER_APPLICATIONS)),
        });
        install_current_user(
            runner.as_ref(),
            filesystem.as_ref(),
            &host(),
            &package,
            Arc::new(|_| {}),
        )
        .unwrap();
        assert!(filesystem.contains(Path::new(USER_APPLICATIONS).join("ChatGPT.app")));
        assert!(!filesystem.contains(Path::new(SYSTEM_APPLICATIONS).join("ChatGPT.app")));
        runner.assert_drained();
    }

    #[test]
    fn fresh_install_preserves_dmg_bundle_basename_and_reports_progress() {
        let (filesystem, source_bundle) = fixture_filesystem();
        let runner = Arc::new(FakeRunner::new());
        let filesystem_for_ditto = filesystem.clone();
        runner.set_hook(Arc::new(move |invocation| {
            if invocation.program() == "/usr/bin/ditto" {
                filesystem_for_ditto
                    .copy_tree(
                        PathBuf::from(invocation.arguments()[0].clone()),
                        PathBuf::from(invocation.arguments()[1].clone()),
                    )
                    .unwrap();
            }
        }));
        queue_fresh_install(runner.as_ref(), "5848");
        let release = release("5848");
        let progress_values = Arc::new(Mutex::new(Vec::new()));
        let progress_for_sink = progress_values.clone();
        let progress: PlatformProgressSink = Arc::new(move |value| {
            progress_for_sink
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .push(value);
        });

        install_current_user(
            runner.as_ref(),
            filesystem.as_ref(),
            &host(),
            &package(&release),
            progress,
        )
        .unwrap();

        let installed = Path::new(SYSTEM_APPLICATIONS).join("ChatGPT.app");
        assert!(filesystem.contains(&installed));
        assert!(!filesystem.contains(Path::new(USER_APPLICATIONS).join("ChatGPT.app")));
        assert_eq!(
            progress_values
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .iter()
                .map(|progress| progress.completed_bytes)
                .collect::<Vec<_>>(),
            [Some(0), Some(1), Some(2), Some(3)]
        );
        let invocations = runner.invocations();
        let attach = invocations.first().unwrap();
        assert_eq!(attach.program(), "/usr/bin/hdiutil");
        assert!(attach
            .arguments()
            .iter()
            .any(|argument| argument == "-readonly"));
        assert!(attach
            .arguments()
            .iter()
            .any(|argument| argument == "-nobrowse"));
        assert!(attach
            .arguments()
            .iter()
            .any(|argument| argument == "-plist"));
        assert!(invocations.iter().all(|invocation| {
            invocation.program() != "xattr"
                && !invocation
                    .arguments()
                    .iter()
                    .any(|argument| argument == "-noverify" || argument == "--force")
        }));
        let ditto = invocations
            .iter()
            .find(|invocation| invocation.program() == "/usr/bin/ditto")
            .unwrap();
        assert_eq!(ditto.arguments()[0], source_bundle.as_os_str());
        assert!(Path::new(&ditto.arguments()[1])
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with(STAGING_PREFIX) && name.ends_with(".app")));
        runner.assert_drained();
    }

    #[test]
    fn downloaded_bundle_version_drift_does_not_block_signed_stable_install() {
        let (filesystem, _) = fixture_filesystem();
        let runner = Arc::new(FakeRunner::new());
        let filesystem_for_ditto = filesystem.clone();
        runner.set_hook(Arc::new(move |invocation| {
            if invocation.program() == "/usr/bin/ditto" {
                filesystem_for_ditto
                    .copy_tree(
                        PathBuf::from(invocation.arguments()[0].clone()),
                        PathBuf::from(invocation.arguments()[1].clone()),
                    )
                    .unwrap();
            }
        }));
        queue_fresh_install_with_identity(runner.as_ref(), stable_bundle_id(), "1");
        let release = release("9999");

        let installed = install_current_user(
            runner.as_ref(),
            filesystem.as_ref(),
            &host(),
            &package(&release),
            Arc::new(|_| {}),
        )
        .unwrap();

        assert_eq!(installed.stable_identity, stable_bundle_id());
        assert_eq!(
            installed.platform_version,
            PlatformVersion::parse_mac_bundle("1").unwrap()
        );
        runner.assert_drained();
    }

    #[test]
    fn signature_failures_detach_without_copying_or_installing() {
        for (identity, failed_step) in [
            ("Identifier=com.openai.codex\nTeamIdentifier=WRONG\n", None),
            (
                "Identifier=com.openai.chat\nTeamIdentifier=2DC432GLL2\n",
                None,
            ),
            (
                "Identifier=com.openai.codex.beta\nTeamIdentifier=2DC432GLL2\n",
                None,
            ),
            (
                "Identifier=com.openai.codex\nTeamIdentifier=not set\n",
                None,
            ),
            ("Identifier=com.openai.codex\n", None),
            ("TeamIdentifier=2DC432GLL2\n", None),
            ("", Some(0)),
            (
                "Identifier=com.openai.codex\nTeamIdentifier=2DC432GLL2\n",
                Some(1),
            ),
            (
                "Identifier=com.openai.codex\nTeamIdentifier=2DC432GLL2\n",
                Some(2),
            ),
        ] {
            let (filesystem, _) = fixture_filesystem();
            let runner = FakeRunner::new();
            runner.queue_success("/usr/bin/hdiutil", mount_plist(MOUNT_POINT));
            queue_read_and_validate(&runner, "5848");
            if failed_step == Some(0) {
                runner.queue_failure("/usr/bin/codesign", Some(1), "display failed");
            } else {
                runner.queue_success_stderr("/usr/bin/codesign", identity);
                if failed_step == Some(1) {
                    runner.queue_failure("/usr/bin/codesign", Some(1), "invalid signature");
                } else if failed_step == Some(2) {
                    runner.queue_success("/usr/bin/codesign", Vec::new());
                    runner.queue_failure("/usr/sbin/spctl", Some(1), "assessment denied");
                }
            }
            runner.queue_success("/usr/bin/hdiutil", Vec::new());
            let error = install_current_user(
                &runner,
                filesystem.as_ref(),
                &host(),
                &package(&release("5848")),
                Arc::new(|_| {}),
            )
            .unwrap_err();
            assert_eq!(error.code(), InstallerErrorCode::PackageSignatureInvalid);
            assert!(runner
                .invocations()
                .iter()
                .all(|call| call.program() != "/usr/bin/ditto"));
            assert!(!filesystem.contains(Path::new(SYSTEM_APPLICATIONS).join("ChatGPT.app")));
            assert!(!filesystem.contains(Path::new(USER_APPLICATIONS).join("ChatGPT.app")));
            runner.assert_drained();
        }
    }

    #[test]
    fn staged_signature_failure_cleans_copy_without_committing_and_preserves_error() {
        let (filesystem, _) = fixture_filesystem();
        let runner = Arc::new(FakeRunner::new());
        install_copy_hook(&runner, &filesystem);
        runner.queue_success("/usr/bin/hdiutil", mount_plist(MOUNT_POINT));
        queue_read_with_identity(&runner, stable_bundle_id(), "5848");
        super::super::signature::tests::queue_valid_signature(&runner);
        runner.queue_success("/usr/bin/ditto", Vec::new());
        queue_read_with_identity(&runner, stable_bundle_id(), "5848");
        runner.queue_success_stderr(
            "/usr/bin/codesign",
            "Identifier=com.openai.codex\nTeamIdentifier=2DC432GLL2\n",
        );
        runner.queue_failure("/usr/bin/codesign", Some(3), "modified code");
        for _ in 0..MAX_DETACH_ATTEMPTS {
            runner.queue_failure("/usr/bin/hdiutil", Some(1), "busy");
        }

        let failure = install_current_user(
            runner.as_ref(),
            filesystem.as_ref(),
            &host(),
            &package(&release("5848")),
            Arc::new(|_| {}),
        )
        .unwrap_err();
        assert_eq!(failure.code(), InstallerErrorCode::PackageSignatureInvalid);
        let calls = runner.invocations();
        let staging = calls
            .iter()
            .find(|call| call.program() == "/usr/bin/ditto")
            .map(|call| PathBuf::from(&call.arguments()[1]))
            .unwrap();
        assert!(calls
            .iter()
            .any(|call| call.program() == "/usr/bin/codesign"
                && call.arguments()[0] == "--verify"
                && call
                    .arguments()
                    .last()
                    .is_some_and(|path| Path::new(path) == staging)));
        assert!(!filesystem.contains(&staging));
        assert!(!filesystem.contains(Path::new(SYSTEM_APPLICATIONS).join("ChatGPT.app")));
        assert!(!filesystem.contains(Path::new(USER_APPLICATIONS).join("ChatGPT.app")));
        runner.assert_drained();
    }

    #[test]
    fn committed_install_succeeds_when_detach_fails() {
        let (filesystem, _) = fixture_filesystem();
        let runner = Arc::new(FakeRunner::new());
        install_copy_hook(&runner, &filesystem);
        runner.queue_success("/usr/bin/hdiutil", mount_plist(MOUNT_POINT));
        queue_read_with_identity(&runner, stable_bundle_id(), "5848");
        super::super::signature::tests::queue_valid_signature(&runner);
        runner.queue_success("/usr/bin/ditto", Vec::new());
        queue_read_with_identity(&runner, stable_bundle_id(), "5848");
        super::super::signature::tests::queue_valid_signature(&runner);
        queue_read_with_identity(&runner, stable_bundle_id(), "5848");
        for _ in 0..MAX_DETACH_ATTEMPTS {
            runner.queue_failure("/usr/bin/hdiutil", Some(1), "busy");
        }
        install_current_user(
            runner.as_ref(),
            filesystem.as_ref(),
            &host(),
            &package(&release("5848")),
            Arc::new(|_| {}),
        )
        .unwrap();
        assert!(filesystem.contains(Path::new(SYSTEM_APPLICATIONS).join("ChatGPT.app")));
        runner.assert_drained();
    }

    #[test]
    fn failed_bundle_copy_cleans_the_generated_staging_bundle() {
        let (filesystem, _) = fixture_filesystem();
        let runner = Arc::new(FakeRunner::new());
        let filesystem_for_ditto = filesystem.clone();
        runner.set_hook(Arc::new(move |invocation| {
            if invocation.program() == "/usr/bin/ditto" {
                filesystem_for_ditto
                    .copy_tree(
                        PathBuf::from(invocation.arguments()[0].clone()),
                        PathBuf::from(invocation.arguments()[1].clone()),
                    )
                    .unwrap();
            }
        }));
        runner.queue_success("/usr/bin/hdiutil", mount_plist(MOUNT_POINT));
        queue_read_and_validate(runner.as_ref(), "5848"); // mounted source
        super::super::signature::tests::queue_valid_signature(runner.as_ref());
        runner.queue_failure("/usr/bin/ditto", Some(1), b"copy failed".to_vec());
        runner.queue_success("/usr/bin/hdiutil", Vec::<u8>::new()); // detach
        let release = release("5848");
        let progress: PlatformProgressSink = Arc::new(|_| {});

        assert_eq!(
            install_current_user(
                runner.as_ref(),
                filesystem.as_ref(),
                &host(),
                &package(&release),
                progress,
            )
            .unwrap_err()
            .code(),
            InstallerErrorCode::MacCopyFailed
        );

        let staging = runner
            .invocations()
            .into_iter()
            .find(|invocation| invocation.program() == "/usr/bin/ditto")
            .map(|invocation| PathBuf::from(invocation.arguments()[1].clone()))
            .expect("the failed copy still used a generated staging bundle");
        assert!(!filesystem.contains(staging));
        runner.assert_drained();
    }

    #[test]
    fn failed_staging_validation_cleans_the_generated_staging_bundle() {
        let (filesystem, _) = fixture_filesystem();
        let runner = Arc::new(FakeRunner::new());
        let filesystem_for_ditto = filesystem.clone();
        runner.set_hook(Arc::new(move |invocation| {
            if invocation.program() == "/usr/bin/ditto" {
                filesystem_for_ditto
                    .copy_tree(
                        PathBuf::from(invocation.arguments()[0].clone()),
                        PathBuf::from(invocation.arguments()[1].clone()),
                    )
                    .unwrap();
            }
        }));
        runner.queue_success("/usr/bin/hdiutil", mount_plist(MOUNT_POINT));
        queue_read_and_validate(runner.as_ref(), "5848"); // mounted source
        super::super::signature::tests::queue_valid_signature(runner.as_ref());
        runner.queue_success("/usr/bin/ditto", Vec::<u8>::new());
        runner.queue_failure("/usr/bin/plutil", Some(1), b"invalid staged plist".to_vec());
        runner.queue_success("/usr/bin/hdiutil", Vec::<u8>::new()); // detach
        let release = release("5848");
        let progress: PlatformProgressSink = Arc::new(|_| {});

        assert_eq!(
            install_current_user(
                runner.as_ref(),
                filesystem.as_ref(),
                &host(),
                &package(&release),
                progress,
            )
            .unwrap_err()
            .code(),
            InstallerErrorCode::PackageParseFailed
        );

        let staging = runner
            .invocations()
            .into_iter()
            .find(|invocation| invocation.program() == "/usr/bin/ditto")
            .map(|invocation| PathBuf::from(invocation.arguments()[1].clone()))
            .expect("the staging validation followed a generated copy target");
        assert!(!filesystem.contains(staging));
        runner.assert_drained();
    }

    #[test]
    fn explicit_system_permission_failure_falls_back_to_user_applications() {
        let (filesystem, _) = fixture_filesystem();
        filesystem.fail_create_dir_under(
            SYSTEM_APPLICATIONS,
            MacosFilesystemErrorKind::PermissionDenied,
        );
        let runner = Arc::new(FakeRunner::new());
        let filesystem_for_ditto = filesystem.clone();
        runner.set_hook(Arc::new(move |invocation| {
            if invocation.program() == "/usr/bin/ditto" {
                filesystem_for_ditto
                    .copy_tree(
                        PathBuf::from(invocation.arguments()[0].clone()),
                        PathBuf::from(invocation.arguments()[1].clone()),
                    )
                    .unwrap();
            }
        }));
        queue_fresh_install(runner.as_ref(), "5848");
        let release = release("5848");
        let progress: PlatformProgressSink = Arc::new(|_| {});

        install_current_user(
            runner.as_ref(),
            filesystem.as_ref(),
            &host(),
            &package(&release),
            progress,
        )
        .unwrap();

        assert!(filesystem.contains(Path::new(USER_APPLICATIONS).join("ChatGPT.app")));
        assert!(!filesystem.contains(Path::new(SYSTEM_APPLICATIONS).join("ChatGPT.app")));
        runner.assert_drained();
    }

    #[test]
    fn managed_system_fresh_install_hands_only_verified_revisions_to_commit_owner() {
        const BUNDLE_ID: &str = "ai.opencode.desktop";
        let (filesystem, source_bundle) = fixture_filesystem();
        let target = Path::new(SYSTEM_APPLICATIONS).join("OpenCode.app");
        let runner = FakeRunner::new();
        runner.queue_success("/usr/bin/hdiutil", mount_plist(MOUNT_POINT));
        queue_managed_read(&runner, BUNDLE_ID, "1.2.3", "1.2.3");
        queue_managed_read(&runner, BUNDLE_ID, "1.2.3", "1.2.3");
        runner.queue_success("/usr/bin/hdiutil", Vec::<u8>::new());
        let policy = managed_policy(
            BUNDLE_ID,
            ManagedBundleVersionSource::InfoPlist,
            ManagedVersionEquivalence::Exact,
        );
        let commits = Arc::new(Mutex::new(Vec::new()));
        let commits_for_callback = Arc::clone(&commits);
        let filesystem_for_callback = Arc::clone(&filesystem);
        let source_for_callback = source_bundle.clone();
        let target_for_callback = target.clone();

        let installed = install_managed_system_exact(
            &runner,
            filesystem.as_ref(),
            ManagedDmgSystemCommitRequest {
                artifact_path: Path::new(ARTIFACT),
                target_path: &target,
                intent: ManagedDmgSystemIntent::Fresh,
                product: &policy,
                expected_release_version: Some("1.2.3"),
            },
            move |source| {
                commits_for_callback
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .push((
                        source.bundle_path.clone(),
                        source.source_revision,
                        source.target_revision,
                    ));
                filesystem_for_callback
                    .copy_tree(source_for_callback.clone(), target_for_callback.clone())
                    .unwrap();
                Ok::<(), ()>(())
            },
        )
        .unwrap();

        assert_eq!(installed.target_path, target);
        assert_eq!(installed.local_version, "1.2.3");
        let recorded = commits
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        assert_eq!(recorded.len(), 1);
        assert_eq!(recorded[0].0, source_bundle);
        assert_ne!(recorded[0].1, [0; 32]);
        assert_eq!(recorded[0].2, [0; 32]);
        assert!(runner
            .invocations()
            .iter()
            .all(|invocation| invocation.program() != "/usr/bin/ditto"));
        runner.assert_drained();
    }

    #[test]
    fn managed_system_update_rechecks_target_and_running_state_before_commit() {
        const BUNDLE_ID: &str = "ai.opencode.desktop";
        let (filesystem, source_bundle) = fixture_filesystem();
        let target = Path::new(SYSTEM_APPLICATIONS).join("OpenCode.app");
        add_bundle(filesystem.as_ref(), &target);
        let runner = FakeRunner::new();
        runner.queue_success("/usr/bin/hdiutil", mount_plist(MOUNT_POINT));
        queue_managed_read(&runner, BUNDLE_ID, "1.2.3", "1.2.3");
        queue_managed_read(&runner, BUNDLE_ID, "1.2.2", "1.2.2");
        runner.queue_success("/usr/bin/osascript", b"not_running\n".to_vec());
        queue_managed_read(&runner, BUNDLE_ID, "1.2.3", "1.2.3");
        runner.queue_success("/usr/bin/hdiutil", Vec::<u8>::new());
        let policy = managed_policy(
            BUNDLE_ID,
            ManagedBundleVersionSource::InfoPlist,
            ManagedVersionEquivalence::Exact,
        );
        let filesystem_for_callback = Arc::clone(&filesystem);
        let source_for_callback = source_bundle.clone();
        let target_for_callback = target.clone();
        let captured_target_revision = Arc::new(Mutex::new(None));
        let captured_for_callback = Arc::clone(&captured_target_revision);

        let installed = install_managed_system_exact(
            &runner,
            filesystem.as_ref(),
            ManagedDmgSystemCommitRequest {
                artifact_path: Path::new(ARTIFACT),
                target_path: &target,
                intent: ManagedDmgSystemIntent::Update,
                product: &policy,
                expected_release_version: Some("1.2.3"),
            },
            move |source| {
                *captured_for_callback
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner()) =
                    Some(source.target_revision);
                filesystem_for_callback
                    .remove_dir_all(&target_for_callback)
                    .unwrap();
                filesystem_for_callback
                    .copy_tree(source_for_callback.clone(), target_for_callback.clone())
                    .unwrap();
                Ok::<(), ()>(())
            },
        )
        .unwrap();

        assert_eq!(installed.local_version, "1.2.3");
        assert_ne!(
            captured_target_revision
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .expect("update commit must carry the target revision"),
            [0; 32]
        );
        assert_eq!(
            runner
                .invocations()
                .iter()
                .filter(|invocation| invocation.program() == "/usr/bin/osascript")
                .count(),
            1
        );
        runner.assert_drained();
    }

    #[test]
    fn managed_system_running_application_blocks_before_commit() {
        const BUNDLE_ID: &str = "ai.opencode.desktop";
        let (filesystem, _) = fixture_filesystem();
        let target = Path::new(SYSTEM_APPLICATIONS).join("OpenCode.app");
        add_bundle(filesystem.as_ref(), &target);
        let runner = FakeRunner::new();
        runner.queue_success("/usr/bin/hdiutil", mount_plist(MOUNT_POINT));
        queue_managed_read(&runner, BUNDLE_ID, "1.2.3", "1.2.3");
        queue_managed_read(&runner, BUNDLE_ID, "1.2.2", "1.2.2");
        runner.queue_success("/usr/bin/osascript", b"running\n".to_vec());
        runner.queue_success("/usr/bin/hdiutil", Vec::<u8>::new());
        let policy = managed_policy(
            BUNDLE_ID,
            ManagedBundleVersionSource::InfoPlist,
            ManagedVersionEquivalence::Exact,
        );
        let commit_called = Arc::new(Mutex::new(false));
        let commit_called_for_callback = Arc::clone(&commit_called);

        let failure = install_managed_system_exact(
            &runner,
            filesystem.as_ref(),
            ManagedDmgSystemCommitRequest {
                artifact_path: Path::new(ARTIFACT),
                target_path: &target,
                intent: ManagedDmgSystemIntent::Update,
                product: &policy,
                expected_release_version: Some("1.2.3"),
            },
            move |_| {
                *commit_called_for_callback
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner()) = true;
                Ok::<(), ()>(())
            },
        )
        .unwrap_err();

        assert!(matches!(
            failure,
            ManagedDmgSystemFailure::Package(ManagedDmgFailureKind::ApplicationRunning)
        ));
        assert!(!*commit_called
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()));
        runner.assert_drained();
    }

    #[test]
    fn managed_system_unknown_commit_outcome_preserves_package_error_and_detaches() {
        const BUNDLE_ID: &str = "ai.opencode.desktop";
        let (filesystem, _) = fixture_filesystem();
        let target = Path::new(SYSTEM_APPLICATIONS).join("OpenCode.app");
        let runner = FakeRunner::new();
        runner.queue_success("/usr/bin/hdiutil", mount_plist(MOUNT_POINT));
        queue_managed_read(&runner, BUNDLE_ID, "1.2.3", "1.2.3");
        runner.queue_success("/usr/bin/hdiutil", Vec::<u8>::new());
        let policy = managed_policy(
            BUNDLE_ID,
            ManagedBundleVersionSource::InfoPlist,
            ManagedVersionEquivalence::Exact,
        );

        let failure = install_managed_system_exact(
            &runner,
            filesystem.as_ref(),
            ManagedDmgSystemCommitRequest {
                artifact_path: Path::new(ARTIFACT),
                target_path: &target,
                intent: ManagedDmgSystemIntent::Fresh,
                product: &policy,
                expected_release_version: Some("1.2.3"),
            },
            |_| Err::<(), _>("outcome-unknown"),
        )
        .unwrap_err();

        assert!(matches!(
            failure,
            ManagedDmgSystemFailure::Commit("outcome-unknown")
        ));
        assert!(!filesystem.contains(target));
        assert_eq!(
            runner
                .invocations()
                .iter()
                .filter(|invocation| invocation.program() == "/usr/bin/hdiutil")
                .count(),
            2
        );
        runner.assert_drained();
    }

    #[test]
    fn managed_system_post_commit_identity_mismatch_requires_recovery() {
        const BUNDLE_ID: &str = "ai.opencode.desktop";
        let (filesystem, source_bundle) = fixture_filesystem();
        let target = Path::new(SYSTEM_APPLICATIONS).join("OpenCode.app");
        let runner = FakeRunner::new();
        runner.queue_success("/usr/bin/hdiutil", mount_plist(MOUNT_POINT));
        queue_managed_read(&runner, BUNDLE_ID, "1.2.3", "1.2.3");
        queue_managed_read(&runner, BUNDLE_ID, "1.2.2", "1.2.2");
        runner.queue_success("/usr/bin/hdiutil", Vec::<u8>::new());
        let policy = managed_policy(
            BUNDLE_ID,
            ManagedBundleVersionSource::InfoPlist,
            ManagedVersionEquivalence::Exact,
        );
        let filesystem_for_callback = Arc::clone(&filesystem);
        let source_for_callback = source_bundle.clone();
        let target_for_callback = target.clone();

        let failure = install_managed_system_exact(
            &runner,
            filesystem.as_ref(),
            ManagedDmgSystemCommitRequest {
                artifact_path: Path::new(ARTIFACT),
                target_path: &target,
                intent: ManagedDmgSystemIntent::Fresh,
                product: &policy,
                expected_release_version: Some("1.2.3"),
            },
            move |_| {
                filesystem_for_callback
                    .copy_tree(source_for_callback.clone(), target_for_callback.clone())
                    .unwrap();
                Ok::<(), ()>(())
            },
        )
        .unwrap_err();

        assert!(matches!(
            failure,
            ManagedDmgSystemFailure::Package(ManagedDmgFailureKind::RecoveryRequired)
        ));
        runner.assert_drained();
    }

    #[test]
    fn managed_update_preserves_the_selected_bundle_path_without_scope_fallback() {
        const BUNDLE_ID: &str = "com.tencent.workbuddy.mac";
        let (filesystem, _) = fixture_filesystem();
        let target = Path::new(USER_APPLICATIONS).join("Existing WorkBuddy.app");
        add_bundle(filesystem.as_ref(), &target);
        let runner = Arc::new(FakeRunner::new());
        install_copy_hook(&runner, &filesystem);
        runner.queue_success("/usr/bin/hdiutil", mount_plist(MOUNT_POINT));
        queue_managed_read(runner.as_ref(), BUNDLE_ID, "36279234", "5.3.14");
        queue_managed_read(runner.as_ref(), BUNDLE_ID, "36200000", "5.3.13");
        runner.queue_success("/usr/bin/osascript", b"not_running\n".to_vec());
        runner.queue_success("/usr/bin/ditto", Vec::<u8>::new());
        queue_managed_read(runner.as_ref(), BUNDLE_ID, "36279234", "5.3.14");
        queue_managed_read(runner.as_ref(), BUNDLE_ID, "36200000", "5.3.13");
        runner.queue_success("/usr/bin/osascript", b"not_running\n".to_vec());
        queue_managed_read(runner.as_ref(), BUNDLE_ID, "36279234", "5.3.14");
        queue_managed_read(runner.as_ref(), BUNDLE_ID, "36200000", "5.3.13");
        runner.queue_success("/usr/bin/hdiutil", Vec::<u8>::new());
        let commits = Arc::new(Mutex::new(0_u8));
        let commits_for_gate = commits.clone();
        let policy = managed_policy(
            BUNDLE_ID,
            ManagedBundleVersionSource::InfoPlist,
            ManagedVersionEquivalence::DottedPrefix,
        );

        let installed = install_managed_exact(
            runner.as_ref(),
            filesystem.as_ref(),
            ManagedDmgInstallRequest {
                artifact_path: Path::new(ARTIFACT),
                intent: ManagedDmgInstallIntent::Update {
                    target: target.clone(),
                },
                product: &policy,
                expected_release_version: Some("5.3.14.36279234"),
            },
            move || {
                *commits_for_gate
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner()) += 1;
                Ok(())
            },
            |_| Ok(()),
        )
        .unwrap();

        assert_eq!(installed.target_path, target);
        assert_eq!(installed.local_version, "5.3.14");
        assert_eq!(
            *commits
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
            1
        );
        assert!(filesystem.contains(&installed.target_path));
        assert!(!filesystem.contains(Path::new(USER_APPLICATIONS).join("ChatGPT.app")));
        assert!(!filesystem.contains(Path::new(SYSTEM_APPLICATIONS).join("ChatGPT.app")));
        runner.assert_drained();
    }

    #[test]
    fn managed_update_cancellation_before_commit_keeps_the_original_bundle() {
        const BUNDLE_ID: &str = "com.qoder.work.cn";
        let (filesystem, _) = fixture_filesystem();
        let target = Path::new(USER_APPLICATIONS).join("QoderWork CN.app");
        add_bundle(filesystem.as_ref(), &target);
        let runner = Arc::new(FakeRunner::new());
        install_copy_hook(&runner, &filesystem);
        runner.queue_success("/usr/bin/hdiutil", mount_plist(MOUNT_POINT));
        queue_managed_read(runner.as_ref(), BUNDLE_ID, "0.9.15", "0.9.15");
        queue_managed_read(runner.as_ref(), BUNDLE_ID, "0.9.12", "0.9.12");
        runner.queue_success("/usr/bin/osascript", b"not_running\n".to_vec());
        runner.queue_success("/usr/bin/ditto", Vec::<u8>::new());
        queue_managed_read(runner.as_ref(), BUNDLE_ID, "0.9.15", "0.9.15");
        runner.queue_success("/usr/bin/hdiutil", Vec::<u8>::new());
        let policy = managed_policy(
            BUNDLE_ID,
            ManagedBundleVersionSource::InfoPlist,
            ManagedVersionEquivalence::Exact,
        );

        let failure = install_managed_exact(
            runner.as_ref(),
            filesystem.as_ref(),
            ManagedDmgInstallRequest {
                artifact_path: Path::new(ARTIFACT),
                intent: ManagedDmgInstallIntent::Update {
                    target: target.clone(),
                },
                product: &policy,
                expected_release_version: Some("0.9.15"),
            },
            || Err(InstallerError::new(InstallerErrorCode::DownloadCancelled)),
            |_| Ok(()),
        )
        .unwrap_err();

        assert_eq!(failure.kind(), ManagedDmgFailureKind::Cancelled);
        assert!(filesystem.contains(&target));
        let staging = runner
            .invocations()
            .into_iter()
            .find(|invocation| invocation.program() == "/usr/bin/ditto")
            .map(|invocation| PathBuf::from(invocation.arguments()[1].clone()))
            .expect("managed update must stage before the commit gate");
        assert!(!filesystem.contains(staging));
        runner.assert_drained();
    }

    #[test]
    fn managed_post_commit_failure_restores_and_reverifies_the_old_bundle() {
        const BUNDLE_ID: &str = "com.qoder.work.cn";
        let (filesystem, _) = fixture_filesystem();
        let target = Path::new(USER_APPLICATIONS).join("QoderWork CN.app");
        add_bundle(filesystem.as_ref(), &target);
        let runner = Arc::new(FakeRunner::new());
        install_copy_hook(&runner, &filesystem);
        runner.queue_success("/usr/bin/hdiutil", mount_plist(MOUNT_POINT));
        queue_managed_read(runner.as_ref(), BUNDLE_ID, "0.9.15", "0.9.15");
        queue_managed_read(runner.as_ref(), BUNDLE_ID, "0.9.12", "0.9.12");
        runner.queue_success("/usr/bin/osascript", b"not_running\n".to_vec());
        runner.queue_success("/usr/bin/ditto", Vec::<u8>::new());
        queue_managed_read(runner.as_ref(), BUNDLE_ID, "0.9.15", "0.9.15");
        queue_managed_read(runner.as_ref(), BUNDLE_ID, "0.9.12", "0.9.12");
        runner.queue_success("/usr/bin/osascript", b"not_running\n".to_vec());
        queue_managed_read(runner.as_ref(), BUNDLE_ID, "0.9.15", "0.9.15");
        queue_managed_read(runner.as_ref(), BUNDLE_ID, "0.9.12", "0.9.12");
        queue_managed_read(runner.as_ref(), BUNDLE_ID, "0.9.15", "0.9.15"); // replacement before swap
        queue_managed_read(runner.as_ref(), BUNDLE_ID, "0.9.12", "0.9.12"); // restored
        queue_managed_read(runner.as_ref(), BUNDLE_ID, "0.9.15", "0.9.15"); // delete swapped replacement
        runner.queue_success("/usr/bin/hdiutil", Vec::<u8>::new());
        let policy = managed_policy(
            BUNDLE_ID,
            ManagedBundleVersionSource::InfoPlist,
            ManagedVersionEquivalence::Exact,
        );

        let failure = install_managed_exact(
            runner.as_ref(),
            filesystem.as_ref(),
            ManagedDmgInstallRequest {
                artifact_path: Path::new(ARTIFACT),
                intent: ManagedDmgInstallIntent::Update {
                    target: target.clone(),
                },
                product: &policy,
                expected_release_version: Some("0.9.15"),
            },
            || Ok(()),
            |_| {
                Err(InstallerError::new(
                    InstallerErrorCode::InstallationVerifyFailed,
                ))
            },
        )
        .unwrap_err();

        assert_eq!(
            failure.kind(),
            ManagedDmgFailureKind::VerificationFailedRestored
        );
        assert!(filesystem.contains(&target));
        runner.assert_drained();
    }

    #[test]
    fn managed_running_application_blocks_before_any_staging_write() {
        const BUNDLE_ID: &str = "com.tencent.workbuddy.mac";
        let (filesystem, _) = fixture_filesystem();
        let target = Path::new(USER_APPLICATIONS).join("WorkBuddy.app");
        add_bundle(filesystem.as_ref(), &target);
        let runner = FakeRunner::new();
        runner.queue_success("/usr/bin/hdiutil", mount_plist(MOUNT_POINT));
        queue_managed_read(&runner, BUNDLE_ID, "100", "5.3.14");
        queue_managed_read(&runner, BUNDLE_ID, "99", "5.3.13");
        runner.queue_success("/usr/bin/osascript", b"running\n".to_vec());
        runner.queue_success("/usr/bin/hdiutil", Vec::<u8>::new());
        let policy = managed_policy(
            BUNDLE_ID,
            ManagedBundleVersionSource::InfoPlist,
            ManagedVersionEquivalence::DottedPrefix,
        );

        let failure = install_managed_exact(
            &runner,
            filesystem.as_ref(),
            ManagedDmgInstallRequest {
                artifact_path: Path::new(ARTIFACT),
                intent: ManagedDmgInstallIntent::Update {
                    target: target.clone(),
                },
                product: &policy,
                expected_release_version: Some("5.3.14.1"),
            },
            || Ok(()),
            |_| Ok(()),
        )
        .unwrap_err();

        assert_eq!(failure.kind(), ManagedDmgFailureKind::ApplicationRunning);
        assert!(filesystem.contains(target));
        assert!(runner
            .invocations()
            .iter()
            .all(|invocation| invocation.program() != "/usr/bin/ditto"));
        runner.assert_drained();
    }

    #[test]
    fn trae_managed_install_uses_bounded_product_json_for_release_verification() {
        const BUNDLE_ID: &str = "cn.trae.solo.app";
        let (filesystem, source_bundle) = fixture_filesystem();
        add_trae_product_json(filesystem.as_ref(), &source_bundle, "2.3.71801");
        let runner = Arc::new(FakeRunner::new());
        install_copy_hook(&runner, &filesystem);
        runner.queue_success("/usr/bin/hdiutil", mount_plist(MOUNT_POINT));
        queue_managed_read(runner.as_ref(), BUNDLE_ID, "0.1.51", "0.1.51");
        runner.queue_success("/usr/bin/ditto", Vec::<u8>::new());
        queue_managed_read(runner.as_ref(), BUNDLE_ID, "0.1.51", "0.1.51");
        queue_managed_read(runner.as_ref(), BUNDLE_ID, "0.1.51", "0.1.51");
        runner.queue_success("/usr/bin/hdiutil", Vec::<u8>::new());
        let policy = managed_policy(
            BUNDLE_ID,
            ManagedBundleVersionSource::TraeProductJson,
            ManagedVersionEquivalence::Exact,
        );

        let installed = install_managed_exact(
            runner.as_ref(),
            filesystem.as_ref(),
            ManagedDmgInstallRequest {
                artifact_path: Path::new(ARTIFACT),
                intent: ManagedDmgInstallIntent::Fresh {
                    parent: PathBuf::from(USER_APPLICATIONS),
                },
                product: &policy,
                expected_release_version: Some("2.3.71801"),
            },
            || Ok(()),
            |_| Ok(()),
        )
        .unwrap();

        assert_eq!(installed.local_version, "2.3.71801");
        assert_eq!(
            installed.target_path,
            Path::new(USER_APPLICATIONS).join("ChatGPT.app")
        );
        runner.assert_drained();
    }

    #[test]
    fn restore_refuses_to_delete_a_replacement_with_changed_identity() {
        let filesystem = FakeFilesystem::new();
        let parent = Path::new(SYSTEM_APPLICATIONS);
        let target = parent.join("ChatGPT.app");
        let backup = recovery_backup_path(&target).unwrap();
        filesystem.add_dir(parent);
        add_bundle(&filesystem, &target);
        filesystem.add_dir(&backup);
        let runner = FakeRunner::new();
        let host = host();
        let policy = BundleTransactionPolicy::Codex { host: &host };
        runner.queue_success("/usr/bin/plutil", plist(stable_bundle_id(), "5848"));
        let expected = policy
            .inspect_source(&runner, &filesystem, &target)
            .unwrap();

        assert_eq!(
            restore_backup(
                &runner,
                &filesystem,
                &policy,
                RestoreRequest {
                    parent,
                    target: &target,
                    backup: &backup,
                    expected_replacement: &expected,
                    expected_backup: &expected,
                    record: &mut UpdateRecord::new(&target, &backup, expected.identity).unwrap(),
                },
            )
            .unwrap_err()
            .code(),
            InstallerErrorCode::MacTargetPathConflict
        );
        assert!(filesystem.contains(&target));
        assert!(filesystem.contains(&backup));
        runner.assert_drained();
    }
    #[test]
    fn cleanup_requires_signature_versions_and_filesystem_identity() {
        for changed in [
            "inode",
            "dev",
            "bundle_id",
            "team",
            "short_version",
            "build_version",
            "normalized_build_version",
            "valid",
        ] {
            let fs = FakeFilesystem::new();
            let parent = Path::new(USER_APPLICATIONS);
            let path = parent.join("Codex.app");
            add_bundle(&fs, &path);
            let runner = FakeRunner::new();
            let host = host();
            let policy = BundleTransactionPolicy::Codex { host: &host };
            queue_read_and_validate(&runner, "1");
            let mut expected = policy.inspect_source(&runner, &fs, &path).unwrap();
            match changed {
                "inode" => expected.identity.inode += 1,
                "dev" => expected.identity.dev += 1,
                _ => {
                    let mut raw: serde_json::Value =
                        serde_json::from_slice(&plist(stable_bundle_id(), "1")).unwrap();
                    match changed {
                        "bundle_id" => raw["CFBundleIdentifier"] = "com.example.other".into(),
                        "short_version" => raw["CFBundleShortVersionString"] = "changed".into(),
                        "build_version" => raw["CFBundleVersion"] = "2".into(),
                        "normalized_build_version" => raw["CFBundleVersion"] = "1.0.0.99".into(),
                        _ => {}
                    }
                    runner.queue_success("/usr/bin/plutil", raw.to_string());
                    if changed == "team" {
                        runner.queue_success_stderr(
                            "/usr/bin/codesign",
                            "Identifier=com.openai.codex\nTeamIdentifier=WRONG\n",
                        );
                    } else if changed != "bundle_id" {
                        super::super::signature::tests::queue_valid_signature(&runner);
                    }
                }
            }
            let result =
                remove_expected_replacement(&runner, &fs, &policy, parent, &path, &expected);
            assert_eq!(result.is_ok(), changed == "valid", "{changed}");
            assert_eq!(fs.contains(&path), changed != "valid", "{changed}");
            if let Err(reason) = result {
                assert_eq!(reason.code(), InstallerErrorCode::MacTargetPathConflict);
            }
            runner.assert_drained();
        }
    }

    #[test]
    fn rollback_preserves_both_apps_when_target_changes_before_or_after_swap() {
        for after_swap in [false, true] {
            let fs = Arc::new(FakeFilesystem::new());
            let parent = Path::new(USER_APPLICATIONS);
            let target = parent.join("Codex.app");
            let backup = transaction_path(&target, &transaction_id(), true).unwrap();
            add_bundle(&fs, &target);
            add_bundle(&fs, &backup);
            let runner = FakeRunner::new();
            let host = host();
            let policy = BundleTransactionPolicy::Codex { host: &host };
            queue_read_and_validate(&runner, "2");
            let replacement = policy.inspect_source(&runner, &*fs, &target).unwrap();
            queue_read_and_validate(&runner, "1");
            let old = policy.inspect_source(&runner, &*fs, &backup).unwrap();
            queue_read_and_validate(&runner, "1");
            super::super::signature::tests::queue_valid_signature(&runner);
            if after_swap {
                queue_read_and_validate(&runner, "2");
                super::super::signature::tests::queue_valid_signature(&runner);
                queue_read_and_validate(&runner, "1");
                let fs = fs.clone();
                let backup = backup.clone();
                let reads = std::sync::atomic::AtomicUsize::new(0);
                runner.set_hook(Arc::new(move |call| {
                    if call.program() == "/usr/bin/plutil"
                        && reads.fetch_add(1, std::sync::atomic::Ordering::SeqCst) == 2
                    {
                        fs.remove_dir_all(&backup).unwrap();
                        add_bundle(&fs, &backup);
                    }
                }));
            } else {
                fs.remove_dir_all(&target).unwrap();
                add_bundle(&fs, &target);
            }
            let reason = restore_backup(
                &runner,
                &*fs,
                &policy,
                RestoreRequest {
                    parent,
                    target: &target,
                    backup: &backup,
                    expected_replacement: &replacement,
                    expected_backup: &old,
                    record: &mut UpdateRecord::new(&target, &backup, old.identity).unwrap(),
                },
            )
            .unwrap_err();
            assert_eq!(reason.code(), InstallerErrorCode::MacTargetPathConflict);
            let message = reason.to_dto().details.redacted_message.unwrap();
            assert!(message.contains("Codex.app") && message.contains(STAGING_PREFIX));
            assert!(fs.contains(&target) && fs.contains(&backup));
            runner.assert_drained();
        }
    }

    #[test]
    fn unique_transaction_names_do_not_collide_with_crash_leftovers() {
        let fs = FakeFilesystem::new();
        let target = Path::new(USER_APPLICATIONS).join("Codex.app");
        let first = recovery_backup_path(&target).unwrap();
        fs.add_dir(&first);
        let second = recovery_backup_path(&target).unwrap();
        assert_ne!(first, second);
        assert!(recovery_candidate(&second).unwrap().1 > 0);
        ensure_generated_path_absent(
            &fs,
            target.parent().unwrap(),
            &second,
            BACKUP_PREFIX,
            BACKUP_SUFFIX,
        )
        .unwrap();
        let staging = target.with_file_name(format!(
            "{STAGING_PREFIX}{}{STAGING_SUFFIX}",
            transaction_id()
        ));
        assert!(is_transaction_staging(&staging));
    }

    #[test]
    fn recovery_selects_newest_verified_complete_backup_and_keeps_rejected_candidates() {
        for rejected in [
            "team",
            "signature",
            "spctl",
            "bundle_id",
            "plist",
            "executable",
        ] {
            let fs = FakeFilesystem::new();
            let parent = Path::new(USER_APPLICATIONS);
            let target = parent.join("Codex.app");
            let candidate = |time| {
                parent.join(format!(
                    "{BACKUP_PREFIX}Codex.app--{time}-{}{BACKUP_SUFFIX}",
                    Uuid::new_v4()
                ))
            };
            let oldest = candidate(1);
            let good = candidate(2);
            let bad = candidate(3);
            for path in [&oldest, &good, &bad] {
                add_bundle(&fs, path);
                UpdateRecord::new(&target, path, fs.identity(path).unwrap())
                    .unwrap()
                    .save(&fs)
                    .unwrap();
            }
            fs.add_file(good.join("chosen"));
            let runner = FakeRunner::new();
            if rejected == "plist" {
                runner.queue_failure("/usr/bin/plutil", Some(1), "unreadable");
            } else {
                let mut raw: serde_json::Value =
                    serde_json::from_slice(&plist(stable_bundle_id(), "1")).unwrap();
                if rejected == "bundle_id" {
                    raw["CFBundleIdentifier"] = "other".into();
                }
                if rejected == "executable" {
                    raw["CFBundleExecutable"] = "missing".into();
                }
                runner.queue_success("/usr/bin/plutil", raw.to_string());
                match rejected {
                    "team" => runner.queue_success_stderr(
                        "/usr/bin/codesign",
                        "Identifier=com.openai.codex\nTeamIdentifier=WRONG\n",
                    ),
                    "signature" | "spctl" => {
                        runner.queue_success_stderr(
                            "/usr/bin/codesign",
                            "Identifier=com.openai.codex\nTeamIdentifier=2DC432GLL2\n",
                        );
                        if rejected == "signature" {
                            runner.queue_failure("/usr/bin/codesign", Some(1), "bad signature");
                        } else {
                            runner.queue_success("/usr/bin/codesign", vec![]);
                            runner.queue_failure("/usr/sbin/spctl", Some(1), "rejected");
                        }
                    }
                    _ => {}
                }
            }
            queue_read_and_validate(&runner, "1");
            super::super::signature::tests::queue_valid_signature(&runner);
            let host = host();
            recover_interrupted_updates(
                &runner,
                &fs,
                &BundleTransactionPolicy::Codex { host: &host },
                parent,
            )
            .unwrap();
            assert!(fs.contains(target.join("chosen")), "{rejected}");
            assert!(fs.contains(&oldest) && fs.contains(&bad));
            assert!(!fs.contains(&good));
            runner.assert_drained();
        }
    }
    #[test]
    fn recovery_preserves_target_created_during_verification() {
        let fs = Arc::new(FakeFilesystem::new());
        let parent = Path::new(USER_APPLICATIONS);
        let target = parent.join("Codex.app");
        let backup = recovery_backup_path(&target).unwrap();
        add_bundle(&fs, &backup);
        UpdateRecord::new(&target, &backup, fs.identity(&backup).unwrap())
            .unwrap()
            .save(&*fs)
            .unwrap();
        let runner = FakeRunner::new();
        queue_read_and_validate(&runner, "1");
        super::super::signature::tests::queue_valid_signature(&runner);
        let hook_fs = fs.clone();
        let hook_target = target.clone();
        runner.set_hook(Arc::new(move |call| {
            if call.program() == "/usr/sbin/spctl" {
                add_bundle(&hook_fs, &hook_target);
                hook_fs.add_file(hook_target.join("concurrent-app"));
            }
        }));
        let host = host();
        recover_interrupted_updates(
            &runner,
            &*fs,
            &BundleTransactionPolicy::Codex { host: &host },
            parent,
        )
        .unwrap();
        assert!(fs.contains(target.join("concurrent-app")) && fs.contains(&backup));
        assert!(fs.rename_exclusive(&backup, &target).is_err());
        assert!(fs.contains(target.join("concurrent-app")) && fs.contains(&backup));
        runner.assert_drained();
    }
    #[test]
    fn recovery_recognizes_legacy_and_timestamped_double_dash_names() {
        let target = Path::new(USER_APPLICATIONS).join("Codex--Work.app");
        let legacy =
            target.with_file_name(format!("{BACKUP_PREFIX}Codex--Work.app{BACKUP_SUFFIX}"));
        assert_eq!(recovery_candidate(&legacy), Some(("Codex--Work.app", 0)));
        let modern = recovery_backup_path(&target).unwrap();
        assert_eq!(recovery_candidate(&modern).unwrap().0, "Codex--Work.app");
    }

    #[test]
    fn recovery_keeps_finished_missing_corrupt_and_inconsistent_records_after_uninstall() {
        for case in [
            "committed",
            "rolled-back",
            "missing",
            "corrupt",
            "identity",
            "target",
            "backup",
            "prefix",
            "id",
            "legacy",
        ] {
            let fs = FakeFilesystem::new();
            let parent = Path::new(USER_APPLICATIONS);
            let target = parent.join("Codex--Work.app");
            let backup = if case == "legacy" {
                parent.join(format!("{BACKUP_PREFIX}Codex--Work.app{BACKUP_SUFFIX}"))
            } else {
                recovery_backup_path(&target).unwrap()
            };
            add_bundle(&fs, &backup);
            if case == "legacy" {
                let runner = FakeRunner::new();
                let host = host();
                recover_interrupted_updates(
                    &runner,
                    &fs,
                    &BundleTransactionPolicy::Codex { host: &host },
                    parent,
                )
                .unwrap();
                assert!(fs.contains(&backup));
                continue;
            }
            let mut record =
                UpdateRecord::new(&target, &backup, fs.identity(&backup).unwrap()).unwrap();
            match case {
                "committed" => record.phase = UpdatePhase::Committed,
                "rolled-back" => record.phase = UpdatePhase::RolledBack,
                "identity" => record.backup_identity.inode += 1,
                "target" => record.target = parent.join("Other.app"),
                "backup" => record.backup = parent.join("other"),
                "prefix" => record.installer_prefix = "other".into(),
                "id" => record.transaction_id = "broken".into(),
                _ => {}
            }
            if !matches!(case, "missing" | "legacy") {
                fs.add_file_with_contents(
                    record_path(&backup).unwrap(),
                    serde_json::to_vec(&record).unwrap(),
                );
            }
            if case == "corrupt" {
                fs.add_file_with_contents(record_path(&backup).unwrap(), b"{broken".to_vec());
            }
            let runner = FakeRunner::new();
            let host = host();
            recover_interrupted_updates(
                &runner,
                &fs,
                &BundleTransactionPolicy::Codex { host: &host },
                parent,
            )
            .unwrap();
            assert!(fs.contains(&backup), "{case}");
            assert!(!fs.contains(&target), "{case}");
            runner.assert_drained();
        }
    }

    #[test]
    fn transaction_record_atomic_write_roundtrips_and_removes() {
        let dir = tempfile::tempdir().unwrap();
        let fs = super::super::StdMacosFilesystem;
        let backup = recovery_backup_path(&dir.path().join("Codex.app")).unwrap();
        let mut record = UpdateRecord::new(
            &dir.path().join("Codex.app"),
            &backup,
            super::super::FileIdentity { dev: 1, inode: 2 },
        )
        .unwrap();
        record.save(&fs).unwrap();
        record.advance(&fs, UpdatePhase::Committed).unwrap();
        assert!(matches!(
            read_record(&fs, &backup).unwrap().phase,
            UpdatePhase::Committed
        ));
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 3);
        record.remove(&fs).unwrap();
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    }
    #[test]
    fn transaction_paths_reject_root_empty_and_unbound_names_without_panicking() {
        let identity = super::super::FileIdentity { dev: 1, inode: 2 };
        for path in ["/", "", ".", "/Applications/..", "/Applications/no-name"] {
            assert!(record_path(Path::new(path)).is_err(), "{path}");
            assert!(transaction_path(Path::new(path), &transaction_id(), true).is_err());
            assert!(UpdateRecord::new(Path::new(path), Path::new(path), identity).is_err());
        }
    }

    #[test]
    fn atomic_record_write_rejects_paths_without_parent_or_filename() {
        let fs = super::super::StdMacosFilesystem;
        for path in ["/", "", ".", "record.json"] {
            assert!(fs.write_record_atomic(Path::new(path), b"test").is_err());
        }
    }

    #[test]
    fn recovery_binds_names_identity_and_independent_proof_and_rejects_phase_rewind() {
        for staging in [false, true] {
            for mutation in [
                "id",
                "target",
                "backup-root",
                "identity",
                "proof",
                "missing-proof",
                "replacement-identity",
                "committed-rewind",
            ] {
                let fs = FakeFilesystem::new();
                let parent = Path::new(USER_APPLICATIONS);
                let target = parent.join("Codex--Work.app");
                let id = transaction_id();
                let backup = transaction_path(&target, &id, staging).unwrap();
                add_bundle(&fs, &backup);
                let mut record =
                    UpdateRecord::new(&target, &backup, fs.identity(&backup).unwrap()).unwrap();
                assert_eq!(record.transaction_id, id);
                assert_eq!(record.record_file, fixed_record_path(&target, &id).unwrap());
                record.save(&fs).unwrap();
                let fixed = record.record_file.clone();
                match mutation {
                    "id" => record.transaction_id = transaction_id(),
                    "target" => record.target = parent.join("Other.app"),
                    "backup-root" => record.backup = PathBuf::from("/"),
                    "identity" => record.backup_identity.inode += 1,
                    "proof" => {
                        fs.add_file_with_contents(record.auxiliary_path("binding"), vec![0; 32])
                    }
                    "missing-proof" => {
                        fs.remove_record(&record.auxiliary_path("binding")).unwrap();
                    }
                    "replacement-identity" => {
                        fs.remove_dir_all(&backup).unwrap();
                        add_bundle(&fs, &backup);
                        record.backup_identity = fs.identity(&backup).unwrap();
                    }
                    "committed-rewind" => {
                        record.advance(&fs, UpdatePhase::Committed).unwrap();
                        record.phase = UpdatePhase::Swapped;
                    }
                    _ => unreachable!(),
                }
                fs.add_file_with_contents(&fixed, serde_json::to_vec(&record).unwrap());
                let runner = FakeRunner::new();
                let host = host();
                recover_interrupted_updates(
                    &runner,
                    &fs,
                    &BundleTransactionPolicy::Codex { host: &host },
                    parent,
                )
                .unwrap();
                assert!(!fs.contains(&target), "{mutation}");
                assert!(!fs.contains(parent.join("Other.app")), "{mutation}");
                assert!(fs.contains(&backup) && fs.contains(&fixed), "{mutation}");
                runner.assert_drained();
            }
        }
    }

    #[test]
    fn recovery_selects_newest_eligible_backup_across_swap_and_fallback() {
        for newest_staging in [false, true] {
            let fs = FakeFilesystem::new();
            let parent = Path::new(USER_APPLICATIONS);
            let target = parent.join("Codex.app");
            let mut paths = Vec::new();
            for (time, staging) in [
                (1, !newest_staging),
                (2, newest_staging),
                (3, !newest_staging),
            ] {
                let id = format!("{time}-{}", Uuid::new_v4());
                let backup = transaction_path(&target, &id, staging).unwrap();
                add_bundle(&fs, &backup);
                fs.add_file(backup.join(format!("version-{time}")));
                let mut record =
                    UpdateRecord::new(&target, &backup, fs.identity(&backup).unwrap()).unwrap();
                record.save(&fs).unwrap();
                if time == 3 {
                    record.advance(&fs, UpdatePhase::Committed).unwrap();
                }
                paths.push(backup);
            }
            let runner = FakeRunner::new();
            queue_read_and_validate(&runner, "1");
            super::super::signature::tests::queue_valid_signature(&runner);
            let host = host();
            recover_interrupted_updates(
                &runner,
                &fs,
                &BundleTransactionPolicy::Codex { host: &host },
                parent,
            )
            .unwrap();
            assert!(fs.contains(target.join("version-2")));
            assert!(fs.contains(&paths[0]) && fs.contains(&paths[2]));
            assert!(!fs.contains(&paths[1]));
            runner.assert_drained();
        }
    }

    fn record_failure_transaction(fallback: bool, operation: &'static str, phase: &str) {
        let (fs, source_path) = fixture_filesystem();
        let parent = Path::new(USER_APPLICATIONS);
        let target_path = parent.join("Renamed Codex.app");
        add_bundle(&fs, &target_path);
        fs.add_file(target_path.join("old-marker"));
        fs.add_file(source_path.join("new-marker"));
        let runner = Arc::new(FakeRunner::new());
        install_copy_hook(&runner, &fs);
        let host = host();
        let policy = BundleTransactionPolicy::Codex { host: &host };
        queue_read_and_validate(&runner, "2");
        let source = policy.inspect_source(&*runner, &*fs, &source_path).unwrap();
        queue_read_and_validate(&runner, "1");
        let existing = policy
            .inspect_existing(&*runner, &*fs, &target_path)
            .unwrap();
        let target = TargetPlan {
            parent: parent.into(),
            target: target_path.clone(),
            existing: Some(existing),
            allows_permission_fallback: false,
        };
        if fallback {
            fs.fail_operation("swap", MacosFilesystemErrorKind::Unsupported);
        }
        let rollback = matches!(phase, "terminal" | "tampered-rollback" | "swapped-invalid");
        if phase != "tampered-rollback" {
            let skip = match phase {
                "swapped" | "swapped-invalid" => 2,
                "terminal" => 3,
                "committed" => 4,
                "cleanup" => {
                    if fallback {
                        3
                    } else {
                        0
                    }
                }
                _ => unreachable!(),
            };
            fs.fail_operation_after(operation, skip, MacosFilesystemErrorKind::Other);
        }
        runner.queue_success("/usr/bin/ditto", vec![]);
        queue_read_and_validate(&runner, "2");
        queue_read_and_validate(&runner, "1");
        runner.queue_success("/usr/bin/osascript", b"[]".to_vec());
        super::super::signature::tests::queue_valid_signature(&runner);
        queue_read_and_validate(&runner, "2"); // Installed replacement must always be checked.
        if rollback {
            queue_read_and_validate(&runner, "1");
            super::super::signature::tests::queue_valid_signature(&runner);
            queue_read_and_validate(&runner, "2");
            super::super::signature::tests::queue_valid_signature(&runner);
            if fallback {
                queue_read_and_validate(&runner, "2");
                super::super::signature::tests::queue_valid_signature(&runner);
                queue_read_and_validate(&runner, "1");
            } else {
                queue_read_and_validate(&runner, "1");
                queue_read_and_validate(&runner, "2");
                super::super::signature::tests::queue_valid_signature(&runner);
            }
        } else if phase != "committed" {
            queue_read_and_validate(&runner, "1");
            super::super::signature::tests::queue_valid_signature(&runner);
        }
        let outside = Path::new("/outside/sentinel.transaction.json");
        fs.add_file_with_contents(outside, b"untouched".to_vec());
        let checked = std::cell::Cell::new(false);
        let result = install_at_target(
            &*runner,
            &*fs,
            &source,
            &target,
            &policy,
            Arc::new(|_| {}),
            TransactionHooks {
                before_commit: || Ok(()),
                post_commit_verify: |_: &VerifiedBundle| {
                    checked.set(true);
                    if phase == "tampered-rollback" {
                        let entry = fs
                            .read_dir(parent)
                            .unwrap()
                            .into_iter()
                            .find(|p| {
                                if fallback {
                                    is_transaction_backup(p)
                                } else {
                                    is_transaction_staging(p)
                                }
                            })
                            .unwrap();
                        let mut record = read_record(&*fs, &entry).unwrap();
                        let fixed = record.record_file.clone();
                        record.backup = if fallback {
                            PathBuf::from("/outside/sentinel")
                        } else {
                            PathBuf::from("/")
                        };
                        record.target = PathBuf::from("/outside/Other.app");
                        fs.add_file_with_contents(fixed, serde_json::to_vec(&record).unwrap());
                    }
                    if matches!(phase, "tampered-rollback" | "swapped-invalid") {
                        Err(error(
                            InstallerErrorCode::InstallationVerifyFailed,
                            "注入验证失败",
                        ))
                    } else {
                        Ok(())
                    }
                },
            },
        );
        assert!(result.is_err(), "{fallback} {operation} {phase}");
        assert!(checked.get(), "post-commit verification was skipped");
        assert!(fs.contains(target_path.join(if rollback { "old-marker" } else { "new-marker" })));
        assert_eq!(fs.read_file_bounded(outside, 32).unwrap(), b"untouched");
        if phase == "committed" {
            let entry = fs
                .read_dir(parent)
                .unwrap()
                .into_iter()
                .find(|p| {
                    if fallback {
                        is_transaction_backup(p)
                    } else {
                        is_transaction_staging(p)
                    }
                })
                .unwrap();
            let mut record = read_record(&*fs, &entry).unwrap();
            assert!(matches!(record.phase, UpdatePhase::Swapped));
            assert!(fs.contains(record.auxiliary_path("terminal")));
            record.phase = UpdatePhase::Swapped;
            fs.add_file_with_contents(&record.record_file, serde_json::to_vec(&record).unwrap());
            fs.remove_dir_all(&target_path).unwrap(); // User uninstalls the accepted replacement.
            recover_interrupted_updates(&*runner, &*fs, &policy, parent).unwrap();
            assert!(!fs.contains(&target_path));
            assert!(fs.contains(&entry));
        }
        runner.assert_drained();
    }

    #[test]
    fn swapped_record_write_and_rename_failures_still_verify_and_roll_back_invalid_replacement() {
        for fallback in [false, true] {
            for operation in ["write_record", "rename_record"] {
                for phase in ["swapped", "swapped-invalid"] {
                    record_failure_transaction(fallback, operation, phase);
                }
            }
        }
    }

    #[test]
    fn committed_record_write_and_rename_failures_cannot_restore_after_uninstall() {
        for fallback in [false, true] {
            for operation in ["write_record", "rename_record"] {
                record_failure_transaction(fallback, operation, "committed");
            }
        }
    }

    #[test]
    fn terminal_marker_write_and_rename_failures_verify_then_roll_back() {
        for fallback in [false, true] {
            for operation in ["write_record", "rename_record"] {
                record_failure_transaction(fallback, operation, "terminal");
            }
        }
    }

    #[test]
    fn record_delete_failure_preserves_verified_installation() {
        for fallback in [false, true] {
            record_failure_transaction(fallback, "remove_record", "cleanup");
        }
    }

    #[test]
    fn rollback_uses_memory_record_and_fixed_path_despite_disk_path_tampering() {
        for fallback in [false, true] {
            record_failure_transaction(fallback, "write_record", "tampered-rollback");
        }
    }
    #[test]
    fn rollback_rejects_mismatched_memory_authority_without_touching_either_app() {
        for mutation in ["target", "backup", "id", "record-path", "identity"] {
            let fs = FakeFilesystem::new();
            let parent = Path::new(USER_APPLICATIONS);
            let target = parent.join("Codex.app");
            let backup = transaction_path(&target, &transaction_id(), true).unwrap();
            add_bundle(&fs, &target);
            add_bundle(&fs, &backup);
            let runner = FakeRunner::new();
            let host = host();
            let policy = BundleTransactionPolicy::Codex { host: &host };
            queue_read_and_validate(&runner, "2");
            let replacement = policy.inspect_source(&runner, &fs, &target).unwrap();
            queue_read_and_validate(&runner, "1");
            let old = policy.inspect_source(&runner, &fs, &backup).unwrap();
            let mut record = UpdateRecord::new(&target, &backup, old.identity).unwrap();
            record.save(&fs).unwrap();
            let fixed = record.record_file.clone();
            let before = fs.read_file_bounded(&fixed, 16 * 1024).unwrap();
            match mutation {
                "target" => record.target = parent.join("Other.app"),
                "backup" => record.backup = PathBuf::from("/"),
                "id" => record.transaction_id = transaction_id(),
                "record-path" => record.record_file = PathBuf::from("/outside/record.json"),
                "identity" => record.backup_identity.inode += 1,
                _ => unreachable!(),
            }
            assert!(restore_backup(
                &runner,
                &fs,
                &policy,
                RestoreRequest {
                    parent,
                    target: &target,
                    backup: &backup,
                    expected_replacement: &replacement,
                    expected_backup: &old,
                    record: &mut record,
                }
            )
            .is_err());
            assert_eq!(fs.identity(&target).unwrap(), replacement.identity);
            assert_eq!(fs.identity(&backup).unwrap(), old.identity);
            assert_eq!(fs.read_file_bounded(&fixed, 16 * 1024).unwrap(), before);
            runner.assert_drained();
        }
    }
    // Every filesystem operation (including the copy command's effect)
    // is a crash boundary. Recreate the runner after the crash: no in-memory
    // UpdateRecord or queued command response survives a process restart.
    fn crash_transaction(
        fallback: bool,
        rollback: bool,
        crash: usize,
        rename_failure: bool,
    ) -> (usize, bool, bool) {
        let (fs, source_path) = fixture_filesystem();
        let parent = Path::new(USER_APPLICATIONS);
        let target_path = parent.join("Codex.app");
        add_bundle(&fs, &target_path);
        fs.add_file(target_path.join("old-marker"));
        fs.add_file(source_path.join("new-marker"));
        let runner = Arc::new(FakeRunner::new());
        install_copy_hook(&runner, &fs);
        let host = host();
        let policy = BundleTransactionPolicy::Codex { host: &host };
        queue_read_and_validate(&runner, "2");
        let source = policy.inspect_source(&*runner, &*fs, &source_path).unwrap();
        queue_read_and_validate(&runner, "1");
        let existing = policy
            .inspect_existing(&*runner, &*fs, &target_path)
            .unwrap();
        let old_identity = existing.identity;
        let target = TargetPlan {
            parent: parent.into(),
            target: target_path.clone(),
            existing: Some(existing),
            allows_permission_fallback: false,
        };
        if fallback {
            fs.fail_operation("swap", MacosFilesystemErrorKind::Unsupported);
        }
        runner.queue_success("/usr/bin/ditto", vec![]);
        queue_read_and_validate(&runner, "2");
        queue_read_and_validate(&runner, "1");
        runner.queue_success("/usr/bin/osascript", b"[]".to_vec());
        super::super::signature::tests::queue_valid_signature(&runner);
        queue_read_and_validate(&runner, "2");
        queue_read_and_validate(&runner, "1");
        super::super::signature::tests::queue_valid_signature(&runner);
        if rollback {
            queue_read_and_validate(&runner, "2");
            super::super::signature::tests::queue_valid_signature(&runner);
            if fallback {
                queue_read_and_validate(&runner, "2");
                super::super::signature::tests::queue_valid_signature(&runner);
                queue_read_and_validate(&runner, "1");
            } else {
                queue_read_and_validate(&runner, "1");
                queue_read_and_validate(&runner, "2");
                super::super::signature::tests::queue_valid_signature(&runner);
            }
        }
        fs.crash_after_operation(crash);
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            install_at_target(
                &*runner,
                &*fs,
                &source,
                &target,
                &policy,
                Arc::new(|_| {}),
                TransactionHooks {
                    before_commit: || Ok(()),
                    post_commit_verify: |_: &VerifiedBundle| {
                        let backup = fs
                            .read_dir(parent)
                            .unwrap()
                            .into_iter()
                            .find(|p| {
                                if fallback {
                                    is_transaction_backup(p)
                                } else {
                                    is_transaction_staging(p)
                                }
                            })
                            .unwrap();
                        let record = read_record(&*fs, &backup).unwrap();
                        assert!(
                            !fs.contains(record.auxiliary_path("terminal")),
                            "terminal must not precede inventory verification"
                        );
                        if rename_failure {
                            fs.fail_operation("rename", MacosFilesystemErrorKind::Other);
                        }
                        if rollback {
                            Err(error(
                                InstallerErrorCode::InstallationVerifyFailed,
                                "模拟清单验证失败",
                            ))
                        } else {
                            Ok(())
                        }
                    },
                },
            )
        }));
        let count = fs.completed_operations();
        if crash != usize::MAX {
            let panic = outcome.err().expect("crash boundary was not reached");
            assert_eq!(
                panic.downcast_ref::<&str>(),
                Some(&"simulated filesystem crash")
            );
        } else {
            let result = outcome.unwrap();
            assert_eq!(result.is_err(), rollback);
            if !rename_failure {
                runner.assert_drained();
            }
        }
        // Disable crash injection before restarting, including baseline runs.
        fs.crash_after_operation(usize::MAX);
        if rename_failure {
            assert!(
                !fs.contains(&target_path),
                "delete must succeed before rename fails"
            );
            let backup = fs
                .read_dir(parent)
                .unwrap()
                .into_iter()
                .find(|p| is_transaction_backup(p))
                .unwrap();
            let record = read_record(&*fs, &backup).unwrap();
            assert!(matches!(record.phase, UpdatePhase::RollingBack));
            assert_eq!(fs.identity(&backup).unwrap(), old_identity);
        }
        let restarted = FakeRunner::new();
        let missing = !fs.contains(&target_path);
        let interrupted_rollback = missing
            && fs.read_dir(parent).unwrap().iter().any(|p| {
                read_record(&*fs, p).is_some_and(|r| matches!(r.phase, UpdatePhase::RollingBack))
            });
        if missing {
            queue_read_and_validate(&restarted, "1");
            super::super::signature::tests::queue_valid_signature(&restarted);
        }
        recover_interrupted_updates(&restarted, &*fs, &policy, parent).unwrap();
        assert!(
            fs.contains(&target_path),
            "fallback={fallback}, rollback={rollback}, crash={crash}"
        );
        if fs.contains(target_path.join("old-marker")) {
            assert_eq!(fs.identity(&target_path).unwrap(), old_identity);
        } else {
            assert!(fs.contains(target_path.join("new-marker")));
            // Independently verify the surviving replacement after restart,
            // including signature, version, executable and exact target path.
            queue_read_and_validate(&restarted, "2");
            super::super::signature::tests::queue_valid_signature(&restarted);
            let verified =
                verify_transaction_bundle(&restarted, &*fs, &policy, &target_path).unwrap();
            assert!(policy.copies_are_equivalent(&source, &verified));
        }
        restarted.assert_drained();
        (count, missing, interrupted_rollback)
    }

    #[test]
    fn update_recovers_after_every_filesystem_operation_in_swap_and_fallback() {
        for fallback in [false, true] {
            for rollback in [false, true] {
                let (count, _, _) = crash_transaction(fallback, rollback, usize::MAX, false);
                assert!(count > 10);
                let mut missing_target_crashes = 0;
                for crash in 1..=count {
                    let (_, missing, _) = crash_transaction(fallback, rollback, crash, false);
                    missing_target_crashes += usize::from(missing);
                }
                if fallback {
                    assert!(
                        missing_target_crashes > 0,
                        "must exercise missing-target recovery"
                    );
                }
                eprintln!("fallback={fallback}, rollback={rollback}: {count} crash boundaries, {missing_target_crashes} missing targets restored");
            }
        }
    }

    #[test]
    fn fallback_delete_then_crash_recovers_on_restart() {
        let (count, _, _) = crash_transaction(true, true, usize::MAX, false);
        assert!(
            (1..=count).any(|crash| crash_transaction(true, true, crash, false).2),
            "must crash between deleting the replacement and restoring the old backup"
        );
    }

    #[test]
    fn fallback_delete_then_rename_failure_recovers_on_restart() {
        crash_transaction(true, true, usize::MAX, true);
    }

    #[test]
    fn rolling_back_recovers_despite_legacy_terminal_but_requires_binding() {
        for staging in [false, true] {
            for valid_binding in [false, true] {
                let fs = FakeFilesystem::new();
                let parent = Path::new(USER_APPLICATIONS);
                let target = parent.join("Codex.app");
                let backup = transaction_path(&target, &transaction_id(), staging).unwrap();
                add_bundle(&fs, &backup);
                let identity = fs.identity(&backup).unwrap();
                let mut record = UpdateRecord::new(&target, &backup, identity).unwrap();
                record.save(&fs).unwrap();
                record.retire(&fs).unwrap();
                record.advance(&fs, UpdatePhase::RollingBack).unwrap();
                if !valid_binding {
                    fs.remove_record(&record.auxiliary_path("binding")).unwrap();
                }
                let runner = FakeRunner::new();
                if valid_binding {
                    queue_read_and_validate(&runner, "1");
                    super::super::signature::tests::queue_valid_signature(&runner);
                }
                let host = host();
                recover_interrupted_updates(
                    &runner,
                    &fs,
                    &BundleTransactionPolicy::Codex { host: &host },
                    parent,
                )
                .unwrap();
                assert_eq!(fs.contains(&target), valid_binding);
                assert_eq!(fs.contains(&backup), !valid_binding);
                if valid_binding {
                    assert_eq!(fs.identity(&target).unwrap(), identity);
                }
                runner.assert_drained();
            }
        }
    }

    #[test]
    fn rollback_state_or_terminal_revocation_failure_preserves_both_apps() {
        for staging in [false, true] {
            for operation in ["write_record", "rename_record", "remove_record"] {
                let fs = FakeFilesystem::new();
                let parent = Path::new(USER_APPLICATIONS);
                let target = parent.join("Codex.app");
                let backup = transaction_path(&target, &transaction_id(), staging).unwrap();
                add_bundle(&fs, &target);
                add_bundle(&fs, &backup);
                let runner = FakeRunner::new();
                let host = host();
                let policy = BundleTransactionPolicy::Codex { host: &host };
                queue_read_and_validate(&runner, "2");
                let replacement = policy.inspect_source(&runner, &fs, &target).unwrap();
                queue_read_and_validate(&runner, "1");
                let old = policy.inspect_source(&runner, &fs, &backup).unwrap();
                let mut record = UpdateRecord::new(&target, &backup, old.identity).unwrap();
                record.save(&fs).unwrap();
                record.retire(&fs).unwrap();
                fs.fail_operation(operation, MacosFilesystemErrorKind::Other);
                queue_read_and_validate(&runner, "1");
                super::super::signature::tests::queue_valid_signature(&runner);
                queue_read_and_validate(&runner, "2");
                super::super::signature::tests::queue_valid_signature(&runner);
                assert!(restore_backup(
                    &runner,
                    &fs,
                    &policy,
                    RestoreRequest {
                        parent,
                        target: &target,
                        backup: &backup,
                        expected_replacement: &replacement,
                        expected_backup: &old,
                        record: &mut record,
                    }
                )
                .is_err());
                assert_eq!(fs.identity(&target).unwrap(), replacement.identity);
                assert_eq!(fs.identity(&backup).unwrap(), old.identity);
                assert!(fs.contains(record.auxiliary_path("terminal")));
                runner.assert_drained();
            }
        }
    }
}
