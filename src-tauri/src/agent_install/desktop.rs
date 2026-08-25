//! Closed desktop source resolve + macOS DMG install. Windows EXE is not
//! installed through a generic elevated ShellExecute.

use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
};

use super::fetch::{fetch_artifact_bytes, fetch_metadata_bytes};
use super::sources::{
    current_host_target, parse_traework_latest, parse_workbuddy_update, resolve_qoderwork_source,
    workbuddy_update_url, AgentArch, AgentPlatform, PackageFormat, ResolvedDesktopSource,
    SourceResolveError, QODERWORK_REDIRECT_HOSTS, TRAEWORK_DOWNLOAD_HOSTS,
    TRAEWORK_METADATA_ENDPOINTS, TRAEWORK_METADATA_HOSTS, WORKBUDDY_DOWNLOAD_HOSTS,
    WORKBUDDY_METADATA_HOSTS,
};
use super::types::AgentReasonCode;
use crate::codex_desktop::cancellation::Cancellation;
use crate::config::get_home_dir;
use crate::services::external_agents::AgentCatalogId;

const TRAE_BUNDLE_ID: &str = "cn.trae.solo.app";
const MAX_PLIST_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopObservation {
    pub installed: bool,
    pub local_version: Option<String>,
}

pub async fn resolve_desktop_source(
    agent_id: AgentCatalogId,
) -> Result<ResolvedDesktopSource, SourceResolveError> {
    let (platform, arch) = current_host_target().ok_or(SourceResolveError::PlatformUnsupported)?;
    match agent_id {
        AgentCatalogId::QoderWork => resolve_qoderwork_source(platform, arch),
        AgentCatalogId::TraeWork => resolve_traework(platform, arch).await,
        AgentCatalogId::WorkBuddy => resolve_workbuddy(platform, arch).await,
        _ => Err(SourceResolveError::PlatformUnsupported),
    }
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

pub fn windows_exe_unavailable(source: &ResolvedDesktopSource) -> bool {
    source.platform == AgentPlatform::Windows && source.format == PackageFormat::Exe
}

#[allow(dead_code)]
pub async fn install_resolved_source(
    source: &ResolvedDesktopSource,
    cancellation: &dyn Cancellation,
) -> Result<(), AgentReasonCode> {
    let bytes = download_resolved_source(source, cancellation).await?;
    if cancellation.is_cancelled() {
        return Err(AgentReasonCode::Cancelled);
    }
    install_macos_dmg(source.product, &bytes)
}

pub async fn download_resolved_source(
    source: &ResolvedDesktopSource,
    cancellation: &dyn Cancellation,
) -> Result<Vec<u8>, AgentReasonCode> {
    if cancellation.is_cancelled() {
        return Err(AgentReasonCode::Cancelled);
    }
    if windows_exe_unavailable(source) {
        return Err(AgentReasonCode::InteractiveUserUnavailable);
    }
    if source.format != PackageFormat::Dmg || source.platform != AgentPlatform::Macos {
        return Err(AgentReasonCode::PlatformUnsupported);
    }
    let hosts = download_hosts(source.product)?;
    fetch_artifact_bytes(source.download_url.clone(), hosts, cancellation)
        .await
        .map_err(source_reason)
}

pub fn finish_macos_dmg_install(
    product: AgentCatalogId,
    bytes: &[u8],
) -> Result<(), AgentReasonCode> {
    install_macos_dmg(product, bytes)
}

fn download_hosts(product: AgentCatalogId) -> Result<&'static [&'static str], AgentReasonCode> {
    match product {
        AgentCatalogId::QoderWork => Ok(QODERWORK_REDIRECT_HOSTS),
        AgentCatalogId::TraeWork => Ok(TRAEWORK_DOWNLOAD_HOSTS),
        AgentCatalogId::WorkBuddy => Ok(WORKBUDDY_DOWNLOAD_HOSTS),
        _ => Err(AgentReasonCode::ExecutorNotImplemented),
    }
}

fn install_macos_dmg(product: AgentCatalogId, bytes: &[u8]) -> Result<(), AgentReasonCode> {
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (product, bytes);
        return Err(AgentReasonCode::PlatformUnsupported);
    }
    #[cfg(target_os = "macos")]
    {
        let job_dir = job_temp_dir()?;
        let dmg_path = job_dir.join("installer.dmg");
        write_exclusive(&dmg_path, bytes)?;
        let mounted = mount_dmg(&dmg_path);
        let result = mounted.and_then(|mount| {
            let outcome = install_from_mount(product, &mount);
            let _ = detach_dmg(&mount);
            outcome
        });
        let _ = fs::remove_dir_all(&job_dir);
        result
    }
}

#[cfg(target_os = "macos")]
fn job_temp_dir() -> Result<PathBuf, AgentReasonCode> {
    let root = crate::config::get_user_temp_dir().join("fyagent-agent-installer");
    fs::create_dir_all(&root).map_err(|_| AgentReasonCode::ExecutorNotImplemented)?;
    let dir = root.join(uuid::Uuid::new_v4().to_string());
    fs::create_dir(&dir).map_err(|_| AgentReasonCode::ExecutorNotImplemented)?;
    Ok(dir)
}

