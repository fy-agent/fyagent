// This backend is an integration-test fixture imported as a separate module;
// the library test target itself does not instantiate it.
#![allow(dead_code)]

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use zeroize::Zeroizing;

use super::{
    BackendProbe, SecretBackend, SecretBackendKind, SecretMaterial, SecretRef, SecretServiceError,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MemoryFailureMode {
    Healthy,
    Locked,
    Denied,
    Unavailable,
}

struct MemoryState {
    records: HashMap<SecretRef, Zeroizing<Vec<u8>>>,
    mode: MemoryFailureMode,
    operation_count: usize,
}

#[derive(Clone)]
pub(crate) struct MemorySecretBackend {
    state: Arc<Mutex<MemoryState>>,
}

impl MemorySecretBackend {
    pub(crate) fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(MemoryState {
                records: HashMap::new(),
                mode: MemoryFailureMode::Healthy,
                operation_count: 0,
            })),
        }
    }

    pub(crate) fn set_mode(&self, mode: MemoryFailureMode) {
        self.state.lock().expect("memory secret backend lock").mode = mode;
    }

    pub(crate) fn operation_count(&self) -> usize {
        self.state
            .lock()
            .expect("memory secret backend lock")
            .operation_count
    }

    pub(crate) fn contains(&self, secret_ref: &SecretRef) -> bool {
        self.state
            .lock()
            .expect("memory secret backend lock")
            .records
            .contains_key(secret_ref)
    }

    fn checked_state(&self) -> Result<std::sync::MutexGuard<'_, MemoryState>, SecretServiceError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| SecretServiceError::internal())?;
        state.operation_count += 1;
        match state.mode {
            MemoryFailureMode::Healthy => Ok(state),
            MemoryFailureMode::Locked => Err(SecretServiceError::locked()),
            MemoryFailureMode::Denied => Err(SecretServiceError::permission_denied()),
            MemoryFailureMode::Unavailable => Err(SecretServiceError::backend_unavailable()),
        }
    }
}

impl SecretBackend for MemorySecretBackend {
    fn kind(&self) -> SecretBackendKind {
        SecretBackendKind::OsKeyring
    }

    fn create_new(
        &self,
        secret_ref: &SecretRef,
        material: &SecretMaterial,
    ) -> Result<(), SecretServiceError> {
        let mut state = self.checked_state()?;
        if state.records.contains_key(secret_ref) {
            return Err(SecretServiceError::already_exists());
        }
        state.records.insert(
            secret_ref.clone(),
            Zeroizing::new(material.as_bytes().to_vec()),
        );
        Ok(())
    }

    fn replace(
        &self,
        secret_ref: &SecretRef,
        material: &SecretMaterial,
    ) -> Result<(), SecretServiceError> {
        let mut state = self.checked_state()?;
        let Some(record) = state.records.get_mut(secret_ref) else {
            return Err(SecretServiceError::missing());
        };
        *record = Zeroizing::new(material.as_bytes().to_vec());
        Ok(())
    }

    fn read(&self, secret_ref: &SecretRef) -> Result<SecretMaterial, SecretServiceError> {
        let state = self.checked_state()?;
        let Some(record) = state.records.get(secret_ref) else {
            return Err(SecretServiceError::missing());
        };
        SecretMaterial::from_native_input(
            record.as_slice().to_vec(),
            super::SecretPurpose::CodexApiKey,
        )
    }

    fn probe(&self, secret_ref: &SecretRef) -> Result<BackendProbe, SecretServiceError> {
        let state = self.checked_state()?;
        Ok(if state.records.contains_key(secret_ref) {
            BackendProbe::ready()
        } else {
            BackendProbe::missing()
        })
    }

    fn delete(&self, secret_ref: &SecretRef) -> Result<(), SecretServiceError> {
        let mut state = self.checked_state()?;
        if state.records.remove(secret_ref).is_some() {
            Ok(())
        } else {
            Err(SecretServiceError::missing())
        }
    }
}
