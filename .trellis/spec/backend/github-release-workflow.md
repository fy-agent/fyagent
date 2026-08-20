# GitHub Release Workflow Contract

## 1. Scope and release authority

This contract owns `.github/workflows/release.yml`, the repository release
helpers under `scripts/release/`, their fixture suites, and the transaction
that may publish a FyAgent GitHub Release. Read it before changing release
events, eligibility, native runners, signer configuration, artifact ownership,
attestation, Release notes, or the final publication request.

Per-asset NSIS mechanics, install-path behavior, Windows signing evidence, and
manual native install/uninstall diagnostics are owned by
[Windows Installer](./windows-installer.md). Frozen Shell-user startup and
single-instance input containment are owned by
[Windows Runtime Security](./windows-runtime-security.md). This workflow owns
their orchestration, frozen inputs, aggregate evidence, and publication gate.

The workflow supports two entry modes:

```yaml
workflow_dispatch:
  inputs:
    source_sha:
      required: true
push:
  tags:
    - "v*.*.*"
```

The YAML tag filter is only routing. The repository-owned eligibility engine
accepts exactly stable `vX.Y.Z` with no prerelease, build metadata, missing
component, or leading zero.

- `workflow_dispatch` is an optional full three-target diagnostic preflight for
  the current trusted `dev/laiyongjie` HEAD. It may build and package native targets,
  prove and seal Windows bytes, create workflow artifacts, and attest candidate
  bytes, but it can never create or update a GitHub Release and is not a
  release-closure prerequisite.
- a tag `push` is the only formal publication path. The remote tag must be an
  annotated tag whose target commit equals the current remote `main` HEAD and
  the exact successful full push CI source.
- `main` is the release-authority branch. Runtime eligibility proves its live
  remote HEAD and exact-source CI directly; it does not infer those facts from
  branch protection, a ruleset, merge settings, or a separate provenance
  workflow, and this project does not claim that those administrator controls
  exist.
- no branch push, manual signed mode, manual tag dispatch, partial target mode,
  cross-architecture substitute, local publish path, or update-in-place path
  exists.

Creating/pushing the tag and invoking the remote preflight are parent-task
operations. Local tests and this specification do not authorize either action.

## 2. Frozen release identity

Eligibility is the sole producer of these values:

```text
app_version   = canonical Cargo stable version
release_tag   = "v" + app_version
source_sha    = current remote dev/laiyongjie HEAD (preflight) | main HEAD (formal)
workflow_sha  = source_sha
release_mode  = preflight | formal
ci_run_id     = exact successful push CI run for the mode's authority branch
ci_attempt    = exact successful attempt of that run
```

Every platform build, metadata writer, signer boundary, asset verifier,
attestation, and publication step consumes these values unchanged. Downstream
jobs must not strip a ref, reread a second version source, select a newer CI
attempt, or substitute a different source/workflow SHA.

Preflight binds the exact current `dev/laiyongjie` HEAD and its successful push
CI. Formal mode independently binds the exact current `main` HEAD, its
successful push CI, and the remote annotated tag. Preflight is artifact-producing
diagnostic evidence, not formal release authority or a closure gate.

The frozen output has exact keys:

```json
{
  "appVersion": "X.Y.Z",
  "releaseTag": "vX.Y.Z",
  "sourceSha": "<40 lowercase hex>",
  "workflowSha": "<same SHA>",
  "ciRunId": "<positive decimal>",
  "ciRunAttempt": "<positive decimal>",
  "mode": "preflight | formal"
}
```

Later remote checks compare every key to this frozen value. A newer successful
rerun is not silently substituted after the initial decision.

## 3. Repository-owned eligibility and remote evidence

`scripts/release/dev-release-eligibility.mjs` is pure logic over normalized
schema `fyagent-dev-release-eligibility-input/v1`. It performs no network or
Git operation. The repository-owned remote collector reads GitHub through the
workflow token, constructs that exact schema, calls the pure evaluator, and
may compare against a previously frozen output.

Eligibility fails closed unless all of these facts agree:

1. repository name/id are `fy-agent/fyagent` / `1313497021`;
2. the workflow is `Release` at `.github/workflows/release.yml` and its
   workflow SHA equals the candidate source;
3. the canonical version is stable `X.Y.Z` and the tag is exactly `vX.Y.Z`;
4. the live authority branch target (`dev/laiyongjie` for preflight, `main` for
   formal) equals candidate, event, workflow, and checkout source SHA;
