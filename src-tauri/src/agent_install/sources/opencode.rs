//! Official OpenCode Desktop source. Artifact URLs stay on the locale-neutral
//! stable aliases; GitHub latest is display-only enrichment and must not gate
//! installability. FyAgent does not invoke OpenCode's Electron updater.

use url::Url;

use super::{
    bounded_version, https_url_on_allowlist, opaque_release_id, AgentArch, AgentPlatform,
    PackageFormat, ResolvedDesktopSource, SourceResolveError,
};
use crate::services::external_agents::AgentCatalogId;
use crate::services::tooling::{self, FIXED_GITHUB_OPENCODE_REPO};

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
    // Construct the stable alias first so GitHub reachability cannot change
    // installability or the opaque release capability.
    let mut source = resolve_opencode_desktop_inner(platform, architecture, None)?;
    let client = crate::proxy::http_client::get();
    if let Some(tag) =
        tooling::fetch_github_latest_version(&client, FIXED_GITHUB_OPENCODE_REPO).await
    {
        if let Some(version) = bounded_version(&tag) {
            source.display_version = Some(version.to_string());
        }
    }
    Ok(source)
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
        (AgentPlatform::Windows, AgentArch::Aarch64) => true,
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
