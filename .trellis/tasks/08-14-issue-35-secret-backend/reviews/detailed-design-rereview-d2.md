# Issue #35 D2 immutable detailed-design rereview

DETAILED_DESIGN_REREVIEW_D2=APPROVE
DESIGN_AUTHORITY_SHA=a338ee18edad759c5507be6372af3813eff1f429
P0=0
P1=0
P2=0
evidence=static_design

## Review basis

Same immutable candidate as the D2 product rereview and this architecture rereview: `a338ee18edad759c5507be6372af3813eff1f429`. Working-tree design docs match that commit; `reviews/product-rereview-d2.md` is untracked and was not used as authority. Counts, owner matrix, SQLite/v17 boundary and no-value public surface were re-read from `secret-contract-v1.md`, `device-local-secret-store.md`, `technical-design-overview.md` and `detailed-design-overview.md`. No test, build, dependency resolution, browser, server, native runtime or screenshot evidence was used. This rereview does not claim implementation.

## Public counts

Independently counted from the D2 contract mirrors. Totals are unchanged by ARR.

| Kind | Claimed | Verified literals |
| --- | --- | --- |
| #35 commands | 15 | Rust `SecretCommandName` 15 variants: `ListSecretSummaries`, `ListSecretBackendOptions`, `BeginSecretCapture`, `RotateSecret`, `ListSecretCandidates`, `DiscardSecretCandidate`, `SetSecretLocked`, `GetSecretDeleteImpact`, `DeleteSecret`, `GetSecretCleanupImpact`, `RetrySecretCleanup`, `ValidateSecret`, `CheckSecretApplyReadiness`, `MigrateLegacyCodexSecrets`, `ListSecretAudit`. TypeScript `SecretCommandName` has the same 15 snake_case names. |
| main-integration handler | +1, not a 16th #35 command | Rust `SecretMainIntegrationCommandName::ResumeStagedImportCutover`; TypeScript `"resume_staged_import_cutover"`. `secret-contract-v1.md` staged-resume prose: this handler is not one of the two cleanup-named commands and does not add a 16th secret command. `detailed-design-overview.md` §2 registration receipt proves `15 + resume_staged_import_cutover`. |
| error codes | 47 | Rust `SecretErrorCode` 47 variants; TypeScript `SecretErrorCode` 47 `SECRET_*` literals. Discard fresh-missing failure reuses existing `SECRET_READ_FAILED`; no new error literal. |
| user actions | 24 | Rust `SecretUserAction` 24 variants; TypeScript `SecretUserAction` 24 literals; `SECRET_ACTION_DESTINATIONS_V1` is `satisfies Record<SecretUserAction, SecretActionDestination>` so every action has one destination. No generic `retry`. |
| journals | 8 | `DurableSecretOperationJournalRepr`: `CaptureCandidate`, `MigrateLegacy`, `RotateCandidate`, `ActivateCandidate`, `DiscardCandidate`, `DeleteSecret`, `DetachProviderOwner`, `StagedImport`. `technical-design-overview.md` and `device-local-secret-store.md` repeat the same eight-kind list. No ninth generic recovery operation. |
| recovery kinds | 4 | `SecretRecoveryKind` and `DurableSecretRecoveryRecord`: `ActivationCleanup`, `CaptureCompensation`, `DeleteFinalization`, `OwnerDetachFinalization` only. Discard `RecoveryRequired` is a journal-internal checkpoint, not a fifth durable recovery kind. |
| hardware operations | 5 | `SecretBackendOperation::{CaptureVerify,Validate,ResolveForApply,Delete,Revoke}`. Missing-readback scopes map to `Validate`. |
| prepared slots | 12 / delete-missing 10 | `device-local-secret-store.md` §11.3: activation+recovery 10 slots (8 delete/missing) plus candidate-discard `RecordDelete`/`RecordMissingReadback` = 12 / 10. |

## Owner boundaries

`detailed-design-overview.md` §1 and `device-local-secret-store.md` §2:

