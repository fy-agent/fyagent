//! Stable macOS bundle discovery and lifecycle inspection.
//!
//! Bundle names are intentionally not an identity signal. A current Stable
//! package may be named `ChatGPT.app`, while an older valid installation may
//! still be named `Codex.app`; the fixed bundle ID is used only to find an
//! already installed Stable application, not to admit downloaded content.

use std::{
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
};

use serde::Deserialize;

use super::{
    command, error, is_not_found, stable_bundle_id, CommandRunner, MacosFileKind, MacosFilesystem,
    MacosHost,
};
use crate::codex_desktop::{
    error::{InstallerError, InstallerErrorCode},
    platform::{RuntimeInspection, TrustedRuntimeInstance},
    types::{
        CpuArchitecture, DesktopPlatform, InstalledApplication, InstalledApplicationSummary,
        LaunchTarget, LocalInstallStatus, PlatformVersion, ReleaseDescriptor,
    },
};

const PLUTIL_OUTPUT_FORMAT: &str = "json";

/// This script is a fixed program constant. It receives no paths or metadata
/// as interpolation input, and its JSON output is parsed before it influences
/// the install decision. It emits only the Stable Codex identity because the
/// macOS command runner truncates stdout at 16 KiB; a full NSWorkspace dump
/// exceeds that bound on a typical workstation and was previously parsed as
/// `MAC_APP_RUNNING`.
const RUNNING_APPLICATIONS_JXA: &str = r#"
ObjC.import('AppKit');
const workspace = $.NSWorkspace.sharedWorkspace;
const applications = workspace.runningApplications;
const result = [];
for (let index = 0; index < applications.count; index += 1) {
  const application = applications.objectAtIndex(index);
  const bundleIdentifier = application.bundleIdentifier ? ObjC.unwrap(application.bundleIdentifier) : null;
  if (bundleIdentifier !== "com.openai.codex") {
    continue;
  }
  const url = application.bundleURL;
  result.push({
    bundleIdentifier: bundleIdentifier,
    bundlePath: url ? ObjC.unwrap(url.path) : null,
    processIdentifier: application.processIdentifier,
    launchTimestampMs: application.launchDate
      ? Math.floor(Number(application.launchDate.timeIntervalSince1970) * 1000)
      : null,
    isFinishedLaunching: ObjC.unwrap(application.isFinishedLaunching),
  });
}
JSON.stringify(result);
"#;

/// Fixed JXA program used only after Rust has re-enumerated an exact trusted
/// running application. The mode, PID, and expected bundle path are internal
/// direct `osascript` arguments (never shell input or IPC fields). The script
/// repeats the Stable bundle-ID/path checks immediately before asking AppKit
/// to terminate the NSRunningApplication.
const TERMINATE_RUNNING_APPLICATION_JXA: &str = r#"
ObjC.import('AppKit');
const args = ObjC.unwrap($.NSProcessInfo.processInfo.arguments);
const [mode, pidText, expectedBundlePath, expectedLaunchTimestampText] = args.slice(-4);
const pid = Number(pidText);
const expectedLaunchTimestamp = Number(expectedLaunchTimestampText);
if (!Number.isSafeInteger(pid) || pid <= 0) {
  $.exit(2);
}
if (!Number.isSafeInteger(expectedLaunchTimestamp) || expectedLaunchTimestamp <= 0) {
  $.exit(7);
}
if (mode !== 'normal' && mode !== 'force') {
  $.exit(3);
}
const application = $.NSRunningApplication.runningApplicationWithProcessIdentifier(pid);
if (!application || !application.bundleIdentifier || !application.bundleURL) {
  $.exit(4);
}
const bundleIdentifier = ObjC.unwrap(application.bundleIdentifier);
const bundlePath = ObjC.unwrap(application.bundleURL.path);
const launchTimestamp = application.launchDate
  ? Math.floor(Number(application.launchDate.timeIntervalSince1970) * 1000)
  : 0;
if (bundleIdentifier !== 'com.openai.codex'
  || bundlePath !== expectedBundlePath
  || launchTimestamp !== expectedLaunchTimestamp) {
  $.exit(5);
}
const accepted = mode === 'force' ? application.forceTerminate() : application.terminate();
if (!accepted) {
  $.exit(6);
}
"#;

#[derive(Debug, Clone)]
pub(crate) struct BundleInfo {
    bundle_path: PathBuf,
    bundle_name: OsString,
    bundle_identifier: String,
    platform_version: PlatformVersion,
    display_version: Option<String>,
    display_name: Option<String>,
}

impl BundleInfo {
    pub(crate) fn bundle_path(&self) -> &Path {
        &self.bundle_path
    }

    pub(crate) fn bundle_name(&self) -> &OsStr {
        &self.bundle_name
    }

    pub(crate) fn bundle_identifier(&self) -> &str {
        &self.bundle_identifier
    }

