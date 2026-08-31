//! Reviewed Claude Desktop mirror source. Metadata and artifact URLs are
//! code-owned; remote url/hash/filename fields are never download authority.

use serde::Deserialize;
use url::Url;

use super::{
    bounded_version, https_url_on_allowlist, opaque_release_id, AgentArch, AgentPlatform,
    PackageFormat, ResolvedDesktopSource, SourceResolveError, MAX_SOURCE_METADATA_BYTES,
};
use crate::services::external_agents::AgentCatalogId;

pub const CLAUDE_METADATA_HOSTS: &[&str] = &["claudeapp.agentsmirror.com"];
pub const CLAUDE_DOWNLOAD_HOSTS: &[&str] = CLAUDE_METADATA_HOSTS;
pub const CLAUDE_OFFICIAL_PAGE: &str = "https://claude.com/download";
pub const CLAUDE_MANIFEST_URL: &str = "https://claudeapp.agentsmirror.com/latest/manifest";
pub const CLAUDE_MACOS_UNIVERSAL_DMG: &str = "https://claudeapp.agentsmirror.com/latest/mac";

const CLAUDE_OFFICIAL_REDIRECT: &str =
    "https://api.anthropic.com/api/desktop/darwin/universal/dmg/latest/redirect";
const CLAUDE_ENDPOINT_KIND: &str = "claude-desktop-darwin-universal-dmg";
const MAX_ARTIFACT_BYTES: u64 = 2 * 1024 * 1024 * 1024;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawClaudeManifest {
    schema_version: u64,
    version: String,
    sources: RawClaudeSources,
}

#[derive(Debug, Deserialize)]
struct RawClaudeSources {
    macos: Option<RawClaudeMacos>,
}

#[derive(Debug, Deserialize)]
struct RawClaudeMacos {
    universal: Option<RawClaudeUniversal>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawClaudeUniversal {
    platform: Option<String>,
    arch: Option<String>,
    format: Option<String>,
    version: Option<String>,
    redirect: Option<String>,
    content_length: Option<u64>,
}

pub fn claude_official_page() -> &'static str {
    CLAUDE_OFFICIAL_PAGE
}

pub fn claude_manifest_url() -> Result<Url, SourceResolveError> {
    let parsed = Url::parse(CLAUDE_MANIFEST_URL).map_err(|_| SourceResolveError::SchemaInvalid)?;
    https_url_on_allowlist(&parsed, CLAUDE_METADATA_HOSTS)?;
    Ok(parsed)
}