5. preflight event/ref/workflow ref are the `dev/laiyongjie` branch and its explicit
   `source_sha` input equals the frozen source; remote tag evidence is absent;
6. formal event/ref/workflow ref are the same version tag, the remote ref
   points to a Git `tag` object rather than directly to a commit, the annotated
   tag name is exact, and its target is the frozen commit;
7. the CI workflow belongs to the same repository, is active, is named `CI`,
   and has path `.github/workflows/ci.yml`;
8. among exact-source `push` runs whose head repository is the same repository
   and whose head branch is the mode's authority branch, the latest run number/attempt is
   completed successfully; an older green run cannot mask a later failed,
   cancelled, timed-out, or running attempt;
9. the selected attempt contains exactly one completed/successful
   `CI / Required` job and one matching check-run from the `github-actions`
   app, bound to the same run, attempt, check suite, source SHA, API URL, and
   job details URL.

Unknown keys, malformed IDs/SHA/statuses, incomplete pagination, HTTP errors,
wrong repository/workflow/event/branch, a moved branch, a lightweight tag,
missing evidence, duplicate Required results, or evidence URL drift are
failures. Tokens and API responses are not written to Release notes or logs.

### Repository owner-transfer boundary

Before the 2026-08-10 owner transfer, the factual repository URL was
`https://github.com/NongHua123/fyagent`. GitHub now redirects that URL with
HTTP 301 to `https://github.com/fy-agent/fyagent`, and both locations resolve
to numeric repository ID `1313497021`. That continuity preserves historical
run and source evidence only. It is not an eligibility alias: current
collection and evaluation require the exact canonical name
`fy-agent/fyagent`, and a payload, workflow reference, metadata URL, or
head-repository name that still presents the former owner fails closed.

Initial eligibility freezes the decision before any build. Formal publication
then performs two independent live rechecks with the same collector and exact
frozen value:

- once when the publish job begins, before creating a draft;
- once after draft upload/re-download/digest verification and immediately
  before the one final publication PATCH.

A branch move, tag replacement, CI attempt change, identity drift, or API
failure at either point stops publication. The workflow never moves/deletes
the tag to repair a failed run.

The independent [Windows Installer](./windows-installer.md) contract
additionally requires every version component to fit `0..65535` before Tauri
packages an installer. That narrow representation gate does not create a
second application version.

## 4. Job and trust topology

```text
eligibility
  ├─ build-windows (x64, ARM64) ─┐
  └─ build-macos   (universal) ──┴─ pin-release-build-inputs
                                                 ├─ preflight proof ──────┐
                                                 └─ formal transform      │
                                                     └─ fresh formal seal ┤
                                                                          └─ verify-assets
                                                                               └─ attest
                                                                                    └─ publish (formal push only)
```

All build jobs receive only frozen values and check out `source_sha`
directly. Before any signer code executes, the secret-free pin job waits for
all native builds, validates the exact directory/file set, records every
file's version/source/size/SHA-256 in `trusted-build-inputs.json`, and uploads
one `trusted-build-inputs` artifact. Its original artifact ID/digest are job
outputs. All trusted consumers download that original ID. Deleting it causes a
failure; uploading a same-name replacement yields another ID and cannot
replace the pinned bytes. Because `download-artifact` extracts a single match
directly into the requested path, the one macOS artifact is downloaded by
exact name into its own explicit `installers-macos-universal` directory.

The preflight and formal Windows paths are mutually exclusive:

- `prove-windows-preflight` contains no provider configuration or secret
  expression. It explicitly requests unsigned mode, proves strict
  `NotSigned`, binds the raw bytes, and uniquely creates the preflight final
  installer and private signing fragment.
- `sign-windows-formal` is the only secret-bearing provider job. It consumes
  pinned raw bytes, validates the mode/configuration matrix, executes the
  provider-neutral transform, and uploads only an explicitly untrusted
  `formal-candidate-*` artifact. It cannot create trusted signing fragments or
  final installer artifacts and never executes the candidate.
- `seal-windows-formal` runs on a fresh matching native runner with no signer
  secret, credential, adapter, dependency install, or candidate build. It
  downloads the original pinned raw input plus the untrusted candidate,
  re-proves raw `NotSigned`, admits only byte-identical unsigned output or an
  Authenticode-only mutation, independently probes the public signature
  policy, and exclusively creates the formal final pair.
