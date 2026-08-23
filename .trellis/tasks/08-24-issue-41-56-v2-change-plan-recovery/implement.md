# Implementation plan

1. Freeze the TypeScript wire contract and strict parser tests against the
   shared backend DTO fixture.
2. Add the Tauri and browser feature ports, event cleanup, and ACL/architecture
   coverage without changing Rust/backend registration.
3. Build the route-local plan preview and execution workspace with one confirm,
   event-driven snapshots, polling fallback, cancellation, and honest outcome
   presentation.
4. Integrate the module only into the Codex Models detail and refresh the safe
   Provider summary after terminal execution.
5. Run focused tests, V2 gates, renderer build, full `mise run check`, then a
   reversible isolated native Tauri UAT. Record exact evidence before creating
   the stacked replacement PR.

## Native UAT evidence

- Evidence time: 2026-08-24 04:44 Asia/Shanghai.
- Built and ran the debug macOS bundle with the temporary identifier
  `com.fyagent.ucpuat` and isolated home
  `/tmp/fyagent-ucp-uat.vrEDW2`. The checked-in identifier was restored to
  `com.fyagent.desktop` immediately after the run.
- Seeded two reversible fixtures, `UAT Current` and `UAT Target`, then used the
  native Models -> Codex surface to preview and apply the target once. The
  preview exposed the four required sections and did not expose either canary,
  a digest, a raw configuration, or an absolute managed path.
- The native terminal surface reported all five phases as `succeeded`, the job
  as `succeeded/applied_restart_recommended`, restart as `recommended`, usage
  evidence as `not_observed`, and all four managed resources as `matched`.
- Independent SQLite readback recorded plan
  `3f15cdd8-a176-4339-b3a4-0e993ec16e58` consumed once and job
  `9be7512d-ae1c-423a-a646-96053f4a2fdc` at revision/event sequence `7/7`.
  Events progressed monotonically from `planned` through `terminal`.
- Independent file readback confirmed `currentProviderCodex=uat-target`, the
  target model in `config.toml`, one target-canary match in `auth.json`, and
  zero old-canary matches. No secret value is recorded in this evidence.
- Completion screenshot SHA-256:
  `0c1ccf0f2a008206f54629047c53a6dbf8bf64774e1f0481ebd19fe8ebc6e61e`
  (118139 bytes). The production-installed FyAgent process was relaunched after
  UAT.

## Fresh verification

- `mise run typecheck` passed after tightening the typed apply mock used by the
  component test.
- `mise run test:v2:browser` passed all 116 Playwright cases across the locked
  viewports.
- `mise run check` passed from environment preflight through frontend tests,
  Rust fmt/check/clippy/tests, task/docs contracts, supported-platform scan,
  Python lock, version, and release checks.
