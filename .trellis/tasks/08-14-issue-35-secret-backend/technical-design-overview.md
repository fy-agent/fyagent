# Issue #35 technical design overview

## 0. Status and authority

- `DESIGN_FREEZE=PENDING`
- Evidence: `source_report + code_audit + static_design` only.
- Design review stage forbids dependency resolution, test, build, browser, renderer, server, native runtime and screenshot.

Normative order:

1. `prd.md` — product/scope truth.
2. `secret-contract-v1.md` — exact public TS/Rust contract, native-only traits/signatures, identifier validation, state/error/action/audit matrices.
3. `device-local-secret-store.md` — exact local schema, permission/durability/journal/reconcile, startup/import/restore and native platform design.
4. `research/codex-secret-call-graph.md` — complete Codex source→consumer→sink graph, single-writer map, #55/#41 compatibility and scanner inventory.
5. this overview and `detailed-design-overview.md` — architecture/integration index; they must not redefine a competing wire shape.

Any conflict is resolved by updating every affected authority file before review; no working-tree draft is consumable downstream.

## 1. Architecture

```text
V2 / legacy renderer (no material)
  -> typed secret lifecycle/readiness commands
  -> SecretService
       ├─ DeviceLocalSecretStore
       │    record/ref/binding/candidate/journal/audit/recovery
       ├─ NativeSecretCapture
       │    macOS NSSecureTextField | Windows CredUI
       ├─ SecretBackendRegistry
       │    macOS Keychain | Windows Credential Manager | future hardware
       └─ sealed native consumers
            #41 existing writer | proxy send | fixed usage/balance/model-fetch/coding-plan

#55 Change Plan
  -> candidate activation projection + atomic activation admission
  -> #41 activation prepare/confirm -> activation lease/baseline
  -> #35 pre-mutation compare + binding CAS/exact scrub -> release
  -> re-read bound owner -> separate apply projection/admission
  -> #41 apply prepare/confirm -> new lease/baseline/backup
  -> #35 one-shot resolve inside final writer/readback closure
```

SQLite is not the secret authority. #35 adds no schema version or secret table and withdraws all v17 ownership. Prompt/Memory retains its separately adjudicated v17. Provider rows contain token-free Provider configuration only; a read-time device-local join produces credential summaries.

## 2. Exact contract decisions

`secret-contract/v1` freezes:

- strict native-generated IDs and validating serde/TS decoders;
- runtime owner `provider/codex` only; Agent requests typed-reject;
- separate `SecretOwnerCredentialSummary` for legacy/unbound owner state and `SecretRefAggregate` for one ref with many bindings;
- stable availability without migration/confirmation; required `lockSource` and `revocation` views;
- full binding-set revision/digest/exact-row CAS;
- candidate lifecycle, `candidateEquality|explicitReplacement` comparison policy and immutable activation/staged-import/apply projections;
- exact public commands and envelopes with no arbitrary message/raw error;
- native-only backend, capture, prepare/confirm/resolve/activation APIs;
- complete stable error/action/audit matrix with a unique executable fresh-flow destination for each condition/source; a terminated readiness never repeats its consumed operation id;
- a native-owned, single-use `SecretCaptureIntent` registry: backend-option discovery snapshots owner/binding/legacy authority, while begin-capture accepts only the intent id and selected registered backend; renderer never constructs an authority expectation;
- typed staged-owner/import-cutover authority and scope-bound one-shot backend reads;
- output-only `SecretRefDisplay`, forbidden-key-safe public field names and closed Codex feature scope.

Public IPC has no `get/read/reveal/copy/export secret`, no `set_secret(value)`, and no renderer-callable resolve. `activate_candidate_from_change_plan`, `prepare_for_apply`, `confirm_for_apply` and `resolve_for_apply` remain native-only. Backend operation contexts have private fields/factories and consume the corresponding opaque plan admission, readiness claim, journal receipt, runtime binding or staged authority; no crate sibling can assemble a scope from scalar IDs. `SecretInternalError` likewise has private fields and one exhaustive checked factory deriving retry/action/effect from a closed context.

## 3. Device-local authority

Root: `<app.path().app_local_data_dir()>/device-local/secrets/v1/`.

```text
store.lock
state.json
journal/sop_<uuidv4>.json
audit/sae_<uuidv4>.json
```

`state.json` contains only non-material backend instance, record, binding, candidate, four-arm recovery, owner migration and managed-artifact summary data. Journal/audit forbid material, material digest, raw Provider/config, path and arbitrary error. All documents use strict bounds, unknown-field rejection, canonical payload hash, durable readback and one-process lifetime lock.

