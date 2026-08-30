# Local validation evidence

## Environment

- Worktree base: `origin/main` at `4e47aab51b272f819da31773b348a2ea0ed8dee2`
- Toolchain entry: `mise exec -- ...`
- Node.js: `24.19.0`
- pnpm: `10.12.3`
- Local host: macOS; hosted Ubuntu and native Windows ARM64 remain PR evidence

## Passed checks

### Focused workflow and toolchain contracts

```text
pnpm test:unit tests/windowsMsvcEnv.test.ts tests/systemCheck.test.ts \
  tests/hdiutilRetry.test.ts tests/ciWorkflow.test.ts \
  tests/githubWorkflowTriggers.test.ts
```

Result: 5 files, 47 tests passed.

### Focused Release and metadata contracts

```text
pnpm test:unit tests/releaseWorkflow.test.ts tests/releaseAssets.test.ts \
  tests/writePlatformMetadata.test.ts tests/localBuildBoundary.test.ts \
  tests/codexWindowsUserScopeContract.test.ts
```

Result: 5 files, 130 tests passed.

### Repository Release aggregate

```text
node scripts/tasks/release-check.mjs --ci
```

Result: passed, including version/changelog/lock/DEP0040/task-docs/NSIS/
supported-platform checks and 29 contract files with 494 tests.

### Frontend CI-equivalent unit discovery

```text
pnpm test:unit \
  --exclude tests/developmentEnvironment.test.ts \
  --exclude tests/miseTaskContract.test.ts \
  --exclude tests/systemCheck.test.ts \
  --exclude tests/taskDocs.test.ts
```

Result: 168 files, 1455 tests passed. Expected mocked-error and MSW warnings
were emitted by existing tests; there were no failures.

### Other portable job gates

```text
pnpm typecheck
pnpm test:i18n
node --throw-deprecation ./node_modules/vitest/vitest.mjs run tests/desktop-acceptance
node --throw-deprecation scripts/desktop-acceptance/verify-mock-contract.mjs
pnpm test:native-fetch
pnpm test:desktop:visual:preflight
pnpm format:check
node scripts/tasks/supported-platform-check.mjs
git diff --check
```

Results:

- TypeScript typecheck passed.
- Locale parity: 3 tests passed.
- Desktop mock acceptance: 7 tests passed and mock-only contract verified.
- Native Fetch/Tauri mock: 4 tests passed.
- Visual baseline preflight reported `ready-for-candidate-capture` without
  writes.
- Source formatting passed.
- Supported-platform scan passed for 2269 current files.
- Git whitespace validation passed.

### Canonical local gate

```text
mise run python:sync
mise run check
```

Result: passed after creating the worktree-local locked `.venv`. The complete
gate included:

- environment/toolchain ownership;
- TypeScript and source formatting;
- 172 frontend/unit test files with 1538 passed and 1 skipped;
- locale parity, desktop mock acceptance, and visual preflight;
- Rust formatting, workspace check, Clippy with warnings denied, and all
  non-ignored workspace tests (including 2907 core library tests and 40 user
  helper tests);
- mise task, maintained-document, lockfile, supported-platform, and version
  contracts;
- the local Release aggregate with 33 contract files, 577 passed and 1
  skipped, plus native-fetch verification.

The first `mise run check` invocation stopped before code validation because a
fresh worktree had no `.venv`; `mise run python:sync` created the locked local
environment, after which the canonical gate completed successfully.

## Evidence boundary

These local checks prove workflow structure, pure contracts, and the portable
test paths. They do not prove GitHub's Ubuntu image or the native
`windows-11-vs2026-arm` image executed successfully. The pull-request Actions
run is the authoritative hosted evidence for those runner assignments.

## Delivery

- Work branch: `ci/runner-platform-boundaries`
- Implementation commit: `aff8d57f`
- Pull request: `fy-agent/fyagent#166`
- Base branch: `main`

The pull-request checks remain the authoritative hosted evidence for the
migrated Ubuntu control plane and the explicit Windows ARM64 VS2026 runner.
