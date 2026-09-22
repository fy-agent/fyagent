//! Final-only extraction.
//!
//! There is no default pass-through. A message is only allowed into a package
//! when a provider-specific, version-pinned, fixture-proven rule says what it
//! is. "No rule" and "rule cannot decide" both fail the export; neither
//! degrades into "take the last assistant message", which is the mistake this
//! module exists to prevent.

pub mod rules;
mod storage;
pub mod strict_text;

use std::path::{Path, PathBuf};

use serde_json::Value;

use super::identity;
use super::model::{
    limits, ExtractionReport, MessageKind, MigratableMessage, MigratableSession, MigrationError,
    MigrationResult, OmittedCounts, OriginIdentity, PathFamily,
};

/// One record from a provider's ordered raw stream.
#[derive(Debug, Clone)]
pub struct RawRecord {
    pub value: Value,
    pub ts: Option<i64>,
}

/// Everything a rule read from one source session, before classification.
#[derive(Debug, Clone, Default)]
pub struct RawSource {
    pub records: Vec<RawRecord>,
    /// Version recorded by the source itself; never inferred from the current CLI.
    pub source_cli_version: Option<String>,
    /// Canonical native store that actually contains this source. Independent
    /// exported files have no store binding and cannot inherit local receipts.
    pub canonical_store_path: Option<PathBuf>,
    /// Native session ID, when the source really carries one.
    pub session_id: Option<String>,
    /// Absolute source workspace. Only its basename and path family survive
    /// into the package.
    pub workspace_path: Option<String>,
    pub title: Option<String>,
    pub created_at: Option<i64>,
    pub last_active_at: Option<i64>,
    /// Stable local key for the source store, used to look up the random
    /// namespace that anonymizes the origin identity.
    pub store_key: String,
    /// Diagnostic only; never drives an automatic merge.
    pub store_fingerprint: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Classification {
    /// Not a message record (session header, metadata, state event).
    Skip,
    RealUser {
        text: String,
        omitted: OmittedCounts,
    },
    FinalAnswer {
        text: String,
        omitted: OmittedCounts,
    },
    /// Mid-turn assistant narration.
    Commentary,
    /// Developer rules, AGENTS.md, environment context, IDE injections.
    RuntimeInjection,
    ToolEvent,
    Reasoning,
    /// A whole record that is an attachment. The Codex rule never produces it
    /// because attachments arrive as blocks inside a message and are counted
    /// by [`strict_text`]; a format that stores them as their own record needs
    /// this.
    #[allow(dead_code)]
    Attachment,
    /// Blocks the export wherever it occurs, including the final turn.
    Indeterminate {
        reason: String,
    },
}

pub trait ExtractionRule: Send + Sync {
    fn provider_id(&self) -> &'static str;
    /// Rule identity and version, e.g. `codex.rollout.phase-v1`.
    fn rule_id(&self) -> &'static str;
    /// Provider versions this rule has fixture evidence for. An empty list
    /// means the rule is not releasable and every export is blocked.
    fn verified_versions(&self) -> &'static [&'static str];
    fn load_source(&self, source_path: &str) -> MigrationResult<RawSource>;
    fn classify(&self, record: &RawRecord) -> Classification;
}

/// Extract with the version recorded by the source when available. Hermes has
/// no durable producer-version field; its current, explicit schema is checked
/// by the reader and its parser must be verified against the local CLI version.
/// This does not claim a historical producer version in origin.cliVersion.
#[cfg(any(test, feature = "test-hooks"))]
pub fn extract_session(
    provider_id: &str,
    source_path: &str,
    detected_version: Option<&str>,
) -> MigrationResult<MigratableSession> {
    extract_session_with_source(provider_id, source_path, detected_version)
        .map(|(session, _)| session)
}

