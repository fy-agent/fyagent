//! Claude CLI lifecycle. The official npm package is the only fresh-install
//! source; updates never change an existing distribution owner.

use super::grok_npm;
use super::lifecycle::ToolLifecycleAction;
#[cfg(target_os = "macos")]
use super::npm_runtime;
use super::ToolVersion;
#[cfg(target_os = "macos")]
use fyagent_user_helper::claude::CLAUDE_MIN_NODE_MAJOR;
#[cfg(any(target_os = "macos", test))]
use fyagent_user_helper::claude::{installation_owner, ClaudeOwner};
#[cfg(any(target_os = "macos", target_os = "windows"))]
use fyagent_user_helper::grok_npm::OfficialNpmTool;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ClaudeLifecycleError {
    UnsupportedAction,
    OperationConflict,
    HostMissing,
    OwnerUnsupported,
    SourceUnverified,
    ExecutionFailed,
    VerificationFailed,
}

impl ClaudeLifecycleError {
    pub(super) const fn message(self) -> &'static str {
        match self {
            Self::UnsupportedAction => "Claude Code 的此项 CLI 操作暂不可用。",
            Self::OperationConflict => "另一个 CLI 安装或更新正在进行。",
            Self::HostMissing => {
                "Claude Code 安装需要 Node.js 22 或更高版本及 npm，请先检查安装依赖。"
            }
            Self::OwnerUnsupported => {
                "当前 Claude Code 不是可确认的 npm 安装，请使用原安装方式更新；未更换安装来源。"
            }
            Self::SourceUnverified => "暂时无法从官方 npm 获取可用的 Claude Code 版本。",
            Self::ExecutionFailed => "Claude Code 安装未完成，请检查网络及当前用户的安装权限。",
            Self::VerificationFailed => "无法确认 Claude Code 已安装到指定版本，请刷新安装状态。",
        }
    }
}

#[cfg(target_os = "macos")]
pub(super) async fn version() -> ToolVersion {
    let observed = tokio::task::spawn_blocking(observe).await;
    let (version, error, broken, owner) = match observed {
        Ok(Ok(Some(install))) => {
            let owner = match install.owner {
                ClaudeOwner::Npm => "official_npm",
                ClaudeOwner::Native => "native_internal",
                ClaudeOwner::External => "external",
            };
            let broken = install.version.is_none();
            (
                install.version,
                broken.then(|| "Claude Code is installed but not runnable".to_string()),
                broken,
                Some(owner.to_string()),
            )
        }
        Ok(Ok(None)) => (
            None,
            Some("Claude Code is not installed".to_string()),
            false,
            None,
        ),
        _ => (
            None,
            Some("Claude Code installation ownership is unavailable".to_string()),
            false,
            None,
        ),
    };
    let latest_version = super::versions::fetch_npm_latest_for_tool(
        &crate::proxy::http_client::get(),
        fyagent_user_helper::claude::CLAUDE_NPM_PACKAGE,
        "claude",
        version.as_deref(),
    )
    .await;
    ToolVersion {
        name: "claude".to_string(),
        version,
        error,
        latest_version,
        installed_but_broken: broken,
        distribution_owner: owner,
        latest_source: Some("npm".to_string()),
    }
}

#[cfg(target_os = "windows")]
pub(super) async fn version() -> ToolVersion {
    let observed = windows_operation(fyagent_user_helper::GrokToolAction::Observe, None).await;
    match observed {
        Ok(observed) => {
            let latest_version = super::versions::fetch_npm_latest_for_tool(
                &crate::proxy::http_client::get(),
                fyagent_user_helper::claude::CLAUDE_NPM_PACKAGE,
                "claude",
                observed.normalized_version.as_deref(),
            )
            .await;
            ToolVersion {
                name: "claude".to_string(),
                installed_but_broken: observed.detected && observed.normalized_version.is_none(),
                error: (!observed.detected).then(|| "Claude Code is not installed".to_string()),
                version: observed.normalized_version,
                latest_version,
                distribution_owner: observed.owner.map(|owner| owner.as_str().to_string()),
                latest_source: Some("npm".to_string()),
            }
        }
        Err(error) => ToolVersion {
            name: "claude".to_string(),
            version: None,
            latest_version: super::versions::fetch_npm_latest_for_tool(
                &crate::proxy::http_client::get(),
                fyagent_user_helper::claude::CLAUDE_NPM_PACKAGE,
                "claude",
                None,
            )
            .await,
            error: Some(error.message().to_string()),
            installed_but_broken: false,
            distribution_owner: None,
            latest_source: None,
        },
    }
}

