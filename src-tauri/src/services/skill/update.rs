//! Private, revision-bound progress for a Skill update. The IPC contract stays
//! Result<InstalledSkill>: a partially projected update must return an error.
use super::{InstalledSkill, SkillService, SkillTargetId, SkillUpdateInfo};
use crate::{config::get_app_config_dir, database::Database};
use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{fs, io::Write, path::Path, sync::Arc};

#[cfg(test)]
mod tests;

static UPDATE_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

pub(super) async fn lock() -> tokio::sync::MutexGuard<'static, ()> {
    UPDATE_LOCK.lock().await
}

#[derive(Serialize, Deserialize)]
struct PendingUpdate {
    before: InstalledSkill,
    desired: InstalledSkill,
    source_path: String,
    source_before: String,
    source_after: String,
    backup_id: String,
    targets: Vec<TargetProgress>,
}

#[derive(Serialize, Deserialize)]
struct TargetProgress {
    target: SkillTargetId,
    path: String,
    before: TargetBefore,
    applied: bool,
}

#[derive(Serialize, Deserialize)]
enum TargetBefore {
    Missing,
    Revision(String),
    // A blocked root cannot be inspected. After the user removes the obstacle,
    // only absence or an exact copy of the old managed source is admissible.
    Unavailable,
}

// Use the existing bounded tree reader, including hidden files and empty dirs.
// The discovery content hash deliberately excludes hidden files and is not a
// sufficient guard against later user edits.
fn revision(path: &Path) -> Result<Option<String>> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let root = if metadata.file_type().is_symlink() {
        let resolved = path.canonicalize()?;
        let managed = SkillService::get_ssot_dir()?.canonicalize()?;
        if !resolved.starts_with(&managed) {
            bail!("Skill 更新拒绝指向受管根以外的链接");
        }
        resolved
    } else {
        path.to_path_buf()
    };
    let snapshot = SkillService::scan_vendor_tree(&root)?;
    let mut digest = Sha256::new();
    for entry in snapshot.entries {
        let name = entry.relative.as_os_str().as_encoded_bytes();
        digest.update((name.len() as u64).to_le_bytes());
        digest.update(name);
        match entry.content_hash {
            Some(hash) => {
                digest.update([1]);
                digest.update(entry.identity.size.to_le_bytes());
                digest.update(hash);
            }
            None => digest.update([0]),
        }
    }
    Ok(Some(format!("{:x}", digest.finalize())))
}

fn save(path: &Path, pending: &PendingUpdate) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| anyhow!("Invalid update state path"))?;
    fs::create_dir_all(parent)?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    temporary.write_all(&serde_json::to_vec(pending)?)?;
    temporary.as_file().sync_all()?;
    temporary.persist(path).map_err(|error| error.error)?;
    Ok(())
}

fn same_record(left: &InstalledSkill, right: &InstalledSkill) -> Result<bool> {
    Ok(serde_json::to_value(left)? == serde_json::to_value(right)?)
}

// Assignment is mutable while a failed projection is pending. All other
// installation and source metadata still bind the retry to its admitted update.
fn same_version(left: &InstalledSkill, right: &InstalledSkill) -> Result<bool> {
    let mut left = left.clone();
    left.apps = right.apps.clone();
    same_record(&left, right)
}

fn matches_pending_version(pending: &PendingUpdate, current: &InstalledSkill) -> Result<bool> {
    Ok(same_version(current, &pending.before)? || same_version(current, &pending.desired)?)
}

fn state_path(id: &str) -> std::path::PathBuf {
    get_app_config_dir()
        .join("skill-updates")
        .join(format!("{:x}.json", Sha256::digest(id.as_bytes())))
}

pub(super) fn is_pending(id: &str) -> Result<bool> {
    match fs::symlink_metadata(state_path(id)) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.into()),
    }
}

pub(super) fn clear_after_uninstall(id: &str) -> Result<()> {
    match fs::remove_file(state_path(id)) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).context("Skill 已卸载，但旧更新进度清理失败"),
    }
}

