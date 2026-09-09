use chrono::{Datelike, NaiveDate};
use regex::Regex;
use std::sync::LazyLock;
use tauri::AppHandle;

use crate::config::write_text_file;
use crate::openclaw_config::get_openclaw_dir;

/// Allowed workspace filenames (whitelist for security)
const ALLOWED_FILES: &[&str] = &[
    "AGENTS.md",
    "SOUL.md",
    "USER.md",
    "IDENTITY.md",
    "TOOLS.md",
    "MEMORY.md",
    "HEARTBEAT.md",
    "BOOTSTRAP.md",
    "BOOT.md",
];

fn validate_filename(filename: &str) -> Result<(), String> {
    if !ALLOWED_FILES.contains(&filename) {
        return Err(format!(
            "Invalid workspace filename: {filename}. Allowed: {}",
            ALLOWED_FILES.join(", ")
        ));
    }
    Ok(())
}

// --- Daily memory files (memory/YYYY-MM-DD.md) ---

static DAILY_MEMORY_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\A[0-9]{4}-[0-9]{2}-[0-9]{2}\.md\z").unwrap());

fn validate_daily_memory_filename(filename: &str) -> Result<(), String> {
    // Match the renderer's 0001..=9999 Gregorian date domain, including leap days.
    let valid = DAILY_MEMORY_RE.is_match(filename)
        && NaiveDate::parse_from_str(&filename[..10], "%Y-%m-%d")
            .is_ok_and(|date| date.year() >= 1);
    if !valid {
        return Err(format!(
            "Invalid daily memory filename: {filename}. Expected: YYYY-MM-DD.md"
        ));
    }
    Ok(())
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyMemoryFileInfo {
    pub filename: String,
    pub date: String,
    pub size_bytes: u64,
    pub modified_at: u64,
    pub preview: String,
}

// --- Daily memory commands ---

/// List all daily memory files under `workspace/memory/`.
#[tauri::command]
pub async fn list_daily_memory_files() -> Result<Vec<DailyMemoryFileInfo>, String> {
    let memory_dir = get_openclaw_dir().join("workspace").join("memory");

    if !memory_dir.exists() {
        return Ok(Vec::new());
    }

    let mut files: Vec<DailyMemoryFileInfo> = Vec::new();

    let entries = std::fs::read_dir(&memory_dir)
        .map_err(|e| format!("Failed to read memory directory: {e}"))?;

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if validate_daily_memory_filename(&name).is_err() {
            continue;
        }

        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        if !meta.is_file() {
            continue;
        }

        let date = name.trim_end_matches(".md").to_string();

        let size_bytes = meta.len();
        let modified_at = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let preview = std::fs::read_to_string(entry.path())
            .unwrap_or_default()
            .chars()
            .take(200)
            .collect::<String>();

        files.push(DailyMemoryFileInfo {
            filename: name,
            date,
            size_bytes,
            modified_at,
            preview,
        });
    }

    // Sort by filename descending (newest date first, YYYY-MM-DD.md)
    files.sort_by(|a, b| b.filename.cmp(&a.filename));

    Ok(files)
}

/// Read a daily memory file.
#[tauri::command]
pub async fn read_daily_memory_file(filename: String) -> Result<Option<String>, String> {
    validate_daily_memory_filename(&filename)?;

    let path = get_openclaw_dir()
        .join("workspace")
        .join("memory")
        .join(&filename);

    if !path.exists() {
        return Ok(None);
    }

    std::fs::read_to_string(&path)
        .map(Some)
        .map_err(|e| format!("Failed to read daily memory file {filename}: {e}"))
}

/// Write a daily memory file (atomic write).
#[tauri::command]
pub async fn write_daily_memory_file(filename: String, content: String) -> Result<(), String> {
    validate_daily_memory_filename(&filename)?;

    let memory_dir = get_openclaw_dir().join("workspace").join("memory");

    std::fs::create_dir_all(&memory_dir)
        .map_err(|e| format!("Failed to create memory directory: {e}"))?;

    let path = memory_dir.join(&filename);

    write_text_file(&path, &content)
        .map_err(|e| format!("Failed to write daily memory file {filename}: {e}"))
}

