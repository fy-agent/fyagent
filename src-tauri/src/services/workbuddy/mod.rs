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
    current_paths, get_workbuddy_model_ids, get_workbuddy_status, load_workbuddy_files,
    normalized_target_ids, restore_workbuddy_from_backup_at_locked, save_workbuddy_models,
    save_workbuddy_models_at_locked, write_lock,
};

#[cfg(test)]
pub(crate) use config::arm_next_workbuddy_primary_write_fault;
pub(crate) use model_fetch::fetch_workbuddy_models;
pub(crate) use types::{SaveWorkBuddyModelsOutcome, SaveWorkBuddyModelsRequest};

pub(crate) fn credential_matches_model_id(credential: &str, model_id: &str) -> bool {
    let credential = credential.trim();
    !credential.is_empty() && model_id.trim().contains(credential)
}
