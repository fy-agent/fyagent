//! Official OpenCode Desktop source. Artifact URLs stay on the locale-neutral
//! stable aliases, which are versionless: the alias target can move to a new
//! product line (it now serves the v2 desktop) independently of the GitHub
//! Releases line (still 1.x). GitHub latest therefore is not a truthful
//! display or expected version for the downloaded artifact and is not used;
//! the installed bundle reports its own version after install. FyAgent does
//! not invoke OpenCode's Electron updater.
//!
//! Windows ARM64 has no stable alias. Its versioned official file is derived
//! from the x64 alias redirect (`/files/bin/<x.y.z>/opencode-desktop-win-x64.exe`)
//! and bound to that exact version.

use url::Url;

#[cfg(test)]
use super::bounded_version;
use super::{
    https_url_on_allowlist, opaque_release_id, AgentArch, AgentPlatform, PackageFormat,
    ResolvedDesktopSource, SourceResolveError,
};
use crate::services::external_agents::AgentCatalogId;
#[cfg(test)]
use crate::services::tooling;

pub const OPENCODE_DOWNLOAD_HOSTS: &[&str] = &[
    "opencode.ai",
    "www.opencode.ai",
    "github.com",
    "objects.githubusercontent.com",
    "release-assets.githubusercontent.com",
    "github-releases.githubusercontent.com",
];
pub const OPENCODE_OFFICIAL_PAGE: &str = "https://opencode.ai/download";
pub const OPENCODE_DARWIN_AARCH64_DMG: &str =
    "https://opencode.ai/download/stable/darwin-aarch64-dmg";
pub const OPENCODE_DARWIN_X64_DMG: &str = "https://opencode.ai/download/stable/darwin-x64-dmg";
pub const OPENCODE_WINDOWS_X64_NSIS: &str = "https://opencode.ai/download/stable/windows-x64-nsis";
const OPENCODE_FILES_ORIGIN: &str = "https://opencode.ai";
const OPENCODE_FILES_PREFIX: &str = "/files/bin/";
const OPENCODE_WINDOWS_X64_FILE: &str = "opencode-desktop-win-x64.exe";
const OPENCODE_WINDOWS_ARM64_FILE: &str = "opencode-desktop-win-arm64.exe";
const MAX_OPENCODE_VERSION_SEGMENT_DIGITS: usize = 6;

#[cfg(test)]
pub fn resolve_opencode_desktop(
    platform: AgentPlatform,
    architecture: AgentArch,
) -> Result<ResolvedDesktopSource, SourceResolveError> {
    resolve_opencode_desktop_inner(platform, architecture, None)
}

#[cfg(test)]
pub fn resolve_opencode_desktop_with_version(
    platform: AgentPlatform,
    architecture: AgentArch,
    display_version: &str,
) -> Result<ResolvedDesktopSource, SourceResolveError> {
    let version = bounded_version(display_version).ok_or(SourceResolveError::SchemaInvalid)?;
    resolve_opencode_desktop_inner(platform, architecture, Some(version))
}

pub async fn resolve_opencode_desktop_latest(
    platform: AgentPlatform,
    architecture: AgentArch,
) -> Result<ResolvedDesktopSource, SourceResolveError> {
    // The stable alias is versionless. A display version taken from GitHub
    // latest would also become the macOS DMG expected release version and
    // reject the real alias artifact whenever the two lines diverge.
    resolve_opencode_desktop_inner(platform, architecture, None)
}

