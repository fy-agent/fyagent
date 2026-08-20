//! QoderWork CN MCP sync and import.
//!
//! Canonical live file: `~/.qoderworkcn/mcp.json` (`mcpServers` map).
//! Skip writes when neither the home directory nor the file exists.
//! QoderWork is not an [`AppType`](crate::app_config::AppType).

use serde_json::{Map, Value};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::app_config::{McpApps, McpServer, MultiAppConfig};
use crate::config::{atomic_write, get_home_dir};
use crate::error::AppError;

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

fn read_json_value(path: &Path) -> Result<Value, AppError> {
    if !path.exists() {
        return Ok(serde_json::json!({}));
    }
    let content = fs::read_to_string(path).map_err(|e| AppError::io(path, e))?;
    serde_json::from_str(&content).map_err(|e| AppError::json(path, e))
}

fn backup_canonical_if_present() -> Result<(), AppError> {
    let path = canonical_mcp_path();
    if !path.exists() {
        return Ok(());
    }
    let backup = backup_mcp_path();
    fs::copy(&path, &backup).map_err(|e| AppError::io(&backup, e))?;
    Ok(())
}

fn write_json_value(path: &Path, value: &Value) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
    }
    let json =
        serde_json::to_string_pretty(value).map_err(|e| AppError::JsonSerialize { source: e })?;
    backup_canonical_if_present()?;
    atomic_write(path, json.as_bytes())
}

fn read_mcp_servers_map_from(path: &Path) -> Result<HashMap<String, Value>, AppError> {
    if !path.exists() {
        return Ok(HashMap::new());
    }
    let root = read_json_value(path)?;
    Ok(root
        .get("mcpServers")
        .and_then(|value| value.as_object())
        .map(|object| {
            object
                .iter()
                .map(|(id, spec)| (id.clone(), spec.clone()))
                .collect()
        })
        .unwrap_or_default())
}

fn read_live_mcp_servers_map() -> Result<HashMap<String, Value>, AppError> {
    read_mcp_servers_map_from(&canonical_mcp_path())
}

fn set_live_mcp_servers_map(servers: &HashMap<String, Value>) -> Result<(), AppError> {
    let path = canonical_mcp_path();
    let mut root = if path.exists() {
        read_json_value(&path)?
    } else {
        serde_json::json!({})
    };
    let obj = root
        .as_object_mut()
        .ok_or_else(|| AppError::Config("~/.qoderworkcn/mcp.json 根必须是对象".into()))?;
    let mut out = Map::new();
    for (id, spec) in servers {
        let mut object = spec
            .as_object()
            .cloned()
            .ok_or_else(|| AppError::McpValidation(format!("MCP 服务器 '{id}' 不是对象")))?;
        object.remove("enabled");
        object.remove("source");
        object.remove("id");
        object.remove("name");
        object.remove("description");
        object.remove("tags");
        object.remove("homepage");
        object.remove("docs");
        out.insert(id.clone(), Value::Object(object));
    }
    obj.insert("mcpServers".into(), Value::Object(out));
    write_json_value(&path, &root)
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
