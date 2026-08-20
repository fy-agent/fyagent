# Remove all Linux and WSL support

## Goal

Make FyAgent's current repository contract support exactly Windows and macOS.
Remove every first-party product, runtime, renderer, toolchain, build, test, CI,
release, packaging, documentation, and current-spec surface for Linux and WSL,
then complete the Trellis lifecycle with reviewable local commits, an archived
task, and a clean working tree.

## Background and approved decisions

- This is a product-support contraction, not a wording-only cleanup. A path is
  out of scope for support even when it is admitted only implicitly by a
  `unix` or `not(windows)` predicate.
- WSL is part of the removal scope. FyAgent must no longer detect WSL, expose a
  distribution or environment badge, translate WSL paths, invoke `wsl.exe`, or
  configure/install/update/launch tools through WSL.
- Every current GitHub Actions job must run on a Windows or macOS runner.
  Platform-neutral control-plane jobs do not retain Ubuntu runners.
- The stable public `CI / Required` check and the release workflow's
  fail-closed authority, exact-SHA checks, signing, attestation, and atomic
  publication behavior must be preserved.
- The 84 affected versioned release-note files inventoried under
  `docs/release-notes/**` are deleted in full and their current links/indexes
  are repaired. Git history and already-published release pages remain the
  source of truth for those historical facts.
- Mixed current documents such as `CHANGELOG.md`, audit/history documents,
  `.omo` plans, and the single affected workspace-journal line are edited
  locally so the current tracked snapshot has no residual first-party support
  reference. New session journal text uses neutral Windows/macOS wording.
- Existing `.trellis/tasks/archive/**` is immutable historical evidence and is
  excluded from cleanup and residual scanning. This task receives the same
  exclusion after it is archived.
- Generated third-party transitive platform metadata in `pnpm-lock.yaml` and
  `src-tauri/Cargo.lock` is an explicit residual-audit exception. Both files
  remain generator-owned and must pass frozen/locked installation. The
  exception does not cover manifests, direct dependencies, `mise.lock`, code,
  configuration, tests, docs, or specs.
- Incidental substrings inside encoded SVG payloads are not platform support.
  SVG filenames and references remain audited, while opaque encoded payloads
  are excluded from textual residual matching.

## Requirements

### R1. Exact supported-platform invariant

- The only supported desktop platforms are Windows and macOS.
- Platform types, enums, host guards, target lists, fixtures, manifests, and
  user-facing support matrices use positive Windows/macOS allowlists.
- Unknown hosts fail closed. Generic unsupported-platform handling may remain,
  but it must not recognize, advertise, configure, build, test, package, or
  publish the retired platforms.
- Rust `cfg(unix)`, `not(windows)`, and equivalent TypeScript/Node fallbacks are
  reviewed and narrowed where they would otherwise admit an unsupported host.

### R2. Native runtime, packaging, and tool actions

- Delete the dedicated Linux compatibility module, Flatpak assets, Linux-only
  runtime environment workarounds, direct dependencies, target triples,
  packaging branches, auto-launch behavior, and installer/download logic.
- Remove WSL-specific detection, DTO fields, path/config resolution, command
  wrapping, lifecycle actions, environment preference flags, and fixtures.
- Preserve ordinary Windows UNC/SMB behavior, Windows user-scope and child
  process security, macOS POSIX behavior, and shared containment protections.
- Bundle configuration must enumerate or guard the supported targets rather
  than relying on an open-ended host target such as `all`.

### R3. Renderer and cross-layer contracts

- Remove WSL environment/distribution fields and invoke parameters from Rust
  producers, TypeScript facades, React state, UI badges/actions, mocks, tests,
  and all locale catalogs in one contract change.
- Remove Linux-only window-control settings and the complete persisted setting
  data path. Preserve the current Windows drag region and macOS system-titlebar
  behavior.
- Renderer and V2 platform models, terminal options, acceptance fixtures, and
  visual manifests positively enumerate Windows/macOS or fail closed.
- Legacy persisted values that refer to removed fields or unsupported platform
  values are ignored or normalized safely; they are not migrated into a new
  compatibility feature.

### R4. Toolchain and repository automation

- Contract `mise.toml`, generated `mise.lock`, environment checks, system
  checks, host-native artifact logic, generated task documentation, and their
  tests to Windows/macOS.
- Remove the Codex session hook's WSL mount conversion while retaining any
  separate MSYS/Cygwin path handling needed on Windows.
- Regenerate owner-generated artifacts through their canonical commands; do
  not hand-edit third-party lockfiles to manufacture a textual result.

### R5. CI contract

- Remove the Linux backend job and migrate every remaining workflow and
  labeler job off Ubuntu to an appropriate Windows or macOS runner.
- Move the unique Rust formatting responsibility to the retained macOS backend
  job.
- Update exact job sets, `needs`, output maps, required-gate evaluators,
  trigger/permission tests, and current CI specs without changing the public
  `CI / Required` check name or weakening fail-closed evidence handling.