- macOS: 0700 dirs, 0600 files, owner/mode/no-follow/fstat checks, parent fsync.
- Windows: protected DACL for frozen interactive user SID + LocalSystem, no reparse point, owner/DACL readback, `LockFileEx`, write-through replace/readback.
- corruption, missing compiled truth in a non-empty root, lock failure or unknown object fails closed; no empty-state regeneration or silent rollback.
- device-local root never enters SQL export, binary backup, WebDAV/S3 snapshot or config-dir override.

### 3.1 Durable side effects

Every OS-store mutation has a durable material-free intent before the side effect. Exactly eight journals exist—`captureCandidate|migrateLegacy|rotateCandidate|activateCandidate|discardCandidate|deleteSecret|detachProviderOwner|stagedImport`—and each is a strict tagged variant with its own required authority fields and phase enum; there is no generic ninth recovery operation or optional field bag. Phase transitions are exact and restart-reconcilable without enumerating keyring entries. General recovery state/CAS is a separate four-arm tagged union—`activationCleanup|captureCompensation|deleteFinalization|ownerDetachFinalization`—whose kind-specific fields and steps mirror its journal pointer. Every recovery delete and its fresh missing readback are separately prepared/authorized and separated by a durable `backendApplied` checkpoint: this includes uncommitted-candidate compensation, admitted user delete, and activation/cleanup old-record retirement. `supersededByRotation` is minted only after missing readback. Candidate explicit discard/expiry remains a reachable discard journal with immutable pending disposition until backend absence is confirmed. `terminal` is durable before journal retirement, and audit append failure never repeats the backend mutation.

Durable envelopes use a strict `DeviceInstanceId=dev_*` namespace that survives restart. The process-local `DeviceSecretStoreInstanceId` is regenerated for every opened store, is neither persistent nor Serde/Clone/Debug, and only seals live scopes, pending state and receipts against replay into another process. Backend record handles validate both identities plus the exact registered `Arc` object; a journal never serializes the process nonce.

If binding activation is durable before Provider scrub, the public owner remains `bindingState=bound`; the ref aggregate is `availability=stale` with `SECRET_OPERATION_RECOVERY_REQUIRED`, and the candidate is `cleanupRequired` with action `completeRecovery`. Every Codex consumer remains blocked until `providerFinalized` is durable. Public redaction is defense, not a substitute for internal cleanup.

## 4. Platform backends and capture

### macOS

- direct `security-framework 3.7.0` plus its lock-compatible direct `security-framework-sys` and Core Foundation dependencies. Create-only uses a typed raw CFDictionary and `SecItemAdd` because `PasswordOptions` does not expose a public create-only dictionary and the public convenience helper updates duplicates; the dictionary includes `SecAccessControl` with `ProtectionMode::AccessibleWhenUnlockedThisDeviceOnly`, no user-presence flag and `kSecAttrSynchronizable=false`. Duplicate remains a stable write failure rather than update. Read/update/delete use exact service/account/non-sync queries, and find/delete matching never treats an access-control object as caller authority;
- service `com.fyagent.secrets.v1`, account validated `SecretRef`;
- write→fresh read→constant-time verify; delete→fresh missing read;
- `NSAlert + NSSecureTextField` on Tauri main thread, oneshot result, no Keychain/DB/file work inside UI closure.

### Windows

- existing `windows 0.61` plus `Win32_Security_Credentials`;
- `CRED_TYPE_GENERIC`, target `FyAgent/secret/v1/<ref>`, `CRED_PERSIST_LOCAL_MACHINE`, no enumerate/default/Enterprise fallback;
- `CredWriteW/ReadW/DeleteW/Free`, strict blob checks and volatile clear before free;
- only `CredUIPromptForCredentialsW` with non-null FyAgent HWND and `GENERIC_CREDENTIALS | ALWAYS_SHOW_UI | DO_NOT_PERSIST | EXCLUDE_CERTIFICATES | KEEP_USERNAME`; no WindowsCredentials/Unpack family mixing.

Both use `zeroize 1.8.2`. Raw OS errors never cross adapter boundary. Linux returns stable unavailable. Repository MSRV stays 1.85; exact dependency/license/advisory/MSRV validation occurs only after freeze.

## 5. Backend/hardware model

