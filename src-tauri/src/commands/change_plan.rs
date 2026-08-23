use tauri::{AppHandle, Manager};

use crate::app_config::AppType;
use crate::services::change_plan::{
    ApplyChangePlanOutcome, ChangeJobSnapshot, ChangePlan, ChangePlanErrorCode, ChangePlanService,
    WriterReceipt,
};
use crate::services::ProviderService;
use crate::store::AppState;

#[tauri::command]
pub async fn create_codex_provider_switch_plan(
    app_handle: AppHandle,
    target_provider_id: String,
) -> Result<ChangePlan, ChangePlanErrorCode> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app_handle
            .try_state::<AppState>()
            .ok_or(ChangePlanErrorCode::Internal)?;
        ChangePlanService::plan_codex_switch(&state, &target_provider_id)
    })
    .await
    .map_err(|_| ChangePlanErrorCode::Internal)?
}

#[tauri::command]
pub async fn apply_change_plan(
    app_handle: AppHandle,
    plan_id: String,
    plan_digest: String,
) -> Result<ApplyChangePlanOutcome, ChangePlanErrorCode> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app_handle
            .try_state::<AppState>()
            .ok_or(ChangePlanErrorCode::Internal)?;
        ChangePlanService::apply_codex_switch_with_writer(
            &state,
            &plan_id,
            &plan_digest,
            |target_provider_id| {
                ProviderService::with_live_config_result(AppType::Codex, || {
                    ProviderService::switch_with_lock_held(
                        &state,
                        AppType::Codex,
                        target_provider_id,
                    )
                    .map(|_| true)
                })
                .map(|result| WriterReceipt {
                    live_config_changed: result.live_config_changed,
                })
            },
        )
    })
    .await
    .map_err(|_| ChangePlanErrorCode::Internal)?
}

#[tauri::command]
pub async fn get_change_job(
    app_handle: AppHandle,
    job_id: String,
) -> Result<ChangeJobSnapshot, ChangePlanErrorCode> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app_handle
            .try_state::<AppState>()
            .ok_or(ChangePlanErrorCode::Internal)?;
        ChangePlanService::get_job(&state, &job_id)
    })
    .await
    .map_err(|_| ChangePlanErrorCode::Internal)?
}

#[tauri::command]
pub async fn list_recoverable_change_jobs(
    app_handle: AppHandle,
) -> Result<Vec<ChangeJobSnapshot>, ChangePlanErrorCode> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app_handle
            .try_state::<AppState>()
            .ok_or(ChangePlanErrorCode::Internal)?;
        ChangePlanService::list_recoverable_jobs(&state)
    })
    .await
    .map_err(|_| ChangePlanErrorCode::Internal)?
}
