//! First-party desktop source descriptors. Remote URLs never become IPC
//! capabilities; parsers accept only closed platform/arch branches and
//! product-owned HTTPS allowlists.

mod qoderwork;
mod traework;
mod workbuddy;

use sha2::{Digest, Sha256};
use url::Url;

use crate::services::external_agents::AgentCatalogId;

pub use qoderwork::{resolve_qoderwork_source, QODERWORK_REDIRECT_HOSTS};
pub use traework::{
    parse_traework_latest, TRAEWORK_DOWNLOAD_HOSTS, TRAEWORK_METADATA_ENDPOINTS,
    TRAEWORK_METADATA_HOSTS,
};
pub use workbuddy::{
    parse_workbuddy_update, workbuddy_update_url, WORKBUDDY_DOWNLOAD_HOSTS,
    WORKBUDDY_METADATA_HOSTS,
};

pub const SOURCE_RELEASE_ID_SCHEMA: &str = "fyagent-agent-release-v1";
pub const MAX_SOURCE_METADATA_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentPlatform {
    Windows,
    Macos,
}

impl AgentPlatform {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Windows => "windows",
            Self::Macos => "macos",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentArch {
    X86_64,
    Aarch64,
}

impl AgentArch {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::X86_64 => "x86_64",
            Self::Aarch64 => "aarch64",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageFormat {
    Dmg,
    Exe,
}

impl PackageFormat {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Dmg => "dmg",
            Self::Exe => "exe",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedDesktopSource {
    pub product: AgentCatalogId,
    pub platform: AgentPlatform,
    pub architecture: AgentArch,
    pub format: PackageFormat,
    pub release_id: String,
    pub display_version: Option<String>,
    pub download_url: Url,
    pub versionless_latest: bool,
    pub official_page: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceResolveError {
    PlatformUnsupported,
    SchemaInvalid,
    HostRejected,
    ArtifactRejected,
    CodePackageRejected,
    Cancelled,
}

pub fn current_host_target() -> Option<(AgentPlatform, AgentArch)> {
    #[cfg(target_os = "macos")]
    {
        let arch = if cfg!(target_arch = "aarch64") {
            AgentArch::Aarch64
        } else if cfg!(target_arch = "x86_64") {
            AgentArch::X86_64
        } else {
            return None;
        };
        Some((AgentPlatform::Macos, arch))
    }
    #[cfg(target_os = "windows")]
    {
        let arch = if cfg!(target_arch = "aarch64") {
            AgentArch::Aarch64
        } else if cfg!(target_arch = "x86_64") {
            AgentArch::X86_64
        } else {
            return None;
        };
        Some((AgentPlatform::Windows, arch))
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        None
    }
}

pub fn https_url_on_allowlist(url: &Url, hosts: &[&str]) -> Result<(), SourceResolveError> {
    if url.scheme() != "https" || url.host_str().is_none() {
        return Err(SourceResolveError::HostRejected);
    }
    if url.port().is_some() {
        return Err(SourceResolveError::HostRejected);
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(SourceResolveError::HostRejected);
    }
    let host = url.host_str().unwrap_or_default();
    if !hosts.contains(&host) {
        return Err(SourceResolveError::HostRejected);
    }
    Ok(())
}

pub fn opaque_release_id(fields: &[(&str, &str)]) -> String {
    let mut canonical = format!("schema={SOURCE_RELEASE_ID_SCHEMA}\n");
    for (key, value) in fields {
        canonical.push_str(key);
        canonical.push('=');
        canonical.push_str(value);
        canonical.push('\n');
    }
    format!("v1:{:x}", Sha256::digest(canonical.as_bytes()))
}

pub fn bounded_version(value: &str) -> Option<&str> {
    if value.is_empty() || value.len() > 64 {
        return None;
    }
    if !value
        .bytes()
        .all(|byte| byte.is_ascii_digit() || byte == b'.')
        || value.starts_with('.')
        || value.ends_with('.')
        || value.contains("..")
    {
        return None;
    }
    Some(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn https_allowlist_rejects_http_userinfo_and_nondefault_ports() {
        let hosts = ["static.qoder.com.cn"];
        let ok = Url::parse(
            "https://static.qoder.com.cn/qoder-work-cn/releases/latest/QoderWorkCN-arm64.dmg",
        )
        .unwrap();
        assert!(https_url_on_allowlist(&ok, &hosts).is_ok());
        let http = Url::parse(
            "http://static.qoder.com.cn/qoder-work-cn/releases/latest/QoderWorkCN-arm64.dmg",
        )
        .unwrap();
        assert_eq!(
            https_url_on_allowlist(&http, &hosts),
            Err(SourceResolveError::HostRejected)
        );
        let userinfo = Url::parse(
            "https://user:pass@static.qoder.com.cn/qoder-work-cn/releases/latest/QoderWorkCN-arm64.dmg",
        )
        .unwrap();
        assert_eq!(
            https_url_on_allowlist(&userinfo, &hosts),
            Err(SourceResolveError::HostRejected)
        );
        let port = Url::parse(
            "https://static.qoder.com.cn:8443/qoder-work-cn/releases/latest/QoderWorkCN-arm64.dmg",
        )
        .unwrap();
        assert_eq!(
            https_url_on_allowlist(&port, &hosts),
            Err(SourceResolveError::HostRejected)
        );
    }
}
