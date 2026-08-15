# Issue #35 initial review disposition

Status: `WORKING_TREE_AUDIT_APPROVED`, immutable rereviews still pending. The V9 working-tree audit records three independent lanes on one final hash set with `P0=0/P1=0/P2=0`. This historical map does not itself close or approve the immutable design candidate; the next gate is to commit that exact authority as `D` and have all three reviewers reread the same commit. Evidence remains static design only; no test/build/runtime was run.

The stable-hash V6 and V7 audits are preserved as `REQUEST_CHANGES` history. `v9-working-tree-audit.md` is the independent zero-finding pre-commit receipt. No earlier row is retroactively rewritten, and no row becomes immutable-review authority until the same-SHA rereviews approve `D`.

## Product findings

| Finding | Revised authority |
| --- | --- |
| PR-001 scope/global claim | `prd.md` §3.3; call graph §10; scanner has `contract_schema`, `codex_feature_runtime`, inventory and global=`NOT_CLAIMED` |
| PR-002 #55/#41 sequence | PRD §§5.1–5.2; contract §§7–8; technical §7; handoff Consumer sequence; activation and apply are separate plan/lease operations |
| PR-003 capability material/lifecycle | contract §7.3–7.4; device-store §11.3; material-free, one-shot, all revision/sink/expiry recheck |
| PR-004 Agent ambiguity | PRD §3.2; contract §1; Agent is wire-reserved and runtime typed-reject |
| PR-005 existing binding + unknown | PRD §6.2; device-store §7.2; call graph §4.3; equality requires successful read + constant-time compare |
| PR-006 historical artifact authorization | PRD §7; device-store §8.6; v1 is permanently scan/report-only with no rewrite/delete authority or command |
| PR-007 revoked folded to missing | PRD §§4.2,5.5; contract stable state/error matrix; explicit source/time/action |
| PR-008 confirmation in stable summary | PRD §4.2; contract §§1,3; operation-only step/capability |
| PR-009 policy/backend lock | PRD §4.2; contract issue/action matrix; required `lockSource` |
| PR-010 shared rotate/lock CAS | PRD §4.4; contract §4; revision+digest+exact rows, not count |
| PR-011 migration DTO/recovery | draft contract/device-store define typed migration/report/command/error DTO plus discriminated recovery; closure remains subject to same-snapshot contract/product rereview |
| PR-012 hardware option ambiguity | PRD §§4.3,8; contract/handoff capability matrix; hidden without registered adapter |
| PR-013 GitHub authority | `research/issue-35-authority.md` exact IDs/times/digests/mapping |

## Architecture findings

| Finding | Revised authority |
| --- | --- |
| AR-001 crash journal | device-store §§5–7; intent precedes every OS mutation, exact phase recovery |
| AR-002 sync overwrites local refs | device-store §§3,8–9; immutable local-data root excluded from all snapshots |
| AR-003 hot import/restore bypass | device-store §8; staged temp DB before cutover, same AppState, no direct post-sync writer |
| AR-004 #55 secret-bearing digest | call graph §§1,6; handoff #55 delta; incompatible baselines explicitly recorded |
| AR-005 v17/shared ownership | device-store §§0,2; detailed §1; #35 no schema, Prompt/Memory retains v17, one owner per file |
| AR-006 capability binding | contract §§7.3–7.4; device-store §11.3; full record/binding/backend/device/capability CAS |
| AR-007 Provider boundary/call graph | call graph §§3–9; technical §8; typed public/mutation DTO and exact sources/consumers/sinks |
| AR-008 unsafe legacy scrub | PRD §6.2; device-store §7.2; exact equality/conflict/comparison-pending behavior |
| AR-009 destructive CAS | contract §4; device-store §4.3; exact binding-set rows/revisions |
| AR-010 ref/schema validation | contract §2/§6.1; device-store §4.1; validating UUIDv4 newtype, no SQL CHECK/table |
| AR-011 Windows capture feasibility | OS research; device-store §10.3–10.4; one CredUI family, flags/HWND/buffers/zeroization |
| AR-012 hardware singleton | contract/device-store hardware sections; per instance and record generations/capabilities |

## Detailed design findings

| Finding | Revised authority |
| --- | --- |
| DD-001 missing call graph/owners | call graph §§4–9; detailed §1; proxy/terminal/model-fetch/balance/failover/deeplink/import included |
| DD-002 incomplete TS/Rust contract | `secret-contract-v1.md` proposes a strict wire mirror/newtypes/unions/requests/results/envelopes; authoritative rereviewer must verify literal/null/unknown-field parity and no invalid combinations before closure |
| DD-003 backend read/capability contradiction | contract §7; explicit read/verify, material-free by-value capability and exact owner-private consuming writer/runtime executors |
| DD-004 native APIs/features/errors | OS research + device-store §10; direct SecurityFramework/Windows APIs, exact capture sequence/error mapping |
| DD-005 MSRV | OS research; direct deps keep 1.85, exact post-freeze lock/license/advisory/MSRV gate |
| DD-006 public projection/scope | PRD §3.3; call graph §§5,10; typed Provider/live/failover/universal projections and full Codex inventory |
| DD-007 startup/import/artifacts | device-store §8; detailed §8; exact startup and staged import/restore/sync/backup order |
| DD-008 write-ahead reconcile | device-store §§5–7; per phase deterministic recovery/fault plan |
| DD-009 schema/lifecycle/CAS | no #35 SQLite schema; device-store §4 split policy/retirement transitions and exact binding CAS |
| DD-010 AppState/thread model | device-store §§9.3,10–11; detailed §§2–3; same DB, injected deps, spawn_blocking/main-thread order |
| DD-011 error semantics/IDs | contract §§2,10–11; strict server IDs and complete state/retry/action/audit matrix |
| DD-012 V2 Tauri path | detailed §1.2/§9; only `shared/platform/tauri/credentials.ts`, exact tests/browser spec |
| DD-013 Windows evidence path | native evidence plan §§2–10; x64 mandatory, pre-evidence push, real/injected/UAT separation |
| DD-014 exact commands/source freeze | execution plan Phases 3–6; exact existing/new mise tasks and SHA/diff gates |
| DD-015 scanner baseline | PRD §3.3; call graph §10; detailed §11; four levels, generated canary, exact baseline/no broad waiver |

## Rereview rule

No row becomes CLOSED merely because this table points to a document. Reviewers must verify exact definitions and cross-file consistency on one immutable authority commit. Any reviewer-raised P0/P1/P2 returns the design to revision; `DESIGN_FREEZE` remains pending.
