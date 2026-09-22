//! Codex 0.154.0 native history projection.
//!
//! A response-item-only injection restores model context but creates no visible
//! turns. Codex's own external-session importer emits both UI events and model
//! response items in a legacy rollout. We use that versioned representation for
//! a NEW native session, never copying source runtime configuration or logs.
//! Source: openai/codex rust-v0.154.0, sessions/export.rs and protocol.rs.

use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use super::super::identity::content_digest;
use super::super::model::{
    MessageKind, MigratableMessage, MigratableSession, MigrationError, MigrationResult,
};
use super::{
    NativeRestoreInput, NativeSessionWriter, NativeWriteContext, NativeWriteOutcome, NonceCarrier,
    ReadbackVerdict,
};

pub(super) const VERIFIED_VERSION: &str = "0.154.0";

pub struct CodexWriter;

fn protocol_error(method: &str, reason: impl Into<String>) -> MigrationError {
    MigrationError::NativeProtocolFailed {
        provider_id: "codex".into(),
        method: method.into(),
        reason: reason.into(),
    }
}

/// Used by export as well as restore: matching a native ID in an unrelated
/// copied store never inherits this installation's source mapping.
pub(crate) fn store_identity_at(home: &Path) -> MigrationResult<String> {
    let installation = std::fs::read_to_string(home.join("installation_id"))
        .ok()
        .and_then(|id| uuid::Uuid::parse_str(id.trim()).ok())
        .ok_or_else(|| MigrationError::TargetStoreUnidentified {
            provider_id: "codex".into(),
        })?;
    let instance = super::super::identity::store_instance_id("codex", home)?;
    Ok(format!("codex:{installation}:{instance}"))
}

impl NativeSessionWriter for CodexWriter {
    fn provider_id(&self) -> &'static str {
        "codex"
    }

    fn write_strategy(&self) -> &'static str {
        "codex.legacy-rollout.v1"
    }

    fn verified_write_versions(&self) -> &'static [&'static str] {
        &["=0.154.0"]
    }

    fn resolve_target_store_id(&self) -> MigrationResult<String> {
        store_identity_at(&crate::codex_config::get_codex_config_dir())
    }

    fn restore(
        &self,
        input: &NativeRestoreInput,
        context: &dyn NativeWriteContext,
    ) -> NativeWriteOutcome {
        let no_effect = |error| NativeWriteOutcome::ProvenNoSideEffect { error };
        // Admission/readback must be possible before creating a native file.
        if let Err(error) = cli_path() {
            return no_effect(error);
        }
        match self.resolve_target_store_id() {
            Ok(store) if store == input.target_store_id => {}
            _ => {
                return no_effect(MigrationError::TargetStoreUnidentified {
                    provider_id: "codex".into(),
                })
            }
        }
        let id = match input.target_native_id.as_deref() {
            Some(id) => match uuid::Uuid::parse_str(id) {
                Ok(parsed) if parsed.to_string() == id => id.to_owned(),
                _ => {
                    return no_effect(protocol_error(
                        "identity",
                        "invalid preallocated session ID",
                    ))
                }
            },
            None => uuid::Uuid::new_v4().to_string(),
        };
        let now = chrono::Utc::now();
        let timestamp = now.to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let bytes = match encode_rollout(
            &input.session.messages,
            &id,
            &input.target_workspace,
            &timestamp,
        ) {
            Ok(bytes) => bytes,
            Err(error) => return no_effect(error),
        };
        // This ID is allocated locally, unlike thread/start. Persist it BEFORE
        // any native file creation; a crash can always reconcile that exact ID.
        if let Err(error) = context.record_native_id(&id) {
            return no_effect(error);
        }
        let home = crate::codex_config::get_codex_config_dir();
        match rollout_exists(&home, &id) {
            Ok(false) => {}
            Ok(true) => {
                return NativeWriteOutcome::Unresolved {
                    error: protocol_error("create", "native session ID already exists"),
                }
            }
            Err(error) => return NativeWriteOutcome::Unresolved { error },
        }
        let folder = home
            .join("sessions")
            .join(now.format("%Y/%m/%d").to_string());
        if let Err(error) = std::fs::create_dir_all(&folder) {
            return no_effect(protocol_error("create", error.to_string()));
        }
        let path = folder.join(format!(
            "rollout-{}-{id}.jsonl",
            now.format("%Y-%m-%dT%H-%M-%S")
        ));
        // A staged, synced file is published without replacing any destination.
        // Neither a collision nor a later failure may overwrite user history.
        let mut file = match tempfile::NamedTempFile::new_in(&folder) {
            Ok(file) => file,
            Err(error) => return no_effect(protocol_error("create", error.to_string())),
        };
        if let Err(error) = file
            .write_all(&bytes)
            .and_then(|_| file.as_file().sync_all())
        {
            return no_effect(protocol_error("stage", error.to_string()));
        }
        if let Err(error) = file.persist_noclobber(&path) {
            return NativeWriteOutcome::Unresolved {
                error: protocol_error("persist", error.error.to_string()),
            };
        }
        #[cfg(target_os = "macos")]
        if let Err(error) = std::fs::File::open(&folder).and_then(|directory| directory.sync_all())
        {
            return NativeWriteOutcome::Unresolved {
                error: protocol_error("persist", error.to_string()),
            };
        }
        NativeWriteOutcome::Written {
            target_native_id: Some(id),
            nonce_carrier: NonceCarrier::NotUsed,
        }
    }

    fn verify_readback(
        &self,
        native_id: &str,
        expected: &MigratableSession,
    ) -> MigrationResult<ReadbackVerdict> {
        uuid::Uuid::parse_str(native_id)
            .map_err(|_| protocol_error("thread/read", "Invalid native ID"))?;
        let mut rpc = CodexReadClient::start(&crate::codex_config::get_codex_config_dir())?;
        let response = rpc.call(
            "thread/read",
            json!({"threadId":native_id,"includeTurns":true}),
        )?;
        let thread = response
            .get("thread")
            .ok_or_else(|| protocol_error("thread/read", "Missing thread"))?;
        if thread.get("id").and_then(Value::as_str) != Some(native_id) {
            return Err(protocol_error("thread/read", "Unexpected thread identity"));
        }
        let observed = content_digest(&decode_visible_history(thread)?);
        if observed != expected.content_digest {
            return Ok(ReadbackVerdict::Mismatch { observed });
        }
        Ok(ReadbackVerdict::Visible {
            observed_digest: observed,
        })
    }

    fn locate_by_nonce(&self, _nonce: &str) -> MigrationResult<Vec<String>> {
        // We always record the target ID before publishing the file. A missing
        // mapping cannot be reconstructed from content or a time window.
        Ok(Vec::new())
    }
}

