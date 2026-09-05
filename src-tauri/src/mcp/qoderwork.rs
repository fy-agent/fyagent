//! QoderWork CN MCP sync and import.
//!
//! Canonical live file: `~/.qoderworkcn/mcp.json` (`mcpServers` map).
//! Skip writes when neither the home directory nor the file exists.
//! QoderWork is not an [`AppType`](crate::app_config::AppType).

use serde_json::Value;
use std::collections::HashMap;
#[cfg(test)]
use std::fs;
#[cfg(test)]
use std::path::Path;
use std::path::PathBuf;

use crate::app_config::{McpApps, McpServer, MultiAppConfig};
use crate::config::get_home_dir;
use crate::error::AppError;

use super::json_document::{read_servers as read_mcp_servers_map_from, write_servers};
use super::validation::validate_server_spec;

fn qoderwork_home() -> PathBuf {
    get_home_dir().join(".qoderworkcn")
}

fn canonical_mcp_path() -> PathBuf {
    qoderwork_home().join("mcp.json")
}

fn backup_mcp_path() -> PathBuf {
    qoderwork_home().join("mcp.json.backup")
}

fn should_sync_qoderwork_mcp() -> bool {
    qoderwork_home().exists() || canonical_mcp_path().exists()
}

fn read_live_mcp_servers_map() -> Result<HashMap<String, Value>, AppError> {
    read_mcp_servers_map_from(&canonical_mcp_path())
}

fn set_live_mcp_servers_map(servers: &HashMap<String, Value>) -> Result<(), AppError> {
    write_servers(
        &canonical_mcp_path(),
        &backup_mcp_path(),
        "~/.qoderworkcn/mcp.json 根必须是对象",
        servers,
    )
}

fn normalize_imported_spec(mut spec: Value) -> Value {
    if spec.get("type").and_then(|value| value.as_str()) == Some("streamable-http") {
        if let Some(object) = spec.as_object_mut() {
            object.insert("type".into(), Value::String("http".into()));
        }
    }
    spec
}

/// 从 QoderWork CN live 配置导入 mcpServers。
pub fn import_from_qoderwork(config: &mut MultiAppConfig) -> Result<usize, AppError> {
    let path = canonical_mcp_path();
    if !path.exists() {
        return Ok(0);
    }
    let map = read_mcp_servers_map_from(&path)?;
    let servers = config.mcp.servers.get_or_insert_with(HashMap::new);
    let mut changed = 0;
    for (id, spec) in map {
        let spec = normalize_imported_spec(spec);
        if let Err(error) = validate_server_spec(&spec) {
            log::warn!("跳过无效 QoderWork MCP 服务器 '{id}': {error}");
            continue;
        }
        if let Some(existing) = servers.get_mut(&id) {
            if !existing.apps.qoderwork {
                existing.apps.qoderwork = true;
                changed += 1;
            }
        } else {
            servers.insert(
                id.clone(),
                McpServer {
                    id: id.clone(),
                    name: id.clone(),
                    server: spec,
                    apps: McpApps {
                        qoderwork: true,
                        ..McpApps::default()
                    },
                    description: None,
                    homepage: None,
                    docs: None,
                    tags: Vec::new(),
                },
            );
            changed += 1;
        }
    }
    Ok(changed)
}

pub fn sync_single_server_to_qoderwork(
    _config: &MultiAppConfig,
    id: &str,
    server_spec: &Value,
) -> Result<(), AppError> {
    if !should_sync_qoderwork_mcp() {
        return Ok(());
    }
    let mut current = read_live_mcp_servers_map()?;
    current.insert(id.to_string(), server_spec.clone());
    set_live_mcp_servers_map(&current)
}

pub fn remove_server_from_qoderwork(id: &str) -> Result<(), AppError> {
    if !should_sync_qoderwork_mcp() {
        return Ok(());
    }
    let mut current = read_live_mcp_servers_map()?;
    current.remove(id);
    set_live_mcp_servers_map(&current)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use serial_test::serial;
    use std::ffi::OsString;
    use tempfile::TempDir;

    struct EnvGuard(Option<OsString>);
    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match self.0.take() {
                Some(value) => std::env::set_var("FYAGENT_TEST_HOME", value),
                None => std::env::remove_var("FYAGENT_TEST_HOME"),
            }
        }
    }

    fn with_test_home<T>(run: impl FnOnce(&Path) -> T) -> T {
        let temp = TempDir::new().expect("tempdir");
        let _guard = EnvGuard(std::env::var_os("FYAGENT_TEST_HOME"));
        std::env::set_var("FYAGENT_TEST_HOME", temp.path());
        run(temp.path())
    }

    #[test]
    #[serial]
    fn skips_write_when_qoderwork_home_and_mcp_file_are_absent() {
        with_test_home(|home| {
            sync_single_server_to_qoderwork(
                &Default::default(),
                "demo",
                &json!({ "command": "echo" }),
            )
            .expect("skip write");
            assert!(!home.join(".qoderworkcn").exists());
            assert!(!canonical_mcp_path().exists());
        });
    }

    #[test]
    #[serial]
    fn writes_canonical_mcp_json_and_backup_when_home_exists() {
        with_test_home(|home| {
            fs::create_dir_all(home.join(".qoderworkcn")).expect("create qoderwork home");
            sync_single_server_to_qoderwork(
                &Default::default(),
                "demo",
                &json!({ "command": "echo", "args": ["hi"] }),
            )
            .expect("write mcp");
            let written = fs::read_to_string(canonical_mcp_path()).expect("read canonical");
            assert!(written.contains("\"demo\""));
            assert!(written.contains("echo"));
            remove_server_from_qoderwork("demo").expect("remove");
            let after = fs::read_to_string(canonical_mcp_path()).expect("read after remove");
            assert!(!after.contains("\"demo\""));
            assert!(backup_mcp_path().exists());
        });
    }

    #[test]
    #[serial]
    fn imports_streamable_http_as_http() {
        with_test_home(|home| {
            let dir = home.join(".qoderworkcn");
            fs::create_dir_all(&dir).expect("create qoderwork home");
            fs::write(
                dir.join("mcp.json"),
                r#"{"mcpServers":{"remote":{"type":"streamable-http","url":"https://example.test/mcp"}}}"#,
            )
            .expect("write mcp");
            let mut config = MultiAppConfig::default();
            let changed = import_from_qoderwork(&mut config).expect("import");
            assert_eq!(changed, 1);
            let server = config
                .mcp
                .servers
                .as_ref()
                .expect("servers")
                .get("remote")
                .expect("imported");
            assert!(server.apps.qoderwork);
            assert_eq!(
                server.server.get("type").and_then(|value| value.as_str()),
                Some("http")
            );
        });
    }
}
