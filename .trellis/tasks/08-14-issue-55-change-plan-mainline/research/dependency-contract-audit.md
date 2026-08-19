# Upstream #35 and downstream #41 contract audit

Evidence level: `local_metadata_readback + exact-SHA code_audit`. No tests,
builds, runtime actions, messages, or writes to either dependency lane.

## Upstream #35 SecretBackend

- Thread: `01a0004c-4068-7650-95ee-b805b3008d68`.
- Worktree: `/Users/serendipity/.codex/worktrees/issue-35-secret-backend`.
- Branch: `codex/issue-35-secret-backend`.
- Audited HEAD: `afc317a7be3883a4a438d48fad5fb8b6d1f1f5ab`.
- State at audit: Trellis `planning`; branch contains no contract commit beyond
  the baseline.

Stable intent is limited to an opaque, non-derivable `secretRef`, device-local
scope, fail-closed resolution, no secret value/hash in ordinary state, and safe
rotation ordering. Missing pieces include wire schema/version, resolver lease,
error codes, capability matrix, hardware confirmation, legacy migration, and a
committed handoff SHA.

Decision: Issue #55 may freeze an opaque port and invalidation semantics, but may
not invent the SecretBackend wire contract or claim real resolution acceptance
until #35 supplies an immutable handoff.

## Downstream #41 configuration apply

- Thread: `01a0004d-52f1-7a30-a137-730bd102c0a1`.
- Worktree: `/Users/serendipity/.codex/worktrees/issue41/fyagent`.
- Branch: `codex/issue-41-configuration-apply`.
- Audited HEAD: `afc317a7be3883a4a438d48fad5fb8b6d1f1f5ab`.
- State at audit: Trellis `planning`.

The old UCP contract is not yet consumable by #41 because it lacks the complete
schema, canonical digest vectors, plan-time affected resources, public plan read
API, secret invalid reasons, and immediate planned-job handshake. The current
blocking apply call also cannot support #41's visible backup/progress/cancel
boundary.

Required handoff to #41:

1. exact immutable SHA shared by Rust DTO, TS decoder, fixture, spec, and schema;
2. schema/canonical digest definition and fixed vectors;
3. baseline and affectedResources contract;
4. persistence and public read/discovery APIs;
5. invalid reason enum with zero-write behavior;
6. one-confirmation admission returning a planned snapshot immediately;
7. explicit ownership: #41 consumes the ledger/job and owns only its V2
   workspace/domain execution extensions.