/// Source metadata is returned from the same read snapshot for exact native
/// receipt inheritance; callers must not reopen a live transcript to identify it.
pub fn extract_session_with_source(
    provider_id: &str,
    source_path: &str,
    detected_version: Option<&str>,
) -> MigrationResult<(MigratableSession, RawSource)> {
    let rule =
        rules::rule_for(provider_id).ok_or_else(|| MigrationError::ExtractionRuleUnavailable {
            provider_id: provider_id.to_string(),
            detected_version: detected_version.map(str::to_string),
        })?;
    let source = rule.load_source(source_path)?;
    // Codex/OpenCode persist a source version. A missing header is not made
    // trustworthy by installing a newer CLI. Hermes's absence is structural,
    // not a fallback from a malformed/unknown version.
    let version = source.source_cli_version.as_deref().or_else(|| {
        (provider_id == "hermes")
            .then_some(detected_version)
            .flatten()
    });
    if !version_satisfies(version, rule.verified_versions()) {
        return Err(MigrationError::ExtractionRuleVersionMismatch {
            provider_id: provider_id.to_string(),
            detected: version.map(str::to_string),
            verified: rule
                .verified_versions()
                .iter()
                .map(|v| v.to_string())
                .collect(),
        });
    }
    let session = build_session(rule, &source, source_path)?;
    Ok((session, source))
}

/// Shared by [`extract_session`] and the rule fixtures, so a unit test
/// exercises the same invariants as production.
pub(crate) fn build_session(
    rule: &dyn ExtractionRule,
    source: &RawSource,
    source_path: &str,
) -> MigrationResult<MigratableSession> {
    let mut messages: Vec<MigratableMessage> = Vec::new();
    let mut omitted = OmittedCounts::default();
    let mut session_bytes: u64 = 0;

    for record in &source.records {
        match rule.classify(record) {
            Classification::Skip => {}
            Classification::RealUser {
                text,
                omitted: block_omitted,
            } => {
                omitted.merge(&block_omitted);
                push_message(
                    &mut messages,
                    &mut session_bytes,
                    MessageKind::UserText,
                    text,
                    record.ts,
                )?;
            }
            Classification::FinalAnswer {
                text,
                omitted: block_omitted,
            } => {
                omitted.merge(&block_omitted);
                push_message(
                    &mut messages,
                    &mut session_bytes,
                    MessageKind::AssistantFinal,
                    text,
                    record.ts,
                )?;
            }
            Classification::Commentary => omitted.commentary_messages += 1,
            Classification::RuntimeInjection => omitted.runtime_injections += 1,
            Classification::ToolEvent => omitted.tool_events += 1,
            Classification::Reasoning => omitted.reasoning_blocks += 1,
            Classification::Attachment => omitted.attachments += 1,
            Classification::Indeterminate { reason } => {
                // Any position, including the last turn. A rule that cannot
                // tell commentary from a final answer must not guess.
                return Err(MigrationError::FinalAnswerIndeterminate {
                    seq: messages.len() as u32,
                    reason,
                });
            }
        }
    }

    if messages.is_empty() {
        return Err(MigrationError::SourceUnreadable {
            provider_id: rule.provider_id().to_string(),
            reason: "no user or final-answer message survived extraction".to_string(),
        });
    }

    let origin = build_origin(rule.provider_id(), source, source_path)?;
    let content_digest = identity::content_digest(&messages);
    let snapshot_id = identity::snapshot_id(&origin.origin_id, &content_digest);

    let workspace_label = source
        .workspace_path
        .as_deref()
        .and_then(source_basename)
        .as_deref()
        .and_then(identity::accept_workspace_label);

    Ok(MigratableSession {
        snapshot_id,
        content_digest,
        origin,
        title: source.title.clone(),
        created_at: source.created_at,
        last_active_at: source.last_active_at,
        workspace_label,
        extraction: ExtractionReport {
            rule_id: rule.rule_id().to_string(),
            rule_verified_versions: rule
                .verified_versions()
                .iter()
                .map(|value| value.to_string())
                .collect(),
            open_user_messages: open_user_messages(&messages),
            omitted,
            source_path_family: source
                .workspace_path
                .as_deref()
                .map(PathFamily::classify)
                .unwrap_or(PathFamily::Unknown),
        },
        messages,
    })
}

