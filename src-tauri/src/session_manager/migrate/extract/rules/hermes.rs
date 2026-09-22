//! Hermes 0.20.5 native SQLite rows, before its lossy display/replay projection.
//! No source producer version is persisted. We require the verified schema,
//! explicit finish/provenance fields, and keep origin.cliVersion unknown.
use std::collections::HashSet;
use std::path::Path;

use serde_json::{json, Value};

use super::super::storage::{self, MAX_SOURCE_RECORDS};
use super::super::{parse_timestamp_ms, Classification, ExtractionRule, RawRecord, RawSource};
use crate::session_manager::migrate::model::{MigrationError, MigrationResult, OmittedCounts};

const PROVIDER: &str = "hermes";
pub const VERIFIED_VERSIONS: &[&str] = &["=0.20.5"];
pub struct HermesFinishV1;

impl ExtractionRule for HermesFinishV1 {
    fn provider_id(&self) -> &'static str {
        PROVIDER
    }
    fn rule_id(&self) -> &'static str {
        "hermes.finish-provenance-v1"
    }
    fn verified_versions(&self) -> &'static [&'static str] {
        VERIFIED_VERSIONS
    }
    fn load_source(&self, source_path: &str) -> MigrationResult<RawSource> {
        let (path, id) = source_path.strip_prefix("sqlite:").and_then(|v| v.rsplit_once('#'))
            .filter(|(_, id)| !id.is_empty()).ok_or_else(|| storage::unreadable(PROVIDER,
                "verified extraction requires the native SQLite source; legacy JSONL lacks the required provenance columns"))?;
        load_sqlite(Path::new(path), id)
    }
    fn classify(&self, record: &RawRecord) -> Classification {
        classify(&record.value)
    }
}

fn load_sqlite(path: &Path, id: &str) -> MigrationResult<RawSource> {
    let path = path
        .canonicalize()
        .map_err(|e| storage::unreadable(PROVIDER, e))?;
    let conn = storage::open_readonly(PROVIDER, &path)?;
    let mut bytes = 0;
    let mut statement = conn.prepare("SELECT id,title,cwd,started_at,last_activity_at,parent_session_id,end_reason FROM sessions WHERE id=?1")
        .map_err(|e| storage::unreadable(PROVIDER, format!("native provenance schema unavailable: {e}")))?;
    let mut rows = statement
        .query([id])
        .map_err(|e| storage::unreadable(PROVIDER, e))?;
    let row = rows
        .next()
        .map_err(|e| storage::unreadable(PROVIDER, e))?
        .ok_or_else(|| storage::unreadable(PROVIDER, "session not found"))?;
    let session_id = storage::text_column(row, 0, &mut bytes)?;
    let title = storage::text_column(row, 1, &mut bytes)?;
    let workspace_path = storage::text_column(row, 2, &mut bytes)?;
    let created_at = row
        .get::<_, Option<f64>>(3)
        .map_err(|e| storage::unreadable(PROVIDER, e))?
        .map(|v| (v * 1000.0) as i64);
    let last_active_at = row
        .get::<_, Option<f64>>(4)
        .map_err(|e| storage::unreadable(PROVIDER, e))?
        .map(|v| (v * 1000.0) as i64);
    if storage::text_column(row, 5, &mut bytes)?.is_some()
        || storage::text_column(row, 6, &mut bytes)?.as_deref() == Some("compression")
    {
        return Err(storage::unreadable(
            PROVIDER,
            "compression/child lineage requires an independently verified full-history projection",
        ));
    }
    // Fail when required columns are absent. In particular, silently selecting
    // only role/content would turn old runtime injections into human messages.
    let mut statement = conn.prepare("SELECT role,content,tool_calls,tool_call_id,tool_name,finish_reason,display_kind,display_metadata,timestamp,compacted,reasoning,reasoning_content,reasoning_details FROM messages WHERE session_id=?1 AND active=1 ORDER BY id ASC LIMIT ?2")
        .map_err(|e| storage::unreadable(PROVIDER, format!("native provenance schema unavailable: {e}")))?;
    let mut rows = statement
        .query(rusqlite::params![id, MAX_SOURCE_RECORDS as i64 + 1])
        .map_err(|e| storage::unreadable(PROVIDER, e))?;
    let mut records = Vec::new();
    while let Some(row) = rows.next().map_err(|e| storage::unreadable(PROVIDER, e))? {
        storage::check_budget(bytes, records.len())?;
        let role = storage::text_column(row, 0, &mut bytes)?;
        let content = decode_content(storage::text_column(row, 1, &mut bytes)?)?;
        let tool_calls = decode_optional_json(storage::text_column(row, 2, &mut bytes)?)?;
        let tool_call_id = storage::text_column(row, 3, &mut bytes)?;
        let tool_name = storage::text_column(row, 4, &mut bytes)?;
        let finish_reason = storage::text_column(row, 5, &mut bytes)?;
        let display_kind = storage::text_column(row, 6, &mut bytes)?;
        let display_metadata = decode_optional_json(storage::text_column(row, 7, &mut bytes)?)?;
        let timestamp: Option<f64> = row.get(8).map_err(|e| storage::unreadable(PROVIDER, e))?;
        if row
            .get::<_, i64>(9)
            .map_err(|e| storage::unreadable(PROVIDER, e))?
            != 0
        {
            return Err(storage::unreadable(
                PROVIDER,
                "compacted row cannot prove original text provenance",
            ));
        }
        let mut reasoning_count = 0;
        for column in 10..=12 {
            if storage::text_column(row, column, &mut bytes)?.is_some_and(|v| !v.is_empty()) {
                reasoning_count += 1;
            }
        }
        let value = json!({"role":role,"content":content,"tool_calls":tool_calls,"tool_call_id":tool_call_id,
            "tool_name":tool_name,"finish_reason":finish_reason,"display_kind":display_kind,"display_metadata":display_metadata,
            "reasoning_count":reasoning_count});
        records.push(RawRecord {
            value,
            ts: timestamp.and_then(|v| parse_timestamp_ms(&json!(v))),
        });
    }
    validate_tool_chain(&records)?;
    Ok(RawSource {
        records,
        session_id,
        title,
        workspace_path,
        created_at,
        last_active_at,
        store_key: path.to_string_lossy().into_owned(),
        canonical_store_path: Some(path),
        ..RawSource::default()
    })
}

