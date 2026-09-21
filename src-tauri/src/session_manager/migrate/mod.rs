//! Cross-device session migration.
//!
//! Scope is the user's own words plus each turn's final answer. Tool events,
//! progress narration, reasoning, attachments and runtime injections are
//! excluded by construction, and a transcript whose final answers cannot be
//! identified fails to export rather than exporting something approximate.
//!
//! The existing scan and display path (`session_manager::providers`) is
//! untouched: its `SessionMessage` projection synthesizes tool text and is a
//! display contract, not a migration source.
//!
//! Module map:
//!
//! | module | responsibility |
//! |---|---|
//! | [`model`] | package/receipt DTOs, the single error vocabulary, limits |
//! | [`identity`] | source identity, content digest, snapshot, slot, device binding |
//! | [`extract`] | final-only extraction and the version-pinned provider rules |
//! | [`package`] | package file read/write, schema and size gates |
//! | [`export`] | strict preview and package export |
//! | [`capability`] | release capability matrix and the local runtime probe |
//! | [`receipt`] | the single local receipt table |
//! | [`native`] | per-provider native write, readback and nonce lookup |
//! | [`restore`] | crash-window ordering around one native write |
//! | [`reconcile`] | resolving writes whose outcome is unknown |

pub mod capability;
pub mod export;
pub mod extract;
pub mod identity;
pub mod model;
pub mod native;
pub mod package;
pub mod receipt;
pub mod reconcile;
pub mod restore;

#[cfg(test)]
mod tests {
    use super::*;

    /// Every provider the product promises must appear in the capability
    /// matrix, including the ones with no verified path. A provider that is
    /// simply missing from the matrix reads as "not part of the product",
    /// which is a different and false statement.
    #[test]
    fn the_capability_matrix_is_the_single_place_that_answers_what_works() {
        let matrix = capability::release_capability_matrix();
        for provider in [
            "codex",
            "claude",
            "opencode",
            "openclaw",
            "gemini",
            "grokbuild",
            "hermes",
        ] {
            let row = matrix
                .iter()
                .find(|row| row.provider_id == provider)
                .unwrap_or_else(|| panic!("{provider} is missing from the matrix"));
            // A write strategy without any verified stage would claim a
            // capability nobody exercised.
            if row.write_strategy.is_some() {
                assert!(
                    !row.verified_stages.is_empty(),
                    "{provider} advertises a write strategy with no verified stage"
                );
            }
            // And the reverse: a stage past package verification needs a
            // write path, or nothing could have reached it. Export and
            // restore stay independent — OpenCode and Hermes are restore
            // targets with no export rule at all.
            let claims_native_stage = row
                .verified_stages
                .iter()
                .any(|(stage, _)| *stage != model::RestoreStage::PackageVerified);
            if claims_native_stage {
                assert!(
                    row.write_strategy.is_some(),
                    "{provider} claims a native stage without a write strategy"
                );
            }
        }
    }
}