pub fn parse_claude_desktop_manifest(
    body: &[u8],
    platform: AgentPlatform,
    architecture: AgentArch,
) -> Result<ResolvedDesktopSource, SourceResolveError> {
    match (platform, architecture) {
        (AgentPlatform::Macos, AgentArch::Aarch64 | AgentArch::X86_64) => {}
        _ => return Err(SourceResolveError::PlatformUnsupported),
    }
    if body.is_empty() || body.len() > MAX_SOURCE_METADATA_BYTES {
        return Err(SourceResolveError::SchemaInvalid);
    }

    let manifest: RawClaudeManifest =
        serde_json::from_slice(body).map_err(|_| SourceResolveError::SchemaInvalid)?;
    if manifest.schema_version != 2 {
        return Err(SourceResolveError::SchemaInvalid);
    }
    let version = bounded_version(&manifest.version).ok_or(SourceResolveError::SchemaInvalid)?;
    let universal = manifest
        .sources
        .macos
        .as_ref()
        .and_then(|macos| macos.universal.as_ref())
        .ok_or(SourceResolveError::SchemaInvalid)?;
    if universal.platform.as_deref() != Some("darwin")
        || universal.arch.as_deref() != Some("universal")
        || universal.format.as_deref() != Some("dmg")
    {
        return Err(SourceResolveError::SchemaInvalid);
    }
    let branch_version = universal
        .version
        .as_deref()
        .and_then(bounded_version)
        .ok_or(SourceResolveError::SchemaInvalid)?;
    if branch_version != version {
        return Err(SourceResolveError::SchemaInvalid);
    }
    if let Some(redirect) = universal.redirect.as_deref() {
        if redirect != CLAUDE_OFFICIAL_REDIRECT {
            return Err(SourceResolveError::SchemaInvalid);
        }
    }
    if let Some(content_length) = universal.content_length {
        if content_length == 0 || content_length > MAX_ARTIFACT_BYTES {
            return Err(SourceResolveError::SchemaInvalid);
        }
    }

    let download_url =
        Url::parse(CLAUDE_MACOS_UNIVERSAL_DMG).map_err(|_| SourceResolveError::SchemaInvalid)?;
    https_url_on_allowlist(&download_url, CLAUDE_DOWNLOAD_HOSTS)?;

    Ok(ResolvedDesktopSource {
        product: AgentCatalogId::ClaudeCode,
        platform,
        architecture,
        format: PackageFormat::Dmg,
        release_id: opaque_release_id(&[
            ("product", "claude-code"),
            ("surface", "desktop"),
            ("platform", platform.as_str()),
            ("architecture", architecture.as_str()),
            ("format", PackageFormat::Dmg.as_str()),
            ("version", version),
            ("endpoint", CLAUDE_ENDPOINT_KIND),
        ]),
        display_version: Some(version.to_string()),
        download_url,
        versionless_latest: false,
        official_page: claude_official_page(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_manifest(
        version: &str,
        branch_version: &str,
        platform: &str,
        arch: &str,
        format: &str,
        redirect: Option<&str>,
        extra_authority_fields: bool,
    ) -> Vec<u8> {
        let redirect_json = match redirect {
            Some(value) => format!(r#""redirect": "{value}","#),
            None => String::new(),
        };
        let extra = if extra_authority_fields {
            r#"
        "url": "https://evil.example/Claude.dmg",
        "fileName": "stolen.dmg",
        "assetName": "Claude-mac-universal.dmg",
        "sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "buildHash": "bbbb",
        "etag": "\"ignored\"",
        "lastModified": "Wed, 01 Jan 2026 00:00:00 GMT","#
        } else {
            ""
        };
        format!(
            r#"{{
  "schemaVersion": 2,
  "generatedAt": "2026-01-01T00:00:00Z",
  "version": "{version}",
  "sources": {{
    "macos": {{
      "universal": {{
        "platform": "{platform}",
        "arch": "{arch}",
        "format": "{format}",
        {redirect_json}
        {extra}
        "version": "{branch_version}",
        "contentLength": 1024
      }}
    }}
  }}
}}"#
        )
        .into_bytes()
    }

    #[test]
    fn schema_v2_universal_macos_selects_the_fixed_dmg_endpoint() {
        let body = fixture_manifest(
            "1.2.3",
            "1.2.3",
            "darwin",
            "universal",
            "dmg",
            Some(CLAUDE_OFFICIAL_REDIRECT),
            true,
        );
        let arm =
            parse_claude_desktop_manifest(&body, AgentPlatform::Macos, AgentArch::Aarch64).unwrap();
        assert_eq!(arm.product, AgentCatalogId::ClaudeCode);
        assert_eq!(arm.format, PackageFormat::Dmg);
        assert!(!arm.versionless_latest);
        assert_eq!(arm.display_version.as_deref(), Some("1.2.3"));
        assert_eq!(arm.download_url.as_str(), CLAUDE_MACOS_UNIVERSAL_DMG);
        assert_eq!(arm.official_page, CLAUDE_OFFICIAL_PAGE);
        assert!(arm.release_id.starts_with("v1:"));
        assert!(!arm.release_id.contains("http"));
        assert!(!arm.release_id.contains("evil"));
        assert!(!arm.download_url.as_str().contains("evil"));
        assert!(!arm.download_url.as_str().contains("anthropic.com"));

        let intel =
            parse_claude_desktop_manifest(&body, AgentPlatform::Macos, AgentArch::X86_64).unwrap();
        assert_eq!(intel.download_url.as_str(), CLAUDE_MACOS_UNIVERSAL_DMG);
        assert_ne!(arm.release_id, intel.release_id);
    }

    #[test]
    fn extra_url_and_hash_fields_are_not_download_authority() {
        let body = fixture_manifest(
            "9.9.9",
            "9.9.9",
            "darwin",
            "universal",
            "dmg",
            Some(CLAUDE_OFFICIAL_REDIRECT),
            true,
        );
        let source =
            parse_claude_desktop_manifest(&body, AgentPlatform::Macos, AgentArch::Aarch64).unwrap();
        assert_eq!(source.download_url.as_str(), CLAUDE_MACOS_UNIVERSAL_DMG);
        assert_ne!(
            source.download_url.as_str(),
            "https://evil.example/Claude.dmg"
        );
    }

    #[test]
    fn version_mismatch_and_wrong_platform_arch_format_fail_closed() {
        let mismatched = fixture_manifest(
            "1.2.3",
            "9.9.9",
            "darwin",
            "universal",
            "dmg",
            Some(CLAUDE_OFFICIAL_REDIRECT),
            false,
        );
        assert_eq!(
            parse_claude_desktop_manifest(&mismatched, AgentPlatform::Macos, AgentArch::Aarch64),
            Err(SourceResolveError::SchemaInvalid)
        );

        let windows_branch = fixture_manifest(
            "1.2.3",
            "1.2.3",
            "win32",
            "universal",
            "dmg",
            Some(CLAUDE_OFFICIAL_REDIRECT),
            false,
        );
        assert_eq!(
            parse_claude_desktop_manifest(
                &windows_branch,
                AgentPlatform::Macos,
                AgentArch::Aarch64
            ),
            Err(SourceResolveError::SchemaInvalid)
        );

        let intel_arch = fixture_manifest(
            "1.2.3",
            "1.2.3",
            "darwin",
            "x64",
            "dmg",
            Some(CLAUDE_OFFICIAL_REDIRECT),
            false,
        );
        assert_eq!(
            parse_claude_desktop_manifest(&intel_arch, AgentPlatform::Macos, AgentArch::Aarch64),
            Err(SourceResolveError::SchemaInvalid)
        );

        let zip = fixture_manifest(
            "1.2.3",
            "1.2.3",
            "darwin",
            "universal",
            "zip",
            Some(CLAUDE_OFFICIAL_REDIRECT),
            false,
        );
        assert_eq!(
            parse_claude_desktop_manifest(&zip, AgentPlatform::Macos, AgentArch::Aarch64),
            Err(SourceResolveError::SchemaInvalid)
        );
    }

    #[test]
    fn wrong_redirect_schema_and_oversized_body_are_rejected() {
        let wrong_redirect = fixture_manifest(
            "1.2.3",
            "1.2.3",
            "darwin",
            "universal",
            "dmg",
            Some("https://evil.example/redirect"),
            false,
        );
        assert_eq!(
            parse_claude_desktop_manifest(
                &wrong_redirect,
                AgentPlatform::Macos,
                AgentArch::Aarch64
            ),
            Err(SourceResolveError::SchemaInvalid)
        );

        let missing_redirect =
            fixture_manifest("1.2.3", "1.2.3", "darwin", "universal", "dmg", None, false);
        assert!(parse_claude_desktop_manifest(
            &missing_redirect,
            AgentPlatform::Macos,
            AgentArch::Aarch64
        )
        .is_ok());

        let mut schema_v5: serde_json::Value = serde_json::from_slice(&fixture_manifest(
            "1.2.3",
            "1.2.3",
            "darwin",
            "universal",
            "dmg",
            Some(CLAUDE_OFFICIAL_REDIRECT),
            false,
        ))
        .unwrap();
        schema_v5["schemaVersion"] = serde_json::json!(5);
        assert_eq!(
            parse_claude_desktop_manifest(
                &serde_json::to_vec(&schema_v5).unwrap(),
                AgentPlatform::Macos,
                AgentArch::Aarch64
            ),
            Err(SourceResolveError::SchemaInvalid)
        );

        let oversized = vec![b'{'; MAX_SOURCE_METADATA_BYTES + 1];
        assert_eq!(
            parse_claude_desktop_manifest(&oversized, AgentPlatform::Macos, AgentArch::Aarch64),
            Err(SourceResolveError::SchemaInvalid)
        );
    }

    #[test]
    fn windows_and_unknown_hosts_are_not_claude_sources() {
        let body = fixture_manifest(
            "1.2.3",
            "1.2.3",
            "darwin",
            "universal",
            "dmg",
            Some(CLAUDE_OFFICIAL_REDIRECT),
            false,
        );
        assert_eq!(
            parse_claude_desktop_manifest(&body, AgentPlatform::Windows, AgentArch::X86_64),
            Err(SourceResolveError::PlatformUnsupported)
        );
        assert_eq!(
            parse_claude_desktop_manifest(&body, AgentPlatform::Windows, AgentArch::Aarch64),
            Err(SourceResolveError::PlatformUnsupported)
        );

        let manifest = Url::parse(CLAUDE_MANIFEST_URL).unwrap();
        assert!(https_url_on_allowlist(&manifest, CLAUDE_METADATA_HOSTS).is_ok());
        let proxy = Url::parse("https://evil.example/latest/mac").unwrap();
        assert_eq!(
            https_url_on_allowlist(&proxy, CLAUDE_DOWNLOAD_HOSTS),
            Err(SourceResolveError::HostRejected)
        );
        assert_eq!(claude_manifest_url().unwrap().as_str(), CLAUDE_MANIFEST_URL);
    }
}
