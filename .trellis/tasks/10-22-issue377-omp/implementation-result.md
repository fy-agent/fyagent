# C component handoff

The implementation adds Oh My Pi to the software catalog and Models page, with typed read/preview/apply/discovery commands. Catalog v6 contains eight entries; the existing seven MCP/Skills/Prompts assignment identities remain unchanged. OAuth continues to belong to omp, while this editor supports custom API providers and preserves existing authentication fields.

The native service uses yaml-edit 0.3.3 CST editing plus independent YAML semantic validation, revision-bound opaque plans, the existing private atomic-write/recovery owner, and compensation limited to the bytes written by this operation. It preserves unrelated providers, fields, comments and model roles. New nested collections are populated before insertion so flow-style YAML stays valid. Blank OMP_PROFILE and relative PI_CODING_AGENT_DIR follow official v18.12.5 resolution semantics.

Windows discovery uses the existing authenticated ordinary-user helper, with OmpProbe action 23 and terminal kind 8. The four app commands are registered and permitted, and blocking file work runs off the async command thread. This component retains its baseline wire version 4; the parent must merge it with CodexModels action 22 / kind 7 and retain the integrated version 5.

## Verification

- Canonical native runner with `usage_filters=omp_`: 34 main-crate tests and 4 helper tests passed. These include actual read → preview → apply, revisions, conflict rejection, rollback, secret redaction, profile resolution, final output size limits, flow YAML and command registration/permissions.
- The full helper library batch passed 90 tests; the Windows helper-runtime binary check succeeded.
- The catalog/runtime/complete ACL batch passed 10 tests, including the exact 406-command set after the four OMP commands were added. The five focused repository contracts passed 82 tests.
- Renderer: 191 focused tests passed, typecheck/lint/format passed, and the production bundle passed the unchanged route budgets (8 routes; 646,194 initial JS bytes and 46,993 CSS bytes).
- Browser fixture: all 10 new cases have passing evidence across the initial and targeted reruns. This was not a single all-green run: a selector was corrected, and one 900×600 startup timeout under parallel native compilation passed in an isolated rerun without increasing the timeout.
- The official omp 18.12.5 binary reads the production service's isolated output: `config path`, `config get modelRoles`, and `models find fyagent-issue377-fixture --json --no-extensions` all exited 0 with empty stderr. Both the default role and discovered model use `fyagent-issue377-fixture/issue377-fixture`. Host evidence is retained outside the repository; it contains no real account credentials and sends no inference request.

The verified component is ready for the parent's A/B/C integration. The active task stays open for that integration handoff. Packaged native UI, real ordinary-user helper RPC and authenticated provider generation are separate acceptance layers; the browser fixtures and CLI parsing do not claim those results.

## Implementation provenance

Real international CodeBuddy CLI 2.172.1 produced the native implementation through Auto and the available `primary-model` alias. Host review rejected the first hand-written parser, required the CST rewrite, repaired the concrete review findings and verified the final behavior. Real Cursor Auto produced the frontend. No underlying vendor model is inferred from either alias, and interrupted tool sessions are not recorded as successful tests.

## Integration notes

Keep the union of A/C helper actions, terminal frames, handlers, permissions and payload bounds. Review shared changes in `src-tauri/src/lib.rs`, the Windows helper transport, `src-tauri/user-helper/src/{cli,protocol,main,windows}.rs`, catalog/runtime DTOs, feature ports/browser fixtures and the platform structure manifest. The parent owns combined verification and publication.