#[cfg(target_os = "macos")]
#[derive(Debug, Clone, PartialEq, Eq)]
struct Installation {
    path: String,
    real: std::path::PathBuf,
    owner: ClaudeOwner,
    version: Option<String>,
}

#[cfg(target_os = "macos")]
fn observe() -> Result<Option<Installation>, ClaudeLifecycleError> {
    let installs = super::enumerate_tool_installations("claude");
    match super::default_install(&installs) {
        Some(install) => Ok(Some(Installation {
            owner: installation_owner(
                &install.path,
                &install.real.to_string_lossy(),
                &install.source,
            ),
            path: install.path.clone(),
            real: install.real.clone(),
            version: install.version.clone(),
        })),
        None if installs.is_empty() => Ok(None),
        None => Err(ClaudeLifecycleError::OwnerUnsupported),
    }
}

#[cfg(target_os = "macos")]
pub(super) async fn run(action: ToolLifecycleAction) -> Result<(), ClaudeLifecycleError> {
    if !matches!(
        action,
        ToolLifecycleAction::Install | ToolLifecycleAction::Update
    ) {
        return Err(ClaudeLifecycleError::UnsupportedAction);
    }
    let before = tokio::task::spawn_blocking(observe)
        .await
        .map_err(|_| ClaudeLifecycleError::ExecutionFailed)??;
    let manifest = grok_npm::resolve_published_manifest(OfficialNpmTool::Claude)
        .await
        .map_err(|_| ClaudeLifecycleError::SourceUnverified)?;
    if let Some(installed) = &before {
        if action == ToolLifecycleAction::Install || installed.owner != ClaudeOwner::Npm {
            return Err(ClaudeLifecycleError::OwnerUnsupported);
        }
        if installed
            .version
            .as_deref()
            .and_then(|local| super::versions::compare_semver(local, manifest.version()))
            .is_some_and(|order| !order.is_lt())
        {
            return Ok(());
        }
    } else if action == ToolLifecycleAction::Update {
        return Err(ClaudeLifecycleError::VerificationFailed);
    }
    let matching = grok_npm::registries_matching_manifest(&manifest).await;
    if matching.is_empty() {
        return Err(ClaudeLifecycleError::SourceUnverified);
    }
    tokio::task::spawn_blocking(move || {
        // Recheck after the network wait; a changed install owner or path must
        // not become permission to update whichever CLI is now on PATH.
        if observe()? != before {
            return Err(ClaudeLifecycleError::OwnerUnsupported);
        }
        let anchor = before.as_ref().map(|install| install.path.as_str());
        let npm_major = npm_runtime::npm_major(anchor).ok_or(ClaudeLifecycleError::HostMissing)?;
        if npm_runtime::node_major(anchor).is_none_or(|major| major < CLAUDE_MIN_NODE_MAJOR)
            || !npm_runtime::node_matches_host(anchor)
        {
            return Err(ClaudeLifecycleError::HostMissing);
        }
        let prefix = npm_runtime::prefix(anchor).ok_or(ClaudeLifecycleError::HostMissing)?;
        let bin_directory = prefix.join("bin");
        if let Some(anchor) = anchor {
            let expected = std::path::Path::new(anchor)
                .parent()
                .and_then(|path| path.canonicalize().ok());
            if expected.is_none() || expected != bin_directory.canonicalize().ok() {
                return Err(ClaudeLifecycleError::OwnerUnsupported);
            }
        } else if !npm_runtime::execution_path()
            .is_some_and(|path| std::env::split_paths(&path).any(|entry| entry == bin_directory))
        {
            // Do not silently edit a shell profile to make a new global prefix
            // visible. An undiscoverable install is not an actionable success.
            return Err(ClaudeLifecycleError::OwnerUnsupported);
        }
        for registry in matching {
            let plan = grok_npm::plan_for_registry(
                &manifest,
                registry,
                fyagent_user_helper::grok_npm::npm_major_allows_scripts(npm_major),
            )
            .map_err(|_| ClaudeLifecycleError::SourceUnverified)?;
            let output = npm_runtime::install(OfficialNpmTool::Claude, anchor, &plan)
                .map_err(|_| ClaudeLifecycleError::ExecutionFailed)?;
            if !output.status.success() {
                continue;
            }
            let after = observe()?.ok_or(ClaudeLifecycleError::VerificationFailed)?;
            if after.owner != ClaudeOwner::Npm
                || after.version.as_deref() != Some(manifest.version())
                || before
                    .as_ref()
                    .is_some_and(|before| before.path != after.path)
            {
                return Err(ClaudeLifecycleError::VerificationFailed);
            }
            return Ok(());
        }
        Err(ClaudeLifecycleError::ExecutionFailed)
    })
    .await
    .map_err(|_| ClaudeLifecycleError::ExecutionFailed)?
}

