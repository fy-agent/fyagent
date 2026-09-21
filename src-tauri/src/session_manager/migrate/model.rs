//! Migration data model.
//!
//! The package DTOs are an ordered message sequence, never question/answer
//! pairs: consecutive user messages and an unanswered trailing user message are
//! real conversation states, not malformed data.
//!
//! These types are the export/restore contract. They are deliberately separate
//! from the display-oriented [`crate::session_manager::SessionMessage`], which
//! synthesizes `[Tool: name]` text and inlines tool results and therefore can
//! never be used as migration input.

use serde::{Deserialize, Serialize};

/// The only package schema this build reads or writes.
pub const PACKAGE_SCHEMA: &str = "fyagent.session.v1";

/// Structural limits. Every one of them fails the operation; nothing is ever
/// truncated, because a truncated transcript is a falsified transcript.
pub mod limits {
    /// Cumulative bytes read from one source store during extraction.
    pub const SOURCE_READ_BYTES: u64 = 256 * 1024 * 1024;
    /// One message body.
    pub const MESSAGE_BYTES: u64 = 4 * 1024 * 1024;
    /// Messages in one session.
    pub const SESSION_MESSAGES: u32 = 4000;
    /// Total body bytes in one session.
    pub const SESSION_BYTES: u64 = 32 * 1024 * 1024;
    /// Sessions in one package.
    pub const PACKAGE_SESSIONS: u32 = 200;
    /// Package file size, checked from file metadata before parsing.
    pub const PACKAGE_FILE_BYTES: u64 = 128 * 1024 * 1024;
    /// JSON nesting depth in a package file.
    pub const JSON_DEPTH: u32 = 64;
    /// Identity strings longer than this are rejected, never truncated.
    pub const IDENTITY_FIELD_CHARS: usize = 200;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SessionPackage {
    pub schema: String,
    pub exported_at: i64,
    pub exporter: ExporterInfo,
    pub sessions: Vec<MigratableSession>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExporterInfo {
    pub app: String,
    pub app_version: String,
    pub platform: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MigratableSession {
    /// Semantic snapshot of source identity plus content. Recomputed on read;
    /// the value carried by a package file is never trusted.
    pub snapshot_id: String,
    /// Pure content digest, derived from `messages` alone.
    pub content_digest: String,
    pub origin: OriginIdentity,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_active_at: Option<i64>,
    /// Basename of the source workspace. Absolute paths never enter a package.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_label: Option<String>,
    /// Ordered messages. Never paired up, padded, or merged by role.
    pub messages: Vec<MigratableMessage>,
    pub extraction: ExtractionReport,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OriginIdentity {
    /// Stable source identifier carried through to the receipt. A new native ID
    /// on the target never replaces it.
    pub origin_id: String,
    pub provider_id: String,
    /// Native session ID of the source. Absent stays absent: an empty string,
    /// a file name, or a timestamp must never stand in for it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cli_version: Option<String>,
    /// Diagnostic only. Never drives an automatic merge.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub store_fingerprint: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MigratableMessage {
    /// Dense index from 0, equal to the position in `messages`.
    pub seq: u32,
    pub kind: MessageKind,
    /// Verbatim source text. Never trimmed, reflowed, path-rewritten, or
    /// sanitized.
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ts: Option<i64>,
}

/// Only these two kinds exist inside a package. Commentary, tool events,
/// reasoning, attachments and runtime injections are counted in
/// [`OmittedCounts`] and dropped; anything undecidable fails the export.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MessageKind {
    UserText,
    AssistantFinal,
}

impl MessageKind {
    /// Domain byte used by the content digest.
    pub(crate) fn digest_byte(self) -> u8 {
        match self {
            MessageKind::UserText => b'U',
            MessageKind::AssistantFinal => b'A',
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExtractionReport {
    /// Rule that produced this session, e.g. `codex.rollout.phase-v1`.
    pub rule_id: String,
    /// Provider versions the rule has fixture evidence for.
    pub rule_verified_versions: Vec<String>,
    /// `seq` of every `UserText` with no following `AssistantFinal`. A derived
    /// fact about an unfinished turn, not an error.
    pub open_user_messages: Vec<u32>,
    pub omitted: OmittedCounts,
    pub source_path_family: PathFamily,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OmittedCounts {
    pub tool_events: u32,
    pub reasoning_blocks: u32,
    pub commentary_messages: u32,
    pub attachments: u32,
    pub runtime_injections: u32,
    pub unknown_blocks: u32,
}

impl OmittedCounts {
    pub fn merge(&mut self, other: &OmittedCounts) {
        self.tool_events += other.tool_events;
        self.reasoning_blocks += other.reasoning_blocks;
        self.commentary_messages += other.commentary_messages;
        self.attachments += other.attachments;
        self.runtime_injections += other.runtime_injections;
        self.unknown_blocks += other.unknown_blocks;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PathFamily {
    Posix,
    Windows,
    Unknown,
}

impl PathFamily {
    /// Classify a raw source path without keeping it. Only the shape survives
    /// into the package, so the target can warn about foreign-OS paths that
    /// remain inside verbatim message bodies.
    pub fn classify(raw: &str) -> Self {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return PathFamily::Unknown;
        }
        let windows_drive = {
            let mut chars = trimmed.chars();
            matches!(chars.next(), Some(c) if c.is_ascii_alphabetic())
                && matches!(chars.next(), Some(':'))
                && matches!(chars.next(), Some('\\') | Some('/'))
        };
        if windows_drive || trimmed.starts_with("\\\\") {
            return PathFamily::Windows;
        }
        if trimmed.starts_with('/') || trimmed.starts_with("~/") {
            return PathFamily::Posix;
        }
        PathFamily::Unknown
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreRequest {
    pub package_path: String,
    /// Generated once per explicit user action. A retry of the same action must
    /// reuse it.
    pub request_id: String,
    /// Restore only these snapshots; empty means every session in the package.
    #[serde(default)]
    pub snapshot_ids: Vec<String>,
    pub target_provider_id: String,
    /// Absolute directory the user picked on the target machine.
    pub target_workspace: String,
    pub request_kind: RestoreRequestKind,
}

/// There is deliberately no `Overwrite`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RestoreRequestKind {
    /// Takes the idempotency slot: one semantic snapshot lands in one target
    /// store exactly once.
    DefaultImport,
    /// Explicit extra copy. No slot, its own mapping, and every prior mapping
    /// is kept.
    SaveAsNewCopy,
}

impl RestoreRequestKind {
    pub fn as_str(self) -> &'static str {
        match self {
            RestoreRequestKind::DefaultImport => "defaultImport",
            RestoreRequestKind::SaveAsNewCopy => "saveAsNewCopy",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "defaultImport" => Some(RestoreRequestKind::DefaultImport),
            "saveAsNewCopy" => Some(RestoreRequestKind::SaveAsNewCopy),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreAttempt {
    /// Primary key. One row per attempt, never reused.
    pub attempt_id: String,
    pub request_id: String,
    pub snapshot_id: String,
    pub request_kind: RestoreRequestKind,
    /// Present only for `DefaultImport`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idempotency_slot: Option<String>,
    pub content_digest: String,
    pub origin: OriginIdentity,
    pub target_provider_id: String,
    pub target_store_id: String,
    /// Pre-allocated before any write, so a target that returns no ID can still
    /// be reconciled.
    pub target_native_nonce: String,
    /// Returned by the target. Absent while unknown; never guessed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_native_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_workspace: Option<String>,
    pub stage: RestoreStage,
    pub attempt_count: u32,
    /// Parallel to `stage` and never merged into it.
    pub user_attestation: Option<UserAttestation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<MigrationError>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Advanced by system evidence only. A renderer can never push a stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RestoreStage {
    /// Package parsed and passed schema, size and extraction-rule checks.
    PackageVerified,
    /// Write decided, outcome not yet known. The crash window lives here.
    NativeWritePending,
    /// The target write call returned success.
    NativeWritten,
    /// History confirmed through the target's own read channel — not by us
    /// scanning its files on disk.
    NativeReadbackVerified,
    /// The target application was launched and loaded the session.
    TargetOpened,
    /// Still readable after the target restarted.
    RestartReadbackVerified,
    /// The next outbound request really carried the migrated history.
    NextTurnRequestVerified,
    /// A real model reply referenced the migrated facts.
    NextTurnReplyVerified,
    /// Write outcome unknown; reconciliation must run before anything else.
    NeedsReconciliation,
    /// Several candidates or none locatable. No automatic action allowed.
    Ambiguous,
    /// Positive evidence that no side effect was produced.
    Failed,
}

impl RestoreStage {
    pub fn as_str(self) -> &'static str {
        match self {
            RestoreStage::PackageVerified => "packageVerified",
            RestoreStage::NativeWritePending => "nativeWritePending",
            RestoreStage::NativeWritten => "nativeWritten",
            RestoreStage::NativeReadbackVerified => "nativeReadbackVerified",
            RestoreStage::TargetOpened => "targetOpened",
            RestoreStage::RestartReadbackVerified => "restartReadbackVerified",
            RestoreStage::NextTurnRequestVerified => "nextTurnRequestVerified",
            RestoreStage::NextTurnReplyVerified => "nextTurnReplyVerified",
            RestoreStage::NeedsReconciliation => "needsReconciliation",
            RestoreStage::Ambiguous => "ambiguous",
            RestoreStage::Failed => "failed",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        let stage = match raw {
            "packageVerified" => RestoreStage::PackageVerified,
            "nativeWritePending" => RestoreStage::NativeWritePending,
            "nativeWritten" => RestoreStage::NativeWritten,
            "nativeReadbackVerified" => RestoreStage::NativeReadbackVerified,
            "targetOpened" => RestoreStage::TargetOpened,
            "restartReadbackVerified" => RestoreStage::RestartReadbackVerified,
            "nextTurnRequestVerified" => RestoreStage::NextTurnRequestVerified,
            "nextTurnReplyVerified" => RestoreStage::NextTurnReplyVerified,
            "needsReconciliation" => RestoreStage::NeedsReconciliation,
            "ambiguous" => RestoreStage::Ambiguous,
            "failed" => RestoreStage::Failed,
            _ => return None,
        };
        Some(stage)
    }

    /// Stages at or above native readback are the only ones that may be
    /// presented as "restored".
    pub fn is_restored(self) -> bool {
        matches!(
            self,
            RestoreStage::NativeReadbackVerified
                | RestoreStage::TargetOpened
                | RestoreStage::RestartReadbackVerified
                | RestoreStage::NextTurnRequestVerified
                | RestoreStage::NextTurnReplyVerified
        )
    }

    /// Unresolved write outcome: another native write is forbidden until
    /// reconciliation produces authoritative evidence.
    pub fn blocks_native_write(self) -> bool {
        matches!(
            self,
            RestoreStage::NativeWritePending
                | RestoreStage::NeedsReconciliation
                | RestoreStage::Ambiguous
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserAttestation {
    pub attested_at: i64,
    /// What the user says they saw. Display and manual tracing only.
    pub claimed_stage: RestoreStage,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// Release-level capability, decided by engineering acceptance rather than by
/// asking the user to run a probe.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseCapability {
    pub provider_id: String,
    /// Fixture-proven extraction rule. Required before any export.
    pub extraction_rule: Option<ExtractionRuleInfo>,
    /// `None` means this release verified no write path at all.
    pub write_strategy: Option<String>,
    /// Per-stage verified versions. An empty list means never verified.
    pub verified_stages: Vec<(RestoreStage, Vec<String>)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractionRuleInfo {
    pub rule_id: String,
    pub verified_versions: Vec<String>,
}

/// Runtime probe of this machine, kept separate from the release matrix.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalProviderProbe {
    pub provider_id: String,
    pub installed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detected_version: Option<String>,
    /// Safe to export from. Says nothing about restoring.
    pub extraction_supported: bool,
    /// Safe to restore into.
    pub write_supported: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason_code: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportOutcome {
    pub path: String,
    pub session_count: u32,
    pub byte_len: u64,
    /// SHA-256 of the package file. Evidence only: it never takes part in
    /// idempotency, because re-exporting the same conversation legitimately
    /// produces different bytes.
    pub package_file_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageReadResult {
    pub package: SessionPackage,
    /// Receipts already recorded on this machine for the packaged snapshots.
    pub attempts: Vec<RestoreAttempt>,
}

/// One camelCase error vocabulary shared by Rust, the IPC boundary and the UI.
///
/// There is deliberately no "succeeded with zero messages" outcome: every path
/// that would produce an empty transcript maps to one of these codes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "code", content = "detail")]
pub enum MigrationError {
    // —— source reading ——
    SourceUnreadable {
        provider_id: String,
        reason: String,
    },
    SourceTooLarge {
        read_bytes: u64,
        limit: u64,
    },

    // —— extraction ——
    ExtractionRuleUnavailable {
        provider_id: String,
        detected_version: Option<String>,
    },
    ExtractionRuleVersionMismatch {
        provider_id: String,
        detected: Option<String>,
        verified: Vec<String>,
    },
    FinalAnswerIndeterminate {
        seq: u32,
        reason: String,
    },
    RuntimeInjectionUnclassified {
        seq: u32,
        kinds: Vec<String>,
    },

    // —— package ——
    PackageSchemaUnsupported {
        found: String,
        supported: Vec<String>,
    },
    PackageUnknownField {
        pointer: String,
        field: String,
    },
    PackageMalformed {
        reason: String,
    },
    PackageTooLarge {
        bytes: u64,
        limit: u64,
    },
    JsonTooDeep {
        depth: u32,
        limit: u32,
    },
    MessageTooLarge {
        seq: u32,
        bytes: u64,
        limit: u64,
    },
    TooManyMessages {
        count: u32,
        limit: u32,
    },
    SessionTooLarge {
        bytes: u64,
        limit: u64,
    },
    TooManySessions {
        count: u32,
        limit: u32,
    },
    IdentityFieldInvalid {
        field: String,
        reason: String,
    },
    PackageWriteFailed {
        path: String,
        reason: String,
    },

    // —— target capability ——
    ProviderNotInstalled {
        provider_id: String,
    },
    ProviderVersionUnsupported {
        provider_id: String,
        detected: Option<String>,
        verified: Vec<String>,
    },
    CapabilityProbeFailed {
        provider_id: String,
        reason: String,
    },
    TargetStoreUnidentified {
        provider_id: String,
    },
    /// Restoring a transcript into a different product is a cross-model
    /// conversion, not a recovery, so it is refused rather than approximated.
    ProviderMismatch {
        source_provider_id: String,
        target_provider_id: String,
    },

    // —— target write ——
    TargetDirectoryNotFound {
        path: String,
    },
    NativeImportFailed {
        provider_id: String,
        exit_code: Option<i32>,
        stderr_tail: String,
    },
    NativeProtocolFailed {
        provider_id: String,
        method: String,
        reason: String,
    },
    NativeReadbackMismatch {
        attempt_id: String,
        expected_digest: String,
        observed: String,
    },

    // —— receipts and reconciliation ——
    SourceSnapshotConflict {
        origin_id: String,
        existing_attempt_id: String,
    },
    IdempotencySlotTaken {
        existing_attempt_id: String,
        existing_stage: RestoreStage,
    },
    ReconciliationRequired {
        attempt_id: String,
        reason: String,
    },
    AmbiguousNativeMatch {
        attempt_id: String,
        candidates: Vec<String>,
    },
    ReceiptStoreFailed {
        reason: String,
    },
    AttemptNotFound {
        attempt_id: String,
    },
}

impl MigrationError {
    /// Stable machine code. UI copy is keyed off this, never off `detail`.
    pub fn code(&self) -> &'static str {
        match self {
            MigrationError::SourceUnreadable { .. } => "sourceUnreadable",
            MigrationError::SourceTooLarge { .. } => "sourceTooLarge",
            MigrationError::ExtractionRuleUnavailable { .. } => "extractionRuleUnavailable",
            MigrationError::ExtractionRuleVersionMismatch { .. } => "extractionRuleVersionMismatch",
            MigrationError::FinalAnswerIndeterminate { .. } => "finalAnswerIndeterminate",
            MigrationError::RuntimeInjectionUnclassified { .. } => "runtimeInjectionUnclassified",
            MigrationError::PackageSchemaUnsupported { .. } => "packageSchemaUnsupported",
            MigrationError::PackageUnknownField { .. } => "packageUnknownField",
            MigrationError::PackageMalformed { .. } => "packageMalformed",
            MigrationError::PackageTooLarge { .. } => "packageTooLarge",
            MigrationError::JsonTooDeep { .. } => "jsonTooDeep",
            MigrationError::MessageTooLarge { .. } => "messageTooLarge",
            MigrationError::TooManyMessages { .. } => "tooManyMessages",
            MigrationError::SessionTooLarge { .. } => "sessionTooLarge",
            MigrationError::TooManySessions { .. } => "tooManySessions",
            MigrationError::IdentityFieldInvalid { .. } => "identityFieldInvalid",
            MigrationError::PackageWriteFailed { .. } => "packageWriteFailed",
            MigrationError::ProviderNotInstalled { .. } => "providerNotInstalled",
            MigrationError::ProviderVersionUnsupported { .. } => "providerVersionUnsupported",
            MigrationError::CapabilityProbeFailed { .. } => "capabilityProbeFailed",
            MigrationError::TargetStoreUnidentified { .. } => "targetStoreUnidentified",
            MigrationError::ProviderMismatch { .. } => "providerMismatch",
            MigrationError::TargetDirectoryNotFound { .. } => "targetDirectoryNotFound",
            MigrationError::NativeImportFailed { .. } => "nativeImportFailed",
            MigrationError::NativeProtocolFailed { .. } => "nativeProtocolFailed",
            MigrationError::NativeReadbackMismatch { .. } => "nativeReadbackMismatch",
            MigrationError::SourceSnapshotConflict { .. } => "sourceSnapshotConflict",
            MigrationError::IdempotencySlotTaken { .. } => "idempotencySlotTaken",
            MigrationError::ReconciliationRequired { .. } => "reconciliationRequired",
            MigrationError::AmbiguousNativeMatch { .. } => "ambiguousNativeMatch",
            MigrationError::ReceiptStoreFailed { .. } => "receiptStoreFailed",
            MigrationError::AttemptNotFound { .. } => "attemptNotFound",
        }
    }

    /// Serialize for the IPC boundary. Commands return this string so the
    /// renderer always receives `{code, detail?}` and never a prose message.
    pub fn to_ipc_string(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| format!("{{\"code\":\"{}\"}}", self.code()))
    }
}

impl std::fmt::Display for MigrationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_ipc_string())
    }
}

impl std::error::Error for MigrationError {}

pub type MigrationResult<T> = Result<T, MigrationError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_payload_is_camel_case_code_and_detail() {
        let error = MigrationError::FinalAnswerIndeterminate {
            seq: 7,
            reason: "phase unknown".to_string(),
        };
        let json: serde_json::Value = serde_json::from_str(&error.to_ipc_string()).unwrap();
        assert_eq!(json["code"], "finalAnswerIndeterminate");
        assert_eq!(json["detail"]["seq"], 7);
        assert_eq!(json["detail"]["reason"], "phase unknown");
        assert_eq!(error.code(), json["code"].as_str().unwrap());
    }

    #[test]
    fn every_error_code_round_trips_through_serde() {
        // The `code()` table and the serde tag are two hand-written lists; a
        // drift between them would give the UI an unmapped code.
        let samples = vec![
            MigrationError::SourceUnreadable {
                provider_id: "codex".into(),
                reason: "io".into(),
            },
            MigrationError::SourceTooLarge {
                read_bytes: 1,
                limit: 2,
            },
            MigrationError::ExtractionRuleUnavailable {
                provider_id: "gemini".into(),
                detected_version: None,
            },
            MigrationError::ExtractionRuleVersionMismatch {
                provider_id: "codex".into(),
                detected: Some("0.1.0".into()),
                verified: vec!["=0.154.0".into()],
            },
            MigrationError::FinalAnswerIndeterminate {
                seq: 0,
                reason: "x".into(),
            },
            MigrationError::RuntimeInjectionUnclassified {
                seq: 0,
                kinds: vec!["y".into()],
            },
            MigrationError::PackageSchemaUnsupported {
                found: "v2".into(),
                supported: vec![PACKAGE_SCHEMA.into()],
            },
            MigrationError::PackageUnknownField {
                pointer: "/sessions/0".into(),
                field: "extra".into(),
            },
            MigrationError::PackageMalformed { reason: "x".into() },
            MigrationError::PackageTooLarge { bytes: 1, limit: 2 },
            MigrationError::JsonTooDeep {
                depth: 65,
                limit: 64,
            },
            MigrationError::MessageTooLarge {
                seq: 1,
                bytes: 2,
                limit: 3,
            },
            MigrationError::TooManyMessages { count: 1, limit: 2 },
            MigrationError::SessionTooLarge { bytes: 1, limit: 2 },
            MigrationError::TooManySessions { count: 1, limit: 2 },
            MigrationError::IdentityFieldInvalid {
                field: "sessionId".into(),
                reason: "control".into(),
            },
            MigrationError::PackageWriteFailed {
                path: "/tmp/p".into(),
                reason: "io".into(),
            },
            MigrationError::ProviderNotInstalled {
                provider_id: "grokbuild".into(),
            },
            MigrationError::ProviderVersionUnsupported {
                provider_id: "codex".into(),
                detected: None,
                verified: vec![],
            },
            MigrationError::CapabilityProbeFailed {
                provider_id: "codex".into(),
                reason: "timeout".into(),
            },
            MigrationError::TargetStoreUnidentified {
                provider_id: "claude".into(),
            },
            MigrationError::ProviderMismatch {
                source_provider_id: "codex".into(),
                target_provider_id: "opencode".into(),
            },
            MigrationError::TargetDirectoryNotFound { path: "/x".into() },
            MigrationError::NativeImportFailed {
                provider_id: "opencode".into(),
                exit_code: Some(1),
                stderr_tail: String::new(),
            },
            MigrationError::NativeProtocolFailed {
                provider_id: "codex".into(),
                method: "thread/start".into(),
                reason: "closed".into(),
            },
            MigrationError::NativeReadbackMismatch {
                attempt_id: "a".into(),
                expected_digest: "b".into(),
                observed: "c".into(),
            },
            MigrationError::SourceSnapshotConflict {
                origin_id: "o".into(),
                existing_attempt_id: "a".into(),
            },
            MigrationError::IdempotencySlotTaken {
                existing_attempt_id: "a".into(),
                existing_stage: RestoreStage::NativeWritten,
            },
            MigrationError::ReconciliationRequired {
                attempt_id: "a".into(),
                reason: "unknown".into(),
            },
            MigrationError::AmbiguousNativeMatch {
                attempt_id: "a".into(),
                candidates: vec![],
            },
            MigrationError::ReceiptStoreFailed { reason: "x".into() },
            MigrationError::AttemptNotFound {
                attempt_id: "a".into(),
            },
        ];

        for error in samples {
            let encoded = error.to_ipc_string();
            let decoded: MigrationError = serde_json::from_str(&encoded).unwrap();
            assert_eq!(decoded, error);
            let json: serde_json::Value = serde_json::from_str(&encoded).unwrap();
            assert_eq!(json["code"].as_str().unwrap(), error.code());
        }
    }

    #[test]
    fn every_stage_string_round_trips() {
        for stage in [
            RestoreStage::PackageVerified,
            RestoreStage::NativeWritePending,
            RestoreStage::NativeWritten,
            RestoreStage::NativeReadbackVerified,
            RestoreStage::TargetOpened,
            RestoreStage::RestartReadbackVerified,
            RestoreStage::NextTurnRequestVerified,
            RestoreStage::NextTurnReplyVerified,
            RestoreStage::NeedsReconciliation,
            RestoreStage::Ambiguous,
            RestoreStage::Failed,
        ] {
            assert_eq!(RestoreStage::parse(stage.as_str()), Some(stage));
            let json = serde_json::to_string(&stage).unwrap();
            assert_eq!(json, format!("\"{}\"", stage.as_str()));
        }
    }

    #[test]
    fn package_rejects_unknown_fields() {
        let raw = r#"{"schema":"fyagent.session.v1","exportedAt":1,"exporter":{"app":"fyagent","appVersion":"1","platform":"macos"},"sessions":[],"extra":true}"#;
        let error = serde_json::from_str::<SessionPackage>(raw).unwrap_err();
        assert!(error.to_string().contains("extra"));
    }

    #[test]
    fn write_blocking_stages_cover_every_unresolved_outcome() {
        assert!(RestoreStage::NativeWritePending.blocks_native_write());
        assert!(RestoreStage::NeedsReconciliation.blocks_native_write());
        assert!(RestoreStage::Ambiguous.blocks_native_write());
        // `failed` is proven side-effect-free, so an idempotent retry is allowed.
        assert!(!RestoreStage::Failed.blocks_native_write());
        assert!(!RestoreStage::NativeWritten.blocks_native_write());
    }

    #[test]
    fn restored_stages_start_at_native_readback() {
        assert!(!RestoreStage::NativeWritten.is_restored());
        assert!(RestoreStage::NativeReadbackVerified.is_restored());
        assert!(RestoreStage::NextTurnReplyVerified.is_restored());
    }

    #[test]
    fn path_family_classifies_source_shapes() {
        assert_eq!(PathFamily::classify("/Users/a/b"), PathFamily::Posix);
        assert_eq!(PathFamily::classify("C:\\work\\repo"), PathFamily::Windows);
        assert_eq!(
            PathFamily::classify("\\\\server\\share"),
            PathFamily::Windows
        );
        assert_eq!(PathFamily::classify("relative/path"), PathFamily::Unknown);
        assert_eq!(PathFamily::classify(""), PathFamily::Unknown);
    }
}
