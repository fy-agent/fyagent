//! Package file reading and writing.
//!
//! One versioned JSON file, written atomically. Deliberately not compressed or
//! zipped: the body is text only, and a single file the user can open and read
//! is what makes "there really are no tool logs in here" checkable. That
//! auditability is worth more than the bytes.

use std::path::Path;

use super::identity;
use super::model::{
    limits, ExportOutcome, ExporterInfo, MessageKind, MigratableSession, MigrationError,
    MigrationResult, SessionPackage, PACKAGE_SCHEMA,
};

pub fn exporter_info() -> ExporterInfo {
    ExporterInfo {
        app: "fyagent".to_string(),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        platform: exporter_platform().to_string(),
    }
}

fn exporter_platform() -> &'static str {
    #[cfg(target_os = "macos")]
    {
        "macos"
    }
    #[cfg(target_os = "windows")]
    {
        "windows"
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        "unsupported"
    }
}

pub fn build_package(sessions: Vec<MigratableSession>, exported_at: i64) -> SessionPackage {
    SessionPackage {
        schema: PACKAGE_SCHEMA.to_string(),
        exported_at,
        exporter: exporter_info(),
        sessions,
    }
}

/// Serialize and write a package, then report what landed on disk.
///
/// `packageFileDigest` is evidence only. It never takes part in idempotency:
/// re-exporting the same conversation legitimately changes `exportedAt` and
/// JSON layout, and that must still hit the same slot.
pub fn write_package(
    package: &SessionPackage,
    target_path: &str,
) -> MigrationResult<ExportOutcome> {
    if package.sessions.len() as u32 > limits::PACKAGE_SESSIONS {
        return Err(MigrationError::TooManySessions {
            count: package.sessions.len() as u32,
            limit: limits::PACKAGE_SESSIONS,
        });
    }
    for session in &package.sessions {
        validate_session(session)?;
    }

    let body =
        serde_json::to_vec_pretty(package).map_err(|error| MigrationError::PackageWriteFailed {
            path: target_path.to_string(),
            reason: error.to_string(),
        })?;
    if body.len() as u64 > limits::PACKAGE_FILE_BYTES {
        return Err(MigrationError::PackageTooLarge {
            bytes: body.len() as u64,
            limit: limits::PACKAGE_FILE_BYTES,
        });
    }

    crate::config::atomic_write(Path::new(target_path), &body).map_err(|error| {
        MigrationError::PackageWriteFailed {
            path: target_path.to_string(),
            reason: error.to_string(),
        }
    })?;

    Ok(ExportOutcome {
        path: target_path.to_string(),
        session_count: package.sessions.len() as u32,
        byte_len: body.len() as u64,
        package_file_digest: format!("sha256:{}", identity::sha256_hex(&body)),
    })
}

/// Read and fully validate a package file.
pub fn read_package(path: &str) -> MigrationResult<SessionPackage> {
    let file_path = Path::new(path);
    let metadata =
        std::fs::metadata(file_path).map_err(|error| MigrationError::PackageMalformed {
            reason: format!("{path}: {error}"),
        })?;
    if metadata.len() > limits::PACKAGE_FILE_BYTES {
        return Err(MigrationError::PackageTooLarge {
            bytes: metadata.len(),
            limit: limits::PACKAGE_FILE_BYTES,
        });
    }

    let raw =
        std::fs::read_to_string(file_path).map_err(|error| MigrationError::PackageMalformed {
            reason: format!("{path}: {error}"),
        })?;
    parse_package(&raw)
}