`SecretBackend` supports exact instance lookup, capabilities, write/read-verify/probe/prepare-confirm/delete/revoke and never selects fallback. Every durable record binds `DeviceInstanceId`; every live handle, scope, pending state and receipt also binds the same lifetime `DeviceSecretStoreInstanceId` plus a private `RegisteredBackendHandleBinding`. Equality requires both identities, `Arc::ptr_eq` and all scalar generations/CAS. Every raw platform read/delete/probe result carries the observed backend/device generations, and the registered wrapper validates them before material or a receipt can leave it. Every authorization and pending confirmation is minted and consumed by that exact handle. `BackendOperationBroker` is a real stateful object held as one `Arc` by `SecretService`; it privately owns capture-intent, capability and pending-confirmation registries and is the only path for mint/claim/fresh-revalidate/terminalize. `SecretMaterial` owns `Zeroizing<Vec<u8>>`, has no Serialize/Deserialize/Clone/ordinary Debug/public getter. Only a sealed, named consumer implementation may borrow it; the closed result cannot carry material, and static ownership audit limits any necessary transient copy to the exact authorized sink.

Hardware is per instance and per record, not a singleton Boolean. The capability snapshot binds backend/device/capability generations, five operation-confirmation policies (`captureVerify|validate|resolveForApply|delete|revoke`), consumers, sinks, residency, persistent projection and revocation. A fresh missing-readback uses the `validate` policy but always has its own authorization slot and post-delete durable checkpoint. No adapter is registered in MVP, so Add/Replace hides hardware; an existing record stays visible but unavailable and never falls back.

## 6. Candidate and lifecycle state

```text
native capture/reconcile
  -> durable intent -> backend write/read/verify
  -> verifiedPendingPlan candidate (binding unchanged)
  -> #55 immutable activation plan
  -> exact CAS activation + Provider scrub
  -> bound/ready

rotate
  -> verified new candidate (old binding unchanged)
  -> approved activation switches exact binding set
  -> if old delete pending: active new ref stale + candidate cleanupRequired + consumers blocked
  -> cleanup terminal: active new ref ready + old revoked tombstone

explicit discard / expiry
  -> one discardCandidate journal fixes pendingTerminalDisposition=discarded|expired
  -> delete/readback uncertainty: candidate remains verifiedPendingPlan + discardCandidate action
  -> confirmed missing + state commit: terminal discarded|expired and pending disposition disappears
```

Policy lock and retirement are independent fields, so unlock cannot resurrect stale/revoked records. Accidental backend absence is missing; intentional user delete and central/device revocation are revoked with source/time. Presence is always probe-derived and never persisted as authority.

Automatic one-value migration and scrub-only reconciliation use `candidateEquality`; explicit replace/reconcile/rotate use `explicitReplacement`, which still validates the exact approved source set/revisions but never requires the old value to equal the new candidate. Backend-option discovery mints the single-use capture intent containing that current policy/source/binding snapshot; begin-capture only claims it. A backend may assert central/device revocation only through explicit `observe_revocation_once` after consuming an exact Revoke authorization and validating `centralRevocation + SourceAndTime`; the resulting non-clone receipt binds the lifetime store, registered object and complete ref/store/record/binding-set/backend/device/capability snapshot. Ordinary read/probe returns only a non-persistable hint. Persistence fresh-revalidates and consumes the receipt. Missing, locked, denied and unavailable never imply revocation.

## 7. #55 / #41 integration

### #55

- uses only typed candidate/apply projection and stable readiness;
- hashes sanitized structural projection, never Provider/live bytes that contain material and never a material-derived digest;
- persists no capability or backend locator;
- delegates admitted apply to #41, not direct `ProviderService::switch`.

Known readback `ca552f4d`/`6859e9ce` is an incompatible baseline because it still digests secret-bearing Provider/live projections and directly writes. This does not block upstream #35 design freeze; after the immutable #35 handoff it blocks code integration until #55 publishes a compatible successor.

### #41

Canonical order is two distinct operations; an unbound candidate can never be smuggled into an apply projection:

1. #55 admits an immutable candidate-activation plan;
2. #41 performs activation-specific prepare and any old-record hardware confirmation before the lease;
3. #41 acquires the per-app Provider lease, rechecks the activation admission/final baseline, then calls #35;
4. #35 fresh-resolves the complete source set and revisions, fresh-reads candidate/backend material, constant-time compares before mutation, then performs local CAS and exact Provider scrub through the already-held lease-bound context; it never writes the live target;
5. #41 releases the activation lease; only the now-bound owner can enter live-apply readiness;
6. #55 creates a separate immutable `SecretApplyPlanProjection` for target and optional rollback;
7. #41 prepares target/rollback capabilities and completes optional confirmation before acquiring a new Provider lease;
8. under that lease it rechecks the apply plan/final baseline, creates the sanitized structural backup and consumes the target capability immediately before first live-target mutation;
9. inside one owner-private sealed executor it writes the exact sink and performs readback; only a typed non-sensitive result exits;
10. rollback uses a separately prepared capability; restart requires fresh preparation.

