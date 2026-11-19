//! Tauri commands for Portable Workspace Packs (`.fyagent-pack`, <= 100 MiB)
//! and Agentless SSH Remote Config Push.

use tauri::State;
use tauri_plugin_dialog::DialogExt;

use crate::{
    services::{
        remote_push::{
            self,
            ssh_config::{SshHostEntry, SshTargetSpec},
            transport::{RemotePushApplyResult, SshConnectionTestResult},
            RemotePushPreviewResult, RemotePushRequest,
        },
        workspace_pack::{
            self, WorkspacePackExportOptions, WorkspacePackExportOutcome,
            WorkspacePackExportPreview, WorkspacePackExportRequest, WorkspacePackImportOutcome,
            WorkspacePackImportPreview, WorkspacePackImportRequest, WorkspacePackInspectRequest,
        },
    },
    store::AppState,
};

#[tauri::command]
pub(crate) async fn pick_workspace_pack_export_path(
    app: tauri::AppHandle,
    default_name: String,
) -> Result<Option<String>, String> {
    let name = if !default_name.is_empty()
        && default_name.len() <= 200
        && default_name.ends_with(".fyagent-pack")
        && !default_name.contains(['/', '\\', ':'])
        && !default_name.chars().any(char::is_control)
    {
        default_name
    } else {
        "fyagent-workspace.fyagent-pack".to_string()
    };
    let file = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .add_filter("FyAgent 迁移包", &["fyagent-pack"])
            .set_file_name(name)
            .blocking_save_file()
    })
    .await
    .map_err(|_| "无法打开保存位置窗口".to_string())?;
    file.map(|f| {
        f.into_path()
            .map(|p| p.to_string_lossy().into_owned())
            .map_err(|_| "请选择本机上的保存位置".to_string())
    })
    .transpose()
}

#[tauri::command]
pub(crate) async fn pick_workspace_pack_file(
    app: tauri::AppHandle,
) -> Result<Option<String>, String> {
    let file = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .add_filter("FyAgent 迁移包", &["fyagent-pack"])
            .blocking_pick_file()
    })
    .await
    .map_err(|_| "无法打开文件选择窗口".to_string())?;
    file.map(|f| {
        f.into_path()
            .map(|p| p.to_string_lossy().into_owned())
            .map_err(|_| "请选择本机上的 .fyagent-pack 迁移包".to_string())
    })
    .transpose()
}

#[tauri::command]
pub(crate) async fn preview_workspace_pack_export(
    state: State<'_, AppState>,
    options: WorkspacePackExportOptions,
) -> Result<WorkspacePackExportPreview, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        workspace_pack::preview_workspace_pack_export(&state, options)
    })
    .await
    .map_err(|e| format!("估算迁移包体积失败：{e}"))?
}

#[tauri::command]
pub(crate) async fn export_workspace_pack(
    state: State<'_, AppState>,
    request: WorkspacePackExportRequest,
) -> Result<WorkspacePackExportOutcome, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        workspace_pack::export_workspace_pack(&state, request)
    })
    .await
    .map_err(|e| format!("导出迁移包任务异常：{e}"))?
}

#[tauri::command]
pub(crate) async fn inspect_workspace_pack(
    request: WorkspacePackInspectRequest,
) -> Result<WorkspacePackImportPreview, String> {
    tauri::async_runtime::spawn_blocking(move || workspace_pack::inspect_workspace_pack(request))
        .await
        .map_err(|e| format!("校验迁移包任务异常：{e}"))?
}

#[tauri::command]
pub(crate) async fn apply_workspace_pack_import(
    state: State<'_, AppState>,
    request: WorkspacePackImportRequest,
) -> Result<WorkspacePackImportOutcome, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        workspace_pack::apply_workspace_pack_import(&state, request)
    })
    .await
    .map_err(|e| format!("导入迁移包任务异常：{e}"))?
}

#[tauri::command]
pub(crate) async fn list_ssh_hosts() -> Result<Vec<SshHostEntry>, String> {
    tauri::async_runtime::spawn_blocking(remote_push::list_ssh_hosts)
        .await
        .map_err(|e| format!("读取 SSH 主机列表失败：{e}"))
}

#[tauri::command]
pub(crate) async fn test_ssh_target(
    target: SshTargetSpec,
) -> Result<SshConnectionTestResult, String> {
    tauri::async_runtime::spawn_blocking(move || remote_push::test_ssh_target(target))
        .await
        .map_err(|e| format!("测试 SSH 连接失败：{e}"))?
}

#[tauri::command]
pub(crate) async fn preview_ssh_config_push(
    state: State<'_, AppState>,
    request: RemotePushRequest,
) -> Result<RemotePushPreviewResult, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        remote_push::preview_ssh_config_push(&state, request)
    })
    .await
    .map_err(|e| format!("预览 SSH 推送清单失败：{e}"))?
}

#[tauri::command]
pub(crate) async fn apply_ssh_config_push(
    state: State<'_, AppState>,
    request: RemotePushRequest,
) -> Result<RemotePushApplyResult, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        remote_push::apply_ssh_config_push(&state, request)
    })
    .await
    .map_err(|e| format!("执行 SSH 配置推送失败：{e}"))?
}