#[cfg(target_os = "macos")]
fn write_exclusive(path: &Path, bytes: &[u8]) -> Result<(), AgentReasonCode> {
    let mut file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(|_| AgentReasonCode::ExecutorNotImplemented)?;
    file.write_all(bytes)
        .map_err(|_| AgentReasonCode::ExecutorNotImplemented)?;
    file.flush()
        .map_err(|_| AgentReasonCode::ExecutorNotImplemented)?;
    file.sync_all()
        .map_err(|_| AgentReasonCode::ExecutorNotImplemented)?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn mount_dmg(path: &Path) -> Result<PathBuf, AgentReasonCode> {
    let output = Command::new("hdiutil")
        .args(["attach", "-nobrowse", "-readonly", "-plist"])
        .arg(path)
        .output()
        .map_err(|_| AgentReasonCode::ExecutorNotImplemented)?;
    if !output.status.success() {
        return Err(AgentReasonCode::ExecutorNotImplemented);
    }
    parse_mount_point(&output.stdout).ok_or(AgentReasonCode::ExecutorNotImplemented)
}

#[cfg(target_os = "macos")]
fn detach_dmg(mount: &Path) -> Result<(), AgentReasonCode> {
    let status = Command::new("hdiutil")
        .args(["detach", "-quiet"])
        .arg(mount)
        .status()
        .map_err(|_| AgentReasonCode::ExecutorNotImplemented)?;
    if status.success() {
        Ok(())
    } else {
        Err(AgentReasonCode::ExecutorNotImplemented)
    }
}

#[cfg(target_os = "macos")]
fn install_from_mount(product: AgentCatalogId, mount: &Path) -> Result<(), AgentReasonCode> {
    let app = discover_single_app(mount)?;
    if product == AgentCatalogId::TraeWork {
        let id = read_bundle_id(&app).ok_or(AgentReasonCode::SourceNotVerified)?;
        if id != TRAE_BUNDLE_ID {
            return Err(AgentReasonCode::SourceNotVerified);
        }
    }
    let dest_parent = user_applications_dir()?;
    fs::create_dir_all(&dest_parent).map_err(|_| AgentReasonCode::ExecutorNotImplemented)?;
    let dest = dest_parent.join(
        app.file_name()
            .ok_or(AgentReasonCode::ExecutorNotImplemented)?,
    );
    let status = Command::new("ditto")
        .arg(&app)
        .arg(&dest)
        .status()
        .map_err(|_| AgentReasonCode::ExecutorNotImplemented)?;
    if !status.success() {
        return Err(AgentReasonCode::ExecutorNotImplemented);
    }
    if product == AgentCatalogId::TraeWork {
        let installed_id = read_bundle_id(&dest).ok_or(AgentReasonCode::SourceNotVerified)?;
        if installed_id != TRAE_BUNDLE_ID {
            return Err(AgentReasonCode::SourceNotVerified);
        }
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn discover_single_app(mount: &Path) -> Result<PathBuf, AgentReasonCode> {
    let mut found = None;
    for entry in fs::read_dir(mount).map_err(|_| AgentReasonCode::ExecutorNotImplemented)? {
        let entry = entry.map_err(|_| AgentReasonCode::ExecutorNotImplemented)?;
        let path = entry.path();
        let meta =
            fs::symlink_metadata(&path).map_err(|_| AgentReasonCode::ExecutorNotImplemented)?;
        if path.extension().and_then(|ext| ext.to_str()) != Some("app") {
            continue;
        }
        if meta.file_type().is_symlink() || !meta.is_dir() {
            return Err(AgentReasonCode::SourceNotVerified);
        }
        if found.is_some() {
            return Err(AgentReasonCode::SourceNotVerified);
        }
        found = Some(path);
    }
    found.ok_or(AgentReasonCode::SourceNotVerified)
}

fn user_applications_dir() -> Result<PathBuf, AgentReasonCode> {
    Ok(get_home_dir().join("Applications"))
}

fn applications_roots() -> [PathBuf; 2] {
    [
        user_applications_dir().unwrap_or_else(|_| PathBuf::from("/Applications")),
        PathBuf::from("/Applications"),
    ]
}

pub fn observe_desktop(agent_id: AgentCatalogId) -> DesktopObservation {
    match agent_id {
        AgentCatalogId::TraeWork => observe_trae_bundle(),
        AgentCatalogId::QoderWork | AgentCatalogId::WorkBuddy => DesktopObservation {
            installed: false,
            local_version: None,
        },
        _ => DesktopObservation {
            installed: false,
            local_version: None,
        },
    }
}

fn is_regular_app_bundle(path: &Path) -> bool {
    if path.extension().and_then(|ext| ext.to_str()) != Some("app") {
        return false;
    }
    fs::symlink_metadata(path)
        .map(|meta| !meta.file_type().is_symlink() && meta.is_dir())
        .unwrap_or(false)
}

fn observe_trae_bundle() -> DesktopObservation {
    for root in applications_roots() {
        let Ok(entries) = fs::read_dir(&root) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !is_regular_app_bundle(&path) {
                continue;
            }
            if read_bundle_id(&path).as_deref() == Some(TRAE_BUNDLE_ID) {
                return DesktopObservation {
                    installed: true,
                    local_version: read_bundle_version(&path),
                };
            }
        }
    }
    DesktopObservation {
        installed: false,
        local_version: None,
    }
}

pub fn launch_trae_if_present() -> Result<(), AgentReasonCode> {
    for root in applications_roots() {
        let Ok(entries) = fs::read_dir(&root) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !is_regular_app_bundle(&path) {
                continue;
            }
            if read_bundle_id(&path).as_deref() != Some(TRAE_BUNDLE_ID) {
                continue;
            }
            if !path.starts_with(&root) {
                return Err(AgentReasonCode::SourceNotVerified);
            }
            #[cfg(target_os = "macos")]
            {
                let status = Command::new("open")
                    .arg(&path)
                    .status()
                    .map_err(|_| AgentReasonCode::InteractiveUserUnavailable)?;
                return if status.success() {
                    Ok(())
                } else {
                    Err(AgentReasonCode::InteractiveUserUnavailable)
                };
            }
            #[cfg(not(target_os = "macos"))]
            {
                return Err(AgentReasonCode::PlatformUnsupported);
            }
        }
    }
    Err(AgentReasonCode::InstalledNotRunnable)
}

fn read_bundle_id(app: &Path) -> Option<String> {
    plist_string(app, "CFBundleIdentifier")
}

fn read_bundle_version(app: &Path) -> Option<String> {
    plist_string(app, "CFBundleShortVersionString").or_else(|| plist_string(app, "CFBundleVersion"))
}

fn plist_string(app: &Path, key: &str) -> Option<String> {
    let bytes = fs::read(app.join("Contents/Info.plist")).ok()?;
    if bytes.len() > MAX_PLIST_BYTES {
        return None;
    }
    let text = String::from_utf8_lossy(&bytes);
    let needle = format!("<key>{key}</key>");
    let start = text.find(&needle)? + needle.len();
    let rest = text[start..].trim_start();
    let rest = rest.strip_prefix("<string>")?;
    let end = rest.find("</string>")?;
    let value = rest[..end].trim();
    if value.is_empty() || value.len() > 128 {
        return None;
    }
    Some(value.to_string())
}

fn parse_mount_point(plist: &[u8]) -> Option<PathBuf> {
    let text = String::from_utf8_lossy(plist);
    let key = "<key>mount-point</key>";
    let start = text.find(key)? + key.len();
    let rest = text[start..].trim_start();
    let rest = rest.strip_prefix("<string>")?;
    let end = rest.find("</string>")?;
    let value = rest[..end].trim();
    if value.starts_with('/') {
        Some(PathBuf::from(value))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_exe_is_not_an_installable_format_here() {
        let source = ResolvedDesktopSource {
            product: AgentCatalogId::QoderWork,
            platform: AgentPlatform::Windows,
            architecture: AgentArch::X86_64,
            format: PackageFormat::Exe,
            release_id: "v1:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                .to_string(),
            display_version: None,
            download_url: url::Url::parse(
                "https://static.qoder.com.cn/qoder-work-cn/releases/latest/QoderWorkCN-Setup-User-x64.exe",
            )
            .unwrap(),
            versionless_latest: true,
            official_page: "https://qoder.com.cn/download",
        };
        assert!(windows_exe_unavailable(&source));
    }

    #[tokio::test]
    async fn windows_exe_install_is_not_started() {
        let source = ResolvedDesktopSource {
            product: AgentCatalogId::QoderWork,
            platform: AgentPlatform::Windows,
            architecture: AgentArch::X86_64,
            format: PackageFormat::Exe,
            release_id: "v1:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                .to_string(),
            display_version: None,
            download_url: url::Url::parse(
                "https://static.qoder.com.cn/qoder-work-cn/releases/latest/QoderWorkCN-Setup-User-x64.exe",
            )
            .unwrap(),
            versionless_latest: true,
            official_page: "https://qoder.com.cn/download",
        };
        assert_eq!(
            install_resolved_source(&source, &crate::codex_desktop::cancellation::NeverCancelled,)
                .await,
            Err(AgentReasonCode::InteractiveUserUnavailable)
        );
    }

    #[test]
    fn mount_plist_requires_an_absolute_path() {
        let plist =
            br#"<plist><dict><key>mount-point</key><string>/Volumes/App</string></dict></plist>"#;
        assert_eq!(
            parse_mount_point(plist),
            Some(PathBuf::from("/Volumes/App"))
        );
        assert_eq!(parse_mount_point(b"<plist></plist>"), None);
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
}
