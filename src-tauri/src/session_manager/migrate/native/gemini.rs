//! Gemini 0.46.0 native JSONL publication, verified through the installed loader.
//! This restores an already validated package; it does not infer finals from raw
//! Gemini history, whose recorder does not persist a reliable completion signal.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use serde_json::{json, Value};

use super::{
    provider_cli_path, resolve_cli, run_with_output_limit, NativeRestoreInput, NativeSessionWriter,
    NativeWriteContext, NativeWriteOutcome, NonceCarrier, ReadbackVerdict, RunOutcome,
    MAX_NATIVE_READBACK_OUTPUT,
};
use crate::session_manager::migrate::identity;
use crate::session_manager::migrate::model::{
    MessageKind, MigratableMessage, MigratableSession, MigrationError, MigrationResult,
};

const PROVIDER_ID: &str = "gemini";
pub const WRITE_STRATEGY: &str = "gemini.native-jsonl.v046-v1";
pub const VERIFIED_VERSIONS: &[&str] = &["=0.46.0"];
const TIMEOUT: Duration = Duration::from_secs(90);

// Discover modules only from the selected installation's active entry. No
// import of the user's settings, no CLI main(), no summary/model call.
const NATIVE_BRIDGE: &str = r#"
const fs=await import('node:fs/promises');
const path=await import('node:path');
const {pathToFileURL}=await import('node:url');
const [entryArg,inputArg]=process.argv.slice(1);
const input=JSON.parse(await fs.readFile(inputArg,'utf8'));
const result=(status,fields={})=>console.log(JSON.stringify({protocol:1,status,...fields}));
const entry=await fs.realpath(entryArg);
const bundle=path.dirname(entry);
const pkg=JSON.parse(await fs.readFile(path.join(bundle,'..','package.json'),'utf8'));
if(pkg.name!=='@google/gemini-cli'||pkg.version!=='0.46.0') {
 result('versionMismatch');process.exit(0);
}
const entryText=await fs.readFile(entry,'utf8');
const mainRef=entryText.match(/await import\("\.\/(gemini-[A-Z0-9]+\.js)"\)/)?.[1];
if(!mainRef)throw new Error('unsupported native entry');
const mainText=await fs.readFile(path.join(bundle,mainRef),'utf8');
const imports=[...mainText.matchAll(/import\s*\{([\s\S]*?)\}\s*from\s*"(\.\/chunk-[A-Z0-9]+\.js)"/g)];
const coreRef=imports.find(m=>m[1].split(',').some(s=>s.trim()==='loadConversationRecord'))?.[2];
if(!coreRef)throw new Error('native loader not exported');
const core=await import(pathToFileURL(path.join(bundle,coreRef)).href);
const {Storage,loadConversationRecord,convertSessionToClientHistory,getProjectHash}=core;
if([Storage,loadConversationRecord,convertSessionToClientHistory,getProjectHash].some(x=>typeof x!=='function'))throw new Error('native bridge unavailable');
if(input.op==='probe'){result('ready');process.exit(0);}
function projection(messages) {
 const clean=[];
 for(const message of messages) {
  if(message.type==='info'||message.type==='warning'||message.type==='error')continue;
  if(message.type!=='user'&&message.type!=='gemini')throw new Error('unknown message role');
  if(message.toolCalls?.length||message.thoughts?.length)throw new Error('non-text native history');
  const parts=typeof message.content==='string'?[{text:message.content}]:message.content;
  if(!Array.isArray(parts)||parts.length!==1||typeof parts[0]?.text!=='string'||Object.keys(parts[0]).some(k=>k!=='text'))throw new Error('non-text native content');
  clean.push({role:message.type==='user'?'user':'assistant',text:parts[0].text});
 }
 const native=convertSessionToClientHistory(messages).map(m=>({role:m.content.role==='user'?'user':'assistant',
  text:m.content.parts.length===1&&typeof m.content.parts[0].text==='string'?m.content.parts[0].text:null}));
 if(JSON.stringify(native)!==JSON.stringify(clean))throw new Error('native replay changes message text');
 return clean;
}
const root=path.join(process.env.GEMINI_CLI_HOME,'.gemini','tmp');
async function matchingSessions() {
 const candidates=[];let visited=0;
 async function walk(dir,depth) {
  if(depth>3)return;
  for(const item of await fs.readdir(dir,{withFileTypes:true})) {
   if(++visited>10000)throw new Error('native index too large');
   if(item.isSymbolicLink())continue;
   const full=path.join(dir,item.name);
   if(item.isDirectory())await walk(full,depth+1);
   else if(item.isFile()&&item.name.startsWith('session-')&&/\.jsonl?$/.test(item.name)&&item.name.includes(input.id.slice(0,8))) {
    const meta=await loadConversationRecord(full,{metadataOnly:true});
    if(meta?.sessionId===input.id)candidates.push(full);
   }
  }
 }
 await walk(root,0);return candidates;
}
if(input.op==='read') {
 const matches=await matchingSessions();
 if(matches.length===0){result('notVisible');process.exit(0);}
 if(matches.length!==1){result('ambiguous');process.exit(0);}
 const conversation=await loadConversationRecord(matches[0]);
 if(conversation?.sessionId!==input.id)throw new Error('native identity changed');
 try {result('visible',{id:input.id,messages:projection(conversation.messages)});}
 catch {result('bodyNotPreserved');}
 process.exit(0);
}
const stamp=new Date().toISOString();
const messages=input.messages.map((m,index)=>({id:`fyagent-${input.id}-${index}`,timestamp:stamp,
 type:m.role==='user'?'user':'gemini',content:[{text:m.text}]}));