/// Reads the exact release version from the x64 stable alias redirect target.
/// Only `https://opencode.ai/files/bin/<x.y.z>/opencode-desktop-win-x64.exe`
/// without query or fragment is accepted.
fn opencode_version_from_x64_redirect(location: &Url) -> Option<&str> {
    if location.scheme() != "https"
        || !matches!(location.host_str(), Some("opencode.ai" | "www.opencode.ai"))
        || location.port().is_some()
        || !location.username().is_empty()
        || location.password().is_some()
        || location.query().is_some()
        || location.fragment().is_some()
    {
        return None;
    }
    let (version, file) = location
        .path()
        .strip_prefix(OPENCODE_FILES_PREFIX)?
        .split_once('/')?;
    if file != OPENCODE_WINDOWS_X64_FILE {
        return None;
    }
    let segments: Vec<&str> = version.split('.').collect();
    let numeric = segments.len() == 3
        && segments.iter().all(|segment| {
            !segment.is_empty()
                && segment.len() <= MAX_OPENCODE_VERSION_SEGMENT_DIGITS
                && segment.bytes().all(|byte| byte.is_ascii_digit())
        });
    numeric.then_some(version)
}

/// Builds the versioned official Windows ARM64 installer source from the
/// x64 alias redirect. The version is part of the release ID, so a later
/// alias move requires a fresh confirmation.
pub fn resolve_opencode_windows_arm64_from_x64_redirect(
    location: &Url,
) -> Result<ResolvedDesktopSource, SourceResolveError> {
    let version =
        opencode_version_from_x64_redirect(location).ok_or(SourceResolveError::SchemaInvalid)?;
    let download_url = Url::parse(&format!(
        "{OPENCODE_FILES_ORIGIN}{OPENCODE_FILES_PREFIX}{version}/{OPENCODE_WINDOWS_ARM64_FILE}"
    ))
    .map_err(|_| SourceResolveError::SchemaInvalid)?;
    https_url_on_allowlist(&download_url, OPENCODE_DOWNLOAD_HOSTS)?;
    let (platform, architecture) = (AgentPlatform::Windows, AgentArch::Aarch64);
    if ambiguous_or_missing_arch_token(download_url.path(), platform, architecture) {
        return Err(SourceResolveError::ArtifactRejected);
    }
    let format = PackageFormat::Exe;
    let fields = [
        ("product", "opencode"),
        ("surface", "desktop"),
        ("platform", platform.as_str()),
        ("architecture", architecture.as_str()),
        ("format", format.as_str()),
        ("version", version),
        ("endpoint", "opencode-windows-arm64-nsis"),
    ];
    Ok(ResolvedDesktopSource {
        product: AgentCatalogId::OpenCode,
        platform,
        architecture,
        format,
        release_id: opaque_release_id(&fields),
        display_version: Some(version.to_string()),
        artifact_size_bytes: None,
        download_url,
        versionless_latest: false,
        official_page: OPENCODE_OFFICIAL_PAGE,
    })
}

fn resolve_opencode_desktop_inner(
    platform: AgentPlatform,
    architecture: AgentArch,
    display_version: Option<&str>,
) -> Result<ResolvedDesktopSource, SourceResolveError> {
    let (endpoint_kind, url, format) = match (platform, architecture) {
        (AgentPlatform::Macos, AgentArch::Aarch64) => (
            "opencode-darwin-aarch64-dmg",
            OPENCODE_DARWIN_AARCH64_DMG,
            PackageFormat::Dmg,
        ),
        (AgentPlatform::Macos, AgentArch::X86_64) => (
            "opencode-darwin-x64-dmg",
            OPENCODE_DARWIN_X64_DMG,
            PackageFormat::Dmg,
        ),
        (AgentPlatform::Windows, AgentArch::X86_64) => (
            "opencode-windows-x64-nsis",
            OPENCODE_WINDOWS_X64_NSIS,
            PackageFormat::Exe,
        ),
        // No versionless ARM64 alias exists; see
        // `resolve_opencode_windows_arm64_from_x64_redirect`.
        (AgentPlatform::Windows, AgentArch::Aarch64) => {
            return Err(SourceResolveError::PlatformUnsupported)
        }
    };
    let download_url = Url::parse(url).map_err(|_| SourceResolveError::SchemaInvalid)?;
    https_url_on_allowlist(&download_url, OPENCODE_DOWNLOAD_HOSTS)?;
    if ambiguous_or_missing_arch_token(download_url.path(), platform, architecture) {
        return Err(SourceResolveError::ArtifactRejected);
    }

    let fields = [
        ("product", "opencode"),
        ("surface", "desktop"),
        ("platform", platform.as_str()),
        ("architecture", architecture.as_str()),
        ("format", format.as_str()),
        ("alias", "stable"),
        ("endpoint", endpoint_kind),
    ];

    Ok(ResolvedDesktopSource {
        product: AgentCatalogId::OpenCode,
        platform,
        architecture,
        format,
        release_id: opaque_release_id(&fields),
        display_version: display_version.map(str::to_string),
        artifact_size_bytes: None,
        download_url,
        versionless_latest: true,
        official_page: OPENCODE_OFFICIAL_PAGE,
    })
}

