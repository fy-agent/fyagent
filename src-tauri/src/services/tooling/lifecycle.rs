use super::*;
use std::str::FromStr;

pub(super) fn is_lifecycle_writable(tool: &str) -> bool {
    tool == "grok"
}

pub(super) fn normalize_requested_tools(tools: &[String]) -> Vec<&'static str> {
    let set: std::collections::HashSet<&str> = tools.iter().map(|s| s.as_str()).collect();
    VALID_TOOLS
        .iter()
        .copied()
        .filter(|tool| set.contains(tool))
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ToolLifecycleAction {
    Install,
    Update,
    InstallOfficialNpm,
}

impl FromStr for ToolLifecycleAction {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "install" => Ok(Self::Install),
            "update" => Ok(Self::Update),
            "install_official_npm" => Ok(Self::InstallOfficialNpm),
            _ => Err(format!("Unsupported tool action: {value}")),
        }
    }
}

#[cfg(any(test, target_os = "windows"))]
pub(super) fn build_tool_lifecycle_command(
    tools: &[&str],
    action: ToolLifecycleAction,
) -> Result<String, String> {
    let mut lines = Vec::new();

    #[cfg(target_os = "macos")]
    {
        lines.push("set -e".to_string());
        lines.push("set -o pipefail".to_string());
    }

    #[cfg(target_os = "windows")]
    lines.push("@echo off".to_string());

    for tool in tools {
        if !is_lifecycle_writable(tool) {
            return Err(CODEX_CLI_LIFECYCLE_DISABLED_MESSAGE.to_string());
        }

        let label = tool_display_name(tool);
        lines.push(format!("echo ========== {label} =========="));
        lines.push(build_tool_action_line(tool, action)?);

        #[cfg(target_os = "windows")]
        lines.push("if errorlevel 1 exit /b %errorlevel%".to_string());

        #[cfg(target_os = "macos")]
        lines.push(String::new());
    }

    Ok(lines.join(if cfg!(target_os = "windows") {
        "\r\n"
    } else {
        "\n"
    }))
}

#[cfg(any(test, target_os = "windows"))]
pub(super) fn tool_display_name(tool: &str) -> &'static str {
    match tool {
        "claude" => "Claude Code",
        "codex" => "Codex",
        "gemini" => "Gemini CLI",
        "grok" => "Grok Build",
        "opencode" => "OpenCode",
        "openclaw" => "OpenClaw",
        "hermes" => "Hermes",
        _ => "Unknown",
    }
}

#[cfg(target_os = "macos")]
pub(super) const GROK_INSTALL_UNIX: &str =
    "bash -c 'tmp=$(mktemp) && curl -fsSL https://x.ai/cli/install.sh -o $tmp && bash $tmp; status=$?; rm -f $tmp; exit $status'";

#[cfg(target_os = "windows")]
pub(super) fn grok_install_windows_command() -> String {
    fyagent_user_helper::grok::grok_native_windows_powershell_command()
}

#[derive(Debug, Clone, Copy)]
pub(super) enum LifecycleCommandShell {
    #[cfg_attr(target_os = "windows", allow(dead_code))]
    Posix,
    #[cfg_attr(target_os = "macos", allow(dead_code))]
    WindowsBatch,
}

pub(super) fn npm_install_command_for(tool: &str) -> Option<&'static str> {
    if !is_lifecycle_writable(tool) {
        return None;
    }

    match tool {
        "grok" => Some("npm i -g @xai-official/grok@latest"),
        _ => None,
    }
}

pub(super) fn official_update_args(tool: &str) -> Option<&'static str> {
    match tool {
        "grok" => Some("update"),
        _ => None,
    }
}

pub(super) fn bare_official_update_command(tool: &str) -> Option<String> {
    official_update_args(tool).map(|args| format!("{tool} {args}"))
}

pub(super) fn chain_update_commands(
    primary: String,
    fallback: String,
    shell: LifecycleCommandShell,
) -> String {
    if fallback.trim().is_empty() {
        return primary;
    }
    match shell {
        LifecycleCommandShell::Posix => format!("{primary} || {fallback}"),
        LifecycleCommandShell::WindowsBatch => format!("{primary} || call {fallback}"),
    }
}