- **#35 module** owns only the listed new secret/core/platform/capture/V2/scanner paths. It does not edit existing Provider/proxy/import/schema/registration files, does not own the DB, and does not acquire a Provider lease.
- **#55** exclusively owns Change Plan domain/DAO/commands and the owner-private admission factory. ARR adds no new #55 public request or material surface.
- **#41** exclusively owns Configuration Apply coordinator, sanitized backup, Provider lease/write/readback/recovery. ARR-002 completes the old-record checkpoint; it does not move lease or writer ownership into #35.
- **main integration** serially owns the exact shared paths in `research/codex-secret-call-graph.md` §9.4, including AppState/startup/registration, Provider/DAO, import/restore/sync cutover, and the separate staged-resume handler.
- Canonical owner literals remain `#35 module | #55 | #41 | main integration`. Prompt/Memory is a retained external schema lane, not a fifth owner.

## No SQLite / v17 secret store

- `detailed-design-overview.md` §1.4: `database/schema.rs` stays with Prompt/Memory v17; #35 adds no schema/table/version and only verifies absence of secret state in SQLite.
- `device-local-secret-store.md` §0.1: #35 withdraws SQLite v17 ownership. Records, bindings, journals, audit and per-device capability live in `device-local-secret-store/v1` under `app_local_data_dir`, independent of `PRAGMA user_version`.
- `secret-contract-v1.md` closure map AR-005: normative #35 boundary adds no SQLite schema/`user_version` transition and does not occupy v17.

## No-value public boundary

- Renderer cannot supply timestamp, operation id, backend locator, ref display or material (`detailed-design-overview.md` §4).
- Public staged resume request is exactly `{stageId, expectedResumeCas:{revision,digest}}`. Every resume result arm is `{stageId, currentResumeCas, status, action, issue}` with no candidate, owner, ref, summary, path, locator or material (`secret-contract-v1.md` staged-resume section; `device-local-secret-store.md` §0.9 / §8.3).
- Candidate-discard preparation views are native-only and are not command results (`secret-contract-v1.md` `SecretCandidateDiscardPreparationView` comment).
- `FORBIDDEN_SEMANTIC_FIELDS_V1` remains the sole public-key denylist; ARR did not add a value-bearing field to any #35/#55/#41 public DTO.
- Test builder accepts only closed no-value fixture modes; raw root/path, material and service constructors stay non-public (`detailed-design-overview.md` §2).

## ARR impact on detailed design

ARR-001/002/003 change internal authority and crash-proofing only. They do not add a command, public action, error literal, journal kind, recovery kind or hardware operation. Slot count rises to 12 by adding two discard slots; slots are not user operations. Detailed-design totals and owner matrix therefore stay closed.

Unverified and out of scope: actual handler registration in source, scanner execution, lock/license/advisory/MSRV resolution, native macOS/Windows evidence, and any #55/#41 implementation SHA compatibility. Those remain later gates.

## D2 authority SHA-256 snapshot

| Authority path | SHA-256 |
| --- | --- |
| `.trellis/tasks/08-14-issue-35-secret-backend/secret-contract-v1.md` | `44da40384499df4e1936e12e7006cd89e5f0bc41e98343892df14c5e654e5041` |
| `.trellis/tasks/08-14-issue-35-secret-backend/device-local-secret-store.md` | `07fb3ea341a51ec92a5f50e1745fac1e3eb51037c0e173f5cea4cc4b06a62bb8` |
| `.trellis/tasks/08-14-issue-35-secret-backend/technical-design-overview.md` | `2f5f13d006d3e20b50689e357438297dbac91e0e54e20f3f66be786c5f5fd69c` |
| `.trellis/tasks/08-14-issue-35-secret-backend/detailed-design-overview.md` | `ae4e768e1a2270600e1aa4fb95ed494b5f48aaf445a4147bc8afa7fb173124fe` |

## Verdict

D2 detailed design is freezable on `static_design` evidence. Public counts, owner boundaries, SQLite/v17 non-ownership and the no-value public surface remain closed after the ARR algebra changes.
