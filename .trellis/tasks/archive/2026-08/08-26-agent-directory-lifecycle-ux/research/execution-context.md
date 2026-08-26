# Curated Execution Context — Agent Directory Lifecycle UX

This file is a task-scoped context summary for execution context injection. It does not
replace the owning specs; it exists because some owning specs exceed Trellis'
single-file injection budget and the relevant Agent Directory clauses are near
the end of those documents.

## 1. Stable Product Set

The Agent catalog remains the only supported-software registry. The current
catalog order is:

```text
qoderwork
trae-work
workbuddy
grokbuild
codex
claude-code
opencode
```

Do not create a second page-local array to decide what software exists.
Catalog display names/descriptions and `PRODUCT_DIRECTORY` remain the existing
owners used by the V2 page.

## 2. New Directory Behavior for This Task

The older V3 implementation currently documents/implements scan-on-demand and
filters normal cards to installed-only. This parent task intentionally changes
that renderer behavior:

- catalog success -> all 7 rows are immediately present;
- first directory entry -> automatic background readiness scan;
- per-row readiness settles progressively;
- scan state controls status/actions, not row existence;
- first unresolved / not-installed / unknown / unavailable / failed row cannot
  enter configuration;
- a retained successful installed result may remain usable during rescan;
- technical read failure is never converted to not-installed.

The existing four-section Agent configuration shell (`models / skills / mcp /
prompts`) remains unchanged except for the directory gate that enters it.

## 3. Existing Readiness Contract

Renderer uses only the existing `AgentInstallReadiness` shape:

```text
installState = not_installed | installed | installed_not_runnable |
               unknown | unavailable
updateState  = unavailable | unknown | up_to_date | update_available |
               latest_unknown
allowedActions = closed AgentActionId[]
releaseId = optional opaque backend-generated value
```

Renderer must not add or infer URL, path, command, token, hash, package format,
signer identity, or bypass fields.

For directory configuration gating, this task preserves the existing renderer
compatibility treatment that `installed` and `installed_not_runnable` prove the
software exists. If a particular configuration owner later requires a runnable
process, apply that narrower check there rather than redefining the global
install state.

## 4. Existing Lifecycle Action Contract

Generic actions stay on the existing closed port:

```text
startAgentAction(agentId, install|update, expectedReleaseId?)
getAgentActionJob(jobId)
cancelAgentAction(jobId)
```

The job stages are:

```text
checking -> downloading -> installing -> verifying_installation
         -> succeeded | failed | cancelled
```

Generic job snapshots do not contain a numeric download percentage. Show the
real stage; do not fabricate percent.

`allowedActions` is authoritative for whether generic install/update can be
offered. Platform/source safety may legitimately remove an action.

## 5. Codex Special Owner

Codex install/update remains outside the generic Agent action job. Generic
readiness returns `managed_by_codex_desktop` for those actions.

Use the existing V2 Codex Desktop installer owner/view model. It already owns:

- local/remote status;
- install/update/launch primary action derivation;
- job event subscription and stale snapshot rejection;
- cancellation;
- terminal refresh;
- true determinate progress, downloaded bytes, and speed when available.

Do not duplicate the Codex downloader, job subscription, progress math, or
expected-release validation inside Agent Directory.

## 6. Scan Owner

`useAgentDirectoryScan` already uses seven disabled TanStack readiness queries
and manual `refetch()` calls. It already has request-id stale protection,
partial settle state, current failures, retained result data, and a scan
single-flight check.

Extend this owner. Do not call Tauri directly from the page and do not create a
second query client/cache.

The first-entry auto-start can live in the scan hook or the Agent page,
whichever makes keep-alive semantics clearer. It must run once per mounted
page session, while manual rescan remains available.

## 7. Existing Generic Action Runner to Reuse

`AgentInstallReadinessSection` already implements the useful generic runner:

```text
start action
  -> set checking/busy
  -> if jobId: poll getActionJob at bounded interval
  -> classify terminal/timeout/reason
  -> reread readiness
```

Prefer extracting this behavior into a reusable hook/view-model over copying it
into `AgentDirectory`.

## 8. Side Navigation Motion Contract

The current tuned SelectionLens spring is:

```text
type: spring
stiffness: 520
damping: 42
mass: 0.62
```

The new configuration-group collapse should reuse this spring source, not a
new cubic-bezier. Preserve active lens behavior, `aria-expanded`, keyboard
navigation, and `prefers-reduced-motion`.

Project dependencies already include Radix Collapsible and Framer Motion. Use
those adopted capabilities in a V2 shared owner; do not import the legacy V1
collapsible wrapper and do not add another package.

## 9. Reuse / Architecture Rules

Order of preference:

```text
existing shared owner
  -> already-adopted dependency
  -> maintained open-source candidate (only if a real gap remains)
  -> bespoke implementation with recorded reason
```

This planning pass found no dependency gap. Execution agents should not repeat
broad package research. Targeted research is appropriate only when the current
baseline contradicts this context or exposes a concrete missing capability.

V2 page/widget code must continue using FeaturePorts instead of direct Tauri
imports. Shared code should be promoted only for a real/likely second consumer;
do not turn this quick iteration into a new framework.

## 10. Serial Execution Shape

This task is executed by one agent on one development branch. Do not create
parallel Trellis children or Worktrees.

Recommended order:

1. shared collapse motion + SideNavigation;
2. scan controller / per-row projection / first-entry auto scan;
3. generic action runner + thin Codex projection;
4. AgentDirectory card composition + Page.css;
5. unit/browser integration, final checks, and owning-spec updates.

Run focused validation at each boundary, but do not manufacture public APIs or
temporary adapters merely to preserve the phase split. Adjacent phases may be
combined when the current code makes that simpler and the stable behavior and
safety boundaries remain unchanged.
