use super::SkillService;
use crate::app_config::{AppType, InstalledSkill, SkillTargetId};
use crate::database::Database;
use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use std::sync::Arc;

pub(super) fn toggle_app(db: &Arc<Database>, id: &str, app: &AppType, enabled: bool) -> Result<()> {
    let target = SkillTargetId::try_from(app)?;
    toggle_target(db, id, &target, enabled)
}

pub(super) fn toggle_target(
    db: &Arc<Database>,
    id: &str,
    app: &SkillTargetId,
    enabled: bool,
) -> Result<()> {
    let mut skill = SkillService::adopt_observed_if_needed(db, id)?;
    skill.apps.set_enabled_for_target(app, enabled);

    if enabled {
        SkillService::sync_to_app_dir(&skill.directory, app)?;
    } else {
        SkillService::remove_from_target(&skill.directory, app)?;
    }

    db.update_skill_apps(id, &skill.apps)?;
    log::info!("Skill {} 的 {:?} 状态已更新为 {}", skill.name, app, enabled);
    Ok(())
}

pub(super) fn sync_to_target(db: &Arc<Database>, app: &SkillTargetId) -> Result<()> {
    let skills = db.get_all_installed_skills()?;
    let ssot_dir = SkillService::get_ssot_dir()?;
    let app_dir = SkillService::get_target_skills_dir(app)?;

    let indexed_skills: HashMap<String, &InstalledSkill> = skills
        .values()
        .map(|skill| (skill.directory.to_lowercase(), skill))
        .collect();

    if app.requires_copy() {
        for skill in skills.values() {
            if skill.apps.is_enabled_for_target(app) {
                if let Err(err) = SkillService::sync_to_app_dir(&skill.directory, app) {
                    log::warn!(
                        "同步 skill {} 到 {app:?} 失败，跳过该条: {err}",
                        skill.directory
                    );
                }
            } else if let Err(err) = SkillService::remove_from_target(&skill.directory, app) {
                log::warn!(
                    "从 {app:?} 安全移除 skill {} 失败，跳过该条: {err}",
                    skill.directory
                );
            }
        }
        return Ok(());
    }

    if app_dir.exists() {
        for entry in fs::read_dir(&app_dir)? {
            let entry = entry?;
            let path = entry.path();
            let dir_name = entry.file_name().to_string_lossy().to_string();

            if dir_name.starts_with('.') {
                continue;
            }

            if let Some(skill) = indexed_skills.get(&dir_name.to_lowercase()) {
                if !skill.apps.is_enabled_for_target(app) {
                    SkillService::remove_path(&path)?;
                }
                continue;
            }

            if SkillService::is_symlink_to_ssot(&path, &ssot_dir) {
                SkillService::remove_path(&path)?;
            }
        }
    }

    for skill in skills.values() {
        if skill.apps.is_enabled_for_target(app) {
            if let Err(err) = SkillService::sync_to_app_dir(&skill.directory, app) {
                log::warn!(
                    "同步 skill {} 到 {app:?} 失败，跳过该条: {err}",
                    skill.directory
                );
            }
        }
    }

    Ok(())
}

pub(super) fn sync_to_app(db: &Arc<Database>, app: &AppType) -> Result<()> {
    let Ok(target) = SkillTargetId::try_from(app) else {
        return Ok(());
    };
    sync_to_target(db, &target)
}
