# Admit hosted Linux in CI toolchain verifier

## Goal

Fix the hosted Ubuntu regression discovered by PR `fy-agent/fyagent#166`:
portable CI jobs reach `scripts/ci/verify-toolchain.mjs`, but its invocation
resolver currently rejects `process.platform === "linux"` before checking any
tool version.

## Requirements

- Reuse the repository's reviewed POSIX host predicate rather than introducing
  an open-ended "not Windows" fallback. The shared predicate must remain
  dependency-free because native CI invokes the verifier before `pnpm install`.
- Leave direct Node/pnpm/Rust/uv/Python invocation unchanged on approved POSIX
  hosts (`darwin`, `linux`).
- Preserve the Windows-only `pnpm.cmd`/`ComSpec` boundary and unsafe-token
  rejection exactly.
- Continue to reject unknown runtime platforms before launching a tool.
- Add executable coverage for Linux admission and unknown-host rejection.
- Refresh the supported-platform structure digest for every modified protected
  surface.
- Keep the fix inside the existing PR and do not change runner assignments,
  product behavior, or Release trust topology.

## Acceptance Criteria

- [x] `resolveToolInvocation` returns unchanged argv for macOS and Linux.
- [x] Windows batch invocation and unsafe-token tests remain green.
- [x] An unknown platform still fails closed.
- [x] Early CI toolchain admission does not load package dependencies before
      `pnpm install`.
- [x] Focused CI toolchain tests, supported-platform scan, Release CI aggregate,
      typecheck, and formatting pass locally.
- [x] PR `#166` is updated and the replacement full CI passes the migrated
      Ubuntu jobs and native Windows ARM64 matrix.

## Local evidence

- `mise run check` completed successfully: 172 JavaScript test files with
  1,542 passed / 1 skipped, the main Rust library with 2,907 passed / 5
  ignored, and all non-ignored Rust integration suites passed.
- The focused regression set completed with 168 passed / 1 skipped.
- `node scripts/tasks/release-check.mjs --ci` completed with 29 test files and
  498 passed tests.
- The supported-platform scanner admitted 2,278 current files.
- An isolated smoke test copied only the verifier, the dependency-free platform
  helper, and locked version manifests into a temporary tree without
  `node_modules`; the verifier emitted the expected Node and pnpm facts.

## Remote evidence

GitHub Actions run `33295082976` failed only because the Ubuntu `Frontend`,
`Repository Contracts`, and `Desktop Acceptance Contract` jobs reported:

```text
Unsupported CI runner platform: linux
```

The affected product tests completed successfully; the failure came from the
repository-owned toolchain verifier and its diagnostic aggregator.

Replacement run `33295990452` completed successfully for commit `287b9c0d`.
The migrated Repository Contracts, Frontend Checks, and Desktop Acceptance
Contract jobs passed on Ubuntu; both Windows native architecture jobs passed,
including ARM64 on `windows-11-vs2026-arm`; the macOS and Windows backend jobs,
Credential Manager native CRUD evidence, and the final `CI / Required`
aggregate also passed.
