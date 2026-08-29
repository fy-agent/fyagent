# Current contract and reuse review

## Required full-contract reads

Before implementation, read the complete `.trellis/spec/backend/external-agent-p0.md` and `.trellis/spec/frontend/v2-agent-models.md`. They are intentionally omitted from automatic JSONL injection because each exceeds the configured context-file size limit; this research note is not a replacement for those authoritative contracts.

## Current action boundary

`src-tauri/src/agent_install/types.rs` currently defines `StartAgentActionRequest` with only `agent_id`, `action` and optional `expected_release_id`. This is sufficient to choose a product release but cannot identify an installed target.

`src-tauri/src/agent_install/desktop.rs` currently returns `DesktopObservation { installed, local_version }`. macOS scans user/system Applications in order; Windows scans a fixed set of roots and relative EXE paths. The observation loses candidate count, scope, path identity, provenance and ambiguity.

`src-tauri/src/agent_install/mod.rs` therefore cannot distinguish:

- one trusted installation;
- two trusted installations;
- one stale registration plus one real installation;
- an existing system installation from a fresh user-scope destination.

## Existing candidate concepts

`src-tauri/src/codex_desktop/platform.rs` already contains:

- `RestartInstallationScope`;
- `TrustedInstallationCandidate` with a private stable key;
- `RestartCandidateInspection` with trusted/ambiguous/untrusted outcomes;
- `PreparedInstallPackage` retaining the downloader-owned artifact capability;
- `PlatformInstallPlan` for target-volume planning.

These are stronger than the current generic Agent observation and should be the first reuse candidate. Their current semantics are Codex-specific, so implementation must preserve Codex exact-identity and restart safety. The safe options are either extracting genuinely common private primitives or adapting Codex and Agent implementations to a narrow shared interface.

## Frontend reuse

The V2 shared layer already owns FeaturePorts, queries, `AssignmentPanel`, dialogs, catalog components and control primitives. Candidate selection should not be embedded in `AgentDirectory.tsx` or copied into platform-specific panels. One shared target picker can serve:

1. Agent directory install/update;
2. an Agent detail lifecycle section;
3. later multi-install management, if/when exposed.

This is a real common semantic: select one backend-authorized lifecycle target and explain why another is unavailable.

## Architectural decision

- Add one installation-inventory owner, not one scanner per product.
- Keep raw target capabilities backend-private.
- Use snapshot-scoped opaque IDs/revisions to bind user intent.
- Keep platform adapters responsible only for evidence.
- Treat ambiguity as a state, not an ordering problem.
