//! Native path pickers for session packages. Selecting a path neither reads
//! nor writes a package; the migration commands remain the validation owner.

use tauri_plugin_dialog::DialogExt;

#[tauri::command]
pub(crate) async fn pick_session_package_file(
    app: tauri::AppHandle,
) -> Result<Option<String>, String> {
    let file = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .add_filter("FyAgent 会话包", &["json"])
            .blocking_pick_file()
    })
    .await
    .map_err(|_| "无法打开文件选择窗口".to_string())?;
    file.map(|file| {
        file.into_path()
            .map(|path| path.to_string_lossy().into_owned())
            .map_err(|_| "请选择本机上的会话包文件".to_string())
    })
    .transpose()
}

#[tauri::command]
pub(crate) async fn pick_session_package_export_path(
    app: tauri::AppHandle,
    default_name: String,
) -> Result<Option<String>, String> {
    // Only a suggested basename crosses this boundary. The native dialog
    // owns the selected directory, extension and overwrite confirmation.
    let name = if !default_name.is_empty()
        && default_name.len() <= 200
        && default_name.ends_with(".json")
        && !default_name.contains(['/', '\\', ':'])
        && !default_name.chars().any(char::is_control)
    {
        default_name
    } else {
        "fyagent-sessions.json".to_string()
    };
    let file = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .add_filter("FyAgent 会话包", &["json"])
            .set_file_name(name)
            .blocking_save_file()
    })
    .await
    .map_err(|_| "无法打开保存位置窗口".to_string())?;
    file.map(|file| {
        file.into_path()
            .map(|path| path.to_string_lossy().into_owned())
            .map_err(|_| "请选择本机上的保存位置".to_string())
    })
    .transpose()
}
