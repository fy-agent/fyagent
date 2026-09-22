//! OpenCode 1.18.30 persisted finish/step and synthetic-part provenance.
//! `time.completed` is also written on abort, and is never the final signal.
//! Evidence: v1.18.30 session/processor.ts and implementation/extraction-source-evidence.md.
use std::collections::{HashMap, HashSet};
use std::path::Path;

use serde_json::{json, Value};

use super::super::storage::{self, MAX_SOURCE_RECORDS};
use super::super::{Classification, ExtractionRule, RawRecord, RawSource};
use crate::session_manager::migrate::model::{MigrationResult, OmittedCounts};

const PROVIDER: &str = "opencode";
pub const VERIFIED_VERSIONS: &[&str] = &["=1.18.30"];
pub struct OpenCodeFinishV1;

impl ExtractionRule for OpenCodeFinishV1 {
    fn provider_id(&self) -> &'static str {
        PROVIDER
    }
    fn rule_id(&self) -> &'static str {
        "opencode.finish-parts-v1"
    }
    fn verified_versions(&self) -> &'static [&'static str] {
        VERIFIED_VERSIONS
    }
    fn load_source(&self, source_path: &str) -> MigrationResult<RawSource> {
        if let Some(rest) = source_path.strip_prefix("sqlite:") {
            let (path, id) = rest
                .rsplit_once(":ses_")
                .ok_or_else(|| storage::unreadable(PROVIDER, "invalid SQLite session reference"))?;
            return load_sqlite(Path::new(path), &format!("ses_{id}"));
        }
        let path = Path::new(source_path)
            .canonicalize()
            .map_err(|e| storage::unreadable(PROVIDER, e))?;
        if path.is_dir() {
            return Err(storage::unreadable(
                PROVIDER,
                "legacy directory history lacks the verified 1.18.30 source envelope",
            ));
        }
        let value = storage::read_json(PROVIDER, &path)?;
        // An exported JSON file is its own source, never proof that a matching
        // session ID resides in the current machine's native database.
        source_from_export(value, path.to_string_lossy().into_owned())
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
    let mut statement = conn.prepare("SELECT id,title,directory,version,time_created,time_updated,revert FROM session WHERE id=?1")
        .map_err(|e| storage::unreadable(PROVIDER, e))?;
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
    let source_cli_version = storage::text_column(row, 3, &mut bytes)?;
    let created_at = row.get(4).map_err(|e| storage::unreadable(PROVIDER, e))?;
    let last_active_at = row.get(5).map_err(|e| storage::unreadable(PROVIDER, e))?;
    if storage::text_column(row, 6, &mut bytes)?.is_some() {
        return Err(storage::unreadable(
            PROVIDER,
            "reverted session requires a verified native rewind projection",
        ));
    }
    let mut parts_by_message: HashMap<String, Vec<Value>> = HashMap::new();
    let mut stmt = conn
        .prepare("SELECT id,message_id,data FROM part WHERE session_id=?1 ORDER BY id ASC LIMIT ?2")
        .map_err(|e| storage::unreadable(PROVIDER, e))?;
    let mut rows = stmt
        .query(rusqlite::params![id, MAX_SOURCE_RECORDS as i64 + 1])
        .map_err(|e| storage::unreadable(PROVIDER, e))?;
    let mut count = 0;
    while let Some(row) = rows.next().map_err(|e| storage::unreadable(PROVIDER, e))? {
        count += 1;
        storage::check_budget(bytes, count)?;
        let part_id = storage::text_column(row, 0, &mut bytes)?
            .ok_or_else(|| storage::unreadable(PROVIDER, "part ID missing"))?;
        let message_id = storage::text_column(row, 1, &mut bytes)?
            .ok_or_else(|| storage::unreadable(PROVIDER, "part message ID missing"))?;
        let raw = storage::text_column(row, 2, &mut bytes)?
            .ok_or_else(|| storage::unreadable(PROVIDER, "part data missing"))?;
        let mut part = storage::parse_json(PROVIDER, &raw)?;
        let object = part
            .as_object_mut()
            .ok_or_else(|| storage::unreadable(PROVIDER, "part data is not an object"))?;
        object.insert("id".into(), json!(part_id));
        parts_by_message.entry(message_id).or_default().push(part);
    }
    let mut records = Vec::new();
    let mut stmt = conn.prepare("SELECT id,time_created,data FROM message WHERE session_id=?1 ORDER BY time_created ASC,id ASC LIMIT ?2")
        .map_err(|e| storage::unreadable(PROVIDER, e))?;
    let mut rows = stmt
        .query(rusqlite::params![id, MAX_SOURCE_RECORDS as i64 + 1])
        .map_err(|e| storage::unreadable(PROVIDER, e))?;
    while let Some(row) = rows.next().map_err(|e| storage::unreadable(PROVIDER, e))? {
        storage::check_budget(bytes, records.len())?;
        let message_id = storage::text_column(row, 0, &mut bytes)?
            .ok_or_else(|| storage::unreadable(PROVIDER, "message ID missing"))?;
        let ts = row.get(1).map_err(|e| storage::unreadable(PROVIDER, e))?;
        let raw = storage::text_column(row, 2, &mut bytes)?
            .ok_or_else(|| storage::unreadable(PROVIDER, "message data missing"))?;
        let mut info = storage::parse_json(PROVIDER, &raw)?;
        info.as_object_mut()
            .ok_or_else(|| storage::unreadable(PROVIDER, "message data is not an object"))?
            .insert("id".into(), json!(message_id));
        let parts = parts_by_message.remove(&message_id).unwrap_or_default();
        records.push(RawRecord {
            value: json!({"info":info,"parts":parts}),
            ts,
        });
    }
    if !parts_by_message.is_empty() {
        return Err(storage::unreadable(PROVIDER, "orphaned native parts"));
    }
    validate_parent_links(&records)?;
    Ok(RawSource {
        records,
        session_id,
        title,
        workspace_path,
        source_cli_version,
        created_at,
        last_active_at,
        store_key: path.to_string_lossy().into_owned(),
        canonical_store_path: Some(path),
        ..RawSource::default()
    })
}

