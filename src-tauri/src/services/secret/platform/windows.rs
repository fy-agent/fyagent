use std::{ptr, sync::Mutex};

use windows_sys::Win32::{
    Foundation::{
        GetLastError, ERROR_ACCESS_DENIED, ERROR_BAD_USERNAME, ERROR_CANCELLED,
        ERROR_INVALID_PARAMETER, ERROR_NOT_FOUND, ERROR_NO_SUCH_LOGON_SESSION,
    },
    Security::Credentials::{
        CredDeleteW, CredFree, CredReadW, CredWriteW, CREDENTIALW, CRED_PERSIST_LOCAL_MACHINE,
        CRED_TYPE_GENERIC,
    },
};

use super::super::{
    BackendProbe, SecretBackend, SecretBackendKind, SecretMaterial, SecretPurpose, SecretRef,
    SecretServiceError,
};

const TARGET_PREFIX: &str = "FyAgent/secret/v1/";
const USERNAME: &str = "FyAgent";
static CREDENTIAL_LOCK: Mutex<()> = Mutex::new(());

#[derive(Clone, Copy)]
enum CredentialOperation {
    Create,
    Replace,
    Read,
    Probe,
    Delete,
}

fn map_last_error(error: u32, operation: CredentialOperation) -> SecretServiceError {
    match error {
        ERROR_NOT_FOUND => SecretServiceError::missing(),
        ERROR_ACCESS_DENIED => SecretServiceError::permission_denied(),
        ERROR_NO_SUCH_LOGON_SESSION => SecretServiceError::backend_unavailable(),
        ERROR_CANCELLED => SecretServiceError::locked(),
        ERROR_INVALID_PARAMETER | ERROR_BAD_USERNAME => SecretServiceError::invalid_input(),
        _ => match operation {
            CredentialOperation::Create | CredentialOperation::Replace => {
                SecretServiceError::write_failed()
            }
            CredentialOperation::Read | CredentialOperation::Probe => {
                SecretServiceError::read_failed()
            }
            CredentialOperation::Delete => SecretServiceError::delete_failed(),
        },
    }
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

fn target(secret_ref: &SecretRef) -> Vec<u16> {
    wide(&format!("{TARGET_PREFIX}{}", secret_ref.as_str()))
}

struct CredentialGuard(*mut CREDENTIALW);

impl CredentialGuard {
    fn credential(&self) -> Result<&CREDENTIALW, SecretServiceError> {
        unsafe { self.0.as_ref() }.ok_or_else(SecretServiceError::read_failed)
    }
}

impl Drop for CredentialGuard {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { CredFree(self.0.cast()) };
        }
    }
}

pub(crate) struct WindowsSecretBackend;

