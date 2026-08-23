// Issue #35 freezes the complete CRUD/probe callback contract before every
// operation has a production caller. Keep the dormant methods available to
// later UCP adapters without weakening clippy for the rest of the crate.
#![allow(dead_code)]

#[cfg(test)]
use subtle::ConstantTimeEq;

use super::{
    BackendProbe, SecretBackendKind, SecretDeleteReceiptDto, SecretHandle, SecretMaterial,
    SecretPurpose, SecretRef, SecretServiceError, SecretSummaryDto, SecretVersion,
};

pub(crate) trait SecretBackend: Send + Sync {
    fn kind(&self) -> SecretBackendKind;
    fn create_new(
        &self,
        secret_ref: &SecretRef,
        material: &SecretMaterial,
    ) -> Result<(), SecretServiceError>;
    fn replace(
        &self,
        secret_ref: &SecretRef,
        material: &SecretMaterial,
    ) -> Result<(), SecretServiceError>;
    fn read(&self, secret_ref: &SecretRef) -> Result<SecretMaterial, SecretServiceError>;
    fn probe(&self, secret_ref: &SecretRef) -> Result<BackendProbe, SecretServiceError>;
    fn delete(&self, secret_ref: &SecretRef) -> Result<(), SecretServiceError>;
}

mod callback_sealed {
    pub(crate) trait Sealed {}
}

pub(crate) trait SecretMaterialCallback: callback_sealed::Sealed {
    type Output;
    fn consume(self, material: &[u8]) -> Self::Output;
}

pub(crate) struct SecretService<B> {
    backend: B,
}

impl<B> SecretService<B>
where
    B: SecretBackend,
{
    pub(crate) fn new(backend: B) -> Self {
        Self { backend }
    }

    pub(crate) fn create(
        &self,
        material: SecretMaterial,
        purpose: SecretPurpose,
    ) -> Result<SecretSummaryDto, SecretServiceError> {
        let handle = SecretHandle::generate();
        self.create_reserved(&handle, &material, purpose)
    }

    /// Commit a handle that was generated for a side-effect-free preview.
    /// `create_new` is deliberately not an upsert: replay cannot overwrite an
    /// existing system-keyring item.
    pub(crate) fn create_reserved(
        &self,
        handle: &SecretHandle,
        material: &SecretMaterial,
        purpose: SecretPurpose,
    ) -> Result<SecretSummaryDto, SecretServiceError> {
        self.backend.create_new(handle.secret_ref(), material)?;
        Ok(SecretSummaryDto::from_probe(
            handle,
            purpose,
            self.backend.kind(),
            BackendProbe::ready(),
        ))
    }

    pub(crate) fn replace(
        &self,
        handle: &SecretHandle,
        material: SecretMaterial,
        purpose: SecretPurpose,
    ) -> Result<SecretSummaryDto, SecretServiceError> {
        self.backend.replace(handle.secret_ref(), &material)?;
        let next = SecretHandle::new(handle.secret_ref().clone(), SecretVersion::generate());
        Ok(SecretSummaryDto::from_probe(
            &next,
            purpose,
            self.backend.kind(),
            BackendProbe::ready(),
        ))
    }

    pub(crate) fn probe(
        &self,
        handle: &SecretHandle,
        purpose: SecretPurpose,
    ) -> Result<SecretSummaryDto, SecretServiceError> {
        let probe = match self.backend.probe(handle.secret_ref()) {
            Ok(probe) => probe,
            Err(error) => error.as_probe().ok_or(error)?,
        };
        Ok(SecretSummaryDto::from_probe(
            handle,
            purpose,
            self.backend.kind(),
            probe,
        ))
    }

    pub(crate) fn with_material<C>(
        &self,
        handle: &SecretHandle,
        callback: C,
    ) -> Result<C::Output, SecretServiceError>
    where
        C: SecretMaterialCallback,
    {
        let material = self.backend.read(handle.secret_ref())?;
        Ok(callback.consume(material.as_bytes()))
    }

    pub(crate) fn delete(
        &self,
        handle: &SecretHandle,
    ) -> Result<SecretDeleteReceiptDto, SecretServiceError> {
        self.backend.delete(handle.secret_ref())?;
        Ok(SecretDeleteReceiptDto::deleted(handle.secret_ref().clone()))
    }
}

#[cfg(test)]
pub(crate) struct MaterialMatches<'a> {
    expected: &'a [u8],
}

#[cfg(test)]
impl<'a> MaterialMatches<'a> {
    pub(crate) fn new(expected: &'a [u8]) -> Self {
        Self { expected }
    }
}

#[cfg(test)]
impl callback_sealed::Sealed for MaterialMatches<'_> {}

#[cfg(test)]
impl SecretMaterialCallback for MaterialMatches<'_> {
    type Output = bool;

    fn consume(self, material: &[u8]) -> Self::Output {
        bool::from(material.ct_eq(self.expected))
    }
}
