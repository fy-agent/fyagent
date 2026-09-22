//! Export side: preview one source session, or write a package.
//!
//! Source records carry their own format/version evidence. Installing a newer
//! target CLI does not change an older source's provenance. Hermes does not
//! persist a source version, so its pinned schema rule also needs the local
//! version probe; Codex/OpenCode use their native source headers.

use serde::Deserialize;

use super::model::{
    ExportOutcome, MigratableSession, MigrationError, MigrationResult, RestoreAttempt,
};
use super::{capability, extract, identity, native, package, receipt::ReceiptStore};
use crate::database::Database;

/// One source session the user selected for export.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportItem {
    pub provider_id: String,
    /// Path produced by the existing session scan, not typed by the user.
    pub source_path: String,
}

/// Strict preview of what would be exported.
///
/// This is the only preview the export flow uses. The display projection in
/// `session_manager::providers` synthesizes tool text and would show the user
/// something the package does not contain.
pub fn preview_session(
    db: &Database,
    provider_id: &str,
    source_path: &str,
) -> MigrationResult<MigratableSession> {
    let detected_version = if provider_id == "hermes" {
        capability::require_extraction_gate(provider_id)?.detected_version
    } else {
        None
    };
    let (mut session, source) = extract::extract_session_with_source(
        provider_id,
        source_path,
        detected_version.as_deref(),
    )?;
    if let (Some(native_id), Some(source_store)) = (
        source.session_id.as_deref(),
        source.canonical_store_path.as_deref(),
    ) {
        let receipts = ReceiptStore::new(db).for_native(provider_id, native_id)?;
        if receipts.is_empty() {
            return Ok(session);
        }
        let source_store_id = match provider_id {
            "codex" => native::codex::store_identity_at(source_store),
            "opencode" | "hermes" => identity::store_instance_id(provider_id, source_store),
            _ => {
                return Err(MigrationError::TargetStoreUnidentified {
                    provider_id: provider_id.into(),
                })
            }
        }?;
        inherit_origin(&mut session, &source_store_id, &receipts)?;
    }
    Ok(session)
}

