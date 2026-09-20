//! Read-only CLI host checks shared by install confirmation and execution.

use crate::{agent_install::AgentReasonCode, services::external_agents::AgentCatalogId};

pub(crate) struct CliInstallPreflight {
    pub paths: Vec<std::path::PathBuf>,
    pub location_label: String,
    pub requires_npm: bool,
}

#[cfg(target_os = "macos")]
pub(super) async fn check(
    agent: AgentCatalogId,
    action: crate::agent_install::AgentActionId,
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
        let prefix = super::npm_runtime::prefix(anchor).ok_or(AgentReasonCode::ToolHostMissing)?;
        let writable_prefix = crate::agent_install::existing_directory(&prefix)?;
        if !crate::agent_install::directory_writable(&writable_prefix) {
            return Err(AgentReasonCode::PermissionDenied);
        }
        let cache_path =
            super::npm_runtime::cache(anchor).ok_or(AgentReasonCode::ToolHostMissing)?;
        let writable_cache = crate::agent_install::existing_directory(&cache_path)?;
        if !crate::agent_install::directory_writable(&writable_cache) {
            return Err(AgentReasonCode::PermissionDenied);
        }
        if agent == AgentCatalogId::GrokBuild {
            for candidate in [
                prefix
                    .join("lib")
                    .join("node_modules")
                    .join("@iarna")
                    .join("toml")
                    .join("package.json"),
                prefix
                    .join("node_modules")
                    .join("@iarna")
                    .join("toml")
                    .join("package.json"),
            ] {
                if candidate.exists() {
                    if let Ok(content) = std::fs::read_to_string(&candidate) {
                        if let Some(pos) = content.find("\"version\"") {
                            let snippet = &content[pos..pos.min(pos + 40)];
                            if !snippet.contains("3.0.0") {
                                return Err(AgentReasonCode::CandidateConflict);
                            }
                        }
                    }
                }
            }
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
        Ok(CliInstallPreflight {
            location_label: crate::agent_install::display_user_path(&prefix),
            paths,
            requires_npm: true,
        })
    })
    .await
    .map_err(|_| AgentReasonCode::ToolHostMissing)?
}

#[cfg(target_os = "windows")]
pub(super) async fn check(
    agent: AgentCatalogId,
    action: crate::agent_install::AgentActionId,
) -> Result<CliInstallPreflight, AgentReasonCode> {
    use crate::codex_desktop::platform::windows::{
        run_claude_tool_operation, run_grok_tool_operation,
    };
    use fyagent_user_helper::{GrokOwner, GrokToolAction};
    let observed = tokio::task::spawn_blocking(move || match agent {
        AgentCatalogId::ClaudeCode => run_claude_tool_operation(
            crate::windows_runtime::require_interactive_user_context(),
            GrokToolAction::Preflight,
            None,
        ),
        AgentCatalogId::GrokBuild => run_grok_tool_operation(
            crate::windows_runtime::require_interactive_user_context(),
            GrokToolAction::Preflight,
            None,
            None,
        ),
        _ => unreachable!("only policy-admitted CLI agents reach preflight"),
    })
    .await
    .map_err(|_| AgentReasonCode::InteractiveUserUnavailable)?
    .map_err(
        |error| match error.to_dto().details.platform_error_code.as_deref() {
            Some("tool_permission_denied") => AgentReasonCode::PermissionDenied,
            Some("grok_tool_host_missing") => AgentReasonCode::ToolHostMissing,
            Some("grok_tool_owner_mismatch") => AgentReasonCode::ToolOwnerUnsupported,
            _ => AgentReasonCode::InteractiveUserUnavailable,
        },
    )?;
    if (action == crate::agent_install::AgentActionId::Install && observed.detected)
        || (action == crate::agent_install::AgentActionId::Update && !observed.detected)
    {
        return Err(AgentReasonCode::TargetChanged);
    }
    let native = observed.owner == Some(GrokOwner::Native);
    if agent == AgentCatalogId::ClaudeCode && native {
        return Err(AgentReasonCode::ToolOwnerUnsupported);
    }
    Ok(CliInstallPreflight {
        paths: vec![
            crate::windows_runtime::user_local_app_data_dir(),
            crate::windows_runtime::user_roaming_app_data_dir(),
        ],
        location_label: if native {
            "当前桌面用户的 CLI 安装位置"
        } else {
            "当前桌面用户的 npm 全局目录"
        }
        .to_string(),
        requires_npm: !native,
    })
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub(super) async fn check(
    _agent: AgentCatalogId,
    _action: crate::agent_install::AgentActionId,
) -> Result<CliInstallPreflight, AgentReasonCode> {
    Err(AgentReasonCode::PlatformUnsupported)
}
