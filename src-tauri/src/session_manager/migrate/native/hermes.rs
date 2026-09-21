//! Hermes 0.20.5: controlled installed SDK write, official CLI export readback.
//! Foreign-session import is deliberately avoided: it trims and merges text.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use serde_json::{json, Value};

use super::{
    provider_cli_path, run_with_output_limit, run_with_timeout, NativeRestoreInput,
    NativeSessionWriter, NativeWriteContext, NativeWriteOutcome, NonceCarrier, ReadbackVerdict,
    RunOutcome, MAX_NATIVE_READBACK_OUTPUT,
};
use crate::session_manager::migrate::identity;
use crate::session_manager::migrate::model::{
    MessageKind, MigratableMessage, MigratableSession, MigrationError, MigrationResult,
};

const PROVIDER_ID: &str = "hermes";
pub const WRITE_STRATEGY: &str = "hermes.sessiondb.v0205-v1";
pub const VERIFIED_VERSIONS: &[&str] = &["=0.20.5"];
const TIMEOUT: Duration = Duration::from_secs(90);

// Fixed program, never assembled from message text. The first operation uses
// the exact native resume decoder without opening a database. Only an exact
// native display/model projection permits the later write operation.
const SDK_BRIDGE: &str = r#"
import json,sys
from pathlib import Path
sys.path.insert(0,sys.argv[1])
from hermes_cli import __version__
if __version__ != '0.20.5':
    print(json.dumps({'protocol':1,'status':'versionMismatch'}));sys.exit(0)
from hermes_state import SessionDB
data=json.loads(Path(sys.argv[2]).read_text(encoding='utf-8'))
def result(status,**fields):
    print(json.dumps({'protocol':1,'status':status,**fields},ensure_ascii=False))
if data['op']=='probe':
    result('ready');sys.exit(0)
messages=data['messages']
expected=[(m['role'],m['text']) for m in messages]
rows=[]
columns=[c.strip() for c in SessionDB._CONVERSATION_ROW_COLUMNS.split(',')]
for index,m in enumerate(messages):
    row={name:None for name in columns}
    row.update(id=index+1,role=m['role'],content=SessionDB._encode_content(m['text']))
    if m['role']=='assistant':row['finish_reason']='stop'
    rows.append(row)
decoder=object.__new__(SessionDB)
for display in (False,True):
    decoded=decoder._rows_to_conversation(rows,session_id=data['id'],include_ancestors=display,
        repair_alternation=not display,include_row_ids=False)
    if [(m['role'],m['content']) for m in decoded] != expected:
        result('bodyNotPreserved');sys.exit(0)
# The CLI appends the next user prompt before its request-time alternation repair.
# A trailing unanswered user would be merged with that new prompt, changing a
# historical message boundary. Verify that future step before creating anything.
next_row={name:None for name in columns}
next_row.update(id=len(rows)+1,role='user',content=SessionDB._encode_content('fyagent-next-turn-preflight'))
continued=decoder._rows_to_conversation(rows+[next_row],session_id=data['id'],include_ancestors=False,
    repair_alternation=True,include_row_ids=False)
if [(m['role'],m['content']) for m in continued] != expected+[('user','fyagent-next-turn-preflight')]:
    result('bodyNotPreserved');sys.exit(0)
if data['op']=='preflight':
    result('ready');sys.exit(0)
if data['op']!='write':raise ValueError('unknown operation')
db=SessionDB(db_path=Path(data['db_path']))
try:
    if db.get_session(data['id']) is not None:
        result('alreadyExists');sys.exit(0)
    db.create_session(data['id'],source='cli',cwd=data['workspace'])
    for m in messages:
        db.append_message(data['id'],m['role'],m['text'],
            finish_reason='stop' if m['role']=='assistant' else None)
    if data.get('title'):db.set_session_title(data['id'],data['title'])
    result('written',id=data['id'])
finally:
    db.close()
"#;

pub struct HermesWriter;

