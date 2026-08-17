use std::path::PathBuf;

use super::device_store::journal::{
    mint_delete_applied_cas, write_journal, StagedImportResumePreimage,
};
use super::device_store::schema::{
    CandidateDiscardDeleteCheckpoint, DeleteAppliedRole, DeleteDisposition, DiscardSlot,
    JournalEnvelope, JournalError, StoredBindingSetCas, StoredBindingState, StoredCandidateKind,
    StoredCandidateRecord, StoredCandidateState, StoredOwner, StoredOwnerBindingRecord,
    StoredPolicyState, StoredRetirementState, StoredSecretRecord, TerminalDisposition,
};
use super::device_store::{utc_now, DeviceLocalSecretStore};
use super::testing::InMemorySecretBackend;
use super::{
    SecretCandidateId, SecretCommandError, SecretCommandId, SecretCommandSuccess,
    SecretContractVersionV1, SecretErrorView, SecretInternalError, SecretMaterial, SecretOperationId,
    SecretPurpose, SecretRef, SecretService, SchemaVersionV1,
};

pub(crate) fn command_error_from_internal(error: SecretInternalError) -> SecretCommandError {
    SecretCommandError {
        contract_version: SecretContractVersionV1::V1,
        schema_version: SchemaVersionV1,
        command_id: SecretCommandId::generate(),
        error: SecretErrorView::checked_from_internal(error, None, None, None),
    }
}

pub(crate) fn command_success<T>(data: T) -> SecretCommandSuccess<T> {
    SecretCommandSuccess {
        contract_version: SecretContractVersionV1::V1,
        schema_version: SchemaVersionV1,
        command_id: SecretCommandId::generate(),
        data,
    }
}

/// Phase 2A helper: SecretService lives in the included operation.rs
/// namespace. This module only adds command envelope helpers.
pub(crate) fn service_unavailable() -> SecretInternalError {
    SecretInternalError::input_invalid()
}

#[allow(dead_code)]
fn _service_ref(_: &SecretService) {}