/// A retry can cross a date boundary; checking only today's publication path
/// would miss an older rollout with the same native identity. Never follow
/// symlinks or interpret user content while checking native filenames.
fn rollout_exists(home: &Path, native_id: &str) -> MigrationResult<bool> {
    let suffix = format!("-{native_id}.jsonl");
    let mut pending = vec![home.join("sessions"), home.join("archived_sessions")];
    let mut visited = 0_usize;
    while let Some(directory) = pending.pop() {
        let entries = match std::fs::read_dir(&directory) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(_) => {
                return Err(protocol_error(
                    "identity",
                    "unable to inspect existing native IDs",
                ))
            }
        };
        for entry in entries {
            let entry = entry
                .map_err(|_| protocol_error("identity", "unable to inspect native directory"))?;
            visited += 1;
            if visited > 100_000 {
                return Err(protocol_error(
                    "identity",
                    "native ID check exceeded its directory limit",
                ));
            }
            if entry.file_name().to_string_lossy().ends_with(&suffix) {
                return Ok(true);
            }
            let kind = entry
                .file_type()
                .map_err(|_| protocol_error("identity", "unable to inspect native entry"))?;
            if kind.is_dir() {
                pending.push(entry.path());
            }
        }
    }
    Ok(false)
}

fn cli_path() -> MigrationResult<PathBuf> {
    if super::user_cli_execution_blocked().is_some() {
        return Err(protocol_error(
            "start",
            "Ordinary-user Codex execution is unavailable in this Windows build",
        ));
    }
    let executable =
        super::provider_cli_path("codex").ok_or_else(|| MigrationError::ProviderNotInstalled {
            provider_id: "codex".into(),
        })?;
    // The general probe may have resolved another PATH entry. Verify the
    // executable this adapter will actually use before recording a native ID.
    let mut command = Command::new(&executable);
    command.arg("--version");
    let detected = match super::run_with_timeout(command, Duration::from_secs(5)) {
        super::RunOutcome::Exited {
            status: Some(0),
            stdout,
            stderr,
        } => super::super::extract::normalize_version(&format!("{stdout}\n{stderr}"))
            .map(|version| version.to_string()),
        _ => None,
    };
    if detected.as_deref() != Some(VERIFIED_VERSION) {
        return Err(MigrationError::ProviderVersionUnsupported {
            provider_id: "codex".into(),
            detected,
            verified: vec![format!("={VERIFIED_VERSION}")],
        });
    }
    Ok(executable)
}

