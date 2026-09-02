//! Closed desktop source resolve + macOS DMG install. Windows EXE is not
//! installed through a generic elevated ShellExecute.

use std::{
    fs,
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};

use super::fetch::fetch_metadata_bytes;
#[cfg(target_os = "windows")]
use super::fetch::{artifact_download_hosts, fetch_artifact_to_job};
use super::lifecycle_policy::{lifecycle_policy, ManagedDesktopSourceId};
#[cfg(target_os = "windows")]
use super::sources::PackageFormat;
use super::sources::{
    bounded_version, claude_manifest_url, current_host_target, parse_claude_desktop_manifest,
    parse_qoderwork_latest, parse_traework_latest, parse_workbuddy_update,
    qoderwork_latest_yml_url, resolve_opencode_desktop_latest, workbuddy_update_url, AgentArch,
    AgentPlatform, ResolvedDesktopSource, SourceResolveError, CLAUDE_METADATA_HOSTS,
    QODERWORK_METADATA_HOSTS, TRAEWORK_METADATA_ENDPOINTS, TRAEWORK_METADATA_HOSTS,
    WORKBUDDY_METADATA_HOSTS,
};
use super::types::{
    AgentReasonCode, AgentSurface, InstallationEvidenceCode, InstallationOwner,
    InstallationPackageKind, InstallationScope,
};
#[cfg(target_os = "windows")]
use crate::codex_desktop::cancellation::Cancellation;
#[cfg(target_os = "windows")]
use crate::codex_desktop::{
    download::{DownloadProgressSink, DownloadedArtifact},
    temp::JobTempDir,
    verify::ArtifactKind,
};
use crate::config::get_home_dir;
use crate::services::external_agents::AgentCatalogId;

const MAX_TRAE_PRODUCT_JSON_BYTES: usize = 256 * 1024;
const MAX_WINDOWS_IDENTITY_WINDOW: usize = 512 * 1024;
const MAX_WINDOWS_IDENTITY_FILE: u64 = 512 * 1024 * 1024;

pub(super) struct DesktopProduct {
    pub(super) agent_id: AgentCatalogId,
    pub(super) macos_bundle_id: &'static str,
    pub(super) windows_product_names: &'static [&'static str],
    pub(super) windows_relative_exes: &'static [&'static str],
}

const DESKTOP_PRODUCTS: &[DesktopProduct] = &[
    DesktopProduct {
        agent_id: AgentCatalogId::WorkBuddy,
        macos_bundle_id: "com.workbuddy.workbuddy",
        windows_product_names: &["WorkBuddy"],
        windows_relative_exes: &["WorkBuddy/WorkBuddy.exe"],
    },
    DesktopProduct {
        agent_id: AgentCatalogId::QoderWork,
        macos_bundle_id: "com.qoder.work.cn",
        windows_product_names: &["QoderWork CN", "QoderWorkCN"],
        windows_relative_exes: &[
            "QoderWork CN/QoderWork CN.exe",
            "QoderWorkCN/QoderWorkCN.exe",
        ],
    },
    DesktopProduct {
        agent_id: AgentCatalogId::TraeWork,
        macos_bundle_id: "cn.trae.solo.app",
        windows_product_names: &["TRAE SOLO CN", "Trae Work CN", "TraeWork CN", "TraeWork_CN"],
        windows_relative_exes: &[
            "TRAE SOLO CN/TRAE SOLO CN.exe",
            "Trae Work CN/Trae Work CN.exe",
            "TraeWork_CN/TraeWork_CN.exe",
        ],
    },
    DesktopProduct {
        agent_id: AgentCatalogId::OpenCode,
        macos_bundle_id: "ai.opencode.desktop",
        windows_product_names: &[],
        windows_relative_exes: &[],
    },
    DesktopProduct {
        agent_id: AgentCatalogId::ClaudeCode,
        macos_bundle_id: "com.anthropic.claudefordesktop",
        windows_product_names: &[],
        windows_relative_exes: &[],
    },
];

pub(super) fn desktop_product(agent_id: AgentCatalogId) -> Option<&'static DesktopProduct> {
    DESKTOP_PRODUCTS
        .iter()
        .find(|product| product.agent_id == agent_id)
}

/// Platform evidence stays on the struct for install verification; this AND
/// is the inventory-facing update eligibility for a discovered desktop app.
/// Windows discovery keeps the raw evidence `true` and applies the same AND
/// at inventory projection so post-install readback still sees a trusted EXE.
pub(super) fn discovered_update_eligible(
    agent_id: AgentCatalogId,
    evidence_eligible: bool,
) -> bool {
    evidence_eligible
        && lifecycle_policy(agent_id, AgentSurface::Desktop).is_ok_and(|policy| policy.update)
}

#[cfg(target_os = "macos")]
pub(super) fn macos_bundle_id_for(agent_id: AgentCatalogId) -> Option<&'static str> {
    desktop_product(agent_id).map(|product| product.macos_bundle_id)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DesktopInstallationEvidence {
    pub stable_key: String,
    pub path: PathBuf,
    pub scope: InstallationScope,
    pub package_kind: InstallationPackageKind,
    pub local_version: Option<String>,
    pub owner: InstallationOwner,
    pub launch_eligible: bool,
    pub update_eligible: bool,
    pub reason_codes: Vec<AgentReasonCode>,
    pub evidence_codes: Vec<InstallationEvidenceCode>,
}

