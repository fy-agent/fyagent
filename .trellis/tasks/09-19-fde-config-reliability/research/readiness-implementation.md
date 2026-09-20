# R7 readiness implementation

## Approved boundary

Native readiness currently overwrites a positive CLI observation when installation inventory is multiple/unknown. Add readiness v5 `configurationEligibility { state, evidence }` as an independent native projection, computed before the inventory overlay. Retain inventory installation trust and target/action admission. Renderer consumes this closed field for navigation and explains preserved CLI observation with source uncertainty. No Health, real user files, lifecycle executors, or vendor configuration changes.

Files: agent_install/mod.rs and types.rs; shared readiness DTO/parser; directory projection/card; focused fixture/tests and owner specs. Existing compact surface shape is unchanged. v5 requires native and renderer shipped together; older payloads fail closed.

## Validation scope

Synthetic native positive/failed/absent/unavailable CLI observations against single/multiple/unknown inventory; strict parser missing/invalid/mismatched evidence; directory navigation with uncertain inventory; existing action admission/fixture regression; typecheck. No actual machine branch reproduction or real CLI invocation.

## Delivered behavior

- Readiness contract v5 adds required `configurationEligibility` with exact state/evidence shape. Native records CLI detected/runnable separately from inventory installation trust; multiple/unknown still clears the single installation version and preserves target selection rules.
- Renderer parser rejects missing/excess/unknown/source-mismatched evidence and v4 payloads. UI derives navigation only from native eligibility, shows CLI evidence plus inventory uncertainty, and does not dispatch an install/update merely by opening configuration.
- No lifecycle action admission, Health, config files, login, credentials, installed applications, or real CLI execution changed. Existing Tooling observation and native install target revalidation remain owners.
- Desktop eligibility reflects the existing native installation observation; configuration navigation remains separate from write permission. Compact surface payloads unchanged.

## Checks executed

- `rtk mise run test:unit -- tests/renderer/features/agent-install-readiness.test.ts tests/renderer/platform/agentInstallReadinessPort.test.ts tests/renderer/pages/agents tests/renderer/features/agent-lifecycle-capabilities.test.ts`: PASS, 13 files / 158 tests. This is renderer/fixture evidence, not native UAT. Includes uncertain-inventory navigation and fail-closed DTO cases.
- `rtk mise run typecheck`: first run PASS; later concurrent tree run initially blocked by MCP `featurePages.test.tsx:106` unsupported ByRoleOptions `exact`; root corrected this unrelated failure; final rerun PASS.
- Focused `rtk pnpm exec eslint` across changed source and renderer tests: PASS.
- Scoped rustfmt (skip_children=true), frontend format:files and `git diff --check`: PASS.
- `rtk mise run rust:test -- agent_install::tests`: build exited 101 before test execution due to concurrent MCP private `mcp::validation` reference. Root reported correction and requested unified Rust rerun rather than competing build jobs. New pure native tests are present but native results remain PENDING root verification.

## Root integration

Ship native and renderer readiness v5 together; v4 fails closed, no mixed-version fallback. No command/ACL/database changes. Root updates supported-platform review seal after all native files stabilize. Browser fixture readiness payloads updated, but browser/native runtime not executed. R1 owner's approved `editable: true` fixture additions in tests/browser/support/features.ts and tests/renderer/pages/agents/Page.test.tsx travel with this commit. Other worktree changes left untouched.

Remaining: root Rust directed tests and final check; original machine-specific Claude branch still unreproduced; no statement of physical/installed app acceptance.
