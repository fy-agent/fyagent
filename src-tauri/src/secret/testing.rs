use std::collections::HashMap;
use std::sync::Mutex;

use super::{SecretInternalError, SecretMaterial, SecretPurpose};

/// In-memory fake backend for focused tests. It does not import platform,
/// capture, or V2 modules.
pub(crate) struct InMemorySecretBackend {
    records: Mutex<HashMap<String, Vec<u8>>>,
}

impl InMemorySecretBackend {
    pub(crate) fn new() -> Self {
        Self {
            records: Mutex::new(HashMap::new()),
        }
    }

    pub(crate) fn write(
        &self,
        locator: &str,
        material: SecretMaterial,
    ) -> Result<(), SecretInternalError> {
        let mut stored = None;
        let receipt = material.write_to_sealed_callback(TestingWriteCallback {
            stored: &mut stored,
        });
        let _ = receipt;
        let bytes = stored.ok_or_else(SecretInternalError::input_invalid)?;
        self.records
            .lock()
            .map_err(|_| SecretInternalError::input_invalid())?
            .insert(locator.to_string(), bytes);
        Ok(())
    }

    pub(crate) fn read(&self, locator: &str) -> Result<SecretMaterial, SecretInternalError> {
        let guard = self
            .records
            .lock()
            .map_err(|_| SecretInternalError::input_invalid())?;
        let bytes = guard
            .get(locator)
            .cloned()
            .ok_or_else(SecretInternalError::input_invalid)?;
        SecretMaterial::from_native_input(bytes, SecretPurpose::CodexApiKey)
    }

    pub(crate) fn delete(&self, locator: &str) -> Result<(), SecretInternalError> {
        self.records
            .lock()
            .map_err(|_| SecretInternalError::input_invalid())?
            .remove(locator)
            .ok_or_else(SecretInternalError::input_invalid)?;
        Ok(())
    }

    pub(crate) fn validate(&self, locator: &str) -> Result<(), SecretInternalError> {
        let material = self.read(locator)?;
        let _ = material;
        Ok(())
    }
}

struct TestingWriteCallback<'a> {
    stored: &'a mut Option<Vec<u8>>,
}

impl super::backend_material_callback_sealed::Sealed for TestingWriteCallback<'_> {}

impl super::BackendMaterialWriteCallback for TestingWriteCallback<'_> {
    type Receipt = ();

    fn write_once(self, material: &[u8]) -> Self::Receipt {
        *self.stored = Some(material.to_vec());
    }
}

pub(crate) type InMemorySecretStore = InMemorySecretBackend;
