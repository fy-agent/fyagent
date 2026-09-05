//! One rolling preimage below every managed JSON/text/auth writer.
//!
//! The sidecar is written before the primary and contains only local recovery
//! control data. Its hashes never cross IPC. A crash or external edit cannot
//! turn a stale backup into permission to overwrite the current file.

use std::fs::{self, File, OpenOptions};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::{cell::RefCell, collections::HashMap, marker::PhantomData, rc::Rc};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::{atomic_write_unbacked, rolling_backup_path};
use crate::error::AppError;

const MAX_FILE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_RECORD_BYTES: u64 = 4096;
static WRITER: Mutex<()> = Mutex::new(());

thread_local! {
    static OPERATION: RefCell<Option<HashMap<PathBuf, String>>> = const { RefCell::new(None) };
}

/// Synchronous domain writers may touch one config several times (for example
/// source selection followed by MCP reconciliation). Keep the operation's
/// first preimage, not the last internal intermediate file. This guard adds no
/// write authority and must never cross an await/thread boundary.
pub(crate) struct FileMutationScope {
    owner: bool,
    not_send: PhantomData<Rc<()>>,
}

pub(crate) fn file_mutation_scope() -> FileMutationScope {
    let owner = OPERATION.with(|operation| {
        let mut operation = operation.borrow_mut();
        if operation.is_some() {
            false
        } else {
            *operation = Some(HashMap::new());
            true
        }
    });
    FileMutationScope {
        owner,
        not_send: PhantomData,
    }
}

impl Drop for FileMutationScope {
    fn drop(&mut self) {
        if self.owner {
            OPERATION.with(|operation| *operation.borrow_mut() = None);
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RecoveryRecord {
    version: u8,
    receipt_id: String,
    path_sha256: String,
    preimage_sha256: Option<String>,
    postimage_sha256: Option<String>,
}

/// Display metadata only. The transport owner supplies its trusted path label.
pub(crate) struct FileRecovery {
    pub receipt_id: String,
    pub had_file: bool,
    pub can_restore: bool,
}

fn failure(code: &'static str) -> AppError {
    AppError::Config(code.to_string())
}

fn record_path(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".fyagent.undo.json");
    path.with_file_name(name)
}

fn path_digest(path: &Path) -> String {
    format!("{:x}", Sha256::digest(path.as_os_str().as_encoded_bytes()))
}

pub(super) fn validate_file_leaf(path: &Path) -> Result<(), AppError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if !metadata.is_file() || metadata.file_type().is_symlink() {
                return Err(failure("config_file_not_regular"));
            }
            #[cfg(target_os = "windows")]
            {
                use std::os::windows::fs::MetadataExt;
                use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;
                if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
                    return Err(failure("config_file_not_regular"));
                }
            }
            Ok(())
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(AppError::io(path, error)),
    }
}

fn read_file(path: &Path, limit: u64) -> Result<Option<Vec<u8>>, AppError> {
    validate_file_leaf(path)?;
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(target_os = "macos")]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT;
        options.custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    }
    let file = match options.open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(AppError::io(path, error)),
    };
    let metadata = file.metadata().map_err(|error| AppError::io(path, error))?;
    if !metadata.is_file() || metadata.len() > limit {
        return Err(failure("config_file_size_or_type_invalid"));
    }
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::fs::MetadataExt;
        use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;
        if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return Err(failure("config_file_not_regular"));
        }
    }
    let mut bytes = Vec::new();
    file.take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| AppError::io(path, error))?;
    if bytes.len() as u64 > limit {
        return Err(failure("config_file_size_or_type_invalid"));
    }
    Ok(Some(bytes))
}

fn digest(bytes: Option<&[u8]>) -> Option<String> {
    bytes.map(|bytes| format!("{:x}", Sha256::digest(bytes)))
}

