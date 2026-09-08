# Research: G1 Apple `/Applications` authorization spike

- **Query**: Remaining G1 / Apple `/Applications` authorization evidence: entitlements, Developer ID requestability, objc2 `NSWorkspace.requestAuthorization` / authorized `FileManager`, whether a signed Developer ID HIL can run here, and confirmed forbidden shortcuts.
- **Scope**: mixed (repository signing pipeline + local Mac observation + Apple SDK/docs)
- **Date**: 2026-08-31
- **Extends**: `research/macos-authorization-options.md` (does not replace it)

Numbering note: PRD **G1** is the reuse/owner-convergence gate; PRD **G2** is system-commit adapter selection. `wave-ownership.md` labels this Apple-authorization spike **G1**. Rows below use `G1-AUTH-*` so they do not collide with reuse checkboxes.

## Findings

### Files Found

| File Path | Description |
|---|---|
| `src-tauri/entitlements.macos.plist` | Checked-in macOS entitlements used by Tauri bundle config and formal `codesign --entitlements`. |
| `src-tauri/tauri.conf.json` | `bundle.macOS.entitlements` = `entitlements.macos.plist`; `hardenedRuntime` true; `minimumSystemVersion` `12.0`. |
| `scripts/release/macos-developer-id.sh` | Formal `sign-app` copies that plist; no provisioning-profile embed step. |
| `scripts/release/macos-signing-policy.sh` | Public identity: `Developer ID Application: William Wang (HY446996QX)`, team `HY446996QX`, bundle `com.fyagent.desktop`. |
| `scripts/release/verify-macos-signed-app.sh` | Verifies Developer ID identity, hardened runtime, timestamp, dual arch; does not inspect entitlement keys or `embedded.provisionprofile`. |
| `.github/workflows/release.yml` | Formal `build-macos` is the only job that imports Apple cert secrets and calls `sign-app` / notarize / staple. Preflight does not receive those secrets. |
| `.trellis/spec/backend/github-release-workflow.md` | Contract: formal mode reseals with the checked-in entitlements; no privileged-file-operations or profile language. |
| `src-tauri/src/agent_install/macos.rs` | Fresh `MacSystemApplications` and all-users update return `AgentReasonCode::AuthorizationRequired`. |
| `src-tauri/src/agent_install/inventory.rs` | System destination is listed with `actionable: false` and `AuthorizationRequired`; user `~/Applications` remains the executable fresh destination. |
| `src-tauri/Cargo.toml` | Direct dep `objc2-app-kit` `0.2` with feature `NSColor` only. |

No `*.provisionprofile` / `embedded.provisionprofile` exists in the repository. No `com.apple.developer.security.privileged-file-operations` string exists outside this task’s docs.

### Code Patterns

#### 1. Current entitlements (unrestricted hardened-runtime exceptions only)

```1:12:src-tauri/entitlements.macos.plist
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>com.apple.security.cs.allow-jit</key>
  <true/>
  <key>com.apple.security.cs.allow-unsigned-executable-memory</key>
  <true/>
  <key>com.apple.security.cs.disable-library-validation</key>
  <true/>
</dict>
</plist>
```

`com.apple.developer.security.privileged-file-operations` is absent.

#### 2. Formal pipeline preserves whatever is in that plist; it does not add a profile

`scripts/release/macos-developer-id.sh` `sign_app` (lines 294–304):

```text
codesign --force \
  --sign "$EXPECTED_AUTHORITY" \
  --options runtime \
  --timestamp \
  --entitlements "$ENTITLEMENTS" \
  "$app_path"
```

`$ENTITLEMENTS` is `src-tauri/entitlements.macos.plist`. There is no copy of a Developer ID provisioning profile to `FyAgent.app/Contents/embedded.provisionprofile`.

`.trellis/spec/backend/github-release-workflow.md` §6 macOS: formal `build-macos` reseals with Developer ID + hardened runtime + secure timestamp + the checked-in entitlements, then notarizes the DMG once and staples. Preflight candidate code receives no Apple Developer ID/notarization secrets.

