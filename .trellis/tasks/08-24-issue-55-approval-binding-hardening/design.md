# Issue 55 approval binding hardening design

## Immutable approval binding

Build the plan object with an empty digest, then derive the per-plan approval
binding from every immutable public execution/preview field plus the random
`proofId`, `processEpochId`, baseline binding, and contract identity. On apply,
rebuild the same input from the stored row and reject `stale` before admission
if it differs from the stored digest. The caller-supplied digest remains the
first `invalid_digest` gate.

## Live baseline state

Classify Codex live state as `available`, `missing`, or `unavailable`.
`available` binds the canonical secret-bearing projection only in memory;
`missing` binds a process-private sentinel and is a valid absence baseline;
`unavailable` covers malformed or unreadable existing files and is never an
admissible baseline. Persist only the non-sensitive state enum inside the
secret-free baseline approval binding.

The existing Provider writer uses a different live projection under proxy
takeover. This slice cannot compare that projection authoritatively, so plan
creation rejects takeover mode instead of admitting a job that would produce
a false post-write mismatch.

## DTO compatibility

Add closed enum variants without implementing cancellation yet. New jobs use
`not_started` for untouched steps; `cancelled` is reserved for the executor
increment. No schema column or migration is required because statuses are
stored as strings/JSON and schema v20 is already the active version.
