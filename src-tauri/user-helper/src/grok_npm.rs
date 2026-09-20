//! Closed Grok official-npm install plan.
//!
//! This module is the only owner of `@xai-official/grok` install argv,
//! registry allowlisting, and the bounded helper control payload. It is
//! pure: no process launch, filesystem, or network I/O.

use crate::grok::{GROK_NPM_PACKAGE, MAX_NORMALIZED_VERSION_BYTES};

pub const GROK_NPM_REGISTRY_ENV: &str = "GROK_NPM_REGISTRY";
pub const GROK_NPM_VERSION_ENV: &str = "GROK_NPM_VERSION";
pub const GROK_NPM_PACKAGE_INTEGRITY_ENV: &str = "GROK_NPM_PACKAGE_INTEGRITY";
pub const GROK_NPM_PLATFORM_PACKAGE_ENV: &str = "GROK_NPM_PLATFORM_PACKAGE";
pub const GROK_NPM_PLATFORM_INTEGRITY_ENV: &str = "GROK_NPM_PLATFORM_INTEGRITY";
pub const GROK_NPM_ALLOW_SCRIPTS_ENV: &str = "GROK_NPM_ALLOW_SCRIPTS";
pub const GROK_NPM_ALLOW_SCRIPTS_PACKAGE: &str = "@xai-official/grok";
pub const GROK_NPM_ALLOW_SCRIPTS_MAJOR: u32 = 12;
pub const GROK_NPM_PLAN_CONTROL_BYTES: usize = 80;
pub const GROK_NPM_PLAN_CONTROL_VERSION: u8 = 2;

pub const CLOSED_DEP_TAG_NONE: u8 = 0;
pub const CLOSED_DEP_TAG_IARNA_TOML: u8 = 1;
pub const CLOSED_DEP_IARNA_TOML_NAME: &str = "@iarna/toml";
pub const CLOSED_DEP_VERSION_MAX_BYTES: usize = 16;

const PLAN_MAGIC: [u8; 8] = *b"FYAGROKP";
const VERSION_FIELD_BYTES: usize = 32;

/// Package identity is selected by a code-owned product action, never by a URL
/// or package name from the renderer. Both products share registry/argv policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OfficialNpmTool {
    Grok,
    Claude,
}

