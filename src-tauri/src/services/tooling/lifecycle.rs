use super::*;
use std::str::FromStr;

pub(super) fn is_lifecycle_writable(tool: &str) -> bool {
    tool != "codex"
}

pub(super) fn normalize_requested_tools(tools: &[String]) -> Vec<&'static str> {
    let set: std::collections::HashSet<&str> = tools.iter().map(|s| s.as_str()).collect();
    VALID_TOOLS
        .iter()
        .copied()
        .filter(|tool| set.contains(tool))
        .collect()
}

#[derive(Debug, Clone, Copy)]
pub(super) enum ToolLifecycleAction {
    Install,
    Update,
}

impl FromStr for ToolLifecycleAction {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "install" => Ok(Self::Install),
            "update" => Ok(Self::Update),
            _ => Err(format!("Unsupported tool action: {value}")),
        }
    }
}

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
pub(super) const CLAUDE_INSTALL_UNIX: &str =
    "bash -c 'tmp=$(mktemp) && curl -fsSL https://claude.ai/install.sh -o $tmp && bash $tmp; status=$?; rm -f $tmp; exit $status'";
#[cfg(target_os = "macos")]
pub(super) const OPENCODE_INSTALL_UNIX: &str =
    "bash -c 'tmp=$(mktemp) && curl -fsSL https://opencode.ai/install -o $tmp && bash $tmp; status=$?; rm -f $tmp; exit $status'";
#[cfg(target_os = "macos")]
pub(super) const GROK_INSTALL_UNIX: &str =
    "bash -c 'tmp=$(mktemp) && curl -fsSL https://x.ai/cli/install.sh -o $tmp && bash $tmp; status=$?; rm -f $tmp; exit $status'";
pub(super) const HERMES_INSTALL_UNIX: &str =
    "bash -c 'tmp=$(mktemp) && curl -fsSL https://raw.githubusercontent.com/NousResearch/hermes-agent/main/scripts/install.sh -o $tmp && bash $tmp; status=$?; rm -f $tmp; exit $status'";
const HERMES_UPDATE_UNIX: &str =
    "hermes update || bash -c 'tmp=$(mktemp) && curl -fsSL https://raw.githubusercontent.com/NousResearch/hermes-agent/main/scripts/install.sh -o $tmp && bash $tmp; status=$?; rm -f $tmp; exit $status'";

#[cfg(target_os = "windows")]
const HERMES_INSTALL_WINDOWS_SCRIPT: &str =
    "irm https://raw.githubusercontent.com/NousResearch/hermes-agent/main/scripts/install.ps1 | iex";
#[cfg(target_os = "windows")]
const GROK_INSTALL_WINDOWS_SCRIPT: &str = "irm https://x.ai/cli/install.ps1 | iex";

#[cfg(target_os = "windows")]
fn powershell_encoded_command(script: &str) -> String {
    use base64::{engine::general_purpose::STANDARD, Engine as _};

    let mut bytes = Vec::with_capacity(script.len() * 2);
    for unit in script.encode_utf16() {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    STANDARD.encode(bytes)
}

#[cfg(target_os = "windows")]
fn hermes_install_windows_command() -> String {
    format!(
        "powershell -NoProfile -ExecutionPolicy Bypass -EncodedCommand {}",
        powershell_encoded_command(HERMES_INSTALL_WINDOWS_SCRIPT)
    )
}

#[cfg(target_os = "windows")]
pub(super) fn grok_install_windows_command() -> String {
    format!(
        "powershell -NoProfile -ExecutionPolicy Bypass -EncodedCommand {}",
        powershell_encoded_command(GROK_INSTALL_WINDOWS_SCRIPT)
    )
}

#[cfg(target_os = "windows")]
fn hermes_update_windows_command() -> String {
    format!("hermes update || {}", hermes_install_windows_command())
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
        "claude" => Some("npm i -g @anthropic-ai/claude-code@latest"),
        "gemini" => Some("npm i -g @google/gemini-cli@latest"),
        "grok" => Some("npm i -g @xai-official/grok@latest"),
        "opencode" => Some("npm i -g opencode-ai@latest"),
        "openclaw" => Some("npm i -g openclaw@latest"),
        _ => None,
    }
}

