//! Secret-free TRAE SOLO CN custom-model observation.
//!
//! Work CN listing is owned by TRAE cloud `model` / `model_list`. FyAgent must
//! not write `state.vscdb` or the model-list backup: launch overwrites local
//! rows that were never registered with `add_custom_model`.

use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::OnceLock,
};

use hmac::{Hmac, Mac};
use rusqlite::{params, types::ValueRef, Connection};
use serde::Serialize;
use serde_json::{json, Value};
use sha2::Sha256;
use uuid::Uuid;

use super::traework::{TraeErrorCode, TraeErrorDto};

const ITEM_TABLE: &str = "ItemTable";
const MAP_SUFFIX: &str = "AI.agent.model.model_list_map";
const LITE_LIST: &str = "solo_work_lite";
const REMOTE_LIST: &str = "solo_work_remote";
const AGENT_LIST: &str = "solo_agent";
const AGENT_LITE_LIST: &str = "solo_agent_lite";
const AGENT_REMOTE_LIST: &str = "solo_agent_remote";
const CODER_LIST: &str = "solo_coder";
const DESIGN_LITE_LIST: &str = "solo_design_lite";
const DESIGN_REMOTE_LIST: &str = "solo_design_remote";
const ASSISTANT_LIST: &str = "assistant";
const CHAT_AGENT_LIST: &str = "agent";
const WORK_LISTS: [&str; 10] = [
    LITE_LIST,
    REMOTE_LIST,
    AGENT_LIST,
    AGENT_LITE_LIST,
    AGENT_REMOTE_LIST,
    CODER_LIST,
    DESIGN_LITE_LIST,
    DESIGN_REMOTE_LIST,
    ASSISTANT_LIST,
    CHAT_AGENT_LIST,
];
const MAX_MODELS: usize = 1_000;
type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TraeWorkModelIdsResult {
    pub model_ids: Vec<String>,
    pub revision: Option<String>,
    pub truncated: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct TraePaths {
    pub(crate) db: PathBuf,
}

impl TraePaths {
    pub(crate) fn from_home(home: &Path) -> Self {
        #[cfg(target_os = "windows")]
        let directory = home
            .join("AppData")
            .join("Roaming")
            .join("TRAE SOLO CN")
            .join("User")
            .join("globalStorage");
        #[cfg(target_os = "macos")]
        let directory = home
            .join("Library")
            .join("Application Support")
            .join("TRAE SOLO CN")
            .join("User")
            .join("globalStorage");
        Self {
            db: directory.join("state.vscdb"),
        }
    }
}

fn current_paths() -> TraePaths {
    #[cfg(target_os = "windows")]
    {
        let directory = crate::config::get_user_roaming_app_data_dir()
            .join("TRAE SOLO CN")
            .join("User")
            .join("globalStorage");
        TraePaths {
            db: directory.join("state.vscdb"),
        }
    }
    #[cfg(target_os = "macos")]
    TraePaths::from_home(&crate::config::get_home_dir())
}

pub(crate) async fn get_traework_model_ids() -> Result<TraeWorkModelIdsResult, TraeErrorDto> {
    let paths = current_paths();
    tokio::task::spawn_blocking(move || get_traework_model_ids_at(&paths))
        .await
        .map_err(|_| TraeErrorDto::new(TraeErrorCode::StateUnavailable))?
}

pub(crate) fn get_traework_model_ids_at(
    paths: &TraePaths,
) -> Result<TraeWorkModelIdsResult, TraeErrorDto> {
    let loaded = load_map(paths)?;
    let ids = project_custom_ids(&loaded.map)?;
    let truncated = ids.len() > MAX_MODELS;
    Ok(TraeWorkModelIdsResult {
        model_ids: ids.into_iter().take(MAX_MODELS).collect(),
        revision: loaded.revision,
        truncated,
    })
}

struct LoadedMap {
    revision: Option<String>,
    map: Value,
}

fn load_map(paths: &TraePaths) -> Result<LoadedMap, TraeErrorDto> {
    if !paths.db.exists() {
        return Ok(LoadedMap {
            revision: None,
            map: empty_map(),
        });
    }
    let connection = Connection::open(&paths.db)
        .map_err(|_| TraeErrorDto::new(TraeErrorCode::ModelsStoreUnavailable))?;
    let keys = map_row_keys(&connection)
        .map_err(|_| TraeErrorDto::new(TraeErrorCode::ModelsStoreUnavailable))?;
    let Some(key) = prefer_vendor_map_key(&keys) else {
        return Ok(LoadedMap {
            revision: None,
            map: empty_map(),
        });
    };
    let row = match connection.query_row(
        &format!("SELECT value FROM {ITEM_TABLE} WHERE key = ?1"),
        params![&key],
        |row| {
            let value = match row.get_ref(0)? {
                ValueRef::Null => Vec::new(),
                ValueRef::Blob(bytes) | ValueRef::Text(bytes) => bytes.to_vec(),
                other => {
                    return Err(rusqlite::Error::InvalidColumnType(
                        0,
                        "value".to_owned(),
                        other.data_type(),
                    ));
                }
            };
            Ok(value)
        },
    ) {
        Ok(value) => Some(value),
        Err(rusqlite::Error::QueryReturnedNoRows) => None,
        Err(_) => {
            return Err(TraeErrorDto::new(TraeErrorCode::ModelsStoreUnavailable));
        }
    };
    let Some(value) = row else {
        return Ok(LoadedMap {
            revision: None,
            map: empty_map(),
        });
    };
    let map = if value.is_empty() {
        empty_map()
    } else {
        serde_json::from_slice(&value)
            .map_err(|_| TraeErrorDto::new(TraeErrorCode::ModelsStoreUnavailable))?
    };
    if !map.is_object() {
        return Err(TraeErrorDto::new(TraeErrorCode::ModelsStoreUnavailable));
    }
    reject_secret_model_ids(&map)?;
    Ok(LoadedMap {
        revision: Some(revision_for(&value)),
        map,
    })
}

fn map_row_keys(connection: &Connection) -> Result<Vec<String>, rusqlite::Error> {
    let mut statement =
        connection.prepare(&format!("SELECT key FROM {ITEM_TABLE} WHERE key LIKE ?1"))?;
    let keys = statement
        .query_map(params![format!("%{MAP_SUFFIX}")], |row| row.get(0))?
        .collect::<Result<Vec<String>, _>>()?;
    Ok(keys)
}

fn prefer_vendor_map_key(keys: &[String]) -> Option<String> {
    let colon = format!(":{MAP_SUFFIX}");
    let underscored = format!("_{MAP_SUFFIX}");
    keys.iter()
        .find(|key| key.ends_with(&colon))
        .cloned()
        .or_else(|| keys.iter().find(|key| key.ends_with(&underscored)).cloned())
        .or_else(|| keys.first().cloned())
}

fn array_list_names(map: &Value) -> Result<Vec<String>, TraeErrorDto> {
    let object = map
        .as_object()
        .ok_or_else(|| TraeErrorDto::new(TraeErrorCode::ModelsStoreUnavailable))?;
    Ok(object
        .iter()
        .filter(|(_, value)| value.is_array())
        .map(|(name, _)| name.clone())
        .collect())
}

fn present_work_lists(map: &Value) -> Result<Vec<&'static str>, TraeErrorDto> {
    let object = map
        .as_object()
        .ok_or_else(|| TraeErrorDto::new(TraeErrorCode::ModelsStoreUnavailable))?;
    let mut names = Vec::new();
    for name in WORK_LISTS {
        match object.get(name) {
            Some(Value::Array(_)) => names.push(name),
            Some(_) => return Err(TraeErrorDto::new(TraeErrorCode::ModelsStoreUnavailable)),
            None => {}
        }
    }
    if names.is_empty() {
        return Err(TraeErrorDto::new(TraeErrorCode::ModelsStoreUnavailable));
    }
    Ok(names)
}