/// Extract every selected session and write one package file.
///
/// Any session that cannot be extracted fails the whole export. A package that
/// silently contains fewer conversations than the user selected is worse than
/// no package: the omission is only discoverable on the other machine, after
/// the source is out of reach.
pub fn export_package(
    db: &Database,
    items: &[ExportItem],
    target_path: &str,
) -> MigrationResult<ExportOutcome> {
    let mut sessions = Vec::with_capacity(items.len());
    for item in items {
        sessions.push(preview_session(db, &item.provider_id, &item.source_path)?);
    }
    let package = package::build_package(sessions, chrono::Utc::now().timestamp_millis());
    package::write_package(&package, target_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_items_are_parsed_as_camel_case_pairs() {
        let items: Vec<ExportItem> =
            serde_json::from_str(r#"[{"providerId":"codex","sourcePath":"/tmp/rollout.jsonl"}]"#)
                .expect("parse items");
        assert_eq!(items[0].provider_id, "codex");
        assert_eq!(items[0].source_path, "/tmp/rollout.jsonl");
    }

    #[test]
    fn a_provider_without_a_rule_is_refused_before_any_source_read() {
        // An unknown provider has no rule or CLI. The refusal must
        // come from the gate, not from a half-read source file.
        let db = Database::memory().expect("isolated DB");
        let error =
            preview_session(&db, "unknown-provider", "/nonexistent/path").expect_err("must fail");
        assert!(
            matches!(
                error.code(),
                "providerNotInstalled" | "extractionRuleUnavailable" | "capabilityProbeFailed"
            ),
            "unexpected code {}",
            error.code()
        );
    }
}

/// Inherit only an exact native mapping in this device's current store. Bodies
/// and timestamps never establish source identity, even when they are equal.
fn inherit_origin(
    session: &mut MigratableSession,
    source_store_id: &str,
    receipts: &[RestoreAttempt],
) -> MigrationResult<()> {
    let mut origin: Option<&super::model::OriginIdentity> = None;
    for attempt in receipts.iter().filter(|attempt| {
        attempt.target_provider_id == session.origin.provider_id
            && attempt.target_store_id == source_store_id
            && attempt.target_native_id.as_deref() == session.origin.session_id.as_deref()
            && attempt.stage != super::model::RestoreStage::Failed
    }) {
        if let Some(existing) = origin {
            if existing.origin_id != attempt.origin.origin_id {
                return Err(MigrationError::ReceiptStoreFailed {
                    reason: "one native session has conflicting source mappings".into(),
                });
            }
        }
        origin = Some(&attempt.origin);
    }
    if let Some(origin) = origin {
        session.origin = origin.clone();
        session.snapshot_id =
            identity::snapshot_id(&session.origin.origin_id, &session.content_digest);
    }
    Ok(())
}

#[cfg(test)]
mod inheritance_tests {
    use super::super::model::{
        ExtractionReport, MessageKind, MigratableMessage, OmittedCounts, OriginIdentity,
        PathFamily, RestoreRequestKind, RestoreStage,
    };
    use super::*;
    fn session() -> MigratableSession {
        let messages = vec![MigratableMessage {
            seq: 0,
            kind: MessageKind::UserText,
            text: "same text".into(),
            ts: None,
        }];
        let digest = identity::content_digest(&messages);
        MigratableSession {
            snapshot_id: identity::snapshot_id("native-derived", &digest),
            content_digest: digest,
            origin: OriginIdentity {
                origin_id: "native-derived".into(),
                provider_id: "codex".into(),
                session_id: Some("native-target".into()),
                cli_version: Some("0.154.0".into()),
                store_fingerprint: None,
            },
            title: None,
            created_at: None,
            last_active_at: None,
            workspace_label: None,
            messages,
            extraction: ExtractionReport {
                rule_id: "codex.rollout.phase-v1".into(),
                rule_verified_versions: vec!["=0.154.0".into()],
                open_user_messages: vec![0],
                omitted: OmittedCounts::default(),
                source_path_family: PathFamily::Posix,
            },
        }
    }
    fn receipt() -> RestoreAttempt {
        RestoreAttempt {
            attempt_id: "attempt".into(),
            request_id: "request".into(),
            snapshot_id: "old-snapshot".into(),
            request_kind: RestoreRequestKind::DefaultImport,
            idempotency_slot: Some("slot".into()),
            content_digest: "earlier-content".into(),
            origin: OriginIdentity {
                origin_id: "original-source".into(),
                provider_id: "codex".into(),
                session_id: Some("native-source".into()),
                cli_version: Some("0.154.0".into()),
                store_fingerprint: None,
            },
            target_provider_id: "codex".into(),
            target_store_id: "store-instance".into(),
            target_native_nonce: "nonce".into(),
            target_native_id: Some("native-target".into()),
            target_workspace: Some("/tmp".into()),
            stage: RestoreStage::NativeReadbackVerified,
            attempt_count: 1,
            user_attestation: None,
            last_error: None,
            created_at: 0,
            updated_at: 0,
        }
    }
    #[test]
    fn a_continued_restored_session_keeps_its_original_source() {
        let mut current = session();
        let digest = current.content_digest.clone();
        inherit_origin(&mut current, "store-instance", &[receipt()]).unwrap();
        assert_eq!(current.origin.origin_id, "original-source");
        assert_eq!(current.content_digest, digest);
        assert_eq!(
            current.snapshot_id,
            identity::snapshot_id("original-source", &digest)
        );
        assert_ne!(current.snapshot_id, "old-snapshot");
    }
    #[test]
    fn equal_text_in_another_store_or_native_id_does_not_inherit() {
        let original = session();
        let mut other_store = original.clone();
        inherit_origin(&mut other_store, "replacement-store", &[receipt()]).unwrap();
        assert_eq!(other_store, original);
        let mut wrong_id = receipt();
        wrong_id.target_native_id = Some("another-native-id".into());
        let mut current = original.clone();
        inherit_origin(&mut current, "store-instance", &[wrong_id]).unwrap();
        assert_eq!(current, original);
    }
    #[test]
    fn conflicting_exact_mappings_block_export_instead_of_guessing() {
        let a = receipt();
        let mut b = a.clone();
        b.origin.origin_id = "other-origin".into();
        assert_eq!(
            inherit_origin(&mut session(), "store-instance", &[a, b])
                .unwrap_err()
                .code(),
            "receiptStoreFailed"
        );
    }
    #[test]
    fn optional_origin_metadata_does_not_create_a_false_source_conflict() {
        let a = receipt();
        let mut b = a.clone();
        b.origin.cli_version = None;
        b.origin.store_fingerprint = Some("extra metadata".into());
        let mut current = session();
        inherit_origin(&mut current, "store-instance", &[a, b]).unwrap();
        assert_eq!(current.origin.origin_id, "original-source");
    }
}
