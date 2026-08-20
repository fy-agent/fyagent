#[cfg(any(target_os = "windows", test))]
use serde_json::Map;
use serde_json::Value;
#[cfg(any(target_os = "windows", test))]
use std::path::Path;
use std::path::PathBuf;
use std::sync::{OnceLock, RwLock};
#[cfg(target_os = "macos")]
use tauri_plugin_store::StoreExt;

use crate::error::AppError;

/// Store 中的键名
const STORE_KEY_APP_CONFIG_DIR: &str = "app_config_dir_override";
#[cfg(any(target_os = "windows", test))]
const WINDOWS_APP_PATHS_STORE_MAX_BYTES: usize = 64 * 1024;

/// 缓存当前的 app_config_dir 覆盖路径，避免存储 AppHandle
static APP_CONFIG_DIR_OVERRIDE: OnceLock<RwLock<Option<PathBuf>>> = OnceLock::new();

fn override_cache() -> &'static RwLock<Option<PathBuf>> {
    APP_CONFIG_DIR_OVERRIDE.get_or_init(|| RwLock::new(None))
}

fn update_cached_override(value: Option<PathBuf>) {
    if let Ok(mut guard) = override_cache().write() {
        *guard = value;
    }
}

/// 获取缓存中的 app_config_dir 覆盖路径
pub fn get_app_config_dir_override() -> Option<PathBuf> {
    override_cache().read().ok()?.clone()
}

#[cfg(target_os = "macos")]
fn read_override_from_store(app: &tauri::AppHandle) -> Option<PathBuf> {
    let store = match app.store_builder("app_paths.json").build() {
        Ok(store) => store,
        Err(e) => {
            log::warn!("无法创建 Store: {e}");
            return None;
        }
    };

    match store.get(STORE_KEY_APP_CONFIG_DIR) {
        Some(Value::String(path_str)) => {
            let path_str = path_str.trim();
            if path_str.is_empty() {
                return None;
            }

            let path = resolve_path(path_str);

            if !path.exists() {
                log::warn!(
                    "Store 中配置的 app_config_dir 不存在: {path:?}\n\
                     将使用默认路径。"
                );
                return None;
            }

            log::info!("使用 Store 中的 app_config_dir: {path:?}");
            Some(path)
        }
        Some(_) => {
            log::warn!("Store 中的 {STORE_KEY_APP_CONFIG_DIR} 类型不正确，应为字符串");
            None
        }
        None => None,
    }
}

#[cfg(target_os = "windows")]
fn windows_store_path(app: &tauri::AppHandle) -> PathBuf {
    crate::windows_runtime::tauri_user_store_path(&app.config().identifier, "app_paths.json")
}

#[cfg(any(target_os = "windows", test))]
fn windows_store_limit_error(max_bytes: usize) -> AppError {
    AppError::Message(format!(
        "Windows app_paths Store 超过 {max_bytes} 字节上限；现有文件未修改，请先重置 app_config_dir 以重建该可选 Store"
    ))
}

/// Load the optional Windows store for an update without allowing a large
/// Shell-user-controlled file to allocate unbounded memory in the elevated
/// process. An oversized store is preserved for ordinary updates. A caller
/// that explicitly resets the optional override may replace it with a fresh
/// empty document so the setting remains recoverable.
#[cfg(any(target_os = "windows", test))]
fn load_windows_store_for_update(
    store_path: &Path,
    reset_requested: bool,
    max_bytes: usize,
) -> Result<Map<String, Value>, AppError> {
    match crate::config::read_bounded_file(store_path, max_bytes) {
        Ok(bytes) => Ok(serde_json::from_slice::<Map<String, Value>>(&bytes).unwrap_or_default()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Map::new()),
        Err(error) if error.kind() == std::io::ErrorKind::InvalidData && reset_requested => {
            log::warn!(
                "Windows Shell 用户路径 Store 超过 {max_bytes} 字节；显式重置将重建该可选 Store"
            );
            Ok(Map::new())
        }
        Err(error) if error.kind() == std::io::ErrorKind::InvalidData => {
            Err(windows_store_limit_error(max_bytes))
        }
        Err(error) => Err(AppError::io(store_path, error)),
    }
}

#[cfg(any(target_os = "windows", test))]
fn encode_windows_store(
    document: &Map<String, Value>,
    max_bytes: usize,
) -> Result<Vec<u8>, AppError> {
    let encoded = serde_json::to_vec_pretty(document)
        .map_err(|error| AppError::Message(format!("序列化 Store 失败: {error}")))?;
    if encoded.len() > max_bytes {
        return Err(windows_store_limit_error(max_bytes));
    }
    Ok(encoded)
}

#[cfg(any(target_os = "windows", test))]
fn write_windows_store(
    store_path: &Path,
    document: &Map<String, Value>,
    max_bytes: usize,
) -> Result<(), AppError> {
    let encoded = encode_windows_store(document, max_bytes)?;
    crate::config::atomic_write(store_path, &encoded)
}

