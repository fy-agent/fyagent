# Validation and handoff

Date: 2026-09-21. Workspace: parent integration checkout on
`codex/next-iteration-engineering-20260920`. This subtask did not commit or stage
the shared tree. Parent owns the final commit, native validation and UI wiring.

## Implemented contract

- `get_provider_summary` returns independent actual `live` state and saved DB
  sources. It uses the existing target resolver and live reader, never network,
  a configuration writer, or a secret resolver.
- Closed live state is `{ target, state, exists, connection }`. The states are
  configured, not_configured, missing and unreadable. Configured has at least
  one safe model/endpoint; unspecified baseUrl/modelId/protocol stay null.
- Codex selected provider and explicit file profile are honored. Saved current
  ID cannot supply live routing. Known live/auth/header/TOML and saved-source
  credential collisions fail the live projection closed with no raw error.
- Missing, malformed or inaccessible files and failed write-target metadata
  preserve the valid saved-source list; original saved summary safety failures
  still reject the summary. No raw config, key, token, reference or error string
  is exposed by the new field.
- Runtime parsing isolates invalid/unsafe/wrong-target live members. Old hosts
  without the member produce unreadable/unknown, never missing. Native always
  sends live. UI must not equate unknown with no existing configuration.

## Final focused checks

After the final source change:

```text
rtk proxy mise run test:unit -- tests/renderer/platform/providerLiveSummaryPort.test.ts tests/renderer/platform/providerApiPort.test.ts tests/renderer/platform/featurePorts.test.ts tests/architecture/rustModuleBoundaries.test.ts
PASS: 4 files, 65 tests.

rtk proxy mise exec -- pnpm exec eslint src/shared/features/models.ts src/shared/features/types.ts src/shared/platform/tauri/feature-ports/models.ts src/shared/platform/tauri/feature-ports/providerLiveSummary.ts tests/renderer/platform/providerLiveSummaryPort.test.ts
PASS.

rtk proxy mise exec -- rustfmt --check --edition 2021 src-tauri/src/commands/provider.rs
PASS, including owned child modules. Full-tree mutation formatting was avoided.

rtk proxy git diff --check -- <owned tracked paths>
PASS.
```

`rtk proxy mise run typecheck` ran before the final control-character predicate
lint adjustment. It reported only the shared-tree errors below, outside this
package. Parent was notified; this worker did not edit that file:

```text
tests/remainingPlatformSurface.test.ts:764 and :769
  TS2339 Property 'block' does not exist on type 'RustAllowance'.
tests/remainingPlatformSurface.test.ts:771
  TS7006 Parameter 'part' implicitly has an 'any' type.
```

## Pending acceptance

- Parent runs final typecheck after reconciling concurrent changes.
- Rust compilation, Clippy and tests were deliberately not run in this worker,
  following the parent's serial native-build ownership. New native test filter:
  `commands::provider::live_summary` (8 tests).
- The native tests use an actual temporary home/file and an in-memory DB for
  external routing vs DB current, missing/empty/corrupt files, metadata failure,
  corrupt auth, raw/encoded credential collisions and unchanged file bytes.
  They are source/test coverage, not executed native evidence yet.
- Root wires FirstUseGuide/AgentModelsSection. No UI/Page file is changed here.
  Keep/replace must distinguish unknown from absence and must not present
  configured as proof of valid credentials or connectivity.
- No full renderer build, browser run, model request, release or UAT evidence
  is claimed by this package. The observation describes explicit file state;
  consumer defaults, CLI/process overrides and runtime behavior remain unknown.