The capability is material-free, single-use, native-only and bound to plan/operation/owner/ref/record/binding set/backend/device/capability/consumer/sink/expiry. Its registry id stays private: only the operation-owned bundle can atomically claim a role for revalidation and then move that exact slot, or terminally discard it. Callers cannot borrow an id, reorder claim/take-role or widen field visibility. Any drift fails before material acquisition and mutation. #41's eventual compatible implementation SHA blocks integrated source freeze, not #35 design freeze.

For the Codex v1 apply surface, `CodexLiveSecretSinkId` is a closed, path-free enum with exactly `codexAuthJsonOpenAiApiKey` and `codexConfigTomlExperimentalBearerToken`. Every target/rollback credential projection carries exactly one `liveSinkId`; that same ID is part of #55 admission digest, #41 writer construction/readback and final baseline. The first ID addresses only the API-key slot in `auth.json`; the second addresses only the bearer-token slot in `config.toml`. Unknown IDs, OAuth fields and any path string are rejected before backup or lease mutation.

## 8. Provider and Codex boundaries

Internal `Provider` remains native DB mechanics; public/mutation types are separate. Codex public signatures cannot return internal `Provider`, `UniversalProvider` or raw `serde_json::Value`. All legacy and inactive/inline TOML locations are structurally inventoried before scrub.

The normative call graph decisions are:

- public list/get/live/failover/universal DTO: sanitized typed projection;
- Add/Edit/Provider-list/card/feature/form/editor/template plus shared types/schema/query/sort/MSW paths: token-free draft/public/mutation DTO only; no internal `settingsConfig`, shared API-key input or empty `OPENAI_API_KEY` substitute;
- new Codex inline credential mutation/deep link: native reject before renderer decode, merge, event or preview and before side effect;
- Codex `OPENAI_*` env inspection returns only name/presence/source category and creates no plaintext env backup; process env, HKCU/HKLM and shell files enter the registered source inventory;
- Codex common-config rejects new secret-bearing TOML and exposes existing JSON/bak/migrated, SQLite setting, localStorage/live occurrences only as blocked no-value legacy sources;
- one canonical inventory separates current-scrubbable `LegacySourceRef` occurrences from adjacent-blocked env/common-config observations and seals the complete scan in a `pub(crate)`-nameable but field-private/non-forgeable no-value coverage receipt. A named main-integration bridge alone mints it by consuming private full-inventory authority. The receipt binds a non-value-derived inventory revision plus exact 11-domain structural revision/presence/count identity; startup Clean, capture/owner projection and Provider-delete legacy blocking each fresh-revalidate through that bridge. An empty observation set without all-domain proof is incomplete and blocked; no receipt field contains value, path, locator or value-derived digest;
- arbitrary Codex request header/body override is rejected/fail-closed; primary material never enters shared HeaderMap/raw transport buffers;
- proxy: per-attempt resolve at final send; secret failure stops failover;
- usage/balance/model-fetch and the primary coding-plan API-key path: fixed native consumers only; coding-plan uses exact `codingPlanUsageProbe` plus a closed `CodingPlanPrimaryAdapter`, while generic `UsageProbeKind` remains only `Usage|Balance`;
- every credential-bearing proxy/usage/balance/model-fetch/primary-coding-plan request uses a dedicated client with `redirect::Policy::none()`; a 3xx is a stable upstream result, never a second request or Authorization-forwarding hop;
- generic secret-bearing usage script and Provider terminal: stable reject, effect none;
- takeover/backup/restore and Codex history/template migration: structural placeholders; OAuth preserved, never copied; no raw settings backup before the clean gate;
- import/restore/sync, including the shared sync-protocol cutover: staged temp-DB preflight before main-DB/Skills mutation; no new AppState and no best-effort Codex post-sync writer;
- stream check/proxy health/logs/cache/tray/crash/diagnostics/audit: stable typed metadata only; no raw URL, OS/network/upstream error/body. Codex MCP env/http-header material remains a named Level-3 adjacent domain, never Provider-primary PASS.

## 9. Startup and restore order

```text
SecretBootstrap::open -> OpenedDeviceLocalSecretStore + lifetime lock
  -> DB preflight without automatic backup (no #35 migration)
  -> one AppState consumes the same opened handle and retains Arc<Database>
  -> same SecretService journal/legacy reconcile + supplemental coverage receipt and token-free Provider gate
  -> app.manage + static registration receipt for 15 #35 commands plus the separate main-integration staged-resume handler
  -> Clean: first sanitized backup
  -> publish consumer gate
  -> start sync workers and Codex consumers last
  -> Blocked: publish scrubbed blocker, no backup/workers/consumers
```

