use tauri::State;

use crate::change_plan::{ChangePlan, ChangePlanErrorCode, ChangePlanService};
use crate::store::AppState;

#[tauri::command]
pub fn create_codex_provider_switch_plan(
    state: State<'_, AppState>,
    #[allow(non_snake_case)] targetProviderId: String,
) -> Result<ChangePlan, ChangePlanErrorCode> {
    ChangePlanService::plan_codex_switch(state.inner(), &targetProviderId)
}