- `verify-assets` admits the preflight proof only when both formal jobs were
  skipped, or the formal fresh seal only when the preflight proof was skipped.
  It also requires every native build and the immutable input pin to succeed
  before aggregating the exact installer and evidence sets.
- `attest` declares an explicit non-cancellation status condition so the
  intentionally skipped half of the mutually exclusive Windows topology does
  not trigger GitHub Actions' implicit success-only dependency propagation.
  Its first step then requires both direct needs, `eligibility` and
  `verify-assets`, to report `success`; any other non-cancelled result fails the
  job visibly instead of silently skipping attestation.
- `publish` also declares an explicit non-cancellation status condition and is
  reachable only for a formal tag push after successful `eligibility` and
  `attest` direct needs. A successful dispatch therefore attests its candidate
  bytes but still skips publication.

The Release workflow deliberately has no job that launches a Windows setup
executable or performs install -> verify -> uninstall. Successful matching
native build/package jobs are the platform acceptance boundary. The manual
`verify-windows-nsis-lifecycle.ps1` harness may be used for diagnostics, but
the workflow does not invoke it and its result is not a preflight, attestation,
or publication gate.

The formal provider therefore has authority to transform its candidate or
cause a denial of service, but it cannot replace pinned build inputs or create
trusted release evidence. No Release job executes the final setup bytes, and
signer material remains isolated from the fresh sealing boundary.

## 5. Runner, architecture, and toolchain contract

Direct third-party Actions use reviewed full commit SHAs. Required jobs do not
use `*-latest`, restore candidate-controlled release caches, or execute mise.
Node is established before pnpm. pnpm and Rust Action caches are explicitly
disabled.

| Target          | Runner/build environment         | Exact installer output             |
| --------------- | -------------------------------- | ---------------------------------- |
| Windows x64     | `windows-2025`, native `X64`     | x64 NSIS setup EXE                 |
| Windows ARM64   | `windows-11-arm`, native `ARM64` | ARM64 NSIS setup EXE               |
| macOS universal | `macos-15`, both Apple targets   | DMG and ZIP from one universal app |

Each target verifies documented `runner.os`/`runner.arch`, the requested
runner label, source HEAD, Node 24.19.0, pnpm 10.12.3, and Rust 1.97.1. There is
no emulator, architecture impersonation,
opposite-architecture toolchain, or reduced-target fallback. ARM runner
unavailability blocks acceptance.

## 6. Platform build, package, and security gates

### Windows

- application and bundle commands use the formal release manifest and verify
  exact PE architecture, `requireAdministrator`, `uiAccess=false`, and one
  execution-level manifest entry;
- `verify-windows-nsis-contract.mjs` runs before packaging and on the produced
  setup. It binds the reviewed Tauri template, Windows-only NSIS target,
  standard NSIS install-directory handling, independent strict ProgramData
  runtime creation/admission, and bounded uninstall ownership;
- raw setup bytes leave build runners only after strict `NotSigned` proof and
  an empty PE security directory;
- the final x64 and ARM64 bytes must agree on signed/unsigned mode. Complete
  provider configuration requires `Valid`, expected publisher/certificate,
  Code Signing EKU, and timestamp policy. Missing signer mode may produce
  strict unsigned evidence. Partial, empty-active, malformed, failed,
  mismatched, or post-sign-mutated states fail and never downgrade;
- each matching native Windows runner compiles the release application,
  verifies its PE architecture and elevated manifest, and packages exactly one
  NSIS setup executable before proof/signing and fresh sealing;
- before raw upload, and again after preflight/formal sealing, the x64 and
  ARM64 setup PE resources must each contain exactly one canonical FyAgent icon
  group whose referenced frames match `src-tauri/icons/icon.ico` byte-for-byte;
  a configured path without matching final resources is not accepted. Each
  verifier invocation is a mandatory, unmasked command: shell constructs that
  ignore or overwrite its nonzero status are forbidden by workflow mutation
  tests;
- installer execution, registry/shortcut/runtime observation, uninstall, and
  user-data preservation remain available through the manual lifecycle
  diagnostic and are not Release acceptance requirements.

### macOS

- one universal app contains `arm64` and `x86_64`, the frozen version, and
  bundle identifier `com.fyagent.desktop`;
