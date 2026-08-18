# Issue #35 immutable architecture rereview

ARCHITECTURE_REREVIEW=REQUEST_CHANGES
DESIGN_AUTHORITY_SHA=f2f26b8b6b5aa4acf8bbd257cee9ee22713aebaf
P0=0
P1=3
P2=0
evidence=static_design

## Review basis

All authority below was read from the immutable commit with `git show f2f26b8b6b5aa4acf8bbd257cee9ee22713aebaf:<path>`. Working-tree review files and post-commit edits were excluded from authority. No test, build, dependency resolution, browser, server, native runtime or screenshot evidence was used.

## Findings

### ARR-001 — P1 — candidate discard/expiry has no representable fresh-missing authorization

Exact evidence:

- `device-local-secret-store.md@D:1267-1271` requires `discardCandidate` to consume a candidate-delete authorization, persist `backendApplied`, then consume an independent candidate-missing-readback authorization using the record's `Validate` policy before `missingReadbackVerified` and terminal state.
- `secret-contract-v1.md@D:8260-8288` maps `General::CandidateDelete` only to `Delete`; `secret-contract-v1.md@D:8693-8699` has no candidate-missing operation/slot.
- `secret-contract-v1.md@D:8336-8349` permits `AuthorizedBackendMissingReadback` only for activation old-record or the three recovery missing slots. A general candidate-discard scope is rejected as `dependency_changed`.
- `secret-contract-v1.md@D:9500-9538` makes a `BackendDeleteAppliedCas` mandatory for the only missing-readback wrapper, while `DiscardCandidateJournalPhase::BackendApplied` at `secret-contract-v1.md@D:11330-11342` carries no such typed checkpoint/authority.

Impact: the closed Rust authority algebra cannot implement the normative discard/expiry terminal path. An implementation must either skip the required fresh missing proof, reuse the delete authorization, or add an unreviewed scope/API. Hardware `Validate` confirmation for this path is likewise not representable, so terminal `discarded|expired` cannot be proven under the frozen contract.

Minimum closure: add one closed candidate-discard preparation algebra with distinct delete and missing-readback slots, two one-shot authorizations, a reservation fulfilled only by a durable delete-applied checkpoint, `Validate` mapping for the missing slot, and a matching `BackendDeleteAppliedCas` (or equally strong typed checkpoint) in the discard journal. Mirror the new slot/pending/result shape in the strict Rust/TS decoder and scanner allowlists without adding a sixth hardware operation or a fifth recovery kind.

### ARR-002 — P1 — activation old-record crash state drops the delete receipt provenance required for supersession

Exact evidence:

- `device-local-secret-store.md@D:284-298`, `:669-677`, and `:1341-1351` require crash-visible `oldRecordDeleteApplied` to retain `deleteDisposition + backendCompletedAt + BackendDeleteAppliedCas`; only a later fresh missing receipt may persist `supersededByRotation`, with `revokedAt` equal to the retained backend completion time.
- The contract's own recovery preimage at `secret-contract-v1.md@D:2302-2307` and invariant at `:2355` require that same delete receipt and timestamp.
- The canonical Rust journal arm at `secret-contract-v1.md@D:11311-11321` stores only `delete_applied_cas`.
- The canonical Rust recovery phase at `secret-contract-v1.md@D:12450-12456` also stores only the CAS, and its `RecoveryRequired` arm has no checkpoint payload capable of retaining disposition/time.

Impact: after a crash between durable delete and fresh missing readback, the strict Rust state cannot reconstruct the backend disposition or the authoritative `backendCompletedAt`. It therefore cannot truthfully mint the required terminal supersession/revocation timestamp, reproduce the specified recovery digest, or distinguish the exact pre-terminal checkpoint without out-of-contract side state.

Minimum closure: make both `ActivateCandidateJournalPhase::OldRecordDeleteApplied` and the activation-cleanup nonterminal/recovery checkpoint carry the closed `{deleteDisposition, backendCompletedAt, deleteAppliedCas}` record; make the strict codec and recovery preimage consume exactly those fields; preserve the rule that the subsequent missing receipt and terminal supersession commit atomically with no standalone empty-suffix phase.

### ARR-003 — P1 — staged-resume CAS preimage type cannot encode the frozen digest domain

Exact evidence:

- `device-local-secret-store.md@D:1548-1566` freezes the digest preimage with mandatory `operationId + phase` and exactly five checkpoint literals: `intent|sourcesScrubbed|cutoverCommitted|liveOwnerMinted|localBindingFinalized`; every phase/checkpoint/authority change increments revision and recomputes the digest.
- `secret-contract-v1.md@D:11203-11212` defines `StagedImportRecoveryCheckpoint` with only `SourcesScrubbed|CutoverCommitted|LiveOwnerMinted`.
- `secret-contract-v1.md@D:11216-11228` defines `StagedImportResumePreimageIdentity/Preimage` without `operationId` or `phase`, and cannot represent `intent` or `localBindingFinalized`.
- The journal nevertheless has `Intent` and `LocalBindingFinalized` at `secret-contract-v1.md@D:11379-11394`, so this is a direct mismatch inside the claimed canonical Rust shape.

