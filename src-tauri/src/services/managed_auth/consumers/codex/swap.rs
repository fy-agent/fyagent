//! Narrow Codex `auth.json` swapper with revision CAS and rollback.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use crate::services::managed_auth::ManagedAuthCoreError;

use super::auth_document::{
    classify_auth_bytes, CodexChatGptAuthDocument, CodexNativeAuthState, MAX_AUTH_JSON_BYTES,
};
use super::observation::revision_for_bytes;

/// External auth writes are not proven to hot-reload a live Codex process.
pub(crate) const CODEX_EXTERNAL_WRITE_HOT_RELOAD_PROVEN: bool = false;

fn auth_json_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CodexAuthSwapReceipt {
    pub revision: String,
    pub account_id: String,
    pub changed: bool,
    pub pending_restart: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CodexAuthSwapError {
    Stale,
    Io,
    Invalid,
    ExternalChange,
    IdentityMismatch,
}

impl From<CodexAuthSwapError> for ManagedAuthCoreError {
    fn from(error: CodexAuthSwapError) -> Self {
        match error {
            CodexAuthSwapError::Stale | CodexAuthSwapError::ExternalChange => Self::Stale,
            CodexAuthSwapError::Io => Self::Io,
            CodexAuthSwapError::Invalid | CodexAuthSwapError::IdentityMismatch => Self::InvalidData,
        }
    }
}

pub(crate) fn swap_codex_chatgpt_auth(
    auth_path: &Path,
    expected_auth_revision: Option<&str>,
    target: &CodexChatGptAuthDocument,
) -> Result<CodexAuthSwapReceipt, CodexAuthSwapError> {
    let _guard = auth_json_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    swap_codex_chatgpt_auth_locked(auth_path, expected_auth_revision, target)
}

fn swap_codex_chatgpt_auth_locked(
    auth_path: &Path,
    expected_auth_revision: Option<&str>,
    target: &CodexChatGptAuthDocument,
) -> Result<CodexAuthSwapReceipt, CodexAuthSwapError> {
    let target_account = target
        .account_id()
        .ok_or(CodexAuthSwapError::IdentityMismatch)?
        .to_string();
    let serialized = target
        .serialize_bytes()
        .map_err(|_| CodexAuthSwapError::Invalid)?;
    let current = read_auth_bytes(auth_path)?;

    // Revision CAS:
    // - Some(rev) requires the live file to exist with that exact revision
    // - None requires the live file to still be missing
    match (expected_auth_revision, current.as_ref()) {
        (Some(expected), Some(bytes)) if revision_for_bytes(bytes) == expected => {}
        (None, None) => {}
        _ => return Err(CodexAuthSwapError::Stale),
    }

    if let Some(bytes) = current.as_ref() {
        if bytes == &serialized {
            return Ok(CodexAuthSwapReceipt {
                revision: revision_for_bytes(bytes),
                account_id: target_account,
                changed: false,
                pending_restart: false,
            });
        }
    }

    let preimage = current.clone();
    let written_revision = revision_for_bytes(&serialized);
    write_auth_json_0600(auth_path, &serialized).map_err(|_| CodexAuthSwapError::Io)?;
    match read_auth_bytes(auth_path) {
        Ok(Some(readback)) if readback == serialized => {
            let revision = revision_for_bytes(&readback);
            match classify_auth_bytes(&readback, revision.clone()) {
                CodexNativeAuthState::ChatGptKnown { account_id, .. }
                    if account_id == target_account =>
                {
                    Ok(CodexAuthSwapReceipt {
                        revision,
                        account_id,
                        changed: true,
                        pending_restart: !CODEX_EXTERNAL_WRITE_HOT_RELOAD_PROVEN,
                    })
                }
                _ => {
                    try_restore_preimage(auth_path, preimage.as_deref(), &written_revision)?;
                    Err(CodexAuthSwapError::IdentityMismatch)
                }
            }
        }
        Ok(Some(readback)) => {
            let live_revision = revision_for_bytes(&readback);
            if live_revision != written_revision {
                return Err(CodexAuthSwapError::ExternalChange);
            }
            try_restore_preimage(auth_path, preimage.as_deref(), &written_revision)?;
            Err(CodexAuthSwapError::Io)
        }
        Ok(None) | Err(_) => {
            try_restore_preimage(auth_path, preimage.as_deref(), &written_revision)?;
            Err(CodexAuthSwapError::Io)
        }
    }
}

fn try_restore_preimage(
    path: &Path,
    preimage: Option<&[u8]>,
    written_revision: &str,
) -> Result<(), CodexAuthSwapError> {
    let live = read_auth_bytes(path).unwrap_or(None);
    if let Some(bytes) = live.as_ref() {
        if revision_for_bytes(bytes) != written_revision {
            return Err(CodexAuthSwapError::ExternalChange);
        }
    }
    match preimage {
        Some(bytes) => write_auth_json_0600(path, bytes).map_err(|_| CodexAuthSwapError::Io),
        None => {
            let _ = fs::remove_file(path);
            Ok(())
        }
    }
}

fn write_auth_json_0600(path: &Path, bytes: &[u8]) -> Result<(), ManagedAuthCoreError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|_| ManagedAuthCoreError::Io)?;
    }
    crate::config::atomic_write(path, bytes).map_err(|_| ManagedAuthCoreError::Io)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))
            .map_err(|_| ManagedAuthCoreError::Io)?;
    }
    Ok(())
}

