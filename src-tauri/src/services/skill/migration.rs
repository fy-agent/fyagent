use super::*;

#[derive(Debug, Clone, Deserialize)]
struct LegacySkillMigrationRow {
    directory: String,
    app_type: String,
}

pub(super) fn migrate_skills_to_ssot(db: &Arc<Database>) -> Result<usize> {
    let ssot_dir = SkillService::get_ssot_dir()?;
    let agents_lock = parse_agents_lock();
    let snapshot: Vec<LegacySkillMigrationRow> =
        match db.get_setting("skills_ssot_migration_snapshot")? {
            Some(value) if !value.trim().is_empty() => match serde_json::from_str(&value) {
                Ok(rows) => rows,
                Err(err) => {
                    log::warn!("解析 skills 迁移快照失败，将回退到文件系统扫描: {err}");
                    Vec::new()
                }
            },
            _ => Vec::new(),
        };

    let has_snapshot = !snapshot.is_empty();
    let mut discovered: HashMap<String, SkillApps> = HashMap::new();

    if has_snapshot {
        for row in &snapshot {
            if SkillService::require_valid_directory(&row.directory).is_err() {
                log::warn!("跳过 SSOT 迁移快照中非法的 directory: {:?}", row.directory);
                continue;
            }
            if let Ok(app) = row.app_type.parse::<SkillTargetId>() {
                discovered
                    .entry(row.directory.clone())
                    .or_default()
                    .set_enabled_for_target(&app, true);
            }
        }
    }

    for app in SkillTargetId::all() {
        let app_dir = match SkillService::get_target_skills_dir(&app) {
            Ok(d) => d,
            Err(_) => continue,
        };

        let entries = match fs::read_dir(&app_dir) {
            Ok(e) => e,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let dir_name = entry.file_name().to_string_lossy().to_string();
            if dir_name.starts_with('.') || !path.join("SKILL.md").exists() {
                continue;
            }
            if has_snapshot && !discovered.contains_key(&dir_name) {
                continue;
            }

            let ssot_path = ssot_dir.join(&dir_name);
            if !ssot_path.exists() {
                SkillService::copy_dir_recursive(&path, &ssot_path)?;
            }

            if !has_snapshot {
                discovered
                    .entry(dir_name)
                    .or_default()
                    .set_enabled_for_target(&app, true);
            }
        }
    }

    db.clear_skills()?;
    save_repos_from_lock(db, &agents_lock, discovered.keys());

    let mut count = 0;
    for (directory, apps) in discovered {
        let ssot_path = ssot_dir.join(&directory);
        let skill_md = ssot_path.join("SKILL.md");
        let (name, description) = SkillService::read_skill_name_desc(&skill_md, &directory);
        let (id, repo_owner, repo_name, repo_branch, readme_url) =
            build_repo_info_from_lock(&agents_lock, &directory);
        let content_hash = SkillService::compute_dir_hash(&ssot_path).ok();

        let skill = InstalledSkill {
            id,
            name,
            description,
            directory,
            repo_owner,
            repo_name,
            repo_branch,
            readme_url,
            apps,
            installed_at: chrono::Utc::now().timestamp(),
            content_hash,
            updated_at: 0,
            path: None,
        };

        db.save_skill(&skill)?;
        count += 1;
    }

    let _ = db.set_setting("skills_ssot_migration_snapshot", "");
    log::info!("Skills 迁移完成，共 {count} 个");
    Ok(count)
}