impl WindowsSecretBackend {
    pub(crate) fn new() -> Self {
        Self
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, ()>, SecretServiceError> {
        CREDENTIAL_LOCK
            .lock()
            .map_err(|_| SecretServiceError::internal())
    }

    fn read_locked(
        &self,
        secret_ref: &SecretRef,
        operation: CredentialOperation,
        purpose: SecretPurpose,
    ) -> Result<SecretMaterial, SecretServiceError> {
        let target = target(secret_ref);
        let mut raw = ptr::null_mut();
        let read = unsafe { CredReadW(target.as_ptr(), CRED_TYPE_GENERIC, 0, &mut raw) };
        if read == 0 {
            return Err(map_last_error(unsafe { GetLastError() }, operation));
        }
        let guard = CredentialGuard(raw);
        let credential = guard.credential()?;
        if credential.Type != CRED_TYPE_GENERIC
            || credential.Persist != CRED_PERSIST_LOCAL_MACHINE
            || credential.CredentialBlobSize == 0
            || credential.CredentialBlobSize > 2_560
            || credential.CredentialBlob.is_null()
        {
            return Err(SecretServiceError::verify_failed());
        }
        let bytes = unsafe {
            std::slice::from_raw_parts(
                credential.CredentialBlob,
                credential.CredentialBlobSize as usize,
            )
        }
        .to_vec();
        SecretMaterial::from_native_input(bytes, purpose)
    }

    fn probe_locked(
        &self,
        secret_ref: &SecretRef,
        purpose: SecretPurpose,
    ) -> Result<BackendProbe, SecretServiceError> {
        match self.read_locked(secret_ref, CredentialOperation::Probe, purpose) {
            Ok(_) => Ok(BackendProbe::ready()),
            Err(error) if error.code() == super::super::SecretErrorCode::Missing => {
                Ok(BackendProbe::missing())
            }
            Err(error) => Err(error),
        }
    }

    fn write_locked(
        &self,
        secret_ref: &SecretRef,
        material: &SecretMaterial,
        operation: CredentialOperation,
    ) -> Result<(), SecretServiceError> {
        let mut target_name = target(secret_ref);
        let mut username = wide(USERNAME);
        let credential = CREDENTIALW {
            Type: CRED_TYPE_GENERIC,
            TargetName: target_name.as_mut_ptr(),
            CredentialBlobSize: material.as_bytes().len() as u32,
            CredentialBlob: material.as_bytes().as_ptr() as *mut u8,
            Persist: CRED_PERSIST_LOCAL_MACHINE,
            UserName: username.as_mut_ptr(),
            ..CREDENTIALW::default()
        };
        let written = unsafe { CredWriteW(&credential, 0) };
        if written == 0 {
            return Err(map_last_error(unsafe { GetLastError() }, operation));
        }
        let actual = self.read_locked(secret_ref, CredentialOperation::Read, material.purpose())?;
        if actual.ct_eq_slice(material.as_bytes()) {
            return Ok(());
        }
        Err(SecretServiceError::verify_failed())
    }
}

impl SecretBackend for WindowsSecretBackend {
    fn kind(&self) -> SecretBackendKind {
        SecretBackendKind::OsKeyring
    }

    fn create_new(
        &self,
        secret_ref: &SecretRef,
        material: &SecretMaterial,
    ) -> Result<(), SecretServiceError> {
        let _guard = self.lock()?;
        match self.probe_locked(secret_ref, material.purpose())? {
            BackendProbe {
                availability: super::super::SecretAvailability::Missing,
                ..
            } => {}
            _ => return Err(SecretServiceError::already_exists()),
        }
        // Credential Manager exposes upsert semantics. The instance mutex plus
        // this explicit read-before-write prevents FyAgent's own create paths
        // from silently replacing a record. Win32 offers no atomic create-only
        // primitive here, so a process that somehow races this exact random
        // target is not ruled out by this leaf; the first production binding
        // owner must add authoritative generation/CAS lifecycle protection.
        self.write_locked(secret_ref, material, CredentialOperation::Create)
    }

    fn replace(
        &self,
        secret_ref: &SecretRef,
        material: &SecretMaterial,
    ) -> Result<(), SecretServiceError> {
        let _guard = self.lock()?;
        match self.probe_locked(secret_ref, material.purpose())? {
            BackendProbe {
                availability: super::super::SecretAvailability::Ready,
                ..
            } => self.write_locked(secret_ref, material, CredentialOperation::Replace),
            _ => Err(SecretServiceError::missing()),
        }
    }

    fn read(
        &self,
        secret_ref: &SecretRef,
        purpose: SecretPurpose,
    ) -> Result<SecretMaterial, SecretServiceError> {
        let _guard = self.lock()?;
        self.read_locked(secret_ref, CredentialOperation::Read, purpose)
    }

    fn probe(
        &self,
        secret_ref: &SecretRef,
        purpose: SecretPurpose,
    ) -> Result<BackendProbe, SecretServiceError> {
        let _guard = self.lock()?;
        self.probe_locked(secret_ref, purpose)
    }

    fn delete(&self, secret_ref: &SecretRef) -> Result<(), SecretServiceError> {
        let _guard = self.lock()?;
        let target = target(secret_ref);
        let deleted = unsafe { CredDeleteW(target.as_ptr(), CRED_TYPE_GENERIC, 0) };
        if deleted == 0 {
            Err(map_last_error(
                unsafe { GetLastError() },
                CredentialOperation::Delete,
            ))
        } else {
            Ok(())
        }
    }
}