`tests/releaseWorkflow.test.ts` freezes `tauri.conf.json` entitlements path as `entitlements.macos.plist` and asserts the macOS job does not use ad-hoc `codesign --force --sign -`. It does not freeze privileged-file-operations or a profile embed.

#### 3. Shipped Developer ID app matches the plist (notarization did not add the restricted entitlement)

Observed 2026-08-31 on this Mac, `/Applications/FyAgent.app` version `0.4.2`, `LSMinimumSystemVersion` `12.0`:

- Identity: `Developer ID Application: William Wang (HY446996QX)`, `TeamIdentifier=HY446996QX`, CodeDirectory `flags=0x10000(runtime)`.
- Entitlements (via `codesign -d --entitlements -`): the same three `com.apple.security.cs.*` keys as the plist.
- `Contents/embedded.provisionprofile`: missing.

So the current unrestricted entitlements survive this repo’s sign → notary → staple path. The restricted privileged-file-operations key is not present after notarization because it was never claimed.

#### 4. Production `/Applications` commit is still `authorization_required`

```177:179:src-tauri/src/agent_install/macos.rs
            super::inventory::FreshDestinationCapability::MacSystemApplications => {
                return Err(AgentReasonCode::AuthorizationRequired)
            }
```

All-users existing-target update at lines 163–166 returns the same code. Inventory exposes the system destination as non-actionable (`inventory.rs` ~866–879). There is no `MacSystemCommitPort` implementation, no `NSWorkspace.requestAuthorization` call site in `src-tauri/`, and no sudo / AppleScript-admin / SMJobBless path under `src-tauri/src/agent_install/`.

### Apple API and entitlement (SDK + official docs)

#### Entitlement

Apple Bundle Resources docs (JSON snapshot 2026-08-31, copyright 2026):