fn push_message(
    messages: &mut Vec<MigratableMessage>,
    session_bytes: &mut u64,
    kind: MessageKind,
    text: String,
    ts: Option<i64>,
) -> MigrationResult<()> {
    let seq = messages.len() as u32;
    let bytes = text.len() as u64;
    if bytes > limits::MESSAGE_BYTES {
        return Err(MigrationError::MessageTooLarge {
            seq,
            bytes,
            limit: limits::MESSAGE_BYTES,
        });
    }
    *session_bytes += bytes;
    if *session_bytes > limits::SESSION_BYTES {
        return Err(MigrationError::SessionTooLarge {
            bytes: *session_bytes,
            limit: limits::SESSION_BYTES,
        });
    }
    if seq >= limits::SESSION_MESSAGES {
        return Err(MigrationError::TooManyMessages {
            count: seq + 1,
            limit: limits::SESSION_MESSAGES,
        });
    }
    messages.push(MigratableMessage {
        seq,
        kind,
        text,
        ts,
    });
    Ok(())
}

/// A `userText` with no `assistantFinal` before the next `userText` (or before
/// the end). Derived from the sequence itself, which is why it stays out of the
/// content digest.
fn open_user_messages(messages: &[MigratableMessage]) -> Vec<u32> {
    let mut open = Vec::new();
    let mut pending: Option<u32> = None;
    for message in messages {
        match message.kind {
            MessageKind::UserText => {
                if let Some(seq) = pending.replace(message.seq) {
                    open.push(seq);
                }
            }
            MessageKind::AssistantFinal => {
                pending = None;
            }
        }
    }
    if let Some(seq) = pending {
        open.push(seq);
    }
    open
}

fn build_origin(
    provider_id: &str,
    source: &RawSource,
    source_path: &str,
) -> MigrationResult<OriginIdentity> {
    let session_id = match source.session_id.as_deref() {
        Some(raw) => {
            identity::validate_identity_field("origin.sessionId", raw)?;
            Some(raw.to_string())
        }
        None => None,
    };
    if let Some(fingerprint) = source.store_fingerprint.as_deref() {
        identity::validate_identity_field("origin.storeFingerprint", fingerprint)?;
    }

    let origin_id = match session_id.as_deref() {
        Some(id) => {
            let namespace = identity::source_store_namespace(provider_id, &source.store_key)?;
            identity::origin_id_from_native(provider_id, &namespace, id)
        }
        // No trustworthy native ID: mint a random identity, but keep it stable
        // for this source file so re-exporting the same conversation does not
        // look like a different source every time.
        None => identity::stable_random_origin_id(
            provider_id,
            &format!("{}|{source_path}", source.store_key),
        )?,
    };

    Ok(OriginIdentity {
        origin_id,
        provider_id: provider_id.to_string(),
        session_id,
        cli_version: source.source_cli_version.clone(),
        store_fingerprint: source.store_fingerprint.clone(),
    })
}

/// `detected` must parse and fall inside one of the `verified` requirements.
///
/// Anything else is a closed gate: an undetectable version is not evidence
/// that the installed build matches a fixture.
pub fn version_satisfies(detected: Option<&str>, verified: &[&str]) -> bool {
    if verified.is_empty() {
        return false;
    }
    let Some(detected) = detected.and_then(normalize_version) else {
        return false;
    };
    verified.iter().any(|requirement| {
        semver::VersionReq::parse(requirement)
            .map(|req| req.matches(&detected))
            .unwrap_or(false)
    })
}

/// CLI `--version` output is not a bare semver string: Hermes prints
/// `v0.20.5, upstream …` and Grok prints `1.0.34 (3736acbc8658)`.
pub fn normalize_version(raw: &str) -> Option<semver::Version> {
    let token = raw
        .split_whitespace()
        .map(|part| part.trim_matches(|c: char| c == ',' || c == '(' || c == ')'))
        .find_map(|part| {
            let candidate = part.strip_prefix('v').unwrap_or(part);
            semver::Version::parse(candidate).ok()
        });
    token
}

