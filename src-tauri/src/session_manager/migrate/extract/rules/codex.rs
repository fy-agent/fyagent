//! `codex.rollout.phase-v1`.
//!
//! Evidence: the isolated probe recorded in
//! `.trellis/tasks/09-22-session-cross-device-recovery/evidence/codex-probe/findings.md`
//! and the field table in `deliverables/technical-design.md` §6.4, taken from
//! a codex-cli 0.154.0 rollout. Two structured facts drive the rule:
//!
//! * `payload.phase` classifies an assistant message as `commentary` or
//!   `final_answer`. The generated app-server schema states that providers do
//!   not emit it consistently and that callers must treat a missing value as
//!   "phase unknown", so a missing phase blocks the export rather than falling
//!   back to "the last assistant message wins".
//! * `content_item_kinds` separates a real user message (`user.text`) from a
//!   runtime injection that also carries `role: "user"`, such as
//!   `agents_md.instructions` or `environments.environment_context`.
//!
//! `RUNTIME_KINDS` is the observed set, not an official enumeration. An
//! unlisted kind is indeterminate on purpose.
//!
//! Unknown provenance remains blocked; the native legacy writer explicitly
//! records user.text and final_answer, so its verified output is re-exportable.

use std::collections::HashMap;
use std::path::Path;

use serde_json::Value;

use crate::session_manager::migrate::extract::strict_text::extract_text_strict;
use crate::session_manager::migrate::extract::{
    parse_timestamp_ms, read_jsonl_records, Classification, ExtractionRule, RawRecord, RawSource,
};
use crate::session_manager::migrate::model::MigrationResult;

const PROVIDER_ID: &str = "codex";
pub const RULE_ID: &str = "codex.rollout.phase-v1";

/// Only this build has fixture evidence. Widening the range requires a new
/// artifact, not an assumption that later versions behave the same.
pub const VERIFIED_VERSIONS: &[&str] = &["=0.154.0"];

const RUNTIME_KINDS: &[&str] = &[
    "agents_md.instructions",
    "environments.environment_context",
    "host_skills.instructions",
    "permissions.instructions",
    "collaboration_mode.instructions",
    "multi_agent.role_instructions",
    "multi_agent.mode_instructions",
];

const USER_TEXT_KINDS: &[&str] = &["user.text"];

const TOOL_PAYLOAD_TYPES: &[&str] = &[
    "function_call",
    "function_call_output",
    "local_shell_call",
    "local_shell_call_output",
    "custom_tool_call",
    "custom_tool_call_output",
    "web_search_call",
];

pub struct CodexRolloutPhaseV1;

impl ExtractionRule for CodexRolloutPhaseV1 {
    fn provider_id(&self) -> &'static str {
        PROVIDER_ID
    }

    fn rule_id(&self) -> &'static str {
        RULE_ID
    }

    fn verified_versions(&self) -> &'static [&'static str] {
        VERIFIED_VERSIONS
    }

    fn load_source(&self, source_path: &str) -> MigrationResult<RawSource> {
        let path = Path::new(source_path)
            .canonicalize()
            .map_err(|e| super::super::storage::unreadable(PROVIDER_ID, e))?;
        let records = read_jsonl_records(PROVIDER_ID, &path)?;
        if records
            .iter()
            .filter(|record| {
                record.value.get("type").and_then(Value::as_str) == Some("session_meta")
            })
            .count()
            != 1
        {
            return Err(super::super::storage::unreadable(
                PROVIDER_ID,
                "expected exactly one native session header",
            ));
        }
        validate_final_events(&records)?;
        let home = crate::codex_config::get_codex_config_dir()
            .canonicalize()
            .ok();
        let store = home.filter(|home| {
            ["sessions", "archived_sessions"].iter().any(|name| {
                home.join(name)
                    .canonicalize()
                    .ok()
                    .is_some_and(|dir| path.starts_with(dir))
            })
        });
        let store_key = store
            .as_deref()
            .unwrap_or(&path)
            .to_string_lossy()
            .into_owned();
        let fingerprint = store.as_ref().and_then(|root| {
            use std::io::Read;
            let file = std::fs::File::open(root.join("installation_id")).ok()?;
            let mut raw = String::new();
            file.take(257).read_to_string(&mut raw).ok()?;
            if raw.len() > 256 {
                return None;
            }
            uuid::Uuid::parse_str(raw.trim())
                .ok()
                .map(|id| id.to_string())
        });
        let mut source = build_source(records, store_key, fingerprint);
        source.canonical_store_path = store;
        Ok(source)
    }

    fn classify(&self, record: &RawRecord) -> Classification {
        classify_record(&record.value)
    }
}

