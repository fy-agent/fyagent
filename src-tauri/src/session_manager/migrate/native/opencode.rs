//! Version-pinned, final-text-only OpenCode import and official export readback.
//! The target's resolved model is used for the new session; no source route is copied.

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

const PROVIDER_ID: &str = "opencode";
pub const WRITE_STRATEGY: &str = "opencode.import-staged.create-only-v3";
pub const VERIFIED_VERSIONS: &[&str] = &["=1.18.30"];
const TIMEOUT: Duration = Duration::from_secs(90);

pub struct OpenCodeWriter;

impl NativeSessionWriter for OpenCodeWriter {
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
        identity::store_instance_id(PROVIDER_ID, &crate::opencode_config::get_opencode_db_path())
    }

    fn restore(
        &self,
        input: &NativeRestoreInput,
        context: &dyn NativeWriteContext,
    ) -> NativeWriteOutcome {
        let prepared = (|| {
            if input
                .session
                .messages
                .first()
                .is_some_and(|m| m.kind == MessageKind::AssistantFinal)
            {
                return Err(protocol(
                    "native history",
                    "OpenCode requires a user message before the first final answer.",
                ));
            }
            let cli = checked_cli()?;
            if self.resolve_target_store_id()? != input.target_store_id {
                return Err(protocol(
                    "target store",
                    "The target store changed; reopen the restore dialog.",
                ));
            }
            let native_id = match &input.target_native_id {
                Some(id) if valid_native_id(id) => id.clone(),
                Some(_) => {
                    return Err(protocol(
                        "native ID",
                        "Invalid preallocated native session ID.",
                    ))
                }
                None => format!("ses_{}", uuid::Uuid::new_v4().simple()),
            };
            // Early diagnostic only. INSERT under the publication transaction is the
            // atomic guarantee; the official upserting importer never sees this store.
            if native_id_exists(&crate::opencode_config::get_opencode_db_path(), &native_id)? {
                return Err(protocol(
                    "native ID",
                    "This native session already exists; reconcile the existing receipt.",
                ));
            }
            validate_store_schema(&open_store_readonly(
                &crate::opencode_config::get_opencode_db_path(),
            )?)?;
            let staging = tempfile::Builder::new()
                .prefix("fyagent-opencode-")
                .tempdir()
                .map_err(|_| {
                    protocol(
                        "prepare import",
                        "Unable to create a private staging directory.",
                    )
                })?;
            let staged_db = staging.path().join("opencode.db");
            let mut command = target_command(&cli);
            command
                .env("OPENCODE_DB", &staged_db)
                .args(["debug", "config", "--pure"])
                .current_dir(&input.target_workspace);
            let config = checked_output(command, "debug config --pure")?;
            let model = parse_target_model(&config)?;
            let body = build_native_session(&native_id, input, &model);
            let file = staging.path().join("import.json");
            std::fs::write(&file, body.to_string()).map_err(|_| {
                protocol("prepare import", "Unable to write the private import file.")
            })?;
            Ok((cli, native_id, staging, file, staged_db))
        })();
        let (cli, native_id, _staging, file, staged_db) = match prepared {
            Ok(prepared) => prepared,
            Err(error) => return NativeWriteOutcome::ProvenNoSideEffect { error },
        };
        if let Err(error) = context.record_native_id(&native_id) {
            return NativeWriteOutcome::ProvenNoSideEffect { error };
        }
        let mut command = target_command(&cli);
        command
            .env("OPENCODE_DB", &staged_db)
            .args(["import", "--pure"])
            .arg(&file)
            .current_dir(&input.target_workspace);
        if checked_output(command, "private import --pure").is_err() {
            // Even a timed-out importer only has access to the disposable store.
            return NativeWriteOutcome::ProvenNoSideEffect {
                error: protocol(
                    "private import --pure",
                    "Private staging failed; no target session was published.",
                ),
            };
        }
        let mut command = target_command(&cli);
        command
            .env("OPENCODE_DB", &staged_db)
            .args(["export", "--pure", &native_id])
            .current_dir(&input.target_workspace);
        let validated = checked_export_output(command, "private export --pure")
            .and_then(|output| {
                serde_json::from_str::<Value>(&output)
                    .map_err(|_| protocol("private export --pure", "Invalid staged native export."))
            })
            .is_ok_and(|exported| {
                matches!(
                    compare_export(&exported, &native_id, &input.session),
                    ReadbackVerdict::Visible { .. }
                )
            });
        if !validated {
            return NativeWriteOutcome::ProvenNoSideEffect {
                error: protocol(
                    "private export --pure",
                    "Staged history did not match; no target session was published.",
                ),
            };
        }
        if self.resolve_target_store_id().as_ref().ok() != Some(&input.target_store_id) {
            return NativeWriteOutcome::ProvenNoSideEffect {
                error: protocol(
                    "target store",
                    "The target store changed during staging; reopen the restore dialog.",
                ),
            };
        }
        // No CLI runs while the real database write lock is held. Dropping the
        // TempDir removes the private DB, input, and any WAL/SHM companions.
        publish_staged_session(
            &crate::opencode_config::get_opencode_db_path(),
            &staged_db,
            &native_id,
            input.session.messages.len(),
        )
    }

    fn verify_readback(
        &self,
        native_id: &str,
        expected: &MigratableSession,
    ) -> MigrationResult<ReadbackVerdict> {
        if !valid_native_id(native_id) {
            return Err(protocol("export --pure", "Invalid native session ID."));
        }
        let cli = checked_cli()?;
        let mut command = target_command(&cli);
        command.args(["export", "--pure", native_id]);
        match run_with_output_limit(command, TIMEOUT, MAX_NATIVE_READBACK_OUTPUT) {
            RunOutcome::Exited {
                status: Some(0),
                stdout,
                ..
            } => {
                let exported: Value = serde_json::from_str(&stdout).map_err(|_| {
                    protocol("export --pure", "The native export was not valid JSON.")
                })?;
                Ok(compare_export(&exported, native_id, expected))
            }
            _ => Ok(ReadbackVerdict::Blocked {
                reason: "OpenCode did not return a valid native export; absence is not proven."
                    .into(),
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

fn target_command(cli: &Path) -> Command {
    let mut command = Command::new(cli);
    // Bind the exact store that was fingerprinted, including FyAgent overrides.
    command.env(
        "OPENCODE_DB",
        crate::opencode_config::get_opencode_db_path(),
    );
    command.env(
        "OPENCODE_CONFIG_DIR",
        crate::opencode_config::get_opencode_dir(),
    );
    command.env("OPENCODE_DISABLE_MODELS_FETCH", "true");
    command.env("OPENCODE_DISABLE_AUTOUPDATE", "true");
    command
}

fn checked_cli() -> MigrationResult<PathBuf> {
    if super::user_cli_execution_blocked().is_some() {
        return Err(protocol(
            "native process",
            "OpenCode restoration requires the ordinary-user Windows execution helper.",
        ));
    }
    let cli =
        provider_cli_path(PROVIDER_ID).ok_or_else(|| MigrationError::ProviderNotInstalled {
            provider_id: PROVIDER_ID.into(),
        })?;
    let mut command = target_command(&cli);
    command.arg("--version");
    let version = checked_output(command, "--version")?;
    if version.trim() != "1.18.30" {
        return Err(MigrationError::ProviderVersionUnsupported {
            provider_id: PROVIDER_ID.into(),
            detected: Some(version.trim().chars().take(64).collect()),
            verified: VERIFIED_VERSIONS.iter().map(|s| (*s).into()).collect(),
        });
    }
    Ok(cli)
}

fn checked_output(command: Command, method: &str) -> MigrationResult<String> {
    checked_response(run_with_timeout(command, TIMEOUT), method)
}

fn checked_export_output(command: Command, method: &str) -> MigrationResult<String> {
    checked_response(
        run_with_output_limit(command, TIMEOUT, MAX_NATIVE_READBACK_OUTPUT),
        method,
    )
}

fn checked_response(outcome: RunOutcome, method: &str) -> MigrationResult<String> {
    match outcome {
        RunOutcome::Exited {
            status: Some(0),
            stdout,
            ..
        } => Ok(stdout),
        // Configuration can contain credentials. Never surface output or stderr.
        _ => Err(protocol(
            method,
            "The target CLI did not return a successful response.",
        )),
    }
}

#[derive(Debug, PartialEq, Eq)]
struct TargetModel {
    provider_id: String,
    model_id: String,
}

fn parse_target_model(output: &str) -> MigrationResult<TargetModel> {
    let config: Value = serde_json::from_str(output).map_err(|_| {
        protocol(
            "debug config --pure",
            "Unable to read the target model selection.",
        )
    })?;
    let model = config.get("model").and_then(Value::as_str).unwrap_or("");
    let Some((provider, id)) = model.split_once('/') else {
        return Err(protocol(
            "target model",
            "Choose a default model in this device's OpenCode configuration before restoring.",
        ));
    };
    if provider.is_empty() || id.is_empty() || model.chars().any(char::is_whitespace) {
        return Err(protocol(
            "target model",
            "The target default model must have provider/model form.",
        ));
    }
    Ok(TargetModel {
        provider_id: provider.into(),
        model_id: id.into(),
    })
}

fn valid_native_id(id: &str) -> bool {
    id.strip_prefix("ses_")
        .is_some_and(|body| body.len() == 32 && body.bytes().all(|b| b.is_ascii_hexdigit()))
}

fn native_id_exists(path: &Path, native_id: &str) -> MigrationResult<bool> {
    let conn =
        rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|_| {
                protocol(
                    "native ID check",
                    "The target session database could not be inspected.",
                )
            })?;
    conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM session WHERE id = ?1)",
        [native_id],
        |row| row.get(0),
    )
    .map_err(|_| {
        protocol(
            "native ID check",
            "The target session schema is not recognized.",
        )
    })
}

// Exact OpenCode 1.18.30 table_info signatures (name:type:notnull:pk).
// Unknown layouts and target triggers fail closed before content publication.
const NATIVE_TABLES: &[(&str, &str)] = &[
    ("project", "id:text:0:1,worktree:text:1:0,vcs:text:0:0,name:text:0:0,icon_url:text:0:0,icon_url_override:text:0:0,icon_color:text:0:0,time_created:integer:1:0,time_updated:integer:1:0,time_initialized:integer:0:0,sandboxes:text:1:0,commands:text:0:0"),
    ("session", "id:text:0:1,project_id:text:1:0,workspace_id:text:0:0,parent_id:text:0:0,slug:text:1:0,directory:text:1:0,path:text:0:0,title:text:1:0,version:text:1:0,share_url:text:0:0,summary_additions:integer:0:0,summary_deletions:integer:0:0,summary_files:integer:0:0,summary_diffs:text:0:0,metadata:text:0:0,cost:real:1:0,tokens_input:integer:1:0,tokens_output:integer:1:0,tokens_reasoning:integer:1:0,tokens_cache_read:integer:1:0,tokens_cache_write:integer:1:0,revert:text:0:0,permission:text:0:0,agent:text:0:0,model:text:0:0,time_created:integer:1:0,time_updated:integer:1:0,time_compacting:integer:0:0,time_archived:integer:0:0"),
    ("message", "id:text:0:1,session_id:text:1:0,time_created:integer:1:0,time_updated:integer:1:0,data:text:1:0"),
    ("part", "id:text:0:1,message_id:text:1:0,session_id:text:1:0,time_created:integer:1:0,time_updated:integer:1:0,data:text:1:0"),
];

fn open_store_readonly(path: &Path) -> MigrationResult<rusqlite::Connection> {
    rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(
        |_| {
            protocol(
                "store schema",
                "The existing native store could not be opened.",
            )
        },
    )
}

fn validate_store_schema(db: &rusqlite::Connection) -> MigrationResult<()> {
    let check = || -> rusqlite::Result<bool> {
        for (table, expected) in NATIVE_TABLES {
            let mut statement = db.prepare(&format!("PRAGMA table_xinfo({table})"))?;
            let actual = statement
                .query_map([], |row| {
                    Ok(format!(
                        "{}:{}:{}:{}:{}",
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?.to_ascii_lowercase(),
                        row.get::<_, i64>(3)?,
                        row.get::<_, i64>(5)?,
                        row.get::<_, i64>(6)?
                    ))
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            let expected: Vec<_> = expected
                .split(',')
                .map(|column| format!("{column}:0"))
                .collect();
            if actual != expected {
                return Ok(false);
            }
            let triggers: i64 = db.query_row(
                "SELECT count(*) FROM sqlite_master WHERE type = 'trigger' AND tbl_name = ?1",
                [table],
                |row| row.get(0),
            )?;
            if triggers != 0 {
                return Ok(false);
            }
        }
        Ok(true)
    };
    if check().unwrap_or(false) {
        Ok(())
    } else {
        Err(protocol(
            "store schema",
            "This native database layout is not verified for OpenCode 1.18.30.",
        ))
    }
}

fn copy_native_rows(
    source: &rusqlite::Connection,
    target: &rusqlite::Transaction<'_>,
    table: &str,
    key: &str,
    value: &str,
) -> rusqlite::Result<usize> {
    let signature = NATIVE_TABLES
        .iter()
        .find(|(name, _)| *name == table)
        .unwrap()
        .1;
    let columns: Vec<_> = signature
        .split(',')
        .map(|column| column.split(':').next().unwrap())
        .collect();
    let names = columns.join(",");
    let mut select = source.prepare(&format!("SELECT {names} FROM {table} WHERE {key} = ?1"))?;
    let mut rows = select.query([value])?;
    let placeholders = vec!["?"; columns.len()].join(",");
    let mut insert = target.prepare(&format!(
        "INSERT INTO {table} ({names}) VALUES ({placeholders})"
    ))?;
    let mut count = 0;
    while let Some(row) = rows.next()? {
        let values = (0..columns.len())
            .map(|index| row.get::<_, rusqlite::types::Value>(index))
            .collect::<rusqlite::Result<Vec<_>>>()?;
        count += insert.execute(rusqlite::params_from_iter(values))?;
    }
    Ok(count)
}

fn publish_staged_session(
    target_path: &Path,
    staged_path: &Path,
    native_id: &str,
    message_count: usize,
) -> NativeWriteOutcome {
    let prepared = (|| {
        let source = open_store_readonly(staged_path)?;
        validate_store_schema(&source)?;
        let target = rusqlite::Connection::open_with_flags(
            target_path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE,
        )
        .map_err(|_| protocol("publish", "The existing target store could not be opened."))?;
        target
            .busy_timeout(Duration::from_secs(5))
            .and_then(|_| target.execute_batch("PRAGMA foreign_keys = ON;"))
            .map_err(|_| protocol("publish", "The target connection could not be prepared."))?;
        Ok((source, target))
    })();
    let (source, mut target) = match prepared {
        Ok(value) => value,
        Err(error) => return NativeWriteOutcome::ProvenNoSideEffect { error },
    };
    let transaction =
        match target.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate) {
            Ok(value) => value,
            Err(_) => {
                return NativeWriteOutcome::ProvenNoSideEffect {
                    error: protocol(
                        "publish",
                        "The target transaction could not be started; no session was published.",
                    ),
                }
            }
        };
    let publish = (|| {
        // Validate after acquiring the write lock so concurrent schema changes
        // cannot invalidate the gate between inspection and INSERT.
        validate_store_schema(&transaction)?;
        let copy = || -> rusqlite::Result<bool> {
            let project_id: String = source.query_row(
                "SELECT project_id FROM session WHERE id = ?1",
                [native_id],
                |row| row.get(0),
            )?;
            let project_exists: bool = transaction.query_row(
                "SELECT EXISTS(SELECT 1 FROM project WHERE id = ?1)",
                [&project_id],
                |row| row.get(0),
            )?;
            if !project_exists
                && copy_native_rows(&source, &transaction, "project", "id", &project_id)? != 1
            {
                return Ok(false);
            }
            // These are deliberately plain INSERTs. Any primary-key collision,
            // including one appearing after preflight, rolls back every row.
            Ok(
                copy_native_rows(&source, &transaction, "session", "id", native_id)? == 1
                    && copy_native_rows(&source, &transaction, "message", "session_id", native_id)?
                        == message_count
                    && copy_native_rows(&source, &transaction, "part", "session_id", native_id)?
                        == message_count,
            )
        };
        if copy().unwrap_or(false) {
            Ok(())
        } else {
            Err(protocol("publish", "Native rows could not be inserted without a conflict; the transaction was rolled back."))
        }
    })();
    if let Err(error) = publish {
        return match transaction.rollback() {
            Ok(()) => NativeWriteOutcome::ProvenNoSideEffect { error },
            Err(_) => NativeWriteOutcome::Unresolved {
                error: protocol(
                    "publish rollback",
                    "The target rollback could not be confirmed; reconcile before retrying.",
                ),
            },
        };
    }
    match transaction.commit() {
        Ok(()) => NativeWriteOutcome::Written {
            target_native_id: Some(native_id.into()),
            nonce_carrier: NonceCarrier::NotUsed,
        },
        Err(_) => NativeWriteOutcome::Unresolved {
            error: protocol(
                "publish commit",
                "The target commit could not be confirmed; reconcile before retrying.",
            ),
        },
    }
}

fn build_native_session(native_id: &str, input: &NativeRestoreInput, model: &TargetModel) -> Value {
    let workspace = input.target_workspace.to_string_lossy();
    let now = chrono::Utc::now().timestamp_millis();
    let mut parent = String::new();
    let messages: Vec<Value> = input.session.messages.iter().map(|message| {
        let id = format!("msg_{:010}_{}", message.seq, uuid::Uuid::new_v4().simple());
        // Native pagination sorts by this field; source timestamps can go backwards.
        let at = now + i64::from(message.seq);
        let info = match message.kind {
            MessageKind::UserText => {
                parent = id.clone();
                json!({"id":id,"sessionID":native_id,"role":"user","time":{"created":at},
                    "agent":"build","model":{"providerID":model.provider_id,"modelID":model.model_id}})
            }
            MessageKind::AssistantFinal => json!({
                "id":id,"sessionID":native_id,"role":"assistant","time":{"created":at,"completed":at},
                "parentID":parent,"providerID":model.provider_id,"modelID":model.model_id,"mode":"build","agent":"build",
                "path":{"cwd":workspace,"root":workspace},"cost":0,
                "tokens":{"input":0,"output":0,"reasoning":0,"cache":{"read":0,"write":0}},"finish":"stop"
            }),
        };
        json!({"info":info,"parts":[{"id":format!("prt_{}",uuid::Uuid::new_v4().simple()),
            "sessionID":native_id,"messageID":id,"type":"text","text":message.text}]})
    }).collect();
    json!({"info":{"id":native_id,"slug":format!("restored-{}", &native_id[4..12]),
        "projectID":"global","directory":workspace,"title":input.session.title.as_deref().unwrap_or("Restored session"),
        "version":"1.18.30","time":{"created":now,"updated":now},
        "model":{"providerID":model.provider_id,"id":model.model_id}},"messages":messages})
}

fn compare_export(
    exported: &Value,
    native_id: &str,
    expected: &MigratableSession,
) -> ReadbackVerdict {
    let blocked = || ReadbackVerdict::Blocked {
        reason: "The native export is not a matching final-text-only session.".into(),
    };
    if exported.pointer("/info/id").and_then(Value::as_str) != Some(native_id) {
        return blocked();
    }
    let Some(messages) = exported.get("messages").and_then(Value::as_array) else {
        return blocked();
    };
    let mut projected = Vec::new();
    for message in messages {
        let kind = match message.pointer("/info/role").and_then(Value::as_str) {
            Some("user") => MessageKind::UserText,
            Some("assistant")
                if message.pointer("/info/finish").and_then(Value::as_str) == Some("stop")
                    && message
                        .pointer("/info/time/completed")
                        .and_then(Value::as_i64)
                        .is_some()
                    && message.pointer("/info/error").is_none_or(Value::is_null) =>
            {
                MessageKind::AssistantFinal
            }
            _ => return blocked(),
        };
        let Some(parts) = message.get("parts").and_then(Value::as_array) else {
            return blocked();
        };
        if parts.len() != 1
            || parts[0].get("type").and_then(Value::as_str) != Some("text")
            || parts[0].get("synthetic").and_then(Value::as_bool) == Some(true)
            || parts[0].get("ignored").and_then(Value::as_bool) == Some(true)
        {
            return blocked();
        }
        let Some(text) = parts[0].get("text").and_then(Value::as_str) else {
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

    const TEST_SCHEMA: &str = r#"CREATE TABLE `project` (
          `id` text PRIMARY KEY,
          `worktree` text NOT NULL,
          `vcs` text,
          `name` text,
          `icon_url` text,
          `icon_url_override` text,
          `icon_color` text,
          `time_created` integer NOT NULL,
          `time_updated` integer NOT NULL,
          `time_initialized` integer,
          `sandboxes` text NOT NULL,
          `commands` text
        );
CREATE TABLE `session` (
          `id` text PRIMARY KEY,
          `project_id` text NOT NULL,
          `workspace_id` text,
          `parent_id` text,
          `slug` text NOT NULL,
          `directory` text NOT NULL,
          `path` text,
          `title` text NOT NULL,
          `version` text NOT NULL,
          `share_url` text,
          `summary_additions` integer,
          `summary_deletions` integer,
          `summary_files` integer,
          `summary_diffs` text,
          `metadata` text,
          `cost` real DEFAULT 0 NOT NULL,
          `tokens_input` integer DEFAULT 0 NOT NULL,
          `tokens_output` integer DEFAULT 0 NOT NULL,
          `tokens_reasoning` integer DEFAULT 0 NOT NULL,
          `tokens_cache_read` integer DEFAULT 0 NOT NULL,
          `tokens_cache_write` integer DEFAULT 0 NOT NULL,
          `revert` text,
          `permission` text,
          `agent` text,
          `model` text,
          `time_created` integer NOT NULL,
          `time_updated` integer NOT NULL,
          `time_compacting` integer,
          `time_archived` integer,
          CONSTRAINT `fk_session_project_id_project_id_fk` FOREIGN KEY (`project_id`) REFERENCES `project`(`id`) ON DELETE CASCADE
        );
CREATE TABLE `message` (
          `id` text PRIMARY KEY,
          `session_id` text NOT NULL,
          `time_created` integer NOT NULL,
          `time_updated` integer NOT NULL,
          `data` text NOT NULL,
          CONSTRAINT `fk_message_session_id_session_id_fk` FOREIGN KEY (`session_id`) REFERENCES `session`(`id`) ON DELETE CASCADE
        );
CREATE TABLE `part` (
          `id` text PRIMARY KEY,
          `message_id` text NOT NULL,
          `session_id` text NOT NULL,
          `time_created` integer NOT NULL,
          `time_updated` integer NOT NULL,
          `data` text NOT NULL,
          CONSTRAINT `fk_part_message_id_message_id_fk` FOREIGN KEY (`message_id`) REFERENCES `message`(`id`) ON DELETE CASCADE
        );"#;

    #[test]
    fn target_model_is_parsed_without_copying_provider_options() {
        assert_eq!(parse_target_model(r#"{"model":"local/vendor/model","provider":{"local":{"options":{"apiKey":"secret"}}}}"#).unwrap(),
            TargetModel { provider_id:"local".into(), model_id:"vendor/model".into() });
        for invalid in [
            r#"{}"#,
            r#"{"model":"fake"}"#,
            r#"{"model":"/model"}"#,
            r#"{"model":"p/"}"#,
            r#"{"model":"p/model name"}"#,
        ] {
            assert!(parse_target_model(invalid).is_err());
        }
    }

    #[test]
    fn native_id_validation_rejects_arguments_and_paths() {
        assert!(valid_native_id("ses_00112233445566778899aabbccddeeff"));
        for id in [
            "ses_",
            "--help",
            "../../x",
            "ses_00112233445566778899aabbccddeeff/x",
        ] {
            assert!(!valid_native_id(id));
        }
    }

    #[test]
    fn existing_native_id_is_never_safe_to_import_again() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("opencode.db");
        let db = rusqlite::Connection::open(&path).unwrap();
        db.execute_batch("CREATE TABLE session(id TEXT PRIMARY KEY); INSERT INTO session VALUES ('ses_existing');").unwrap();
        assert!(native_id_exists(&path, "ses_existing").unwrap());
        assert!(!native_id_exists(&path, "ses_other").unwrap());
    }

    #[cfg(target_os = "macos")]
    #[test]
    #[serial_test::serial]
    fn restore_checks_version_and_store_and_records_reused_id_before_import() {
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
        let db_path = root.path().join("opencode.db");
        let _environment = EnvGuard(
            ["FYAGENT_TEST_HOME", "OPENCODE_DB"]
                .into_iter()
                .map(|key| (key, std::env::var_os(key)))
                .collect(),
        );
        std::env::set_var("FYAGENT_TEST_HOME", root.path());
        std::env::set_var("OPENCODE_DB", &db_path);
        let database = rusqlite::Connection::open(&db_path).unwrap();
        database.execute_batch(TEST_SCHEMA).unwrap();
        let bin = root.path().join(".local/bin");
        std::fs::create_dir_all(&bin).unwrap();
        let receipt = root.path().join("receipt");
        let publication = root.path().join("publication.json");
        let version_flag = root.path().join("wrong-version");
        let executable = bin.join("opencode");
        std::fs::write(
            &executable,
            format!(
                r##"#!/usr/bin/python3
import json,sys,sqlite3,os
from pathlib import Path
receipt=Path({receipt})
publication=Path({publication})
if sys.argv[1]=='--version':
    print('1.18.31' if Path({version_flag}).exists() else '1.18.30')
elif sys.argv[1]=='debug':
    print('{{"model":"target/local"}}')
elif sys.argv[1]=='import':
    body=json.loads(Path(sys.argv[3]).read_text())
    assert receipt.read_text()==body['info']['id']
    assert os.environ['OPENCODE_DB'] != {target_db}
    db=sqlite3.connect(os.environ['OPENCODE_DB'])
    db.executescript({schema})
    sid=body['info']['id']
    db.execute("INSERT INTO project(id,worktree,time_created,time_updated,sandboxes) VALUES ('global','/',1,1,'[]')")
    db.execute("INSERT INTO session(id,project_id,slug,directory,title,version,time_created,time_updated) VALUES (?,'global','test','/','test','1.18.30',1,1)",(sid,))
    for message in body['messages']:
        mid=message['info']['id']
        db.execute("INSERT INTO message VALUES (?,?,1,1,?)",(mid,sid,json.dumps(message['info'])))
        for part in message['parts']:
            db.execute("INSERT INTO part VALUES (?,?,?,1,1,?)",(part['id'],mid,sid,json.dumps(part)))
    db.commit()
    with publication.open('x') as output:json.dump(body,output)
elif sys.argv[1]=='export':
    print(publication.read_text())
else:raise RuntimeError('unexpected native operation')
"##,
                receipt = json!(receipt),
                publication = json!(publication),
                version_flag = json!(version_flag),
                target_db = json!(db_path),
                schema = json!(TEST_SCHEMA)
            ),
        )
        .unwrap();
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700)).unwrap();
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
        let id = "ses_00112233445566778899aabbccddeeff";
        let mut input = NativeRestoreInput {
            session: expectation(vec![MigratableMessage {
                seq: 0,
                kind: MessageKind::UserText,
                text: "Q\r\n中文".into(),
                ts: None,
            }]),
            target_workspace: root.path().to_path_buf(),
            target_store_id: OpenCodeWriter.resolve_target_store_id().unwrap(),
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
            OpenCodeWriter.restore(&input, &context),
            NativeWriteOutcome::ProvenNoSideEffect {
                error: MigrationError::ProviderVersionUnsupported { .. }
            }
        ));
        assert_eq!(context.count.get(), 0);
        assert!(!publication.exists());
        std::fs::remove_file(version_flag).unwrap();
        let correct_store = input.target_store_id.clone();
        input.target_store_id = "different-store".into();
        assert!(matches!(
            OpenCodeWriter.restore(&input, &context),
            NativeWriteOutcome::ProvenNoSideEffect { .. }
        ));
        assert_eq!(context.count.get(), 0);
        input.target_store_id = correct_store;
        let failed_context = Context {
            path: receipt.clone(),
            fail: true,
            count: Cell::new(0),
        };
        assert!(matches!(
            OpenCodeWriter.restore(&input, &failed_context),
            NativeWriteOutcome::ProvenNoSideEffect { .. }
        ));
        assert!(!publication.exists());
        assert!(
            matches!(OpenCodeWriter.restore(&input,&context),NativeWriteOutcome::Written{target_native_id:Some(ref observed),..} if observed==id)
        );
        let published: Value =
            serde_json::from_slice(&std::fs::read(&publication).unwrap()).unwrap();
        assert_eq!(published["info"]["id"], id);
        assert_eq!(published["messages"][0]["parts"][0]["text"], "Q\r\n中文");
        assert_eq!(context.count.get(), 1);
        assert!(native_id_exists(&db_path, id).unwrap());
        assert!(matches!(
            OpenCodeWriter.restore(&input, &context),
            NativeWriteOutcome::ProvenNoSideEffect { .. }
        ));
        assert_eq!(context.count.get(), 1);
    }
    fn fixture_store(path: &Path) -> rusqlite::Connection {
        let db = rusqlite::Connection::open(path).unwrap();
        db.execute_batch(TEST_SCHEMA).unwrap();
        db.execute_batch("PRAGMA foreign_keys = OFF").unwrap();
        db
    }

    fn fixture_rows(
        db: &rusqlite::Connection,
        session: &str,
        project: &str,
        message: &str,
        part: &str,
    ) {
        db.execute("INSERT INTO project(id,worktree,name,time_created,time_updated,sandboxes) VALUES (?1,'/target/workspace','original metadata',1,1,'[]')", [project]).unwrap();
        db.execute("INSERT INTO session(id,project_id,slug,directory,title,version,time_created,time_updated) VALUES (?1,?2,'test','/target/workspace','test','1.18.30',1,1)", [session, project]).unwrap();
        db.execute(
            "INSERT INTO message VALUES (?1,?2,1,1,'{}')",
            [message, session],
        )
        .unwrap();
        db.execute(
            "INSERT INTO part VALUES (?1,?2,?3,1,1,'{}')",
            [part, message, session],
        )
        .unwrap();
    }

    fn snapshot(db: &rusqlite::Connection) -> Vec<Vec<Vec<rusqlite::types::Value>>> {
        NATIVE_TABLES
            .iter()
            .map(|(table, _)| {
                let mut statement = db
                    .prepare(&format!("SELECT * FROM {table} ORDER BY id"))
                    .unwrap();
                let count = statement.column_count();
                statement
                    .query_map([], |row| (0..count).map(|i| row.get(i)).collect())
                    .unwrap()
                    .collect::<rusqlite::Result<Vec<_>>>()
                    .unwrap()
            })
            .collect()
    }

    #[test]
    fn publication_creates_missing_project_and_preserves_existing_project_metadata() {
        let root = tempfile::tempdir().unwrap();
        let staged = root.path().join("stage.db");
        let target = root.path().join("target.db");
        let source = fixture_store(&staged);
        let destination = fixture_store(&target);
        fixture_rows(&source, "new", "git-project", "m1", "p1");
        assert!(matches!(
            publish_staged_session(&target, &staged, "new", 1),
            NativeWriteOutcome::Written { .. }
        ));
        assert_eq!(snapshot(&source), snapshot(&destination));
        destination
            .execute("UPDATE project SET name = 'preserved target metadata'", [])
            .unwrap();
        source
            .execute("UPDATE session SET id = 'other'", [])
            .unwrap();
        source
            .execute("UPDATE message SET id = 'm2', session_id = 'other'", [])
            .unwrap();
        source
            .execute(
                "UPDATE part SET id = 'p2', message_id = 'm2', session_id = 'other'",
                [],
            )
            .unwrap();
        assert!(matches!(
            publish_staged_session(&target, &staged, "other", 1),
            NativeWriteOutcome::Written { .. }
        ));
        assert_eq!(
            destination
                .query_row("SELECT name FROM project", [], |r| r.get::<_, String>(0))
                .unwrap(),
            "preserved target metadata"
        );
    }

    #[test]
    fn publication_rolls_back_all_rows_on_session_message_or_part_collision() {
        for collision in ["session", "message", "part"] {
            let root = tempfile::tempdir().unwrap();
            let staged = root.path().join("stage.db");
            let target = root.path().join("target.db");
            let source = fixture_store(&staged);
            let destination = fixture_store(&target);
            fixture_rows(&source, "new", "new-project", "new-message", "new-part");
            assert!(!native_id_exists(&target, "new").unwrap());
            // Simulate a concurrent writer arriving after the diagnostic check.
            fixture_rows(
                &destination,
                if collision == "session" { "new" } else { "old" },
                "old-project",
                if collision == "message" {
                    "new-message"
                } else {
                    "old-message"
                },
                if collision == "part" {
                    "new-part"
                } else {
                    "old-part"
                },
            );
            let before = snapshot(&destination);
            assert!(matches!(
                publish_staged_session(&target, &staged, "new", 1),
                NativeWriteOutcome::ProvenNoSideEffect { .. }
            ));
            assert_eq!(snapshot(&destination), before, "collision in {collision}");
        }
    }

    #[test]
    fn publication_fails_closed_on_schema_drift_trigger_missing_project_and_wrong_count() {
        for issue in ["schema", "trigger", "project", "count"] {
            let root = tempfile::tempdir().unwrap();
            let staged = root.path().join("stage.db");
            let target = root.path().join("target.db");
            let source = fixture_store(&staged);
            let destination = fixture_store(&target);
            fixture_rows(&source, "new", "new-project", "m1", "p1");
            match issue {
                "schema" => destination.execute_batch("ALTER TABLE session ADD COLUMN unknown TEXT").unwrap(),
                "trigger" => destination.execute_batch("CREATE TRIGGER custom AFTER INSERT ON session BEGIN UPDATE project SET name='unexpected'; END;").unwrap(),
                "project" => { source.execute("DELETE FROM project", []).unwrap(); },
                _ => (),
            }
            let before = snapshot(&destination);
            assert!(matches!(
                publish_staged_session(
                    &target,
                    &staged,
                    "new",
                    if issue == "count" { 2 } else { 1 }
                ),
                NativeWriteOutcome::ProvenNoSideEffect { .. }
            ));
            assert_eq!(snapshot(&destination), before, "issue {issue}");
        }
    }

    /// Explicit opt-in only. The harness supplies an empty synthetic HOME/XDG,
    /// an exact-version local CLI, a fresh native DB, and a loopback-only model.
    #[cfg(target_os = "macos")]
    #[test]
    #[ignore = "requires FYAGENT_OPENCODE_PROBE_ROOT and isolated native CLI harness"]
    #[serial_test::serial]
    fn isolated_actual_writer_create_only_probe() {
        use std::cell::RefCell;
        let root = PathBuf::from(
            std::env::var_os("FYAGENT_OPENCODE_PROBE_ROOT").expect("explicit isolated probe root"),
        );
        assert!(root.is_absolute());
        for key in [
            "HOME",
            "FYAGENT_TEST_HOME",
            "OPENCODE_DB",
            "OPENCODE_CONFIG_DIR",
            "XDG_CONFIG_HOME",
            "XDG_DATA_HOME",
            "XDG_CACHE_HOME",
            "XDG_STATE_HOME",
            "TMPDIR",
        ] {
            assert!(
                PathBuf::from(std::env::var_os(key).expect(key)).starts_with(&root),
                "nonisolated {key}"
            );
        }
        let target = crate::opencode_config::get_opencode_db_path();
        let workspace = root.join("project").canonicalize().unwrap();
        let database = open_store_readonly(&target).unwrap();
        let kinds = [
            MessageKind::UserText,
            MessageKind::UserText,
            MessageKind::AssistantFinal,
            MessageKind::AssistantFinal,
            MessageKind::UserText,
        ];
        let texts = [
            "  first\r\n",
            "KITE-73 中文",
            " answer\r\n  ",
            "C:\\foo",
            "unanswered",
        ];
        let input = NativeRestoreInput {
            session: expectation(
                kinds
                    .into_iter()
                    .zip(texts)
                    .enumerate()
                    .map(|(i, (kind, text))| MigratableMessage {
                        seq: i as u32,
                        kind,
                        text: text.into(),
                        ts: Some(100 - i as i64),
                    })
                    .collect(),
            ),
            target_workspace: workspace.clone(),
            target_store_id: OpenCodeWriter.resolve_target_store_id().unwrap(),
            target_native_nonce: String::new(),
            target_native_id: Some("ses_00112233445566778899aabbccddeeff".into()),
        };
        struct Context<'a> {
            root: &'a Path,
            target: &'a Path,
            collision: bool,
            at_receipt: RefCell<Option<Vec<Vec<Vec<rusqlite::types::Value>>>>>,
        }
        impl NativeWriteContext for Context<'_> {
            fn record_native_id(&self, id: &str) -> MigrationResult<()> {
                assert!(!native_id_exists(self.target, id).unwrap());
                let db = rusqlite::Connection::open(self.target).unwrap();
                if self.collision {
                    fixture_rows(
                        &db,
                        id,
                        "concurrent-project",
                        "concurrent-message",
                        "concurrent-part",
                    );
                }
                self.at_receipt.replace(Some(snapshot(&db)));
                std::fs::write(
                    self.root.join(if self.collision {
                        "collision-receipt"
                    } else {
                        "receipt"
                    }),
                    id,
                )
                .unwrap();
                Ok(())
            }
        }
        let before_projects: i64 = database
            .query_row("SELECT count(*) FROM project WHERE vcs = 'git'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(
            before_projects, 0,
            "the target Git project must be absent initially"
        );
        let context = Context {
            root: &root,
            target: &target,
            collision: false,
            at_receipt: RefCell::new(None),
        };
        let outcome = OpenCodeWriter.restore(&input, &context);
        assert!(
            matches!(outcome, NativeWriteOutcome::Written { .. }),
            "{outcome:?}"
        );
        let id = input.target_native_id.as_deref().unwrap();
        assert!(matches!(
            OpenCodeWriter.verify_readback(id, &input.session).unwrap(),
            ReadbackVerdict::Visible { .. }
        ));
        let (project, directory, worktree): (String, String, String) = database.query_row(
            "SELECT s.project_id,s.directory,p.worktree FROM session s JOIN project p ON p.id = s.project_id WHERE s.id = ?1", [id], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?))).unwrap();
        assert_ne!(project, "global");
        assert_eq!(directory, workspace.to_string_lossy());
        assert_eq!(worktree, workspace.to_string_lossy());
        let mut collision_input = input.clone();
        collision_input.target_native_id = Some("ses_ffeeddccbbaa99887766554433221100".into());
        let collision = Context {
            root: &root,
            target: &target,
            collision: true,
            at_receipt: RefCell::new(None),
        };
        assert!(matches!(
            OpenCodeWriter.restore(&collision_input, &collision),
            NativeWriteOutcome::ProvenNoSideEffect { .. }
        ));
        assert_eq!(
            snapshot(&database),
            *collision.at_receipt.borrow().as_ref().unwrap()
        );
        std::fs::write(root.join("rust-writer-result.json"), serde_json::to_vec_pretty(&json!({
            "native_id": id, "project_id": project, "workspace": workspace, "db": target,
            "messages": input.session.messages, "digest": input.session.content_digest,
            "official_readback": "visible", "missing_project_created": true,
            "race_collision": "full rollback; concurrent native records byte-for-byte unchanged",
            "receipt_before_target_publication": true
        })).unwrap()).unwrap();
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
    fn native_projection_keeps_order_and_text_and_rejects_nonfinal_readback() {
        let kinds = [
            MessageKind::UserText,
            MessageKind::UserText,
            MessageKind::AssistantFinal,
            MessageKind::AssistantFinal,
            MessageKind::UserText,
        ];
        let texts = [
            "  first\r\n",
            "KITE-73 中文",
            " answer\r\n  ",
            "C:\\foo",
            "unanswered",
        ];
        let session = expectation(
            kinds
                .into_iter()
                .zip(texts)
                .enumerate()
                .map(|(i, (kind, text))| MigratableMessage {
                    seq: i as u32,
                    kind,
                    text: text.into(),
                    ts: None,
                })
                .collect(),
        );
        let input = NativeRestoreInput {
            session,
            target_workspace: PathBuf::from("/target"),
            target_store_id: "test".into(),
            target_native_nonce: String::new(),
            target_native_id: None,
        };
        let id = "ses_00112233445566778899aabbccddeeff";
        let mut exported = build_native_session(
            id,
            &input,
            &TargetModel {
                provider_id: "local".into(),
                model_id: "target".into(),
            },
        );
        assert_eq!(
            exported.pointer("/info/version").and_then(Value::as_str),
            Some("1.18.30")
        );
        assert_eq!(
            exported.pointer("/info/model"),
            Some(&json!({"providerID":"local","id":"target"}))
        );
        assert!(matches!(
            compare_export(&exported, id, &input.session),
            ReadbackVerdict::Visible { .. }
        ));
        let temporary = tempfile::tempdir().unwrap();
        let source = temporary.path().join("native-export.json");
        std::fs::write(&source, exported.to_string()).unwrap();
        let extracted = identity::with_local_state_dir(temporary.path(), || {
            crate::session_manager::migrate::extract::extract_session(
                PROVIDER_ID,
                source.to_str().unwrap(),
                None,
            )
        })
        .unwrap();
        assert_eq!(extracted.content_digest, input.session.content_digest);
        assert_eq!(extracted.origin.cli_version.as_deref(), Some("1.18.30"));
        exported["messages"][2]["info"]["time"]["completed"] = Value::Null;
        assert!(matches!(
            compare_export(&exported, id, &input.session),
            ReadbackVerdict::Blocked { .. }
        ));
        exported["messages"][2]["info"]["time"]["completed"] = json!(1);
        exported["messages"][0]["parts"][0]["synthetic"] = json!(true);
        assert!(matches!(
            compare_export(&exported, id, &input.session),
            ReadbackVerdict::Blocked { .. }
        ));
        exported["messages"][0]["parts"][0]["synthetic"] = json!(false);
        exported["messages"][0]["parts"][0]["text"] = json!("first\n");
        assert!(matches!(
            compare_export(&exported, id, &input.session),
            ReadbackVerdict::Mismatch { .. }
        ));
    }
}
