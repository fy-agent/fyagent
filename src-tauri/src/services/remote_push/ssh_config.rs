//! Read-only `~/.ssh/config` host discovery and strict SSH target validation.

use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SshHostEntry {
    pub alias: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identity_file: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SshTargetSpec {
    /// Either a `~/.ssh/config` Host alias or a hostname / IP address.
    pub host: String,
    #[serde(default)]
    pub user: Option<String>,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub identity_file: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ValidatedSshTarget {
    pub destination: String,
    pub port: Option<u16>,
    pub identity_file: Option<String>,
}

fn is_safe_ssh_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 253
        && !value.starts_with('-')
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | ':' | '@'))
}

pub fn validate_ssh_target(spec: &SshTargetSpec) -> Result<ValidatedSshTarget, String> {
    let raw_host = spec.host.trim();
    if raw_host.is_empty() {
        return Err("请填写 SSH 主机名或选择 ~/.ssh/config 中的主机别名".to_string());
    }
    if !is_safe_ssh_token(raw_host) {
        return Err("SSH 主机名包含不允许的字符或以 '-' 开头".to_string());
    }

    let destination = match spec
        .user
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        Some(user) => {
            if user.contains('@') || !is_safe_ssh_token(user) {
                return Err("SSH 用户名格式无效".to_string());
            }
            if raw_host.contains('@') {
                raw_host.to_string()
            } else {
                format!("{user}@{raw_host}")
            }
        }
        None => raw_host.to_string(),
    };

    if let Some(port) = spec.port {
        if port == 0 {
            return Err("SSH 端口号必须为 1–65535".to_string());
        }
    }

    let identity_file = match spec
        .identity_file
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        Some(path) => {
            if path.starts_with('-')
                || path
                    .chars()
                    .any(|c| c.is_control() || matches!(c, ';' | '|' | '&' | '`' | '$'))
            {
                return Err("SSH 私钥路径包含非法字符".to_string());
            }
            Some(path.to_string())
        }
        None => None,
    };

    Ok(ValidatedSshTarget {
        destination,
        port: spec.port,
        identity_file,
    })
}

pub fn parse_ssh_config_text(content: &str) -> Vec<SshHostEntry> {
    let mut entries: Vec<SshHostEntry> = Vec::new();
    let mut seen = HashSet::new();
    let mut current_indices: Vec<usize> = Vec::new();

    for raw_line in content.lines() {
        let line = raw_line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let mut parts = line.splitn(2, |c: char| c.is_whitespace() || c == '=');
        let Some(keyword) = parts.next().map(|s| s.trim().to_ascii_lowercase()) else {
            continue;
        };
        let value = parts
            .next()
            .map(|s| s.trim().trim_start_matches('=').trim())
            .unwrap_or("");
        if value.is_empty() {
            continue;
        }

        match keyword.as_str() {
            "host" => {
                current_indices.clear();
                for token in value.split_whitespace() {
                    if token.contains('*')
                        || token.contains('?')
                        || token.starts_with('!')
                        || !is_safe_ssh_token(token)
                    {
                        continue;
                    }
                    if seen.insert(token.to_string()) {
                        let idx = entries.len();
                        entries.push(SshHostEntry {
                            alias: token.to_string(),
                            host_name: None,
                            user: None,
                            port: None,
                            identity_file: None,
                        });
                        current_indices.push(idx);
                    }
                }
            }
            "match" => {
                current_indices.clear();
            }
            "hostname" => {
                for &idx in &current_indices {
                    if entries[idx].host_name.is_none() {
                        entries[idx].host_name = Some(value.to_string());
                    }
                }
            }
            "user" => {
                for &idx in &current_indices {
                    if entries[idx].user.is_none() {
                        entries[idx].user = Some(value.to_string());
                    }
                }
            }
            "port" => {
                if let Ok(port) = value.parse::<u16>() {
                    for &idx in &current_indices {
                        if entries[idx].port.is_none() {
                            entries[idx].port = Some(port);
                        }
                    }
                }
            }
            "identityfile" => {
                for &idx in &current_indices {
                    if entries[idx].identity_file.is_none() {
                        entries[idx].identity_file = Some(value.trim_matches('"').to_string());
                    }
                }
            }
            _ => {}
        }
    }

    entries
}

pub fn list_ssh_config_hosts() -> Vec<SshHostEntry> {
    let Some(home) = dirs::home_dir() else {
        return Vec::new();
    };
    let path: PathBuf = home.join(".ssh").join("config");
    let Ok(content) = fs::read_to_string(path) else {
        return Vec::new();
    };
    parse_ssh_config_text(&content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_named_hosts_and_skips_wildcards() {
        let cfg = r#"
Host *
  ServerAliveInterval 60

Host dev-box gpu-1
  HostName 10.0.0.12
  User alice
  Port 2222
  IdentityFile ~/.ssh/id_ed25519
"#;
        let hosts = parse_ssh_config_text(cfg);
        assert_eq!(hosts.len(), 2);
        assert_eq!(hosts[0].alias, "dev-box");
        assert_eq!(hosts[0].host_name.as_deref(), Some("10.0.0.12"));
        assert_eq!(hosts[0].user.as_deref(), Some("alice"));
        assert_eq!(hosts[0].port, Some(2222));
        assert_eq!(hosts[1].alias, "gpu-1");
    }

    #[test]
    fn rejects_option_injection_in_host_or_identity() {
        assert!(validate_ssh_target(&SshTargetSpec {
            host: "-oProxyCommand=evil".to_string(),
            user: None,
            port: None,
            identity_file: None,
        })
        .is_err());

        assert!(validate_ssh_target(&SshTargetSpec {
            host: "dev-box".to_string(),
            user: Some("alice".to_string()),
            port: Some(22),
            identity_file: Some("~/.ssh/key;rm -rf /".to_string()),
        })
        .is_err());

        let valid = validate_ssh_target(&SshTargetSpec {
            host: "dev-box".to_string(),
            user: Some("alice".to_string()),
            port: Some(2222),
            identity_file: Some("~/.ssh/id_ed25519".to_string()),
        })
        .unwrap();
        assert_eq!(valid.destination, "alice@dev-box");
    }
}
