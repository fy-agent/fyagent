# R1 / R2 parser implementation handoff

## Result

Implemented native provider preservation and paired OpenCode model UI protection. No real Agent configuration, credentials, login, installed app, or network credential resolver was touched. All native execution used temporary fixtures / in-memory database; renderer tests are mocked-port tests, not native application UAT.

- OpenCode npm and model name are optional. Provider/model-limit extension fields survive typed projection. Imports persist the complete original JSON, not a typed subset. Live writes copy the original object through existing writers; missing full-config provider fragments fail closed. Removed now-unused lossy get/set typed-provider helpers.
- OpenClaw apiKey remains native opaque JSON (string/object/null/absent), with explicit null preserved. Debug output is redacted. Parsing/import/copy never stringify or resolve vendor references. String-only credential extraction rejects opaque values. Public summary stays id/name-only and rejects collisions with strings inside opaque credential objects.
- OpenCode model snapshots add required `editable: boolean`. Existing missing npm or unsupported provider/options/model-map shape is read-only, with native rejection before backup/write. Only explicit new providers receive the default npm package.
- The UI selects an existing provider by exact ID, defaults to the first editable entry, exposes readonly builtin summaries and permits explicit creation in builtin-only configurations. Switching dirty drafts requires discard confirmation; cancel retains target/draft/key.
- Save requests add `providerId: string | null`. A string must match an existing exact ID. Null explicitly creates a provider and rejects derived-ID collisions. The old single-provider/name fallback is removed. Overwrite HMAC v2 binds providerId, so tokens cannot be moved to another provider.

## Commits and ownership

- `8a86c481d0d0c0d8a7f4a32afcad433855ed31b8` — preserve opaque vendor provider documents (7 native/domain files).
- The second Models commit contains the dedicated service, DTO, strict port, panel, tests, focused specs and this handoff. Read its SHA from Git history after commit.
- No health, managed_auth, agent_install, MCP, release/dependency or model_probe implementation was changed by this owner.
- Shared integration: `src/shared/features/models.ts`, `src/shared/platform/tauri/feature-ports/models.ts`, `src/pages/models/OpenCodeModelsPanel.tsx` can overlap the evidence line. Merge exact providerId/eligibility changes without dropping probe work.
- Readiness owner already committed `tests/renderer/pages/agents/Page.test.tsx` OpenCode editable fixture. The Models commit removes an accidental editable field from the TRAE browser fixture in `tests/browser/support/features.ts` (OpenCode's fixture has no providers and needs no field).
- `src-tauri/src/opencode_config.rs` is in the protected platform-source review manifest; root owns the final seal/hash. This owner did not edit that manifest or platform path branches.

## Executed verification

| Command                                                                                                            | Actual result                                                                 |
| ------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------- |
| `rtk mise run test:unit -- tests/renderer/pages/models/Page.test.tsx tests/renderer/platform/featurePorts.test.ts` | 53/53 PASS on Vitest 4.1.11, 49.86s after providerId/selector correction      |
| `rtk mise run typecheck`                                                                                           | PASS after final product edits                                                |
| Scoped `rtk mise exec -- pnpm exec eslint` for six affected TS/TSX production/test files                           | PASS after final product edits                                                |
| `rtk git diff --check`                                                                                             | PASS                                                                          |
| `rtk mise run rust:test -- config_reliability_`                                                                    | Final root execution: 7/7 PASS, 0.56s fixture runtime; log directly inspected |
| `rtk mise run rust:test -- services::opencode_models::tests`                                                       | Final root execution: 8/8 PASS, 0.20s runtime; log directly inspected         |

Native filter evidence: `research/verification/rust-config_reliability_.log` and `research/verification/rust-services-opencode_models-tests.log`. The filters overlap; do not describe them as 15 unique tests. Their initial queued run had only 5 cases and was superseded by the final 7-case run. The first earlier build was blocked by an unrelated MCP fixture compile error; root fixed that fixture before final verification.

Fixtures cover native import → DB → live round-trip, provider neighbors and nested unknown fields, literal/object/null/absent credentials, no npm injection, secret-free summaries/snapshots, read-only rejection with unchanged bytes/no backup, exact IDs despite display-name changes, explicit creation beside builtin, collision/missing-target rejection, HMAC target binding, stale revisions and existing backup/overwrite behavior.

## Review correction and limits

Root review found that first-provider-only UI blocked editable neighbors/new custom providers. That finding is fixed by the selector plus exact native target contract, with mixed/builtin-only UI/native regression tests. Earlier review text is historical, not the final state.

OpenClaw has no current Models target/editor; this change repairs its existing native import/readback/summary consumers and does not claim a new editor, credential resolver or login flow. Existing legacy raw OpenClaw command boundaries are unchanged; no new raw renderer DTO is introduced. JSON semantic preservation is verified, not byte-for-byte OpenCode formatting preservation. Real installed Agent acceptance and final cross-line/native application UAT belong to root.
