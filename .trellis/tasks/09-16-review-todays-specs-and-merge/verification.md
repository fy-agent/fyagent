# Verification

## Completed focused checks

```text
python ./.trellis/scripts/task.py validate \
  .trellis/tasks/09-16-review-todays-specs-and-merge
```

- Result: pass; `implement.jsonl` 7 entries, `check.jsonl` 6 entries.

```text
mise run format:check
```

- Result: pass; all matched source/config files use repository Prettier style.

```text
mise run test:unit -- \
  tests/renderer/pages/prompts/presets.test.ts \
  tests/renderer/features/mcpCatalog.test.ts \
  tests/miseTaskContract.test.ts
```

- Result: pass; 3 files, 64 passed, 1 repository-declared skip.
- Covers exact prompt categories/count/search, exact 20-ID MCP membership and the current
  SwiftPM-before-leftover-Xcode helper order.

```text
mise run typecheck
```

- Result: pass.

```text
mise run check:contracts
```

- Result: pass.
- Task/API/docs/lock/version/release/platform contracts passed.
- Contract suite: 34 files, 619 passed, 1 repository-declared skip.
- Native-fetch contract: 1 file, 4 passed.

## Pending completion gates

- After the work commit, archive this task, record the session, rerun
  `check:contracts`, then validate the exact pushed PR head and merge-group CI.

## Full prearchive gate

```text
TRELLIS_CONTEXT_ID=fyagent-review-20260916-specs \
  mise run check:prearchive \
  --exclude-active-task .trellis/tasks/09-16-review-todays-specs-and-merge
```

- Result: pass; exit code 0.
- Environment, typecheck, lint, formatting, desktop mock/preflight, Rust fmt/check/clippy,
  task/docs/lock/version/release/platform contracts all passed.
- Full Vitest suite: 189 files, 1713 passed, 1 repository-declared skip.
- Rust main library: 3264 passed, 5 explicitly ignored; all executed binary,
  integration, helper and doc-test targets passed with zero failures.
- Release contract suite: 34 files, 619 passed, 1 repository-declared skip;
  native-fetch contract: 4 passed.

## Structural checks

- `git diff --check`: pass.
- Trellis task validation: pass; 7 implement and 6 check context entries.
- Relative-link scan: all 103 current SPEC/task Markdown files resolve.
- All required injected SPEC files fit the default 32768-byte limit; largest
  reviewed required file is `frontend/managed-auth.md` at 29477 bytes.

## Evidence limits

- No real OpenAI/xAI subscription login, quota or model-entitlement call was run.
- No signed privileged-helper install/HIL or Windows desktop HIL was run.
- This is sufficient for a SPEC/test-only audit because no product runtime behavior changed;
  today’s implementation tasks retain their separately recorded browser/native evidence.
