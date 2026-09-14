# Implementation plan

## Phase A — establish evidence

- [x] Capture branch status, six-commit range, changed-file inventory and any existing PR.
- [x] Review the two archived/related Trellis task sets and compare acceptance criteria with the actual commits.
- [x] Inspect all changed SPEC hunks, file sizes and semantic ownership; record findings in `research/spec-audit.md`.

## Phase B — improve SPEC ownership

- [x] Split `external-agent-lifecycle.md` into core lifecycle and product/source owner while preserving links and seven-section contracts.
- [x] Split supported-platform identity/snapshot governance out of `task-runner-contract.md`.
- [x] Split Agent-specific Windows helper/registry scenarios out of `windows-runtime-security.md`.
- [x] Update `backend/index.md` and affected cross-references; run link/context validation and keep every owner under 32768 bytes.
- [x] Compress duplicated Claude/Grok npm and Renderer concision prose without removing validation, error, security or evidence boundaries.

## Phase C — code and test audit

- [x] Trace live npm metadata/argv, PATH-default selection, npm 12 allow-scripts, Windows LocalProcess and post-install readback through code/tests.
- [x] Trace secondary-page copy/layout changes, React warning guard lifecycle, scroll ownership, performance config and repository snapshot tests.
- [x] Fix only demonstrated defects; add or strengthen regression assertions for every code fix. No product-code defect was demonstrated; the required corrections were SPEC ownership and routing.

## Phase D — local gates

- [x] `task.py validate` with curated context.
- [x] `mise run format:check` and `mise run check:contracts`.
- [x] `mise run check` (full repository gate).
- [x] Run focused browser/performance commands required by the changed owners, preserving real timing configuration.
- [x] Re-run the exact failing command after every repair and finish with a clean full-scope pass. No gate failed; focused and full-scope passes are recorded in `research/commit-audit.md`.

## Phase E — commit, PR and merge

- [ ] Review final diff and write coherent work commit(s) without amending the six baseline commits.
- [ ] Archive this Trellis task and record the session journal after work commits.
- [ ] Run canonical post-archive contracts/readback, freeze the exact reviewed head, and leave a clean worktree for merge handoff.

## Post-archive merge executor (outside task-closure evidence)

- Push the frozen `dev/laiyongjie` head and create/update a PR to `main`.
- Inspect every required PR check. A new fix commit invalidates the prior exact-head readiness and must repeat the applicable local/Trellis lifecycle before auto-merge is re-enabled.
- Enable auto-merge only with `--match-head-commit <exact-sha>`; never use `--admin`, direct `main` push, squash or rebase.
- Require the Merge Queue `merge_group` `CI / Required` authority, then read back the resulting remote `main` merge SHA.

## Stop/rollback gates

- A proposed fix that changes public behavior beyond the six-commit intent is out of scope.
- A platform check that cannot be proven on the current host remains explicit
  residual evidence; it is never converted into a passing mock claim.
- Do not merge with required checks pending, skipped unexpectedly or failing.
