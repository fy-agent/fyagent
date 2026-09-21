# Provider subscription exit follow-up

Authorized by the parent after the independent #42/#43/#64 review found that
Claude/Codex/Grok Build could activate a subscription without an equivalent
target-specific exit control. This is a follow-up in the existing task; the
parent's active-task pointer and other writers remain unchanged.

Owner: config_pack_73, renderer-only continuation on the integration checkout.
Allowed: new ProviderSubscriptionRestore component; existing
XaiSubscriptionSection integration; Provider DTO/port/query key; Tauri/browser
adapter wiring; focused tests and related frontend spec. Do not modify Models
Page handlers, provider credentials, or native proxy code. Parent owns the new
read-only native command and the full native checks.

Native contract agreed with parent:

```ts
get_proxy_restore_preview({ app }): {
  app: "claude" | "codex" | "grokbuild";
  enabled: boolean;
  canRestore: boolean;
  targets: { path: string; exists: boolean }[];
}
```

Targets are native display-only paths. They deliberately have no backupPath:
the true original may live in a DB proof rather than a same-named rolling file.
Use existing CopyablePath and Dialog. Exit uses the existing
set_proxy_takeover_for_app({appType: app, enabled: false}) owner; confirmation
never submits a filesystem path or backup content.

Success requires a fresh preview with enabled false, actual Provider live state
that is present and readable (including known missing/empty file states), and a
fresh managed-auth overview. Unknown/partial outcomes retain conflict guidance
and block further target writes through the existing callback. Other target
state is untouched. Tests cover cancellation, target isolation, duplicate
confirmation, stale preview, native conflict and failed/mismatched readback.

Ordinary binding keeps its existing `disabled` / `onBeginWrite` guard. Optional
`recoveryDisabled`, `onBeginRecovery`, and `onRecoveryConfirmed(app)` callbacks
allow recovery to share the real writer lock without being permanently disabled
by an earlier unknown write result. A failed exit retains the ordinary block.
After a fresh check, an enabled target can open a new confirmation; an already
exited target repeats all readbacks without executing restore again. The
confirmation recheck compares app, enabled/canRestore, each actual path and
existence before mutation. Root owns the Models Page callback wiring.

## Final renderer checks

Run after the final source/test edit on 2026-09-21:

- `mise run test:unit -- tests/renderer/platform/providerProxyRestorePort.test.ts
tests/renderer/pages/models/ProviderSubscriptionRestore.test.tsx
tests/renderer/pages/models/XaiSubscriptionSection.test.tsx
tests/renderer/pages/models/OpenCodeSubscriptionRestore.test.tsx
tests/renderer/platform/xaiSubscriptionPort.test.ts`: 5 files, 137 tests passed.
- `mise run typecheck`: passed, including root's typed Page callbacks.
- Locked local ESLint over the changed renderer DTOs, ports, adapters, two
  components and two new test files: passed.
- `mise run format:files` for owned changed source/test/spec files: passed.
- `git diff --check`: passed.

Regression assertions include five changed-preview refusals, three targets'
readback, native conflict, missing/unreadable/wrong-target live state, failed
overview, still-enabled proxy, ordinary-write blocking through retry, read-only
completion after an earlier mutation, synchronous duplicate guards, target A
success retained after target B failure, and late failure after unmount.

Native tests/full build/browser/real-account/Windows runtime were not run by
this renderer owner. The parent owns the native command and the combined
validation. These synthetic renderer results do not prove native file recovery
or target runtime pickup. No integration-tree commit/staging was performed;
the parent retains all shared-tree commit and acceptance ownership.