fn empty_map() -> Value {
    json!({ LITE_LIST: [], REMOTE_LIST: [] })
}

fn list_rows<'a>(map: &'a Value, name: &str) -> Result<&'a [Value], TraeErrorDto> {
    match map.get(name) {
        None => Ok(&[]),
        Some(Value::Array(rows)) => Ok(rows),
        Some(_) => Err(TraeErrorDto::new(TraeErrorCode::ModelsStoreUnavailable)),
    }
}

fn row_str(row: &Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        row.get(*key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
    })
}

fn row_model_id(row: &Value) -> Option<String> {
    if let Some(display_name) = row_str(row, &["display_name", "display_name"]) {
        return Some(display_name);
    }
    if let Some(name) = row_str(row, &["name"]) {
        if let Some((_, model_id)) = name.split_once("//") {
            let model_id = model_id.trim();
            if !model_id.is_empty() {
                return Some(model_id.to_owned());
            }
        }
        return Some(name);
    }
    row_str(row, &["custom_model_id", "custom_model_id"])
}

fn is_preset_row(row: &Value) -> bool {
    row.get("is_preset")
        .or_else(|| row.get("is_preset"))
        .and_then(Value::as_bool)
        == Some(true)
}

fn document_secrets(map: &Value) -> Result<Vec<String>, TraeErrorDto> {
    let mut secrets = Vec::new();
    for name in array_list_names(map)? {
        for row in list_rows(map, &name)? {
            for key in ["ak", "sk"] {
                if let Some(secret) = row
                    .get(key)
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                {
                    secrets.push(secret.to_string());
                }
            }
        }
    }
    Ok(secrets)
}

