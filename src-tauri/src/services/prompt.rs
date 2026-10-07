use indexmap::IndexMap;

use crate::app_config::AppType;
use crate::config::write_text_file;
use crate::error::AppError;
use crate::prompt::Prompt;
use crate::prompt_files::prompt_file_path;
use crate::store::AppState;
use std::sync::{Mutex, MutexGuard};

#[cfg(test)]
thread_local! {
    static AFTER_PROMPT_WRITE_HOOK: std::cell::RefCell<Option<Box<dyn FnOnce()>>> = const { std::cell::RefCell::new(None) };
    static AFTER_PROMPT_RECEIPT_VALIDATION_HOOK: std::cell::RefCell<Option<Box<dyn FnOnce()>>> = const { std::cell::RefCell::new(None) };
    static BEFORE_PROMPT_LOCK_HOOK: std::cell::RefCell<Option<Box<dyn FnOnce()>>> = const { std::cell::RefCell::new(None) };
}

static PROMPT_MUTATION_LOCK: Mutex<()> = Mutex::new(());

// Prompt DB rows and the shared live file form one lifecycle state. Serialize
// each synchronous mutation from its first DB read through file/DAO commit and
// any compensation. Public wrappers own the lock; nested paths call `_unlocked`
// helpers so upsert/import can route into enable without recursively locking.
fn prompt_mutation_lock() -> MutexGuard<'static, ()> {
    #[cfg(test)]
    BEFORE_PROMPT_LOCK_HOOK.with(|hook| {
        if let Some(hook) = hook.borrow_mut().take() {
            hook();
        }
    });
    PROMPT_MUTATION_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn after_prompt_receipt_validation() {
    #[cfg(test)]
    AFTER_PROMPT_RECEIPT_VALIDATION_HOOK.with(|hook| {
        if let Some(hook) = hook.borrow_mut().take() {
            hook();
        }
    });
}

