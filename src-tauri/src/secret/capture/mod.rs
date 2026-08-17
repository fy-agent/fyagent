//! Process-local single-use capture intent registry and local orchestration.
//!
//! Success stages an unbound `verifiedPendingPlan` candidate only. Cancel,
//! expire, replay, empty/invalid material, and user cancel are zero-write:
//! no keychain, no journal, no candidate, no owner binding, no live/auth.json.

use std::collections::HashMap;
use std::sync::Mutex;

use crate::secret::device_store::journal::{write_journal, list_journals};
use crate::secret::device_store::schema::{
    CaptureLikePhase, JournalEnvelope, JournalOperationKind, StoredBindingSetCas,
    StoredCandidateKind, StoredCandidateRecord, StoredCandidateState, StoredPolicyState,
    StoredRetirementState, StoredSecretRecord,
};
use crate::secret::device_store::{utc_now, DeviceLocalSecretStore};
use crate::secret::testing::InMemorySecretBackend;
use crate::secret::{
    BeginCaptureIntent, SecretBackendInstanceId, SecretCandidateId, SecretCaptureIntentId,
    SecretInternalError, SecretMaterial, SecretOperationId, SecretPurpose, SecretRef,
    SecretSourceFreeErrorCode, SecretTerminalOperationContext,
};

#[cfg(target_os = "macos")]
pub(crate) mod macos;
#[cfg(target_os = "windows")]
pub(crate) mod windows;

pub(crate) trait CapturePrompt: Send + Sync {
    fn prompt_once(&self) -> Result<SecretMaterial, SecretInternalError>;
}

pub(crate) struct ProgrammaticCapturePrompt {
    bytes: Vec<u8>,
}

impl ProgrammaticCapturePrompt {
    pub(crate) fn new(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }
}

impl CapturePrompt for ProgrammaticCapturePrompt {
    fn prompt_once(&self) -> Result<SecretMaterial, SecretInternalError> {
        SecretMaterial::from_native_input(self.bytes.clone(), SecretPurpose::CodexApiKey)
    }
}

pub(crate) struct CancelCapturePrompt;