fn reject_secret_model_ids(map: &Value) -> Result<(), TraeErrorDto> {
    let secrets = document_secrets(map)?;
    for name in array_list_names(map)? {
        for row in list_rows(map, &name)? {
            if is_preset_row(row) {
                continue;
            }
            if let Some(model_id) = row_model_id(row) {
                if secrets
                    .iter()
                    .any(|secret| credential_matches_model_id(secret, &model_id))
                {
                    return Err(TraeErrorDto::new(TraeErrorCode::CredentialCollision));
                }
            }
        }
    }
    Ok(())
}

fn project_custom_ids(map: &Value) -> Result<Vec<String>, TraeErrorDto> {
    reject_secret_model_ids(map)?;
    let mut seen = HashSet::new();
    let mut ids = Vec::new();
    for name in present_work_lists(map)? {
        for row in list_rows(map, name)? {
            if is_preset_row(row) {
                continue;
            }
            let Some(model_id) = row_model_id(row) else {
                continue;
            };
            if seen.insert(model_id.clone()) {
                ids.push(model_id);
            }
        }
    }
    Ok(ids)
}

fn credential_matches_model_id(credential: &str, model_id: &str) -> bool {
    let credential = credential.trim();
    !credential.is_empty() && model_id.trim().contains(credential)
}

