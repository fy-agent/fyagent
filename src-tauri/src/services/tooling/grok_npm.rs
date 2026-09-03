//! Host-owned Grok npm manifest loading and registry selection.
//!
//! Version and SHA-512 truth come from the bundled JSON compiled into the
//! signed application. Registry metadata is compared against that manifest;
//! `@latest` is never queried.

use std::collections::BTreeMap;
use std::time::Duration;

use fyagent_user_helper::{
    grok_npm::{current_platform_package, GrokNpmInstallPlan, GrokNpmPlanError, GrokNpmRegistry},
    GROK_NPM_PACKAGE,
};

const MANIFEST_JSON: &str = include_str!("grok_npm_manifest.json");
const METADATA_TIMEOUT: Duration = Duration::from_secs(20);
const METADATA_MAX_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct GrokNpmManifest {
    version: String,
    integrity: BTreeMap<String, String>,
}

impl GrokNpmManifest {
    pub(super) fn version(&self) -> &str {
        &self.version
    }

    pub(super) fn package_integrity(&self) -> Option<&str> {
        self.integrity.get(GROK_NPM_PACKAGE).map(String::as_str)
    }

    pub(super) fn platform_integrity(&self, platform_package: &str) -> Option<&str> {
        self.integrity.get(platform_package).map(String::as_str)
    }
}

pub(super) fn bundled_manifest() -> Result<GrokNpmManifest, GrokNpmPlanError> {
    parse_manifest(MANIFEST_JSON)
}

pub(super) fn bundled_manifest_version() -> Option<String> {
    bundled_manifest().ok().map(|manifest| manifest.version)
}

pub(super) fn parse_manifest(json: &str) -> Result<GrokNpmManifest, GrokNpmPlanError> {
    let value: serde_json::Value =
        serde_json::from_str(json).map_err(|_| GrokNpmPlanError::Missing)?;
    if value.get("channel").and_then(|value| value.as_str()) != Some("stable") {
        return Err(GrokNpmPlanError::Missing);
    }
    if value.get("package").and_then(|value| value.as_str()) != Some(GROK_NPM_PACKAGE) {
        return Err(GrokNpmPlanError::InvalidPlatformPackage);
    }
    let version = value
        .get("version")
        .and_then(|value| value.as_str())
        .ok_or(GrokNpmPlanError::InvalidVersion)?;
    let integrity = value
        .get("integrity")
        .and_then(|value| value.as_object())
        .ok_or(GrokNpmPlanError::InvalidIntegrity)?;
    let mut map = BTreeMap::new();
    for (name, hash) in integrity {
        let hash = hash.as_str().ok_or(GrokNpmPlanError::InvalidIntegrity)?;
        map.insert(name.clone(), hash.to_string());
    }
    if !map.contains_key(GROK_NPM_PACKAGE) {
        return Err(GrokNpmPlanError::InvalidIntegrity);
    }
    let platform = current_platform_package().ok_or(GrokNpmPlanError::InvalidPlatformPackage)?;
    if !map.contains_key(platform) {
        return Err(GrokNpmPlanError::InvalidPlatformPackage);
    }
    GrokNpmInstallPlan::new(
        version,
        GrokNpmRegistry::Npmjs,
        map.get(GROK_NPM_PACKAGE).cloned().unwrap_or_default(),
        platform,
        map.get(platform).cloned().unwrap_or_default(),
        false,
    )?;
    Ok(GrokNpmManifest {
        version: version.to_string(),
        integrity: map,
    })
}

pub(super) fn plan_for_registry(
    manifest: &GrokNpmManifest,
    registry: GrokNpmRegistry,
    allow_install_scripts: bool,
) -> Result<GrokNpmInstallPlan, GrokNpmPlanError> {
    let platform = current_platform_package().ok_or(GrokNpmPlanError::InvalidPlatformPackage)?;
    GrokNpmInstallPlan::new(
        manifest.version(),
        registry,
        manifest.package_integrity().unwrap_or_default(),
        platform,
        manifest.platform_integrity(platform).unwrap_or_default(),
        allow_install_scripts,
    )
}

pub(super) fn default_install_command() -> Option<String> {
    let manifest = bundled_manifest().ok()?;
    let plan = plan_for_registry(&manifest, GrokNpmRegistry::Tencent, false).ok()?;
    Some(format!("npm {}", plan.npm_argv().join(" ")))
}

#[cfg(test)]
pub(super) fn first_matching_registry(
    outcomes: &[(GrokNpmRegistry, Option<&str>)],
    expected_integrity: &str,
) -> Option<GrokNpmRegistry> {
    outcomes.iter().find_map(|(registry, integrity)| {
        integrity
            .filter(|value| *value == expected_integrity)
            .map(|_| *registry)
    })
}

#[cfg(test)]
pub(super) fn matching_registries_in_order(
    outcomes: &[(GrokNpmRegistry, Option<&str>)],
    expected_integrity: &str,
) -> Vec<GrokNpmRegistry> {
    outcomes
        .iter()
        .filter_map(|(registry, integrity)| {
            integrity
                .filter(|value| *value == expected_integrity)
                .map(|_| *registry)
        })
        .collect()
}