/// Read-only JSON-RPC session. No inference, tools, trust changes, or resume
/// operation is reachable through the production verification caller.
struct CodexReadClient {
    child: Child,
    input: ChildStdin,
    responses: Receiver<Result<Value, String>>,
    next_id: u64,
}

impl CodexReadClient {
    fn start(home: &Path) -> MigrationResult<Self> {
        let executable = cli_path()?;
        let mut command = Command::new(&executable);
        #[cfg(target_os = "macos")]
        {
            use std::os::unix::process::CommandExt;
            command.process_group(0);
        }
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            crate::windows_runtime::configure_shell_user_command(&mut command, executable.parent())
                .map_err(|error| protocol_error("start", error.to_string()))?;
            command.creation_flags(0x08000000);
        }
        command
            .args(["app-server", "--stdio"])
            .env("CODEX_HOME", home)
            .current_dir(home)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let mut child = command
            .spawn()
            .map_err(|error| protocol_error("start", error.to_string()))?;
        let input = child.stdin.take().expect("piped stdin");
        let stdout = child.stdout.take().expect("piped stdout");
        let (sender, responses) = mpsc::channel();
        std::thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            const MAX_FRAME: u64 = 256 * 1024 * 1024;
            let mut total = 0_u64;
            loop {
                let mut frame = Vec::new();
                let result = (&mut reader)
                    .take(MAX_FRAME + 1)
                    .read_until(b'\n', &mut frame);
                match result {
                    Ok(0) => break,
                    Ok(size) => {
                        total += size as u64;
                        if size as u64 > MAX_FRAME || total > MAX_FRAME * 2 {
                            let _ =
                                sender.send(Err("Codex readback output exceeded the limit".into()));
                            break;
                        }
                        let parsed = serde_json::from_slice(&frame)
                            .map_err(|_| "Invalid Codex JSON-RPC frame".into());
                        if sender.send(parsed).is_err() {
                            break;
                        }
                    }
                    Err(_) => {
                        let _ = sender.send(Err("Codex readback stream failed".into()));
                        break;
                    }
                }
            }
        });
        let mut client = Self {
            child,
            input,
            responses,
            next_id: 0,
        };
        client.call("initialize", json!({"clientInfo":{"name":"fyagent","version":env!("CARGO_PKG_VERSION")},"capabilities":{"experimentalApi":true}}))?;
        client.send(json!({"method":"initialized","params":{}}))?;
        Ok(client)
    }

    fn send(&mut self, message: Value) -> MigrationResult<()> {
        serde_json::to_writer(&mut self.input, &message)
            .map_err(|_| protocol_error("send", "Failed to send request"))?;
        self.input
            .write_all(b"\n")
            .and_then(|_| self.input.flush())
            .map_err(|_| protocol_error("send", "Failed to send request"))
    }

    fn call(&mut self, method: &str, params: Value) -> MigrationResult<Value> {
        self.next_id += 1;
        let id = self.next_id;
        self.send(json!({"id":id,"method":method,"params":params}))?;
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            let wait = deadline.saturating_duration_since(Instant::now());
            let message = self
                .responses
                .recv_timeout(wait)
                .map_err(|_| protocol_error(method, "Codex readback timed out or disconnected"))?
                .map_err(|reason| protocol_error(method, reason))?;
            if message.get("id").and_then(Value::as_u64) != Some(id) {
                continue;
            }
            if message.get("error").is_some() {
                // The external error may include workspace text or logs; keep
                // it out of the renderer and receipt's diagnostic field.
                return Err(protocol_error(
                    method,
                    "Codex rejected the readback request",
                ));
            }
            return message
                .get("result")
                .cloned()
                .ok_or_else(|| protocol_error(method, "Missing result"));
        }
    }
}