fn revision_for(bytes: &[u8]) -> String {
    mac_bytes(revision_mac_key(), bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn revision_mac_key() -> &'static [u8; 32] {
    static KEY: OnceLock<[u8; 32]> = OnceLock::new();
    KEY.get_or_init(random_mac_key)
}

fn mac_bytes(key: &[u8; 32], bytes: &[u8]) -> [u8; 32] {
    let mut mac = HmacSha256::new_from_slice(key).expect("the fixed-size MAC key is always valid");
    mac.update(bytes);
    let digest = mac.finalize().into_bytes();
    let mut output = [0u8; 32];
    output.copy_from_slice(&digest);
    output
}

fn random_mac_key() -> [u8; 32] {
    let mut key = [0u8; 32];
    key[..16].copy_from_slice(Uuid::new_v4().as_bytes());
    key[16..].copy_from_slice(Uuid::new_v4().as_bytes());
    key
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::params;

    fn write_fixture(paths: &TraePaths, map: &Value) {
        std::fs::create_dir_all(paths.db.parent().expect("state.vscdb has a parent")).unwrap();
        let connection = Connection::open(&paths.db).unwrap();
        connection
            .execute(
                "CREATE TABLE ItemTable (key TEXT UNIQUE ON CONFLICT REPLACE, value BLOB)",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO ItemTable (key, value) VALUES (?1, ?2)",
                params![
                    "machine:AI.agent.model.model_list_map",
                    serde_json::to_vec(map).unwrap()
                ],
            )
            .unwrap();
    }

    fn write_text_fixture(paths: &TraePaths, map: &Value) {
        std::fs::create_dir_all(paths.db.parent().expect("state.vscdb has a parent")).unwrap();
        let connection = Connection::open(&paths.db).unwrap();
        connection
            .execute(
                "CREATE TABLE ItemTable (key TEXT UNIQUE ON CONFLICT REPLACE, value TEXT)",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO ItemTable (key, value) VALUES (?1, ?2)",
                params![
                    "machine:AI.agent.model.model_list_map",
                    serde_json::to_string(map).unwrap()
                ],
            )
            .unwrap();
    }

    fn preset_and_custom(custom_id: &str, secret: &str) -> Value {
        json!({
            "solo_work_lite": [
                {
                    "is_preset": true,
                    "name": "preset-lite",
                    "display_name": "Preset",
                    "custom_model_id": "preset-lite",
                    "base_url": "https://api.example.test/v1",
                    "ak": "PRESET-AK",
                    "sk": "PRESET-SK",
                    "selectable": true,
                    "status": true
                },
                {
                    "is_preset": false,
                    "name": custom_id,
                    "display_name": custom_id,
                    "custom_model_id": custom_id,
                    "base_url": "https://api.example.test/v1",
                    "ak": secret,
                    "sk": "",
                    "selectable": true,
                    "status": true
                }
            ],
            "solo_work_remote": [
                {
                    "is_preset": true,
                    "name": "preset-remote",
                    "display_name": "Preset",
                    "custom_model_id": "preset-remote",
                    "ak": "PRESET-AK",
                    "sk": "PRESET-SK"
                }
            ]
        })
    }

    #[test]
    fn projects_secret_free_custom_ids_from_lite_only() {
        let temp = tempfile::TempDir::new().unwrap();
        let paths = TraePaths::from_home(temp.path());
        write_fixture(&paths, &preset_and_custom("custom-a", "USER-TRAE-KEY"));
        let ids = get_traework_model_ids_at(&paths).unwrap();
        assert_eq!(ids.model_ids, vec!["custom-a"]);
        let debug = format!("{ids:?}");
        let json = serde_json::to_string(&ids).unwrap();
        for secret in ["USER-TRAE-KEY", "PRESET-AK", "PRESET-SK"] {
            assert!(!debug.contains(secret));
            assert!(!json.contains(secret));
        }
        assert!(!json.contains("ak"));
        assert!(!json.contains("sk"));
    }

    #[test]
    fn reads_item_table_text_values() {
        let temp = tempfile::TempDir::new().unwrap();
        let paths = TraePaths::from_home(temp.path());
        write_text_fixture(&paths, &preset_and_custom("custom-text", "USER-TRAE-KEY"));
        let ids = get_traework_model_ids_at(&paths).unwrap();
        assert_eq!(ids.model_ids, vec!["custom-text"]);
    }

    #[test]
    fn fails_closed_when_custom_id_contains_document_secret() {
        let temp = tempfile::TempDir::new().unwrap();
        let paths = TraePaths::from_home(temp.path());
        write_fixture(&paths, &preset_and_custom("USER-TRAE-KEY", "USER-TRAE-KEY"));
        assert_eq!(
            get_traework_model_ids_at(&paths).unwrap_err().code,
            TraeErrorCode::CredentialCollision
        );
    }

    #[test]
    fn prefers_colon_work_key_when_underscore_ide_map_also_exists() {
        let temp = tempfile::TempDir::new().unwrap();
        let paths = TraePaths::from_home(temp.path());
        let work = json!({
            "solo_work_lite": [{
                "is_preset": false,
                "name": "custom_openai_compatible//colon-custom",
                "display_name": "colon-custom",
                "custom_model_id": "colon-custom",
                "ak": "WORK-AK",
                "sk": ""
            }],
            "solo_work_remote": [{
                "is_preset": true,
                "name": "preset-remote",
                "display_name": "Preset",
                "ak": "PRESET-AK",
                "sk": "PRESET-SK"
            }]
        });
        let ide = json!({
            "solo_agent": [{
                "is_preset": false,
                "name": "underscore-custom",
                "display_name": "underscore-custom",
                "custom_model_id": "underscore-custom",
                "ak": "IDE-AK"
            }]
        });
        std::fs::create_dir_all(paths.db.parent().expect("state.vscdb has a parent")).unwrap();
        let connection = Connection::open(&paths.db).unwrap();
        connection
            .execute(
                "CREATE TABLE ItemTable (key TEXT UNIQUE ON CONFLICT REPLACE, value TEXT)",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO ItemTable (key, value) VALUES (?1, ?2)",
                params![
                    "3994:AI.agent.model.model_list_map",
                    serde_json::to_string(&work).unwrap()
                ],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO ItemTable (key, value) VALUES (?1, ?2)",
                params![
                    "3994_AI.agent.model.model_list_map",
                    serde_json::to_string(&ide).unwrap()
                ],
            )
            .unwrap();

        let ids = get_traework_model_ids_at(&paths).unwrap();
        assert_eq!(ids.model_ids, vec!["colon-custom"]);
        let json = serde_json::to_string(&ids).unwrap();
        for secret in [
            "WORK-AK",
            "IDE-AK",
            "PRESET-AK",
            "PRESET-SK",
            "underscore-custom",
        ] {
            assert!(!json.contains(secret));
        }
        assert!(!json.contains("ak"));
        assert!(!json.contains("sk"));
    }
}