/// Parse a package body. Split out from [`read_package`] so the gates are
/// testable without touching the filesystem.
pub fn parse_package(raw: &str) -> MigrationResult<SessionPackage> {
    let depth = max_json_depth(raw);
    if depth > limits::JSON_DEPTH {
        return Err(MigrationError::JsonTooDeep {
            depth,
            limit: limits::JSON_DEPTH,
        });
    }

    // `serde_json` lets a later duplicate key win silently, so a file could
    // carry two `messages` arrays and be read differently by a tool that keeps
    // the first. A package whose meaning depends on the reader is malformed.
    reject_duplicate_keys(raw)?;

    // Read the schema first so a future v2 package is rejected by version,
    // not misreported as a pile of unknown fields.
    let loose: serde_json::Value =
        serde_json::from_str(raw).map_err(|error| MigrationError::PackageMalformed {
            reason: error.to_string(),
        })?;
    let schema = loose
        .get("schema")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    if schema != PACKAGE_SCHEMA {
        return Err(MigrationError::PackageSchemaUnsupported {
            found: schema.to_string(),
            supported: vec![PACKAGE_SCHEMA.to_string()],
        });
    }

    let package: SessionPackage =
        serde_json::from_str(raw).map_err(|error| map_deserialize_error(&error))?;

    if package.sessions.len() as u32 > limits::PACKAGE_SESSIONS {
        return Err(MigrationError::TooManySessions {
            count: package.sessions.len() as u32,
            limit: limits::PACKAGE_SESSIONS,
        });
    }
    for session in &package.sessions {
        validate_session(session)?;
    }
    Ok(package)
}

/// Walk the document rejecting any object with a repeated key.
///
/// Also rejects a trailing second document, because `from_str` requires the
/// input to end after the first value.
pub(crate) fn reject_duplicate_keys(raw: &str) -> MigrationResult<()> {
    serde_json::from_str::<UniqueKeys>(raw)
        .map(|_| ())
        .map_err(|error| MigrationError::PackageMalformed {
            reason: error.to_string(),
        })
}

/// Structure-only shadow parse: values are discarded, keys are checked.
struct UniqueKeys;

impl<'de> serde::Deserialize<'de> for UniqueKeys {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(UniqueKeysVisitor)
    }
}

struct UniqueKeysVisitor;

impl<'de> serde::de::Visitor<'de> for UniqueKeysVisitor {
    type Value = UniqueKeys;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("any JSON value with unique object keys")
    }

    fn visit_map<A>(self, mut map: A) -> Result<UniqueKeys, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let mut seen = std::collections::HashSet::new();
        while let Some(key) = map.next_key::<String>()? {
            if !seen.insert(key.clone()) {
                return Err(serde::de::Error::custom(format!("duplicate key `{key}`")));
            }
            map.next_value::<UniqueKeys>()?;
        }
        Ok(UniqueKeys)
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<UniqueKeys, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        while seq.next_element::<UniqueKeys>()?.is_some() {}
        Ok(UniqueKeys)
    }

    fn visit_bool<E>(self, _: bool) -> Result<UniqueKeys, E> {
        Ok(UniqueKeys)
    }
    fn visit_i64<E>(self, _: i64) -> Result<UniqueKeys, E> {
        Ok(UniqueKeys)
    }
    fn visit_u64<E>(self, _: u64) -> Result<UniqueKeys, E> {
        Ok(UniqueKeys)
    }
    fn visit_f64<E>(self, _: f64) -> Result<UniqueKeys, E> {
        Ok(UniqueKeys)
    }
    fn visit_str<E>(self, _: &str) -> Result<UniqueKeys, E> {
        Ok(UniqueKeys)
    }
    fn visit_unit<E>(self) -> Result<UniqueKeys, E> {
        Ok(UniqueKeys)
    }
    fn visit_none<E>(self) -> Result<UniqueKeys, E> {
        Ok(UniqueKeys)
    }
    fn visit_some<D>(self, deserializer: D) -> Result<UniqueKeys, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(self)
    }
}

/// `deny_unknown_fields` produces a distinct code so the UI can say "this file
/// carries a field this build does not understand" instead of a generic
/// malformed-JSON message.
fn map_deserialize_error(error: &serde_json::Error) -> MigrationError {
    let message = error.to_string();
    if let Some(rest) = message.strip_prefix("unknown field `") {
        if let Some(end) = rest.find('`') {
            return MigrationError::PackageUnknownField {
                pointer: format!("line {} column {}", error.line(), error.column()),
                field: rest[..end].to_string(),
            };
        }
    }
    MigrationError::PackageMalformed { reason: message }
}

