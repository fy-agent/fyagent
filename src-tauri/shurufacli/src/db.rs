//! 无守护进程时，SQLite 的「热」来自操作系统页缓存，而不是常驻连接。
//!
//! 每次 CLI 都是新进程：打开文件、读最近 N 条摘要、写本轮、退出。
//! 为把冷启动压到毫秒级：
//! - WAL + mmap：后续进程直接命中页缓存，相当于跨进程的内存视图
//! - `user_version` 快路径：已迁移的库跳过 DDL
//! - 每次写完 `wal_checkpoint(TRUNCATE)`：避免下次打开回放膨胀的 WAL
//! - 表极小，只存摘要，不存完整对话

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use rusqlite::{Connection, OpenFlags, OptionalExtension, params};

const SCHEMA_VERSION: i32 = 1;
const MMAP_BYTES: i64 = 8 * 1024 * 1024;
const CACHE_KIB: i64 = -1024;
const WAL_AUTOCHECKPOINT_PAGES: i64 = 64;
const JOURNAL_SIZE_LIMIT: i64 = 512 * 1024;

pub struct Store {
    conn: Connection,
}

impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("无法创建数据目录 {}", parent.display()))?;
        }

        let conn = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_CREATE
                | OpenFlags::SQLITE_OPEN_URI
                | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .with_context(|| format!("无法打开 SQLite {}", path.display()))?;

        configure_hot_path(&conn)?;
        migrate(&conn)?;
        Ok(Self { conn })
    }

    pub fn recent_summaries(&self, limit: usize) -> Result<Vec<String>> {
        if limit == 0 {
            return Ok(Vec::new());
        }

        let mut stmt = self
            .conn
            .prepare_cached("SELECT summary FROM turns ORDER BY id DESC LIMIT ?1")?;
        let rows = stmt.query_map(params![limit as i64], |row| row.get::<_, String>(0))?;
        let mut summaries = Vec::with_capacity(limit);
        for row in rows {
            summaries.push(row?);
        }
        summaries.reverse();
        Ok(summaries)
    }

    pub fn append_turn(&self, user_text: &str, summary: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO turns (created_at, user_text, summary) VALUES (?1, ?2, ?3)",
            params![now_unix(), user_text, summary],
        )?;
        checkpoint(&self.conn)
    }

    pub fn clear(&self) -> Result<usize> {
        let deleted = self.conn.execute("DELETE FROM turns", [])?;
        self.conn.execute_batch("VACUUM")?;
        configure_hot_path(&self.conn)?;
        checkpoint(&self.conn)?;
        Ok(deleted)
    }

    pub fn turn_count(&self) -> Result<usize> {
        let n: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM turns", [], |row| row.get(0))?;
        Ok(n as usize)
    }
}

fn configure_hot_path(conn: &Connection) -> Result<()> {
    conn.busy_timeout(std::time::Duration::from_millis(3000))?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.pragma_update(None, "temp_store", "MEMORY")?;
    conn.pragma_update(None, "mmap_size", MMAP_BYTES)?;
    conn.pragma_update(None, "cache_size", CACHE_KIB)?;
    conn.pragma_update(None, "wal_autocheckpoint", WAL_AUTOCHECKPOINT_PAGES)?;
    conn.pragma_update(None, "journal_size_limit", JOURNAL_SIZE_LIMIT)?;

    let mode: String = conn.query_row("PRAGMA journal_mode = WAL", [], |row| row.get(0))?;
    if !mode.eq_ignore_ascii_case("wal") {
        bail!("无法启用 WAL（当前 journal_mode={mode}）");
    }
    Ok(())
}

fn migrate(conn: &Connection) -> Result<()> {
    let version: i32 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    match version {
        0 => {
            conn.execute_batch(
                "
                CREATE TABLE turns (
                    id INTEGER PRIMARY KEY,
                    created_at INTEGER NOT NULL,
                    user_text TEXT NOT NULL,
                    summary TEXT NOT NULL
                );
                ",
            )?;
            conn.pragma_update(None, "user_version", SCHEMA_VERSION)?;
        }
        SCHEMA_VERSION => {}
        other => bail!("不支持的 SQLite schema 版本 {other}"),
    }
    Ok(())
}

fn checkpoint(conn: &Connection) -> Result<()> {
    conn.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(()))
        .optional()?;
    Ok(())
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn open_tmp() -> (TempDir, Store) {
        let dir = TempDir::new().unwrap();
        let store = Store::open(&dir.path().join("context.db")).unwrap();
        (dir, store)
    }

    #[test]
    fn append_and_read_oldest_first() {
        let (_dir, store) = open_tmp();
        store.append_turn("打开灯", "用户要开灯").unwrap();
        store.append_turn("调暗一点", "用户要把灯调暗").unwrap();
        store.append_turn("关掉", "用户要关灯").unwrap();

        let all = store.recent_summaries(8).unwrap();
        assert_eq!(all, ["用户要开灯", "用户要把灯调暗", "用户要关灯"]);

        let last_two = store.recent_summaries(2).unwrap();
        assert_eq!(last_two, ["用户要把灯调暗", "用户要关灯"]);
    }

    #[test]
    fn skip_ddl_on_second_open() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("context.db");
        {
            let store = Store::open(&path).unwrap();
            store.append_turn("你好", "打招呼").unwrap();
        }
        let store = Store::open(&path).unwrap();
        assert_eq!(store.turn_count().unwrap(), 1);
        assert_eq!(store.recent_summaries(4).unwrap(), ["打招呼"]);
    }

    #[test]
    fn clear_wipes_session() {
        let (_dir, store) = open_tmp();
        store.append_turn("a", "A").unwrap();
        store.append_turn("b", "B").unwrap();
        assert_eq!(store.clear().unwrap(), 2);
        assert_eq!(store.turn_count().unwrap(), 0);
        assert!(store.recent_summaries(8).unwrap().is_empty());
    }
}
