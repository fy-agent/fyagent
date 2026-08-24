use tauri::{AppHandle, Emitter, Manager};

use super::provider::ProviderQuickSetupRequest;
use crate::app_config::AppType;
use crate::services::change_plan::{
    ApplyChangePlanOutcome, CancelChangeJobOutcome, ChangeJobEventHint, ChangeJobSnapshot,
    ChangePlan, ChangePlanErrorCode, ChangePlanService, WriterReceipt,
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
pub async fn create_codex_provider_upsert_plan(
    app_handle: AppHandle,
    request: ProviderQuickSetupRequest,
) -> Result<ChangePlan, ChangePlanErrorCode> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app_handle
            .try_state::<AppState>()
            .ok_or(ChangePlanErrorCode::Internal)?;
        let provider = request
            .into_provider(&AppType::Codex)
            .map_err(|_| ChangePlanErrorCode::InvalidTarget)?;
        ChangePlanService::plan_codex_upsert(&state, provider)
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
    let app_for_work = app_handle.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let state = app_for_work
            .try_state::<AppState>()
            .ok_or(ChangePlanErrorCode::Internal)?;
        let app_for_events = app_for_work.clone();
        ChangePlanService::apply_with_writers(
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
            |provider| {
                ProviderService::apply_quick_setup_with_lock_held(&state, AppType::Codex, provider)
                    .map(|result| WriterReceipt {
                        live_config_changed: result.live_config_changed,
                    })
            },
            move |hint: ChangeJobEventHint| {
                let _ = app_for_events.emit("change-job://updated", hint);
            },
        )
    })
    .await
    .map_err(|_| ChangePlanErrorCode::Internal)?
}

#[tauri::command]
pub async fn cancel_change_job(
    app_handle: AppHandle,
    job_id: String,
) -> Result<CancelChangeJobOutcome, ChangePlanErrorCode> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app_handle
            .try_state::<AppState>()
            .ok_or(ChangePlanErrorCode::Internal)?;
        ChangePlanService::cancel_job(&state, &job_id)
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
        let app_for_events = app_handle.clone();
        ChangePlanService::get_job_with_observer(&state, &job_id, move |hint| {
            let _ = app_for_events.emit("change-job://updated", hint);
        })
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
        let app_for_events = app_handle.clone();
        ChangePlanService::list_recoverable_jobs_with_observer(&state, move |hint| {
            let _ = app_for_events.emit("change-job://updated", hint);
        })
    })
    .await
    .map_err(|_| ChangePlanErrorCode::Internal)?
}
