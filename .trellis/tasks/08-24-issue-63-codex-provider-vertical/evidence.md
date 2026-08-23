# Issue #63 Codex Provider vertical evidence

Evidence date: 2026-08-24 (Asia/Shanghai)

## Source baseline and boundary

- Branch: `codex/issue-63-codex-provider-vertical`.
- Stacked base: `codex/ucp-integration-35-41@a44ed49ce82f9e805d21045ef7a647f09d040085`.
- Dependencies: Draft PRs #130, #132, #134 and #136.
- Scope is one Codex create/edit plus set-current vertical. It does not add a
  second Provider writer, a general execution engine, network work during
  apply, WorkBuddy behavior or another confirmation.

## Fresh local evidence

| Gate | Result | Evidence level |
| --- | --- | --- |
| native `codex_provider_upsert_native_keyring_hil` | PASS: create/edit, injected writer failure, rollback readback, rotation and cleanup | matching-host macOS Keychain HIL |
| `mise run test:v2 -- tests/v2/pages/models/quickSetup.test.ts tests/v2/pages/models/Page.test.tsx` | PASS: 51 passed | focused renderer contract |
| `mise run typecheck:v2` | PASS | V2 type contract |
| `mise run lint:v2` | PASS | V2 lint |
| `mise run test:v2:browser` | PASS: 116 passed across four locked viewports | browser interaction regression |
| `mise run check` | PASS: frontend 1479 passed/1 skipped after the empty-key regression; Rust 2847 library tests plus integration/doc tests; contracts/release passed | complete final-code current-host repository gate |
| isolated native debug bundle | PASS: preview, one confirmation, five terminal phases and independent readback | native macOS Tauri UAT |
| `git diff --check` | PASS | patch hygiene |

The task metadata added after that full gate is validated separately by the
pre-archive contract gate before the implementation commit is created.

## Behavioral evidence

- Preview inserted one ready plan and zero jobs. Provider/current/live files,
  Keychain and network targets were untouched.
- Public and persisted plan state contained `sec_…5bb4` and `os_keyring`, not
  the full SecretRef, submitted key, secret digest or private proof.
- One confirmation consumed the plan once, produced one job with monotonic
  events `1..7`, called the existing Provider writer once and returned
  `succeeded/applied_restart_recommended` with four matched resources.
- The Provider row persisted only `secretRef/version/backend`; Codex's external
  native `auth.json` contained the approved material as disclosed by preview.
  A canary scan found no match in FyAgent-owned database/logs or Codex
  `config.toml`.
- Injected edit failure kept the old credential and live baseline usable,
  deleted the failed new reference and reported verified rollback. Successful
  rotation deletes the prior reference only after real target readback.
- API-key-only drift, Provider/live drift, expiry and process-private proof
  loss all stop before Keychain or Provider writes. Restart/crash ambiguity is
  `recovery_required` and never replays the writer.

## Not yet established

- Final-head GitHub Required CI, especially Windows Backend, is pending until
  the stacked Draft PR is pushed.
- Issue #35 still lacks matching-host Windows Credential Manager HIL; that is a
  dependency blocker for merge/closure, not evidence supplied by this macOS
  slice.
- This Draft implementation does not merge `main`, release a build or close
  Issue #63.
