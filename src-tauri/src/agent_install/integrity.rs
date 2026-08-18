//! Integrity layer (#26). Defaults unknown until a real factor exists.

use serde::{Deserialize, Serialize};

use super::types::LayerState;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationSource {
    VendorManifest,
    PlatformSignature,
    PackageManagerMetadata,
    FyagentComputedHash,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FactorState {
    pub state: LayerState,
    pub value: Option<String>,
}

impl FactorState {
    pub const fn unknown() -> Self {
        Self {
            state: LayerState::Unknown,
            value: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IntegrityLayerState {
    pub integrity_state: LayerState,
    pub hash: FactorState,
    pub signature: FactorState,
    pub revocation: FactorState,
    pub verification_source: Vec<VerificationSource>,
    pub expected_signer: Option<String>,
    pub observed_signer: Option<String>,
    pub manifest_key_fingerprint: Option<String>,
    pub integrity_summary: String,
    pub checked_at: String,
}

impl IntegrityLayerState {
    pub fn unknown(checked_at: impl Into<String>) -> Self {
        Self {
            integrity_state: LayerState::Unknown,
            hash: FactorState::unknown(),
            signature: FactorState::unknown(),
            revocation: FactorState::unknown(),
            verification_source: Vec::new(),
            expected_signer: None,
            observed_signer: None,
            manifest_key_fingerprint: None,
            integrity_summary: "PENDING_RUNTIME_VERIFICATION".to_owned(),
            checked_at: checked_at.into(),
        }
    }

    pub fn package_manager_warn(checked_at: impl Into<String>) -> Self {
        Self {
            integrity_state: LayerState::Warn,
            hash: FactorState::unknown(),
            signature: FactorState::unknown(),
            revocation: FactorState::unknown(),
            verification_source: vec![VerificationSource::PackageManagerMetadata],
            expected_signer: None,
            observed_signer: None,
            manifest_key_fingerprint: None,
            integrity_summary: "package_manager_metadata".to_owned(),
            checked_at: checked_at.into(),
        }
    }

    pub fn rollup(
        hash: FactorState,
        signature: FactorState,
        revocation: FactorState,
    ) -> LayerState {
        if [hash.state, signature.state, revocation.state]
            .iter()
            .any(|state| matches!(state, LayerState::Fail))
        {
            return LayerState::Fail;
        }
        if [hash.state, signature.state, revocation.state]
            .iter()
            .all(|state| matches!(state, LayerState::Unknown))
        {
            return LayerState::Unknown;
        }
        match (hash.state, signature.state) {
            (LayerState::Ok, LayerState::Ok) => LayerState::Ok,
            (LayerState::Ok, LayerState::Warn) | (LayerState::Warn, _) => LayerState::Warn,
            (LayerState::Ok, LayerState::Unknown) => LayerState::Warn,
            (LayerState::Unknown, _)
            | (LayerState::Fail, _)
            | (LayerState::Ok, LayerState::Fail) => LayerState::Unknown,
        }
    }

    pub fn from_factors(
        hash: FactorState,
        signature: FactorState,
        revocation: FactorState,
        sources: Vec<VerificationSource>,
        summary: impl Into<String>,
        checked_at: impl Into<String>,
    ) -> Self {
        Self {
            integrity_state: Self::rollup(hash.clone(), signature.clone(), revocation.clone()),
            hash,
            signature,
            revocation,
            verification_source: sources,
            expected_signer: None,
            observed_signer: None,
            manifest_key_fingerprint: None,
            integrity_summary: summary.into(),
            checked_at: checked_at.into(),
        }
    }
}

pub fn claude_code_from_manifest(
    expected_sha256: &str,
    computed_sha256: &str,
    signed: bool,
    checked_at: &str,
) -> IntegrityLayerState {
    let hash_state = if expected_sha256.eq_ignore_ascii_case(computed_sha256) {
        LayerState::Ok
    } else {
        LayerState::Fail
    };
    let signature_state = if signed {
        LayerState::Ok
    } else {
        LayerState::Warn
    };
    IntegrityLayerState::from_factors(
        FactorState {
            state: hash_state,
            value: Some(computed_sha256.to_owned()),
        },
        FactorState {
            state: signature_state,
            value: None,
        },
        FactorState::unknown(),
        vec![VerificationSource::VendorManifest],
        if hash_state == LayerState::Fail {
            "claude_code_sha256_mismatch"
        } else if signed {
            "claude_code_signed_manifest"
        } else {
            "claude_code_unsigned_pre_gpg_floor"
        },
        checked_at,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_install::registry::source_layer_for;
    use crate::agent_install::types::AgentId;

    #[test]
    fn missing_hash_signature_revocation_is_unknown_not_ok() {
        let layer = IntegrityLayerState::unknown("t0");
        assert_eq!(layer.integrity_state, LayerState::Unknown);
    }

    #[test]
    fn any_factor_fail_fails_layer() {
        let layer = IntegrityLayerState::from_factors(
            FactorState::unknown(),
            FactorState::unknown(),
            FactorState {
                state: LayerState::Fail,
                value: Some("revoked".to_owned()),
            },
            Vec::new(),
            "revoked",
            "t0",
        );
        assert_eq!(layer.integrity_state, LayerState::Fail);
    }

    #[test]
    fn revoked_blocks_install() {
        assert_eq!(
            IntegrityLayerState::from_factors(
                FactorState {
                    state: LayerState::Ok,
                    value: Some("abc".to_owned()),
                },
                FactorState {
                    state: LayerState::Ok,
                    value: Some("signer".to_owned()),
                },
                FactorState {
                    state: LayerState::Fail,
                    value: Some("revoked".to_owned()),
                },
                Vec::new(),
                "revoked",
                "t0",
            )
            .integrity_state,
            LayerState::Fail
        );
    }

    #[test]
    fn claude_code_manifest_sha256_mismatch_fails() {
        let layer = claude_code_from_manifest("aaa", "bbb", true, "t0");
        assert_eq!(layer.integrity_state, LayerState::Fail);
    }

    #[test]
    fn claude_code_unsigned_pre_gpg_floor_is_warn() {
        let layer = claude_code_from_manifest("aaa", "aaa", false, "t0");
        assert_eq!(layer.integrity_state, LayerState::Warn);
        assert_ne!(layer.integrity_state, LayerState::Ok);
    }

    #[test]
    fn claude_code_signed_manifest_ok() {
        let layer = claude_code_from_manifest("aaa", "aaa", true, "t0");
        assert_eq!(layer.integrity_state, LayerState::Ok);
    }

    #[test]
    fn four_guide_agents_stay_pending_runtime_verification() {
        for id in [
            AgentId::QoderworkCn,
            AgentId::DingtalkWukong,
            AgentId::Workbuddy,
            AgentId::TraeWork,
        ] {
            let source = source_layer_for(id).expect("exists");
            assert!(source.package_install_blocked());
            let layer = IntegrityLayerState::unknown("t0");
            assert_eq!(layer.integrity_summary, "PENDING_RUNTIME_VERIFICATION");
        }
    }

    #[test]
    fn codex_cli_package_manager_integrity_is_warn() {
        let layer = IntegrityLayerState::package_manager_warn("t0");
        assert_eq!(layer.integrity_state, LayerState::Warn);
        assert_eq!(
            layer.verification_source,
            vec![VerificationSource::PackageManagerMetadata]
        );
    }
}
