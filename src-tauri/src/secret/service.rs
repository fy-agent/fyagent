use super::{
    SecretCommandError, SecretCommandId, SecretCommandSuccess, SecretContractVersionV1,
    SecretErrorView, SecretInternalError, SecretService, SchemaVersionV1,
};

pub(crate) fn command_error_from_internal(error: SecretInternalError) -> SecretCommandError {
    SecretCommandError {
        contract_version: SecretContractVersionV1::V1,
        schema_version: SchemaVersionV1,
        command_id: SecretCommandId::generate(),
        error: SecretErrorView::checked_from_internal(error, None, None, None),
    }
}

pub(crate) fn command_success<T>(data: T) -> SecretCommandSuccess<T> {
    SecretCommandSuccess {
        contract_version: SecretContractVersionV1::V1,
        schema_version: SchemaVersionV1,
        command_id: SecretCommandId::generate(),
        data,
    }
}

/// Phase 2A helper: SecretService lives in the included operation.rs
/// namespace. This module only adds command envelope helpers.
pub(crate) fn service_unavailable() -> SecretInternalError {
    SecretInternalError::input_invalid()
}

#[allow(dead_code)]
fn _service_ref(_: &SecretService) {}
