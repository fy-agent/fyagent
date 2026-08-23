//! WorkBuddy's isolated configuration domain.
//!
//! This module has no Provider/AppType input and owns the only four IPC
//! operations needed by the WorkBuddy page.

pub mod config;
pub mod document;
pub mod error;
pub mod model_fetch;
pub mod types;
pub mod url;

#[cfg(target_os = "windows")]
mod windows_storage;

pub(crate) use config::{
    apply_workbuddy_change_locked, get_workbuddy_model_ids, get_workbuddy_status,
    inspect_workbuddy_change_locked, lock_workbuddy_mutation, preview_workbuddy_change_locked,
    restore_workbuddy_change_snapshot_locked, save_workbuddy_models,
    workbuddy_target_matches_locked, WorkBuddyChangeSnapshot, WorkBuddyMutationGuard,
};
pub(crate) use model_fetch::fetch_workbuddy_models;

pub(crate) fn credential_matches_model_id(credential: &str, model_id: &str) -> bool {
    let credential = credential.trim();
    !credential.is_empty() && model_id.trim().contains(credential)
}
