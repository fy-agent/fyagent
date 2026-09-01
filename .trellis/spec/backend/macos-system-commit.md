# macOS Privileged System-Commit Helper Contract

## 1. Scope / Trigger

Read this contract before changing the nested macOS privileged helper, its
Swift package, C ABI, `MacSystemCommitPort`, product/slot integers, Bless /
Authorization rights, nested Developer ID signing, or the production
`/Applications` enablement gate.

This owner is the last-write boundary for known desktop products in a fixed
`/Applications/<KnownName>.app` slot. Download, DMG mount, identity, inventory,
job stages, and user-scope `~/Applications` transactions stay on
[One-click Executable Software Installer](./codex-desktop-installer.md) and
[External Agent P0 Safety](./external-agent-p0.md). Renderer IPC stays on the
existing closed Agent/Codex actions; this module adds no Tauri command.

Production `/Applications` actions stay disabled until formal Developer ID
signing, notarization, and real-machine HIL. Code plus portable tests may
exist while `production_enabled() == false`. That state is not a delivered
system one-click install.

The Swift package is not a Cargo workspace member. Workspace members remain
exactly `[".", "user-helper"]`.

## 2. Signatures

```text
app identifier:     com.fyagent.desktop
team identifier:    HY446996QX
helper identifier:  com.fyagent.desktop.system-commit-helper
mach service:       com.fyagent.desktop.system-commit-helper
helper path:        FyAgent.app/Contents/Library/LaunchServices/com.fyagent.desktop.system-commit-helper
client path:        FyAgent.app/Contents/Frameworks/libFyAgentPrivilegedClient.dylib
commit right:       com.fyagent.desktop.system-application.commit
remove right:       com.fyagent.desktop.privileged-helper.remove
receipt directory:  /Library/Application Support/FyAgent/SystemCommit/v1
stage name:         .fyagent-system-stage-<uuid>.app
backup name:        .fyagent-system-backup-<uuid>.backup
protocol version:   1
ABI version:        1
```

Helper `CFBundleVersion` equals the workspace Cargo version. `version:set`
still writes only Cargo.toml and the two local lock blocks; the same change
that bumps the canonical version must update
`src-tauri/macos-privileged-helper/Resources/helper-info.plist` and the
`SMPrivilegedExecutables` requirement in `src-tauri/Info.plist`. Do not add
the Swift package as a Cargo member.

C header: `src-tauri/macos-privileged-helper/include/fyagent_privileged_bridge.h`

```c
enum { FYAGENT_PRIVILEGED_ABI_VERSION = 1 };

typedef struct FyAgentPrivilegedRequest {
  uint32_t abi_version;
  uint32_t size;
  uint32_t protocol_version;
  uint32_t operation;   /* 1=status 2=ensure_helper 3=commit 4=remove_helper */
  uint32_t action;      /* 1=fresh_install 2=update_existing; 0 otherwise */
  uint32_t product;     /* frozen table */
  uint32_t target_slot; /* frozen table */
  uint32_t reserved0;
  uint8_t  operation_id[16];
  uint8_t  expected_source_revision[32];
} FyAgentPrivilegedRequest;

int32_t fyagent_privileged_invoke(
  const FyAgentPrivilegedRequest *request,
  /* bounded response buffer; no path/URL/command/Authorization bytes */
);
```

Rust crate-private port:

```rust
pub trait MacSystemCommitPort {
    fn helper_status(&self) -> HelperStatus;
    fn production_enabled(&self) -> bool;
    fn ensure_helper_ready(&self, intent: UserIntent) -> Result<HelperStatus, AgentReasonCode>;
    fn commit_known_application(
        &self,
        commit: AuthorizedSystemCommit,
    ) -> Result<SystemCommitOutcome, AgentReasonCode>;
    fn remove_helper(&self, intent: UserIntent) -> Result<(), AgentReasonCode>;
}

pub fn system_scope_rejection() -> AgentReasonCode; // AuthorizationRequired while disabled
```

Product integers (C ABI / XPC; display names never travel):

| value | product         | expected bundle ID        | fresh `/Applications` basename | extra existing-only slots |
| ----- | --------------- | ------------------------- | ------------------------------ | ------------------------- |
| 1     | CodexDesktop    | `com.openai.codex`        | `ChatGPT.app`                  | `Codex.app`               |
| 2     | OpenCodeDesktop | `ai.opencode.desktop`     | `OpenCode.app`                 | none                      |
| 3     | QoderWork       | `com.qoder.work.cn`       | `QoderWork CN.app`             | none                      |
| 4     | TraeWork        | `cn.trae.solo.app`        | `TRAE SOLO CN.app`             | none                      |
| 5     | WorkBuddy       | `com.workbuddy.workbuddy` | `WorkBuddy.app`                | none                      |