fn decode_optional_json(raw: Option<String>) -> MigrationResult<Value> {
    raw.filter(|v| !v.is_empty())
        .map_or(Ok(Value::Null), |v| storage::parse_json(PROVIDER, &v))
}

fn decode_content(raw: Option<String>) -> MigrationResult<Value> {
    match raw {
        Some(raw) => match raw.strip_prefix("\0json:") {
            Some(json) => storage::parse_json(PROVIDER, json),
            None => Ok(Value::String(raw)),
        },
        None => Ok(Value::Null),
    }
}

fn indeterminate(reason: &str) -> Classification {
    Classification::Indeterminate {
        reason: format!("Hermes: {reason}"),
    }
}

fn calls(value: &Value) -> Result<Vec<&str>, ()> {
    match value.get("tool_calls") {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(Value::Array(items)) => items
            .iter()
            .map(|v| {
                v.get("id")
                    .and_then(Value::as_str)
                    .filter(|v| !v.is_empty())
                    .ok_or(())
            })
            .collect(),
        _ => Err(()),
    }
}

fn validate_tool_chain(records: &[RawRecord]) -> MigrationResult<()> {
    let mut pending = HashSet::new();
    for (index, record) in records.iter().enumerate() {
        let value = &record.value;
        let fail = |reason: &str| MigrationError::FinalAnswerIndeterminate {
            seq: index as u32,
            reason: format!("Hermes: {reason}"),
        };
        let tool_calls = calls(value).map_err(|_| fail("malformed tool calls"))?;
        match value.get("role").and_then(Value::as_str) {
            Some("assistant") if !tool_calls.is_empty() => {
                if !pending.is_empty() {
                    return Err(fail("new tool step before preceding results"));
                }
                for id in tool_calls {
                    if !pending.insert(id) {
                        return Err(fail("duplicate tool call ID"));
                    }
                }
            }
            Some("tool") => {
                let id = value
                    .get("tool_call_id")
                    .and_then(Value::as_str)
                    .ok_or_else(|| fail("tool result ID missing"))?;
                if !pending.remove(id) {
                    return Err(fail("unmatched tool result"));
                }
            }
            Some("user" | "assistant") if !pending.is_empty() => {
                return Err(fail("tool results missing before next visible turn"))
            }
            _ => {}
        }
    }
    if !pending.is_empty() {
        return Err(MigrationError::FinalAnswerIndeterminate {
            seq: records.len() as u32,
            reason: "Hermes: unfinished tool chain".into(),
        });
    }
    Ok(())
}

