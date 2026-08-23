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
#[cfg(any(test, target_os = "windows"))]
#[allow(unused_imports)]
pub(crate) use error::SecretErrorCode;
pub(crate) use error::SecretServiceError;
pub(crate) use material::SecretMaterial;
#[cfg(test)]
#[allow(unused_imports)]
pub(crate) use memory::{MemoryFailureMode, MemorySecretBackend};
pub(crate) use platform::NativeSecretBackend;
pub(crate) use types::{
    BackendProbe, SecretBackendKind, SecretDeleteReceiptDto, SecretHandle, SecretPurpose,
    SecretRef, SecretSummaryDto, SecretVersion,
};
#[cfg(any(test, target_os = "windows"))]
#[allow(unused_imports)]
pub(crate) use types::{SecretAvailability, SecretPresence};
