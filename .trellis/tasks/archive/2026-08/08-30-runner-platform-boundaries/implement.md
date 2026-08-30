# Implementation plan: CI and Release runner boundaries

## 1. Freeze planning and task context

- Record official runner-label/migration evidence and the repository's prior
  hosted-image metadata decision.
- Set the task branch/scope and enter `in_progress` only after artifacts are
  complete.

## 2. Update runner ownership

- Move portable CI, push-policy, Labeler, and Release control-plane jobs to
  `ubuntu-24.04`.
- Keep native backend/build/sign/notarize jobs unchanged.
- Replace all Windows ARM64 matrix routes with
  `windows-11-vs2026-arm`.

## 3. Update Windows toolchain admission

- Export one Visual Studio `[17.0,19.0)` range from
  `windows-msvc-env.mjs`.
- Reuse it in `system-check.mjs` and update prerequisite guidance/tests.

## 4. Close Linux contract gaps

- Admit native Linux Bash in the existing fixture helpers.
- Run the NSIS verifier's static contract on Linux; keep native PowerShell 5.1
  validation Windows-only.

## 5. Update executable and maintained contracts

- Update CI/Release runner assertions, exact matrices, release target metadata,
  and host-boundary tests.
- Update CI, Release, development-environment, and task-runner specs.
- Preserve the explicit ban on `ImageOS`/`ImageVersion` metadata.

## 6. Validation

Run, in escalating order:

```text
pnpm test:unit tests/windowsMsvcEnv.test.ts tests/hdiutilRetry.test.ts
pnpm test:unit tests/ciWorkflow.test.ts tests/githubWorkflowTriggers.test.ts
pnpm test:unit tests/releaseWorkflow.test.ts tests/releaseAssets.test.ts tests/writePlatformMetadata.test.ts
pnpm test:unit tests/localBuildBoundary.test.ts tests/codexWindowsUserScopeContract.test.ts
node scripts/tasks/release-check.mjs --ci
pnpm typecheck
pnpm format:check
python ./.trellis/scripts/task.py validate 08-30-runner-platform-boundaries
```

If targeted suites reveal another Linux-only failure in an existing portable
test, fix the actual host-independent contract when it is within scope; do not
exclude the suite. Stop and reassess before changing native product behavior.

## 7. Review and delivery

- Run the Trellis check workflow and inspect the full diff for accidental
  native trust-boundary changes.
- Update specs with final executable behavior.
- Commit with a Conventional Commit subject, push the isolated branch, and
  create a PR to `main`.
- Inspect PR checks; hosted Windows ARM64 success is remote evidence, not a
  claim made from local tests.

## Rollback points

- Runner migration and ARM64 label changes are separable commits only if
  debugging requires it; the final PR should keep workflow/tests/specs atomic.
- Do not revert to containers, cross-builds, or relaxed architecture checks as
  a workaround for hosted runner failure.
