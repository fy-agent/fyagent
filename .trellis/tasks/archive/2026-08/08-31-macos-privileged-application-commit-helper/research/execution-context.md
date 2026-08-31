# Execution Context

This file is the compact implementation/check context for the task. It summarizes large owning specs without injecting their full, potentially truncated content.

## 1. Owning specifications

Implementation must reread the current versions of:

- `.trellis/spec/backend/reuse.md`
- `.trellis/spec/backend/modular-boundaries.md`
- `.trellis/spec/backend/external-agent-p0.md`
- `.trellis/spec/backend/codex-desktop-installer.md`
- `.trellis/spec/backend/application-identity.md`
- `.trellis/spec/backend/github-release-workflow.md`
- `.trellis/spec/backend/macos-dmg-layout.md`
- `.trellis/spec/frontend/reuse.md`
- `.trellis/spec/frontend/v2-agent-models.md`
- `.trellis/spec/guides/code-reuse-thinking-guide.md`

## 2. Stable constraints extracted from specs

### Reuse

- existing owner -> adopted dependency -> maintained OSS -> one narrow adapter -> bespoke last;
- no second downloader, DMG transaction, product registry, job store, raw invoke parser or formatter;
- dependency adoption requires license/maintenance/provenance/platform/toolchain/security review and tests.

### IPC and target authority

- renderer supplies only closed Agent/Codex actions and opaque inventory/target/revision/release capabilities;
- no renderer/helper URL, path, command, token, hash, scope, package format or bypass;
- fresh re-enumeration immediately before side effects;
- multiple/stale/drifted target never resolves to “first” or “nearest”.

### macOS installer

- existing transaction owns read-only mount, unique direct app, generated same-volume staging/backup, atomic replacement, rollback, exact cleanup and post-install readback;
- system target currently disabled with `authorization_required`, with no silent user-scope fallback;
- source/stage/installed local identity and product-comparable version must agree;
- copy/rename success alone is not installation success;
- rollback uncertainty is `recovery_required`.

### Application identity

- display name/path is not product identity;
- Codex/new ChatGPT uses reviewed stable identity and must not match ChatGPT Classic by name;
- helper product/target slots must come from one backend-owned closed policy;
- remote publication fields are not new local package admission gates.

### Release

- formal Developer ID/hardened runtime/one-DMG notarization owner remains existing release workflow;
- nested code signs inside-out before main app;
- final mounted artifact is reverified;
- local/manual builds cannot claim formal compatibility;
- task runners and workflow mutations need fixture/contract tests and cleanup.

### Frontend

- helper status/stages/reasons project through existing lifecycle owners;
- no direct Tauri import or raw wire parsing in pages;
- no fake success, fake progress or silent fallback;
- user copy distinguishes authorization cancellation, helper failure, app failure, rollback restored and recovery required.

## 3. Sibling task dependency

The predecessor `08-31-macos-agent-install-update-experience` is archived; use the current tree. The next task `08-31-macos-agent-directory-install-policy` owns Agent directory order, domestic install-only policy, Claude/OpenCode surface changes, and Claude Desktop source. This task must not implement those surfaces. See `research/sibling-boundary.md` and `research/implementation-seam.md`.

## 4. Selected external reuse

- Blessed: SMJobBless install/update diagnostics only;
- SecureXPC: typed authenticated XPC and FD transfer;
- Authorized: operation custom rights and Authorization external form;
- SwiftAuthorizationSample/Mist: reference only, no generic path/command routes;
- Apple Service Management/Authorization/XPC/system file APIs: platform authority.

## 5. Hard negative checklist

Reject implementation if any are present:

- root `Process`, shell, sudo, AppleScript administrator prompt or network client;
- generic copy/move/delete/chown/chmod/path route;
- source/target path over IPC/XPC;
- standalone broad client sidecar;
- PID-based trust;
- manually duplicated Swift product identity table;
- floating Swift dependency branch or prebuilt helper binary;
- runtime SMJobBless -> SMAppService fallback chain;
- production system action before formal signed/notarized HIL;
- edits that change Windows helper/runtime behavior.

## 6. Required evidence at check time

- exact dependency revisions/licenses;
- protocol/forbidden-wire tests;
- mutual signing requirement tests;
- source FD TOCTOU tests;
- transaction/fault/recovery tests;
- Rust lifecycle/readback tests;
- release layout/inside-out signature verifier tests;
- formal HIL matrix or an explicit blocker with system capability still disabled;
- negative scan showing no duplicate owners/unsafe helper operations.
