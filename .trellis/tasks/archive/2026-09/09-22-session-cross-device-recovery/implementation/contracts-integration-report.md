# Session migration contract integration

Status: complete. Reviewed source identities are frozen; focused checks and the final canonical prearchive contracts composite passed. All writer locks for this work package are released to root.

## Reproduced failures

The focused canonical run of `remainingPlatformSurface`, `dep0040Contract`, `verifyCommitMessages`, and `architecture/rootGovernance` completed with 43 passing and 6 failing tests. Three failures were the existing 5-second per-test watchdog (dependency import scan, actual Vite configuration discovery, and temporary Git topology fixtures); no assertion failed in those cases. Root owns their non-concurrent rerun. No timeout or gate threshold was raised.

The other three failures were real platform contract drift: newly introduced implicit host predicates, additional platform-sensitive candidates, and changed frozen source identities. A separate prearchive failure was `spawnSync python ENOENT`: the active-task proof selected a bare PATH interpreter even though the repository's managed environment existed.

## Changes owned by this work package

- `scripts/tasks/supported-platform-check.mjs` now resolves the direct Trellis session proof through the project's `.venv` interpreter. It selects the standard Windows or admitted POSIX development-host layout, rejects unknown task hosts, and never retries through PATH. Missing environment reports the canonical `mise run python:sync` recovery. Direct-session identity, stale/fallback rejection, task metadata validation, and private exclusion transport are unchanged. The checker still imports only Node built-ins.
- `tests/remainingPlatformSurface.test.ts` covers the managed interpreter argv, admitted development-host layouts, unsupported task hosts, missing environment without a fallback attempt, and continued direct-session checks.
- Exact unsupported migration fallbacks are frozen by their complete bodies. The package exporter records `unsupported` metadata outside the two product hosts; identity directory access, child access, atomic publication, locks, machine/user identity, and file identity return errors. The common launcher returns no candidates and an execution-block reason on other hosts. Its cleanup fallback only consumes the unused process-group ID and retains generic child kill/reap, without admitting execution. Negative fixtures weaken or remove each reviewed body and must fail. No broad host allowance was added.
- The pre-existing synthetic OpenCode research evidence remains at its original referenced path. Its temporary-directory/environment block and the import/export calls carrying that environment are pinned by existing exact source contracts. The result's scope text is admitted only at its exact evidence location. Negative fixtures move the environment assignment out of its Python scope or replace the isolated subprocess environment with the ambient environment. The research helper was read, not executed.
- The opt-in actual OpenCode writer fixture is pinned by its test-only module, explicit Mac/test/ignore attributes, required synthetic root and complete environment containment assertions. Moving it out of the module or weakening the containment assertion fails. These exact source contracts consume the same snapshot's text entries so both Rust and the Python/JSON evidence are checked; the Rust inputs are retained in full.
- After root explicitly extended this work package to `scripts/ci/classify-changes.mjs`, the nine reviewed `research/session-recovery-20260921` files received exact ownership in contracts, backend, and docs/spec. Each file independently schedules those domains. New files in that directory, nested paths, and other research projects remain unknown and fail; there is no research-wide exemption. The public classifier schema and Full CI treatment of CI authority are unchanged.

Rust changes were made by their separate owners: broad host predicates were replaced with explicit Mac/Windows branches and unsupported fallbacks. This work package does not modify business Rust or frontend components.

## Review before updating source identities

- `src-tauri/src/lib.rs`: migration command registration only; the existing native host dispatch remains unchanged.
- `identity.rs` and `identity_platform.rs`: only explicit Mac/Windows identity primitives, with OS handle/descriptor binding and unsupported refusal. The former platform-dependent test filename is now portable. Root's `test-hooks` feature exposes only the injected test identity path and disables the native machine/user probe for that test build; the production branch retains its host restrictions. Raw machine identifiers remain outside package contents.
- Native Codex: explicit Mac directory fsync/process-group handling and Windows interactive-user setup. OpenCode, Hermes, and Gemini shell fixtures are explicitly Mac-only. Gemini's embedded JavaScript directory fsync now selects Mac positively. Common launcher candidates select Mac/Windows positively and reject unsupported execution, while cleanup retains generic child termination.
- Package metadata names Mac, Windows, or unsupported, without claiming another product target.
- Checker/test additions preserve complete inventory matching, Git index mode verification, unsupported-host rejection, zero findings, and the existing snapshot watchdog.
- Synthetic result JSON is a source-evidence record with no real user history or credentials; it explicitly does not prove Mac-to-Windows transfer or interactive model continuation. It is added as a regular `100644` source candidate rather than excluded.

After root's canonical Rust formatting and real Git index preparation, `supported-platform-structure-assets.json` was updated for exactly nine new and three changed reviewed candidates, producing 129 candidates total. Existing unrelated identities remain byte-for-byte unchanged. Canonical locale ordering and executable-mode exceptions remain unchanged. `contracts-source-identity.json` records each reviewed path, `100644` mode, prior identity and final SHA-256. The CI classifier and its test introduce no additional platform-sensitive candidates.

## Validation

- Read-only runtime check with `TRELLIS_CONTEXT_ID=session-recovery-20260922`: managed Python resolved the exact `.trellis/tasks/09-22-session-cross-device-recovery` direct-session pointer.
- Canonical scoped formatter completed for the checker and its focused test.
- `mise run test:unit tests/remainingPlatformSurface.test.ts`: 29/29 passed, exit 0 (4.71 seconds). This includes the whole repository snapshot, exact native/source contracts, unsupported-host negative cases, source identity/mode checks, and the managed interpreter regression.
- `mise run supported-platform:check`: passed, exit 0; 3,001 current files inspected without an active-task exclusion.
- First final prearchive composite: managed Python, direct session proof, platform checks, dependency/deprecation scan, task/docs/lock/version checks and native-fetch all passed. Release contract tests reached 651 passed / 1 failed / 1 skipped; the only failure was the newly indexed research paths' missing CI ownership. This failure was fixed rather than waived. The prior dep0040 and commit-message timeout cases passed in that real composite.
- `mise run test:unit tests/classifyChanges.test.ts`: 64/64 passed, exit 0 (1.71 seconds), including twelve new individual-path/unknown-path regression cases.
- Final `TRELLIS_CONTEXT_ID=session-recovery-20260922 mise run check:contracts:prearchive --exclude-active-task .trellis/tasks/09-22-session-cross-device-recovery`: exit 0. All 35 release contract files passed, with 664 tests passed / 1 existing skipped; native-fetch passed 4/4. Platform checks again inspected 3,001 current files successfully. The direct-session exclusion remained restricted to this task. Log: projectless `work/contracts-prearchive-final.log`.
- Root owns the final whole-frontend run; its preceding run reported 2,121 passed / 1 failed / 1 skipped, with only the same now-fixed research ownership failure. No product-source edits were made by this work package, no test watchdog was raised, and no unrelated frozen digest was refreshed.
- Scoped canonical formatting and `git diff --check` passed. Root will refresh the Git index contents and perform final aggregate checks; no commit was created by this worker.

Windows real-machine acceptance remains explicitly deferred by the user. Contract checks are portable/source checks, not Windows execution evidence.