pub(super) fn official_update_args(tool: &str) -> Option<&'static str> {
    match tool {
        "claude" | "grok" | "hermes" => Some("update"),
        "openclaw" => Some("update --yes"),
        "opencode" => Some("upgrade"),
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
        return Some(chain_update_commands(
            grok_install_windows_command(),
            npm_install_command_for(tool)?.to_string(),
            shell,
        ));
    }

    if tool == "hermes" {
        return Some(
            match (action, shell) {
                (ToolLifecycleAction::Install, LifecycleCommandShell::Posix) => HERMES_INSTALL_UNIX,
                (ToolLifecycleAction::Update, LifecycleCommandShell::Posix) => HERMES_UPDATE_UNIX,
                #[cfg(target_os = "windows")]
                (ToolLifecycleAction::Install, LifecycleCommandShell::WindowsBatch) => {
                    return Some(hermes_install_windows_command());
                }
                #[cfg(target_os = "windows")]
                (ToolLifecycleAction::Update, LifecycleCommandShell::WindowsBatch) => {
                    return Some(hermes_update_windows_command());
                }
                #[cfg(target_os = "macos")]
                (_, LifecycleCommandShell::WindowsBatch) => return None,
            }
            .to_string(),
        );
    }

    let install = npm_install_command_for(tool)?;
    match action {
        ToolLifecycleAction::Install => Some(install.to_string()),
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
    fn grok_windows_install_prefers_powershell_with_npm_fallback() {
        let install = static_fallback_command_for("grok", ToolLifecycleAction::Install);
        let native = grok_install_windows_command();
        assert!(
            install.starts_with(&native),
            "native installer first: {install}"
        );
        assert!(
            install.ends_with("|| call npm i -g @xai-official/grok@latest"),
            "npm fallback should remain available: {install}"
        );
        let expected_encoded = powershell_encoded_command(GROK_INSTALL_WINDOWS_SCRIPT);
        assert_eq!(
            native
                .split_once("-EncodedCommand ")
                .map(|(_, encoded)| encoded),
            Some(expected_encoded.as_str())
        );
    }

    #[test]
    fn hermes_windows_static_fallback_uses_powershell_installer_without_pip() {
        let install = static_fallback_command_for("hermes", ToolLifecycleAction::Install);
        assert!(
            install.starts_with("powershell -NoProfile -ExecutionPolicy Bypass -EncodedCommand "),
            "should use PowerShell EncodedCommand installer: {install}"
        );
        let encoded = install
            .split_once("-EncodedCommand ")
            .map(|(_, encoded)| encoded)
            .expect("installer should include encoded command");
        assert_eq!(
            encoded,
            powershell_encoded_command(HERMES_INSTALL_WINDOWS_SCRIPT)
        );
        let install_prefix = install
            .split_once("-EncodedCommand ")
            .map(|(prefix, _)| prefix)
            .expect("installer should include encoded command");
        assert!(
            !install_prefix.contains("|")
                && !install_prefix.contains("-Command")
                && !install_prefix.contains("python")
                && !install_prefix.contains("pip"),
            "should hide PowerShell pipe from cmd.exe and avoid system Python/pip: {install}"
        );

        let update = static_fallback_command("hermes");
        assert!(
            update.starts_with(
                "hermes update || powershell -NoProfile -ExecutionPolicy Bypass -EncodedCommand "
            ),
            "should try CLI update before PowerShell installer: {update}"
        );
        let fallback = update
            .split_once("||")
            .map(|(_, fallback)| fallback)
            .expect("update should include a fallback command");
        let fallback_prefix = fallback
            .split_once("-EncodedCommand ")
            .map(|(prefix, _)| prefix)
            .expect("fallback should include encoded command");
        assert!(
            !fallback_prefix.contains('|')
                && !fallback_prefix.contains("-Command")
                && !update.contains("call powershell")
                && !fallback_prefix.contains("python")
                && !fallback_prefix.contains("pip"),
            "PowerShell fallback should be encoded, not called like a batch file or use pip: {update}"
        );
    }
}