/// Store-side of `SecretService` for Phase 2A.
///
/// The full contract `SecretService` needs #55/#41/main traits and `AppState`.
/// This type is the device-store surface only: list projections and store-side
/// candidate discard against a TempDir-injected `DeviceLocalSecretStore` and an
/// `InMemorySecretBackend` double.
pub(crate) struct SecretServiceLocal {
    store: DeviceLocalSecretStore,
    backend: InMemorySecretBackend,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct LocalOwnerSummary {
    pub owner_id: String,
    pub state: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secret_ref: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct LocalRefSummary {
    pub secret_ref: String,
    pub availability: String,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct LocalSecretSummaryProjection {
    pub owners: Vec<LocalOwnerSummary>,
    pub refs: Vec<LocalRefSummary>,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct LocalCandidateProjection {
    pub candidate_id: String,
    pub candidate_revision: u64,
    pub secret_ref: String,
    pub state: StoredCandidateState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending_terminal_disposition: Option<TerminalDisposition>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SeededPendingCandidate {
    pub candidate_id: String,
    pub candidate_revision: u64,
    pub secret_ref: String,
    pub record_revision: u64,
    pub backend_locator: String,
    pub owner_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum LocalDiscardOutcome {
    Discarded { candidate_id: String },
    AlreadyTerminal { candidate_id: String, state: StoredCandidateState },
}

impl From<JournalError> for SecretInternalError {
    fn from(_: JournalError) -> Self {
        SecretInternalError::input_invalid()
    }
}

pub(crate) fn list_secret_summaries_from_store(
    store: &DeviceLocalSecretStore,
    secret_ref: Option<&str>,
    include_unbound_owners: bool,
) -> Result<LocalSecretSummaryProjection, SecretInternalError> {
    let payload = store.load()?.payload;
    let mut refs: Vec<LocalRefSummary> = payload
        .secrets
        .iter()
        .filter(|row| secret_ref.map(|want| row.secret_ref == want).unwrap_or(true))
        .map(|row| LocalRefSummary {
            secret_ref: row.secret_ref.clone(),
            availability: match row.retirement_state {
                StoredRetirementState::Live => "live".to_string(),
                StoredRetirementState::Stale => "stale".to_string(),
                StoredRetirementState::Revoked => "revoked".to_string(),
            },
        })
        .collect();
    refs.sort_by(|a, b| a.secret_ref.cmp(&b.secret_ref));

    let mut owners: Vec<LocalOwnerSummary> = payload
        .owner_bindings
        .iter()
        .filter(|row| match row.state {
            StoredBindingState::Unbound => include_unbound_owners,
            StoredBindingState::Bound => secret_ref
                .map(|want| row.secret_ref.as_deref() == Some(want))
                .unwrap_or(true),
        })
        .map(|row| LocalOwnerSummary {
            owner_id: row.owner.owner_id.clone(),
            state: match row.state {
                StoredBindingState::Unbound => "unbound".to_string(),
                StoredBindingState::Bound => "bound".to_string(),
            },
            secret_ref: row.secret_ref.clone(),
        })
        .collect();
    owners.sort_by(|a, b| a.owner_id.cmp(&b.owner_id));
    Ok(LocalSecretSummaryProjection { owners, refs })
}

pub(crate) fn list_secret_candidates_from_store(
    store: &DeviceLocalSecretStore,
    include_terminal: bool,
) -> Result<Vec<LocalCandidateProjection>, SecretInternalError> {
    let payload = store.load()?.payload;
    let mut rows: Vec<LocalCandidateProjection> = payload
        .candidates
        .iter()
        .filter(|row| include_terminal || !row.state.is_terminal())
        .map(|row| LocalCandidateProjection {
            candidate_id: row.candidate_id.clone(),
            candidate_revision: row.candidate_revision,
            secret_ref: row.secret_ref.clone(),
            state: row.state,
            pending_terminal_disposition: row.pending_terminal_disposition,
        })
        .collect();
    rows.sort_by(|a, b| a.candidate_id.cmp(&b.candidate_id));
    Ok(rows)
}

pub(crate) fn seed_pending_candidate_in_store(
    store: &DeviceLocalSecretStore,
    backend: &InMemorySecretBackend,
    material: SecretMaterial,
    bind_owner: bool,
) -> Result<SeededPendingCandidate, SecretInternalError> {
    let now = utc_now();
    let secret_ref = SecretRef::generate().as_str().to_string();
    let candidate_id = SecretCandidateId::generate().as_str().to_string();
    let locator = format!("loc-{}", &secret_ref[4..12]);
    backend.write(&locator, material)?;
    let mut payload = store.load()?.payload;
    payload.secrets.push(StoredSecretRecord {
        secret_ref: secret_ref.clone(),
        purpose: "codexApiKey".to_string(),
        backend_instance_id: "sbi_aaaaaaaaaaa4aaa8aaaaaaaaaaaaaaaa".to_string(),
        backend_locator: Some(locator.clone()),
        record_revision: 1,
        binding_set_cas: StoredBindingSetCas {
            revision: 1,
            digest: "0".repeat(64),
            count: 0,
        },
        backend_generation: 1,
        device_binding_generation: 1,
        capability_revision: 1,
        policy_state: StoredPolicyState::Active,
        retirement_state: StoredRetirementState::Live,
        created_at: now.clone(),
        updated_at: now.clone(),
    });
    payload.candidates.push(StoredCandidateRecord {
        candidate_id: candidate_id.clone(),
        candidate_revision: 1,
        kind: StoredCandidateKind::NewBinding,
        state: StoredCandidateState::VerifiedPendingPlan,
        secret_ref: secret_ref.clone(),
        record_revision: 1,
        backend_instance_id: "sbi_aaaaaaaaaaa4aaa8aaaaaaaaaaaaaaaa".to_string(),
        backend_generation: 1,
        device_binding_generation: 1,
        capability_revision: 1,
        created_at: now.clone(),
        expires_at: now.clone(),
        updated_at: now.clone(),
        pending_terminal_disposition: None,
    });
    let owner_id = if bind_owner {
        let owner_id = format!("owner-{}", &secret_ref[4..10]);
        payload.owner_bindings.push(StoredOwnerBindingRecord {
            owner: StoredOwner {
                kind: "provider".to_string(),
                namespace: "codex".to_string(),
                owner_id: owner_id.clone(),
                slot: "primaryApiKey".to_string(),
            },
            purpose: "codexApiKey".to_string(),
            owner_binding_revision: 1,
            state: StoredBindingState::Bound,
            secret_ref: Some(secret_ref.clone()),
            binding_revision: Some(1),
            created_at: now.clone(),
            updated_at: now.clone(),
        });
        Some(owner_id)
    } else {
        None
    };
    payload.secrets.sort_by(|a, b| a.secret_ref.cmp(&b.secret_ref));
    payload
        .candidates
        .sort_by(|a, b| a.candidate_id.cmp(&b.candidate_id));
    payload.store_revision = payload.store_revision.saturating_add(1);
    payload.updated_at = now;
    store.store(payload)?;
    Ok(SeededPendingCandidate {
        candidate_id,
        candidate_revision: 1,
        secret_ref,
        record_revision: 1,
        backend_locator: locator,
        owner_id,
    })
}

pub(crate) fn seed_unbound_owner_in_store(
    store: &DeviceLocalSecretStore,
    owner_id: &str,
) -> Result<(), SecretInternalError> {
    let now = utc_now();
    let mut payload = store.load()?.payload;
    payload.owner_bindings.push(StoredOwnerBindingRecord {
        owner: StoredOwner {
            kind: "provider".to_string(),
            namespace: "codex".to_string(),
            owner_id: owner_id.to_string(),
            slot: "primaryApiKey".to_string(),
        },
        purpose: "codexApiKey".to_string(),
        owner_binding_revision: 1,
        state: StoredBindingState::Unbound,
        secret_ref: None,
        binding_revision: None,
        created_at: now.clone(),
        updated_at: now.clone(),
    });
    payload.store_revision = payload.store_revision.saturating_add(1);
    payload.updated_at = now;
    store.store(payload)?;
    Ok(())
}

pub(crate) fn discard_secret_candidate_in_store(
    store: &DeviceLocalSecretStore,
    candidate_id: &str,
    expected_revision: u64,
    backend: &InMemorySecretBackend,
) -> Result<LocalDiscardOutcome, SecretInternalError> {
    let payload = store.load()?.payload;
    let candidate = payload
        .candidates
        .iter()
        .find(|row| row.candidate_id == candidate_id)
        .cloned()
        .ok_or_else(SecretInternalError::input_invalid)?;
    if candidate.state.is_terminal() {
        return Ok(LocalDiscardOutcome::AlreadyTerminal {
            candidate_id: candidate.candidate_id,
            state: candidate.state,
        });
    }
    if candidate.candidate_revision != expected_revision {
        return Err(SecretInternalError::dependency_changed());
    }
    let record = payload
        .secrets
        .iter()
        .find(|row| row.secret_ref == candidate.secret_ref)
        .cloned()
        .ok_or_else(SecretInternalError::input_invalid)?;
    let locator = record
        .backend_locator
        .clone()
        .ok_or_else(SecretInternalError::input_invalid)?;

    let now = utc_now();
    let operation_id = SecretOperationId::generate().as_str().to_string();
    let mut journal = JournalEnvelope::discard_intent(
        operation_id.clone(),
        store.device_instance_id().as_str().to_string(),
        now.clone(),
        TerminalDisposition::Discarded,
    )?;
    write_journal(store.root(), &journal).map_err(|_| SecretInternalError::input_invalid())?;

    let disposition = backend.delete_or_already_missing(&locator)?;
    let completed_at = utc_now();
    let cas = mint_delete_applied_cas(
        &operation_id,
        journal.operation_kind(),
        DeleteAppliedRole::DiscardRecordDelete,
        disposition,
        &completed_at,
        candidate.record_revision,
    )?;
    let checkpoint = CandidateDiscardDeleteCheckpoint::checked(
        disposition,
        completed_at,
        cas,
    )?;
    journal.consume_discard_slot(
        DiscardSlot::RecordDelete,
        Some(checkpoint),
        None,
        utc_now(),
    )?;
    write_journal(store.root(), &journal).map_err(|_| SecretInternalError::input_invalid())?;
    set_pending_disposition(store, candidate_id, Some(TerminalDisposition::Discarded))?;

    backend.validate_missing(&locator)?;
    let missing_checked_at = utc_now();
    journal.consume_discard_slot(
        DiscardSlot::RecordMissingReadback,
        None,
        Some(missing_checked_at),
        utc_now(),
    )?;
    write_journal(store.root(), &journal).map_err(|_| SecretInternalError::input_invalid())?;

    journal.finalize_discard_terminal(utc_now())?;
    write_journal(store.root(), &journal).map_err(|_| SecretInternalError::input_invalid())?;
    finalize_discarded_candidate(store, candidate_id)?;
    Ok(LocalDiscardOutcome::Discarded {
        candidate_id: candidate_id.to_string(),
    })
}

fn set_pending_disposition(
    store: &DeviceLocalSecretStore,
    candidate_id: &str,
    disposition: Option<TerminalDisposition>,
) -> Result<(), SecretInternalError> {
    let mut payload = store.load()?.payload;
    let row = payload
        .candidates
        .iter_mut()
        .find(|row| row.candidate_id == candidate_id)
        .ok_or_else(SecretInternalError::input_invalid)?;
    row.pending_terminal_disposition = disposition;
    row.updated_at = utc_now();
    payload.store_revision = payload.store_revision.saturating_add(1);
    payload.updated_at = utc_now();
    store.store(payload)?;
    Ok(())
}

fn finalize_discarded_candidate(
    store: &DeviceLocalSecretStore,
    candidate_id: &str,
) -> Result<(), SecretInternalError> {
    let mut payload = store.load()?.payload;
    let candidate = payload
        .candidates
        .iter()
        .find(|row| row.candidate_id == candidate_id)
        .cloned()
        .ok_or_else(SecretInternalError::input_invalid)?;
    payload
        .secrets
        .retain(|row| row.secret_ref != candidate.secret_ref);
    let row = payload
        .candidates
        .iter_mut()
        .find(|row| row.candidate_id == candidate_id)
        .ok_or_else(SecretInternalError::input_invalid)?;
    row.state = StoredCandidateState::Discarded;
    row.pending_terminal_disposition = None;
    row.updated_at = utc_now();
    payload.store_revision = payload.store_revision.saturating_add(1);
    payload.updated_at = utc_now();
    store.store(payload)?;
    Ok(())
}

impl SecretServiceLocal {
    pub(crate) fn open(root: PathBuf) -> Result<Self, SecretInternalError> {
        Ok(Self {
            store: DeviceLocalSecretStore::open(root)?,
            backend: InMemorySecretBackend::new(),
        })
    }

    pub(crate) fn store(&self) -> &DeviceLocalSecretStore {
        &self.store
    }

    pub(crate) fn backend(&self) -> &InMemorySecretBackend {
        &self.backend
    }

    pub(crate) fn list_secret_summaries(
        &self,
        secret_ref: Option<&str>,
        include_unbound_owners: bool,
    ) -> Result<LocalSecretSummaryProjection, SecretInternalError> {
        list_secret_summaries_from_store(&self.store, secret_ref, include_unbound_owners)
    }

    pub(crate) fn list_secret_candidates(
        &self,
        include_terminal: bool,
    ) -> Result<Vec<LocalCandidateProjection>, SecretInternalError> {
        list_secret_candidates_from_store(&self.store, include_terminal)
    }

    pub(crate) fn seed_pending_candidate(
        &self,
        backend: &InMemorySecretBackend,
        material: SecretMaterial,
        bind_owner: bool,
    ) -> Result<SeededPendingCandidate, SecretInternalError> {
        seed_pending_candidate_in_store(&self.store, backend, material, bind_owner)
    }

    pub(crate) fn seed_unbound_owner(&self, owner_id: &str) -> Result<(), SecretInternalError> {
        seed_unbound_owner_in_store(&self.store, owner_id)
    }

    pub(crate) fn discard_secret_candidate(
        &self,
        candidate_id: &str,
        expected_revision: u64,
        backend: &InMemorySecretBackend,
    ) -> Result<LocalDiscardOutcome, SecretInternalError> {
        discard_secret_candidate_in_store(&self.store, candidate_id, expected_revision, backend)
    }
}

#[cfg(test)]
pub(crate) fn seed_opened_store_pending_candidate(
    store: &DeviceLocalSecretStore,
) -> Result<SeededPendingCandidate, SecretInternalError> {
    let backend = InMemorySecretBackend::new();
    let material = SecretMaterial::from_native_input(
        b"seed-from-opened-store".to_vec(),
        SecretPurpose::CodexApiKey,
    )?;
    seed_pending_candidate_in_store(store, &backend, material, true)
}

#[allow(dead_code)]
fn _keep_resume_codec(preimage: StagedImportResumePreimage) {
    let _ = preimage;
}

#[allow(dead_code)]
fn _keep_purpose(_: SecretPurpose) {}
