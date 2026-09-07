//! Shared macOS npm execution through the existing bounded Tooling process
//! owner. Package identities and registries stay in the closed npm plan.

use std::process::{Command, Output, Stdio};
use std::time::Duration;

use fyagent_user_helper::grok_npm::{GrokNpmInstallPlan, OfficialNpmTool};

use super::{
    isolate_child_process_group, login_shell_path, merge_path_segments,
    wait_child_output_with_limit, CommandDeadline,
};

const OUTPUT_LIMIT: usize = 32 * 1024;

pub(super) fn execution_path() -> Option<String> {
    let inherited = std::env::var("PATH").unwrap_or_default();
    login_shell_path().map(|login| merge_path_segments(&login, &inherited))
}

pub(super) fn run_command(command: &str, timeout: Duration) -> Result<Output, String> {
    let mut cmd = Command::new("/bin/bash");
    cmd.arg("-c")
        .arg(command)
        .current_dir(crate::config::get_home_dir())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(path) = execution_path() {
        cmd.env("PATH", path);
    }
    isolate_child_process_group(&mut cmd);
    let child = cmd.spawn().map_err(|_| "无法启动 npm 进程".to_string())?;
    wait_child_output_with_limit(
        child,
        CommandDeadline::from_timeout(Some(timeout)),
        Some(OUTPUT_LIMIT),
    )
}

pub(super) fn npm_major(bin_path: Option<&str>) -> Option<u32> {
    let command = match bin_path {
        Some(path) => super::anchored_npm_command(path, "--version")?,
        None => "npm --version".to_string(),
    };
    let output = run_command(&command, Duration::from_secs(20)).ok()?;
    if !output.status.success() {
        return None;
    }
    fyagent_user_helper::grok_npm::parse_npm_major(&super::decode_command_output(&output.stdout))
}

pub(super) fn prefix(bin_path: Option<&str>) -> Option<std::path::PathBuf> {
    let command = match bin_path {
        Some(path) => super::anchored_npm_command(path, "prefix -g")?,
        None => "npm prefix -g".to_string(),
    };
    let output = run_command(&command, Duration::from_secs(20)).ok()?;
    if !output.status.success() {
        return None;
    }
    let text = super::decode_command_output(&output.stdout);
    let text = text.trim();
    if text.chars().any(char::is_control) {
        return None;
    }
    let path = std::path::PathBuf::from(text);
    path.is_absolute().then_some(path)
}

pub(super) fn node_major(bin_path: Option<&str>) -> Option<u32> {
    let command = match bin_path {
        Some(path) => {
            let directory = std::path::Path::new(path).parent()?;
            format!(
                "PATH={}:\"$PATH\" node --version",
                super::shell_single_quote(&directory.to_string_lossy())
            )
        }
        None => "node --version".to_string(),
    };
    let output = run_command(&command, Duration::from_secs(20)).ok()?;
    if !output.status.success() {
        return None;
    }
    let version = super::decode_command_output(&output.stdout);
    semver::Version::parse(version.trim().strip_prefix('v').unwrap_or(version.trim()))
        .ok()
        .map(|version| version.major)
        .and_then(|major| u32::try_from(major).ok())
}

pub(super) fn node_matches_host(bin_path: Option<&str>) -> bool {
    let prefix = bin_path
        .and_then(|path| std::path::Path::new(path).parent())
        .map(|directory| {
            format!(
                "PATH={}:\"$PATH\" ",
                super::shell_single_quote(&directory.to_string_lossy())
            )
        })
        .unwrap_or_default();
    let Ok(output) = run_command(
        &format!("{prefix}node -p process.arch"),
        Duration::from_secs(20),
    ) else {
        return false;
    };
    let expected = match std::env::consts::ARCH {
        "aarch64" => "arm64",
        "x86_64" => "x64",
        _ => return false,
    };
    output.status.success() && super::decode_command_output(&output.stdout).trim() == expected
}

pub(super) fn install(
    tool: OfficialNpmTool,
    bin_path: Option<&str>,
    plan: &GrokNpmInstallPlan,
) -> Result<Output, String> {
    let args = plan.npm_argv_for(tool).join(" ");
    let command = match bin_path {
        Some(path) => super::anchored_npm_command(path, &args)
            .ok_or_else(|| "无法找到此安装使用的 npm".to_string())?,
        None => format!("npm {args}"),
    };
    let command = if tool == OfficialNpmTool::Grok {
        format!(
            "{}={} {command}",
            fyagent_user_helper::GROK_NPM_REGISTRY_ENV,
            super::shell_single_quote(plan.registry_url())
        )
    } else {
        command
    };
    run_command(&command, Duration::from_secs(300))
}
