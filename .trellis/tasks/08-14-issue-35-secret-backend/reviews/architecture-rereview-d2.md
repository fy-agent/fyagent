# Issue #35 D2 immutable architecture rereview

ARCHITECTURE_REREVIEW_D2=APPROVE
DESIGN_AUTHORITY_SHA=a338ee18edad759c5507be6372af3813eff1f429
P0=0
P1=0
P2=0
evidence=static_design

## Review basis

Working-tree HEAD is `a338ee18edad759c5507be6372af3813eff1f429` (`docs(secrets): close design rereview findings`). The only untracked path is `reviews/product-rereview-d2.md`, which is not authority. Authority was read from the matching working-tree design files; SHA-256 of the four primary blobs was independently recomputed from `git show a338ee18…:<path>` and matches the D2 product rereview snapshot. D1 `reviews/architecture-rereview.md` (`REQUEST_CHANGES`, `P1=3`) was used only as the finding list to close. No test, build, dependency resolution, browser, server, native runtime or screenshot evidence was used. This rereview does not claim implementation.

## ARR closure

### ARR-001 — CLOSED — candidate discard/expiry now has a representable fresh-missing authorization

D1 required a closed two-slot discard algebra: distinct delete and missing-readback slots, two one-shot authorizations, a reservation fulfilled only by a durable delete-applied checkpoint, `Validate` mapping for the missing slot, a matching `BackendDeleteAppliedCas` in the discard journal, and no sixth hardware operation or fifth recovery kind.

Exact D2 evidence:

- `secret-contract-v1.md` TypeScript slots at the `CandidateDiscardConfirmationSlot` / `SecretCandidateDiscardHardwareConfirmStep` block: `recordDelete` is `operation="delete"` / `scope="candidateDiscardRecordDelete"`; `recordMissingReadback` is `operation="validate"` / `scope="candidateDiscardRecordMissingReadback"`. `CandidateDiscardDeleteCheckpoint` is branded `{deleteDisposition,backendCompletedAt,deleteAppliedCas}`. `DiscardCandidateJournalPhaseWire` is `intent → backendApplied{checkpoint} → missingReadbackVerified{checkpoint,missingCheckedAt} → recoveryRequired{same checkpoint union} → terminal`.
- `secret-contract-v1.md` Rust `wire_enum!(CandidateDiscardDeleteOperation { Delete })` / `CandidateDiscardMissingReadbackOperation { Validate }` and `CandidateDiscardConfirmationSlot::{RecordDelete,RecordMissingReadback}`.
- `secret-contract-v1.md` `BackendAuthorizationScope::require_missing_readback` now accepts `SecretNonApplyBackendOperation::CandidateDiscard { slot: RecordMissingReadback }` in addition to activation old-record and the three recovery missing slots. `require_delete_mode` todo text forbids using the missing slot as Delete.
- `secret-contract-v1.md` `PreparedCandidateDiscardBundle` holds independent `record_delete` + `record_missing_readback`; the missing prepared type carries `delete_applied_cas_reservation`. `AuthorizedBackendMissingReadback::readback_missing_once` requires the actual `BackendDeleteAppliedCas`.
- `secret-contract-v1.md` `DiscardCandidateJournalPhase::{BackendApplied,MissingReadbackVerified}` and `DiscardCandidateRecoveryCheckpoint` embed `CandidateDiscardDeleteCheckpoint { delete_disposition, backend_completed_at, delete_applied_cas }`. `CandidateDeleteJournalRow` stores both literal slots and independent confirmation policies.
- `device-local-secret-store.md` §6 `DiscardCandidateJournal` / `CandidateDiscardPreparedBundle` and §7.1.1: sequence is consume RecordDelete → durable `backendApplied{CandidateDiscardDeleteCheckpoint}` → consume RecordMissingReadback only after the matching actual CAS → `missingReadbackVerified` → atomic terminal. No `stateFinalized` and no general recovery row / fifth kind.
- Hardware algebra remains exactly five operations: `secret-contract-v1.md` `wire_enum!(SecretBackendOperation { CaptureVerify, Validate, ResolveForApply, Delete, Revoke })` plus the comment that missing-readback scopes map to `Validate`. Recovery kinds remain exactly four: `wire_enum!(SecretRecoveryKind { ActivationCleanup, CaptureCompensation, DeleteFinalization, OwnerDetachFinalization })` and `DurableSecretRecoveryRecord` has those four arms only.

