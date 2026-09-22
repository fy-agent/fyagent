# Execution result: remove client projects

Status: **executed, not accepted**. Task remains `in_progress`. This is the Grok implementation report. It does not claim GPT acceptance.

## Baseline

| Item | Value |
| --- | --- |
| Worktree | `/Users/<username>/.codex/worktrees/remove-client-projects/fyagent` |
| Branch | `codex/remove-client-projects-20260921` |
| HEAD at start of implementation | `c0b2ec21caccd33c082884270aa2e5dbfb7e9d9e` (`Merge pull request #195 from fy-agent/codex/next-iteration-engineering-20260920`) |
| Requested origin/main baseline | `c0b2ec21caccd33c082884270aa2e5dbfb7e9d9e` |
| HEAD evidence | Raw `git rev-parse HEAD` is `c0b2ec21…`. The earlier `21bd05d2` line was a misread of an RTK log that hid the merge; the branch never moved. |
| Current `SCHEMA_VERSION` | `25` (was `24`) |
| Commit | none (diff left uncommitted) |
| PR / push / install | none |

Architecture used current `src/pages`, `src/app`, `src/shared`, `src-tauri` owners. Old V2 paths in the planning notes were not treated as live.

## What changed

Customer projects, delivery kits, and project verification are removed from the product. They had no remaining callers outside the retired workspace.

### Product surface

- Navigation no longer has `客户项目` / `/projects`.
- Eight primary routes remain: agents, health, auth, models, skills, mcp, prompts, memory.
- Unknown hashes, including `#/projects` and `#/projects?project=…`, replace-redirect to `/agents`.
- Page, domain, ports, query keys, and Tauri feature-ports for projects / delivery-kits / verification are gone.

### Native host

- Commands, services, DAOs, permissions, capabilities, and bundled kit JSON are gone.
- `AppState` no longer composes `fde_workspace`.
- Registered application commands: 417 → 383.
- Renderer invoke literals: 159 → 125.

### Storage

- New installs do not create `fde_*` or `verification_*` tables or `fde_resource_*` triggers.
- Historical schema-22 merge still accepts both predecessor shapes and keeps proxy settings.
- Schema 24 → 25 drops leftover generation triggers so shared Provider/Skill/MCP/prompt writes no longer bump retired counters.
- Leftover table rows stay in upgraded databases as unconsumed historical data. Tables are not dropped. Schema numbers are not reused.
- SQL dumps omit retired tables and triggers so an import cannot revive the module.
- Old SQL dumps that still contain those tables restore shared config; triggers are dropped; there is no DAO/service that consumes the leftover rows.
- Sync skip/preserve no longer treats retired tables as a live product restore set.
- User files under the app config `projects/` directory are not deleted.

### Docs / specs

Updated current manuals and contracts: `README.md`, `docs/user-manual/zh/README.md`, `CHANGELOG.md` Unreleased, frontend/backend spec indexes, navigation, database persistence, security-boundaries. Deleted the current `4.7-fde-projects` page and the four focused delivery-kit / verification specs.

Left historical: Git history, `docs/release-notes/v0.4.6-*`, demo manifests, archived Trellis tasks, and the planning files in this task directory.

## Data behavior

| Scenario | Behavior |
| --- | --- |
| Fresh database | No retired tables or triggers. `user_version = 25`. |
| Upgrade from 24 | Triggers dropped. Existing `fde_*` / `verification_*` rows remain unread and unwritten. |
| Upgrade from schema-22 FDE | Proxy/settings retained. Leftover project/evidence rows retained. Triggers dropped. |
| Upgrade from schema-22 subscription-only | OpenCode proxy completed. Retired tables are not created. |
| New SQL export | Retired tables/triggers omitted. |
| Old SQL import containing `fde_*` | Shared providers/settings restore. SQL `CREATE TRIGGER` stays authorizer-denied. Product does not load the module. Live leftover tables are archived before replace. |
| Binary restore of v22 | Shared config restored; exact known retired triggers are accepted then dropped before DML; leftover historical rows may remain on the restored snapshot; no generation write. Live leftover tables are archived before replace. |
| User disk | No cleanup of `~/.fyagent/projects` or other real work files. Leftover live retired tables are copied to `retired-customer-projects/` before a replace that would drop them. |

## Tests run and results

