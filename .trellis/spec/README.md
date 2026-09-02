# FyAgent Trellis Specifications

This directory contains the durable engineering contracts for the current
FYAgent repository. Specs are written for implementation and review: they
identify an owner, define executable boundaries, name failure behavior, and
point to tests. Task PRDs, investigation logs, release notes, and one-off
acceptance evidence belong under `.trellis/tasks/`, not here.

## Read the smallest sufficient set

1. Start with the layer index: [backend](./backend/index.md),
   [frontend](./frontend/index.md), or [thinking guides](./guides/index.md).
2. Select the focused contract whose trigger matches the task.
3. Follow only the cross-layer links named by that contract.
4. Load a compatibility router only when an old task references it or the
   change spans several focused contracts.

Do not inject an entire layer by default. A focused change should normally
need one primary spec plus a small number of linked contracts.

## Document classes

### Focused executable contract

A focused code-spec owns one implementation concept. Commands, DTOs,
cross-layer flows, database behavior, and infrastructure integrations use this
seven-part structure:

1. Scope / Trigger
2. Signatures
3. Contracts
4. Validation & Error Matrix
5. Good / Base / Bad Cases
6. Tests Required
7. Wrong vs Correct

The document names concrete repository paths and assertions. It does not
repeat a task history or use examples as a substitute for a contract.

### Compatibility router

A compatibility router preserves a stable path that is referenced by archived
tasks. It contains a reading map and only the invariants shared by all linked
contracts. New work should cite the focused contract instead of expanding the
router back into a monolith.

Current compatibility routers include:

- `backend/external-agent-p0.md`
- `backend/external-agent-configuration.md`
- `frontend/v2-agent-models.md`
- `frontend/v2-skills-mcp.md`
- `frontend/v2-shell.md`

### Thinking guide

Files under `guides/` are short review checklists. They help an engineer decide
what to inspect, then link to executable specs for the actual rules. A guide
must not become a second authority for command payloads, persistence, security,
or UI behavior.

## Authority and evidence

Use this order when facts disagree:

1. current production source, manifests, generated metadata, and executable
   tests in this checkout;
2. the focused spec that describes those owners;
3. a compatibility router;
4. an archived task or historical note.

Specs must be updated after a verified implementation changes the contract.
Do not preserve a stale statement merely because an old task cited it.
Conversely, do not promote an unverified observation or proposed behavior into
a durable contract.

Prefer references to code-owned constants over duplicating volatile versions,
remote release locators, hashes, or timestamps. Freeze a literal in a spec only
when the literal itself is an interoperability or security boundary and tests
assert it.

## Ownership and cross-layer rules

- One side-effect, secret, source URL, parser, state machine, or layout recipe
  has one authoritative owner.
- Renderer code submits closed semantic IDs and bounded mutation data. Native
  code owns filesystem paths, process launch, registry access, installer
  identity, and other host effects.
- Unknown, unavailable, partial, and unverified states remain distinct from
  absence and success.
- Cross-layer specs link to one another instead of copying full DTO or workflow
  descriptions into both layers.
- Shared frontend geometry and controls live in shared owners; feature specs
  describe only feature-specific composition and policy.

## When to split or merge

Split a spec when independent tasks can change different owners without needing
the rest of the document, or when a compatibility path contains several
unrelated command/state/persistence contracts. Preserve the old path as a
router when archived references are common.

Do not split solely by line count. A cohesive security, release, migration, or
transaction workflow may remain long when reviewing it end-to-end is safer
than distributing its invariants across several files.

Merge or remove a spec when it only duplicates another authority, documents a
completed task rather than a lasting rule, or cannot name a current owner and
testable behavior.

## Maintenance checklist

- Every non-index spec is reachable from its layer index.
- Relative links resolve and concrete repository paths exist.
- New infrastructure and cross-layer contracts contain all seven sections.
- Tests name observable assertions, not only commands to run.
- `TODO`, `TBD`, phase plans, stale dates, and one-off acceptance logs are not
  left in durable specs.
- A compatibility router stays small and never regains duplicate detail.
- `git diff --check` and the repository contract gates pass after changes.
