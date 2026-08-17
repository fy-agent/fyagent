use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::atomic::{read_limited, write_private_file};
use super::schema::{
    ActivateCandidatePhase, ActivationCleanupKind, ActivationCleanupLink, ActivationOldRecordDeleteCheckpoint,
    ActivationOldRecordDurableCheckpoint, ActivationTerminalOutcome, CaptureLikePhase,
    CaptureTerminalOutcome, CandidateDiscardDeleteCheckpoint, DeleteAppliedCas, DeleteAppliedRole,
    DeleteDisposition, DeleteFinalizationKind, DeleteFinalizationLink, DeleteSecretPhase,
    DetachProviderOwnerPhase, DiscardCandidatePhase, DiscardSlot, JOURNAL_MAX_BYTES, JournalEnvelope,
    JournalError, JournalOperationKind, OwnerDetachFinalizationKind, OwnerDetachFinalizationLink,
    SCHEMA_VERSION, SlotOccupation, StagedImportPhase, StagedImportResumePhase, StagedSourceSetCas,
    TerminalDisposition, sha256_hex, valid_hex_n, valid_prefixed_id, valid_rfc3339_utc_millis,
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
    if !envelope.operation_id().starts_with("sop_") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "journal filename must be sop_*",
        ));
    }
    if envelope.schema_version() != SCHEMA_VERSION {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "schemaVersion must be 1"));
    }
    let _ = JournalOperationKind::ALL
        .iter()
        .find(|kind| **kind == envelope.operation_kind())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "unknown journal kind"))?;
    validate_envelope(envelope).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, format!("{e:?}")))?;
    let dest = journal_path(root, envelope.operation_id());
    let tmp = temp_journal_path(root, envelope.operation_id());
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
    let envelope: JournalEnvelope =
        serde_json::from_slice(&bytes).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    validate_envelope(&envelope)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("{e:?}")))?;
    Ok(envelope)
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

pub fn validate_envelope(envelope: &JournalEnvelope) -> Result<(), JournalError> {
    if envelope.schema_version() != SCHEMA_VERSION || envelope.attempt() < 1 {
        return Err(JournalError::InvalidInput);
    }
    if !valid_prefixed_id(envelope.operation_id(), "sop_") {
        return Err(JournalError::InvalidInput);
    }
    if !valid_prefixed_id(envelope.device_instance_id(), "dev_") {
        return Err(JournalError::InvalidInput);
    }
    match envelope {
        JournalEnvelope::CaptureCandidate { phase, .. }
        | JournalEnvelope::MigrateLegacy { phase, .. }
        | JournalEnvelope::RotateCandidate { phase, .. } => validate_capture_like(phase),
        JournalEnvelope::ActivateCandidate { phase, .. } => validate_activate(phase),
        JournalEnvelope::DiscardCandidate {
            terminal_disposition,
            record_delete_slot,
            record_missing_readback_slot,
            phase,
            ..
        } => validate_discard(
            *terminal_disposition,
            *record_delete_slot,
            *record_missing_readback_slot,
            phase,
        ),
        JournalEnvelope::DeleteSecret { phase, .. } => validate_delete(phase),
        JournalEnvelope::DetachProviderOwner { phase, .. } => validate_detach(phase),
        JournalEnvelope::StagedImport { phase, .. } => validate_staged(phase),
    }
}

fn validate_capture_like(phase: &CaptureLikePhase) -> Result<(), JournalError> {
    match phase {
        CaptureLikePhase::BackendApplied { verify_receipt_id } => {
            if valid_hex_n(verify_receipt_id, 32) {
                Ok(())
            } else {
                Err(JournalError::InvalidInput)
            }
        }
        CaptureLikePhase::RecoveryRequired { last_error_code, recovery } => {
            if last_error_code.is_empty() || !valid_prefixed_id(&recovery.recovery_id, "src_") {
                Err(JournalError::InvalidInput)
            } else {
                Ok(())
            }
        }
        _ => Ok(()),
    }
}

