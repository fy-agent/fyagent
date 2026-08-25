use url::Url;

use super::{
    https_url_on_allowlist, opaque_release_id, AgentArch, AgentPlatform, PackageFormat,
    ResolvedDesktopSource, SourceResolveError,
};
use crate::services::external_agents::AgentCatalogId;

pub const QODERWORK_REDIRECT_HOSTS: &[&str] = &["static.qoder.com.cn"];
pub const QODERWORK_OFFICIAL_PAGE: &str = "https://qoder.com.cn/download";

const WINDOWS_USER_X64: &str =
    "https://static.qoder.com.cn/qoder-work-cn/releases/latest/QoderWorkCN-Setup-User-x64.exe";
const MACOS_ARM64: &str =
    "https://static.qoder.com.cn/qoder-work-cn/releases/latest/QoderWorkCN-arm64.dmg";
const MACOS_X64: &str =
    "https://static.qoder.com.cn/qoder-work-cn/releases/latest/QoderWorkCN-x64.dmg";

pub fn qoderwork_official_page() -> &'static str {
    QODERWORK_OFFICIAL_PAGE
}

pub fn resolve_qoderwork_source(
    platform: AgentPlatform,
    architecture: AgentArch,
) -> Result<ResolvedDesktopSource, SourceResolveError> {
    let (endpoint_kind, url, format) = match (platform, architecture) {
        (AgentPlatform::Windows, AgentArch::X86_64) => (
            "qoderwork-cn-win-user-x64",
            WINDOWS_USER_X64,
            PackageFormat::Exe,
        ),
        (AgentPlatform::Macos, AgentArch::Aarch64) => {
            ("qoderwork-cn-darwin-arm64", MACOS_ARM64, PackageFormat::Dmg)
        }
        (AgentPlatform::Macos, AgentArch::X86_64) => {
            ("qoderwork-cn-darwin-x64", MACOS_X64, PackageFormat::Dmg)
        }
        (AgentPlatform::Windows, AgentArch::Aarch64) => {
            return Err(SourceResolveError::PlatformUnsupported)
        }
    };

    let download_url = Url::parse(url).map_err(|_| SourceResolveError::SchemaInvalid)?;
    https_url_on_allowlist(&download_url, QODERWORK_REDIRECT_HOSTS)?;

    Ok(ResolvedDesktopSource {
        product: AgentCatalogId::QoderWork,
        platform,
        architecture,
        format,
        release_id: opaque_release_id(&[
            ("product", "qoderwork"),
            ("platform", platform.as_str()),
            ("architecture", architecture.as_str()),
            ("format", format.as_str()),
            ("alias", "latest"),
            ("endpoint", endpoint_kind),
        ]),
        display_version: None,
        download_url,
        versionless_latest: true,
        official_page: qoderwork_official_page(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qoderwork_selects_fixed_latest_aliases_without_inventing_semver() {
        let macos = resolve_qoderwork_source(AgentPlatform::Macos, AgentArch::Aarch64).unwrap();
        assert!(macos.versionless_latest);
        assert_eq!(macos.display_version, None);
        assert_eq!(macos.format, PackageFormat::Dmg);
        assert_eq!(macos.format.as_str(), "dmg");
        assert_eq!(macos.official_page, qoderwork_official_page());
        assert_eq!(macos.download_url.as_str(), MACOS_ARM64);
        assert_eq!(
            resolve_qoderwork_source(AgentPlatform::Macos, AgentArch::X86_64)
                .unwrap()
                .download_url
                .as_str(),
            MACOS_X64
        );
        assert_eq!(
            resolve_qoderwork_source(AgentPlatform::Windows, AgentArch::X86_64)
                .unwrap()
                .download_url
                .as_str(),
            WINDOWS_USER_X64
        );
        assert_eq!(
            resolve_qoderwork_source(AgentPlatform::Windows, AgentArch::Aarch64),
            Err(SourceResolveError::PlatformUnsupported)
        );
    }

    #[test]
    fn qoderwork_release_id_is_stable_for_the_alias_not_http_validators() {
        let first = resolve_qoderwork_source(AgentPlatform::Macos, AgentArch::Aarch64).unwrap();
        let second = resolve_qoderwork_source(AgentPlatform::Macos, AgentArch::Aarch64).unwrap();
        assert_eq!(first.release_id, second.release_id);
        assert!(first.release_id.starts_with("v1:"));
        assert_ne!(
            first.release_id,
            resolve_qoderwork_source(AgentPlatform::Macos, AgentArch::X86_64)
                .unwrap()
                .release_id
        );
        let serialized = serde_json::to_string(&first.release_id).unwrap();
        assert!(!serialized.contains("http"));
        assert!(!serialized.contains("static.qoder"));
    }
}
