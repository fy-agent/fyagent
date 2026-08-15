# Issue #35 V6 working-tree audit receipt

Status: `REQUEST_CHANGES`. This is a static pre-commit audit receipt, not an authoritative rereview or freeze receipt.

Evidence boundary: `source_report + code_audit + static_design`. No dependency resolution, test, build, browser, renderer, server, native runtime or screenshot ran.

## Snapshot identity

All three reviewers re-hashed their assigned authority files at start and finish and reported `SNAPSHOT_DRIFT=NO`.

- `secret-contract-v1.md`: `8aa1fed639af104a0dea750fd1abfdfdb0b0c008ac9b0c80e75f381e5cfb95b8`
- `prd.md`: `11993e7c75072ee492cd1d41980542fd112e0ca8ec83beb5c09ee956a167e025`
- `design.md`: `7888bc8417e139f4c4d9b5a2254703f2c8baf40e2c280416e44b1ab0cbce4fec`
- `device-local-secret-store.md`: `e13631f3908cab4dc6a3199b67d8138c8bbe759db9e6ef88d1a6385c8e5983df`
- `technical-design-overview.md`: `df863d0c41d58f46165f4336d4804707ee99e61b03c3490b37c40ecae7e389a9`
- `detailed-design-overview.md`: `4e8bee1156af1bebc3128fd6b35dba78f46b37a10516503e2c72f65f1f04365a`
- `research/secret-surface-inventory.md`: `c437e94d0486abd31f130f2cb477b4de6b9b70d0ce401f779ec75435d14c6098`
- `research/codex-secret-call-graph.md`: `f6f8ae85934d2e0c3e678f66d84a8d522a2f6ffe2591c677d6fcce1063778f92`
- `research/secretRef-contract-handoff.md`: `698e93b9fe4a038e70794571d6ad5e18e35f21967b971513b53ff0cf4aa9b124`
- `research/native-evidence-plan.md`: `9321df93eb7f2454a97260485066e4950b819915f6fe7247da1abbef1602a23b`
- `research/runtime-preflight.md`: `62848518fafce39ad33040c10192ee0092cd61f7ec7235f7a929c00f472aa39d`

## Reviewer results

| Lane | Result | P0 | P1 | P2 | P3 |
| --- | --- | ---: | ---: | ---: | ---: |
| Product | `REQUEST_CHANGES` | 0 | 3 | 2 | 0 |
| Contract / architecture | `REQUEST_CHANGES` | 0 | 7 | 2 | 0 |
| Surface / native evidence | `REQUEST_CHANGES` | 0 | 4 | 2 | 0 |

## Required V7 closure

### Product

1. Provider delete must model binding and current legacy sources orthogonally; legacy blocks deletion without minting an impact id.
2. Device-local recovery state/CAS must mirror all four public recovery kinds, not activation cleanup only.
3. Cancelled/expired/terminated readiness actions must create a fresh impact/plan/staged flow, not replay the consumed operation id or self-loop.
4. Legacy and staged-import user actions require total TS/Rust destinations and exact executable flows.
5. Explicit discard failure needs the same immutable pending terminal disposition and reachable journal as expiry.

### Contract / architecture

1. Journal authority must be exactly eight operation-specific variants with independent phase/required-field shapes; no generic ninth recovery operation.
2. Staged import must bind durable temp-object id, process nonce, stage/owner/row revision and an opaque admission identity through backend scope and equality checks.
3. Staged prepare/confirm needs a consuming discard path plus exact crash reopen, old-admission reconciliation, fresh identity and recovery admission sequence.
4. Backend prepare/confirm/cancel/read/delete must be bound to the exact registered instance handle and generation.
5. Revocation must use a non-clone, consuming, full-CAS scope receipt after capability validation; source/time alone is insufficient.
6. Every authority document must use the same startup order: open store, no-backup DB preflight, same service reconcile, manage/register, Clean sanitized backup, publish gate, workers.
7. The closed source/owner/scanner map must include all current renderer/API/fixture value paths.
8. `SecretRefDisplay` must be output-only and must not implement Rust `Deserialize`.
9. Public DTO fields must not use the canonical forbidden key `credential`; structural state needs a scanner-safe name.

### Surface / native evidence

1. Add/Edit dialogs, Provider card, Codex feature/forms/editor/templates and usage API require exact main-integration ownership and token-free migration.
2. Codex deep-link helper/dialog/tests must reject before renderer decode/merge/preview.
3. `src-tauri/src/services/sync_protocol.rs` must enter the staged cutover owner map and gate.
4. `src-tauri/src/codex_history_migration.rs` must stop creating raw settings backup before the clean gate; old generations remain scan/report-only.
5. All enumerated existing Codex fixtures must enter the exact generator/baseline and use runtime canaries or token-free negative assertions without a test waiver.
6. Provider-delete preview drift uses Provider-owned `PROVIDER_DELETE_IMPACT_STALE + refreshProviderDeleteImpact + effect=none`; `refreshDeleteImpact` remains secret-delete-only.

V7 must receive three fresh independent working-tree audits. This receipt cannot be marked closed by the document owner and is never consumable by #55 or #41.
