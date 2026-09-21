# Installer preflight and helper recovery delivery (#27 / #29)

Implementation is complete in `codex/next-installer-27-29`, based on
`2c09c4be2c5f50b4060fd6f7a13a7be31c364c54`. Integration acceptance is pending.
The parent owns final Rust validation, integration, PR delivery and Issue closure.

## Actual gaps repaired

- Generic single-target installs started immediately, and the frontend could
  silently rebind a stale target. Every directory install/update now prepares a
  native summary and requires confirmation. Execution consumes the exact
  request plus native destination/runtime binding held in the existing inventory
  cache. Changed revisions or destinations require another preflight.
- Supported paths now check platform/architecture, required native tools or
  Node/npm, target write access and temporary/target volume space before package
  download. Unknown package size remains explicitly unknown.
- Codex reuses its existing service/platform preflight and job flow, with one
  five-minute confirmation capability. Fresh macOS installs are fixed to the
  confirmed user Applications directory; updates preserve the exact existing
  bundle. Local target identity is checked before download and before install.
- Windows CLI preflight is a new closed, read-only ordinary-user helper action.
  It uses the existing frozen Explorer user, authenticated pipe and helper
  identity. Directory access checks use that ordinary token. Permission failure
  has its own bounded reason instead of being mislabeled as missing Node/npm.
- Directory cards show native cancellation only while the job permits it.
  Terminal outcomes remain available in the hook, and uncertain recovery results
  cannot immediately be overwritten by another attempt for the same Agent.
  Error text explains permission, space, target drift, native cancellation and
  recovery without rendering raw diagnostics.

## Existing mechanisms intentionally reused

Inventory opaque IDs/revisions/TTL; single-flight action jobs; Codex job store and
progress stream; managed desktop download and temp ownership; macOS atomic
replace/rollback; Windows pinned ordinary-user helper, native wizard/UAC and
PackageManager; helper admission/quarantine. No new raw path, URL, argv, executable
or bypass input is accepted. The macOS system-commit production gate remains
closed. No actual Agent install, privilege request, configuration mutation,
network model call or user helper installation was performed during validation.

## Validation evidence

Passed after the final relevant frontend implementation:

- `mise run bootstrap` (initial checkout setup and dependency validation).
- `mise run typecheck`.
- Focused ESLint via `mise exec -- pnpm exec eslint` over every modified/new
  `.ts`/`.tsx` file (file set from `git diff --name-only` plus untracked sources).
- `mise run test:unit -- tests/renderer/pages/agents/useAgentLifecycleAction.test.tsx tests/renderer/pages/agents/AgentInstallReadinessSection.test.tsx tests/renderer/pages/agents/CodexDesktopInstallerPanel.test.tsx tests/renderer/platform/featurePorts.test.ts tests/renderer/platform/agentInstallReadinessPort.test.ts tests/renderer/platform/tauriAclContract.test.ts tests/architecture/rustModuleBoundaries.test.ts tests/codexUserHelperContract.test.ts tests/codexDesktopDtoContract.test.ts tests/codexWindowsUserScopeContract.test.ts tests/renderer/pages/agents/Page.test.tsx tests/renderer/pages/agents/codexDirectoryActionProjection.test.ts tests/shared/codexInstallConfirmation.test.ts`: **13 files, 159 tests passed**.
- `mise run rust:fmt` and `git diff --check`.

An earlier `mise run rust:check` passed on aarch64 macOS after the initial native
preflight/confirmation implementation. That is **not final Rust evidence**:
subsequent inventory confirmation consumption, recovery guard, helper permission
reason and native tests were added. Per parent scheduling, do not repeat the heavy
compile in this checkout; the final committed Rust changes and all new native
fixtures are **pending the parent integration Rust suite**. No Windows compile or
native Windows runtime has been claimed. No full browser/performance suite ran.

New native tests pending execution cover: Codex no-job preflight and single-use
confirmation; target drift before download; insufficient space before job
creation; macOS writable-user preflight and no confirmed system fallback;
Agent inventory confirmation consumption and destination drift; unknown-result
retry preservation; disk probe empty/unavailable states; closed helper preflight
wire actions and ambiguous-owner refusal.

## Native installation/write identity audit

| Supported path | Effective writer and authority |
| --- | --- |
| macOS managed DMG | Current user for confirmed user Applications; existing shared transaction; system-commit remains disabled until its native gate is reviewed |
| macOS Codex DMG | Current user; fresh user Applications root or exact existing writable target; no confirmed-root fallback |
| macOS Claude/Grok npm | Current user's existing npm prefix/Node, owner and runtime checks; no sudo |
| macOS Grok native update | Existing native tool and directory under the current user, fixed update operation |
| Windows managed EXE | Frozen Explorer user's authenticated helper; fixed vendor installer and native wizard/UAC choose final destination |
| Windows Codex MSIX | Frozen Explorer user's helper and PackageManager registration for that user |
| Windows Claude/Grok CLI | Frozen Explorer user's helper; closed observed owner and npm/native runtime; prefix access queried under the helper's token |

Windows EXE wizard preflight deliberately does not claim knowledge of its later
chosen destination or successful elevation. Confirmation states that the vendor
window chooses the location and final changes.

## Remaining native/HIL evidence

- Windows compile/Clippy plus real installed helper startup and version pairing.
- Alice's Explorer desktop with Bob's elevated FyAgent host: actual install
  registration, CLI prefix and filesystem owner remain Alice, including
  multi-session/domain identity and changed/absent Explorer cases.
- Native UAC rejection, helper admission rejection, helper loss/timeout and
  partial deployment quarantine; known completed outcomes, original installation
  recovery and next-step UI must be observed on real Windows.
- Writable/unwritable prefixes, insufficient target volume capacity, required
  Node/architecture failure, actual vendor wizard handoff and final installed
  readback/configuration preservation. Fixture tests are not this evidence.
- Signed macOS system helper production acceptance remains out of scope and its
  gate is still closed. Real DMG install/update/rollback was not run on user apps.

## Integration notes for #25 / #34

`AgentDirectory.tsx` is this package's only-writer file. It now mounts a shared
Codex confirmation plus the generic confirmation and exposes cancellation. Root
can add the separately delivered `AgentSourceLinks` and recommendation selection
wiring after cherry-picking; this package does not alter first-use/model defaults.

The new summaries already contain platform, architecture and version/channel,
with the same release ID. They currently contain **no URL**. The smallest native
source-display extension is `agent_install/preflight.rs::inspect_preflight`:
its `desktop::resolve_desktop_source` result (`sources/mod.rs::ResolvedDesktopSource`)
already holds `download_url`, `official_page`, `format`, `versionless_latest`.
Project those as read-only metadata; do not add URL input to start requests.
For Codex, `services/codex_desktop/install_confirmation.rs::prepare_install` holds
`ReleaseDescriptor.download_endpoint`; `TrustedDownloadEndpoint::url()` is the
closed actual download entry point. Avoid a second renderer URL registry.

Potential integration overlap: `src-tauri/src/lib.rs` command registry,
`src/shared/features/ports.ts`, browser port assembly, ACL command-count assertion,
and the existing Codex ordinary command list (now eight) may also be touched by
other packages. Preserve both sides' additions.
