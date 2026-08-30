# Design — Installation Inventory and Target Authority

## 1. Domain separation

The current desktop adapter answers only `installed/version`. The target design separates four concepts:

```text
Evidence adapter -> Candidate normalizer -> Inventory snapshot -> Action target validation
```

- Evidence adapters know platform facts.
- Normalization/dedup owns one semantic candidate model.
- Inventory snapshot is the renderer-facing, read-only selection source.
- Action validation re-enumerates and binds the selected target before side effects.

No platform adapter chooses the default winner.

## 2. Proposed internal model

Names may follow existing conventions, but the semantics are fixed:

```rust
struct InstallationCandidate {
    candidate_id: OpaqueCandidateId,
    candidate_revision: OpaqueRevision,
    agent_id: AgentCatalogId,
    scope: InstallationScope,
    owner: InstallationOwner,
    package_kind: PackageKind,
    identity: TrustedInstallationIdentity, // private
    target: InstallationTargetCapability,  // private path/handle/package identity
    local_version: Option<String>,
    eligibility: CandidateEligibility,
    evidence: Vec<InstallationEvidence>,
    location_label: BoundedLocationLabel,
}

struct FreshInstallDestination {
    destination_id: OpaqueDestinationId,
    destination_revision: OpaqueRevision,
    scope: InstallationScope,
    owner: InstallationOwner,
    requires_elevation: bool,
    eligibility: DestinationEligibility,
    location_label: BoundedLocationLabel,
}
```

`InstallationTargetCapability` is not serializable and is created only from platform-owned evidence. It may hold a canonical bundle path, a pinned Windows package identity or another backend capability, but never accepts one from IPC.

Candidate IDs should be snapshot-scoped random opaque IDs or another non-path-derived opaque representation. Revisions bind identity, scope, owner, evidence generation and relevant version. A guessed/replayed ID cannot authorize a path because execution always performs a fresh lookup and comparison.

## 3. Wire contracts

Prefer a distinct inventory command instead of inflating readiness with a full list:

```text
get_agent_installation_inventory({ agentId })
  -> InstallationInventoryDto {
       contractVersion,
       inventoryId,
       agentId,
       state,
       candidates[],
       freshDestinations[],
       reasonCodes[]
     }
```

Candidate wire projection contains only bounded user-facing fields. `locationLabel` is a display value, not a capability. A separate closed action may reveal a currently revalidated candidate in Finder/Explorer; it accepts only `inventoryId + candidateId + revision`.

Evolve lifecycle request semantics:

```text
StartAgentActionRequest {
  agentId,
  action,
  inventoryId?,
  targetId?,
  expectedTargetRevision?,
  expectedReleaseId?
}
```

Rules:

- update: existing candidate required;
- install: fresh destination required unless package adapter delegates target choice to a verified vendor installer contract;
- launch: target required when inventory is not uniquely trusted;
- Auth actions: target fields rejected unless the desktop launch adapter explicitly requires an installed candidate, and Stage 4 owns final semantics.

## 4. Inventory state

```text
not_observed  adapter ran and found no trusted candidate
single        exactly one trusted executable candidate
multiple      two or more trusted candidates
unsupported   no reviewed inventory adapter on this platform
unknown       scan failed, timed out, or evidence was insufficient
```

Untrusted/stale observations may be included as non-actionable rows without changing `single/multiple` trusted counts. Failure dominates; partial evidence does not silently become not-installed.

## 5. Deduplication policy

Deduplication is identity-based, not display-based:

- macOS: verified bundle identity + canonical bundle record;
- Windows packaged app: exact package identity/PFN/AUMID as applicable;
- Windows unpackaged app: canonical executable file identity plus closed product identity and registration evidence;
- same version/display name is insufficient;
- registry and App Paths that resolve to the same canonical executable merge;
- stale registration pointing to a missing target remains a separate non-actionable observation.

Conflicting version/publisher/scope evidence is retained and produces a conflict reason; policy does not choose whichever source was read last.

## 6. Safe location projection

Known roots map to symbolic labels:

```text
/Applications/<App>.app
~/Applications/<App>.app
%LOCALAPPDATA%\Programs\...
%PROGRAMFILES%\...
Windows package: <package display name> (<scope>)
```

User profile names and backend-only canonical paths are redacted. Custom locations receive a bounded, reviewed display label and a controlled reveal action. Labels are never accepted back as identifiers and are excluded from logs/errors by default.

## 7. Reuse decision

First evaluate lifting a product-neutral candidate core from `codex_desktop::platform`:

- `TrustedInstallationCandidate` already represents a verified installation with scope and private stable key;
- `RestartCandidateInspection` already expresses not-installed/trusted/ambiguous/untrusted;
- `PreparedInstallPackage` already carries a retained package capability.

Do not expose those Codex-specific types directly to Agent pages. Preferred shape:

1. extract shared private primitives only where their invariants are truly product-neutral;
2. keep Codex adapters and Agent adapters behind their existing facades;
3. add compatibility tests proving Codex behavior is unchanged.

If extraction would weaken Codex's exact identity rules, keep the implementation in its current owner and introduce a narrow adapter to a shared inventory interface instead of copying internals.

## 8. Frontend design

`shared/features` owns DTOs, strict parsers, query keys and a target-selection view model. `shared/ui` owns one picker/dialog that renders:

- existing candidates;
- fresh destinations;
- scope/owner/version/location label;
- disabled reason and refresh action;
- selected target with `aria-checked`/radio semantics;
- explicit ambiguity state.

The component does not initiate installation. Agent pages pass the selected typed target to the lifecycle controller.

## 9. Compatibility and migration

- Keep existing readiness consumers while introducing the inventory query.
- Bump readiness/action contract versions only when parser and tests land together.
- During migration, unique trusted candidates may satisfy legacy launch calls; legacy update calls must not run if target authority is ambiguous.
- No persisted path migration is needed because current requests do not carry candidate paths.

## 10. Error codes

Add closed reasons as needed, for example:

```text
target_selection_required
target_changed
target_not_executable
target_scope_unsupported
inventory_expired
candidate_conflict
```

Do not overload `interactive_user_unavailable` for every target problem; that code describes process/user-context capability, not inventory ambiguity.
