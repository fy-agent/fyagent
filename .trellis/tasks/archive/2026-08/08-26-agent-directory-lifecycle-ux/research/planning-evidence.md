# Planning Evidence — Agent Directory Lifecycle UX

Date: 2026-08-26

## 1. Local Code Evidence

### Existing motion owner

- `src/v2/shared/ui/SelectionLens.tsx` already owns a tuned physics spring:
  `type: spring`, `stiffness: 520`, `damping: 42`, `mass: 0.62`.
- `src/v2/app/styles/shell.css` currently animates only the caret with
  `160ms ease`; the collapsible list uses `[hidden] { display: none; }`, so
  content open/close has no layout animation.
- `.trellis/spec/frontend/v2-shell.md` intentionally centralizes V2 motion and
  requires reduced-motion support. The new behavior should evolve that shared
  owner instead of importing Motion ad hoc in SideNavigation.

### Existing catalog / scan owner

- `src/v2/pages/agents/AgentDirectory.tsx` currently filters cards through
  `projectInstalledEntries`, so idle state is empty and not-installed/unknown
  entries disappear.
- `src/v2/pages/agents/useAgentDirectoryScan.ts` already keeps request IDs,
  settled IDs, current success/failure IDs, retained results, and disabled
  TanStack queries for all 7 Agent IDs. It is an extension point, not code to
  replace wholesale.
- `src/v2/shared/features/queries.ts` already centralizes readiness query keys
  and calls `FeaturePorts.agentInstallReadiness.get`.
- `src/v2/shared/features/directory.ts` / catalog types are the existing
  product-order owner. No second frontend supported-software registry is
  needed.

### Existing install/update owners

- `src/v2/shared/features/agent-install-readiness.ts` defines the closed
  readiness/action contract (`install`, `update`, `launch`, auth actions) and
  rejects extra wire fields.
- `src/v2/shared/platform/tauri/feature-ports/agentInstallReadiness.ts` already
  wraps all generic Agent lifecycle invokes.
- `src/v2/pages/agents/AgentInstallReadinessSection.tsx` already contains the
  generic action loop: start action, poll job every bounded interval, surface
  stage/failure, then reread readiness. This is reusable behavior even though
  the section is not currently mounted by `AgentsPage`.
- `src-tauri/src/agent_install/jobs.rs` job snapshot contains stage and
  cancellable state, but no byte/percent progress. A generic percentage would
  therefore be fabricated.
- `src-tauri/src/agent_install/mod.rs` reuses Tooling for Claude/Grok/OpenCode,
  a managed desktop flow for Qoder/TRAE/WorkBuddy, and explicitly rejects
  Codex install/update with `managed_by_codex_desktop`.
- `src/v2/shared/codex-desktop/useCodexDesktopInstaller.ts` is the established
  Codex view model and already owns event subscription, stale snapshot
  rejection, terminal reread, percentage, downloaded bytes and speed.

### Existing dependency stack

`package.json` already includes:

- `@radix-ui/react-collapsible ^1.1.12`
- `framer-motion ^12.23.25`
- `@tanstack/react-query ^5.90.3`

No new dependency is justified by this feature.

## 2. Official External Evidence

### Radix Collapsible

Official docs: https://www.radix-ui.com/primitives/docs/components/collapsible

Relevant evidence:

- controlled `open` / `onOpenChange` is supported;
- Root/Trigger/Content expose `data-state=open|closed`;
- Content exposes `--radix-collapsible-content-width/height` for size animation;
- primitive follows the Disclosure WAI-ARIA pattern and supports Enter/Space.

Conclusion: use the already-adopted primitive semantics instead of writing a
second disclosure implementation. V2 should wrap/use the package in its own
shared layer rather than import the legacy V1 wrapper.

### Motion physics spring

Official docs: https://motion.dev/docs/react-transitions

Relevant evidence:

- physics springs are configured with `stiffness`, `damping`, and `mass`;
- spring animation can retarget naturally from current motion state;
- time/bounce settings are overridden when physics parameters are set.

Conclusion: the existing SelectionLens values are a real reusable physics
signature. Do not translate them to an unrelated CSS `ease` curve.

### TanStack Query lazy/disabled queries

Official docs:
https://tanstack.com/query/latest/docs/framework/react/guides/disabling-queries

Relevant evidence:

- `enabled: false` prevents automatic mount/background fetch;
- the returned `refetch` can manually trigger the query;
- docs explicitly describe enabling later as a lazy-query pattern.

Conclusion: the repository's existing readiness hooks can support automatic
first-entry progressive scanning without adding another data-fetch layer.

## 3. Architecture Decision

The preferred stack is therefore:

```text
Catalog (existing)
  -> all 7 rows visible
TanStack readiness queries (existing)
  -> progressive per-row state
Agent action façade / Codex installer (existing)
  -> install/update + real progress semantics
Radix Collapsible + shared Motion spring (existing dependencies)
  -> smooth, accessible side-nav collapse
```

No external package search found a capability gap that justifies adding a new
dependency. Execution agents should research further only if the current code
has materially changed or a concrete capability gap is discovered.
