#![allow(non_snake_case)]

use tauri::State;

#[tauri::command]
pub async fn get_tool_versions(
    tools: Option<Vec<String>>,
) -> Result<Vec<crate::services::tooling::ToolVersion>, String> {
    crate::services::tooling::get_tool_versions(tools).await
}

#[tauri::command]
pub async fn run_tool_lifecycle_action(tools: Vec<String>, action: String) -> Result<(), String> {
    crate::services::tooling::run_tool_lifecycle_action(tools, action).await
}

#[tauri::command]
pub async fn probe_tool_installations(
    tools: Vec<String>,
) -> Result<Vec<crate::services::tooling::ToolInstallationReport>, String> {
    crate::services::tooling::probe_tool_installations(tools).await
}

#[tauri::command]
pub async fn open_provider_terminal(
    state: State<'_, crate::store::AppState>,
    app: String,
    #[allow(non_snake_case)] providerId: String,
    cwd: Option<String>,
) -> Result<bool, String> {
    crate::services::tooling::open_provider_terminal(state.inner(), app, providerId, cwd).await
}
