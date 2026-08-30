# Code Reuse Thinking Guide

Use this guide before creating new code or adding a dependency. Binding
frontend rules live in [Frontend Reuse](../frontend/reuse.md); backend rules
live in [Backend Reuse](../backend/reuse.md).

## Decision order

1. **Existing FyAgent owner** — reuse or minimally extend the current shared
   component, service, parser, façade, platform adapter, or primitive.
2. **Already adopted primitive** — check the standard library, framework, and
   dependencies already present in repository manifests.
3. **Maintained open-source candidate** — research official documentation,
   repository, releases, license, advisories, platforms, and footprint.
4. **One FyAgent adapter** — contain external APIs and project semantics at the
   correct shared boundary rather than exposing package-specific glue widely.
5. **Bespoke implementation** — use only when the earlier options are
   unsuitable; record the concrete reason for non-trivial choices.

This order is not permission to add a large dependency for a small helper.
Architecture, security, secret, native-platform, and licensing constraints are
stronger than reuse convenience.

## Search before design

Search names, behavior, tests, types, commands, and manifests in the relevant
tree. Look for the semantic owner, not only an identical function name.

```bash
rg -n "concept|command|component|error code" <relevant-paths>
```

Ask:

- Does an existing owner already validate or normalize this data?
- Is another route/module already using the UI or orchestration pattern?
- Would extending the owner preserve a clear API, or create speculative flags?
- Is the apparent duplicate actually separated by a security/platform boundary?
- Does a dependency solve the reviewed requirement without introducing a
  competing framework or excessive transitive surface?

## Review an external candidate

Confirm from primary sources:

- required capability in the version being considered;
- license and redistribution compatibility;
- maintenance/release ownership and known security advisories;
- macOS/Windows production support and development-host needs;
- runtime, bundle, build, and transitive dependency cost;
- API stability and whether one project adapter can contain future churn.

Reject a candidate that weakens an existing authority boundary even when its
API is convenient.

## Decide where the owner belongs

Share on the first implementation when there is an existing shared owner, a
current second consumer, or a concrete near-term sibling. Keep code local when
it is genuinely one-off and a shared abstraction would require speculative
parameters.

- Frontend placement follows [Frontend Reuse](../frontend/reuse.md) and
  [Directory Structure](../frontend/directory-structure.md).
- Backend placement follows [Backend Reuse](../backend/reuse.md) and
  [Rust Host Modular Boundaries](../backend/modular-boundaries.md).
- Cross-layer decoding has one boundary owner; consumers receive typed
  projections rather than repeating raw payload extraction.

## Review after implementation

- Search again for missed copies and parallel constants.
- Confirm all consumers use the same semantic owner.
- Confirm the abstraction did not expose secrets, paths, commands, platform
  internals, or package-specific types beyond its boundary.
- Keep action/status transitions in one exhaustive reducer/dispatcher rather
  than scattered conditionals.
- Record why a viable shared or external option was not used when the decision
  is likely to be revisited.

## Stop signs

- Copying leftover renderer UI into V2 instead of using V2 owners/ports.
- Creating page-local tabs, search, pagination, dialog, assignment, or split
  layout while a shared owner exists.
- Parsing the same Tauri/event/config payload independently in multiple files.
- Adding a second constant table for IDs, order, URLs, paths, or capabilities.
- Adding a dependency without official capability/license/security/platform
  review.
- Generalizing a single one-line case only to satisfy an abstraction rule.
