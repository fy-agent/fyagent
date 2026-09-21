//! Small bounded, read-only source primitives. No display projection is reused.
use std::path::Path;

use rusqlite::{Connection, OpenFlags};
use serde_json::Value;

use super::super::model::{limits, MigrationError, MigrationResult};

pub(super) const MAX_SOURCE_RECORDS: usize = limits::SESSION_MESSAGES as usize * 16;

pub(super) fn unreadable(provider: &str, reason: impl std::fmt::Display) -> MigrationError {
    MigrationError::SourceUnreadable {
        provider_id: provider.to_string(),
        reason: reason.to_string(),
    }
}

pub(super) fn check_budget(bytes: u64, records: usize) -> MigrationResult<()> {
    if bytes > limits::SOURCE_READ_BYTES {
        return Err(MigrationError::SourceTooLarge {
            read_bytes: bytes,
            limit: limits::SOURCE_READ_BYTES,
        });
    }
    if records >= MAX_SOURCE_RECORDS {
        return Err(MigrationError::TooManyMessages {
            count: records as u32,
            limit: MAX_SOURCE_RECORDS as u32,
        });
    }
    Ok(())
}

pub(super) fn parse_json(provider: &str, raw: &str) -> MigrationResult<Value> {
    super::super::package::reject_duplicate_keys(raw)?;
    serde_json::from_str(raw).map_err(|e| unreadable(provider, format!("invalid native JSON: {e}")))
}

pub(super) fn read_json(provider: &str, path: &Path) -> MigrationResult<Value> {
    use std::io::Read;
    let file = std::fs::File::open(path).map_err(|e| unreadable(provider, e))?;
    let mut raw = String::new();
    file.take(limits::SOURCE_READ_BYTES + 1)
        .read_to_string(&mut raw)
        .map_err(|e| unreadable(provider, e))?;
    check_budget(raw.len() as u64, 0)?;
    parse_json(provider, &raw)
}

pub(super) fn open_readonly(provider: &str, path: &Path) -> MigrationResult<Connection> {
    let conn = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|e| unreadable(provider, e))?;
    conn.busy_timeout(std::time::Duration::from_secs(2))
        .map_err(|e| unreadable(provider, e))?;
    // A single SQLite snapshot prevents a concurrent append or compaction from
    // making metadata, messages and parts describe different source revisions.
    conn.execute_batch("PRAGMA query_only=ON; BEGIN DEFERRED;")
        .map_err(|e| unreadable(provider, e))?;
    Ok(conn)
}

/// Query native text as ValueRef so its allocation is bounded before to_owned.
pub(super) fn text_column(
    row: &rusqlite::Row<'_>,
    index: usize,
    read_bytes: &mut u64,
) -> MigrationResult<Option<String>> {
    match row.get_ref(index).map_err(|e| unreadable("native", e))? {
        rusqlite::types::ValueRef::Null => Ok(None),
        rusqlite::types::ValueRef::Text(bytes) => {
            *read_bytes = read_bytes.saturating_add(bytes.len() as u64);
            check_budget(*read_bytes, 0)?;
            Ok(Some(
                std::str::from_utf8(bytes)
                    .map_err(|e| unreadable("native", e))?
                    .to_string(),
            ))
        }
        _ => Err(unreadable("native", "expected native text column")),
    }
}