fn classify(value: &Value) -> Classification {
    let role = value.get("role").and_then(Value::as_str);
    if matches!(role, Some("system" | "developer")) {
        return Classification::RuntimeInjection;
    }
    if role == Some("tool") {
        return Classification::ToolEvent;
    }
    if !matches!(role, Some("user" | "assistant")) {
        return indeterminate("unknown role");
    }
    match value.get("display_kind") {
        Some(Value::String(kind)) if !kind.is_empty() => return Classification::RuntimeInjection,
        None | Some(Value::Null | Value::String(_)) => {}
        _ => return indeterminate("malformed display provenance"),
    }
    if value.get("display_metadata").is_some_and(|v| !v.is_null()) {
        return indeterminate("display metadata lacks a classified display kind");
    }
    if value.get("_compressed_summary").and_then(Value::as_bool) == Some(true)
        || value
            .get("_todo_snapshot_synthetic")
            .and_then(Value::as_bool)
            == Some(true)
    {
        return Classification::RuntimeInjection;
    }
    let Ok(tool_calls) = calls(value) else {
        return indeterminate("malformed tool calls");
    };
    if role == Some("assistant") {
        match value.get("finish_reason").and_then(Value::as_str) {
            Some("tool_calls") if !tool_calls.is_empty() => return Classification::ToolEvent,
            Some("stop")
                if tool_calls.is_empty() && value.get("tool_name").is_none_or(Value::is_null) => {}
            _ => return indeterminate("missing, truncated or nonfinal finish reason"),
        }
    } else if !tool_calls.is_empty() {
        return indeterminate("user row carries tool calls");
    }
    let content = value.get("content").unwrap_or(&Value::Null);
    let mut omitted = OmittedCounts {
        reasoning_blocks: value
            .get("reasoning_count")
            .and_then(Value::as_u64)
            .unwrap_or(0) as u32,
        ..OmittedCounts::default()
    };
    let text = match content {
        Value::String(text) => text.clone(),
        Value::Array(parts) => {
            let mut texts = Vec::new();
            for part in parts {
                match part.get("type").and_then(Value::as_str) {
                    Some("text") => match part.get("text").and_then(Value::as_str) {
                        Some(text) => texts.push(text),
                        None => return indeterminate("text block malformed"),
                    },
                    Some("image_url" | "input_image" | "image" | "file" | "input_audio")
                        if role == Some("user") =>
                    {
                        omitted.attachments += 1
                    }
                    _ => return indeterminate("unknown content block"),
                }
            }
            if texts.is_empty() && omitted.attachments > 0 {
                return Classification::Attachment;
            }
            if texts.len() != 1 {
                return indeterminate("multiple or absent text boundaries");
            }
            texts[0].to_string()
        }
        _ => return indeterminate("clean content missing; api_content cannot replace user text"),
    };
    if ambiguous_runtime_marker(&text) {
        return indeterminate(
            "native runtime marker has no durable provenance; cannot distinguish literal user text",
        );
    }
    if role == Some("user") {
        Classification::RealUser { text, omitted }
    } else if text.is_empty() {
        indeterminate("empty final text")
    } else {
        Classification::FinalAnswer { text, omitted }
    }
}

