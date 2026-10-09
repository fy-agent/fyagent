use indexmap::IndexMap;
use std::str::FromStr;

use tauri::State;

use crate::app_config::AppType;
use crate::error::AppError;
use crate::prompt::Prompt;
use crate::services::PromptService;
use crate::store::AppState;

#[tauri::command]
pub async fn get_prompts(
    app: String,
    state: State<'_, AppState>,
) -> Result<IndexMap<String, Prompt>, String> {
    let app_type = AppType::from_str(&app).map_err(|e| e.to_string())?;
    PromptService::get_prompts(&state, app_type).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn upsert_prompt(
    app: String,
    id: String,
    prompt: Prompt,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let app_type = AppType::from_str(&app).map_err(|e| e.to_string())?;
    PromptService::upsert_prompt(&state, app_type, &id, prompt).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_prompt(
    app: String,
    id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let app_type = AppType::from_str(&app).map_err(|e| e.to_string())?;
    PromptService::delete_prompt(&state, app_type, &id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn enable_prompt(
    app: String,
    id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let app_type = AppType::from_str(&app).map_err(|e| e.to_string())?;
    PromptService::enable_prompt(&state, app_type, &id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn import_prompt_from_file(
    app: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let app_type = AppType::from_str(&app).map_err(|e| e.to_string())?;
    PromptService::import_from_file(&state, app_type).map_err(prompt_import_error)
}

#[tauri::command]
pub async fn get_current_prompt_file_content(app: String) -> Result<Option<String>, String> {
    let app_type = AppType::from_str(&app).map_err(|e| e.to_string())?;
    PromptService::get_current_file_content(app_type).map_err(|e| e.to_string())
}

// This public import reads the source before creating a disabled library entry.
// In this fresh disabled-entry import, Io comes from the source read.
// Library DAO failures remain unconfirmed; this is not a general Io/write rule.
fn prompt_import_error(error: AppError) -> String {
    match error {
        AppError::Io { source, .. } => match source.kind() {
            std::io::ErrorKind::NotFound => {
                "提示词文件不存在，未导入。请先确认目标应用已创建提示词文件。".into()
            }
            std::io::ErrorKind::InvalidData => {
                "提示词文件不是有效 UTF-8，未导入。请保留原文件，将副本转换为 UTF-8 后重试。".into()
            }
            std::io::ErrorKind::PermissionDenied => {
                "无法读取提示词文件，未导入。请检查文件类型和读取权限后重试。".into()
            }
            _ => "无法读取提示词文件，未导入。请检查文件是否可读后重试。".into(),
        },
        _ => "提示词导入未确认完成。请先刷新管理列表核对结果，再决定是否重试。".into(),
    }
}

#[cfg(test)]
mod import_error_tests {
    use super::*;

    #[test]
    fn public_import_errors_do_not_expose_paths_or_contents() {
        for kind in [
            std::io::ErrorKind::InvalidData,
            std::io::ErrorKind::PermissionDenied,
            std::io::ErrorKind::NotFound,
            std::io::ErrorKind::Other,
        ] {
            let message = prompt_import_error(AppError::io(
                "private-source-path",
                std::io::Error::new(kind, "private-source-content"),
            ));
            assert!(!message.contains("private-source"));
            assert!(message.contains("未导入"));
        }
        let unknown = prompt_import_error(AppError::Database("private-db-detail".into()));
        assert!(unknown.contains("未确认完成"));
        assert!(!unknown.contains("未导入"));
        assert!(!unknown.contains("private-db"));
    }
}
