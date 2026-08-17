use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::atomic::{write_private_file, read_limited};
use super::schema::{
    JOURNAL_MAX_BYTES, JournalEnvelope, JournalOperationKind, SCHEMA_VERSION,
};

pub fn journal_dir(root: &Path) -> PathBuf {
    root.join("journal")
}

pub fn journal_path(root: &Path, operation_id: &str) -> PathBuf {
    // Filenames use only the server operation id. secretRef is never used.
    journal_dir(root).join(format!("{operation_id}.json"))
}

pub fn temp_journal_path(root: &Path, operation_id: &str) -> PathBuf {
    let nonce = uuid::Uuid::new_v4().simple().to_string();
    journal_dir(root).join(format!(".tmp-journal-{operation_id}-{nonce}.json"))
}

pub fn write_journal(root: &Path, envelope: &JournalEnvelope) -> io::Result<PathBuf> {
    if !envelope.operation_id.starts_with("sop_") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "journal filename must be sop_*",
        ));
    }
    if envelope.schema_version != SCHEMA_VERSION {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "schemaVersion must be 1"));
    }
    let _ = JournalOperationKind::ALL
        .iter()
        .find(|kind| **kind == envelope.operation_kind)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "unknown journal kind"))?;
    let dest = journal_path(root, &envelope.operation_id);
    let tmp = temp_journal_path(root, &envelope.operation_id);
    let bytes = serde_json::to_vec(envelope)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    if bytes.len() > JOURNAL_MAX_BYTES {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "journal exceeds 64 KiB"));
    }
    write_private_file(&tmp, &bytes)?;
    fs::rename(&tmp, &dest)?;
    Ok(dest)
}

pub fn read_journal(path: &Path) -> io::Result<JournalEnvelope> {
    let bytes = read_limited(path, JOURNAL_MAX_BYTES)?;
    serde_json::from_slice(&bytes).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

pub fn list_journals(root: &Path) -> io::Result<Vec<JournalEnvelope>> {
    let dir = journal_dir(root);
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut rows = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.starts_with("sop_") || !name.ends_with(".json") {
            continue;
        }
        rows.push(read_journal(&entry.path())?);
    }
    Ok(rows)
}

pub fn kind_totality(kind: JournalOperationKind) -> &'static str {
    match kind {
        JournalOperationKind::CaptureCandidate => "captureCandidate",
        JournalOperationKind::MigrateLegacy => "migrateLegacy",
        JournalOperationKind::RotateCandidate => "rotateCandidate",
        JournalOperationKind::ActivateCandidate => "activateCandidate",
        JournalOperationKind::DiscardCandidate => "discardCandidate",
        JournalOperationKind::DeleteSecret => "deleteSecret",
        JournalOperationKind::DetachProviderOwner => "detachProviderOwner",
        JournalOperationKind::StagedImport => "stagedImport",
    }
}