#[cfg(target_os = "windows")]
pub(super) async fn run(action: ToolLifecycleAction) -> Result<(), ClaudeLifecycleError> {
    use fyagent_user_helper::{GrokOwner, GrokToolAction};
    let action = match action {
        ToolLifecycleAction::Install => GrokToolAction::Install,
        ToolLifecycleAction::Update => GrokToolAction::Update,
        _ => return Err(ClaudeLifecycleError::UnsupportedAction),
    };
    let before = windows_operation(GrokToolAction::Observe, None).await?;
    if (action == GrokToolAction::Install && before.detected)
        || (action == GrokToolAction::Update && before.owner != Some(GrokOwner::Npm))
    {
        return Err(ClaudeLifecycleError::OwnerUnsupported);
    }
    let manifest = grok_npm::resolve_published_manifest(OfficialNpmTool::Claude)
        .await
        .map_err(|_| ClaudeLifecycleError::SourceUnverified)?;
    if action == GrokToolAction::Update
        && before
            .normalized_version
            .as_deref()
            .and_then(|local| super::versions::compare_semver(local, manifest.version()))
            .is_some_and(|order| !order.is_lt())
    {
        return Ok(());
    }
    let matching = grok_npm::registries_matching_manifest(&manifest).await;
    if matching.is_empty() {
        return Err(ClaudeLifecycleError::SourceUnverified);
    }
    for registry in matching {
        let plan = grok_npm::plan_for_registry(&manifest, registry, false)
            .map_err(|_| ClaudeLifecycleError::SourceUnverified)?;
        match windows_operation(action, Some(plan)).await {
            Ok(after)
                if after.detected
                    && after.owner == Some(GrokOwner::Npm)
                    && after
                        .normalized_version
                        .as_deref()
                        .and_then(|local| {
                            super::versions::compare_semver(local, manifest.version())
                        })
                        .is_some_and(|order| !order.is_lt()) =>
            {
                return Ok(())
            }
            Ok(_) => return Err(ClaudeLifecycleError::VerificationFailed),
            Err(ClaudeLifecycleError::ExecutionFailed) => continue,
            Err(error) => return Err(error),
        }
    }
    Err(ClaudeLifecycleError::ExecutionFailed)
}

#[cfg(target_os = "windows")]
async fn windows_operation(
    action: fyagent_user_helper::GrokToolAction,
    plan: Option<fyagent_user_helper::GrokNpmInstallPlan>,
) -> Result<fyagent_user_helper::ToolOperationResult, ClaudeLifecycleError> {
    tokio::task::spawn_blocking(move || {
        crate::codex_desktop::platform::windows::run_claude_tool_operation(
            crate::windows_runtime::require_interactive_user_context(),
            action,
            plan,
        )
        .map_err(
            |error| match error.to_dto().details.platform_error_code.as_deref() {
                Some("grok_tool_host_missing") => ClaudeLifecycleError::HostMissing,
                Some("grok_tool_owner_mismatch") => ClaudeLifecycleError::OwnerUnsupported,
                Some("grok_tool_not_detected") => ClaudeLifecycleError::VerificationFailed,
                Some("grok_tool_execution_failed") => ClaudeLifecycleError::ExecutionFailed,
                // A lost/uncertain helper result must never trigger a second install.
                _ => ClaudeLifecycleError::VerificationFailed,
            },
        )
    })
    .await
    .map_err(|_| ClaudeLifecycleError::VerificationFailed)?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installation_owners_are_not_silently_converted_to_npm() {
        assert_eq!(
            installation_owner(
                "/Users/example/bin/claude",
                "/Users/example/lib/node_modules/@anthropic-ai/claude-code/bin/claude",
                "system"
            ),
            ClaudeOwner::Npm
        );
        assert_eq!(
            installation_owner(
                "/Users/example/.local/bin/claude",
                "/Users/example/.local/share/claude/versions/2.1.261",
                "system"
            ),
            ClaudeOwner::Native
        );
        assert_eq!(
            installation_owner(
                "/opt/bin/claude",
                "/opt/Cellar/claude/2.1/claude",
                "homebrew"
            ),
            ClaudeOwner::External
        );
        assert_eq!(
            installation_owner(
                "/Users/example/.bun/bin/claude",
                "/Users/example/.bun/node_modules/@anthropic-ai/claude-code/bin/claude",
                "bun"
            ),
            ClaudeOwner::External
        );
        assert_eq!(
            installation_owner("/tmp/claude", "/tmp/claude", "system"),
            ClaudeOwner::External
        );
    }
}
