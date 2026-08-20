# Implement

Parent does not implement product code. Children implement in parallel; parent then runs a focused check and updates specs.

## Order

1. Children implement independently.
2. Parent: `mise run typecheck` plus the touched Vitest/Rust suites; no full `mise run check` unless a child reports a cross-cut failure.
3. Update `.trellis/spec` for installer, V2 agent/models/skills-mcp, release, and test/docs contract notes.
4. Do not commit unless the user asks.

## Validation

- `mise run typecheck`
- Vitest files each child names in its `implement.md`
- `mise run rust:test` only if installer/stream_check changed
