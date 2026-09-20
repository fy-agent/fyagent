//! Shared host-owned Grok/Claude npm registry selection.
//!
//! Latest version and SHA-512 come from registry metadata at runtime. npm still
//! receives the resolved exact version; `@latest` is never an install argv.

use std::collections::BTreeMap;
use std::time::Duration;

use fyagent_user_helper::grok_npm::{
    current_platform_package, GrokNpmInstallPlan, GrokNpmPlanError, GrokNpmRegistry,
    OfficialNpmTool,
};
#[cfg(any(target_os = "windows", test))]
use fyagent_user_helper::grok_npm::{npm_major_allows_scripts, GROK_NPM_ALLOW_SCRIPTS_PACKAGE};
#[cfg(test)]
use fyagent_user_helper::GROK_NPM_PACKAGE;

const METADATA_TIMEOUT: Duration = Duration::from_secs(20);
const METADATA_MAX_BYTES: usize = 1024 * 1024;
const VERSION_AUTHORITY: [GrokNpmRegistry; 4] = [
    GrokNpmRegistry::Npmjs,
    GrokNpmRegistry::Tencent,
    GrokNpmRegistry::Huawei,
    GrokNpmRegistry::Npmmirror,
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NpmDependency {
    pub name: String,
    pub version: String,
    pub integrity: String,
    pub unpacked_size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GrokNpmManifest {
    pub(super) version: String,
    pub(super) platform_version: String,
    pub(super) integrity: BTreeMap<String, String>,
    pub(super) tool: OfficialNpmTool,
    pub(super) root_size: u64,
    pub(super) platform_size: u64,
    pub(super) dependencies: Vec<NpmDependency>,
    pub(super) total_unpacked_size: u64,
}

impl GrokNpmManifest {
    pub(crate) fn version(&self) -> &str {
        &self.version
    }

    pub(crate) fn platform_version(&self) -> &str {
        &self.platform_version
    }

    pub(super) fn package_integrity(&self) -> Option<&str> {
        self.integrity.get(self.tool.package()).map(String::as_str)
    }

    pub(super) fn platform_integrity(&self, platform_package: &str) -> Option<&str> {
        self.integrity.get(platform_package).map(String::as_str)
    }

    #[allow(dead_code)]
    pub(crate) fn root_size(&self) -> u64 {
        self.root_size
    }

    #[allow(dead_code)]
    pub(crate) fn platform_size(&self) -> u64 {
        self.platform_size
    }

    pub(crate) fn dependencies(&self) -> &[NpmDependency] {
        &self.dependencies
    }

    pub(crate) fn total_unpacked_size(&self) -> u64 {
        self.total_unpacked_size
    }

    #[allow(dead_code)]
    pub(crate) fn required_reserve_bytes(&self) -> Option<u64> {
        self.total_unpacked_size.checked_mul(3)
    }
}

pub(crate) async fn resolve_published_manifest(
    tool: OfficialNpmTool,
) -> Result<GrokNpmManifest, GrokNpmPlanError> {
    let Some(client) = metadata_client() else {
        return Err(GrokNpmPlanError::Missing);
    };
    let platform = tool
        .current_platform_package()
        .ok_or(GrokNpmPlanError::InvalidPlatformPackage)?;
    let mut last_error = GrokNpmPlanError::Missing;
    for registry in VERSION_AUTHORITY {
        match load_manifest_from_registry(&client, tool, platform, registry).await {
            Ok(manifest) => return Ok(manifest),
            Err(error) => last_error = error,
        }
    }
    Err(last_error)
}

pub(super) async fn fetch_published_version(package: &str) -> Option<String> {
    let client = metadata_client()?;
    for registry in VERSION_AUTHORITY {
        let Some(json) = fetch_json(&client, registry, package, "latest").await else {
            continue;
        };
        let Some(version) = json.get("version").and_then(|value| value.as_str()) else {
            continue;
        };
        if GrokNpmInstallPlan::for_execution(version, GrokNpmRegistry::Npmjs, false).is_ok() {
            return Some(version.to_string());
        }
    }
    None
}

pub(super) fn install_command_for_version(version: &str) -> Option<String> {
    let plan = plan_for_registry(
        &GrokNpmManifest {
            version: version.to_string(),
            platform_version: version.to_string(),
            integrity: BTreeMap::new(),
            tool: OfficialNpmTool::Grok,
            root_size: 20_000,
            platform_size: 30_000_000,
            dependencies: Vec::new(),
            total_unpacked_size: 30_020_000,
        },
        GrokNpmRegistry::Tencent,
        false,
    )
    .ok()?;
    Some(format!("npm {}", plan.npm_argv().join(" ")))
}

#[cfg(any(target_os = "windows", test))]
pub(super) fn command_for_plan(plan: &GrokNpmInstallPlan) -> String {
    format!("npm {}", plan.npm_argv().join(" "))
}

/// npm 12+ blocks unlisted lifecycle scripts. Mirror the helper: add a
/// package-scoped allow-scripts flag only when the executing npm major needs it.
#[cfg(any(target_os = "windows", test))]
pub(super) fn command_with_script_policy(command: &str, npm_major: Option<u32>) -> String {
    let flag = format!("--allow-scripts={GROK_NPM_ALLOW_SCRIPTS_PACKAGE}");
    if command.contains("--allow-scripts=") {
        return command.to_string();
    }
    if npm_major.is_some_and(npm_major_allows_scripts) {
        format!("{command} {flag}")
    } else {
        command.to_string()
    }
}

#[cfg(any(target_os = "windows", test))]
pub(super) fn exact_install_version(command: &str) -> Option<&str> {
    let marker = format!("{}@", OfficialNpmTool::Grok.package());
    command.split_whitespace().find_map(|token| {
        token
            .strip_prefix(marker.as_str())
            .filter(|version| !version.is_empty() && *version != "latest")
    })
}

/// Command shape for tests and generic shell fallbacks. Live Grok/Claude
/// install/update resolve the published version first and do not use this.
pub(super) fn default_install_command() -> Option<String> {
    install_command_for_version("1.2.3")
}

#[cfg(test)]
pub(super) fn parse_manifest(json: &str) -> Result<GrokNpmManifest, GrokNpmPlanError> {
    parse_manifest_for(OfficialNpmTool::Grok, json)
}

#[cfg(test)]
fn parse_manifest_for(
    tool: OfficialNpmTool,
    json: &str,
) -> Result<GrokNpmManifest, GrokNpmPlanError> {
    let value: serde_json::Value =
        serde_json::from_str(json).map_err(|_| GrokNpmPlanError::Missing)?;
    if value.get("channel").and_then(|value| value.as_str()) != Some("stable") {
        return Err(GrokNpmPlanError::Missing);
    }
    if value.get("package").and_then(|value| value.as_str()) != Some(tool.package()) {
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
        map.insert(name.clone(), decode_sha512(hash)?.to_string());
    }
    if !map.contains_key(tool.package()) {
        return Err(GrokNpmPlanError::InvalidIntegrity);
    }
    let platform = tool
        .current_platform_package()
        .ok_or(GrokNpmPlanError::InvalidPlatformPackage)?;
    if !map.contains_key(platform) {
        return Err(GrokNpmPlanError::InvalidPlatformPackage);
    }
    GrokNpmInstallPlan::for_execution(version, GrokNpmRegistry::Npmjs, false)?;
    let root_size = value
        .get("root_size")
        .and_then(|value| value.as_u64())
        .unwrap_or(20_000);
    let platform_size = value
        .get("platform_size")
        .and_then(|value| value.as_u64())
        .unwrap_or(30_000_000);
    let mut resolved_dependencies = Vec::new();
    if let Some(deps_val) = value.get("dependencies") {
        let deps = deps_val
            .as_array()
            .ok_or(GrokNpmPlanError::UnsupportedDependency)?;
        for dep in deps {
            let dep_obj = dep
                .as_object()
                .ok_or(GrokNpmPlanError::UnsupportedDependency)?;
            let name = dep_obj
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or(GrokNpmPlanError::InvalidVersion)?;
            let dep_ver = dep_obj
                .get("version")
                .and_then(|v| v.as_str())
                .ok_or(GrokNpmPlanError::InvalidVersion)?;
            let dep_hash = dep_obj
                .get("integrity")
                .and_then(|v| v.as_str())
                .ok_or(GrokNpmPlanError::InvalidIntegrity)?;
            let dep_size = dep_obj
                .get("unpacked_size")
                .and_then(|v| v.as_u64())
                .filter(|&s| s > 0)
                .ok_or(GrokNpmPlanError::InvalidSize)?;
            if let Some(transitive) = dep_obj.get("dependencies") {
                let transitive_obj = transitive
                    .as_object()
                    .ok_or(GrokNpmPlanError::UnsupportedDependency)?;
                if !transitive_obj.is_empty() {
                    return Err(GrokNpmPlanError::UnsupportedDependency);
                }
            }
            resolved_dependencies.push(NpmDependency {
                name: name.to_string(),
                version: dep_ver.to_string(),
                integrity: decode_sha512(dep_hash)?.to_string(),
                unpacked_size: dep_size,
            });
        }
    }
    let mut total_unpacked_size = root_size
        .checked_add(platform_size)
        .ok_or(GrokNpmPlanError::ArithmeticOverflow)?;
    for dep in &resolved_dependencies {
        total_unpacked_size = total_unpacked_size
            .checked_add(dep.unpacked_size)
            .ok_or(GrokNpmPlanError::ArithmeticOverflow)?;
    }
    if let Some(total) = value.get("total_unpacked_size").and_then(|v| v.as_u64()) {
        total_unpacked_size = total;
    }
    Ok(GrokNpmManifest {
        version: version.to_string(),
        platform_version: version.to_string(),
        integrity: map,
        tool,
        root_size,
        platform_size,
        dependencies: resolved_dependencies,
        total_unpacked_size,
    })
}

fn decode_sha512(hash: &str) -> Result<&str, GrokNpmPlanError> {
    use base64::Engine;
    let encoded = hash
        .strip_prefix("sha512-")
        .ok_or(GrokNpmPlanError::InvalidIntegrity)?;
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|_| GrokNpmPlanError::InvalidIntegrity)?;
    if decoded.len() != 64 {
        return Err(GrokNpmPlanError::InvalidIntegrity);
    }
    Ok(hash)
}

pub(super) fn plan_for_registry(
    manifest: &GrokNpmManifest,
    registry: GrokNpmRegistry,
    allow_install_scripts: bool,
) -> Result<GrokNpmInstallPlan, GrokNpmPlanError> {
    if manifest.tool == OfficialNpmTool::Claude {
        return GrokNpmInstallPlan::for_execution(
            manifest.version(),
            registry,
            allow_install_scripts,
        );
    }
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

fn metadata_client() -> Option<reqwest::Client> {
    let builder = reqwest::Client::builder()
        .https_only(true)
        .redirect(reqwest::redirect::Policy::none())
        .timeout(METADATA_TIMEOUT);
    crate::proxy::http_client::apply_installer_proxy(builder)
        .ok()?
        .build()
        .ok()
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
    manifest: &GrokNpmManifest,
) -> Vec<GrokNpmRegistry> {
    let Some(client) = metadata_client() else {
        return Vec::new();
    };
    if manifest.package_integrity().is_none()
        || manifest
            .tool
            .current_platform_package()
            .is_none_or(|platform| manifest.platform_integrity(platform).is_none())
    {
        return Vec::new();
    }
    let mut matching = Vec::new();
    for registry in GrokNpmRegistry::ALL {
        if registry_matches(&client, registry, manifest).await {
            matching.push(registry);
        }
    }
    matching
}

async fn load_manifest_from_registry(
    client: &reqwest::Client,
    tool: OfficialNpmTool,
    platform: &str,
    registry: GrokNpmRegistry,
) -> Result<GrokNpmManifest, GrokNpmPlanError> {
    let root = fetch_json(client, registry, tool.package(), "latest")
        .await
        .ok_or(GrokNpmPlanError::Missing)?;
    let published = parse_published_root(tool.package(), platform, &root)?;
    let platform_doc = fetch_json(client, registry, platform, &published.platform_version)
        .await
        .ok_or(GrokNpmPlanError::InvalidPlatformPackage)?;
    if platform_doc.get("name").and_then(|value| value.as_str()) != Some(platform)
        || platform_doc.get("version").and_then(|value| value.as_str())
            != Some(published.platform_version.as_str())
    {
        return Err(GrokNpmPlanError::InvalidPlatformPackage);
    }
    let platform_integrity = platform_doc
        .get("dist")
        .and_then(|value| value.get("integrity"))
        .and_then(|value| value.as_str())
        .ok_or(GrokNpmPlanError::InvalidIntegrity)?;
    let platform_size = platform_doc
        .get("dist")
        .and_then(|value| value.get("unpackedSize"))
        .and_then(|value| value.as_u64())
        .filter(|&size| size > 0)
        .ok_or(GrokNpmPlanError::InvalidSize)?;
    let mut integrity = BTreeMap::new();
    integrity.insert(
        tool.package().to_string(),
        decode_sha512(&published.package_integrity)?.to_string(),
    );
    integrity.insert(
        platform.to_string(),
        decode_sha512(platform_integrity)?.to_string(),
    );
    let mut resolved_dependencies = Vec::new();
    for (dep_name, dep_spec) in &published.declared_dependencies {
        let exact_version = resolve_declared_dependency(dep_name, dep_spec)?;
        let dep_doc = fetch_json(client, registry, dep_name, exact_version)
            .await
            .ok_or(GrokNpmPlanError::Missing)?;
        if dep_doc.get("name").and_then(|value| value.as_str()) != Some(dep_name.as_str())
            || dep_doc.get("version").and_then(|value| value.as_str()) != Some(exact_version)
        {
            return Err(GrokNpmPlanError::InvalidVersion);
        }
        if let Some(transitive) = dep_doc.get("dependencies") {
            let transitive_obj = transitive
                .as_object()
                .ok_or(GrokNpmPlanError::UnsupportedDependency)?;
            if !transitive_obj.is_empty() {
                return Err(GrokNpmPlanError::UnsupportedDependency);
            }
        }
        let dep_integrity = dep_doc
            .get("dist")
            .and_then(|value| value.get("integrity"))
            .and_then(|value| value.as_str())
            .ok_or(GrokNpmPlanError::InvalidIntegrity)?;
        let dep_size = dep_doc
            .get("dist")
            .and_then(|value| value.get("unpackedSize"))
            .and_then(|value| value.as_u64())
            .filter(|&size| size > 0)
            .ok_or(GrokNpmPlanError::InvalidSize)?;
        integrity.insert(dep_name.clone(), decode_sha512(dep_integrity)?.to_string());
        resolved_dependencies.push(NpmDependency {
            name: dep_name.clone(),
            version: exact_version.to_string(),
            integrity: decode_sha512(dep_integrity)?.to_string(),
            unpacked_size: dep_size,
        });
    }
    let mut total_unpacked_size = published
        .root_size
        .checked_add(platform_size)
        .ok_or(GrokNpmPlanError::ArithmeticOverflow)?;
    for dep in &resolved_dependencies {
        total_unpacked_size = total_unpacked_size
            .checked_add(dep.unpacked_size)
            .ok_or(GrokNpmPlanError::ArithmeticOverflow)?;
    }
    GrokNpmInstallPlan::for_execution(&published.version, GrokNpmRegistry::Npmjs, false)?;
    Ok(GrokNpmManifest {
        version: published.version,
        platform_version: published.platform_version,
        integrity,
        tool,
        root_size: published.root_size,
        platform_size,
        dependencies: resolved_dependencies,
        total_unpacked_size,
    })
}

#[derive(Debug)]
struct PublishedRoot {
    version: String,
    package_integrity: String,
    platform_version: String,
    root_size: u64,
    declared_dependencies: BTreeMap<String, String>,
}

pub(super) fn resolve_declared_dependency(
    name: &str,
    spec: &str,
) -> Result<&'static str, GrokNpmPlanError> {
    match name {
        "@iarna/toml" => {
            if spec == "^3.0.0" || spec == "~3.0.0" || spec == "3.0.0" || spec == "=3.0.0" {
                Ok("3.0.0")
            } else {
                Err(GrokNpmPlanError::UnsupportedDependency)
            }
        }
        _ => Err(GrokNpmPlanError::UnsupportedDependency),
    }
}

fn parse_published_root(
    package: &str,
    platform: &str,
    json: &serde_json::Value,
) -> Result<PublishedRoot, GrokNpmPlanError> {
    if json.get("name").and_then(|value| value.as_str()) != Some(package) {
        return Err(GrokNpmPlanError::InvalidPlatformPackage);
    }
    let version = json
        .get("version")
        .and_then(|value| value.as_str())
        .ok_or(GrokNpmPlanError::InvalidVersion)?;
    GrokNpmInstallPlan::for_execution(version, GrokNpmRegistry::Npmjs, false)?;
    let package_integrity = json
        .get("dist")
        .and_then(|value| value.get("integrity"))
        .and_then(|value| value.as_str())
        .ok_or(GrokNpmPlanError::InvalidIntegrity)?;
    decode_sha512(package_integrity)?;
    let root_size = json
        .get("dist")
        .and_then(|value| value.get("unpackedSize"))
        .and_then(|value| value.as_u64())
        .filter(|&size| size > 0)
        .ok_or(GrokNpmPlanError::InvalidSize)?;
    let platform_version = json
        .get("optionalDependencies")
        .and_then(|value| value.get(platform))
        .and_then(|value| value.as_str())
        .unwrap_or(version);
    GrokNpmInstallPlan::for_execution(platform_version, GrokNpmRegistry::Npmjs, false)?;
    let mut declared_dependencies = BTreeMap::new();
    if let Some(deps_val) = json.get("dependencies") {
        let deps = deps_val
            .as_object()
            .ok_or(GrokNpmPlanError::UnsupportedDependency)?;
        for (name, spec_val) in deps {
            let spec = spec_val
                .as_str()
                .ok_or(GrokNpmPlanError::UnsupportedDependency)?;
            declared_dependencies.insert(name.clone(), spec.to_string());
        }
    }
    Ok(PublishedRoot {
        version: version.to_string(),
        package_integrity: package_integrity.to_string(),
        platform_version: platform_version.to_string(),
        root_size,
        declared_dependencies,
    })
}

async fn registry_matches(
    client: &reqwest::Client,
    registry: GrokNpmRegistry,
    manifest: &GrokNpmManifest,
) -> bool {
    let Some(expected_package) = manifest.package_integrity() else {
        return false;
    };
    let Some(platform) = manifest.tool.current_platform_package() else {
        return false;
    };
    let Some(expected_platform) = manifest.platform_integrity(platform) else {
        return false;
    };
    let package_ok = fetch_integrity(
        client,
        registry,
        manifest.tool.package(),
        manifest.version(),
    )
    .await
    .is_some_and(|integrity| integrity == expected_package);
    if !package_ok {
        return false;
    }
    let platform_ok = fetch_integrity(client, registry, platform, manifest.platform_version())
        .await
        .is_some_and(|integrity| integrity == expected_platform);
    if !platform_ok {
        return false;
    }
    for dep in manifest.dependencies() {
        let dep_ok = fetch_integrity(client, registry, &dep.name, &dep.version)
            .await
            .is_some_and(|integrity| integrity == dep.integrity);
        if !dep_ok {
            return false;
        }
    }
    true
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
    let json = fetch_json(client, registry, package, version).await?;
    if json.get("version").and_then(|value| value.as_str()) != Some(version)
        || json.get("name").and_then(|value| value.as_str()) != Some(package)
    {
        return None;
    }
    json.get("dist")?
        .get("integrity")
        .and_then(|value| value.as_str())
        .map(str::to_string)
}

async fn fetch_json(
    client: &reqwest::Client,
    registry: GrokNpmRegistry,
    package: &str,
    version: &str,
) -> Option<serde_json::Value> {
    let url = metadata_url(registry, package, version)?;
    let mut response = client
        .get(url)
        .timeout(METADATA_TIMEOUT)
        .send()
        .await
        .ok()?;
    if !response.status().is_success()
        || response
            .content_length()
            .is_some_and(|length| length > METADATA_MAX_BYTES as u64)
    {
        return None;
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.ok()? {
        if bytes.len().checked_add(chunk.len())? > METADATA_MAX_BYTES {
            return None;
        }
        bytes.extend_from_slice(&chunk);
    }
    if bytes.is_empty() {
        return None;
    }
    serde_json::from_slice(&bytes).ok()
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

    fn fixture_sha512() -> String {
        use base64::Engine;
        format!(
            "sha512-{}",
            base64::engine::general_purpose::STANDARD.encode([0u8; 64])
        )
    }

    fn grok_fixture_json() -> String {
        let hash = fixture_sha512();
        let platform = current_platform_package().expect("current platform");
        format!(
            r#"{{"channel":"stable","package":"@xai-official/grok","version":"1.2.3","integrity":{{"@xai-official/grok":"{hash}","{platform}":"{hash}"}}}}"#
        )
    }

    #[test]
    fn published_root_reads_latest_document_version_not_the_tag() {
        let hash = fixture_sha512();
        let platform = OfficialNpmTool::Grok
            .current_platform_package()
            .expect("platform");
        let json = serde_json::json!({
            "name": "@xai-official/grok",
            "version": "1.0.25",
            "dist": { "integrity": hash, "unpackedSize": 18363 },
        });
        let published = parse_published_root("@xai-official/grok", platform, &json).unwrap();
        assert_eq!(published.version, "1.0.25");
        assert_eq!(published.platform_version, "1.0.25");
        assert_eq!(published.root_size, 18363);
        assert!(parse_published_root(
            "@xai-official/grok",
            platform,
            &serde_json::json!({
                "name": "@xai-official/grok",
                "version": "latest",
                "dist": { "integrity": hash, "unpackedSize": 18363 },
            })
        )
        .is_err());
    }

    #[test]
    fn published_root_reads_optional_package_version_and_dependencies() {
        let hash = fixture_sha512();
        let platform = OfficialNpmTool::Grok
            .current_platform_package()
            .expect("platform");
        let mut json = serde_json::json!({
            "name": "@xai-official/grok",
            "version": "1.0.25",
            "dist": { "integrity": hash, "unpackedSize": 18363 },
            "dependencies": {
                "@iarna/toml": "^3.0.0"
            }
        });
        let mut deps = serde_json::Map::new();
        deps.insert(platform.to_string(), serde_json::json!("1.0.26"));
        json.as_object_mut().expect("object").insert(
            "optionalDependencies".to_string(),
            serde_json::Value::Object(deps),
        );
        let published = parse_published_root("@xai-official/grok", platform, &json).unwrap();
        assert_eq!(published.version, "1.0.25");
        assert_eq!(published.platform_version, "1.0.26");
        assert_eq!(published.root_size, 18363);
        assert_eq!(
            published
                .declared_dependencies
                .get("@iarna/toml")
                .map(String::as_str),
            Some("^3.0.0")
        );
    }

    #[test]
    fn published_root_rejects_missing_or_zero_unpacked_size() {
        let hash = fixture_sha512();
        let platform = OfficialNpmTool::Grok
            .current_platform_package()
            .expect("platform");
        for invalid_dist in [
            serde_json::json!({ "integrity": hash }),
            serde_json::json!({ "integrity": hash, "unpackedSize": 0 }),
            serde_json::json!({ "integrity": hash, "unpackedSize": -1 }),
            serde_json::json!({ "integrity": hash, "unpackedSize": "large" }),
        ] {
            let json = serde_json::json!({
                "name": "@xai-official/grok",
                "version": "1.0.25",
                "dist": invalid_dist,
            });
            assert_eq!(
                parse_published_root("@xai-official/grok", platform, &json).unwrap_err(),
                GrokNpmPlanError::InvalidSize
            );
        }
    }

    #[test]
    fn resolve_declared_dependency_freezes_iarna_toml_and_rejects_others() {
        assert_eq!(
            resolve_declared_dependency("@iarna/toml", "^3.0.0"),
            Ok("3.0.0")
        );
        assert_eq!(
            resolve_declared_dependency("@iarna/toml", "~3.0.0"),
            Ok("3.0.0")
        );
        assert_eq!(
            resolve_declared_dependency("@iarna/toml", "=3.0.0"),
            Ok("3.0.0")
        );
        assert_eq!(
            resolve_declared_dependency("@iarna/toml", "3.0.0"),
            Ok("3.0.0")
        );
        // Strictly reject unproven or broader ranges
        assert_eq!(
            resolve_declared_dependency("@iarna/toml", "^3.1.0"),
            Err(GrokNpmPlanError::UnsupportedDependency)
        );
        assert_eq!(
            resolve_declared_dependency("@iarna/toml", "^3.0.1"),
            Err(GrokNpmPlanError::UnsupportedDependency)
        );
        assert_eq!(
            resolve_declared_dependency("@iarna/toml", "~3.1.0"),
            Err(GrokNpmPlanError::UnsupportedDependency)
        );
        assert_eq!(
            resolve_declared_dependency("@iarna/toml", ">=3.0.0"),
            Err(GrokNpmPlanError::UnsupportedDependency)
        );
        assert_eq!(
            resolve_declared_dependency("@iarna/toml", "*"),
            Err(GrokNpmPlanError::UnsupportedDependency)
        );
        assert_eq!(
            resolve_declared_dependency("@iarna/toml", "3.x"),
            Err(GrokNpmPlanError::UnsupportedDependency)
        );
        assert_eq!(
            resolve_declared_dependency("@iarna/toml", "^4.0.0"),
            Err(GrokNpmPlanError::UnsupportedDependency)
        );
        assert_eq!(
            resolve_declared_dependency("lodash", "^4.17.21"),
            Err(GrokNpmPlanError::UnsupportedDependency)
        );
    }

    #[test]
    fn parse_published_root_rejects_malformed_dependencies() {
        let hash = fixture_sha512();
        let platform = OfficialNpmTool::Grok
            .current_platform_package()
            .expect("platform");
        for invalid_deps in [
            serde_json::json!(["@iarna/toml"]),
            serde_json::json!(123),
            serde_json::json!("^3.0.0"),
            serde_json::json!({ "@iarna/toml": 300 }),
            serde_json::json!({ "@iarna/toml": ["3.0.0"] }),
        ] {
            let json = serde_json::json!({
                "name": "@xai-official/grok",
                "version": "1.0.25",
                "dist": { "integrity": hash, "unpackedSize": 1000 },
                "dependencies": invalid_deps,
            });
            assert_eq!(
                parse_published_root("@xai-official/grok", platform, &json).unwrap_err(),
                GrokNpmPlanError::UnsupportedDependency
            );
        }
    }

    #[test]
    fn parse_manifest_rejects_transitive_dependencies_and_invalid_size() {
        let hash = fixture_sha512();
        let platform = current_platform_package().expect("platform");
        // Non-empty transitive dependencies
        let with_transitive = format!(
            r#"{{"channel":"stable","package":"@xai-official/grok","version":"1.2.3","root_size":18363,"platform_size":29292216,"dependencies":[{{"name":"@iarna/toml","version":"3.0.0","integrity":"{hash}","unpacked_size":99960,"dependencies":{{"foo":"1.0.0"}}}}],"integrity":{{"@xai-official/grok":"{hash}","{platform}":"{hash}"}}}}"#
        );
        assert_eq!(
            parse_manifest(&with_transitive).unwrap_err(),
            GrokNpmPlanError::UnsupportedDependency
        );
        // Malformed transitive dependencies (e.g. array)
        let malformed_transitive = format!(
            r#"{{"channel":"stable","package":"@xai-official/grok","version":"1.2.3","root_size":18363,"platform_size":29292216,"dependencies":[{{"name":"@iarna/toml","version":"3.0.0","integrity":"{hash}","unpacked_size":99960,"dependencies":["foo"]}}],"integrity":{{"@xai-official/grok":"{hash}","{platform}":"{hash}"}}}}"#
        );
        assert_eq!(
            parse_manifest(&malformed_transitive).unwrap_err(),
            GrokNpmPlanError::UnsupportedDependency
        );
        // Zero unpacked_size in dependency
        let zero_size = format!(
            r#"{{"channel":"stable","package":"@xai-official/grok","version":"1.2.3","root_size":18363,"platform_size":29292216,"dependencies":[{{"name":"@iarna/toml","version":"3.0.0","integrity":"{hash}","unpacked_size":0}}],"integrity":{{"@xai-official/grok":"{hash}","{platform}":"{hash}"}}}}"#
        );
        assert_eq!(
            parse_manifest(&zero_size).unwrap_err(),
            GrokNpmPlanError::InvalidSize
        );
    }

    #[test]
    fn parse_manifest_accepts_current_platform_fixture() {
        let manifest = parse_manifest(&grok_fixture_json()).expect("fixture");
        assert_eq!(manifest.version(), "1.2.3");
        assert_ne!(manifest.version(), "latest");
        let platform = current_platform_package().expect("current platform");
        assert!(manifest.package_integrity().unwrap().starts_with("sha512-"));
        assert!(manifest
            .platform_integrity(platform)
            .unwrap()
            .starts_with("sha512-"));
    }

    #[test]
    fn default_install_command_uses_exact_version_and_tencent() {
        let command = default_install_command().expect("default command");
        assert!(command.contains("@xai-official/grok@1.2.3"));
        assert!(command.contains("--registry=https://mirrors.tencent.com/npm/"));
        assert!(!command.contains("@latest"));
        assert!(!command.contains("npm config"));
        assert!(!command.contains("dangerously-allow-all"));
    }

    #[test]
    fn command_for_plan_uses_the_resolved_version() {
        let plan = GrokNpmInstallPlan::for_execution("1.0.25", GrokNpmRegistry::Npmjs, false)
            .expect("plan");
        let command = command_for_plan(&plan);
        assert!(command.contains("@xai-official/grok@1.0.25"), "{command}");
        assert!(!command.contains("@xai-official/grok@1.2.3"), "{command}");
        assert!(command.contains("registry.npmjs.org"), "{command}");
        assert!(!command.contains("@latest"), "{command}");
    }

    #[test]
    fn command_with_script_policy_adds_flag_only_for_npm_12() {
        let command = "npm i -g @xai-official/grok@1.0.25 --registry=https://registry.npmjs.org/";
        let with_flag = command_with_script_policy(command, Some(12));
        assert!(
            with_flag.contains("--allow-scripts=@xai-official/grok"),
            "{with_flag}"
        );
        assert!(!command_with_script_policy(command, Some(11)).contains("--allow-scripts="));
        assert!(!command_with_script_policy(command, None).contains("--allow-scripts="));
        let already = format!("{command} --allow-scripts=@xai-official/grok");
        assert_eq!(command_with_script_policy(&already, Some(12)), already);
        assert_eq!(exact_install_version(&with_flag), Some("1.0.25"));
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
    fn metadata_url_can_resolve_latest_tag_but_install_uses_exact_version() {
        let latest = metadata_url(GrokNpmRegistry::Npmjs, GROK_NPM_PACKAGE, "latest").expect("url");
        assert!(latest.as_str().ends_with("/latest"));
        let exact = metadata_url(GrokNpmRegistry::Tencent, GROK_NPM_PACKAGE, "1.2.3").expect("url");
        assert!(exact.as_str().contains("1.2.3"));
        assert!(!exact.as_str().contains("latest"));
        assert_eq!(exact.scheme(), "https");
    }

    #[test]
    fn manifest_calculates_total_unpacked_size_and_3x_reserve() {
        let hash = fixture_sha512();
        let platform = current_platform_package().expect("platform");
        let json = format!(
            r#"{{"channel":"stable","package":"@xai-official/grok","version":"1.2.3","root_size":18363,"platform_size":29292216,"dependencies":[{{"name":"@iarna/toml","version":"3.0.0","integrity":"{hash}","unpacked_size":99960}}],"integrity":{{"@xai-official/grok":"{hash}","{platform}":"{hash}","@iarna/toml":"{hash}"}}}}"#
        );
        let manifest = parse_manifest(&json).expect("manifest");
        assert_eq!(manifest.root_size(), 18363);
        assert_eq!(manifest.platform_size(), 29292216);
        assert_eq!(manifest.dependencies().len(), 1);
        assert_eq!(manifest.dependencies()[0].name, "@iarna/toml");
        assert_eq!(manifest.dependencies()[0].version, "3.0.0");
        assert_eq!(manifest.dependencies()[0].unpacked_size, 99960);
        let expected_total = 18363 + 29292216 + 99960;
        assert_eq!(manifest.total_unpacked_size(), expected_total);
        assert_eq!(manifest.required_reserve_bytes(), Some(expected_total * 3));
    }

    #[test]
    fn parse_manifest_rejects_arithmetic_overflow() {
        let hash = fixture_sha512();
        let platform = current_platform_package().expect("platform");
        let json = format!(
            r#"{{"channel":"stable","package":"@xai-official/grok","version":"1.2.3","root_size":{},"platform_size":100,"integrity":{{"@xai-official/grok":"{hash}","{platform}":"{hash}"}}}}"#,
            u64::MAX
        );
        assert_eq!(
            parse_manifest(&json).unwrap_err(),
            GrokNpmPlanError::ArithmeticOverflow
        );
    }
}