Minimum closure from D1 is met. No new P1/P2.

### ARR-002 — CLOSED — activation old-record crash state retains delete-receipt provenance

D1 required `ActivateCandidateJournalPhase::OldRecordDeleteApplied` and the activation-cleanup nonterminal/recovery checkpoint to carry `{deleteDisposition,backendCompletedAt,deleteAppliedCas}`; the codec/recovery preimage must consume those fields; missing receipt and terminal supersession must commit atomically with no standalone empty-suffix phase; `revokedAt=backendCompletedAt`.

Exact D2 evidence:

- `secret-contract-v1.md` `ActivationOldRecordDurableCheckpoint::OldRecordDeleteApplied { delete_disposition, backend_completed_at, delete_applied_cas }`.
- `secret-contract-v1.md` `ActivateCandidateJournalPhase::OldRecordDeleteApplied { checkpoint: ActivationOldRecordDeleteCheckpoint }` and `RecoveryRequired { checkpoint: ActivationOldRecordDurableCheckpoint, recovery: ActivationCleanupRecoveryLink }`.
- `secret-contract-v1.md` `ActivationOldRecordDeleteCheckpoint` and `RecoveryOldRecordDeleteCheckpoint` are the same three-field record. Failure uses `into_durable_failure_checkpoint`; recovery uses `checked_from_durable_failure_checkpoint` / `into_recovery_required_checkpoint`. Scanner text forbids `From/Into` and CAS-only reconstruction.
- `secret-contract-v1.md` `ActivationCleanupRecoveryPhase::OldRecordDeleteApplied { checkpoint: RecoveryOldRecordDeleteCheckpoint }` and `RecoveryRequired { checkpoint: ActivationOldRecordDurableCheckpoint }`.
- `secret-contract-v1.md` `AuthorizedActivationOldRecordMissingReadback::verify_missing_once` sets `revoked_at = applied.checkpoint.backend_completed_at` and returns supersession + missing in one completion; the journal comment forbids an empty-suffix missing-verified phase.
- `device-local-secret-store.md` `ActivateCandidateJournalPhase` old-record arm carries `ActivationOldRecordDeleteCheckpoint`; `recoveryRequired` retains `ActivationOldRecordDurableCheckpoint`. §7.3–§7.4: crash-visible checkpoints are only `none|deleteApplied|stateFinalized`; `deleteApplied` keeps the three-field receipt plus remaining `verifyOldRecordMissing`; the missing receipt is consumed in the same device-authority transaction that writes `supersededByRotation` and `revokedAt=backendCompletedAt`.
- Recovery CAS preimage grammar in `secret-contract-v1.md` encodes `deleteReceipt\0<deleted|alreadyMissing>\0<backendCompletedAt>\0<deleteAppliedCasRevision>\0<deleteAppliedCasDigest>` and terminal `supersession\0supersededByRotation\0<backendCompletedAt>`.

Minimum closure from D1 is met. No new P1/P2.

### ARR-003 — CLOSED — staged-resume CAS preimage encodes the frozen digest domain

D1 required `operationId` plus the closed journal phase on the preimage identity, replacement of the three-arm checkpoint with an exact five-arm union, structurally required/forbidden phase-specific receipt/promoted-owner rows, one canonical digest fixture per phase, and revision change on every fresh nonce/admission or phase/checkpoint transition.

Exact D2 evidence:

