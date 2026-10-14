use crate::error::AppError;
#[cfg(target_os = "macos")]
use auto_launch::{AutoLaunch, AutoLaunchBuilder};

/// 获取 macOS 上的 .app bundle 路径
/// 将 `/path/to/FyAgent.app/Contents/MacOS/FyAgent` 转换为 `/path/to/FyAgent.app`
#[cfg(target_os = "macos")]
fn get_macos_app_bundle_path(exe_path: &std::path::Path) -> Option<std::path::PathBuf> {
    let path_str = exe_path.to_string_lossy();
    // 查找 .app/Contents/MacOS/ 模式
    if let Some(app_pos) = path_str.find(".app/Contents/MacOS/") {
        let app_bundle_end = app_pos + 4; // ".app" 的结束位置
        Some(std::path::PathBuf::from(&path_str[..app_bundle_end]))
    } else {
        None
    }
}

/// 初始化 macOS 的 AutoLaunch 实例。
///
/// Windows 正式版始终以管理员权限运行，不能安全地注册为登录自启：
/// 该平台仅清理 FyAgent 自己的旧值，绝不触碰历史产品的 OS 注册。
#[cfg(target_os = "macos")]
fn get_auto_launch() -> Result<AutoLaunch, AppError> {
    // macOS derives its login-item name from the bundle.
    let app_name = "FyAgent";
    let exe_path =
        std::env::current_exe().map_err(|e| AppError::Message(format!("无法获取应用路径: {e}")))?;

    // macOS 需要使用 .app bundle 路径，否则 AppleScript login item 会打开终端
    let app_path = get_macos_app_bundle_path(&exe_path).unwrap_or(exe_path);

    // macOS 使用 AppleScript 方式（默认），需要 .app bundle 路径。
    let auto_launch = AutoLaunchBuilder::new()
        .set_app_name(app_name)
        .set_app_path(&app_path.to_string_lossy())
        .build()
        .map_err(|e| AppError::Message(format!("创建 AutoLaunch 失败: {e}")))?;

    Ok(auto_launch)
}

#[cfg(target_os = "windows")]
const WINDOWS_AUTO_LAUNCH_VALUE: &str = "FyAgent";

#[cfg(target_os = "windows")]
fn clear_windows_auto_launch_entry() -> Result<(), AppError> {
    use std::io::ErrorKind;

    let run_key = match crate::windows_runtime::open_shell_user_run_update() {
        Ok(key) => key,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(AppError::Message(format!(
                "无法读取 Windows 启动项以清理 FyAgent 旧自启: {error}"
            )));
        }
    };

    match run_key.delete_value(WINDOWS_AUTO_LAUNCH_VALUE) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(AppError::Message(format!(
            "无法清理 Windows FyAgent 旧自启项: {error}"
        ))),
    }
}

/// Remove the one known legacy value after single-instance primary admission.
///
/// Startup treats failure as a warning: a missing or temporarily unavailable
/// Alice hive must not block the application, and a secondary instance must
/// never reach this side effect.
#[cfg(target_os = "windows")]
pub fn enforce_platform_auto_launch_policy() -> Result<(), AppError> {
    clear_windows_auto_launch_entry()
}

/// 启用开机自启
pub fn enable_auto_launch() -> Result<(), AppError> {
    #[cfg(target_os = "windows")]
    {
        clear_windows_auto_launch_entry()?;
        Err(AppError::Message(
            "Windows 版本已禁用开机自启；已清理 FyAgent 的旧启动项".to_owned(),
        ))
    }

    #[cfg(target_os = "macos")]
    {
        let auto_launch = get_auto_launch()?;
        auto_launch
            .enable()
            .map_err(|e| AppError::Message(format!("启用开机自启失败: {e}")))?;
        log::info!("已启用开机自启");
        Ok(())
    }
}

