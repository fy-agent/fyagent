# Implementation seam freeze

Date: 2026-08-31
Baseline: `1d0aeecc5b4cff9dc914907f24a7ed321daff75b` on `dev/laiyongjie`

Parallel implementers must treat this file as the shared ABI. Do not invent a
second path, command, or product table.

## Identifiers

```text
app identifier:              com.fyagent.desktop
team identifier:             HY446996QX
helper identifier:           com.fyagent.desktop.system-commit-helper
mach service:                com.fyagent.desktop.system-commit-helper
helper bundle path:          FyAgent.app/Contents/Library/LaunchServices/com.fyagent.desktop.system-commit-helper
client artifact path:        FyAgent.app/Contents/Frameworks/libFyAgentPrivilegedClient.dylib
commit right:                com.fyagent.desktop.system-application.commit
remove right:                com.fyagent.desktop.privileged-helper.remove
receipt directory:           /Library/Application Support/FyAgent/SystemCommit/v1
stage name:                  .fyagent-system-stage-<uuid>.app
backup name:                 .fyagent-system-backup-<uuid>.backup
protocol version:            1
```

Helper `CFBundleVersion` equals the workspace Cargo version (`0.4.2` at
baseline). Do not invent a source-hash auto-increment. Do not add a Cargo
workspace member; Swift stays an SPM package under
`src-tauri/macos-privileged-helper/`.

## Product numeric enum (frozen)

These integers are the C ABI / XPC wire values. Display names never travel.

| value | product            | expected bundle ID              | fresh `/Applications` basename | extra existing-only slots |
| ----- | ------------------ | ------------------------------- | ------------------------------ | ------------------------- |
| 1     | CodexDesktop       | `com.openai.codex`              | `ChatGPT.app`                  | `Codex.app`               |
| 2     | OpenCodeDesktop    | `ai.opencode.desktop`           | `OpenCode.app`                 | none                      |
| 3     | QoderWork          | `com.qoder.work.cn`             | `QoderWork CN.app`             | none                      |
| 4     | TraeWork           | `cn.trae.solo.app`              | `TRAE SOLO CN.app`             | none                      |
| 5     | WorkBuddy          | `com.workbuddy.workbuddy`       | `WorkBuddy.app`                | none                      |

Target-slot integers are product-local:

```text
CodexDesktop:  1 = ChatGPT.app (fresh default), 2 = Codex.app (existing only)
others:        1 = the single fresh-default basename
```

Unknown product/slot => reject before mutation. Do not add Claude Desktop in
this task.

Version source/equivalence stays with the existing owners:

- Codex / OpenCode / QoderWork / WorkBuddy: Info.plist; WorkBuddy dotted-prefix
- TRAE Work: bounded `product.json` tronBuildVersion; exact

Helper revalidates the same closed identity; it does not download or parse
release metadata.

## C ABI (crate-private, macOS only)

Header: `src-tauri/macos-privileged-helper/include/fyagent_privileged_bridge.h`

```c
enum {
  FYAGENT_PRIVILEGED_ABI_VERSION = 1
};

typedef struct FyAgentPrivilegedRequest {
  uint32_t abi_version;
  uint32_t size;
  uint32_t protocol_version;
  uint32_t operation;          /* 1=status 2=ensure_helper 3=commit 4=remove_helper */
  uint32_t action;             /* 1=fresh_install 2=update_existing; 0 otherwise */
  uint32_t product;            /* frozen table */
  uint32_t target_slot;        /* frozen table */
  uint32_t reserved0;
  uint8_t  operation_id[16];   /* UUID bytes */
  uint8_t  expected_source_revision[32];
  uint8_t  expected_target_revision[32];
  int32_t  source_directory_fd; /* -1 when unused */
  uint32_t reserved1;
} FyAgentPrivilegedRequest;

typedef struct FyAgentPrivilegedReply {
  uint32_t abi_version;
  uint32_t size;
  uint32_t protocol_version;
  uint32_t outcome;            /* 1=committed 2=rollback_restored 3=recovery_required 4=ready 5=failed */
  uint32_t reason;             /* closed helper reason; 0=none */
  uint32_t helper_state;       /* 1=ready 2=update_required 3=incompatible 4=recovery_required 5=missing */
  uint32_t reserved0;
  uint8_t  operation_id[16];
} FyAgentPrivilegedReply;

int fyagent_privileged_invoke(
  const FyAgentPrivilegedRequest *request,
  FyAgentPrivilegedReply *reply
);
```

Rules:

- No JSON, path, URL, command, argv, Team ID, or Authorization bytes in this
  struct. Authorization lives only inside the Swift bridge/XPC payload.
- Rust never logs the FD, requirement string, or Authorization external form.
- Non-macOS builds compile a Rust stub that returns a stable “not packaged”
  reason and never links Swift.

## Rust facade

Module: `src-tauri/src/macos_system_commit/` (crate-private, `mod` in `lib.rs`).

```text
MacSystemCommitPort
  helper_status()
  production_enabled() -> bool   // false in this task
  ensure_helper_ready(user_intent)
  commit_known_application(AuthorizedSystemCommit)
  remove_helper(user_intent)
```

