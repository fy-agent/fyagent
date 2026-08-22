use crate::init_status::{InitErrorPayload, SkillsMigrationPayload};
use tauri::AppHandle;

#[tauri::command]
pub async fn open_external(app: AppHandle, url: String) -> Result<bool, String> {
    crate::platform::process_launch::open_http_url_as_user(app, url).await?;
    Ok(true)
}

#[tauri::command]
pub async fn copy_text_to_clipboard(text: String) -> Result<bool, String> {
    tokio::task::spawn_blocking(move || {
        let mut clipboard =
            arboard::Clipboard::new().map_err(|e| format!("访问系统剪贴板失败: {e}"))?;
        clipboard
            .set_text(text)
            .map_err(|e| format!("写入系统剪贴板失败: {e}"))?;
        Ok(true)
    })
    .await
    .map_err(|e| format!("剪贴板任务执行失败: {e}"))?
}

#[tauri::command]
pub async fn is_portable_mode() -> Result<bool, String> {
    let exe_path = std::env::current_exe().map_err(|e| format!("获取可执行路径失败: {e}"))?;
    Ok(exe_path
        .parent()
        .is_some_and(|directory| directory.join("portable.ini").is_file()))
}

#[tauri::command]
pub async fn get_init_error() -> Result<Option<InitErrorPayload>, String> {
    Ok(crate::init_status::get_init_error())
}

#[tauri::command]
pub async fn get_migration_result() -> Result<bool, String> {
    Ok(crate::init_status::take_migration_success())
}

#[tauri::command]
pub async fn get_skills_migration_result() -> Result<Option<SkillsMigrationPayload>, String> {
    Ok(crate::init_status::take_skills_migration_result())
}

#[tauri::command]
pub async fn set_window_theme(window: tauri::Window, theme: String) -> Result<(), String> {
    use tauri::Theme;

    let tauri_theme = match theme.as_str() {
        "dark" => Some(Theme::Dark),
        "light" => Some(Theme::Light),
        _ => None,
    };

    window.set_theme(tauri_theme).map_err(|e| e.to_string())
}
