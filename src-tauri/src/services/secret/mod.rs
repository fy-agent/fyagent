mod backend;
mod error;
mod material;
#[cfg(test)]
mod memory;
mod platform;
mod types;

pub(crate) use backend::DecodeSecret;
#[cfg(test)]
#[allow(unused_imports)]
pub(crate) use backend::MaterialMatches;
pub(crate) use backend::{SecretBackend, SecretService};
pub(crate) use error::{SecretErrorCode, SecretServiceError};
pub(crate) use material::SecretMaterial;
#[allow(unused_imports)]
pub(crate) use material::MAX_SECRET_BYTES;
#[cfg(test)]
#[allow(unused_imports)]
pub(crate) use memory::MemoryFailureMode;
#[cfg(test)]
pub(crate) use memory::MemorySecretBackend;
pub(crate) use platform::NativeSecretBackend;
#[cfg(any(test, not(any(target_os = "macos", target_os = "windows"))))]
pub(crate) use platform::UnavailableSecretBackend;
pub(crate) use types::{
    BackendProbe, SecretAvailability, SecretBackendKind, SecretDeleteReceiptDto, SecretHandle,
    SecretPresence, SecretPurpose, SecretRef, SecretSummaryDto, SecretVersion,
};