/// Validate one session against every structural and identity rule.
///
/// Digests and the snapshot ID are recomputed here. A value a package file
/// claims for itself is never trusted.
pub fn validate_session(session: &MigratableSession) -> MigrationResult<()> {
    identity::validate_identity_field("origin.originId", &session.origin.origin_id)?;
    identity::validate_identity_field("contentDigest", &session.content_digest)?;
    identity::validate_identity_field("snapshotId", &session.snapshot_id)?;
    if let Some(session_id) = session.origin.session_id.as_deref() {
        identity::validate_identity_field("origin.sessionId", session_id)?;
    }
    if let Some(fingerprint) = session.origin.store_fingerprint.as_deref() {
        identity::validate_identity_field("origin.storeFingerprint", fingerprint)?;
    }
    if session.origin.provider_id.trim().is_empty() {
        return Err(MigrationError::IdentityFieldInvalid {
            field: "origin.providerId".to_string(),
            reason: "empty".to_string(),
        });
    }

    let count = session.messages.len() as u32;
    if count > limits::SESSION_MESSAGES {
        return Err(MigrationError::TooManyMessages {
            count,
            limit: limits::SESSION_MESSAGES,
        });
    }
    if count == 0 {
        // "Succeeded with zero messages" is never a valid outcome.
        return Err(MigrationError::PackageMalformed {
            reason: "session carries no messages".to_string(),
        });
    }

    let mut total_bytes: u64 = 0;
    for (index, message) in session.messages.iter().enumerate() {
        if message.seq != index as u32 {
            return Err(MigrationError::PackageMalformed {
                reason: format!(
                    "message seq {} is out of order at position {index}",
                    message.seq
                ),
            });
        }
        let bytes = message.text.len() as u64;
        if bytes > limits::MESSAGE_BYTES {
            return Err(MigrationError::MessageTooLarge {
                seq: message.seq,
                bytes,
                limit: limits::MESSAGE_BYTES,
            });
        }
        total_bytes += bytes;
    }
    if total_bytes > limits::SESSION_BYTES {
        return Err(MigrationError::SessionTooLarge {
            bytes: total_bytes,
            limit: limits::SESSION_BYTES,
        });
    }

    let recomputed_digest = identity::content_digest(&session.messages);
    if recomputed_digest != session.content_digest {
        return Err(MigrationError::PackageMalformed {
            reason: format!(
                "contentDigest does not match the transcript (expected {recomputed_digest})"
            ),
        });
    }
    let recomputed_snapshot = identity::snapshot_id(&session.origin.origin_id, &recomputed_digest);
    if recomputed_snapshot != session.snapshot_id {
        return Err(MigrationError::PackageMalformed {
            reason: format!(
                "snapshotId does not match origin and content (expected {recomputed_snapshot})"
            ),
        });
    }

    if session.extraction.rule_id.trim().is_empty()
        || session.extraction.rule_verified_versions.is_empty()
    {
        return Err(MigrationError::ExtractionRuleUnavailable {
            provider_id: session.origin.provider_id.clone(),
            detected_version: None,
        });
    }

    let derived_open = derive_open_user_messages(session);
    if derived_open != session.extraction.open_user_messages {
        return Err(MigrationError::PackageMalformed {
            reason: "openUserMessages does not match the transcript".to_string(),
        });
    }

    Ok(())
}

fn derive_open_user_messages(session: &MigratableSession) -> Vec<u32> {
    let mut open = Vec::new();
    let mut pending: Option<u32> = None;
    for message in &session.messages {
        match message.kind {
            MessageKind::UserText => {
                if let Some(seq) = pending.replace(message.seq) {
                    open.push(seq);
                }
            }
            MessageKind::AssistantFinal => pending = None,
        }
    }
    if let Some(seq) = pending {
        open.push(seq);
    }
    open
}