try {
 if(JSON.stringify(projection(messages))!==JSON.stringify(input.messages))throw new Error('projection mismatch');
} catch {result('bodyNotPreserved');process.exit(0);}
if(input.op==='preflight'){result('ready');process.exit(0);}
if(input.op!=='write')throw new Error('unknown operation');
if((await matchingSessions()).length){result('alreadyExists');process.exit(0);}
const storage=new Storage(input.workspace);await storage.initialize();
const chats=path.join(storage.getProjectTempDir(),'chats');
await fs.mkdir(chats,{recursive:true});
const file=path.join(chats,`session-${Date.now()}-${input.id.slice(0,8)}.jsonl`);
const metadata={sessionId:input.id,projectHash:getProjectHash(storage.getProjectRoot()),
 startTime:stamp,lastUpdated:stamp,kind:'main',...(input.title?{summary:input.title}:{})};
const body=[metadata,...messages].map(x=>JSON.stringify(x)).join('\n')+'\n';
const handle=await fs.open(file,'wx',0o600);
try {await handle.writeFile(body,'utf8');await handle.sync();}finally{await handle.close();}
if(process.platform==='darwin'){const dir=await fs.open(chats,'r');try{await dir.sync();}finally{await dir.close();}}
const readback=await loadConversationRecord(file);
if(readback?.sessionId!==input.id||JSON.stringify(projection(readback.messages))!==JSON.stringify(input.messages))throw new Error('native readback mismatch');
result('written',{id:input.id});
"#;

pub struct GeminiWriter;
impl NativeSessionWriter for GeminiWriter {
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
        identity::store_instance_id(
            PROVIDER_ID,
            &crate::gemini_config::get_gemini_dir().join("tmp"),
        )
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
                    "The target Gemini store changed; reopen the restore dialog.",
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
                None => uuid::Uuid::new_v4().to_string(),
            };
            let messages:Vec<Value>=input.session.messages.iter().map(|m|json!({
                "role":match m.kind { MessageKind::UserText=>"user",MessageKind::AssistantFinal=>"assistant" },"text":m.text})).collect();
            let mut request = json!({"op":"preflight","id":id,"workspace":input.target_workspace,"title":input.session.title,"messages":messages});
            let response = runtime.call(&request, &input.target_workspace)?;
            match response.get("status").and_then(Value::as_str) {
                Some("ready") => {}
                Some("versionMismatch") => return Err(unsupported_version()),
                Some("bodyNotPreserved") => return Err(protocol(
                    "native fidelity",
                    "This Gemini version filters part of the user history; no session was written.",
                )),
                _ => {
                    return Err(protocol(
                        "native preflight",
                        "The installed native loader could not confirm exact replay.",
                    ))
                }
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
                        "Native replay would change the transcript; no session was written.",
                    ),
                }
            }
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
            return Err(protocol("native readback", "Invalid native session ID."));
        }
        let runtime = Runtime::discover()?;
        let response = runtime.call(&json!({"op":"read","id":native_id}), &runtime.home)?;
        match response.get("status").and_then(Value::as_str) {
            Some("visible") if response.get("id").and_then(Value::as_str) == Some(native_id) => {
                compare_projection(&response, expected)
            }
            Some("notVisible") => Ok(ReadbackVerdict::NotVisible),
            Some("versionMismatch") => Err(unsupported_version()),
            Some("ambiguous") => Ok(ReadbackVerdict::Blocked {
                reason: "Multiple native files claim this session ID; no match was selected."
                    .into(),
            }),
            _ => Ok(ReadbackVerdict::Blocked {
                reason: "The installed Gemini loader could not verify exact native history.".into(),
            }),
        }
    }
    fn locate_by_nonce(&self, _nonce: &str) -> MigrationResult<Vec<String>> {
        Ok(Vec::new())
    }
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
    uuid::Uuid::parse_str(id).is_ok_and(|value| value.to_string() == id)
}

