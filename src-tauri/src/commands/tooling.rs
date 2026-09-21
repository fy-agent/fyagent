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
    // Keep the legacy command shape, but installation authority belongs to
    // the inventory-bound preflight and start_agent_action path.
    let _ = (tools, action);
    Err("请从软件详情检查安装条件并确认本次安装。".to_string())
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

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn legacy_cli_install_ipc_requires_confirmed_agent_action() {
        for tool in ["grok", "claude"] {
            for action in [
                "install",
                "install_official_npm",
                "install_native",
                "update",
            ] {
                assert_eq!(
                    super::run_tool_lifecycle_action(vec![tool.to_string()], action.to_string())
                        .await,
                    Err("请从软件详情检查安装条件并确认本次安装。".to_string())
                );
            }
        }
    }
}