fn source_from_export(value: Value, store_key: String) -> MigrationResult<RawSource> {
    let info = value
        .get("info")
        .and_then(Value::as_object)
        .ok_or_else(|| storage::unreadable(PROVIDER, "native export info missing"))?;
    if info.get("revert").is_some_and(|v| !v.is_null()) {
        return Err(storage::unreadable(
            PROVIDER,
            "reverted session requires native rewind projection",
        ));
    }
    let messages = value
        .get("messages")
        .and_then(Value::as_array)
        .ok_or_else(|| storage::unreadable(PROVIDER, "native export messages missing"))?;
    storage::check_budget(0, messages.len())?;
    let records: Vec<_> = messages
        .iter()
        .map(|v| RawRecord {
            value: v.clone(),
            ts: v.pointer("/info/time/created").and_then(Value::as_i64),
        })
        .collect();
    validate_parent_links(&records)?;
    Ok(RawSource {
        records,
        session_id: info.get("id").and_then(Value::as_str).map(str::to_string),
        source_cli_version: info
            .get("version")
            .and_then(Value::as_str)
            .map(str::to_string),
        workspace_path: info
            .get("directory")
            .and_then(Value::as_str)
            .map(str::to_string),
        title: info
            .get("title")
            .and_then(Value::as_str)
            .map(str::to_string),
        created_at: info
            .get("time")
            .and_then(|v| v.get("created"))
            .and_then(Value::as_i64),
        last_active_at: info
            .get("time")
            .and_then(|v| v.get("updated"))
            .and_then(Value::as_i64),
        store_key,
        ..RawSource::default()
    })
}

fn validate_parent_links(records: &[RawRecord]) -> MigrationResult<()> {
    let mut users = HashSet::new();
    let mut ids = HashSet::new();
    for record in records {
        let info = &record.value["info"];
        let id = info
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| storage::unreadable(PROVIDER, "message ID missing"))?;
        if !ids.insert(id) {
            return Err(storage::unreadable(PROVIDER, "duplicate message ID"));
        }
        match info.get("role").and_then(Value::as_str) {
            Some("user") => {
                users.insert(id);
            }
            Some("assistant")
                if !info
                    .get("parentID")
                    .and_then(Value::as_str)
                    .is_some_and(|id| users.contains(id)) =>
            {
                return Err(storage::unreadable(
                    PROVIDER,
                    "assistant parent is absent or out of source order",
                ));
            }
            _ => {}
        }
    }
    Ok(())
}

fn unknown(reason: &str) -> Classification {
    Classification::Indeterminate {
        reason: format!("OpenCode: {reason}"),
    }
}

