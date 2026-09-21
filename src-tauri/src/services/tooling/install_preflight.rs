//! Read-only CLI host checks shared by install confirmation and execution.

#[cfg(any(target_os = "macos", test))]
use fyagent_user_helper::closed_dep::admit_closed_iarna_toml_at_prefix;
use fyagent_user_helper::GrokNpmInstallPlan;

use crate::{agent_install::AgentReasonCode, services::external_agents::AgentCatalogId};

#[cfg(any(target_os = "windows", test))]
fn map_cli_helper_platform_error(code: Option<&str>) -> AgentReasonCode {
    match code {
        Some("tool_permission_denied") => AgentReasonCode::PermissionDenied,
        Some("grok_tool_host_missing") => AgentReasonCode::ToolHostMissing,
        Some("grok_tool_owner_mismatch") => AgentReasonCode::ToolOwnerUnsupported,
        Some("insufficient_disk_space") => AgentReasonCode::InsufficientDiskSpace,
        Some("tool_candidate_conflict") => AgentReasonCode::CandidateConflict,
        Some("tool_target_changed") => AgentReasonCode::TargetChanged,
        _ => AgentReasonCode::InteractiveUserUnavailable,
    }
}

#[cfg(any(target_os = "macos", test))]
fn reject_closed_iarna_conflict(prefix: &std::path::Path) -> Result<(), AgentReasonCode> {
    admit_closed_iarna_toml_at_prefix(prefix).map_err(|_| AgentReasonCode::CandidateConflict)
}

pub(crate) struct CliInstallPreflight {
    pub paths: Vec<std::path::PathBuf>,
    pub location_label: String,
    pub requires_npm: bool,
    pub npm_target: Option<fyagent_user_helper::NpmTargetBinding>,
}

#[cfg(target_os = "macos")]
pub(super) async fn check(
    agent: AgentCatalogId,
    action: crate::agent_install::AgentActionId,
    _plan: Option<&GrokNpmInstallPlan>,
) -> Result<CliInstallPreflight, AgentReasonCode> {
    tokio::task::spawn_blocking(move || {
        let tool = match agent {
            AgentCatalogId::ClaudeCode => "claude",
            AgentCatalogId::GrokBuild => "grok",
            _ => return Err(AgentReasonCode::ActionNotSupported),
        };
        let installs = super::enumerate_tool_installations(tool);
        let existing = super::default_install(&installs);
        let update = action == crate::agent_install::AgentActionId::Update;
        if update && existing.is_none() {
            return Err(AgentReasonCode::TargetChanged);
        }
        if !update && !installs.is_empty() {
            return Err(AgentReasonCode::TargetChanged);
        }
        let anchor = existing.map(|install| install.path.as_str());
        if let Some(install) = existing {
            let native = fyagent_user_helper::grok::is_native_install_path(
                &install.path,
                &install.real.to_string_lossy(),
            );
            if native && agent == AgentCatalogId::GrokBuild {
                let parent = install
                    .real
                    .parent()
                    .ok_or(AgentReasonCode::ToolOwnerUnsupported)?;
                if !crate::agent_install::directory_writable(parent) {
                    return Err(AgentReasonCode::PermissionDenied);
                }
                return Ok(CliInstallPreflight {
                    paths: vec![parent.to_path_buf()],
                    location_label: crate::agent_install::display_user_path(parent),
                    requires_npm: false,
                    npm_target: None,
                });
            }
        }
        if super::npm_runtime::npm_major(anchor).is_none()
            || !super::npm_runtime::node_matches_host(anchor)
            || (agent == AgentCatalogId::ClaudeCode
                && super::npm_runtime::node_major(anchor)
                    .is_none_or(|major| major < fyagent_user_helper::claude::CLAUDE_MIN_NODE_MAJOR))
        {
            return Err(AgentReasonCode::ToolHostMissing);
        }
        let destination = super::npm_runtime::observe_destination(anchor)
            .ok_or(AgentReasonCode::ToolHostMissing)?;
        let prefix = std::path::PathBuf::from(&destination.prefix);
        let writable_prefix = crate::agent_install::existing_directory(&prefix)?;
        if !crate::agent_install::directory_writable(&writable_prefix) {
            return Err(AgentReasonCode::PermissionDenied);
        }
        let cache_path = std::path::PathBuf::from(&destination.cache);
        let writable_cache = crate::agent_install::existing_directory(&cache_path)?;
        if !crate::agent_install::directory_writable(&writable_cache) {
            return Err(AgentReasonCode::PermissionDenied);
        }
        if agent == AgentCatalogId::GrokBuild {
            reject_closed_iarna_conflict(&prefix)?;
        }
        let bin = prefix.join("bin");
        if let Some(anchor) = anchor {
            if std::path::Path::new(anchor)
                .parent()
                .and_then(|path| path.canonicalize().ok())
                != bin.canonicalize().ok()
            {
                return Err(AgentReasonCode::ToolOwnerUnsupported);
            }
        } else if !super::npm_runtime::execution_path()
            .is_some_and(|path| std::env::split_paths(&path).any(|entry| entry == bin))
        {
            return Err(AgentReasonCode::ToolOwnerUnsupported);
        }
        let mut paths = vec![writable_prefix];
        if !paths.contains(&writable_cache) {
            paths.push(writable_cache);
        }
        let temp_ancestor =
            crate::agent_install::existing_directory(std::path::Path::new(&destination.temp))?;
        if !paths.contains(&temp_ancestor) {
            paths.push(temp_ancestor);
        }
        Ok(CliInstallPreflight {
            location_label: crate::agent_install::display_user_path(&prefix),
            paths,
            requires_npm: true,
            npm_target: Some(
                fyagent_user_helper::NpmTargetBinding::from_observation(&destination)
                    .map_err(|_| AgentReasonCode::ToolHostMissing)?,
            ),
        })
    })
    .await
    .map_err(|_| AgentReasonCode::ToolHostMissing)?
}

