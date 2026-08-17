fn validate_material(
    bytes: &[u8],
    purpose: SecretPurpose,
) -> Result<(), SecretInternalError> {
    if purpose != SecretPurpose::CodexApiKey
        || bytes.is_empty()
        || bytes.len() > 2560
        || bytes.contains(&0)
        || std::str::from_utf8(bytes).is_err()
    {
        Err(SecretInternalError::input_invalid())
    } else {
        Ok(())
    }
}

// True owner: crate::secret::material. crate::secret does not re-export
// this type. Only capture and crate::secret::backend may construct/consume it.
pub(in crate::secret) struct SecretMaterial(Zeroizing<Vec<u8>>);

// This crate-private seal and public-in-secret callback trait are defined in
// crate::secret::backend. crate::secret::material imports only the callback
// trait for SecretMaterial::write_to_sealed_callback. backend.rs implements
// only its platform callback; each allowlisted lane adapter implements one
// seal/base/route triple without making the seal public outside the crate.
pub(crate) mod backend_material_callback_sealed {
    pub(crate) trait Sealed {}
}

pub(crate) trait BackendMaterialWriteCallback:
    backend_material_callback_sealed::Sealed
{
    // Every implementer and receipt type is listed in 7.1.1. No callback may
    // return bytes/String/header/material or store the borrow beyond this call.
    type Receipt;
    fn write_once(self, material: &[u8]) -> Self::Receipt;
}

// #35 core owns only these route traits. Concrete #41, main-integration and
// runtime types implement the seal + base callback + exactly one marker in
// their lane-owned adapter module. backend.rs therefore compiles without
// naming or constructing any lane's not-yet-landed concrete callback type.
pub(crate) trait ApplyMaterialAdapter: BackendMaterialWriteCallback {}
pub(crate) trait ActivationEqualityMaterialAdapter: BackendMaterialWriteCallback {}
pub(crate) trait RecoveryEqualityMaterialAdapter: BackendMaterialWriteCallback {}
pub(crate) trait StagedImportEqualityMaterialAdapter: BackendMaterialWriteCallback {}
pub(crate) trait MigrationEqualityMaterialAdapter: BackendMaterialWriteCallback {}
pub(crate) trait ProxyMaterialAdapter: BackendMaterialWriteCallback {}
pub(crate) trait UsageMaterialAdapter: BackendMaterialWriteCallback {}
pub(crate) trait CodingPlanMaterialAdapter: BackendMaterialWriteCallback {}
pub(crate) trait ModelFetchMaterialAdapter: BackendMaterialWriteCallback {}

impl SecretMaterial {
    pub(in crate::secret) fn from_native_input(
        bytes: Vec<u8>,
        purpose: SecretPurpose,
    ) -> Result<Self, SecretInternalError> {
        // Own and zeroize before any validation branch can fail.
        let bytes = Zeroizing::new(bytes);
        validate_material(bytes.as_slice(), purpose)?;
        Ok(Self(bytes))
    }

    pub(in crate::secret) fn ct_eq(&self, other: &Self) -> bool {
        bool::from(self.0.as_slice().ct_eq(other.0.as_slice()))
    }

    pub(in crate::secret) fn ct_eq_slice(&self, other: &[u8]) -> bool {
        bool::from(self.0.as_slice().ct_eq(other))
    }

    pub(in crate::secret) fn write_to_sealed_callback<C>(
        self,
        callback: C,
    ) -> C::Receipt
    where
        C: BackendMaterialWriteCallback,
    {
        callback.write_once(self.0.as_slice())
    }
}

// No Serialize, Deserialize, Clone or unrestricted Debug implementation.