impl NativeSessionWriter for HermesWriter {
    fn provider_id(&self) -> &'static str {
        PROVIDER_ID
    }
    fn write_strategy(&self) -> &'static str {
        WRITE_STRATEGY
    }
    fn verified_write_versions(&self) -> &'static [&'static str] {
        VERIFIED_VERSIONS
    }
    fn resolve_target_store_id(&self) -> MigrationResult<String> {
        identity::store_instance_id(PROVIDER_ID, &db_path())
    }

    fn restore(
        &self,
        input: &NativeRestoreInput,
        context: &dyn NativeWriteContext,
    ) -> NativeWriteOutcome {
        let prepared = (|| {
            let runtime = Runtime::discover()?;
            if self.resolve_target_store_id()? != input.target_store_id {
                return Err(protocol(
                    "target store",
                    "The target store changed; reopen the restore dialog.",
                ));
            }
            let id = match &input.target_native_id {
                Some(id) if valid_native_id(id) => id.clone(),
                Some(_) => {
                    return Err(protocol(
                        "native ID",
                        "Invalid preallocated native session ID.",
                    ))
                }
                None => format!("fyagent_{}", uuid::Uuid::new_v4().simple()),
            };
            let mut request = write_request(input, &id);
            request["op"] = json!("preflight");
            let preflight = runtime.call(&request, &input.target_workspace)?;
            match preflight.get("status").and_then(Value::as_str) {
                Some("ready")=>{},
                Some("bodyNotPreserved")=>return Err(protocol("native fidelity", "This Hermes version would change message text or merge turns when continuing; no session was written.")),
                Some("versionMismatch")=>return Err(unsupported_version()),
                _=>return Err(protocol("native preflight", "The installed Hermes SDK could not confirm exact native replay.")),
            }
            request["op"] = json!("write");
            Ok((runtime, id, request))
        })();
        let (runtime, id, request) = match prepared {
            Ok(value) => value,
            Err(error) => return NativeWriteOutcome::ProvenNoSideEffect { error },
        };
        if let Err(error) = context.record_native_id(&id) {
            return NativeWriteOutcome::ProvenNoSideEffect { error };
        }
        match runtime.call(&request, &input.target_workspace) {
            Ok(response)
                if response.get("status").and_then(Value::as_str) == Some("written")
                    && response.get("id").and_then(Value::as_str) == Some(id.as_str()) =>
            {
                NativeWriteOutcome::Written {
                    target_native_id: Some(id),
                    nonce_carrier: NonceCarrier::NotUsed,
                }
            }
            Ok(response)
                if response.get("status").and_then(Value::as_str) == Some("bodyNotPreserved") =>
            {
                NativeWriteOutcome::ProvenNoSideEffect {
                    error: protocol(
                        "native fidelity",
                        "The native replay preflight changed; nothing was written.",
                    ),
                }
            }
            // An existing ID is evidence to reconcile, never permission to append.
            _ => NativeWriteOutcome::Unresolved {
                error: protocol(
                    "native write",
                    "The native writer did not confirm a new session; reconcile before retrying.",
                ),
            },
        }
    }

    fn verify_readback(
        &self,
        native_id: &str,
        expected: &MigratableSession,
    ) -> MigrationResult<ReadbackVerdict> {
        if !valid_native_id(native_id) {
            return Err(protocol("sessions export", "Invalid native session ID."));
        }
        let runtime = Runtime::discover()?;
        let probe = runtime.call(
            &json!({"op":"probe"}),
            &crate::hermes_config::get_hermes_dir(),
        )?;
        if probe.get("status").and_then(Value::as_str) != Some("ready") {
            return Err(unsupported_version());
        }
        let mut command = Command::new(&runtime.cli);
        command
            .args([
                "sessions",
                "export",
                "-",
                "--session-id",
                native_id,
                "--format",
                "jsonl",
            ])
            .env("HERMES_HOME", crate::hermes_config::get_hermes_dir())
            .env("PYTHONDONTWRITEBYTECODE", "1");
        match run_with_output_limit(command, TIMEOUT, MAX_NATIVE_READBACK_OUTPUT) {
            RunOutcome::Exited {
                status: Some(0),
                stdout,
                ..
            } => {
                let Ok(exported) = serde_json::from_str::<Value>(&stdout) else {
                    return Ok(ReadbackVerdict::Blocked {
                        reason: "Hermes did not return a single native session JSON object.".into(),
                    });
                };
                Ok(compare_export(&exported, native_id, expected))
            }
            _ => Ok(ReadbackVerdict::Blocked {
                reason: "Hermes native export is unavailable; session absence is not proven."
                    .into(),
            }),
        }
    }
    fn locate_by_nonce(&self, _nonce: &str) -> MigrationResult<Vec<String>> {
        Ok(Vec::new())
    }
}

