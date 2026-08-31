# Acceptance Evidence Plan

Date: 2026-08-31

No implementation acceptance is claimed by this planning task. This file defines the evidence required later.

## G1 — Backend product policy

Required evidence:

- exhaustive product/surface/action unit matrix;
- direct action tests proving `action_not_supported` / `surface_not_supported`;
- fake transport/helper/filesystem counters proving zero side effects on rejection;
- readiness and inventory wire fixtures showing no update action/eligibility for the three install-only products.

Completion condition:

```text
One backend policy owner controls legal surfaces, readiness, inventory and dispatch.
```

## G2 — Directory ordering

Required evidence:

- pure bucket/rank tests;
- scan reducer/projection tests for initial scan, rescan and stale failure;
- AgentDirectory DOM order test;
- focus preservation test;
- post-install readiness patch moves a card only after authoritative reread.

Completion condition:

```text
Installed domestic -> installed other -> unresolved -> not installed,
stable within each bucket and no progressive-scan jitter.
```

## G3 — Claude source

Required evidence:

- fixed endpoint constants/enums;
- bounded manifest parser fixture for exact v2 universal branch;
- wrong schema/platform/arch/format/version/redirect tests;
- remote URL/hash fields ignored as capability/admission;
- cache/retry/cancel/force-refresh tests through existing HTTP owner;
- sanitized real-manifest observation at implementation time;
- current DMG fixture/bundle metadata evidence.

Completion condition:

```text
Claude release descriptor is generated without accepting a remote artifact URL,
and the existing downloader/DMG owner consumes it.
```

## G4 — OpenCode update

Required evidence:

- fixed repository latest-version metadata test;
- exact arm64/x64 stable endpoint mapping;
- installed vs remote version comparison;
- metadata/download race leads to refresh, not unknown success;
- negative scan proving no Electron updater invocation/dependency.

Completion condition:

```text
One-click update is available through FyAgent’s shared transaction only.
```

## G5 — Reuse/architecture

Required evidence:

- diff/call-graph review showing shared downloader/job/DMG/helper use;
- architecture tests for module visibility/dependency direction;
- dependency manifest/lock review proving no accidental package addition;
- negative searches listed in `reuse-decision.md`;
- Codex and existing managed Agent regression tests.

Completion condition:

```text
No second downloader, updater, DMG transaction, progress formatter, target authority or helper.
```

## G6 — macOS runtime HIL

Required evidence from a real app build on supported macOS:

| Scenario | Evidence |
| --- | --- |
| Claude fresh install | job snapshots, final path, inventory/version reread |
| Claude update | before/after version, exact target path, no auto-launch |
| OpenCode update | before/after version, exact target path, no auto-launch |
| Chinese three | install button works; installed state has no update action/network call |
| Directory order | screenshot/DOM trace after scan and after install |
| Cancel/network failure | persistent terminal state and cleanup |
| Application running | safe refusal, old app intact |
| Explicit launch | only button click launches selected app |

Apple Silicon HIL is required. Intel execution may be recorded as unavailable if no Intel host exists, but architecture/source selection must have contract fixtures.

## G7 — `/Applications` signed helper HIL

Hard dependency: `08-31-macos-privileged-application-commit-helper`.

Required evidence:

- Developer ID-signed/notarized app containing the reviewed helper;
- fresh Claude system install;
- Claude/OpenCode in-place system update;
- administrator cancellation;
- rollback and recovery-required fault injection;
- no duplicate `~/Applications` app;
- helper/product policy remains closed and renderer submits no path.

Completion condition:

```text
System install/update is not marked accepted from mocks, ad-hoc builds or user-scope fallback.
```

## G8 — Windows deferral and regression

Required evidence:

- no new Windows source/installer/helper code in the task diff;
- shared product contract tests reflect the intentional CLI/update removals;
- all unrelated Windows tests/build contracts remain unchanged and green.

Completion condition:

```text
Windows desktop adaptation remains a future task without reintroducing removed product policies.
```

## Evidence ledger template

Execution should append rows rather than replacing the plan:

| Gate | Commit/build | Command/scenario | Result | Artifact/log | Residual |
| --- | --- | --- | --- | --- | --- |
| G1 | pending | pending | pending | pending | pending |
| G2 | pending | pending | pending | pending | pending |
| G3 | pending | pending | pending | pending | pending |
| G4 | pending | pending | pending | pending | pending |
| G5 | pending | pending | pending | pending | pending |
| G6 | pending | pending | pending | pending | pending |
| G7 | pending | pending | pending | pending | pending |
| G8 | pending | pending | pending | pending | pending |