- the workflow explicitly re-seals the complete app with an identity-free
  ad-hoc signature, then requires strict deep signature verification. The
  result must report an ad-hoc signature and no real TeamIdentifier
  (`TeamIdentifier=not set`); a certificate authority, Developer ID identity,
  notarization, or stapled ticket is rejected. Ad-hoc integrity is not
  Developer ID or Apple trust;
- the DMG container itself must remain truly unsigned. ZIP and DMG package the
  same verified ad-hoc-sealed app and are re-opened to prove version and
  executable digest identity;
- DMG creation removes its explicit output before the first attempt and after
  every failed attempt. DMG verification preserves the completed input across
  attempts. Both operations retry the same arguments only when the captured
  diagnostic contains `Resource busy` or `Resource temporarily unavailable`,
  with at most five attempts and `2`, `4`, `8`, and `16` second delays. Any
  other diagnostic, an exhausted retry budget, or inability to remove a
  partial creation output returns the original `hdiutil` status immediately.
  The workflow does not pipe `hdiutil`, force-detach images, or kill disk-image
  helpers.

## 7. Assets, metadata, signing disclosure, and attestation

The installer allowlist contains exactly four versioned files:

```text
FyAgent-X.Y.Z-macOS.dmg
FyAgent-X.Y.Z-macOS.zip
FyAgent-X.Y.Z-Windows-x64-setup.exe
FyAgent-X.Y.Z-Windows-arm64-setup.exe
```

Any Windows format other than the two NSIS setup executables, plus v-prefixed
filenames, unversioned names, architecture aliases, missing files, extras,
directories, symlinks, empty files, or overwrites is forbidden.

`download-manifest.json` schema `fyagent-download-manifest/v3` binds each
installer's exact name, platform, architecture, format, size, SHA-256, URL,
version, tag, source SHA, and publication instant. Its download URL uses the
canonical `https://github.com/fy-agent/fyagent/releases/download/` prefix;
redirecting pre-transfer URLs are not emitted as current metadata.

Three `fyagent-platform-build/v2` records—`macos-universal.json`,
`windows-x64.json`, and `windows-arm64.json`—bind target/runner, toolchain,
repository/workflow/run, source, release mode, and the same Required CI
run/attempt in both modes. `build-metadata.json` schema
`fyagent-build-metadata/v2` reconstructs those records through exact key
allowlists and emits non-null `requiredCi`.

The two private Windows fragments are normalized into public
`signing-status.json`. Release notes generate their Windows table only from
that verified metadata. Unsigned assets must explicitly say both architectures
are not Authenticode signed and still list SHA-256, source SHA, and
attestation. Signed mode reports the verified public certificate policy; no
credential or adapter secret is included.

Attestation subjects are the four installers plus `download-manifest.json`,
`build-metadata.json`, and `signing-status.json` (seven subjects). The
Sigstore bundle is copied to `artifact-attestation.sigstore.json`; it is the
eighth Release attachment and does not attest itself.

## 8. Permissions and publication transaction

Workflow default permission is `contents: read`.

- remote eligibility/rechecks receive only `contents: read`, `actions: read`,
  and `checks: read`;
- attestation receives `contents: read`, `id-token: write`,
  `attestations: write`, and `artifact-metadata: write`;
- the formal publish job alone receives `contents: write` after every build,
  Windows proof/seal, exact-asset, metadata, and attestation dependency
  succeeds;
- provider secrets exist only in the formal transform job. They never reach
  builds, preflight, fresh sealing, aggregation, notes, or specs.

The publish job has an explicit formal tag-push condition; dispatch evaluates
to false. It performs this transaction:

1. re-evaluate live remote eligibility against the frozen identity;
2. require the exact eight attachments and dynamic English notes file
   `docs/release-notes/${RELEASE_TAG}-en.md`;
3. generate the signing disclosure from verified metadata;
4. list all Releases, including drafts, and refuse any existing release with
   the tag;
5. create one private draft with a run/source ownership marker;
6. upload all attachments, list them, re-download by identity, and prove exact
   name, asset ID, non-empty state, and SHA-256 equality;
7. re-read the draft identity/state/marker and re-evaluate live remote
   eligibility against the same frozen output immediately before publication;
8. issue one PATCH to `draft=false`, `prerelease=false`, `make_latest=true`;
9. re-read by Release ID, verify exact published identity/asset IDs, and
   independently confirm it is Latest.

