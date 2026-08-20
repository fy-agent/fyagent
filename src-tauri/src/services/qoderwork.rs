//! Transactional, privacy-safe access to QoderWork's documented user Hooks.
//!
//! The renderer receives only a lossless projection of the documented
//! event/matcher/command/timeout shape. Unknown values remain on disk and make
//! structured saves fail closed; they are never serialized into errors,
//! revisions, confirmation capabilities, or logs.

use std::{
    collections::HashMap,
    fmt, fs,
    io::{self, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::{Mutex as StdMutex, MutexGuard as StdMutexGuard},
    time::{Duration, Instant},
};

#[cfg(target_os = "windows")]
use std::path::Component;

use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::Sha256;
use tokio::sync::Mutex;
use uuid::Uuid;

const SETTINGS_FILE_NAME: &str = "settings.json";
const BACKUP_FILE_NAME: &str = "settings.json.backup";
const MAX_DOCUMENT_BYTES: u64 = 2 * 1024 * 1024;
const MAX_GROUPS: usize = 256;
const MAX_COMMANDS_PER_GROUP: usize = 64;
const MAX_TOTAL_COMMANDS: usize = 1_024;
const MAX_MATCHER_BYTES: usize = 4_096;
const MAX_COMMAND_BYTES: usize = 4_096;
const MIN_TIMEOUT_SECONDS: u16 = 1;
const MAX_TIMEOUT_SECONDS: u16 = 600;
const OVERWRITE_TOKEN_TTL: Duration = Duration::from_secs(3 * 60);
const EXPIRED_TOKEN_RETENTION: Duration = Duration::from_secs(3 * 60);
const MAX_PENDING_OVERWRITES: usize = 32;

type HmacSha256 = Hmac<Sha256>;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub enum QoderHookEvent {
    SessionStart,
    SessionEnd,
    UserPromptSubmit,
    PreToolUse,
    PostToolUse,
    PostToolUseFailure,
    Stop,
    SubagentStart,
    SubagentStop,
    PreCompact,
    Notification,
    PermissionRequest,
}

impl QoderHookEvent {
    fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "SessionStart" => Self::SessionStart,
            "SessionEnd" => Self::SessionEnd,
            "UserPromptSubmit" => Self::UserPromptSubmit,
            "PreToolUse" => Self::PreToolUse,
            "PostToolUse" => Self::PostToolUse,
            "PostToolUseFailure" => Self::PostToolUseFailure,
            "Stop" => Self::Stop,
            "SubagentStart" => Self::SubagentStart,
            "SubagentStop" => Self::SubagentStop,
            "PreCompact" => Self::PreCompact,
            "Notification" => Self::Notification,
            "PermissionRequest" => Self::PermissionRequest,
            _ => return None,
        })
    }

    const fn as_str(self) -> &'static str {
        match self {
            Self::SessionStart => "SessionStart",
            Self::SessionEnd => "SessionEnd",
            Self::UserPromptSubmit => "UserPromptSubmit",
            Self::PreToolUse => "PreToolUse",
            Self::PostToolUse => "PostToolUse",
            Self::PostToolUseFailure => "PostToolUseFailure",
            Self::Stop => "Stop",
            Self::SubagentStart => "SubagentStart",
            Self::SubagentStop => "SubagentStop",
            Self::PreCompact => "PreCompact",
            Self::Notification => "Notification",
            Self::PermissionRequest => "PermissionRequest",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum QoderHookType {
    Command,
}

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QoderHookCommand {
    #[serde(rename = "type")]
    pub kind: QoderHookType,
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u16>,
}

impl fmt::Debug for QoderHookCommand {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("QoderHookCommand")
            .field("kind", &self.kind)
            .field("command", &"[REDACTED]")
            .field("timeout", &self.timeout)
            .finish()
    }
}

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QoderHookGroup {
    pub event: QoderHookEvent,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matcher: Option<String>,
    pub hooks: Vec<QoderHookCommand>,
}

impl fmt::Debug for QoderHookGroup {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("QoderHookGroup")
            .field("event", &self.event)
            .field("matcher", &self.matcher.as_ref().map(|_| "[REDACTED]"))
            .field("hook_count", &self.hooks.len())
            .finish()
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct QoderHooksSnapshot {
    pub revision: Option<String>,
    pub exists: bool,
    pub groups: Vec<QoderHookGroup>,
    pub restart_required: bool,
    pub supported_structure: bool,
}

#[derive(Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SaveQoderworkHooksRequest {
    pub expected_revision: Option<String>,
    pub groups: Vec<QoderHookGroup>,
    #[serde(default)]
    pub overwrite_token: Option<String>,
}

impl fmt::Debug for SaveQoderworkHooksRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SaveQoderworkHooksRequest")
            .field("expected_revision", &"[REDACTED]")
            .field("group_count", &self.groups.len())
            .field("overwrite_token", &"[REDACTED]")
            .finish()
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "state")]
pub enum SaveQoderworkHooksOutcome {
    #[serde(rename = "saved")]
    Saved { snapshot: QoderHooksSnapshot },
    #[serde(rename = "overwrite_confirmation_required")]
    OverwriteConfirmationRequired { token: String },
    #[serde(rename = "concurrent_modification")]
    ConcurrentModification,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
pub enum QoderHooksErrorCode {
    #[serde(rename = "QODER_HOOKS_READ_FAILED")]
    ReadFailed,
    #[serde(rename = "QODER_HOOKS_DOCUMENT_TOO_LARGE")]
    DocumentTooLarge,
    #[serde(rename = "QODER_HOOKS_INVALID_JSON")]
    InvalidJson,
    #[serde(rename = "QODER_HOOKS_ROOT_NOT_OBJECT")]
    RootNotObject,
    #[serde(rename = "QODER_HOOKS_UNSUPPORTED_STRUCTURE")]
    UnsupportedStructure,
    #[serde(rename = "QODER_HOOKS_INVALID_REQUEST")]
    InvalidRequest,
    #[serde(rename = "QODER_HOOKS_OVERWRITE_TOKEN_INVALID")]
    OverwriteTokenInvalid,
    #[serde(rename = "QODER_HOOKS_OVERWRITE_TOKEN_EXPIRED")]
    OverwriteTokenExpired,
    #[serde(rename = "QODER_HOOKS_BACKUP_FAILED")]
    BackupFailed,
    #[serde(rename = "QODER_HOOKS_WRITE_FAILED")]
    WriteFailed,
    #[serde(rename = "QODER_HOOKS_WRITE_STATE_UNKNOWN")]
    WriteStateUnknown,
    #[serde(rename = "QODER_HOOKS_INTERNAL_ERROR")]
    InternalError,
}

impl QoderHooksErrorCode {
    const fn message_key(self) -> &'static str {
        match self {
            Self::ReadFailed => "qoderwork.hooks.error.readFailed",
            Self::DocumentTooLarge => "qoderwork.hooks.error.documentTooLarge",
            Self::InvalidJson => "qoderwork.hooks.error.invalidJson",
            Self::RootNotObject => "qoderwork.hooks.error.rootNotObject",
            Self::UnsupportedStructure => "qoderwork.hooks.error.unsupportedStructure",
            Self::InvalidRequest => "qoderwork.hooks.error.invalidRequest",
            Self::OverwriteTokenInvalid => "qoderwork.hooks.error.overwriteTokenInvalid",
            Self::OverwriteTokenExpired => "qoderwork.hooks.error.overwriteTokenExpired",
            Self::BackupFailed => "qoderwork.hooks.error.backupFailed",
            Self::WriteFailed => "qoderwork.hooks.error.writeFailed",
            Self::WriteStateUnknown => "qoderwork.hooks.error.writeStateUnknown",
            Self::InternalError => "qoderwork.hooks.error.internal",
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct QoderHooksErrorDto {
    pub code: QoderHooksErrorCode,
    pub message_key: String,
}

#[derive(Debug, Clone, thiserror::Error)]
#[error("Qoder Hooks error: {code:?}")]
pub struct QoderHooksError {
    code: QoderHooksErrorCode,
}

impl QoderHooksError {
    fn new(code: QoderHooksErrorCode) -> Self {
        Self { code }
    }

    #[cfg(test)]
    const fn code(&self) -> QoderHooksErrorCode {
        self.code
    }
}

impl From<QoderHooksError> for QoderHooksErrorDto {
    fn from(value: QoderHooksError) -> Self {
        Self {
            code: value.code,
            message_key: value.code.message_key().to_string(),
        }
    }
}

#[derive(Debug)]
struct PendingOverwrite {
    request_digest: [u8; 32],
    expected_revision: Option<String>,
    expires_at: Instant,
}

/// Process-local authority for Qoder's one document.
///
/// The main application must manage one instance. Keys, pending request
/// digests, commands, matchers, revisions and paths are intentionally absent
/// from its `Debug` representation.
pub struct QoderHooksState {
    write_lock: Mutex<()>,
    pending_overwrites: StdMutex<HashMap<String, PendingOverwrite>>,
    revision_key: [u8; 32],
    request_key: [u8; 32],
}

impl fmt::Debug for QoderHooksState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("QoderHooksState")
            .field("sensitive_state", &"[REDACTED]")
            .finish()
    }
}

impl Default for QoderHooksState {
    fn default() -> Self {
        Self {
            write_lock: Mutex::new(()),
            pending_overwrites: StdMutex::new(HashMap::new()),
            revision_key: random_mac_key(),
            request_key: random_mac_key(),
        }
    }
}

impl QoderHooksState {
    pub fn new() -> Self {
        Self::default()
    }

    fn pending(&self) -> StdMutexGuard<'_, HashMap<String, PendingOverwrite>> {
        self.pending_overwrites
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    #[cfg(test)]
    fn expire_token(&self, token: &str) {
        if let Some(pending) = self.pending().get_mut(token) {
            pending.expires_at = Instant::now() - Duration::from_secs(1);
        }
    }
}

#[derive(Debug, Clone)]
struct QoderPaths {
    home: PathBuf,
    directory: PathBuf,
    settings: PathBuf,
    backup: PathBuf,
}

impl QoderPaths {
    fn from_home(home: &Path) -> Self {
        let directory = home.join(".qoderwork");
        Self {
            home: home.to_path_buf(),
            settings: directory.join(SETTINGS_FILE_NAME),
            backup: directory.join(BACKUP_FILE_NAME),
            directory,
        }
    }

