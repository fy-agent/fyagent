use super::schema::{JournalEnvelope, JournalOperationKind, RecoveryKind};

/// Crash recovery maps unfinished journal rows onto exactly four arms.
/// There is no generic/fifth recovery kind.
pub fn recovery_kind_for_journal(row: &JournalEnvelope) -> Option<RecoveryKind> {
    match row.operation_kind {
        JournalOperationKind::ActivateCandidate => Some(RecoveryKind::ActivationCleanup),
        JournalOperationKind::CaptureCandidate
        | JournalOperationKind::MigrateLegacy
        | JournalOperationKind::RotateCandidate => Some(RecoveryKind::CaptureCompensation),
        JournalOperationKind::DeleteSecret => Some(RecoveryKind::DeleteFinalization),
        JournalOperationKind::DetachProviderOwner => Some(RecoveryKind::OwnerDetachFinalization),
        JournalOperationKind::DiscardCandidate | JournalOperationKind::StagedImport => None,
    }
}

pub fn recovery_kind_totality(kind: RecoveryKind) -> &'static str {
    match kind {
        RecoveryKind::ActivationCleanup => "activationCleanup",
        RecoveryKind::CaptureCompensation => "captureCompensation",
        RecoveryKind::DeleteFinalization => "deleteFinalization",
        RecoveryKind::OwnerDetachFinalization => "ownerDetachFinalization",
    }
}

pub fn recover_open_journals(rows: &[JournalEnvelope]) -> Vec<(String, RecoveryKind)> {
    rows.iter()
        .filter_map(|row| {
            recovery_kind_for_journal(row).map(|kind| (row.operation_id.clone(), kind))
        })
        .collect()
}