/// These are exact installed-runtime wire markers, not a language heuristic.
/// Their underscore metadata is lost in SessionDB. A collision BLOCKS instead
/// of quietly deleting a user's literal copy of the same text.
fn ambiguous_runtime_marker(text: &str) -> bool {
    let text = text.trim();
    const PREFIXES: &[&str] = &[
        "[CONTEXT COMPACTION — REFERENCE ONLY]",
        "[CONTEXT SUMMARY]:",
        "[IMPORTANT: Background process ",
        "[Your active task list was preserved across context compression]\n",
        "[System: Your previous tool call ",
    ];
    PREFIXES.iter().any(|v| text.starts_with(v))
        || text.contains("[END OF PRIOR CONTEXT — COMPACTION SUMMARY BELOW]")
        || RUNTIME_TEXTS.contains(&text)
}

const RUNTIME_TEXTS: &[&str] = &[
    "Continue from the compressed conversation context above. This marker exists because no human user turn was available.",
    "Continue from the compressed conversation context above. This marker exists because the compacted transcript contained no preserved user turn.",
    "You've reached the maximum number of tool-calling iterations allowed. Please provide a final response summarizing what you've found and accomplished so far, without calling any more tools.",
    "[System: The previous response was cut off by a network error mid-stream. Continue exactly where you left off. Do not restart or repeat prior text. Finish the answer directly.]",
    "[System: Your previous response was truncated by the output length limit. Continue exactly where you left off. Do not restart or repeat prior text. Finish the answer directly.]",
    "[System: Your previous response contained only internal reasoning and never produced a visible answer or tool call. Do not keep thinking. Produce your final answer as plain text now (or make the tool call you were planning).]",
    "[System: Continue now. Execute the required tool calls and only send your final answer after completing the task.]",
    "Your previous turn indicated a tool call but none was included. Do not narrate a plan or restate intent — issue the actual tool call now to continue the task.",
    "You just executed tool calls but returned an empty response. Please process the tool results above and continue with the task.",
];

