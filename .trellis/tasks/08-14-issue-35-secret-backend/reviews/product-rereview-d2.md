# Issue #35 D2 immutable product rereview

`PRODUCT_REREVIEW_D2=APPROVE`

- Authority commit: `a338ee18edad759c5507be6372af3813eff1f429`
- Branch: `codex/issue-35-secret-backend`
- Evidence: `static_design`
- Blocking findings: `P0=0, P1=0, P2=0`
- Decision: D2 is product-freezable. Its remaining `PENDING`/draft labels correctly require this rereview plus the separate freeze receipt; they do not represent an open product-contract decision.

## Scope and authority boundary

The rereview used only immutable `git show a338ee18edad759c5507be6372af3813eff1f429:<path>` blobs. It covered the Issue authority, PRD, product design, technical and detailed design, exact contract, device-local store, source/surface inventory, Codex call graph, downstream handoff, execution plan, OS-keyring decision, native evidence plan, and runtime preflight. Earlier D reviews and later working-tree files were not used as authority.

The review revalidated the complete product boundary: device-local `secretRef` plus OS keyring MVP; hardware hidden-until-registered and no-fallback behavior; Codex-first scope and adjacent debt; capture/migrate/replace/rotate/lock/delete/revoke/missing/permission/recovery flows; the no-value public boundary; #55/#41/#63 consumption; and truthful native/Windows evidence gates.

## ARR product-impact readback

- `ARR-001` strengthens candidate discard/expiry with independent `RecordDelete/Delete` and `RecordMissingReadback/Validate` one-shot slots plus the durable three-field delete checkpoint. The user still invokes the same `discardCandidate` flow and sees the same pending/terminal states. Fresh-read failure uses the existing `SECRET_READ_FAILED → discardCandidate` mapping. No command, public action, error literal, journal, or recovery kind is added.
- `ARR-002` retains `deleteDisposition + backendCompletedAt + deleteAppliedCas` across normal activation, crash-visible failure, and activation recovery. The fresh missing receipt and supersession/terminal transition are atomic, with `revokedAt=backendCompletedAt`. This removes ambiguous crash reconstruction without changing rotate/cleanup user decisions or the no-rollback rule.
- `ARR-003` binds staged resume CAS to the immutable journal `operationId` and exact cumulative five-phase algebra. The public request remains exactly `stageId + expectedResumeCas`; every result arm remains exactly `stageId + currentResumeCas + status + action + issue`. No candidate, owner, ref, summary, audit, path, locator, or material field is exposed.

ARR therefore changes internal authority and crash-proofing only. The product totals remain exactly 15 #35 commands plus one separate main-integration resume handler, 47 secret errors, 24 actions, five backend policy operations, eight journals, and four recovery kinds. The explicit slot count is 12, including 10 delete/missing slots; slots do not become new user operations.

## Key user-flow closure

1. Native capture or legacy reconcile writes and verifies only an unbound candidate. A token-free #55 activation plan and #41-held activation lease are required before exact binding CAS and approved current-source scrub.
2. Candidate activation never silently applies to the live target. After activation releases its lease, the bound owner enters a separate #55 apply plan and a fresh #41 prepare/confirm/lease/backup/writer/readback operation.
3. Proxy, usage/balance, Provider-primary coding-plan, and model fetch resolve only at owner-private final-send boundaries. Secret failure is network-free, redirect-free, failover-neutral, and has a deterministic action.
4. Rotation keeps the new binding after switch. Old-record delete and fresh-missing readback are independently authorized; uncertainty yields typed cleanup and never rolls back to the old credential.
5. Candidate discard/expiry remains reachable until delete, durable checkpoint, fresh missing readback, and terminal state all complete. A terminal expiry refreshes current summary before a wholly new capture/rotation authority is minted.
6. User delete, accidental missing, logical/backend lock, permission denial, central/device revoke, backend unavailable, and Provider owner detach remain distinct states and actions. Provider deletion retains the secret and is blocked without an impact id whenever any current or adjacent-blocked legacy source exists.
7. Startup/import stays fail-closed. Only complete eleven-domain no-value coverage can open the consumer gate; staged import follows admission → authority match → #35 prepare/confirm → cutover context, and stale/replayed resume CAS is zero-write.

## Downstream-consumable boundaries