fn validate_activate(phase: &ActivateCandidatePhase) -> Result<(), JournalError> {
    match phase {
        super::schema::ActivateCandidatePhase::OldRecordDeleteApplied { checkpoint } => {
            ActivationOldRecordDeleteCheckpoint::checked(
                checkpoint.delete_disposition,
                checkpoint.backend_completed_at.clone(),
                checkpoint.delete_applied_cas.clone(),
            )
            .map(|_| ())
        }
        super::schema::ActivateCandidatePhase::RecoveryRequired {
            last_error_code,
            checkpoint,
            recovery,
        } => {
            if last_error_code.is_empty() || !valid_prefixed_id(&recovery.recovery_id, "src_") {
                return Err(JournalError::InvalidInput);
            }
            match checkpoint {
                ActivationOldRecordDurableCheckpoint::None => Ok(()),
                ActivationOldRecordDurableCheckpoint::OldRecordDeleteApplied {
                    delete_disposition,
                    backend_completed_at,
                    delete_applied_cas,
                } => ActivationOldRecordDeleteCheckpoint::checked(
                    *delete_disposition,
                    backend_completed_at.clone(),
                    delete_applied_cas.clone(),
                )
                .map(|_| ()),
            }
        }
        _ => Ok(()),
    }
}

fn validate_discard(
    terminal_disposition: TerminalDisposition,
    record_delete_slot: SlotOccupation,
    record_missing_readback_slot: SlotOccupation,
    phase: &DiscardCandidatePhase,
) -> Result<(), JournalError> {
    match phase {
        DiscardCandidatePhase::Intent => {
            if record_delete_slot != SlotOccupation::Unused
                || record_missing_readback_slot != SlotOccupation::Unused
            {
                return Err(JournalError::InvalidTransition);
            }
        }
        DiscardCandidatePhase::BackendApplied { checkpoint } => {
            if record_delete_slot != SlotOccupation::Consumed
                || record_missing_readback_slot != SlotOccupation::Unused
            {
                return Err(JournalError::InvalidTransition);
            }
            CandidateDiscardDeleteCheckpoint::checked(
                checkpoint.delete_disposition,
                checkpoint.backend_completed_at.clone(),
                checkpoint.delete_applied_cas.clone(),
            )?;
        }
        DiscardCandidatePhase::MissingReadbackVerified {
            checkpoint,
            missing_checked_at,
        } => {
            if record_delete_slot != SlotOccupation::Consumed
                || record_missing_readback_slot != SlotOccupation::Consumed
            {
                return Err(JournalError::InvalidTransition);
            }
            if !valid_rfc3339_utc_millis(missing_checked_at) {
                return Err(JournalError::InvalidInput);
            }
            CandidateDiscardDeleteCheckpoint::checked(
                checkpoint.delete_disposition,
                checkpoint.backend_completed_at.clone(),
                checkpoint.delete_applied_cas.clone(),
            )?;
        }
        DiscardCandidatePhase::RecoveryRequired { .. } => {}
        DiscardCandidatePhase::Terminal {
            terminal_disposition: phase_disposition,
        } => {
            if *phase_disposition != terminal_disposition {
                return Err(JournalError::InvalidTransition);
            }
            if record_delete_slot != SlotOccupation::Consumed
                || record_missing_readback_slot != SlotOccupation::Consumed
            {
                return Err(JournalError::InvalidTransition);
            }
        }
    }
    Ok(())
}

fn validate_delete(phase: &DeleteSecretPhase) -> Result<(), JournalError> {
    match phase {
        DeleteSecretPhase::BackendApplied {
            backend_completed_at,
            ..
        } => {
            if valid_rfc3339_utc_millis(backend_completed_at) {
                Ok(())
            } else {
                Err(JournalError::InvalidInput)
            }
        }
        DeleteSecretPhase::MissingReadbackVerified { missing_checked_at }
        | DeleteSecretPhase::StateFinalized {
            revoked_at: missing_checked_at,
            ..
        }
        | DeleteSecretPhase::Terminal {
            revoked_at: missing_checked_at,
            ..
        } => {
            if valid_rfc3339_utc_millis(missing_checked_at) {
                Ok(())
            } else {
                Err(JournalError::InvalidInput)
            }
        }
        _ => Ok(()),
    }
}