struct Runtime {
    cli: PathBuf,
    node: PathBuf,
    home: PathBuf,
}
impl Runtime {
    fn discover() -> MigrationResult<Self> {
        if super::user_cli_execution_blocked().is_some() {
            return Err(protocol(
                "native process",
                "Gemini restoration requires the ordinary-user Windows execution helper.",
            ));
        }
        let cli =
            provider_cli_path(PROVIDER_ID).ok_or_else(|| MigrationError::ProviderNotInstalled {
                provider_id: PROVIDER_ID.into(),
            })?;
        let node = resolve_cli(PROVIDER_ID, "node", &[]).ok_or_else(|| {
            protocol(
                "native runtime",
                "The installed Gemini Node runtime is unavailable.",
            )
        })?;
        let directory = crate::gemini_config::get_gemini_dir();
        if directory.file_name().and_then(|s| s.to_str()) != Some(".gemini") {
            return Err(protocol("native target","This Gemini override directory cannot be bound to the verified native home layout."));
        }
        let home = directory
            .parent()
            .ok_or_else(|| protocol("native target", "The Gemini home directory is invalid."))?
            .to_path_buf();
        Ok(Self { cli, node, home })
    }
    fn call(&self, request: &Value, cwd: &Path) -> MigrationResult<Value> {
        let file = tempfile::Builder::new()
            .prefix("fyagent-gemini-")
            .suffix(".json")
            .tempfile()
            .map_err(|_| protocol("native request", "Unable to create a private request file."))?;
        std::fs::write(file.path(), request.to_string()).map_err(|_| {
            protocol(
                "native request",
                "Unable to write the private request file.",
            )
        })?;
        let mut command = Command::new(&self.node);
        command
            .args(["--input-type=module", "--eval", NATIVE_BRIDGE])
            .arg(&self.cli)
            .arg(file.path())
            .env("GEMINI_CLI_HOME", &self.home)
            .env("GEMINI_CLI_NO_RELAUNCH", "true")
            .current_dir(cwd);
        match run_with_output_limit(command, TIMEOUT, MAX_NATIVE_READBACK_OUTPUT) {
            RunOutcome::Exited {
                status: Some(0),
                stdout,
                ..
            } => {
                let value: Value = serde_json::from_str(&stdout).map_err(|_| {
                    protocol(
                        "native response",
                        "The installed Gemini bridge returned invalid JSON.",
                    )
                })?;
                if value.get("protocol").and_then(Value::as_u64) != Some(1) {
                    return Err(protocol(
                        "native response",
                        "Unexpected native bridge protocol.",
                    ));
                }
                Ok(value)
            }
            _ => Err(protocol(
                "native response",
                "The installed Gemini bridge did not confirm completion.",
            )),
        }
    }
}

