# Implement — Stage 4

## 1. Preflight/spec

- [ ] Recheck official Claude/OpenCode/Grok CLI references at implementation time and freeze minimum supported versions/output contracts.
- [ ] Review existing Auth Center/Codex OAuth ownership and Tooling terminal/command APIs.
- [ ] Update external-agent and frontend specs to distinguish handoff, verification and provider-owned Auth.

## 2. Backend domain

- [ ] Add Auth observation discriminated union and closed capability/reason enums.
- [ ] Add Auth session DTO/stages/outcomes/store with single-flight, deadlines and terminal immutability.
- [ ] Add private adapter dispatcher and thin Tauri commands/ACL entries.
- [ ] Keep output parsing bounded, allowlisted and secret-rejecting.

## 3. Adapters

- [ ] Claude status/login/logout + before/after verification.
- [ ] OpenCode provider list/connect/logout + provider-set verification.
- [ ] Grok official login/logout handoff-only behavior.
- [ ] Qoder/TRAE/WorkBuddy exact-candidate launch handoff-only behavior.
- [ ] Codex Auth Center delegation without new OAuth logic.

## 4. Frontend

- [ ] Add strict parsers, FeaturePort/query keys and Auth session controller.
- [ ] Add/extend one shared Auth status/session panel.
- [ ] Replace immediate-success copy with awaiting/verified/handoff-only copy.
- [ ] Distinguish stop-waiting from cancel-login.
- [ ] Render Provider connections without a global OpenCode logged-in switch.
- [ ] Disable unsupported actions rather than letting backend reject a visible button.

## 5. Tests

- [ ] Exact DTO/enum/version and forbidden-field serialization tests.
- [ ] Claude exit 0/1, malformed/oversized/secret JSON, timeout and state-change polling.
- [ ] OpenCode zero/one/multiple provider, malformed/secret output and before/after set comparison.
- [ ] Grok/desktop handoff never becomes verified.
- [ ] Single-flight, illegal transition, terminal replay, stop-waiting and reload recovery.
- [ ] Windows interactive-user context drift and macOS terminal launch failures.
- [ ] UI keyboard/copy/loading/retry/handoff/provider-list browser tests.
- [ ] Regression tests for existing Agent install readiness and Auth Center.

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

## Rollback point

Land read-only Auth observations before replacing actions. Migrate adapters one at a time; an adapter without authoritative verification ships as handoff-only rather than retaining the old immediate-success result.
