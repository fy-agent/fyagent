use crate::services::projects::{domain::*, ProjectsService};
use tauri::State;

fn public_error(e: crate::error::AppError) -> String {
    if let crate::error::AppError::InvalidInput(code) = &e {
        if code.starts_with("projects_") {
            return code.clone();
        }
    }
    "projects_unavailable".into()
}
#[tauri::command]
pub(crate) fn projects_list_customers(
    state: State<'_, std::sync::Arc<ProjectsService>>,
) -> Result<Vec<Customer>, String> {
    state.customers().map_err(public_error)
}
#[tauri::command]
pub(crate) fn projects_create_customer(
    state: State<'_, std::sync::Arc<ProjectsService>>,
    name: String,
) -> Result<Customer, String> {
    state.create_customer(&name).map_err(public_error)
}
#[tauri::command]
pub(crate) fn projects_update_customer(
    state: State<'_, std::sync::Arc<ProjectsService>>,
    customer_id: String,
    expected_revision: i64,
    name: String,
    archived: bool,
) -> Result<Customer, String> {
    state
        .update_customer(&customer_id, expected_revision, &name, archived)
        .map_err(public_error)
}
#[tauri::command]
pub(crate) fn projects_list(
    state: State<'_, std::sync::Arc<ProjectsService>>,
) -> Result<Vec<Project>, String> {
    state.list().map_err(public_error)
}
#[tauri::command]
pub(crate) fn projects_get(
    state: State<'_, std::sync::Arc<ProjectsService>>,
    project_id: String,
) -> Result<Project, String> {
    state.get(&project_id).map_err(public_error)
}
#[tauri::command]
pub(crate) fn projects_create(
    state: State<'_, std::sync::Arc<ProjectsService>>,
    customer_id: String,
    name: String,
) -> Result<Project, String> {
    state.create(&customer_id, &name).map_err(public_error)
}
#[tauri::command]
pub(crate) fn projects_update(
    state: State<'_, std::sync::Arc<ProjectsService>>,
    request: ProjectMutation,
    name: String,
    archived: bool,
) -> Result<Project, String> {
    state
        .update(&request, &name, archived)
        .map_err(public_error)
}
#[tauri::command]
pub(crate) fn projects_resource_options(
    state: State<'_, std::sync::Arc<ProjectsService>>,
) -> Result<Vec<ResourceOption>, String> {
    state.resource_options().map_err(public_error)
}
#[tauri::command]
pub(crate) fn projects_credential_options(
    state: State<'_, std::sync::Arc<ProjectsService>>,
) -> Result<Vec<CredentialOption>, String> {
    state.credential_options().map_err(public_error)
}
#[tauri::command]
pub(crate) fn projects_bind_resource(
    state: State<'_, std::sync::Arc<ProjectsService>>,
    request: ProjectMutation,
    kind: ResourceKind,
    agent_id: String,
    raw_id: String,
    model: Option<String>,
) -> Result<Project, String> {
    state
        .bind_resource(&request, kind, &agent_id, &raw_id, model)
        .map_err(public_error)
}
#[tauri::command]
pub(crate) fn projects_remove_resource(
    state: State<'_, std::sync::Arc<ProjectsService>>,
    request: ProjectMutation,
    kind: ResourceKind,
    agent_id: String,
    raw_id: String,
) -> Result<Project, String> {
    state
        .remove_resource(&request, kind, &agent_id, &raw_id)
        .map_err(public_error)
}
#[tauri::command]
pub(crate) fn projects_bind_credential(
    state: State<'_, std::sync::Arc<ProjectsService>>,
    request: ProjectMutation,
    credential_id: String,
    purpose: String,
    consumer: String,
) -> Result<Project, String> {
    state
        .bind_credential(&request, &credential_id, &purpose, &consumer)
        .map_err(public_error)
}
#[tauri::command]
pub(crate) fn projects_remove_credential(
    state: State<'_, std::sync::Arc<ProjectsService>>,
    request: ProjectMutation,
    credential_id: String,
) -> Result<Project, String> {
    state
        .remove_credential(&request, &credential_id)
        .map_err(public_error)
}
#[tauri::command]
pub(crate) fn projects_get_context(
    state: State<'_, std::sync::Arc<ProjectsService>>,
    project_id: String,
) -> Result<ProjectContext, String> {
    state.context(&project_id).map_err(public_error)
}
#[tauri::command]
pub(crate) fn projects_write_context(
    state: State<'_, std::sync::Arc<ProjectsService>>,
    request: ProjectMutation,
    content: String,
    recover: Option<bool>,
) -> Result<ProjectContext, String> {
    if recover.unwrap_or(false) {
        state.write_context_with_recovery(&request, &content, true)
    } else {
        state.write_context(&request, &content)
    }
    .map_err(public_error)
}
#[tauri::command]
pub(crate) fn projects_bind_delivery_kit(
    state: State<'_, std::sync::Arc<ProjectsService>>,
    request: BindKitRequest,
) -> Result<Project, String> {
    state.bind_delivery_kit(&request).map_err(public_error)
}
#[tauri::command]
pub(crate) fn projects_dependency_snapshot(
    state: State<'_, std::sync::Arc<ProjectsService>>,
    project_id: String,
) -> Result<ProjectDependencySnapshot, String> {
    state.dependency_snapshot(&project_id).map_err(public_error)
}

#[tauri::command]
pub(crate) fn projects_prepare_codex(
    state: State<'_, std::sync::Arc<ProjectsService>>,
    request: ProjectMutation,
) -> Result<ProjectContext, String> {
    state.prepare_codex(&request).map_err(public_error)
}
