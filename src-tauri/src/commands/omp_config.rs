use crate::omp_config::{
    self, OmpApplyConfirmation, OmpCliDiscovery, OmpConfigRead, OmpSavePlan, OmpSavePreview,
    OmpSaveResult,
};

#[tauri::command]
pub async fn read_omp_config() -> Result<OmpConfigRead, String> {
    tokio::task::spawn_blocking(omp_config::read_omp_config)
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn preview_omp_save(plan: OmpSavePlan) -> Result<OmpSavePreview, String> {
    tokio::task::spawn_blocking(move || omp_config::preview_omp_save(&plan))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn apply_omp_save(plan: OmpApplyConfirmation) -> Result<OmpSaveResult, String> {
    tokio::task::spawn_blocking(move || omp_config::apply_omp_save(&plan))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn discover_omp_cli() -> Result<OmpCliDiscovery, String> {
    omp_config::discover_omp_cli()
        .await
        .map_err(|e| e.to_string())
}