pub(super) struct DesktopInstallationDiscovery {
    pub(super) installations: Vec<DesktopInstallationEvidence>,
    pub(super) complete: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DesktopInstallationBaseline {
    installations: Vec<BaselineInstallation>,
    complete: bool,
}

impl DesktopInstallationBaseline {
    #[cfg(target_os = "windows")]
    pub(super) const fn complete(&self) -> bool {
        self.complete
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BaselineInstallation {
    path: PathBuf,
    scope: InstallationScope,
    stable_key: String,
    local_version: Option<String>,
}

#[cfg(any(target_os = "windows", test))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum WindowsDeploymentExpectation {
    Existing {
        path: PathBuf,
        scope: InstallationScope,
    },
    FreshCurrentUser,
    FreshVendorChoice,
}

pub async fn resolve_desktop_source(
    agent_id: AgentCatalogId,
) -> Result<ResolvedDesktopSource, SourceResolveError> {
    let (platform, arch) = current_host_target().ok_or(SourceResolveError::PlatformUnsupported)?;
    let policy = lifecycle_policy(agent_id, AgentSurface::Desktop)
        .map_err(|_| SourceResolveError::PlatformUnsupported)?;
    match policy.managed_desktop_source {
        Some(ManagedDesktopSourceId::QoderWork) => resolve_qoderwork(platform, arch).await,
        Some(ManagedDesktopSourceId::TraeWork) => resolve_traework(platform, arch).await,
        Some(ManagedDesktopSourceId::WorkBuddy) => resolve_workbuddy(platform, arch).await,
        Some(ManagedDesktopSourceId::OpenCodeDesktop) => {
            resolve_opencode_desktop_latest(platform, arch).await
        }
        Some(ManagedDesktopSourceId::ClaudeDesktop) => resolve_claude_desktop(platform, arch).await,
        Some(ManagedDesktopSourceId::CodexDesktopDedicated)
        | Some(ManagedDesktopSourceId::GrokCliTooling)
        | None => Err(SourceResolveError::PlatformUnsupported),
    }
}

async fn resolve_qoderwork(
    platform: AgentPlatform,
    arch: AgentArch,
) -> Result<ResolvedDesktopSource, SourceResolveError> {
    let url = qoderwork_latest_yml_url(platform, arch)?;
    let body = fetch_metadata_bytes(url, QODERWORK_METADATA_HOSTS).await?;
    parse_qoderwork_latest(&body, platform, arch)
}

async fn resolve_traework(
    platform: AgentPlatform,
    arch: AgentArch,
) -> Result<ResolvedDesktopSource, SourceResolveError> {
    let mut last = SourceResolveError::SchemaInvalid;
    for endpoint in TRAEWORK_METADATA_ENDPOINTS {
        let url = url::Url::parse(endpoint).map_err(|_| SourceResolveError::SchemaInvalid)?;
        match fetch_metadata_bytes(url, TRAEWORK_METADATA_HOSTS).await {
            Ok(body) => return parse_traework_latest(&body, platform, arch),
            Err(error) => last = error,
        }
    }
    Err(last)
}

async fn resolve_workbuddy(
    platform: AgentPlatform,
    arch: AgentArch,
) -> Result<ResolvedDesktopSource, SourceResolveError> {
    let url = workbuddy_update_url(platform, arch)?;
    let body = fetch_metadata_bytes(url, WORKBUDDY_METADATA_HOSTS).await?;
    parse_workbuddy_update(&body, platform, arch)
}

async fn resolve_claude_desktop(
    platform: AgentPlatform,
    arch: AgentArch,
) -> Result<ResolvedDesktopSource, SourceResolveError> {
    let url = claude_manifest_url()?;
    let body = fetch_metadata_bytes(url, CLAUDE_METADATA_HOSTS).await?;
    parse_claude_desktop_manifest(&body, platform, arch)
}

pub fn source_reason(error: SourceResolveError) -> AgentReasonCode {
    match error {
        SourceResolveError::PlatformUnsupported => AgentReasonCode::PlatformUnsupported,
        SourceResolveError::Cancelled => AgentReasonCode::Cancelled,
        _ => AgentReasonCode::SourceNotVerified,
    }
}

pub(super) fn readiness_source_codes(error: SourceResolveError) -> Vec<AgentReasonCode> {
    let mut codes = vec![source_reason(error)];
    if !matches!(
        error,
        SourceResolveError::PlatformUnsupported | SourceResolveError::Cancelled
    ) {
        codes.push(AgentReasonCode::OfficialPageOnly);
    }
    codes
}

#[cfg(target_os = "windows")]
pub async fn download_windows_exe_to_job(
    source: &ResolvedDesktopSource,
    job_directory: &JobTempDir,
    cancellation: &dyn Cancellation,
    progress: &dyn DownloadProgressSink,
) -> Result<DownloadedArtifact, AgentReasonCode> {
    if source.platform != AgentPlatform::Windows || source.format != PackageFormat::Exe {
        return Err(AgentReasonCode::PlatformUnsupported);
    }
    let hosts = artifact_download_hosts(source.product)?;
    fetch_artifact_to_job(
        source.download_url.clone(),
        hosts,
        job_directory,
        ArtifactKind::Exe,
        cancellation,
        progress,
    )
    .await
}

#[cfg(target_os = "windows")]
pub fn verify_windows_exe_source(
    source: &ResolvedDesktopSource,
    artifact: &DownloadedArtifact,
) -> Result<(), AgentReasonCode> {
    let product = desktop_product(source.product).ok_or(AgentReasonCode::SourceNotVerified)?;
    artifact
        .revalidate()
        .map_err(|_| AgentReasonCode::SourceNotVerified)?;
    super::windows::verify_windows_installer(product, artifact.path(), source.architecture)?;
    artifact
        .revalidate()
        .map_err(|_| AgentReasonCode::SourceNotVerified)
}

pub(super) fn user_applications_dir() -> Result<PathBuf, AgentReasonCode> {
    Ok(get_home_dir().join("Applications"))
}

#[cfg(target_os = "macos")]
pub(super) fn user_applications_writable() -> bool {
    use std::{ffi::CString, os::unix::ffi::OsStrExt};

    let applications = match user_applications_dir() {
        Ok(path) => path,
        Err(_) => return false,
    };
    let probe = if applications.exists() {
        applications
    } else {
        match applications.parent() {
            Some(parent) => parent.to_path_buf(),
            None => return false,
        }
    };
    let Ok(path) = CString::new(probe.as_os_str().as_bytes()) else {
        return false;
    };
    // Read-only capability projection: do not create probe files while the UI
    // merely asks for an inventory snapshot.
    unsafe { libc::access(path.as_ptr(), libc::W_OK) == 0 }
}

fn isolation_home_override() -> Option<PathBuf> {
    let home = std::env::var("FYAGENT_TEST_HOME").ok()?;
    let trimmed = home.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(PathBuf::from(trimmed))
    }
}

#[allow(dead_code)]
fn macos_application_roots() -> Vec<PathBuf> {
    let home_apps = user_applications_dir().unwrap_or_else(|_| PathBuf::from("/Applications"));
    if isolation_home_override().is_some() {
        vec![home_apps]
    } else {
        vec![home_apps, PathBuf::from("/Applications")]
    }
}

#[allow(dead_code)]
fn windows_program_roots() -> Vec<PathBuf> {
    if let Some(home) = isolation_home_override() {
        return vec![
            home.join("AppData").join("Local").join("Programs"),
            home.join("Program Files"),
            home.join("Program Files (x86)"),
        ];
    }
    production_windows_program_roots()
}

#[cfg(target_os = "windows")]
fn production_windows_program_roots() -> Vec<PathBuf> {
    let mut roots = vec![crate::windows_runtime::user_local_app_data_dir().join("Programs")];
    roots.extend(crate::windows_runtime::machine_program_files_directories());
    roots
}

#[cfg(target_os = "macos")]
fn production_windows_program_roots() -> Vec<PathBuf> {
    Vec::new()
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
mod non_product_host {
    use super::*;

    pub(super) fn launch_on_host(_product: &DesktopProduct) -> Result<(), AgentReasonCode> {
        Err(AgentReasonCode::PlatformUnsupported)
    }

    pub(super) fn production_windows_program_roots() -> Vec<PathBuf> {
        Vec::new()
    }

    pub(super) fn launch_macos_bundle(_path: &Path) -> Result<(), AgentReasonCode> {
        Err(AgentReasonCode::PlatformUnsupported)
    }

    pub(super) fn launch_windows_exe(_path: &Path) -> Result<(), AgentReasonCode> {
        Err(AgentReasonCode::PlatformUnsupported)
    }
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
use non_product_host::{
    launch_macos_bundle, launch_on_host, launch_windows_exe, production_windows_program_roots,
};

pub(super) fn discover_desktop_installations(
    agent_id: AgentCatalogId,
) -> Vec<DesktopInstallationEvidence> {
    discover_desktop_installation_inventory(agent_id).installations
}

pub(super) fn discover_desktop_installation_inventory(
    agent_id: AgentCatalogId,
) -> DesktopInstallationDiscovery {
    let Some(product) = desktop_product(agent_id) else {
        return DesktopInstallationDiscovery {
            installations: Vec::new(),
            complete: true,
        };
    };
    discover_desktop_installation_inventory_on_host(product)
}

pub(super) fn capture_desktop_installation_baseline(
    agent_id: AgentCatalogId,
) -> DesktopInstallationBaseline {
    let discovery = discover_desktop_installation_inventory(agent_id);
    DesktopInstallationBaseline {
        installations: discovery
            .installations
            .into_iter()
            .map(|candidate| BaselineInstallation {
                path: fs::canonicalize(&candidate.path).unwrap_or(candidate.path),
                scope: candidate.scope,
                stable_key: candidate.stable_key,
                local_version: candidate.local_version,
            })
            .collect(),
        complete: discovery.complete,
    }
}

#[cfg(target_os = "macos")]
pub(super) fn verify_desktop_deployment(
    agent_id: AgentCatalogId,
    baseline: &DesktopInstallationBaseline,
    target_path: &Path,
    expected_scope: InstallationScope,
    expected_local_version: &str,
) -> Result<(), AgentReasonCode> {
    verify_desktop_deployment_candidates(
        baseline,
        discover_desktop_installations(agent_id),
        target_path,
        expected_scope,
        expected_local_version,
    )
}

#[cfg(any(target_os = "macos", test))]
fn verify_desktop_deployment_candidates(
    baseline: &DesktopInstallationBaseline,
    after: Vec<DesktopInstallationEvidence>,
    target_path: &Path,
    expected_scope: InstallationScope,
    expected_local_version: &str,
) -> Result<(), AgentReasonCode> {
    let target = fs::canonicalize(target_path)
        .map_err(|_| AgentReasonCode::InstallationVerificationFailed)?;
    let selected = after
        .iter()
        .find(|candidate| {
            candidate.scope == expected_scope
                && candidate.package_kind == InstallationPackageKind::AppBundle
                && fs::canonicalize(&candidate.path)
                    .map(|path| path == target)
                    .unwrap_or(false)
        })
        .ok_or(AgentReasonCode::InstallationVerificationFailed)?;
    if selected.local_version.as_deref() != Some(expected_local_version) {
        return Err(AgentReasonCode::InstallationVerificationFailed);
    }
    for candidate in after {
        let canonical = fs::canonicalize(&candidate.path)
            .map_err(|_| AgentReasonCode::InstallationVerificationFailed)?;
        let existed_before = baseline
            .installations
            .iter()
            .any(|item| item.path == canonical && item.scope == candidate.scope);
        if canonical != target && !existed_before {
            return Err(AgentReasonCode::InstallationVerificationFailed);
        }
    }
    Ok(())
}

#[cfg(any(target_os = "windows", test))]
pub(super) fn verify_windows_deployment_candidates(
    baseline: &DesktopInstallationBaseline,
    after: Vec<DesktopInstallationEvidence>,
    expectation: &WindowsDeploymentExpectation,
    expected_local_version: Option<&str>,
) -> Result<(), AgentReasonCode> {
    if !baseline.complete {
        return Err(AgentReasonCode::NativeProjectionUnavailable);
    }
    let actionable = after
        .iter()
        .filter(|candidate| {
            candidate.package_kind == InstallationPackageKind::Exe
                && candidate.launch_eligible
                && candidate.update_eligible
        })
        .collect::<Vec<_>>();
    let selected = match expectation {
        WindowsDeploymentExpectation::Existing { path, scope } => {
            let target = fs::canonicalize(path).unwrap_or_else(|_| path.clone());
            let selected = actionable
                .iter()
                .copied()
                .find(|candidate| {
                    candidate.scope == *scope
                        && fs::canonicalize(&candidate.path)
                            .unwrap_or_else(|_| candidate.path.clone())
                            == target
                })
                .ok_or(AgentReasonCode::InstallationVerificationFailed)?;
            let before = baseline
                .installations
                .iter()
                .find(|item| item.path == target && item.scope == *scope)
                .ok_or(AgentReasonCode::InstallationVerificationFailed)?;
            if before.stable_key == selected.stable_key
                && before.local_version == selected.local_version
            {
                return Err(AgentReasonCode::InstallationVerificationFailed);
            }
            selected
        }
        WindowsDeploymentExpectation::FreshCurrentUser => {
            let new = actionable
                .iter()
                .copied()
                .filter(|candidate| {
                    candidate.scope == InstallationScope::CurrentUser
                        && !baseline_contains_candidate(baseline, candidate)
                })
                .collect::<Vec<_>>();
            match new.as_slice() {
                [candidate] => *candidate,
                _ => return Err(AgentReasonCode::InstallationVerificationFailed),
            }
        }
        WindowsDeploymentExpectation::FreshVendorChoice => {
            let new = actionable
                .iter()
                .copied()
                .filter(|candidate| !baseline_contains_candidate(baseline, candidate))
                .collect::<Vec<_>>();
            match new.as_slice() {
                [candidate] => *candidate,
                _ => return Err(AgentReasonCode::InstallationVerificationFailed),
            }
        }
    };

    if let Some(expected) = expected_local_version {
        let actual = selected
            .local_version
            .as_deref()
            .ok_or(AgentReasonCode::InstallationVerificationFailed)?;
        if !super::desktop_versions_equivalent(actual, expected) {
            return Err(AgentReasonCode::InstallationVerificationFailed);
        }
    }
    for candidate in actionable {
        if candidate.path != selected.path && !baseline_contains_candidate(baseline, candidate) {
            return Err(AgentReasonCode::InstallationVerificationFailed);
        }
    }
    Ok(())
}

#[cfg(target_os = "windows")]
pub(super) fn verify_windows_deployment(
    agent_id: AgentCatalogId,
    baseline: &DesktopInstallationBaseline,
    expectation: &WindowsDeploymentExpectation,
    expected_local_version: Option<&str>,
) -> Result<(), AgentReasonCode> {
    let discovery = discover_desktop_installation_inventory(agent_id);
    if !discovery.complete {
        return Err(AgentReasonCode::NativeProjectionUnavailable);
    }
    verify_windows_deployment_candidates(
        baseline,
        discovery.installations,
        expectation,
        expected_local_version,
    )
}

#[cfg(any(target_os = "windows", test))]
fn baseline_contains_candidate(
    baseline: &DesktopInstallationBaseline,
    candidate: &DesktopInstallationEvidence,
) -> bool {
    let canonical = fs::canonicalize(&candidate.path).unwrap_or_else(|_| candidate.path.clone());
    baseline
        .installations
        .iter()
        .any(|item| item.path == canonical && item.scope == candidate.scope)
}

#[cfg(target_os = "macos")]
fn discover_desktop_installation_inventory_on_host(
    product: &DesktopProduct,
) -> DesktopInstallationDiscovery {
    DesktopInstallationDiscovery {
        installations: discover_macos_installations(product, &macos_application_roots()),
        complete: true,
    }
}

#[cfg(target_os = "windows")]
fn discover_desktop_installation_inventory_on_host(
    product: &DesktopProduct,
) -> DesktopInstallationDiscovery {
    let discovery = super::windows::discover_windows_inventory(product, &windows_program_roots());
    DesktopInstallationDiscovery {
        installations: discovery.installations,
        complete: discovery.complete,
    }
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn discover_desktop_installation_inventory_on_host(
    _product: &DesktopProduct,
) -> DesktopInstallationDiscovery {
    DesktopInstallationDiscovery {
        installations: Vec::new(),
        complete: true,
    }
}

#[allow(dead_code)]
fn is_regular_app_bundle(path: &Path) -> bool {
    if path.extension().and_then(|ext| ext.to_str()) != Some("app") {
        return false;
    }
    fs::symlink_metadata(path)
        .map(|meta| !meta.file_type().is_symlink() && meta.is_dir())
        .unwrap_or(false)
}

#[allow(dead_code)]
fn is_regular_file(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .map(|meta| !meta.file_type().is_symlink() && meta.is_file())
        .unwrap_or(false)
}

#[allow(dead_code)]
fn discover_macos_installations(
    product: &DesktopProduct,
    roots: &[PathBuf],
) -> Vec<DesktopInstallationEvidence> {
    let user_root = user_applications_dir().ok();
    let mut found = Vec::new();
    for root in roots {
        let Ok(entries) = fs::read_dir(root) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !is_regular_app_bundle(&path)
                || read_bundle_id(&path).as_deref() != Some(product.macos_bundle_id)
                || !path.starts_with(root)
            {
                continue;
            }
            let canonical = fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
            let scope = if user_root.as_ref().is_some_and(|value| root == value) {
                InstallationScope::CurrentUser
            } else if root == Path::new("/Applications") {
                InstallationScope::AllUsers
            } else {
                InstallationScope::Custom
            };
            found.push(DesktopInstallationEvidence {
                stable_key: format!("{}:{}", product.macos_bundle_id, canonical.display()),
                path,
                scope,
                package_kind: InstallationPackageKind::AppBundle,
                local_version: read_macos_local_version(product, &canonical),
                owner: InstallationOwner::VendorInstaller,
                launch_eligible: true,
                update_eligible: discovered_update_eligible(product.agent_id, true),
                reason_codes: Vec::new(),
                evidence_codes: vec![InstallationEvidenceCode::BundleIdentity],
            });
        }
    }
    found
}

#[allow(dead_code)]
pub(super) fn discover_windows_known_path_installations(
    product: &DesktopProduct,
    roots: &[PathBuf],
) -> Vec<DesktopInstallationEvidence> {
    let mut found = Vec::new();
    for (root_index, root) in roots.iter().enumerate() {
        for relative in product.windows_relative_exes {
            let path = root.join(Path::new(relative));
            if !is_regular_file(&path) || !path.starts_with(root) {
                continue;
            }
            let Some(identity) = read_windows_exe_identity(&path) else {
                continue;
            };
            if !product
                .windows_product_names
                .iter()
                .any(|name| identity.product_name == *name)
            {
                continue;
            }
            let canonical = fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
            found.push(DesktopInstallationEvidence {
                stable_key: format!("{}:{}", identity.product_name, canonical.display()),
                path: path.clone(),
                scope: if root_index == 0 {
                    InstallationScope::CurrentUser
                } else {
                    InstallationScope::AllUsers
                },
                package_kind: InstallationPackageKind::Exe,
                local_version: read_windows_local_version(product, &path, identity.product_version),
                owner: InstallationOwner::VendorInstaller,
                launch_eligible: true,
                update_eligible: true,
                reason_codes: Vec::new(),
                evidence_codes: vec![
                    InstallationEvidenceCode::KnownPath,
                    InstallationEvidenceCode::FileIdentity,
                ],
            });
        }
    }
    found
}

struct WindowsExeIdentity {
    product_name: String,
    product_version: Option<String>,
}

#[allow(dead_code)]
fn read_windows_exe_identity(path: &Path) -> Option<WindowsExeIdentity> {
    let bytes = read_windows_identity_window(path)?;
    let product_name = utf16le_value_after_key(&bytes, "ProductName")
        .or_else(|| utf16le_value_after_key(&bytes, "FileDescription"))?;
    let product_version = utf16le_value_after_key(&bytes, "ProductVersion")
        .or_else(|| utf16le_value_after_key(&bytes, "FileVersion"));
    Some(WindowsExeIdentity {
        product_name,
        product_version,
    })
}

#[allow(dead_code)]
fn read_windows_identity_window(path: &Path) -> Option<Vec<u8>> {
    let meta = fs::metadata(path).ok()?;
    if meta.len() == 0 || meta.len() > MAX_WINDOWS_IDENTITY_FILE {
        return None;
    }
    let mut file = fs::File::open(path).ok()?;
    let len = meta.len() as usize;
    if len <= MAX_WINDOWS_IDENTITY_WINDOW * 2 {
        let mut bytes = vec![0_u8; len];
        file.read_exact(&mut bytes).ok()?;
        return Some(bytes);
    }
    let mut bytes = vec![0_u8; MAX_WINDOWS_IDENTITY_WINDOW * 2];
    file.read_exact(&mut bytes[..MAX_WINDOWS_IDENTITY_WINDOW])
        .ok()?;
    file.seek(SeekFrom::End(-(MAX_WINDOWS_IDENTITY_WINDOW as i64)))
        .ok()?;
    file.read_exact(&mut bytes[MAX_WINDOWS_IDENTITY_WINDOW..])
        .ok()?;
    Some(bytes)
}

#[allow(dead_code)]
fn utf16le_value_after_key(bytes: &[u8], key: &str) -> Option<String> {
    let mut needle: Vec<u8> = key.encode_utf16().flat_map(u16::to_le_bytes).collect();
    needle.extend_from_slice(&[0, 0]);
    let mut offset = 0;
    while offset + needle.len() + 2 <= bytes.len() {
        if bytes[offset..].starts_with(&needle) {
            let start = offset + needle.len();
            if let Some(value) = read_utf16le_zstring(&bytes[start..]) {
                return Some(value);
            }
            let aligned = (start + 3) & !3;
            if aligned > start {
                if let Some(value) = read_utf16le_zstring(bytes.get(aligned..)?) {
                    return Some(value);
                }
            }
        }
        offset += 2;
    }
    None
}

#[allow(dead_code)]
fn read_utf16le_zstring(bytes: &[u8]) -> Option<String> {
    let units: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
        .take_while(|unit| *unit != 0)
        .take(128)
        .collect();
    if units.is_empty() {
        return None;
    }
    let value = String::from_utf16(&units).ok()?;
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.len() > 128 {
        None
    } else {
        Some(trimmed.to_string())
    }
}

pub(super) fn launch_desktop_installation(
    agent_id: AgentCatalogId,
    selected_path: &Path,
) -> Result<(), AgentReasonCode> {
    let candidates = discover_desktop_installations(agent_id);
    let selected = candidates
        .into_iter()
        .find(|candidate| candidate.path == selected_path)
        .ok_or(AgentReasonCode::TargetChanged)?;
    match selected.package_kind {
        InstallationPackageKind::AppBundle => launch_macos_bundle(&selected.path),
        InstallationPackageKind::Exe => launch_windows_exe(&selected.path),
        _ => Err(AgentReasonCode::TargetNotExecutable),
    }
}

#[allow(dead_code)]
fn launch_macos_if_present(
    product: &DesktopProduct,
    roots: &[PathBuf],
) -> Result<(), AgentReasonCode> {
    for root in roots {
        let Ok(entries) = fs::read_dir(root) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !is_regular_app_bundle(&path) {
                continue;
            }
            if read_bundle_id(&path).as_deref() != Some(product.macos_bundle_id) {
                continue;
            }
            if !path.starts_with(root) {
                return Err(AgentReasonCode::SourceNotVerified);
            }
            return launch_macos_bundle(&path);
        }
    }
    Err(AgentReasonCode::InstalledNotRunnable)
}

#[allow(dead_code)]
fn launch_windows_if_present(
    product: &DesktopProduct,
    roots: &[PathBuf],
) -> Result<(), AgentReasonCode> {
    let candidates = super::windows::discover_windows_installations(product, roots)
        .into_iter()
        .filter(|candidate| candidate.launch_eligible)
        .collect::<Vec<_>>();
    match candidates.as_slice() {
        [candidate] => launch_windows_exe(&candidate.path),
        [] => Err(AgentReasonCode::InstalledNotRunnable),
        _ => Err(AgentReasonCode::TargetSelectionRequired),
    }
}

#[cfg(target_os = "macos")]
fn launch_macos_bundle(path: &Path) -> Result<(), AgentReasonCode> {
    crate::platform::process_launch::launch_trusted_macos_application_as_user(path).map_err(
        |public_code| {
            if public_code == "external_launch_invalid_macos_application" {
                AgentReasonCode::TargetNotExecutable
            } else {
                AgentReasonCode::ApplicationLaunchFailed
            }
        },
    )
}

#[cfg(target_os = "windows")]
fn launch_windows_exe(path: &Path) -> Result<(), AgentReasonCode> {
    crate::platform::process_launch::launch_trusted_windows_exe_as_user(path)
        .map_err(|_| AgentReasonCode::InteractiveUserUnavailable)
}

#[cfg(target_os = "windows")]
fn launch_macos_bundle(_path: &Path) -> Result<(), AgentReasonCode> {
    Err(AgentReasonCode::PlatformUnsupported)
}

#[cfg(target_os = "macos")]
fn launch_windows_exe(_path: &Path) -> Result<(), AgentReasonCode> {
    Err(AgentReasonCode::PlatformUnsupported)
}

#[allow(dead_code)]
fn read_macos_local_version(product: &DesktopProduct, app: &Path) -> Option<String> {
    if product.agent_id == AgentCatalogId::TraeWork {
        if let Some(version) = trae_macos_product_json(app).and_then(read_trae_tron_build_version) {
            return Some(version);
        }
    }
    read_bundle_version(app)
}

#[allow(dead_code)]
fn read_windows_local_version(
    product: &DesktopProduct,
    exe: &Path,
    pe_version: Option<String>,
) -> Option<String> {
    if product.agent_id == AgentCatalogId::TraeWork {
        if let Some(version) = trae_windows_product_json(exe).and_then(read_trae_tron_build_version)
        {
            return Some(version);
        }
    }
    pe_version
}

#[allow(dead_code)]
fn trae_macos_product_json(app: &Path) -> Option<PathBuf> {
    let path = app
        .join("Contents")
        .join("Resources")
        .join("app")
        .join("product.json");
    if path.starts_with(app) && is_regular_file(&path) {
        Some(path)
    } else {
        None
    }
}

#[allow(dead_code)]
fn trae_windows_product_json(exe: &Path) -> Option<PathBuf> {
    let dir = exe.parent()?;
    let path = dir.join("resources").join("app").join("product.json");
    if path.starts_with(dir) && is_regular_file(&path) {
        Some(path)
    } else {
        None
    }
}

#[allow(dead_code)]
fn read_trae_tron_build_version(path: PathBuf) -> Option<String> {
    let meta = fs::metadata(&path).ok()?;
    if meta.len() == 0 || meta.len() > MAX_TRAE_PRODUCT_JSON_BYTES as u64 {
        return None;
    }
    let bytes = fs::read(&path).ok()?;
    let value: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    let version = value.get("tronBuildVersion")?.as_str()?;
    bounded_version(version).map(str::to_string)
}

struct StructuredBundlePlist {
    bundle_identifier: Option<String>,
    short_version: Option<String>,
    bundle_version: Option<String>,
}

#[allow(dead_code)]
fn read_bundle_id(app: &Path) -> Option<String> {
    bounded_plist_string(read_structured_plist(app)?.bundle_identifier)
}

#[allow(dead_code)]
fn read_bundle_version(app: &Path) -> Option<String> {
    let raw = read_structured_plist(app)?;
    bounded_plist_string(raw.short_version).or_else(|| bounded_plist_string(raw.bundle_version))
}

#[allow(dead_code)]
fn read_structured_plist(app: &Path) -> Option<StructuredBundlePlist> {
    #[cfg(any(target_os = "macos", test))]
    {
        let raw = crate::codex_desktop::platform::macos::bundle::read_structured_bundle_plist(app)?;
        Some(StructuredBundlePlist {
            bundle_identifier: raw.bundle_identifier,
            short_version: raw.short_version,
            bundle_version: raw.bundle_version,
        })
    }
    #[cfg(not(any(target_os = "macos", test)))]
    {
        let _ = app;
        None
    }
}

#[allow(dead_code)]
fn bounded_plist_string(value: Option<String>) -> Option<String> {
    let value = value?;
    if value.is_empty()
        || value.trim() != value
        || value.len() > 128
        || value.chars().any(char::is_control)
    {
        None
    } else {
        Some(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn grok_and_codex_do_not_resolve_managed_desktop_sources() {
        assert_eq!(
            resolve_desktop_source(AgentCatalogId::GrokBuild).await,
            Err(SourceResolveError::PlatformUnsupported)
        );
        assert_eq!(
            resolve_desktop_source(AgentCatalogId::Codex).await,
            Err(SourceResolveError::PlatformUnsupported)
        );
    }

    #[test]
    fn source_failure_surfaces_official_page_fallback_not_cancel() {
        assert_eq!(
            source_reason(SourceResolveError::Cancelled),
            AgentReasonCode::Cancelled
        );
        assert_eq!(
            readiness_source_codes(SourceResolveError::HostRejected),
            vec![
                AgentReasonCode::SourceNotVerified,
                AgentReasonCode::OfficialPageOnly,
            ]
        );
        assert_eq!(
            readiness_source_codes(SourceResolveError::PlatformUnsupported),
            vec![AgentReasonCode::PlatformUnsupported]
        );
        assert_eq!(
            readiness_source_codes(SourceResolveError::Cancelled),
            vec![AgentReasonCode::Cancelled]
        );
    }

    #[cfg(target_os = "macos")]
    fn write_fake_macos_app(root: &Path, name: &str, bundle_id: &str, version: &str) {
        let app = root.join(format!("{name}.app"));
        fs::create_dir_all(app.join("Contents")).expect("app contents");
        fs::write(
            app.join("Contents/Info.plist"),
            format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleIdentifier</key>
    <string>{bundle_id}</string>
    <key>CFBundleShortVersionString</key>
    <string>{version}</string>
</dict>
</plist>
"#
            ),
        )
        .expect("plist");
    }

    fn utf16le_key_value(key: &str, value: &str) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend(key.encode_utf16().flat_map(u16::to_le_bytes));
        bytes.extend_from_slice(&[0, 0]);
        bytes.extend(value.encode_utf16().flat_map(u16::to_le_bytes));
        bytes.extend_from_slice(&[0, 0]);
        bytes
    }

    fn write_fake_windows_exe(path: &Path, product_name: &str, version: &str) {
        fs::create_dir_all(path.parent().expect("exe parent")).expect("exe directory");
        let mut bytes = b"MZ".to_vec();
        bytes.extend_from_slice(&[0; 32]);
        bytes.extend(utf16le_key_value("ProductName", product_name));
        bytes.extend(utf16le_key_value("ProductVersion", version));
        fs::write(path, bytes).expect("fake exe");
    }

    fn product(agent_id: AgentCatalogId) -> &'static DesktopProduct {
        desktop_product(agent_id).expect("closed desktop product")
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_observation_keys_off_bundle_id_not_folder_name() {
        let root = tempfile::tempdir().expect("temp");
        let apps = root.path().join("Applications");
        fs::create_dir_all(&apps).expect("applications");
        write_fake_macos_app(
            &apps,
            "NotTheProductName",
            "com.workbuddy.workbuddy",
            "5.3.14",
        );
        write_fake_macos_app(&apps, "WorkBuddy", "com.evil.workbuddy", "9.9.9");
        write_fake_macos_app(&apps, "QoderWork CN", "com.qoder.work.cn", "0.9.12");
        write_fake_macos_app(&apps, "TRAE SOLO CN", "cn.trae.solo.app", "0.1.51");

        let workbuddy = discover_macos_installations(
            product(AgentCatalogId::WorkBuddy),
            std::slice::from_ref(&apps),
        );
        assert_eq!(workbuddy.len(), 1);
        assert_eq!(workbuddy[0].local_version.as_deref(), Some("5.3.14"));

        let qoder = discover_macos_installations(
            product(AgentCatalogId::QoderWork),
            std::slice::from_ref(&apps),
        );
        assert_eq!(qoder.len(), 1);
        assert_eq!(qoder[0].local_version.as_deref(), Some("0.9.12"));

        let trae = discover_macos_installations(
            product(AgentCatalogId::TraeWork),
            std::slice::from_ref(&apps),
        );
        assert_eq!(trae.len(), 1);
        assert_eq!(trae[0].local_version.as_deref(), Some("0.1.51"));
    }

    #[test]
    fn opencode_is_a_managed_desktop_product_with_official_bundle_id() {
        let item = product(AgentCatalogId::OpenCode);
        assert_eq!(item.macos_bundle_id, "ai.opencode.desktop");
        assert!(item.windows_product_names.is_empty());
        assert!(item.windows_relative_exes.is_empty());
    }

    #[test]
    fn claude_is_a_managed_desktop_product_with_official_bundle_id() {
        let item = product(AgentCatalogId::ClaudeCode);
        assert_eq!(item.macos_bundle_id, "com.anthropic.claudefordesktop");
        assert!(item.windows_product_names.is_empty());
        assert!(item.windows_relative_exes.is_empty());
        assert!(desktop_product(AgentCatalogId::GrokBuild).is_none());
    }

    #[test]
    fn discovered_update_eligibility_ands_product_policy() {
        assert!(!discovered_update_eligible(AgentCatalogId::QoderWork, true));
        assert!(!discovered_update_eligible(AgentCatalogId::TraeWork, true));
        assert!(!discovered_update_eligible(AgentCatalogId::WorkBuddy, true));
        assert!(discovered_update_eligible(AgentCatalogId::OpenCode, true));
        assert!(discovered_update_eligible(AgentCatalogId::ClaudeCode, true));
        assert!(!discovered_update_eligible(
            AgentCatalogId::WorkBuddy,
            false
        ));
        assert!(!discovered_update_eligible(AgentCatalogId::OpenCode, false));
        assert!(!discovered_update_eligible(AgentCatalogId::GrokBuild, true));
    }

    #[test]
    fn managed_desktop_launch_delegates_to_process_launch_and_does_not_scan_launch_services() {
        let source = include_str!("desktop.rs");
        let production = source
            .split("#[cfg(test)]")
            .next()
            .expect("production source");
        assert!(production.contains("launch_trusted_macos_application_as_user"));
        assert!(production.contains("read_structured_bundle_plist"));
        assert!(!production.contains("Command::new(\"open\")"));
        for needle in [
            "mdfind",
            "lsregister",
            "NSWorkspace",
            "MDQuery",
            "LaunchServices",
        ] {
            assert!(
                !production.contains(needle),
                "managed discovery must not grow a Launch Services scanner: {needle}"
            );
        }
    }

    #[cfg(target_os = "macos")]
    fn convert_plist_to_binary(app: &Path) {
        let plist = app.join("Contents/Info.plist");
        let status = std::process::Command::new("plutil")
            .args(["-convert", "binary1"])
            .arg(&plist)
            .status()
            .expect("plutil");
        assert!(status.success());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn opencode_desktop_discovery_covers_zero_one_many_binary_wrong_id_symlink_and_nested() {
        let root = tempfile::tempdir().expect("temp");
        let user = root.path().join("UserApps");
        let system = root.path().join("SystemApps");
        fs::create_dir_all(&user).expect("user apps");
        fs::create_dir_all(&system).expect("system apps");

        assert!(discover_macos_installations(
            product(AgentCatalogId::OpenCode),
            &[user.clone(), system.clone()],
        )
        .is_empty());

        write_fake_macos_app(&user, "OpenCode", "ai.opencode.desktop", "1.2.3");
        write_fake_macos_app(&system, "OpenCode", "ai.opencode.desktop", "1.4.0");
        write_fake_macos_app(&user, "OpenCode Fake", "ai.opencode.imposter", "9.9.9");
        let nested = user.join("Nested");
        fs::create_dir_all(&nested).expect("nested");
        write_fake_macos_app(&nested, "OpenCode", "ai.opencode.desktop", "0.0.1");
        convert_plist_to_binary(&system.join("OpenCode.app"));

        let symlink_app = user.join("OpenCode Link.app");
        std::os::unix::fs::symlink(user.join("OpenCode.app"), &symlink_app).expect("symlink");

        let found = discover_macos_installations(
            product(AgentCatalogId::OpenCode),
            &[user.clone(), system.clone()],
        );
        assert_eq!(found.len(), 2);
        assert!(found.iter().all(|item| item
            .evidence_codes
            .contains(&InstallationEvidenceCode::BundleIdentity)));
        assert!(found
            .iter()
            .any(|item| item.local_version.as_deref() == Some("1.2.3")));
        assert!(found
            .iter()
            .any(|item| item.local_version.as_deref() == Some("1.4.0")));
        assert!(!found.iter().any(|item| item.path == symlink_app));
        assert!(!found.iter().any(|item| item.path.starts_with(&nested)));

        fs::write(
            user.join("OpenCode.app/Contents/Info.plist"),
            b"not a plist",
        )
        .expect("corrupt");
        let after_corrupt = discover_macos_installations(
            product(AgentCatalogId::OpenCode),
            std::slice::from_ref(&user),
        );
        assert!(after_corrupt.is_empty());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn claude_desktop_discovery_keys_off_bundle_id_and_info_plist_version() {
        let root = tempfile::tempdir().expect("temp");
        let apps = root.path().join("Applications");
        fs::create_dir_all(&apps).expect("applications");
        write_fake_macos_app(&apps, "Claude", "com.anthropic.claudefordesktop", "1.2.3");
        write_fake_macos_app(&apps, "Claude Fake", "com.anthropic.imposter", "9.9.9");

        let found = discover_macos_installations(
            product(AgentCatalogId::ClaudeCode),
            std::slice::from_ref(&apps),
        );
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].local_version.as_deref(), Some("1.2.3"));
        assert_eq!(found[0].package_kind, InstallationPackageKind::AppBundle);
        assert!(found[0]
            .path
            .file_name()
            .is_some_and(|name| name == "Claude.app"));
    }

    fn write_trae_product_json(install_root: &Path, tron_build_version: &str) {
        fs::create_dir_all(install_root).expect("product json dir");
        fs::write(
            install_root.join("product.json"),
            format!(r#"{{"appVersion":"0.1.51","tronBuildVersion":"{tron_build_version}"}}"#),
        )
        .expect("product json");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn trae_macos_local_version_uses_tron_build_not_electron_app_version() {
        let root = tempfile::tempdir().expect("temp");
        let apps = root.path().join("Applications");
        fs::create_dir_all(&apps).expect("applications");
        write_fake_macos_app(&apps, "TRAE SOLO CN", "cn.trae.solo.app", "0.1.51");
        write_trae_product_json(
            &apps
                .join("TRAE SOLO CN.app")
                .join("Contents")
                .join("Resources")
                .join("app"),
            "2.3.71801",
        );
        let macos = discover_macos_installations(
            product(AgentCatalogId::TraeWork),
            std::slice::from_ref(&apps),
        );
        assert_eq!(macos.len(), 1);
        assert_eq!(macos[0].local_version.as_deref(), Some("2.3.71801"));
    }

    #[test]
    fn trae_windows_local_version_uses_tron_build_not_electron_app_version() {
        let root = tempfile::tempdir().expect("temp");
        let programs = root.path().join("Programs");
        write_fake_windows_exe(
            &programs.join("TRAE SOLO CN").join("TRAE SOLO CN.exe"),
            "TRAE SOLO CN",
            "0.1.51",
        );
        write_trae_product_json(
            &programs.join("TRAE SOLO CN").join("resources").join("app"),
            "2.3.71801",
        );
        let windows = discover_windows_known_path_installations(
            product(AgentCatalogId::TraeWork),
            std::slice::from_ref(&programs),
        );
        assert_eq!(windows.len(), 1);
        assert_eq!(windows[0].local_version.as_deref(), Some("2.3.71801"));
    }

    #[test]
    fn windows_observation_requires_closed_relative_exe_and_product_name() {
        let root = tempfile::tempdir().expect("temp");
        let programs = root.path().join("Programs");
        write_fake_windows_exe(
            &programs.join("WorkBuddy").join("WorkBuddy.exe"),
            "WorkBuddy",
            "5.3.14",
        );
        write_fake_windows_exe(
            &programs.join("QoderWork CN").join("QoderWork CN.exe"),
            "Not Qoder",
            "0.9.12",
        );
        write_fake_windows_exe(
            &programs.join("QoderWorkCN").join("QoderWorkCN.exe"),
            "QoderWorkCN",
            "0.9.12",
        );
        write_fake_windows_exe(
            &programs.join("Other").join("WorkBuddy.exe"),
            "WorkBuddy",
            "5.3.14",
        );

        let workbuddy = discover_windows_known_path_installations(
            product(AgentCatalogId::WorkBuddy),
            std::slice::from_ref(&programs),
        );
        assert_eq!(workbuddy.len(), 1);
        assert_eq!(workbuddy[0].local_version.as_deref(), Some("5.3.14"));

        let qoder = discover_windows_known_path_installations(
            product(AgentCatalogId::QoderWork),
            std::slice::from_ref(&programs),
        );
        assert_eq!(qoder.len(), 1);
        assert_eq!(qoder[0].local_version.as_deref(), Some("0.9.12"));

        assert!(discover_windows_known_path_installations(
            product(AgentCatalogId::TraeWork),
            std::slice::from_ref(&programs),
        )
        .is_empty());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn vendor_config_directories_are_not_macos_install_evidence() {
        let root = tempfile::tempdir().expect("temp");
        fs::create_dir_all(root.path().join(".workbuddy")).expect("workbuddy config");
        fs::create_dir_all(root.path().join(".qoderwork")).expect("qoder config");
        fs::create_dir_all(root.path().join(".trae")).expect("trae config");
        let apps = root.path().join("Applications");
        fs::create_dir_all(&apps).expect("applications");

        for agent_id in [
            AgentCatalogId::WorkBuddy,
            AgentCatalogId::QoderWork,
            AgentCatalogId::TraeWork,
        ] {
            let item = product(agent_id);
            assert!(discover_macos_installations(item, std::slice::from_ref(&apps)).is_empty());
            assert_eq!(
                launch_macos_if_present(item, std::slice::from_ref(&apps)),
                Err(AgentReasonCode::InstalledNotRunnable)
            );
        }
    }

    #[test]
    fn vendor_config_directories_are_not_windows_known_path_evidence() {
        let root = tempfile::tempdir().expect("temp");
        fs::create_dir_all(root.path().join(".workbuddy")).expect("workbuddy config");
        fs::create_dir_all(root.path().join(".qoderwork")).expect("qoder config");
        fs::create_dir_all(root.path().join(".trae")).expect("trae config");
        let programs = root.path().join("Programs");
        fs::create_dir_all(&programs).expect("programs");

        for agent_id in [
            AgentCatalogId::WorkBuddy,
            AgentCatalogId::QoderWork,
            AgentCatalogId::TraeWork,
        ] {
            assert!(discover_windows_known_path_installations(
                product(agent_id),
                std::slice::from_ref(&programs),
            )
            .is_empty());
        }
    }

    fn deployment_evidence(
        path: PathBuf,
        scope: InstallationScope,
        version: &str,
    ) -> DesktopInstallationEvidence {
        DesktopInstallationEvidence {
            stable_key: format!("test:{}", path.display()),
            path,
            scope,
            package_kind: InstallationPackageKind::AppBundle,
            local_version: Some(version.to_string()),
            owner: InstallationOwner::VendorInstaller,
            launch_eligible: true,
            update_eligible: true,
            reason_codes: Vec::new(),
            evidence_codes: vec![InstallationEvidenceCode::BundleIdentity],
        }
    }

    fn windows_deployment_evidence(
        path: PathBuf,
        scope: InstallationScope,
        version: &str,
        stable_key: &str,
    ) -> DesktopInstallationEvidence {
        DesktopInstallationEvidence {
            stable_key: stable_key.to_string(),
            path,
            scope,
            package_kind: InstallationPackageKind::Exe,
            local_version: Some(version.to_string()),
            owner: InstallationOwner::VendorInstaller,
            launch_eligible: true,
            update_eligible: true,
            reason_codes: Vec::new(),
            evidence_codes: vec![InstallationEvidenceCode::FileIdentity],
        }
    }

    #[test]
    fn deployment_readback_requires_the_exact_selected_path_scope_and_version() {
        let root = tempfile::tempdir().expect("temp");
        let user = root.path().join("User Applications");
        let system = root.path().join("System Applications");
        let selected = user.join("Product.app");
        let existing_system = system.join("Product.app");
        fs::create_dir_all(&selected).expect("selected bundle");
        fs::create_dir_all(&existing_system).expect("existing system bundle");
        let selected = fs::canonicalize(selected).expect("selected canonical");
        let existing_system = fs::canonicalize(existing_system).expect("system canonical");
        let baseline = DesktopInstallationBaseline {
            installations: vec![BaselineInstallation {
                path: existing_system.clone(),
                scope: InstallationScope::AllUsers,
                stable_key: "test:existing-system".to_string(),
                local_version: Some("1.0.0".to_string()),
            }],
            complete: true,
        };
        let after = vec![
            deployment_evidence(selected.clone(), InstallationScope::CurrentUser, "2.0.0"),
            deployment_evidence(existing_system, InstallationScope::AllUsers, "1.0.0"),
        ];

        assert_eq!(
            verify_desktop_deployment_candidates(
                &baseline,
                after.clone(),
                &selected,
                InstallationScope::CurrentUser,
                "2.0.0",
            ),
            Ok(())
        );
        assert_eq!(
            verify_desktop_deployment_candidates(
                &baseline,
                after.clone(),
                &selected,
                InstallationScope::AllUsers,
                "2.0.0",
            ),
            Err(AgentReasonCode::InstallationVerificationFailed)
        );
        assert_eq!(
            verify_desktop_deployment_candidates(
                &baseline,
                after,
                &selected,
                InstallationScope::CurrentUser,
                "2.0.1",
            ),
            Err(AgentReasonCode::InstallationVerificationFailed)
        );
    }

    #[test]
    fn deployment_readback_rejects_an_undeclared_cross_scope_copy() {
        let root = tempfile::tempdir().expect("temp");
        let selected = root.path().join("User Applications/Product.app");
        let duplicate = root.path().join("System Applications/Product.app");
        fs::create_dir_all(&selected).expect("selected bundle");
        fs::create_dir_all(&duplicate).expect("duplicate bundle");
        let selected = fs::canonicalize(selected).expect("selected canonical");
        let duplicate = fs::canonicalize(duplicate).expect("duplicate canonical");

        assert_eq!(
            verify_desktop_deployment_candidates(
                &DesktopInstallationBaseline {
                    installations: Vec::new(),
                    complete: true,
                },
                vec![
                    deployment_evidence(selected.clone(), InstallationScope::CurrentUser, "2.0.0",),
                    deployment_evidence(duplicate, InstallationScope::AllUsers, "2.0.0",),
                ],
                &selected,
                InstallationScope::CurrentUser,
                "2.0.0",
            ),
            Err(AgentReasonCode::InstallationVerificationFailed)
        );
    }

    #[test]
    fn windows_fresh_readback_requires_one_new_candidate_in_the_selected_scope() {
        let user = PathBuf::from("C:/Users/Alice/AppData/Local/Programs/WorkBuddy/WorkBuddy.exe");
        let machine = PathBuf::from("C:/Program Files/WorkBuddy/WorkBuddy.exe");
        let baseline = DesktopInstallationBaseline {
            installations: Vec::new(),
            complete: true,
        };
        assert_eq!(
            verify_windows_deployment_candidates(
                &baseline,
                vec![windows_deployment_evidence(
                    user.clone(),
                    InstallationScope::CurrentUser,
                    "5.3.14",
                    "file:new-user",
                )],
                &WindowsDeploymentExpectation::FreshCurrentUser,
                Some("5.3.14.36279234"),
            ),
            Ok(())
        );
        assert_eq!(
            verify_windows_deployment_candidates(
                &baseline,
                vec![windows_deployment_evidence(
                    machine,
                    InstallationScope::AllUsers,
                    "5.3.14",
                    "file:new-machine",
                )],
                &WindowsDeploymentExpectation::FreshCurrentUser,
                Some("5.3.14.36279234"),
            ),
            Err(AgentReasonCode::InstallationVerificationFailed)
        );
        assert_eq!(
            verify_windows_deployment_candidates(
                &baseline,
                vec![
                    windows_deployment_evidence(
                        user,
                        InstallationScope::CurrentUser,
                        "5.3.14",
                        "file:new-user",
                    ),
                    windows_deployment_evidence(
                        PathBuf::from("D:/Apps/WorkBuddy.exe"),
                        InstallationScope::Custom,
                        "5.3.14",
                        "file:new-custom",
                    ),
                ],
                &WindowsDeploymentExpectation::FreshVendorChoice,
                Some("5.3.14.36279234"),
            ),
            Err(AgentReasonCode::InstallationVerificationFailed)
        );
    }

    #[test]
    fn windows_readback_never_succeeds_from_an_incomplete_baseline() {
        let baseline = DesktopInstallationBaseline {
            installations: Vec::new(),
            complete: false,
        };
        let after = vec![windows_deployment_evidence(
            PathBuf::from("C:/Users/Alice/AppData/Local/Programs/WorkBuddy/WorkBuddy.exe"),
            InstallationScope::CurrentUser,
            "5.3.14",
            "file:new-user",
        )];

        assert_eq!(
            verify_windows_deployment_candidates(
                &baseline,
                after,
                &WindowsDeploymentExpectation::FreshCurrentUser,
                Some("5.3.14.36279234"),
            ),
            Err(AgentReasonCode::NativeProjectionUnavailable)
        );
    }

    #[test]
    fn windows_update_readback_requires_authoritative_change_at_the_same_target() {
        let path = PathBuf::from("C:/Users/Alice/AppData/Local/Programs/Qoder/Qoder.exe");
        let baseline = DesktopInstallationBaseline {
            installations: vec![BaselineInstallation {
                path: path.clone(),
                scope: InstallationScope::CurrentUser,
                stable_key: "file:before".to_string(),
                local_version: Some("0.9.12".to_string()),
            }],
            complete: true,
        };
        let expectation = WindowsDeploymentExpectation::Existing {
            path: path.clone(),
            scope: InstallationScope::CurrentUser,
        };
        assert_eq!(
            verify_windows_deployment_candidates(
                &baseline,
                vec![windows_deployment_evidence(
                    path.clone(),
                    InstallationScope::CurrentUser,
                    "0.9.15",
                    "file:after",
                )],
                &expectation,
                Some("0.9.15"),
            ),
            Ok(())
        );
        assert_eq!(
            verify_windows_deployment_candidates(
                &baseline,
                vec![windows_deployment_evidence(
                    path,
                    InstallationScope::CurrentUser,
                    "0.9.12",
                    "file:before",
                )],
                &expectation,
                Some("0.9.15"),
            ),
            Err(AgentReasonCode::InstallationVerificationFailed)
        );
    }
}
