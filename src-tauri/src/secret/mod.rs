#![allow(
    dead_code,
    unused_imports,
    unused_variables,
    unused_mut,
    unused_macros,
    clippy::all
)]

mod compat;
use compat::{ConstantTimeEq, Zeroizing};

include!("types.rs");
include!("error.rs");
include!("material.rs");
include!("backend.rs");
include!("operation.rs");

pub(crate) mod platform;
pub(crate) mod capture;

pub mod device_store;
mod service;
mod migration;
mod redaction;
pub(crate) mod testing;

#[path = "../commands/secret.rs"]
mod command_api;

#[cfg(test)]
#[path = "secret_core_tests.rs"]
mod secret_core_tests;

pub(crate) use service::command_error_from_internal as service_command_error;

impl From<SecretInternalError> for crate::error::AppError {
    fn from(_: SecretInternalError) -> Self {
        crate::error::AppError::Message("secret operation failed".to_string())
    }
}
