# Configuration reliability final stage review

Review date: 2026-09-19. Read-only product review. No product edits or additional test runs.

## Result

No new blocking defect found in the reviewed slice. The five commits are suitable for integration together, retaining their native DTO / renderer parser / UI coupling. This is a fixed-commit review with existing fixture evidence readback, not installed-application UAT.

## Fixed scope

Repository: `~/.codex/worktrees/fyagent-fde-reliability/fyagent`.

Base `cf784343`; reviewed commits:

- `dea92576` — readable unknown Grok profile and categorized drift.
- `8a86c481` — opaque vendor provider preservation.
- `42207182` — read-only Health overview, selector and path discovery.
- `375f24b8` — library-only MCP creation and validation before mutation.
- `1d94030b` — exact OpenCode provider targets and mixed-provider UI.

Used `git show`/`git diff` at these revisions for product authority. Did not substitute the root integration worktree for this scope. Later release/structure-test changes (`0adb4b31`, `226b8958`) and earlier dependency/notarization changes are outside this verdict.

## Behavior reviewed

- **R1:** Import and live write now preserve the raw OpenCode provider object; typed projections supply advisory metadata only. Missing npm is not injected. Model snapshot marks such a provider read-only. Mixed provider lists expose an explicit selector and an independent new-provider choice. Native save resolves existing identity only from providerId, rejects missing/stale IDs and slug collisions, and binds overwrite tokens to that same identity. It retains optimistic revision checks, backup creation after validation, unknown fields and unrelated providers. Both native and renderer fixtures exercise builtin-plus-custom behavior.
- **R2:** OpenClaw apiKey string/object/null/absence remains native opaque Value with no resolution or stringification. Raw import/live write avoids typed reserialization loss. Debug redacts the entire typed config; public provider summary stays id/name and rejects collisions with opaque credential string leaves. Round-trip fixture covers literal, vendor reference, unknown nested object, explicit null and absence.
- **R3:** Grok profile unknown is selected only after successful TOML parsing and an unrecognized non-official route. Invalid TOML remains unreadable. No guessed vendor catalog, write permission or claimed authentication was added.
- **R4:** Drift returns only closed source/endpoint/model/credential category codes. Reads continue to preserve source documents and selection; action remains navigation to configuration, with no implicit repair command. Credential-state drift is not advertised as a test of credential value equivalence or authentication success.
- **R6:** Health admission calls observe_overview rather than management reconciliation. Projected proxy slots stay in memory; the management overview retains its separate persistence behavior. Proxy route observation uses read-only selection lookup and returns unknown for stale selection instead of repairing it. Tool Health now passes no login-shell path to the pure path collector; the production Health path no longer invokes the login shell. Existing fixture logs include the admission/collect preservation case, direct overview DB/vault/native-byte invariance, stale proxy selection and the shell-execution marker case.
- **MCP:** Global manual creation starts with no assignments; an explicitly passed context target selects only that target; editing clones stored flags. Native base validation runs before lock/read/persist/live-write work and rejects a present non-string type instead of treating it as missing. Valid library-only entries preserve unknown extensions and do not create target configuration directories.

## Earlier stage findings reconciled

The earlier report `research/implementation-review.md` described the pre-freeze state and correctly recorded pending work. In the fixed reviewed slice:

- Mixed OpenCode provider lockout is closed by explicit target selection and exact native identity.
- MCP invalid type fallback is closed by an explicit absent/string/invalid split before mutation.
- Health admission integration preservation fixture exists and its execution appears in the verified Health/Managed Auth logs.
- Health login-shell side effect is closed by the pure path collection call and marker fixture.

These conclusions supersede only those earlier findings, not their history or unrelated review scope.

## Evidence readback

Read `.trellis/tasks/09-19-fde-config-reliability/research/verification/results.json` and the actual five referenced log files. Each file SHA-256 matched its recorded digest, each had successful nonzero test groups, and the relevant named regressions were present:

| Filter | Passed | Evidence |
| --- | ---: | --- |
| config_reliability_ | 7 | opaque import/write, null and extension preservation, builtin read-only, exact target, target-bound overwrite |
| health | 41 | admission/collect preservation, managed overview invariance, no-login-shell, drift/Grok/native OAuth regressions |
| managed_auth | 125 | preservation and route observations; overlaps Health |
| mcp_library_only_upsert | 1 | invalid inputs leave DB unchanged; valid zero-target entry creates no native target directories |
| services::opencode_models::tests | 8 | target/creation/overwrite/read-only and existing model persistence; overlaps config filter |

Do not sum these counts: filters overlap. The log files are fixture/temporary-home/loopback native evidence, not real-account, installed-app or remote-release acceptance. Renderer test additions were inspected, including exact providerId payload and zero/one explicit MCP assignment; previously recorded renderer/typecheck results were not independently rerun in this review.

## Integration boundary

Native OpenCode providerId and editable, strict renderer parsing and the selector panel must ship together. Root still owns final combined checks and native app walkthrough after integration. No additional architecture, broad tests, release research or product UI explanations are requested by this review.
