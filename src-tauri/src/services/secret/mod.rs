mod backend;
mod error;
mod material;
#[cfg(test)]
mod memory;
mod platform;
mod types;

#[cfg(test)]
pub(crate) use backend::MaterialMatches;
pub(crate) use backend::{SecretBackend, SecretService};
pub(crate) use error::{SecretErrorCode, SecretServiceError};
pub(crate) use material::SecretMaterial;
#[cfg(test)]
pub(crate) use memory::{MemoryFailureMode, MemorySecretBackend};
pub(crate) use platform::NativeSecretBackend;
pub(crate) use types::{
    BackendProbe, SecretAvailability, SecretBackendKind, SecretDeleteReceiptDto, SecretHandle,
    SecretPresence, SecretPurpose, SecretRef, SecretSummaryDto, SecretVersion,
};
