//! Claude-only consent. Paths, credentials, inputs and bytes never cross IPC.
use std::{
    cell::Cell,
    collections::VecDeque,
    fs,
    io::Read,
    path::PathBuf,
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};

use super::{live, ProviderService, QuickSetupApplyFailureCode, QUICK_SETUP_CLAUDE_PROVIDER_ID};
use crate::{
    app_config::AppType,
    config::{self, FileWriteTarget},
    error::AppError,
    provider::Provider,
    services::mcp::McpService,
    store::AppState,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

const TTL: Duration = Duration::from_secs(180);
const CAPACITY: usize = 16;
const MAX_BYTES: usize = 64 * 1024 * 1024;
const STORE_BYTES: usize = 128 * 1024 * 1024;
static PREVIEWS: OnceLock<Mutex<VecDeque<Draft>>> = OnceLock::new();

#[derive(Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClaudeFileTarget {
    ClaudeSettings,
    ClaudeMcp,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeSidecar {
    target: ClaudeFileTarget,
    backup_path: Option<String>,
    undo_path: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeQuickSetupPreview {
    contract_version: u8,
    preview_id: String,
    write_targets: Vec<FileWriteTarget>,
    preserved_paths: Vec<String>,
    sidecars: Vec<ClaudeSidecar>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ClaudeQuickSetupApplyRequest {
    pub preview_id: String,
}
#[derive(Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ClaudeOverall {
    Applied,
    Partial,
    Stale,
    RolledBack,
    Unknown,
}
#[derive(Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ClaudeProviderState {
    Applied,
    RolledBack,
    Unchanged,
    Unknown,
}
#[derive(Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ClaudeFileState {
    Applied,
    Unchanged,
    /// An attempted write failed; exact preimage is retained or restored.
    /// This says nothing about Provider or the other file.
    RolledBack,
    Conflict,
    NotAttempted,
    Unknown,
}
#[derive(Serialize)]
pub struct ClaudeFileOutcome {
    target: ClaudeFileTarget,
    state: ClaudeFileState,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeQuickSetupOutcome {
    contract_version: u8,
    overall: ClaudeOverall,
    provider_state: ClaudeProviderState,
    files: Vec<ClaudeFileOutcome>,
}
impl ClaudeQuickSetupOutcome {
    pub fn unknown() -> Self {
        Self::empty(
            ClaudeOverall::Unknown,
            ClaudeProviderState::Unknown,
            ClaudeFileState::Unknown,
        )
    }
    fn stale() -> Self {
        Self::empty(
            ClaudeOverall::Stale,
            ClaudeProviderState::Unchanged,
            ClaudeFileState::NotAttempted,
        )
    }
    fn empty(
        overall: ClaudeOverall,
        provider_state: ClaudeProviderState,
        state: ClaudeFileState,
    ) -> Self {
        Self {
            contract_version: 1,
            overall,
            provider_state,
            files: [
                ClaudeFileTarget::ClaudeSettings,
                ClaudeFileTarget::ClaudeMcp,
            ]
            .into_iter()
            .map(|target| ClaudeFileOutcome { target, state })
            .collect(),
        }
    }
}

struct ProjectedFile {
    target: ClaudeFileTarget,
    path: PathBuf,
    before: Option<Vec<u8>>,
    after: Option<Vec<u8>>,
    attempted: Cell<bool>,
    write_failed: Cell<bool>,
}
impl ProjectedFile {
    fn read(path: &PathBuf) -> Result<Option<Vec<u8>>, AppError> {
        config::file_write_target(path)?;
        match fs::File::open(path) {
            Ok(file) => {
                let mut bytes = Vec::new();
                file.take((MAX_BYTES + 1) as u64)
                    .read_to_end(&mut bytes)
                    .map_err(|e| AppError::io(path, e))?;
                if bytes.len() > MAX_BYTES {
                    return Err(AppError::Config("claude_preview_too_large".into()));
                }
                Ok(Some(bytes))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(AppError::io(path, e)),
        }
    }
    fn changed(&self) -> bool {
        self.before != self.after
    }
    fn write(&self) -> Result<(), AppError> {
        if !self.changed() {
            return Ok(());
        }
        // The existing writer checks ownership under its own mutex. Keep this
        // scope around one file only: DB/local-settings writes have other owners.
        let hash = self
            .before
            .as_ref()
            .map(|bytes| format!("{:x}", Sha256::digest(bytes)));
        let _owned = config::file_restore_scope(vec![(self.path.clone(), hash)])?;
        let bytes = self
            .after
            .as_deref()
            .ok_or_else(|| AppError::Config("claude_projection_missing".into()))?;
        self.attempted.set(true);
        let result = config::atomic_write(&self.path, bytes);
        self.write_failed.set(result.is_err());
        result
    }
    fn outcome(&self, transaction_failed: bool) -> ClaudeFileOutcome {
        let failed = transaction_failed || self.write_failed.get();
        let state = match Self::read(&self.path) {
            Err(_) => ClaudeFileState::Unknown,
            Ok(current) if current == self.after && !failed => {
                if self.changed() {
                    ClaudeFileState::Applied
                } else {
                    ClaudeFileState::Unchanged
                }
            }
            Ok(current) if current == self.before => {
                if failed && self.changed() && self.attempted.get() {
                    ClaudeFileState::RolledBack
                } else if self.changed() && self.attempted.get() {
                    ClaudeFileState::Unknown
                } else if self.changed() {
                    ClaudeFileState::NotAttempted
                } else {
                    ClaudeFileState::Unchanged
                }
            }
            Ok(current) if current == self.after => ClaudeFileState::Unknown,
            Ok(_) => ClaudeFileState::Conflict,
        };
        ClaudeFileOutcome {
            target: self.target,
            state,
        }
    }
}
pub(super) struct ClaudeWriteProjection {
    provider: Provider,
    inputs: Value,
    files: Vec<ProjectedFile>,
}
impl ClaudeWriteProjection {
    fn capture(state: &AppState, request: &Provider) -> Result<Self, AppError> {
        if request.id != QUICK_SETUP_CLAUDE_PROVIDER_ID || request.uses_subscription_proxy() {
            return Err(AppError::Config("claude_preview_request_invalid".into()));
        }
        ProviderService::reject_quick_setup_secret_collisions(&AppType::Claude, request)?;
        ProviderService::validate_quick_setup_base_url(&AppType::Claude, request)?;
        let mut provider = super::ProviderCredentials::merge_edit(&state.db, "claude", request)?;
        let existing = state.db.get_provider_by_id(&provider.id, "claude")?;
        ProviderService::normalize_provider_if_claude(&AppType::Claude, &mut provider);
        ProviderService::validate_provider_settings(&AppType::Claude, &provider)?;
        live::normalize_provider_common_config_for_storage(
            &state.db,
            &AppType::Claude,
            &mut provider,
        )?;
        ProviderService::normalize_usage_script_credential_overrides(
            &AppType::Claude,
            &mut provider,
        );
        if let Some(existing) = &existing {
            provider.in_failover_queue = existing.in_failover_queue;
        }
        let backup = futures::executor::block_on(state.db.get_live_backup("claude"))?;
        let takeover = backup.is_some()
            || state
                .proxy_service
                .detect_takeover_in_live_config_for_app(&AppType::Claude);
        let servers = McpService::get_all_servers(state)?;
        let settings_path = config::get_claude_settings_path();
        let root_path = config::get_claude_mcp_path();
        let settings_before = ProjectedFile::read(&settings_path)?;
        let root_before = ProjectedFile::read(&root_path)?;
        let parse = |bytes: &Option<Vec<u8>>| -> Result<Value, AppError> {
            bytes
                .as_ref()
                .map(|bytes| {
                    serde_json::from_slice(bytes)
                        .map_err(|_| AppError::Config("claude_preview_json_invalid".into()))
                })
                .unwrap_or_else(|| Ok(json!({})))
        };
        let settings = if takeover {
            let projected = futures::executor::block_on(
                state
                    .proxy_service
                    .build_claude_live_from_provider_while_proxy_active(&provider),
            )
            .map_err(AppError::Message)?;
            live::sanitize_claude_settings_for_live(&projected)
        } else {
            let mut effective = provider.clone();
            effective.settings_config = live::build_effective_settings_with_common_config(
                &state.db,
                &AppType::Claude,
                &provider,
            )?;
            live::build_claude_quick_setup_live_projection(&parse(&settings_before)?, &effective)?
        };
        let root_after = if !takeover && !servers.is_empty() {
            Some(config::json_file_contents(
                &crate::claude_mcp::build_collection_projection(&parse(&root_before)?, &servers)?,
            )?)
        } else {
            root_before.clone()
        };
        let inputs = json!({
            "provider": provider, "existing": existing,
            "localCurrent": crate::settings::get_current_provider(&AppType::Claude),
            "dbCurrent": state.db.get_current_provider("claude")?,
            "snippet": state.db.get_config_snippet("claude")?, "servers": servers,
            "backup": backup.as_ref().map(|b| (&b.app_type, &b.original_config, &b.backed_up_at)),
            "takeover": takeover,
        });
        Ok(Self {
            provider,
            inputs,
            files: vec![
                ProjectedFile {
                    target: ClaudeFileTarget::ClaudeSettings,
                    path: settings_path,
                    before: settings_before,
                    after: Some(config::json_file_contents(&settings)?),
                    attempted: Cell::new(false),
                    write_failed: Cell::new(false),
                },
                ProjectedFile {
                    target: ClaudeFileTarget::ClaudeMcp,
                    path: root_path,
                    before: root_before,
                    after: root_after,
                    attempted: Cell::new(false),
                    write_failed: Cell::new(false),
                },
            ],
        })
    }
    fn matches(&self, current: &Self) -> bool {
        self.inputs == current.inputs
            && self
                .files
                .iter()
                .zip(&current.files)
                .all(|(a, b)| a.path == b.path && a.before == b.before && a.after == b.after)
    }
    pub(super) fn write_settings(&self) -> Result<(), AppError> {
        self.files[0].write()
    }
    pub(super) fn write_root(&self) -> Result<(), AppError> {
        self.files[1].write()
    }
    pub(super) fn snapshots(&self) -> Vec<super::QuickSetupFileSnapshot> {
        self.files
            .iter()
            .filter(|file| file.changed())
            .map(|file| super::QuickSetupFileSnapshot {
                path: file.path.clone(),
                bytes: file.before.clone(),
            })
            .collect()
    }
    fn size(&self) -> usize {
        self.files
            .iter()
            .map(|file| {
                file.before.as_ref().map_or(0, Vec::len) + file.after.as_ref().map_or(0, Vec::len)
            })
            .sum::<usize>()
            + self.inputs.to_string().len()
    }
}
struct Draft {
    id: String,
    created: Instant,
    projection: ClaudeWriteProjection,
}
fn store() -> &'static Mutex<VecDeque<Draft>> {
    PREVIEWS.get_or_init(|| Mutex::new(VecDeque::new()))
}

impl ProviderService {
    pub fn preview_claude_quick_setup(
        state: &AppState,
        provider: Provider,
    ) -> Result<ClaudeQuickSetupPreview, AppError> {
        let _guard = futures::executor::block_on(state.proxy_service.lock_switch_for_app("claude"));
        let projection = ClaudeWriteProjection::capture(state, &provider)?;
        let id = uuid::Uuid::new_v4().to_string();
        let mut dto = ClaudeQuickSetupPreview {
            contract_version: 1,
            preview_id: id.clone(),
            write_targets: Vec::new(),
            preserved_paths: Vec::new(),
            sidecars: Vec::new(),
        };
        for file in &projection.files {
            let target = config::file_write_target(&file.path)?;
            if file.changed() {
                dto.sidecars.push(ClaudeSidecar {
                    target: file.target,
                    backup_path: target.exists.then(|| target.backup_path.clone()),
                    undo_path: config::file_recovery_display_path(&file.path)?,
                });
                dto.write_targets.push(target);
            } else {
                dto.preserved_paths.push(target.path);
            }
        }
        let size = projection.size();
        if size > STORE_BYTES {
            return Err(AppError::Config("claude_preview_too_large".into()));
        }
        let mut drafts = store()
            .lock()
            .map_err(|_| AppError::Config("claude_preview_unavailable".into()))?;
        drafts.retain(|draft| draft.created.elapsed() < TTL);
        while drafts.len() >= CAPACITY
            || drafts
                .iter()
                .map(|draft| draft.projection.size())
                .sum::<usize>()
                + size
                > STORE_BYTES
        {
            drafts.pop_front();
        }
        drafts.push_back(Draft {
            id,
            created: Instant::now(),
            projection,
        });
        Ok(dto)
    }
    pub fn apply_claude_quick_setup_preview(
        state: &AppState,
        request: ClaudeQuickSetupApplyRequest,
    ) -> ClaudeQuickSetupOutcome {
        if uuid::Uuid::parse_str(&request.preview_id)
            .ok()
            .filter(|id| {
                id.get_version() == Some(uuid::Version::Random)
                    && id.to_string() == request.preview_id
            })
            .is_none()
        {
            return ClaudeQuickSetupOutcome::stale();
        }
        let draft = store().lock().ok().and_then(|mut drafts| {
            drafts.retain(|draft| draft.created.elapsed() < TTL);
            let index = drafts
                .iter()
                .position(|draft| draft.id == request.preview_id)?;
            drafts.remove(index)
        });
        let Some(draft) = draft else {
            return ClaudeQuickSetupOutcome::stale();
        };
        let _guard = futures::executor::block_on(state.proxy_service.lock_switch_for_app("claude"));
        if draft.created.elapsed() >= TTL {
            return ClaudeQuickSetupOutcome::stale();
        }
        let projected = draft.projection;
        match ClaudeWriteProjection::capture(state, &projected.provider) {
            Ok(current) if projected.matches(&current) => {}
            _ => return ClaudeQuickSetupOutcome::stale(),
        }
        let result = Self::apply_provider_activation_with_claude_projection_locked(
            state,
            AppType::Claude,
            projected.provider.clone(),
            Some(&projected),
        );
        let failed = result.is_err();
        let files: Vec<_> = projected
            .files
            .iter()
            .map(|file| file.outcome(failed))
            .collect();
        let verified_files = files.iter().all(|file| {
            matches!(
                file.state,
                ClaudeFileState::Applied | ClaudeFileState::Unchanged
            )
        });
        let (overall, provider_state) = match result {
            Ok(_) => (
                if verified_files {
                    ClaudeOverall::Applied
                } else {
                    ClaudeOverall::Partial
                },
                ClaudeProviderState::Applied,
            ),
            Err(error)
                if error.code == QuickSetupApplyFailureCode::ApplyFailedRolledBack
                    && files.iter().all(|file| {
                        matches!(
                            file.state,
                            ClaudeFileState::RolledBack
                                | ClaudeFileState::Unchanged
                                | ClaudeFileState::NotAttempted
                        )
                    }) =>
            {
                (ClaudeOverall::RolledBack, ClaudeProviderState::RolledBack)
            }
            Err(_) => (ClaudeOverall::Unknown, ClaudeProviderState::Unknown),
        };
        ClaudeQuickSetupOutcome {
            contract_version: 1,
            overall,
            provider_state,
            files,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        app_config::{McpApps, McpServer},
        database::Database,
    };
    use serial_test::serial;
    use std::sync::Arc;

    struct Home {
        root: tempfile::TempDir,
        env: Vec<(&'static str, Option<std::ffi::OsString>)>,
        settings: Option<crate::settings::AppSettings>,
    }
    impl Home {
        fn new() -> Self {
            let root = tempfile::tempdir().unwrap();
            let scratch = root.path().join("scratch");
            fs::create_dir(&scratch).unwrap();
            let mut home = Self {
                root,
                env: Vec::new(),
                settings: None,
            };
            for (name, value) in [
                ("FYAGENT_TEST_HOME", home.root.path()),
                ("HOME", home.root.path()),
                ("USERPROFILE", home.root.path()),
                ("LOCALAPPDATA", scratch.as_path()),
                ("TMPDIR", scratch.as_path()),
                ("TEMP", scratch.as_path()),
                ("TMP", scratch.as_path()),
            ] {
                home.env.push((name, std::env::var_os(name)));
                std::env::set_var(name, value);
            }
            assert!(crate::app_store::get_app_config_dir_override().is_none());
            home.settings = Some(crate::settings::get_settings());
            crate::settings::update_settings(crate::settings::AppSettings::default()).unwrap();
            for path in [
                config::get_home_dir(),
                config::get_app_config_dir(),
                config::get_user_temp_dir(),
                config::get_claude_settings_path(),
                config::get_claude_mcp_path(),
            ] {
                assert!(path.starts_with(home.root.path()));
            }
            home
        }
        fn state(&self) -> AppState {
            AppState::new(Arc::new(Database::memory().unwrap()))
        }
    }
    impl Drop for Home {
        fn drop(&mut self) {
            if let Some(settings) = self.settings.take() {
                let _ = crate::settings::update_settings(settings);
            }
            for (name, previous) in self.env.drain(..).rev() {
                match previous {
                    Some(value) => std::env::set_var(name, value),
                    None => std::env::remove_var(name),
                }
            }
        }
    }
    fn provider() -> Provider {
        Provider::with_id(
            QUICK_SETUP_CLAUDE_PROVIDER_ID.into(),
            "Fixture".into(),
            json!({"env":{"ANTHROPIC_BASE_URL":"https://claude.example.test", "ANTHROPIC_AUTH_TOKEN":"fixture-private-key", "ANTHROPIC_MODEL":"fixture-model"}}),
            None,
        )
    }
    fn server(enabled: bool) -> McpServer {
        McpServer {
            id: "managed".into(),
            name: "Managed".into(),
            server: json!({"command":"npx", "args":["-y", "fixture-mcp"], "name":"strip-ui"}),
            apps: McpApps {
                claude: enabled,
                ..Default::default()
            },
            description: None,
            homepage: None,
            docs: None,
            tags: vec![],
        }
    }
    fn seed_root() -> Value {
        json!({"unknownRoot":{"keep":true}, "mcpServers":{"unmanaged":{"command":"npx", "args":["untouched"], "name":"native-user-name", "id":"native-id", "tags":["native"], "extension":{"keep":true}}, "managed":{"command":"old-command"}}})
    }
    fn apply(state: &AppState, preview: ClaudeQuickSetupPreview) -> ClaudeQuickSetupOutcome {
        ProviderService::apply_claude_quick_setup_preview(
            state,
            ClaudeQuickSetupApplyRequest {
                preview_id: preview.preview_id,
            },
        )
    }

    #[test]
    #[serial]
    fn claude_preview_first_and_existing_settings_mcp_matrix() {
        for existing in [false, true] {
            for enabled in [None, Some(true), Some(false)] {
                let home = Home::new();
                let state = home.state();
                if existing {
                    config::write_json_file(
                        &config::get_claude_settings_path(),
                        &json!({"custom":"preserve", "env":{"UNRELATED":"keep"}}),
                    )
                    .unwrap();
                }
                let root = seed_root();
                config::write_json_file(&config::get_claude_mcp_path(), &root).unwrap();
                if let Some(enabled) = enabled {
                    state.db.save_mcp_server(&server(enabled)).unwrap();
                }
                let before = fs::read(config::get_claude_mcp_path()).unwrap();
                let preview =
                    ProviderService::preview_claude_quick_setup(&state, provider()).unwrap();
                let wire = serde_json::to_string(&preview).unwrap();
                assert!(!wire.contains("fixture-private-key"));
                assert!(!wire.contains("ANTHROPIC"));
                assert_eq!(
                    preview.write_targets.len(),
                    if enabled.is_none() { 1 } else { 2 }
                );
                assert_eq!(preview.sidecars[0].backup_path.is_some(), existing);
                assert_eq!(fs::read(config::get_claude_mcp_path()).unwrap(), before);
                assert!(state
                    .db
                    .get_provider_by_id(QUICK_SETUP_CLAUDE_PROVIDER_ID, "claude")
                    .unwrap()
                    .is_none());
                let outcome = apply(&state, preview);
                assert!(outcome.overall == ClaudeOverall::Applied);
                let settings: Value =
                    config::read_json_file(&config::get_claude_settings_path()).unwrap();
                assert_eq!(
                    settings["env"]["ANTHROPIC_AUTH_TOKEN"],
                    "fixture-private-key"
                );
                if existing {
                    assert_eq!(settings["custom"], "preserve");
                    assert_eq!(settings["env"]["UNRELATED"], "keep");
                }
                let after: Value = config::read_json_file(&config::get_claude_mcp_path()).unwrap();
                assert_eq!(after["unknownRoot"], root["unknownRoot"]);
                assert_eq!(
                    after["mcpServers"]["unmanaged"],
                    root["mcpServers"]["unmanaged"]
                );
                match enabled {
                    None => assert_eq!(fs::read(config::get_claude_mcp_path()).unwrap(), before),
                    Some(false) => assert!(after["mcpServers"].get("managed").is_none()),
                    Some(true) => {
                        assert!(after["mcpServers"]["managed"].get("name").is_none());
                        #[cfg(windows)]
                        assert_eq!(after["mcpServers"]["managed"]["command"], "cmd");
                    }
                }
            }
        }
    }

    #[test]
    #[serial]
    fn claude_preview_missing_files_is_read_only_and_empty_mcp_stays_absent() {
        let home = Home::new();
        let state = home.state();
        let preview = ProviderService::preview_claude_quick_setup(&state, provider()).unwrap();
        assert!(!config::get_claude_config_dir().exists());
        assert!(!config::get_claude_mcp_path().exists());
        assert!(!config::rolling_backup_path(&config::get_claude_settings_path()).exists());
        assert_eq!(preview.write_targets.len(), 1);
        assert!(preview.sidecars[0].backup_path.is_none());
        assert!(apply(&state, preview).overall == ClaudeOverall::Applied);
        assert!(!config::get_claude_mcp_path().exists());
    }

    #[test]
    #[serial]
    fn claude_preview_nonempty_disabled_creates_root_after_first_settings() {
        let home = Home::new();
        let state = home.state();
        state.db.save_mcp_server(&server(false)).unwrap();
        let preview = ProviderService::preview_claude_quick_setup(&state, provider()).unwrap();
        assert_eq!(preview.write_targets.len(), 2);
        assert!(apply(&state, preview).overall == ClaudeOverall::Applied);
        let root: Value = config::read_json_file(&config::get_claude_mcp_path()).unwrap();
        assert_eq!(root, json!({"mcpServers":{}}));
    }

    #[test]
    #[serial]
    fn claude_preview_changed_file_or_mcp_input_is_stale_and_single_claim() {
        for change_mcp in [false, true] {
            let home = Home::new();
            let state = home.state();
            let preview = ProviderService::preview_claude_quick_setup(&state, provider()).unwrap();
            let id = preview.preview_id.clone();
            if change_mcp {
                state.db.save_mcp_server(&server(true)).unwrap();
            } else {
                config::write_json_file(&config::get_claude_mcp_path(), &json!({"external":true}))
                    .unwrap();
            }
            assert!(apply(&state, preview).overall == ClaudeOverall::Stale);
            assert!(!config::get_claude_settings_path().exists());
            assert!(state
                .db
                .get_provider_by_id(QUICK_SETUP_CLAUDE_PROVIDER_ID, "claude")
                .unwrap()
                .is_none());
            assert!(
                ProviderService::apply_claude_quick_setup_preview(
                    &state,
                    ClaudeQuickSetupApplyRequest { preview_id: id }
                )
                .overall
                    == ClaudeOverall::Stale
            );
        }
    }

    #[test]
    #[serial]
    fn claude_preview_takeover_preserves_mcp_and_uses_native_builder() {
        let home = Home::new();
        let state = home.state();
        state.db.save_mcp_server(&server(true)).unwrap();
        config::write_json_file(&config::get_claude_mcp_path(), &seed_root()).unwrap();
        let before = fs::read(config::get_claude_mcp_path()).unwrap();
        futures::executor::block_on(state.db.save_live_backup("claude", "{\"env\":{}}")).unwrap();
        let preview = ProviderService::preview_claude_quick_setup(&state, provider()).unwrap();
        assert_eq!(preview.write_targets.len(), 1);
        assert!(apply(&state, preview).overall == ClaudeOverall::Applied);
        assert_eq!(fs::read(config::get_claude_mcp_path()).unwrap(), before);
        let settings: Value = config::read_json_file(&config::get_claude_settings_path()).unwrap();
        assert_ne!(
            settings["env"]["ANTHROPIC_AUTH_TOKEN"],
            "fixture-private-key"
        );
    }

    #[test]
    #[serial]
    fn claude_preview_successful_files_restore_independently_and_keep_provider() {
        let home = Home::new();
        let state = home.state();
        state.db.save_mcp_server(&server(true)).unwrap();
        assert!(
            apply(
                &state,
                ProviderService::preview_claude_quick_setup(&state, provider()).unwrap()
            )
            .overall
                == ClaudeOverall::Applied
        );
        let settings = config::get_claude_settings_path();
        let root = config::get_claude_mcp_path();
        let settings_receipt = config::file_recovery(&settings)
            .unwrap()
            .unwrap()
            .receipt_id;
        let root_receipt = config::file_recovery(&root).unwrap().unwrap().receipt_id;
        fs::write(&root, b"{\"external\":true}").unwrap();
        assert!(config::restore_file_recovery(&root, &root_receipt).is_err());
        config::restore_file_recovery(&settings, &settings_receipt).unwrap();
        assert!(!settings.exists());
        assert_eq!(fs::read(&root).unwrap(), b"{\"external\":true}");
        assert!(state
            .db
            .get_provider_by_id(QUICK_SETUP_CLAUDE_PROVIDER_ID, "claude")
            .unwrap()
            .is_some());
    }

    #[test]
    #[serial]
    fn claude_preview_failed_provider_insert_restores_owned_settings() {
        let home = Home::new();
        let state = home.state();
        state.db.conn.lock().unwrap().execute_batch("CREATE TRIGGER reject_claude BEFORE INSERT ON providers WHEN NEW.app_type='claude' BEGIN SELECT RAISE(ABORT,'fixture'); END;").unwrap();
        let preview = ProviderService::preview_claude_quick_setup(&state, provider()).unwrap();
        let outcome = apply(&state, preview);
        assert!(outcome.overall == ClaudeOverall::RolledBack);
        assert!(!config::get_claude_settings_path().exists());
        assert!(state
            .db
            .get_provider_by_id(QUICK_SETUP_CLAUDE_PROVIDER_ID, "claude")
            .unwrap()
            .is_none());
    }

    #[test]
    #[serial]
    fn claude_preview_capacity_expiry_and_duplicate_apply_are_bounded() {
        let home = Home::new();
        let state = home.state();
        let first = ProviderService::preview_claude_quick_setup(&state, provider()).unwrap();
        for _ in 0..CAPACITY {
            ProviderService::preview_claude_quick_setup(&state, provider()).unwrap();
        }
        assert!(apply(&state, first).overall == ClaudeOverall::Stale);
        let expired = ProviderService::preview_claude_quick_setup(&state, provider()).unwrap();
        store()
            .lock()
            .unwrap()
            .iter_mut()
            .find(|draft| draft.id == expired.preview_id)
            .unwrap()
            .created = Instant::now() - TTL;
        assert!(apply(&state, expired).overall == ClaudeOverall::Stale);
        let preview = ProviderService::preview_claude_quick_setup(&state, provider()).unwrap();
        let id = preview.preview_id.clone();
        assert!(apply(&state, preview).overall == ClaudeOverall::Applied);
        assert!(
            ProviderService::apply_claude_quick_setup_preview(
                &state,
                ClaudeQuickSetupApplyRequest { preview_id: id }
            )
            .overall
                == ClaudeOverall::Stale
        );
    }
    #[test]
    #[serial]
    fn claude_owned_compensation_restores_root_despite_external_settings() {
        let home = Home::new();
        let _state = home.state();
        let settings = config::get_claude_settings_path();
        let root = config::get_claude_mcp_path();
        config::write_json_file(&settings, &json!({"before":"settings"})).unwrap();
        config::write_json_file(&root, &json!({"before":"root"})).unwrap();
        let a = super::super::QuickSetupFileSnapshot::capture(settings.clone()).unwrap();
        let b = super::super::QuickSetupFileSnapshot::capture(root.clone()).unwrap();
        let _operation = config::file_mutation_scope();
        config::write_json_file(&settings, &json!({"owned":"settings"})).unwrap();
        config::write_json_file(&root, &json!({"owned":"root"})).unwrap();
        fs::write(&settings, b"{\"external\":true}").unwrap();
        assert!(a.restore_owned().is_err());
        b.restore_owned().unwrap();
        assert_eq!(fs::read(&settings).unwrap(), b"{\"external\":true}");
        assert!(b.matches_current().unwrap());
    }

    #[test]
    #[serial]
    fn claude_preview_failed_compensation_is_unknown() {
        let home = Home::new();
        let state = home.state();
        state.db.conn.lock().unwrap().execute_batch("CREATE TRIGGER tamper_claude AFTER INSERT ON providers WHEN NEW.app_type='claude' BEGIN UPDATE providers SET name='tampered' WHERE id=NEW.id AND app_type=NEW.app_type; END; CREATE TRIGGER deny_rollback BEFORE DELETE ON providers WHEN OLD.app_type='claude' BEGIN SELECT RAISE(ABORT,'fixture rollback blocked'); END;").unwrap();
        let preview = ProviderService::preview_claude_quick_setup(&state, provider()).unwrap();
        assert!(apply(&state, preview).overall == ClaudeOverall::Unknown);
        assert!(!config::get_claude_settings_path().exists());
        assert!(state
            .db
            .get_provider_by_id(QUICK_SETUP_CLAUDE_PROVIDER_ID, "claude")
            .unwrap()
            .is_some());
    }
    #[test]
    #[serial]
    fn claude_legacy_request_entry_rejects_without_business_writes() {
        let home = Home::new();
        let state = home.state();
        let settings = config::get_claude_settings_path();
        let root = config::get_claude_mcp_path();
        let before_settings = ProjectedFile::read(&settings).unwrap();
        let before_root = ProjectedFile::read(&root).unwrap();
        let local = crate::settings::get_current_provider(&AppType::Claude);
        assert!(
            ProviderService::apply_legacy_quick_setup(&state, AppType::Claude, provider()).is_err()
        );
        assert_eq!(ProjectedFile::read(&settings).unwrap(), before_settings);
        assert_eq!(ProjectedFile::read(&root).unwrap(), before_root);
        assert!(!config::get_claude_config_dir().exists());
        assert!(state
            .db
            .get_provider_by_id(QUICK_SETUP_CLAUDE_PROVIDER_ID, "claude")
            .unwrap()
            .is_none());
        assert!(state.db.get_current_provider("claude").unwrap().is_none());
        assert_eq!(
            crate::settings::get_current_provider(&AppType::Claude),
            local
        );
        assert!(
            futures::executor::block_on(state.db.get_live_backup("claude"))
                .unwrap()
                .is_none()
        );
        assert!(!config::rolling_backup_path(&settings).exists());
        assert!(!config::rolling_backup_path(&root).exists());
    }
    #[test]
    #[serial]
    fn claude_apply_root_writer_failure_reports_partial_retained_preimage() {
        let home = Home::new();
        let state = home.state();
        state.db.save_mcp_server(&server(true)).unwrap();
        let root = config::get_claude_mcp_path();
        config::write_json_file(&root, &seed_root()).unwrap();
        let before = fs::read(&root).unwrap();
        let preview = ProviderService::preview_claude_quick_setup(&state, provider()).unwrap();
        // Fault occurs after preview revalidation, during Provider persistence,
        // before the real root atomic writer. No synthetic outcome is returned.
        let marker = home.root.path().join(".claude.json.fyagent.undo.json");
        assert!(marker.starts_with(home.root.path()));
        state.db.conn.lock().unwrap().update_hook(Some(
            move |action: rusqlite::hooks::Action, _: &str, table: &str, _: i64| {
                if table == "providers" && action == rusqlite::hooks::Action::SQLITE_INSERT {
                    fs::remove_file(&marker).unwrap();
                    fs::create_dir(&marker).unwrap();
                }
            },
        ));
        let result = apply(&state, preview);
        state
            .db
            .conn
            .lock()
            .unwrap()
            .update_hook(None::<fn(rusqlite::hooks::Action, &str, &str, i64)>);
        assert!(result.overall == ClaudeOverall::Partial);
        assert!(result.provider_state == ClaudeProviderState::Applied);
        assert!(result.files[0].state == ClaudeFileState::Applied);
        assert!(result.files[1].state == ClaudeFileState::RolledBack);
        assert!(result.files[1].state != ClaudeFileState::NotAttempted);
        assert_eq!(fs::read(&root).unwrap(), before);
        assert!(state
            .db
            .get_provider_by_id(QUICK_SETUP_CLAUDE_PROVIDER_ID, "claude")
            .unwrap()
            .is_some());
        assert_eq!(
            state.db.get_current_provider("claude").unwrap().as_deref(),
            Some(QUICK_SETUP_CLAUDE_PROVIDER_ID)
        );
    }
}
