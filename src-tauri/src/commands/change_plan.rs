use serde::Serialize;
use tauri::{Emitter, Manager, State};

use crate::change_plan::{
    ApplyChangePlanOutcome, CancelChangeJobOutcome, ChangeJobSnapshot, ChangePlan,
    ChangePlanErrorCode, ChangePlanService, CodexProviderUpsertPlanRequest,
    WorkBuddyModelsPlanRequest,
};
use crate::store::AppState;

#[tauri::command]
pub fn create_codex_provider_switch_plan(
    state: State<'_, AppState>,
    #[allow(non_snake_case)] targetProviderId: String,
) -> Result<ChangePlan, ChangePlanErrorCode> {
    ChangePlanService::plan_codex_switch(state.inner(), &targetProviderId)
}

#[tauri::command]
pub fn create_codex_provider_upsert_plan(
    state: State<'_, AppState>,
    request: CodexProviderUpsertPlanRequest,
) -> Result<ChangePlan, ChangePlanErrorCode> {
    ChangePlanService::plan_codex_provider_upsert(state.inner(), request)
}

#[tauri::command]
pub fn create_workbuddy_models_plan(
    state: State<'_, AppState>,
    request: WorkBuddyModelsPlanRequest,
) -> Result<ChangePlan, ChangePlanErrorCode> {
    ChangePlanService::plan_workbuddy_models(state.inner(), request)
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ChangeJobUpdatedEvent {
    job_id: String,
    event_seq: i64,
}

fn emit_job_update(app_handle: &tauri::AppHandle, job: &ChangeJobSnapshot) {
    let _ = app_handle.emit(
        "change-job://updated",
        ChangeJobUpdatedEvent {
            job_id: job.job_id.clone(),
            event_seq: job.event_seq,
        },
    );
}

#[tauri::command]
pub async fn apply_change_plan(
    app_handle: tauri::AppHandle,
    #[allow(non_snake_case)] planId: String,
    #[allow(non_snake_case)] planDigest: String,
) -> Result<ApplyChangePlanOutcome, ChangePlanErrorCode> {
    let app_for_work = app_handle.clone();
    let outcome = tauri::async_runtime::spawn_blocking(move || {
        let state = app_for_work
            .try_state::<AppState>()
            .ok_or(ChangePlanErrorCode::Internal)?;
        let app_for_events = app_for_work.clone();
        ChangePlanService::apply_change_plan_with_observer(
            state.inner(),
            &planId,
            &planDigest,
            move |job| emit_job_update(&app_for_events, job),
        )
    })
    .await
    .map_err(|_| ChangePlanErrorCode::Internal)??;
    Ok(outcome)
}

#[tauri::command]
pub fn get_change_job(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
    #[allow(non_snake_case)] jobId: String,
) -> Result<ChangeJobSnapshot, ChangePlanErrorCode> {
    ChangePlanService::get_job_with_observer(state.inner(), &jobId, |job| {
        emit_job_update(&app_handle, job)
    })
}

#[tauri::command]
pub fn list_recoverable_change_jobs(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<Vec<ChangeJobSnapshot>, ChangePlanErrorCode> {
    ChangePlanService::list_recoverable_jobs_with_observer(state.inner(), |job| {
        emit_job_update(&app_handle, job)
    })
}

#[tauri::command]
pub fn cancel_change_job(
    state: State<'_, AppState>,
    #[allow(non_snake_case)] jobId: String,
) -> Result<CancelChangeJobOutcome, ChangePlanErrorCode> {
    ChangePlanService::cancel_job(state.inner(), &jobId)
}
