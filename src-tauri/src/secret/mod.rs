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

include!("command_map.rs");

#[cfg(test)]
#[path = "secret_core_tests.rs"]
mod secret_core_tests;

pub(crate) use service::command_error_from_internal as service_command_error;
pub(crate) use service::command_success;
pub(crate) use service::{
    list_secret_candidates_from_store, list_secret_summaries_from_store,
    seed_pending_candidate_in_store, seed_unbound_owner_in_store, LocalCandidateProjection,
    LocalSecretSummaryProjection, SeededPendingCandidate,
};
#[cfg(test)]
pub(crate) use service::seed_opened_store_pending_candidate;
#[cfg(test)]
pub(crate) use testing::InMemorySecretBackend;

pub(crate) fn command_unavailable_error() -> SecretCommandError {
    service_command_error(SecretInternalError::input_invalid())
}

pub(crate) fn command_unavailable<T>() -> SecretCommandResult<T> {
    Err(command_unavailable_error())
}

impl From<SecretInternalError> for crate::error::AppError {
    fn from(_: SecretInternalError) -> Self {
        crate::error::AppError::Message("secret operation failed".to_string())
    }
}