#[cfg(any(target_os = "windows", test))]
fn valid_override(raw: &str, require_absolute: bool) -> Option<PathBuf> {
    let resolved = resolve_path(raw.trim());
    ((!require_absolute || resolved.is_absolute()) && resolved.is_dir()).then_some(resolved)
}

#[cfg(target_os = "windows")]
fn read_override_from_store(app: &tauri::AppHandle) -> Option<PathBuf> {
    let path = windows_store_path(app);
    let bytes = match crate::config::read_bounded_file(&path, WINDOWS_APP_PATHS_STORE_MAX_BYTES) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return None,
        Err(error) => {
            log::warn!("无法读取 Windows Shell 用户路径 Store: {error}");
            return None;
        }
    };
    let document: Value = match serde_json::from_slice(&bytes) {
        Ok(document) => document,
        Err(error) => {
            log::warn!("Windows Shell 用户路径 Store 已损坏: {error}");
            return None;
        }
    };

    match document.get(STORE_KEY_APP_CONFIG_DIR) {
        Some(Value::String(path_str)) if !path_str.trim().is_empty() => {
            if let Some(resolved) = valid_override(path_str, true) {
                Some(resolved)
            } else {
                log::warn!(
                    "Store 中配置的 Windows app_config_dir 不是已存在的绝对目录，将使用默认路径"
                );
                None
            }
        }
        Some(Value::String(_)) | None => None,
        Some(_) => {
            log::warn!("Store 中的 {STORE_KEY_APP_CONFIG_DIR} 类型不正确，应为字符串");
            None
        }
    }
}

/// 从 Store 刷新 app_config_dir 覆盖值并更新缓存
pub fn refresh_app_config_dir_override(app: &tauri::AppHandle) -> Option<PathBuf> {
    let value = read_override_from_store(app);
    update_cached_override(value.clone());
    value
}

/// 写入 app_config_dir 到 Tauri Store
pub fn set_app_config_dir_to_store(
    app: &tauri::AppHandle,
    path: Option<&str>,
) -> Result<(), AppError> {
    #[cfg(target_os = "windows")]
    {
        let store_path = windows_store_path(app);
        let requested_path = path.map(str::trim).filter(|path| !path.is_empty());
        let mut document = load_windows_store_for_update(
            &store_path,
            requested_path.is_none(),
            WINDOWS_APP_PATHS_STORE_MAX_BYTES,
        )?;

        match requested_path {
            Some(path) => {
                let resolved = valid_override(path, true).ok_or_else(|| {
                    AppError::Message("Windows app_config_dir 必须是已存在的绝对目录".to_owned())
                })?;
                document.insert(
                    STORE_KEY_APP_CONFIG_DIR.to_owned(),
                    Value::String(resolved.to_string_lossy().into_owned()),
                );
            }
            None => {
                document.remove(STORE_KEY_APP_CONFIG_DIR);
            }
        }

        write_windows_store(&store_path, &document, WINDOWS_APP_PATHS_STORE_MAX_BYTES)?;
        refresh_app_config_dir_override(app);
        Ok(())
    }

    #[cfg(target_os = "macos")]
    {
        let store = app
            .store_builder("app_paths.json")
            .build()
            .map_err(|e| AppError::Message(format!("创建 Store 失败: {e}")))?;

        match path {
            Some(p) => {
                let trimmed = p.trim();
                if !trimmed.is_empty() {
                    store.set(STORE_KEY_APP_CONFIG_DIR, Value::String(trimmed.to_string()));
                    log::info!("已将 app_config_dir 写入 Store: {trimmed}");
                } else {
                    store.delete(STORE_KEY_APP_CONFIG_DIR);
                    log::info!("已从 Store 中删除 app_config_dir 配置");
                }
            }
            None => {
                store.delete(STORE_KEY_APP_CONFIG_DIR);
                log::info!("已从 Store 中删除 app_config_dir 配置");
            }
        }

        store
            .save()
            .map_err(|e| AppError::Message(format!("保存 Store 失败: {e}")))?;

        refresh_app_config_dir_override(app);
        Ok(())
    }
}

/// 解析路径，支持 ~ 开头的相对路径
fn resolve_path(raw: &str) -> PathBuf {
    if raw == "~" {
        return crate::config::get_home_dir();
    } else if let Some(stripped) = raw.strip_prefix("~/") {
        return crate::config::get_home_dir().join(stripped);
    } else if let Some(stripped) = raw.strip_prefix("~\\") {
        return crate::config::get_home_dir().join(stripped);
    }

    PathBuf::from(raw)
}