| Command | Result |
| --- | --- |
| `pnpm exec tsc --noEmit` | pass |
| `pnpm lint` | pass |
| `pnpm test:unit tests/renderer tests/architecture` | pass, 127 files / 1033 tests |
| `pnpm test:unit tests/remainingPlatformSurface.test.ts tests/taskDocs.test.ts` | pass, 52 tests |
| `pnpm build:renderer` | pass; 8 route chunks; 659477 initial JS bytes |
| Playwright `shell.spec.ts` + `navigation-selection.spec.ts` | pass, 29 tests (Chromium viewports + WebKit selection) |
| Playwright `navigation-performance.spec.ts` | pass, 4 tests including production boot of eight routes |
| `cargo fmt --all --check` | pass |
| `node scripts/tasks/rust.mjs check` | pass |
| `node scripts/tasks/rust.mjs clippy` (`-D warnings`, `--all-targets`) | pass |
| `cargo test --lib` | pass, 3505 passed, 5 ignored |
| `cargo test --tests` (lib + `src-tauri/tests`) | pass, 3767 passed, 6 ignored |
| Focused retired-storage / ACL / probe tests | pass |

Renderer regressions added or updated:

- `/projects` and `/projects?project=…` redirect to `/agents` with no 客户项目 nav.
- New install schema does not create retired tables (`fresh_install_does_not_create_retired_customer_project_storage`).
- Old dump import restores shared config and omits retired objects from new dumps.
- ACL asserts retired commands are unregistered.

## Not verified

- The `mise run rust:test` wrapper itself. Cargo was invoked directly: `cargo test --lib` (3505 passed) and `cargo test --tests` (3767 passed, including `src-tauri/tests`).
- Full `pnpm test:unit` including `tests/miseTaskContract.test.ts` `env:check --json`: this worktree has no `.venv` (`uv-managed Python and .venv` failed). Unrelated to the removal.
- Windows real machine, Tauri packaged app, and user `~/.fyagent` live data. No production install or data mutation was performed.
- GPT acceptance.

## Failures

None remaining in the gates listed above after fixing:

- capability JSON trailing comma
- hardcoded schema 24 assertion
- ACL command count 417 → 383
- structure-asset digests for edited sealed files
- Clippy dead code from verification-only model identity probe
- Playwright top-level nav count 6 → 5

## Residual identifiers (allowed)

| Residual | Why it remains |
| --- | --- |
| This task's `prd.md` / `design.md` / `research/module-scope.md` | Planning record, not product code |
| `.trellis/tasks/archive/**`, `.trellis/tasks/09-19-fde-*` | Historical tasks |
| `docs/release-notes/v0.4.6-*`, `CHANGELOG.md` 0.4.6 entries | Historical release record; Unreleased notes the retirement |
| `docs/fyagent/development/demos/0.4.6-fac051ea/manifest.json` | Historical demo snapshot |
| `src-tauri/src/database/backup.rs` dump-omit tests and archive-before-replace | Compatibility: prevent dumps from resurrecting the module; archive leftover rows before replace |
| `src-tauri/src/database/retired_customer_projects.rs` | Closed historical trigger SQL and retired table names; not a product DAO |
| `src-tauri/src/database/fde_restore_tests.rs` leftover table names | Upgrade/import/archive regression fixtures |
| `tests/renderer/platform/tauriAclContract.test.ts` `projects_*` / `list_delivery_kits` `toBe(false)` | Removal regression |
| `tests/renderer/app/router-shell.test.tsx` `/projects` redirect | Removal regression |
| `profiles` commands, tray `projects_label`, `~/.claude/projects`, Gemini `/projects/` URLs | Unrelated “project” meanings |
| `managed-auth` `verificationUri`, Codex desktop `verification` phase | Unrelated verification words |
| `ProviderService::extract_credentials` under `#[cfg(test)]` | Test-only credential parsing; no product UI |

No live product entry, command, port, or DAO remains.

## Suggestions (out of scope)

- Do not delete configuration Profiles (`list_profiles` / tray submenu). That is a different feature.
- Do not delete Managed Auth, Models, Skills, MCP, prompts, or memory.
- Optional later: a one-shot operator UI to open leftover archives. Not done here. Automatic archive-before-replace is implemented in rework 1.

## Rework 1 (final-acceptance return)

Status: **repaired, still not accepted**. No commit/push/PR/install. `independent-review.md` was not present in the task directory when this rework started.

### HEAD correction

The first report listed starting HEAD `21bd05d2`. Raw `git rev-parse HEAD` is `c0b2ec21caccd33c082884270aa2e5dbfb7e9d9e`. That earlier hash came from an RTK log that hid the merge commit. The branch never moved.

### Fixes