fn classify(value: &Value) -> Classification {
    let Some(info) = value.get("info") else {
        return unknown("message info missing");
    };
    let Some(parts) = value.get("parts").and_then(Value::as_array) else {
        return unknown("parts missing");
    };
    match info.get("role").and_then(Value::as_str) {
        Some("user") => classify_user(parts),
        Some("assistant") => classify_assistant(info, parts),
        _ => unknown("unknown role"),
    }
}

fn classify_user(parts: &[Value]) -> Classification {
    let mut texts = Vec::new();
    let mut omitted = OmittedCounts::default();
    let mut compaction = false;
    for part in parts {
        match part.get("type").and_then(Value::as_str) {
            Some("text") => {
                for flag in ["synthetic", "ignored"] {
                    if part.get(flag).is_some_and(|v| !v.is_boolean()) {
                        return unknown("malformed user provenance flag");
                    }
                }
                if part.get("synthetic").and_then(Value::as_bool) == Some(true)
                    || part.get("ignored").and_then(Value::as_bool) == Some(true)
                {
                    omitted.runtime_injections += 1;
                } else if let Some(text) = part.get("text").and_then(Value::as_str) {
                    texts.push(text);
                } else {
                    return unknown("user text missing");
                }
            }
            Some("file") => omitted.attachments += 1,
            Some("agent" | "subtask") => omitted.runtime_injections += 1,
            Some("compaction") => {
                omitted.runtime_injections += 1;
                compaction = true;
            }
            _ => return unknown("unclassified user part"),
        }
    }
    if compaction && !texts.is_empty() {
        return unknown("mixed compaction and user text provenance");
    }
    if texts.is_empty() {
        return Classification::RuntimeInjection;
    }
    // Distinct user blocks are independently authored parts; inserting or
    // guessing a separator would change the source transcript.
    if texts.len() != 1 {
        return unknown("multiple user text part boundaries are not representable");
    }
    Classification::RealUser {
        text: texts[0].to_string(),
        omitted,
    }
}