/// Find the largest index `<= i` that is a valid UTF-8 char boundary.
/// Equivalent to the unstable `str::floor_char_boundary` (stabilized in 1.91).
fn floor_char_boundary(s: &str, mut i: usize) -> usize {
    if i >= s.len() {
        return s.len();
    }
    while !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

/// Find the smallest index `>= i` that is a valid UTF-8 char boundary.
/// Equivalent to the unstable `str::ceil_char_boundary` (stabilized in 1.91).
fn ceil_char_boundary(s: &str, mut i: usize) -> usize {
    if i >= s.len() {
        return s.len();
    }
    while !s.is_char_boundary(i) {
        i += 1;
    }
    i
}

/// Search result for daily memory full-text search.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyMemorySearchResult {
    pub filename: String,
    pub date: String,
    pub size_bytes: u64,
    pub modified_at: u64,
    pub snippet: String,
    pub match_count: usize,
}

/// Full-text search across all daily memory files.
///
/// Performs case-insensitive search on both the date field and file content.
/// Returns results sorted by filename descending (newest first), each with a
/// snippet showing ~120 characters of context around the first match.
#[tauri::command]
pub async fn search_daily_memory_files(
    query: String,
) -> Result<Vec<DailyMemorySearchResult>, String> {
    let memory_dir = get_openclaw_dir().join("workspace").join("memory");

    if !memory_dir.exists() || query.is_empty() {
        return Ok(Vec::new());
    }

    let query_lower = query.to_lowercase();
    let mut results: Vec<DailyMemorySearchResult> = Vec::new();

    let entries = std::fs::read_dir(&memory_dir)
        .map_err(|e| format!("Failed to read memory directory: {e}"))?;

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if validate_daily_memory_filename(&name).is_err() {
            continue;
        }
        let meta = match entry.metadata() {
            Ok(m) if m.is_file() => m,
            _ => continue,
        };

        let date = name.trim_end_matches(".md").to_string();
        let content = std::fs::read_to_string(entry.path()).unwrap_or_default();
        let content_lower = content.to_lowercase();

        // Count matches in content
        let content_matches: Vec<usize> = content_lower
            .match_indices(&query_lower)
            .map(|(i, _)| i)
            .collect();

        // Also check date field
        let date_matches = date.to_lowercase().contains(&query_lower);

        if content_matches.is_empty() && !date_matches {
            continue;
        }

        // Build snippet around first content match (~120 chars of context)
        let snippet = if let Some(&first_pos) = content_matches.first() {
            let start = if first_pos > 50 {
                floor_char_boundary(&content, first_pos - 50)
            } else {
                0
            };
            let end = ceil_char_boundary(&content, (first_pos + 70).min(content.len()));
            let mut s = String::new();
            if start > 0 {
                s.push_str("...");
            }
            s.push_str(&content[start..end]);
            if end < content.len() {
                s.push_str("...");
            }
            s
        } else {
            // Date-only match — use beginning of file as preview
            let end = ceil_char_boundary(&content, 120.min(content.len()));
            let mut s = content[..end].to_string();
            if end < content.len() {
                s.push_str("...");
            }
            s
        };

        let size_bytes = meta.len();
        let modified_at = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        results.push(DailyMemorySearchResult {
            filename: name,
            date,
            size_bytes,
            modified_at,
            snippet,
            match_count: content_matches.len(),
        });
    }

    // Sort by filename descending (newest date first)
    results.sort_by(|a, b| b.filename.cmp(&a.filename));

    Ok(results)
}

/// Delete a daily memory file (idempotent).
#[tauri::command]
pub async fn delete_daily_memory_file(filename: String) -> Result<(), String> {
    validate_daily_memory_filename(&filename)?;

    let path = get_openclaw_dir()
        .join("workspace")
        .join("memory")
        .join(&filename);

    if path.exists() {
        std::fs::remove_file(&path)
            .map_err(|e| format!("Failed to delete daily memory file {filename}: {e}"))?;
    }

    Ok(())
}

// --- Workspace file commands ---