impl OfficialNpmTool {
    pub const fn package(self) -> &'static str {
        match self {
            Self::Grok => GROK_NPM_PACKAGE,
            Self::Claude => crate::claude::CLAUDE_NPM_PACKAGE,
        }
    }

    pub const fn executable(self) -> &'static str {
        match self {
            Self::Grok => "grok",
            Self::Claude => "claude",
        }
    }

    pub fn current_platform_package(self) -> Option<&'static str> {
        match self {
            Self::Grok => current_platform_package(),
            Self::Claude => {
                #[cfg(target_os = "macos")]
                match std::env::consts::ARCH {
                    "aarch64" => Some("@anthropic-ai/claude-code-darwin-arm64"),
                    "x86_64" => Some("@anthropic-ai/claude-code-darwin-x64"),
                    _ => None,
                }
                #[cfg(target_os = "windows")]
                match std::env::consts::ARCH {
                    "aarch64" => Some("@anthropic-ai/claude-code-win32-arm64"),
                    "x86_64" => Some("@anthropic-ai/claude-code-win32-x64"),
                    _ => None,
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GrokNpmRegistry {
    Tencent,
    Huawei,
    Npmmirror,
    Npmjs,
}

impl GrokNpmRegistry {
    pub const ALL: [Self; 4] = [Self::Tencent, Self::Huawei, Self::Npmmirror, Self::Npmjs];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Tencent => "https://mirrors.tencent.com/npm/",
            Self::Huawei => "https://repo.huaweicloud.com/repository/npm/",
            Self::Npmmirror => "https://registry.npmmirror.com/",
            Self::Npmjs => "https://registry.npmjs.org/",
        }
    }

    pub const fn host(self) -> &'static str {
        match self {
            Self::Tencent => "mirrors.tencent.com",
            Self::Huawei => "repo.huaweicloud.com",
            Self::Npmmirror => "registry.npmmirror.com",
            Self::Npmjs => "registry.npmjs.org",
        }
    }

    pub const fn path_prefix(self) -> &'static str {
        match self {
            Self::Tencent => "/npm/",
            Self::Huawei => "/repository/npm/",
            Self::Npmmirror => "/",
            Self::Npmjs => "/",
        }
    }

    pub const fn index(self) -> u8 {
        match self {
            Self::Tencent => 0,
            Self::Huawei => 1,
            Self::Npmmirror => 2,
            Self::Npmjs => 3,
        }
    }

    pub const fn from_index(index: u8) -> Option<Self> {
        match index {
            0 => Some(Self::Tencent),
            1 => Some(Self::Huawei),
            2 => Some(Self::Npmmirror),
            3 => Some(Self::Npmjs),
            _ => None,
        }
    }

    pub fn parse_allowed(url: &str) -> Option<Self> {
        let parsed = parse_https_url(url)?;
        Self::ALL
            .iter()
            .copied()
            .find(|registry| registry.matches_parsed(&parsed))
    }

    fn matches_parsed(self, parsed: &ParsedHttpsUrl<'_>) -> bool {
        parsed.host == self.host() && parsed.path == self.path_prefix()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GrokNpmInstallPlan {
    version: String,
    registry: GrokNpmRegistry,
    package_integrity: String,
    platform_package: String,
    platform_integrity: String,
    allow_install_scripts: bool,
    dependency_name: Option<String>,
    dependency_version: Option<String>,
    reserve_budget_bytes: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GrokNpmPlanError {
    Missing,
    InvalidRegistry,
    InvalidVersion,
    InvalidIntegrity,
    InvalidPlatformPackage,
    LatestForbidden,
    InvalidSize,
    UnsupportedDependency,
    ArithmeticOverflow,
}

impl GrokNpmInstallPlan {
    pub fn new(
        version: impl Into<String>,
        registry: GrokNpmRegistry,
        package_integrity: impl Into<String>,
        platform_package: impl Into<String>,
        platform_integrity: impl Into<String>,
        allow_install_scripts: bool,
    ) -> Result<Self, GrokNpmPlanError> {
        let version = version.into();
        let package_integrity = package_integrity.into();
        let platform_package = platform_package.into();
        let platform_integrity = platform_integrity.into();
        validate_exact_version(&version)?;
        if !package_integrity.is_empty() {
            validate_integrity(&package_integrity)?;
        }
        if !platform_integrity.is_empty() {
            validate_integrity(&platform_integrity)?;
        }
        if !platform_package.is_empty() {
            validate_platform_package(&platform_package)?;
        }
        Ok(Self {
            version,
            registry,
            package_integrity,
            platform_package,
            platform_integrity,
            allow_install_scripts,
            dependency_name: None,
            dependency_version: None,
            reserve_budget_bytes: None,
        })
    }

    pub fn for_execution(
        version: impl Into<String>,
        registry: GrokNpmRegistry,
        allow_install_scripts: bool,
    ) -> Result<Self, GrokNpmPlanError> {
        Self::new(version, registry, "", "", "", allow_install_scripts)
    }

    pub fn with_allow_install_scripts(mut self, allow: bool) -> Self {
        self.allow_install_scripts = allow;
        self
    }

    pub fn with_npm_major(self, major: u32) -> Self {
        self.with_allow_install_scripts(npm_major_allows_scripts(major))
    }

    pub fn with_dependency(
        mut self,
        name: impl Into<String>,
        version: impl Into<String>,
    ) -> Result<Self, GrokNpmPlanError> {
        let name = name.into();
        let version = version.into();
        if name != CLOSED_DEP_IARNA_TOML_NAME {
            return Err(GrokNpmPlanError::UnsupportedDependency);
        }
        validate_exact_version(&version)?;
        if version.len() > CLOSED_DEP_VERSION_MAX_BYTES {
            return Err(GrokNpmPlanError::InvalidVersion);
        }
        self.dependency_name = Some(name);
        self.dependency_version = Some(version);
        Ok(self)
    }

    pub fn with_reserve_budget_bytes(mut self, budget: u64) -> Result<Self, GrokNpmPlanError> {
        if budget == 0 || budget > 2 * 1024 * 1024 * 1024 * 3 {
            return Err(GrokNpmPlanError::InvalidSize);
        }
        self.reserve_budget_bytes = Some(budget);
        Ok(self)
    }

    pub fn dependency(&self) -> Option<(&str, &str)> {
        match (&self.dependency_name, &self.dependency_version) {
            (Some(name), Some(ver)) => Some((name.as_str(), ver.as_str())),
            _ => None,
        }
    }

    pub fn dependency_name(&self) -> Option<&str> {
        self.dependency_name.as_deref()
    }

    pub fn dependency_version(&self) -> Option<&str> {
        self.dependency_version.as_deref()
    }

    pub fn reserve_budget_bytes(&self) -> Option<u64> {
        self.reserve_budget_bytes
    }

    pub fn has_dependency(&self) -> bool {
        self.dependency_name.is_some()
    }

    pub fn validate_for_tool(&self, tool: OfficialNpmTool) -> Result<(), GrokNpmPlanError> {
        if tool == OfficialNpmTool::Claude && self.has_dependency() {
            return Err(GrokNpmPlanError::UnsupportedDependency);
        }
        Ok(())
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn registry(&self) -> GrokNpmRegistry {
        self.registry
    }

    pub fn registry_url(&self) -> &'static str {
        self.registry.as_str()
    }

    pub fn package_integrity(&self) -> &str {
        &self.package_integrity
    }

    pub fn platform_package(&self) -> &str {
        &self.platform_package
    }

    pub fn platform_integrity(&self) -> &str {
        &self.platform_integrity
    }

    pub fn allow_install_scripts(&self) -> bool {
        self.allow_install_scripts
    }

    pub fn install_spec(&self) -> String {
        format!("{GROK_NPM_PACKAGE}@{}", self.version)
    }

    pub fn npm_argv(&self) -> Vec<String> {
        self.npm_argv_for(OfficialNpmTool::Grok)
    }

    pub fn npm_argv_for(&self, tool: OfficialNpmTool) -> Vec<String> {
        let mut argv = vec![
            "i".to_string(),
            "-g".to_string(),
            format!("{}@{}", tool.package(), self.version),
        ];
        if tool == OfficialNpmTool::Grok {
            if let (Some(name), Some(version)) = (&self.dependency_name, &self.dependency_version) {
                argv.push(format!("{name}@{version}"));
            }
        }
        argv.push(format!("--registry={}", self.registry.as_str()));
        argv.push(format!(
            "--{}:registry={}",
            tool.package()
                .split('/')
                .next()
                .expect("closed scoped package"),
            self.registry.as_str()
        ));
        if tool == OfficialNpmTool::Claude {
            // The official launcher links its native optional package. Do not
            // let an ambient omit=optional setting trigger an external fallback.
            argv.push("--include=optional".to_string());
        }
        if self.allow_install_scripts {
            argv.push(format!("--allow-scripts={}", tool.package()));
        }
        argv
    }

    pub fn env_pairs(&self) -> Vec<(&'static str, String)> {
        let mut pairs = vec![
            (GROK_NPM_REGISTRY_ENV, self.registry.as_str().to_string()),
            (GROK_NPM_VERSION_ENV, self.version.clone()),
            (
                GROK_NPM_ALLOW_SCRIPTS_ENV,
                if self.allow_install_scripts {
                    "1".to_string()
                } else {
                    "0".to_string()
                },
            ),
        ];
        if !self.package_integrity.is_empty() {
            pairs.push((
                GROK_NPM_PACKAGE_INTEGRITY_ENV,
                self.package_integrity.clone(),
            ));
        }
        if !self.platform_package.is_empty() {
            pairs.push((GROK_NPM_PLATFORM_PACKAGE_ENV, self.platform_package.clone()));
        }
        if !self.platform_integrity.is_empty() {
            pairs.push((
                GROK_NPM_PLATFORM_INTEGRITY_ENV,
                self.platform_integrity.clone(),
            ));
        }
        pairs
    }

    pub fn encode_control(&self) -> [u8; GROK_NPM_PLAN_CONTROL_BYTES] {
        encode_plan_control(Some(self))
    }
}

pub fn npm_install_argv_or_reject(
    plan: Option<&GrokNpmInstallPlan>,
) -> Result<Vec<String>, GrokNpmPlanError> {
    let plan = plan.ok_or(GrokNpmPlanError::Missing)?;
    plan.validate_for_tool(OfficialNpmTool::Grok)?;
    let argv = plan.npm_argv();
    if argv
        .iter()
        .any(|arg| arg.contains("@latest") || arg.contains("config"))
    {
        return Err(GrokNpmPlanError::LatestForbidden);
    }
    Ok(argv)
}

pub fn npm_major_allows_scripts(major: u32) -> bool {
    major >= GROK_NPM_ALLOW_SCRIPTS_MAJOR
}

pub fn parse_npm_major(output: &str) -> Option<u32> {
    let first = output.trim().lines().find(|line| !line.trim().is_empty())?;
    let major = first.trim().split('.').next()?;
    major.parse().ok()
}

pub fn version_is_at_least(local: &str, target: &str) -> bool {
    match (parse_release_tuple(local), parse_release_tuple(target)) {
        (Some(local), Some(target)) => local >= target,
        _ => false,
    }
}

fn parse_release_tuple(version: &str) -> Option<(u64, u64, u64)> {
    let core = version.split('-').next()?.split('+').next()?;
    let mut parts = core.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}

#[cfg(target_os = "macos")]
pub fn current_platform_package() -> Option<&'static str> {
    match std::env::consts::ARCH {
        "aarch64" => Some("@xai-official/grok-darwin-arm64"),
        "x86_64" => Some("@xai-official/grok-darwin-x64"),
        _ => None,
    }
}

#[cfg(target_os = "windows")]
pub fn current_platform_package() -> Option<&'static str> {
    match std::env::consts::ARCH {
        "x86_64" => Some("@xai-official/grok-win32-x64"),
        "aarch64" => Some("@xai-official/grok-win32-arm64"),
        _ => None,
    }
}