impl CapturePrompt for CancelCapturePrompt {
    fn prompt_once(&self) -> Result<SecretMaterial, SecretInternalError> {
        Err(SecretInternalError::terminal_operation_failure(
            SecretSourceFreeErrorCode::InputCancelled,
            SecretTerminalOperationContext::Capture(BeginCaptureIntent::NewBinding),
        ))
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum IntentState {
    Ready,
    Claimed,
    Consumed,
    Cancelled,
    Expired,
}

struct IntentRow {
    owner: String,
    purpose: SecretPurpose,
    intent: BeginCaptureIntent,
    backend_instance_id: SecretBackendInstanceId,
    state: IntentState,
}

/// Process-local single-use capture-intent registry.
pub(crate) struct SecretCaptureIntentRegistry {
    rows: Mutex<HashMap<String, IntentRow>>,
}

pub(crate) struct ClaimedCaptureIntent {
    id: SecretCaptureIntentId,
    owner: String,
    purpose: SecretPurpose,
    intent: BeginCaptureIntent,
    backend_instance_id: SecretBackendInstanceId,
}

impl SecretCaptureIntentRegistry {
    pub(crate) fn new() -> Self {
        Self {
            rows: Mutex::new(HashMap::new()),
        }
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, HashMap<String, IntentRow>>, SecretInternalError> {
        self.rows
            .lock()
            .map_err(|_| SecretInternalError::terminal_operation_failure(
                SecretSourceFreeErrorCode::Internal,
                SecretTerminalOperationContext::Capture(BeginCaptureIntent::NewBinding),
            ))
    }

    pub(crate) fn mint(
        &self,
        owner: impl Into<String>,
        purpose: SecretPurpose,
        intent: BeginCaptureIntent,
        backend_instance_id: SecretBackendInstanceId,
    ) -> Result<SecretCaptureIntentId, SecretInternalError> {
        let id = SecretCaptureIntentId::generate();
        let mut rows = self.lock()?;
        rows.insert(
            id.as_str().to_string(),
            IntentRow {
                owner: owner.into(),
                purpose,
                intent,
                backend_instance_id,
                state: IntentState::Ready,
            },
        );
        Ok(id)
    }

    pub(crate) fn claim_once(
        &self,
        capture_intent_id: &SecretCaptureIntentId,
        backend_instance_id: &SecretBackendInstanceId,
    ) -> Result<ClaimedCaptureIntent, SecretInternalError> {
        let mut rows = self.lock()?;
        let row = rows.get_mut(capture_intent_id.as_str()).ok_or_else(|| {
            SecretInternalError::terminal_operation_failure(
                SecretSourceFreeErrorCode::RequestInvalid,
                SecretTerminalOperationContext::Capture(BeginCaptureIntent::NewBinding),
            )
        })?;
        if row.state != IntentState::Ready || &row.backend_instance_id != backend_instance_id {
            return Err(SecretInternalError::terminal_operation_failure(
                SecretSourceFreeErrorCode::CapabilityConsumed,
                SecretTerminalOperationContext::Capture(row.intent),
            ));
        }
        row.state = IntentState::Claimed;
        Ok(ClaimedCaptureIntent {
            id: capture_intent_id.clone(),
            owner: row.owner.clone(),
            purpose: row.purpose,
            intent: row.intent,
            backend_instance_id: row.backend_instance_id.clone(),
        })
    }

    pub(crate) fn cancel(
        &self,
        capture_intent_id: &SecretCaptureIntentId,
    ) -> Result<(), SecretInternalError> {
        self.terminalize(capture_intent_id, IntentState::Cancelled)
    }

    pub(crate) fn expire(
        &self,
        capture_intent_id: &SecretCaptureIntentId,
    ) -> Result<(), SecretInternalError> {
        self.terminalize(capture_intent_id, IntentState::Expired)
    }

    fn terminalize(
        &self,
        capture_intent_id: &SecretCaptureIntentId,
        next: IntentState,
    ) -> Result<(), SecretInternalError> {
        let mut rows = self.lock()?;
        if let Some(row) = rows.get_mut(capture_intent_id.as_str()) {
            if row.state == IntentState::Ready || row.state == IntentState::Claimed {
                row.state = next;
            }
        }
        Ok(())
    }

    fn consume(&self, claim: &ClaimedCaptureIntent) -> Result<(), SecretInternalError> {
        let mut rows = self.lock()?;
        if let Some(row) = rows.get_mut(claim.id.as_str()) {
            row.state = IntentState::Consumed;
        }
        Ok(())
    }
}

pub(crate) enum CaptureLeafBackend<'a> {
    InMemory(&'a InMemorySecretBackend),
    #[cfg(target_os = "macos")]
    MacOs(&'a crate::secret::platform::macos::MacOsSecretStore),
}

pub(crate) struct LocalSecretCapture<'a> {
    store: &'a DeviceLocalSecretStore,
    backend: CaptureLeafBackend<'a>,
    registry: &'a SecretCaptureIntentRegistry,
    prompt: &'a dyn CapturePrompt,
}

pub(crate) struct StagedUnboundCandidate {
    pub candidate_id: SecretCandidateId,
    pub secret_ref: SecretRef,
}

impl<'a> LocalSecretCapture<'a> {
    pub(crate) fn new(
        store: &'a DeviceLocalSecretStore,
        backend: CaptureLeafBackend<'a>,
        registry: &'a SecretCaptureIntentRegistry,
        prompt: &'a dyn CapturePrompt,
    ) -> Self {
        Self {
            store,
            backend,
            registry,
            prompt,
        }
    }

    pub(crate) fn begin_after_claim(
        &self,
        claim: ClaimedCaptureIntent,
    ) -> Result<StagedUnboundCandidate, SecretInternalError> {
        let material = match self.prompt.prompt_once() {
            Ok(material) => material,
            Err(error) => {
                let _ = self
                    .registry
                    .terminalize(&claim.id, IntentState::Cancelled);
                return Err(error);
            }
        };
        let secret_ref = SecretRef::generate();
        self.backend_create_new(&secret_ref, material)?;
        let staged = self.persist_unbound_candidate(&claim, &secret_ref)?;
        self.registry.consume(&claim)?;
        let _ = claim.owner;
        let _ = claim.purpose;
        Ok(staged)
    }

    fn backend_create_new(
        &self,
        secret_ref: &SecretRef,
        material: SecretMaterial,
    ) -> Result<(), SecretInternalError> {
        match self.backend {
            CaptureLeafBackend::InMemory(backend) => {
                let locator = secret_ref.as_str();
                if backend.validate_missing(locator).is_err() {
                    return Err(SecretInternalError::terminal_operation_failure(
                        SecretSourceFreeErrorCode::BackendChanged,
                        SecretTerminalOperationContext::Capture(BeginCaptureIntent::NewBinding),
                    ));
                }
                backend.write(locator, material)
            }
            #[cfg(target_os = "macos")]
            CaptureLeafBackend::MacOs(store) => {
                store.create_new(secret_ref, material).map(|_| ())
            }
        }
    }

    fn persist_unbound_candidate(
        &self,
        claim: &ClaimedCaptureIntent,
        secret_ref: &SecretRef,
    ) -> Result<StagedUnboundCandidate, SecretInternalError> {
        let now = utc_now();
        let operation_id = SecretOperationId::generate().as_str().to_string();
        let mut journal = JournalEnvelope::capture_like_intent(
            JournalOperationKind::CaptureCandidate,
            operation_id,
            self.store.device_instance_id().as_str().to_string(),
            now.clone(),
        )
        .map_err(|_| SecretInternalError::input_invalid())?;
        write_journal(self.store.root(), &journal)
            .map_err(|_| SecretInternalError::input_invalid())?;

        let verify_receipt_id = uuid::Uuid::new_v4().simple().to_string();
        journal
            .advance_capture_like(
                CaptureLikePhase::backend_applied(verify_receipt_id)
                    .map_err(|_| SecretInternalError::input_invalid())?,
                utc_now(),
            )
            .map_err(|_| SecretInternalError::input_invalid())?;
        write_journal(self.store.root(), &journal)
            .map_err(|_| SecretInternalError::input_invalid())?;

        let candidate_id = SecretCandidateId::generate();
        let mut payload = self.store.load()?.payload;
        payload.secrets.push(StoredSecretRecord {
            secret_ref: secret_ref.as_str().to_string(),
            purpose: "codexApiKey".to_string(),
            backend_instance_id: claim.backend_instance_id.as_str().to_string(),
            backend_locator: Some(secret_ref.as_str().to_string()),
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
            candidate_id: candidate_id.as_str().to_string(),
            candidate_revision: 1,
            kind: StoredCandidateKind::NewBinding,
            state: StoredCandidateState::VerifiedPendingPlan,
            secret_ref: secret_ref.as_str().to_string(),
            record_revision: 1,
            backend_instance_id: claim.backend_instance_id.as_str().to_string(),
            backend_generation: 1,
            device_binding_generation: 1,
            capability_revision: 1,
            created_at: now.clone(),
            expires_at: now.clone(),
            updated_at: now.clone(),
            pending_terminal_disposition: None,
        });
        payload
            .secrets
            .sort_by(|left, right| left.secret_ref.cmp(&right.secret_ref));
        payload
            .candidates
            .sort_by(|left, right| left.candidate_id.cmp(&right.candidate_id));
        payload.store_revision = payload.store_revision.saturating_add(1);
        payload.updated_at = utc_now();
        self.store.store(payload)?;

        journal
            .advance_capture_like(CaptureLikePhase::StateFinalized, utc_now())
            .map_err(|_| SecretInternalError::input_invalid())?;
        write_journal(self.store.root(), &journal)
            .map_err(|_| SecretInternalError::input_invalid())?;

        let _ = list_journals;
        Ok(StagedUnboundCandidate {
            candidate_id,
            secret_ref: secret_ref.clone(),
        })
    }
}

#[allow(dead_code)]
fn _keep_claim_fields(claim: &ClaimedCaptureIntent) {
    let _ = (&claim.owner, claim.purpose, claim.intent);
}
