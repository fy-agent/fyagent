# PRD — Agent lifecycle reliability program

## Problem

Agent installation, update, authentication and V2 interaction previously crossed several independent trust and state boundaries. A renderer could not safely treat the first discovered installation as authoritative; platform installers had different scope and rollback semantics; opening an Auth entry point could be mistaken for login success; and frontend state was split among route, tabs, local component state and query effects.

This program coordinates five ordered engineering stages so the product exposes one truthful lifecycle model without creating a second platform framework, a second OAuth implementation, or page-local substitutes for shared infrastructure.

## Program goals

1. Freeze an authoritative installation-target contract: candidate identity, scope, revision, ambiguity and closed IPC.
2. Preserve the exact selected macOS installation during update and verify rollback/readback.
3. Discover and install Windows desktop Agents through reviewed native authorities with bounded elevation, source verification and post-install readback.
4. Replace Auth launch success with observable sessions and authoritative verification where an official observer exists.
5. Make V2 navigation, tabs, route loading, queries, keep-alive behavior and shared assignment surfaces deterministic and testable.
6. Reuse existing inventory, Tooling, Auth Center, Tauri, TanStack Query, React Router and Radix infrastructure instead of introducing parallel frameworks.
7. End with one cross-stage integration matrix that distinguishes automated evidence from real operating-system/account HIL.

## Ordered work

```text
Stage 1 — installation target authority
├── Stage 2 — macOS in-place install/update
├── Stage 3 — Windows discovery and one-click install
└── Stage 4 desktop-Agent target binding

Stage 5 independent frontend reliability work could proceed earlier,
but final install/Auth UI wiring depended on Stages 1 and 4.
```

The approved execution used one serial integration branch, `dev/laiyongjie`. The parent task is an integration/governance task; it does not replace child implementation.

## Acceptance criteria

### Dependency and archival

- [x] Stage 1 completed before platform installation and desktop Auth target wiring.
- [x] Stages 2–5 consumed the reviewed authority boundaries instead of introducing renderer paths or first-match selection.
- [x] All five child tasks are completed and archived under `.trellis/tasks/archive/2026-08/`.
- [x] The integrated branch received exact-head Full CI before the parent integration review.

### Installation target authority

- [x] Candidates have opaque identities, typed scope and target revision.
- [x] Multiple trusted installations require explicit selection; ambiguity never selects the first result.
- [x] Renderer requests contain only closed identifiers/actions and optional opaque backend-generated authority fields.
- [x] Target drift, stale inventory and non-executable candidates fail closed and remain distinguishable.

### macOS deployment

- [x] Managed updates preserve the selected bundle path and scope.
- [x] Fresh-install permission fallback does not silently relocate an existing managed update.
- [x] Replacement uses staging and authoritative bundle identity/version readback.
- [x] Post-commit failure restores and re-verifies the previous bundle before recovery is reported.
- [ ] A signed candidate has exercised running-app, `/Applications`, permission denial, rollback and installed-WebView launch HIL.

### Windows deployment

- [x] Registry, App Paths, MSIX and bounded known paths contribute typed inventory evidence.
- [x] Evidence is normalized/deduplicated without promoting stale or untrusted registrations to executable targets.
- [x] EXE source/product/signer and helper/elevation boundaries are explicit and tested.
- [x] Installer process completion alone cannot produce success; fresh post-install inventory readback is required.
- [x] x64 and ARM64 native contract CI is part of the integrated branch validation.
- [ ] Signed Windows candidates have exercised UAC consent/cancel/timeout, Explorer-user authority, reboot/exit variants, signer mismatch and post-install readback HIL.

### Authentication

- [x] Launching a browser, terminal or desktop application is never returned as verified login success.
- [x] Claude login/logout require authoritative structured status readback.
- [x] OpenCode is provider-scoped and verifies before/after provider-set changes without reading `auth.json`.
- [x] Grok and desktop Agents without a status observer are explicitly handoff-only.
- [x] Codex remains owned by the existing FyAgent Auth Center; no duplicate OAuth command or storage is added.
- [x] Sessions are single-flight per Agent, bounded, terminally immutable, stoppable and queryable by ID.
- [x] A renderer reload can rediscover one canonical active backend session by Agent ID and resume polling without renderer persistence of sensitive execution data.
- [ ] A full backend-process restart can resume an active Auth session.
- [ ] Real-account/disposable-account HIL has verified the supported Auth transitions on native macOS and Windows.

### Frontend reliability and architecture

- [x] Route, hash, selected link, `aria-current`, SelectionLens and expanded navigation groups share route-derived authority.
- [x] Shared Radix-based tabs replace page-local tab state where tab semantics are required.
- [x] Six feature routes are lazy-loaded behind deterministic route boundaries.
- [x] Route-chunk tooling is classified and enforced by repository/CI ownership checks.
- [x] Query keys, enabled lifecycles and invalidation are owned by shared feature/query modules.
- [x] Keep-alive behavior preserves only reviewed state and does not create duplicate lifecycle owners.
- [x] Skill/MCP assignment uses shared surfaces instead of copy-pasted page implementations.
- [x] Unsupported/noop actions are absent or disabled with explicit reason state.
- [x] React warning regressions and four-viewport keyboard/geometry behavior are gated by tests.
- [ ] Installed native Tauri WebViews have completed candidate UAT on both supported desktop operating systems.

### Final integration

- [x] `final-report.md` maps every cross-stage acceptance concern to implementation/test evidence.
- [x] The full current-host prearchive gate and V2 browser matrix pass on the integrated branch.
- [x] Stage 5 archive head `1f308459cf5782afeefafccfeff1e8fc092d3e7c` passes GitHub Full CI run `33310186138` before parent review.
- [x] Remaining native/HIL gaps are explicit and are not represented as release certification.
- [x] The publication procedure requires an external exact-head Full CI run after the parent archive commit is pushed; this avoids a self-referential report commit while keeping final branch verification mandatory.

## Non-goals

- Replacing Tauri, React Router, TanStack Query, Radix or the existing Auth Center with new frameworks.
- Accepting renderer-provided filesystem paths, commands, URLs, scopes, installer switches or credentials.
- Treating cross-compilation, mocks, browser tests or contract tests as native UAC/signing/vendor-account proof.
- Closing related GitHub issues mechanically merely because this parent task is archived.
- Expanding the program into unrelated Agent features or platform support.

## Completion interpretation

The parent can be archived when the five engineering stages are archived, the integrated automated gates pass, and the cross-stage report is complete. The immutable parent archive head is then pushed and verified by an external exact-head Full CI run. Native HIL rows remain release gates and must stay unchecked until executed against exact signed candidates or disposable real accounts.
