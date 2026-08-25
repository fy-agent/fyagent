use serde_json::Value;
use url::Url;

use super::{
    bounded_version, https_url_on_allowlist, opaque_release_id, AgentArch, AgentPlatform,
    PackageFormat, ResolvedDesktopSource, SourceResolveError, MAX_SOURCE_METADATA_BYTES,
};
use crate::services::external_agents::AgentCatalogId;

pub const TRAEWORK_METADATA_ENDPOINTS: &[&str] = &[
    "https://api.trae.cn/icube/api/v1/native/version/trae/cn/latest",
    "https://api.trae.ai/icube/api/v1/native/version/trae/cn/latest",
];
pub const TRAEWORK_METADATA_HOSTS: &[&str] = &["api.trae.cn", "api.trae.ai"];
pub const TRAEWORK_DOWNLOAD_HOSTS: &[&str] = &["lf-cdn.trae.com.cn"];
pub const TRAEWORK_OFFICIAL_PAGE: &str = "https://www.trae.cn/sem-work";

const STABLE_PATH_PREFIX: &str = "/obj/trae-com-cn/pkg/app/releases/stable/";

pub fn traework_official_page() -> &'static str {
    TRAEWORK_OFFICIAL_PAGE
}

pub fn parse_traework_latest(
    body: &[u8],
    platform: AgentPlatform,
    architecture: AgentArch,
) -> Result<ResolvedDesktopSource, SourceResolveError> {
    if body.len() > MAX_SOURCE_METADATA_BYTES {
        return Err(SourceResolveError::SchemaInvalid);
    }
    let value: Value =
        serde_json::from_slice(body).map_err(|_| SourceResolveError::SchemaInvalid)?;
    let solo = value
        .get("data")
        .and_then(|data| data.get("solo"))
        .ok_or(SourceResolveError::SchemaInvalid)?;

    let (platform_key, url_key, format, filename, endpoint_kind) = match (platform, architecture) {
        (AgentPlatform::Windows, AgentArch::X86_64) => (
            "win32",
            "x64",
            PackageFormat::Exe,
            "TraeWork_CN-Setup-x64.exe",
            "traework-cn-win-x64",
        ),
        (AgentPlatform::Macos, AgentArch::Aarch64) => (
            "darwin",
            "apple",
            PackageFormat::Dmg,
            "TraeWork_CN-darwin-arm64.dmg",
            "traework-cn-darwin-arm64",
        ),
        (AgentPlatform::Macos, AgentArch::X86_64) => (
            "darwin",
            "intel",
            PackageFormat::Dmg,
            "TraeWork_CN-darwin-x64.dmg",
            "traework-cn-darwin-x64",
        ),
        (AgentPlatform::Windows, AgentArch::Aarch64) => {
            return Err(SourceResolveError::PlatformUnsupported)
        }
    };

    let downloads = solo
        .get(platform_key)
        .and_then(|node| node.get("download"))
        .and_then(Value::as_array)
        .ok_or(SourceResolveError::SchemaInvalid)?;
    let cn_entry = downloads
        .iter()
        .find(|entry| entry.get("region").and_then(Value::as_str) == Some("cn"))
        .ok_or(SourceResolveError::SchemaInvalid)?;
    let raw_url = cn_entry
        .get(url_key)
        .and_then(Value::as_str)
        .ok_or(SourceResolveError::SchemaInvalid)?;

    if looks_like_trae_code_artifact(raw_url) || body_selects_manifest_for_work(raw_url, &value) {
        return Err(SourceResolveError::CodePackageRejected);
    }

    let download_url = Url::parse(raw_url).map_err(|_| SourceResolveError::ArtifactRejected)?;
    https_url_on_allowlist(&download_url, TRAEWORK_DOWNLOAD_HOSTS)?;
    let version = validate_traework_artifact(&download_url, filename)?;

    Ok(ResolvedDesktopSource {
        product: AgentCatalogId::TraeWork,
        platform,
        architecture,
        format,
        release_id: opaque_release_id(&[
            ("product", "trae-work"),
            ("platform", platform.as_str()),
            ("architecture", architecture.as_str()),
            ("format", format.as_str()),
            ("version", version),
            ("endpoint", endpoint_kind),
        ]),
        display_version: Some(version.to_string()),
        download_url,
        versionless_latest: false,
        official_page: traework_official_page(),
    })
}

fn looks_like_trae_code_artifact(url: &str) -> bool {
    url.contains("TraeCode_")
}

fn body_selects_manifest_for_work(selected_url: &str, root: &Value) -> bool {
    root.get("data")
        .and_then(|data| data.get("manifest"))
        .map(|manifest| manifest.to_string().contains(selected_url))
        .unwrap_or(false)
        && selected_url.contains("TraeCode_")
}

