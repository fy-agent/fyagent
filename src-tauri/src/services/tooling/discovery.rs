use super::*;

/// One read-only installation-distribution report. The owning module keeps
/// diagnostic conflict classification and update-plan projection together so
/// callers do not re-derive policy from raw installations.
#[derive(Debug, serde::Serialize)]
pub struct ToolInstallationReport {
    tool: String,
    installs: Vec<ToolInstallation>,
    is_conflict: bool,
    needs_confirmation: bool,
    command: String,
    anchored: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    distribution_owner: Option<String>,
}

pub(super) fn plan_command_for(tool: &str, installs: &[ToolInstallation]) -> (String, bool, bool) {
    if !is_lifecycle_writable(tool) {
        return (String::new(), false, false);
    }

    match installs_anchored_command(tool, installs) {
        Some(command) => (command, installs.len() >= 2, true),
        None => (static_fallback_command(tool), installs.len() >= 2, false),
    }
}

pub(super) fn is_conflicting(installs: &[ToolInstallation]) -> bool {
    if installs.len() < 2 {
        return false;
    }
    let distinct_versions: std::collections::HashSet<&Option<String>> = installs
        .iter()
        .map(|installation| &installation.version)
        .collect();
    let runnable_mixed = installs.iter().any(|installation| installation.runnable)
        && installs.iter().any(|installation| !installation.runnable);
    distinct_versions.len() > 1 || runnable_mixed
}

pub async fn probe_tool_installations(
    tools: Vec<String>,
) -> Result<Vec<ToolInstallationReport>, String> {
    if elevated_windows_cli_boundary_active() {
        return Err(ELEVATED_WINDOWS_CLI_BOUNDARY_MESSAGE.to_string());
    }

    let requested = normalize_requested_tools(&tools);
    if requested.is_empty() {
        return Err("No supported tools selected".to_string());
    }

    tokio::task::spawn_blocking(move || {
        requested
            .into_iter()
            .map(|tool| {
                let installs = enumerate_tool_installations(tool);
                let (command, needs_confirmation, anchored) = plan_command_for(tool, &installs);
                let is_conflict = is_conflicting(&installs);
                let mut distribution_owner = None;
                #[cfg(target_os = "macos")]
                if tool == "grok" {
                    distribution_owner =
                        super::grok::grok_owner_wire_from_disk(&installs).map(str::to_string);
                }
                ToolInstallationReport {
                    tool: tool.to_string(),
                    installs,
                    is_conflict,
                    needs_confirmation,
                    command,
                    anchored,
                    distribution_owner,
                }
            })
            .collect()
    })
    .await
    .map_err(|e| format!("probe task join error: {e}"))
}

pub(crate) fn run_detected_tool_command_with_timeout(
    tool: &str,
    args: &[&str],
    timeout: Option<std::time::Duration>,
    extra_env: &[(&str, String)],
    working_dir: &Path,
) -> Result<std::process::Output, String> {
    run_detected_tool_command_with_timeout_impl(tool, args, timeout, None, extra_env, working_dir)
}

/// Executes one closed Tooling command while retaining at most `output_limit`
/// bytes from each output stream. The child streams are still drained so an
/// overlong diagnostic cannot deadlock the process; overflow fails with one
/// stable, path-free error instead of returning truncated data to a parser.
pub(crate) fn run_detected_tool_command_with_timeout_and_output_limit(
    tool: &str,
    args: &[&str],
    timeout: Option<std::time::Duration>,
    output_limit: usize,
    extra_env: &[(&str, String)],
    working_dir: &Path,
) -> Result<std::process::Output, String> {
    if output_limit == 0 {
        return Err("Command output limit is invalid".to_string());
    }
    run_detected_tool_command_with_timeout_impl(
        tool,
        args,
        timeout,
        Some(output_limit),
        extra_env,
        working_dir,
    )
}

fn run_detected_tool_command_with_timeout_impl(
    tool: &str,
    args: &[&str],
    timeout: Option<std::time::Duration>,
    output_limit: Option<usize>,
    extra_env: &[(&str, String)],
    working_dir: &Path,
) -> Result<std::process::Output, String> {
    #[cfg(target_os = "windows")]
    detected_tool_execution_boundary_for(crate::windows_runtime::formal_windows_build())
        .map_err(str::to_owned)?;

    if !VALID_TOOLS.contains(&tool) {
        return Err(format!("Unsupported tool: {tool}"));
    }
    if args.iter().any(|arg| {
        arg.is_empty()
            || !arg
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
    }) {
        return Err("Invalid tool command arguments".to_string());
    }

    let deadline = CommandDeadline::from_timeout(timeout);
    let tool_path = locate_default_tool(tool, deadline)?;
    let dir = tool_path
        .parent()
        .ok_or_else(|| format!("Invalid {tool} executable path"))?;

    #[cfg(target_os = "macos")]
    let current_path = std::env::var_os("PATH")
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_default();

    #[cfg(target_os = "windows")]
    {
        run_windows_tool_command_capture(
            &tool_path,
            dir,
            args,
            deadline,
            output_limit,
            extra_env,
            working_dir,
        )
    }

    #[cfg(target_os = "macos")]
    {
        use std::process::{Command, Stdio};

        let mut cmd = Command::new(&tool_path);
        cmd.args(args)
            .env("PATH", format!("{}:{current_path}", dir.display()))
            .current_dir(working_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        apply_extra_env(&mut cmd, extra_env)?;
        isolate_child_process_group(&mut cmd);
        let child = cmd
            .spawn()
            .map_err(|e| format!("Failed to run {tool}: {e}"))?;
        wait_child_output_with_limit(child, deadline, output_limit)
    }
}