fn validate_detach(phase: &DetachProviderOwnerPhase) -> Result<(), JournalError> {
    match phase {
        DetachProviderOwnerPhase::ProviderDetachCommitted {
            provider_detach_commit_id,
        }
        | DetachProviderOwnerPhase::LocalOwnerCasApplied {
            provider_detach_commit_id,
        }
        | DetachProviderOwnerPhase::RecoveryRequired {
            provider_detach_commit_id,
            ..
        }
        | DetachProviderOwnerPhase::Terminal {
            provider_detach_commit_id,
        } => {
            if valid_hex_n(provider_detach_commit_id, 32) {
                Ok(())
            } else {
                Err(JournalError::InvalidInput)
            }
        }
        DetachProviderOwnerPhase::Intent => Ok(()),
    }
}

fn validate_staged(phase: &StagedImportPhase) -> Result<(), JournalError> {
    match phase {
        StagedImportPhase::SourcesScrubbed {
            staged_source_set_cas_after_scrub,
        } => StagedSourceSetCas::after_scrub(
            staged_source_set_cas_after_scrub.revision,
            staged_source_set_cas_after_scrub.digest.clone(),
            staged_source_set_cas_after_scrub.count,
        )
        .map(|_| ()),
        StagedImportPhase::RecoveryRequired { resume_phase, .. } => {
            validate_resume_phase(resume_phase)
        }
        _ => Ok(()),
    }
}

