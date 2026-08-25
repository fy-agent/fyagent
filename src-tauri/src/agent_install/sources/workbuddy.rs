use serde_json::Value;
use url::Url;

use super::{
    bounded_version, https_url_on_allowlist, opaque_release_id, AgentArch, AgentPlatform,
    PackageFormat, ResolvedDesktopSource, SourceResolveError, MAX_SOURCE_METADATA_BYTES,
};
use crate::services::external_agents::AgentCatalogId;

pub const WORKBUDDY_METADATA_HOSTS: &[&str] = &["www.workbuddy.cn"];
pub const WORKBUDDY_DOWNLOAD_HOSTS: &[&str] = &["download.codebuddy.cn"];
pub const WORKBUDDY_OFFICIAL_PAGE: &str = "https://www.workbuddy.cn/home";
pub const WORKBUDDY_PLATFORM_IDS: &[&str] = &[
    "workbuddy-darwin-x64",
    "workbuddy-darwin-arm64",
    "workbuddy-win32-x64-user",
];

const UPDATE_ENDPOINT: &str = "https://www.workbuddy.cn/v2/update";

pub fn workbuddy_official_page() -> &'static str {
    WORKBUDDY_OFFICIAL_PAGE
}

pub fn workbuddy_update_url(
    platform: AgentPlatform,
    architecture: AgentArch,
) -> Result<Url, SourceResolveError> {
    let id = platform_id(platform, architecture)?;
    Url::parse(&format!("{UPDATE_ENDPOINT}?platform={id}"))
        .map_err(|_| SourceResolveError::SchemaInvalid)
}

pub fn parse_workbuddy_update(
    body: &[u8],
    platform: AgentPlatform,
    architecture: AgentArch,
) -> Result<ResolvedDesktopSource, SourceResolveError> {
    if body.len() > MAX_SOURCE_METADATA_BYTES {
        return Err(SourceResolveError::SchemaInvalid);
    }
    let value: Value =
        serde_json::from_slice(body).map_err(|_| SourceResolveError::SchemaInvalid)?;
    let version = value
        .get("productVersion")
        .or_else(|| value.get("version"))
        .and_then(Value::as_str)
        .and_then(bounded_version)
        .ok_or(SourceResolveError::SchemaInvalid)?;
    let raw_url = value
        .get("url")
        .and_then(Value::as_str)
        .ok_or(SourceResolveError::SchemaInvalid)?;
    let mut download_url = Url::parse(raw_url).map_err(|_| SourceResolveError::ArtifactRejected)?;
    https_url_on_allowlist(&download_url, WORKBUDDY_DOWNLOAD_HOSTS)?;

    let (format, expected_filename_prefix, path_platform, endpoint_kind) =
        match (platform, architecture) {
            (AgentPlatform::Windows, AgentArch::X86_64) => (
                PackageFormat::Exe,
                "WorkBuddy-win32-x64-user-",
                "win32-x64-user",
                "workbuddy-win32-x64-user",
            ),
            (AgentPlatform::Macos, AgentArch::Aarch64) => (
                PackageFormat::Dmg,
                "WorkBuddy-darwin-arm64-",
                "darwin-arm64",
                "workbuddy-darwin-arm64",
            ),
            (AgentPlatform::Macos, AgentArch::X86_64) => (
                PackageFormat::Dmg,
                "WorkBuddy-darwin-x64-",
                "darwin-x64",
                "workbuddy-darwin-x64",
            ),
            (AgentPlatform::Windows, AgentArch::Aarch64) => {
                return Err(SourceResolveError::PlatformUnsupported)
            }
        };

    validate_workbuddy_path(
        &download_url,
        path_platform,
        expected_filename_prefix,
        version,
    )?;
    if format == PackageFormat::Dmg {
        download_url = rewrite_zip_to_dmg(&download_url)?;
        https_url_on_allowlist(&download_url, WORKBUDDY_DOWNLOAD_HOSTS)?;
        validate_workbuddy_path(
            &download_url,
            path_platform,
            expected_filename_prefix,
            version,
        )?;
        if !download_url.path().ends_with(".dmg") {
            return Err(SourceResolveError::ArtifactRejected);
        }
    } else if !download_url.path().ends_with(".exe") {
        return Err(SourceResolveError::ArtifactRejected);
    }

    Ok(ResolvedDesktopSource {
        product: AgentCatalogId::WorkBuddy,
        platform,
        architecture,
        format,
        release_id: opaque_release_id(&[
            ("product", "workbuddy"),
            ("platform", platform.as_str()),
            ("architecture", architecture.as_str()),
            ("format", format.as_str()),
            ("version", version),
            ("endpoint", endpoint_kind),
        ]),
        display_version: Some(version.to_string()),
        download_url,
        versionless_latest: false,
        official_page: workbuddy_official_page(),
    })
}

fn platform_id(
    platform: AgentPlatform,
    architecture: AgentArch,
) -> Result<&'static str, SourceResolveError> {
    let id = match (platform, architecture) {
        (AgentPlatform::Macos, AgentArch::X86_64) => "workbuddy-darwin-x64",
        (AgentPlatform::Macos, AgentArch::Aarch64) => "workbuddy-darwin-arm64",
        (AgentPlatform::Windows, AgentArch::X86_64) => "workbuddy-win32-x64-user",
        (AgentPlatform::Windows, AgentArch::Aarch64) => {
            return Err(SourceResolveError::PlatformUnsupported)
        }
    };
    if !WORKBUDDY_PLATFORM_IDS.contains(&id) {
        return Err(SourceResolveError::PlatformUnsupported);
    }
    Ok(id)
}

