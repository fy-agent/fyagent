# Planning Review Rounds

Date: 2026-08-31

## Round 1 — Requirement fidelity and product semantics

### Questions reviewed

- Does “installed first” accidentally classify unknown/error as not installed?
- Does “domestic first” move uninstalled domestic software above installed software?
- Does removing Claude/OpenCode CLI mean deleting all CLI-related product domains?
- Does “Claude Code one-click installer” refer to a real standalone desktop artifact?

### Findings

1. The scan contract has meaningful unknown/error states; a two-bucket installed/uninstalled sort would lie.
2. Domestic priority is conditional on installed status, not a global first group.
3. OpenCode/Claude product IDs are shared with configuration, Skills, MCP, models and sessions. Deleting those IDs or all Tooling support would exceed the request.
4. Anthropic now includes Claude Code in Claude Desktop; the managed artifact is `Claude.app`, not a separate “Claude Code Desktop” package.

### Resolutions

- Use four buckets: installed domestic, installed other, unresolved, confirmed not installed.
- Apply domestic metadata only inside the installed bucket.
- Remove CLI only from Agent lifecycle/install links/actions; preserve other stable domains.
- Keep `claude-code` ID but label the physical component `Claude Desktop`.

### Status

Accepted. No user-owned product decision remains unresolved.

## Round 2 — Architecture and reuse

### Questions reviewed

- Can the task be implemented by UI changes only?
- Does adding Claude/OpenCode require product-specific installers?
- Where should product action policy and domestic priority live?
- Is a new dependency justified?

### Findings

1. UI-only removal would leave direct `start_agent_action(update)` and target eligibility active.
2. Existing source/download/job/DMG/inventory/helper owners already cover the execution primitives.
3. Product/surface/action policy is currently distributed and can drift.
4. Domestic priority is frontend directory metadata; it does not belong in backend filesystem/source policy.
5. The sort is a small fixed product rank and needs no library.

### Resolutions

- Add one crate-private backend lifecycle policy owner used by readiness/inventory/dispatch.
- Add one field to existing `PRODUCT_DIRECTORY`, not a page-local ID set.
- Reuse the complete managed desktop pipeline.
- Plan zero new runtime/development dependencies.

### Status

Accepted with an implementation-time re-audit because predecessor file ownership is still moving.

## Round 3 — Source, supply chain and China-network claims

### Questions reviewed

- Is the Claude mirror sufficiently narrow and reviewable?
- Should mirror-provided URL/hash/publisher fields become installer authority?
- Does a mirror imply Claude service availability in China mainland?
- Should FyAgent use OpenCode’s own Electron updater?

### Findings

1. The reviewed Claude mirror is MIT licensed, product-specific and documents unchanged official package synchronization to GitHub/R2.
2. Current fixed manifest and DMG endpoints are reachable and the ephemeral DMG inspection produced the expected universal notarized `Claude.app`.
3. The existing executable-installer contract deliberately avoids remote publication fields as downloaded-content admission.
4. Anthropic’s official availability list omits China mainland.
5. OpenCode’s updater would create a competing execution/rollback/target authority.

### Resolutions

- Use only fixed Claude manifest/artifact endpoints behind code-owned enums.
- Parse only bounded flow fields; ignore remote URL/hash as capability/admission.
- Keep Bundle ID/version product routing and OS/native install/readback boundaries.
- Explicitly forbid claims that installer reachability enables service/account access.
- Use OpenCode official release metadata + fixed stable DMG; reject upstream updater execution.

### Residual risk

The Claude mirror is externally operated and has no FyAgent SLA. Endpoint/repository/provenance drift disables the source until re-reviewed.

### Status

Accepted with fail-closed stop conditions.

## Round 4 — State machine, UX and testability

### Questions reviewed

- Should the list reorder as each scan result arrives?
- How should a retained installed result with current refresh failure sort?
- How does a newly installed card move without another full scan?
- Can product-specific UI conditionals be avoided?

### Findings

1. Progressive reordering would produce up to seven card moves and unstable keyboard position.
2. Retained readiness is useful for configuration but is not current scan proof.
3. `applyReadiness` already provides a post-action authoritative patch.
4. The generic lifecycle hook already derives install/update from backend `allowedActions`.

### Resolutions

- Keep canonical order until the initial scan completes.
- Freeze the last committed order during rescan.
- Treat current failure as unresolved only for ordering; retain stale display/configuration semantics.
- Recalculate after a complete scan or an authoritative patch when not scanning.
- Do not add product-specific hide/update logic in React.

### Status

Accepted. Test matrix defined in `sorting-policy.md` and `acceptance-evidence.md`.

## Round 5 — Scope, dependency and rollout

### Questions reviewed

- Can this task safely run concurrently with the current macOS lifecycle refactor?
- Can `/Applications` acceptance be claimed without the helper?
- What shared changes are allowed before Windows implementation?
- How should partial source failure roll back?

### Findings

1. The predecessor currently touches the same surface/source/job files; concurrent implementation risks overwrites and duplicate abstractions.
2. `/Applications` fresh install/update still depends on the separate signed helper task.
3. CLI/update removals are long-term product decisions; Windows desktop executor work is not needed now.
4. Sorting and install-only policy are independently useful even if Claude source or helper HIL is blocked.

### Resolutions

- Keep this as a separate serial task and re-audit after predecessor completion.
- Keep system target `authorization_required` until helper signed HIL passes.
- Do not add Windows installer code; preserve the product policy for later Windows work.
- Roll out in reversible slices: policy -> surfaces -> sorting -> sources -> UI -> helper HIL.
- If Claude source fails review, disable it and use official-page fallback; never re-enable CLI as an implicit fallback.

### Status

Accepted. Planning is converged and has no blocking open question.

## Final review result

```text
Goal: explicit
In scope: explicit
Out of scope: explicit
Observable acceptance: defined
Product decisions: resolved
Technical unknowns: researched or gated
Reuse review: complete
Security/source review: complete
Implementation approval: not granted
Task status: planning
```