#[cfg(target_os = "windows")]
pub(super) async fn check(
    agent: AgentCatalogId,
    action: crate::agent_install::AgentActionId,
    plan: Option<&GrokNpmInstallPlan>,
) -> Result<CliInstallPreflight, AgentReasonCode> {
    use crate::codex_desktop::platform::windows::{
        run_claude_tool_operation, run_grok_tool_operation,
    };
    use fyagent_user_helper::{GrokOwner, GrokToolAction};
    let plan = plan.cloned();
    let observed = tokio::task::spawn_blocking(move || match agent {
        AgentCatalogId::ClaudeCode => run_claude_tool_operation(
            crate::windows_runtime::require_interactive_user_context(),
            GrokToolAction::Preflight,
            plan,
        ),
        AgentCatalogId::GrokBuild => run_grok_tool_operation(
            crate::windows_runtime::require_interactive_user_context(),
            GrokToolAction::Preflight,
            None,
            plan,
        ),
        _ => unreachable!("only policy-admitted CLI agents reach preflight"),
    })
    .await
    .map_err(|_| AgentReasonCode::InteractiveUserUnavailable)?
    .map_err(|error| {
        map_cli_helper_platform_error(error.to_dto().details.platform_error_code.as_deref())
    })?;
    if (action == crate::agent_install::AgentActionId::Install && observed.detected)
        || (action == crate::agent_install::AgentActionId::Update && !observed.detected)
    {
        return Err(AgentReasonCode::TargetChanged);
    }
    let native = observed.owner == Some(GrokOwner::Native);
    if agent == AgentCatalogId::ClaudeCode && native {
        return Err(AgentReasonCode::ToolOwnerUnsupported);
    }
    if native {
        return Ok(CliInstallPreflight {
            paths: vec![crate::windows_runtime::user_local_app_data_dir()],
            location_label: "当前桌面用户的 CLI 安装位置".to_string(),
            requires_npm: false,
            npm_target: None,
        });
    }
    let destination = observed
        .npm_destination
        .ok_or(AgentReasonCode::ToolHostMissing)?;
    let npm_target = fyagent_user_helper::NpmTargetBinding::from_observation(&destination)
        .map_err(|_| AgentReasonCode::ToolHostMissing)?;
    let prefix = std::path::PathBuf::from(npm_target.prefix());
    let cache = std::path::PathBuf::from(npm_target.cache());
    let temp = std::path::PathBuf::from(npm_target.temp());
    let mut paths = vec![crate::agent_install::existing_directory(&prefix)?];
    for extra in [cache, temp] {
        let ancestor = crate::agent_install::existing_directory(&extra)?;
        if !paths.contains(&ancestor) {
            paths.push(ancestor);
        }
    }
    Ok(CliInstallPreflight {
        location_label: crate::agent_install::display_user_path(&prefix),
        paths,
        requires_npm: true,
        npm_target: Some(npm_target),
    })
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub(super) async fn check(
    _agent: AgentCatalogId,
    _action: crate::agent_install::AgentActionId,
    _plan: Option<&GrokNpmInstallPlan>,
) -> Result<CliInstallPreflight, AgentReasonCode> {
    Err(AgentReasonCode::PlatformUnsupported)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helper_platform_errors_keep_space_and_conflict_actionable() {
        assert_eq!(
            map_cli_helper_platform_error(Some("insufficient_disk_space")),
            AgentReasonCode::InsufficientDiskSpace
        );
        assert_eq!(
            map_cli_helper_platform_error(Some("tool_candidate_conflict")),
            AgentReasonCode::CandidateConflict
        );
        assert_eq!(
            map_cli_helper_platform_error(Some("tool_target_changed")),
            AgentReasonCode::TargetChanged
        );
        assert_eq!(
            map_cli_helper_platform_error(Some("grok_tool_execution_failed")),
            AgentReasonCode::InteractiveUserUnavailable
        );
    }
}