fn read_pending(id: &str) -> Result<Option<PendingUpdate>> {
    match fs::read(state_path(id)) {
        Ok(bytes) => Ok(Some(
            serde_json::from_slice(&bytes)
                .context("未完成的 Skill 更新记录不可读；原文件已保留")?,
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

pub(super) fn pending_info(current: &InstalledSkill) -> Result<Option<SkillUpdateInfo>> {
    let Some(pending) = read_pending(&current.id)? else {
        return Ok(None);
    };
    if !matches_pending_version(&pending, current)? {
        bail!("未完成的 Skill 更新记录与当前条目不一致");
    }
    Ok(Some(SkillUpdateInfo {
        id: current.id.clone(),
        name: current.name.clone(),
        current_hash: current.content_hash.clone(),
        remote_hash: pending
            .desired
            .content_hash
            .ok_or_else(|| anyhow!("未完成的 Skill 更新记录缺少内容哈希"))?,
    }))
}

pub(super) fn resume(
    db: &Arc<Database>,
    current: &InstalledSkill,
) -> Result<Option<InstalledSkill>> {
    let Some(pending) = read_pending(&current.id)? else {
        return Ok(None);
    };
    let dest = SkillService::get_ssot_dir()?.join(&current.directory);
    let observed = revision(&dest)?;
    if observed.as_deref() == Some(pending.source_after.as_str()) {
        // The exact admitted download already lives in SSOT. A target retry
        // must not depend on a second download or accept a newer upstream ZIP.
        return commit(db, current, &pending.desired, &dest, &dest).map(Some);
    }
    if observed.as_deref() != Some(pending.source_before.as_str()) {
        bail!("Skill 源在未完成更新后被修改，已保留外部变化");
    }
    Ok(None)
}

pub(super) fn backup_is_pending(backup: &Path) -> Result<bool> {
    let root = get_app_config_dir().join("skill-updates");
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error.into()),
    };
    for entry in entries {
        let path = entry?.path();
        if path
            .extension()
            .is_some_and(|extension| extension == "json")
        {
            // An unreadable progress record blocks retention cleanup rather
            // than accidentally deleting the only preimage for a retry.
            let pending: PendingUpdate = serde_json::from_slice(&fs::read(path)?)?;
            if backup
                .file_name()
                .is_some_and(|name| name == pending.backup_id.as_str())
            {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn replace_source(source: &Path, dest: &Path, before: &str, after: &str) -> Result<()> {
    let parent = dest
        .parent()
        .ok_or_else(|| anyhow!("Invalid Skill source path"))?;
    if parent != SkillService::get_ssot_dir()? {
        bail!("Skill 存储位置已变化，更新未执行");
    }
    let stage = tempfile::Builder::new()
        .prefix(".update-stage-")
        .tempdir_in(parent)?;
    let copied = stage.path().join("new");
    let old = stage.path().join("old");
    fs::create_dir(&copied)?;
    let snapshot = SkillService::scan_vendor_tree(source)?;
    SkillService::copy_vendor_tree(source, &copied, &snapshot)?;
    if revision(source)?.as_deref() != Some(after)
        || revision(&copied)?.as_deref() != Some(after)
        || revision(dest)?.as_deref() != Some(before)
    {
        bail!("Skill 在暂存期间发生变化，更新未执行");
    }
    fs::rename(dest, &old)?;
    if let Err(error) = fs::rename(&copied, dest) {
        if let Err(restore) = fs::rename(&old, dest) {
            // TempDir must not destroy the retained old tree on drop.
            let retained = stage.keep();
            bail!(
                "Skill 替换失败: {error}; 前像恢复失败: {restore}; 已保留于 {}",
                retained.display()
            );
        }
        return Err(error.into());
    }
    if revision(dest)?.as_deref() != Some(after) {
        // Never overwrite unexpected post-publish content during recovery.
        let retained = stage.keep();
        bail!(
            "Skill 更新源读回不一致；更新前像已保留于 {}",
            retained.display()
        );
    }
    Ok(())
}

pub(super) fn commit(
    db: &Arc<Database>,
    current: &InstalledSkill,
    desired: &InstalledSkill,
    source: &Path,
    dest: &Path,
) -> Result<InstalledSkill> {
    let state_path = state_path(&current.id);
    let after = revision(source)?.ok_or_else(|| anyhow!("Skill 下载源缺失"))?;
    let mut pending = match fs::read(&state_path) {
        Ok(bytes) => {
            let pending: PendingUpdate = serde_json::from_slice(&bytes)
                .context("未完成的 Skill 更新记录不可读；原文件已保留")?;
            if pending.source_path != dest.to_string_lossy()
                || pending.source_after != after
                || !matches_pending_version(&pending, current)?
                || pending.desired.id != desired.id
                || pending.desired.directory != desired.directory
                || pending.desired.content_hash != desired.content_hash
                || pending.desired.repo_branch != desired.repo_branch
                || pending.desired.name != desired.name
                || pending.desired.description != desired.description
                || pending.desired.readme_url != desired.readme_url
            {
                bail!("Skill 或下载内容在未完成更新后发生变化，已拒绝旧操作重试");
            }
            let backup = SkillService::backup_path_for_id(&pending.backup_id)?;
            let metadata = SkillService::read_backup_metadata(&backup)?;
            if !same_record(&metadata.skill, &pending.before)?
                || revision(&backup.join("skill"))?.as_deref() != Some(&pending.source_before)
            {
                bail!("Skill 更新前像已变化，已拒绝继续写入");
            }
            pending
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let before = revision(dest)?.ok_or_else(|| anyhow!("旧 Skill 源缺失，无法安全更新"))?;
            let targets = current
                .apps
                .enabled_targets()
                .into_iter()
                .map(|target| {
                    let path =
                        SkillService::get_target_skills_dir(&target)?.join(&current.directory);
                    let before = match revision(&path) {
                        Ok(Some(hash)) => TargetBefore::Revision(hash),
                        Ok(None) => TargetBefore::Missing,
                        Err(_) => TargetBefore::Unavailable,
                    };
                    Ok(TargetProgress {
                        target,
                        path: path.to_string_lossy().into_owned(),
                        before,
                        applied: false,
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            // F04: an error or absent backup is terminal before source/DB/target
            // writes. Verify that the saved preimage is the one just observed.
            let backup = SkillService::create_uninstall_backup(current)
                .map_err(|error| {
                    log::warn!("Skill update backup failed: {error:#}");
                    anyhow!(crate::error::format_skill_error(
                        "UPDATE_BACKUP_FAILED",
                        &[],
                        None
                    ))
                })?
                .ok_or_else(|| anyhow!("未取得旧 Skill 的可用备份，更新未执行"))?;
            if revision(&backup.join("skill"))?.as_deref() != Some(&before)
                || revision(dest)?.as_deref() != Some(&before)
            {
                bail!("Skill 在备份期间发生变化，更新未执行");
            }
            let pending = PendingUpdate {
                before: current.clone(),
                desired: desired.clone(),
                source_path: dest.to_string_lossy().into_owned(),
                source_before: before,
                source_after: after,
                backup_id: backup
                    .file_name()
                    .ok_or_else(|| anyhow!("Invalid backup"))?
                    .to_string_lossy()
                    .into_owned(),
                targets,
            };
            save(&state_path, &pending)?;
            pending
        }
        Err(error) => return Err(error.into()),
    };

    let observed = revision(dest)?;
    if observed.as_deref() == Some(&pending.source_before) {
        // Reuse the existing staged, bounded directory replacement instead of
        // deleting the sole old tree before the new tree has been copied.
        replace_source(source, dest, &pending.source_before, &pending.source_after)?;
    } else if observed.as_deref() != Some(&pending.source_after) {
        bail!("Skill 源在未完成更新后被修改，已保留外部变化");
    }
    if revision(dest)?.as_deref() != Some(&pending.source_after) {
        bail!("Skill 更新源读回不一致，旧版本备份已保留");
    }
    let updated = if same_version(current, &pending.desired)? {
        db.get_installed_skill(&current.id)?
            .ok_or_else(|| anyhow!("Skill no longer installed: {}", current.id))?
    } else {
        SkillService::persist_updated_skill_metadata(db, &pending.desired)?
    };
    if !same_version(&updated, &pending.desired)? {
        bail!("Skill 安装记录在更新期间发生变化，已拒绝旧操作重试");
    }

    // A newly enabled target is projected by the ordinary assignment service
    // before its DB flag is committed. Enroll it as already applied: verify the
    // new source below, but never adopt later external bytes as an overwriteable
    // preimage. Retain disabled entries until completion so re-enabling an old
    // target while another target is blocked keeps its original drift guard.
    let mut assignments_changed = false;
    for target in updated.apps.enabled_targets() {
        if !pending.targets.iter().any(|item| item.target == target) {
            let path = SkillService::get_target_skills_dir(&target)?.join(&updated.directory);
            pending.targets.push(TargetProgress {
                target,
                path: path.to_string_lossy().into_owned(),
                before: TargetBefore::Unavailable,
                applied: true,
            });
            assignments_changed = true;
        }
    }
    if assignments_changed {
        save(&state_path, &pending)?;
    }
    let mut failures = Vec::new();
    let mut conflicts = Vec::new();
    for index in 0..pending.targets.len() {
        let target = &pending.targets[index];
        // Disabling a target deliberately removes its projection. A retry must
        // not recreate it, inspect a new user's replacement, or report it failed.
        if !updated.apps.is_enabled_for_target(&target.target) {
            continue;
        }
        let result = (|| -> Result<()> {
            let path =
                SkillService::get_target_skills_dir(&target.target)?.join(&current.directory);
            if path.to_string_lossy() != target.path {
                bail!("目标目录已变化，已拒绝旧操作重试");
            }
            let observed = revision(&path)?;
            if observed.as_deref() == Some(&pending.source_after) {
                return Ok(());
            }
            if target.applied {
                bail!("已完成目标后来被修改，已保留外部变化");
            }
            let expected = match &target.before {
                TargetBefore::Missing => observed.is_none(),
                TargetBefore::Revision(hash) => observed.as_ref() == Some(hash),
                TargetBefore::Unavailable => {
                    observed.is_none() || observed.as_deref() == Some(&pending.source_before)
                }
            };
            if !expected {
                bail!("未完成目标后来被修改，已保留外部变化");
            }
            SkillService::sync_to_app_dir(&current.directory, &target.target)?;
            if revision(&path)?.as_deref() != Some(&pending.source_after) {
                bail!("目标同步后的读回不一致");
            }
            Ok(())
        })();
        match result {
            Ok(()) => {
                pending.targets[index].applied = true;
                save(&state_path, &pending)?;
            }
            Err(error) => {
                log::warn!(
                    "Skill update target {} failed: {error:#}",
                    target.target.as_str()
                );
                failures.push(target.target.as_str());
                if target.applied {
                    conflicts.push(target.target.as_str());
                }
            }
        }
    }
    if !failures.is_empty() {
        let applied = pending
            .targets
            .iter()
            .filter(|target| target.applied && updated.apps.is_enabled_for_target(&target.target))
            .map(|target| target.target.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        bail!(crate::error::format_skill_error(
            "UPDATE_INCOMPLETE",
            &[
                ("applied", &applied),
                ("failed", &failures.join(",")),
                ("conflicted", &conflicts.join(","))
            ],
            Some("retryFailedTargets"),
        ));
    }
    fs::remove_file(&state_path).context("Skill 内容已更新，但更新进度清理未完成")?;
    Ok(updated)
}