- `secret-contract-v1.md` `StagedImportResumePhase` is the five-arm union `Intent | SourcesScrubbed{after-scrub CAS} | CutoverCommitted{CAS,cutover} | LiveOwnerMinted{CAS,cutover,promotedLiveOwner} | LocalBindingFinalized{same three cumulative fields}`.
- `secret-contract-v1.md` `StagedImportResumePreimageIdentity` includes `operation_id: SecretOperationId`. `StagedImportResumePreimage` is `{identity, phase: StagedImportResumePhase}`. `DurableSecretOperationJournal` owns immutable `operation_id`.
- `secret-contract-v1.md` `StagedImportJournalPhase` now has matching `Intent`, `SourcesScrubbed`, `CutoverCommitted`, `LiveOwnerMinted`, `LocalBindingFinalized`, plus `RecoveryRequired { resume_phase: StagedImportResumePhase }` and terminal with the three cumulative fields. The former three-arm `StagedImportRecoveryCheckpoint` is gone.
- `secret-contract-v1.md` canonical preimage grammar starts `operation\0<operationId>` then `phase\0<intent|sourcesScrubbed|cutoverCommitted|liveOwnerMinted|localBindingFinalized>` and forbids omitted/extra cumulative rows. Five named fixtures: `staged_resume_intent_v1`, `staged_resume_sources_scrubbed_v1`, `staged_resume_cutover_committed_v1`, `staged_resume_live_owner_minted_v1`, `staged_resume_local_binding_finalized_v1`. Phase/nonce/admission/receipt/owner change increments revision then recomputes digest.
- `device-local-secret-store.md` §6 / §8.3: `StagedImportResumePhase` is the only phase algebra; `intent` has no receipts; later arms accumulate; `localBindingFinalized` is distinguished by the phase literal. Public request remains `stageId + expectedResumeCas{revision,digest}`.

Minimum closure from D1 is met. No new P1/P2.

## Architecture boundaries otherwise confirmed

- Secret records, bindings, candidates, journals, audit and recovery remain device-local under `app_local_data_dir/device-local/secrets/v1`. `detailed-design-overview.md` §1.4 and `device-local-secret-store.md` §0.1: #35 owns no SQLite schema or v17 `user_version` transition; Prompt/Memory keeps v17; export/sync never copy that authority.
- Durable `DeviceInstanceId` and process-local `DeviceSecretStoreInstanceId` stay distinct. One stateful `Arc<BackendOperationBroker>` owns capture-intent, prepared-capability and pending-confirmation registries.
- Candidate activation and live apply use separate #55 plans and separate #41 leases; #35 never acquires the Provider lease. Staged import order remains temp token/projection → #55 admission → main-integration authority-match → #35 prepare/confirm → cutover context.
- Hardware operations remain five; recovery kinds remain four; discard `RecoveryRequired` is a journal-internal checkpoint, not a fifth `DurableSecretRecoveryRecord` kind.
- Native/MSRV/lock/license/advisory, implementation, UAT and production evidence remain downstream gates.

## D2 authority SHA-256 snapshot

| Authority path | SHA-256 |
| --- | --- |
| `.trellis/tasks/08-14-issue-35-secret-backend/secret-contract-v1.md` | `44da40384499df4e1936e12e7006cd89e5f0bc41e98343892df14c5e654e5041` |
| `.trellis/tasks/08-14-issue-35-secret-backend/device-local-secret-store.md` | `07fb3ea341a51ec92a5f50e1745fac1e3eb51037c0e173f5cea4cc4b06a62bb8` |
| `.trellis/tasks/08-14-issue-35-secret-backend/technical-design-overview.md` | `2f5f13d006d3e20b50689e357438297dbac91e0e54e20f3f66be786c5f5fd69c` |
| `.trellis/tasks/08-14-issue-35-secret-backend/detailed-design-overview.md` | `ae4e768e1a2270600e1aa4fb95ed494b5f48aaf445a4147bc8afa7fb173124fe` |

Hashes above were recomputed from `git show a338ee18edad759c5507be6372af3813eff1f429:<path>`. Remaining D2 blobs listed in `reviews/product-rereview-d2.md` were not re-hashed in this pass and are not required to close ARR-001/002/003.

## Verdict

D2 closed ARR-001, ARR-002 and ARR-003 in the frozen Rust/TS authority shapes. Architecture freeze is no longer blocked by those three algebra gaps. This is `static_design` only.