fn ambiguous_or_missing_arch_token(
    path: &str,
    platform: AgentPlatform,
    architecture: AgentArch,
) -> bool {
    match (platform, architecture) {
        (AgentPlatform::Macos, AgentArch::Aarch64) => {
            !path.ends_with("darwin-aarch64-dmg")
                || path.contains("darwin-x64-dmg")
                || path.contains("windows-")
                || path.contains("/zh/")
        }
        (AgentPlatform::Macos, AgentArch::X86_64) => {
            !path.ends_with("darwin-x64-dmg")
                || path.contains("darwin-aarch64-dmg")
                || path.contains("windows-")
                || path.contains("/zh/")
        }
        (AgentPlatform::Windows, AgentArch::X86_64) => {
            !path.ends_with("windows-x64-nsis")
                || path.contains("darwin-")
                || path.contains("aarch64")
                || path.contains("arm64")
                || path.contains("/zh/")
        }
        (AgentPlatform::Windows, AgentArch::Aarch64) => {
            !path.ends_with("-win-arm64.exe")
                || path.contains("x64")
                || path.contains("darwin")
                || path.contains("/zh/")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opencode_desktop_maps_each_supported_host_to_exactly_one_official_alias() {
        let arm = resolve_opencode_desktop(AgentPlatform::Macos, AgentArch::Aarch64).unwrap();
        assert_eq!(arm.format, PackageFormat::Dmg);
        assert!(arm.versionless_latest);
        assert!(arm.display_version.is_none());
        assert_eq!(arm.download_url.as_str(), OPENCODE_DARWIN_AARCH64_DMG);
        assert!(arm.download_url.path().ends_with("darwin-aarch64-dmg"));
        assert!(!arm.download_url.as_str().contains("/zh/"));
        assert!(!arm.download_url.as_str().contains("1.18.19"));

        let intel = resolve_opencode_desktop(AgentPlatform::Macos, AgentArch::X86_64).unwrap();
        assert_eq!(intel.download_url.as_str(), OPENCODE_DARWIN_X64_DMG);
        assert_eq!(intel.format, PackageFormat::Dmg);
        assert!(intel.download_url.path().ends_with("darwin-x64-dmg"));
        assert_ne!(arm.release_id, intel.release_id);
        assert!(arm.release_id.starts_with("v1:"));
        assert!(!arm.release_id.contains("http"));

        let windows = resolve_opencode_desktop(AgentPlatform::Windows, AgentArch::X86_64).unwrap();
        assert_eq!(windows.format, PackageFormat::Exe);
        assert_eq!(windows.platform, AgentPlatform::Windows);
        assert_eq!(windows.architecture, AgentArch::X86_64);
        assert!(windows.versionless_latest);
        assert!(windows.display_version.is_none());
        assert_eq!(windows.download_url.as_str(), OPENCODE_WINDOWS_X64_NSIS);
        assert!(windows.download_url.path().ends_with("windows-x64-nsis"));
        assert!(!windows.download_url.as_str().contains("/zh/"));
        assert_ne!(windows.release_id, arm.release_id);
        assert_ne!(windows.release_id, intel.release_id);
        assert!(!windows.release_id.contains("http"));
    }

    #[test]
    fn frozen_github_latest_version_binds_display_version_not_artifact_url() {
        let arm = resolve_opencode_desktop_with_version(
            AgentPlatform::Macos,
            AgentArch::Aarch64,
            "1.2.3",
        )
        .unwrap();
        assert!(arm.versionless_latest);
        assert_eq!(arm.display_version.as_deref(), Some("1.2.3"));
        assert_eq!(arm.download_url.as_str(), OPENCODE_DARWIN_AARCH64_DMG);
        assert!(!arm.release_id.contains("http"));
        assert!(!arm.release_id.contains("1.2.3"));

        let versionless =
            resolve_opencode_desktop(AgentPlatform::Macos, AgentArch::Aarch64).unwrap();
        assert_eq!(arm.release_id, versionless.release_id);

        let windows = resolve_opencode_desktop_with_version(
            AgentPlatform::Windows,
            AgentArch::X86_64,
            "1.2.3",
        )
        .unwrap();
        assert_eq!(windows.format, PackageFormat::Exe);
        assert!(windows.versionless_latest);
        assert_eq!(windows.display_version.as_deref(), Some("1.2.3"));
        assert_eq!(windows.download_url.as_str(), OPENCODE_WINDOWS_X64_NSIS);
        assert_eq!(
            windows.release_id,
            resolve_opencode_desktop(AgentPlatform::Windows, AgentArch::X86_64)
                .unwrap()
                .release_id
        );

        assert_eq!(
            resolve_opencode_desktop_with_version(
                AgentPlatform::Macos,
                AgentArch::Aarch64,
                "1.2.3-beta"
            ),
            Err(SourceResolveError::SchemaInvalid)
        );
        assert_eq!(
            resolve_opencode_desktop_with_version(
                AgentPlatform::Windows,
                AgentArch::Aarch64,
                "1.2.3"
            ),
            Err(SourceResolveError::PlatformUnsupported)
        );
    }

    #[test]
    fn github_latest_failure_still_resolves_installable_stable_source() {
        for (platform, architecture, url, format) in [
            (
                AgentPlatform::Macos,
                AgentArch::Aarch64,
                OPENCODE_DARWIN_AARCH64_DMG,
                PackageFormat::Dmg,
            ),
            (
                AgentPlatform::Macos,
                AgentArch::X86_64,
                OPENCODE_DARWIN_X64_DMG,
                PackageFormat::Dmg,
            ),
            (
                AgentPlatform::Windows,
                AgentArch::X86_64,
                OPENCODE_WINDOWS_X64_NSIS,
                PackageFormat::Exe,
            ),
        ] {
            let source = resolve_opencode_desktop_inner(platform, architecture, None)
                .expect("stable alias must remain installable without GitHub");
            assert_eq!(source.download_url.as_str(), url);
            assert_eq!(source.format, format);
            assert!(source.versionless_latest);
            assert!(source.display_version.is_none());
            assert!(source.release_id.starts_with("v1:"));
        }
    }

    #[tokio::test]
    async fn latest_resolution_is_versionless_and_never_consults_github() {
        for (platform, architecture) in [
            (AgentPlatform::Macos, AgentArch::Aarch64),
            (AgentPlatform::Macos, AgentArch::X86_64),
            (AgentPlatform::Windows, AgentArch::X86_64),
        ] {
            let latest = resolve_opencode_desktop_latest(platform, architecture)
                .await
                .unwrap();
            assert!(latest.versionless_latest);
            // No display version means no macOS expected-release gate can
            // reject the alias artifact on a GitHub/alias line mismatch.
            assert!(latest.display_version.is_none());
            assert_eq!(
                latest,
                resolve_opencode_desktop(platform, architecture).unwrap()
            );
        }
    }

    #[test]
    fn github_latest_parser_rejects_draft_prerelease_and_foreign_repos() {
        let stable = br#"{"tag_name":"v1.2.3","draft":false,"prerelease":false}"#;
        assert_eq!(
            tooling::parse_github_latest_release_tag(stable).as_deref(),
            Some("1.2.3")
        );
        assert_eq!(
            tooling::parse_github_latest_release_tag(
                br#"{"tag_name":"v1.2.3","draft":true,"prerelease":false}"#
            ),
            None
        );
        assert_eq!(
            tooling::parse_github_latest_release_tag(
                br#"{"tag_name":"v1.2.3","draft":false,"prerelease":true}"#
            ),
            None
        );
        assert_eq!(tooling::FIXED_GITHUB_OPENCODE_REPO, "anomalyco/opencode");
        assert!(tooling::github_latest_release_url("anomalyco/opencode").is_some());
        assert_eq!(tooling::github_latest_release_url("evil/other"), None);
        assert!(!format!("{stable:?}").contains("electron-updater"));
    }

    #[test]
    fn opencode_desktop_fails_closed_for_unsupported_arch_and_rejects_cross_arch_paths() {
        assert_eq!(
            resolve_opencode_desktop(AgentPlatform::Windows, AgentArch::Aarch64),
            Err(SourceResolveError::PlatformUnsupported)
        );
        assert!(ambiguous_or_missing_arch_token(
            "/download/stable/darwin-x64-dmg",
            AgentPlatform::Macos,
            AgentArch::Aarch64
        ));
        assert!(ambiguous_or_missing_arch_token(
            "/download/stable/darwin-aarch64-dmg",
            AgentPlatform::Macos,
            AgentArch::X86_64
        ));
        assert!(ambiguous_or_missing_arch_token(
            "/download/stable/darwin-aarch64-dmg-and-darwin-x64-dmg",
            AgentPlatform::Macos,
            AgentArch::Aarch64
        ));
        assert!(!ambiguous_or_missing_arch_token(
            "/download/stable/darwin-aarch64-dmg",
            AgentPlatform::Macos,
            AgentArch::Aarch64
        ));
        assert!(ambiguous_or_missing_arch_token(
            "/download/stable/windows-x64-nsis",
            AgentPlatform::Macos,
            AgentArch::Aarch64
        ));
        assert!(ambiguous_or_missing_arch_token(
            "/download/stable/darwin-x64-dmg",
            AgentPlatform::Windows,
            AgentArch::X86_64
        ));
        assert!(ambiguous_or_missing_arch_token(
            "/zh/download/stable/windows-x64-nsis",
            AgentPlatform::Windows,
            AgentArch::X86_64
        ));
        assert!(!ambiguous_or_missing_arch_token(
            "/download/stable/windows-x64-nsis",
            AgentPlatform::Windows,
            AgentArch::X86_64
        ));
        assert!(ambiguous_or_missing_arch_token(
            "/download/stable/windows-x64-nsis",
            AgentPlatform::Windows,
            AgentArch::Aarch64
        ));
        assert!(!ambiguous_or_missing_arch_token(
            "/files/bin/2.0.20/opencode-desktop-win-arm64.exe",
            AgentPlatform::Windows,
            AgentArch::Aarch64
        ));
        for rejected in [
            "/files/bin/2.0.20/opencode-desktop-win-x64.exe",
            "/files/bin/2.0.20/opencode-desktop-win-x64-win-arm64.exe",
            "/files/bin/2.0.20/opencode-desktop-darwin-win-arm64.exe",
            "/zh/files/bin/2.0.20/opencode-desktop-win-arm64.exe",
        ] {
            assert!(ambiguous_or_missing_arch_token(
                rejected,
                AgentPlatform::Windows,
                AgentArch::Aarch64
            ));
        }
    }

    #[test]
    fn windows_arm64_source_is_versioned_from_the_exact_x64_redirect() {
        let location =
            Url::parse("https://opencode.ai/files/bin/2.0.20/opencode-desktop-win-x64.exe")
                .unwrap();
        let arm = resolve_opencode_windows_arm64_from_x64_redirect(&location).unwrap();
        assert_eq!(
            arm.download_url.as_str(),
            "https://opencode.ai/files/bin/2.0.20/opencode-desktop-win-arm64.exe"
        );
        assert_eq!(arm.platform, AgentPlatform::Windows);
        assert_eq!(arm.architecture, AgentArch::Aarch64);
        assert_eq!(arm.format, PackageFormat::Exe);
        assert_eq!(arm.display_version.as_deref(), Some("2.0.20"));
        assert!(!arm.versionless_latest);
        assert!(arm.release_id.starts_with("v1:"));

        let next = resolve_opencode_windows_arm64_from_x64_redirect(
            &Url::parse("https://opencode.ai/files/bin/2.0.21/opencode-desktop-win-x64.exe")
                .unwrap(),
        )
        .unwrap();
        assert_ne!(arm.release_id, next.release_id);
        let x64 = resolve_opencode_desktop(AgentPlatform::Windows, AgentArch::X86_64).unwrap();
        assert_ne!(arm.release_id, x64.release_id);
    }

    #[test]
    fn windows_arm64_rejects_any_other_x64_redirect_shape() {
        for rejected in [
            "http://opencode.ai/files/bin/2.0.20/opencode-desktop-win-x64.exe",
            "https://evil.example/files/bin/2.0.20/opencode-desktop-win-x64.exe",
            "https://github.com/files/bin/2.0.20/opencode-desktop-win-x64.exe",
            "https://opencode.ai:8443/files/bin/2.0.20/opencode-desktop-win-x64.exe",
            "https://user@opencode.ai/files/bin/2.0.20/opencode-desktop-win-x64.exe",
            "https://opencode.ai/files/bin/2.0.20/opencode-desktop-win-x64.exe?x=1",
            "https://opencode.ai/files/bin/2.0.20/opencode-desktop-win-x64.exe#a",
            "https://opencode.ai/files/bin/2.0/opencode-desktop-win-x64.exe",
            "https://opencode.ai/files/bin/2.0.20.1/opencode-desktop-win-x64.exe",
            "https://opencode.ai/files/bin/2.0.20-beta/opencode-desktop-win-x64.exe",
            "https://opencode.ai/files/bin/v2.0.20/opencode-desktop-win-x64.exe",
            "https://opencode.ai/files/bin/1234567.0.0/opencode-desktop-win-x64.exe",
            "https://opencode.ai/files/bin/2.0.20/opencode-desktop-win-arm64.exe",
            "https://opencode.ai/files/bin/2.0.20/sub/opencode-desktop-win-x64.exe",
            "https://opencode.ai/files/bin/2.0.20/opencode-desktop-darwin-x64.dmg",
            "https://opencode.ai/download/stable/windows-x64-nsis",
        ] {
            let location = Url::parse(rejected).unwrap();
            assert_eq!(
                resolve_opencode_windows_arm64_from_x64_redirect(&location),
                Err(SourceResolveError::SchemaInvalid),
                "{rejected}"
            );
        }
    }

    #[test]
    fn opencode_desktop_hosts_are_official_only() {
        let url = Url::parse(OPENCODE_DARWIN_AARCH64_DMG).unwrap();
        assert!(https_url_on_allowlist(&url, OPENCODE_DOWNLOAD_HOSTS).is_ok());
        let windows = Url::parse(OPENCODE_WINDOWS_X64_NSIS).unwrap();
        assert!(https_url_on_allowlist(&windows, OPENCODE_DOWNLOAD_HOSTS).is_ok());
        let proxy =
            Url::parse("https://ghproxy.example/opencode.ai/download/stable/darwin-aarch64-dmg")
                .unwrap();
        assert_eq!(
            https_url_on_allowlist(&proxy, OPENCODE_DOWNLOAD_HOSTS),
            Err(SourceResolveError::HostRejected)
        );
        let github = Url::parse(
            "https://github.com/anomalyco/opencode/releases/download/v9.9.9/OpenCode.dmg",
        )
        .unwrap();
        assert!(https_url_on_allowlist(&github, OPENCODE_DOWNLOAD_HOSTS).is_ok());
    }
}
