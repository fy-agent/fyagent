use super::*;
use crate::app_config::AppType;
use crate::services::ProviderService;
use std::str::FromStr;

#[allow(non_snake_case)]
pub(super) async fn open_provider_terminal(
    state: &crate::store::AppState,
    app: String,
    #[allow(non_snake_case)] providerId: String,
    cwd: Option<String>,
) -> Result<bool, String> {
    let app_type = AppType::from_str(&app).map_err(|e| e.to_string())?;
    let launch_cwd = resolve_launch_cwd(cwd)?;

    let providers = ProviderService::list(state, app_type.clone())
        .map_err(|e| format!("获取提供商列表失败: {e}"))?;
    let provider = providers
        .get(&providerId)
        .ok_or_else(|| format!("提供商 {providerId} 不存在"))?;

    let env_vars = extract_env_vars_from_config(&provider.settings_config, &app_type);
    launch_terminal_with_env(env_vars, launch_cwd.as_deref())
        .map_err(|e| format!("启动终端失败: {e}"))?;

    Ok(true)
}

fn extract_env_vars_from_config(
    config: &serde_json::Value,
    app_type: &AppType,
) -> Vec<(String, String)> {
    let mut env_vars = Vec::new();
    let Some(obj) = config.as_object() else {
        return env_vars;
    };

    if let Some(env) = obj.get("env").and_then(|v| v.as_object()) {
        for (key, value) in env {
            if let Some(str_val) = value.as_str() {
                env_vars.push((key.clone(), str_val.to_string()));
            }
        }

        let base_url_key = match app_type {
            AppType::Claude | AppType::ClaudeDesktop => Some("ANTHROPIC_BASE_URL"),
            AppType::Gemini => Some("GOOGLE_GEMINI_BASE_URL"),
            _ => None,
        };
        if let Some(key) = base_url_key {
            if let Some(url_str) = env.get(key).and_then(|v| v.as_str()) {
                env_vars.push((key.to_string(), url_str.to_string()));
            }
        }
    }

    if *app_type == AppType::Codex {
        if let Some(auth) = obj.get("auth").and_then(|v| v.as_str()) {
            env_vars.push(("OPENAI_API_KEY".to_string(), auth.to_string()));
        }
    }
    if *app_type == AppType::Gemini {
        if let Some(api_key) = obj.get("api_key").and_then(|v| v.as_str()) {
            env_vars.push(("GEMINI_API_KEY".to_string(), api_key.to_string()));
        }
    }

    env_vars
}

pub(super) fn resolve_launch_cwd(cwd: Option<String>) -> Result<Option<PathBuf>, String> {
    let Some(raw_path) = cwd.filter(|value| !value.trim().is_empty()) else {
        return Ok(None);
    };
    if raw_path.contains('\n') || raw_path.contains('\r') {
        return Err("目录路径包含非法换行符".to_string());
    }

    let path = Path::new(&raw_path);
    if !path.exists() {
        return Err(format!("目录不存在: {raw_path}"));
    }
    let resolved = std::fs::canonicalize(path).map_err(|e| format!("解析目录失败: {e}"))?;
    if !resolved.is_dir() {
        return Err(format!("选择的路径不是文件夹: {}", resolved.display()));
    }

    #[cfg(target_os = "windows")]
    let resolved = {
        let s = resolved.to_string_lossy();
        if let Some(unc) = s.strip_prefix(r"\\?\UNC\") {
            PathBuf::from(format!(r"\\{unc}"))
        } else if let Some(stripped) = s.strip_prefix(r"\\?\") {
            PathBuf::from(stripped)
        } else {
            resolved
        }
    };

    Ok(Some(resolved))
}

fn launch_terminal_with_env(
    env_vars: Vec<(String, String)>,
    cwd: Option<&Path>,
) -> Result<(), String> {
    let config_file = create_claude_config(&env_vars)?;

    #[cfg(target_os = "macos")]
    {
        launch_macos_terminal(&config_file, cwd)?;
        Ok(())
    }

    #[cfg(target_os = "windows")]
    {
        launch_windows_terminal(&config_file, cwd)?;
        Ok(())
    }
}

fn create_claude_config(env_vars: &[(String, String)]) -> Result<PathBuf, String> {
    let mut config_obj = serde_json::Map::new();
    let mut env_obj = serde_json::Map::new();
    for (key, value) in env_vars {
        env_obj.insert(key.clone(), serde_json::Value::String(value.clone()));
    }
    config_obj.insert("env".to_string(), serde_json::Value::Object(env_obj));
    let config_json =
        serde_json::to_string_pretty(&config_obj).map_err(|e| format!("序列化配置失败: {e}"))?;
    write_persisted_temp_file("fyagent_claude_", ".json", config_json.as_bytes())
}

pub(crate) fn launch_terminal_running(command_line: &str, label: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    let temp_dir = std::env::temp_dir();
    #[cfg(target_os = "macos")]
    let pid = std::process::id();

    #[cfg(target_os = "macos")]
    let (script_file, script_content) = {
        let file = temp_dir.join(format!("fyagent_{}_{}.sh", label, pid));
        let content = format!(
            r#"#!/usr/bin/env sh
trap 'rm -f "{script_path}"' EXIT
echo "[fyagent] Starting: {label}"
echo ""
{cmd}
echo ""
echo "[fyagent] Command exited. Press Enter to close."
read -r _
"#,
            script_path = file.display(),
            label = label,
            cmd = command_line,
        );
        (file, content)
    };

    #[cfg(target_os = "macos")]
    {
        use std::os::unix::fs::PermissionsExt;

        std::fs::write(&script_file, &script_content)
            .map_err(|e| format!("写入启动脚本失败: {e}"))?;
        std::fs::set_permissions(&script_file, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("设置脚本权限失败: {e}"))?;

        let preferred = crate::settings::get_preferred_terminal();
        let terminal = preferred.as_deref().unwrap_or("terminal");
        let result = match terminal {
            "iterm2" => launch_macos_iterm2(&script_file),
            "warp" => launch_macos_warp(&script_file),
            "alacritty" => launch_macos_open_app("Alacritty", &script_file, true),
            "kitty" => launch_macos_open_app("kitty", &script_file, false),
            "ghostty" => launch_macos_ghostty(&script_file),
            "wezterm" => launch_macos_open_app("WezTerm", &script_file, true),
            "kaku" => launch_macos_open_app("Kaku", &script_file, true),
            _ => launch_macos_terminal_app(&script_file),
        };
        if result.is_err() && terminal != "terminal" {
            log::warn!(
                "首选终端 {} 启动失败，回退到 Terminal.app: {:?}",
                terminal,
                result.as_ref().err()
            );
            return launch_macos_terminal_app(&script_file);
        }
        result
    }

    #[cfg(target_os = "windows")]
    {
        let content = format!(
            "@echo off\r\necho [fyagent] Starting: {label}\r\necho.\r\n{cmd}\r\necho.\r\necho [fyagent] Command exited. Press any key to close.\r\npause >nul\r\ndel \"%~f0\" >nul 2>&1\r\n",
            label = label,
            cmd = command_line,
        );
        let bat_file = write_persisted_temp_file("fyagent_terminal_", ".bat", content.as_bytes())?;
        let result = crate::platform::process_launch::launch_terminal_script_as_user(&bat_file);
        if result.is_err() {
            let _ = std::fs::remove_file(&bat_file);
        }
        result.map_err(|error| format!("普通用户终端启动失败: {error}"))
    }
}