- **#55 Change Plan:** consumes only closed no-value activation/apply/staged projections and owns admission, structural digest, comparison policy, role and sink planning. ARR adds no new #55 public request or material surface. The currently named #55 baseline remains an incompatible implementation input; a compatible immutable successor is required before integration/source freeze, not before D2 product freeze.
- **#41 Configuration Apply:** continues to own pre-confirmation, Provider lease, final baseline, structural backup, writer/readback, rollback, and activation-cleanup coordination. ARR-002 makes the old-record checkpoint complete; it does not move Provider lease or writer ownership into #35. A compatible immutable #41 successor remains a later integration gate.
- **#63 / main integration:** retains shared Provider/startup/import/proxy registration, the sole eleven-domain inventory bridge, and the separate staged-resume handler. ARR-003 changes only its internal resume preimage and phase evidence; the exact public five-field resume boundary and 15+1 registration split stay unchanged.
- **#35 core:** retains device-local authority, backend registry/broker, candidate/recovery state machines, and no-value contracts. ARR adds neither SQLite schema ownership nor a downstream writer.

## Out of scope and evidence truth

This approval is `static_design` only. It does not claim implementation, dependency resolution, tests, build, command registration, compatible downstream SHAs, source freeze, runtime, UAT, native macOS/Windows execution, merge, deployment, or production. Matching-host macOS and Windows Rust 1.85 locked all-targets evidence, real CRUD/capture/failure matrices, cleanup readback, and artifact scans remain later gates. Missing Windows x64 native/failure/UAT evidence still blocks `DONE`; it does not block this design freeze.

## D2 authority SHA-256 snapshot

| D2 path | SHA-256 |
| --- | --- |
| `.trellis/tasks/08-14-issue-35-secret-backend/research/issue-35-authority.md` | `365188c6be12092a5e535ba300800eb69918599dbeada824a7a033aecabc8f33` |
| `.trellis/tasks/08-14-issue-35-secret-backend/prd.md` | `1b1c957d414a4506618ba18a998bd9c2f032d529bfb10aca34edff55064da7fc` |
| `.trellis/tasks/08-14-issue-35-secret-backend/design.md` | `ec5a46de3c315f76160ad6426a1d7bd448afc14eb413c5cdc7fdf39c814619a2` |
| `.trellis/tasks/08-14-issue-35-secret-backend/technical-design-overview.md` | `2f5f13d006d3e20b50689e357438297dbac91e0e54e20f3f66be786c5f5fd69c` |
| `.trellis/tasks/08-14-issue-35-secret-backend/detailed-design-overview.md` | `ae4e768e1a2270600e1aa4fb95ed494b5f48aaf445a4147bc8afa7fb173124fe` |
| `.trellis/tasks/08-14-issue-35-secret-backend/secret-contract-v1.md` | `44da40384499df4e1936e12e7006cd89e5f0bc41e98343892df14c5e654e5041` |
| `.trellis/tasks/08-14-issue-35-secret-backend/device-local-secret-store.md` | `07fb3ea341a51ec92a5f50e1745fac1e3eb51037c0e173f5cea4cc4b06a62bb8` |
| `.trellis/tasks/08-14-issue-35-secret-backend/research/secret-surface-inventory.md` | `3d12125a5c279db01d44dbdf8210ebc2e9ce455af12e6d60202cb6b778736f11` |
| `.trellis/tasks/08-14-issue-35-secret-backend/research/codex-secret-call-graph.md` | `af66de4fa8fb83a1c565ff3902dcb4eca71b1a5ee791036e11b8bfebc3554ea1` |
| `.trellis/tasks/08-14-issue-35-secret-backend/research/secretRef-contract-handoff.md` | `13efb1342360b22f2852229c207b129d90493dd43ebe0dcf783960d32bc8ea62` |
| `.trellis/tasks/08-14-issue-35-secret-backend/execution-plan.md` | `6d476bf26010deb1548a4cc6fb8bec53bc93bada8a76f5bd4d7e3b5b1ad9deee` |
| `.trellis/tasks/08-14-issue-35-secret-backend/research/source-audit.md` | `600bdf7893ac9eb10aa7e3ab226c38be96f223aa07dd50024c08a5fa471e0f7f` |
| `.trellis/tasks/08-14-issue-35-secret-backend/research/os-keyring-options.md` | `c6a1e8cbbc6cd4691642e351a9ca6e8851347bfe32ba6d40b095a8fca644e4af` |
| `.trellis/tasks/08-14-issue-35-secret-backend/research/native-evidence-plan.md` | `127d75d5e31ada40cfd0dd6cef6a23d100d2bce21329df39257f738c491a9181` |
| `.trellis/tasks/08-14-issue-35-secret-backend/research/runtime-preflight.md` | `62848518fafce39ad33040c10192ee0092cd61f7ec7235f7a929c00f472aa39d` |