fn validate_resume_phase(phase: &StagedImportResumePhase) -> Result<(), JournalError> {
    match phase {
        StagedImportResumePhase::Intent {} => Ok(()),
        StagedImportResumePhase::SourcesScrubbed {
            staged_source_set_cas_after_scrub,
        }
        | StagedImportResumePhase::CutoverCommitted {
            staged_source_set_cas_after_scrub,
            ..
        }
        | StagedImportResumePhase::LiveOwnerMinted {
            staged_source_set_cas_after_scrub,
            ..
        }
        | StagedImportResumePhase::LocalBindingFinalized {
            staged_source_set_cas_after_scrub,
            ..
        } => StagedSourceSetCas::after_scrub(
            staged_source_set_cas_after_scrub.revision,
            staged_source_set_cas_after_scrub.digest.clone(),
            staged_source_set_cas_after_scrub.count,
        )
        .map(|_| ()),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DeleteAppliedCasPreimage<'a> {
    operation_id: &'a str,
    kind: JournalOperationKind,
    role: DeleteAppliedRole,
    disposition: DeleteDisposition,
    completed_at: &'a str,
    expected_revision: u64,
}

pub fn mint_delete_applied_cas(
    operation_id: &str,
    kind: JournalOperationKind,
    role: DeleteAppliedRole,
    disposition: DeleteDisposition,
    completed_at: &str,
    expected_revision: u64,
) -> Result<DeleteAppliedCas, JournalError> {
    if !valid_prefixed_id(operation_id, "sop_")
        || !valid_rfc3339_utc_millis(completed_at)
        || expected_revision < 1
    {
        return Err(JournalError::InvalidInput);
    }
    let preimage = DeleteAppliedCasPreimage {
        operation_id,
        kind,
        role,
        disposition,
        completed_at,
        expected_revision,
    };
    let bytes = serde_json::to_vec(&preimage).map_err(|_| JournalError::InvalidInput)?;
    DeleteAppliedCas::checked(expected_revision, sha256_hex(&bytes))
}

fn require_meta(
    operation_id: String,
    device_instance_id: String,
    created_at: String,
    attempt: u32,
) -> Result<(String, String, String, u32), JournalError> {
    if !valid_prefixed_id(&operation_id, "sop_")
        || !valid_prefixed_id(&device_instance_id, "dev_")
        || !valid_rfc3339_utc_millis(&created_at)
        || attempt < 1
    {
        return Err(JournalError::InvalidInput);
    }
    Ok((operation_id, device_instance_id, created_at, attempt))
}

impl JournalEnvelope {
    pub fn capture_like_intent(
        kind: JournalOperationKind,
        operation_id: String,
        device_instance_id: String,
        created_at: String,
    ) -> Result<Self, JournalError> {
        let (operation_id, device_instance_id, created_at, attempt) =
            require_meta(operation_id, device_instance_id, created_at, 1)?;
        let phase = CaptureLikePhase::Intent;
        match kind {
            JournalOperationKind::CaptureCandidate => Ok(Self::CaptureCandidate {
                schema_version: SCHEMA_VERSION,
                operation_id,
                device_instance_id,
                created_at: created_at.clone(),
                updated_at: created_at,
                attempt,
                phase,
            }),
            JournalOperationKind::MigrateLegacy => Ok(Self::MigrateLegacy {
                schema_version: SCHEMA_VERSION,
                operation_id,
                device_instance_id,
                created_at: created_at.clone(),
                updated_at: created_at,
                attempt,
                phase,
            }),
            JournalOperationKind::RotateCandidate => Ok(Self::RotateCandidate {
                schema_version: SCHEMA_VERSION,
                operation_id,
                device_instance_id,
                created_at: created_at.clone(),
                updated_at: created_at,
                attempt,
                phase,
            }),
            _ => Err(JournalError::KindPhaseMismatch),
        }
    }

    pub fn discard_intent(
        operation_id: String,
        device_instance_id: String,
        created_at: String,
        terminal_disposition: TerminalDisposition,
    ) -> Result<Self, JournalError> {
        let (operation_id, device_instance_id, created_at, attempt) =
            require_meta(operation_id, device_instance_id, created_at, 1)?;
        Ok(Self::DiscardCandidate {
            schema_version: SCHEMA_VERSION,
            operation_id,
            device_instance_id,
            created_at: created_at.clone(),
            updated_at: created_at,
            attempt,
            terminal_disposition,
            record_delete_slot: SlotOccupation::Unused,
            record_missing_readback_slot: SlotOccupation::Unused,
            phase: DiscardCandidatePhase::Intent,
        })
    }

    pub fn activate_intent(
        operation_id: String,
        device_instance_id: String,
        created_at: String,
    ) -> Result<Self, JournalError> {
        let (operation_id, device_instance_id, created_at, attempt) =
            require_meta(operation_id, device_instance_id, created_at, 1)?;
        Ok(Self::ActivateCandidate {
            schema_version: SCHEMA_VERSION,
            operation_id,
            device_instance_id,
            created_at: created_at.clone(),
            updated_at: created_at,
            attempt,
            phase: super::schema::ActivateCandidatePhase::Intent,
        })
    }

    pub fn activate_old_record_delete_applied(
        operation_id: String,
        device_instance_id: String,
        created_at: String,
        checkpoint: ActivationOldRecordDeleteCheckpoint,
    ) -> Result<Self, JournalError> {
        let mut row = Self::activate_intent(operation_id, device_instance_id, created_at)?;
        row.apply_activation_old_record_delete(checkpoint)?;
        Ok(row)
    }

    pub fn delete_secret_intent(
        operation_id: String,
        device_instance_id: String,
        created_at: String,
    ) -> Result<Self, JournalError> {
        let (operation_id, device_instance_id, created_at, attempt) =
            require_meta(operation_id, device_instance_id, created_at, 1)?;
        Ok(Self::DeleteSecret {
            schema_version: SCHEMA_VERSION,
            operation_id,
            device_instance_id,
            created_at: created_at.clone(),
            updated_at: created_at,
            attempt,
            phase: DeleteSecretPhase::Intent,
        })
    }

    pub fn detach_intent(
        operation_id: String,
        device_instance_id: String,
        created_at: String,
    ) -> Result<Self, JournalError> {
        let (operation_id, device_instance_id, created_at, attempt) =
            require_meta(operation_id, device_instance_id, created_at, 1)?;
        Ok(Self::DetachProviderOwner {
            schema_version: SCHEMA_VERSION,
            operation_id,
            device_instance_id,
            created_at: created_at.clone(),
            updated_at: created_at,
            attempt,
            phase: DetachProviderOwnerPhase::Intent,
        })
    }

    pub fn staged_import_intent(
        operation_id: String,
        device_instance_id: String,
        created_at: String,
    ) -> Result<Self, JournalError> {
        let (operation_id, device_instance_id, created_at, attempt) =
            require_meta(operation_id, device_instance_id, created_at, 1)?;
        Ok(Self::StagedImport {
            schema_version: SCHEMA_VERSION,
            operation_id,
            device_instance_id,
            created_at: created_at.clone(),
            updated_at: created_at,
            attempt,
            phase: StagedImportPhase::Intent,
        })
    }

    pub fn consume_discard_slot(
        &mut self,
        slot: DiscardSlot,
        checkpoint: Option<CandidateDiscardDeleteCheckpoint>,
        missing_checked_at: Option<String>,
        now: String,
    ) -> Result<(), JournalError> {
        let JournalEnvelope::DiscardCandidate {
            updated_at,
            record_delete_slot,
            record_missing_readback_slot,
            phase,
            ..
        } = self
        else {
            return Err(JournalError::KindPhaseMismatch);
        };
        if matches!(phase, DiscardCandidatePhase::Terminal { .. }) {
            return Err(JournalError::AlreadyTerminal);
        }
        match slot {
            DiscardSlot::RecordDelete => {
                if *record_delete_slot == SlotOccupation::Consumed {
                    return Err(JournalError::SlotReuse);
                }
                if *record_missing_readback_slot == SlotOccupation::Consumed {
                    return Err(JournalError::SlotSwap);
                }
                if !matches!(phase, DiscardCandidatePhase::Intent) {
                    return Err(JournalError::InvalidTransition);
                }
                let checkpoint = checkpoint.ok_or(JournalError::MissingCheckpoint)?;
                CandidateDiscardDeleteCheckpoint::checked(
                    checkpoint.delete_disposition,
                    checkpoint.backend_completed_at.clone(),
                    checkpoint.delete_applied_cas.clone(),
                )?;
                *record_delete_slot = SlotOccupation::Consumed;
                *phase = DiscardCandidatePhase::backend_applied(checkpoint);
                *updated_at = now;
                Ok(())
            }
            DiscardSlot::RecordMissingReadback => {
                if *record_delete_slot == SlotOccupation::Unused {
                    return if checkpoint.is_none() {
                        Err(JournalError::MissingCheckpoint)
                    } else if *record_missing_readback_slot == SlotOccupation::Unused
                        && matches!(phase, DiscardCandidatePhase::Intent)
                    {
                        Err(JournalError::SlotSwap)
                    } else {
                        Err(JournalError::MissingCheckpoint)
                    };
                }
                if *record_missing_readback_slot == SlotOccupation::Consumed {
                    return Err(JournalError::SlotReuse);
                }
                let DiscardCandidatePhase::BackendApplied {
                    checkpoint: applied,
                } = phase
                else {
                    return Err(JournalError::InvalidTransition);
                };
                let checked_at = missing_checked_at.ok_or(JournalError::InvalidInput)?;
                *phase = DiscardCandidatePhase::missing_readback_verified(
                    applied.clone(),
                    checked_at,
                )?;
                *record_missing_readback_slot = SlotOccupation::Consumed;
                *updated_at = now;
                Ok(())
            }
        }
    }

    pub fn finalize_discard_terminal(&mut self, now: String) -> Result<(), JournalError> {
        let JournalEnvelope::DiscardCandidate {
            updated_at,
            terminal_disposition,
            record_delete_slot,
            record_missing_readback_slot,
            phase,
            ..
        } = self
        else {
            return Err(JournalError::KindPhaseMismatch);
        };
        if matches!(phase, DiscardCandidatePhase::Terminal { .. }) {
            return Err(JournalError::AlreadyTerminal);
        }
        if *record_delete_slot != SlotOccupation::Consumed
            || *record_missing_readback_slot != SlotOccupation::Consumed
        {
            return Err(JournalError::InvalidTransition);
        }
        if !matches!(phase, DiscardCandidatePhase::MissingReadbackVerified { .. }) {
            return Err(JournalError::InvalidTransition);
        }
        *phase = DiscardCandidatePhase::Terminal {
            terminal_disposition: *terminal_disposition,
        };
        *updated_at = now;
        Ok(())
    }

    pub fn apply_activation_old_record_delete(
        &mut self,
        checkpoint: ActivationOldRecordDeleteCheckpoint,
    ) -> Result<(), JournalError> {
        let JournalEnvelope::ActivateCandidate {
            updated_at, phase, ..
        } = self
        else {
            return Err(JournalError::KindPhaseMismatch);
        };
        ActivationOldRecordDeleteCheckpoint::checked(
            checkpoint.delete_disposition,
            checkpoint.backend_completed_at.clone(),
            checkpoint.delete_applied_cas.clone(),
        )?;
        *phase = super::schema::ActivateCandidatePhase::old_record_delete_applied(checkpoint);
        *updated_at = phase_now(updated_at);
        Ok(())
    }

    pub fn activation_recovery_required(
        &mut self,
        last_error_code: String,
        recovery_id: String,
        recovery_cas: DeleteAppliedCas,
    ) -> Result<(), JournalError> {
        let JournalEnvelope::ActivateCandidate {
            updated_at, phase, ..
        } = self
        else {
            return Err(JournalError::KindPhaseMismatch);
        };
        let super::schema::ActivateCandidatePhase::OldRecordDeleteApplied { checkpoint } = phase
        else {
            return Err(JournalError::MissingCheckpoint);
        };
        *phase = super::schema::ActivateCandidatePhase::recovery_required_preserving(
            last_error_code,
            checkpoint,
            recovery_id,
            recovery_cas,
        )?;
        *updated_at = phase_now(updated_at);
        Ok(())
    }

    pub fn activation_applied_checkpoint(
        &self,
    ) -> Result<&ActivationOldRecordDeleteCheckpoint, JournalError> {
        match self {
            Self::ActivateCandidate {
                phase:
                    super::schema::ActivateCandidatePhase::OldRecordDeleteApplied { checkpoint },
                ..
            } => Ok(checkpoint),
            _ => Err(JournalError::MissingCheckpoint),
        }
    }

    pub fn activation_durable_checkpoint(
        &self,
    ) -> Result<&ActivationOldRecordDurableCheckpoint, JournalError> {
        match self {
            Self::ActivateCandidate {
                phase:
                    super::schema::ActivateCandidatePhase::RecoveryRequired { checkpoint, .. },
                ..
            } => Ok(checkpoint),
            _ => Err(JournalError::MissingCheckpoint),
        }
    }

    pub fn discard_checkpoint(&self) -> Result<&CandidateDiscardDeleteCheckpoint, JournalError> {
        match self {
            Self::DiscardCandidate { phase, .. } => {
                phase.delete_checkpoint().ok_or(JournalError::MissingCheckpoint)
            }
            _ => Err(JournalError::KindPhaseMismatch),
        }
    }

    pub fn advance_capture_like(
        &mut self,
        next: CaptureLikePhase,
        now: String,
    ) -> Result<(), JournalError> {
        validate_capture_like(&next)?;
        match self {
            Self::CaptureCandidate {
                updated_at, phase, ..
            }
            | Self::MigrateLegacy {
                updated_at, phase, ..
            }
            | Self::RotateCandidate {
                updated_at, phase, ..
            } => {
                *phase = next;
                *updated_at = now;
                Ok(())
            }
            _ => Err(JournalError::KindPhaseMismatch),
        }
    }

    pub fn advance_delete_secret(
        &mut self,
        next: DeleteSecretPhase,
        now: String,
    ) -> Result<(), JournalError> {
        validate_delete(&next)?;
        let Self::DeleteSecret {
            updated_at, phase, ..
        } = self
        else {
            return Err(JournalError::KindPhaseMismatch);
        };
        *phase = next;
        *updated_at = now;
        Ok(())
    }

    pub fn advance_detach(
        &mut self,
        next: DetachProviderOwnerPhase,
        now: String,
    ) -> Result<(), JournalError> {
        validate_detach(&next)?;
        let Self::DetachProviderOwner {
            updated_at, phase, ..
        } = self
        else {
            return Err(JournalError::KindPhaseMismatch);
        };
        *phase = next;
        *updated_at = now;
        Ok(())
    }

    pub fn advance_staged(
        &mut self,
        next: StagedImportPhase,
        now: String,
    ) -> Result<(), JournalError> {
        validate_staged(&next)?;
        let Self::StagedImport {
            updated_at, phase, ..
        } = self
        else {
            return Err(JournalError::KindPhaseMismatch);
        };
        *phase = next;
        *updated_at = now;
        Ok(())
    }
}

fn phase_now(previous: &str) -> String {
    previous.to_string()
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StagedImportResumePreimage {
    pub operation_id: String,
    pub phase: StagedImportResumePhase,
}

impl StagedImportResumePreimage {
    pub fn checked(
        operation_id: String,
        phase: StagedImportResumePhase,
    ) -> Result<Self, JournalError> {
        if !valid_prefixed_id(&operation_id, "sop_") {
            return Err(JournalError::InvalidInput);
        }
        validate_resume_phase(&phase)?;
        Ok(Self {
            operation_id,
            phase,
        })
    }

    pub fn encode(&self) -> Result<Vec<u8>, JournalError> {
        serde_json::to_vec(self).map_err(|_| JournalError::InvalidInput)
    }

    pub fn digest(&self) -> Result<String, JournalError> {
        Ok(sha256_hex(&self.encode()?))
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, JournalError> {
        let value: serde_json::Value =
            serde_json::from_slice(bytes).map_err(|_| JournalError::InvalidInput)?;
        reject_unknown_resume_value(&value)?;
        let decoded: Self =
            serde_json::from_value(value).map_err(|_| JournalError::InvalidInput)?;
        Self::checked(decoded.operation_id, decoded.phase)
    }
}

#[allow(dead_code)]
fn _keep_terminal_symbols() -> (CaptureTerminalOutcome, ActivationTerminalOutcome) {
    (
        CaptureTerminalOutcome::CandidateStaged,
        ActivationTerminalOutcome::Activated,
    )
}

#[allow(dead_code)]
fn _keep_recovery_kinds() -> (
    ActivationCleanupKind,
    DeleteFinalizationKind,
    OwnerDetachFinalizationKind,
) {
    (
        ActivationCleanupKind::ActivationCleanup,
        DeleteFinalizationKind::DeleteFinalization,
        OwnerDetachFinalizationKind::OwnerDetachFinalization,
    )
}

#[allow(dead_code)]
fn _keep_links(
    a: ActivationCleanupLink,
    d: DeleteFinalizationLink,
    o: OwnerDetachFinalizationLink,
) {
    let _ = (a, d, o);
}

fn reject_unknown_resume_value(value: &serde_json::Value) -> Result<(), JournalError> {
    let object = value.as_object().ok_or(JournalError::InvalidInput)?;
    if object.len() != 2 || !object.contains_key("operationId") || !object.contains_key("phase") {
        return Err(JournalError::InvalidInput);
    }
    let phase = object
        .get("phase")
        .and_then(|v| v.as_object())
        .ok_or(JournalError::InvalidInput)?;
    let state = phase
        .get("state")
        .and_then(|v| v.as_str())
        .ok_or(JournalError::InvalidInput)?;
    let allowed: &[&str] = match state {
        "intent" => &["state"],
        "sourcesScrubbed" => &["state", "stagedSourceSetCasAfterScrub"],
        "cutoverCommitted" => &["state", "stagedSourceSetCasAfterScrub", "cutoverReceiptId"],
        "liveOwnerMinted" | "localBindingFinalized" => &[
            "state",
            "stagedSourceSetCasAfterScrub",
            "cutoverReceiptId",
            "promotedLiveOwner",
        ],
        _ => return Err(JournalError::InvalidInput),
    };
    if phase.keys().any(|key| !allowed.contains(&key.as_str())) || phase.len() != allowed.len() {
        return Err(JournalError::InvalidInput);
    }
    Ok(())
}
