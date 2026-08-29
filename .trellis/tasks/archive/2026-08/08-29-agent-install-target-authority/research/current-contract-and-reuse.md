# Current contract and reuse review

## Required full-contract reads

Before implementation, read the complete `.trellis/spec/backend/external-agent-p0.md` and `.trellis/spec/frontend/v2-agent-models.md`. They are intentionally omitted from automatic JSONL injection because each exceeds the configured context-file size limit; this research note is not a replacement for those authoritative contracts.

## Baseline action boundary

Before this task, `src-tauri/src/agent_install/types.rs` defined
`StartAgentActionRequest` with only `agent_id`, `action` and optional
`expected_release_id`. This was sufficient to choose a product release but
could not identify an installed target.

`src-tauri/src/agent_install/desktop.rs` returned
`DesktopObservation { installed, local_version }`. macOS scanned user/system
Applications in order; Windows scanned fixed roots and relative EXE paths. The
observation lost candidate count, scope, path identity, provenance and
ambiguity.

`src-tauri/src/agent_install/mod.rs` therefore could not distinguish:

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

These are stronger than the baseline generic Agent observation and were the
first reuse candidate. Their semantics remain Codex-specific: the stable key,
restart target and prepared package are coupled to Codex PFN/AUMID/bundle
identity and service-owned restart planning. Generalizing those concrete
structs during this task would have widened a security boundary and forced
unrelated package kinds into Codex. The conservative reuse is therefore a
narrow adapter through `CodexDesktopService::get_local_status`, while the new
inventory owner adopts the same fail-closed principles: private stable keys,
explicit ambiguity, no display-name winner and action-time reinspection.

This decision is deliberately not a license for two target-selection
algorithms. `agent_install/inventory.rs` is the only generic Agent owner of
normalization, opaque capability IDs, revision, expiry and selection. Codex
keeps its pre-existing product-specific restart selector behind its service;
the Agent inventory consumes only its already-authoritative public outcome.

## Frontend reuse

The V2 shared layer already owns FeaturePorts, queries, `AssignmentPanel`, dialogs, catalog components and control primitives. Candidate selection should not be embedded in `AgentDirectory.tsx` or copied into platform-specific panels. One shared target picker can serve:

1. Agent directory install/update;
2. an Agent detail lifecycle section;
3. later multi-install management, if/when exposed.

This is a real common semantic: select one backend-authorized lifecycle target
and explain why another is unavailable. The implementation uses one shared
`LifecycleTargetPicker` built from the repository's existing primitives and
native `<fieldset>/<input type="radio">` semantics. No new component library
or custom roving-focus implementation was added: native radio keyboard and
assistive semantics are sufficient here, while Radix Tabs remains appropriate
for tab navigation rather than mutually exclusive install-target choice.

## External research applied

- Microsoft App Paths, uninstall registration and MSIX package inventory are
  separate evidence sources; no one source is an authority for every Windows
  package kind. Stage 1 therefore defines provenance as a closed union and
  makes dedup/selection backend-owned rather than equating a fixed path with an
  installation. Stage 3 supplies those adapters.
- Apple code-signing guidance treats designated identity and post-copy
  verification as security properties, not display metadata. Inventory keeps
  signer/path identity private and exposes only closed evidence plus a safe
  label; Stage 2 performs the mutation-time verification.
- WAI-ARIA and native HTML radio semantics support one-of-many selection.
  Reusing the platform control avoids a second keyboard/focus state machine.
- TanStack Query's documented key/invalidation model supports one stable
  `agentInstallationInventory(agentId)` owner. Pages consume the shared query
  or port and never mint snapshot IDs locally.

## Implemented authority

- Readiness v3 reports `inventoryState` and `requiresTargetSelection`.
- Inventory v1 returns snapshot-scoped candidate/destination capabilities,
  closed scope/owner/package/evidence/reason enums and symbolic labels.
- Action v2 accepts only a complete opaque inventory/target/revision triplet.
- The backend caches bounded short-lived snapshots and re-enumerates before any
  selected launch/write dispatch. Missing, expired or changed targets fail
  closed; multiple trusted candidates are never resolved by enumeration order.
- The directory keeps direct install/update only for one eligible target; the
  detail surface uses the shared picker for candidate and fresh-destination
  flows. Both send only backend-generated capabilities.

## Architectural decision

- Add one installation-inventory owner, not one scanner per product.
- Keep raw target capabilities backend-private.
- Use snapshot-scoped opaque IDs/revisions to bind user intent.
- Keep platform adapters responsible only for evidence.
- Treat ambiguity as a state, not an ordering problem.
- Keep Stage 2/3 deployment behind the validated capability; this task freezes
  the contract and intentionally does not implement platform mutation.
