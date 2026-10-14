use std::path::PathBuf;

use crate::app_config::AppType;
use crate::codex_config::get_codex_auth_path;
use crate::config::get_claude_settings_path;
use crate::error::AppError;
use crate::gemini_config::get_gemini_dir;
use crate::openclaw_config::get_openclaw_dir;
use crate::opencode_config::get_opencode_dir;

/// 返回指定应用所使用的提示词文件路径。
pub fn prompt_file_path(app: &AppType) -> Result<PathBuf, AppError> {
    if matches!(app, AppType::ClaudeDesktop) {
        return Err(AppError::localized(
            "app.prompts_unsupported",
            "当前应用暂不支持 Prompts",
            "This app does not support Prompts",
        ));
    }

    let base_dir: PathBuf = match app {
        AppType::Claude => get_base_dir_with_fallback(get_claude_settings_path(), ".claude"),
        AppType::Codex => get_base_dir_with_fallback(get_codex_auth_path(), ".codex"),
        AppType::Gemini => get_gemini_dir(),
        AppType::GrokBuild => crate::grok_config::get_grok_config_dir(),
        AppType::OpenCode => get_opencode_dir(),
        AppType::OpenClaw => get_openclaw_dir(),
        AppType::Hermes => crate::hermes_config::get_hermes_dir(),
        AppType::ClaudeDesktop => unreachable!("handled above"),
    };

    let filename = match app {
        AppType::Claude => "CLAUDE.md",
        AppType::Codex => "AGENTS.md",
        AppType::Gemini => "GEMINI.md",
        AppType::GrokBuild | AppType::OpenCode | AppType::OpenClaw => "AGENTS.md",
        AppType::Hermes => "SOUL.md",
        AppType::ClaudeDesktop => unreachable!("handled above"),
    };

    Ok(base_dir.join(filename))
}

fn get_base_dir_with_fallback(primary_path: PathBuf, fallback_dir: &str) -> PathBuf {
    primary_path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| crate::config::get_home_dir().join(fallback_dir))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_dir_returns_path_for_parent_and_home_fallback() {
        let primary_dir = PathBuf::from("custom-config");
        let path: PathBuf =
            get_base_dir_with_fallback(primary_dir.join("settings.json"), ".claude");
        assert_eq!(path, primary_dir);

        let path: PathBuf = get_base_dir_with_fallback(PathBuf::new(), ".codex");
        assert_eq!(path, crate::config::get_home_dir().join(".codex"));
    }

    #[test]
    fn hermes_prompt_file_uses_soul_md() {
        let path = prompt_file_path(&AppType::Hermes).expect("Hermes prompt path");

        assert_eq!(
            path.file_name().and_then(|name| name.to_str()),
            Some("SOUL.md")
        );
    }
}
