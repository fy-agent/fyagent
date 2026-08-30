# Implementation plan

1. Update workflow/static tests for the portable/native runner mapping, the
   explicit ARM64 VS2026 label, bounded VS discovery, and platform metadata v3.
2. Move portable jobs in CI, push policy, Labeler, and Release control plane to
   `ubuntu-24.04`; make fake/static contract paths Linux-compatible.
3. Replace every active ARM64 runner request with
   `windows-11-vs2026-arm` and support VS 2022/2026 plus architecture-specific
   components in `windows-msvc-env.mjs` and `system-check.mjs`.
4. Collect Windows VS/MSVC facts, update strict metadata validation/types, and
   preserve rejection of ambient hosted-image variables.
5. Update CI, Release, Development Environment, and Task Runner specs; refresh
   repository-owned supported-platform inventories.
6. Run targeted tests, `pnpm typecheck`, `pnpm format:check`, supported-platform
   validation, `release-check --ci`, task validation, and the canonical local
   full check where applicable.
7. Review the complete diff, commit, push, create a PR to `main`, and inspect
   initial checks for repository-caused failures.

## Rollback points

- Runner label/workflow changes are reversible independently from metadata v3.
- Metadata schema and validator/type changes must land atomically.
- Do not push if native trust topology, permissions, or publication semantics
  drift beyond the stated runner assignment.