Manual SQL import, binary restore and sync download use this one sequence: temp object scan/mint `StagedSecretOwnerToken` and projection → #55 staged admission → main-integration authority-match receipt → #35 prepare/confirm → already-held `ImportCutoverCoordinatorContext` → structural scrub/readback/cutover → DAO live owner mint/binding finalize. The token binds `tempDatabaseDurableObjectId + fresh process nonce + stage/owner/row revision`. Prepared cancel/discard terminally consumes both backend state and admission. Public crash resume accepts only `stageId + expectedResumeCas{revision,digest}` and returns a distinct closed resume DTO; every terminal/recovery/stale/replay arm includes the same-shaped `currentResumeCas` and no candidate, owner, ref or summary. The complete object/process/admission/record/backend/checkpoint/cutover/live-owner tuple is internal digest input. Reopen reconciles the old admission, mints a fresh process identity/recovery admission and new CAS; stale/replay is zero-write. A staged token can never authorize current-owner readiness/runtime. Remote data cannot replace local ref/binding/backend/journal/audit. Locked/denied/unavailable migration may expose scrubbed UI state but blocks Codex consumers and material-bearing export/backup.

## 10. Scanner and evidence truth

Four levels are separate:

1. `contract_schema`: strict PASS for DTO/command/event/fixture forbidden fields.
2. `codex_feature_runtime`: strict PASS on the enumerated current Codex artifact set with one generated canary, exact allowed sinks and approved current-source scrub; historical artifacts remain report-only.
3. `repository_static_inventory`: baseline/no-regression for adjacent credential debt.
4. `repository_runtime_global`: `NOT_CLAIMED`.

No blanket path/regex allowlist. Inactive, inline, legacy, test-only, hashed and backend-only are not Codex exclusions. The sole manifest enum is `research/native-evidence-plan.md` §9.1: `source_report | code_audit | ci_compile | unit_test | integration_test | native_contract | native_runtime | failure_path | uat | runtime_screenshot | artifact_scan`. Every item has exactly one class; a failure item separately requires `evidence_origin=real_os|fault_injection`. A final human report may present only the requested evidence subset and may display `UAT`, but it does not redefine the manifest enum or invent a composite class.

## 11. Construction and test safety

`SecretBootstrap::open(&AppHandle)` is the only production root resolver and returns a non-cloneable `OpenedDeviceLocalSecretStore` before DB open. Its DB-preflight authority is a field-private, non-clone `pub(crate)` opaque token (or an equivalent borrow method on the opened handle), so the sibling `store/database` call is legal Rust without exposing a root/path. `AppState::new_production(db, app_handle, opened_store)` consumes that exact handle into `SecretService.store_lifetime`; the service Arc held by AppState therefore retains the lifetime lock and cannot reopen from a path. AppState also retains one `Arc<Database>`. The service is built from device-local authority, mutation gate and backend/capture/clock/id dependencies; it does not own or acquire the Provider database/lease. #41/main-integration coordinators receive managed objects and construct opaque already-held Provider/import contexts. Runtime import/sync reuses that same AppState. Ordinary unit/integration tests use only `AppStateBuilder::new()` plus optional `with_database(Arc<Database>)` and a closed in-memory/fault mode; real keyring tests are ignored and require explicit `FYAGENT_NATIVE_SECRET_TEST=1`, with an additional interactive capture UAT gate.

Provider deletion first reads binding and current legacy state independently. Any legacy occurrence returns a material-free blocked preview with no impact id, source count/categories and `resolveLegacyConflict`; that action enters the native capture-intent flow rather than dead guidance. Deletion remains effect-none. Only no-legacy bound/unbound previews can mint confirmation authority. The bound preview says the backend secret is retained, lists remaining owners/orphan state and explains that backend deletion is separate. Post-Provider-commit crashes use `ownerDetachFinalization`; they never infer authorization from a missing row. Provider-preview drift uses the Provider-owned `refreshProviderDeleteImpact`, not secret-delete `refreshDeleteImpact`.

Dialog work runs on main thread without locks. Device-store/platform I/O runs in `spawn_blocking`. Lock order is frozen in `device-local-secret-store.md`; no code holding Database/Provider lease may call a UI dialog, and secret mutation never acquires Provider lease in reverse order.

## 12. Design-freeze condition

Freeze requires every authority file above to agree, then product/architecture/detailed reviewers re-read one immutable design commit and record P0=0/P1=0/P2=0. The separate freeze receipt names that authority SHA and digests. Only then is `research/secretRef-contract-handoff.md` consumable by #55/#41.
