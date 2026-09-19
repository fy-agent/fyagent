use serde::{Deserialize, Serialize};

pub(crate) type VerificationResult<T> = Result<T, &'static str>;

macro_rules! wire_enum {
    ($name:ident { $($variant:ident),+ }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
        #[serde(rename_all = "snake_case")]
        pub(crate) enum $name { $($variant),+ }
    };
}
wire_enum!(Stage {
    ConfigurationSaved,
    AuthenticationAvailable,
    ToolCallable,
    SamplePassed,
    CustomerAccepted
});
wire_enum!(Outcome {
    Passed,
    Failed,
    Unknown,
    Unsupported,
    Cancelled
});
wire_enum!(SourceClass {
    NativeLocal,
    NativeRemote,
    LocalFixture,
    ManualRecord
});
wire_enum!(Validity {
    Current,
    Stale,
    Revoked,
    Unverifiable
});
wire_enum!(Checker {
    SavedConfigurationReadback,
    SavedModelProbe,
    KitValidator
});
wire_enum!(KitFixture {
    Baseline,
    MissingField
});
wire_enum!(SampleCode {
    Ok,
    InvalidInput,
    DuplicateRow,
    PeriodMismatch,
    CurrencyMismatch,
    ZeroPrevious,
    MissingPeriod,
    InvalidAmount
});
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SampleReceipt {
    pub input_digest: String,
    pub code: SampleCode,
    pub matches_expectation: bool,
}
wire_enum!(ProjectionState {
    Confirmed,
    Different,
    Unavailable
});
wire_enum!(Rollback {
    ReviewConfiguration,
    GuardedFileRecovery,
    ManualOnly,
    NotAvailable
});

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct KitIdentity {
    pub kit_id: String,
    pub kit_version: String,
    pub manifest_digest: String,
}

/// Native-only input. The project owner must provide monotonic revisions even
/// for A -> B -> A. No renderer can mint these snapshots or credential epochs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ProjectDependencySnapshot {
    pub project_id: String,
    pub project_revision: u64,
    pub kit: Option<KitIdentity>,
    pub active: bool,
    pub resource_revision: String,
    pub credential_generation: Option<String>,
    pub configuration_saved: bool,
    pub projection: ProjectionState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ExternalBasis {
    pub reference: String,
    pub issuer: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ManualRequest {
    pub project_id: String,
    pub expected_revision: u64,
    pub stage: Stage,
    pub outcome: Outcome,
    pub person: String,
    pub role: String,
    pub scope: String,
    pub observed_at: String,
    pub external_basis: Option<ExternalBasis>,
    pub basis_evidence_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ManualDetails {
    pub person: String,
    pub role: String,
    pub scope: String,
    pub external_basis: Option<ExternalBasis>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Evidence {
    pub id: String,
    pub dependencies: ProjectDependencySnapshot,
    pub stage: Stage,
    pub outcome: Outcome,
    pub source_class: SourceClass,
    pub checker_id: String,
    pub fixture: Option<KitFixture>,
    pub sample: Option<SampleReceipt>,
    pub checker_version: u32,
    pub app_version: String,
    pub observed_at: String,
    pub recorded_at: String,
    pub expires_at: Option<String>,
    pub reason_code: String,
    pub basis_evidence_ids: Vec<String>,
    pub manual: Option<ManualDetails>,
    pub invalidated_during_run: bool,
}

/// Credential epochs and resource fingerprints never cross IPC/export.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct EvidenceView {
    pub id: String,
    pub project_revision: u64,
    pub kit: Option<KitIdentity>,
    pub stage: Stage,
    pub outcome: Outcome,
    pub validity: Validity,
    pub source_class: SourceClass,
    pub checker_id: String,
    pub fixture: Option<KitFixture>,
    pub sample: Option<SampleReceipt>,
    pub checker_version: u32,
    pub app_version: String,
    pub observed_at: String,
    pub recorded_at: String,
    pub expires_at: Option<String>,
    pub reason_code: String,
    pub basis_evidence_ids: Vec<String>,
    pub manual: Option<ManualDetails>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct HandoffNotes {
    pub items: Vec<HandoffItem>,
    pub rollback: Option<Rollback>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct HandoffItem {
    pub title: String,
    pub owner: Option<String>,
    pub completed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct VerificationSnapshot {
    pub schema_version: u32,
    pub project_id: String,
    pub project_revision: Option<u64>,
    pub available: bool,
    pub checked_at: String,
    pub evidence: Vec<EvidenceView>,
    pub handoff: HandoffNotes,
    pub handoff_revision: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RunRequest {
    pub project_id: String,
    pub expected_revision: u64,
    pub checker: Checker,
    pub run_id: String,
    pub fixture: Option<KitFixture>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RevokeRequest {
    pub project_id: String,
    pub evidence_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SaveHandoffRequest {
    pub project_id: String,
    pub expected_revision: u64,
    pub handoff_revision: u64,
    pub handoff: HandoffNotes,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct HandoffPreview {
    pub snapshot: VerificationSnapshot,
    pub json: String,
    pub markdown: String,
}