pub(super) fn tool_action_shell_command_for_shell(
    tool: &str,
    action: ToolLifecycleAction,
    shell: LifecycleCommandShell,
) -> Option<String> {
    if !is_lifecycle_writable(tool) {
        return None;
    }

    #[cfg(target_os = "windows")]
    if tool == "grok"
        && matches!(action, ToolLifecycleAction::Install)
        && matches!(shell, LifecycleCommandShell::WindowsBatch)
    {
        return Some(grok_install_windows_command());
    }

    let install = npm_install_command_for(tool)?;
    match action {
        ToolLifecycleAction::Install => Some(install.to_string()),
        ToolLifecycleAction::InstallOfficialNpm => {
            if tool == "grok" {
                Some(install.to_string())
            } else {
                None
            }
        }
        ToolLifecycleAction::Update => match prefers_official_update(tool, shell)
            .then(|| bare_official_update_command(tool))
            .flatten()
        {
            Some(update) => Some(chain_update_commands(update, install.to_string(), shell)),
            None => Some(install.to_string()),
        },
    }
}

pub(super) fn tool_action_shell_command(tool: &str, action: ToolLifecycleAction) -> Option<String> {
    #[cfg(target_os = "windows")]
    let shell = LifecycleCommandShell::WindowsBatch;
    #[cfg(target_os = "macos")]
    let shell = LifecycleCommandShell::Posix;

    tool_action_shell_command_for_shell(tool, action, shell)
}

#[cfg(any(test, target_os = "windows"))]
fn grok_official_npm_command(tool: &str) -> Result<String, String> {
    if tool != "grok" {
        return Err("install_official_npm is only valid for Grok Build".to_string());
    }
    npm_install_command_for("grok")
        .map(str::to_string)
        .ok_or_else(|| "Official npm install is unavailable for Grok Build".to_string())
}

#[cfg(any(test, target_os = "windows"))]
fn build_tool_action_line(tool: &str, action: ToolLifecycleAction) -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        let command = match action {
            ToolLifecycleAction::Update => {
                let installs = enumerate_tool_installations(tool);
                installs_anchored_command(tool, &installs)
                    .unwrap_or_else(|| static_fallback_command(tool))
            }
            ToolLifecycleAction::Install => {
                static_fallback_command_for(tool, ToolLifecycleAction::Install)
            }
            ToolLifecycleAction::InstallOfficialNpm => grok_official_npm_command(tool)?,
        };
        if command.is_empty() {
            return Err(format!("Unsupported tool action target: {tool}"));
        }
        Ok(format!("call {command}"))
    }

    #[cfg(target_os = "macos")]
    {
        let command = match action {
            ToolLifecycleAction::Update => {
                let installs = enumerate_tool_installations(tool);
                installs_anchored_command(tool, &installs)
                    .unwrap_or_else(|| static_fallback_command(tool))
            }
            ToolLifecycleAction::Install => install_command_for(tool),
            ToolLifecycleAction::InstallOfficialNpm => grok_official_npm_command(tool)?,
        };
        if command.is_empty() {
            return Err(format!("Unsupported tool action target: {tool}"));
        }
        Ok(command)
    }
}

#[cfg(target_os = "windows")]
pub(super) fn win_double_quote(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\\\""))
}

#[cfg(target_os = "windows")]
pub(super) fn windows_cmd_double_quote_arg(value: &str) -> String {
    win_double_quote(value)
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::super::{static_fallback_command, static_fallback_command_for};
    use super::*;

    #[test]
    fn grok_windows_install_uses_official_powershell_without_npm_fallback() {
        let install = static_fallback_command_for("grok", ToolLifecycleAction::Install);
        let native = grok_install_windows_command();
        assert_eq!(install, native);
        assert!(!install.contains("npm"));
        assert_eq!(
            native,
            fyagent_user_helper::grok::grok_native_windows_powershell_command()
        );
    }

    #[test]
    fn non_grok_windows_lifecycle_commands_are_empty() {
        for tool in [
            "claude", "gemini", "opencode", "openclaw", "hermes", "codex",
        ] {
            assert!(
                static_fallback_command_for(tool, ToolLifecycleAction::Install).is_empty(),
                "{tool} install must not construct a command"
            );
            assert!(
                static_fallback_command(tool).is_empty(),
                "{tool} update must not construct a command"
            );
        }
    }
}
