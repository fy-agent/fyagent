# Review Receipt — Grok architecture

- Review ID: `ses_fcfc8a1d2ffeM5dWuZSqU5NLbw`
- Reviewer / Model: OpenCode external reviewer / `vibekey/grok-4.6`
- Mode: `read_only code_audit` in a disposable product-only clone
- Base SHA: `e94307cd810d7c5157b3791da2a8d7ef6a01b8a7`
- Head SHA: staged product snapshot on the base SHA; commit not created yet
- Product diff digest: `cd38c07602e747179444641b6b77da9db1d70876f5507d2ccca3e21be421d018`
- Scope: architecture, Secret boundary, duplicate state machines, scope drift
- Verdict: **PASS**

## Findings

| ID | Severity | File/line | Evidence | Required action | Status |
|---|---|---|---|---|---|
| G-01 | P1 | Change Plan Secret admission | Unknown/managed/malformed credentials previously admitted fail-open | Require saved, extractable material or preserved strict login at plan and apply | fixed and re-reviewed |
| G-02 | P1 | Provider takeover projection | Preview and real hot-switch previously used different builders | Reuse the exact pure projection builder for normal/backup/live takeover | fixed and re-reviewed |
| G-03 | P1 | Apply stale-ready state | A consumed plan could remain confirmable in the renderer | Consume the local plan after every admitted outcome | fixed and re-reviewed |
| G-04 | P2 | Reconcile concurrency | Lock-external snapshots could fail CAS or duplicate terminal handling | Lock, reread by job ID and terminal-fast-return | fixed and re-reviewed |
| G-05 | P2 | `src-tauri/src/services/change_plan/service.rs` | An unchanged unresolved recovery appends one event on each `get/list` reconcile | Deduplicate unchanged recovery observations in a follow-up if polling is introduced | accepted follow-up; does not replay writer or block this PR |

Final re-review found no P0/P1. It confirmed schema/sync, zero-side-effect planning, single-writer admission, takeover parity, read-only recovery, strict IPC/ACL, seven canonical Agent IDs, and Grok/xAI locale coverage.

## Not verified

- Cargo/Vitest, runtime, hosted cross-platform CI, real Provider UAT and WebDAV device round-trip were not executed by Grok. They remain separate evidence.

## Scope drift

- No fake coordinator, Change Plan cancel, install executor/job/probe, SecretRef, or plaintext secret surface was found. Sync preservation is local-ledger policy, not a product Backup/Restore action.

## Final statement

- PASS at `code_audit` evidence level. The P2 ledger-growth note is non-blocking and must not be promoted to runtime or CI proof.