/// 从旧的 settings.json 迁移 app_config_dir 到 Store
pub fn migrate_app_config_dir_from_settings(app: &tauri::AppHandle) -> Result<(), AppError> {
    // app_config_dir 已从 settings.json 移除，此函数保留但不再执行迁移
    // 如果用户在旧版本设置过 app_config_dir，需要在 Store 中手动配置
    log::info!("app_config_dir 迁移功能已移除，请在设置中重新配置");

    let _ = refresh_app_config_dir_override(app);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn document_with_encoded_size(size: usize) -> Map<String, Value> {
        let mut document = Map::new();
        document.insert("padding".to_owned(), Value::String(String::new()));
        let fixed_size = serde_json::to_vec_pretty(&document).unwrap().len();
        assert!(fixed_size <= size);
        document.insert(
            "padding".to_owned(),
            Value::String("x".repeat(size - fixed_size)),
        );
        document
    }

    #[test]
    fn absolute_override_validation_rejects_cwd_and_non_directories() {
        let directory = tempfile::tempdir().expect("temporary override directory");
        assert_eq!(
            valid_override(&directory.path().to_string_lossy(), true),
            Some(directory.path().to_path_buf())
        );
        assert_eq!(valid_override(".", true), None);

        let file = directory.path().join("not-a-directory");
        std::fs::write(&file, b"fixture").expect("write override fixture");
        assert_eq!(valid_override(&file.to_string_lossy(), true), None);
    }

    #[test]
    fn windows_store_encoder_accepts_exact_limit_and_rejects_limit_plus_one() {
        let max_bytes = WINDOWS_APP_PATHS_STORE_MAX_BYTES;
        let exact = document_with_encoded_size(max_bytes);
        assert_eq!(
            encode_windows_store(&exact, max_bytes).unwrap().len(),
            max_bytes
        );

        let over_limit = document_with_encoded_size(max_bytes + 1);
        let error = encode_windows_store(&over_limit, max_bytes).unwrap_err();
        assert!(error
            .to_string()
            .contains(&format!("超过 {max_bytes} 字节上限")));
    }

    #[test]
    fn windows_store_limit_applies_to_pretty_encoded_bytes() {
        let document = Map::from_iter([
            ("first".to_owned(), Value::String("one".to_owned())),
            ("second".to_owned(), Value::String("two".to_owned())),
        ]);
        let compact_size = serde_json::to_vec(&document).unwrap().len();
        let pretty_size = serde_json::to_vec_pretty(&document).unwrap().len();
        assert!(pretty_size > compact_size);

        let error = encode_windows_store(&document, compact_size).unwrap_err();
        assert!(error
            .to_string()
            .contains(&format!("超过 {compact_size} 字节上限")));
    }

    #[test]
    fn windows_store_write_is_immediately_readable_with_the_same_limit() {
        let directory = tempfile::tempdir().unwrap();
        let store_path = directory.path().join("app_paths.json");
        let document = Map::from_iter([(
            STORE_KEY_APP_CONFIG_DIR.to_owned(),
            Value::String("C:\\Users\\Alice\\.fyagent".to_owned()),
        )]);

        write_windows_store(&store_path, &document, WINDOWS_APP_PATHS_STORE_MAX_BYTES).unwrap();
        let bytes =
            crate::config::read_bounded_file(&store_path, WINDOWS_APP_PATHS_STORE_MAX_BYTES)
                .unwrap();
        assert_eq!(
            serde_json::from_slice::<Map<String, Value>>(&bytes).unwrap(),
            document
        );
    }

    #[test]
    fn over_limit_update_preserves_existing_store() {
        let directory = tempfile::tempdir().unwrap();
        let store_path = directory.path().join("app_paths.json");
        let original = br#"{"keep":"original"}"#;
        std::fs::write(&store_path, original).unwrap();
        let document = document_with_encoded_size(129);

        let error = write_windows_store(&store_path, &document, 128).unwrap_err();
        assert!(error.to_string().contains("现有文件未修改"));
        assert_eq!(std::fs::read(&store_path).unwrap(), original);
    }

    #[test]
    fn explicit_reset_recovers_an_oversized_optional_store() {
        let directory = tempfile::tempdir().unwrap();
        let store_path = directory.path().join("app_paths.json");
        let max_bytes = 64;
        let oversized = document_with_encoded_size(max_bytes + 1);
        let oversized_bytes = serde_json::to_vec_pretty(&oversized).unwrap();
        std::fs::write(&store_path, &oversized_bytes).unwrap();

        let error = load_windows_store_for_update(&store_path, false, max_bytes).unwrap_err();
        assert!(error.to_string().contains("请先重置 app_config_dir"));
        assert_eq!(std::fs::read(&store_path).unwrap(), oversized_bytes);

        let reset_document = load_windows_store_for_update(&store_path, true, max_bytes).unwrap();
        assert!(reset_document.is_empty());
        write_windows_store(&store_path, &reset_document, max_bytes).unwrap();

        let repaired = crate::config::read_bounded_file(&store_path, max_bytes).unwrap();
        assert_eq!(
            serde_json::from_slice::<Map<String, Value>>(&repaired).unwrap(),
            Map::new()
        );
    }
}