fn rewrite_zip_to_dmg(url: &Url) -> Result<Url, SourceResolveError> {
    let path = url.path();
    let Some(stripped) = path.strip_suffix(".zip") else {
        return Err(SourceResolveError::ArtifactRejected);
    };
    let mut rewritten = url.clone();
    rewritten.set_path(&format!("{stripped}.dmg"));
    Ok(rewritten)
}

fn validate_workbuddy_path(
    url: &Url,
    path_platform: &str,
    filename_prefix: &str,
    version: &str,
) -> Result<(), SourceResolveError> {
    let expected_prefix = format!("/workbuddy/saas/{path_platform}/");
    let path = url.path();
    if !path.starts_with(&expected_prefix) || path.contains("..") {
        return Err(SourceResolveError::ArtifactRejected);
    }
    let filename = url
        .path_segments()
        .and_then(|mut segments| segments.next_back())
        .ok_or(SourceResolveError::ArtifactRejected)?;
    if !filename.starts_with(filename_prefix) || !filename.contains(version) {
        return Err(SourceResolveError::ArtifactRejected);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const ARM_FIXTURE: &str = r#"{
      "version":"5.3.14.36279234",
      "url":"https://download.codebuddy.cn/workbuddy/saas/darwin-arm64/WorkBuddy-darwin-arm64-5.3.14.36279234-825709d4.zip",
      "productVersion":"5.3.14.36279234",
      "sha256hash":"a7c18fecd2939f8bd7a00ab5accdd905dbc5bbd5927291c9c5762541c1bd6a61"
    }"#;
    const WIN_FIXTURE: &str = r#"{
      "version":"5.3.14.36279234",
      "url":"https://download.codebuddy.cn/workbuddy/saas/win32-x64-user/WorkBuddy-win32-x64-user-5.3.14.36279234-deadbeef.exe",
      "productVersion":"5.3.14.36279234",
      "sha256hash":"ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
    }"#;
    const INTEL_FIXTURE: &str = r#"{
      "version":"5.3.14.36279234",
      "url":"https://download.codebuddy.cn/workbuddy/saas/darwin-x64/WorkBuddy-darwin-x64-5.3.14.36279234-825709d4.zip",
      "productVersion":"5.3.14.36279234"
    }"#;

    #[test]
    fn workbuddy_uses_closed_platform_ids_and_official_zip_to_dmg() {
        assert_eq!(
            workbuddy_update_url(AgentPlatform::Macos, AgentArch::Aarch64)
                .unwrap()
                .as_str(),
            "https://www.workbuddy.cn/v2/update?platform=workbuddy-darwin-arm64"
        );
        let arm = parse_workbuddy_update(
            ARM_FIXTURE.as_bytes(),
            AgentPlatform::Macos,
            AgentArch::Aarch64,
        )
        .unwrap();
        assert_eq!(arm.display_version.as_deref(), Some("5.3.14.36279234"));
        assert!(arm.download_url.as_str().ends_with(".dmg"));
        assert!(!arm.download_url.as_str().ends_with(".zip"));
        assert_eq!(arm.format, PackageFormat::Dmg);
        assert_eq!(arm.official_page, workbuddy_official_page());

        let intel = parse_workbuddy_update(
            INTEL_FIXTURE.as_bytes(),
            AgentPlatform::Macos,
            AgentArch::X86_64,
        )
        .unwrap();
        assert!(intel
            .download_url
            .as_str()
            .ends_with("WorkBuddy-darwin-x64-5.3.14.36279234-825709d4.dmg"));

        let windows = parse_workbuddy_update(
            WIN_FIXTURE.as_bytes(),
            AgentPlatform::Windows,
            AgentArch::X86_64,
        )
        .unwrap();
        assert!(windows.download_url.as_str().ends_with(".exe"));
        assert_eq!(
            parse_workbuddy_update(
                WIN_FIXTURE.as_bytes(),
                AgentPlatform::Windows,
                AgentArch::Aarch64
            ),
            Err(SourceResolveError::PlatformUnsupported)
        );
    }

    #[test]
    fn workbuddy_ignores_remote_hash_and_rejects_unapproved_hosts() {
        let parsed = parse_workbuddy_update(
            ARM_FIXTURE.as_bytes(),
            AgentPlatform::Macos,
            AgentArch::Aarch64,
        )
        .unwrap();
        let wire = serde_json::to_string(&parsed.release_id).unwrap();
        assert!(!wire.contains("sha256"));
        assert!(!wire.contains("a7c18fecd2939f8bd7a00ab5accdd905dbc5bbd5927291c9c5762541c1bd6a61"));
        assert!(!wire.contains("http"));

        let mut evil: Value = serde_json::from_str(ARM_FIXTURE).unwrap();
        evil["url"] = serde_json::json!(
            "https://evil.example/workbuddy/saas/darwin-arm64/WorkBuddy-darwin-arm64-5.3.14.36279234-825709d4.zip"
        );
        assert_eq!(
            parse_workbuddy_update(
                &serde_json::to_vec(&evil).unwrap(),
                AgentPlatform::Macos,
                AgentArch::Aarch64
            ),
            Err(SourceResolveError::HostRejected)
        );
    }
}