pub fn plan_from_env<F>(get: F) -> Result<GrokNpmInstallPlan, GrokNpmPlanError>
where
    F: Fn(&str) -> Option<String>,
{
    let registry = get(GROK_NPM_REGISTRY_ENV)
        .ok_or(GrokNpmPlanError::Missing)
        .and_then(|url| {
            GrokNpmRegistry::parse_allowed(&url).ok_or(GrokNpmPlanError::InvalidRegistry)
        })?;
    let version = get(GROK_NPM_VERSION_ENV).ok_or(GrokNpmPlanError::Missing)?;
    let package_integrity = get(GROK_NPM_PACKAGE_INTEGRITY_ENV).unwrap_or_default();
    let platform_package = get(GROK_NPM_PLATFORM_PACKAGE_ENV).unwrap_or_default();
    let platform_integrity = get(GROK_NPM_PLATFORM_INTEGRITY_ENV).unwrap_or_default();
    let allow = match get(GROK_NPM_ALLOW_SCRIPTS_ENV).as_deref() {
        Some("1") => true,
        Some("0") | None => false,
        Some(_) => return Err(GrokNpmPlanError::InvalidVersion),
    };
    GrokNpmInstallPlan::new(
        version,
        registry,
        package_integrity,
        platform_package,
        platform_integrity,
        allow,
    )
}