Codex slots: `1` = ChatGPT.app (fresh default), `2` = Codex.app (existing
only). Every other product uses slot `1` as its single fresh-default basename.
Unknown product/slot is rejected before mutation. Claude Desktop is not in
this table; user-scope Claude install stays on the Agent desktop path.

## 3. Contracts

- One production registration path: Blessed `SMJobBless` on macOS 12+. Do not
  add a runtime fallback to a second elevation mechanism.
- Open-source pins: Blessed `0.6.0`, Authorized `1.0.0`, EmbeddedPropertyList
  `2.0.2`, Required `0.1.1`, SecureXPC revision
  `1cece54562c7626d042f007d2f38cfe325565850`. Commit `Package.resolved`. Do
  not vendor a second XPC/Bless stack.
- Helper operations are only status, ensure helper, commit known slot, and
  remove helper. Request and renderer IPC must not contain a path, URL,
  command, Authorization bytes, or free-form filename.
- Copy inside the helper must not spawn `ditto`, `/bin/sh`, or `Process` for
  the transaction. Prefer FD/`openat` copy that does not follow attacker-
  controlled symlinks. Stage and backup names are the frozen templates above.
- `agent_install` and Codex reuse `MacSystemCommitPort` after user-scope
  staging validation. They do not talk to XPC directly and do not invent a
  second helper.
- `ProductionMacSystemCommitPort.production_enabled()` is `false` until signed
  / notarized HIL. While false, inventory and deploy adapters call
  `system_scope_rejection()` → `authorization_required`. `commit_known_application`
  must not claim helper success. Missing packaging maps ensure/remove to
  `helper_not_packaged`.
- Job stages stay the existing Agent/Codex set. Do not add helper-specific
  stages. Reuse `awaiting_user` for Authorization UI.
- Formal `build-macos` builds the SPM package, embeds the helper and client at
  the frozen paths, runs `verify-macos-privileged-helper.sh --structure-only`,
  then `macos-developer-id.sh sign-app`. Signing is inside-out: client dylib,
  then helper executable, then the app. Do not `--deep` nested code. The
  published subject remains one notarized DMG; the helper is not a second
  Apple submission.
- Formal signing requires both nested binaries. `FYAGENT_ALLOW_APP_ONLY_SIGN=1`
  is local/diagnostic only.
- `.build/` and `dist/` under `src-tauri/macos-privileged-helper/` are
  gitignored build outputs.

## 4. Validation & Error Matrix

| Condition                                                                  | Required result                                                                                        |
| -------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------ |
| `production_enabled() == false` and a system target is selected            | `authorization_required`; zero `/Applications` write; no `~/Applications` relabel                      |
| Helper binary or client dylib missing in production invoke                 | `helper_not_packaged` for ensure/remove; commit still `authorization_required` while disabled          |
| Unknown product integer or slot                                            | `target_slot_invalid`; zero mutation                                                                   |
| User cancels Bless or commit Authorization                                 | `helper_install_authorization_cancelled` / `operation_authorization_cancelled`; existing app unchanged |
| Invalid or expired Authorization                                           | `operation_authorization_invalid`                                                                      |
| Helper signature, peer, or protocol mismatch                               | `helper_signature_invalid` / `helper_peer_rejected` / `helper_protocol_incompatible`                   |
| Installed helper is older than the bundled requirement                     | `helper_update_required`                                                                               |
| Installed helper is newer than this app                                    | `helper_downgrade_rejected`                                                                            |
| Source capability drifted before commit                                    | `source_capability_invalid` / `source_changed`                                                         |
| Commit fails after backup and restore succeeds                             | `rollback_restored`                                                                                    |
| Commit fails and authority of the slot is unknown                          | `recovery_required`                                                                                    |
| Renderer sends path/URL/command/Authorization bytes                        | Reject at Agent/Codex IPC; helper never sees it                                                        |
| Swift package added as a Cargo workspace member                            | `version:check` / workspace contract fails                                                             |
| Formal `sign-app` without both nested binaries                             | Fail; do not sign an app-only bundle                                                                   |
| `sudo`, AppleScript admin, `AuthorizationExecuteWithPrivileges`, or setuid | Forbidden; tests and review reject                                                                     |

