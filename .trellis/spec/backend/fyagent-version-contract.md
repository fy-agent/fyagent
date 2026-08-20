# FyAgent Application Version and Installer Asset Contract

## 1. Scope / Trigger

Read this contract before changing the application version, formal release tag,
installer names, download manifest, build metadata, or any script that updates
Cargo version metadata.

FyAgent has one manually maintained application-version source. Release jobs
freeze that value together with a tag and source commit before any platform
build starts. Installer filenames use the unprefixed application version; they
never derive a second version from a Git ref or package-manager manifest.

This contract does not authorize creating or moving a Git tag, publishing a
Release, changing a toolchain, or rewriting historical release records.

## 2. Canonical Metadata

`src-tauri/Cargo.toml`:

```toml
[workspace]
members = [".", "user-helper"]
resolver = "2"

[workspace.package]
version = "X.Y.Z"

[package]
name = "fyagent"
version.workspace = true
```

`src-tauri/user-helper/Cargo.toml`:

```toml
[package]
name = "fyagent-user-helper"
version.workspace = true
```

- `src-tauri/Cargo.toml [workspace.package].version` is the only manually
  maintained application-version literal.
- The workspace contains exactly the root package and `user-helper`, in that
  order. Both package manifests inherit `workspace.package.version` through
  exactly one `version.workspace = true` assignment and contain no literal
  package version.
- `package.json` is private and does not declare an application version.
- `src-tauri/tauri.conf.json` omits `version`, so Tauri inherits Cargo
  metadata.
- `src-tauri/Cargo.lock` contains exactly two source-less local package blocks,
  named `fyagent` and `fyagent-user-helper`. Each appears once and each version
  equals the workspace version; no other source-less local package is accepted.

The accepted version grammar is stable SemVer `X.Y.Z`: no `v` prefix,
prerelease, build metadata, leading zero, or omitted component. Components use
Cargo's unsigned 64-bit SemVer range; installer- or packager-specific numeric
limits are not application-version authority.

Windows is a narrower release representation, not a second version authority.
Each numeric component must fit an unsigned 16-bit field (`0..65535`) before a
formal release asset name is accepted or Tauri invokes NSIS. The release and
NSIS contract gates compare decimal strings with `BigInt`; they never coerce a
component through JavaScript `Number`. A canonical Cargo version outside this
range remains valid application metadata but cannot produce a Windows-inclusive
FyAgent Release until the canonical version changes.

## 3. Version Command Interface

```text
mise run version:get
mise run version:check [-- --tag vX.Y.Z]
mise run version:set -- X.Y.Z [--apply]
mise run version:bump -- patch|minor|major [--apply]
```

- `get` prints only the canonical stable version.
- `check` validates the complete metadata/lock contract. With `--tag`, it
  accepts exactly `v` plus the canonical version.
- `set` and `bump` preview by default. `--apply` is the only project-level
  write authorization.
- A write changes only `src-tauri/Cargo.toml` and the version field in both
  local `fyagent` and `fyagent-user-helper` blocks of
  `src-tauri/Cargo.lock`. The helper manifest already inherits the workspace
  value and must not be rewritten. Dependencies, package.json, Tauri
  configuration, release workflow, docs, tags, and assets are also outside the
  write set.
- Each target uses a unique same-directory temporary file, complete write,
  `fsync`, close, and rename. If a later write or post-write contract check
  fails, every already replaced target is restored through the same atomic
  replacement path and temporary files are removed.
- The two files are not one power-loss-atomic filesystem transaction. A crash
  between renames can leave detectable version drift; a later structurally
  valid `--apply` may repair only the canonical and local-lock values.

## 4. Frozen Release and Asset Values

The release eligibility boundary is the sole producer of:

```text
app_version = canonical Cargo version
release_tag = "v" + app_version
source_sha  = lowercase full Git commit SHA
release_mode = preflight | formal
ci_run_id / ci_run_attempt = exact successful push CI attempt
                           for the mode's authority branch
```

Every platform, evidence, attestation, and publication step consumes those
outputs unchanged. A downstream step must not trim `GITHUB_REF_NAME`, reread a
different version field, substitute another source SHA, or select another CI
attempt. Preflight requires the source to equal the live remote
`dev/laiyongjie` HEAD and binds that branch's successful
`.github/workflows/ci.yml` push attempt; it cannot publish. Formal mode
requires the source to equal the live remote `main` HEAD, binds that branch's
successful push CI, and additionally requires an annotated `vX.Y.Z` tag whose
target is that exact commit. The eligibility engine and
[GitHub Release Workflow](./github-release-workflow.md) own the branch split;
do not treat `main` as the preflight authority.

The installer allowlist contains exactly four versioned files:

```text
FyAgent-X.Y.Z-macOS.dmg
FyAgent-X.Y.Z-macOS.zip
FyAgent-X.Y.Z-Windows-x64-setup.exe
FyAgent-X.Y.Z-Windows-arm64-setup.exe
```

