# Local Integration Audit

Date: 2026-08-31
Mode: read-only inspection; no product code or existing task artifact was modified.

## 1. Worktree protection

The repository is currently on the user's working branch with a large in-progress macOS Agent lifecycle change set. The active sibling task is:

```text
.trellis/tasks/08-31-macos-agent-install-update-experience
```

Its `wave-ownership.md` explicitly says system `/Applications` commit is out of that task and will be owned by a later Trellis task on the same branch. It requires:

- `MacSystemApplications` stays `authorization_required` and non-actionable;
- no sudo, administrator AppleScript, generic helper, or false user-scope success;
- the later helper task owns the privileged system commit.

Therefore this task is intentionally standalone and has no Trellis parent link. A parent link would mutate the sibling task's `task.json`, violating the user's isolation requirement.

## 2. Existing owners to reuse

### 2.1 Downloaded artifact capability

The current Agent macOS adapter already consumes `codex_desktop::download::DownloadedArtifact` instead of a renderer path. The artifact can revalidate its downloader-owned file identity before use.

Reuse outcome:

- keep download, redirects, cancellation, temp directory, fixed artifact naming and progress outside the helper;
- do not let helper download or receive an artifact path;
- system commit starts after the app bundle itself has been verified and opened as a local capability.

### 2.2 Managed DMG transaction

`src-tauri/src/codex_desktop/platform/macos/dmg.rs` already owns:

- read-only DMG mount/detach;
- exactly one direct app-bundle discovery;
- source/stage/installed identity checks;
- generated same-volume staging and backup names;
- exact selected target preservation;
- commit hooks;
- rollback and `recovery_required` semantics;
- safe cleanup that refuses identity drift.

`src-tauri/src/agent_install/macos.rs` is already a narrow product adapter over `install_managed_exact` for QoderWork, TRAE Work and WorkBuddy. It currently rejects system-scope existing/fresh targets with `AuthorizationRequired`.

Reuse outcome:

- retain this transaction as the semantic gold standard;
- split/extend it around a commit backend rather than create another independent installer;
- user-scope remains on the existing unprivileged backend;
- system-scope calls one `MacSystemCommitPort` whose physical root transaction follows the same invariants and test vectors.

### 2.3 Installation target authority

The Agent inventory already owns opaque `inventoryId`, `targetId` and `expectedTargetRevision`. Renderer requests contain no paths. Multiple candidates require explicit selection and fresh revalidation.

Reuse outcome:

- do not add a helper path picker or persistent destination preference;
- derive a closed helper target slot only after fresh inventory validation;
- a stale/changed candidate authorizes zero helper calls.

### 2.4 Product identity

The backend already owns closed desktop identities and product-specific version projection/equivalence. Codex has a separate application-identity contract, including the `ChatGPT.app` / historical `Codex.app` filename issue.

Reuse outcome:

- generate the helper's finite product/target-slot policy from the backend owner;
- never match product display names;
- do not add Team ID or remote publication fields as new downloaded-content admission gates;
- helper locally revalidates the existing closed bundle/version/shape policy before root mutation.

### 2.5 Job and UI

Agent/Codex lifecycle already owns job stage, cancellation, terminal state and authoritative readback. The helper must not create a second renderer-facing job system.

Reuse outcome:

- add helper phases/reasons to the existing lifecycle only where necessary;
- retain the existing job ID as the user-visible operation owner;
- helper operation IDs are backend-private correlation/replay capabilities.

## 3. Current platform and build facts

### 3.1 Minimum system

`src-tauri/tauri.conf.json` currently sets:

```text
minimumSystemVersion = 12.0
hardenedRuntime = true
```

This prevents a macOS-13-only `SMAppService` implementation from being the sole production solution.

### 3.2 Entitlements and sandbox

`src-tauri/entitlements.macos.plist` currently contains hardened-runtime exceptions used by the application, but no privileged-file-operations entitlement. FyAgent is not using App Sandbox for this distribution.

The sibling task's signed spike found that `NSWorkspaceAuthorization`/authorized `FileManager` does not expose a sufficient public fresh-create/copy primitive for absent `/Applications/<App>.app` targets. It leaves the system destination disabled and assigns helper work to this task.

### 3.3 Existing helper

`src-tauri/user-helper` is a Windows-only ordinary-user helper:

- it is `asInvoker`, not a root daemon;
- it uses a Windows authenticated pipe and PackageBridge;
- it has closed action/product enums and no renderer path.

The closed-action and negative-test philosophy is reusable. The Windows executable, protocol transport, bridge layout and privilege model are not reusable for macOS.

### 3.4 Current signing pipeline

The formal macOS flow currently:

1. imports the Developer ID identity into a temporary keychain;
2. signs the top-level `FyAgent.app` with hardened runtime and app entitlements;
3. verifies both app architectures and the expected app identifier/team/timestamp;
4. creates/signs/notarizes/staples one DMG;
5. remounts and verifies the final app.

Gaps for a helper:

- no Swift helper/client build target;
- no `Contents/Library/LaunchServices` helper embedding;
- no app `SMPrivilegedExecutables` generation/verification;
- no helper embedded info/launchd plist verification;
- no nested helper/client inside-out signing step;
- no helper version/requirement/universal-slice verifier;
- no signed SMJobBless/XPC/HIL gate.

The new implementation must extend this existing formal chain, not create a second release pipeline.

## 4. Likely integration surfaces

These are planning locations, not a frozen edit list. Implementation must re-audit after the sibling task settles.

```text
src-tauri/src/agent_install/**
src-tauri/src/codex_desktop/platform/macos/**
src-tauri/src/platform/**
src-tauri/Info.plist
src-tauri/entitlements.macos.plist
src-tauri/tauri.conf.json
src-tauri/build.rs / Cargo.toml as needed for the private bridge
scripts/release/macos-developer-id.sh
scripts/release/verify-macos-signed-app.sh
scripts/release/macos-signing-policy.sh
.github/workflows/release.yml
release workflow tests and supported-platform structure checks
V2 Agent/Codex lifecycle state/reason projections
```

## 5. Integration risks

| Risk | Consequence | Task response |
|---|---|---|
| Current task changes transaction/product types | Merge conflicts or duplicate abstractions | Wait for helper-facing seam to stabilize; re-audit in Phase 0 |
| Swift client implemented as executable sidecar | Larger attack surface; other processes can invoke it | Keep client code inside signed FyAgent process through private library/FFI |
| Source passed as user path | Root TOCTOU/path replacement | Pass an opened directory FD and revalidate with fd-relative operations |
| Swift helper hand-copies product constants | Identity/target policy drift | Generate Rust/Swift projections from one closed source |
| Helper reproduces complete DMG installer | Second owner and inconsistent rollback | Keep download/mount/source verification in existing Rust owner; helper owns only root commit |
| Top-level-only signing | Invalid/untrusted nested helper | Add inside-out nested signing and explicit verifier |
| Portable tests treated as native proof | System target enabled unsafely | Formal signed/notarized HIL is an enablement gate |

## 6. Local conclusion

The repository already has the difficult non-privileged parts: target authority, downloaded artifact capability, product policy, DMG validation, transaction semantics, job ownership and release signing foundation. The missing scope is a narrow system commit backend plus its Swift build/signing lifecycle. This confirms the task should not become a general installer rewrite.
