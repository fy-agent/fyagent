# Codex Provider Change Plan Switch — Final Evidence

Verified on 2026-08-14 against implementation commit
`ca552f4d918cacc734f81f7efdef70619da139b8` on
`codex/unified-change-plan-codex-switch`.

## Acceptance criteria

- **AC-01 / AC-02:** focused Rust contract, store, plan-side-effect, shared
  fixture, safe-name, and redaction tests prove unique IDs, stable semantic
  digests, zero plan mutation/network access, and bounded serialization.
- **AC-03 / AC-04:** atomic admission rejects digest mismatch, expiry,
  consumption/replay, and current/target/live drift before a writer call; the UI
  offers replan and no force path.
- **AC-05 / AC-06:** additive v16 plan/job/event tables persist monotonic
  snapshots. A successful result requires DB current, device current, unchanged
  target definition, and Codex live projection readback; `liveConfigChanged`
  only selects restart guidance.
- **AC-07:** writer error, baseline restored, target reached, mismatch,
  unavailable readback, recovery-required, and interrupted-job reconciliation
  have distinct tested outcomes. Reconciliation never replays the writer and
  re-reads the latest snapshot after acquiring its lock.
- **AC-08:** the shared dialog covers preview, stale, running, terminal resource
  truth, restart, recovery, evidence, focus, missed/duplicate event polling, and
  unknown-code fallback. App integration confirms Codex invokes
  `apply_change_plan` and does not invoke the direct switch command.
- **AC-09:** non-Codex switching remains direct; Provider create/edit and all
  protected product scopes are unchanged. Tables are idempotently added while
  `SCHEMA_VERSION` remains 16.
- **AC-10:** phases 1–6 are separate commits, all focused and full gates below
  are fresh, and delivery is limited to the named branch with no PR or main
  merge.

## Focused and integration verification

Passed before the final full gate:

```text
mise run rust:test -- change_plan_contract
mise run rust:test -- change_plan_store
mise run rust:test -- codex_provider_switch_plan
mise run rust:test -- change_plan_no_side_effects
mise run rust:test -- codex_provider_change_job
mise run rust:test -- change_plan_reconciliation
mise run test:unit -- change-plan                 # 3 files, 9 tests
mise run test:unit -- useProviderActions          # 31 tests
mise run test:unit -- ProviderList                # 4 tests
mise run test:unit -- App.test                    # 7 tests
mise run test:unit -- windowsSigningAdapter       # 32 tests
mise run typecheck
```

The shared DTO fixture is decoded by both Rust and TypeScript tests. The
cross-layer integration test verifies command registration, DTO naming, event
shape, production entry routing, and the absence of a direct Codex mutation
bypass.

## Final quality and scope gates

```text
mise run check                                    # PASS, exit 0
git diff --check                                 # PASS
python3 .trellis/scripts/task.py validate 08-14-unified-change-plan-codex-switch
```

The cumulative implementation diff from planning freeze `4bfee69c` contains 30
files before this evidence update. Review found no changed protected path and no
schema-v17 claim. Tracked renderer DTO/fixture/UI output contains no API key,
access token, secret, raw settings/config/error, or absolute-path sentinel. The
read-only plan implementation contains no HTTP client or socket call.

The first full gate exposed a deterministic macOS `/var` versus
`/private/var` canonical-path expectation in the Windows signing adapter test.
The expectation now uses `realpathSync`, and the affected focused test plus the
single permitted final full-gate rerun pass. The App integration harness also
gained all four Change Plan IPC handlers so the production entry is exercised
without an unhandled request.

## Current-host trial build

- Product version: `0.3.4`
- Source used for build: `ca552f4d918cacc734f81f7efdef70619da139b8`
- Successful command:
  `pnpm tauri build --target aarch64-apple-darwin --debug --bundles app`
- App:
  `/Users/serendipity/.codex/worktrees/d8328f41-5cd7-406e-9f01-95b42c640e2b/fyagent/src-tauri/target/aarch64-apple-darwin/debug/bundle/macos/FyAgent.app`
- Trial zip:
  `/Users/serendipity/.codex/worktrees/d8328f41-5cd7-406e-9f01-95b42c640e2b/fyagent/src-tauri/target/aarch64-apple-darwin/debug/bundle/macos/FyAgent_0.3.4_aarch64-debug.app.zip`
- App executable SHA-256:
  `71d7b1d7899d982473af8b5d8aa56cfb874f19b4dac0263310eef17d10eea2dc`
- Trial zip SHA-256:
  `8364f002c3423345a7548b610034070c45b73751a72f00f5e2e67f167ae55d70`

The preferred `mise run build:debug` command compiled the app and produced the
same `.app`, but its subsequent optional DMG step failed twice inside Tauri's
generated `bundle_dmg.sh`. Because the canonical task forbids forwarded bundle
arguments, the equivalent repository Tauri command above was rerun with
`--bundles app` and exited 0. No passing DMG evidence is claimed.

Static bundle inspection found the generated renderer asset in the native
binary, found the Change Plan UI text and IPC calls in that generated asset, and
found all four backend commands in the native binary. A direct process smoke and
a separate LaunchServices `open -n` smoke each kept the new app alive for eight
seconds; the runtime log reported `FyAgent v0.3.4 started` and `主窗口已显示`, and
both smoke processes were then terminated.

Evidence level is **current-host macOS arm64 debug bundle + static inclusion +
process/LaunchServices startup smoke**. The startup used the current host's real
application data and therefore ran normal schema/default-pricing/usage-rollup/
backup startup maintenance. It did not execute a Provider switch. This is not a
signed/notarized release, DMG, Windows validation, production validation, real
Agent-use observation, or user acceptance.

For local trial:

```bash
open "/Users/serendipity/.codex/worktrees/d8328f41-5cd7-406e-9f01-95b42c640e2b/fyagent/src-tauri/target/aarch64-apple-darwin/debug/bundle/macos/FyAgent.app"
```
