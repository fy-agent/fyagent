# Release, Signing and HIL Plan

Date: 2026-08-31

## 1. Current formal macOS chain

FyAgent already has a public Developer ID policy:

```text
app identifier: com.fyagent.desktop
team identifier: HY446996QX
formal authority: Developer ID Application: William Wang (HY446996QX)
bundle: FyAgent.app
```

Current scripts prepare an isolated keychain, sign the top-level app, create/sign one DMG, submit that DMG to notarytool, staple, remount and verify the app. `verify-macos-signed-app.sh` checks both arm64/x86_64 slices of the main app, exact identifier/team/authority, hardened runtime, timestamp and sealed resources.

This is the correct foundation. It is not yet sufficient for a nested helper/client.

## 2. Required nested artifacts

```text
FyAgent.app/Contents/Frameworks/<client bridge>
FyAgent.app/Contents/Library/LaunchServices/com.fyagent.desktop.system-commit-helper
FyAgent.app/Contents/Info.plist: SMPrivilegedExecutables
```

The helper executable embeds:

- info plist with `CFBundleIdentifier`, `CFBundleVersion`, `SMAuthorizedClients`;
- launchd plist with exact `Label` and `MachServices`;
- no free-form ProgramArguments/business parameters.

## 3. Deterministic build inputs

- checked-in Swift `Package.swift` and `Package.resolved`;
- exact Swift/Xcode toolchain from the formal runner evidence;
- one signing policy source for app/helper identifiers and Team ID;
- app release version as helper `CFBundleVersion`/minimum client version;
- generated product policy snapshots;
- no source mutation/auto-increment during build;
- no runtime/downloaded Swift packages outside package resolution.

## 4. Universal build

Both client and helper must contain arm64 and x86_64 slices because the formal app is verified as universal.

Verifier requirements:

- `lipo -archs` exact expected slices;
- Mach-O executable/dylib type correct;
- no unexpected linked writable/rpath dependency outside system and bundled signed client framework;
- minimum deployment target compatible with macOS 12;
- Swift runtime linkage works when helper runs from `/Library/PrivilegedHelperTools`, not from app bundle;
- helper does not depend on a relative app-bundle library unavailable after bless.

The SwiftAuthorizationSample warning is relevant: a blessed helper is copied outside the app bundle. It must rely on OS-provided Swift runtime/system libraries or be otherwise valid at its installed location.

## 5. Signing order

Formal inside-out sequence:

1. Verify unsigned build inputs/layout and embedded plist source.
2. Sign the client framework/dylib with Developer ID, hardened runtime and timestamp.
3. Sign the helper executable with Developer ID, hardened runtime and timestamp.
4. Inspect helper designated requirement for both architectures.
5. Generate/validate app `SMPrivilegedExecutables` against the signed helper.
6. Generate/validate helper `SMAuthorizedClients` against the signed app policy and minimum version.
7. Verify helper embedded info/launchd plists and label/filename equality.
8. Sign the top-level app using the existing entitlements and policy.
9. Run `codesign --verify --deep --strict --verbose=4` plus explicit nested checks.
10. Build the existing styled DMG.
11. Sign, notarize and staple the final DMG using the current single-submission flow.
12. Mount final DMG and repeat main/client/helper architecture, signature, plist and requirement checks.

Do not rely on `--deep` to sign nested code. It is a verification supplement; each nested code object is signed explicitly.

## 6. Requirement verifier

Automated release tests must prove:

- app `SMPrivilegedExecutables` has exactly the expected helper label;
- its requirement accepts formal helper and rejects wrong identifier/team fixture;
- helper `SMAuthorizedClients` has the formal app identifier/team and minimum client version;
- it accepts current formal app and rejects old/wrong/ad-hoc fixtures;
- helper `CFBundleVersion` is valid and equals expected release/security version;
- helper filename, bundle identifier, launchd label and Mach service are consistent;
- no second helper or unexpected LaunchServices executable exists;
- client bridge is sealed and signed by the same Team;
- no helper entitlement or app entitlement is accidentally widened;
- final notarization ticket is valid on app/DMG as required by current policy.

## 7. CI/release integration

Likely existing surfaces to extend after re-audit:

```text
scripts/release/macos-developer-id.sh
scripts/release/verify-macos-signed-app.sh
scripts/release/macos-signing-policy.sh
scripts/tasks/host-native.mjs
scripts/tasks/supported-platform-check.mjs
.github/workflows/ci.yml
.github/workflows/release.yml
tests/releaseWorkflow.test.ts
change classifier and structure assets
```

Rules:

- no second signing/notary workflow;
- no long-lived keychain or certificate artifact;
- formal secrets remain confined to existing formal job;
- preflight builds helper/client and validates structure without claiming real helper install;
- release evidence includes exact Swift package revisions and helper/client hashes for traceability, not remote admission;
- cleanup removes temporary keychain/build artifacts.

## 8. Development testing levels

### Level 0 — pure tests

- Swift/Rust unit tests;
- fake filesystem/transaction/fault injection;
- generated policy and protocol tests;
- no signature claim.

### Level 1 — local signed fixture

- locally signed app/helper using a stable development or dedicated self-signed test identity;
- XPC mutual identity and helper lifecycle experiments;
- never promoted as formal compatibility evidence.

### Level 2 — Developer ID signed HIL

- exact formal Team/identifier/requirements;
- real SMJobBless/admin auth/root XPC;
- app installed from final/notarized artifact where possible.

### Level 3 — formal notarized release candidate

- final DMG from release workflow;
- notary accepted/stapled;
- full matrix and post-install inventory evidence;
- only this level may enable production system target.

## 9. HIL matrix

| Area | Cases |
|---|---|
| OS/arch | macOS 12 + current; Apple Silicon; universal x86_64 verification; Intel HIL when available |
| Helper install | missing, success, cancel, wrong credential, permanently disabled, signature/plist failure |
| Helper version | older update, equal no-op, installed newer downgrade reject, protocol incompatible |
| Peer auth | correct, wrong Team, wrong identifier, ad-hoc, old app, tampered helper |
| App commit | fresh fixed slot, update exact fixed slot, target absent/drifted/running |
| Source | valid FD, path replaced after open, wrong product, symlink/special file, source drift |
| Transaction | copy failure, disk full, rename failure, verify failure, rollback success/uncertain |
| Crash recovery | kill at every receipt phase, restart health, cleanup only owned artifacts |
| Readback | exact path/scope/version, no undeclared duplicate, readback failure not green |
| Removal | active transaction reject, authorized removal, cancellation, reinstall |
| UX | clear helper/app stages and reasons, no silent user-scope fallback |

## 10. Current environment limitation

The sibling task's read-only spike reported no suitable local Developer ID identity/profile/secrets for a formal helper HIL. This task therefore records formal signed HIL as a future implementation gate, not a planning claim.

If implementation reaches the gate without the required environment:

- complete portable code/tests and structure verifier honestly;
- keep the system destination disabled;
- report exact missing identity/runner/host evidence;
- do not substitute ad-hoc signing or mock authorization for production proof.

## 11. Rollback and release disablement

If a released helper has a security or reliability issue:

- backend capability disables system actions in the next app release;
- new helper version raises minimum client version and rejects vulnerable app versions;
- explicit removal remains available when safe;
- existing applications in `/Applications` are not automatically moved/deleted;
- no fallback to sudo/AppleScript/user-scope disguised as success;
- future SMAppService migration is separately reviewed and does not silently coexist.