impl Drop for CodexReadClient {
    fn drop(&mut self) {
        // npm's entrypoint may be a Node wrapper with a native child. Killing
        // only the wrapper would leak the app-server and its output reader.
        #[cfg(target_os = "macos")]
        unsafe {
            libc::kill(-(self.child.id() as i32), libc::SIGKILL);
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Produces a native projection from already validated, final-only messages.
/// The writer allocates the ID locally, then records it before publication.
pub(super) fn encode_rollout(
    messages: &[MigratableMessage],
    native_id: &str,
    workspace: &Path,
    timestamp: &str,
) -> Result<Vec<u8>, MigrationError> {
    uuid::Uuid::parse_str(native_id).map_err(|_| MigrationError::IdentityFieldInvalid {
        field: "targetNativeId".into(),
        reason: "A locally allocated UUID is required".into(),
    })?;
    if messages.is_empty() || messages[0].kind != MessageKind::UserText {
        return Err(MigrationError::PackageMalformed {
            reason: "Codex history must begin with a user message".into(),
        });
    }
    // The verified Codex reader omits empty assistant events and strips
    // whitespace-only user events from visible content. Refuse these shapes
    // before any native write; inventing a placeholder would alter the text.
    if messages.iter().any(|message| match message.kind {
        MessageKind::UserText => message.text.trim().is_empty(),
        MessageKind::AssistantFinal => message.text.is_empty(),
    }) {
        return Err(protocol_error(
            "encode",
            "Codex 0.154.0 cannot preserve an empty or whitespace-only message in visible history",
        ));
    }
    let mut records = Vec::new();
    append(
        &mut records,
        timestamp,
        "session_meta",
        json!({
            "session_id": native_id,
            "id": native_id,
            "timestamp": timestamp,
            "cwd": workspace,
            "originator": "fyagent",
            "cli_version": VERIFIED_VERSION,
            "source": "cli",
            "history_mode": "legacy"
        }),
    );
    let mut current_turn: Option<String> = None;
    let mut last_final: Option<&str> = None;
    for message in messages {
        let role = match message.kind {
            MessageKind::UserText => {
                if let Some(turn) = current_turn.take() {
                    finish_turn(&mut records, timestamp, &turn, last_final);
                }
                let turn = format!("fyagent-import-{}", message.seq);
                append(
                    &mut records,
                    timestamp,
                    "event_msg",
                    json!({"type":"task_started", "turn_id":turn,
                        "model_context_window":null, "collaboration_mode_kind":"default"}),
                );
                append(
                    &mut records,
                    timestamp,
                    "event_msg",
                    json!({"type":"user_message", "message":message.text,
                        "images":[], "local_images":[]}),
                );
                current_turn = Some(turn);
                last_final = None;
                "user"
            }
            MessageKind::AssistantFinal => {
                append(
                    &mut records,
                    timestamp,
                    "event_msg",
                    json!({"type":"agent_message", "message":message.text,
                        "phase":"final_answer"}),
                );
                last_final = Some(&message.text);
                "assistant"
            }
        };
        let (text_kind, provenance) = match message.kind {
            MessageKind::UserText => ("input_text", "user.text"),
            MessageKind::AssistantFinal => ("output_text", "assistant.final_answer"),
        };
        let mut payload = json!({
            "type":"message", "role":role,
            "content":[{"type":text_kind,"text":message.text}],
            "internal_chat_message_metadata_passthrough":{
                "turn_id":current_turn,
                "content_item_kinds":[provenance]
            }
        });
        if message.kind == MessageKind::AssistantFinal {
            payload["phase"] = json!("final_answer");
        }
        append(&mut records, timestamp, "response_item", payload);
    }
    if let Some(turn) = current_turn {
        finish_turn(&mut records, timestamp, &turn, last_final);
    }
    let mut bytes = Vec::new();
    for record in records {
        serde_json::to_writer(&mut bytes, &record).map_err(|error| {
            MigrationError::PackageMalformed {
                reason: error.to_string(),
            }
        })?;
        bytes.push(b'\n');
    }
    Ok(bytes)
}

fn append(records: &mut Vec<Value>, timestamp: &str, kind: &str, payload: Value) {
    records.push(json!({"timestamp":timestamp,"type":kind,"payload":payload}));
}

fn finish_turn(records: &mut Vec<Value>, timestamp: &str, turn: &str, final_text: Option<&str>) {
    let event = match final_text {
        Some(text) => json!({"type":"task_complete", "turn_id":turn, "last_agent_message":text}),
        None => json!({"type":"turn_aborted", "turn_id":turn, "reason":"interrupted"}),
    };
    append(records, timestamp, "event_msg", event);
}

/// Decode only the official app-server's visible user/final-answer items.
/// The caller compares the resulting ordered text digest with its local intent.
pub(super) fn decode_visible_history(
    thread: &Value,
) -> Result<Vec<MigratableMessage>, MigrationError> {
    let fail = || MigrationError::NativeProtocolFailed {
        provider_id: "codex".into(),
        method: "thread/read".into(),
        reason: "Unrecognized or incomplete visible history".into(),
    };
    let turns = thread
        .get("turns")
        .and_then(Value::as_array)
        .ok_or_else(fail)?;
    let mut messages = Vec::new();
    for turn in turns {
        let items = turn
            .get("items")
            .and_then(Value::as_array)
            .ok_or_else(fail)?;
        for item in items {
            let (kind, text) = match item.get("type").and_then(Value::as_str) {
                Some("userMessage") => {
                    let content = item
                        .get("content")
                        .and_then(Value::as_array)
                        .ok_or_else(fail)?;
                    // Our projection emits one exact text block. Reject an
                    // altered native transcript instead of silently concatenating.
                    if content.len() != 1
                        || content[0].get("type").and_then(Value::as_str) != Some("text")
                    {
                        return Err(fail());
                    }
                    (
                        MessageKind::UserText,
                        content[0]
                            .get("text")
                            .and_then(Value::as_str)
                            .ok_or_else(fail)?,
                    )
                }
                Some("agentMessage")
                    if item.get("phase").and_then(Value::as_str) == Some("final_answer") =>
                {
                    (
                        MessageKind::AssistantFinal,
                        item.get("text").and_then(Value::as_str).ok_or_else(fail)?,
                    )
                }
                _ => return Err(fail()),
            };
            messages.push(MigratableMessage {
                seq: messages.len() as u32,
                kind,
                text: text.to_owned(),
                ts: None,
            });
        }
    }
    if messages.is_empty() {
        return Err(fail());
    }
    Ok(messages)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn message(seq: u32, kind: MessageKind, text: &str) -> MigratableMessage {
        MigratableMessage {
            seq,
            kind,
            text: text.into(),
            ts: None,
        }
    }

    #[test]
    fn existing_rollouts_block_republication_across_dates_and_archives() {
        let home = tempfile::tempdir().unwrap();
        let id = "d48fbc0c-029c-4143-8794-c26891c02bf3";
        assert!(!rollout_exists(home.path(), id).unwrap());
        for root in ["sessions/2001/01/01", "archived_sessions/2000/12/31"] {
            let dir = home.path().join(root);
            std::fs::create_dir_all(&dir).unwrap();
            let file = dir.join(format!("rollout-2001-01-01T00-00-00-{id}.jsonl"));
            std::fs::write(&file, "existing body").unwrap();
            assert!(rollout_exists(home.path(), id).unwrap());
            assert_eq!(std::fs::read_to_string(&file).unwrap(), "existing body");
            std::fs::remove_file(file).unwrap();
        }
    }

    #[test]
    fn native_projection_preserves_text_and_missing_answers() {
        let messages = vec![
            message(0, MessageKind::UserText, "unfinished\r\nD:\\work\\ 🪁"),
            message(1, MessageKind::UserText, "next question"),
            message(2, MessageKind::AssistantFinal, "  final\r\n"),
        ];
        let encoded = encode_rollout(
            &messages,
            "d48fbc0c-029c-4143-8794-c26891c02bf3",
            Path::new("/target"),
            "2026-09-22T00:00:00Z",
        )
        .unwrap();
        let rows: Vec<Value> = String::from_utf8(encoded)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        let model_items: Vec<_> = rows
            .iter()
            .filter(|row| row["type"] == "response_item")
            .collect();
        assert_eq!(model_items.len(), 3);
        for (item, message) in model_items.iter().zip(messages.iter()) {
            assert_eq!(item["payload"]["content"][0]["text"], message.text);
        }
        assert!(rows
            .iter()
            .any(|row| row["payload"]["type"] == "turn_aborted"));
        assert_eq!(
            rows.iter()
                .filter(|row| row["payload"]["type"] == "task_complete")
                .count(),
            1
        );
        assert!(rows[0]["payload"].get("model_provider").is_none());
        assert!(rows[0]["payload"].get("base_instructions").is_none());
    }

    #[test]
    fn rejects_native_reader_loss_before_writing() {
        for messages in [
            vec![message(0, MessageKind::UserText, "   ")],
            vec![
                message(0, MessageKind::UserText, "Q"),
                message(1, MessageKind::AssistantFinal, ""),
            ],
        ] {
            assert!(encode_rollout(
                &messages,
                "d48fbc0c-029c-4143-8794-c26891c02bf3",
                Path::new("/target"),
                "2026-09-22T00:00:00Z"
            )
            .is_err());
        }
        assert!(encode_rollout(
            &[
                message(0, MessageKind::UserText, "Q"),
                message(1, MessageKind::AssistantFinal, " \r\n")
            ],
            "d48fbc0c-029c-4143-8794-c26891c02bf3",
            Path::new("/target"),
            "2026-09-22T00:00:00Z"
        )
        .is_ok());
    }

    #[test]
    fn visible_history_rejects_empty_or_unexpected_content() {
        assert!(decode_visible_history(&json!({"turns":[]})).is_err());
        assert!(decode_visible_history(
            &json!({"turns":[{"items":[{"type":"commandExecution"}]}]})
        )
        .is_err());
        let value = json!({"turns":[{"items":[
            {"type":"userMessage","content":[{"type":"text","text":"Q"}]},
            {"type":"agentMessage","phase":"final_answer","text":"A"}
        ]}]});
        assert_eq!(
            decode_visible_history(&value).unwrap(),
            vec![
                message(0, MessageKind::UserText, "Q"),
                message(1, MessageKind::AssistantFinal, "A")
            ]
        );
    }

    #[test]
    fn native_projection_reexports_without_unknown_provenance() {
        use crate::session_manager::migrate::extract::rules::codex::CodexRolloutPhaseV1;
        use crate::session_manager::migrate::extract::{
            build_session, ExtractionRule, RawRecord, RawSource,
        };
        let messages = vec![
            message(0, MessageKind::UserText, "unanswered"),
            message(1, MessageKind::UserText, "  原文\r\nD:\\work\\ 🪁  "),
            message(2, MessageKind::AssistantFinal, "first final"),
            message(3, MessageKind::AssistantFinal, "second final"),
        ];
        let encoded = encode_rollout(
            &messages,
            "d48fbc0c-029c-4143-8794-c26891c02bf3",
            Path::new("/target"),
            "2026-09-22T00:00:00Z",
        )
        .unwrap();
        let records = String::from_utf8(encoded)
            .unwrap()
            .lines()
            .map(|line| RawRecord {
                value: serde_json::from_str(line).unwrap(),
                ts: None,
            })
            .collect();
        let source = RawSource {
            records,
            session_id: Some("native-id".into()),
            workspace_path: Some("/target".into()),
            title: None,
            created_at: None,
            last_active_at: None,
            store_key: "isolated".into(),
            store_fingerprint: Some("isolated".into()),
            source_cli_version: Some("0.154.0".into()),
            canonical_store_path: None,
        };
        let state = tempfile::tempdir().unwrap();
        let session =
            crate::session_manager::migrate::identity::with_local_state_dir(state.path(), || {
                build_session(&CodexRolloutPhaseV1, &source, "isolated-source")
            })
            .unwrap();
        assert_eq!(session.messages, messages);
        assert_eq!(session.extraction.open_user_messages, vec![0]);
        assert_eq!(session.extraction.rule_id, CodexRolloutPhaseV1.rule_id());
    }
}
