# Implementation plan: Windows and macOS only

## 0. Approval and preparation gate

**Depends on:** final planning summary delivered; a subsequent explicit user
approval.

**Actions**

1. Run the Trellis task validator and read the final PRD, design, research,
   implementation context, and check context top to bottom.
2. Start the task through `.trellis/scripts/task.py start` only after the
   approval gate is satisfied.
3. Invoke `trellis-before-dev` and load the affected backend, frontend,
   cross-layer, CI, release, environment, brand, identity, and security specs.
4. Reconfirm `git status --short`, branch, task identity, and absence of
   unrelated user-owned edits.
5. Commit the approved Trellis planning artifacts as their own small
   administrative commit if the task-start workflow does not do so.

**Completion definition:** task status is active, all required specs are loaded,
the baseline is understood, and implementation begins from a clean, attributable
diff.

## 1. Native runtime and package closure

**Goal:** eliminate direct and implicit unsupported native/runtime/build paths
without disturbing Windows or macOS behavior.

**Actions**

- Delete `src-tauri/src/linux_fix.rs`, its module/call sites, runtime
  environment workarounds, auto-launch/terminal/deep-link/single-
  instance branches, direct target dependencies, and target-specific tests.
- Replace broad `cfg(unix)`, `not(windows)`, and equivalent host fallbacks with
  explicit macOS or supported-platform predicates after checking each use.
- Close the Tauri bundle target configuration and host build guard to Windows
  and macOS packages.
- Regenerate Cargo-owned metadata and verify that no direct/root dependency for
  the retired platform remains; do not edit transitive lock rows manually.
- Update native platform fixtures and the related brand/identity specs that
  must change with this batch.

**Focused validation**

```powershell
mise run rust:fmt:check
mise run rust:check
mise run rust:test
mise run assets:icons:check
pnpm exec vitest run tests/localBuildBoundary.test.ts tests/singleInstanceActivationContract.test.ts
```

**Commit:** `refactor(native): restrict desktop runtime to supported platforms`

**Completion definition:** native compilation and tests pass; bundle/runtime
dispatch is closed to Windows/macOS; the deleted package/assets have no live
reference.

## 2. Remove the WSL cross-layer feature

**Depends on:** phase 1.

**Goal:** remove environment discovery, path/config behavior, tool lifecycle,
wire fields, UI, copy, and tests as one typed contraction.

**Actions**

- Remove native detection, `wsl.exe` invocation, path conversion, distribution
  fields, config roots, command wrapping, environment preferences, lifecycle
  arguments, and fixtures from `misc`, Claude MCP, config, Codex config, and
  adjacent modules.
- Keep ordinary UNC/SMB and containment/security logic, adding or retaining a
  focused regression that distinguishes it from the removed special case.
- Contract `ToolVersion` and lifecycle invoke shapes in Rust and TypeScript;
  remove renderer state, About badges, flags, shell selection, translations,
  mocks, and assertions.
- Treat old persisted optional fields as ignored input through existing
  deserialization/default behavior; do not add a compatibility service.
- Synchronize frontend type-safety and Windows runtime-security specs.

**Focused validation**

```powershell
mise run rust:fmt:check
mise run rust:test
pnpm run typecheck
pnpm run test:i18n
pnpm exec vitest run tests/components/AboutSection.test.tsx tests/hooks/useSettings.test.tsx tests/codexWindowsUserScopeContract.test.ts tests/desktop-acceptance/desktopAcceptanceContract.test.ts
```

If the focused Vitest path selection differs from the repository's actual
layout, run the smallest discoverable test-file set and record the exact command
in the task context instead of weakening coverage.

**Commit:** `refactor(app): remove subsystem tool environments`

**Completion definition:** the native-to-renderer data flow contains no removed
fields or parameters, host-only tool actions pass, UNC/security coverage remains,
and all locale catalogs have exact key parity.

## 3. Close renderer and V2 platform behavior

**Depends on:** phase 2.

**Goal:** ensure UI layout, persisted settings, platform types, fixtures, and
acceptance evidence enumerate only supported behavior.

**Actions**

- Remove the retired-platform custom-window-controls setting from native DTO,
  persistence, hooks, setting UI, App decoration flow, locale catalogs, and
  tests.
