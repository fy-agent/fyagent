//! Official OpenCode Desktop DMG source. Artifact URLs stay on the fixed
//! stable aliases; latest tag/version is frozen from the shared GitHub owner.
//! FyAgent does not invoke OpenCode's Electron updater.

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

#[cfg(test)]
pub fn resolve_opencode_desktop(
    platform: AgentPlatform,
    architecture: AgentArch,
) -> Result<ResolvedDesktopSource, SourceResolveError> {
    resolve_opencode_desktop_inner(platform, architecture, None)
}

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
    let client = crate::proxy::http_client::get();
    let version = tooling::fetch_github_latest_version(&client, FIXED_GITHUB_OPENCODE_REPO)
        .await
        .ok_or(SourceResolveError::SchemaInvalid)?;
    resolve_opencode_desktop_with_version(platform, architecture, &version)
}

fn resolve_opencode_desktop_inner(
    platform: AgentPlatform,
    architecture: AgentArch,
    display_version: Option<&str>,
) -> Result<ResolvedDesktopSource, SourceResolveError> {
    let (endpoint_kind, url) = match (platform, architecture) {
        (AgentPlatform::Macos, AgentArch::Aarch64) => {
            ("opencode-darwin-aarch64-dmg", OPENCODE_DARWIN_AARCH64_DMG)
        }
        (AgentPlatform::Macos, AgentArch::X86_64) => {
            ("opencode-darwin-x64-dmg", OPENCODE_DARWIN_X64_DMG)
        }
        _ => return Err(SourceResolveError::PlatformUnsupported),
    };
    let download_url = Url::parse(url).map_err(|_| SourceResolveError::SchemaInvalid)?;
    https_url_on_allowlist(&download_url, OPENCODE_DOWNLOAD_HOSTS)?;
    if ambiguous_or_missing_arch_token(download_url.path(), architecture) {
        return Err(SourceResolveError::ArtifactRejected);
    }

    let mut fields = vec![
        ("product", "opencode"),
        ("surface", "desktop"),
        ("platform", platform.as_str()),
        ("architecture", architecture.as_str()),
        ("format", PackageFormat::Dmg.as_str()),
        ("alias", "stable"),
        ("endpoint", endpoint_kind),
    ];
    if let Some(version) = display_version {
        fields.push(("version", version));
    }

    Ok(ResolvedDesktopSource {
        product: AgentCatalogId::OpenCode,
        platform,
        architecture,
        format: PackageFormat::Dmg,
        release_id: opaque_release_id(&fields),
        display_version: display_version.map(str::to_string),
        download_url,
        versionless_latest: display_version.is_none(),
        official_page: OPENCODE_OFFICIAL_PAGE,
    })
}

fn ambiguous_or_missing_arch_token(path: &str, architecture: AgentArch) -> bool {
    let (required, forbidden) = match architecture {
        AgentArch::Aarch64 => ("darwin-aarch64-dmg", "darwin-x64-dmg"),
        AgentArch::X86_64 => ("darwin-x64-dmg", "darwin-aarch64-dmg"),
    };
    !path.ends_with(required) || path.contains(forbidden)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opencode_desktop_maps_each_macos_arch_to_exactly_one_official_dmg() {
        let arm = resolve_opencode_desktop(AgentPlatform::Macos, AgentArch::Aarch64).unwrap();
        assert_eq!(arm.format, PackageFormat::Dmg);
        assert!(arm.versionless_latest);
        assert!(arm.display_version.is_none());
        assert_eq!(arm.download_url.as_str(), OPENCODE_DARWIN_AARCH64_DMG);
        assert!(arm.download_url.path().ends_with("darwin-aarch64-dmg"));
        assert!(!arm.download_url.as_str().contains("1.18.19"));

        let intel = resolve_opencode_desktop(AgentPlatform::Macos, AgentArch::X86_64).unwrap();
        assert_eq!(intel.download_url.as_str(), OPENCODE_DARWIN_X64_DMG);
        assert!(intel.download_url.path().ends_with("darwin-x64-dmg"));
        assert_ne!(arm.release_id, intel.release_id);
        assert!(arm.release_id.starts_with("v1:"));
        assert!(!arm.release_id.contains("http"));
    }

    #[test]
    fn frozen_github_latest_version_binds_display_version_not_artifact_url() {
        let arm = resolve_opencode_desktop_with_version(
            AgentPlatform::Macos,
            AgentArch::Aarch64,
            "1.2.3",
        )
        .unwrap();
        assert!(!arm.versionless_latest);
        assert_eq!(arm.display_version.as_deref(), Some("1.2.3"));
        assert_eq!(arm.download_url.as_str(), OPENCODE_DARWIN_AARCH64_DMG);
        assert!(!arm.release_id.contains("http"));
        assert!(!arm.release_id.contains("1.2.3"));

        let versionless =
            resolve_opencode_desktop(AgentPlatform::Macos, AgentArch::Aarch64).unwrap();
        assert_ne!(arm.release_id, versionless.release_id);

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
                AgentArch::X86_64,
                "1.2.3"
            ),
            Err(SourceResolveError::PlatformUnsupported)
        );
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
    fn opencode_desktop_fails_closed_off_macos_and_rejects_cross_arch_paths() {
        assert_eq!(
            resolve_opencode_desktop(AgentPlatform::Windows, AgentArch::X86_64),
            Err(SourceResolveError::PlatformUnsupported)
        );
        assert_eq!(
            resolve_opencode_desktop(AgentPlatform::Windows, AgentArch::Aarch64),
            Err(SourceResolveError::PlatformUnsupported)
        );
        assert!(ambiguous_or_missing_arch_token(
            "/download/stable/darwin-x64-dmg",
            AgentArch::Aarch64
        ));
        assert!(ambiguous_or_missing_arch_token(
            "/download/stable/darwin-aarch64-dmg",
            AgentArch::X86_64
        ));
        assert!(ambiguous_or_missing_arch_token(
            "/download/stable/darwin-aarch64-dmg-and-darwin-x64-dmg",
            AgentArch::Aarch64
        ));
        assert!(!ambiguous_or_missing_arch_token(
            "/download/stable/darwin-aarch64-dmg",
            AgentArch::Aarch64
        ));
    }

    #[test]
    fn opencode_desktop_hosts_are_official_only() {
        let url = Url::parse(OPENCODE_DARWIN_AARCH64_DMG).unwrap();
        assert!(https_url_on_allowlist(&url, OPENCODE_DOWNLOAD_HOSTS).is_ok());
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