/// Last path segment of a source workspace, for either path family.
///
/// `providers::utils` is private to the display readers and stays untouched by
/// this feature, so the few characters of path handling the package needs live
/// here instead of widening that module's visibility.
pub(crate) fn source_basename(value: &str) -> Option<String> {
    let trimmed = value.trim().trim_end_matches(['/', '\\']);
    trimmed
        .split(['/', '\\'])
        .next_back()
        .filter(|segment| !segment.is_empty())
        .map(str::to_string)
}

/// Milliseconds from an RFC3339 string or a numeric seconds/milliseconds value.
pub(crate) fn parse_timestamp_ms(value: &Value) -> Option<i64> {
    if let Some(number) = value.as_i64() {
        return Some(if number > 1_000_000_000_000 {
            number
        } else {
            number * 1000
        });
    }
    if let Some(number) = value.as_f64() {
        let number = number as i64;
        return Some(if number > 1_000_000_000_000 {
            number
        } else {
            number * 1000
        });
    }
    chrono::DateTime::parse_from_rfc3339(value.as_str()?)
        .ok()
        .map(|parsed| parsed.timestamp_millis())
}

/// Read a JSONL source with the cumulative read budget enforced.
pub(crate) fn read_jsonl_records(
    provider_id: &str,
    path: &Path,
) -> MigrationResult<Vec<RawRecord>> {
    use std::io::{BufRead, BufReader, Read};
    let file = std::fs::File::open(path).map_err(|e| storage::unreadable(provider_id, e))?;
    let mut reader = BufReader::new(file.take(limits::SOURCE_READ_BYTES + 1));
    let mut records = Vec::new();
    let mut read_bytes = 0_u64;
    loop {
        let mut line = String::new();
        let bytes = reader
            .read_line(&mut line)
            .map_err(|e| storage::unreadable(provider_id, e))?;
        if bytes == 0 {
            break;
        }
        read_bytes += bytes as u64;
        storage::check_budget(read_bytes, records.len())?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let value = storage::parse_json(provider_id, trimmed)?;
        let ts = value.get("timestamp").and_then(parse_timestamp_ms);
        records.push(RawRecord { value, ts });
    }
    Ok(records)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user(seq: u32) -> MigratableMessage {
        MigratableMessage {
            seq,
            kind: MessageKind::UserText,
            text: format!("u{seq}"),
            ts: None,
        }
    }

    fn assistant(seq: u32) -> MigratableMessage {
        MigratableMessage {
            seq,
            kind: MessageKind::AssistantFinal,
            text: format!("a{seq}"),
            ts: None,
        }
    }

    #[test]
    fn consecutive_user_messages_are_open_until_answered() {
        // Follow-up questions and a resend after an interruption are real
        // states, not malformed data.
        let messages = vec![user(0), user(1), assistant(2), user(3)];
        assert_eq!(open_user_messages(&messages), vec![0, 3]);
    }

    #[test]
    fn a_fully_answered_transcript_has_no_open_messages() {
        let messages = vec![user(0), assistant(1), user(2), assistant(3)];
        assert!(open_user_messages(&messages).is_empty());
    }

    #[test]
    fn trailing_user_message_is_open_not_an_error() {
        assert_eq!(open_user_messages(&[user(0)]), vec![0]);
    }

    #[test]
    fn version_gate_is_closed_without_a_detected_version() {
        assert!(!version_satisfies(None, &["=0.154.0"]));
        assert!(!version_satisfies(Some("0.154.0"), &[]));
        assert!(version_satisfies(Some("0.154.0"), &["=0.154.0"]));
        assert!(!version_satisfies(Some("0.155.0"), &["=0.154.0"]));
    }

    #[test]
    fn version_gate_parses_real_cli_version_output() {
        // Shapes taken from research/session-recovery-20260921/local-cli-evidence.json
        assert!(version_satisfies(Some("v0.20.5"), &["=0.20.5"]));
        assert!(version_satisfies(
            Some("1.0.34 (3736acbc8658)"),
            &["=1.0.34"]
        ));
        assert!(version_satisfies(Some("opencode 1.18.30"), &["=1.18.30"]));
        assert_eq!(normalize_version("not a version"), None);
    }
}
