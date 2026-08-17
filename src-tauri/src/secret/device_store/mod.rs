use std::fs::File;
use std::path::{Path, PathBuf};

use chrono::{SecondsFormat, Utc};
use uuid::Uuid;

use super::{
    DeviceInstanceId, SecretInternalError, SecretService, SecretServiceConstructionToken,
    SecretServiceDeps, SecretStoreLifetime,
};

pub mod atomic;
pub mod journal;
pub mod reconcile;
pub mod schema;

use atomic::{ensure_private_dir, open_lock_file, read_limited, write_atomic_json};
use schema::{
    STATE_MAX_BYTES, StateEnvelope, StatePayload, envelope_from_payload, verify_envelope,
};

/// Device-local secret store. Root is an injected PathBuf only.
/// Tests must pass a TempDir path; this type never calls get_app_config_dir
/// or production app_local_data_dir.
pub struct DeviceLocalSecretStore {
    root: PathBuf,
    device_instance_id: DeviceInstanceId,
    _lock: File,
}

impl DeviceLocalSecretStore {
    pub fn open(root: PathBuf) -> Result<Self, SecretInternalError> {
        ensure_private_dir(&root).map_err(|_| SecretInternalError::input_invalid())?;
        ensure_private_dir(&root.join("journal")).map_err(|_| SecretInternalError::input_invalid())?;
        ensure_private_dir(&root.join("audit")).map_err(|_| SecretInternalError::input_invalid())?;
        let lock = open_lock_file(&root.join("store.lock"))
            .map_err(|_| SecretInternalError::input_invalid())?;
        let state_path = root.join("state.json");
        let device_instance_id = if state_path.exists() {
            let envelope = load_state(&state_path)?;
            DeviceInstanceId::parse(envelope.payload.device_instance_id)
                .map_err(|_| SecretInternalError::input_invalid())?
        } else {
            let id = DeviceInstanceId::generate();
            let now = utc_now();
            let payload = StatePayload::empty(id.as_str().to_string(), now);
            let envelope =
                envelope_from_payload(payload).map_err(|_| SecretInternalError::input_invalid())?;
            persist_state(&root, &envelope)?;
            id
        };
        Ok(Self {
            root,
            device_instance_id,
            _lock: lock,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn device_instance_id(&self) -> &DeviceInstanceId {
        &self.device_instance_id
    }

    pub fn load(&self) -> Result<StateEnvelope, SecretInternalError> {
        load_state(&self.root.join("state.json"))
    }

    pub fn store(&self, payload: StatePayload) -> Result<StateEnvelope, SecretInternalError> {
        let envelope =
            envelope_from_payload(payload).map_err(|_| SecretInternalError::input_invalid())?;
        persist_state(&self.root, &envelope)?;
        Ok(envelope)
    }
}

fn load_state(path: &Path) -> Result<StateEnvelope, SecretInternalError> {
    let bytes = read_limited(path, STATE_MAX_BYTES).map_err(|_| SecretInternalError::input_invalid())?;
    let envelope: StateEnvelope =
        serde_json::from_slice(&bytes).map_err(|_| SecretInternalError::input_invalid())?;
    verify_envelope(&envelope).map_err(|_| SecretInternalError::input_invalid())?;
    Ok(envelope)
}

fn persist_state(root: &Path, envelope: &StateEnvelope) -> Result<(), SecretInternalError> {
    verify_envelope(envelope).map_err(|_| SecretInternalError::input_invalid())?;
    let bytes =
        serde_json::to_vec(envelope).map_err(|_| SecretInternalError::input_invalid())?;
    write_atomic_json(root, &root.join("state.json"), &bytes)
        .map_err(|_| SecretInternalError::input_invalid())
}

fn utc_now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

pub(crate) fn new_production_service(
    construction: SecretServiceConstructionToken,
    app_handle: tauri::AppHandle,
    opened_store: super::OpenedDeviceLocalSecretStore,
) -> Result<std::sync::Arc<SecretService>, SecretInternalError> {
    super::new_production_service(construction, app_handle, opened_store)
}

#[cfg(any(test, feature = "test-hooks"))]
pub(crate) fn new_test_service(
    construction: SecretServiceConstructionToken,
    mode: super::test_support::SecretTestFixtureMode,
) -> std::sync::Arc<SecretService> {
    super::new_test_service(construction, mode)
}

#[allow(dead_code)]
fn _keep_deps_visible(_: SecretServiceDeps, _: SecretStoreLifetime) {}

#[allow(dead_code)]
fn _unused_uuid() -> String {
    Uuid::new_v4().simple().to_string()
}
