# Implementation Plan — Refine CI change classification and title policy

## 1. Establish regression evidence first

- [x] Update `tests/classifyChanges.test.ts` with representative historical
      path-set fixtures:
  - CI-authority / #144 style → Full CI;
  - frontend / #147 style → frontend (+ existing contract behavior where
    applicable);
  - frontend+backend / #148/#149 style → affected union;
  - docs/spec / #150 style → contracts/docs;
  - release-only / #151 style → contracts/release evidence, not Full CI;
  - mixed release+product paths → ordinary union.
- [x] Preserve tracked-path ownership, unknown-path failure, rename/copy, and
      malformed-input tests.
- [x] Update `tests/verifyCommitMessages.test.ts` so valid subjects and PR
      titles well beyond 72 and beyond a likely replacement 100/120 cap pass,
      while empty/non-conventional values still fail.

## 2. Refine the classifier

- [x] Replace broad `.github/**`, `scripts/release/**`, `scripts/tasks/**`, and
      release/CI-test Full CI coupling with typed path ownership.
- [x] Keep CI authority and global toolchain authority Full CI.
- [x] Map release authority to contracts/release evidence without new public
      domain unless technically necessary.
- [x] Preserve product-domain union and fail-closed unknown-path handling.
- [x] Add classifier diagnostic metadata/reasons without duplicating path
      ownership outside the classifier.

## 3. Surface classification diagnostics in Actions

- [x] Update `.github/workflows/ci.yml` to render the classifier/event plan into
      the step summary.
- [x] Ensure `workflow_dispatch` Full CI is explicitly reported as event-level
      forcing.
- [x] Keep all existing job outputs and `CI / Required` aggregation compatible.

## 4. Remove title/subject hard length limits

- [x] Remove `SUBJECT_MAX_LENGTH` and all 72-character validation branches from
      `scripts/ci/verify-commit-messages.mjs`.
- [x] Preserve Conventional Commit structure, merge/revert, and squash suffix
      behavior.
- [x] Search for and remove stale CI/spec assertions that encode the 72-char
      hard gate.

## 5. Correct misleading PR label ownership

- [x] Replace `.github/labeler.yml` blanket `tests/**` frontend mapping with
      proven frontend-owned test paths.
- [x] Add/update workflow/contract coverage if the labeler configuration is
      executable-test-covered in the repository.

## 6. Update maintained contract

- [x] Update `.trellis/spec/backend/github-ci-workflow.md` with typed
      control-plane ownership, release-only behavior, classifier diagnostics,
      and no repository-defined subject/PR-title maximum length.

## 7. Validation

Run targeted checks first:

```bash
mise exec -- pnpm vitest run tests/classifyChanges.test.ts tests/verifyCommitMessages.test.ts tests/ciWorkflow.test.ts tests/githubWorkflowTriggers.test.ts tests/requiredCiGate.test.ts
```

Run repository CI/release contracts applicable to the changed files using the
canonical task entrypoints discovered from `mise tasks` / current specs.

Then run the repository final required local check:

```bash
mise run check
```

Also verify policy residue:

```bash
rg -n "SUBJECT_MAX_LENGTH|exceeds 72|72-character subject cap|72 characters" scripts tests .github .trellis/spec
```

Expected result: no CI/PR/commit policy residue for the removed hard cap.

## 8. Review gates

- [x] Review classifier narrowing path-by-path; any ambiguous global path stays
      conservative rather than guessed narrow.
- [x] Review `CI / Required` dependency/result semantics for no topology drift.
- [x] Review classifier diagnostics for no duplicated workflow path policy.
- [x] Review changed-files labels independently from CI domains; labels must not
      become a second Required CI authority.
- [x] Review final diff for unrelated refactors/dependencies.

## Rollback points

- If a narrowed path lacks sufficient contract evidence, restore Full CI for
  that path family only.
- If classifier diagnostic schema changes break gate consumers, keep the stable
  report shape and render diagnostics from backward-compatible extra metadata
  or textual output.
- Do not roll back by weakening unknown-path failure or Required CI aggregation.