pub fn encode_plan_control(plan: Option<&GrokNpmInstallPlan>) -> [u8; GROK_NPM_PLAN_CONTROL_BYTES] {
    let mut bytes = [0_u8; GROK_NPM_PLAN_CONTROL_BYTES];
    bytes[..8].copy_from_slice(&PLAN_MAGIC);
    bytes[8] = GROK_NPM_PLAN_CONTROL_VERSION;
    let Some(plan) = plan else {
        return bytes;
    };
    bytes[9] = 1;
    bytes[10] = plan.registry.index();
    bytes[11] = u8::from(plan.allow_install_scripts);
    let version = plan.version.as_bytes();
    bytes[12] = version.len() as u8;
    bytes[13..13 + version.len()].copy_from_slice(version);
    if let (Some(name), Some(dep_version)) = (&plan.dependency_name, &plan.dependency_version) {
        if name == CLOSED_DEP_IARNA_TOML_NAME {
            bytes[45] = CLOSED_DEP_TAG_IARNA_TOML;
            let dep_bytes = dep_version.as_bytes();
            if dep_bytes.len() <= CLOSED_DEP_VERSION_MAX_BYTES {
                bytes[46] = dep_bytes.len() as u8;
                bytes[47..47 + dep_bytes.len()].copy_from_slice(dep_bytes);
            }
        }
    }
    if let Some(budget) = plan.reserve_budget_bytes {
        bytes[63..71].copy_from_slice(&budget.to_le_bytes());
    }
    bytes
}

