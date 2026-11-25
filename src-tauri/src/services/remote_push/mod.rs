//! Agentless SSH remote configuration push and Windows-host WSL sync service.
//!
//! Renders local configurations into ready-to-use files and pushes them over
//! standard POSIX shell stdin with automatic backups and atomic renames.
//! Requires ZERO FyAgent installation on the target host or distribution.

pub mod profiles;
pub mod render;
pub mod ssh_config;
pub mod subsystem;
pub mod transport;

use serde::{Deserialize, Serialize};

use self::{
    render::{render_remote_push_bundle_with_mapping, RemotePathMapping, RemotePushSelection},
    ssh_config::{list_ssh_config_hosts, validate_ssh_target, SshHostEntry, SshTargetSpec},
    transport::{
        apply_ssh_push_at, probe_remote_existing_paths_at, test_ssh_connection,
        RemotePushApplyResult, RemotePushFileAction, RemotePushFilePreview,
        SshConnectionTestResult,
    },
};
use crate::store::AppState;

const REMOTE_BACKUP_DIR_NAME: &str = ".fyagent-remote-backups";

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemotePushRequest {
    pub target: SshTargetSpec,
    pub selection: RemotePushSelection,
    #[serde(default)]
    pub path_mapping: Option<RemotePathMapping>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemotePushPreviewResult {
    pub destination: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_home: Option<String>,
    pub reachable: bool,
    pub files: Vec<RemotePushFilePreview>,
    pub total_bytes: usize,
    pub warnings: Vec<String>,
}

pub fn list_ssh_hosts() -> Vec<SshHostEntry> {
    list_ssh_config_hosts()
}

pub fn test_ssh_target(target: SshTargetSpec) -> Result<SshConnectionTestResult, String> {
    let validated = validate_ssh_target(&target)?;
    Ok(test_ssh_connection(&validated))
}

pub fn preview_ssh_config_push(
    state: &AppState,
    request: RemotePushRequest,
) -> Result<RemotePushPreviewResult, String> {
    let validated = validate_ssh_target(&request.target)?;
    let bundle = render_remote_push_bundle_with_mapping(
        state,
        &request.selection,
        request.path_mapping.as_ref(),
        REMOTE_BACKUP_DIR_NAME,
    )?;
    let mut warnings = bundle.warnings;
    let display_prefix = bundle.display_prefix;
    let target_root = bundle.target_root;

    let rel_paths: Vec<&str> = bundle
        .files
        .iter()
        .map(|f| f.remote_rel_path.as_str())
        .collect();
    let (reachable, remote_home, existing_set) = if rel_paths.is_empty() {
        (true, None, std::collections::HashSet::new())
    } else {
        match probe_remote_existing_paths_at(&validated, &rel_paths, target_root.as_deref()) {
            Ok((home, existing)) => (true, home, existing),
            Err(err) => {
                warnings.push(format!(
                    "{err}（仍可查看待推送文件列表，推送前请确认免密 SSH 已就绪）"
                ));
                (false, None, std::collections::HashSet::new())
            }
        }
    };

    let files: Vec<RemotePushFilePreview> = bundle
        .files
        .iter()
        .map(|f| {
            let action = if !reachable {
                RemotePushFileAction::Unknown
            } else if existing_set.contains(&f.remote_rel_path) {
                RemotePushFileAction::Update
            } else {
                RemotePushFileAction::Create
            };
            RemotePushFilePreview {
                remote_path: format!("{display_prefix}{}", f.remote_rel_path),
                category: f.category,
                bytes: f.content.len(),
                mode: f.mode.to_string(),
                contains_secret: f.contains_secret,
                action,
                summary: f.summary.clone(),
            }
        })
        .collect();

    let total_bytes = files.iter().map(|f| f.bytes).sum();

    Ok(RemotePushPreviewResult {
        destination: validated.destination,
        remote_home,
        reachable,
        files,
        total_bytes,
        warnings,
    })
}

pub fn apply_ssh_config_push(
    state: &AppState,
    request: RemotePushRequest,
) -> Result<RemotePushApplyResult, String> {
    let validated = validate_ssh_target(&request.target)?;
    let bundle = render_remote_push_bundle_with_mapping(
        state,
        &request.selection,
        request.path_mapping.as_ref(),
        REMOTE_BACKUP_DIR_NAME,
    )?;
    apply_ssh_push_at(
        &validated,
        bundle.target_root.as_deref(),
        &bundle.display_prefix,
        &bundle.files,
        bundle.warnings,
    )
}
