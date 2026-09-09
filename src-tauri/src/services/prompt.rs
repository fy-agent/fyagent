use indexmap::IndexMap;

use crate::app_config::AppType;
use crate::config::write_text_file;
use crate::error::AppError;
use crate::prompt::Prompt;
use crate::prompt_files::prompt_file_path;
use crate::store::AppState;

/// 安全地获取当前 Unix 时间戳
fn get_unix_timestamp() -> Result<i64, AppError> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .map_err(|e| AppError::Message(format!("Failed to get system time: {e}")))
}

pub struct PromptService;

impl PromptService {
    pub fn get_prompts(
        state: &AppState,
        app: AppType,
    ) -> Result<IndexMap<String, Prompt>, AppError> {
        state.db.get_prompts(app.as_str())
    }

    pub fn upsert_prompt(
        state: &AppState,
        app: AppType,
        _id: &str,
        prompt: Prompt,
    ) -> Result<(), AppError> {
        let is_enabled = prompt.enabled;
        // Only an actual enabled -> disabled transition may clear the live file.
        // New or already-disabled entries are library-only changes.
        let was_enabled = !is_enabled
            && state
                .db
                .get_prompts(app.as_str())?
                .get(&prompt.id)
                .is_some_and(|previous| previous.enabled);

        state.db.save_prompt(app.as_str(), &prompt)?;

        if is_enabled {
            // 启用提示词：写入内容到文件
            let target_path = prompt_file_path(&app)?;
            write_text_file(&target_path, &prompt.content)?;
        } else if was_enabled {
            // 禁用提示词：检查是否还有其他已启用的提示词
            let prompts = state.db.get_prompts(app.as_str())?;
            let any_enabled = prompts.values().any(|p| p.enabled);

            if !any_enabled {
                // 所有提示词都已禁用，清空文件
                let target_path = prompt_file_path(&app)?;
                if target_path.exists() {
                    write_text_file(&target_path, "")?;
                }
            }
        }

        Ok(())
    }

    pub fn delete_prompt(state: &AppState, app: AppType, id: &str) -> Result<(), AppError> {
        let prompts = state.db.get_prompts(app.as_str())?;

        if let Some(prompt) = prompts.get(id) {
            if prompt.enabled {
                return Err(AppError::InvalidInput("无法删除已启用的提示词".to_string()));
            }
        }

        state.db.delete_prompt(app.as_str(), id)?;
        Ok(())
    }

    pub fn enable_prompt(state: &AppState, app: AppType, id: &str) -> Result<(), AppError> {
        // 回填当前 live 文件内容到已启用的提示词，或创建备份
        let target_path = prompt_file_path(&app)?;
        if target_path.exists() {
            if let Ok(live_content) = std::fs::read_to_string(&target_path) {
                if !live_content.trim().is_empty() {
                    let mut prompts = state.db.get_prompts(app.as_str())?;

                    // 尝试回填到当前已启用的提示词
                    if let Some((enabled_id, enabled_prompt)) = prompts
                        .iter_mut()
                        .find(|(_, p)| p.enabled)
                        .map(|(id, p)| (id.clone(), p))
                    {
                        let timestamp = get_unix_timestamp()?;
                        enabled_prompt.content = live_content.clone();
                        enabled_prompt.updated_at = Some(timestamp);
                        log::info!("回填 live 提示词内容到已启用项: {enabled_id}");
                        state.db.save_prompt(app.as_str(), enabled_prompt)?;
                    } else {
                        // 没有已启用的提示词，则创建一次备份（避免重复备份）
                        let content_exists = prompts
                            .values()
                            .any(|p| p.content.trim() == live_content.trim());
                        if !content_exists {
                            let timestamp = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs() as i64;
                            let backup_id = format!("backup-{timestamp}");
                            let backup_prompt = Prompt {
                                id: backup_id.clone(),
                                name: format!(
                                    "原始提示词 {}",
                                    chrono::Local::now().format("%Y-%m-%d %H:%M")
                                ),
                                content: live_content,
                                description: Some("自动备份的原始提示词".to_string()),
                                enabled: false,
                                created_at: Some(timestamp),
                                updated_at: Some(timestamp),
                            };
                            log::info!("回填 live 提示词内容，创建备份: {backup_id}");
                            state.db.save_prompt(app.as_str(), &backup_prompt)?;
                        }
                    }
                }
            }
        }

        // 启用目标提示词并写入文件
        let mut prompts = state.db.get_prompts(app.as_str())?;

        for prompt in prompts.values_mut() {
            prompt.enabled = false;
        }

        if let Some(prompt) = prompts.get_mut(id) {
            prompt.enabled = true;
            write_text_file(&target_path, &prompt.content)?; // 原子写入
            state.db.save_prompt(app.as_str(), prompt)?;
        } else {
            return Err(AppError::InvalidInput(format!("提示词 {id} 不存在")));
        }

        // Save all prompts to disable others
        for (_, prompt) in prompts.iter() {
            state.db.save_prompt(app.as_str(), prompt)?;
        }

        Ok(())
    }