pub fn decode_plan_control(bytes: &[u8]) -> Result<Option<GrokNpmInstallPlan>, GrokNpmPlanError> {
    if bytes.len() != GROK_NPM_PLAN_CONTROL_BYTES {
        return Err(GrokNpmPlanError::Missing);
    }
    if bytes[..8] != PLAN_MAGIC {
        return Err(GrokNpmPlanError::Missing);
    }
    if bytes[8] != GROK_NPM_PLAN_CONTROL_VERSION {
        return Err(GrokNpmPlanError::Missing);
    }
    match bytes[9] {
        0 => {
            if bytes[10..].iter().any(|byte| *byte != 0) {
                return Err(GrokNpmPlanError::Missing);
            }
            Ok(None)
        }
        1 => {
            let registry =
                GrokNpmRegistry::from_index(bytes[10]).ok_or(GrokNpmPlanError::InvalidRegistry)?;
            let allow = match bytes[11] {
                0 => false,
                1 => true,
                _ => return Err(GrokNpmPlanError::InvalidVersion),
            };
            let version_len = usize::from(bytes[12]);
            if version_len == 0 || version_len > VERSION_FIELD_BYTES {
                return Err(GrokNpmPlanError::InvalidVersion);
            }
            let version = std::str::from_utf8(&bytes[13..13 + version_len])
                .map_err(|_| GrokNpmPlanError::InvalidVersion)?;
            validate_exact_version(version)?;
            if bytes[13 + version_len..45].iter().any(|byte| *byte != 0) {
                return Err(GrokNpmPlanError::InvalidVersion);
            }
            let dep_tag = bytes[45];
            let dep_version_len = usize::from(bytes[46]);
            let dep_info = match dep_tag {
                CLOSED_DEP_TAG_NONE => {
                    if dep_version_len != 0 || bytes[47..63].iter().any(|byte| *byte != 0) {
                        return Err(GrokNpmPlanError::Missing);
                    }
                    None
                }
                CLOSED_DEP_TAG_IARNA_TOML => {
                    if dep_version_len == 0 || dep_version_len > CLOSED_DEP_VERSION_MAX_BYTES {
                        return Err(GrokNpmPlanError::InvalidVersion);
                    }
                    let dep_version = std::str::from_utf8(&bytes[47..47 + dep_version_len])
                        .map_err(|_| GrokNpmPlanError::InvalidVersion)?;
                    validate_exact_version(dep_version)?;
                    if bytes[47 + dep_version_len..63]
                        .iter()
                        .any(|byte| *byte != 0)
                    {
                        return Err(GrokNpmPlanError::InvalidVersion);
                    }
                    Some((CLOSED_DEP_IARNA_TOML_NAME, dep_version))
                }
                _ => return Err(GrokNpmPlanError::UnsupportedDependency),
            };
            let raw_budget = u64::from_le_bytes(
                bytes[63..71]
                    .try_into()
                    .map_err(|_| GrokNpmPlanError::InvalidSize)?,
            );
            if raw_budget == 0 || raw_budget > 2 * 1024 * 1024 * 1024 * 3 {
                return Err(GrokNpmPlanError::InvalidSize);
            }
            if bytes[71..80].iter().any(|byte| *byte != 0) {
                return Err(GrokNpmPlanError::Missing);
            }
            let mut plan = GrokNpmInstallPlan::for_execution(version, registry, allow)?;
            plan = plan.with_reserve_budget_bytes(raw_budget)?;
            if let Some((dep_name, dep_version)) = dep_info {
                plan = plan.with_dependency(dep_name, dep_version)?;
            }
            Ok(Some(plan))
        }
        _ => Err(GrokNpmPlanError::Missing),
    }
}

fn validate_exact_version(version: &str) -> Result<(), GrokNpmPlanError> {
    if version.eq_ignore_ascii_case("latest") || version.contains('@') || version.contains('/') {
        return Err(GrokNpmPlanError::LatestForbidden);
    }
    if version.is_empty() || version.len() > MAX_NORMALIZED_VERSION_BYTES {
        return Err(GrokNpmPlanError::InvalidVersion);
    }
    if !version
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
        || !version.chars().next().is_some_and(|c| c.is_ascii_digit())
    {
        return Err(GrokNpmPlanError::InvalidVersion);
    }
    Ok(())
}