- Preserve Windows drag-region and macOS system-titlebar/full-screen behavior.
- Remove `isLinux` and non-Windows fallbacks that implicitly classify unknown
  hosts as macOS; use explicit `isWindows`/`isMacOS` or fail-closed unknown.
- Contract terminal selections, V2 platform unions/detection, `/home` fixtures,
  desktop acceptance manifests, visual baseline manifests, and user copy.
- Keep generic unsupported Codex Desktop behavior fail closed.
- Synchronize component and frontend quality specs.

**Focused validation**

```powershell
mise run check:frontend
mise run lint:v2
mise run typecheck:v2
mise run test:v2
mise run test:v2:browser
mise run test:desktop:mock
mise run test:desktop:visual:preflight
mise run build:renderer
```

**Commit:** `refactor(renderer): close platform UI to supported hosts`

**Completion definition:** renderer/V2 types, persistence, layout, locales,
fixtures, and visual contracts agree on Windows/macOS and all frontend gates
pass.

## 4. Contract local toolchain and development automation

**Depends on:** phases 1-3.

**Goal:** remove host/toolchain admission from project-owned automation and
regenerate its authoritative artifacts.

**Actions**

- Update `mise.toml`, `.mise/tasks/**`, environment/system/toolchain scripts,
  host-native artifact selection, generated task docs, and contract tests to
  four Windows/macOS host variants.
- Run the canonical toolchain-lock generator so `mise.lock` contains only the
  supported project-owned platform entries.
- Remove the Codex hook's WSL mount conversion while retaining distinct Windows
  shell conversion paths.
- Update development-environment and related current specs.
- Verify frozen dependency resolution for both JavaScript and Rust lockfiles.

**Focused validation**

```powershell
mise run toolchain:lock -- --apply
mise run tasks:docs:generate -- --apply
pnpm install --frozen-lockfile
cargo metadata --locked --manifest-path src-tauri/Cargo.toml --no-deps *> $null
mise run env:check -- --json
mise run system:check -- --json
mise run tasks:validate
mise run tasks:docs:check
```

After the intended generated changes are inspected, record the hashes of
`mise.lock` and the generated task reference, rerun both `--apply` commands,
and require that the hashes remain unchanged. Generated changes are committed,
not discarded.

**Commit:** `build(toolchain): restrict development hosts to supported platforms`

**Completion definition:** canonical generated artifacts are up to date,
frozen/locked dependency checks pass, and project automation rejects unsupported
hosts without losing supported Windows/macOS variants.

## 5. Migrate and contract GitHub Actions CI

**Depends on:** phase 4.

**Goal:** retain exact required-check authority while every current job runs on
a supported runner.

**Actions**

- Delete `backend-linux`; transfer Rust formatting to `backend-macos`.
- Migrate remaining platform-neutral CI/control-plane and labeler jobs to
  `macos-15` (or retain Windows where the job is explicitly Windows-native).
- Replace incompatible runner commands with portable repository scripts.
- Update `needs`, exact required-job sets, change-classification results,
  evaluator mappings, fixtures, issue-form OS choices, tests, and CI specs.
- Preserve workflow names, triggers, permissions, concurrency, Required attempt
  evidence, and fail-closed output semantics.

**Focused validation**

```powershell
mise run check:contracts
pnpm exec vitest run tests/ciWorkflow.test.ts tests/requiredCiGate.test.ts tests/ciStepOutcomes.test.ts tests/ciToolchainContract.test.ts tests/localBuildBoundary.test.ts tests/githubWorkflowTriggers.test.ts
```

Use the repository's discoverable exact filenames if a listed test name has
moved; the phase must cover topology, permissions, runners, job exactness, and
portable commands.

**Commit:** `ci(actions): run checks only on supported runners`

**Completion definition:** workflow/labeler contracts have no unsupported job
or runner, `CI / Required` is stable, and exact topology/permission tests pass.

## 6. Contract release evidence and publication

**Depends on:** phases 1, 4, and 5.

**Goal:** publish exactly the remaining three targets/four installers while
preserving formal release authority.

**Actions**

- Remove the retired build job, container evidence, installer rules, metadata
  records, manifest rows, artifact directories, pinning inputs, workflow
  dependencies, subject lists, and attachment assertions.
