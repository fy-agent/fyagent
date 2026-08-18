# Issue #35 V7 working-tree audit receipt

Status: `REQUEST_CHANGES`. This is a static pre-commit audit receipt, not an authoritative rereview or freeze receipt.

Evidence boundary: `source_report + code_audit + static_design + lock-local API/MSRV inspection`. No dependency resolution, test, build, browser, renderer, server, native runtime, UAT or screenshot ran.

## Snapshot identity

All three independent reviewers re-hashed the thirteen assigned candidate files at start and finish and reported `HASH_DRIFT=NO`.

- `secret-contract-v1.md`: `c88c0577158327073fb97d6ddcb412d67a73262a6e85702203ef88935a4c6b2e`
- `device-local-secret-store.md`: `3f11a656939f6b73440e8d174a7491a920db42121885bd6c2cb56943d5b83bfc`
- `prd.md`: `5064134649f394be73dc48801ba3af0e25d6c6038db6eefdc3003642a2146e6d`
- `design.md`: `75286cc02ceb0bef025ea3b3ee22acc555aa020a81240878f22d4e9cc6cb27c6`
- `technical-design-overview.md`: `8ece2ef9bcc3e8b0c9714bbfef52b8a68698972d18a45d15d4a4ce35b7d28bf0`
- `detailed-design-overview.md`: `2063896977396303bd7712c07b6883beb4f6772fd896721ea5073a6307530dce`
- `execution-plan.md`: `4fc21b4cc8017ada58cd51db99405c2f8d42fce93ca4edbc9478ede2a579769c`
- `research/secret-surface-inventory.md`: `1344ccfe17a3bfe705f36c8f827d071067fc51a447b6e988642e859f929031f2`
- `research/codex-secret-call-graph.md`: `f8d3c1925d39c0201b5f945db104a08e0dec0bd21a1a574cf6b5311b13a34069`
- `research/secretRef-contract-handoff.md`: `8f6b0a2182744daf519ada32ddf2e4dfcd37ad11765ec4fb3804295dee673c1e`
- `research/native-evidence-plan.md`: `9321df93eb7f2454a97260485066e4950b819915f6fe7247da1abbef1602a23b`
- `research/runtime-preflight.md`: `62848518fafce39ad33040c10192ee0092cd61f7ec7235f7a929c00f472aa39d`
- `research/source-audit.md`: `fb1c660d004c7a010ef5d5fd5459c3149261fd0433e8368331e5cd5d46762898`

## Reviewer results

| Lane | Result | P0 | P1 | P2 | P3 |
| --- | --- | ---: | ---: | ---: | ---: |
| Product | `REQUEST_CHANGES` | 0 | 1 | 2 | 0 |
| Contract / architecture | `REQUEST_CHANGES` | 0 | 10 | 2 | 0 |
| Surface / native evidence | `REQUEST_CHANGES` | 0 | 5 | 1 | 0 |
| **Total** | `REQUEST_CHANGES` | **0** | **16** | **5** | **0** |

## Required next-candidate closure

### Product and executable user flows

1. Use one staged-import resume request/result: `stageId + expectedResumeCas{revision,digest}`; the structured identity is internal digest preimage only.
2. Terminal candidate expiry must have one truthful fresh route. It may not claim direct `retryCapture` while the wire only supports `refreshSummary`; the owner-card path must mint new authority and never reuse the terminal candidate/operation.
3. `resolveLegacyConflict` must be a typed capture/reconcile flow with owner, current legacy snapshot and binding authority, not dead external guidance.

### Contract and architecture

1. Use one staged sequence: temp authority/projection → #55 admission → authority-match receipt → #35 prepare/confirm → cutover context.
2. Add the missing admitted-delete execution slot for `deleteFinalization`.
3. Keep capture-compensation delete and fresh missing readback as separately authorized actions with a durable checkpoint between them.
4. Give activation and activation-recovery old-record cleanup a separately authorized fresh missing readback; supersession becomes terminal only afterward.
5. Mint persistent central/device revocation only through an explicit consuming `Revoke` authorization. Ordinary read/probe may return a non-persistable hint only.
6. Bind every backend record/scope/receipt to the lifetime device-store instance and exact registered backend object; validate returned backend/device generations before data or receipts leave the wrapper.
7. Encapsulate capability registry claim/discard so callers never need a private capability ID and cannot reorder claim versus role extraction.
8. Make every backend operation context private to the operation broker and require its opaque admission/readiness/journal/runtime/staged authority.
9. Make the bootstrap token a legally usable opaque sibling-module type or hide it behind the opened-store method.
10. Remove commandless generic `retry`; every retryable condition needs one closed executable route or `none`.
11. Make `SecretInternalError` fields private and derive the complete tuple only through exhaustive checked factories.

### Surface and evidence

1. Add Codex `OPENAI_*` environment detection/removal/backup/restore to the no-value owner map; no value IPC/UI or plaintext env backup.
2. Block and migrate secret-bearing Codex common-config TOML across legacy JSON/bak/migrated, SQLite settings, IPC, localStorage and live merge.
3. Add the complete public Provider chain: shared types/schema/query/list/sort/MSW/update fixtures; Codex uses token-free public/mutation DTOs and disconnects shared API-key input.
4. Reject Codex arbitrary request header/body overrides in the primary credential path and cover the shared raw HTTP transport copy.
5. Replace stream-check/proxy raw URL/error/body persistence and UI projection with closed diagnostic status; cover reflected canaries.
6. Register Codex MCP env/header credential material as exact Level-3 adjacent debt, replace static secret fixtures and enforce no-regression without counting it in Provider-primary Level-2 PASS.

## Gate

This V7 receipt is immutable revision input. It cannot be marked closed by its document owner and cannot be consumed by #55/#41/#63. A new candidate requires three fresh independent audits on one new stable hash set. `DESIGN_FREEZE` remains pending.
