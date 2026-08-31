//! Official OpenCode Desktop DMG source. Version is not frozen from research
//! constants; the stable architecture-specific endpoints are versionless
//! latest aliases owned by OpenCode.

use url::Url;

use super::{
    https_url_on_allowlist, opaque_release_id, AgentArch, AgentPlatform, PackageFormat,
    ResolvedDesktopSource, SourceResolveError,
};
use crate::services::external_agents::AgentCatalogId;

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

pub fn resolve_opencode_desktop(
    platform: AgentPlatform,
    architecture: AgentArch,
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

    Ok(ResolvedDesktopSource {
        product: AgentCatalogId::OpenCode,
        platform,
        architecture,
        format: PackageFormat::Dmg,
        release_id: opaque_release_id(&[
            ("product", "opencode"),
            ("surface", "desktop"),
            ("platform", platform.as_str()),
            ("architecture", architecture.as_str()),
            ("format", PackageFormat::Dmg.as_str()),
            ("alias", "stable"),
            ("endpoint", endpoint_kind),
        ]),
        display_version: None,
        download_url,
        versionless_latest: true,
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
