# Issue requirement mapping

## #31 — Multiple installations

https://github.com/fy-agent/fyagent/issues/31

- List detected locations, versions, environments and conflicts.
- Let the user choose the target managed in the current operation.
- Keep the current usable version when update fails.

Stage 1 owns the list/choice/conflict contract. Stage 2/3 own failure-safe platform execution.

## #47 — Preserve and reread

https://github.com/fy-agent/fyagent/issues/47

- Existing installs default to no write.
- Multiple installations or source drift require explicit authority selection.
- Observation must not be named verification.

Stage 1 keeps inventory read-only and represents unknown separately from absent.

## #101 — Directory and takeover PRD

https://github.com/fy-agent/fyagent/issues/101

- Agent directory is the only capability source.
- Machine observation has `observed/not_observed/not_supported/unknown` semantics.
- UI cannot derive support from a logo, display name or local appearance.

Stage 1 preserves Catalog ownership and adds a separate installation-inventory observation contract.

## #141 — UAT evidence

https://github.com/fy-agent/fyagent/issues/141

Windows UAT observed registry/executable/CLI/parallel-install version drift. This is evidence that first-match fixed-root detection is insufficient. Stage 1 must retain conflicting candidates and provenance; Stage 3 adds the actual Windows evidence adapters.