pub(super) async fn registries_matching_manifest(
    client: &reqwest::Client,
    manifest: &GrokNpmManifest,
) -> Vec<GrokNpmRegistry> {
    let Some(expected_package) = manifest.package_integrity() else {
        return Vec::new();
    };
    let Some(platform) = current_platform_package() else {
        return Vec::new();
    };
    let Some(expected_platform) = manifest.platform_integrity(platform) else {
        return Vec::new();
    };
    let mut matching = Vec::new();
    for registry in GrokNpmRegistry::ALL {
        if registry_matches(
            client,
            registry,
            manifest.version(),
            expected_package,
            platform,
            expected_platform,
        )
        .await
        {
            matching.push(registry);
        }
    }
    matching
}

async fn registry_matches(
    client: &reqwest::Client,
    registry: GrokNpmRegistry,
    version: &str,
    expected_package: &str,
    platform_package: &str,
    expected_platform: &str,
) -> bool {
    let package_ok = fetch_integrity(client, registry, GROK_NPM_PACKAGE, version)
        .await
        .is_some_and(|integrity| integrity == expected_package);
    if !package_ok {
        return false;
    }
    fetch_integrity(client, registry, platform_package, version)
        .await
        .is_some_and(|integrity| integrity == expected_platform)
}

async fn fetch_integrity(
    client: &reqwest::Client,
    registry: GrokNpmRegistry,
    package: &str,
    version: &str,
) -> Option<String> {
    if version.eq_ignore_ascii_case("latest") {
        return None;
    }
    let url = metadata_url(registry, package, version)?;
    let response = client
        .get(url)
        .timeout(METADATA_TIMEOUT)
        .send()
        .await
        .ok()?;
    if !response.status().is_success() {
        return None;
    }
    let bytes = response.bytes().await.ok()?;
    if bytes.is_empty() || bytes.len() > METADATA_MAX_BYTES {
        return None;
    }
    let json: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    if json.get("version").and_then(|value| value.as_str()) != Some(version) {
        return None;
    }
    json.get("dist")?
        .get("integrity")
        .and_then(|value| value.as_str())
        .map(str::to_string)
}

fn metadata_url(registry: GrokNpmRegistry, package: &str, version: &str) -> Option<url::Url> {
    let base = url::Url::parse(registry.as_str()).ok()?;
    if base.scheme() != "https" || !base.username().is_empty() || base.password().is_some() {
        return None;
    }
    let encoded_package = package.replace('/', "%2f");
    base.join(&format!("{encoded_package}/{version}")).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_manifest_is_exact_and_has_current_platform() {
        let manifest = bundled_manifest().expect("bundled manifest");
        assert_ne!(manifest.version(), "latest");
        assert!(manifest.version().chars().next().unwrap().is_ascii_digit());
        let platform = current_platform_package().expect("current platform");
        assert!(manifest.package_integrity().unwrap().starts_with("sha512-"));
        assert!(manifest
            .platform_integrity(platform)
            .unwrap()
            .starts_with("sha512-"));
        assert!(MANIFEST_JSON.contains(manifest.version()));
        assert!(!MANIFEST_JSON.contains("@latest"));
    }

    #[test]
    fn default_install_command_uses_manifest_version_and_tencent() {
        let command = default_install_command().expect("default command");
        let version = bundled_manifest().expect("manifest").version;
        assert!(command.contains(&format!("@xai-official/grok@{version}")));
        assert!(command.contains("--registry=https://mirrors.tencent.com/npm/"));
        assert!(!command.contains("@latest"));
        assert!(!command.contains("npm config"));
        assert!(!command.contains("dangerously-allow-all"));
    }

    #[test]
    fn hash_mismatch_skips_to_the_next_registry_without_downgrade() {
        let expected = "sha512-expected";
        let selected = matching_registries_in_order(
            &[
                (GrokNpmRegistry::Tencent, Some("sha512-wrong")),
                (GrokNpmRegistry::Huawei, Some(expected)),
                (GrokNpmRegistry::Npmmirror, Some(expected)),
                (GrokNpmRegistry::Npmjs, None),
            ],
            expected,
        );
        assert_eq!(
            selected,
            [GrokNpmRegistry::Huawei, GrokNpmRegistry::Npmmirror]
        );
        assert_eq!(
            first_matching_registry(
                &[
                    (GrokNpmRegistry::Tencent, Some("sha512-wrong")),
                    (GrokNpmRegistry::Huawei, Some(expected)),
                ],
                expected
            ),
            Some(GrokNpmRegistry::Huawei)
        );
        assert_eq!(
            first_matching_registry(
                &[(GrokNpmRegistry::Tencent, Some("sha512-wrong"))],
                expected
            ),
            None
        );
    }

    #[test]
    fn parse_manifest_rejects_latest_and_missing_platform() {
        assert!(parse_manifest(
            r#"{"channel":"stable","package":"@xai-official/grok","version":"latest","integrity":{}}"#
        )
        .is_err());
        assert!(parse_manifest("{}").is_err());
    }

    #[test]
    fn metadata_url_never_asks_for_latest() {
        let url = metadata_url(GrokNpmRegistry::Tencent, GROK_NPM_PACKAGE, "1.0.13").expect("url");
        assert!(url.as_str().contains("1.0.13"));
        assert!(!url.as_str().contains("latest"));
        assert_eq!(url.scheme(), "https");
    }
}