fn validate_traework_artifact<'a>(
    url: &'a Url,
    expected_filename: &str,
) -> Result<&'a str, SourceResolveError> {
    if url.cannot_be_a_base() {
        return Err(SourceResolveError::ArtifactRejected);
    }
    let path = url.path();
    if !path.starts_with(STABLE_PATH_PREFIX) || path.contains("..") {
        return Err(SourceResolveError::ArtifactRejected);
    }
    let rest = &path[STABLE_PATH_PREFIX.len()..];
    let (version, remainder) = rest
        .split_once('/')
        .ok_or(SourceResolveError::ArtifactRejected)?;
    let version = bounded_version(version).ok_or(SourceResolveError::ArtifactRejected)?;
    let expected_tail = match expected_filename {
        "TraeWork_CN-Setup-x64.exe" => format!("win32/{expected_filename}"),
        "TraeWork_CN-darwin-arm64.dmg" => format!("darwin/{expected_filename}"),
        "TraeWork_CN-darwin-x64.dmg" => format!("darwin/{expected_filename}"),
        _ => return Err(SourceResolveError::ArtifactRejected),
    };
    if remainder != expected_tail {
        return Err(SourceResolveError::ArtifactRejected);
    }
    if url
        .path_segments()
        .and_then(|mut segments| segments.next_back())
        != Some(expected_filename)
    {
        return Err(SourceResolveError::ArtifactRejected);
    }
    Ok(version)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = r#"{
      "success": true,
      "data": {
        "manifest": {
          "win32": {
            "download": [
              {"region":"cn","x64":"https://lf-cdn.trae.com.cn/obj/trae-com-cn/pkg/app/releases/stable/2.3.76125/win32/TraeCode_CN-Setup-x64.exe"}
            ]
          }
        },
        "solo": {
          "win32": {
            "download": [
              {"region":"cn","x64":"https://lf-cdn.trae.com.cn/obj/trae-com-cn/pkg/app/releases/stable/2.3.76922/win32/TraeWork_CN-Setup-x64.exe"},
              {"region":"sg","x64":"https://lf-cdn.trae.ai/obj/trae-ai-sg/pkg/app/releases/stable/2.3.76922/win32/TraeWork_CN-Setup-x64.exe"}
            ]
          },
          "darwin": {
            "download": [
              {
                "region":"cn",
                "apple":"https://lf-cdn.trae.com.cn/obj/trae-com-cn/pkg/app/releases/stable/2.3.76922/darwin/TraeWork_CN-darwin-arm64.dmg",
                "intel":"https://lf-cdn.trae.com.cn/obj/trae-com-cn/pkg/app/releases/stable/2.3.76922/darwin/TraeWork_CN-darwin-x64.dmg"
              }
            ]
          }
        }
      }
    }"#;

    #[test]
    fn traework_selects_solo_cn_and_never_manifest_code() {
        let windows = parse_traework_latest(
            FIXTURE.as_bytes(),
            AgentPlatform::Windows,
            AgentArch::X86_64,
        )
        .unwrap();
        assert_eq!(windows.display_version.as_deref(), Some("2.3.76922"));
        assert!(windows
            .download_url
            .as_str()
            .ends_with("/win32/TraeWork_CN-Setup-x64.exe"));
        assert!(!windows.download_url.as_str().contains("TraeCode_"));
        assert_eq!(windows.format, PackageFormat::Exe);
        assert_eq!(windows.official_page, traework_official_page());

        let arm =
            parse_traework_latest(FIXTURE.as_bytes(), AgentPlatform::Macos, AgentArch::Aarch64)
                .unwrap();
        assert!(arm
            .download_url
            .as_str()
            .ends_with("/darwin/TraeWork_CN-darwin-arm64.dmg"));
        let intel =
            parse_traework_latest(FIXTURE.as_bytes(), AgentPlatform::Macos, AgentArch::X86_64)
                .unwrap();
        assert!(intel
            .download_url
            .as_str()
            .ends_with("/darwin/TraeWork_CN-darwin-x64.dmg"));
        assert_ne!(arm.release_id, intel.release_id);
        assert_eq!(
            parse_traework_latest(
                FIXTURE.as_bytes(),
                AgentPlatform::Windows,
                AgentArch::Aarch64
            ),
            Err(SourceResolveError::PlatformUnsupported)
        );
    }

    #[test]
    fn traework_rejects_code_package_unapproved_host_and_stale_literal_fallback() {
        let mut swapped: Value = serde_json::from_str(FIXTURE).unwrap();
        swapped["data"]["solo"]["win32"]["download"][0]["x64"] = serde_json::json!(
            "https://lf-cdn.trae.com.cn/obj/trae-com-cn/pkg/app/releases/stable/2.3.76125/win32/TraeCode_CN-Setup-x64.exe"
        );
        assert_eq!(
            parse_traework_latest(
                &serde_json::to_vec(&swapped).unwrap(),
                AgentPlatform::Windows,
                AgentArch::X86_64
            ),
            Err(SourceResolveError::CodePackageRejected)
        );

        swapped = serde_json::from_str(FIXTURE).unwrap();
        swapped["data"]["solo"]["win32"]["download"][0]["x64"] = serde_json::json!(
            "https://evil.example/obj/trae-com-cn/pkg/app/releases/stable/2.3.76922/win32/TraeWork_CN-Setup-x64.exe"
        );
        assert_eq!(
            parse_traework_latest(
                &serde_json::to_vec(&swapped).unwrap(),
                AgentPlatform::Windows,
                AgentArch::X86_64
            ),
            Err(SourceResolveError::HostRejected)
        );

        let serialized = serde_json::to_string(
            &parse_traework_latest(FIXTURE.as_bytes(), AgentPlatform::Macos, AgentArch::Aarch64)
                .unwrap()
                .release_id,
        )
        .unwrap();
        assert!(!serialized.contains("2.3.76922"));
        assert!(!serialized.contains("http"));
    }
}
