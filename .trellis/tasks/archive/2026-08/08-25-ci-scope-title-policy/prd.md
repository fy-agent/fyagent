# Refine CI change classification and title policy

## Goal

Make FyAgent CI run the checks that correspond to the real risk surface of a
change without weakening the fail-closed Required CI contract, and remove the
repository-defined maximum-length gate for otherwise-valid Conventional Commit
subjects and pull-request titles.

The motivating regression is PR #151 / commit `f72e032`: release orchestration
and release-contract changes were classified as generic control-plane changes,
which forced every product domain even though no `src/**` or `src-tauri/**`
product code changed.

## Confirmed Facts

- PR and merge-group base/head selection is explicit and was not the cause of
  the over-broad run.
- `scripts/ci/classify-changes.mjs` currently treats broad `.github/**`,
  `scripts/release/**`, `scripts/tasks/**`, release/CI contract tests, and other
  control-plane paths as `forceFull=true`.
- `workflow_dispatch` is the explicit event-level Full CI path and must remain
  so.
- Unknown paths currently fail classification rather than being silently
  skipped; that fail-closed behavior must remain.
- `Repository Contracts` already runs release-oriented contract coverage via
  the repository task/release-check suite.
- `scripts/ci/verify-commit-messages.mjs` applies one 72-character hard limit
  to normal commit subjects and to PR titles because PR titles are validated by
  the same `validateCommitSubject()` helper.
- `.github/labeler.yml` currently maps every `tests/**` change to the
  `frontend` label, which mislabels release/CI-contract-only PRs.

## Requirements

### R1 — Preserve the existing Required CI topology

- Keep the always-created Required CI workflow and stable `CI / Required`
  aggregate.
- Keep change detection in the repository-owned classifier and job-level
  conditions; do not replace the topology with workflow-level path filtering.
- Keep `workflow_dispatch` as an explicit Full CI diagnostic path.

### R2 — Introduce typed control-plane impact without adding a new build system

- CI authority changes that can change which checks run or how Required CI is
  evaluated must continue to force every existing domain.
- Release authority changes must not force unrelated product domains solely
  because they are release infrastructure.
- Repository task/configuration paths must be classified according to their
  actual responsibility instead of receiving a blanket Full CI classification.
- Global toolchain changes may remain Full CI in this iteration.
- The classifier must remain the single path-ownership authority for Required
  CI.

### R3 — Release-only changes use release/contracts evidence, not Full CI

For a change set equivalent to PR #151—release workflow/scripts/release tests
and related specs, with no product-code changes—the classifier must request the
lightweight contracts/release validation path without automatically requesting
frontend, desktop, backend, or Windows-native product jobs.

Mixed changes must union normally. For example:

- release + frontend → contracts/release evidence + frontend;
- release + Rust/Tauri backend → contracts/release evidence + backend;
- release + Windows-native implementation → contracts/release evidence + the
  applicable backend/Windows-native domains.

### R4 — Keep fail-closed safety boundaries

- CI workflow/classifier/required-gate authority remains Full CI.
- Global toolchain authority remains Full CI for this iteration unless current
  executable evidence proves a narrower owner without ambiguity.
- Newly unowned repository paths must continue to make classification fail;
  they must not silently become no-op paths.
- Rename/copy/deletion ownership behavior must remain intact.

### R5 — Remove repository-defined title/subject maximum length

- Remove the 72-character hard cap from normal Conventional Commit subjects.
- Remove the same hard cap from pull-request titles.
- Do not replace 72 with another repository-defined maximum such as 100, 120,
  200, or 256.
- Preserve non-empty checks, Conventional Commit type/scope/description format,
  merge-subject handling, revert handling, and GitHub squash suffix handling.
- A long title that is structurally invalid must still fail for format, not for
  length.

### R6 — Improve classifier diagnostics

- The `Classify Changes` job must make the selected plan and the reason for a
  Full CI escalation inspectable in the Actions run summary.
- Diagnostics should expose changed paths, their classification/impact where
  practical, selected domains, `forceFull`, and any Full CI reason without
  creating a second path-ownership implementation in workflow YAML.

### R7 — Align PR labels with actual frontend ownership

- Remove the blanket `tests/** → frontend` label mapping.
- Keep frontend labels for actual frontend source/tooling/tests.
- CI/release-only contract tests must not receive `frontend` merely because
  they live under `tests/`.

### R8 — Preserve the contract in executable regression coverage and specs

- Add regression coverage for representative #144, #147, #148/#149, #150,
  and #151-style path sets.
- Add tests proving valid Conventional Commit subjects and PR titles well over
  the former 72-character limit pass.
- Update the maintained GitHub CI workflow contract to describe typed
  control-plane ownership, fail-closed behavior, diagnostics, and the absence
  of a repository-defined maximum title/subject length.

## Acceptance Criteria

- [x] A CI-authority fixture (`ci.yml`, classifier, or required-gate authority)
      still yields every domain with `forceFull=true`.
- [x] A PR-#151-style release-only fixture yields contracts/release evidence
      without frontend, desktop, backend, or Windows-native domains and without
      `forceFull`.
- [x] Frontend-only and frontend+backend fixtures retain their existing affected
      domain behavior.
- [x] Docs/spec-only fixtures retain lightweight contracts/docs behavior.
- [x] Mixed release+product fixtures union the relevant domains rather than
      becoming Full CI merely because release files are present.
- [x] All currently tracked repository paths remain owned, and unknown paths
      still make the classifier CLI fail closed.
- [x] Actions classification output includes an inspectable summary explaining
      the selected domains and any Full CI escalation.
- [x] A valid Conventional Commit subject longer than 72 characters passes.
- [x] A valid PR title longer than 72 characters passes.
- [x] A substantially longer valid subject/title (well beyond a replacement
      100/120-character cap) passes.
- [x] Invalid or empty commit/PR titles still fail for the existing structural
      rules.
- [x] No repository CI/spec policy retains `SUBJECT_MAX_LENGTH`, `exceeds 72`,
      or the documented `72-character subject cap` contract.
- [x] Release/CI-contract-only tests no longer imply a `frontend` PR label via
      blanket `tests/**` ownership.
- [x] Targeted CI/classifier/commit-convention/workflow/required-gate tests pass.
- [x] The repository's required final check passes before completion.

## Out of Scope

- Replacing the current CI architecture with Nx, Turborepo, Bazel, or another
  dependency-graph/build system.
- Splitting Required CI into many separately-required workflows.
- Introducing a new first-class `release` CI domain/job unless implementation
  evidence shows the existing contracts job cannot express the required
  validation safely.
- Relaxing Conventional Commit structure or allowed commit types.
- Removing external limits imposed by Git/GitHub themselves; this task removes
  FyAgent-owned hard caps only.
- Broad CI cost optimization unrelated to incorrect path ownership.

## Blocking Open Questions

None. The user approved the previously reviewed scope and explicitly requested
creation and execution of this Trellis task.
