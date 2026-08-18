//! #55-owned D2 admission refs. Not #35 types; no resolve_for_apply.
#![cfg_attr(not(test), allow(dead_code))]

const PROJECTION_DIGEST_LEN: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretLineV1 {
    CandidateActivation,
    CodexProviderApply,
    StagedImport,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretProjectionRefV1 {
    pub line: SecretLineV1,
    pub projection_digest: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretCapabilityV1 {
    CredentialFree,
    TypedDisabled { reason: &'static str },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SecretProjectionRefErrorV1 {
    #[error("sha256_prefix_rejected")]
    Sha256PrefixRejected,
    #[error("digest_not_64_lowercase_hex")]
    DigestNot64LowercaseHex,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretAdmissionRequestV1 {
    pub projection_ref: Option<SecretProjectionRefV1>,
}

impl SecretProjectionRefV1 {
    pub fn parse(
        line: SecretLineV1,
        projection_digest: &str,
    ) -> Result<Self, SecretProjectionRefErrorV1> {
        if projection_digest.starts_with("sha256:") {
            return Err(SecretProjectionRefErrorV1::Sha256PrefixRejected);
        }
        if !is_64_lowercase_hex(projection_digest) {
            return Err(SecretProjectionRefErrorV1::DigestNot64LowercaseHex);
        }
        Ok(Self {
            line,
            projection_digest: projection_digest.to_owned(),
        })
    }
}

pub fn admit_secret_work<W>(request: &SecretAdmissionRequestV1, writer: W) -> SecretCapabilityV1
where
    W: FnOnce(),
{
    match &request.projection_ref {
        None => {
            writer();
            SecretCapabilityV1::CredentialFree
        }
        Some(_) => SecretCapabilityV1::TypedDisabled {
            reason: "secret_projection_ref_unresolved",
        },
    }
}

fn is_64_lowercase_hex(value: &str) -> bool {
    value.len() == PROJECTION_DIGEST_LEN
        && value
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

#[cfg(test)]
mod tests {
    use super::{
        admit_secret_work, SecretAdmissionRequestV1, SecretCapabilityV1, SecretLineV1,
        SecretProjectionRefErrorV1, SecretProjectionRefV1,
    };
    use std::sync::atomic::{AtomicUsize, Ordering};

    const VALID_DIGEST: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    #[test]
    fn change_plan_secret_ref_parse_rejects_sha256_prefix() {
        let prefixed = format!("sha256:{VALID_DIGEST}");
        let error = SecretProjectionRefV1::parse(SecretLineV1::CodexProviderApply, &prefixed)
            .expect_err("sha256: prefix must be rejected");
        assert_eq!(error, SecretProjectionRefErrorV1::Sha256PrefixRejected);
    }

    #[test]
    fn change_plan_secret_ref_parse_rejects_non_64_hex() {
        assert_eq!(
            SecretProjectionRefV1::parse(SecretLineV1::CandidateActivation, "abc"),
            Err(SecretProjectionRefErrorV1::DigestNot64LowercaseHex)
        );
        assert_eq!(
            SecretProjectionRefV1::parse(
                SecretLineV1::StagedImport,
                "0123456789ABCDEF0123456789abcdef0123456789abcdef0123456789abcdef",
            ),
            Err(SecretProjectionRefErrorV1::DigestNot64LowercaseHex)
        );
    }

    #[test]
    fn change_plan_secret_refs_equal_when_same_valid_digest() {
        let first = SecretProjectionRefV1::parse(SecretLineV1::CandidateActivation, VALID_DIGEST)
            .expect("valid digest");
        let second = SecretProjectionRefV1::parse(SecretLineV1::CandidateActivation, VALID_DIGEST)
            .expect("valid digest");
        assert_eq!(first, second);
        assert_eq!(first.projection_digest, second.projection_digest);
    }

    #[test]
    fn change_plan_secret_admission_admits_switch_when_credential_free() {
        let calls = AtomicUsize::new(0);
        let admitted = admit_secret_work(
            &SecretAdmissionRequestV1 {
                projection_ref: None,
            },
            || {
                calls.fetch_add(1, Ordering::SeqCst);
            },
        );
        assert_eq!(admitted, SecretCapabilityV1::CredentialFree);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn change_plan_secret_admission_typed_disabled_skips_writer() {
        let calls = AtomicUsize::new(0);
        let projection_ref =
            SecretProjectionRefV1::parse(SecretLineV1::CodexProviderApply, VALID_DIGEST)
                .expect("valid digest");
        let admitted = admit_secret_work(
            &SecretAdmissionRequestV1 {
                projection_ref: Some(projection_ref),
            },
            || {
                calls.fetch_add(1, Ordering::SeqCst);
            },
        );
        assert!(matches!(admitted, SecretCapabilityV1::TypedDisabled { .. }));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }
}