- Advance platform/build/download evidence schema versions and remove the
  container shape from the platform metadata declaration, writer, validator,
  fixtures, tests, and docs.
- Contract exact sets to 3 metadata, 4 installers, 7 subjects, 8 attachments,
  and six trusted installer/metadata input directories.
- Migrate release control-plane jobs to `macos-15` and replace GNU-specific
  `stat`, digest, and traversal commands with portable Node operations.
- Preserve eligibility identity, exact-SHA Required CI authority, pin/seal
  boundaries, Windows signing records, attestation generation, permissions,
  and the one draft/final fail-closed publication transaction.
- Keep formal tag-specific release notes mandatory; remove tests that require
  a retained note for the already-published current version and replace them
  with a dynamic formal-tag note contract.
- Synchronize release/version current specs.

**Focused validation**

```powershell
mise run release:check
mise run check:contracts
pnpm exec vitest run tests/releaseWorkflow.test.ts tests/releaseAssets.test.ts tests/releaseCheckAggregation.test.ts tests/downloadManifest.test.ts tests/writePlatformMetadata.test.ts tests/versionConsistency.test.ts tests/currentDocsContract.test.ts
node --test tests/version.test.mjs
```

**Commit:** `build(release): publish only supported platform artifacts`

**Completion definition:** exact release sets and new schemas validate, local
preflight contracts pass, and no eligibility/signing/attestation/publication
authority is weakened.

## 7. Remove current documentation and history residue

**Depends on:** phases 1-6.

**Goal:** make the current tracked documentation/spec/history snapshot tell one
truthful Windows/macOS-only story.

**Actions**

- Delete the 84 inventoried affected versioned release-note files as whole
  documents; make the release-note index forward-looking and repair all links.
- Delete the four tracked Flatpak files and remove their current ignore,
  classifier, cleaning, identity, brand, and documentation references.
- Remove obsolete installation/package/support content from README,
  CONTRIBUTING, manuals, FAQ, changelog, release/support/audit docs, issue
  templates, user copy, and tests.
- Clean mixed `.omo` plans and the single affected workspace-journal line; do
  not edit any existing archived Trellis task.
- Finish synchronization of every affected current `.trellis/spec/**` file and
  index.
- Run broken-link, locale, generated-doc, and current-doc contract audits.

**Focused validation**

```powershell
mise run tasks:validate
mise run tasks:docs:check
mise run check:contracts
pnpm run test:i18n
pnpm exec vitest run tests/currentDocsContract.test.ts
git diff --check
```

**Commit:** `docs: align the repository with supported platforms`

**Completion definition:** current docs/specs/history surfaces have no
unexplained residual, all deleted-note references are repaired, and archived
task history is untouched.

## 8. Add the durable supported-platform audit

**Depends on:** phases 1-7.

**Goal:** turn the approved path/content/implicit-predicate audit into a
portable, fail-closed repository regression gate.

**Actions**

- Add the segmented-token supported-platform checker and its unit/contract
  tests without introducing a forbidden literal into the check source.
- Scan tracked filenames, bounded semantic content, Rust platform predicates,
  and TypeScript/Node non-Windows fallbacks using the closed exclusions in
  `design.md`.
- Default to excluding only archived task history, the two approved generated
  lockfiles, and opaque SVG content. Validate that the optional pre-archive
  argument accepts only this exact active task path and cannot become a wildcard
  bypass.
- Wire the default checker into the canonical contract/check task that will run
  after this task is archived.
- Decode all current tracked raster assets, scan exposed metadata, perform a
  contact-sheet visual review, and record the reviewed set in a sorted
  path-and-SHA-256 identity manifest. Make inventory or digest drift fail before
  treating image payload bytes as reviewed; retain container/metadata checks and
  their bounded decompression limits.
- Freeze the reviewed platform-sensitive source inventory bidirectionally by
  canonical path, Git `100644` mode, regular non-symlink type, and SHA-256.
  Keep normal text and semantic scanners active; the inventory is a review
  seal, not a content exclusion. Cover equivalent Cargo/Rust/JS syntax and
  same-file relocation with production-authority mutation tests.