fn db_path() -> PathBuf {
    crate::hermes_config::get_hermes_dir().join("state.db")
}
fn protocol(method: &str, reason: &str) -> MigrationError {
    MigrationError::NativeProtocolFailed {
        provider_id: PROVIDER_ID.into(),
        method: method.into(),
        reason: reason.into(),
    }
}
fn unsupported_version() -> MigrationError {
    MigrationError::ProviderVersionUnsupported {
        provider_id: PROVIDER_ID.into(),
        detected: None,
        verified: VERIFIED_VERSIONS.iter().map(|s| (*s).into()).collect(),
    }
}
fn valid_native_id(id: &str) -> bool {
    id.strip_prefix("fyagent_")
        .is_some_and(|body| body.len() == 32 && body.bytes().all(|b| b.is_ascii_hexdigit()))
}

struct Runtime {
    cli: PathBuf,
    python: PathBuf,
    source: PathBuf,
}
impl Runtime {
    fn discover() -> MigrationResult<Self> {
        if super::user_cli_execution_blocked().is_some() {
            return Err(protocol(
                "native process",
                "Hermes restoration requires the ordinary-user Windows execution helper.",
            ));
        }
        let cli =
            provider_cli_path(PROVIDER_ID).ok_or_else(|| MigrationError::ProviderNotInstalled {
                provider_id: PROVIDER_ID.into(),
            })?;
        let real = cli.canonicalize().map_err(|_| {
            protocol(
                "SDK discovery",
                "Unable to resolve the installed Hermes launcher.",
            )
        })?;
        let bin = real.parent().ok_or_else(|| {
            protocol(
                "SDK discovery",
                "Hermes installation has no runtime directory.",
            )
        })?;
        let python = ["python", "python3", "python.exe"]
            .iter()
            .map(|name| bin.join(name))
            .find(|p| p.is_file())
            .ok_or_else(|| {
                protocol(
                    "SDK discovery",
                    "The selected Hermes installation has no adjacent Python runtime.",
                )
            })?;
        let source = bin
            .ancestors()
            .take(4)
            .find(|p| {
                p.join("hermes_state.py").is_file() && p.join("hermes_cli/__init__.py").is_file()
            })
            .map(Path::to_path_buf)
            .ok_or_else(|| {
                protocol(
                    "SDK discovery",
                    "This Hermes installation layout has not been verified.",
                )
            })?;
        Ok(Self {
            cli,
            python,
            source,
        })
    }

    fn call(&self, request: &Value, cwd: &Path) -> MigrationResult<Value> {
        let file = tempfile::Builder::new()
            .prefix("fyagent-hermes-")
            .suffix(".json")
            .tempfile()
            .map_err(|_| protocol("SDK request", "Unable to create a private request file."))?;
        std::fs::write(file.path(), request.to_string())
            .map_err(|_| protocol("SDK request", "Unable to write the private request file."))?;
        let mut command = Command::new(&self.python);
        command
            .args(["-I", "-B", "-c", SDK_BRIDGE])
            .arg(&self.source)
            .arg(file.path())
            .env("HERMES_HOME", crate::hermes_config::get_hermes_dir())
            .current_dir(cwd);
        match run_with_timeout(command, TIMEOUT) {
            RunOutcome::Exited {
                status: Some(0),
                stdout,
                ..
            } => {
                let response: Value = serde_json::from_str(&stdout).map_err(|_| {
                    protocol(
                        "SDK response",
                        "The installed SDK returned an invalid response.",
                    )
                })?;
                if response.get("protocol").and_then(Value::as_u64) != Some(1) {
                    return Err(protocol(
                        "SDK response",
                        "Unexpected native bridge protocol.",
                    ));
                }
                Ok(response)
            }
            _ => Err(protocol(
                "SDK response",
                "The installed Hermes SDK did not confirm completion.",
            )),
        }
    }
}