    fn validate_fixed_shape(&self) -> Result<(), QoderHooksError> {
        let expected = Self::from_home(&self.home);
        if !self.home.is_absolute()
            || expected.directory != self.directory
            || expected.settings != self.settings
            || expected.backup != self.backup
        {
            return Err(QoderHooksError::new(QoderHooksErrorCode::ReadFailed));
        }
        Ok(())
    }
}

#[derive(Debug)]
struct LoadedDocument {
    exists: bool,
    original_bytes: Vec<u8>,
    root: Map<String, Value>,
    groups: Vec<QoderHookGroup>,
    supported_structure: bool,
    revision: Option<String>,
}

pub async fn get_qoderwork_hooks(
    state: &QoderHooksState,
) -> Result<QoderHooksSnapshot, QoderHooksError> {
    let paths = QoderPaths::from_home(&crate::config::get_home_dir());
    let revision_key = state.revision_key;
    tokio::task::spawn_blocking(move || {
        let loaded = load_current_document(&paths, &revision_key)?;
        Ok(snapshot_from_loaded(&loaded))
    })
    .await
    .map_err(|_| QoderHooksError::new(QoderHooksErrorCode::InternalError))?
}

pub async fn save_qoderwork_hooks(
    state: &QoderHooksState,
    request: SaveQoderworkHooksRequest,
) -> Result<SaveQoderworkHooksOutcome, QoderHooksError> {
    validate_groups(&request.groups)?;
    let paths = QoderPaths::from_home(&crate::config::get_home_dir());
    let _guard = state.write_lock.lock().await;
    save_locked(state, &paths, &request)
}

fn save_locked(
    state: &QoderHooksState,
    paths: &QoderPaths,
    request: &SaveQoderworkHooksRequest,
) -> Result<SaveQoderworkHooksOutcome, QoderHooksError> {
    paths.validate_fixed_shape()?;
    let pending = request
        .overwrite_token
        .as_deref()
        .map(|token| consume_overwrite_token(state, token, paths, request))
        .transpose()?;

    let loaded = load_current_document(paths, &state.revision_key)?;
    if request.expected_revision != loaded.revision {
        return Ok(SaveQoderworkHooksOutcome::ConcurrentModification);
    }
    if let Some(pending) = &pending {
        if pending.expected_revision != loaded.revision {
            return Ok(SaveQoderworkHooksOutcome::ConcurrentModification);
        }
    }
    if !loaded.supported_structure {
        return Err(QoderHooksError::new(
            QoderHooksErrorCode::UnsupportedStructure,
        ));
    }

    if loaded.groups == request.groups {
        return Ok(SaveQoderworkHooksOutcome::Saved {
            snapshot: snapshot_from_loaded(&loaded),
        });
    }

    if pending.is_none() && is_destructive_change(&loaded.groups, &request.groups) {
        let token = issue_overwrite_token(state, paths, request);
        return Ok(SaveQoderworkHooksOutcome::OverwriteConfirmationRequired { token });
    }

    let mut root = loaded.root.clone();
    root.insert(
        "hooks".to_string(),
        Value::Object(groups_to_document_hooks(&request.groups)),
    );
    let replacement = serde_json::to_vec_pretty(&Value::Object(root))
        .map_err(|_| QoderHooksError::new(QoderHooksErrorCode::InvalidRequest))?;
    if replacement.len() as u64 > MAX_DOCUMENT_BYTES {
        return Err(QoderHooksError::new(QoderHooksErrorCode::DocumentTooLarge));
    }

    if !commit_document(paths, &loaded, &replacement)? {
        return Ok(SaveQoderworkHooksOutcome::ConcurrentModification);
    }

    // The namespace is authoritative after an atomic commit. A failure here
    // cannot be described as an unwritten save, so it has a dedicated
    // state-unknown result and never includes the document or command text.
    let reread = load_current_document(paths, &state.revision_key)
        .map_err(|_| QoderHooksError::new(QoderHooksErrorCode::WriteStateUnknown))?;
    if !reread.exists || reread.original_bytes != replacement || !reread.supported_structure {
        return Err(QoderHooksError::new(QoderHooksErrorCode::WriteStateUnknown));
    }
    Ok(SaveQoderworkHooksOutcome::Saved {
        snapshot: snapshot_from_loaded(&reread),
    })
}

fn load_current_document(
    paths: &QoderPaths,
    revision_key: &[u8; 32],
) -> Result<LoadedDocument, QoderHooksError> {
    paths.validate_fixed_shape()?;
    let bytes = read_settings(paths)?;
    load_document_bytes(bytes, revision_key)
}

fn load_document_bytes(
    bytes: Option<Vec<u8>>,
    revision_key: &[u8; 32],
) -> Result<LoadedDocument, QoderHooksError> {
    let Some(original_bytes) = bytes else {
        return Ok(LoadedDocument {
            exists: false,
            original_bytes: Vec::new(),
            root: Map::new(),
            groups: Vec::new(),
            supported_structure: true,
            revision: None,
        });
    };
    if original_bytes.len() as u64 > MAX_DOCUMENT_BYTES {
        return Err(QoderHooksError::new(QoderHooksErrorCode::DocumentTooLarge));
    }
    let value: Value = serde_json::from_slice(&original_bytes)
        .map_err(|_| QoderHooksError::new(QoderHooksErrorCode::InvalidJson))?;
    let root = value
        .as_object()
        .cloned()
        .ok_or_else(|| QoderHooksError::new(QoderHooksErrorCode::RootNotObject))?;
    let (groups, supported_structure) = project_hooks(&root);
    Ok(LoadedDocument {
        exists: true,
        revision: Some(revision_for(revision_key, &original_bytes)),
        original_bytes,
        root,
        groups,
        supported_structure,
    })
}

fn snapshot_from_loaded(loaded: &LoadedDocument) -> QoderHooksSnapshot {
    QoderHooksSnapshot {
        revision: loaded.revision.clone(),
        exists: loaded.exists,
        groups: loaded.groups.clone(),
        restart_required: true,
        supported_structure: loaded.supported_structure,
    }
}

fn project_hooks(root: &Map<String, Value>) -> (Vec<QoderHookGroup>, bool) {
    let Some(hooks_value) = root.get("hooks") else {
        return (Vec::new(), true);
    };
    let Some(hooks) = hooks_value.as_object() else {
        return (Vec::new(), false);
    };

    let mut groups = Vec::new();
    let mut supported = true;
    for (event_name, event_groups) in hooks {
        let Some(event) = QoderHookEvent::parse(event_name) else {
            supported = false;
            continue;
        };
        let Some(event_groups) = event_groups.as_array() else {
            supported = false;
            continue;
        };
        for raw_group in event_groups {
            match project_group(event, raw_group) {
                Some(group) => groups.push(group),
                None => supported = false,
            }
        }
    }
    if validate_groups(&groups).is_err() {
        // A syntactically documented but over-budget file is still not a
        // supported projection. Do not turn the bounded IPC DTO into a second
        // 2 MiB document transport.
        groups.clear();
        supported = false;
    }
    (groups, supported)
}

fn project_group(event: QoderHookEvent, value: &Value) -> Option<QoderHookGroup> {
    let object = value.as_object()?;
    if object.keys().any(|key| key != "matcher" && key != "hooks") {
        return None;
    }
    let matcher = match object.get("matcher") {
        Some(Value::String(value)) => Some(value.clone()),
        Some(_) => return None,
        None => None,
    };
    let hooks = object.get("hooks")?.as_array()?;
    let commands = hooks
        .iter()
        .map(project_command)
        .collect::<Option<Vec<_>>>()?;
    Some(QoderHookGroup {
        event,
        matcher,
        hooks: commands,
    })
}

fn project_command(value: &Value) -> Option<QoderHookCommand> {
    let object = value.as_object()?;
    if object
        .keys()
        .any(|key| key != "type" && key != "command" && key != "timeout")
    {
        return None;
    }
    if object.get("type")?.as_str()? != "command" {
        return None;
    }
    let command = object.get("command")?.as_str()?.to_string();
    let timeout = match object.get("timeout") {
        Some(value) => Some(u16::try_from(value.as_u64()?).ok()?),
        None => None,
    };
    let command = QoderHookCommand {
        kind: QoderHookType::Command,
        command,
        timeout,
    };
    validate_command(&command).ok()?;
    Some(command)
}

fn validate_groups(groups: &[QoderHookGroup]) -> Result<(), QoderHooksError> {
    if groups.len() > MAX_GROUPS {
        return Err(invalid_request());
    }
    let mut total_commands = 0usize;
    for group in groups {
        if group.hooks.len() > MAX_COMMANDS_PER_GROUP {
            return Err(invalid_request());
        }
        if group
            .matcher
            .as_ref()
            .is_some_and(|value| value.len() > MAX_MATCHER_BYTES || value.contains('\0'))
        {
            return Err(invalid_request());
        }
        total_commands = total_commands
            .checked_add(group.hooks.len())
            .ok_or_else(invalid_request)?;
        if total_commands > MAX_TOTAL_COMMANDS {
            return Err(invalid_request());
        }
        for command in &group.hooks {
            validate_command(command)?;
        }
    }
    Ok(())
}

fn validate_command(command: &QoderHookCommand) -> Result<(), QoderHooksError> {
    if command.command.trim().is_empty()
        || command.command.len() > MAX_COMMAND_BYTES
        || command.command.contains('\0')
        || command
            .timeout
            .is_some_and(|timeout| !(MIN_TIMEOUT_SECONDS..=MAX_TIMEOUT_SECONDS).contains(&timeout))
    {
        return Err(invalid_request());
    }
    Ok(())
}

fn invalid_request() -> QoderHooksError {
    QoderHooksError::new(QoderHooksErrorCode::InvalidRequest)
}

fn groups_to_document_hooks(groups: &[QoderHookGroup]) -> Map<String, Value> {
    let mut hooks = Map::<String, Value>::new();
    for group in groups {
        let event_groups = hooks
            .entry(group.event.as_str().to_string())
            .or_insert_with(|| Value::Array(Vec::new()))
            .as_array_mut()
            .expect("the service creates event arrays itself");
        let mut object = Map::new();
        if let Some(matcher) = &group.matcher {
            object.insert("matcher".to_string(), Value::String(matcher.clone()));
        }
        object.insert(
            "hooks".to_string(),
            Value::Array(
                group
                    .hooks
                    .iter()
                    .map(|hook| {
                        let mut object = Map::new();
                        object.insert("type".to_string(), Value::String("command".to_string()));
                        object.insert("command".to_string(), Value::String(hook.command.clone()));
                        if let Some(timeout) = hook.timeout {
                            object.insert("timeout".to_string(), Value::Number(timeout.into()));
                        }
                        Value::Object(object)
                    })
                    .collect(),
            ),
        );
        event_groups.push(Value::Object(object));
    }
    hooks
}

fn is_destructive_change(existing: &[QoderHookGroup], next: &[QoderHookGroup]) -> bool {
    let mut used = vec![false; next.len()];
    existing.iter().any(|old_group| {
        match next
            .iter()
            .enumerate()
            .find(|(index, candidate)| !used[*index] && *candidate == old_group)
        {
            Some((index, _)) => {
                used[index] = true;
                false
            }
            None => true,
        }
    })
}

fn issue_overwrite_token(
    state: &QoderHooksState,
    paths: &QoderPaths,
    request: &SaveQoderworkHooksRequest,
) -> String {
    let token = new_opaque_token();
    let pending = PendingOverwrite {
        request_digest: request_digest(&state.request_key, paths, request),
        expected_revision: request.expected_revision.clone(),
        expires_at: Instant::now() + OVERWRITE_TOKEN_TTL,
    };
    let now = Instant::now();
    let mut entries = state.pending();
    entries.retain(|_, pending| pending.expires_at + EXPIRED_TOKEN_RETENTION > now);
    while entries.len() >= MAX_PENDING_OVERWRITES {
        let Some(oldest) = entries
            .iter()
            .min_by_key(|(_, pending)| pending.expires_at)
            .map(|(token, _)| token.clone())
        else {
            break;
        };
        entries.remove(&oldest);
    }
    entries.insert(token.clone(), pending);
    token
}

fn consume_overwrite_token(
    state: &QoderHooksState,
    token: &str,
    paths: &QoderPaths,
    request: &SaveQoderworkHooksRequest,
) -> Result<PendingOverwrite, QoderHooksError> {
    let pending = state
        .pending()
        .remove(token)
        .ok_or_else(|| QoderHooksError::new(QoderHooksErrorCode::OverwriteTokenInvalid))?;
    if pending.expires_at <= Instant::now() {
        return Err(QoderHooksError::new(
            QoderHooksErrorCode::OverwriteTokenExpired,
        ));
    }
    let actual = request_digest(&state.request_key, paths, request);
    if !constant_time_equals(&pending.request_digest, &actual) {
        return Err(QoderHooksError::new(
            QoderHooksErrorCode::OverwriteTokenInvalid,
        ));
    }
    Ok(pending)
}

fn request_digest(
    key: &[u8; 32],
    paths: &QoderPaths,
    request: &SaveQoderworkHooksRequest,
) -> [u8; 32] {
    let mut mac = HmacSha256::new_from_slice(key).expect("fixed HMAC key is valid");
    mac.update(b"fyagent-qoder-hooks-overwrite-v1");
    update_length_prefixed(&mut mac, path_digest_bytes(&paths.settings).as_bytes());
    update_optional_string(&mut mac, request.expected_revision.as_deref());
    let groups =
        serde_json::to_vec(&request.groups).expect("validated Qoder Hooks DTOs are serializable");
    update_length_prefixed(&mut mac, &groups);
    mac_bytes_from_mac(mac)
}

fn path_digest_bytes(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn update_length_prefixed(mac: &mut HmacSha256, value: &[u8]) {
    mac.update(&(value.len() as u64).to_be_bytes());
    mac.update(value);
}

fn update_optional_string(mac: &mut HmacSha256, value: Option<&str>) {
    match value {
        Some(value) => {
            mac.update(&[1]);
            update_length_prefixed(mac, value.as_bytes());
        }
        None => mac.update(&[0]),
    }
}

fn new_opaque_token() -> String {
    format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
}

fn random_mac_key() -> [u8; 32] {
    let mut key = [0u8; 32];
    key[..16].copy_from_slice(Uuid::new_v4().as_bytes());
    key[16..].copy_from_slice(Uuid::new_v4().as_bytes());
    key
}

fn revision_for(key: &[u8; 32], bytes: &[u8]) -> String {
    mac_bytes(key, bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn mac_bytes(key: &[u8; 32], bytes: &[u8]) -> [u8; 32] {
    let mut mac = HmacSha256::new_from_slice(key).expect("fixed HMAC key is valid");
    mac.update(bytes);
    mac_bytes_from_mac(mac)
}

fn mac_bytes_from_mac(mac: HmacSha256) -> [u8; 32] {
    let bytes = mac.finalize().into_bytes();
    let mut output = [0u8; 32];
    output.copy_from_slice(&bytes);
    output
}

fn constant_time_equals(left: &[u8; 32], right: &[u8; 32]) -> bool {
    left.iter()
        .zip(right)
        .fold(0u8, |difference, (left, right)| difference | (left ^ right))
        == 0
}

fn read_settings(paths: &QoderPaths) -> Result<Option<Vec<u8>>, QoderHooksError> {
    let mut storage = match FixedDocumentStorage::open(paths, false) {
        Ok(storage) => storage,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(map_read_error(error)),
    };
    storage.read_settings().map_err(map_read_error)
}

fn map_read_error(error: io::Error) -> QoderHooksError {
    QoderHooksError::new(if error.kind() == io::ErrorKind::InvalidData {
        QoderHooksErrorCode::DocumentTooLarge
    } else {
        QoderHooksErrorCode::ReadFailed
    })
}

fn commit_document(
    paths: &QoderPaths,
    loaded: &LoadedDocument,
    replacement: &[u8],
) -> Result<bool, QoderHooksError> {
    let mut storage = FixedDocumentStorage::open(paths, true)
        .map_err(|_| QoderHooksError::new(QoderHooksErrorCode::WriteFailed))?;
    let mut snapshot = storage.snapshot_settings().map_err(map_read_error)?;
    let expected = loaded.exists.then_some(loaded.original_bytes.as_slice());
    if snapshot.bytes() != expected {
        return Ok(false);
    }
    match storage.commit(&mut snapshot, replacement) {
        Ok(()) => Ok(true),
        Err(CommitError::Concurrent) => Ok(false),
        Err(CommitError::Backup) => Err(QoderHooksError::new(QoderHooksErrorCode::BackupFailed)),
        Err(CommitError::Primary) => Err(QoderHooksError::new(QoderHooksErrorCode::WriteFailed)),
    }
}

#[derive(Debug)]
struct FileSnapshot {
    bytes: Option<Vec<u8>>,
    identity: Option<FileIdentity>,
    #[cfg(target_os = "windows")]
    leaf: Option<fs::File>,
}

impl FileSnapshot {
    fn bytes(&self) -> Option<&[u8]> {
        self.bytes.as_deref()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FileIdentity {
    primary: u64,
    secondary: u64,
    size: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CommitError {
    Concurrent,
    Backup,
    Primary,
}

/// Fixed-path storage used only after `QoderPaths::validate_fixed_shape`.
/// Ancestors and leaves are revalidated before every side effect. Windows
/// opens every component with no-follow semantics and holds the handles for
/// the operation; Unix rejects symlinks and hard-linked credential leaves.
struct FixedDocumentStorage {
    paths: QoderPaths,
    #[cfg(target_os = "windows")]
    held_directories: Vec<fs::File>,
    #[cfg(target_os = "windows")]
    production_context: bool,
}

impl FixedDocumentStorage {
    fn open(paths: &QoderPaths, create_directory: bool) -> io::Result<Self> {
        paths.validate_fixed_shape().map_err(|_| storage_error())?;

        #[cfg(target_os = "windows")]
        let production_context = windows_production_context_for(&paths.home)?;
        #[cfg(target_os = "windows")]
        revalidate_windows_production_context(production_context)?;
        #[cfg(target_os = "windows")]
        let held_directories =
            open_windows_directory_chain(paths, create_directory, production_context)?;

        #[cfg(target_os = "macos")]
        {
            validate_unix_directory(&paths.home)?;
            match fs::symlink_metadata(&paths.directory) {
                Ok(metadata) => validate_unix_directory_metadata(&metadata)?,
                Err(error) if error.kind() == io::ErrorKind::NotFound && create_directory => {
                    fs::create_dir(&paths.directory)?;
                    validate_unix_directory_metadata(&fs::symlink_metadata(&paths.directory)?)?;
                }
                Err(error) if error.kind() == io::ErrorKind::NotFound => return Err(error),
                Err(error) => return Err(error),
            }
        }

        Ok(Self {
            paths: paths.clone(),
            #[cfg(target_os = "windows")]
            held_directories,
            #[cfg(target_os = "windows")]
            production_context,
        })
    }

    fn read_settings(&mut self) -> io::Result<Option<Vec<u8>>> {
        self.recheck()?;
        #[cfg(target_os = "windows")]
        {
            let mut leaf = match self.open_windows_leaf(SETTINGS_FILE_NAME)? {
                Some(leaf) => leaf,
                None => return Ok(None),
            };
            let bytes = read_bounded_file(&mut leaf)?;
            self.recheck()?;
            Ok(Some(bytes))
        }
        #[cfg(target_os = "macos")]
        read_bounded_regular_file(&self.paths.settings)
    }

    fn snapshot_settings(&mut self) -> io::Result<FileSnapshot> {
        self.recheck()?;
        #[cfg(target_os = "windows")]
        {
            let Some(mut leaf) = self.open_windows_leaf(SETTINGS_FILE_NAME)? else {
                return Ok(FileSnapshot {
                    bytes: None,
                    identity: None,
                    leaf: None,
                });
            };
            let identity = file_identity(&leaf)?;
            let bytes = read_bounded_file(&mut leaf)?;
            self.recheck()?;
            Ok(FileSnapshot {
                bytes: Some(bytes),
                identity: Some(identity),
                leaf: Some(leaf),
            })
        }
        #[cfg(target_os = "macos")]
        {
            let bytes = read_bounded_regular_file(&self.paths.settings)?;
            let identity = match bytes {
                Some(_) => Some(file_identity_at(&self.paths.settings)?),
                None => None,
            };
            self.recheck()?;
            Ok(FileSnapshot { bytes, identity })
        }
    }

    fn snapshot_matches(&mut self, snapshot: &FileSnapshot) -> io::Result<bool> {
        self.recheck()?;
        #[cfg(target_os = "windows")]
        {
            let matches = match (&snapshot.leaf, snapshot.bytes()) {
                (Some(held), Some(expected)) => {
                    if file_identity(held)? != snapshot.identity.ok_or_else(storage_error)? {
                        return Err(storage_error());
                    }
                    let Some(namespace_leaf) = self.open_windows_leaf(SETTINGS_FILE_NAME)? else {
                        return Ok(false);
                    };
                    if file_identity(&namespace_leaf)? != file_identity(held)? {
                        return Ok(false);
                    }
                    let mut held = held.try_clone()?;
                    read_bounded_file(&mut held)? == expected
                }
                (None, None) => self.open_windows_leaf(SETTINGS_FILE_NAME)?.is_none(),
                _ => false,
            };
            self.recheck()?;
            Ok(matches)
        }
        #[cfg(target_os = "macos")]
        {
            let current = read_bounded_regular_file(&self.paths.settings)?;
            let identity = match current {
                Some(_) => Some(file_identity_at(&self.paths.settings)?),
                None => None,
            };
            self.recheck()?;
            Ok(current.as_deref() == snapshot.bytes() && identity == snapshot.identity)
        }
    }

    fn commit(
        &mut self,
        snapshot: &mut FileSnapshot,
        replacement: &[u8],
    ) -> Result<(), CommitError> {
        if !self
            .snapshot_matches(snapshot)
            .map_err(|_| CommitError::Concurrent)?
        {
            return Err(CommitError::Concurrent);
        }
        if let Some(original) = snapshot.bytes() {
            #[cfg(target_os = "windows")]
            self.write_windows_named_atomically(
                BACKUP_FILE_NAME,
                original,
                WindowsTargetExpectation::Discover,
            )
            .map_err(|_| CommitError::Backup)?;
            #[cfg(target_os = "macos")]
            write_same_directory_atomically(&self.paths.backup, original)
                .map_err(|_| CommitError::Backup)?;
        }
        self.recheck().map_err(|_| CommitError::Primary)?;
        if !self
            .snapshot_matches(snapshot)
            .map_err(|_| CommitError::Primary)?
        {
            return Err(CommitError::Concurrent);
        }
        #[cfg(target_os = "windows")]
        {
            let expectation = match snapshot.leaf.as_ref() {
                Some(leaf) => WindowsTargetExpectation::Existing(leaf),
                None => WindowsTargetExpectation::Missing,
            };
            self.write_windows_named_atomically(SETTINGS_FILE_NAME, replacement, expectation)
                .map_err(|_| CommitError::Primary)
        }
        #[cfg(target_os = "macos")]
        write_same_directory_atomically(&self.paths.settings, replacement)
            .map_err(|_| CommitError::Primary)
    }

    fn recheck(&self) -> io::Result<()> {
        #[cfg(target_os = "windows")]
        return recheck_windows_directory_chain(&self.paths, &self.held_directories);

        #[cfg(target_os = "macos")]
        {
            validate_unix_directory(&self.paths.home)?;
            validate_unix_directory(&self.paths.directory)
        }
    }

    #[cfg(target_os = "windows")]
    fn windows_directory(&self) -> io::Result<&fs::File> {
        self.held_directories.last().ok_or_else(storage_error)
    }

    #[cfg(target_os = "windows")]
    fn open_windows_leaf(&self, name: &str) -> io::Result<Option<fs::File>> {
        use windows::{
            Wdk::Storage::FileSystem::{
                FILE_NON_DIRECTORY_FILE, FILE_OPEN_REPARSE_POINT, FILE_SYNCHRONOUS_IO_NONALERT,
            },
            Win32::Storage::FileSystem::{
                FILE_ATTRIBUTE_NORMAL, FILE_GENERIC_READ, FILE_READ_ATTRIBUTES, FILE_SHARE_DELETE,
                FILE_SHARE_READ, SYNCHRONIZE,
            },
        };
        self.recheck()?;
        let file = match windows_open_relative(
            self.windows_directory()?,
            std::ffi::OsStr::new(name),
            (FILE_GENERIC_READ | FILE_READ_ATTRIBUTES | SYNCHRONIZE).0,
            (FILE_SHARE_READ | FILE_SHARE_DELETE).0,
            WindowsRelativeDisposition::Open,
            FILE_ATTRIBUTE_NORMAL.0,
            (FILE_NON_DIRECTORY_FILE | FILE_OPEN_REPARSE_POINT | FILE_SYNCHRONOUS_IO_NONALERT).0,
        ) {
            Ok(file) => file,
            Err(WindowsRelativeOpenError::NotFound) => return Ok(None),
            Err(WindowsRelativeOpenError::Rejected) => return Err(storage_error()),
        };
        file_identity(&file)?;
        self.recheck()?;
        Ok(Some(file))
    }

    #[cfg(target_os = "windows")]
    fn write_windows_named_atomically(
        &self,
        target_name: &str,
        data: &[u8],
        expectation: WindowsTargetExpectation<'_>,
    ) -> io::Result<()> {
        use windows::{
            Wdk::Storage::FileSystem::{
                FILE_NON_DIRECTORY_FILE, FILE_OPEN_REPARSE_POINT, FILE_SYNCHRONOUS_IO_NONALERT,
            },
            Win32::Storage::FileSystem::{
                FILE_ATTRIBUTE_NORMAL, FILE_GENERIC_READ, FILE_GENERIC_WRITE, FILE_READ_ATTRIBUTES,
                SYNCHRONIZE, WRITE_DAC,
            },
        };

        let discovered;
        let existing = match expectation {
            WindowsTargetExpectation::Discover => {
                discovered = self.open_windows_leaf(target_name)?;
                discovered.as_ref()
            }
            WindowsTargetExpectation::Existing(file) => Some(file),
            WindowsTargetExpectation::Missing => {
                if self.open_windows_leaf(target_name)?.is_some() {
                    return Err(storage_error());
                }
                None
            }
        };
        self.recheck()?;

        let temp_name = format!(".{target_name}.tmp.{}", Uuid::new_v4());
        let mut temp = windows_open_relative(
            self.windows_directory()?,
            std::ffi::OsStr::new(&temp_name),
            (FILE_GENERIC_READ
                | FILE_GENERIC_WRITE
                | FILE_READ_ATTRIBUTES
                | windows::Win32::Storage::FileSystem::DELETE
                | SYNCHRONIZE
                | WRITE_DAC)
                .0,
            0,
            WindowsRelativeDisposition::Create,
            FILE_ATTRIBUTE_NORMAL.0,
            (FILE_NON_DIRECTORY_FILE | FILE_OPEN_REPARSE_POINT | FILE_SYNCHRONOUS_IO_NONALERT).0,
        )
        .map_err(|_| storage_error())?;
        file_identity(&temp)?;

        let result = (|| {
            temp.seek(SeekFrom::Start(0))?;
            temp.write_all(data)?;
            temp.flush()?;
            temp.sync_all()?;
            file_identity(&temp)?;
            self.recheck()?;
            if let Some(existing) = existing {
                copy_windows_dacl(existing, &temp)?;
                let Some(current) = self.open_windows_leaf(target_name)? else {
                    return Err(storage_error());
                };
                if file_identity(&current)? != file_identity(existing)? {
                    return Err(storage_error());
                }
            } else if self.open_windows_leaf(target_name)?.is_some() {
                return Err(storage_error());
            }
            revalidate_windows_production_context(self.production_context)?;
            windows_rename_by_handle(
                self.windows_directory()?,
                &temp,
                target_name,
                existing.is_some(),
            )
        })();
        if result.is_err() {
            let _ = windows_mark_delete_by_handle(&temp);
        }
        result
    }
}

#[cfg(target_os = "windows")]
enum WindowsTargetExpectation<'a> {
    Discover,
    Existing(&'a fs::File),
    Missing,
}

#[cfg(target_os = "windows")]
#[derive(Clone, Copy)]
enum WindowsRelativeDisposition {
    Open,
    Create,
    OpenIf,
}

#[cfg(target_os = "windows")]
enum WindowsRelativeOpenError {
    NotFound,
    Rejected,
}

#[cfg(target_os = "windows")]
#[allow(clippy::too_many_arguments)]
fn windows_open_relative(
    parent: &fs::File,
    name: &std::ffi::OsStr,
    desired_access: u32,
    share_access: u32,
    disposition: WindowsRelativeDisposition,
    attributes: u32,
    create_options: u32,
) -> Result<fs::File, WindowsRelativeOpenError> {
    use std::{mem::size_of, os::windows::io::FromRawHandle};
    use std::{os::windows::ffi::OsStrExt, os::windows::io::AsRawHandle};
    use windows::{
        core::PWSTR,
        Wdk::{
            Foundation::OBJECT_ATTRIBUTES,
            Storage::FileSystem::{NtCreateFile, FILE_CREATE, FILE_OPEN, FILE_OPEN_IF},
        },
        Win32::{
            Foundation::{
                HANDLE, INVALID_HANDLE_VALUE, OBJ_CASE_INSENSITIVE, OBJ_DONT_REPARSE,
                STATUS_NO_SUCH_FILE, STATUS_OBJECT_NAME_NOT_FOUND, STATUS_OBJECT_PATH_NOT_FOUND,
                UNICODE_STRING,
            },
            Storage::FileSystem::{FILE_ACCESS_RIGHTS, FILE_FLAGS_AND_ATTRIBUTES, FILE_SHARE_MODE},
            System::IO::IO_STATUS_BLOCK,
        },
    };

    let path = Path::new(name);
    let mut components = path.components();
    if !matches!(components.next(), Some(Component::Normal(component)) if component == name)
        || components.next().is_some()
    {
        return Err(WindowsRelativeOpenError::Rejected);
    }
    let mut wide = name.encode_wide().collect::<Vec<_>>();
    if wide.is_empty() || wide.contains(&0) {
        return Err(WindowsRelativeOpenError::Rejected);
    }
    let byte_length = wide
        .len()
        .checked_mul(size_of::<u16>())
        .and_then(|length| u16::try_from(length).ok())
        .ok_or(WindowsRelativeOpenError::Rejected)?;
    let object_name = UNICODE_STRING {
        Length: byte_length,
        MaximumLength: byte_length,
        Buffer: PWSTR(wide.as_mut_ptr()),
    };
    let object_attributes = OBJECT_ATTRIBUTES {
        Length: size_of::<OBJECT_ATTRIBUTES>() as u32,
        RootDirectory: HANDLE(parent.as_raw_handle()),
        ObjectName: &object_name,
        Attributes: OBJ_CASE_INSENSITIVE | OBJ_DONT_REPARSE,
        SecurityDescriptor: std::ptr::null(),
        SecurityQualityOfService: std::ptr::null(),
    };
    let disposition = match disposition {
        WindowsRelativeDisposition::Open => FILE_OPEN,
        WindowsRelativeDisposition::Create => FILE_CREATE,
        WindowsRelativeDisposition::OpenIf => FILE_OPEN_IF,
    };
    let mut handle = HANDLE::default();
    let mut io_status = IO_STATUS_BLOCK::default();
    let status = unsafe {
        NtCreateFile(
            &mut handle,
            FILE_ACCESS_RIGHTS(desired_access),
            &object_attributes,
            &mut io_status,
            None,
            FILE_FLAGS_AND_ATTRIBUTES(attributes),
            FILE_SHARE_MODE(share_access),
            disposition,
            windows::Wdk::Storage::FileSystem::NTCREATEFILE_CREATE_OPTIONS(create_options),
            None,
            0,
        )
    };
    if status.is_err() {
        return if matches!(
            status,
            STATUS_NO_SUCH_FILE | STATUS_OBJECT_NAME_NOT_FOUND | STATUS_OBJECT_PATH_NOT_FOUND
        ) {
            Err(WindowsRelativeOpenError::NotFound)
        } else {
            Err(WindowsRelativeOpenError::Rejected)
        };
    }
    if handle.0.is_null() || handle == INVALID_HANDLE_VALUE {
        return Err(WindowsRelativeOpenError::Rejected);
    }
    Ok(unsafe { fs::File::from_raw_handle(handle.0) })
}

#[cfg(target_os = "windows")]
fn windows_rename_by_handle(
    directory: &fs::File,
    temp: &fs::File,
    final_name: &str,
    target_existed: bool,
) -> io::Result<()> {
    use std::{mem::size_of, os::windows::ffi::OsStrExt, os::windows::io::AsRawHandle};
    use windows::{
        Wdk::Storage::FileSystem::{
            FileRenameInformationEx, NtSetInformationFile, FILE_RENAME_INFORMATION,
            FILE_RENAME_POSIX_SEMANTICS, FILE_RENAME_REPLACE_IF_EXISTS,
        },
        Win32::{Foundation::HANDLE, System::IO::IO_STATUS_BLOCK},
    };

    validate_windows_directory_handle(directory)?;
    file_identity(temp)?;
    let wide = std::ffi::OsStr::new(final_name)
        .encode_wide()
        .collect::<Vec<_>>();
    if wide.is_empty() || wide.contains(&0) {
        return Err(storage_error());
    }
    let name_bytes = wide
        .len()
        .checked_mul(size_of::<u16>())
        .ok_or_else(storage_error)?;
    let buffer_size = size_of::<FILE_RENAME_INFORMATION>()
        .checked_add(name_bytes)
        .ok_or_else(storage_error)?;
    let mut storage = vec![0usize; buffer_size.div_ceil(size_of::<usize>())];
    let information = storage.as_mut_ptr().cast::<FILE_RENAME_INFORMATION>();
    let mut io_status = IO_STATUS_BLOCK::default();
    let status = unsafe {
        (*information).Anonymous.Flags = if target_existed {
            FILE_RENAME_REPLACE_IF_EXISTS | FILE_RENAME_POSIX_SEMANTICS
        } else {
            0
        };
        (*information).RootDirectory = HANDLE(directory.as_raw_handle());
        (*information).FileNameLength = u32::try_from(name_bytes).map_err(|_| storage_error())?;
        std::ptr::copy_nonoverlapping(
            wide.as_ptr(),
            (*information).FileName.as_mut_ptr(),
            wide.len(),
        );
        NtSetInformationFile(
            HANDLE(temp.as_raw_handle()),
            &mut io_status,
            information.cast(),
            u32::try_from(buffer_size).map_err(|_| storage_error())?,
            FileRenameInformationEx,
        )
    };
    if status.is_err() {
        return Err(storage_error());
    }
    Ok(())
}

#[cfg(target_os = "windows")]
struct OwnedWindowsSecurityDescriptor(windows::Win32::Security::PSECURITY_DESCRIPTOR);

#[cfg(target_os = "windows")]
impl Drop for OwnedWindowsSecurityDescriptor {
    fn drop(&mut self) {
        use windows::Win32::Foundation::{LocalFree, HLOCAL};
        if !self.0 .0.is_null() {
            unsafe {
                let _ = LocalFree(Some(HLOCAL(self.0 .0)));
            }
        }
    }
}

#[cfg(target_os = "windows")]
fn copy_windows_dacl(source: &fs::File, destination: &fs::File) -> io::Result<()> {
    use std::os::windows::io::AsRawHandle;
    use windows::Win32::{
        Foundation::HANDLE,
        Security::{
            Authorization::{GetSecurityInfo, SetSecurityInfo, SE_FILE_OBJECT},
            GetSecurityDescriptorControl, DACL_SECURITY_INFORMATION,
            PROTECTED_DACL_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR, SE_DACL_PROTECTED,
            UNPROTECTED_DACL_SECURITY_INFORMATION,
        },
    };
    let mut dacl = std::ptr::null_mut();
    let mut descriptor = PSECURITY_DESCRIPTOR::default();
    let status = unsafe {
        GetSecurityInfo(
            HANDLE(source.as_raw_handle()),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION,
            None,
            None,
            Some(&mut dacl),
            None,
            Some(&mut descriptor),
        )
    };
    if status.0 != 0 || descriptor.0.is_null() {
        return Err(storage_error());
    }
    let descriptor = OwnedWindowsSecurityDescriptor(descriptor);
    let mut control = 0u16;
    let mut revision = 0u32;
    unsafe { GetSecurityDescriptorControl(descriptor.0, &mut control, &mut revision) }
        .map_err(|_| storage_error())?;
    let protection = if control & SE_DACL_PROTECTED.0 != 0 {
        PROTECTED_DACL_SECURITY_INFORMATION
    } else {
        UNPROTECTED_DACL_SECURITY_INFORMATION
    };
    let status = unsafe {
        SetSecurityInfo(
            HANDLE(destination.as_raw_handle()),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION | protection,
            None,
            None,
            Some(dacl),
            None,
        )
    };
    if status.0 != 0 {
        return Err(storage_error());
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn windows_mark_delete_by_handle(file: &fs::File) -> io::Result<()> {
    use std::{mem::size_of, os::windows::io::AsRawHandle};
    use windows::Win32::{
        Foundation::HANDLE,
        Storage::FileSystem::{
            FileDispositionInfo, SetFileInformationByHandle, FILE_DISPOSITION_INFO,
        },
    };
    let information = FILE_DISPOSITION_INFO { DeleteFile: true };
    unsafe {
        SetFileInformationByHandle(
            HANDLE(file.as_raw_handle()),
            FileDispositionInfo,
            (&raw const information).cast(),
            size_of::<FILE_DISPOSITION_INFO>() as u32,
        )
    }
    .map_err(|_| storage_error())
}

#[cfg(target_os = "windows")]
fn read_bounded_file(file: &mut fs::File) -> io::Result<Vec<u8>> {
    let identity = file_identity(file)?;
    if identity.size > MAX_DOCUMENT_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Qoder settings exceeds the supported size",
        ));
    }
    file.seek(SeekFrom::Start(0))?;
    let mut bytes = Vec::with_capacity(identity.size as usize);
    Read::by_ref(file)
        .take(MAX_DOCUMENT_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 != identity.size || file_identity(file)? != identity {
        return Err(storage_error());
    }
    Ok(bytes)
}

#[cfg(target_os = "macos")]
fn read_bounded_regular_file(path: &Path) -> io::Result<Option<Vec<u8>>> {
    let mut file = match open_regular_file_read(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    let identity = file_identity(&file)?;
    if identity.size > MAX_DOCUMENT_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Qoder settings exceeds the supported size",
        ));
    }
    file.seek(SeekFrom::Start(0))?;
    let mut bytes = Vec::with_capacity(identity.size as usize);
    Read::by_ref(&mut file)
        .take(MAX_DOCUMENT_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 != identity.size || bytes.len() as u64 > MAX_DOCUMENT_BYTES {
        return Err(storage_error());
    }
    if file_identity(&file)? != identity {
        return Err(storage_error());
    }
    Ok(Some(bytes))
}

#[cfg(target_os = "macos")]
fn write_same_directory_atomically(path: &Path, data: &[u8]) -> io::Result<()> {
    let parent = path.parent().ok_or_else(storage_error)?;
    let file_name = path
        .file_name()
        .ok_or_else(storage_error)?
        .to_string_lossy();
    for _ in 0..8 {
        let temp = parent.join(format!(".{file_name}.tmp.{}", Uuid::new_v4()));
        let mut file = match create_new_private_file(&temp) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        };
        let result = (|| {
            file.write_all(data)?;
            file.flush()?;
            file.sync_all()?;
            set_private_permissions(&temp)?;
            validate_existing_target_for_replace(path)?;
            replace_file_without_delete_gap(&temp, path)?;
            sync_parent(parent)
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temp);
        }
        return result;
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not create a unique Qoder settings temporary file",
    ))
}

#[cfg(target_os = "macos")]
fn validate_existing_target_for_replace(path: &Path) -> io::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => validate_regular_metadata(&metadata),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

#[cfg(target_os = "macos")]
fn open_regular_file_read(path: &Path) -> io::Result<fs::File> {
    let metadata = fs::symlink_metadata(path)?;
    validate_regular_metadata(&metadata)?;
    let file = fs::OpenOptions::new().read(true).open(path)?;
    if file_identity(&file)? != file_identity_from_metadata(&metadata)? {
        return Err(storage_error());
    }
    Ok(file)
}

#[cfg(target_os = "macos")]
fn create_new_private_file(path: &Path) -> io::Result<fs::File> {
    use std::os::unix::fs::OpenOptionsExt;
    fs::OpenOptions::new()
        .write(true)
        .read(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
}

#[cfg(target_os = "macos")]
fn set_private_permissions(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
}

#[cfg(target_os = "macos")]
fn replace_file_without_delete_gap(temp: &Path, target: &Path) -> io::Result<()> {
    fs::rename(temp, target)
}

#[cfg(target_os = "macos")]
fn sync_parent(parent: &Path) -> io::Result<()> {
    fs::File::open(parent)?.sync_all()
}

#[cfg(target_os = "macos")]
fn validate_unix_directory(path: &Path) -> io::Result<()> {
    validate_unix_directory_metadata(&fs::symlink_metadata(path)?)
}

#[cfg(target_os = "macos")]
fn validate_unix_directory_metadata(metadata: &fs::Metadata) -> io::Result<()> {
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(storage_error());
    }
    Ok(())
}

fn validate_regular_metadata(metadata: &fs::Metadata) -> io::Result<()> {
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(storage_error());
    }
    #[cfg(target_os = "macos")]
    {
        use std::os::unix::fs::MetadataExt;
        if metadata.nlink() != 1 {
            return Err(storage_error());
        }
    }
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::fs::MetadataExt;
        use windows::Win32::Storage::FileSystem::{
            FILE_ATTRIBUTE_OFFLINE, FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS,
            FILE_ATTRIBUTE_RECALL_ON_OPEN, FILE_ATTRIBUTE_REPARSE_POINT,
        };
        let unsafe_attributes = FILE_ATTRIBUTE_REPARSE_POINT.0
            | FILE_ATTRIBUTE_OFFLINE.0
            | FILE_ATTRIBUTE_RECALL_ON_OPEN.0
            | FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS.0;
        if metadata.file_attributes() & unsafe_attributes != 0 {
            return Err(storage_error());
        }
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn file_identity(file: &fs::File) -> io::Result<FileIdentity> {
    file_identity_from_metadata(&file.metadata()?)
}

#[cfg(target_os = "macos")]
fn file_identity_at(path: &Path) -> io::Result<FileIdentity> {
    file_identity_from_metadata(&fs::symlink_metadata(path)?)
}

#[cfg(target_os = "macos")]
fn file_identity_from_metadata(metadata: &fs::Metadata) -> io::Result<FileIdentity> {
    use std::os::unix::fs::MetadataExt;
    validate_regular_metadata(metadata)?;
    Ok(FileIdentity {
        primary: metadata.dev(),
        secondary: metadata.ino(),
        size: metadata.size(),
    })
}

#[cfg(target_os = "windows")]
fn file_identity(file: &fs::File) -> io::Result<FileIdentity> {
    use std::os::windows::io::AsRawHandle;
    use windows::Win32::{
        Foundation::HANDLE,
        Storage::FileSystem::{GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION},
    };
    let mut information = BY_HANDLE_FILE_INFORMATION::default();
    unsafe { GetFileInformationByHandle(HANDLE(file.as_raw_handle()), &mut information) }
        .map_err(|_| storage_error())?;
    let metadata = file.metadata()?;
    validate_regular_metadata(&metadata)?;
    if information.nNumberOfLinks != 1 {
        return Err(storage_error());
    }
    Ok(FileIdentity {
        primary: u64::from(information.dwVolumeSerialNumber),
        secondary: (u64::from(information.nFileIndexHigh) << 32)
            | u64::from(information.nFileIndexLow),
        size: (u64::from(information.nFileSizeHigh) << 32) | u64::from(information.nFileSizeLow),
    })
}

#[cfg(target_os = "windows")]
fn open_windows_directory_chain(
    paths: &QoderPaths,
    create_directory: bool,
    production_context: bool,
) -> io::Result<Vec<fs::File>> {
    use std::os::windows::fs::OpenOptionsExt;
    use windows::Win32::Storage::FileSystem::{
        FILE_ADD_FILE, FILE_ADD_SUBDIRECTORY, FILE_ATTRIBUTE_DIRECTORY, FILE_DELETE_CHILD,
        FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_GENERIC_READ,
        FILE_READ_ATTRIBUTES, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, FILE_TRAVERSE,
        SYNCHRONIZE,
    };

    let mut components = paths.home.components();
    let prefix = match components.next() {
        Some(Component::Prefix(prefix)) => prefix.as_os_str(),
        _ => return Err(storage_error()),
    };
    if !matches!(components.next(), Some(Component::RootDir)) {
        return Err(storage_error());
    }
    let mut current = PathBuf::from(prefix);
    current.push(Path::new(r"\"));
    let root = fs::OpenOptions::new()
        .access_mode((FILE_GENERIC_READ | FILE_TRAVERSE | FILE_READ_ATTRIBUTES | SYNCHRONIZE).0)
        .share_mode((FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE).0)
        .custom_flags((FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT).0)
        .open(&current)
        .map_err(|_| storage_error())?;
    validate_windows_directory_handle(&root)?;
    let volume = directory_identity(&root)?.0;
    let mut held = vec![root];
    let components = components.collect::<Vec<_>>();
    let last_index = components.len().checked_sub(1).ok_or_else(storage_error)?;
    for (index, component) in components.into_iter().enumerate() {
        let Component::Normal(name) = component else {
            return Err(storage_error());
        };
        let parent = held.last().ok_or_else(storage_error)?;
        validate_windows_directory_handle(parent)?;
        let mut desired_access =
            (FILE_GENERIC_READ | FILE_TRAVERSE | FILE_READ_ATTRIBUTES | SYNCHRONIZE).0;
        if index == last_index {
            desired_access |= FILE_ADD_SUBDIRECTORY.0;
        }
        let next = windows_open_relative(
            parent,
            name,
            desired_access,
            (FILE_SHARE_READ | FILE_SHARE_WRITE).0,
            WindowsRelativeDisposition::Open,
            FILE_ATTRIBUTE_DIRECTORY.0,
            (windows::Wdk::Storage::FileSystem::FILE_DIRECTORY_FILE
                | windows::Wdk::Storage::FileSystem::FILE_OPEN_REPARSE_POINT
                | windows::Wdk::Storage::FileSystem::FILE_SYNCHRONOUS_IO_NONALERT)
                .0,
        )
        .map_err(|_| storage_error())?;
        validate_windows_directory_handle(&next)?;
        if directory_identity(&next)?.0 != volume {
            return Err(storage_error());
        }
        held.push(next);
    }

    let home = held.last().ok_or_else(storage_error)?;
    revalidate_windows_production_context(production_context)?;
    let qoder = windows_open_relative(
        home,
        std::ffi::OsStr::new(".qoderwork"),
        (FILE_GENERIC_READ
            | FILE_TRAVERSE
            | FILE_READ_ATTRIBUTES
            | FILE_ADD_FILE
            | FILE_DELETE_CHILD
            | SYNCHRONIZE)
            .0,
        (FILE_SHARE_READ | FILE_SHARE_WRITE).0,
        if create_directory {
            WindowsRelativeDisposition::OpenIf
        } else {
            WindowsRelativeDisposition::Open
        },
        FILE_ATTRIBUTE_DIRECTORY.0,
        (windows::Wdk::Storage::FileSystem::FILE_DIRECTORY_FILE
            | windows::Wdk::Storage::FileSystem::FILE_OPEN_REPARSE_POINT
            | windows::Wdk::Storage::FileSystem::FILE_SYNCHRONOUS_IO_NONALERT)
            .0,
    )
    .map_err(|error| match error {
        WindowsRelativeOpenError::NotFound if !create_directory => {
            io::Error::from(io::ErrorKind::NotFound)
        }
        WindowsRelativeOpenError::NotFound | WindowsRelativeOpenError::Rejected => storage_error(),
    })?;
    validate_windows_directory_handle(&qoder)?;
    if directory_identity(&qoder)?.0 != volume {
        return Err(storage_error());
    }
    held.push(qoder);
    recheck_windows_directory_chain(paths, &held)?;
    revalidate_windows_production_context(production_context)?;
    Ok(held)
}

#[cfg(target_os = "windows")]
fn validate_windows_directory_handle(file: &fs::File) -> io::Result<()> {
    use std::os::windows::{fs::MetadataExt, io::AsRawHandle};
    use windows::Win32::{
        Foundation::HANDLE,
        Storage::FileSystem::{
            GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION, FILE_ATTRIBUTE_DIRECTORY,
            FILE_ATTRIBUTE_OFFLINE, FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS,
            FILE_ATTRIBUTE_RECALL_ON_OPEN, FILE_ATTRIBUTE_REPARSE_POINT,
        },
    };
    let metadata = file.metadata()?;
    let unsafe_attributes = FILE_ATTRIBUTE_REPARSE_POINT.0
        | FILE_ATTRIBUTE_OFFLINE.0
        | FILE_ATTRIBUTE_RECALL_ON_OPEN.0
        | FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS.0;
    if !metadata.is_dir() || metadata.file_attributes() & unsafe_attributes != 0 {
        return Err(storage_error());
    }
    let mut information = BY_HANDLE_FILE_INFORMATION::default();
    unsafe { GetFileInformationByHandle(HANDLE(file.as_raw_handle()), &mut information) }
        .map_err(|_| storage_error())?;
    if information.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY.0 == 0
        || information.dwFileAttributes & unsafe_attributes != 0
    {
        return Err(storage_error());
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn directory_identity(file: &fs::File) -> io::Result<(u64, u64)> {
    use std::os::windows::io::AsRawHandle;
    use windows::Win32::{
        Foundation::HANDLE,
        Storage::FileSystem::{GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION},
    };
    validate_windows_directory_handle(file)?;
    let mut information = BY_HANDLE_FILE_INFORMATION::default();
    unsafe { GetFileInformationByHandle(HANDLE(file.as_raw_handle()), &mut information) }
        .map_err(|_| storage_error())?;
    Ok((
        u64::from(information.dwVolumeSerialNumber),
        (u64::from(information.nFileIndexHigh) << 32) | u64::from(information.nFileIndexLow),
    ))
}

#[cfg(target_os = "windows")]
fn recheck_windows_directory_chain(paths: &QoderPaths, held: &[fs::File]) -> io::Result<()> {
    use windows::{
        Wdk::Storage::FileSystem::{
            FILE_DIRECTORY_FILE, FILE_OPEN_REPARSE_POINT, FILE_SYNCHRONOUS_IO_NONALERT,
        },
        Win32::Storage::FileSystem::{
            FILE_ATTRIBUTE_DIRECTORY, FILE_GENERIC_READ, FILE_READ_ATTRIBUTES, FILE_SHARE_DELETE,
            FILE_SHARE_READ, FILE_SHARE_WRITE, FILE_TRAVERSE, SYNCHRONIZE,
        },
    };

    let mut components = paths.home.components();
    if !matches!(components.next(), Some(Component::Prefix(_)))
        || !matches!(components.next(), Some(Component::RootDir))
    {
        return Err(storage_error());
    }
    let names = components
        .map(|component| match component {
            Component::Normal(name) => Ok(name),
            _ => Err(storage_error()),
        })
        .collect::<io::Result<Vec<_>>>()?;
    let expected_len = names.len() + 2;
    if held.len() != expected_len {
        return Err(storage_error());
    }
    for file in held {
        validate_windows_directory_handle(file)?;
    }

    for (index, name) in names.into_iter().enumerate() {
        let parent = held.get(index).ok_or_else(storage_error)?;
        let expected = held.get(index + 1).ok_or_else(storage_error)?;
        let current = windows_open_relative(
            parent,
            name,
            (FILE_GENERIC_READ | FILE_TRAVERSE | FILE_READ_ATTRIBUTES | SYNCHRONIZE).0,
            (FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE).0,
            WindowsRelativeDisposition::Open,
            FILE_ATTRIBUTE_DIRECTORY.0,
            (FILE_DIRECTORY_FILE | FILE_OPEN_REPARSE_POINT | FILE_SYNCHRONOUS_IO_NONALERT).0,
        )
        .map_err(|_| storage_error())?;
        validate_windows_directory_handle(&current)?;
        if directory_identity(&current)? != directory_identity(expected)? {
            return Err(storage_error());
        }
    }

    let home_index = held.len().checked_sub(2).ok_or_else(storage_error)?;
    let current_qoder = windows_open_relative(
        held.get(home_index).ok_or_else(storage_error)?,
        std::ffi::OsStr::new(".qoderwork"),
        (FILE_GENERIC_READ | FILE_TRAVERSE | FILE_READ_ATTRIBUTES | SYNCHRONIZE).0,
        (FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE).0,
        WindowsRelativeDisposition::Open,
        FILE_ATTRIBUTE_DIRECTORY.0,
        (FILE_DIRECTORY_FILE | FILE_OPEN_REPARSE_POINT | FILE_SYNCHRONOUS_IO_NONALERT).0,
    )
    .map_err(|_| storage_error())?;
    validate_windows_directory_handle(&current_qoder)?;
    if directory_identity(&current_qoder)?
        != directory_identity(held.last().ok_or_else(storage_error)?)?
    {
        return Err(storage_error());
    }
    Ok(())
}

#[cfg(all(target_os = "windows", test))]
fn windows_production_context_for(_home: &Path) -> io::Result<bool> {
    Ok(false)
}

#[cfg(all(target_os = "windows", not(test)))]
fn windows_production_context_for(home: &Path) -> io::Result<bool> {
    #[cfg(feature = "test-hooks")]
    if let Some(test_home) = std::env::var_os("FYAGENT_TEST_HOME") {
        let test_home = PathBuf::from(test_home);
        if test_home.is_absolute() && test_home == home {
            return Ok(false);
        }
        return Err(storage_error());
    }

    if home != crate::windows_runtime::require_interactive_user_context().user_profile() {
        return Err(storage_error());
    }
    Ok(true)
}

#[cfg(target_os = "windows")]
fn revalidate_windows_production_context(production_context: bool) -> io::Result<()> {
    if production_context
        && !crate::windows_runtime::revalidate_interactive_user_context(
            crate::windows_runtime::require_interactive_user_context(),
        )
    {
        return Err(storage_error());
    }
    Ok(())
}

fn storage_error() -> io::Error {
    io::Error::new(
        io::ErrorKind::PermissionDenied,
        "Qoder Hooks storage is unavailable",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use sha2::Digest;

    struct HomeGuard(Option<std::ffi::OsString>);

    impl HomeGuard {
        fn set(home: &Path) -> Self {
            let guard = Self(std::env::var_os("FYAGENT_TEST_HOME"));
            std::env::set_var("FYAGENT_TEST_HOME", home);
            guard
        }
    }

    impl Drop for HomeGuard {
        fn drop(&mut self) {
            match self.0.take() {
                Some(value) => std::env::set_var("FYAGENT_TEST_HOME", value),
                None => std::env::remove_var("FYAGENT_TEST_HOME"),
            }
        }
    }

    fn command(value: &str, timeout: Option<u16>) -> QoderHookCommand {
        QoderHookCommand {
            kind: QoderHookType::Command,
            command: value.to_string(),
            timeout,
        }
    }

    fn group(event: QoderHookEvent, matcher: Option<&str>, value: &str) -> QoderHookGroup {
        QoderHookGroup {
            event,
            matcher: matcher.map(str::to_string),
            hooks: vec![command(value, Some(60))],
        }
    }

    fn request(revision: Option<String>, groups: Vec<QoderHookGroup>) -> SaveQoderworkHooksRequest {
        SaveQoderworkHooksRequest {
            expected_revision: revision,
            groups,
            overwrite_token: None,
        }
    }

    fn paths(temp: &tempfile::TempDir) -> QoderPaths {
        QoderPaths::from_home(temp.path())
    }

    fn seed(paths: &QoderPaths, value: Value) {
        fs::create_dir_all(&paths.directory).unwrap();
        fs::write(&paths.settings, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
    }

    fn token(outcome: SaveQoderworkHooksOutcome) -> String {
        match outcome {
            SaveQoderworkHooksOutcome::OverwriteConfirmationRequired { token } => token,
            other => panic!("expected confirmation token, got {other:?}"),
        }
    }

    #[tokio::test]
    #[serial]
    async fn missing_read_is_side_effect_free_and_restart_is_always_required() {
        let temp = tempfile::tempdir().unwrap();
        let _home = HomeGuard::set(temp.path());
        let snapshot = get_qoderwork_hooks(&QoderHooksState::new()).await.unwrap();
        assert_eq!(
            snapshot,
            QoderHooksSnapshot {
                revision: None,
                exists: false,
                groups: Vec::new(),
                restart_required: true,
                supported_structure: true,
            }
        );
        assert!(!temp.path().join(".qoderwork").exists());
    }

    #[test]
    fn projection_accepts_only_the_documented_shape_and_events() {
        let key = random_mac_key();
        let bytes = serde_json::to_vec(&serde_json::json!({
            "theme": "dark",
            "hooks": {
                "SessionStart": [{
                    "matcher": "startup|resume",
                    "hooks": [{"type":"command","command":"notify", "timeout":30}]
                }],
                "Stop": [{"hooks":[{"type":"command","command":"check"}]}]
            }
        }))
        .unwrap();
        let loaded = load_document_bytes(Some(bytes), &key).unwrap();
        assert!(loaded.supported_structure);
        assert_eq!(loaded.groups.len(), 2);
        assert_eq!(loaded.groups[0].event, QoderHookEvent::SessionStart);
        assert_eq!(loaded.groups[1].hooks[0].timeout, None);
    }

    #[test]
    fn invalid_json_non_object_and_unsupported_values_are_controlled() {
        let key = random_mac_key();
        assert_eq!(
            load_document_bytes(Some(b"not-json".to_vec()), &key)
                .unwrap_err()
                .code(),
            QoderHooksErrorCode::InvalidJson
        );
        assert_eq!(
            load_document_bytes(Some(b"[]".to_vec()), &key)
                .unwrap_err()
                .code(),
            QoderHooksErrorCode::RootNotObject
        );
        for value in [
            serde_json::json!({"hooks": []}),
            serde_json::json!({"hooks": {"Unknown": []}}),
            serde_json::json!({"hooks":{"Stop":[{"extra":"raw","hooks":[]}]}}),
            serde_json::json!({"hooks":{"Stop":[{"hooks":[{"type":"prompt","command":"secret"}]}]}}),
        ] {
            let loaded =
                load_document_bytes(Some(serde_json::to_vec(&value).unwrap()), &key).unwrap();
            assert!(!loaded.supported_structure);
        }
    }

    #[test]
    fn unsupported_raw_commands_never_cross_the_snapshot_or_error_boundary() {
        let sentinel = "RAW-UNKNOWN-QODER-COMMAND-SENTINEL";
        let loaded = load_document_bytes(
            Some(
                serde_json::to_vec(&serde_json::json!({
                    "hooks": {
                        "Stop": [{
                            "hooks": [{"type":"prompt", "command": sentinel, "private": sentinel}]
                        }]
                    }
                }))
                .unwrap(),
            ),
            &random_mac_key(),
        )
        .unwrap();
        let snapshot = snapshot_from_loaded(&loaded);
        assert!(!snapshot.supported_structure);
        assert!(snapshot.groups.is_empty());
        assert!(!serde_json::to_string(&snapshot).unwrap().contains(sentinel));
        assert!(
            !QoderHooksError::new(QoderHooksErrorCode::UnsupportedStructure)
                .to_string()
                .contains(sentinel)
        );
    }

    #[test]
    fn public_snapshot_and_save_outcomes_use_the_exact_camel_case_wire() {
        let snapshot = QoderHooksSnapshot {
            revision: Some("opaque".to_string()),
            exists: true,
            groups: vec![group(QoderHookEvent::Stop, None, "command")],
            restart_required: true,
            supported_structure: true,
        };
        assert_eq!(
            serde_json::to_value(SaveQoderworkHooksOutcome::Saved {
                snapshot: snapshot.clone()
            })
            .unwrap(),
            serde_json::json!({
                "state": "saved",
                "snapshot": {
                    "revision": "opaque",
                    "exists": true,
                    "groups": [{
                        "event": "Stop",
                        "hooks": [{"type":"command", "command":"command", "timeout":60}]
                    }],
                    "restartRequired": true,
                    "supportedStructure": true
                }
            })
        );
        assert_eq!(
            serde_json::to_value(SaveQoderworkHooksOutcome::OverwriteConfirmationRequired {
                token: "opaque-token".to_string()
            })
            .unwrap(),
            serde_json::json!({
                "state": "overwrite_confirmation_required",
                "token": "opaque-token"
            })
        );
    }

    #[tokio::test]
    #[serial]
    async fn save_preserves_unknown_top_level_fields_and_writes_backup_first() {
        let temp = tempfile::tempdir().unwrap();
        let _home = HomeGuard::set(temp.path());
        let paths = paths(&temp);
        seed(
            &paths,
            serde_json::json!({"theme":{"custom":true},"hooks":{}}),
        );
        let state = QoderHooksState::new();
        let before = get_qoderwork_hooks(&state).await.unwrap();
        let original = fs::read(&paths.settings).unwrap();
        let outcome = save_qoderwork_hooks(
            &state,
            request(
                before.revision,
                vec![group(QoderHookEvent::PreToolUse, Some("Bash"), "guard")],
            ),
        )
        .await
        .unwrap();
        let SaveQoderworkHooksOutcome::Saved { snapshot } = outcome else {
            panic!("save should commit");
        };
        assert_eq!(snapshot.groups.len(), 1);
        assert_eq!(fs::read(&paths.backup).unwrap(), original);
        let root: Value = serde_json::from_slice(&fs::read(&paths.settings).unwrap()).unwrap();
        assert_eq!(root["theme"], serde_json::json!({"custom":true}));
        assert_eq!(
            root["hooks"]["PreToolUse"][0]["hooks"][0]["type"],
            "command"
        );
        assert!(fs::read_dir(&paths.directory).unwrap().all(|entry| !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .contains(".tmp.")));
    }

    #[tokio::test]
    #[serial]
    async fn revision_drift_writes_nothing() {
        let temp = tempfile::tempdir().unwrap();
        let _home = HomeGuard::set(temp.path());
        let paths = paths(&temp);
        seed(&paths, serde_json::json!({"hooks":{}}));
        let state = QoderHooksState::new();
        let stale = get_qoderwork_hooks(&state).await.unwrap().revision;
        fs::write(&paths.settings, br#"{"external":true,"hooks":{}}"#).unwrap();
        let authoritative = fs::read(&paths.settings).unwrap();
        assert_eq!(
            save_qoderwork_hooks(
                &state,
                request(stale, vec![group(QoderHookEvent::Stop, None, "command")]),
            )
            .await
            .unwrap(),
            SaveQoderworkHooksOutcome::ConcurrentModification
        );
        assert_eq!(fs::read(&paths.settings).unwrap(), authoritative);
        assert!(!paths.backup.exists());
    }

    #[tokio::test]
    #[serial]
    async fn destructive_save_token_is_request_bound_expiring_and_one_use() {
        let temp = tempfile::tempdir().unwrap();
        let _home = HomeGuard::set(temp.path());
        let paths = paths(&temp);
        seed(
            &paths,
            serde_json::json!({"hooks":{"Stop":[{"hooks":[{"type":"command","command":"old"}]}]}}),
        );
        let state = QoderHooksState::new();
        let snapshot = get_qoderwork_hooks(&state).await.unwrap();
        let next = request(
            snapshot.revision.clone(),
            vec![group(QoderHookEvent::Stop, None, "new")],
        );
        let issued = token(save_qoderwork_hooks(&state, next.clone()).await.unwrap());

        let mut mismatched = next.clone();
        mismatched.groups[0].hooks[0].command = "different".to_string();
        mismatched.overwrite_token = Some(issued.clone());
        assert_eq!(
            save_qoderwork_hooks(&state, mismatched)
                .await
                .unwrap_err()
                .code(),
            QoderHooksErrorCode::OverwriteTokenInvalid
        );
        let mut reused = next.clone();
        reused.overwrite_token = Some(issued);
        assert_eq!(
            save_qoderwork_hooks(&state, reused)
                .await
                .unwrap_err()
                .code(),
            QoderHooksErrorCode::OverwriteTokenInvalid
        );

        let expiring = token(save_qoderwork_hooks(&state, next.clone()).await.unwrap());
        state.expire_token(&expiring);
        let mut expired = next;
        expired.overwrite_token = Some(expiring);
        assert_eq!(
            save_qoderwork_hooks(&state, expired)
                .await
                .unwrap_err()
                .code(),
            QoderHooksErrorCode::OverwriteTokenExpired
        );
        assert!(!paths.backup.exists());
    }

    #[tokio::test]
    #[serial]
    async fn repeated_confirmation_preflights_keep_pending_capabilities_bounded() {
        let temp = tempfile::tempdir().unwrap();
        let _home = HomeGuard::set(temp.path());
        let paths = paths(&temp);
        seed(
            &paths,
            serde_json::json!({"hooks":{"Stop":[{"hooks":[{"type":"command","command":"old"}]}]}}),
        );
        let state = QoderHooksState::new();
        let snapshot = get_qoderwork_hooks(&state).await.unwrap();
        let next = request(
            snapshot.revision,
            vec![group(QoderHookEvent::Stop, None, "new")],
        );
        let mut tokens = Vec::new();
        for _ in 0..(MAX_PENDING_OVERWRITES + 5) {
            tokens.push(token(
                save_qoderwork_hooks(&state, next.clone()).await.unwrap(),
            ));
        }
        assert_eq!(state.pending().len(), MAX_PENDING_OVERWRITES);

        let mut evicted = next.clone();
        evicted.overwrite_token = Some(tokens[0].clone());
        assert_eq!(
            save_qoderwork_hooks(&state, evicted)
                .await
                .unwrap_err()
                .code(),
            QoderHooksErrorCode::OverwriteTokenInvalid
        );

        let mut newest = next;
        newest.overwrite_token = tokens.last().cloned();
        assert!(matches!(
            save_qoderwork_hooks(&state, newest).await.unwrap(),
            SaveQoderworkHooksOutcome::Saved { .. }
        ));
    }

    #[tokio::test]
    #[serial]
    async fn confirmed_destructive_save_commits_and_token_cannot_be_reused() {
        let temp = tempfile::tempdir().unwrap();
        let _home = HomeGuard::set(temp.path());
        let paths = paths(&temp);
        seed(
            &paths,
            serde_json::json!({"hooks":{"Stop":[{"hooks":[{"type":"command","command":"old"}]}]}}),
        );
        let state = QoderHooksState::new();
        let snapshot = get_qoderwork_hooks(&state).await.unwrap();
        let mut next = request(
            snapshot.revision,
            vec![group(QoderHookEvent::Stop, None, "new")],
        );
        next.overwrite_token = Some(token(
            save_qoderwork_hooks(&state, next.clone()).await.unwrap(),
        ));
        let saved = save_qoderwork_hooks(&state, next.clone()).await.unwrap();
        assert!(matches!(saved, SaveQoderworkHooksOutcome::Saved { .. }));
        assert_eq!(
            save_qoderwork_hooks(&state, next).await.unwrap_err().code(),
            QoderHooksErrorCode::OverwriteTokenInvalid
        );
    }

    #[test]
    fn revision_is_an_opaque_hmac_not_a_bare_digest() {
        let bytes = br#"{"hooks":{}}"#;
        let key = random_mac_key();
        let revision = revision_for(&key, bytes);
        assert_eq!(revision.len(), 64);
        assert_ne!(revision, format!("{:x}", Sha256::digest(bytes)));
        assert_ne!(revision, revision_for(&random_mac_key(), bytes));
    }

    #[test]
    fn bounds_reject_commands_matchers_timeouts_and_total_counts_without_execution() {
        let too_long = "x".repeat(MAX_COMMAND_BYTES + 1);
        assert_eq!(
            validate_groups(&[group(QoderHookEvent::Stop, None, &too_long)])
                .unwrap_err()
                .code(),
            QoderHooksErrorCode::InvalidRequest
        );
        let matcher = "m".repeat(MAX_MATCHER_BYTES + 1);
        assert!(validate_groups(&[group(QoderHookEvent::Stop, Some(&matcher), "ok")]).is_err());
        assert!(validate_groups(&[QoderHookGroup {
            event: QoderHookEvent::Stop,
            matcher: None,
            hooks: vec![command("ok", Some(0))],
        }])
        .is_err());
        assert!(validate_groups(&[QoderHookGroup {
            event: QoderHookEvent::Stop,
            matcher: None,
            hooks: vec![command("ok", Some(MAX_TIMEOUT_SECONDS + 1))],
        }])
        .is_err());
    }

    #[tokio::test]
    #[serial]
    async fn oversized_file_is_rejected_before_json_parsing() {
        let temp = tempfile::tempdir().unwrap();
        let _home = HomeGuard::set(temp.path());
        let paths = paths(&temp);
        fs::create_dir_all(&paths.directory).unwrap();
        let file = fs::File::create(&paths.settings).unwrap();
        file.set_len(MAX_DOCUMENT_BYTES + 1).unwrap();
        drop(file);
        let error = get_qoderwork_hooks(&QoderHooksState::new())
            .await
            .unwrap_err();
        assert_eq!(error.code(), QoderHooksErrorCode::DocumentTooLarge);
    }

    #[test]
    fn debug_errors_revisions_tokens_and_requests_never_expose_command_sentinels() {
        let sentinel = "QODER-SECRET-COMMAND-SENTINEL";
        let request = SaveQoderworkHooksRequest {
            expected_revision: Some(format!("revision-{sentinel}")),
            groups: vec![group(QoderHookEvent::Stop, Some(sentinel), sentinel)],
            overwrite_token: Some(format!("token-{sentinel}")),
        };
        let outputs = [
            format!("{request:?}"),
            format!("{:?}", request.groups[0]),
            QoderHooksError::new(QoderHooksErrorCode::InvalidRequest).to_string(),
            serde_json::to_string(&QoderHooksErrorDto::from(QoderHooksError::new(
                QoderHooksErrorCode::InvalidRequest,
            )))
            .unwrap(),
        ];
        for output in outputs {
            assert!(!output.contains(sentinel));
        }
    }

    #[cfg(target_os = "windows")]
    #[tokio::test]
    #[serial]
    async fn windows_reparse_directory_and_hardlinked_leaf_fail_closed() {
        use std::process::{Command, Stdio};

        let temp = tempfile::tempdir().unwrap();
        let _home = HomeGuard::set(temp.path());
        let redirected = tempfile::tempdir().unwrap();
        let qoder = temp.path().join(".qoderwork");
        let status = Command::new("cmd.exe")
            .args(["/d", "/c", "mklink", "/J"])
            .arg(&qoder)
            .arg(redirected.path())
            .stdout(Stdio::null())
            .status()
            .unwrap();
        assert!(status.success());
        assert_eq!(
            get_qoderwork_hooks(&QoderHooksState::new())
                .await
                .unwrap_err()
                .code(),
            QoderHooksErrorCode::ReadFailed
        );
        assert!(fs::read_dir(redirected.path()).unwrap().next().is_none());

        fs::remove_dir(&qoder).unwrap();
        fs::create_dir(&qoder).unwrap();
        let source = temp.path().join("source.json");
        fs::write(&source, br#"{"hooks":{}}"#).unwrap();
        fs::hard_link(&source, qoder.join(SETTINGS_FILE_NAME)).unwrap();
        assert_eq!(
            get_qoderwork_hooks(&QoderHooksState::new())
                .await
                .unwrap_err()
                .code(),
            QoderHooksErrorCode::ReadFailed
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_missing_primary_create_race_is_concurrent_and_writes_no_backup() {
        let temp = tempfile::tempdir().unwrap();
        let _home = HomeGuard::set(temp.path());
        let paths = paths(&temp);
        let mut storage = FixedDocumentStorage::open(&paths, true).unwrap();
        let mut snapshot = storage.snapshot_settings().unwrap();
        assert!(snapshot.bytes().is_none());

        let raced = br#"{"external":true,"hooks":{}}"#;
        fs::write(&paths.settings, raced).unwrap();
        assert_eq!(
            storage.commit(&mut snapshot, br#"{"hooks":{}}"#),
            Err(CommitError::Concurrent)
        );
        assert_eq!(fs::read(&paths.settings).unwrap(), raced);
        assert!(!paths.backup.exists());
        assert!(fs::read_dir(&paths.directory).unwrap().all(|entry| !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .contains(".tmp.")));
    }
}