fn build_source(
    records: Vec<RawRecord>,
    store_key: String,
    store_fingerprint: Option<String>,
) -> RawSource {
    let mut session_id = None;
    let mut source_cli_version = None;
    let mut workspace_path = None;
    let mut created_at = None;
    let mut last_active_at = None;

    for record in &records {
        if created_at.is_none() {
            created_at = record.ts;
        }
        if record.ts.is_some() {
            last_active_at = record.ts;
        }
        if record.value.get("type").and_then(Value::as_str) != Some("session_meta") {
            continue;
        }
        let Some(payload) = record.value.get("payload") else {
            continue;
        };
        if source_cli_version.is_none() {
            source_cli_version = payload
                .get("cli_version")
                .and_then(Value::as_str)
                .map(str::to_string);
        }
        if session_id.is_none() {
            session_id = payload
                .get("id")
                .and_then(Value::as_str)
                .map(str::to_string);
        }
        if workspace_path.is_none() {
            workspace_path = payload
                .get("cwd")
                .and_then(Value::as_str)
                .map(str::to_string);
        }
        if let Some(ts) = payload.get("timestamp").and_then(parse_timestamp_ms) {
            created_at.get_or_insert(ts);
        }
    }

    RawSource {
        records,
        source_cli_version,
        canonical_store_path: None,
        session_id,
        workspace_path,
        // The rollout does not carry a user-facing title; the display reader
        // derives one from other stores. A package must not invent one.
        title: None,
        created_at,
        last_active_at,
        store_key,
        store_fingerprint,
    }
}

/// A known final event can precede its lossless response item when a crash
/// interrupts persistence. Do not treat that recorded answer as "no answer".
/// The event never supplies exported text, and unphased legacy events make no
/// new finality claim. Counts preserve repeated identical final messages.
fn validate_final_events(records: &[RawRecord]) -> MigrationResult<()> {
    let mut responses: HashMap<String, usize> = HashMap::new();
    for record in records {
        let value = &record.value;
        if value.get("type").and_then(Value::as_str) != Some("response_item") {
            continue;
        }
        let Some(payload) = value.get("payload") else {
            continue;
        };
        if payload.get("type").and_then(Value::as_str) == Some("message")
            && payload.get("role").and_then(Value::as_str) == Some("assistant")
            && payload.get("phase").and_then(Value::as_str) == Some("final_answer")
            && all_blocks_are_output_text(payload.get("content"))
        {
            let text = extract_text_strict(&payload["content"]).text;
            *responses.entry(text).or_default() += 1;
        }
    }
    for (index, record) in records.iter().enumerate() {
        let value = &record.value;
        if value.get("type").and_then(Value::as_str) != Some("event_msg") {
            continue;
        }
        let Some(payload) = value.get("payload") else {
            continue;
        };
        if payload.get("type").and_then(Value::as_str) != Some("agent_message")
            || payload.get("phase").and_then(Value::as_str) != Some("final_answer")
        {
            continue;
        }
        let text = payload.get("message").and_then(Value::as_str);
        let count = text.and_then(|text| responses.get_mut(text));
        match count {
            Some(count) if *count > 0 => *count -= 1,
            _ => return Err(
                crate::session_manager::migrate::model::MigrationError::FinalAnswerIndeterminate {
                    seq: index as u32,
                    reason: "known final event has no matching lossless response item".into(),
                },
            ),
        }
    }
    Ok(())
}