fn after_prompt_file_write() {
    #[cfg(test)]
    AFTER_PROMPT_WRITE_HOOK.with(|hook| {
        if let Some(hook) = hook.borrow_mut().take() {
            hook();
        }
    });
}

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
        let _mutation = prompt_mutation_lock();
        state.db.get_prompts(app.as_str())
    }

    pub fn upsert_prompt(
        state: &AppState,
        app: AppType,
        _id: &str,
        prompt: Prompt,
    ) -> Result<(), AppError> {
        let _mutation = prompt_mutation_lock();
        Self::upsert_prompt_unlocked(state, app, _id, prompt)
    }

    fn upsert_prompt_unlocked(
        state: &AppState,
        app: AppType,
        _id: &str,
        prompt: Prompt,
    ) -> Result<(), AppError> {
        let is_enabled = prompt.enabled;
        let was_enabled = state
            .db
            .get_prompts(app.as_str())?
            .get(&prompt.id)
            .is_some_and(|previous| previous.enabled);

        if is_enabled && !was_enabled {
            // Enabling is a switch, not a library upsert. Persist the content
            // as disabled first so enable_prompt owns the live/DAO transition.
            let mut draft = prompt;
            draft.enabled = false;
            let id = draft.id.clone();
            state.db.save_prompt(app.as_str(), &draft)?;
            return Self::enable_prompt_unlocked(state, app, &id);
        }

        if is_enabled {
            let target_path = prompt_file_path(&app)?;
            let (recovery_after, expected_postimage) =
                Self::write_prompt_file(&target_path, &prompt.content)?;
            if let Err(database_error) = state.db.save_prompt(app.as_str(), &prompt) {
                return match Self::compensate_prompt_file_write(
                    &target_path,
                    recovery_after,
                    expected_postimage,
                ) {
                    Ok(()) => Err(database_error),
                    Err(compensation_error) => Err(AppError::Message(format!(
                        "提示词库提交失败；live 文件补偿也未完成，请检查文件恢复记录。数据库错误：{database_error}；补偿错误：{compensation_error}"
                    ))),
                };
            }
            return Ok(());
        }

        if was_enabled {
            let any_other_enabled = state
                .db
                .get_prompts(app.as_str())?
                .iter()
                .any(|(id, other)| id != &prompt.id && other.enabled);
            if !any_other_enabled {
                let target_path = prompt_file_path(&app)?;
                if target_path.exists() {
                    let (recovery_after, expected_postimage) =
                        Self::write_prompt_file(&target_path, "")?;
                    if let Err(database_error) = state.db.save_prompt(app.as_str(), &prompt) {
                        return match Self::compensate_prompt_file_write(
                            &target_path,
                            recovery_after,
                            expected_postimage,
                        ) {
                            Ok(()) => Err(database_error),
                            Err(compensation_error) => Err(AppError::Message(format!(
                                "提示词停用未提交；live 文件补偿也未完成，请检查文件恢复记录。数据库错误：{database_error}；补偿错误：{compensation_error}"
                            ))),
                        };
                    }
                    return Ok(());
                }
            }
        }

        // Library-only updates and transitions with another enabled prompt do
        // not mutate the shared live file.
        state.db.save_prompt(app.as_str(), &prompt)?;
        Ok(())
    }

    fn write_prompt_file(
        target_path: &std::path::Path,
        content: &str,
    ) -> Result<(Option<String>, Option<Option<String>>), AppError> {
        let _scope = crate::config::file_mutation_scope();
        write_text_file(target_path, content)?;
        after_prompt_file_write();
        // Read the latest ID first, then verify the scope captured that
        // operation; reversing the order would reopen the receipt race window.
        let recovery_after = Self::prompt_file_recovery_id(target_path).map_err(|error| {
            AppError::Message(format!("live 文件已写入，但读取恢复回执失败：{error}"))
        })?;
        let expected_postimage =
            crate::config::file_mutation_expected_hash(target_path).map_err(|error| {
                AppError::Message(format!(
                    "live 文件已写入，但本次写入归属已变化；保留并发写入：{error}"
                ))
            })?;
        // The service lifecycle lock must remain held across this point and
        // the DAO commit; tests pause here to prove a second switch waits.
        after_prompt_receipt_validation();
        Ok((recovery_after, expected_postimage))
    }

    fn prompt_file_recovery_id(target_path: &std::path::Path) -> Result<Option<String>, AppError> {
        Ok(crate::config::file_recovery(target_path)?.map(|recovery| recovery.receipt_id))
    }

    fn compensate_prompt_file_write(
        target_path: &std::path::Path,
        recovery_after: Option<String>,
        expected_postimage: Option<Option<String>>,
    ) -> Result<(), AppError> {
        if expected_postimage.is_none() {
            // The scoped writer did not touch the file. In particular, never
            // treat an older undo marker as a receipt for this no-op.
            return Ok(());
        }
        let Some(receipt_id) = recovery_after else {
            return Err(AppError::Message(
                "找不到本次提示词文件写入的恢复回执".to_string(),
            ));
        };
        let recovery = crate::config::file_recovery(target_path)?;
        if recovery.as_ref().map(|entry| entry.receipt_id.as_str()) != Some(receipt_id.as_str()) {
            return Err(AppError::Message(
                "提示词文件恢复回执已变化，保留当前文件并停止补偿".to_string(),
            ));
        }
        match recovery.filter(|entry| entry.can_restore) {
            Some(_) => crate::config::restore_file_recovery(target_path, &receipt_id),
            None => Err(AppError::Message(
                "找不到可验证的提示词文件恢复记录".to_string(),
            )),
        }
    }

    pub fn delete_prompt(state: &AppState, app: AppType, id: &str) -> Result<(), AppError> {
        let _mutation = prompt_mutation_lock();
        Self::delete_prompt_unlocked(state, app, id)
    }

    fn delete_prompt_unlocked(state: &AppState, app: AppType, id: &str) -> Result<(), AppError> {
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
        let _mutation = prompt_mutation_lock();
        Self::enable_prompt_unlocked(state, app, id)
    }

    fn enable_prompt_unlocked(state: &AppState, app: AppType, id: &str) -> Result<(), AppError> {
        let target_path = prompt_file_path(&app)?;
        let mut prompts = state.db.get_prompts(app.as_str())?;
        if !prompts.contains_key(id) {
            return Err(AppError::InvalidInput(format!("提示词 {id} 不存在")));
        }

        // A live file that differs from the enabled library row may be an
        // external edit or the remainder of an earlier failed switch. Preserve
        // it as its own disabled entry; never assign it to the old row by ID.
        let live_before = if target_path.exists() {
            Some(std::fs::read(&target_path).map_err(|e| AppError::io(&target_path, e))?)
        } else {
            None
        };
        if let Some(bytes) = live_before.as_deref().filter(|bytes| !bytes.is_empty()) {
            let live_content = String::from_utf8(bytes.to_vec()).map_err(|error| {
                AppError::Message(format!("当前提示词文件不是有效 UTF-8，未执行切换：{error}"))
            })?;
            let differs_from_enabled = prompts
                .values()
                .find(|prompt| prompt.enabled)
                .is_some_and(|prompt| prompt.content.as_bytes() != bytes);
            let already_preserved = prompts
                .values()
                .any(|prompt| prompt.content == live_content);
            if (differs_from_enabled || !prompts.values().any(|prompt| prompt.enabled))
                && !already_preserved
            {
                let timestamp = get_unix_timestamp()?;
                let backup_id = format!("backup-{}", uuid::Uuid::new_v4());
                let backup = Prompt {
                    id: backup_id,
                    name: format!(
                        "外部提示词 {}",
                        chrono::Local::now().format("%Y-%m-%d %H:%M")
                    ),
                    content: live_content,
                    description: Some("切换前保留的当前文件内容".to_string()),
                    enabled: false,
                    created_at: Some(timestamp),
                    updated_at: Some(timestamp),
                };
                state.db.save_prompt(app.as_str(), &backup)?;
                prompts.insert(backup.id.clone(), backup);
            }
        }

        for prompt in prompts.values_mut() {
            prompt.enabled = prompt.id == id;
        }
        let target = prompts.get(id).expect("target checked above");
        let (recovery_after, expected_postimage) =
            Self::write_prompt_file(&target_path, &target.content)?;

        if let Err(database_error) = state
            .db
            .save_prompts(app.as_str(), &prompts.values().cloned().collect::<Vec<_>>())
        {
            let compensation = Self::compensate_prompt_file_write(
                &target_path,
                recovery_after,
                expected_postimage,
            );
            return match compensation {
                Ok(()) => Err(database_error),
                Err(compensation_error) => Err(AppError::Message(format!(
                    "提示词库提交失败；live 文件补偿也未完成，请检查文件恢复记录。数据库错误：{database_error}；补偿错误：{compensation_error}"
                ))),
            };
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
        let _mutation = prompt_mutation_lock();
        Self::import_from_file_at_unlocked(state, app, timestamp)
    }

    fn import_from_file_at_unlocked(
        state: &AppState,
        app: AppType,
        timestamp: i64,
    ) -> Result<String, AppError> {
        let file_path = prompt_file_path(&app)?;

        // Read directly: exists() would collapse metadata errors into absence.
        // Preserve the source read kind before creating a disabled library row.
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

        Self::upsert_prompt_unlocked(state, app, &id, prompt)?;
        Ok(id)
    }

    pub fn get_current_file_content(app: AppType) -> Result<Option<String>, AppError> {
        let _mutation = prompt_mutation_lock();
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
        let _mutation = prompt_mutation_lock();
        Self::import_from_file_on_first_launch_unlocked(state, app)
    }

    fn import_from_file_on_first_launch_unlocked(
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

    struct TestHome {
        previous_home: Option<std::ffi::OsString>,
        previous_settings: Option<crate::settings::AppSettings>,
    }

    impl TestHome {
        fn set(path: &Path) -> Self {
            #[cfg(target_os = "windows")]
            crate::initialize_windows_user_context().expect("Windows test user context");
            let previous_home = std::env::var_os("FYAGENT_TEST_HOME");
            let mut guard = Self {
                previous_home,
                previous_settings: None,
            };
            std::env::set_var("FYAGENT_TEST_HOME", path);
            assert_eq!(crate::config::get_home_dir(), path);
            // Snapshot after the override is active: lazy settings initialization
            // must resolve only inside this fixture, never the real user profile.
            guard.previous_settings = Some(crate::settings::get_settings());
            crate::settings::reload_settings().unwrap();
            guard
        }
    }

    impl Drop for TestHome {
        fn drop(&mut self) {
            // Restore the in-memory snapshot without persisting it. In
            // particular, do not reload after restoring FYAGENT_TEST_HOME:
            // on Windows that would consult the frozen Shell-user context.
            if let Some(settings) = self.previous_settings.take() {
                crate::settings::replace_settings_in_memory_for_test(settings);
            }
            match self.previous_home.take() {
                Some(value) => std::env::set_var("FYAGENT_TEST_HOME", value),
                None => std::env::remove_var("FYAGENT_TEST_HOME"),
            }
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
    fn public_import_missing_source_preserves_library_and_recovery() {
        let (_home, _guard, state, live) = setup();
        seed_live_with_recovery(&live);
        state
            .db
            .save_prompt("claude", &prompt("existing", false))
            .unwrap();
        fs::remove_file(&live).unwrap();
        let before = snapshot(live.parent().unwrap());
        let error = PromptService::import_from_file(&state, AppType::Claude).unwrap_err();
        assert!(matches!(error, AppError::Io { ref source, .. }
            if source.kind() == std::io::ErrorKind::NotFound));
        let library = state.db.get_prompts("claude").unwrap();
        assert_eq!(library.len(), 1);
        assert_eq!(
            library["existing"].content,
            prompt("existing", false).content
        );
        assert!(!live.exists());
        assert_eq!(snapshot(live.parent().unwrap()), before);
    }

    #[test]
    #[serial]
    fn public_import_directory_source_preserves_read_failure_kind_and_recovery() {
        let (_home, _guard, state, live) = setup();
        seed_live_with_recovery(&live);
        state
            .db
            .save_prompt("claude", &prompt("existing", false))
            .unwrap();
        fs::remove_file(&live).unwrap();
        fs::create_dir(&live).unwrap();
        fs::write(live.join("retained.txt"), "source directory preimage").unwrap();
        let source_before = snapshot(&live);
        let recovery_paths = [
            live.with_extension("md.fyagent.backup"),
            live.with_extension("md.fyagent.undo.json"),
        ];
        let recovery_snapshot = || {
            recovery_paths
                .iter()
                .map(|path| {
                    let metadata = fs::metadata(path).unwrap();
                    FileSnapshot {
                        name: path.file_name().unwrap().to_os_string(),
                        bytes: fs::read(path).unwrap(),
                        modified: metadata.modified().unwrap(),
                        permissions: metadata.permissions(),
                    }
                })
                .collect::<Vec<_>>()
        };
        let recovery_before = recovery_snapshot();
        let expected_kind = fs::read_to_string(&live).unwrap_err().kind();
        let error = PromptService::import_from_file(&state, AppType::Claude).unwrap_err();
        assert!(matches!(error, AppError::Io { ref source, .. }
            if source.kind() == expected_kind));
        let library = state.db.get_prompts("claude").unwrap();
        assert_eq!(library.len(), 1);
        assert_eq!(
            library["existing"].content,
            prompt("existing", false).content
        );
        assert!(live.is_dir());
        assert_eq!(snapshot(&live), source_before);
        assert_eq!(recovery_snapshot(), recovery_before);
    }

    #[test]
    #[serial]
    fn public_import_dao_refusal_is_not_source_io_and_preserves_preimages() {
        let (_home, _guard, state, live) = setup();
        seed_live_with_recovery(&live);
        state
            .db
            .save_prompt("claude", &prompt("existing", false))
            .unwrap();
        let before = snapshot(live.parent().unwrap());
        state
            .db
            .conn
            .lock()
            .unwrap()
            .execute_batch(
                "CREATE TEMP TRIGGER reject_public_import BEFORE INSERT ON prompts
             WHEN NEW.app_type = 'claude' AND NEW.id LIKE 'imported-%'
             BEGIN SELECT RAISE(ABORT, 'injected public import refusal'); END;",
            )
            .unwrap();
        let error = PromptService::import_from_file(&state, AppType::Claude).unwrap_err();
        assert!(matches!(error, AppError::Database(_)));
        let library = state.db.get_prompts("claude").unwrap();
        assert_eq!(library.len(), 1);
        assert_eq!(
            library["existing"].content,
            prompt("existing", false).content
        );
        assert_eq!(snapshot(live.parent().unwrap()), before);
    }

    #[test]
    #[serial]
    fn public_import_invalid_utf8_preserves_library_source_and_recovery() {
        let (_home, _guard, state, live) = setup();
        seed_live_with_recovery(&live);
        state
            .db
            .save_prompt("claude", &prompt("existing", false))
            .unwrap();
        fs::write(&live, [0xff, 0xfe, 0x80]).unwrap();
        let before = snapshot(live.parent().unwrap());
        let error = PromptService::import_from_file(&state, AppType::Claude).unwrap_err();
        assert!(matches!(error, AppError::Io { ref source, .. }
            if source.kind() == std::io::ErrorKind::InvalidData));
        let library = state.db.get_prompts("claude").unwrap();
        assert_eq!(library.len(), 1);
        assert_eq!(
            library["existing"].content,
            prompt("existing", false).content
        );
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
    fn iteration_resources_enable_db_rejection_keeps_old_prompt_and_retry_switches_cleanly() {
        let (_home, _guard, state, live) = setup();
        let mut original = prompt("switch-a", true);
        original.content = "Original A library body\n".to_string();
        PromptService::upsert_prompt(&state, AppType::Claude, "switch-a", original.clone())
            .unwrap();
        let mut target = prompt("switch-b", false);
        target.content = "Target B library body\n".to_string();
        PromptService::upsert_prompt(&state, AppType::Claude, "switch-b", target.clone()).unwrap();
        state
            .db
            .reject_prompt_state_for_test("switch-b", true, true);
        assert!(PromptService::enable_prompt(&state, AppType::Claude, "switch-b").is_err());
        let after_failure = state.db.get_prompts("claude").unwrap();
        assert!(after_failure["switch-a"].enabled);
        assert_eq!(after_failure["switch-a"].content, original.content);
        assert!(!after_failure["switch-b"].enabled);
        assert_eq!(fs::read_to_string(&live).unwrap(), original.content);
        assert_eq!(
            fs::read(live.with_extension("md.fyagent.backup")).unwrap(),
            original.content.as_bytes()
        );

        // Recreate the stale live/old-DB state that a crash or failed
        // compensation can leave; retry must not attribute B's bytes to A.
        write_text_file(&live, &target.content).unwrap();
        state
            .db
            .reject_prompt_state_for_test("switch-b", true, false);
        PromptService::enable_prompt(&state, AppType::Claude, "switch-b").unwrap();
        let after_retry = state.db.get_prompts("claude").unwrap();
        assert!(after_retry["switch-b"].enabled);
        assert!(!after_retry["switch-a"].enabled);
        assert_eq!(after_retry["switch-a"].content, original.content);
        assert_eq!(after_retry.values().filter(|item| item.enabled).count(), 1);
        assert_eq!(fs::read_to_string(&live).unwrap(), target.content);
    }

    #[test]
    #[serial]
    fn iteration_resources_enabled_edit_db_rejection_restores_library_and_live() {
        let (_home, _guard, state, live) = setup();
        let original = prompt("edit-enabled", true);
        PromptService::upsert_prompt(&state, AppType::Claude, "edit-enabled", original.clone())
            .unwrap();
        state
            .db
            .reject_prompt_state_for_test("edit-enabled", true, true);
        let mut edited = original.clone();
        edited.content = "Rejected edit body\n".to_string();

        assert!(
            PromptService::upsert_prompt(&state, AppType::Claude, "edit-enabled", edited,).is_err()
        );

        let saved = state.db.get_prompts("claude").unwrap();
        assert!(saved["edit-enabled"].enabled);
        assert_eq!(saved["edit-enabled"].content, original.content);
        assert_eq!(fs::read_to_string(&live).unwrap(), original.content);
        assert_eq!(
            fs::read(live.with_extension("md.fyagent.backup")).unwrap(),
            original.content.as_bytes()
        );
    }

    #[test]
    #[serial]
    fn iteration_resources_enable_file_refusal_leaves_prompt_library_unchanged() {
        let (_home, _guard, state, live) = setup();
        let original = prompt("file-a", true);
        let target = prompt("file-b", false);
        state.db.save_prompt("claude", &original).unwrap();
        state.db.save_prompt("claude", &target).unwrap();
        fs::create_dir(&live).unwrap();

        assert!(PromptService::enable_prompt(&state, AppType::Claude, "file-b").is_err());

        let saved = state.db.get_prompts("claude").unwrap();
        assert!(saved["file-a"].enabled);
        assert_eq!(saved["file-a"].content, original.content);
        assert!(!saved["file-b"].enabled);
        assert_eq!(fs::read_dir(&live).unwrap().count(), 0);
        assert!(!live.with_extension("md.fyagent.backup").exists());
    }

    #[test]
    #[serial]
    fn iteration_resources_disable_file_refusal_leaves_enabled_row_and_obstacle_unchanged() {
        let (_home, _guard, state, live) = setup();
        let active = prompt("disable-file", true);
        state.db.save_prompt("claude", &active).unwrap();
        fs::create_dir(&live).unwrap();
        fs::write(live.join("preserve.txt"), b"blocked live path").unwrap();
        let mut disabled = active.clone();
        disabled.enabled = false;

        assert!(
            PromptService::upsert_prompt(&state, AppType::Claude, "disable-file", disabled,)
                .is_err()
        );

        let saved = state.db.get_prompts("claude").unwrap();
        assert!(saved["disable-file"].enabled);
        assert_eq!(saved["disable-file"].content, active.content);
        assert_eq!(
            fs::read(live.join("preserve.txt")).unwrap(),
            b"blocked live path"
        );
        assert!(!live.with_extension("md.fyagent.backup").exists());
    }

    #[test]
    #[serial]
    fn iteration_resources_disable_db_refusal_restores_live_and_retry_commits_both() {
        let (_home, _guard, state, live) = setup();
        let active = prompt("disable-dao", true);
        state.db.save_prompt("claude", &active).unwrap();
        write_text_file(&live, &active.content).unwrap();
        state
            .db
            .reject_prompt_state_for_test("disable-dao", false, true);
        let mut disabled = active.clone();
        disabled.enabled = false;

        assert!(PromptService::upsert_prompt(
            &state,
            AppType::Claude,
            "disable-dao",
            disabled.clone(),
        )
        .is_err());
        let after_failure = state.db.get_prompts("claude").unwrap();
        assert!(after_failure["disable-dao"].enabled);
        assert_eq!(fs::read_to_string(&live).unwrap(), active.content);
        assert_eq!(
            fs::read(live.with_extension("md.fyagent.backup")).unwrap(),
            active.content.as_bytes()
        );

        state
            .db
            .reject_prompt_state_for_test("disable-dao", false, false);
        PromptService::upsert_prompt(&state, AppType::Claude, "disable-dao", disabled).unwrap();
        let after_retry = state.db.get_prompts("claude").unwrap();
        assert!(!after_retry["disable-dao"].enabled);
        assert_eq!(fs::read(&live).unwrap(), b"");
        assert!(
            crate::config::file_recovery(&live)
                .unwrap()
                .unwrap()
                .can_restore
        );
    }

    #[test]
    #[serial]
    fn iteration_resources_compensation_refuses_external_drift_and_noop_keeps_old_receipt() {
        let (_home, _guard, _state, live) = setup();
        write_text_file(&live, "Initial live\n").unwrap();
        let (operation_receipt, operation_postimage) =
            PromptService::write_prompt_file(&live, "Operation live\n").unwrap();
        write_text_file(&live, "External live edit\n").unwrap();
        let external_receipt = PromptService::prompt_file_recovery_id(&live).unwrap();

        assert!(PromptService::compensate_prompt_file_write(
            &live,
            operation_receipt,
            operation_postimage,
        )
        .is_err());
        assert_eq!(fs::read_to_string(&live).unwrap(), "External live edit\n");
        assert_eq!(
            PromptService::prompt_file_recovery_id(&live).unwrap(),
            external_receipt
        );

        let (noop_receipt, noop_postimage) =
            PromptService::write_prompt_file(&live, "External live edit\n").unwrap();
        assert_eq!(noop_receipt, external_receipt);
        assert!(noop_postimage.is_none());
        PromptService::compensate_prompt_file_write(&live, noop_receipt, noop_postimage).unwrap();
        assert_eq!(fs::read_to_string(&live).unwrap(), "External live edit\n");
        assert_eq!(
            PromptService::prompt_file_recovery_id(&live).unwrap(),
            external_receipt
        );
    }

    #[test]
    #[serial]
    fn iteration_resources_prompt_scope_rejects_other_managed_writer_before_receipt_capture() {
        let (_home, _guard, state, live) = setup();
        let original = prompt("writer-a", true);
        let target_b = prompt("writer-b", false);
        state.db.save_prompt("claude", &original).unwrap();
        state.db.save_prompt("claude", &target_b).unwrap();
        write_text_file(&live, &original.content).unwrap();

        let (written_tx, written_rx) = std::sync::mpsc::channel();
        let (resume_tx, resume_rx) = std::sync::mpsc::channel();
        let b_state = state.clone();
        let b = std::thread::spawn(move || {
            AFTER_PROMPT_WRITE_HOOK.with(|hook| {
                *hook.borrow_mut() = Some(Box::new(move || {
                    written_tx.send(()).unwrap();
                    resume_rx.recv().unwrap();
                }));
            });
            PromptService::enable_prompt(&b_state, AppType::Claude, "writer-b")
        });
        written_rx.recv().unwrap();
        // A different managed file writer does not share PromptService's
        // lifecycle mutex or thread-local operation. The scope identity guard
        // must still reject its fresh receipt instead of compensating it.
        let external_path = live.clone();
        std::thread::spawn(move || {
            write_text_file(&external_path, "Other managed writer content\n")
        })
        .join()
        .unwrap()
        .unwrap();
        let other_receipt = PromptService::prompt_file_recovery_id(&live).unwrap();
        resume_tx.send(()).unwrap();
        let error = b.join().unwrap().expect_err("B lost its receipt ownership");
        assert!(error.to_string().contains("归属已变化"), "{error}");
        let saved = state.db.get_prompts("claude").unwrap();
        assert!(saved["writer-a"].enabled);
        assert!(!saved["writer-b"].enabled);
        assert_eq!(saved["writer-a"].content, original.content);
        assert_eq!(
            fs::read_to_string(&live).unwrap(),
            "Other managed writer content\n"
        );
        assert_eq!(
            PromptService::prompt_file_recovery_id(&live).unwrap(),
            other_receipt
        );
    }

    #[test]
    #[serial]
    fn iteration_resources_prompt_enable_lifecycle_lock_serializes_two_successful_switches() {
        let (_home, _guard, state, live) = setup();
        let original = prompt("race-a", true);
        let target_b = prompt("race-b", false);
        let target_c = prompt("race-c", false);
        state.db.save_prompt("claude", &original).unwrap();
        state.db.save_prompt("claude", &target_b).unwrap();
        state.db.save_prompt("claude", &target_c).unwrap();
        write_text_file(&live, &original.content).unwrap();

        let (validated_tx, validated_rx) = std::sync::mpsc::channel();
        let (resume_tx, resume_rx) = std::sync::mpsc::channel();
        let b_state = state.clone();
        let b = std::thread::spawn(move || {
            AFTER_PROMPT_RECEIPT_VALIDATION_HOOK.with(|hook| {
                *hook.borrow_mut() = Some(Box::new(move || {
                    validated_tx.send(()).unwrap();
                    resume_rx.recv().unwrap();
                }));
            });
            PromptService::enable_prompt(&b_state, AppType::Claude, "race-b")
        });

        validated_rx.recv().unwrap();
        let (attempting_tx, attempting_rx) = std::sync::mpsc::channel();
        let (c_result_tx, c_result_rx) = std::sync::mpsc::channel();
        let c_state = state.clone();
        let c = std::thread::spawn(move || {
            BEFORE_PROMPT_LOCK_HOOK.with(|hook| {
                *hook.borrow_mut() = Some(Box::new(move || {
                    assert!(matches!(
                        PROMPT_MUTATION_LOCK.try_lock(),
                        Err(std::sync::TryLockError::WouldBlock)
                    ));
                    attempting_tx.send(()).unwrap();
                }));
            });
            let result = PromptService::enable_prompt(&c_state, AppType::Claude, "race-c");
            c_result_tx.send(result).unwrap();
        });
        attempting_rx.recv().unwrap();
        assert!(matches!(
            c_result_rx.try_recv(),
            Err(std::sync::mpsc::TryRecvError::Empty)
        ));
        resume_tx.send(()).unwrap();

        b.join().unwrap().unwrap();
        c.join().unwrap();
        c_result_rx.recv().unwrap().unwrap();
        let saved = state.db.get_prompts("claude").unwrap();
        assert!(!saved["race-a"].enabled);
        assert!(!saved["race-b"].enabled);
        assert!(saved["race-c"].enabled);
        assert_eq!(saved.values().filter(|prompt| prompt.enabled).count(), 1);
        assert_eq!(fs::read_to_string(&live).unwrap(), target_c.content);
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
