//! DMG mount, discovery, and same-volume replacement transaction.
//!
//! The transaction never uses a remote-supplied filename for a staging or
//! backup path. It only removes generated transaction paths or a target which
//! was freshly re-verified as the exact Stable bundle identity.

use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

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

const STAGING_PREFIX: &str = ".fyagent-codex-install-";
const BACKUP_PREFIX: &str = ".fyagent-codex-backup-";
const STAGING_SUFFIX: &str = ".app";
const BACKUP_SUFFIX: &str = ".backup";
const MAX_DETACH_ATTEMPTS: usize = 3;

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
    // Re-open the downloader-owned fixed DMG and bind it to the descriptor
    // immediately before `hdiutil` resolves the path. This closes the gap
    // between the earlier platform preparation and this mount.
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
        let targets = plan_targets(runner, filesystem, host, &source_bundle)?;
        let mut last_permission_error = None;

        for target in targets {
            match install_at_target(
                runner,
                filesystem,
                host,
                &source_bundle,
                &target,
                progress.clone(),
            ) {
                Ok(installed) => return Ok(installed),
                Err(attempt) if attempt.permission_denied && target.allows_permission_fallback => {
                    last_permission_error = Some(attempt.error);
                }
                Err(attempt) => return Err(attempt.error),
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
            Ok(bundle::installed_application(&installed))
        }
        (Ok(_), Err(detach_error)) => Err(detach_error),
        (Err(primary_error), _) => Err(primary_error),
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
struct TargetPlan {
    parent: PathBuf,
    target: PathBuf,
    existing_stable: Option<BundleInfo>,
    allows_permission_fallback: bool,
}

fn plan_targets(
    runner: &dyn CommandRunner,
    filesystem: &dyn MacosFilesystem,
    host: &MacosHost,
    source_bundle: &BundleInfo,
) -> Result<Vec<TargetPlan>, InstallerError> {
    let installed = bundle::scan_stable_bundles(runner, filesystem, host)?;
    match installed.as_slice() {
        [existing] => {
            bundle::ensure_not_running(runner, filesystem, existing.bundle_path())?;
            let parent = existing.bundle_path().parent().ok_or_else(|| {
                error(
                    InstallerErrorCode::InternalError,
                    "installed Stable bundle has no parent directory",
                )
            })?;
            Ok(vec![TargetPlan {
                parent: parent.to_path_buf(),
                target: existing.bundle_path().to_path_buf(),
                existing_stable: Some(existing.clone()),
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
                let target = parent.join(source_bundle.bundle_name());
                match target_is_available(filesystem, &target) {
                    Ok(()) => candidates.push(TargetPlan {
                        parent: parent.to_path_buf(),
                        target,
                        existing_stable: None,
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

struct InstallAttemptError {
    error: InstallerError,
    permission_denied: bool,
}

impl InstallAttemptError {
    fn terminal(error: InstallerError) -> Self {
        Self {
            error,
            permission_denied: false,
        }
    }

    fn permission_denied(error: InstallerError) -> Self {
        Self {
            error,
            permission_denied: true,
        }
    }
}

fn install_at_target(
    runner: &dyn CommandRunner,
    filesystem: &dyn MacosFilesystem,
    host: &MacosHost,
    source_bundle: &BundleInfo,
    target: &TargetPlan,
    progress: PlatformProgressSink,
) -> Result<BundleInfo, InstallAttemptError> {
    filesystem
        .create_dir_all(&target.parent)
        .map_err(|filesystem_error| {
            let error = error(
                InstallerErrorCode::MacCopyFailed,
                "target Applications directory could not be prepared",
            );
            if target.allows_permission_fallback && is_permission_denied(filesystem_error) {
                InstallAttemptError::permission_denied(error)
            } else {
                InstallAttemptError::terminal(error)
            }
        })?;
    let parent = filesystem.canonicalize(&target.parent).map_err(|_| {
        InstallAttemptError::terminal(error(
            InstallerErrorCode::MacCopyFailed,
            "target Applications directory could not be canonicalized",
        ))
    })?;
    if filesystem.file_kind(&parent) != Ok(MacosFileKind::Directory) {
        return Err(InstallAttemptError::terminal(error(
            InstallerErrorCode::MacCopyFailed,
            "target Applications path is not a directory",
        )));
    }
    let target_path = target
        .existing_stable
        .as_ref()
        .map(|existing| existing.bundle_path().to_path_buf())
        .unwrap_or_else(|| parent.join(source_bundle.bundle_name()));
    if let Some(existing) = &target.existing_stable {
        if target_path != target.target || target_path != existing.bundle_path() {
            return Err(InstallAttemptError::terminal(error(
                InstallerErrorCode::InternalError,
                "existing Stable bundle target changed during installation planning",
            )));
        }
    } else {
        target_is_available(filesystem, &target_path).map_err(InstallAttemptError::terminal)?;
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
    .map_err(InstallAttemptError::terminal)?;
    ensure_generated_path_absent(filesystem, &parent, &backup, BACKUP_PREFIX, BACKUP_SUFFIX)
        .map_err(InstallAttemptError::terminal)?;

    // `create_dir_all(parent)` alone does not prove a standard Applications
    // directory is writable. Probe the generated staging location first so a
    // genuinely permission-denied fresh install can fall back to the user's
    // Applications directory without interpreting an arbitrary `ditto`
    // failure as a permission condition.
    match filesystem.create_dir_all(&staging) {
        Ok(()) => {}
        Err(filesystem_error)
            if target.allows_permission_fallback && is_permission_denied(filesystem_error) =>
        {
            return Err(InstallAttemptError::permission_denied(error(
                InstallerErrorCode::MacCopyFailed,
                "system Applications directory is not writable for this user",
            )));
        }
        Err(_) => {
            return Err(InstallAttemptError::terminal(error(
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
        InstallAttemptError::terminal(error(
            InstallerErrorCode::MacCopyFailed,
            "generated staging directory could not be removed after write probing",
        ))
    })?;

    let copy_output = match runner.run(&command(
        "ditto",
        vec![
            source_bundle.bundle_path().to_path_buf().into_os_string(),
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
            return Err(InstallAttemptError::terminal(error(
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
        return Err(InstallAttemptError::terminal(error(
            InstallerErrorCode::MacCopyFailed,
            "application bundle copy failed",
        )));
    }
    let staging_bundle = match verify_staged_bundle(runner, filesystem, host, &parent, &staging) {
        Ok(bundle) => bundle,
        Err(verify_error) => {
            let _ = remove_generated_path(
                filesystem,
                &parent,
                &staging,
                STAGING_PREFIX,
                STAGING_SUFFIX,
            );
            return Err(InstallAttemptError::terminal(verify_error));
        }
    };
    progress.report_progress(JobProgress::new(
        ProgressPhase::Installation,
        Some(1),
        Some(3),
    ));

    if target.existing_stable.is_some() {
        // Re-read and re-check both identity and running state immediately
        // before moving the old application aside. This closes the user-action
        // gap between preflight and the irreversible rename.
        let existing_bundle = bundle::read_bundle_info(runner, filesystem, &target_path)
            .map_err(InstallAttemptError::terminal)?;
        bundle::validate_stable_bundle(runner, filesystem, host, &existing_bundle, None)
            .map_err(InstallAttemptError::terminal)?;
        bundle::ensure_not_running(runner, filesystem, &target_path)
            .map_err(InstallAttemptError::terminal)?;
        filesystem.rename(&target_path, &backup).map_err(|_| {
            InstallAttemptError::terminal(error(
                InstallerErrorCode::MacCopyFailed,
                "existing Stable application could not be moved to its backup",
            ))
        })?;
        if filesystem.rename(&staging, &target_path).is_err() {
            let _ = restore_backup(
                runner,
                filesystem,
                &parent,
                &target_path,
                &backup,
                &staging_bundle,
            );
            let _ = remove_generated_path(
                filesystem,
                &parent,
                &staging,
                STAGING_PREFIX,
                STAGING_SUFFIX,
            );
            return Err(InstallAttemptError::terminal(error(
                InstallerErrorCode::MacCopyFailed,
                "new Stable application could not replace the existing bundle",
            )));
        }
        progress.report_progress(JobProgress::new(
            ProgressPhase::Installation,
            Some(2),
            Some(3),
        ));
        let installed_bundle =
            match verify_installed_replacement(runner, filesystem, &target_path, &staging_bundle) {
                Ok(installed) => installed,
                Err(_) => {
                    let restored = restore_backup(
                        runner,
                        filesystem,
                        &parent,
                        &target_path,
                        &backup,
                        &staging_bundle,
                    );
                    return Err(InstallAttemptError::terminal(if restored.is_ok() {
                        error(
                            InstallerErrorCode::InstallationVerifyFailed,
                            "replacement Stable application could not be verified and was restored",
                        )
                    } else {
                        error(
                    InstallerErrorCode::InstallationVerifyFailed,
                    "replacement Stable application could not be verified or safely restored",
                )
                    }));
                }
            };
        remove_generated_path(filesystem, &parent, &backup, BACKUP_PREFIX, BACKUP_SUFFIX)
            .map_err(InstallAttemptError::terminal)?;
        Ok(installed_bundle)
    } else {
        if filesystem.rename(&staging, &target_path).is_err() {
            let _ = remove_generated_path(
                filesystem,
                &parent,
                &staging,
                STAGING_PREFIX,
                STAGING_SUFFIX,
            );
            return Err(InstallAttemptError::terminal(error(
                InstallerErrorCode::MacCopyFailed,
                "new Stable application could not be moved into Applications",
            )));
        }
        progress.report_progress(JobProgress::new(
            ProgressPhase::Installation,
            Some(2),
            Some(3),
        ));
        let installed_bundle =
            verify_installed_replacement(runner, filesystem, &target_path, &staging_bundle)
                .map_err(|_| {
                    let _ = remove_expected_replacement(
                        runner,
                        filesystem,
                        &parent,
                        &target_path,
                        &staging_bundle,
                    );
                    InstallAttemptError::terminal(error(
                        InstallerErrorCode::InstallationVerifyFailed,
                        "new application could not be verified after installation",
                    ))
                })?;
        Ok(installed_bundle)
    }
}

fn verify_staged_bundle(
    runner: &dyn CommandRunner,
    filesystem: &dyn MacosFilesystem,
    host: &MacosHost,
    parent: &Path,
    staging: &Path,
) -> Result<BundleInfo, InstallerError> {
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
    let _ = host;
    bundle::read_bundle_info(runner, filesystem, &canonical_staging)
}

fn verify_installed_replacement(
    runner: &dyn CommandRunner,
    filesystem: &dyn MacosFilesystem,
    target: &Path,
    expected_source: &BundleInfo,
) -> Result<BundleInfo, InstallerError> {
    let installed = bundle::read_bundle_info(runner, filesystem, target)?;
    if installed.bundle_identifier() != expected_source.bundle_identifier()
        || installed.platform_version() != expected_source.platform_version()
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
    parent: &Path,
    target: &Path,
    backup: &Path,
    expected_replacement: &BundleInfo,
) -> Result<(), InstallerError> {
    match filesystem.file_kind(target) {
        Err(filesystem_error) if is_not_found(filesystem_error) => {}
        Ok(MacosFileKind::Directory) => {
            remove_expected_replacement(runner, filesystem, parent, target, expected_replacement)?;
        }
        _ => {
            return Err(error(
                InstallerErrorCode::InstallationVerifyFailed,
                "replacement path could not be safely removed during restore",
            ))
        }
    }
    ensure_generated_path(filesystem, parent, backup, BACKUP_PREFIX, BACKUP_SUFFIX)?;
    filesystem.rename(backup, target).map_err(|_| {
        error(
            InstallerErrorCode::InstallationVerifyFailed,
            "Stable application backup could not be restored",
        )
    })
}

fn remove_expected_replacement(
    runner: &dyn CommandRunner,
    filesystem: &dyn MacosFilesystem,
    parent: &Path,
    target: &Path,
    expected: &BundleInfo,
) -> Result<(), InstallerError> {
    let current = bundle::read_bundle_info(runner, filesystem, target)?;
    if current.bundle_identifier() != expected.bundle_identifier()
        || current.platform_version() != expected.platform_version()
    {
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

    fn queue_read_and_validate(runner: &FakeRunner, version: &str) {
        queue_read_with_identity(runner, stable_bundle_id(), version);
    }

    fn queue_read_with_identity(runner: &FakeRunner, bundle_id: &str, version: &str) {
        runner.queue_success("plutil", plist(bundle_id, version));
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
    fn replacement_after_platform_verification_never_reaches_a_second_dmg_mount() {
        let trusted_bytes = b"trusted dmg";
        let release = release_for_artifact(trusted_bytes, "5848");
        let (_root, artifact) = downloaded_artifact_for(&release, trusted_bytes);
        let (filesystem, _) = fixture_filesystem_at(artifact.path());
        let runner = FakeRunner::new();
        let package =
            prepare_install_package(&runner, filesystem.as_ref(), &host(), &release, artifact)
                .unwrap();
        runner.assert_drained();
        let mount_count_before = runner
            .invocations()
            .iter()
            .filter(|invocation| invocation.program() == "hdiutil")
            .count();
        let mut replacement = fs::read(package.artifact_path()).unwrap();
        replacement[0] ^= 0x01;
        fs::write(package.artifact_path(), replacement).unwrap();

        let error = install_current_user(
            &runner,
            filesystem.as_ref(),
            &host(),
            &package,
            Arc::new(|_| {}),
        )
        .expect_err("a post-verification replacement must not be mounted");

        assert_eq!(error.code(), InstallerErrorCode::ChecksumMismatch);
        assert_eq!(
            runner
                .invocations()
                .iter()
                .filter(|invocation| invocation.program() == "hdiutil")
                .count(),
            mount_count_before
        );
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
    fn restore_refuses_to_delete_a_replacement_with_changed_identity() {
        let filesystem = FakeFilesystem::new();
        let parent = Path::new(SYSTEM_APPLICATIONS);
        let target = parent.join("ChatGPT.app");
        let backup = parent.join(format!("{BACKUP_PREFIX}test{BACKUP_SUFFIX}"));
        filesystem.add_dir(parent);
        add_bundle(&filesystem, &target);
        filesystem.add_dir(&backup);
        let runner = FakeRunner::new();
        runner.queue_success("plutil", plist(stable_bundle_id(), "5848"));
        let expected = bundle::read_bundle_info(&runner, &filesystem, &target).unwrap();
        runner.queue_success("plutil", plist("com.example.changed", "5848"));

        assert_eq!(
            restore_backup(&runner, &filesystem, parent, &target, &backup, &expected)
                .unwrap_err()
                .code(),
            InstallerErrorCode::InstallationVerifyFailed
        );
        assert!(filesystem.contains(&target));
        assert!(filesystem.contains(&backup));
        runner.assert_drained();
    }
}
