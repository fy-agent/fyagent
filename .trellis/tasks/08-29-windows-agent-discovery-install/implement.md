# Implement — Stage 3

## 1. Preflight

- [ ] Verify Stage 1 contract is merged and Windows current behavior still matches the research baseline.
- [ ] Capture official source/scope/architecture/signer evidence for each product before enabling execution.
- [ ] Update specs for Windows inventory, installer descriptor, helper protocol and assisted-vs-managed semantics.

## 2. Inventory owner

- [ ] Extend link-safe registry traversal with closed Uninstall/App Paths locations and WOW64 views.
- [ ] Bind per-user reads to the frozen Explorer SID.
- [ ] Add PackageManager and known-path evidence adapters.
- [ ] Replace production PE byte scanning with Win32 version-resource APIs.
- [ ] Add canonical file/package identity, dedup, conflicts and partial-failure projection into Stage 1.

## 3. Product/source policy

- [ ] Add installer scope, interaction mode, supported architecture and signer/install identity policy.
- [ ] Keep Qoder User x64 distinct from any future System installer.
- [ ] Mark WorkBuddy vendor-choice destination honestly.
- [ ] Keep unsupported/unknown combinations disabled/manual.

## 4. Download, signature and helper

- [ ] Reuse streaming download/temp/prepared-package capability.
- [ ] Implement WinVerifyTrust admission and bounded expected-signer policy.
- [ ] Extend the fixed user-helper/bridge protocol with one closed installer operation.
- [ ] Revalidate helper image, interactive context and package pin before side effects.
- [ ] Launch vendor UI with observable process handle and map UAC cancellation separately.
- [ ] Add `awaiting_user/incomplete` job states and truthful cancellation semantics.

## 5. Post-install readback

- [ ] Capture pre/post Stage 1 inventory snapshots.
- [ ] Poll within bounded deadlines after installer termination/hand-off.
- [ ] Require trusted product/version/scope result; do not accept exit code alone.
- [ ] Classify duplicate, no-result, version drift and scope drift explicitly.
- [ ] Keep vendor-assisted operations distinct from managed rollback-capable operations.

## 6. Frontend

- [ ] Reuse shared target/destination picker.
- [ ] Reuse/extend one lifecycle status surface for package verification, UAC, waiting and post-readback.
- [ ] Show product/arch/scope capability honestly; do not show unsupported buttons.
- [ ] Prevent double submission and keep the selected target stable during the job.

## 7. Tests and HIL

- [ ] Fake registry backends for HKU/HKLM, WOW64, links, malformed/bounded values and access denied.
- [ ] Dedup fixtures combining registry/App Paths/known path/package records.
- [ ] File version and signer fixtures; replacement-after-verify rejection.
- [ ] Helper nonce/SID/image/bridge/context drift and process-handle tests.
- [ ] UAC cancelled, process nonzero, process zero/no candidate, duplicate candidate, custom path and stale registry tests.
- [ ] Qoder/TRAE/WorkBuddy x64 disposable Windows HIL; record unsupported architectures separately.
- [ ] Codex PackageManager/helper and Windows runtime security regressions.

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

## Rollback point

Land read-only inventory before installer execution. Enable each product/architecture independently only after its source, signer, helper and post-install HIL pass. A product without complete evidence remains manual/assisted instead of inheriting another product's executor.