Only the two versioned NSIS setup executables are accepted for Windows.
Non-allowlisted Windows package formats, portable archives, v-prefixed names,
architecture aliases, and unversioned installer names are rejected.

`download-manifest.json` schema `fyagent-download-manifest/v3` binds product,
version, tag, source SHA, publication time, and each installer's exact name,
platform, architecture, format, size, SHA-256, and URL. It rejects missing,
extra, nested, empty, symlinked, wrong-version, or malformed files.

Exactly three platform metadata records use schema
`fyagent-platform-build/v2`: `macos-universal.json`,
`windows-x64.json`, and `windows-arm64.json`. Each binds its requested and
observed native environment, actual toolchain, source SHA, and output inventory.

`build-metadata.json` uses schema `fyagent-build-metadata/v2` and independently
binds those three platform records, repository/workflow identity, source SHA,
release mode, and requested versus observed native environments.

`signing-status.json` binds both final Windows setup executables to the same
version/source SHA and to their post-sign SHA-256/size plus verified
Authenticode state. It is a release attachment and attestation subject; native
per-architecture signing fragments are private workflow inputs and are never
published.

The attestation subject set contains exactly seven files: the four installers,
download manifest, build metadata, and signing status. The Sigstore bundle is
the eighth and final Release attachment and does not attest itself.

## 5. Change and Failure Rules

| Condition                                                                                                | Required result                                                                                                     |
| -------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------- |
| Workspace member, resolver, inherited version, private package flag, or duplicate version field drifts   | `version:check` fails before release or version writes.                                                             |
| Version is not stable `X.Y.Z`                                                                            | `get`, `set`, `bump`, or `check` fails without writes.                                                              |
| A component exceeds `65535` while entering a Windows bundle or formal Release                            | The NSIS/release contract fails before packaging; the canonical Cargo value is not rewritten.                       |
| Either local `fyagent` / `fyagent-user-helper` lock block is missing, duplicated, sourced, or mismatched | `version:check` fails; `set` may repair only version drift in both local blocks after every other preflight passes. |
| Tag differs from `v` plus the canonical version                                                          | Eligibility/version checking fails before platform builds.                                                          |
| An asset contains a v-prefixed, wrong, or missing version                                                | Platform or aggregate validation rejects it.                                                                        |
| Installer, metadata, signing status, or attestation subject set is missing or has extras                 | Evidence generation/publication stops.                                                                              |
| A write fails after one canonical file was replaced                                                      | Restore all touched files, remove temporary files, and fail with rollback evidence.                                 |

## 6. Tests Required

- Node version tests cover get/check/set/bump, stable SemVer rejection,
  preview/apply, structural preflight, exact tag equality, local lock drift,
  duplicate/missing metadata, CRLF preservation, unique temporary files, and
  rollback after write or post-write failure. Workspace fixtures require
  exactly `members = [".", "user-helper"]`, version inheritance in both
  manifests, exactly two source-less local lock packages, and synchronized
  root-manifest plus two-lock-block updates while the helper manifest remains
  byte-identical.
- `tests/versionConsistency.test.ts` delegates to the canonical script rather
  than implementing another version parser.
- Download/release asset tests assert all four exact names, the two Windows NSIS
  setup executables and architecture mapping, URL shape, and
  missing/extra/non-allowlisted/symlink rejection.
- Release tests assert frozen output consumption and that the download,
  build, signing, attestation, and publication stages use the same version,
  source SHA, CI run, and attempt. They cover `dev/laiyongjie` preflight and
  `main` formal authority-branch movement, annotated versus lightweight tags,
  and exact frozen rechecks before publication.
- Windows release tests accept `65535`, reject `65536`, and use an integer path
  that also rejects values beyond JavaScript's safe-number range without
  truncation.

Safe local checks are run through the repository toolchain:

```bash
mise run version:check
mise run test:unit -- tests/versionConsistency.test.ts \
  tests/downloadManifest.test.ts tests/releaseAssets.test.ts
mise run release:check
```

The standalone `tests/version.test.mjs` suite additionally exercises the
atomic version utility. Native installer/signing evidence remains owned by
[Windows Installer](./windows-installer.md). Local static tests do not
establish an x64 or ARM64 package, Authenticode state, attestation, or public
Release. The matching native Release jobs stop at successful build/package and
Windows proof/sealing; the manual installation lifecycle is diagnostic rather
than a Release gate.

## 7. Wrong vs Correct

Wrong:

```text
package.json.version = "X.Y.Z"
tauri.conf.json.version = "X.Y.Z"
GITHUB_REF_NAME is stripped and reused as an installer version
FyAgent-vX.Y.Z-Windows-setup.exe
```

Correct:

```bash
mise run version:set -- X.Y.Z --apply
mise run version:check -- --tag vX.Y.Z
```

Then eligibility freezes the canonical version, tag, source SHA, and release
mode once; every platform and evidence step consumes those exact values.
