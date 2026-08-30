# Implement — Stage 3

## 1. Preflight

- [x] Verify Stage 1 contract is present (`0a048728`) and archived
  (`abb16eed`), then compare the pre-Stage-3 Windows behavior with
  `research/current-windows-gap.md`.
- [x] Capture official source/scope/architecture plus bounded PE ProductName,
  version-resource and signer-leaf evidence for QoderWork CN, TRAE Work CN and
  WorkBuddy. Production continues to resolve latest endpoints; research
  versions/URLs are not pinned.
- [x] Update the backend/frontend/module specs for Windows inventory, closed
  installer descriptors, Helper protocol v3, authoritative readback and
  assisted-vs-managed semantics.

## 2. Inventory owner

- [x] Extend link-safe registry traversal with closed Uninstall/App Paths
  locations and explicit 32/64 WOW64 views.
- [x] Bind per-user reads to the frozen Explorer SID under
  `HKEY_USERS\\<SID>`; never use elevated-process HKCU.
- [x] Add product-known-path evidence and retain Codex PackageManager
  regression coverage. The three reviewed products are EXE-only and have no
  proven PFN/AUMID, so no speculative PackageManager adapter is shipped.
- [x] Replace production PE byte scanning with Win32 version-resource APIs;
  the byte-window parser remains test-host compatibility only.
- [x] Add stable file identity, provenance merge, custom-scope projection,
  stale/conflicting observations and partial-failure projection. Incomplete
  inventory remains `unknown` and disables fresh destinations.

## 3. Product/source policy

- [x] Add closed installer scope, vendor-UI interaction, supported
  architecture, ProductName and signer-leaf policy.
- [x] Keep Qoder User x64 distinct from any future System installer.
- [x] Mark TRAE/WorkBuddy as vendor-choice/unknown-scope; WorkBuddy's official
  guide explicitly exposes location choice.
- [x] Keep Windows ARM64 and every unproven source/scope/silent-mode
  combination disabled/manual.

## 4. Download, signature and helper

- [x] Reuse the existing streaming downloader, fixed job directory, retained
  artifact, pin factory and PackageBridge; no second downloader was added.
- [x] Implement WinVerifyTrust admission, exactly-one-signer validation and
  actual signer-leaf resolution through `CryptMsgGetAndVerifySigner`.
- [x] Extend the fixed Helper/bridge protocol with the exact
  `agent-exe-install + product enum` operation. Protocol v3 binds the action in
  `Hello(action)` before bridge control/admission.
- [x] Revalidate Helper image, frozen interactive context, retained package
  pin, bridge identity and action/product before side effects.
- [x] Launch vendor UI with `ShellExecuteExW + SEE_MASK_NOCLOSEPROCESS`, map
  UAC/user cancellation separately, require an observable process handle and
  inspect its exit status without killing the installer.
- [x] Add `launching_installer`, `awaiting_user` and terminal `incomplete`
  states. Cancellation closes at the external-launch boundary; timeout means
  “stop waiting”, not “cancel installation”.

## 5. Post-install readback

- [x] Capture complete pre/post normalized inventory snapshots; an incomplete
  baseline or final readback can never authorize success.
- [x] Poll within a bounded post-installer deadline.
- [x] Require one trusted ProductName/signer/file-identity/version/scope result;
  process launch and exit code are hints only.
- [x] Classify duplicate, no-result, unchanged update, version drift, scope
  drift, unobservable process, timeout and nonzero exit explicitly.
- [x] Keep Windows vendor-assisted operations distinct from the macOS managed
  rollback-capable transaction.

## 6. Frontend

- [x] Reuse the Stage 1 target/destination picker and authoritative inventory
  query; readiness no longer has a second known-path-only observer.
- [x] Extend the shared lifecycle status surface for launch, vendor UI wait,
  incomplete result and distinct Windows installer reasons.
- [x] Show product/architecture/scope capability honestly and omit unsupported
  actions.
- [x] Preserve existing single-flight/double-submit protection and bind every
  job to the selected inventory/target revision.

## 7. Tests and HIL

- [x] Portable registry/static contracts cover fixed HKU/HKLM locations,
  explicit WOW64 views, link rejection, bounded child/value rules and partial
  access/error projection.
- [x] Evidence fixtures cover Registry/App Paths/known-path provenance merge,
  stable-file dedup, custom paths, stale registrations and conflicts. Current
  products have no package records; Codex remains the PackageManager fixture.
- [x] File-version/ProductName/signer policy fixtures plus retained-file and
  replacement/drift rejection are covered.
- [x] Existing and extended Helper suites cover nonce/SID/image/bridge/context,
  Hello-action binding, process-handle requirements and settlement/quarantine.
- [x] Closed-state/readback tests cover UAC/user cancellation mapping, nonzero
  exit, exit-zero/no candidate, duplicate candidate, custom path, stale
  registration and incomplete inventory.
- [ ] Qoder/TRAE/WorkBuddy x64 disposable Windows HIL; record unsupported architectures separately.
- [x] Codex PackageManager/Helper and Windows runtime security regressions pass
  locally; the Helper crate also type-checks for `x86_64-pc-windows-msvc`
  using a resource-embedding-only RC shim.
- [x] Hosted Windows backend checks for committed SHA `1b321e77`（CI run
  `33293527563`）；Windows backend、x64/ARM64 native contracts and aggregate
  `CI / Required` all passed.

## Validation

```bash
mise run rust:fmt:check
mise run rust:clippy
mise run rust:test
mise run typecheck:v2
mise run test:v2
mise run test:v2:browser
```

Hosted Windows backend checks and native HIL are mandatory; macOS/local portable tests do not prove Windows registry/helper/UAC behavior.

### Local validation evidence — 2026-08-30

- `mise run supported-platform:check` — passed; platform-sensitive paths and
  SHA-256 manifest updated without widening exclusions.
- `mise run rust:fmt:check` — passed.
- `mise run rust:clippy` — passed with warnings denied.
- `mise run rust:test` — passed: 2,934 core tests (5 ignored), 114 Codex
  installer-domain tests, 43 Helper tests, and the remaining integration
  suites.
- `mise run test:unit` — passed: 1,536 tests in 172 files; one existing test
  skipped.
- `mise run typecheck:v2` / `mise run lint:v2` / `mise run test:v2` — passed;
  382 V2 tests.
- `mise run test:v2:browser` — passed; 136 Playwright tests across all
  configured viewports.
- Real Windows WinVerifyTrust, Alice/Bob Shell identity, UAC interaction,
  vendor installer hand-off, Registry views and post-install HIL remain
  unverified until explicitly run on a disposable Windows x64 host.

## Rollback point

Land read-only inventory before installer execution. Enable each product/architecture independently only after its source, signer, helper and post-install HIL pass. A product without complete evidence remains manual/assisted instead of inheriting another product's executor.
