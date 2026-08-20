# Design: Windows and macOS only

## 1. Design objective

The implementation changes the repository from a five-target desktop release
contract with additional WSL behavior to one positive, closed platform model:

```text
supported desktop platforms = { windows, macos }
```

This invariant applies to runtime dispatch, renderer types, persisted settings,
tool actions, build targets, host checks, CI runners, release evidence,
documentation, and current Trellis specifications. Unknown values remain
possible only at external boundaries and must fail closed.

The change deliberately avoids a compatibility shim. Keeping a hidden branch,
generic `unix` branch, broad `not(windows)` branch, or open-ended bundle target
would continue to implement the support being retired even if the public copy
were removed.

## 2. Scope boundaries

### Current first-party surface

The support invariant covers all tracked source, configuration, scripts,
workflows, tests, current docs, current specs, issue templates, repository-owned
generated artifacts, and user-visible assets. It also covers implicit target
admission and direct dependencies, not just literal platform words.

### Historical and generated boundaries

- Existing `.trellis/tasks/archive/**` is immutable task history.
- The 84 affected versioned release-note files are removed from the current
  snapshot rather than selectively rewritten. Existing Git commits, tags,
  release pages, and remote assets remain unchanged historical evidence.
- `pnpm-lock.yaml` and `src-tauri/Cargo.lock` remain generator-owned. Residual
  third-party target metadata is accepted only when frozen/locked regeneration
  still produces it and no first-party direct dependency or manifest entry
  requires the retired platform.
- Encoded SVG data is opaque asset content. Filename/reference auditing remains
  mandatory, but a random substring inside encoded bytes is not support logic.

## 3. Cross-layer contraction

| Contract              | Producers                                            | Consumers                                           | Result                                                                            |
| --------------------- | ---------------------------------------------------- | --------------------------------------------------- | --------------------------------------------------------------------------------- |
| Supported platform    | Rust host/build logic, Node tasks, workflow matrices | renderer types, CI/release tests, docs              | Positive Windows/macOS allowlist; unknown fails closed                            |
| Tool environment      | Rust `ToolVersion` and lifecycle commands            | settings API, About UI, locales, mocks              | Host-only model; remove environment type, distribution, flag, and shell selection |
| Window controls       | persisted settings and Rust DTO                      | settings hook, App layout, titlebar controls        | Remove the retired-platform setting end to end; preserve Windows/macOS layouts    |
| V2 platform           | platform detection                                   | domain types, fixtures, acceptance/visual manifests | `windows`, `macos`, browser/unknown boundary only                                 |
| Build target evidence | platform metadata writers                            | build metadata, manifest, verifier, attest/publish  | Three target records and four installers                                          |
| Required CI evidence  | workflow jobs                                        | required-gate evaluator and release authority       | Remove one job, transfer formatting, retain stable public check                   |

Every wire-shape edit is atomic across native producer, TypeScript facade,
state/UI consumer, locale, mock, and test. No optional deprecated field is kept
merely to ease source compatibility inside this repository.

## 4. Native runtime and bundle model

### Explicit target predicates

- Delete `src-tauri/src/linux_fix.rs` and all module declarations/calls.
- Delete Linux-only WebKit/Wayland environment mutation, tray/deep-link/single-
  instance branches, auto-launch and terminal branches, and package detection.
- Remove the direct target dependency on `webkit2gtk` and any direct dependency
  used only by the retired platform. Regenerate `Cargo.lock`; do not manually
  edit unrelated transitive entries.
- Replace `cfg(unix)` and `not(windows)` only where the intended implementation
  is actually macOS. Shared Windows/macOS code should either be unconditional
  with supported-host guards or use an explicit two-platform predicate.
- Test-only compilation helpers may use `cfg(test)` independently, but a test
  predicate must not make a production adapter compile on an unsupported host.

### Bundle target closure

The Tauri configuration cannot retain `targets: "all"`. The canonical bundle
configuration and platform overrides will enumerate the supported packages, and
the surrounding Node/Tauri entry points will reject an unsupported host before
building. Windows keeps the NSIS contract; macOS keeps DMG/ZIP production.

### Security invariants retained

Removing WSL path/command handling must not weaken:

- ordinary Windows UNC/SMB parsing and containment;
- Windows user-scope configuration and sanitized child environments;
- path traversal and symlink/reparse-point protections;
- macOS paths and subprocess behavior;
- credential/log redaction.

Focused tests distinguish an ordinary `\\server\share` path from the removed
special namespace instead of deleting all UNC coverage.

