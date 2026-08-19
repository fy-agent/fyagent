# Prior Feature Wave Practice Applied to This Task

## Sources

- Source task: `.trellis/tasks/08-12-prompt-memory-frontend-refactor/`
- Original optimized taskbook:
  `/Users/serendipity/fyagent/.trellis/tasks/08-13-prompt-memory-feature-wave-skill/research/optimized-thread-prompt-verbatim.md`
- Eighteen-commit integration retrospective:
  `/Users/serendipity/fyagent/.trellis/tasks/08-13-prompt-memory-feature-wave-skill/research/commit-retrospective.md`
- Source completion SHA: `e252dc5e`
- Later integration baseline: `ac660b5e`

The local `trellis mem` executable is unavailable. The reference package records
that the prior review used the same underlying Codex JSONL sessions and preserves
the original taskbook verbatim; this task consumes the durable reference files
rather than reconstructing instructions from a summary.

## Practices carried forward

1. Freeze an immutable code baseline and do not work in the user’s dirty checkout.
2. Compile natural-language intent into scope, code map, protected paths, evidence
   order, and rollback boundaries.
3. Model the conflict budget in advance. Use parallel owners only where business
   files are exclusive; use one sequential owner when shared registration and
   contracts dominate.
4. Commit design freeze before product code, then keep backend contract, plan,
   apply/readback, shared UI, entry migration, and final evidence separate.
5. Run focused module tests first. Do not use an early full suite to hide incomplete
   modules.
6. Treat generated/runtime evidence as stale after any source change; freeze source
   before final evidence.
7. Audit the cumulative branch diff and protected paths, not only the last commit.
8. Integrate by immutable SHA and known shared-file decisions, not moving branch
   names or title-based assumptions.
9. Keep evidence levels honest: plan/test/browser/local readback do not prove real
   Agent usage or production deployment.

## Adaptation for Change Plan

Prompt/Memory had three naturally exclusive frontend directories, so parallel
implementation was safe. Change Plan’s first vertical slice shares Rust DTOs,
SQLite schema/DAO, Tauri registration, Provider writer seams, TypeScript DTOs, and
the production switch entry. The reusable rule therefore selects a single owner
and sequential commits rather than copying the old three-agent topology.

The expected protected/conflict surface is explicit before implementation:

- protected V2/Prompt/Memory/WorkBuddy/Profile/Workspace Pack paths
- shared database schema and command registration owned only by this task
- existing Provider writer reused, not rewritten
- future Native Integration schema version reserved rather than silently collided