#[cfg(test)]
mod tests {
    use super::*;
    fn raw(value: Value) -> RawRecord {
        RawRecord { value, ts: None }
    }
    #[test]
    fn clean_user_content_survives_without_api_sidecar_or_trimming() {
        let value = json!({"role":"user","content":"  中\r\nC:\\work  ","api_content":"INJECTED"});
        assert!(
            matches!(classify(&value), Classification::RealUser { text, .. } if text == "  中\r\nC:\\work  ")
        );
        assert!(matches!(
            classify(&json!({"role":"user","api_content":"injected"})),
            Classification::Indeterminate { .. }
        ));
    }
    #[test]
    fn requires_positive_finish_not_just_last_assistant_position() {
        for finish in [
            Value::Null,
            json!("length"),
            json!("error"),
            json!("content_filter"),
            json!("new_reason"),
        ] {
            assert!(matches!(
                classify(
                    &json!({"role":"assistant","content":"looks final","finish_reason":finish})
                ),
                Classification::Indeterminate { .. }
            ));
        }
        assert!(
            matches!(classify(&json!({"role":"assistant","content":"  final\r\n ","finish_reason":"stop"})), Classification::FinalAnswer { text, .. } if text == "  final\r\n ")
        );
    }
    #[test]
    fn explicit_injection_is_omitted_but_untyped_marker_collision_blocks() {
        assert_eq!(
            classify(
                &json!({"role":"user","content":"notification","display_kind":"internal_notification"})
            ),
            Classification::RuntimeInjection
        );
        assert!(matches!(
            classify(&json!({"role":"user","content":RUNTIME_TEXTS[0]})),
            Classification::Indeterminate { .. }
        ));
        assert!(matches!(
            classify(&json!({"role":"user","content":"ordinary [System: quoted word"})),
            Classification::RealUser { .. }
        ));
    }
    #[test]
    fn complete_tool_chain_is_omitted_and_incomplete_chain_blocks() {
        let records = vec![
            raw(
                json!({"role":"assistant","finish_reason":"tool_calls","tool_calls":[{"id":"c1"}]}),
            ),
            raw(json!({"role":"tool","tool_call_id":"c1","content":"secret"})),
            raw(json!({"role":"assistant","finish_reason":"stop","content":"answer"})),
        ];
        assert!(validate_tool_chain(&records).is_ok());
        assert!(matches!(
            classify(&records[0].value),
            Classification::ToolEvent
        ));
        assert!(validate_tool_chain(&records[..1]).is_err());
        assert!(validate_tool_chain(&records[1..]).is_err());
    }
    #[test]
    fn native_json_content_prefix_decodes_without_guessing_plain_json() {
        assert_eq!(
            decode_content(Some("[1,2]".into())).unwrap(),
            json!("[1,2]")
        );
        assert_eq!(
            decode_content(Some("\0json:[{\"type\":\"text\",\"text\":\"hi\"}]".into())).unwrap(),
            json!([{"type":"text","text":"hi"}])
        );
        assert!(decode_content(Some("\0json:broken".into())).is_err());
    }
    fn sqlite_fixture() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let db = rusqlite::Connection::open(dir.path().join("state.db")).unwrap();
        db.execute_batch("CREATE TABLE sessions(id TEXT PRIMARY KEY,title TEXT,cwd TEXT,started_at REAL,last_activity_at REAL,parent_session_id TEXT,end_reason TEXT); CREATE TABLE messages(id INTEGER PRIMARY KEY,session_id TEXT,role TEXT,content TEXT,tool_calls TEXT,tool_call_id TEXT,tool_name TEXT,finish_reason TEXT,display_kind TEXT,display_metadata TEXT,timestamp REAL,compacted INTEGER DEFAULT 0,active INTEGER DEFAULT 1,reasoning TEXT,reasoning_content TEXT,reasoning_details TEXT,api_content TEXT); INSERT INTO sessions VALUES('fixture','Synthetic','/tmp/project',1700000000,1700000001,NULL,NULL);").unwrap();
        for (index, (role, text)) in [
            ("user", " First\r\n "),
            ("user", "重试"),
            ("assistant", " Answer "),
            ("assistant", "Second final"),
            ("user", "Unanswered"),
        ]
        .iter()
        .enumerate()
        {
            db.execute("INSERT INTO messages(id,session_id,role,content,finish_reason,timestamp,api_content) VALUES(?1,'fixture',?2,?3,?4,1700000000,'MUST_NOT_EXPORT_API_CONTEXT')",rusqlite::params![index,role,text,(*role == "assistant").then_some("stop")]).unwrap();
        }
        dir
    }

    #[test]
    fn native_sqlite_order_raw_bodies_and_unknown_source_version_are_preserved() {
        let dir = sqlite_fixture();
        let path = dir.path().join("state.db");
        let source = format!("sqlite:{}#fixture", path.display());
        let state = tempfile::tempdir().unwrap();
        crate::session_manager::migrate::identity::with_local_state_dir(state.path(), || {
            let (session, raw) =
                super::super::super::extract_session_with_source("hermes", &source, Some("0.20.5"))
                    .unwrap();
            assert_eq!(
                session
                    .messages
                    .iter()
                    .map(|m| m.text.as_str())
                    .collect::<Vec<_>>(),
                vec![
                    " First\r\n ",
                    "重试",
                    " Answer ",
                    "Second final",
                    "Unanswered"
                ]
            );
            assert_eq!(session.extraction.open_user_messages, vec![0, 4]);
            assert!(session.origin.cli_version.is_none());
            assert_eq!(raw.canonical_store_path, Some(path.canonicalize().unwrap()));
        });
    }

    #[test]
    fn missing_middle_finish_blocks_even_when_later_answer_has_stop() {
        let dir = sqlite_fixture();
        let path = dir.path().join("state.db");
        let db = rusqlite::Connection::open(&path).unwrap();
        db.execute("UPDATE messages SET finish_reason=NULL WHERE id=2", [])
            .unwrap();
        let error = super::super::super::extract_session(
            "hermes",
            &format!("sqlite:{}#fixture", path.display()),
            Some("0.20.5"),
        )
        .unwrap_err();
        assert!(matches!(
            error,
            MigrationError::FinalAnswerIndeterminate { .. }
        ));
    }

    #[test]
    fn old_schema_cannot_silently_drop_missing_provenance() {
        let dir = sqlite_fixture();
        let path = dir.path().join("state.db");
        let db = rusqlite::Connection::open(&path).unwrap();
        db.execute("ALTER TABLE messages DROP COLUMN display_kind", [])
            .unwrap();
        assert!(load_sqlite(&path, "fixture").is_err());
    }
}