/// Read an OpenClaw workspace file content.
/// Returns None if the file does not exist.
#[tauri::command]
pub async fn read_workspace_file(filename: String) -> Result<Option<String>, String> {
    validate_filename(&filename)?;

    let path = get_openclaw_dir().join("workspace").join(&filename);

    if !path.exists() {
        return Ok(None);
    }

    std::fs::read_to_string(&path)
        .map(Some)
        .map_err(|e| format!("Failed to read workspace file {filename}: {e}"))
}

/// Write content to an OpenClaw workspace file (atomic write).
/// Creates the workspace directory if it does not exist.
#[tauri::command]
pub async fn write_workspace_file(filename: String, content: String) -> Result<(), String> {
    validate_filename(&filename)?;

    let workspace_dir = get_openclaw_dir().join("workspace");

    // Ensure workspace directory exists
    std::fs::create_dir_all(&workspace_dir)
        .map_err(|e| format!("Failed to create workspace directory: {e}"))?;

    let path = workspace_dir.join(&filename);

    write_text_file(&path, &content)
        .map_err(|e| format!("Failed to write workspace file {filename}: {e}"))
}

/// Open the workspace or memory directory in the system file manager.
/// `subdir`: "workspace" opens `~/.openclaw/workspace/`,
///           "memory" opens `~/.openclaw/workspace/memory/`.
#[tauri::command]
pub async fn open_workspace_directory(handle: AppHandle, subdir: String) -> Result<bool, String> {
    let dir = match subdir.as_str() {
        "memory" => get_openclaw_dir().join("workspace").join("memory"),
        _ => get_openclaw_dir().join("workspace"),
    };

    if !dir.exists() {
        std::fs::create_dir_all(&dir).map_err(|e| format!("Failed to create directory: {e}"))?;
    }

    crate::platform::process_launch::open_directory_as_user(handle, dir)
        .await
        .map_err(|error| format!("Failed to open directory: {error}"))?;

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::fs;
    use std::path::{Path, PathBuf};

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

    fn setup() -> (tempfile::TempDir, TestHome, PathBuf) {
        let home = tempfile::tempdir().unwrap();
        let guard = TestHome::set(home.path());
        let memory = get_openclaw_dir().join("workspace").join("memory");
        assert!(memory.starts_with(home.path()));
        (home, guard, memory)
    }

    #[test]
    fn issue_141_daily_filename_matches_renderer_calendar_domain() {
        for name in [
            "0001-01-01.md",
            "9999-12-31.md",
            "2000-02-29.md",
            "2024-02-29.md",
            "2026-09-08.md",
        ] {
            assert!(validate_daily_memory_filename(name).is_ok(), "{name}");
        }
        for name in [
            "README.md",
            "2026-02-30.md",
            "1900-02-29.md",
            "2026-04-31.md",
            "0000-01-01.md",
            "2026-00-01.md",
            "2026-01-00.md",
            "2026-13-01.md",
            "2026-2-01.md",
            "２０２６-０９-０８.md",
            "2026-09-08.md.bak",
            "2026-09-08.md\n",
            "../2026-09-08.md",
            "2026-09-08.md/child",
        ] {
            assert!(validate_daily_memory_filename(name).is_err(), "{name}");
        }
    }

    #[tokio::test]
    #[serial]
    async fn issue_141_daily_list_and_search_skip_invalid_files_and_directories() {
        let (_home, _guard, memory) = setup();
        fs::create_dir_all(&memory).unwrap();
        let content = format!("{} needle {}", "中文".repeat(40), "日记".repeat(80));
        for name in [
            "2026-09-08.md",
            "2024-02-29.md",
            "2000-02-29.md",
            "README.md",
            "2026-02-30.md",
            "1900-02-29.md",
            "0000-01-01.md",
            "２０２６-０９-０８.md",
            "2026-09-08.md.fyagent.backup",
        ] {
            fs::write(memory.join(name), &content).unwrap();
        }
        fs::create_dir(memory.join("2026-09-09.md")).unwrap();
        let expected = ["2026-09-08.md", "2024-02-29.md", "2000-02-29.md"];

        let listed = list_daily_memory_files().await.unwrap();
        assert_eq!(
            listed
                .iter()
                .map(|file| file.filename.as_str())
                .collect::<Vec<_>>(),
            expected
        );
        for file in &listed {
            assert_eq!(file.date, file.filename[..10]);
            assert_eq!(file.size_bytes, content.len() as u64);
            assert_eq!(file.preview, content.chars().take(200).collect::<String>());
            assert_eq!(
                read_daily_memory_file(file.filename.clone()).await.unwrap(),
                Some(content.clone())
            );
        }

        let found = search_daily_memory_files("NEEDLE".to_string())
            .await
            .unwrap();
        assert_eq!(
            found
                .iter()
                .map(|file| file.filename.as_str())
                .collect::<Vec<_>>(),
            expected
        );
        for file in found {
            assert_eq!(file.match_count, 1);
            assert_eq!(file.date, file.filename[..10]);
            assert!(file.snippet.contains("needle"));
            assert!(file.snippet.starts_with("..."));
            assert!(file.snippet.ends_with("..."));
        }
        let date_match = search_daily_memory_files("2024-02-29".to_string())
            .await
            .unwrap();
        assert_eq!(date_match.len(), 1);
        assert_eq!(date_match[0].filename, "2024-02-29.md");
        assert_eq!(date_match[0].match_count, 0);
    }

    #[tokio::test]
    #[serial]
    async fn issue_141_invalid_daily_crud_does_not_read_create_overwrite_or_delete() {
        let (_home, _guard, memory) = setup();
        let invalid = [
            "README.md",
            "2026-02-30.md",
            "0000-01-01.md",
            "２０２６-０９-０８.md",
        ];

        for name in invalid {
            assert!(
                read_daily_memory_file(name.to_string()).await.is_err(),
                "{name}"
            );
            assert!(
                write_daily_memory_file(name.to_string(), "new".to_string())
                    .await
                    .is_err(),
                "{name}"
            );
            assert!(
                delete_daily_memory_file(name.to_string()).await.is_err(),
                "{name}"
            );
        }
        assert!(
            !memory.exists(),
            "invalid writes must not create the directory"
        );

        fs::create_dir_all(&memory).unwrap();
        for name in invalid {
            let path = memory.join(name);
            fs::write(&path, "Existing non-daily data\n").unwrap();
            assert!(
                read_daily_memory_file(name.to_string()).await.is_err(),
                "{name}"
            );
            assert!(
                write_daily_memory_file(name.to_string(), "replacement".to_string())
                    .await
                    .is_err(),
                "{name}"
            );
            assert!(
                delete_daily_memory_file(name.to_string()).await.is_err(),
                "{name}"
            );
            assert_eq!(
                fs::read_to_string(path).unwrap(),
                "Existing non-daily data\n"
            );
        }
        assert_eq!(fs::read_dir(&memory).unwrap().count(), invalid.len());
    }

    #[tokio::test]
    #[serial]
    async fn issue_141_valid_daily_crud_preserves_leap_day_and_backup_semantics() {
        let (_home, _guard, memory) = setup();
        let filename = "2024-02-29.md".to_string();
        assert!(list_daily_memory_files().await.unwrap().is_empty());
        assert_eq!(
            read_daily_memory_file(filename.clone()).await.unwrap(),
            None
        );
        write_daily_memory_file(filename.clone(), "闰日日记\n".to_string())
            .await
            .unwrap();
        assert_eq!(
            read_daily_memory_file(filename.clone())
                .await
                .unwrap()
                .as_deref(),
            Some("闰日日记\n")
        );

        write_daily_memory_file(filename.clone(), "更新日记\n".to_string())
            .await
            .unwrap();
        assert_eq!(
            fs::read_to_string(memory.join("2024-02-29.md.fyagent.backup")).unwrap(),
            "闰日日记\n"
        );
        assert_eq!(
            read_daily_memory_file(filename.clone())
                .await
                .unwrap()
                .as_deref(),
            Some("更新日记\n")
        );
        delete_daily_memory_file(filename.clone()).await.unwrap();
        delete_daily_memory_file(filename.clone()).await.unwrap();
        assert_eq!(read_daily_memory_file(filename).await.unwrap(), None);
        assert!(list_daily_memory_files().await.unwrap().is_empty());
    }
}
