# Single Renderer Migration

The product entry is `src/index.html` → `src/main.tsx`. Product code is organized
by `app`, `pages`, `widgets`, `shared` and portable `domain` responsibilities;
there is no alternate generation directory or runtime selector. The migration
baseline is commit `ec1393c1`; Git retains the retired implementation and its
historical test evidence.

## Owner map

| Previous owner                                                                             | Current owner or disposition                                                                                                                                                                                 |
| ------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `src/v2/**`                                                                                | The same product implementation under role-based `src/**`.                                                                                                                                                   |
| Portable `src/shared/codex-desktop/**`                                                     | `src/domain/codex-desktop/**`; renderer installer composition remains in `src/shared/codex-desktop/**`.                                                                                                      |
| `src/types/codexDesktop.ts`, old Codex version-state facade                                | `src/domain/codex-desktop/types.ts` and `versionState.ts`.                                                                                                                                                   |
| Configuration presets, DTOs and serialization under `src/config`, `src/types`, `src/utils` | Retained portable contracts in `src/domain/configuration`, with strict unknown-input guards and their behavior tests.                                                                                        |
| `AppSwitcher`, `components/topbar`, renderer `lib/layout`                                  | Retired UI. Current route/sidebar ownership is `app` and `widgets/app-shell`; native logical-window policy remains unchanged. The old switching UI is not a second product path.                             |
| `components/workbuddy`, `lib/api/workbuddy`, `lib/query/workbuddy`                         | Retired UI/adapters. Current model editing, native summaries and Change Plan reconciliation belong to Models and shared typed ports. Old isolated UI evidence is historical, not current feature acceptance. |
| Old `useCodexRestartCoordinator` and `CodexRestartDialog`                                  | Retired with their unreachable renderer. Native restart/activation/exit contracts remain unchanged; this migration does not introduce a replacement restart UI.                                              |
| `src/i18n` and `localeKeyParity.test.ts`                                                   | Retired translation runtime. Current product copy remains Simplified Chinese; multilingual manuals are not evidence of a runtime language selector.                                                          |
| `deplink.html`, offline HTML builder and self-contained preview                            | Removed. Browser regressions use the ordinary HTTP module entry and production bundle. Native deep-link parsing/security tests remain.                                                                       |
| Version-prefixed tests/configuration/tasks                                                 | `tests/renderer`, `tests/browser`, `tests/domain`, one TypeScript/ESLint/Vitest configuration and role-named tasks.                                                                                          |

Archived JSONL context may resolve here when its former UI owner was retired.
The historical reason and reported test result are intentionally not rewritten
as evidence for new code. Source-history references can be inspected at the
migration baseline, while current work should read the focused code-spec.

## Verification and compatibility

`mise run test:unit` runs the renderer and contracts projects together. Use
`mise run typecheck`, `mise run lint`, `mise run test:browser` and the full
prearchive check. The browser gate includes production initialization as well
as development-server geometry and interaction checks; neither proves a native
Windows or macOS release acceptance run.

The persisted quick-setup provider IDs, opaque native IDs, DTO versions and
third-party model versions are unchanged. Only the nonpersistent query-cache
namespace becomes `renderer`; readers and prefix invalidations share factories.
The host code, native command/ACL surface, credentials and user configuration
files are not migration targets.

The native `fyagent://` parser is retained. The current product renderer had
not connected the old general import-confirmation dialog before this migration;
opening a protocol URL must not be advertised as proof of an imported setting.
No HTML generator or user-downloaded test entry replaces that missing product
flow.

All retained raster bytes match the pre-migration sealed digests. Five product
rasters move without alteration and 27 retired-only rasters are removed, leaving
121 entries. This is an identity-preserving inventory update, not a visual
baseline recapture. Deleted renderer tests are not counted as passed tests.

See the [current frontend index](../../../.trellis/spec/frontend/index.md) and
[directory contract](../../../.trellis/spec/frontend/directory-structure.md).
