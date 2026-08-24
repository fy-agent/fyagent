# Design

## Owned shared paths

- `src-tauri/src/database/schema.rs`, sync/backup preserve owners and migration tests
- `src-tauri/src/commands/mod.rs` or nearest command composition owners
- `src-tauri/src/lib.rs` and application ACL manifests/tests
- `src/v2/shared/platform/ports.ts`
- `src/v2/shared/platform/tauri/features.ts` and browser/Tauri composition owners
- route/page connection points not owned by first-wave components
- cross-layer integration and architecture tests

Do not use Git or GitHub.

## Integration sequence

1. Inspect worker diffs and reconcile exported contracts without duplicating domain logic.
2. Implement schema v20 helper/migration and local-only sync invariants.
3. Register commands and exact capability union.
4. Wire strict frontend parsers into FeaturePorts and native-only browser ports.
5. Connect Models/Agents consumers and run focused Rust/V2 tests.
6. Search for old schema versions, fake coordinator, scenario, installer write commands, old Agent IDs and duplicate catalog surfaces.

## Shared contract rule

All parsing happens once at the platform adapter. Pages consume typed feature-domain values. Rust response keys and TS parsers must be exact, including rejection of excess fields and unknown enums.
