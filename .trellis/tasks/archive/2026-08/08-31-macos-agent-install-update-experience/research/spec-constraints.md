# Existing Contract Constraints

Date: 2026-08-31

This is a focused execution summary of the large existing Trellis contracts. It does not replace those specifications. During implementation, update and reread the authoritative files whenever a versioned contract changes:

- `.trellis/spec/backend/external-agent-p0.md`
- `.trellis/spec/backend/codex-desktop-installer.md`
- `.trellis/spec/frontend/v2-agent-models.md`

## 1. Agent catalog and target authority

- The existing catalog has exactly seven product IDs: `qoderwork`, `trae-work`, `workbuddy`, `grokbuild`, `codex`, `claude-code`, and `opencode`.
- There is one catalog and one authoritative installation inventory. Do not add a renderer registry, second catalog, generic path probe, or duplicate installer façade.
- OpenCode CLI/Desktop support therefore requires a closed child facet/surface under the existing `opencode` product, not a new top-level Agent ID.
- Lifecycle requests remain closed and strict. Renderer input may contain only the product ID, closed action, backend-issued release capability, and a complete opaque inventory/target/revision triplet.
- URL, path, command, token, environment, hash, package format, install scope, identity, and bypass fields remain forbidden. Partial target triplets fail closed.
- Target selection is per operation. Multiple candidates require explicit selection; the backend never chooses the first or nearest candidate.
- Every write or launch re-enumerates/revalidates the selected target immediately before side effects. Missing, stale, moved, scope-changed, owner-changed, or identity-changed targets authorize no action.

## 2. Versioned lifecycle changes

Current baseline versions are readiness v3, inventory v1, and Agent action/job v2. The existing generic job snapshot contains only job/action/stage/cancellability/reason data.

This task intentionally needs a versioned extension for:

- install facet/surface identity;
- monotonic sequence and timestamps;
- raw completed/total byte telemetry and attempt information;
- persistent terminal diagnostic projection.

The implementation must either bump the wire versions or provide a rigorously tested compatibility projection. Unknown versions, enums, excess fields, sensitive fields, and malformed progress continue to fail closed. Frontend controls remain driven only by backend `allowedActions` and authoritative inventory/readiness.

## 3. Codex remains a dedicated product port

- Codex install/update remains owned by the dedicated Codex Desktop installer and must not be moved onto the generic Agent job slot.
- Reuse is allowed and required below the product port: streaming transport, protected temporary artifact, job/progress primitives, and the managed macOS DMG transaction may be extracted into shared crate-private infrastructure.
- Shared extraction must preserve Codex source binding, cancellation, process-lifecycle single-flight, cleanup, post-install readback, and platform behavior.
- The task may remove the current equal-or-newer implicit launch, but it must not replace the dedicated Codex installer with a generic renderer-controlled installer.

## 4. Executable installer non-admission policy

FyAgent must not accept or reject downloaded executable software by comparing downloaded content/native package contents to remote publication fields or maintained upstream constants. Prohibited admission comparisons include:

- remote or manifest checksum/digest;
- remote/manifest/`Content-Length` byte size;
- remote package identity, publisher, Team ID, signature, notarization, or Gatekeeper value;
- remote release/package/bundle version equality;
- remote architecture/minimum-OS fields read as content truth.

Operational metadata may select a fixed backend-owned source, show a version, drive progress, or perform conservative disk preflight. A metadata/`Content-Length` mismatch alone does not reject the artifact.

A locally computed byte count or streaming digest may bind one protected job handoff where the existing implementation already needs it. It must not be compared with a remote digest, and no second whole-file hashing pass may be added before `hdiutil`, deployment, or package handoff.

## 5. Preserved macOS transaction invariants

The existing managed macOS installer contract requires:

- fixed first-party source policy and bounded HTTPS/redirect behavior;
- a protected job-owned artifact;
- controlled read-only DMG mount;
- exactly one direct top-level `.app` selected by the reviewed local product policy;
- executable containment and safe generated same-volume staging/backup paths;
- exact selected-path update with no scope fallback;
- running-app and target-revision checks before commit;
- atomic replacement or compensating rollback;
- exact known-only cleanup and detach;
- post-install existence, actual version representation, launch-target shape, and runnable readback.

The current contract deliberately keeps `/Applications` visible but non-executable with `authorization_required` until a separately reviewed authorization adapter exists. This task may change that state only after the Apple-native authorization gate and signed HIL succeed. Failure must not redirect to `~/Applications`.

## 6. Discovery and launch

- Runtime observation and launch are separate operations.
- Renderer paths, executables, bundle IDs, arguments, and arbitrary URLs remain forbidden.
- A launch action operates on a backend-issued candidate capability and revalidates it before opening.
- OpenCode desktop discovery must feed the same normalized inventory used by readiness, target selection, update, and launch; it must not create a second scanner whose answer can diverge.
- Launch completion or failure must be represented by a bounded stable result. Install/update success is not launch success and must not trigger launch as a side effect.

## 7. Grok and other CLI ownership

- Grok Build and OpenCode CLI currently reuse the Tooling domain; this task should preserve one authoritative CLI discovery/owner path.
- A detected vendor `internal` Grok installation must not be silently converted to npm, Homebrew, or another distribution owner after an update failure.
- External process exit is evidence, not sufficient proof of installed state. Success requires authoritative post-action version/path/owner readback.
- If the official installer exposes no byte protocol, the UI remains indeterminate. It may show stage, elapsed time, source class, and a bounded redacted log summary, but no invented percentage or speed.

## 8. Frontend authority and copy

- The exact seven-product catalog and backend `allowedActions` remain authoritative.
- A product facet may be rendered as grouped rows, but UI state must not infer one facet from another.
- Direct install/update is unavailable when there are zero or multiple eligible targets; open the target picker instead of choosing implicitly.
- Action success triggers authoritative readiness/inventory reread rather than optimistic “installed” state.
- Codex UI continues to use its dedicated installer port.
- Generic terminal `failed`, `cancelled`, `incomplete`, `rollback_restored`, and `recovery_required` states remain non-green and visible.
- User-facing copy must not expose internal capability IDs, revisions, helper protocols, local paths, or shell output. The desktop launch button is exactly `打开软件`.

## 9. Required spec updates at completion

Before task closure, update the three authoritative specifications to reflect:

- the closed facet/surface wire model;
- the new lifecycle job/progress contract version;
- the single shared artifact owner beneath dedicated product ports;
- the existing managed desktop registry plus shared structured bundle metadata, and native launch diagnostics within the existing process-launch owner;
- the proven or blocked `/Applications` authorization outcome;
- explicit-launch-only behavior;
- Grok official-owner and progress limitations;
- one shared frontend transfer projection.

The implementation cannot mark acceptance complete while code, tests, and these specifications disagree.