- Replace runner-specific shell utilities with repository-owned Node scripts or
  commands that are portable on the selected runner.

### R6. Release contract

- Future releases contain exactly three platform metadata records, four
  installers, seven attestation subjects, and eight published attachments:
  Windows x64/arm64, macOS universal, and their shared evidence files.
- Remove the retired build job, artifact directories, container evidence,
  installer/metadata rules, target-index entries, download-manifest rows,
  attestation subjects, upload inputs, and publication assertions.
- Version the breaking evidence shapes honestly:
  `fyagent-platform-build/v2`, `fyagent-build-metadata/v2`, and
  `fyagent-download-manifest/v3`. Unchanged signing/evidence schemas remain at
  their existing versions.
- Preserve exact source/tag/branch/workflow eligibility, exact-SHA Required CI
  authority, signer/sealer separation, permissions, attestations, and the
  single fail-closed draft/final publication transaction.
- Formal publication continues to require the release tag's versioned English
  note. The repository need not retain a note for an already-published tag;
  the next release must add its own clean tag-specific note before publishing.

### R7. Documentation, history, and current Trellis contracts

- Delete the 84 inventoried affected versioned release-note files and repair
  all current indexes and links without rewriting Git history or remote
  releases.
- Rewrite current user, contributor, build, release, audit, support, and issue
  text to describe exactly Windows/macOS; delete obsolete packaging guidance
  and examples rather than leaving nonfunctional instructions.
- Update all affected `.trellis/spec/**` documents so the executable project
  contract matches the implementation.
- Do not modify existing archived Trellis tasks. Neutralize the one affected
  current workspace-journal line and keep the new completion entry free of
  retired-platform wording.

### R8. Verification, commits, and finish

- Add a durable first-party support-surface regression check using segmented
  pattern construction so the checker does not create its own residuals.
- Run focused checks after each logical batch and the complete local quality
  matrix before acceptance. Run the final strong residual audit again after
  archival, when the active task has moved under the approved archive
  exclusion.
- Organize implementation into multiple cohesive, reviewable commits. Large
  mechanical deletions may share a commit when they implement one contract,
  but unrelated layers must not be collapsed into one catch-all commit.
- Validate and archive the Trellis task, record the session, commit local
  administrative artifacts, do not push without separate authorization, and
  finish with `git status --short` empty.

## Out of scope

- Rewriting Git history, existing tags, or already-published release pages and
  assets.
- Editing any existing file under `.trellis/tasks/archive/**`.
- Removing generator-owned transitive target metadata from `pnpm-lock.yaml` or
  `src-tauri/Cargo.lock` when the supported dependency graph still emits it.
- Adding a replacement compatibility layer, a new dependency, or unrelated
  refactors.
- Publishing a release, pushing commits, or claiming hosted-runner/native HIL
  evidence that was not actually executed.

## Acceptance criteria

- [ ] The repository's current executable and documented supported set is
  exactly Windows and macOS; unknown hosts fail closed.
- [ ] No first-party feature detects, exposes, invokes, translates, configures,
  installs, updates, launches, builds, tests, packages, or documents Linux or
  WSL within the agreed current-snapshot scope.
- [ ] All implicit Rust and TypeScript/Node platform predicates are either
  narrowed to explicit supported platforms or documented by a focused test as
  genuinely platform-neutral without admitting an unsupported product path.
- [ ] The dedicated compatibility source file and the four Flatpak files are
  deleted, and no unsupported-platform filename or distribution asset remains.
- [ ] WSL DTO fields, invoke parameters, environment preferences, UI/state,
  configuration/path special cases, lifecycle actions, locales, mocks, and
  tests are removed end to end while ordinary UNC behavior remains covered.
- [ ] `.github/**` has no unsupported runner, job, package, command, fixture,
  issue option, or publishing surface. The retained jobs use Windows/macOS,
  and `CI / Required` retains its stable public name and exact fail-closed
  evidence contract.
- [ ] Release verification requires exactly 3 platform metadata records, 4
  installers, 7 attestation subjects, and 8 attachments using the new evidence
  schema versions, while eligibility/signing/attestation/publication authority
  remains intact.
- [ ] The 84 inventoried versioned release-note files are deleted; all current
  docs, indexes, links, issue forms, specs, audit/history text, plans, and
  journal text satisfy the current Windows/macOS-only contract.
- [ ] `mise.lock` and other first-party/generated-by-project artifacts contain
  no retired platform entries. `pnpm-lock.yaml` and `src-tauri/Cargo.lock` are
  reproducible and contain no avoidable direct/root dependency residue.
- [ ] The durable support-surface check and a final tracked-file/path audit have
  zero unexplained findings outside the approved archived-task, generated
  lockfile, and opaque SVG-payload exclusions.
- [ ] Focused tests and the complete local matrix in `implement.md` pass. Any
  unexecuted hosted-runner or native HIL validation is reported explicitly and
  is not represented as completed evidence.
- [ ] The task's implementation/check records validate, the task is archived,
  local work is split into reviewable commits, no remote push occurs, and
  `git status --short` is empty.
