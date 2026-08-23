# Issue 55 approval binding hardening evidence

Evidence date: 2026-08-24 (Asia/Shanghai)

## Review findings closed by this increment

- PR #130 compared the caller digest with the stored digest but did not
  recompute the binding from the stored immutable plan. Stored-row tampering
  could therefore extend expiry or alter preview metadata without invalidating
  admission.
- All Codex live read errors shared one private sentinel. A missing baseline
  followed by a newly malformed file could look unchanged.
- Proxy takeover uses a proxy-owned target projection that the first slice
  does not yet verify, so admitting it would produce false readback truth.
- The frozen wire fixture used `pending` and omitted the requested
  `cancelled`/`not_started` vocabulary.

## Fresh local evidence

| Gate | Result |
| --- | --- |
| `cargo test --locked change_plan --lib` | PASS: 23 passed |
| `cargo clippy --locked --lib --tests -- -D warnings` | PASS: no issues |
| `mise run check:contracts` | PASS |
| `mise run check` | PASS at the reviewed worktree state; Rust fmt/check/clippy/tests, task/docs/platform/release contracts, frontend typecheck/format/unit tests all exited 0 |

Replacement PR Required CI is recorded only after the final commit exists.

## Scope boundary

No schema version, ChangePlan table shape, Provider writer, command
registration, frontend code, SecretRef code, or generic adapter engine is
changed. Proxy-takeover apply remains explicitly unsupported in this first
slice rather than being misclassified as a successful normal projection.