Impact: the frozen Rust types cannot compute the specified resume CAS for the initial and post-binding checkpoints or bind it to operation/phase. The stale/replay guarantee and fresh-nonce/admission CAS invalidation therefore cannot be implemented without weakening the digest or inventing a second preimage contract.

Minimum closure: add the operation id and closed journal phase to the preimage identity, replace the three-arm checkpoint with an exact five-arm phase/checkpoint union, and make phase-specific receipt/promoted-owner rows structurally required/forbidden. Add one canonical digest fixture per phase and require revision change on every fresh nonce/admission or phase/checkpoint transition.

## Architecture boundaries otherwise confirmed

- Secret records, bindings, candidates, journals, audit and recovery remain device-local under `app_local_data_dir/device-local/secrets/v1`; #35 owns no SQLite schema or v17 transition and exports/sync never copy that authority.
- Durable `DeviceInstanceId` and process-local `DeviceSecretStoreInstanceId` are distinct; live handles retain both plus the exact registered backend `Arc`, and returned generations are checked before material or receipts leave the wrapper.
- One stateful `Arc<BackendOperationBroker>` owns exactly capture-intent, prepared-capability and pending-confirmation registries; production/test callers cannot inject or extract those registries.
- Capture intent is server-owned and atomically revalidated. Candidate activation and live apply use separate #55 plans and separate #41 leases; #35 never acquires the Provider lease and material reaches only the existing owner-private one-shot writer/runtime sink.
- Staged import authority ordering, source-value embargo before cutover context, startup same-service ordering, explicit `Revoke`, direct native API selection, hardware device binding and `silentFallback=false` are consistently frozen at the design level.
- Native/MSRV/lock/license/advisory, implementation, UAT and production evidence remain downstream gates. Current #55/#41 implementation SHAs are compatibility inputs for later source freeze, not evidence for this static design rereview.

## Immutable authority SHA-256 snapshot

| Authority path | SHA-256 |
| --- | --- |
| `.trellis/tasks/08-14-issue-35-secret-backend/secret-contract-v1.md` | `29a64c81554c205196140860a30def14835ac2e54f445ad5d739e214025369bf` |
| `.trellis/tasks/08-14-issue-35-secret-backend/device-local-secret-store.md` | `681947575da8a4a4ccad827a3aa3010bbc4cda828bab6e3c7a6a6124eff2ad7e` |
| `.trellis/tasks/08-14-issue-35-secret-backend/design.md` | `2fbcc56cbbbc5a61257c867e7c2dd3502e1518d00273d49fa5fb9fcf5bd71f05` |
| `.trellis/tasks/08-14-issue-35-secret-backend/technical-design-overview.md` | `21bedd66af5a5136125f9d654dabccdbeed8bf6ca6cf269638f836c8e70d6956` |
| `.trellis/tasks/08-14-issue-35-secret-backend/detailed-design-overview.md` | `40681c7b4e0d9522d56e11293275b2a4f309abe28a86483d9c8faa876c04d51c` |
| `.trellis/tasks/08-14-issue-35-secret-backend/execution-plan.md` | `3801eae08742d359a74dc211d011ce73dd8922a765750ec7113766945a647e9b` |
| `.trellis/tasks/08-14-issue-35-secret-backend/research/secretRef-contract-handoff.md` | `3c2f72d246d7df20c6167505d41c46e2003f624b00af013d561914e26f79d34f` |
| `.trellis/tasks/08-14-issue-35-secret-backend/research/codex-secret-call-graph.md` | `3ec8e7b67ff16a1b93e2af79857bd977e4ab3db4fcdcd9079c70fdc7ad8511b4` |
| `.trellis/tasks/08-14-issue-35-secret-backend/research/secret-surface-inventory.md` | `dfa299542460f3aa62fa353f4af575ac7e194c72aab6411a6f868d8c87743ea1` |
| `.trellis/tasks/08-14-issue-35-secret-backend/research/os-keyring-options.md` | `baf55f60c30d45cd8f9e83b8bcc06d1d8e5fec33b2ddca428ba308a32372fe1c` |
| `.trellis/tasks/08-14-issue-35-secret-backend/research/native-evidence-plan.md` | `dfc4f77fbf3079f7ec089546da3d980825a584d852c01f368ed175e65c5fcec4` |
| `.trellis/tasks/08-14-issue-35-secret-backend/research/issue-35-authority.md` | `365188c6be12092a5e535ba300800eb69918599dbeada824a7a033aecabc8f33` |
| `.trellis/tasks/08-14-issue-35-secret-backend/prd.md` | `1b1c957d414a4506618ba18a998bd9c2f032d529bfb10aca34edff55064da7fc` |

## Verdict

The high-level boundary and ownership decisions are coherent, but the three closed Rust authority shapes above cannot encode their own normative safety/recovery contracts. Architecture freeze is blocked at the exact authority commit until all three are closed and reread on a new immutable SHA.