fn read_auth_bytes(path: &Path) -> Result<Option<Vec<u8>>, CodexAuthSwapError> {
    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(CodexAuthSwapError::Io),
    };
    let mut bytes = Vec::new();
    let limit = u64::try_from(MAX_AUTH_JSON_BYTES.saturating_add(1)).expect("auth limit");
    file.take(limit)
        .read_to_end(&mut bytes)
        .map_err(|_| CodexAuthSwapError::Io)?;
    if bytes.len() > MAX_AUTH_JSON_BYTES {
        return Err(CodexAuthSwapError::Invalid);
    }
    Ok(Some(bytes))
}

pub(crate) fn auth_path_in(codex_home: &Path) -> PathBuf {
    codex_home.join("auth.json")
}

/// Capture exact live `auth.json` bytes under the writer lock (for rollback).
pub(crate) fn capture_auth_preimage(
    auth_path: &Path,
) -> Result<Option<Vec<u8>>, CodexAuthSwapError> {
    let _guard = auth_json_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    read_auth_bytes(auth_path)
}

/// Restore exact preimage bytes only when live revision still matches the
/// revision written by this coordinator. External changes stop the overwrite.
pub(crate) fn restore_codex_auth_preimage(
    auth_path: &Path,
    written_revision: &str,
    preimage: Option<&[u8]>,
) -> Result<(), CodexAuthSwapError> {
    let _guard = auth_json_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    try_restore_preimage(auth_path, preimage, written_revision)
}

#[cfg(test)]
pub(crate) fn read_auth_bytes_for_test(path: &Path) -> Option<Vec<u8>> {
    let _guard = auth_json_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    read_auth_bytes(path).ok().flatten()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::managed_auth::consumers::codex::auth_document::CodexChatGptAuthDocument;
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    use serde_json::json;
    use tempfile::tempdir;

    fn jwt(account: &str) -> String {
        let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"none"}"#);
        let payload = URL_SAFE_NO_PAD.encode(
            json!({
                "chatgpt_account_id": account,
                "email": "user@example.com",
                "organizations": [{"id": "org-1"}]
            })
            .to_string()
            .as_bytes(),
        );
        format!("{header}.{payload}.sig")
    }

    fn doc(account: &str) -> CodexChatGptAuthDocument {
        CodexChatGptAuthDocument::from_tokens(
            &jwt(account),
            &jwt(account),
            &format!("refresh-{account}"),
            Some(account),
            Some(1_700_000_000),
        )
        .unwrap()
    }

    #[test]
    fn swap_missing_file_writes_chatgpt_auth() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("auth.json");
        let receipt = swap_codex_chatgpt_auth(&path, None, &doc("acct-a")).unwrap();
        assert!(receipt.changed);
        assert_eq!(receipt.account_id, "acct-a");
        let bytes = read_auth_bytes_for_test(&path).expect("auth written");
        assert!(String::from_utf8_lossy(&bytes).contains("acct-a"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600);
        }
        assert_eq!(
            swap_codex_chatgpt_auth(&path, None, &doc("acct-b")),
            Err(CodexAuthSwapError::Stale)
        );
        let next = swap_codex_chatgpt_auth(&path, Some(&receipt.revision), &doc("acct-b")).unwrap();
        assert_eq!(next.account_id, "acct-b");
    }
}