fn classify_assistant(info: &Value, parts: &[Value]) -> Classification {
    if info.get("error").is_some_and(|v| !v.is_null()) {
        return unknown("assistant error or interruption");
    }
    let Some(finish @ ("stop" | "tool-calls")) = info.get("finish").and_then(Value::as_str) else {
        return unknown("missing or nonfinal finish reason");
    };
    if info
        .pointer("/time/completed")
        .and_then(Value::as_i64)
        .is_none()
    {
        return unknown("assistant persistence incomplete");
    }
    if info.get("summary").and_then(Value::as_bool) == Some(true) {
        return if finish == "stop" {
            Classification::RuntimeInjection
        } else {
            unknown("unfinished compaction summary")
        };
    }
    let mut omitted = OmittedCounts::default();
    let mut texts = Vec::new();
    let mut completed_texts = Vec::new();
    let mut step_reason = None;
    let mut tool_count = 0;
    let mut step_open = false;
    for part in parts {
        match part.get("type").and_then(Value::as_str) {
            Some("text") => {
                if ["synthetic", "ignored"]
                    .iter()
                    .any(|flag| part.get(*flag).is_some_and(|v| !v.is_boolean()))
                {
                    return unknown("malformed assistant provenance flag");
                }
                if part.get("synthetic").and_then(Value::as_bool) == Some(true)
                    || part.get("ignored").and_then(Value::as_bool) == Some(true)
                {
                    omitted.runtime_injections += 1;
                    continue;
                }
                let Some(text) = part.get("text").and_then(Value::as_str) else {
                    return unknown("assistant text missing");
                };
                texts.push(text);
            }
            Some("tool") => {
                let state = &part["state"];
                if !matches!(
                    state.get("status").and_then(Value::as_str),
                    Some("completed" | "error")
                ) || state
                    .pointer("/metadata/interrupted")
                    .and_then(Value::as_bool)
                    == Some(true)
                {
                    return unknown("tool execution pending or interrupted");
                }
                tool_count += 1;
                omitted.tool_events += 1;
            }
            Some("reasoning") => {
                if step_reason == Some("stop") {
                    return unknown("reasoning after final step");
                }
                omitted.reasoning_blocks += 1;
            }
            Some("step-finish") => {
                step_open = false;
                let reason = part.get("reason").and_then(Value::as_str);
                match reason {
                    Some("tool-calls") => {
                        if !texts.is_empty() {
                            omitted.commentary_messages += 1;
                        }
                        texts.clear();
                    }
                    Some("stop") => {
                        if !completed_texts.is_empty() {
                            return unknown("multiple final steps in one message");
                        }
                        completed_texts.append(&mut texts);
                    }
                    _ => return unknown("nonfinal step reason"),
                }
                step_reason = reason;
            }
            Some("step-start") => {
                if step_open {
                    return unknown("nested unfinished step");
                }
                step_open = true;
            }
            Some("patch" | "snapshot") => {}
            Some("file") => omitted.attachments += 1,
            // A retry can leave partial text from an abandoned attempt.
            _ => return unknown("unclassified assistant part"),
        }
    }
    if step_open {
        return unknown("unfinished step after preceding completion");
    }
    if step_reason.is_some_and(|reason| reason != finish) {
        return unknown("message and step finish disagree");
    }
    if finish == "tool-calls" {
        return if tool_count > 0 {
            Classification::ToolEvent
        } else {
            unknown("tool-calls finish without a tool")
        };
    }
    let final_texts = if step_reason.is_some() {
        if !texts.is_empty() {
            return unknown("text after final step");
        }
        completed_texts
    } else {
        if tool_count > 0 {
            return unknown("text/tool mixture lacks step boundaries");
        }
        texts
    };
    if final_texts.len() != 1 || final_texts[0].is_empty() {
        return unknown("final text absent or ambiguous part boundaries");
    }
    Classification::FinalAnswer {
        text: final_texts[0].to_string(),
        omitted,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn message(role: &str, finish: Option<&str>, parts: Value) -> Value {
        json!({"info":{"id":"msg_a","role":role,"finish":finish,"time":{"completed":2}},"parts":parts})
    }
    #[test]
    fn preserves_text_and_drops_only_structurally_synthetic_parts() {
        let value = message(
            "user",
            None,
            json!([{"type":"text","text":"runtime","synthetic":true},{"type":"text","text":"  中\r\nC:\\x  "}]),
        );
        assert!(
            matches!(classify(&value), Classification::RealUser { text, .. } if text == "  中\r\nC:\\x  ")
        );
        let answer = message(
            "assistant",
            Some("stop"),
            json!([{"type":"text","text":"  final\r\n "}]),
        );
        assert!(
            matches!(classify(&answer), Classification::FinalAnswer { text, .. } if text == "  final\r\n ")
        );
    }
    #[test]
    fn ignored_assistant_parts_never_enter_final_text() {
        let value = message(
            "assistant",
            Some("stop"),
            json!([
                {"type":"text","text":"hidden draft","ignored":true},
                {"type":"text","text":"visible final"}
            ]),
        );
        assert!(
            matches!(classify(&value), Classification::FinalAnswer { text, omitted } if text == "visible final" && omitted.runtime_injections == 1)
        );
        let only_ignored = message(
            "assistant",
            Some("stop"),
            json!([
                {"type":"text","text":"hidden draft","ignored":true}
            ]),
        );
        assert!(!matches!(
            classify(&only_ignored),
            Classification::FinalAnswer { .. }
        ));
    }
    #[test]
    fn completion_timestamp_cannot_certify_aborted_text() {
        for finish in [None, Some("length"), Some("unknown")] {
            assert!(matches!(
                classify(&message(
                    "assistant",
                    finish,
                    json!([{"type":"text","text":"partial"}])
                )),
                Classification::Indeterminate { .. }
            ));
        }
    }
    #[test]
    fn excludes_tool_step_commentary_from_final_step() {
        let value = message(
            "assistant",
            Some("stop"),
            json!([
                {"type":"step-start"},{"type":"text","text":"checking"},
                {"type":"tool","state":{"status":"completed"}},
                {"type":"step-finish","reason":"tool-calls"},
                {"type":"step-start"},{"type":"reasoning","text":"hidden"},
                {"type":"text","text":"Done."},{"type":"step-finish","reason":"stop"}
            ]),
        );
        assert!(
            matches!(classify(&value), Classification::FinalAnswer { text, omitted } if text == "Done." && omitted.commentary_messages == 1 && omitted.tool_events == 1)
        );
    }
    #[test]
    fn pending_interrupted_and_unstructured_mixed_steps_block() {
        for state in [
            json!({"status":"pending"}),
            json!({"status":"error","metadata":{"interrupted":true}}),
            json!({"status":"completed"}),
        ] {
            let value = message(
                "assistant",
                Some("stop"),
                json!([{"type":"text","text":"maybe"},{"type":"tool","state":state}]),
            );
            assert!(matches!(
                classify(&value),
                Classification::Indeterminate { .. }
            ));
        }
    }
    #[test]
    fn source_version_and_external_store_are_not_invented() {
        let value = json!({"info":{"id":"ses_a","version":"1.18.30"},"messages":[
            {"info":{"id":"msg_u","role":"user"},"parts":[{"type":"text","text":"u"}]},
            {"info":{"id":"msg_a","role":"assistant","parentID":"msg_u","finish":"stop","time":{"completed":1}},"parts":[{"type":"text","text":"a"}]}
        ]});
        let source = source_from_export(value, "outside.json".into()).unwrap();
        assert_eq!(source.source_cli_version.as_deref(), Some("1.18.30"));
        assert!(source.canonical_store_path.is_none());
    }
    fn sqlite_fixture() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let db = rusqlite::Connection::open(dir.path().join("opencode.db")).unwrap();
        db.execute_batch("CREATE TABLE session(id TEXT PRIMARY KEY,title TEXT,directory TEXT,version TEXT,time_created INTEGER,time_updated INTEGER,revert TEXT); CREATE TABLE message(id TEXT PRIMARY KEY,session_id TEXT,time_created INTEGER,data TEXT); CREATE TABLE part(id TEXT PRIMARY KEY,message_id TEXT,session_id TEXT,data TEXT); INSERT INTO session VALUES('ses_fixture','Synthetic','/tmp/project','1.18.30',1700000000000,1700000000004,NULL);").unwrap();
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
            let id = format!("msg_{index:03}");
            let mut info = json!({"role":role,"time":{"created":1700000000000_i64+index as i64}});
            if *role == "assistant" {
                info["parentID"] = json!("msg_001");
                info["finish"] = json!("stop");
                info["time"]["completed"] = json!(1700000000100_i64 + index as i64);
            }
            db.execute(
                "INSERT INTO message VALUES(?1,'ses_fixture',?2,?3)",
                rusqlite::params![id, 1700000000000_i64 + index as i64, info.to_string()],
            )
            .unwrap();
            db.execute(
                "INSERT INTO part VALUES(?1,?2,'ses_fixture',?3)",
                rusqlite::params![
                    format!("prt_{index:03}"),
                    id,
                    json!({"type":"text","text":text}).to_string()
                ],
            )
            .unwrap();
        }
        dir
    }

    #[test]
    fn native_sqlite_preserves_consecutive_messages_and_missing_answers() {
        let dir = sqlite_fixture();
        let db = dir.path().join("opencode.db");
        let source = format!("sqlite:{}:ses_fixture", db.display());
        let state = tempfile::tempdir().unwrap();
        crate::session_manager::migrate::identity::with_local_state_dir(state.path(), || {
            let (session, raw) = super::super::super::extract_session_with_source(
                "opencode",
                &source,
                Some("999.0.0"),
            )
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
            assert_eq!(session.origin.cli_version.as_deref(), Some("1.18.30"));
            assert_eq!(raw.canonical_store_path, Some(db.canonicalize().unwrap()));
        });
    }

    #[test]
    fn old_source_version_is_not_certified_by_new_installed_cli() {
        let dir = sqlite_fixture();
        let path = dir.path().join("opencode.db");
        let db = rusqlite::Connection::open(&path).unwrap();
        db.execute("UPDATE session SET version='1.17.0'", [])
            .unwrap();
        let source = format!("sqlite:{}:ses_fixture", path.display());
        let error =
            super::super::super::extract_session("opencode", &source, Some("1.18.30")).unwrap_err();
        assert!(
            matches!(error, crate::session_manager::migrate::model::MigrationError::ExtractionRuleVersionMismatch { detected:Some(v), .. } if v == "1.17.0")
        );
    }

    #[test]
    fn ambiguity_in_middle_blocks_whole_sqlite_export() {
        let dir = sqlite_fixture();
        let path = dir.path().join("opencode.db");
        let db = rusqlite::Connection::open(&path).unwrap();
        db.execute(
            "UPDATE message SET data=json_remove(data,'$.finish') WHERE id='msg_002'",
            [],
        )
        .unwrap();
        let source = format!("sqlite:{}:ses_fixture", path.display());
        let error =
            super::super::super::extract_session("opencode", &source, Some("1.18.30")).unwrap_err();
        assert!(matches!(
            error,
            crate::session_manager::migrate::model::MigrationError::FinalAnswerIndeterminate { .. }
        ));
    }
}
