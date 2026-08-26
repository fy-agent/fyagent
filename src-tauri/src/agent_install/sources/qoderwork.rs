use url::Url;

use super::{
    bounded_version, https_url_on_allowlist, opaque_release_id, AgentArch, AgentPlatform,
    PackageFormat, ResolvedDesktopSource, SourceResolveError, MAX_SOURCE_METADATA_BYTES,
};
use crate::services::external_agents::AgentCatalogId;

pub const QODERWORK_REDIRECT_HOSTS: &[&str] = &["static.qoder.com.cn"];
pub const QODERWORK_METADATA_HOSTS: &[&str] = QODERWORK_REDIRECT_HOSTS;
pub const QODERWORK_OFFICIAL_PAGE: &str = "https://qoder.com.cn/download";

const WINDOWS_USER_X64: &str =
    "https://static.qoder.com.cn/qoder-work-cn/releases/latest/QoderWorkCN-Setup-User-x64.exe";
const MACOS_ARM64: &str =
    "https://static.qoder.com.cn/qoder-work-cn/releases/latest/QoderWorkCN-arm64.dmg";
const MACOS_X64: &str =
    "https://static.qoder.com.cn/qoder-work-cn/releases/latest/QoderWorkCN-x64.dmg";
const WINDOWS_LATEST_YML: &str = "https://static.qoder.com.cn/qoder-work-cn/releases/latest.yml";
const MACOS_LATEST_YML: &str = "https://static.qoder.com.cn/qoder-work-cn/releases/latest-mac.yml";

pub fn qoderwork_official_page() -> &'static str {
    QODERWORK_OFFICIAL_PAGE
}

pub fn qoderwork_latest_yml_url(
    platform: AgentPlatform,
    architecture: AgentArch,
) -> Result<Url, SourceResolveError> {
    let url = match (platform, architecture) {
        (AgentPlatform::Windows, AgentArch::X86_64) => WINDOWS_LATEST_YML,
        (AgentPlatform::Macos, AgentArch::Aarch64 | AgentArch::X86_64) => MACOS_LATEST_YML,
        (AgentPlatform::Windows, AgentArch::Aarch64) => {
            return Err(SourceResolveError::PlatformUnsupported)
        }
    };
    let parsed = Url::parse(url).map_err(|_| SourceResolveError::SchemaInvalid)?;
    https_url_on_allowlist(&parsed, QODERWORK_METADATA_HOSTS)?;
    Ok(parsed)
}

pub fn parse_qoderwork_latest(
    body: &[u8],
    platform: AgentPlatform,
    architecture: AgentArch,
) -> Result<ResolvedDesktopSource, SourceResolveError> {
    let version = parse_qoderwork_latest_version(body)?;
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
            ("version", version),
            ("endpoint", endpoint_kind),
        ]),
        display_version: Some(version.to_string()),
        download_url,
        versionless_latest: false,
        official_page: qoderwork_official_page(),
    })
}