fn validate_integrity(value: &str) -> Result<(), GrokNpmPlanError> {
    let digest = value
        .strip_prefix("sha512-")
        .ok_or(GrokNpmPlanError::InvalidIntegrity)?;
    if digest.is_empty()
        || digest.len() > 128
        || !digest
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '/' | '='))
    {
        return Err(GrokNpmPlanError::InvalidIntegrity);
    }
    Ok(())
}

fn validate_platform_package(package: &str) -> Result<(), GrokNpmPlanError> {
    let Some(suffix) = package.strip_prefix("@xai-official/grok-") else {
        return Err(GrokNpmPlanError::InvalidPlatformPackage);
    };
    if suffix.is_empty() || suffix.contains('@') || package.contains("latest") {
        return Err(GrokNpmPlanError::InvalidPlatformPackage);
    }
    Ok(())
}

struct ParsedHttpsUrl<'a> {
    host: &'a str,
    path: &'a str,
}

fn parse_https_url(url: &str) -> Option<ParsedHttpsUrl<'_>> {
    let without_scheme = url.strip_prefix("https://")?;
    if without_scheme.contains('@') {
        return None;
    }
    let slash = without_scheme.find('/')?;
    let host = &without_scheme[..slash];
    if host.is_empty() || host.contains(':') || host.contains('@') {
        return None;
    }
    let path = &without_scheme[slash..];
    if path.contains('?') || path.contains('#') || path.contains('\\') {
        return None;
    }
    Some(ParsedHttpsUrl { host, path })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claude_reuses_exact_version_registry_and_narrow_script_policy() {
        let plan =
            GrokNpmInstallPlan::for_execution("2.1.261", GrokNpmRegistry::Tencent, true).unwrap();
        let args = plan.npm_argv_for(OfficialNpmTool::Claude);
        assert!(args.contains(&"@anthropic-ai/claude-code@2.1.261".to_string()));
        assert!(args.contains(&"--include=optional".to_string()));
        assert!(args.contains(&"--allow-scripts=@anthropic-ai/claude-code".to_string()));
        assert!(args
            .iter()
            .all(|arg| !arg.contains("@latest") && !arg.contains("config")));
        assert!(!args.iter().any(|arg| arg.contains("xai")));
        assert!(OfficialNpmTool::Claude
            .current_platform_package()
            .unwrap()
            .starts_with("@anthropic-ai/claude-code-"));
    }

    fn sample_plan(allow: bool) -> GrokNpmInstallPlan {
        GrokNpmInstallPlan::new(
            "1.0.13",
            GrokNpmRegistry::Tencent,
            "sha512-rBMEx/7ND5DaBRGwzi6fEyf4ZWy4yStPnZ38UaIM2smZzg4E0fieDfLKPK8eRF4l2Xe4+5kSdCAVop99+whG4A==",
            "@xai-official/grok-darwin-arm64",
            "sha512-Nctnwkzj550E512RZ+n+IUuhqGPZ2L7z/ZTIlZdVn0KqZHfzC58vIZPMBzdcOkoK42IrJlLD6GS7Eo6c1y+VGw==",
            allow,
        )
        .expect("valid sample plan")
        .with_reserve_budget_bytes(1024 * 1024 * 3)
        .expect("valid budget")
    }

    #[test]
    fn registry_order_is_tencent_huawei_npmmirror_npmjs() {
        assert_eq!(
            GrokNpmRegistry::ALL.map(GrokNpmRegistry::as_str),
            [
                "https://mirrors.tencent.com/npm/",
                "https://repo.huaweicloud.com/repository/npm/",
                "https://registry.npmmirror.com/",
                "https://registry.npmjs.org/",
            ]
        );
    }

    #[test]
    fn argv_uses_exact_version_and_never_latest_or_npm_config() {
        let argv = sample_plan(false).npm_argv();
        assert_eq!(
            argv,
            [
                "i",
                "-g",
                "@xai-official/grok@1.0.13",
                "--registry=https://mirrors.tencent.com/npm/",
                "--@xai-official:registry=https://mirrors.tencent.com/npm/",
            ]
        );
        assert!(argv.iter().all(|arg| !arg.contains("@latest")));
        assert!(argv.iter().all(|arg| !arg.contains("config")));
        assert!(argv
            .iter()
            .all(|arg| !arg.contains("dangerously-allow-all")));
    }

    #[test]
    fn npm_12_adds_narrow_allow_scripts_and_npm_11_omits_it() {
        let with_flag = sample_plan(false).with_npm_major(12).npm_argv();
        assert!(with_flag.contains(&"--allow-scripts=@xai-official/grok".to_string()));
        let without_flag = sample_plan(false).with_npm_major(11).npm_argv();
        assert!(without_flag
            .iter()
            .all(|arg| !arg.contains("allow-scripts")));
        assert!(!npm_major_allows_scripts(11));
        assert!(npm_major_allows_scripts(12));
    }

    #[test]
    fn missing_plan_is_rejected_and_does_not_invent_latest() {
        assert_eq!(
            npm_install_argv_or_reject(None),
            Err(GrokNpmPlanError::Missing)
        );
        let argv = npm_install_argv_or_reject(Some(&sample_plan(true))).expect("plan argv");
        assert!(!argv.iter().any(|arg| arg.contains("@latest")));
    }

    #[test]
    fn registry_allowlist_rejects_userinfo_http_and_unknown_hosts() {
        assert!(GrokNpmRegistry::parse_allowed("https://mirrors.tencent.com/npm/").is_some());
        assert!(GrokNpmRegistry::parse_allowed("http://mirrors.tencent.com/npm/").is_none());
        assert!(
            GrokNpmRegistry::parse_allowed("https://user:pw@mirrors.tencent.com/npm/").is_none()
        );
        assert!(GrokNpmRegistry::parse_allowed("https://registry.npmjs.org:8443/").is_none());
        assert!(GrokNpmRegistry::parse_allowed("https://example.com/npm/").is_none());
        assert!(GrokNpmRegistry::parse_allowed("https://mirrors.tencent.com/npm/?x=1").is_none());
    }

    #[test]
    fn latest_is_never_a_valid_plan_version() {
        assert_eq!(
            GrokNpmInstallPlan::for_execution("latest", GrokNpmRegistry::Npmjs, false),
            Err(GrokNpmPlanError::LatestForbidden)
        );
        assert_eq!(
            GrokNpmInstallPlan::for_execution("@latest", GrokNpmRegistry::Npmmirror, false),
            Err(GrokNpmPlanError::LatestForbidden)
        );
    }

    #[test]
    fn current_platform_package_is_a_product_host_optional() {
        let package = current_platform_package().expect("product host");
        assert!(
            package.starts_with("@xai-official/grok-darwin-")
                || package.starts_with("@xai-official/grok-win32-")
        );
    }

    #[test]
    fn compact_control_round_trips_and_absent_is_empty() {
        let plan = sample_plan(true);
        assert_eq!(GROK_NPM_PLAN_CONTROL_BYTES, crate::BRIDGE_CONTROL_BYTES);
        let encoded = plan.encode_control();
        assert_eq!(encoded.len(), GROK_NPM_PLAN_CONTROL_BYTES);
        let decoded = decode_plan_control(&encoded)
            .expect("decode")
            .expect("present");
        assert_eq!(decoded.version(), "1.0.13");
        assert_eq!(decoded.registry(), GrokNpmRegistry::Tencent);
        assert!(decoded.allow_install_scripts());
        let absent = encode_plan_control(None);
        assert_eq!(decode_plan_control(&absent), Ok(None));
    }

    #[test]
    fn env_pairs_can_rebuild_a_plan_without_trusting_latest() {
        let plan = sample_plan(true);
        let pairs = plan.env_pairs();
        let get = |key: &str| {
            pairs
                .iter()
                .find(|(name, _)| *name == key)
                .map(|(_, value)| value.clone())
        };
        let rebuilt = plan_from_env(get).expect("env plan");
        assert_eq!(rebuilt.install_spec(), "@xai-official/grok@1.0.13");
        assert_eq!(rebuilt.registry(), GrokNpmRegistry::Tencent);
        assert!(get(GROK_NPM_REGISTRY_ENV).is_some());
        assert_eq!(
            plan_from_env(|_| Some("https://evil.example/npm/".to_string())),
            Err(GrokNpmPlanError::InvalidRegistry)
        );
    }

    #[test]
    fn parse_npm_major_reads_the_first_numeric_line() {
        assert_eq!(parse_npm_major("11.4.2\n"), Some(11));
        assert_eq!(parse_npm_major("10.9.0"), Some(10));
        assert_eq!(parse_npm_major("not a version"), None);
        assert!(version_is_at_least("1.0.13", "1.0.13"));
        assert!(version_is_at_least("1.0.14", "1.0.13"));
        assert!(!version_is_at_least("1.0.12", "1.0.13"));
    }

    #[test]
    fn control_v2_round_trips_dependency_and_budget() {
        let plan = sample_plan(true)
            .with_dependency(CLOSED_DEP_IARNA_TOML_NAME, "3.0.0")
            .unwrap()
            .with_reserve_budget_bytes(3072)
            .unwrap();
        let encoded = plan.encode_control();
        let decoded = decode_plan_control(&encoded).unwrap().unwrap();
        assert_eq!(decoded.version(), "1.0.13");
        assert_eq!(
            decoded.dependency(),
            Some((CLOSED_DEP_IARNA_TOML_NAME, "3.0.0"))
        );
        assert_eq!(decoded.reserve_budget_bytes(), Some(3072));
        let grok_argv = decoded.npm_argv_for(OfficialNpmTool::Grok);
        assert!(grok_argv.contains(&"@iarna/toml@3.0.0".to_string()));
    }

    #[test]
    fn control_v2_rejects_dirty_tail_and_v1_and_unknown_tag() {
        let plan = sample_plan(true)
            .with_dependency(CLOSED_DEP_IARNA_TOML_NAME, "3.0.0")
            .unwrap();
        let mut encoded = plan.encode_control();

        // 1. Dirty unused tail (byte 75 != 0) fails closed with Missing
        encoded[75] = 0xAA;
        assert_eq!(
            decode_plan_control(&encoded),
            Err(GrokNpmPlanError::Missing)
        );
        encoded[75] = 0;

        // 2. Older protocol v1 payload fails with Missing
        encoded[8] = 1;
        assert_eq!(
            decode_plan_control(&encoded),
            Err(GrokNpmPlanError::Missing)
        );
        encoded[8] = GROK_NPM_PLAN_CONTROL_VERSION;

        // 3. Unknown dep tag fails closed with UnsupportedDependency
        encoded[45] = 2;
        assert_eq!(
            decode_plan_control(&encoded),
            Err(GrokNpmPlanError::UnsupportedDependency)
        );
        encoded[45] = CLOSED_DEP_TAG_IARNA_TOML;

        // 4. Zero budget fails closed with InvalidSize
        encoded[63..71].copy_from_slice(&0_u64.to_le_bytes());
        assert_eq!(
            decode_plan_control(&encoded),
            Err(GrokNpmPlanError::InvalidSize)
        );
    }

    #[test]
    fn claude_never_installs_toml_and_rejects_plan_with_dependency() {
        let grok_plan = sample_plan(true)
            .with_dependency(CLOSED_DEP_IARNA_TOML_NAME, "3.0.0")
            .unwrap();
        assert_eq!(
            grok_plan.validate_for_tool(OfficialNpmTool::Claude),
            Err(GrokNpmPlanError::UnsupportedDependency)
        );
        let claude_plan = sample_plan(true);
        assert_eq!(
            claude_plan.validate_for_tool(OfficialNpmTool::Claude),
            Ok(())
        );
        let claude_argv = claude_plan.npm_argv_for(OfficialNpmTool::Claude);
        assert!(!claude_argv.iter().any(|arg| arg.contains("toml")));
    }

    #[test]
    fn for_execution_plan_lacks_dependency_proving_macos_must_use_plan_for_registry() {
        let execution_plan =
            GrokNpmInstallPlan::for_execution("1.0.13", GrokNpmRegistry::Tencent, true).unwrap();
        let grok_argv = execution_plan.npm_argv_for(OfficialNpmTool::Grok);
        // for_execution does NOT include @iarna/toml@3.0.0, which catches the old macOS bug!
        assert!(!grok_argv.iter().any(|arg| arg.contains("@iarna/toml")));
    }
}