    pub(crate) fn platform_version(&self) -> &PlatformVersion {
        &self.platform_version
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawInfoPlist {
    #[serde(rename = "CFBundleIdentifier")]
    bundle_identifier: Option<String>,
    #[serde(rename = "CFBundleVersion")]
    bundle_version: Option<String>,
    #[serde(rename = "CFBundleShortVersionString")]
    short_version: Option<String>,
    #[serde(rename = "CFBundleExecutable")]
    executable: Option<String>,
    #[serde(rename = "CFBundleDisplayName")]
    display_name: Option<String>,
    #[serde(rename = "CFBundleName")]
    bundle_name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawBundleIdentityPlist {
    #[serde(rename = "CFBundleIdentifier")]
    bundle_identifier: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RunningApplication {
    bundle_identifier: Option<String>,
    bundle_path: Option<String>,
    process_identifier: Option<i32>,
    launch_timestamp_ms: Option<u64>,
    is_finished_launching: Option<bool>,
}

#[derive(Debug, Clone)]
struct TrustedRunningApplication {
    instance: TrustedRuntimeInstance,
    is_finished_launching: bool,
}

#[derive(Debug, Clone)]
enum TrustedRunningApplications {
    NotRunning,
    Candidates(Vec<TrustedRunningApplication>),
    Ambiguous,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BundleCandidatePolicy {
    StrictPackage,
    SkipEscapedLocalSymlink,
}

/// Scan direct bundle children under the two standard roots. Any Stable
/// candidate is fully verified before becoming a managed installation; Classic
/// and Beta bundles are never promoted based on their filename or Team alone.
pub(crate) fn inspect_local(
    runner: &dyn CommandRunner,
    filesystem: &dyn MacosFilesystem,
    host: &MacosHost,
) -> Result<LocalInstallStatus, InstallerError> {
    let stable_bundles = scan_stable_bundles(runner, filesystem, host)?;
    match stable_bundles.as_slice() {
        [] => Ok(LocalInstallStatus::NotInstalled {
            platform: DesktopPlatform::Macos,
            architecture: CpuArchitecture::Aarch64,
        }),
        [bundle] => Ok(LocalInstallStatus::Installed {
            application: installed_application(bundle),
        }),
        bundles => {
            let error = error(
                InstallerErrorCode::MacMultipleInstallations,
                "multiple Stable macOS bundles were found in standard Applications directories",
            )
            .to_dto();
            Ok(LocalInstallStatus::Ambiguous {
                candidates: bundles
                    .iter()
                    .map(|bundle| InstalledApplicationSummary::from(&installed_application(bundle)))
                    .collect(),
                error,
            })
        }
    }
}

pub(crate) fn scan_stable_bundles(
    runner: &dyn CommandRunner,
    filesystem: &dyn MacosFilesystem,
    host: &MacosHost,
) -> Result<Vec<BundleInfo>, InstallerError> {
    let mut stable_bundles = Vec::new();
    for root in [host.applications_dir(), host.user_applications_dir()] {
        let root = match canonical_existing_directory(filesystem, root) {
            Ok(root) => root,
            Err(error) if error.code() == InstallerErrorCode::MacAppNotFound => continue,
            Err(error) => return Err(error),
        };

        for entry in filesystem.read_dir(&root).map_err(|_| {
            error(
                InstallerErrorCode::PackageParseFailed,
                "a standard Applications directory could not be enumerated",
            )
        })? {
            // Local discovery must not follow an escaped alias just to learn
            // its identity. Downloaded package discovery uses the strict
            // wrapper below and continues to reject the same path shape.
            let Some(bundle_path) = canonical_top_level_bundle_with_policy(
                filesystem,
                &root,
                &entry,
                BundleCandidatePolicy::SkipEscapedLocalSymlink,
            )?
            else {
                continue;
            };
            let Some(bundle_identifier) =
                probe_bundle_identifier(runner, filesystem, &bundle_path)?
            else {
                continue;
            };
            if bundle_identifier != stable_bundle_id() {
                continue;
            }
            let bundle = read_bundle_info(runner, filesystem, &bundle_path)?;
            if bundle.bundle_identifier() != stable_bundle_id() {
                continue;
            }
            validate_stable_bundle(runner, filesystem, host, &bundle, None)?;
            stable_bundles.push(bundle);
        }
    }
    Ok(stable_bundles)
}

/// Resolves one direct `.app` child and rejects a link that escapes the trusted
/// standard root. A symlink inside a root remains acceptable only when its
/// canonical target is still a direct child of that same root.
pub(crate) fn canonical_top_level_bundle(
    filesystem: &dyn MacosFilesystem,
    canonical_parent: &Path,
    candidate: &Path,
) -> Result<Option<PathBuf>, InstallerError> {
    canonical_top_level_bundle_with_policy(
        filesystem,
        canonical_parent,
        candidate,
        BundleCandidatePolicy::StrictPackage,
    )
}

fn canonical_top_level_bundle_with_policy(
    filesystem: &dyn MacosFilesystem,
    canonical_parent: &Path,
    candidate: &Path,
    policy: BundleCandidatePolicy,
) -> Result<Option<PathBuf>, InstallerError> {
    if !has_app_extension(candidate) {
        return Ok(None);
    }
    let candidate_kind = match filesystem.file_kind(candidate) {
        Ok(kind @ (MacosFileKind::Directory | MacosFileKind::Symlink)) => kind,
        Ok(_) => return Ok(None),
        Err(error) if is_not_found(error) => return Ok(None),
        Err(_) => {
            return Err(error(
                InstallerErrorCode::PackageParseFailed,
                "an application bundle candidate could not be inspected",
            ))
        }
    };

    let canonical = filesystem.canonicalize(candidate).map_err(|_| {
        error(
            InstallerErrorCode::PackageParseFailed,
            "an application bundle candidate could not be canonicalized",
        )
    })?;
    if candidate_kind == MacosFileKind::Symlink
        && policy == BundleCandidatePolicy::SkipEscapedLocalSymlink
        && !canonical.starts_with(canonical_parent)
    {
        return Ok(None);
    }
    if canonical.parent() != Some(canonical_parent)
        || !canonical.starts_with(canonical_parent)
        || filesystem.file_kind(&canonical) != Ok(MacosFileKind::Directory)
    {
        return Err(error(
            InstallerErrorCode::PackageParseFailed,
            "an application bundle candidate escaped its trusted directory",
        ));
    }
    Ok(Some(canonical))
}

pub(crate) fn read_bundle_info(
    runner: &dyn CommandRunner,
    filesystem: &dyn MacosFilesystem,
    canonical_bundle_path: &Path,
) -> Result<BundleInfo, InstallerError> {
    if filesystem.file_kind(canonical_bundle_path) != Ok(MacosFileKind::Directory) {
        return Err(error(
            InstallerErrorCode::PackageParseFailed,
            "application bundle is not a directory",
        ));
    }
    let bundle_name = canonical_bundle_path
        .file_name()
        .filter(|name| has_app_extension(Path::new(name)))
        .map(OsStr::to_os_string)
        .ok_or_else(|| {
            error(
                InstallerErrorCode::PackageParseFailed,
                "application bundle name is invalid",
            )
        })?;
    let raw = read_raw_info_plist(runner, filesystem, canonical_bundle_path)?;

    let bundle_identifier = required_plist_string(raw.bundle_identifier, "bundle identifier")?;
    let bundle_version = required_plist_string(raw.bundle_version, "bundle version")?;
    let short_version = optional_plist_string(raw.short_version, "short version")?;
    let executable = required_plist_string(raw.executable, "bundle executable")?;
    validate_executable_name(&executable)?;
    let display_name = optional_plist_string(raw.display_name.or(raw.bundle_name), "display name")?;
    let platform_version = PlatformVersion::parse_mac_bundle(bundle_version).map_err(|_| {
        error(
            InstallerErrorCode::PackageParseFailed,
            "application bundle version is invalid",
        )
    })?;
    let executable_path = canonical_bundle_path
        .join("Contents")
        .join("MacOS")
        .join(executable);
    if filesystem.file_kind(&executable_path) != Ok(MacosFileKind::File) {
        return Err(error(
            InstallerErrorCode::PackageParseFailed,
            "application executable is missing or is not a regular file",
        ));
    }
    let canonical_executable = filesystem.canonicalize(&executable_path).map_err(|_| {
        error(
            InstallerErrorCode::PackageParseFailed,
            "application executable could not be canonicalized",
        )
    })?;
    if !canonical_executable.starts_with(canonical_bundle_path) {
        return Err(error(
            InstallerErrorCode::PackageParseFailed,
            "application executable escaped its bundle",
        ));
    }

    Ok(BundleInfo {
        bundle_path: canonical_bundle_path.to_path_buf(),
        bundle_name,
        bundle_identifier,
        platform_version,
        display_version: short_version,
        display_name,
    })
}

/// Tries to identify a bundle before running the Stable-only verifier. This is
/// deliberately an exclusion-only probe: a positive result is re-read by
/// `read_bundle_info` below, so it never authorizes a bundle by itself.
fn probe_bundle_identifier(
    runner: &dyn CommandRunner,
    filesystem: &dyn MacosFilesystem,
    canonical_bundle_path: &Path,
) -> Result<Option<String>, InstallerError> {
    let info_plist = canonical_bundle_path.join("Contents").join("Info.plist");
    match filesystem.file_kind(&info_plist) {
        Ok(MacosFileKind::File) => {}
        Ok(_) => return Ok(None),
        Err(filesystem_error) if is_not_found(filesystem_error) => return Ok(None),
        Err(_) => {
            return Err(error(
                InstallerErrorCode::PackageParseFailed,
                "application Info.plist could not be inspected",
            ))
        }
    }

    let output = runner
        .run(&command(
            "plutil",
            vec![
                OsString::from("-convert"),
                OsString::from(PLUTIL_OUTPUT_FORMAT),
                OsString::from("-o"),
                OsString::from("-"),
                OsString::from("--"),
                info_plist.into_os_string(),
            ],
        ))
        .map_err(|_| {
            error(
                InstallerErrorCode::PackageParseFailed,
                "application Info.plist could not be parsed",
            )
        })?;
    if !output.is_success() {
        return Ok(None);
    }
    let raw = match serde_json::from_slice::<RawBundleIdentityPlist>(output.stdout()) {
        Ok(raw) => raw,
        Err(_) => return Ok(None),
    };
    Ok(required_plist_string(raw.bundle_identifier, "bundle identifier").ok())
}

fn read_raw_info_plist(
    runner: &dyn CommandRunner,
    filesystem: &dyn MacosFilesystem,
    canonical_bundle_path: &Path,
) -> Result<RawInfoPlist, InstallerError> {
    let info_plist = canonical_bundle_path.join("Contents").join("Info.plist");
    if filesystem.file_kind(&info_plist) != Ok(MacosFileKind::File) {
        return Err(error(
            InstallerErrorCode::PackageParseFailed,
            "application Info.plist is missing or unreadable",
        ));
    }

    let output = runner
        .run(&command(
            "plutil",
            vec![
                OsString::from("-convert"),
                OsString::from(PLUTIL_OUTPUT_FORMAT),
                OsString::from("-o"),
                OsString::from("-"),
                OsString::from("--"),
                info_plist.into_os_string(),
            ],
        ))
        .map_err(|_| {
            error(
                InstallerErrorCode::PackageParseFailed,
                "application Info.plist could not be parsed",
            )
        })?;
    if !output.is_success() {
        return Err(error(
            InstallerErrorCode::PackageParseFailed,
            "application Info.plist could not be parsed",
        ));
    }
    serde_json::from_slice::<RawInfoPlist>(output.stdout()).map_err(|_| {
        error(
            InstallerErrorCode::PackageParseFailed,
            "application Info.plist did not produce valid JSON metadata",
        )
    })
}

/// Confirms that an already discovered local Stable bundle is still the same
/// operational identity. Publisher, signature, Team ID, architecture and
/// minimum-OS policy remain the responsibility of macOS and are not FyAgent
/// installer admission checks.
pub(crate) fn validate_stable_bundle(
    _runner: &dyn CommandRunner,
    _filesystem: &dyn MacosFilesystem,
    _host: &MacosHost,
    bundle: &BundleInfo,
    _expected_release: Option<&ReleaseDescriptor>,
) -> Result<(), InstallerError> {
    if bundle.bundle_identifier() != stable_bundle_id() {
        return Err(error(
            InstallerErrorCode::MacBundleIdMismatch,
            "application bundle identifier is not the Stable Codex identifier",
        ));
    }
    Ok(())
}

pub(crate) fn ensure_not_running(
    runner: &dyn CommandRunner,
    filesystem: &dyn MacosFilesystem,
    bundle_path: &Path,
) -> Result<(), InstallerError> {
    let bundle_path = filesystem.canonicalize(bundle_path).map_err(|_| {
        error(
            InstallerErrorCode::MacAppRunning,
            "running Stable application state could not be determined",
        )
    })?;
    let applications = running_applications(runner)?;
    for application in applications {
        if application.bundle_identifier.as_deref() != Some(stable_bundle_id()) {
            continue;
        }
        let path = application.bundle_path.ok_or_else(|| {
            error(
                InstallerErrorCode::MacAppRunning,
                "running Stable application path could not be determined",
            )
        })?;
        let running_path = filesystem.canonicalize(Path::new(&path)).map_err(|_| {
            error(
                InstallerErrorCode::MacAppRunning,
                "running Stable application path could not be determined",
            )
        })?;
        if running_path == bundle_path {
            return Err(error(
                InstallerErrorCode::MacAppRunning,
                "the Stable application is running",
            ));
        }
    }
    Ok(())
}

/// Inspect only running applications that match both the Stable bundle ID and
/// the canonical path of an already verified installation. A matching bundle
/// ID at a different path is intentionally ambiguous; it is never selected by
/// display name or process name.
pub(crate) fn inspect_runtime(
    runner: &dyn CommandRunner,
    filesystem: &dyn MacosFilesystem,
    installed: &InstalledApplication,
) -> Result<RuntimeInspection, InstallerError> {
    match inspect_trusted_running_applications(runner, filesystem, installed)? {
        TrustedRunningApplications::NotRunning => Ok(RuntimeInspection::NotRunning),
        TrustedRunningApplications::Ambiguous => Ok(RuntimeInspection::Ambiguous),
        TrustedRunningApplications::Candidates(applications) => match applications.as_slice() {
            [application] if application.is_finished_launching => {
                Ok(RuntimeInspection::Running(vec![application
                    .instance
                    .clone()]))
            }
            [_] => {
                // NSWorkspace can enumerate the new process before AppKit has
                // finished launching it. It is not restart success evidence;
                // the service keeps polling this non-ready state for its full
                // bounded verification window.
                Ok(RuntimeInspection::NotRunning)
            }
            _ => Ok(RuntimeInspection::Ambiguous),
        },
    }
}

pub(crate) fn force_shutdown(
    runner: &dyn CommandRunner,
    filesystem: &dyn MacosFilesystem,
    installed: &InstalledApplication,
    instances: &[TrustedRuntimeInstance],
) -> Result<(), InstallerError> {
    terminate_runtime(runner, filesystem, installed, instances, "force")
}

/// Re-enumerate only to compare against the opaque instance evidence captured
/// by the restart operation. A new matching app, PID reuse, or another Stable
/// app is an identity failure rather than evidence that the original exited.
pub(crate) fn is_runtime_instance_running(
    runner: &dyn CommandRunner,
    filesystem: &dyn MacosFilesystem,
    installed: &InstalledApplication,
    instances: &[TrustedRuntimeInstance],
) -> Result<bool, InstallerError> {
    // Liveness is stricter than readiness: an app that was already bound to a
    // restart operation remains alive even if a subsequent AppKit snapshot is
    // not ready. Treating it as exited could launch a second instance before
    // the original identity has actually gone away.
    match inspect_trusted_running_applications(runner, filesystem, installed)? {
        TrustedRunningApplications::NotRunning => Ok(false),
        TrustedRunningApplications::Candidates(current)
            if current
                .iter()
                .map(|application| &application.instance)
                .eq(instances.iter()) =>
        {
            Ok(true)
        }
        TrustedRunningApplications::Candidates(_) | TrustedRunningApplications::Ambiguous => {
            Err(error(
                InstallerErrorCode::MacAppRunning,
                "the trusted runtime changed before restart verification",
            ))
        }
    }
}

fn inspect_trusted_running_applications(
    runner: &dyn CommandRunner,
    filesystem: &dyn MacosFilesystem,
    installed: &InstalledApplication,
) -> Result<TrustedRunningApplications, InstallerError> {
    let expected_bundle_path = trusted_installed_bundle_path(filesystem, installed)?;
    let applications = running_applications(runner)?;
    let mut candidates = Vec::new();

    for application in applications {
        if application.bundle_identifier.as_deref() != Some(stable_bundle_id()) {
            continue;
        }
        let raw_bundle_path = application.bundle_path.ok_or_else(|| {
            error(
                InstallerErrorCode::MacAppRunning,
                "running Stable application path could not be determined",
            )
        })?;
        let process_id = application
            .process_identifier
            .filter(|pid| *pid > 0)
            .ok_or_else(|| {
                error(
                    InstallerErrorCode::MacAppRunning,
                    "running Stable application process identity could not be determined",
                )
            })?;
        let launch_timestamp_ms = application
            .launch_timestamp_ms
            .filter(|timestamp| *timestamp > 0)
            .ok_or_else(|| {
                error(
                    InstallerErrorCode::MacAppRunning,
                    "running Stable application launch identity could not be determined",
                )
            })?;
        let reported_bundle_path = PathBuf::from(&raw_bundle_path);
        let bundle_path = filesystem
            .canonicalize(Path::new(&raw_bundle_path))
            .map_err(|_| {
                error(
                    InstallerErrorCode::MacAppRunning,
                    "running Stable application path could not be determined",
                )
            })?;
        if bundle_path != expected_bundle_path {
            return Ok(TrustedRunningApplications::Ambiguous);
        }
        candidates.push(TrustedRunningApplication {
            instance: TrustedRuntimeInstance::Macos {
                process_id,
                bundle_path,
                reported_bundle_path,
                launch_timestamp_ms,
            },
            // A missing/invalid readiness value is never upgraded to a ready
            // runtime. The JXA program above always emits this field, so this
            // branch remains fail-closed if its output changes unexpectedly.
            is_finished_launching: application.is_finished_launching == Some(true),
        });
    }

    if candidates.is_empty() {
        Ok(TrustedRunningApplications::NotRunning)
    } else {
        Ok(TrustedRunningApplications::Candidates(candidates))
    }
}

fn trusted_installed_bundle_path(
    filesystem: &dyn MacosFilesystem,
    installed: &InstalledApplication,
) -> Result<PathBuf, InstallerError> {
    if installed.stable_identity != stable_bundle_id() {
        return Err(error(
            InstallerErrorCode::MacBundleIdMismatch,
            "runtime inspection was requested for a non-Stable bundle identity",
        ));
    }
    let LaunchTarget::MacBundlePath(path) = &installed.launch_target else {
        return Err(error(
            InstallerErrorCode::MacBundleIdMismatch,
            "runtime inspection was requested for a non-macOS launch target",
        ));
    };
    filesystem.canonicalize(path).map_err(|_| {
        error(
            InstallerErrorCode::MacAppRunning,
            "verified Stable bundle path is no longer available",
        )
    })
}

fn running_applications(
    runner: &dyn CommandRunner,
) -> Result<Vec<RunningApplication>, InstallerError> {
    let output = runner
        .run(&command(
            "osascript",
            vec![
                OsString::from("-l"),
                OsString::from("JavaScript"),
                OsString::from("-e"),
                OsString::from(RUNNING_APPLICATIONS_JXA),
            ],
        ))
        .map_err(|_| {
            error(
                InstallerErrorCode::MacAppRunning,
                "running Stable application state could not be determined",
            )
        })?;
    if !output.is_success() {
        return Err(error(
            InstallerErrorCode::MacAppRunning,
            "running Stable application state could not be determined",
        ));
    }
    serde_json::from_slice::<Vec<RunningApplication>>(output.stdout()).map_err(|_| {
        error(
            InstallerErrorCode::MacAppRunning,
            "running Stable application state could not be determined",
        )
    })
}

fn terminate_runtime(
    runner: &dyn CommandRunner,
    filesystem: &dyn MacosFilesystem,
    installed: &InstalledApplication,
    instances: &[TrustedRuntimeInstance],
    mode: &'static str,
) -> Result<(), InstallerError> {
    let RuntimeInspection::Running(current) = inspect_runtime(runner, filesystem, installed)?
    else {
        return Err(error(
            InstallerErrorCode::MacAppRunning,
            "the trusted runtime changed before termination",
        ));
    };
    if current != instances {
        return Err(error(
            InstallerErrorCode::MacAppRunning,
            "the trusted runtime changed before termination",
        ));
    }
    let [TrustedRuntimeInstance::Macos {
        process_id,
        reported_bundle_path,
        launch_timestamp_ms,
        ..
    }] = instances
    else {
        return Err(error(
            InstallerErrorCode::MacAppRunning,
            "the requested runtime is not one verified macOS application",
        ));
    };
    let output = runner
        .run(&command(
            "osascript",
            vec![
                OsString::from("-l"),
                OsString::from("JavaScript"),
                OsString::from("-e"),
                OsString::from(TERMINATE_RUNNING_APPLICATION_JXA),
                OsString::from("--"),
                OsString::from(mode),
                OsString::from(process_id.to_string()),
                reported_bundle_path.clone().into_os_string(),
                OsString::from(launch_timestamp_ms.to_string()),
            ],
        ))
        .map_err(|_| {
            error(
                InstallerErrorCode::MacAppRunning,
                "the trusted Stable application could not be asked to terminate",
            )
        })?;
    if !output.is_success() {
        return Err(error(
            InstallerErrorCode::MacAppRunning,
            "the trusted Stable application rejected the termination request",
        ));
    }
    Ok(())
}

pub(crate) fn launch_verified(
    runner: &dyn CommandRunner,
    filesystem: &dyn MacosFilesystem,
    host: &MacosHost,
    installed: &InstalledApplication,
) -> Result<(), InstallerError> {
    if installed.stable_identity != stable_bundle_id() {
        return Err(error(
            InstallerErrorCode::LaunchFailed,
            "launch was requested for a non-Stable bundle identity",
        ));
    }
    let requested_path = match &installed.launch_target {
        LaunchTarget::MacBundlePath(path) => filesystem.canonicalize(path).map_err(|_| {
            error(
                InstallerErrorCode::LaunchFailed,
                "the verified Stable application path is no longer available",
            )
        })?,
        LaunchTarget::WindowsAumid(_) => {
            return Err(error(
                InstallerErrorCode::LaunchFailed,
                "launch target does not belong to macOS",
            ))
        }
    };
    let local = inspect_local(runner, filesystem, host)?;
    let current = match local {
        LocalInstallStatus::Installed { application } => application,
        LocalInstallStatus::Ambiguous { .. } => {
            return Err(error(
                InstallerErrorCode::MacMultipleInstallations,
                "multiple Stable bundles prevent a safe automatic launch",
            ))
        }
        _ => {
            return Err(error(
                InstallerErrorCode::LaunchFailed,
                "the verified Stable application is no longer installed",
            ))
        }
    };
    let current_path = match current.launch_target {
        LaunchTarget::MacBundlePath(path) => path,
        LaunchTarget::WindowsAumid(_) => {
            unreachable!("macOS local scan only produces bundle paths")
        }
    };
    if current_path != requested_path {
        return Err(error(
            InstallerErrorCode::LaunchFailed,
            "the Stable application changed after launch was requested",
        ));
    }
    let output = runner
        .run(&command("open", vec![requested_path.into_os_string()]))
        .map_err(|_| {
            error(
                InstallerErrorCode::LaunchFailed,
                "application launch could not be started",
            )
        })?;
    if !output.is_success() {
        return Err(error(
            InstallerErrorCode::LaunchFailed,
            "application launch was rejected by macOS",
        ));
    }
    Ok(())
}

pub(crate) fn installed_application(bundle: &BundleInfo) -> InstalledApplication {
    InstalledApplication {
        stable_identity: bundle.bundle_identifier.clone(),
        display_name: bundle.display_name.clone(),
        display_version: bundle.display_version.clone(),
        platform_version: bundle.platform_version.clone(),
        architecture: CpuArchitecture::Aarch64,
        location: Some(bundle.bundle_path.to_string_lossy().into_owned()),
        launch_target: LaunchTarget::MacBundlePath(bundle.bundle_path.clone()),
    }
}

fn canonical_existing_directory(
    filesystem: &dyn MacosFilesystem,
    path: &Path,
) -> Result<PathBuf, InstallerError> {
    match filesystem.file_kind(path) {
        Ok(MacosFileKind::Directory) => filesystem.canonicalize(path).map_err(|_| {
            error(
                InstallerErrorCode::PackageParseFailed,
                "a standard Applications directory could not be canonicalized",
            )
        }),
        Err(filesystem_error) if is_not_found(filesystem_error) => Err(error(
            InstallerErrorCode::MacAppNotFound,
            "a standard Applications directory is absent",
        )),
        _ => Err(error(
            InstallerErrorCode::PackageParseFailed,
            "a standard Applications path is not a directory",
        )),
    }
}

fn has_app_extension(path: &Path) -> bool {
    path.extension()
        .and_then(OsStr::to_str)
        .is_some_and(|extension| extension.eq_ignore_ascii_case("app"))
}

fn required_plist_string(
    value: Option<String>,
    field: &'static str,
) -> Result<String, InstallerError> {
    let value = value.ok_or_else(|| {
        error(
            InstallerErrorCode::PackageParseFailed,
            "application Info.plist is missing a required field",
        )
        .with_context("field", field)
    })?;
    if value.is_empty() || value.trim() != value || value.chars().any(char::is_control) {
        return Err(error(
            InstallerErrorCode::PackageParseFailed,
            "application Info.plist has an invalid field",
        )
        .with_context("field", field));
    }
    Ok(value)
}

fn optional_plist_string(
    value: Option<String>,
    field: &'static str,
) -> Result<Option<String>, InstallerError> {
    value
        .map(|value| required_plist_string(Some(value), field))
        .transpose()
}

fn validate_executable_name(value: &str) -> Result<(), InstallerError> {
    if value.is_empty()
        || value.trim() != value
        || value == "."
        || value == ".."
        || value.contains(['/', '\\', '\0'])
        || value.contains("..")
        || value.chars().any(char::is_control)
    {
        return Err(error(
            InstallerErrorCode::PackageParseFailed,
            "application executable name is unsafe",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{path::Path, sync::Arc};

    use super::*;
    use crate::codex_desktop::{
        error::InstallerErrorCode,
        platform::macos::{
            test_support::{FakeFilesystem, FakeRunner},
            MacosHost,
        },
    };

    const SYSTEM_APPLICATIONS: &str = "/Applications";
    const USER_APPLICATIONS: &str = "/Users/test/Applications";

    fn host() -> MacosHost {
        MacosHost::new(
            CpuArchitecture::Aarch64,
            "14.4",
            SYSTEM_APPLICATIONS.into(),
            USER_APPLICATIONS.into(),
        )
        .unwrap()
    }

    fn plist(bundle_identifier: &str, bundle_version: &str, minimum_os: Option<&str>) -> Vec<u8> {
        let minimum_os = minimum_os
            .map(|value| format!(",\"LSMinimumSystemVersion\":\"{value}\""))
            .unwrap_or_default();
        format!(
            "{{\"CFBundleIdentifier\":\"{bundle_identifier}\",\"CFBundleVersion\":\"{bundle_version}\",\"CFBundleShortVersionString\":\"1.0\",\"CFBundleExecutable\":\"Codex\"{minimum_os}}}"
        )
        .into_bytes()
    }

    fn add_bundle(filesystem: &FakeFilesystem, bundle_path: &Path) {
        filesystem.add_dir(bundle_path);
        filesystem.add_file(bundle_path.join("Contents/Info.plist"));
        filesystem.add_file(bundle_path.join("Contents/MacOS/Codex"));
    }

    fn queue_bundle_read(runner: &FakeRunner, plist: Vec<u8>) {
        runner.queue_success("plutil", plist);
    }

    fn queue_stable_bundle_scan(runner: &FakeRunner, bundle_version: &str) {
        let bundle_plist = plist(stable_bundle_id(), bundle_version, None);
        queue_bundle_read(runner, bundle_plist.clone());
        queue_bundle_read(runner, bundle_plist);
    }

    fn read_stable_bundle(
        runner: &FakeRunner,
        filesystem: &FakeFilesystem,
        path: &Path,
    ) -> BundleInfo {
        queue_bundle_read(runner, plist(stable_bundle_id(), "5848", Some("14.0")));
        read_bundle_info(runner, filesystem, path).unwrap()
    }

    #[test]
    fn stable_bundle_uses_identity_not_its_display_directory_name() {
        let filesystem = FakeFilesystem::new();
        let bundle_path = Path::new(SYSTEM_APPLICATIONS).join("ChatGPT.app");
        add_bundle(&filesystem, &bundle_path);
        let runner = FakeRunner::new();

        let bundle = read_stable_bundle(&runner, &filesystem, &bundle_path);
        validate_stable_bundle(&runner, &filesystem, &host(), &bundle, None).unwrap();
        runner.assert_drained();

        let invocations = runner.invocations();
        assert_eq!(invocations[0].program(), "plutil");
        assert_eq!(invocations[0].arguments()[0], "-convert");
        assert_eq!(invocations[0].arguments()[1], "json");
        assert_eq!(invocations.len(), 1);
        assert!(invocations.iter().all(|invocation| {
            invocation.program() != "xattr"
                && !invocation
                    .arguments()
                    .iter()
                    .any(|argument| argument == "--force")
        }));
    }

    #[test]
    fn classic_and_beta_bundles_are_not_promoted_by_directory_name_or_team() {
        let filesystem = FakeFilesystem::new();
        filesystem.add_dir(SYSTEM_APPLICATIONS);
        filesystem.add_dir(USER_APPLICATIONS);
        let classic = Path::new(SYSTEM_APPLICATIONS).join("ChatGPT.app");
        let beta = Path::new(USER_APPLICATIONS).join("Codex Beta.app");
        add_bundle(&filesystem, &classic);
        add_bundle(&filesystem, &beta);
        let runner = FakeRunner::new();
        queue_bundle_read(&runner, plist("com.openai.chat", "5848", None));
        queue_bundle_read(&runner, plist("com.openai.codex.beta", "5848", None));

        assert!(matches!(
            inspect_local(&runner, &filesystem, &host()).unwrap(),
            LocalInstallStatus::NotInstalled {
                platform: DesktopPlatform::Macos,
                architecture: CpuArchitecture::Aarch64,
            }
        ));
        assert!(runner
            .invocations()
            .iter()
            .all(|invocation| invocation.program() == "plutil"));
        runner.assert_drained();
    }

    #[test]
    fn malformed_unrelated_bundle_does_not_block_a_valid_stable_bundle() {
        let filesystem = FakeFilesystem::new();
        let malformed = Path::new(SYSTEM_APPLICATIONS).join("Archive.app");
        let stable = Path::new(SYSTEM_APPLICATIONS).join("ChatGPT.app");
        add_bundle(&filesystem, &malformed);
        add_bundle(&filesystem, &stable);
        let runner = FakeRunner::new();
        runner.queue_success(
            "plutil",
            br#"{"CFBundleIdentifier":"com.example.unrelated"}"#.to_vec(),
        );
        queue_stable_bundle_scan(&runner, "5848");

        let LocalInstallStatus::Installed { application } =
            inspect_local(&runner, &filesystem, &host()).unwrap()
        else {
            panic!("the valid Stable bundle must remain discoverable");
        };
        assert_eq!(application.location.as_deref(), stable.to_str());
        assert_eq!(
            runner
                .invocations()
                .iter()
                .filter(|invocation| invocation.program() == "lipo")
                .count(),
            0
        );
        runner.assert_drained();
    }

    #[test]
    fn escaped_unrelated_bundle_symlink_does_not_block_local_discovery() {
        let filesystem = FakeFilesystem::new();
        let escaped = Path::new(SYSTEM_APPLICATIONS).join("Archive.app");
        let escaped_target = Path::new("/Volumes/Archive/Archive.app");
        let stable = Path::new(SYSTEM_APPLICATIONS).join("ChatGPT.app");
        filesystem.add_dir(escaped_target);
        filesystem.add_symlink(&escaped, escaped_target);
        add_bundle(&filesystem, &stable);
        let runner = FakeRunner::new();
        queue_stable_bundle_scan(&runner, "5848");

        let LocalInstallStatus::Installed { application } =
            inspect_local(&runner, &filesystem, &host()).unwrap()
        else {
            panic!("the valid Stable bundle must remain discoverable");
        };
        assert_eq!(application.location.as_deref(), stable.to_str());
        assert_eq!(
            runner
                .invocations()
                .iter()
                .filter(|invocation| invocation.program() == "plutil")
                .count(),
            2
        );
        runner.assert_drained();
    }

    #[test]
    fn strict_package_candidate_check_rejects_an_escaped_bundle_symlink() {
        let filesystem = FakeFilesystem::new();
        let mount_point = Path::new("/Volumes/Codex Installer");
        let escaped = mount_point.join("Codex.app");
        let escaped_target = Path::new(SYSTEM_APPLICATIONS).join("ChatGPT.app");
        filesystem.add_dir(mount_point);
        filesystem.add_dir(&escaped_target);
        filesystem.add_symlink(&escaped, &escaped_target);

        assert_eq!(
            canonical_top_level_bundle(&filesystem, mount_point, &escaped)
                .unwrap_err()
                .code(),
            InstallerErrorCode::PackageParseFailed
        );
    }

    #[test]
    fn malformed_unrelated_bundle_is_not_installed() {
        let filesystem = FakeFilesystem::new();
        let malformed = Path::new(SYSTEM_APPLICATIONS).join("Archive.app");
        add_bundle(&filesystem, &malformed);
        let runner = FakeRunner::new();
        runner.queue_success("plutil", b"not-json".to_vec());

        assert!(matches!(
            inspect_local(&runner, &filesystem, &host()).unwrap(),
            LocalInstallStatus::NotInstalled {
                platform: DesktopPlatform::Macos,
                architecture: CpuArchitecture::Aarch64,
            }
        ));
        runner.assert_drained();
    }

    #[test]
    fn identified_stable_bundle_with_malformed_strict_metadata_fails_closed() {
        let filesystem = FakeFilesystem::new();
        let stable = Path::new(SYSTEM_APPLICATIONS).join("ChatGPT.app");
        add_bundle(&filesystem, &stable);
        let runner = FakeRunner::new();
        let stable_identity_only = format!(
            "{{\"CFBundleIdentifier\":\"{}\",\"CFBundleVersion\":5848}}",
            stable_bundle_id()
        )
        .into_bytes();
        runner.queue_success("plutil", stable_identity_only.clone());
        runner.queue_success("plutil", stable_identity_only);

        assert_eq!(
            inspect_local(&runner, &filesystem, &host())
                .unwrap_err()
                .code(),
            InstallerErrorCode::PackageParseFailed
        );
        runner.assert_drained();
    }

    #[test]
    fn multiple_stable_bundles_are_ambiguous_and_never_auto_selected() {
        let filesystem = FakeFilesystem::new();
        let system_bundle = Path::new(SYSTEM_APPLICATIONS).join("Codex.app");
        let user_bundle = Path::new(USER_APPLICATIONS).join("ChatGPT.app");
        add_bundle(&filesystem, &system_bundle);
        add_bundle(&filesystem, &user_bundle);
        let runner = FakeRunner::new();

        queue_stable_bundle_scan(&runner, "5848");
        queue_stable_bundle_scan(&runner, "5849");

        let status = inspect_local(&runner, &filesystem, &host()).unwrap();
        let LocalInstallStatus::Ambiguous { candidates, error } = status else {
            panic!("multiple Stable bundles must be ambiguous");
        };
        assert_eq!(candidates.len(), 2);
        assert_eq!(error.code, InstallerErrorCode::MacMultipleInstallations);
        runner.assert_drained();
    }

    #[test]
    fn stable_discovery_does_not_admit_on_team_architecture_gatekeeper_or_minimum_os() {
        let filesystem = FakeFilesystem::new();
        let bundle_path = Path::new(SYSTEM_APPLICATIONS).join("ChatGPT.app");
        add_bundle(&filesystem, &bundle_path);

        let runner = FakeRunner::new();
        queue_bundle_read(&runner, plist(stable_bundle_id(), "5848", Some("15.0")));
        let bundle = read_bundle_info(&runner, &filesystem, &bundle_path).unwrap();
        validate_stable_bundle(&runner, &filesystem, &host(), &bundle, None).unwrap();
        assert!(runner
            .invocations()
            .iter()
            .all(|invocation| invocation.program() == "plutil"));
        runner.assert_drained();
    }

    #[test]
    fn malformed_plist_is_a_package_parse_failure() {
        let filesystem = FakeFilesystem::new();
        let bundle_path = Path::new(SYSTEM_APPLICATIONS).join("ChatGPT.app");
        add_bundle(&filesystem, &bundle_path);
        let runner = FakeRunner::new();
        runner.queue_success("plutil", b"not-json".to_vec());

        assert_eq!(
            read_bundle_info(&runner, &filesystem, &bundle_path)
                .unwrap_err()
                .code(),
            InstallerErrorCode::PackageParseFailed
        );
        runner.assert_drained();
    }

    #[test]
    fn running_applications_script_emits_only_the_stable_bundle_id() {
        assert!(RUNNING_APPLICATIONS_JXA.contains(stable_bundle_id()));
        assert!(
            RUNNING_APPLICATIONS_JXA.contains(r#"if (bundleIdentifier !== "com.openai.codex")"#)
        );
    }

    #[test]
    fn running_state_matches_exact_bundle_id_and_canonical_path() {
        let filesystem = FakeFilesystem::new();
        let bundle_path = Path::new(SYSTEM_APPLICATIONS).join("ChatGPT.app");
        filesystem.add_dir(&bundle_path);
        let runner = FakeRunner::new();
        runner.queue_success(
            "osascript",
            format!(
                "[{{\"bundleIdentifier\":\"{}\",\"bundlePath\":\"{}\"}}]",
                stable_bundle_id(),
                bundle_path.display()
            )
            .into_bytes(),
        );

        assert_eq!(
            ensure_not_running(&runner, &filesystem, &bundle_path)
                .unwrap_err()
                .code(),
            InstallerErrorCode::MacAppRunning
        );
        let invocation = runner.invocations().pop().unwrap();
        assert_eq!(invocation.program(), "osascript");
        assert_eq!(invocation.arguments()[0], "-l");
        assert_eq!(invocation.arguments()[1], "JavaScript");
        assert_eq!(invocation.arguments()[2], "-e");
        assert_ne!(invocation.arguments()[3], bundle_path.as_os_str());
        runner.assert_drained();
    }

    #[test]
    fn runtime_requires_a_matching_bundle_to_finish_launching_before_it_is_ready() {
        let filesystem = FakeFilesystem::new();
        let bundle_path = Path::new(SYSTEM_APPLICATIONS).join("ChatGPT.app");
        filesystem.add_dir(&bundle_path);
        let installed = InstalledApplication {
            stable_identity: stable_bundle_id().to_owned(),
            display_name: Some("Codex".to_owned()),
            display_version: Some("1.0".to_owned()),
            platform_version: PlatformVersion::parse_mac_bundle("5848").unwrap(),
            architecture: CpuArchitecture::Aarch64,
            location: None,
            launch_target: LaunchTarget::MacBundlePath(bundle_path.clone()),
        };
        let runner = FakeRunner::new();

        for is_finished_launching in [false, true] {
            runner.queue_success(
                "osascript",
                serde_json::json!([{
                    "bundleIdentifier": stable_bundle_id(),
                    "bundlePath": bundle_path.to_string_lossy(),
                    "processIdentifier": 4242,
                    "launchTimestampMs": 1000,
                    "isFinishedLaunching": is_finished_launching,
                }])
                .to_string()
                .into_bytes(),
            );
        }

        assert!(matches!(
            inspect_runtime(&runner, &filesystem, &installed).unwrap(),
            RuntimeInspection::NotRunning,
        ));
        let ready = inspect_runtime(&runner, &filesystem, &installed).unwrap();
        assert!(matches!(
            ready,
            RuntimeInspection::Running(ref instances) if instances.len() == 1,
        ));

        let invocations = runner.invocations();
        assert!(invocations.iter().all(|invocation| {
            invocation.program() == "osascript"
                && invocation.arguments()[3]
                    .to_string_lossy()
                    .contains("isFinishedLaunching")
        }));
        runner.assert_drained();
    }

    #[test]
    fn launch_rechecks_the_verified_path_and_never_uses_open_by_name() {
        let filesystem = Arc::new(FakeFilesystem::new());
        let bundle_path = Path::new(SYSTEM_APPLICATIONS).join("ChatGPT.app");
        add_bundle(filesystem.as_ref(), &bundle_path);
        filesystem.add_dir(USER_APPLICATIONS);
        let runner = FakeRunner::new();
        queue_stable_bundle_scan(&runner, "5848");
        runner.queue_success("open", Vec::<u8>::new());
        let installed = InstalledApplication {
            stable_identity: stable_bundle_id().to_owned(),
            display_name: Some("Codex".to_owned()),
            display_version: Some("1.0".to_owned()),
            platform_version: PlatformVersion::parse_mac_bundle("5848").unwrap(),
            architecture: CpuArchitecture::Aarch64,
            location: None,
            launch_target: LaunchTarget::MacBundlePath(bundle_path.clone()),
        };

        launch_verified(&runner, filesystem.as_ref(), &host(), &installed).unwrap();
        let invocation = runner.invocations().pop().unwrap();
        assert_eq!(invocation.program(), "open");
        assert_eq!(invocation.arguments(), [bundle_path.as_os_str()]);
        runner.assert_drained();
    }
}