/// Maximum bracket nesting, ignoring braces inside strings.
///
/// Checked before `serde_json` parses, because a deeply nested document is a
/// stack-exhaustion vector rather than a schema problem.
fn max_json_depth(raw: &str) -> u32 {
    let mut depth: u32 = 0;
    let mut max_depth: u32 = 0;
    let mut in_string = false;
    let mut escaped = false;
    for byte in raw.bytes() {
        if in_string {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_string = false;
            }
            continue;
        }
        match byte {
            b'"' => in_string = true,
            b'{' | b'[' => {
                depth += 1;
                max_depth = max_depth.max(depth);
            }
            b'}' | b']' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    max_depth
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session_manager::migrate::model::{
        ExtractionReport, MigratableMessage, OmittedCounts, OriginIdentity, PathFamily,
    };

    fn session(messages: Vec<(MessageKind, &str)>) -> MigratableSession {
        let messages: Vec<MigratableMessage> = messages
            .into_iter()
            .enumerate()
            .map(|(index, (kind, text))| MigratableMessage {
                seq: index as u32,
                kind,
                text: text.to_string(),
                ts: None,
            })
            .collect();
        let origin = OriginIdentity {
            origin_id: "fyo1:abc".to_string(),
            provider_id: "codex".to_string(),
            session_id: Some("01a0c4f7".to_string()),
            cli_version: None,
            store_fingerprint: None,
        };
        let content_digest = identity::content_digest(&messages);
        let snapshot_id = identity::snapshot_id(&origin.origin_id, &content_digest);
        let mut built = MigratableSession {
            snapshot_id,
            content_digest,
            origin,
            title: None,
            created_at: None,
            last_active_at: None,
            workspace_label: Some("repo".to_string()),
            messages,
            extraction: ExtractionReport {
                rule_id: "codex.rollout.phase-v1".to_string(),
                rule_verified_versions: vec!["=0.154.0".to_string()],
                open_user_messages: Vec::new(),
                omitted: OmittedCounts::default(),
                source_path_family: PathFamily::Posix,
            },
        };
        built.extraction.open_user_messages = derive_open_user_messages(&built);
        built
    }

    fn valid_package() -> SessionPackage {
        build_package(
            vec![session(vec![
                (MessageKind::UserText, "question"),
                (MessageKind::AssistantFinal, "answer"),
            ])],
            1_790_000_000_000,
        )
    }

    #[test]
    fn round_trips_a_valid_package() {
        let package = valid_package();
        let raw = serde_json::to_string(&package).unwrap();
        let parsed = parse_package(&raw).expect("parse");
        assert_eq!(parsed, package);
    }

    #[test]
    fn rejects_a_future_schema_by_version_not_by_field() {
        let mut package = valid_package();
        package.schema = "fyagent.session.v2".to_string();
        let raw = serde_json::to_string(&package).unwrap();
        let error = parse_package(&raw).expect_err("must reject");
        assert_eq!(error.code(), "packageSchemaUnsupported");
    }

    #[test]
    fn unknown_fields_get_their_own_code() {
        let mut raw: serde_json::Value = serde_json::to_value(valid_package()).unwrap();
        raw["sessions"][0]["surpriseField"] = serde_json::json!(true);
        let error = parse_package(&raw.to_string()).expect_err("must reject");
        assert_eq!(error.code(), "packageUnknownField");
        assert!(matches!(
            error,
            MigrationError::PackageUnknownField { ref field, .. } if field == "surpriseField"
        ));
    }

    #[test]
    fn a_tampered_transcript_fails_digest_recomputation() {
        let mut raw: serde_json::Value = serde_json::to_value(valid_package()).unwrap();
        raw["sessions"][0]["messages"][1]["text"] = serde_json::json!("forged answer");
        let error = parse_package(&raw.to_string()).expect_err("must reject");
        assert!(matches!(
            error,
            MigrationError::PackageMalformed { ref reason } if reason.contains("contentDigest")
        ));
    }

    #[test]
    fn a_relabelled_origin_fails_snapshot_recomputation() {
        let mut raw: serde_json::Value = serde_json::to_value(valid_package()).unwrap();
        raw["sessions"][0]["origin"]["originId"] = serde_json::json!("fyo1:someone-else");
        let error = parse_package(&raw.to_string()).expect_err("must reject");
        assert!(matches!(
            error,
            MigrationError::PackageMalformed { ref reason } if reason.contains("snapshotId")
        ));
    }

    #[test]
    fn identity_fields_are_rejected_not_sanitized() {
        let mut raw: serde_json::Value = serde_json::to_value(valid_package()).unwrap();
        raw["sessions"][0]["origin"]["sessionId"] = serde_json::json!("id\u{202e}reversed");
        let error = parse_package(&raw.to_string()).expect_err("must reject");
        assert_eq!(error.code(), "identityFieldInvalid");
    }

    #[test]
    fn deep_nesting_is_rejected_before_parsing() {
        let deep = format!("{}{}{}", "[".repeat(200), "1", "]".repeat(200));
        let error = parse_package(&deep).expect_err("must reject");
        assert!(matches!(
            error,
            MigrationError::JsonTooDeep { depth, limit } if depth == 200 && limit == 64
        ));
    }

    #[test]
    fn braces_inside_strings_do_not_count_as_nesting() {
        // A transcript body legitimately contains code with braces.
        let raw = r#"{"body":"fn main() { let v = [1, 2]; }"}"#;
        assert_eq!(max_json_depth(raw), 1);
        let escaped = r#"{"body":"a \" { quote"}"#;
        assert_eq!(max_json_depth(escaped), 1);
    }

    #[test]
    fn out_of_order_sequence_numbers_are_rejected() {
        let mut package = valid_package();
        package.sessions[0].messages[1].seq = 5;
        let raw = serde_json::to_string(&package).unwrap();
        let error = parse_package(&raw).expect_err("must reject");
        assert!(matches!(
            error,
            MigrationError::PackageMalformed { ref reason } if reason.contains("seq")
        ));
    }

    #[test]
    fn open_user_messages_must_match_the_transcript() {
        let mut package = valid_package();
        package.sessions[0].extraction.open_user_messages = vec![0];
        let raw = serde_json::to_string(&package).unwrap();
        let error = parse_package(&raw).expect_err("must reject");
        assert!(matches!(
            error,
            MigrationError::PackageMalformed { ref reason } if reason.contains("openUserMessages")
        ));
    }

    #[test]
    fn an_unfinished_transcript_is_valid() {
        let unfinished = session(vec![
            (MessageKind::UserText, "q1"),
            (MessageKind::AssistantFinal, "a1"),
            (MessageKind::UserText, "q2 never answered"),
        ]);
        assert_eq!(unfinished.extraction.open_user_messages, vec![2]);
        let package = build_package(vec![unfinished], 1);
        let raw = serde_json::to_string(&package).unwrap();
        parse_package(&raw).expect("an unanswered last turn is a real state");
    }

    #[test]
    fn write_package_is_atomic_and_reports_the_file_digest() {
        let dir = tempfile::tempdir().expect("tempdir");
        let target = dir.path().join("session-package.json");
        let package = valid_package();
        let outcome = write_package(&package, target.to_str().unwrap()).expect("write");

        let bytes = std::fs::read(&target).expect("read back");
        assert_eq!(outcome.byte_len, bytes.len() as u64);
        assert_eq!(
            outcome.package_file_digest,
            format!("sha256:{}", identity::sha256_hex(&bytes))
        );
        assert_eq!(outcome.session_count, 1);
        let reparsed = read_package(target.to_str().unwrap()).expect("reparse");
        assert_eq!(reparsed, package);
    }

    #[test]
    fn re_export_with_a_different_timestamp_keeps_the_same_snapshot() {
        // The package file hash changes; the semantic snapshot must not.
        let first = build_package(vec![session(vec![(MessageKind::UserText, "q")])], 1_000);
        let second = build_package(vec![session(vec![(MessageKind::UserText, "q")])], 2_000);
        assert_ne!(first.exported_at, second.exported_at);
        assert_eq!(
            first.sessions[0].snapshot_id,
            second.sessions[0].snapshot_id
        );
    }

    #[test]
    fn package_never_carries_an_absolute_source_path() {
        let raw = serde_json::to_string(&valid_package()).unwrap();
        assert!(!raw.contains("/Users/"));
        assert!(!raw.contains("C:\\"));
    }
}