1. **Fail-closed binary trigger allowlist.** `is_retired_fde_trigger` no longer uses `contains("fde_resource_")`. Compatibility layer `retired_customer_projects.rs` owns the 20 frozen historical definitions from baseline `project_resource_trigger_sql()`. Unknown trigger SQL is rejected before DML. After copy to the candidate, triggers are disabled (`SQLITE_DBCONFIG_ENABLE_TRIGGER=false`) and dropped, then create/migrate runs. Restore/import finishes only if zero triggers remain. The original backup file is not written. SQL import authorizer still denies `CREATE TRIGGER`, including genuine retired definitions.

2. **Historical data retention.** Leftover `fde_*` / `verification_*` tables stay out of `SYNC_PRESERVE` and new dumps. Before SQL import, sync import, or binary restore replaces a live database that still has those tables, the host writes `retired-customer-projects/historical-fde-*.db`, reopens it, and checks table/row counts. Ordinary `backups/` retention does not delete that directory. Archive failure aborts the replace; live DB stays unchanged. After the first successful replace, leftover tables are gone so later syncs do not keep creating archives. New installs do not create retired tables or that directory. No old business DAO was restored.

3. **User-manual hub.** `docs/user-manual/README.md` now describes the current eight-feature guides instead of a six-chapter sessions/workspaces book. ZH/EN/JA relative links (`./zh|en|ja/README.md`, `../fyagent/development/README.md`) were checked. Session research is not documented as a shipped capability.

4. **Navigation spec.** `.trellis/spec/frontend/navigation.md` no longer lists `/projects` in the AI software group. The Bad case is eight pages, not nine.

5. **Persistence spec.** `.trellis/spec/backend/database-persistence.md` records the exact trigger allowlist, disable-before-DML, archive-before-replace, and authorizer `CREATE TRIGGER` denial.

### Tests

| Command | Result |
| --- | --- |
| `rtk mise run rust:fmt:check` | pass |
| `rtk mise run rust:check` | pass |
| `rtk mise run rust:clippy` | pass |
| `rtk mise run rust:test` | pass (exit 0; locked aarch64-apple-darwin suite) |
| Focused `database::backup` + `retired_customer_projects` lib tests | pass, including keyword-in-comment/literal reject, forged-prefix reject, genuine trigger restore without generation bump during v5 copilot rewrite, SQL authorizer still denying genuine `CREATE TRIGGER`, sync/SQL/binary archive-before-replace, archive-failure abort, fresh install with no archive directory |

### Failures

| Gate | Result |
| --- | --- |
| `rtk mise run check:contracts` | fail at `release:check`: `CHANGELOG.md must start its version history with ## [0.4.6] - YYYY-MM-DD` because first-pass `## [Unreleased]` is the first heading. Not rewritten in this rework (0.4.6 already shipped the add notes). Other contract subtasks (`tasks:validate`, `tasks:docs:check`, `supported-platform:check`, `python:lock:check`, `version:check`) reported ok before that failure. |
| Full `mise run check` | not rerun here. GPT is already running it; one unmodified `OpenCodeSubscriptionRestore.test.tsx` had timed out under load. This rework did not change shared business tests or timeouts. |

### Limits

- Executed ≠ accepted. Do not merge, push, or install.
- No old customer-project module, DAO, or archive UI was restored. Leftover rows are recoverable from `retired-customer-projects/*.db` by opening the SQLite file, not by product flows.
- Browser suite was not rerun in this rework; no renderer UI changed except docs/specs.
- Compatibility trigger text is closed. A semantically equivalent but differently formatted trigger is rejected.

## Next

GPT-only acceptance against this worktree. Do not merge, push, or install until accepted.

## GPT final acceptance — 2026-09-22

User explicitly requested GPT takeover after Grok rework 1 exited. GPT fixed the unreleased changelog heading, added UTF-8-safe trigger normalization with a regression, and finished manual wording. Final canonical `mise run check` exited 0: 2020 unit tests passed (1 skipped); Rust suites total 3862 passed (6 ignored); contract/release gates passed. Existing 651 browser regressions plus 3 production boot checks remain valid because no renderer source changed afterward. Both P1 findings are closed by final source review and regression results. Earlier missing-environment and failing-gate statements above are superseded by this run.

Accepted for local code delivery; no commit, push, PR, release or installation. Windows native and packaged Tauri runtime remain unverified. Full evidence: `/Users/<username>/fyagent/.trellis/tasks/09-21-remove-client-projects/acceptance.md`.