**Focused validation**

```powershell
mise run check:contracts:prearchive -- --exclude-active-task .trellis/tasks/08-12-remove-linux-support
mise run tasks:validate
git diff --check
```

**Commit:** `test(platform): prevent unsupported support surfaces`

**Completion definition:** the exact pre-archive invocation has zero findings,
the default check contains no active-task wildcard, its tests cover every
approved exclusion and rejected bypass, and the prearchive contract gate passes.
The canonical no-argument gate remains strict while this task is active and is
rerun after archival.

## 9. Integrated review and local quality gate

**Depends on:** phases 1-8.

**Actions**

1. Run `trellis-check` with code-quality, cross-layer, tests/release, docs, and
   security perspectives. Resolve every in-scope finding and rerun its focused
   gate.
2. Run `trellis-update-spec` to verify the current specs capture the durable
   Windows/macOS-only contracts; do not add transient implementation notes.
3. Execute the full matrix from a Visual Studio Developer PowerShell where the
   native toolchain is available:

```powershell
pnpm install --frozen-lockfile
mise run env:check -- --json
mise run system:check -- --json
mise run check:prearchive -- --exclude-active-task .trellis/tasks/08-12-remove-linux-support
mise run lint:v2
mise run typecheck:v2
mise run test:v2
mise run test:v2:browser
mise run assets:icons:check
mise run build:renderer
mise run build
git diff --check
```

4. Run the durable support-surface check with the exact active-task exclusion
   shown in phase 7, plus a stronger tracked content/path audit using that same
   exact task path, archived tasks, the two approved generated lockfiles, and
   encoded SVG payloads as specified in `design.md`. Confirm the default
   checker has no active-task wildcard exclusion.
5. Review `git diff --stat`, `git diff --name-status`, each commit, and
   `git status --short` for scope, unintended binary changes, deleted-reference
   integrity, and user-owned edits.

**Completion definition:** all local commands pass, review findings are closed,
the only audit findings are explicit approved exceptions, and every acceptance
criterion has matching evidence. Hosted-runner/native HIL remains reported as
unexecuted unless separately authorized and actually run.

## 10. Trellis finish, archive, and clean-tree proof

**Depends on:** phase 9.

**Actions**

1. Run `trellis-check` using the curated `check.jsonl` spec/research context,
   close its findings, and validate both context manifests. Record actual test
   evidence in the session/final report rather than adding result rows to the
   manifests.
2. Use `trellis-finish-work` to verify quality status, archive the task through
   the canonical task script, and record a neutral Windows/macOS completion
   entry in the workspace journal.
3. Allow the canonical archive operation to make its intended local commit;
   commit any remaining journal/administrative change in a separate small local
   commit if the workflow requires it.
4. Rerun the strong tracked content/path audit with only
   `.trellis/tasks/archive/**`, the two approved generated lockfiles, and opaque
   SVG payloads excluded. Run `node
scripts/tasks/supported-platform-check.mjs` without an active-task argument
   and confirm no active task directory remains.
5. Run `git status --short` and require empty output. Do not push.

**Completion definition:** the task is archived and valid, post-archive residual
audit passes, local commits are reviewable, the working tree is clean, no remote
state changed, and the final report distinguishes executed local evidence from
unexecuted hosted-runner/native HIL evidence.

## Commit sequence and rollback points

The intended sequence is:

1. `chore(trellis): plan supported platform contraction`
2. `refactor(native): restrict desktop runtime to supported platforms`
3. `refactor(app): remove subsystem tool environments`
4. `refactor(renderer): close platform UI to supported hosts`
5. `build(toolchain): restrict development hosts to supported platforms`
6. `ci(actions): run checks only on supported runners`
7. `build(release): publish only supported platform artifacts`
8. `docs: align the repository with supported platforms`
9. `test(platform): prevent unsupported support surfaces`
10. Any narrowly scoped review-fix commit, only if a prior commit cannot be
    amended safely without obscuring already-recorded evidence.
11. Canonical Trellis archive commit and, if necessary, a separate journal
    commit.

Each numbered implementation commit is a rollback point. Failure in a phase is
fixed before the next phase; commits are not pushed, squashed, reset, or
rewritten destructively during this task.