fn classify_record(value: &Value) -> Classification {
    if value.get("type").and_then(Value::as_str) != Some("response_item") {
        return Classification::Skip;
    }
    let Some(payload) = value.get("payload") else {
        return Classification::Indeterminate {
            reason: "response item payload missing".into(),
        };
    };
    let payload_type = payload.get("type").and_then(Value::as_str).unwrap_or("");
    if TOOL_PAYLOAD_TYPES.contains(&payload_type) {
        return Classification::ToolEvent;
    }
    if payload_type == "reasoning" {
        return Classification::Reasoning;
    }
    if payload_type != "message" {
        return Classification::Indeterminate {
            reason: format!("unclassified response item `{payload_type}`"),
        };
    }

    let role = payload.get("role").and_then(Value::as_str).unwrap_or("");
    if role == "developer" || role == "system" {
        return Classification::RuntimeInjection;
    }

    let kinds = payload
        .get("internal_chat_message_metadata_passthrough")
        .and_then(|meta| meta.get("content_item_kinds"))
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .map(|v| v.as_str().unwrap_or("<invalid>").to_string())
                .collect::<Vec<_>>()
        });

    match role {
        "user" => classify_user(payload, kinds.as_deref()),
        "assistant" => classify_assistant(payload),
        other => Classification::Indeterminate {
            reason: format!("unrecognized message role `{other}`"),
        },
    }
}

fn classify_user(payload: &Value, kinds: Option<&[String]>) -> Classification {
    let Some(kinds) = kinds else {
        return Classification::Indeterminate {
            reason: "user kinds missing".to_string(),
        };
    };
    if kinds.is_empty() {
        return Classification::Indeterminate {
            reason: "user kinds empty".to_string(),
        };
    }
    if kinds
        .iter()
        .all(|kind| USER_TEXT_KINDS.contains(&kind.as_str()))
    {
        let strict = extract_text_strict(payload.get("content").unwrap_or(&Value::Null));
        if !strict.unknown_types.is_empty() || !strict.has_text_block {
            return Classification::Indeterminate {
                reason: "unknown or missing user text block".to_string(),
            };
        }
        return Classification::RealUser {
            text: strict.text,
            omitted: strict.omitted,
        };
    }
    if kinds
        .iter()
        .all(|kind| RUNTIME_KINDS.contains(&kind.as_str()))
    {
        return Classification::RuntimeInjection;
    }
    Classification::Indeterminate {
        reason: format!("unrecognized user kinds: {}", kinds.join(",")),
    }
}

fn classify_assistant(payload: &Value) -> Classification {
    match payload.get("phase").and_then(Value::as_str) {
        Some("commentary") => Classification::Commentary,
        Some("final_answer") => {
            if !all_blocks_are_output_text(payload.get("content")) {
                return Classification::Indeterminate {
                    reason: "non-text final block".to_string(),
                };
            }
            let strict = extract_text_strict(payload.get("content").unwrap_or(&Value::Null));
            Classification::FinalAnswer {
                text: strict.text,
                omitted: strict.omitted,
            }
        }
        Some(other) => Classification::Indeterminate {
            reason: format!("unrecognized phase `{other}`"),
        },
        None => Classification::Indeterminate {
            reason: "phase unknown".to_string(),
        },
    }
}

