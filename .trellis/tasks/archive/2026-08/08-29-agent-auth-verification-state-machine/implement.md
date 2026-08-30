# Implement — Stage 4

## 1. Preflight/spec

- [x] Recheck official Claude/OpenCode/Grok CLI references at implementation time and freeze the reviewed output/exit-code contracts without guessing a minimum version.
- [x] Review existing Auth Center/Codex OAuth ownership and Tooling terminal/command APIs.
- [x] Update external-agent, Windows-runtime, and frontend specs to distinguish handoff, verification and provider-owned Auth.

## 2. Backend domain

- [x] Add Auth observation discriminated union and closed capability/reason enums.
- [x] Add Auth session DTO/stages/outcomes/store with single-flight, deadlines and terminal immutability.
- [x] Add private adapter dispatcher and thin Tauri commands/ACL entries.
- [x] Keep output parsing bounded, allowlisted and secret-rejecting.

## 3. Adapters

- [x] Claude status/login/logout + before/after verification.
- [x] OpenCode provider list/connect/logout + provider-set verification.
- [x] Grok official login/logout handoff-only behavior.
- [x] Qoder/TRAE/WorkBuddy exact-candidate launch handoff-only behavior.
- [x] Codex Auth Center delegation without new OAuth logic.

## 4. Frontend

- [x] Add strict parsers, FeaturePort/query keys and Auth session controller.
- [x] Add/extend one shared Auth status/session panel.
- [x] Replace immediate-success copy with awaiting/verified/handoff-only copy.
- [x] Distinguish stop-waiting from cancel-login.
- [x] Render Provider connections without a global OpenCode logged-in switch.
- [x] Disable unsupported actions rather than letting backend reject a visible button.

## 5. Tests

- [x] Exact DTO/enum/version and forbidden-field serialization tests.
- [x] Claude exit 0/1, malformed/oversized/secret JSON, timeout and state-change polling.
- [x] OpenCode zero/one/multiple provider, malformed/secret output and before/after set comparison.
- [x] Grok/desktop handoff never becomes verified.
- [x] Single-flight, illegal transition, terminal replay, stop-waiting and same-process lookup by returned session ID.
- [x] Windows formal interactive-user fail-closed and platform launch-failure contracts.
- [x] UI copy/loading/handoff/provider-list behavior in component tests and four Playwright viewports.
- [x] Regression tests for existing Agent install readiness and Auth Center delegation.
- [x] Automatic recovery after a full renderer process reload through the process-local active-session lookup keyed only by canonical Agent ID; no path/URL/secret persistence was added.
- [ ] Real-account/native macOS and Windows HIL; portable tests do not prove browser, terminal, Explorer-user, or vendor-account behavior.

## Validation

```bash
mise run rust:fmt:check
mise run rust:clippy
mise run rust:test
mise run typecheck:v2
mise run lint:v2
mise run test:v2
mise run test:v2:browser
```

Native HIL must use disposable/test accounts where permitted and record only state transitions/codes, never account identifiers or credential material.

## Verification evidence

- `mise run typecheck:v2` and `mise run lint:v2` passed under the locked Node 24.19.0 runtime.
- `mise run test:v2` passed 55 files / 393 tests.
- Auth-specific Rust tests and the full Agent-install suite passed; the final full Rust run passed 2945 tests with 5 ignored platform fixtures.
- `mise run test:v2:browser` passed 140 tests, including the dedicated Auth scenario across all four configured Chromium viewports.
- `mise run rust:fmt:check`, `mise run rust:clippy`, and `mise run supported-platform:check` passed; the platform manifest covered 2306 current files.
- The renderer remount regression test proves that an active process-local session is recovered by canonical Agent ID and polling resumes without persisting a session ID in browser storage.
- `TRELLIS_CONTEXT_ID=chatgpt-stage4-20260830 mise run check:prearchive --exclude-active-task .trellis/tasks/08-29-agent-auth-verification-state-machine` passed after the recovery command, ACL, structure manifest, and command-count contracts were updated.

## Rollback point

Land read-only Auth observations before replacing actions. Migrate adapters one at a time; an adapter without authoritative verification ships as handoff-only rather than retaining the old immediate-success result.