    pub fn import_from_file(state: &AppState, app: AppType) -> Result<String, AppError> {
        Self::import_from_file_at(state, app, get_unix_timestamp()?)
    }

    fn import_from_file_at(
        state: &AppState,
        app: AppType,
        timestamp: i64,
    ) -> Result<String, AppError> {
        let file_path = prompt_file_path(&app)?;

        if !file_path.exists() {
            return Err(AppError::Message("提示词文件不存在".to_string()));
        }

        let content =
            std::fs::read_to_string(&file_path).map_err(|e| AppError::io(&file_path, e))?;

        // Every explicit import is a new library entry, even within one second.
        let id = format!("imported-{}", uuid::Uuid::new_v4());
        let prompt = Prompt {
            id: id.clone(),
            name: format!(
                "导入的提示词 {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M")
            ),
            content,
            description: Some("从现有配置文件导入".to_string()),
            enabled: false,
            created_at: Some(timestamp),
            updated_at: Some(timestamp),
        };

        Self::upsert_prompt(state, app, &id, prompt)?;
        Ok(id)
    }

    pub fn get_current_file_content(app: AppType) -> Result<Option<String>, AppError> {
        let file_path = prompt_file_path(&app)?;
        if !file_path.exists() {
            return Ok(None);
        }
        let content =
            std::fs::read_to_string(&file_path).map_err(|e| AppError::io(&file_path, e))?;
        Ok(Some(content))
    }

    /// 首次启动时从现有提示词文件自动导入（如果存在）
    /// 返回导入的数量
    pub fn import_from_file_on_first_launch(
        state: &AppState,
        app: AppType,
    ) -> Result<usize, AppError> {
        // 幂等性保护：该应用已有提示词则跳过
        let existing = state.db.get_prompts(app.as_str())?;
        if !existing.is_empty() {
            return Ok(0);
        }

        let file_path = prompt_file_path(&app)?;

        // 检查文件是否存在
        if !file_path.exists() {
            return Ok(0);
        }

        // 读取文件内容
        let content = match std::fs::read_to_string(&file_path) {
            Ok(c) => c,
            Err(e) => {
                log::warn!("读取提示词文件失败: {file_path:?}, 错误: {e}");
                return Ok(0);
            }
        };

        // 检查内容是否为空
        if content.trim().is_empty() {
            return Ok(0);
        }

        log::info!("发现提示词文件，自动导入: {file_path:?}");

        // 创建提示词对象
        let timestamp = get_unix_timestamp()?;
        let id = format!("auto-imported-{timestamp}");
        let prompt = Prompt {
            id: id.clone(),
            name: format!(
                "Auto-imported Prompt {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M")
            ),
            content,
            description: Some("Automatically imported on first launch".to_string()),
            enabled: true, // 首次导入时自动启用
            created_at: Some(timestamp),
            updated_at: Some(timestamp),
        };

        // 保存到数据库
        state.db.save_prompt(app.as_str(), &prompt)?;

        log::info!("自动导入完成: {}", app.as_str());
        Ok(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;
    use serial_test::serial;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;

    struct TestHome(Option<std::ffi::OsString>);

    impl TestHome {
        fn set(path: &Path) -> Self {
            let previous = std::env::var_os("FYAGENT_TEST_HOME");
            std::env::set_var("FYAGENT_TEST_HOME", path);
            crate::settings::reload_settings().unwrap();
            Self(previous)
        }
    }

    impl Drop for TestHome {
        fn drop(&mut self) {
            match self.0.take() {
                Some(value) => std::env::set_var("FYAGENT_TEST_HOME", value),
                None => std::env::remove_var("FYAGENT_TEST_HOME"),
            }
            crate::settings::reload_settings().unwrap();
        }
    }

    fn setup() -> (tempfile::TempDir, TestHome, AppState, PathBuf) {
        let home = tempfile::tempdir().unwrap();
        let guard = TestHome::set(home.path());
        let state = AppState::new(Arc::new(Database::memory().unwrap()));
        let live = prompt_file_path(&AppType::Claude).unwrap();
        assert!(live.starts_with(home.path()));
        fs::create_dir_all(live.parent().unwrap()).unwrap();
        (home, guard, state, live)
    }

    fn prompt(id: &str, enabled: bool) -> Prompt {
        Prompt {
            id: id.to_string(),
            name: format!("Prompt {id}"),
            content: format!("Library content for {id}\n"),
            description: None,
            enabled,
            created_at: Some(1_000),
            updated_at: Some(1_000),
        }
    }

    fn seed_live_with_recovery(live: &Path) {
        fs::write(live, "Previous live content\n").unwrap();
        write_text_file(live, "External live content\r\n保留原文\n").unwrap();
        assert!(live.with_extension("md.fyagent.backup").exists());
        assert!(live.with_extension("md.fyagent.undo.json").exists());
    }

    #[derive(Debug, PartialEq)]
    struct FileSnapshot {
        name: std::ffi::OsString,
        bytes: Vec<u8>,
        modified: std::time::SystemTime,
        permissions: fs::Permissions,
    }

    fn snapshot(directory: &Path) -> Vec<FileSnapshot> {
        let mut files: Vec<_> = fs::read_dir(directory)
            .unwrap()
            .map(|entry| {
                let entry = entry.unwrap();
                let metadata = entry.metadata().unwrap();
                FileSnapshot {
                    name: entry.file_name(),
                    bytes: fs::read(entry.path()).unwrap(),
                    modified: metadata.modified().unwrap(),
                    permissions: metadata.permissions(),
                }
            })
            .collect();
        files.sort_by(|a, b| a.name.cmp(&b.name));
        files
    }

    #[test]
    #[serial]
    fn issue_141_disabled_create_preserves_live_and_recovery() {
        let (_home, _guard, state, live) = setup();
        seed_live_with_recovery(&live);
        // An enabled record with the same key in another app is unrelated.
        state
            .db
            .save_prompt("codex", &prompt("draft", true))
            .unwrap();
        let before = snapshot(live.parent().unwrap());

        PromptService::upsert_prompt(&state, AppType::Claude, "draft", prompt("draft", false))
            .unwrap();

        let saved = state.db.get_prompts("claude").unwrap();
        assert!(!saved["draft"].enabled);
        assert_eq!(saved["draft"].content, prompt("draft", false).content);
        assert!(state.db.get_prompts("codex").unwrap()["draft"].enabled);
        assert_eq!(snapshot(live.parent().unwrap()), before);
    }

    #[test]
    #[serial]
    fn issue_141_disabled_edit_preserves_live_and_recovery() {
        let (_home, _guard, state, live) = setup();
        seed_live_with_recovery(&live);
        let mut draft = prompt("draft", false);
        state.db.save_prompt("claude", &draft).unwrap();
        let before = snapshot(live.parent().unwrap());
        draft.content = "Edited library text\n".to_string();
        draft.updated_at = Some(2_000);

        PromptService::upsert_prompt(&state, AppType::Claude, "draft", draft).unwrap();

        let saved = state.db.get_prompts("claude").unwrap();
        assert_eq!(saved["draft"].content, "Edited library text\n");
        assert_eq!(saved["draft"].updated_at, Some(2_000));
        assert_eq!(snapshot(live.parent().unwrap()), before);
    }

    #[test]
    #[serial]
    fn issue_141_disabled_import_preserves_live_and_recovery() {
        let (_home, _guard, state, live) = setup();
        seed_live_with_recovery(&live);
        let content = fs::read_to_string(&live).unwrap();
        let before = snapshot(live.parent().unwrap());

        let id = PromptService::import_from_file(&state, AppType::Claude).unwrap();

        let saved = state.db.get_prompts("claude").unwrap();
        assert!(!saved[&id].enabled);
        assert_eq!(saved[&id].content, content);
        assert_eq!(snapshot(live.parent().unwrap()), before);
    }

    #[test]
    #[serial]
    fn issue_141_same_timestamp_import_preserves_enabled_entry_and_live() {
        let (_home, _guard, state, live) = setup();
        seed_live_with_recovery(&live);
        let content = fs::read_to_string(&live).unwrap();
        let timestamp = 1_000;
        let first = PromptService::import_from_file_at(&state, AppType::Claude, timestamp).unwrap();
        PromptService::enable_prompt(&state, AppType::Claude, &first).unwrap();
        let before = snapshot(live.parent().unwrap());

        let second =
            PromptService::import_from_file_at(&state, AppType::Claude, timestamp).unwrap();

        assert_eq!(snapshot(live.parent().unwrap()), before);
        assert_ne!(first, second, "each explicit import needs its own identity");
        let saved = state.db.get_prompts("claude").unwrap();
        assert_eq!(saved.len(), 2);
        assert!(saved[&first].enabled);
        assert!(!saved[&second].enabled);
        for id in [&first, &second] {
            assert_eq!(saved[id].content, content);
            assert_eq!(saved[id].created_at, Some(timestamp));
        }
    }

    #[test]
    #[serial]
    fn issue_141_disabled_operations_keep_missing_live_and_existing_recovery() {
        let (_home, _guard, state, live) = setup();
        seed_live_with_recovery(&live);
        fs::remove_file(&live).unwrap();
        let before = snapshot(live.parent().unwrap());
        let mut draft = prompt("draft", false);

        PromptService::upsert_prompt(&state, AppType::Claude, "draft", draft.clone()).unwrap();
        draft.content = "Edited draft\n".to_string();
        PromptService::upsert_prompt(&state, AppType::Claude, "draft", draft).unwrap();
        assert!(PromptService::import_from_file(&state, AppType::Claude).is_err());
        PromptService::delete_prompt(&state, AppType::Claude, "draft").unwrap();

        assert!(!live.exists());
        assert!(state.db.get_prompts("claude").unwrap().is_empty());
        assert_eq!(snapshot(live.parent().unwrap()), before);
    }

    #[test]
    #[serial]
    fn issue_141_enabled_update_disable_and_delete_keep_existing_semantics() {
        let (_home, _guard, state, live) = setup();
        let mut active = prompt("active", true);
        PromptService::upsert_prompt(&state, AppType::Claude, "ignored", active.clone()).unwrap();
        assert_eq!(fs::read_to_string(&live).unwrap(), active.content);
        assert!(!live.with_extension("md.fyagent.backup").exists());
        active.content = "Updated active text\n".to_string();
        PromptService::upsert_prompt(&state, AppType::Claude, "ignored", active.clone()).unwrap();
        assert_eq!(fs::read_to_string(&live).unwrap(), active.content);
        assert!(PromptService::delete_prompt(&state, AppType::Claude, &active.id).is_err());

        // The persisted key is active.id, not the unused auxiliary id argument.
        active.enabled = false;
        PromptService::upsert_prompt(&state, AppType::Claude, "ignored", active.clone()).unwrap();
        assert_eq!(fs::read(&live).unwrap(), b"");
        assert_eq!(
            fs::read_to_string(live.with_extension("md.fyagent.backup")).unwrap(),
            active.content
        );
        let before_delete = snapshot(live.parent().unwrap());
        PromptService::delete_prompt(&state, AppType::Claude, &active.id).unwrap();
        assert!(state.db.get_prompts("claude").unwrap().is_empty());
        assert_eq!(snapshot(live.parent().unwrap()), before_delete);
    }

    #[test]
    #[serial]
    fn issue_141_disabling_with_another_enabled_prompt_preserves_live() {
        let (_home, _guard, state, live) = setup();
        seed_live_with_recovery(&live);
        let mut active = prompt("active", true);
        state.db.save_prompt("claude", &active).unwrap();
        state
            .db
            .save_prompt("claude", &prompt("other", true))
            .unwrap();
        let before = snapshot(live.parent().unwrap());

        active.enabled = false;
        PromptService::upsert_prompt(&state, AppType::Claude, "active", active).unwrap();

        let saved = state.db.get_prompts("claude").unwrap();
        assert!(!saved["active"].enabled);
        assert!(saved["other"].enabled);
        assert_eq!(snapshot(live.parent().unwrap()), before);
    }
}