## 5. WSL and renderer data-flow removal

The following chain is removed as one breaking internal contract:

```text
native environment detection
  -> ToolVersion environment/distribution fields
  -> invoke facade and lifecycle arguments
  -> AboutSection state, badge, shell/flag controls
  -> locale copy, mocks, fixtures, tests
```

Native commands operate on the host environment only. Configuration discovery
uses supported host paths only, and tool lifecycle requests contain no
environment preference parameter. Persisted legacy JSON fields are ignored by
deserialization/defaulting; they do not trigger a migration or recovery path.

The Linux-only custom window-controls chain is also deleted from native DTOs,
persisted settings, renderer hooks, setting rows, App layout, locales, and
tests. The retained behavior is explicit:

- Windows keeps its existing drag-region geometry.
- macOS keeps system decorations and the existing titlebar spacing.
- Full-screen panel behavior remains consistent on both supported platforms.

Terminal option filtering, platform detection, V2 domain unions, acceptance
fixtures, and visual manifests positively list Windows/macOS. Browser and
unknown values may exist only where the web/test boundary already requires a
fail-closed result.

## 6. Toolchain and generated project artifacts

`mise.toml` and the generated `mise.lock` contract to four host variants:
Windows x64/arm64 and macOS x64/arm64. Environment/system checks, host-native
artifact selection, toolchain diagnostics, task metadata, and generated task
documentation are updated together. The lock is regenerated with the existing
repository task and then verified rather than edited by search-and-replace.

The Codex session-start hook loses only its `/mnt/<drive>` conversion branch.
MSYS/Cygwin conversion is preserved when it is a distinct Windows shell
compatibility path.

## 7. CI topology

### Retained public contract

`CI / Required` remains the stable required-check name. Its permission model,
change classification, exact attempt/run evidence, fail-closed handling, and
release-consumed output remain unchanged in meaning.

### Job changes

- Delete `backend-linux`.
- Move `cargo fmt --all --check`, previously unique to that job, into
  `backend-macos`.
- Migrate platform-neutral jobs (`changes`, contracts, frontend, desktop
  acceptance, Required evaluation, and labeler) to `macos-15` unless their
  existing supported-platform behavior requires Windows.
- Retain the Windows and macOS native jobs and their platform-specific checks.
- Update exact required-job lists, `needs`, job-result maps, test fixtures,
  comments, issue-form choices, triggers, and permissions together.

Commands that relied on a GNU userland are replaced with portable Node helpers
or native commands already supported by repository scripts. Shell portability
is verified structurally in tests; a migrated workflow must not contain a
runner label while still depending on commands unavailable on that runner.

## 8. Release evidence and publication transaction

### Exact future artifact sets

Platform metadata:

1. `macos-universal.json`
2. `windows-x64.json`
3. `windows-arm64.json`

Installers:

1. `FyAgent-<version>-macOS.dmg`
2. `FyAgent-<version>-macOS.zip`
3. `FyAgent-<version>-Windows-x64-setup.exe`
4. `FyAgent-<version>-Windows-arm64-setup.exe`

Attestation subjects are those four installers plus
`download-manifest.json`, `build-metadata.json`, and `signing-status.json`
(seven total). Published attachments are those seven files plus
`artifact-attestation.sigstore.json` (eight total).

### Evidence shape versions

This is a breaking contraction of enumerations and exact object shapes. The
versioned schemas advance as follows:

- `fyagent-platform-build/v1` -> `fyagent-platform-build/v2`
- `fyagent-build-metadata/v1` -> `fyagent-build-metadata/v2`
- `fyagent-download-manifest/v2` -> `fyagent-download-manifest/v3`

The platform metadata shape drops container evidence entirely because every
remaining build runs directly on a supported hosted runner. The build metadata
and download manifest accept only the three targets/four installer rules. The
TypeScript declaration, writer, verifier, fixtures, docs, and workflow-produced
JSON advance atomically. Signing-status and attestation-bundle formats do not
change merely because their exact file sets shrink.

### Workflow transaction

- Delete the retired build job and its container/image assertions.
- Contract pinning inputs from ten installer/metadata directories to six.
- Migrate eligibility, pinning, verification, attestation, and publication
  control-plane jobs to `macos-15`.
- Replace `stat -c`, `sha256sum`, and other GNU-specific publication steps with
  Node-based exact-set and digest operations.
- Update all `needs`, conditions, subject lists, upload/download groups, counts,
  and exact attachment assertions together.
