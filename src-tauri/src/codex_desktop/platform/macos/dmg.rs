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
    MacosFilesystem, MacosHost,
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
for (let index = 0; index < applications.count; index += 1) {
  const application = applications.objectAtIndex(index);
  if (!application.bundleIdentifier || !application.bundleURL) {
    continue;
  }
  const bundleIdentifier = ObjC.unwrap(application.bundleIdentifier);
  const bundlePath = ObjC.unwrap(application.bundleURL.path);
  if (bundleIdentifier === expectedBundleIdentifier && bundlePath === expectedBundlePath) {
    console.log('running');
    $.exit(0);
  }
}
console.log('not_running');
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
        [] => Ok(PlatformInstallPlan::new(vec![
            host.applications_dir().to_path_buf(),
            host.user_applications_dir().to_path_buf(),
        ])),
        [existing] => {
            bundle::ensure_not_running(runner, filesystem, existing.bundle_path())?;
            let parent = existing.bundle_path().parent().ok_or_else(|| {
                error(
                    InstallerErrorCode::InternalError,
                    "installed Stable bundle has no parent directory",
                )
            })?;
            Ok(PlatformInstallPlan::new(vec![parent.to_path_buf()]))
        }
        _ => Err(error(
            InstallerErrorCode::MacMultipleInstallations,
            "multiple Stable macOS bundles prevent a safe update",
        )),
    }
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
        let policy = BundleTransactionPolicy::Codex { host };
        let source_bundle = policy.inspect_source_info(filesystem, source_bundle)?;
        let targets = plan_targets(runner, filesystem, host, &source_bundle, &policy)?;
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
        (Ok(installed), Ok(())) => {
            progress.report_progress(JobProgress::new(
                ProgressPhase::Installation,
                Some(3),
                Some(3),
            ));
            Ok(bundle::installed_application(&installed.info))
        }
        (Ok(_), Err(detach_error)) => Err(detach_error),
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
                "hdiutil",
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
            "hdiutil",
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
            "hdiutil",
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
            "osascript",
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
    if !output.is_success() {
        return Err(error(
            InstallerErrorCode::MacAppRunning,
            "managed application runtime state could not be determined",
        ));
    }
    match std::str::from_utf8(output.stdout()).map(str::trim) {
        Ok("not_running") => Ok(()),
        Ok("running") => Err(error(
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
}

fn plan_targets(
    runner: &dyn CommandRunner,
    filesystem: &dyn MacosFilesystem,
    host: &MacosHost,
    source_bundle: &VerifiedBundle,
    policy: &BundleTransactionPolicy<'_>,
) -> Result<Vec<TargetPlan>, InstallerError> {
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

    let transaction_id = Uuid::new_v4().hyphenated().to_string();
    let staging = parent.join(format!("{STAGING_PREFIX}{transaction_id}{STAGING_SUFFIX}"));
    let backup = parent.join(format!("{BACKUP_PREFIX}{transaction_id}{BACKUP_SUFFIX}"));
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
        "ditto",
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
        policy
            .inspect_existing(runner, filesystem, &target_path)
            .map_err(TransactionFailure::terminal)?;
        policy
            .ensure_not_running(runner, filesystem, &target_path)
            .map_err(TransactionFailure::terminal)?;
        filesystem
            .rename(&target_path, &backup)
            .map_err(|filesystem_error| {
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
                },
            );
            let _ = remove_generated_path(
                filesystem,
                &parent,
                &staging,
                STAGING_PREFIX,
                STAGING_SUFFIX,
            );
            let replacement_error = error(
                InstallerErrorCode::MacCopyFailed,
                "new Stable application could not replace the existing bundle",
            );
            return Err(if restored.is_ok() {
                if is_permission_denied(filesystem_error) {
                    TransactionFailure::permission_denied(replacement_error)
                } else {
                    TransactionFailure::Restored(replacement_error)
                }
            } else {
                TransactionFailure::RecoveryRequired(replacement_error)
            });
        }
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
                    },
                );
                return Err(if restored.is_ok() {
                    TransactionFailure::Restored(error(
                        InstallerErrorCode::InstallationVerifyFailed,
                        "replacement Stable application could not be verified and was restored",
                    ))
                } else {
                    TransactionFailure::RecoveryRequired(error(
                        InstallerErrorCode::InstallationVerifyFailed,
                        "replacement Stable application could not be verified or safely restored",
                    ))
                });
            }
        };
        if post_commit_verify(&installed_bundle).is_err() {
            let restored = restore_backup(
                runner,
                filesystem,
                policy,
                RestoreRequest {
                    parent: &parent,
                    target: &target_path,
                    backup: &backup,
                    expected_replacement: &installed_bundle,
                    expected_backup,
                },
            );
            return Err(if restored.is_ok() {
                TransactionFailure::Restored(error(
                    InstallerErrorCode::InstallationVerifyFailed,
                    "post-install inventory verification failed and the previous application was restored",
                ))
            } else {
                TransactionFailure::RecoveryRequired(error(
                    InstallerErrorCode::InstallationVerifyFailed,
                    "post-install inventory verification failed and recovery could not be completed",
                ))
            });
        }
        remove_generated_path(filesystem, &parent, &backup, BACKUP_PREFIX, BACKUP_SUFFIX)
            .map_err(TransactionFailure::RecoveryRequired)?;
        Ok(installed_bundle)
    } else {
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
                    let _ = remove_expected_replacement(
                        runner,
                        filesystem,
                        policy,
                        &parent,
                        &target_path,
                        &staging_bundle,
                    );
                    TransactionFailure::Restored(error(
                        InstallerErrorCode::InstallationVerifyFailed,
                        "new application could not be verified after installation",
                    ))
                })?;
        if post_commit_verify(&installed_bundle).is_err() {
            let removed = remove_expected_replacement(
                runner,
                filesystem,
                policy,
                &parent,
                &target_path,
                &installed_bundle,
            );
            return Err(if removed.is_ok() {
                TransactionFailure::Restored(error(
                    InstallerErrorCode::InstallationVerifyFailed,
                    "post-install inventory verification failed and the new application was removed",
                ))
            } else {
                TransactionFailure::RecoveryRequired(error(
                    InstallerErrorCode::InstallationVerifyFailed,
                    "post-install inventory verification failed and the new application could not be removed safely",
                ))
            });
        }
        Ok(installed_bundle)
    }
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
    if !policy.copies_are_equivalent(expected_source, &installed) {
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
    if !policy.copies_are_equivalent(request.expected_backup, &restored) {
        return Err(error(
            InstallerErrorCode::InstallationVerifyFailed,
            "restored application did not match the verified backup identity",
        ));
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
    let current = policy.inspect_source(runner, filesystem, target)?;
    if !policy.copies_are_equivalent(expected, &current) {
        return Err(error(
            InstallerErrorCode::InstallationVerifyFailed,
            "cleanup refused an application that differs from the staged replacement",
        ));
    }
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
        runner.queue_success("plutil", plist(bundle_id, version));
    }

    fn queue_managed_read(
        runner: &FakeRunner,
        bundle_id: &str,
        bundle_version: &str,
        short_version: &str,
    ) {
        runner.queue_success(
            "plutil",
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
            if invocation.program() == "ditto" {
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
        runner.queue_success("hdiutil", mount_plist(MOUNT_POINT));
        queue_read_with_identity(runner, bundle_id, version); // mounted source
        runner.queue_success("ditto", Vec::<u8>::new());
        queue_read_with_identity(runner, bundle_id, version); // staging
        queue_read_with_identity(runner, bundle_id, version); // target
        runner.queue_success("hdiutil", Vec::<u8>::new());
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
        runner.queue_success("hdiutil", mount_plist(MOUNT_POINT));
        runner.queue_success("hdiutil", Vec::<u8>::new());

        let mount_error = mount_dmg(&runner, &filesystem, Path::new(ARTIFACT))
            .err()
            .expect("an uncanonicalizable mounted path must fail");
        assert_eq!(mount_error.code(), InstallerErrorCode::MacDmgMountFailed);
        let commands = runner
            .invocations()
            .into_iter()
            .map(|invocation| invocation.program())
            .collect::<Vec<_>>();
        assert_eq!(commands, ["hdiutil", "hdiutil"]);
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
    fn fresh_install_preserves_dmg_bundle_basename_and_reports_progress() {
        let (filesystem, source_bundle) = fixture_filesystem();
        let runner = Arc::new(FakeRunner::new());
        let filesystem_for_ditto = filesystem.clone();
        runner.set_hook(Arc::new(move |invocation| {
            if invocation.program() == "ditto" {
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
        assert_eq!(attach.program(), "hdiutil");
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
            .find(|invocation| invocation.program() == "ditto")
            .unwrap();
        assert_eq!(ditto.arguments()[0], source_bundle.as_os_str());
        assert!(Path::new(&ditto.arguments()[1])
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with(STAGING_PREFIX) && name.ends_with(".app")));
        runner.assert_drained();
    }

    #[test]
    fn downloaded_bundle_identity_and_version_drift_do_not_block_fresh_install() {
        let (filesystem, _) = fixture_filesystem();
        let runner = Arc::new(FakeRunner::new());
        let filesystem_for_ditto = filesystem.clone();
        runner.set_hook(Arc::new(move |invocation| {
            if invocation.program() == "ditto" {
                filesystem_for_ditto
                    .copy_tree(
                        PathBuf::from(invocation.arguments()[0].clone()),
                        PathBuf::from(invocation.arguments()[1].clone()),
                    )
                    .unwrap();
            }
        }));
        queue_fresh_install_with_identity(runner.as_ref(), "com.example.future", "1");
        let release = release("9999");

        let installed = install_current_user(
            runner.as_ref(),
            filesystem.as_ref(),
            &host(),
            &package(&release),
            Arc::new(|_| {}),
        )
        .unwrap();

        assert_eq!(installed.stable_identity, "com.example.future");
        assert_eq!(
            installed.platform_version,
            PlatformVersion::parse_mac_bundle("1").unwrap()
        );
        runner.assert_drained();
    }

    #[test]
    fn failed_bundle_copy_cleans_the_generated_staging_bundle() {
        let (filesystem, _) = fixture_filesystem();
        let runner = Arc::new(FakeRunner::new());
        let filesystem_for_ditto = filesystem.clone();
        runner.set_hook(Arc::new(move |invocation| {
            if invocation.program() == "ditto" {
                filesystem_for_ditto
                    .copy_tree(
                        PathBuf::from(invocation.arguments()[0].clone()),
                        PathBuf::from(invocation.arguments()[1].clone()),
                    )
                    .unwrap();
            }
        }));
        runner.queue_success("hdiutil", mount_plist(MOUNT_POINT));
        queue_read_and_validate(runner.as_ref(), "5848"); // mounted source
        runner.queue_failure("ditto", Some(1), b"copy failed".to_vec());
        runner.queue_success("hdiutil", Vec::<u8>::new()); // detach
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
            .find(|invocation| invocation.program() == "ditto")
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
            if invocation.program() == "ditto" {
                filesystem_for_ditto
                    .copy_tree(
                        PathBuf::from(invocation.arguments()[0].clone()),
                        PathBuf::from(invocation.arguments()[1].clone()),
                    )
                    .unwrap();
            }
        }));
        runner.queue_success("hdiutil", mount_plist(MOUNT_POINT));
        queue_read_and_validate(runner.as_ref(), "5848"); // mounted source
        runner.queue_success("ditto", Vec::<u8>::new());
        runner.queue_failure("plutil", Some(1), b"invalid staged plist".to_vec());
        runner.queue_success("hdiutil", Vec::<u8>::new()); // detach
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
            .find(|invocation| invocation.program() == "ditto")
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
            if invocation.program() == "ditto" {
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
        runner.queue_success("hdiutil", mount_plist(MOUNT_POINT));
        queue_managed_read(&runner, BUNDLE_ID, "1.2.3", "1.2.3");
        queue_managed_read(&runner, BUNDLE_ID, "1.2.3", "1.2.3");
        runner.queue_success("hdiutil", Vec::<u8>::new());
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
            .all(|invocation| invocation.program() != "ditto"));
        runner.assert_drained();
    }

    #[test]
    fn managed_system_update_rechecks_target_and_running_state_before_commit() {
        const BUNDLE_ID: &str = "ai.opencode.desktop";
        let (filesystem, source_bundle) = fixture_filesystem();
        let target = Path::new(SYSTEM_APPLICATIONS).join("OpenCode.app");
        add_bundle(filesystem.as_ref(), &target);
        let runner = FakeRunner::new();
        runner.queue_success("hdiutil", mount_plist(MOUNT_POINT));
        queue_managed_read(&runner, BUNDLE_ID, "1.2.3", "1.2.3");
        queue_managed_read(&runner, BUNDLE_ID, "1.2.2", "1.2.2");
        runner.queue_success("osascript", b"not_running\n".to_vec());
        queue_managed_read(&runner, BUNDLE_ID, "1.2.3", "1.2.3");
        runner.queue_success("hdiutil", Vec::<u8>::new());
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
                .filter(|invocation| invocation.program() == "osascript")
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
        runner.queue_success("hdiutil", mount_plist(MOUNT_POINT));
        queue_managed_read(&runner, BUNDLE_ID, "1.2.3", "1.2.3");
        queue_managed_read(&runner, BUNDLE_ID, "1.2.2", "1.2.2");
        runner.queue_success("osascript", b"running\n".to_vec());
        runner.queue_success("hdiutil", Vec::<u8>::new());
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
        runner.queue_success("hdiutil", mount_plist(MOUNT_POINT));
        queue_managed_read(&runner, BUNDLE_ID, "1.2.3", "1.2.3");
        runner.queue_success("hdiutil", Vec::<u8>::new());
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
                .filter(|invocation| invocation.program() == "hdiutil")
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
        runner.queue_success("hdiutil", mount_plist(MOUNT_POINT));
        queue_managed_read(&runner, BUNDLE_ID, "1.2.3", "1.2.3");
        queue_managed_read(&runner, BUNDLE_ID, "1.2.2", "1.2.2");
        runner.queue_success("hdiutil", Vec::<u8>::new());
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
        const BUNDLE_ID: &str = "com.workbuddy.workbuddy";
        let (filesystem, _) = fixture_filesystem();
        let target = Path::new(USER_APPLICATIONS).join("Existing WorkBuddy.app");
        add_bundle(filesystem.as_ref(), &target);
        let runner = Arc::new(FakeRunner::new());
        install_copy_hook(&runner, &filesystem);
        runner.queue_success("hdiutil", mount_plist(MOUNT_POINT));
        queue_managed_read(runner.as_ref(), BUNDLE_ID, "36279234", "5.3.14");
        queue_managed_read(runner.as_ref(), BUNDLE_ID, "36200000", "5.3.13");
        runner.queue_success("osascript", b"not_running\n".to_vec());
        runner.queue_success("ditto", Vec::<u8>::new());
        queue_managed_read(runner.as_ref(), BUNDLE_ID, "36279234", "5.3.14");
        queue_managed_read(runner.as_ref(), BUNDLE_ID, "36200000", "5.3.13");
        runner.queue_success("osascript", b"not_running\n".to_vec());
        queue_managed_read(runner.as_ref(), BUNDLE_ID, "36279234", "5.3.14");
        runner.queue_success("hdiutil", Vec::<u8>::new());
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
        runner.queue_success("hdiutil", mount_plist(MOUNT_POINT));
        queue_managed_read(runner.as_ref(), BUNDLE_ID, "0.9.15", "0.9.15");
        queue_managed_read(runner.as_ref(), BUNDLE_ID, "0.9.12", "0.9.12");
        runner.queue_success("osascript", b"not_running\n".to_vec());
        runner.queue_success("ditto", Vec::<u8>::new());
        queue_managed_read(runner.as_ref(), BUNDLE_ID, "0.9.15", "0.9.15");
        runner.queue_success("hdiutil", Vec::<u8>::new());
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
            .find(|invocation| invocation.program() == "ditto")
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
        runner.queue_success("hdiutil", mount_plist(MOUNT_POINT));
        queue_managed_read(runner.as_ref(), BUNDLE_ID, "0.9.15", "0.9.15");
        queue_managed_read(runner.as_ref(), BUNDLE_ID, "0.9.12", "0.9.12");
        runner.queue_success("osascript", b"not_running\n".to_vec());
        runner.queue_success("ditto", Vec::<u8>::new());
        queue_managed_read(runner.as_ref(), BUNDLE_ID, "0.9.15", "0.9.15");
        queue_managed_read(runner.as_ref(), BUNDLE_ID, "0.9.12", "0.9.12");
        runner.queue_success("osascript", b"not_running\n".to_vec());
        queue_managed_read(runner.as_ref(), BUNDLE_ID, "0.9.15", "0.9.15");
        queue_managed_read(runner.as_ref(), BUNDLE_ID, "0.9.15", "0.9.15");
        queue_managed_read(runner.as_ref(), BUNDLE_ID, "0.9.12", "0.9.12");
        runner.queue_success("hdiutil", Vec::<u8>::new());
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
        const BUNDLE_ID: &str = "com.workbuddy.workbuddy";
        let (filesystem, _) = fixture_filesystem();
        let target = Path::new(USER_APPLICATIONS).join("WorkBuddy.app");
        add_bundle(filesystem.as_ref(), &target);
        let runner = FakeRunner::new();
        runner.queue_success("hdiutil", mount_plist(MOUNT_POINT));
        queue_managed_read(&runner, BUNDLE_ID, "100", "5.3.14");
        queue_managed_read(&runner, BUNDLE_ID, "99", "5.3.13");
        runner.queue_success("osascript", b"running\n".to_vec());
        runner.queue_success("hdiutil", Vec::<u8>::new());
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
            .all(|invocation| invocation.program() != "ditto"));
        runner.assert_drained();
    }

    #[test]
    fn trae_managed_install_uses_bounded_product_json_for_release_verification() {
        const BUNDLE_ID: &str = "cn.trae.solo.app";
        let (filesystem, source_bundle) = fixture_filesystem();
        add_trae_product_json(filesystem.as_ref(), &source_bundle, "2.3.71801");
        let runner = Arc::new(FakeRunner::new());
        install_copy_hook(&runner, &filesystem);
        runner.queue_success("hdiutil", mount_plist(MOUNT_POINT));
        queue_managed_read(runner.as_ref(), BUNDLE_ID, "0.1.51", "0.1.51");
        runner.queue_success("ditto", Vec::<u8>::new());
        queue_managed_read(runner.as_ref(), BUNDLE_ID, "0.1.51", "0.1.51");
        queue_managed_read(runner.as_ref(), BUNDLE_ID, "0.1.51", "0.1.51");
        runner.queue_success("hdiutil", Vec::<u8>::new());
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
        let backup = parent.join(format!("{BACKUP_PREFIX}test{BACKUP_SUFFIX}"));
        filesystem.add_dir(parent);
        add_bundle(&filesystem, &target);
        filesystem.add_dir(&backup);
        let runner = FakeRunner::new();
        let host = host();
        let policy = BundleTransactionPolicy::Codex { host: &host };
        runner.queue_success("plutil", plist(stable_bundle_id(), "5848"));
        let expected = policy
            .inspect_source(&runner, &filesystem, &target)
            .unwrap();
        runner.queue_success("plutil", plist("com.example.changed", "5848"));

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
                },
            )
            .unwrap_err()
            .code(),
            InstallerErrorCode::InstallationVerifyFailed
        );
        assert!(filesystem.contains(&target));
        assert!(filesystem.contains(&backup));
        runner.assert_drained();
    }
}
