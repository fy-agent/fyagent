use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::network::{model_allowed, NetworkError, DEFAULT_CLOUD_MODEL};

pub const DEVICE_SETTINGS_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeviceSettings {
    pub version: u32,
    pub ssid: String,
    pub password: String,
    pub api_key: String,
    pub model: String,
}

impl Default for DeviceSettings {
    fn default() -> Self {
        Self {
            version: DEVICE_SETTINGS_VERSION,
            ssid: String::new(),
            password: String::new(),
            api_key: String::new(),
            model: DEFAULT_CLOUD_MODEL.into(),
        }
    }
}

impl DeviceSettings {
    pub fn validate(&self) -> Result<(), NetworkError> {
        if self.version != DEVICE_SETTINGS_VERSION
            || !field_ok(&self.ssid, 1, 32)
            || !field_ok(&self.password, 0, 64)
            || !field_ok(&self.api_key, 0, 256)
            || !model_allowed(&self.model)
        {
            return Err(NetworkError::Invalid);
        }
        Ok(())
    }
}

fn field_ok(value: &str, min: usize, max: usize) -> bool {
    let len = value.chars().count();
    len >= min && len <= max && !value.chars().any(char::is_control)
}

pub struct DeviceSettingsStore {
    path: PathBuf,
}

impl DeviceSettingsStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
    pub fn load(&self) -> Result<Option<DeviceSettings>, NetworkError> {
        if !self.path.exists() {
            return Ok(None);
        }
        let settings = serde_json::from_slice::<DeviceSettings>(
            &fs::read(&self.path).map_err(|_| NetworkError::Invalid)?,
        )
        .map_err(|_| NetworkError::Invalid)?;
        settings.validate()?;
        Ok(Some(settings))
    }
    pub fn save(&self, settings: DeviceSettings) -> Result<DeviceSettings, NetworkError> {
        settings.validate()?;
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|_| NetworkError::Invalid)?;
        }
        fs::write(
            &self.path,
            serde_json::to_vec_pretty(&settings).map_err(|_| NetworkError::Invalid)?,
        )
        .map_err(|_| NetworkError::Invalid)?;
        Ok(settings)
    }
    #[allow(dead_code)]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn settings() -> DeviceSettings {
        DeviceSettings {
            version: DEVICE_SETTINGS_VERSION,
            ssid: "cafe".into(),
            password: "secret".into(),
            api_key: "sk-demo".into(),
            model: DEFAULT_CLOUD_MODEL.into(),
        }
    }

    #[test]
    fn save_round_trips_and_rejects_unknown_fields() {
        let directory = tempdir().unwrap();
        let store = DeviceSettingsStore::new(directory.path().join("device.json"));
        let saved = store.save(settings()).unwrap();
        assert_eq!(store.load().unwrap(), Some(saved));
        let mut unknown = serde_json::to_value(settings()).unwrap();
        unknown["token"] = serde_json::json!("nope");
        fs::write(store.path(), serde_json::to_vec(&unknown).unwrap()).unwrap();
        assert_eq!(store.load(), Err(NetworkError::Invalid));
    }

    #[test]
    fn empty_ssid_or_unknown_model_is_rejected() {
        let mut invalid = settings();
        invalid.ssid.clear();
        assert_eq!(invalid.validate(), Err(NetworkError::Invalid));
        invalid = settings();
        invalid.model = "x".repeat(65);
        assert_eq!(invalid.validate(), Err(NetworkError::Invalid));
    }

    #[test]
    fn empty_api_key_and_model_is_allowed_for_wifi_only() {
        let mut wifi_only = settings();
        wifi_only.api_key.clear();
        wifi_only.model.clear();
        assert_eq!(wifi_only.validate(), Ok(()));
    }
}
