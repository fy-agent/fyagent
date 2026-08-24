# Design

## Canonical Architecture

```text
V2 Models / Apply UI
  -> bounded FeaturePorts
  -> strict Tauri Change Plan commands
  -> ChangePlanService (orchestration)
  -> SQLite v20 local-only ledger
  -> existing Provider mutation guard
  -> existing ProviderService lock-held writer (single writer)
  -> DB / device / target / live readback
```

ProviderService remains the only configuration mutation owner. Change Plan owns identity,
admission, durable job state and readback-driven recovery. Renderer owns presentation only.

## Latest-main Integration

Use the existing PR #135 branch so review/provenance stays on the same public PR. Bring latest
`origin/main` into the branch conservatively rather than rewriting public history unless a
verified repository constraint requires otherwise. Conflict resolution is semantic:

- current main wins for #140 Models/Codex/WorkBuddy/OpenCode safety contracts;
- #135 supplies Change Plan v20 ledger/orchestration where main has no equivalent;
- overlapping specs/tests are rewritten to the combined current contract;
- supported-platform digest is refreshed only after source settles and its candidate set is
  revalidated.

## Persistence and Sync

- Schema v20 has one physical Change Plan shape.
- Fresh DB and v19 migration call the same idempotent table/index helper.
- Ledger tables are device-local: sync export skips data; sync import preserves receiver data.
- Old #130/#134 `proof_id/process_epoch_id` schema is not reintroduced. Future durable fields
  require a later explicit migration.

## Mutation, Readback and Recovery

- Plan creation may persist only credential-free ledger state; no Provider/live mutation and no
  network request.
- Apply receives `planId + planDigest`, reacquires Provider mutation lock, validates immutable
  plan/TTL/baselines/credential capability, consumes once, and calls the current lock-held writer
  at most once.
- Current #140 Provider/Codex writer semantics remain authoritative, including targeted patch and
  preservation of unrelated user config.
- After mutation, DB/device/target/live projections are reread. Mixed/unknown state is
  `recovery_required`.
- Recovery APIs can update durable observations from readback only; they never own writer
  capability and never replay side effects.

## IPC and UI

- Commands are transport wrappers around service owners.
- Apply payload remains narrow and does not carry a second mutable intent.
- Browser fallback is native-required, not fake business data.
- UI presents one preview/confirmation and backend-owned job state. Positive config result never
  implies real Agent/model usage.

## SPEC and Trellis Governance

SPEC is regenerated conceptually from final executable behavior, not chosen from merge sides.
Applicable backend/frontend docs must describe the same schema, writer, targeted-patch, recovery,
readback and V2 behavior that tests enforce.

Before main merge, all #135-owned Trellis tasks that are already completed/review must be brought
to a truthful terminal state and archived. The canonicalization task is archived only after final
checks and evidence are recorded. A branch with stale completed/review task directories is not
merge-ready.

## Rollback

- Work occurs in an isolated worktree on the PR branch; source checkout remains untouched.
- Before pushing, local branch history can be reset to remote PR head if integration fails.
- Once pushed, any bad integration is corrected on the PR branch; `main` is never directly
  mutated.