fn replace(path: &Path, bytes: Option<&[u8]>, private: bool) -> Result<(), AppError> {
    validate_file_leaf(path)?;
    match bytes {
        Some(bytes) => atomic_write_unbacked(path, bytes, private),
        None => match fs::remove_file(path) {
            Ok(()) => {
                #[cfg(target_os = "macos")]
                if let Some(parent) = path.parent() {
                    File::open(parent)
                        .and_then(|directory| directory.sync_all())
                        .map_err(|error| AppError::io(parent, error))?;
                }
                Ok(())
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(AppError::io(path, error)),
        },
    }
}

pub(super) fn write(path: &Path, bytes: Option<&[u8]>, private: bool) -> Result<(), AppError> {
    write_with(path, bytes, |path, bytes| replace(path, bytes, private))
}

fn write_with(
    path: &Path,
    bytes: Option<&[u8]>,
    apply: impl FnOnce(&Path, Option<&[u8]>) -> Result<(), AppError>,
) -> Result<(), AppError> {
    let _guard = WRITER
        .lock()
        .map_err(|_| failure("config_writer_unavailable"))?;
    if bytes.is_some_and(|bytes| bytes.len() as u64 > MAX_FILE_BYTES) {
        return Err(failure("config_file_size_or_type_invalid"));
    }
    let before = read_file(path, MAX_FILE_BYTES)?;
    if before.as_deref() == bytes {
        return Ok(()); // Preserve the last useful undo, including on repeated saves.
    }
    let backup = rolling_backup_path(path);
    let marker = record_path(path);
    let old_backup = read_file(&backup, MAX_FILE_BYTES)?;
    let old_marker = read_file(&marker, MAX_RECORD_BYTES)?;
    let restore_history = || -> Result<(), AppError> {
        replace(&backup, old_backup.as_deref(), true)?;
        replace(&marker, old_marker.as_deref(), true)
    };
    let previous_receipt = OPERATION.with(|operation| {
        operation
            .borrow()
            .as_ref()
            .and_then(|paths| paths.get(path).cloned())
    });
    let continuation = previous_receipt
        .map(|receipt| -> Result<RecoveryRecord, AppError> {
            let record = read_record(path)?.ok_or_else(|| failure("config_recovery_missing"))?;
            if record.receipt_id != receipt
                || record.postimage_sha256 != digest(before.as_deref())
                || record.preimage_sha256 != digest(old_backup.as_deref())
            {
                return Err(failure("config_external_change"));
            }
            Ok(record)
        })
        .transpose()?;
    let keeps_preimage = continuation.is_some();
    let mut record = continuation.unwrap_or_else(|| RecoveryRecord {
        version: 1,
        receipt_id: uuid::Uuid::new_v4().to_string(),
        path_sha256: path_digest(path),
        preimage_sha256: digest(before.as_deref()),
        postimage_sha256: digest(bytes),
    });
    record.postimage_sha256 = digest(bytes);
    let encoded = serde_json::to_vec(&record).map_err(|_| failure("config_recovery_invalid"))?;
    // Every prerequisite is durable before the first primary-file mutation.
    if let Err(error) = (if keeps_preimage {
        Ok(())
    } else {
        replace(&backup, before.as_deref(), true)
    })
    .and_then(|()| replace(&marker, Some(&encoded), true))
    {
        restore_history()?;
        return Err(error);
    }
    if read_file(path, MAX_FILE_BYTES)?.as_deref() != before.as_deref() {
        restore_history()?;
        return Err(failure("config_external_change"));
    }
    let result = apply(path, bytes);
    let after = read_file(path, MAX_FILE_BYTES);
    if result.is_ok() && after.as_ref().is_ok_and(|after| after.as_deref() == bytes) {
        OPERATION.with(|operation| {
            if let Some(paths) = operation.borrow_mut().as_mut() {
                paths.insert(path.to_path_buf(), record.receipt_id);
            }
        });
        return Ok(());
    }

    // A replace may have completed before a durability/readback error. Roll it
    // back only if the file still contains our postimage, never an external edit.
    match after {
        Ok(after) if after.as_deref() == bytes => {
            replace(path, before.as_deref(), false)?;
            if read_file(path, MAX_FILE_BYTES)? != before {
                return Err(failure("config_recovery_required"));
            }
            restore_history()?;
        }
        Ok(after) if after == before => restore_history()?,
        _ => return Err(failure("config_recovery_required")),
    }
    result.and(Err(failure("config_write_readback_failed")))
}

fn read_record(path: &Path) -> Result<Option<RecoveryRecord>, AppError> {
    let Some(bytes) = read_file(&record_path(path), MAX_RECORD_BYTES)? else {
        return Ok(None);
    };
    let record: RecoveryRecord =
        serde_json::from_slice(&bytes).map_err(|_| failure("config_recovery_invalid"))?;
    let uuid = uuid::Uuid::parse_str(&record.receipt_id)
        .map_err(|_| failure("config_recovery_invalid"))?;
    let valid_hash = |hash: &Option<String>| {
        hash.as_ref().is_none_or(|hash| {
            hash.len() == 64
                && hash
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        })
    };
    if record.version != 1
        || record.path_sha256 != path_digest(path)
        || uuid.get_version() != Some(uuid::Version::Random)
        || uuid.to_string() != record.receipt_id
        || !valid_hash(&record.preimage_sha256)
        || !valid_hash(&record.postimage_sha256)
    {
        return Err(failure("config_recovery_invalid"));
    }
    Ok(Some(record))
}

pub(crate) fn file_recovery(path: &Path) -> Result<Option<FileRecovery>, AppError> {
    let _guard = WRITER
        .lock()
        .map_err(|_| failure("config_writer_unavailable"))?;
    let Some(record) = read_record(path)? else {
        return Ok(None);
    };
    let current = read_file(path, MAX_FILE_BYTES)?;
    let backup = read_file(&rolling_backup_path(path), MAX_FILE_BYTES)?;
    Ok(Some(FileRecovery {
        receipt_id: record.receipt_id,
        had_file: record.preimage_sha256.is_some(),
        can_restore: digest(current.as_deref()) == record.postimage_sha256
            && digest(backup.as_deref()) == record.preimage_sha256,
    }))
}

pub(crate) fn restore_file_recovery(path: &Path, receipt_id: &str) -> Result<(), AppError> {
    let _guard = WRITER
        .lock()
        .map_err(|_| failure("config_writer_unavailable"))?;
    let record = read_record(path)?.ok_or_else(|| failure("config_recovery_missing"))?;
    if record.receipt_id != receipt_id {
        return Err(failure("config_recovery_stale"));
    }
    let current = read_file(path, MAX_FILE_BYTES)?;
    let current_hash = digest(current.as_deref());
    let backup = read_file(&rolling_backup_path(path), MAX_FILE_BYTES)?;
    if digest(backup.as_deref()) != record.preimage_sha256 {
        return Err(failure("config_recovery_invalid"));
    }
    if current_hash != record.postimage_sha256 && current_hash != record.preimage_sha256 {
        return Err(failure("config_external_change"));
    }
    // Also converges a crash after restoration but before marker removal.
    if current_hash != record.preimage_sha256 {
        replace(path, backup.as_deref(), false)?;
    }
    if digest(read_file(path, MAX_FILE_BYTES)?.as_deref()) != record.preimage_sha256 {
        return Err(failure("config_recovery_required"));
    }
    replace(&record_path(path), None, true)
}

pub(super) fn restore_preimage(path: &Path, bytes: Option<&[u8]>) -> Result<(), AppError> {
    let _guard = WRITER
        .lock()
        .map_err(|_| failure("config_writer_unavailable"))?;
    replace(path, bytes, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_operation_retains_the_first_preimage_across_internal_rewrites() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(&path, b"original").unwrap();
        {
            let _scope = file_mutation_scope();
            write(&path, Some(b"source updated"), false).unwrap();
            let first = file_recovery(&path).unwrap().unwrap();
            let _nested = file_mutation_scope();
            write(&path, Some(b"source and mcp updated"), false).unwrap();
            let final_receipt = file_recovery(&path).unwrap().unwrap();
            assert_eq!(first.receipt_id, final_receipt.receipt_id);
            assert_eq!(fs::read(rolling_backup_path(&path)).unwrap(), b"original");
            restore_file_recovery(&path, &final_receipt.receipt_id).unwrap();
            assert_eq!(fs::read(&path).unwrap(), b"original");
        }
        write(&path, Some(b"next operation"), false).unwrap();
        assert_eq!(fs::read(rolling_backup_path(&path)).unwrap(), b"original");
    }

    #[test]
    fn operation_returning_to_original_state_remains_recoverable() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(&path, b"original").unwrap();
        let _scope = file_mutation_scope();
        write(&path, Some(b"intermediate"), false).unwrap();
        write(&path, Some(b"original"), false).unwrap();
        let receipt = file_recovery(&path).unwrap().unwrap();
        assert!(receipt.can_restore);
        restore_file_recovery(&path, &receipt.receipt_id).unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"original");
    }

    #[test]
    fn backup_is_exact_single_rolling_preimage_and_undo_survives_reread() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(&path, b"# keep comments\r\nmodel = 'old'\r\n").unwrap();
        let old = fs::read(&path).unwrap();
        write(&path, Some(b"new"), false).unwrap();
        assert_eq!(fs::read(rolling_backup_path(&path)).unwrap(), old);
        let receipt = file_recovery(&path).unwrap().unwrap();
        assert!(receipt.had_file && receipt.can_restore);
        restore_file_recovery(&path, &receipt.receipt_id).unwrap();
        assert_eq!(fs::read(&path).unwrap(), old);
        assert!(file_recovery(&path).unwrap().is_none());
    }

    #[test]
    fn first_creation_undo_removes_file_without_fabricating_empty_backup() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("auth.json");
        write(&path, Some(b"{}"), true).unwrap();
        let receipt = file_recovery(&path).unwrap().unwrap();
        assert!(!receipt.had_file);
        assert!(!rolling_backup_path(&path).exists());
        restore_file_recovery(&path, &receipt.receipt_id).unwrap();
        assert!(!path.exists());
    }

    #[test]
    fn no_op_does_not_rotate_useful_backup_or_receipt() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        fs::write(&path, b"old").unwrap();
        write(&path, Some(b"new"), false).unwrap();
        let marker = fs::read(record_path(&path)).unwrap();
        write(&path, Some(b"new"), false).unwrap();
        assert_eq!(fs::read(record_path(&path)).unwrap(), marker);
        assert_eq!(fs::read(rolling_backup_path(&path)).unwrap(), b"old");
    }

    #[test]
    fn backup_failure_preserves_primary() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        fs::write(&path, b"old").unwrap();
        fs::create_dir(rolling_backup_path(&path)).unwrap();
        assert!(write(&path, Some(b"new"), false).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"old");
        assert!(!record_path(&path).exists());
    }

    #[test]
    fn failed_primary_write_restores_previous_recovery_history() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        fs::write(&path, b"original").unwrap();
        write(&path, Some(b"first"), false).unwrap();
        let marker = fs::read(record_path(&path)).unwrap();
        let result = write_with(&path, Some(b"second"), |_, _| Err(failure("injected")));
        assert!(result.is_err());
        assert_eq!(fs::read(&path).unwrap(), b"first");
        assert_eq!(fs::read(rolling_backup_path(&path)).unwrap(), b"original");
        assert_eq!(fs::read(record_path(&path)).unwrap(), marker);
    }

    #[test]
    fn late_write_failure_compensates_but_external_edit_is_not_overwritten() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        fs::write(&path, b"original").unwrap();
        let result = write_with(&path, Some(b"new"), |path, bytes| {
            replace(path, bytes, false)?;
            Err(failure("late_failure"))
        });
        assert!(result.is_err());
        assert_eq!(fs::read(&path).unwrap(), b"original");
        let result = write_with(&path, Some(b"new"), |path, _| {
            fs::write(path, b"external").unwrap();
            Ok(())
        });
        assert!(result.is_err());
        assert_eq!(fs::read(&path).unwrap(), b"external");
        assert_eq!(fs::read(rolling_backup_path(&path)).unwrap(), b"original");
    }

    #[test]
    fn undo_rejects_external_edit_corrupt_backup_and_stale_receipt() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("auth.json");
        fs::write(&path, b"old").unwrap();
        write(&path, Some(b"new"), true).unwrap();
        let receipt = file_recovery(&path).unwrap().unwrap();
        assert!(restore_file_recovery(&path, &uuid::Uuid::new_v4().to_string()).is_err());
        fs::write(&path, b"external").unwrap();
        assert!(!file_recovery(&path).unwrap().unwrap().can_restore);
        assert!(restore_file_recovery(&path, &receipt.receipt_id).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"external");
        fs::write(&path, b"new").unwrap();
        fs::write(rolling_backup_path(&path), b"tampered").unwrap();
        assert!(restore_file_recovery(&path, &receipt.receipt_id).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"new");
    }

    #[test]
    fn deletion_is_recoverable() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        fs::write(&path, b"old").unwrap();
        write(&path, None, false).unwrap();
        assert!(!path.exists());
        let receipt = file_recovery(&path).unwrap().unwrap();
        restore_file_recovery(&path, &receipt.receipt_id).unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"old");
    }

    #[test]
    fn copied_receipt_does_not_authorize_a_different_file() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("first.json");
        let target = dir.path().join("second.json");
        fs::write(&source, b"old").unwrap();
        write(&source, Some(b"new"), true).unwrap();
        let receipt = file_recovery(&source).unwrap().unwrap();
        fs::write(&target, b"new").unwrap();
        fs::copy(rolling_backup_path(&source), rolling_backup_path(&target)).unwrap();
        fs::copy(record_path(&source), record_path(&target)).unwrap();
        assert!(restore_file_recovery(&target, &receipt.receipt_id).is_err());
        assert_eq!(fs::read(&target).unwrap(), b"new");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn secrets_are_private_and_symlink_backups_are_rejected() {
        use std::os::unix::fs::{symlink, PermissionsExt};
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("auth.json");
        fs::write(&path, b"old").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
        write(&path, Some(b"new"), true).unwrap();
        for file in [&path, &rolling_backup_path(&path), &record_path(&path)] {
            assert_eq!(
                fs::metadata(file).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        fs::remove_file(rolling_backup_path(&path)).unwrap();
        let unrelated = dir.path().join("unrelated");
        fs::write(&unrelated, b"do not touch").unwrap();
        symlink(&unrelated, rolling_backup_path(&path)).unwrap();
        assert!(write(&path, Some(b"next"), true).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"new");
        assert_eq!(fs::read(unrelated).unwrap(), b"do not touch");
    }
}
