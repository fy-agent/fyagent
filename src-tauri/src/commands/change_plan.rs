use serde::Serialize;
use tauri::{Emitter, Manager, State};

use crate::change_plan::{
    ApplyChangePlanOutcome, ChangeJobSnapshot, ChangePlan, ChangePlanErrorCode, ChangePlanService,
};
use crate::store::AppState;

#[tauri::command]
pub fn create_codex_provider_switch_plan(
    state: State<'_, AppState>,
    #[allow(non_snake_case)] targetProviderId: String,
) -> Result<ChangePlan, ChangePlanErrorCode> {
    ChangePlanService::plan_codex_switch(state.inner(), &targetProviderId)
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ChangeJobUpdatedEvent {
    job_id: String,
    event_seq: i64,
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
        ChangePlanService::apply_codex_switch(state.inner(), &planId, &planDigest)
    })
    .await
    .map_err(|_| ChangePlanErrorCode::Internal)??;
    if let Some(job) = &outcome.job {
        let _ = app_handle.emit(
            "change-job://updated",
            ChangeJobUpdatedEvent {
                job_id: job.job_id.clone(),
                event_seq: job.event_seq,
            },
        );
    }
    Ok(outcome)
}

#[tauri::command]
pub fn get_change_job(
    state: State<'_, AppState>,
    #[allow(non_snake_case)] jobId: String,
) -> Result<ChangeJobSnapshot, ChangePlanErrorCode> {
    ChangePlanService::get_job(state.inner(), &jobId)
}

#[tauri::command]
pub fn list_recoverable_change_jobs(
    state: State<'_, AppState>,
) -> Result<Vec<ChangeJobSnapshot>, ChangePlanErrorCode> {
    ChangePlanService::list_recoverable_jobs(state.inner())
}