fn parse_qoderwork_latest_version(body: &[u8]) -> Result<&str, SourceResolveError> {
    if body.is_empty() || body.len() > MAX_SOURCE_METADATA_BYTES {
        return Err(SourceResolveError::SchemaInvalid);
    }
    let text = std::str::from_utf8(body).map_err(|_| SourceResolveError::SchemaInvalid)?;
    if text.contains('\0') {
        return Err(SourceResolveError::SchemaInvalid);
    }
    let mut found = None;
    for line in text.lines() {
        if line.starts_with(' ') || line.starts_with('\t') {
            continue;
        }
        let Some(rest) = line.strip_prefix("version:") else {
            continue;
        };
        let value = rest.trim().trim_matches('"').trim_matches('\'');
        let version = bounded_version(value).ok_or(SourceResolveError::SchemaInvalid)?;
        if found.is_some() {
            return Err(SourceResolveError::SchemaInvalid);
        }
        found = Some(version);
    }
    found.ok_or(SourceResolveError::SchemaInvalid)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MACOS_YML: &str = "\
version: 0.9.15
files:
  - url: https://static.qoder.com.cn/qoder-work-cn/releases/latest/QoderWorkCN-arm64-mac.zip
    sha512: U7Ls2i+rQK9fguCSYcsPKlR0QDvq1/bq8aCowsqaVL9ScuwOwceUZArSOWKP4xNtuinumDsLaQObg9gipnw3IA==
    size: 243552125
path: QoderWorkCN-arm64-mac.zip
sha512: U7Ls2i+rQK9fguCSYcsPKlR0QDvq1/bq8aCowsqaVL9ScuwOwceUZArSOWKP4xNtuinumDsLaQObg9gipnw3IA==
releaseDate: 2026-08-21T09:12:40.149Z
";

    const WINDOWS_YML: &str = "\
version: 0.9.15
files:
  - url: https://static.qoder.com.cn/qoder-work-cn/releases/latest/QoderWorkCN-Setup-User-x64.exe
    sha512: w+ymKCuZbESvgvil0yQUxllDAxQ5mY8GPRgfgYVXfUs0H6r1ksJ+kmmndVQhOU2q1dVuPug8VhOZxZiSlyPlYA==
    size: 253565152
path: QoderWorkCN-Setup-User-x64.exe
sha512: w+ymKCuZbESvgvil0yQUxllDAxQ5mY8GPRgfgYVXfUs0H6r1ksJ+kmmndVQhOU2q1dVuPug8VhOZxZiSlyPlYA==
releaseDate: 2026-08-21T09:06:52.222Z
";

    #[test]
    fn qoderwork_keeps_archived_aliases_and_reads_yml_version_only() {
        let macos = parse_qoderwork_latest(
            MACOS_YML.as_bytes(),
            AgentPlatform::Macos,
            AgentArch::Aarch64,
        )
        .unwrap();
        assert!(!macos.versionless_latest);
        assert_eq!(macos.display_version.as_deref(), Some("0.9.15"));
        assert_eq!(macos.format, PackageFormat::Dmg);
        assert_eq!(macos.download_url.as_str(), MACOS_ARM64);
        assert!(!macos.download_url.as_str().ends_with(".zip"));
        assert_eq!(
            parse_qoderwork_latest(
                MACOS_YML.as_bytes(),
                AgentPlatform::Macos,
                AgentArch::X86_64
            )
            .unwrap()
            .download_url
            .as_str(),
            MACOS_X64
        );
        assert_eq!(
            parse_qoderwork_latest(
                WINDOWS_YML.as_bytes(),
                AgentPlatform::Windows,
                AgentArch::X86_64
            )
            .unwrap()
            .download_url
            .as_str(),
            WINDOWS_USER_X64
        );
        assert_eq!(
            parse_qoderwork_latest(
                WINDOWS_YML.as_bytes(),
                AgentPlatform::Windows,
                AgentArch::Aarch64
            ),
            Err(SourceResolveError::PlatformUnsupported)
        );
        assert_eq!(
            qoderwork_latest_yml_url(AgentPlatform::Windows, AgentArch::Aarch64),
            Err(SourceResolveError::PlatformUnsupported)
        );
    }

    #[test]
    fn qoderwork_release_id_tracks_yml_version_not_http_or_zip_locators() {
        let first = parse_qoderwork_latest(
            MACOS_YML.as_bytes(),
            AgentPlatform::Macos,
            AgentArch::Aarch64,
        )
        .unwrap();
        let second = parse_qoderwork_latest(
            MACOS_YML.as_bytes(),
            AgentPlatform::Macos,
            AgentArch::Aarch64,
        )
        .unwrap();
        assert_eq!(first.release_id, second.release_id);
        assert!(first.release_id.starts_with("v1:"));
        assert_ne!(
            first.release_id,
            parse_qoderwork_latest(
                MACOS_YML.as_bytes(),
                AgentPlatform::Macos,
                AgentArch::X86_64
            )
            .unwrap()
            .release_id
        );
        let bumped = MACOS_YML.replace("version: 0.9.15", "version: 0.9.16");
        assert_ne!(
            first.release_id,
            parse_qoderwork_latest(bumped.as_bytes(), AgentPlatform::Macos, AgentArch::Aarch64)
                .unwrap()
                .release_id
        );
        let serialized = serde_json::to_string(&first.release_id).unwrap();
        assert!(!serialized.contains("http"));
        assert!(!serialized.contains("static.qoder"));
        assert!(!serialized.contains("0.9.15"));
        assert!(!serialized.contains("zip"));
        assert!(!serialized.contains("sha512"));
    }

    #[test]
    fn qoderwork_rejects_indented_or_missing_yml_version() {
        assert_eq!(
            parse_qoderwork_latest(
                b"files:\n  version: 0.9.15\n",
                AgentPlatform::Macos,
                AgentArch::Aarch64
            ),
            Err(SourceResolveError::SchemaInvalid)
        );
        assert_eq!(
            parse_qoderwork_latest(
                b"path: QoderWorkCN-arm64.dmg\n",
                AgentPlatform::Macos,
                AgentArch::Aarch64
            ),
            Err(SourceResolveError::SchemaInvalid)
        );
    }
}
