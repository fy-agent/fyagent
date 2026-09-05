//! Claude-shaped MCP documents. Vendor adapters retain path and import policy.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde_json::{Map, Value};

use crate::config::{atomic_write, read_json_file};
use crate::error::AppError;

fn read_root(path: &Path) -> Result<Value, AppError> {
    if path.exists() {
        read_json_file(path)
    } else {
        Ok(serde_json::json!({}))
    }
}

pub(super) fn read_servers(path: &Path) -> Result<HashMap<String, Value>, AppError> {
    let root = read_root(path)?;
    Ok(root
        .get("mcpServers")
        .and_then(Value::as_object)
        .map(|servers| {
            servers
                .iter()
                .map(|(id, spec)| (id.clone(), spec.clone()))
                .collect()
        })
        .unwrap_or_default())
}

pub(super) fn write_servers(
    path: &Path,
    backup: &Path,
    root_error: &str,
    servers: &HashMap<String, Value>,
) -> Result<(), AppError> {
    let mut root = read_root(path)?;
    let object = root
        .as_object_mut()
        .ok_or_else(|| AppError::Config(root_error.into()))?;
    let mut projected = Map::new();
    for (id, spec) in servers {
        let mut spec = spec
            .as_object()
            .cloned()
            .ok_or_else(|| AppError::McpValidation(format!("MCP 服务器 '{id}' 不是对象")))?;
        for key in [
            "enabled",
            "source",
            "id",
            "name",
            "description",
            "tags",
            "homepage",
            "docs",
        ] {
            spec.remove(key);
        }
        projected.insert(id.clone(), Value::Object(spec));
    }
    object.insert("mcpServers".into(), Value::Object(projected));
    // Preserve existing JSON ordering; the generic sorted writer has different semantics.
    let json =
        serde_json::to_string_pretty(&root).map_err(|source| AppError::JsonSerialize { source })?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| AppError::io(parent, error))?;
    }
    if path.exists() {
        fs::copy(path, backup).map_err(|error| AppError::io(backup, error))?;
    }
    atomic_write(path, json.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn projection_preserves_unknown_fields_and_backs_up_exact_original_bytes() {
        let home = tempfile::tempdir().unwrap();
        let path = home.path().join("mcp.json");
        let backup = home.path().join("mcp.json.backup");
        let original = "{\"custom\": {\"enabled\": false}, \"mcpServers\": {}}\n";
        fs::write(&path, original).unwrap();
        let servers = HashMap::from([(
            "demo".into(),
            json!({
                "command":"echo", "args":["hello"], "custom":{"enabled":true},
                "env":{"TOKEN":"retain-executable-value"}, "enabled":true, "source":"local",
                "id":"id", "name":"name", "description":"desc", "tags":[], "homepage":"url", "docs":"url"
            }),
        )]);
        write_servers(&path, &backup, "invalid root", &servers).unwrap();
        assert_eq!(fs::read(&backup).unwrap(), original.as_bytes());
        let written: Value = read_json_file(&path).unwrap();
        assert_eq!(written["custom"], json!({"enabled":false}));
        assert_eq!(
            written["mcpServers"]["demo"],
            json!({
                "command":"echo", "args":["hello"], "custom":{"enabled":true},
                "env":{"TOKEN":"retain-executable-value"}
            })
        );
    }

    #[test]
    fn invalid_input_never_overwrites_original_or_backup() {
        let home = tempfile::tempdir().unwrap();
        let path = home.path().join("mcp.json");
        let backup = home.path().join("mcp.json.backup");
        for original in ["not json", "[]", "null"] {
            fs::write(&path, original).unwrap();
            fs::write(&backup, "old backup").unwrap();
            assert!(write_servers(&path, &backup, "invalid root", &HashMap::new()).is_err());
            assert_eq!(fs::read_to_string(&path).unwrap(), original);
            assert_eq!(fs::read_to_string(&backup).unwrap(), "old backup");
        }
        fs::write(&path, "{}").unwrap();
        assert!(write_servers(
            &path,
            &backup,
            "invalid root",
            &HashMap::from([("bad".into(), json!(1))])
        )
        .is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "{}");
        assert_eq!(fs::read_to_string(&backup).unwrap(), "old backup");
    }

    #[test]
    fn backup_failure_aborts_write_and_missing_file_reads_as_empty() {
        let home = tempfile::tempdir().unwrap();
        let path = home.path().join("mcp.json");
        assert!(read_servers(&path).unwrap().is_empty());
        fs::write(&path, "{}").unwrap();
        let backup = home.path().join("missing-parent/backup");
        assert!(write_servers(&path, &backup, "invalid root", &HashMap::new()).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "{}");
    }
}
