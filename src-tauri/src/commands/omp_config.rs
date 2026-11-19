use crate::{
    error::AppError,
    omp_config::{
        self, OmpApplyConfirmation, OmpCliDiscovery, OmpConfigRead, OmpSavePlan, OmpSavePreview,
        OmpSaveResult,
    },
};

fn sanitize_omp_ipc_error(err: AppError) -> String {
    match err {
        AppError::Conflict(_) => "并发冲突: omp 配置文件已在外部变更".to_string(),
        AppError::InvalidInput(_) => "无效输入: omp 配置参数校验未通过".to_string(),
        AppError::Config(_) => "配置错误: omp YAML 语法无效".to_string(),
        _ => "读取或写入 omp 配置失败".to_string(),
    }
}

#[tauri::command]
pub async fn read_omp_config() -> Result<OmpConfigRead, String> {
    tokio::task::spawn_blocking(omp_config::read_omp_config)
        .await
        .map_err(|_| "读取或写入 omp 配置失败".to_string())?
        .map_err(sanitize_omp_ipc_error)
}

#[tauri::command]
pub async fn preview_omp_save(plan: OmpSavePlan) -> Result<OmpSavePreview, String> {
    tokio::task::spawn_blocking(move || omp_config::preview_omp_save(&plan))
        .await
        .map_err(|_| "读取或写入 omp 配置失败".to_string())?
        .map_err(sanitize_omp_ipc_error)
}

#[tauri::command]
pub async fn apply_omp_save(plan: OmpApplyConfirmation) -> Result<OmpSaveResult, String> {
    tokio::task::spawn_blocking(move || omp_config::apply_omp_save(&plan))
        .await
        .map_err(|_| "读取或写入 omp 配置失败".to_string())?
        .map_err(sanitize_omp_ipc_error)
}

#[tauri::command]
pub async fn discover_omp_cli() -> Result<OmpCliDiscovery, String> {
    omp_config::discover_omp_cli()
        .await
        .map_err(|e| e.to_string())
}
