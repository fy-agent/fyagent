use super::{
    CurrentLegacySourceExpectations, LegacySourceExpectation, SecretInternalError, SecretOwner,
};

/// Registered legacy source inventory primitives. The 11-domain
/// main-integration factory stays a typed stub and does not invent adapters.
pub(crate) struct LegacySourceInventoryPolicy;

impl LegacySourceInventoryPolicy {
    pub(crate) fn validate_current_expectations(
        values: Vec<LegacySourceExpectation>,
    ) -> Result<CurrentLegacySourceExpectations, SecretInternalError> {
        CurrentLegacySourceExpectations::checked_from_codex_inventory_bridge(values)
            .map_err(|_| SecretInternalError::input_invalid())
    }

    pub(crate) fn policy_aware_scrub_forbidden_without_authority(
        _owner: &SecretOwner,
    ) -> SecretInternalError {
        SecretInternalError::input_invalid()
    }

    pub(crate) fn main_integration_factory_stub() -> Result<(), SecretInternalError> {
        Err(SecretInternalError::input_invalid())
    }
}