- Key: `com.apple.developer.security.privileged-file-operations` (boolean).
- Abstract: “An entitlement that permits apps to create symbolic links, replace files, and set file attributes.”
- Platform metadata: macOS, `introducedAt` **10.15**.
- Usage: add the entitlement, then call `NSWorkspace.requestAuthorization(to:completionHandler:)`, then `FileManager.init(authorization:)`.
- Request: [Apple privileged-file-operations request form](https://developer.apple.com/contact/request/privileged-file-operations/).

[Supported capabilities (macOS)](https://developer.apple.com/help/account/reference/supported-capabilities-macos/) lists standard ADP / Developer ID / Apple Developer capabilities. **Privileged File Operations is not a row on that table.** It is a managed/special entitlement requested by form, not a self-serve Developer ID checkbox.

Apple Developer Forums thread 130092 (2020, with DTS follow-up): Developer ID Additional Entitlements UI did not offer this entitlement for a non–Mac App Store profile. DTS written reply quoted on that thread: generally the entitlement is for Mac App Store apps; DTS can enable it for a specific Developer ID app. Thread 655277: after approval, the entitlement is selected on the provisioning-profile “Do you need additional entitlements?” page; the template is profile-type specific.

[TN3125](https://developer.apple.com/documentation/technotes/tn3125-inside-code-signing-provisioning-profiles): macOS unrestricted entitlements (hardened runtime, App Sandbox configuration) can be claimed without a profile; **restricted entitlements must be authorized by a provisioning profile**. macOS supports Developer ID profiles; “Some entitlements are not supported by Developer ID profiles.” TN3125 does not name privileged-file-operations specifically.

#### Authorized FileManager method set (macOS 26.5 SDK header)

Source: `MacOSX.sdk/System/Library/Frameworks/AppKit.framework/Headers/NSWorkspace.h` (Command Line Tools SDK 26.5 on this host).

```objc
typedef NS_ENUM(NSInteger, NSWorkspaceAuthorizationType) {
    NSWorkspaceAuthorizationTypeCreateSymbolicLink,
    NSWorkspaceAuthorizationTypeSetAttributes,
    NSWorkspaceAuthorizationTypeReplaceFile,
} API_AVAILABLE(macos(10.14));

/* Only the following NSFileManager methods currently take advantage of an authorization:
   -createSymbolicLinkAtURL:withDestinationURL:error:
   -setAttributes:ofItemAtPath:error:
   -replaceItemAtURL:withItemAtURL:backupItemName:options:resultingItemURL:error:
   ...
   Also note that for -replaceItemAtURL:..., the backupItemName and options parameters will be ignored.
   All other NSFileManager methods invoked on this instance will behave normally.
 */
+ (instancetype)fileManagerWithAuthorization:(NSWorkspaceAuthorization *)authorization API_AVAILABLE(macos(10.14));
```

`NSFileManager.h` documents `-replaceItemAtURL:withItemAtURL:...` as replacing an **existing** `originalItemURL`. `newItemURL` is supposed to sit in a temporary directory (or a uniquely named directory next to the original). Authorized `setAttributes` may only change `NSFileOwnerAccountID`, `NSFileGroupOwnerAccountID`, and `NSFilePosixPermissions`.

There is no `NSWorkspaceAuthorizationTypeCopyItem` / `CreateDirectory` in this SDK. Copy/create of an absent `/Applications/<App>.app` is therefore outside the methods that “take advantage of an authorization.” Other `NSFileManager` methods on that instance “behave normally” (no extra privilege).

`FileManager.init(authorization:)` / `+fileManagerWithAuthorization:` and `requestAuthorizationOfType:completionHandler:` are `API_AVAILABLE(macos(10.14))`, which includes the project minimum macOS 12.0. That is availability of the symbols, not proof that a FyAgent Developer ID package can obtain or use the entitlement.

Apple DTS on forums thread 127371 described `NSWorkspaceAuthorization` as limited and not a general admin `FileManager` for copying into protected locations.

### objc2-app-kit

| Crate | Where | Authorization bindings |
|---|---|---|
| `objc2-app-kit` **0.2.2** | Direct lock for `src-tauri` (`objc2-app-kit = { version = "0.2", features = ["NSColor"] }`) | `NSWorkspaceAuthorizationType::{CreateSymbolicLink, SetAttributes, ReplaceFile}`; `NSWorkspace::requestAuthorizationOfType_completionHandler` behind features **`NSWorkspace`** and **`block2`**; `NSFileManagerNSWorkspaceAuthorization::fileManagerWithAuthorization`. |
| `objc2-app-kit` **0.3.2** | Transitive (other crates in the lockfile) | Same three authorization types; same `requestAuthorizationOfType_completionHandler` / `fileManagerWithAuthorization`. No additional copy/create type. |

The production `src-tauri` crate currently enables only `NSColor` on 0.2. The authorization selectors exist in the crate sources under `~/.cargo/registry/.../objc2-app-kit-0.2.2/src/generated/NSWorkspace.rs` (approx. lines 396–465) but are not compiled into FyAgent unless those features are enabled. Symbol availability in objc2 matches the 10.14 SDK enum; it does not expand Apple’s authorized method set.

### Signed Developer ID HIL in this environment

| Check | Result |
|---|---|
| `security find-identity -v -p codesigning` | `0 valid identities found` |
| `FYAGENT_APPLE_CERTIFICATE_P12_BASE64` / `PASSWORD` / `FYAGENT_APPLE_ID` / `FYAGENT_APPLE_APP_SPECIFIC_PASSWORD` | UNSET in this agent environment |
| Repo provisioning profile | none |
| Formal Release secrets | specified only for GitHub Actions formal `build-macos`; preflight explicitly has none |
| Host OS | macOS 26.6.2 (Build 25G83) — current supported macOS on this machine; **not** a macOS 12 runner |
| Unsigned/debug `cargo` / Tauri debug build | not executed as proof; task/PRD forbid substituting it for signed HIL |

Exact blocker: this environment cannot produce or launch a Developer ID–signed, privileged-file-operations–provisioned, notarized FyAgent (or spike app) because there is no local signing identity, no Apple secrets, and no embedded Developer ID profile that allowlists the restricted entitlement. An unsigned debug build would not be HIL evidence.

A later signed HIL, if ever run, would still have to demonstrate operations the SDK says are unauthorized for this FileManager (fresh absent-target create via copy/createDirectory), plus replace/rollback/cancel on macOS 12 and current OS.

### Forbidden shortcuts (confirmed as out of this path)

Task documents (`macos-authorization-options.md`, PRD D6, design §8.3, `wave-ownership.md`) list and reject:

- `sudo`
- `osascript` … `with administrator privileges`
- deprecated `AuthorizationExecuteWithPrivileges`
- generic privileged helper / arbitrary root XPC or file manager
- silent `~/Applications` fallback while reporting system success
- unsigned development proof as signed/notarized HIL

Current `agent_install` macOS executor does not implement those shortcuts for system commit: it returns `AuthorizationRequired`. Existing `osascript` uses elsewhere (terminal launch, Codex running-app probes) are not `/Applications` install elevation.

### Wave 2 implication (fact from this spike + wave contract)

`wave-ownership.md`: “`/Applications` one-click stays disabled until signed HIL proves G1. Do not enable system commit in Wave 1.” Wave 2 destination ranking is “still disabled until G1.”

G1 native authorization is **not** signed-HIL proven. Entitlement is absent from the Developer ID package. SDK headers document that authorized `FileManager` cannot copy/create an absent `.app` in `/Applications`. Therefore Wave 2 keeps the system target disabled/manual (`authorization_required`), with no user-scope fallback presented as system success.

This spike does not run Gate B (Blessed/SecureXPC). Helper selection remains a later G2/Phase 3B question; it is not opened by Wave 2.

## External References

- [Privileged File Operations entitlement](https://developer.apple.com/documentation/bundleresources/entitlements/com.apple.developer.security.privileged-file-operations) — key, abstract, request form; macOS 10.15 in Apple’s platform metadata.
- [requestAuthorization(to:completionHandler:)](https://developer.apple.com/documentation/appkit/nsworkspace/requestauthorization(to:completionhandler:))
- [FileManager.init(authorization:)](https://developer.apple.com/documentation/foundation/filemanager/init(authorization:)) — macOS 10.14.
- [Entitlement request form](https://developer.apple.com/contact/request/privileged-file-operations/)
- [Supported capabilities (macOS)](https://developer.apple.com/help/account/reference/supported-capabilities-macos/) — no Privileged File Operations row.
- [TN3125 Inside Code Signing: Provisioning Profiles](https://developer.apple.com/documentation/technotes/tn3125-inside-code-signing-provisioning-profiles) — restricted entitlements need a profile; some entitlements are unsupported on Developer ID profiles.
- [Capability requests](https://developer.apple.com/help/account/capabilities/capability-requests/) — Account Holder submits managed capability requests.
- Apple Developer Forums [130092](https://developer.apple.com/forums/thread/130092), [655277](https://developer.apple.com/forums/thread/655277), [127371](https://developer.apple.com/forums/thread/127371) — DTS: entitlement generally Mac App Store; Developer ID enablement is manual; authorized FileManager is a limited option.
- objc2-app-kit 0.2.2 / 0.3.2 generated `NSWorkspace.rs` in the local Cargo registry.

## Related Specs

- `.trellis/spec/backend/github-release-workflow.md` — Developer ID + notarize-once; checked-in entitlements; preflight unsigned.
- `.trellis/spec/backend/secretref-backend.md` — Developer ID/notarization must not be assumed to supply a provisioning-profile identity for restricted entitlements (stated there for Data Protection Keychain; same profile mechanics apply here).
- `.trellis/spec/backend/macos-dmg-layout.md` — DMG `Applications` symlink is Finder drag-install, not an in-app system-commit adapter.

## Caveats / Not Found

- This session did not submit the Apple entitlement request form and did not inspect team `HY446996QX` in Certificates, Identifiers & Profiles (no portal access from the agent).
- Notary behavior if someone added the restricted key **without** a matching Developer ID profile was not reproduced; AMFI/profile rules are documented, notarization rejection of that mis-sign was not measured.
- Apple’s HTML capability table checkmarks did not survive markdown conversion; the factual observation used here is the **absence of the capability name**, not the ADP vs Developer ID check columns for other rows.
- macOS 12 HIL hardware/runner was not available; this host is macOS 26.6.2.
- Forums threads are 2020–era DTS quotes; they are primary-engineer statements, not a 2026 Apple Help matrix row.
- Gate B helper evidence is intentionally out of scope for this file.