`AuthorizedSystemCommit` is constructed only after inventory revalidation. It
is not `Serialize` and is not a Tauri command argument.

`production_enabled()` remains `false`. Tests inject a fake port. Inventory
keeps `MacSystemApplications` as `authorization_required` on the production
path.

## Wire reasons (closed, bump contracts once)

Add these `AgentReasonCode` / frontend parser values. Reuse existing job
stages (`checking`, `awaiting_user`, `staging`, `installing`,
`verifying_installation`, `cancelled`, `failed`, `incomplete`) instead of
adding helper-specific stages.

```text
helper_not_packaged
helper_signature_invalid
helper_install_authorization_cancelled
helper_install_failed
helper_update_required
helper_downgrade_rejected
helper_protocol_incompatible
helper_peer_rejected
operation_authorization_cancelled
operation_authorization_invalid
source_capability_invalid
source_changed
target_slot_invalid
helper_removal_failed
```

Bump `AGENT_INSTALL_READINESS_CONTRACT_VERSION` and
`AGENT_ACTION_CONTRACT_VERSION` from 3 to 4 together with the TypeScript
strict parser. Copy stays user-facing; no paths, URLs, or requirement strings.

Map internal helper outcomes onto existing
`rollback_restored` / `recovery_required` / `cancelled` /
`application_running` / `permission_denied` / `authorization_required` when
those already describe the user-visible fact. Use the new codes when the
failure is helper-specific.

## Swift package layout

```text
src-tauri/macos-privileged-helper/
  Package.swift
  Package.resolved
  Sources/
    FyAgentPrivilegedProtocol/
    FyAgentPrivilegedClient/
    FyAgentPrivilegedHelper/
    FyAgentPrivilegedTransaction/   # fd-relative copy + receipt; testable without root
  include/fyagent_privileged_bridge.h
  Resources/
    helper-info.plist
    helper-launchd.plist
  Tests/
```

Pinned dependencies:

- Blessed `0.6.0`
- Authorized `1.0.0`
- EmbeddedPropertyList `2.0.2` (transitive; lock it)
- Required `0.1.1` (transitive; lock it)
- SecureXPC exact revision `1cece54562c7626d042f007d2f38cfe325565850`
  (latest tag remains `0.8.0`; no newer tag contains the reviewed post-0.8.0
  fixes as of 2026-08-31)

No `branch: main`, no `from:` without `Package.resolved`, no prebuilt helper.

## Copy implementation gate

Prefer Apple `copyfile` only if directory-FD / `O_NOFOLLOW` recursion can be
proved. Otherwise implement a minimal `openat` copier for regular files and
directories. Never call `ditto`, `Process`, `system`, or a shell from the
root helper. Record the choice in
`research/copyfile-adr.md`.

## File ownership for parallel implementers

### Agent A — Swift helper (exclusive)

`src-tauri/macos-privileged-helper/**`

Must not edit Cargo.toml, lib.rs, agent_install, frontend, scripts, or
workflows.

### Agent B — Rust port + wire

Exclusive:

- `src-tauri/src/macos_system_commit/**`
- `src-tauri/src/lib.rs` only to add `mod macos_system_commit;`
- `src-tauri/src/agent_install/macos.rs`
- `src-tauri/src/agent_install/types.rs` (reason enum + contract versions)
- `src-tauri/src/agent_install/mod.rs` (reason mapping only if required)
- `src-tauri/src/agent_install/inventory.rs` only if tests require the
  disabled-system-target assertion to stay honest
- `src/v2/shared/features/agent-install-readiness.ts`
- `src/v2/pages/agents/useAgentLifecycleAction.ts`
- matching `tests/v2/**` parser/copy tests

Must not edit the Swift package, release scripts, or workflows.

Production `macos.rs` continues to return `AuthorizationRequired` for system
scope unless a **test-only** fake port is injected. Do not flip
`production_enabled()`.

### Agent C — signing / verifier

Exclusive:

- `scripts/release/macos-developer-id.sh` (inside-out nested sign)
- `scripts/release/verify-macos-signed-app.sh`
- new `scripts/release/verify-macos-privileged-helper.sh`
- `tests/releaseWorkflow.test.ts`
- `.github/workflows/release.yml` only if the existing sign/verify steps must
  call the new verifier
- `scripts/ci/classify-changes.mjs` only if a new top-level path is otherwise
  unclassified (Swift under `src-tauri/` is already `backend`)

Must not fail unsigned unit tests when the helper binary is absent. Formal
`verify-macos-signed-app.sh` without `--signature-only` should require nested
helper/client once the files exist; the script must look up the frozen paths
above. Do not create a second notarization path.

Do not edit `src-tauri/Cargo.toml` workspace members.

## Negative scan

Reject any diff that adds:

- path/URL/command/argv/destination fields on renderer IPC or helper wire
- `sudo`, AppleScript administrator, `AuthorizationExecuteWithPrivileges`
- root `Process` / `system` / `popen` / `ditto`
- SMAppService runtime fallback
- Windows user-helper behavior changes
- Claude Desktop product identity