/// 禁用开机自启
pub fn disable_auto_launch() -> Result<(), AppError> {
    #[cfg(target_os = "windows")]
    {
        clear_windows_auto_launch_entry()?;
        log::info!("Windows 已禁用并清理 FyAgent 开机自启");
        Ok(())
    }

    #[cfg(target_os = "macos")]
    {
        let auto_launch = get_auto_launch()?;
        auto_launch
            .disable()
            .map_err(|e| AppError::Message(format!("禁用开机自启失败: {e}")))?;
        log::info!("已禁用开机自启");
        Ok(())
    }
}

/// 检查是否已启用开机自启
pub fn is_auto_launch_enabled() -> Result<bool, AppError> {
    #[cfg(target_os = "windows")]
    {
        // Querying is read-only; legacy cleanup belongs to startup and explicit toggles.
        Ok(false)
    }

    #[cfg(target_os = "macos")]
    {
        let auto_launch = get_auto_launch()?;
        auto_launch
            .is_enabled()
            .map_err(|e| AppError::Message(format!("检查开机自启状态失败: {e}")))
    }
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[cfg(target_os = "macos")]
    #[test]
    fn test_get_macos_app_bundle_path_valid() {
        let exe_path = std::path::Path::new("/Applications/FyAgent.app/Contents/MacOS/FyAgent");
        let result = get_macos_app_bundle_path(exe_path);
        assert_eq!(
            result,
            Some(std::path::PathBuf::from("/Applications/FyAgent.app"))
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_get_macos_app_bundle_path_with_spaces() {
        let exe_path =
            std::path::Path::new("/Users/test/My Apps/FyAgent.app/Contents/MacOS/FyAgent");
        let result = get_macos_app_bundle_path(exe_path);
        assert_eq!(
            result,
            Some(std::path::PathBuf::from("/Users/test/My Apps/FyAgent.app"))
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_get_macos_app_bundle_path_not_in_bundle() {
        let exe_path = std::path::Path::new("/usr/local/bin/fyagent");
        let result = get_macos_app_bundle_path(exe_path);
        assert_eq!(result, None);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_get_macos_app_bundle_path_dev_build() {
        // 开发环境下的路径通常不在 .app bundle 内
        let exe_path = std::path::Path::new("/Users/dev/project/target/debug/fyagent");
        let result = get_macos_app_bundle_path(exe_path);
        assert_eq!(result, None);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_windows_is_auto_launch_enabled_does_not_delete_run_entry() {
        use winreg::{enums::HKEY_CURRENT_USER, RegKey};

        struct RunEntryGuard<'a>(&'a RegKey);

        impl Drop for RunEntryGuard<'_> {
            fn drop(&mut self) {
                let _ = self.0.delete_value(WINDOWS_AUTO_LAUNCH_VALUE);
                if let Ok(hkcu) = RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(
                    "Software\\Microsoft\\Windows\\CurrentVersion\\Run",
                    winreg::enums::KEY_SET_VALUE,
                ) {
                    let _ = hkcu.delete_value(WINDOWS_AUTO_LAUNCH_VALUE);
                }
            }
        }

        let _ = crate::initialize_windows_user_context();

        let run_key = crate::windows_runtime::open_shell_user_run_update().unwrap_or_else(|_| {
            let hkcu = RegKey::predef(HKEY_CURRENT_USER);
            let (key, _) = hkcu
                .create_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Run")
                .expect("无法打开或创建 HKCU Run 注册表键");
            key
        });

        let _guard = RunEntryGuard(&run_key);

        run_key
            .set_value(WINDOWS_AUTO_LAUNCH_VALUE, &"test_auto_launch_entry")
            .expect("写入 Windows Run 启动项失败");

        let result = is_auto_launch_enabled();
        let value_after: Result<String, _> = run_key.get_value(WINDOWS_AUTO_LAUNCH_VALUE);

        drop(_guard);

        assert!(
            matches!(result, Ok(false)),
            "期望 is_auto_launch_enabled() 返回 Ok(false)，实际为: {result:?}"
        );
        assert!(
            value_after.is_ok(),
            "is_auto_launch_enabled() 查询时不应删除 HKCU Run 下的 \"{}\" 启动项",
            WINDOWS_AUTO_LAUNCH_VALUE
        );
    }
}