fn write_request(input: &NativeRestoreInput, native_id: &str) -> Value {
    let messages:Vec<Value>=input.session.messages.iter().map(|m|json!({
        "role":match m.kind { MessageKind::UserText=>"user",MessageKind::AssistantFinal=>"assistant" },
        "text":m.text,
    })).collect();
    json!({"op":"write","id":native_id,"db_path":db_path(),"workspace":input.target_workspace,
        "title":input.session.title,"messages":messages})
}

fn compare_export(
    exported: &Value,
    native_id: &str,
    expected: &MigratableSession,
) -> ReadbackVerdict {
    let blocked = || ReadbackVerdict::Blocked {
        reason: "The native export is not a matching final-text-only session.".into(),
    };
    if exported.get("id").and_then(Value::as_str) != Some(native_id) {
        return blocked();
    }
    let Some(messages) = exported.get("messages").and_then(Value::as_array) else {
        return blocked();
    };
    let mut projected = Vec::new();
    for message in messages {
        let no_tools = message
            .get("tool_calls")
            .is_none_or(|v| v.is_null() || v.as_array().is_some_and(Vec::is_empty));
        let no_api_content = message.get("api_content").is_none_or(Value::is_null);
        let no_display_override = message
            .get("display_kind")
            .is_none_or(|v| v.is_null() || v.as_str() == Some(""));
        if !no_tools || !no_api_content || !no_display_override {
            return blocked();
        }
        let kind = match message.get("role").and_then(Value::as_str) {
            Some("user") => MessageKind::UserText,
            Some("assistant")
                if message.get("finish_reason").and_then(Value::as_str) == Some("stop") =>
            {
                MessageKind::AssistantFinal
            }
            _ => return blocked(),
        };
        let Some(text) = message.get("content").and_then(Value::as_str) else {
            return blocked();
        };
        projected.push(MigratableMessage {
            seq: projected.len() as u32,
            kind,
            text: text.into(),
            ts: None,
        });
    }
    let observed = identity::content_digest(&projected);
    if observed == expected.content_digest {
        ReadbackVerdict::Visible {
            observed_digest: observed,
        }
    } else {
        ReadbackVerdict::Mismatch { observed }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_preallocated_hermes_ids_are_accepted() {
        assert!(valid_native_id("fyagent_00112233445566778899aabbccddeeff"));
        for id in ["--help", "../x", "fyagent_", "20260922_120000_abcdef"] {
            assert!(!valid_native_id(id));
        }
    }
    #[test]
    fn readback_requires_exact_id_final_proof_and_body_digest() {
        use crate::session_manager::migrate::model::{
            ExtractionReport, OmittedCounts, OriginIdentity, PathFamily,
        };
        let messages = vec![
            MigratableMessage {
                seq: 0,
                kind: MessageKind::UserText,
                text: "问题\r\nbody".into(),
                ts: None,
            },
            MigratableMessage {
                seq: 1,
                kind: MessageKind::AssistantFinal,
                text: "答案".into(),
                ts: None,
            },
        ];
        let expected = MigratableSession {
            snapshot_id: String::new(),
            content_digest: identity::content_digest(&messages),
            origin: OriginIdentity {
                origin_id: String::new(),
                provider_id: PROVIDER_ID.into(),
                session_id: None,
                cli_version: None,
                store_fingerprint: None,
            },
            title: None,
            created_at: None,
            last_active_at: None,
            workspace_label: None,
            messages: vec![],
            extraction: ExtractionReport {
                rule_id: "test".into(),
                rule_verified_versions: vec![],
                open_user_messages: vec![],
                omitted: OmittedCounts::default(),
                source_path_family: PathFamily::Posix,
            },
        };
        let mut exported = json!({"id":"test-id","messages":[{"role":"user","content":"问题\r\nbody"},
            {"role":"assistant","content":"答案","finish_reason":"stop"}]});
        assert!(matches!(
            compare_export(&exported, "test-id", &expected),
            ReadbackVerdict::Visible { .. }
        ));
        assert!(matches!(
            compare_export(&exported, "prefix", &expected),
            ReadbackVerdict::Blocked { .. }
        ));
        exported["messages"][1]["finish_reason"] = Value::Null;
        assert!(matches!(
            compare_export(&exported, "test-id", &expected),
            ReadbackVerdict::Blocked { .. }
        ));
        exported["messages"][1]["finish_reason"] = json!("stop");
        exported["messages"][0]["content"] = json!("问题\nbody");
        assert!(matches!(
            compare_export(&exported, "test-id", &expected),
            ReadbackVerdict::Mismatch { .. }
        ));
    }
    fn expectation(messages: Vec<MigratableMessage>) -> MigratableSession {
        use crate::session_manager::migrate::model::{
            ExtractionReport, OmittedCounts, OriginIdentity, PathFamily,
        };
        MigratableSession {
            snapshot_id: String::new(),
            content_digest: identity::content_digest(&messages),
            origin: OriginIdentity {
                origin_id: String::new(),
                provider_id: PROVIDER_ID.into(),
                session_id: None,
                cli_version: None,
                store_fingerprint: None,
            },
            title: None,
            created_at: None,
            last_active_at: None,
            workspace_label: None,
            messages,
            extraction: ExtractionReport {
                rule_id: "test".into(),
                rule_verified_versions: vec![],
                open_user_messages: vec![],
                omitted: OmittedCounts::default(),
                source_path_family: PathFamily::Posix,
            },
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    #[serial_test::serial]
    fn restore_preflight_store_and_receipt_order_guard_the_fixed_bridge() {
        use std::cell::Cell;
        use std::ffi::OsString;
        use std::os::unix::fs::PermissionsExt;
        struct EnvGuard(Vec<(&'static str, Option<OsString>)>);
        impl Drop for EnvGuard {
            fn drop(&mut self) {
                for (key, previous) in &self.0 {
                    match previous {
                        Some(value) => std::env::set_var(key, value),
                        None => std::env::remove_var(key),
                    }
                }
            }
        }
        let root = tempfile::tempdir().unwrap();
        let bin = root.path().join(".local/bin");
        std::fs::create_dir_all(&bin).unwrap();
        let _environment = EnvGuard(
            ["FYAGENT_TEST_HOME", "HERMES_HOME", "PATH"]
                .into_iter()
                .map(|key| (key, std::env::var_os(key)))
                .collect(),
        );
        std::env::set_var("FYAGENT_TEST_HOME", root.path());
        std::env::set_var("HERMES_HOME", root.path().join(".hermes"));
        std::env::set_var("PATH", &bin);
        std::fs::create_dir_all(root.path().join(".hermes")).unwrap();
        std::fs::write(root.path().join(".hermes/state.db"), "synthetic store").unwrap();
        let launcher = bin.join("hermes");
        std::fs::write(&launcher, "#!/bin/sh\nexit 1\n").unwrap();
        std::fs::set_permissions(&launcher, std::fs::Permissions::from_mode(0o700)).unwrap();
        std::fs::write(root.path().join("hermes_state.py"), "").unwrap();
        std::fs::create_dir_all(root.path().join("hermes_cli")).unwrap();
        std::fs::write(root.path().join("hermes_cli/__init__.py"), "").unwrap();
        let receipt = root.path().join("receipt");
        let publication = root.path().join("publication.json");
        let version_flag = root.path().join("wrong-version");
        let runtime = bin.join("python");
        std::fs::write(
            &runtime,
            format!(
                r##"#!/usr/bin/python3
import json,sys
from pathlib import Path
request=json.loads(Path(sys.argv[-1]).read_text())
receipt=Path({receipt})
publication=Path({publication})
response={{'protocol':1,'status':'ready'}}
if Path({version_flag}).exists():response['status']='versionMismatch'
elif request['op']=='write':
    assert receipt.read_text()==request['id']
    if publication.exists():response['status']='alreadyExists'
    else:
        with publication.open('x') as output:json.dump(request,output)
        response.update(status='written',id=request['id'])
print(json.dumps(response))
"##,
                receipt = json!(receipt),
                publication = json!(publication),
                version_flag = json!(version_flag)
            ),
        )
        .unwrap();
        std::fs::set_permissions(&runtime, std::fs::Permissions::from_mode(0o700)).unwrap();
        struct Context {
            path: PathBuf,
            fail: bool,
            count: Cell<usize>,
        }
        impl NativeWriteContext for Context {
            fn record_native_id(&self, id: &str) -> MigrationResult<()> {
                self.count.set(self.count.get() + 1);
                if self.fail {
                    return Err(protocol("test receipt", "injected receipt failure"));
                }
                std::fs::write(&self.path, id).unwrap();
                Ok(())
            }
        }
        let id = "fyagent_00112233445566778899aabbccddeeff";
        let mut input = NativeRestoreInput {
            session: expectation(vec![
                MigratableMessage {
                    seq: 0,
                    kind: MessageKind::UserText,
                    text: "Q\r\n中文".into(),
                    ts: None,
                },
                MigratableMessage {
                    seq: 1,
                    kind: MessageKind::AssistantFinal,
                    text: "answer".into(),
                    ts: None,
                },
            ]),
            target_workspace: root.path().to_path_buf(),
            target_store_id: HermesWriter.resolve_target_store_id().unwrap(),
            target_native_nonce: String::new(),
            target_native_id: Some(id.into()),
        };
        let context = Context {
            path: receipt.clone(),
            fail: false,
            count: Cell::new(0),
        };
        std::fs::write(&version_flag, "").unwrap();
        assert!(matches!(
            HermesWriter.restore(&input, &context),
            NativeWriteOutcome::ProvenNoSideEffect {
                error: MigrationError::ProviderVersionUnsupported { .. }
            }
        ));
        assert_eq!(context.count.get(), 0);
        assert!(!publication.exists());
        std::fs::remove_file(version_flag).unwrap();
        let store = input.target_store_id.clone();
        input.target_store_id = "different-store".into();
        assert!(matches!(
            HermesWriter.restore(&input, &context),
            NativeWriteOutcome::ProvenNoSideEffect { .. }
        ));
        assert_eq!(context.count.get(), 0);
        input.target_store_id = store;
        let failed_context = Context {
            path: receipt.clone(),
            fail: true,
            count: Cell::new(0),
        };
        assert!(matches!(
            HermesWriter.restore(&input, &failed_context),
            NativeWriteOutcome::ProvenNoSideEffect { .. }
        ));
        assert!(!publication.exists());
        assert!(
            matches!(HermesWriter.restore(&input,&context),NativeWriteOutcome::Written{target_native_id:Some(ref observed),..} if observed==id)
        );
        let published: Value =
            serde_json::from_slice(&std::fs::read(&publication).unwrap()).unwrap();
        assert_eq!(published["id"], id);
        assert_eq!(published["messages"][0]["text"], "Q\r\n中文");
        assert_eq!(context.count.get(), 1);
        assert!(matches!(
            HermesWriter.restore(&input, &context),
            NativeWriteOutcome::Unresolved { .. }
        ));
        let still_published: Value =
            serde_json::from_slice(&std::fs::read(&publication).unwrap()).unwrap();
        assert_eq!(published, still_published);
    }
}