## 5. Good / Base / Bad Cases

- Good: user-scope OpenCode install writes `~/Applications/OpenCode.app` with
  the existing DMG transaction; a selected `/Applications` target stays
  `authorization_required` while production is disabled.
- Base: portable Swift tests and Rust `macos_system_commit` tests cover the
  product table, ABI layout, and fail-closed port without Blessing a helper.
- Bad: enabling `production_enabled()` because the helper compiled; labeling
  `~/Applications` success as a system install; adding Claude to the helper
  table without a reviewed slot; sending a filesystem path over XPC.

## 6. Tests Required

- `cargo test --lib macos_system_commit`: product/slot table, production
  `production_enabled() == false`, `system_scope_rejection()` is
  `authorization_required`.
- `swift run PrivilegedHelperTests` in `src-tauri/macos-privileged-helper`
  (custom test executable; do not treat `swift test` XCTest discovery as the
  owner).
- `tests/releaseWorkflow.test.ts`: `build-macos` runs
  `build-macos-privileged-helper.sh`, `embed-macos-privileged-helper.sh`, and
  `verify-macos-privileged-helper.sh --structure-only` before `sign-app`.
- Negative: no renderer path/URL/command; no `sudo` / `osascript` elevation;
  Cargo workspace members stay `[".", "user-helper"]`.
- Signed/notarized `/Applications` HIL is required before flipping
  `production_enabled()`. Portable tests do not substitute for that evidence.

## 7. Wrong vs Correct

#### Wrong

```rust
if target.scope == SystemApplications {
    install_to_user_applications(bundle); // silent fallback
}
```

#### Correct

```rust
if target.scope == SystemApplications {
    return Err(macos_system_commit::system_scope_rejection());
}
```

#### Wrong

```c
typedef struct {
  char path[1024];
  char command[256];
} HelperRequest;
```

#### Correct

```c
/* product + target_slot integers only; no path/command/URL */
uint32_t product;
uint32_t target_slot;
```

## Signed development runtime and formal admission

The privileged transaction implementation is shared by development and formal
packages. It has three backend-selected runtime modes:

- `Disabled`: an unsigned or unlinked build. `/Applications` remains
  ineligible and returns `authorization_required`; it must not fall back to a
  per-user destination.
- `DevelopmentSigned`: `mise run dev` builds the existing Swift helper/client,
  links the client C ABI into the Rust executable, assembles a real
  `com.fyagent.desktop.dev` app bundle, and signs every nested executable
  inside-out with the existing
  `Developer ID Application: William Wang (HY446996QX)` identity. The local
  task extracts the configured PKCS#12 into a 0700 per-run directory, converts
  the same private key to traditional RSA PEM for macOS Keychain import, and
  imports that key plus the leaf, pinned Developer ID G2, and Apple Root public
  certificates only into an ephemeral keychain. It signs by the resolved
  identity hash, restores the original keychain search list, and deletes every
  extracted key/certificate file together with the temporary keychain. The
  repository contains neither the PKCS#12 path nor its password; a
  permission-restricted user-local configuration points to those files.
  Development signing does not notarize or staple the app.
- `FormalRelease`: the production helper/client can be linked and packaged, but
  transaction admission remains closed until a Developer ID, notarized HIL
  candidate passes Bless, root XPC, fresh install, update, rollback, recovery,
  and helper removal/upgrade acceptance.

Development uses the separate helper, Mach service, authorization-right, and
receipt namespaces rooted at
`com.fyagent.desktop.dev.system-commit-helper`. Production retains
`com.fyagent.desktop.system-commit-helper`. Both flavors are generated from the
same Blessed, Authorized, SecureXPC, policy, protocol, and transaction sources;
there is no second helper implementation.

Runtime admission is a Rust compile-time capability plus the linked client
artifact. Renderer input and process runtime environment variables cannot turn
it on. The signed development task owns `DEVELOPER_DIR`, compiler/linker/runner
variables, helper artifact paths, and the Tauri target; caller overrides are
rejected.

XPC waiting is operation-specific. Read-only status uses a short bounded wait.
Mutating commit/remove operations use their own bounded waits. A mutating wait
expiry means the root operation may still be running and therefore maps to
`recovery_required` and an `Incomplete` job, never to an ordinary
`helper_unavailable` or `Failed` result. Recovery and authoritative inventory
re-read remain mandatory before another mutation.