- Preserve branch/tag/workflow/repository identity checks, exact-SHA Required
  CI authority, signer/sealer separation, minimal permissions, attestations,
  and the single draft-to-final fail-closed transaction.

Formal publication continues to resolve `docs/release-notes/${tag}-en.md` at
release time. Deleting already-published historical notes does not authorize a
fallback or autogenerated body; a future tag without its new note fails closed.

## 9. Documentation and historical snapshot policy

The current snapshot must not present a retired support path, while externally
published history remains untouched:

- Delete all 84 inventoried affected versioned release-note files as complete
  documents.
- Convert `docs/release-notes/README.md` into a forward-looking release-note
  policy/index that tolerates an empty current set and requires a clean note for
  each future formal tag.
- Remove or repair links from manuals, contributor/release guides, audit docs,
  changelog, and indexes.
- Rewrite current README, FAQ, support matrices, install/update guides, issue
  forms, `.omo` plans, audit/history summaries, and user-visible copy to the
  exact remaining platform set.
- Update current `.trellis/spec/**`; do not touch archived task artifacts.
- Neutralize the one affected prior workspace-journal line. The new completion
  entry says that the platform support matrix was contracted to Windows/macOS,
  avoiding a new current-snapshot residual.

## 10. Residual-audit architecture

A repository-owned Node check, named for the supported-platform contract rather
than the retired platform, provides a repeatable regression gate. It constructs
retired tokens from segments so its own source contains no forbidden literal.
It checks:

1. tracked path names and unsupported distribution extensions/assets;
2. bounded textual platform, runner, package, target-triple, WSL namespace,
   package-manager, XDG/display-stack, and runtime-detection patterns;
3. structural Rust `cfg(unix)`/`not(windows)` and Node/TypeScript non-Windows
   fallbacks for explicit review;
4. exact approved exclusions.

By default, the durable check excludes only `.trellis/tasks/archive/**`; it does
not grant a permanent wildcard exception to active tasks. Before this task is
archived, its invocation supplies the one exact active task directory as an
explicit temporary exclusion because the planning evidence necessarily names
its subject. The argument must resolve beneath `.trellis/tasks/`, must name the
current active task exactly, and is omitted from the post-archive/normal CI
invocation. The check otherwise excludes only the two approved generated
lockfiles from text matching and encoded SVG payloads from content matching;
paths and first-party manifests are never excluded.

Raster content is not added as a new opaque exception. The current 146 tracked
rasters must first pass a one-time decode, exposed-metadata scan, and visual
review. A sorted path-and-SHA-256 manifest then seals that reviewed identity so
an added, removed, renamed, or byte-modified raster fails closed until the same
review is repeated. Container and metadata validation still runs, and the
identity seal does not replace decoded-pixel checks required by the brand-asset
specification.

Platform-sensitive first-party source follows the same review-identity model
without becoming opaque. A bidirectional manifest freezes every Cargo manifest
and build script plus the current selector-bearing executable/configuration
surface by canonical path, Git `100644` mode, regular non-symlink type, and
SHA-256. Those files continue through ordinary text and semantic scanning. The
seal is a second authority layer: equivalent language syntax, same-file code
relocation, or a newly introduced selector-bearing file must cause inventory
or identity drift until the actual source diff is reviewed and the manifest is
deliberately updated.

Before archival, both the durable check and the strong manual audit exclude the
exact active task directory. After archival they are rerun with only
`.trellis/tasks/archive/**` excluded. The post-archive result is the final
acceptance evidence. Any finding not covered by the explicit exceptions fails
the task; a new ad hoc allowlist is not added without evidence and user-visible
justification.

## 11. Compatibility, rollback, and evidence limits

### Compatibility

- This is an intentional support-breaking change for removed environments.
- Windows/macOS public behavior and release filenames remain stable.
- Old persisted renderer fields are ignored safely.
- Old evidence schemas fail exact validation instead of being silently accepted
  as the new contract.

### Rollback

Each implementation batch is committed separately. If a batch fails its
focused contract, correct it before advancing; do not mask the failure in the
next layer. Git history provides the rollback points, and no destructive reset
or history rewrite is required.

### Evidence limits

Local checks can prove static contracts, tests, builds, exact file sets, and
remaining-platform compilation available on the Windows development host.
They cannot prove hosted macOS/Windows runner execution, notarization/signing,
or native HIL without a pushed workflow/formal release. Because this task has
no push authorization, those items remain explicitly unexecuted rather than
being inferred from local tests.
