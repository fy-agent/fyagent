//! Ordinary-user Claude execution, composed from the shared authenticated
//! helper session, closed discovery and bounded process/registry plan owners.

use super::*;
use fyagent_user_helper::claude::{installation_owner, ClaudeOwner, CLAUDE_MIN_NODE_MAJOR};
use fyagent_user_helper::grok_npm::OfficialNpmTool;

#[derive(Debug, PartialEq, Eq)]
struct Candidate {
    entry: PathBuf,
    binary: PathBuf,
    owner: ClaudeOwner,
}

fn observe_candidate() -> Result<Option<Candidate>, HelperErrorCode> {
    let mut selected: Option<Candidate> = None;
    let mut paths = discover_tool_paths(OfficialNpmTool::Claude)?;
    if let Some(npm) = find_path_program(&["npm.cmd", "npm.exe"]) {
        if let Ok(prefix) = npm_prefix(&npm) {
            let entry = prefix.join("claude.cmd");
            if entry.is_file() {
                paths.push(entry);
            }
        }
    }
    for entry in paths {
        let directory = entry.parent().ok_or(HelperErrorCode::ToolOwnerMismatch)?;
        let npm_binary = directory
            .join("node_modules")
            .join("@anthropic-ai")
            .join("claude-code")
            .join("bin")
            .join("claude.exe");
        let binary = if npm_binary.is_file() {
            npm_binary
        } else {
            entry.clone()
        };
        let binary =
            std::fs::canonicalize(&binary).map_err(|_| HelperErrorCode::ToolOwnerMismatch)?;
        let path_text = entry.to_string_lossy();
        let owner = installation_owner(
            &path_text,
            &binary.to_string_lossy(),
            infer_source_marker(&path_text),
        );
        if owner == ClaudeOwner::External {
            return Err(HelperErrorCode::ToolOwnerMismatch);
        }
        if let Some(previous) = &selected {
            if previous.binary != binary || previous.owner != owner {
                return Err(HelperErrorCode::ToolOwnerMismatch);
            }
        } else {
            selected = Some(Candidate {
                entry,
                binary,
                owner,
            });
        }
    }
    Ok(selected)
}

fn version(candidate: &Candidate) -> Result<String, HelperErrorCode> {
    let (output, _) = run_grok_binary(&candidate.binary, &["--version"], grok_version_timeout())?;
    parse_normalized_version(&output).ok_or(HelperErrorCode::ToolNotDetected)
}

fn npm_prefix(npm: &Path) -> Result<PathBuf, HelperErrorCode> {
    let (output, _) = run_grok_binary(npm, &["prefix", "-g"], grok_version_timeout())?;
    let text = output.trim();
    if text.is_empty() || text.chars().any(char::is_control) {
        return Err(HelperErrorCode::ToolHostMissing);
    }
    let path = PathBuf::from(text);
    validate_ordinary_dos_path(&path).map_err(|_| HelperErrorCode::ToolOwnerMismatch)?;
    Ok(path)
}

pub(super) fn execute(
    action: GrokToolAction,
    plan: Option<GrokNpmInstallPlan>,
) -> Result<ToolOperationResult, HelperErrorCode> {
    execute_inner(action, plan)
}

fn execute_inner(
    action: GrokToolAction,
    plan: Option<GrokNpmInstallPlan>,
) -> Result<ToolOperationResult, HelperErrorCode> {
    let before = observe_candidate()?;
    if action == GrokToolAction::Observe {
        return Ok(ToolOperationResult::observed(
            before.is_some(),
            before.as_ref().map(|item| match item.owner {
                ClaudeOwner::Npm => GrokOwner::Npm,
                ClaudeOwner::Native | ClaudeOwner::External => GrokOwner::Native,
            }),
            before.as_ref().and_then(|item| version(item).ok()),
        ));
    }
    let plan = plan.ok_or(HelperErrorCode::ToolExecutionFailed)?;
    match (action, before.as_ref()) {
        (GrokToolAction::Install, None) => {}
        (GrokToolAction::Update, Some(item)) if item.owner == ClaudeOwner::Npm => {
            if version(item).is_ok_and(|local| version_is_at_least(&local, plan.version())) {
                let mut result =
                    ToolOperationResult::observed(true, Some(GrokOwner::Npm), Some(version(item)?));
                result.outcome = fyagent_user_helper::GrokOutcome::NoChange;
                return Ok(result);
            }
        }
        (GrokToolAction::Update, None) => return Err(HelperErrorCode::ToolNotDetected),
        _ => {
            return Err(HelperErrorCode::ToolOwnerMismatch);
        }
    }
    let npm = find_path_program(&["npm.cmd", "npm.exe"]).ok_or(HelperErrorCode::ToolHostMissing)?;
    let node = npm
        .parent()
        .map(|directory| directory.join("node.exe"))
        .filter(|path| path.is_file())
        .or_else(|| find_path_program(&["node.exe"]))
        .ok_or(HelperErrorCode::ToolHostMissing)?;
    let (node_version, _) = run_grok_binary(&node, &["--version"], grok_version_timeout())?;
    if parse_npm_major(node_version.trim().trim_start_matches('v'))
        .is_none_or(|major| major < CLAUDE_MIN_NODE_MAJOR)
    {
        return Err(HelperErrorCode::ToolHostMissing);
    }
    let (node_arch, _) = run_grok_binary(&node, &["-p", "process.arch"], grok_version_timeout())?;
    let expected_arch = match std::env::consts::ARCH {
        "aarch64" => "arm64",
        "x86_64" => "x64",
        _ => return Err(HelperErrorCode::ToolHostMissing),
    };
    if node_arch.trim() != expected_arch {
        return Err(HelperErrorCode::ToolHostMissing);
    }
    let prefix = npm_prefix(&npm)?;
    if let Some(before) = &before {
        let actual =
            std::fs::canonicalize(&prefix).map_err(|_| HelperErrorCode::ToolOwnerMismatch)?;
        let expected = before
            .entry
            .parent()
            .and_then(|directory| std::fs::canonicalize(directory).ok())
            .ok_or(HelperErrorCode::ToolOwnerMismatch)?;
        if actual != expected {
            return Err(HelperErrorCode::ToolOwnerMismatch);
        }
    }
    if observe_candidate()? != before {
        return Err(HelperErrorCode::ToolOwnerMismatch);
    }
    if let Err(code) = execute_npm_plan(&npm, OfficialNpmTool::Claude, &plan) {
        return Err(code);
    }
    // Inspect the admitted npm prefix even when a newly installed launcher has
    // not yet appeared on a reopened terminal's PATH.
    let binary = prefix
        .join("node_modules")
        .join("@anthropic-ai")
        .join("claude-code")
        .join("bin")
        .join("claude.exe");
    let after = Candidate {
        entry: prefix.join("claude.cmd"),
        binary,
        owner: ClaudeOwner::Npm,
    };
    let actual = version(&after)?;
    if actual != plan.version() {
        return Err(HelperErrorCode::ToolExecutionFailed);
    }
    let mut result = ToolOperationResult::observed(true, Some(GrokOwner::Npm), Some(actual));
    result.outcome = if action == GrokToolAction::Install {
        fyagent_user_helper::GrokOutcome::Installed
    } else {
        fyagent_user_helper::GrokOutcome::Updated
    };
    Ok(result)
}