fn all_blocks_are_output_text(content: Option<&Value>) -> bool {
    match content {
        Some(Value::Array(items)) => {
            !items.is_empty()
                && items.iter().all(|item| {
                    item.get("type").and_then(Value::as_str) == Some("output_text")
                        && item.get("text").and_then(Value::as_str).is_some()
                })
        }
        // A bare string body has no block structure to verify against the
        // observed 0.154.0 shape, so it is not accepted as a proven final
        // answer.
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session_manager::migrate::extract::build_session;
    use crate::session_manager::migrate::model::{MessageKind, MigrationError, PathFamily};
    use serde_json::json;

    /// Rollout lines in the shape recorded by the 0.154.0 isolated probe
    /// (`evidence/codex-probe/findings.md`, technical design §6.4 table).
    fn line(payload: Value) -> RawRecord {
        RawRecord {
            value: json!({
                "timestamp": "2026-09-22T01:16:15Z",
                "type": "response_item",
                "payload": payload,
            }),
            ts: Some(1_790_000_000_000),
        }
    }

    fn user_text(text: &str) -> Value {
        json!({
            "type": "message",
            "role": "user",
            "content": [{"type": "input_text", "text": text}],
            "internal_chat_message_metadata_passthrough": {
                "turn_id": "turn-1",
                "content_item_kinds": ["user.text"]
            }
        })
    }

    fn final_answer(text: &str) -> Value {
        json!({
            "type": "message",
            "role": "assistant",
            "phase": "final_answer",
            "content": [{"type": "output_text", "text": text}],
            "internal_chat_message_metadata_passthrough": {
                "turn_id": "turn-1",
                "content_item_kinds": ["assistant.text"]
            }
        })
    }

    fn session_meta() -> RawRecord {
        RawRecord {
            value: json!({
                "timestamp": "2026-09-22T01:16:14Z",
                "type": "session_meta",
                "payload": {"id": "01a0c4f7-e678-7063-8763-390ed30a27e1", "cwd": "/tmp/my-project", "cli_version": "0.154.0"}
            }),
            ts: Some(1_790_000_000_000),
        }
    }

    fn extract(
        records: Vec<RawRecord>,
    ) -> MigrationResult<crate::session_manager::migrate::model::MigratableSession> {
        extract_from_store(records, "unit-test-store", "unit-test-source")
    }

    fn extract_from_store(
        records: Vec<RawRecord>,
        store_key: &str,
        source_path: &str,
    ) -> MigrationResult<crate::session_manager::migrate::model::MigratableSession> {
        let state = tempfile::tempdir().expect("state dir");
        crate::session_manager::migrate::identity::with_local_state_dir(state.path(), || {
            let source = build_source(records, store_key.to_string(), None);
            build_session(&CodexRolloutPhaseV1, &source, source_path)
        })
    }

    #[test]
    fn keeps_user_text_and_final_answers_in_order() {
        let session = extract(vec![
            session_meta(),
            line(user_text("My codeword is maple-73.")),
            line(final_answer("Noted.")),
            line(user_text("Repeat it.")),
            line(final_answer("maple-73")),
        ])
        .expect("extract");

        let projection: Vec<(MessageKind, &str)> = session
            .messages
            .iter()
            .map(|message| (message.kind, message.text.as_str()))
            .collect();
        assert_eq!(
            projection,
            vec![
                (MessageKind::UserText, "My codeword is maple-73."),
                (MessageKind::AssistantFinal, "Noted."),
                (MessageKind::UserText, "Repeat it."),
                (MessageKind::AssistantFinal, "maple-73"),
            ]
        );
        assert_eq!(session.extraction.rule_id, RULE_ID);
        assert_eq!(session.extraction.rule_verified_versions, vec!["=0.154.0"]);
        assert!(session.extraction.open_user_messages.is_empty());
        assert_eq!(
            session.messages.iter().map(|m| m.seq).collect::<Vec<_>>(),
            vec![0, 1, 2, 3]
        );
    }

    #[test]
    fn drops_developer_and_user_role_runtime_injections() {
        let injected_as_user = json!({
            "type": "message",
            "role": "user",
            "content": [{"type": "input_text", "text": "# AGENTS.md instructions"}],
            "internal_chat_message_metadata_passthrough": {
                "content_item_kinds": ["agents_md.instructions", "environments.environment_context"]
            }
        });
        let developer = json!({
            "type": "message",
            "role": "developer",
            "content": [{"type": "input_text", "text": "<permissions>"}],
            "internal_chat_message_metadata_passthrough": {
                "content_item_kinds": ["permissions.instructions"]
            }
        });

        let session = extract(vec![
            session_meta(),
            line(developer),
            line(injected_as_user),
            line(user_text("Fix the login bug")),
            line(final_answer("Fixed.")),
        ])
        .expect("extract");

        assert_eq!(session.messages.len(), 2);
        assert_eq!(session.messages[0].text, "Fix the login bug");
        assert_eq!(session.extraction.omitted.runtime_injections, 2);
        assert!(!session
            .messages
            .iter()
            .any(|message| message.text.contains("AGENTS.md")));
    }

    #[test]
    fn commentary_is_omitted_not_treated_as_a_final_answer() {
        let commentary = json!({
            "type": "message",
            "role": "assistant",
            "phase": "commentary",
            "content": [{"type": "output_text", "text": "Checking now."}],
        });
        let session = extract(vec![
            session_meta(),
            line(user_text("Do it")),
            line(commentary),
            line(final_answer("Done")),
        ])
        .expect("extract");

        assert_eq!(session.messages.len(), 2);
        assert_eq!(session.extraction.omitted.commentary_messages, 1);
        assert!(!session
            .messages
            .iter()
            .any(|message| message.text.contains("Checking now")));
    }

    #[test]
    fn missing_phase_blocks_the_export_at_any_position() {
        let no_phase = json!({
            "type": "message",
            "role": "assistant",
            "content": [{"type": "output_text", "text": "Probably done?"}],
        });

        // Mid-transcript.
        let error = extract(vec![
            session_meta(),
            line(user_text("q1")),
            line(no_phase.clone()),
            line(user_text("q2")),
            line(final_answer("a2")),
        ])
        .expect_err("missing phase must block");
        assert!(matches!(
            error,
            MigrationError::FinalAnswerIndeterminate { ref reason, .. } if reason == "phase unknown"
        ));

        // Last turn: no exemption.
        let error = extract(vec![session_meta(), line(user_text("q1")), line(no_phase)])
            .expect_err("missing phase must block on the final turn too");
        assert_eq!(error.code(), "finalAnswerIndeterminate");
    }

    #[test]
    fn unknown_inject_only_items_are_not_re_exportable() {
        // Restored threads come back with content_item_kinds ["unknown"].
        // Accepting them would let a round trip launder unverified content.
        let injected_user = json!({
            "type": "message",
            "role": "user",
            "content": [{"type": "input_text", "text": "migrated question"}],
            "internal_chat_message_metadata_passthrough": {"content_item_kinds": ["unknown"]}
        });
        let error = extract(vec![session_meta(), line(injected_user)])
            .expect_err("unknown kinds must block");
        assert!(matches!(
            error,
            MigrationError::FinalAnswerIndeterminate { ref reason, .. }
                if reason.contains("unrecognized user kinds")
        ));
    }

    #[test]
    fn mixed_user_and_runtime_provenance_blocks_instead_of_losing_user_text() {
        let mut user = user_text("genuine question");
        user["internal_chat_message_metadata_passthrough"]["content_item_kinds"] =
            json!(["user.text", "agents_md.instructions"]);
        assert!(matches!(
            classify_record(&line(user).value),
            Classification::Indeterminate { .. }
        ));
        let mut user = user_text("genuine question");
        user["internal_chat_message_metadata_passthrough"]["content_item_kinds"] =
            json!(["user.text", null]);
        assert!(matches!(
            classify_record(&line(user).value),
            Classification::Indeterminate { .. }
        ));
    }

    #[test]
    fn recorded_final_missing_response_after_crash_blocks_and_counts_duplicates() {
        let event = || RawRecord {
            value: json!({"type":"event_msg","payload":{"type":"agent_message","phase":"final_answer","message":"same final"}}),
            ts: None,
        };
        let response = || line(final_answer("same final"));
        assert!(validate_final_events(&[event(), response()]).is_ok());
        assert!(validate_final_events(&[event(), response(), event()]).is_err());
        assert!(validate_final_events(&[event(), response(), event(), response()]).is_ok());
        let mut legacy = event();
        legacy.value["payload"]
            .as_object_mut()
            .unwrap()
            .remove("phase");
        assert!(validate_final_events(&[legacy]).is_ok());
    }

    #[test]
    fn missing_user_kinds_block_instead_of_defaulting_to_real_user() {
        let no_kinds = json!({
            "type": "message",
            "role": "user",
            "content": [{"type": "input_text", "text": "who wrote this?"}],
        });
        let error = extract(vec![session_meta(), line(no_kinds)]).expect_err("must block");
        assert!(matches!(
            error,
            MigrationError::FinalAnswerIndeterminate { ref reason, .. }
                if reason == "user kinds missing"
        ));
    }

    #[test]
    fn a_final_answer_with_a_non_text_block_is_indeterminate() {
        let mixed = json!({
            "type": "message",
            "role": "assistant",
            "phase": "final_answer",
            "content": [
                {"type": "output_text", "text": "see the image"},
                {"type": "image", "source": {}}
            ],
        });
        let error = extract(vec![session_meta(), line(user_text("q")), line(mixed)])
            .expect_err("must block");
        assert!(matches!(
            error,
            MigrationError::FinalAnswerIndeterminate { ref reason, .. }
                if reason == "non-text final block"
        ));
    }

    #[test]
    fn tool_events_and_reasoning_are_counted_and_never_inlined() {
        let call = RawRecord {
            value: json!({
                "timestamp": "2026-09-22T01:16:16Z",
                "type": "response_item",
                "payload": {"type": "function_call", "name": "shell", "call_id": "c1"}
            }),
            ts: None,
        };
        let output = RawRecord {
            value: json!({
                "timestamp": "2026-09-22T01:16:17Z",
                "type": "response_item",
                "payload": {"type": "function_call_output", "call_id": "c1",
                            "output": "TOOL_OUTPUT_MUST_NOT_BE_KEPT"}
            }),
            ts: None,
        };
        let reasoning = RawRecord {
            value: json!({
                "timestamp": "2026-09-22T01:16:18Z",
                "type": "response_item",
                "payload": {"type": "reasoning", "summary": []}
            }),
            ts: None,
        };

        let session = extract(vec![
            session_meta(),
            line(user_text("list files")),
            call,
            output,
            reasoning,
            line(final_answer("Two files.")),
        ])
        .expect("extract");

        assert_eq!(session.messages.len(), 2);
        assert_eq!(session.extraction.omitted.tool_events, 2);
        assert_eq!(session.extraction.omitted.reasoning_blocks, 1);
        let body = session
            .messages
            .iter()
            .map(|message| message.text.as_str())
            .collect::<String>();
        assert!(!body.contains("TOOL_OUTPUT_MUST_NOT_BE_KEPT"));
        assert!(!body.contains("[Tool:"));
    }

    #[test]
    fn unanswered_and_repeated_questions_stay_in_the_transcript() {
        let session = extract(vec![
            session_meta(),
            line(user_text("first ask")),
            line(user_text("resend after interrupt")),
            line(final_answer("answer")),
            line(user_text("trailing question")),
        ])
        .expect("extract");

        assert_eq!(session.messages.len(), 4);
        assert_eq!(session.extraction.open_user_messages, vec![0, 3]);
    }

    #[test]
    fn package_keeps_only_the_workspace_basename_and_path_family() {
        let session = extract(vec![
            session_meta(),
            line(user_text("q")),
            line(final_answer("a")),
        ])
        .expect("extract");

        assert_eq!(session.workspace_label.as_deref(), Some("my-project"));
        assert_eq!(session.extraction.source_path_family, PathFamily::Posix);
        let encoded = serde_json::to_string(&session).expect("encode");
        assert!(!encoded.contains("/tmp/my-project"));
    }

    #[test]
    fn body_paths_are_preserved_verbatim() {
        // Product decision: a Windows path inside an answer stays as written,
        // and the UI warns using sourcePathFamily instead.
        let session = extract(vec![
            session_meta(),
            line(user_text("where is it?")),
            line(final_answer("It lives at C:\\work\\repo\\main.rs")),
        ])
        .expect("extract");
        assert_eq!(
            session.messages[1].text,
            "It lives at C:\\work\\repo\\main.rs"
        );
    }

    #[test]
    fn identical_transcripts_from_different_sources_keep_separate_snapshots() {
        let records = || {
            vec![
                session_meta(),
                line(user_text("same question")),
                line(final_answer("same answer")),
            ]
        };
        let state = tempfile::tempdir().expect("state dir");
        let (first, second) =
            crate::session_manager::migrate::identity::with_local_state_dir(state.path(), || {
                let first = build_session(
                    &CodexRolloutPhaseV1,
                    &build_source(records(), "store-a".to_string(), None),
                    "source-a",
                )
                .expect("first");
                let second = build_session(
                    &CodexRolloutPhaseV1,
                    &build_source(records(), "store-b".to_string(), None),
                    "source-b",
                )
                .expect("second");
                (first, second)
            });

        assert_eq!(first.content_digest, second.content_digest);
        assert_ne!(first.origin.origin_id, second.origin.origin_id);
        assert_ne!(first.snapshot_id, second.snapshot_id);
    }
}