The current-document contract permits versioned notes only for the version in
`src-tauri/Cargo.toml`, with the exact `en`, `zh`, or `ja` suffixes. Every
present note must be non-empty and linked from `docs/release-notes/README.md`.
This keeps the required English formal-release note reachable on the same
source SHA that owns the successful Required CI evidence, while rejecting
stale or unrelated version files. After publication, the versioned files may
be removed whole; published history remains in Git history and GitHub
Releases.

No failure handler deletes a draft, retries the final PATCH, updates an
existing Release, or moves/deletes the tag. Before PATCH, failures leave and
report the draft for a separate human decision. After PATCH is attempted, one
read-only observation reports draft/published/unknown; an ambiguous outcome is
never called private or successful.

## 9. Failure matrix

| Condition                                                                                                                               | Required result                                              |
| --------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------ |
| Candidate/version/tag/event/workflow/authority-branch HEAD differs                                                                      | Fail before native builds.                                   |
| Repository name is a former owner or redirect alias, even when numeric ID is unchanged                                                  | Fail before native builds; require exact `fy-agent/fyagent`. |
| Formal tag is lightweight, points elsewhere, or changes                                                                                 | Fail; never repair or move the tag.                          |
| Exact-source authority-branch CI is absent/running/failed/cancelled/timed out, stale, wrong identity, or lacks unique Required evidence | Fail; never accept an older green commit/attempt.            |
| Preflight reaches a publish path or provider secret                                                                                     | Static/remote gate fails.                                    |
| Native runner, architecture, toolchain, or source drifts                                                                                | Fail that target; no fallback.                               |
| Pinned build input ID/digest/manifest/file set drifts                                                                                   | Fail before provider or trusted consumption.                 |
| Signer configuration is partial/invalid or fresh signature proof fails                                                                  | Fail; do not downgrade to unsigned.                          |
| Windows proof/sealed binding or macOS identity fails                                                                                    | Stop aggregation and publication.                            |
| An intentional producer skip propagates past successful asset verification                                                              | Attestation still runs; abnormal direct needs fail visibly.  |
| Four/seven/eight file allowlist or digest differs                                                                                       | Stop verification, attestation, or publication.              |
| Live main/tag/CI identity changes during the transaction                                                                                | Stop before creating the draft or before final PATCH.        |
| A draft/published Release already exists                                                                                                | Refuse update, replacement, or deletion.                     |
| Upload/re-download/pre-PATCH verification fails                                                                                         | Leave draft untouched and report it.                         |
| Final PATCH is failed or ambiguous                                                                                                      | Observe once; do not retry/delete or claim completion.       |

## 10. Validation and evidence boundary

Local gates cover the pure classifier/eligibility/required evaluators, remote
collector fixtures, version/release metadata, workflow structure, exact asset
sets, signing adapter policy, Windows NSIS contract, task docs, type checking,
formatting, and action-pin audits. Hermetic tests must include wrong repository,
workflow, event, branch, SHA, tag type, version, stale success, newer failed or
timed-out attempt, moved branch, pagination, HTTP failure, frozen-output drift,
dispatch publication, both preflight/formal tail-job truth tables, mutation of
explicit status conditions or direct-need assertions, asset loss/extra, signer
policy, and transaction failure.

Local execution cannot establish another platform's PowerShell/NSIS/
Authenticode, native build/package output, macOS bundle, GitHub attestation, or
public Release evidence. The manual Windows install lifecycle is diagnostic
evidence outside this Release closure. Closure requires, in order:

1. the release change is merged to `main`, and that exact current remote HEAD
   completes `CI / Required` successfully;
2. an annotated stable tag is created directly at that SHA and its single
   formal build workflow succeeds;
3. a public, non-prerelease, Latest Release has exact assets, disclosure,
   digests, metadata, and attestation;
4. any later optional bookkeeping push is a new `main` HEAD and must satisfy
   its own CI requirements; it is not part of the release transaction.

A successful `dev/laiyongjie` dispatch preflight may be run to produce and
attest candidate installers, but formal closure neither requires nor infers
success from it.

`windows-11-arm` remains public preview and may block the run. Unsigned Windows
installers may trigger trust prompts; disclosure, SHA-256, and attestation make
the origin auditable but are not equivalent to Authenticode. The repository's
administrative branch-protection and provenance-workflow settings remain
outside runtime eligibility and are not represented as release guarantees.