fn compare_projection(
    response: &Value,
    expected: &MigratableSession,
) -> MigrationResult<ReadbackVerdict> {
    let Some(messages) = response.get("messages").and_then(Value::as_array) else {
        return Err(protocol("native readback", "Missing native history."));
    };
    let mut projected = Vec::new();
    for message in messages {
        let kind = match message.get("role").and_then(Value::as_str) {
            Some("user") => MessageKind::UserText,
            Some("assistant") => MessageKind::AssistantFinal,
            _ => return Err(protocol("native readback", "Unexpected native role.")),
        };
        let text = message
            .get("text")
            .and_then(Value::as_str)
            .ok_or_else(|| protocol("native readback", "Missing native text."))?;
        projected.push(MigratableMessage {
            seq: projected.len() as u32,
            kind,
            text: text.into(),
            ts: None,
        });
    }
    let observed = identity::content_digest(&projected);
    Ok(if observed == expected.content_digest {
        ReadbackVerdict::Visible {
            observed_digest: observed,
        }
    } else {
        ReadbackVerdict::Mismatch { observed }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_ids_are_canonical_uuids_not_paths_or_arguments() {
        assert!(valid_native_id("00112233-4455-4677-8899-aabbccddeeff"));
        for id in [
            "--help",
            "../00112233-4455-4677-8899-aabbccddeeff",
            "00112233445546778899aabbccddeeff",
        ] {
            assert!(!valid_native_id(id));
        }
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

    #[test]
    fn readback_digest_is_sensitive_to_order_role_and_verbatim_text() {
        let expected = expectation(vec![
            MigratableMessage {
                seq: 0,
                kind: MessageKind::UserText,
                text: "  Q\r\n".into(),
                ts: None,
            },
            MigratableMessage {
                seq: 1,
                kind: MessageKind::AssistantFinal,
                text: "A 中文".into(),
                ts: None,
            },
        ]);
        let mut value = json!({"messages":[{"role":"user","text":"  Q\r\n"},{"role":"assistant","text":"A 中文"}]});
        assert!(matches!(
            compare_projection(&value, &expected).unwrap(),
            ReadbackVerdict::Visible { .. }
        ));
        value["messages"][0]["text"] = json!("Q\n");
        assert!(matches!(
            compare_projection(&value, &expected).unwrap(),
            ReadbackVerdict::Mismatch { .. }
        ));
        value["messages"][0]["text"] = json!("  Q\r\n");
        value["messages"].as_array_mut().unwrap().swap(0, 1);
        assert!(matches!(
            compare_projection(&value, &expected).unwrap(),
            ReadbackVerdict::Mismatch { .. }
        ));
        value["messages"][0]["role"] = json!("tool");
        assert!(compare_projection(&value, &expected).is_err());
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
        std::fs::create_dir_all(root.path().join(".gemini/tmp")).unwrap();
        let launcher = bin.join("gemini");
        std::fs::write(&launcher, "#!/bin/sh\nexit 1\n").unwrap();
        std::fs::set_permissions(&launcher, std::fs::Permissions::from_mode(0o700)).unwrap();

        let receipt = root.path().join("receipt");
        let publication = root.path().join("publication.json");
        let version_flag = root.path().join("wrong-version");
        let runtime = bin.join("node");
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
        let id = "00112233-4455-4677-8899-aabbccddeeff";
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
            target_store_id: GeminiWriter.resolve_target_store_id().unwrap(),
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
            GeminiWriter.restore(&input, &context),
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
            GeminiWriter.restore(&input, &context),
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
            GeminiWriter.restore(&input, &failed_context),
            NativeWriteOutcome::ProvenNoSideEffect { .. }
        ));
        assert!(!publication.exists());
        assert!(
            matches!(GeminiWriter.restore(&input,&context),NativeWriteOutcome::Written{target_native_id:Some(ref observed),..} if observed==id)
        );
        let published: Value =
            serde_json::from_slice(&std::fs::read(&publication).unwrap()).unwrap();
        assert_eq!(published["id"], id);
        assert_eq!(published["messages"][0]["text"], "Q\r\n中文");
        assert_eq!(context.count.get(), 1);
        assert!(matches!(
            GeminiWriter.restore(&input, &context),
            NativeWriteOutcome::Unresolved { .. }
        ));
        let still_published: Value =
            serde_json::from_slice(&std::fs::read(&publication).unwrap()).unwrap();
        assert_eq!(published, still_published);
    }
}
