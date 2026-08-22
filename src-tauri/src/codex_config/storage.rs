use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::config::{
    atomic_write, delete_file, get_home_dir, sanitize_provider_name, write_json_file,
    write_text_file,
};
use crate::error::AppError;

use super::FYAGENT_CODEX_MODEL_CATALOG_FILENAME;

pub fn get_codex_config_dir() -> PathBuf {
    if let Some(custom) = crate::settings::get_codex_override_dir() {
        return custom;
    }

    get_home_dir().join(".codex")
}

pub fn get_codex_auth_path() -> PathBuf {
    get_codex_config_dir().join("auth.json")
}

pub fn get_codex_config_path() -> PathBuf {
    get_codex_config_dir().join("config.toml")
}

pub fn get_codex_model_catalog_path() -> PathBuf {
    get_codex_config_dir().join(FYAGENT_CODEX_MODEL_CATALOG_FILENAME)
}

#[allow(dead_code)]
pub fn get_codex_provider_paths(
    provider_id: &str,
    provider_name: Option<&str>,
) -> (PathBuf, PathBuf) {
    let base_name = provider_name
        .map(sanitize_provider_name)
        .unwrap_or_else(|| sanitize_provider_name(provider_id));

    (
        get_codex_config_dir().join(format!("auth-{base_name}.json")),
        get_codex_config_dir().join(format!("config-{base_name}.toml")),
    )
}

#[allow(dead_code)]
pub fn delete_codex_provider_config(
    provider_id: &str,
    provider_name: &str,
) -> Result<(), AppError> {
    let (auth_path, config_path) = get_codex_provider_paths(provider_id, Some(provider_name));
    delete_file(&auth_path).ok();
    delete_file(&config_path).ok();
    Ok(())
}

/// Atomically project Codex auth/config as one logical write. If the config
/// write fails, restore the previous auth bytes so a partial credential switch
/// cannot survive.
pub fn write_codex_live_atomic(
    auth: &Value,
    config_text_opt: Option<&str>,
) -> Result<(), AppError> {
    let auth_path = get_codex_auth_path();
    let config_path = get_codex_config_path();

    if let Some(parent) = auth_path.parent() {
        fs::create_dir_all(parent).map_err(|error| AppError::io(parent, error))?;
    }

    let old_auth = if auth_path.exists() {
        Some(fs::read(&auth_path).map_err(|error| AppError::io(&auth_path, error))?)
    } else {
        None
    };

    let config_text = config_text_opt.unwrap_or_default().to_string();
    if !config_text.trim().is_empty() {
        toml::from_str::<toml::Table>(&config_text)
            .map_err(|error| AppError::toml(&config_path, error))?;
    }

    write_json_file(&auth_path, auth)?;
    if let Err(error) = write_text_file(&config_path, &config_text) {
        if let Some(bytes) = old_auth {
            let _ = atomic_write(&auth_path, &bytes);
        } else {
            let _ = delete_file(&auth_path);
        }
        return Err(error);
    }

    Ok(())
}

pub fn read_codex_config_text() -> Result<String, AppError> {
    let path = get_codex_config_path();
    if path.exists() {
        fs::read_to_string(&path).map_err(|error| AppError::io(&path, error))
    } else {
        Ok(String::new())
    }
}

pub fn validate_config_toml(text: &str) -> Result<(), AppError> {
    if text.trim().is_empty() {
        return Ok(());
    }
    toml::from_str::<toml::Table>(text)
        .map(|_| ())
        .map_err(|error| AppError::toml(Path::new("config.toml"), error))
}

pub fn read_and_validate_codex_config_text() -> Result<String, AppError> {
    let text = read_codex_config_text()?;
    validate_config_toml(&text)?;
    Ok(text)
}
