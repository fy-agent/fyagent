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

The workflow supports diagnostic preflight plus two equivalent formal entry
events:

```yaml
workflow_dispatch:
  inputs:
    mode:
      required: true
      type: choice
      options: [preflight, formal]
    source_sha:
      required: false
push:
  tags:
    - "v*.*.*"
```

The YAML tag filter is only routing. The repository-owned eligibility engine
accepts exactly stable `vX.Y.Z` with no prerelease, build metadata, missing
component, or leading zero.

- `workflow_dispatch(mode=preflight)` is an optional three-target diagnostic.
  The workflow itself must be dispatched from the trusted `main` workflow ref,
  while `source_sha` names the immutable candidate commit to build and may
  differ from the workflow/event SHA. Preflight may build/package native
  targets, prove and seal Windows bytes, create workflow artifacts, and attest
  candidate bytes, but it can never create/update a GitHub Release and is not a
  release closure prerequisite. Preflight candidate code receives no Windows
  signing or Apple Developer ID/notarization secrets.
- formal publication may begin either from the normal stable tag `push`, or
  from `workflow_dispatch(mode=formal)` executed **at that existing stable tag
  ref**. Formal dispatch requires `source_sha` to be empty because the selected
  tag is authority. In both formal events the workflow ref, workflow SHA,
  event SHA, checked-out source, canonical Cargo version, Release tag, and live
  remote tag target must collapse to the same release identity. This makes an
  exact tag/SHA retry possible without moving or pushing the tag again.
- The normal web Actions Run workflow branch picker is not formal-release
  authority. An operator retrying formal mode must dispatch the workflow at
  the tag ref (for example with `gh workflow run release.yml --ref vX.Y.Z -f
mode=formal` or the equivalent Actions API request); dispatching the current
  branch and merely passing a tag string is forbidden.
- The remote tag may be annotated or lightweight. Formal `sourceSha` is that
  tag's target commit.
  Live `main` HEAD may move during the run. Exact-source push CI is not a
  publication gate; the Release compile is the proof.
- `main` is the trusted workflow branch for preflight and the observed formal
  mainline branch.
  Runtime eligibility does not infer publication from branch protection, a
  ruleset, merge settings, or a separate provenance workflow, and this project
  does not claim that those administrator controls exist.
- no branch push, manual signed mode, partial target mode, cross-architecture
  substitute, local publish path, or published update-in-place path exists.

Concurrency is identity-based rather than event-based. Both a tag push and a
formal dispatch for `vX.Y.Z` join `release-formal-vX.Y.Z` with
`cancel-in-progress: false`; preflight uses `release-preflight-<source SHA>`.
This prevents two supported formal entry events from mutating the same Release
transaction concurrently.

Creating/pushing the tag and invoking the remote preflight are parent-task
operations. Local tests and this specification do not authorize either action.

## 2. Frozen release identity

Eligibility is the sole producer of these values:

```text
app_version   = canonical Cargo stable version
release_tag   = "v" + app_version
source_sha    = explicit candidate commit (preflight) | tag target commit (formal)
workflow_sha  = trusted main workflow/event commit (preflight) | source_sha (formal)
release_mode  = preflight | formal
ci_run_id     = null
ci_run_attempt = null
```

Every platform build, metadata writer, signer boundary, asset verifier,
attestation, and publication step consumes these values unchanged. Downstream
jobs must not strip a ref, reread a second version source, or substitute a
different source/workflow SHA.

Preflight binds an explicit candidate SHA while the workflow/event identity is
independently bound to the trusted `main` workflow ref. Formal mode binds the
remote tag's target commit regardless of whether the event is tag `push` or a
manual dispatch at the same tag ref. Preflight is artifact-producing
diagnostic evidence, not formal release authority or a closure gate.

The frozen output has exact keys:

```json
{
  "appVersion": "X.Y.Z",
  "releaseTag": "vX.Y.Z",
  "sourceSha": "<40 lowercase hex>",
  "workflowSha": "<trusted workflow SHA; equals sourceSha only in formal mode>",
  "ciRunId": null,
  "ciRunAttempt": null,
  "mode": "preflight | formal"
}
```

Later remote checks compare every key to this frozen value. A newer tag object
that no longer points at the frozen commit fails the live recheck. A moved
`main` HEAD does not.

## 3. Repository-owned eligibility and remote evidence

`scripts/release/dev-release-eligibility.mjs` is pure logic over normalized
schema `fyagent-dev-release-eligibility-input/v2`. It performs no network or
Git operation. The repository-owned remote collector reads GitHub through the
workflow token, constructs that exact schema, calls the pure evaluator, and
may compare against a previously frozen output.

Eligibility fails closed unless all of these facts agree:

1. repository name/id are `fy-agent/fyagent` / `1313497021`;
2. the workflow is `Release` at `.github/workflows/release.yml`;
3. the canonical version is stable `X.Y.Z` and the tag is exactly `vX.Y.Z`;
4. preflight is `workflow_dispatch(mode=preflight)` from `refs/heads/main`;
   event SHA equals workflow SHA, the explicit `source_sha` input equals the
   frozen candidate source, candidate source may differ from workflow SHA, and
   remote tag evidence is absent;
5. formal event SHA and workflow SHA both equal the frozen source;
6. formal is either a tag `push` or `workflow_dispatch(mode=formal)` with empty
   `source_sha`; event/ref/workflow ref are the same version tag, and the remote ref
   is either a Git `tag` object (annotated: exact name, target type `commit`,
   target SHA equals the frozen commit) or a Git `commit` object (lightweight:
   SHA equals the frozen commit and `tagObject` is null);
7. live `main` HEAD is observed for remote repository evidence but is not
   required to equal either the preflight candidate or a formal frozen source.

Unknown keys, malformed IDs/SHA/statuses, HTTP errors, wrong
repository/workflow/event/ref, a tag whose target is not the frozen commit,
missing tag evidence, or evidence URL drift are failures. A later `main` move
does not invalidate an already frozen preflight workflow SHA. Tokens and API
responses are not written to Release notes or logs. Missing, empty, or failed
exact-source push CI does not fail eligibility.

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

A tag replacement away from the frozen commit, identity drift, or API failure
at either formal recheck stops publication. A later `main` commit does not.
Operators may force-update
`vX.Y.Z` when no GitHub Release exists for that tag; the workflow itself never
moves or deletes the tag. An existing published Release for the tag is an
immutable boundary. An existing draft is recoverable only under the ownership
protocol in section 8; source/provenance mismatch fails closed rather than
being overwritten.

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
                                                                                    └─ publish (formal tag push or tag-ref dispatch)
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
  reachable only for formal mode after successful `eligibility` and `attest`
  direct needs. A preflight dispatch still skips publication; a formal dispatch
  at the exact tag ref follows the same formal signing/sealing/publication path
  as a tag push.

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
use `*-latest` or execute mise. Node is established after pnpm on native
build jobs. `setup-node` may restore the lockfile-keyed pnpm store. Rust Action
caches stay `cache: false`. Native CI backend jobs and Release build jobs may
restore `~/.cargo/registry` and `~/.cargo/git` through `actions/cache`, keyed
on `src-tauri/Cargo.lock` plus runner OS/arch. They never cache
`src-tauri/target`. Repository Cargo config must not set `RUSTC_WRAPPER` or
sccache.

| Target          | Runner/build environment         | Exact installer output              |
| --------------- | -------------------------------- | ----------------------------------- |
| Windows x64     | `windows-2025`, native `X64`     | x64 NSIS setup EXE                  |
| Windows ARM64   | `windows-11-arm`, native `ARM64` | ARM64 NSIS setup EXE                |
| macOS universal | `macos-15`, both Apple targets   | one UDZO DMG from the universal app |

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
- preflight packages a styled macOS DMG from the candidate universal app
  without injecting Apple Developer ID or notarization secrets. It verifies
  version, bundle identifier, both architectures, DMG layout, and byte
  preservation, but does not claim Developer ID signature or notarization;
- formal-mode Apple Developer ID secrets exist only in the guarded signing /
  notarization steps inside `build-macos`. Those steps import a
  temporary keychain, re-seals the complete app with
  `Developer ID Application: William Wang (HY446996QX)` / team `HY446996QX`,
  the hardened runtime, a secure timestamp, and the checked-in entitlements,
  then verifies that identity without requiring a stapler ticket yet. The job
  packages a signed DMG from that app and submits only the DMG to Apple
  notarization. The helper submits without `--wait`, then polls
  `notarytool info` until `Accepted` / `Invalid` or a multi-hour budget;
  `notarytool wait --timeout` is not used because it exits 124 with JSON on
  stderr while Apple may still be In Progress. After Apple accepts that one
  submission, it staples the DMG and the original app from the same ticket.
  It does not emit a ZIP or notarize an app zip as a second serial
  wait. The `build-macos` job sets `timeout-minutes: 360` so the poll can use
  the GitHub-hosted maximum. Strict deep verification must
  report the exact identity, `runtime` flags, a timestamp, and sealed
  resources. The published DMG container must carry a stapled ticket. The
  workflow also staples the original app for ticket proof. The
  app copy inside the already-built DMG is the pre-staple Developer ID binary
  and is checked for signature identity, not a nested ticket. An ad-hoc
  signature, missing team, missing timestamp, or missing required ticket is
  rejected;
- the DMG source folder contains `FyAgent.app`, a symlink named
  `Applications` whose target is `/Applications`, and
  `.background/background.png` copied from
  `src-tauri/icons/dmg-background.png`. `build-macos` calls
  `scripts/release/create-macos-dmg.sh` as the only styled-DMG entry. That
  script creates a UDRW HFS+ image, attaches it, writes Finder layout with
  `scripts/release/write-dmg-layout.py` (`ds_store` + `mac_alias` via the
  uv group `dmg-layout`), converts to UDZO, and verifies. AppleScript,
  Finder, `osascript`, `dmgbuild` CLI, `appdmg`, and `--skip-jenkins` are
  forbidden. Layout constants: window `660x400` pt, icon size `128` pt,
  `FyAgent.app` at `(180, 188)`, `Applications` at `(480, 188)`, volume
  name `FyAgent`. Alias records must be created against the mounted volume
  file, not the staging directory. After `hdiutil attach` of the final DMG,
  the workflow requires the app, the Applications symlink (`-L` and
  `readlink` equals `/Applications`), `.background/background.png`,
  `.DS_Store`, and the signed app. The volume stays UDZO/read-only; Finder
  drag-install is left-to-right. Layout details are in
  [macOS Styled DMG Layout](./macos-dmg-layout.md). No
  `FyAgent-X.Y.Z-macOS.zip` is produced;
- DMG `create` and `convert` remove their explicit destination before the
  first attempt and after every failed attempt. They must not delete the
  UDRW convert source. DMG verification preserves the completed input
  across attempts. `create`, `convert`, and `verify` retry the same
  arguments only when the captured diagnostic contains `Resource busy` or
  `Resource temporarily unavailable`, with at most five attempts and `2`,
  `4`, `8`, and `16` second delays. Any other diagnostic, an exhausted
  retry budget, or inability to remove a partial destination returns the
  original `hdiutil` status immediately. The workflow does not pipe
  `hdiutil`, force-detach images, or kill disk-image helpers.
- `build-macos` installs the same pinned `astral-sh/setup-uv` and managed
  Python 3.14.7 as CI, then `uv sync --locked --group dmg-layout`. Default
  `uv sync --locked` on Linux/Windows must not install that group.

## 7. Assets, metadata, signing disclosure, and attestation

The installer allowlist contains exactly three versioned files:

```text
FyAgent-X.Y.Z-macOS.dmg
FyAgent-X.Y.Z-Windows-x64-setup.exe
FyAgent-X.Y.Z-Windows-arm64-setup.exe
```

Any Windows format other than the two NSIS setup executables, any macOS
format other than the versioned DMG, plus v-prefixed
filenames, unversioned names, architecture aliases, missing files, extras,
directories, symlinks, empty files, or overwrites is forbidden.

`download-manifest.json` schema `fyagent-download-manifest/v3` binds each
installer's exact name, platform, architecture, format, size, SHA-256, URL,
version, tag, source SHA, and publication instant. Its download URL uses the
canonical `https://github.com/fy-agent/fyagent/releases/download/` prefix;
redirecting pre-transfer URLs are not emitted as current metadata.

Three `fyagent-platform-build/v2` records—`macos-universal.json`,
`windows-x64.json`, and `windows-arm64.json`—bind target/runner, toolchain,
repository/workflow/run, source, and release mode. `build-metadata.json`
schema `fyagent-build-metadata/v2` reconstructs those records through exact key
allowlists and emits `requiredCi` as null when frozen eligibility has no CI
run.

The two private Windows fragments are normalized into public
`signing-status.json`. Release notes generate their Windows table only from
that verified metadata. Unsigned assets must explicitly say both architectures
are not Authenticode signed and still list SHA-256, source SHA, and
attestation. Signed mode reports the verified public certificate policy; no
credential or adapter secret is included.

Attestation subjects are the three installers plus `download-manifest.json`,
`build-metadata.json`, and `signing-status.json` (six subjects). The
Sigstore bundle is copied to `artifact-attestation.sigstore.json`; it is the
seventh Release attachment and does not attest itself.

## 8. Permissions and publication transaction

Workflow default permission is `contents: read`.

- remote eligibility/rechecks receive only `contents: read`, `actions: read`,
  and `checks: read`;
- attestation receives `contents: read`, `id-token: write`,
  `attestations: write`, and `artifact-metadata: write`;
- the formal publish job alone receives `contents: write` after every build,
  Windows proof/seal, exact-asset, metadata, and attestation dependency
  succeeds;
- provider secrets exist only in the formal Windows transform job. Apple
  Developer ID secrets exist only in formal-mode `build-macos` steps. Neither
  secret set reaches preflight candidate execution, fresh sealing,
  aggregation, notes, or specs.

The publish job has an explicit formal-mode condition. A stable tag push and a
`workflow_dispatch(mode=formal)` run whose workflow ref is that same tag are
equivalent publication entry events; preflight dispatch evaluates to false.
It performs this transaction:

1. re-evaluate live remote eligibility against the frozen identity;
2. require the exact seven attachments and dynamic English notes file
   `docs/release-notes/${RELEASE_TAG}-en.md`;
3. generate the signing disclosure from verified metadata;
4. list all Releases, including drafts. More than one Release for the tag is a
   failure. A published Release is immutable and fails immediately. A lone
   draft may proceed only through the owned-draft recovery protocol below;
5. after any owned stale draft has been safely recovered, create one fresh
   private draft with a run/attempt/source ownership marker;
6. upload all attachments, list them, re-download by identity, and prove exact
   name, asset ID, non-empty state, and SHA-256 equality;
7. re-read the draft identity/state/marker and re-evaluate live remote
   eligibility against the same frozen output immediately before publication;
8. issue one PATCH to `draft=false`, `prerelease=false`, `make_latest=true`;
9. re-read by Release ID, verify exact published identity/asset IDs, and
   independently confirm it is Latest.

### Owned draft recovery protocol

The marker is intentionally only the first discriminator, not sufficient
proof of ownership:

```text
<!-- fyagent-release-transaction:run=<run>;attempt=<attempt>;source=<sha> -->
```

`scripts/release/verify-release-draft-ownership.mjs` requires that marker to
occur exactly once as the final body suffix and to bind the current frozen
source. The draft itself must also remain private, non-prerelease, named
`FyAgent vX.Y.Z`, have `target_commitish` equal the frozen source SHA, and have
no `published_at` value. The publish job then reads the exact originating
Actions run attempt and its exact attempt job set and fails closed unless all
of these agree:

- repository and head repository are exactly `fy-agent/fyagent` with numeric
  ID `1313497021`;
- the originating workflow is `Release` at `.github/workflows/release.yml`;
- the run/attempt numbers equal the marker and the run `head_sha` equals the
  current frozen source;
- the original run is completed with `failure`, `cancelled`, or `timed_out`;
- the exact attempt contains one `Publish stable GitHub Release` job bound to
  the same run/source/workflow, and its exact transaction step
  `Stage, re-download, and publish one stable Release transaction` ended in a
  recoverable failure/cancellation;
- the draft `created_at` instant falls inside that exact transaction step's
  `started_at`..`completed_at` interval;
- the attempt job response is complete within the bounded one-page set.

Immediately before deletion, the workflow re-reads the draft by Release ID and
re-runs the same ownership/provenance proof. Only then may it issue one DELETE
for that **draft Release ID**. It requires HTTP 204 and then a by-ID GET to
return 404 before creating the replacement draft. Transport ambiguity,
non-204 deletion, a still-resolving ID, source drift, a foreign workflow/run,
successful originating publication work, or any other provenance mismatch
fails closed. It never deletes individual assets to approximate an update and
never deletes or moves the Git tag.

This is deliberately delete-and-recreate rather than asset-by-asset resume:
the next attempt receives one clean transaction and cannot expose a mixture of
old and newly generated assets. Published Releases are never eligible for this
recovery path.

The current-document contract permits versioned notes only for the version in
`src-tauri/Cargo.toml`, with the exact `en`, `zh`, or `ja` suffixes. Every
present note must be non-empty and linked from `docs/release-notes/README.md`.
This keeps the required English formal-release note reachable on the same
source SHA as the tagged formal build, while rejecting stale or unrelated
version files. After publication, the versioned files may
be removed whole; published history remains in Git history and GitHub
Releases.

`CHANGELOG.md` is a separate release-check gate, not part of `version:set`.
When Cargo `workspace.package.version` is `X.Y.Z`, the first version heading
in `CHANGELOG.md` must match `^## \[X.Y.Z\] - 20\d{2}-\d{2}-\d{2}$`. The
text until the next `## [` heading must contain non-empty notes after
stripping HTML comments. Keep a Changelog preamble above that heading is
allowed. `scripts/release/verify-changelog-release.mjs` is invoked by
`mise run release:check`. The write set of `version:set` / `version:bump`
remains `src-tauri/Cargo.toml` plus the two local Cargo.lock package
blocks.

The failure handler for the **current** attempt never deletes its just-created
draft, retries the final PATCH, updates a published Release, or moves/deletes
the tag. Before PATCH, failures leave and report the draft; a later formal run
may recover it only through the protocol above. After PATCH is attempted, one
read-only observation reports draft/published/unknown; an ambiguous outcome is
never called private or successful, and automatic recovery must not proceed
until the Release is again provably an owned private draft.

## 9. Failure matrix

| Condition                                                                                                               | Required result                                                                                      |
| ----------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| Candidate/version/tag/event/workflow SHA differs                                                                        | Fail before native builds.                                                                           |
| Repository name is a former owner or redirect alias, even when numeric ID is unchanged                                  | Fail before native builds; require exact `fy-agent/fyagent`.                                         |
| Formal tag points elsewhere or changes after freeze                                                                     | Fail; never repair or move the tag from the workflow.                                                |
| Exact-source authority-branch CI is absent                                                                              | Continue; Release compile is the proof.                                                              |
| Preflight reaches a publish path or Windows provider secret                                                             | Static/remote gate fails.                                                                            |
| Native runner, architecture, toolchain, or source drifts                                                                | Fail that target; no fallback.                                                                       |
| Pinned build input ID/digest/manifest/file set drifts                                                                   | Fail before provider or trusted consumption.                                                         |
| Signer configuration is partial/invalid, Apple notarization is denied, or fresh signature proof fails                   | Fail; do not downgrade to unsigned.                                                                  |
| `notarytool wait --timeout` exits 124 or writes JSON only to stderr                                                     | Must not fail the job; poll `notarytool info` on the same submission id.                             |
| Apple status remains `In Progress` / `UNKNOWN` inside `FYAGENT_NOTARY_WAIT_SECONDS`                                     | Continue polling; log at least every `FYAGENT_NOTARY_HEARTBEAT_SECONDS`.                             |
| Apple status is `Invalid` or `Rejected`                                                                                 | Fetch `notarytool log` and fail; do not staple.                                                      |
| Apple status is still non-terminal after `FYAGENT_NOTARY_WAIT_SECONDS` (default 18000)                                  | Fail with the submission id and last status; do not start a second Apple upload.                     |
| Windows proof/sealed binding or macOS identity fails                                                                    | Stop aggregation and publication.                                                                    |
| An intentional producer skip propagates past successful asset verification                                              | Attestation still runs; abnormal direct needs fail visibly.                                          |
| Three/six/seven file allowlist or digest differs                                                                        | Stop verification, attestation, or publication.                                                      |
| `CHANGELOG.md` first version heading is missing, empty, or not Cargo `X.Y.Z`                                            | `mise run release:check` fails before tag/publication.                                               |
| Styled DMG layout write fails, or final attach lacks `.DS_Store` / background / Applications symlink                    | Fail `build-macos`; do not publish an unstyled DMG.                                                  |
| Live main identity changes during the transaction                                                                       | Continue; tag target SHA remains the frozen source.                                                  |
| Same-tag published Release already exists                                                                               | Fail; published version is immutable.                                                                |
| Same-tag draft has wrong source, marker, repository, workflow, run attempt, publish job, or transaction-step provenance | Fail closed; do not delete or replace it.                                                            |
| Same-tag draft is proven to be an owned failed transaction for the exact frozen source                                  | Re-read/re-prove; delete only that draft ID; require 204 then by-ID 404; create a fresh transaction. |
| Owned-draft DELETE is ambiguous, non-204, or the Release ID still resolves                                              | Fail; do not create a replacement.                                                                   |
| Upload/re-download/pre-PATCH verification fails                                                                         | Leave draft untouched and report it.                                                                 |
| Final PATCH is failed or ambiguous                                                                                      | Observe once; do not retry/delete or claim completion.                                               |

## 10. Validation and evidence boundary

Local gates cover the pure classifier/eligibility/required evaluators, remote
collector fixtures, version/release metadata, workflow structure, exact asset
sets, signing adapter policy, Windows NSIS contract, task docs, type checking,
formatting, and action-pin audits. Hermetic tests must include wrong repository,
workflow, event, branch, SHA, tag type, version, lightweight and annotated
tags, a moved formal `main` HEAD, a moved preflight branch, HTTP failure,
frozen-output drift, preflight non-publication, formal tag-ref dispatch
publication, formal push/dispatch concurrency identity, both preflight/formal
tail-job truth tables, owned-draft source/marker/run/workflow/job/step proof,
published/foreign draft rejection, bounded deletion confirmation, mutation of
explicit status conditions or direct-need assertions, asset loss/extra, signer
policy, transaction failure, a single `notarytool submit`, `notarytool info`
polling, no `xcrun notarytool wait` invocation, `notarytool log` on a denied
submission, `FYAGENT_NOTARY_WAIT_SECONDS`, `build-macos`
`timeout-minutes: 360`, the Applications symlink inside the DMG, `.background/background.png`,
root `.DS_Store`, `create-macos-dmg.sh`, `write-dmg-layout.py`, `dmg-layout`
uv group, changelog heading contract, and the
absence of a macOS ZIP installer.

Local execution cannot establish another platform's PowerShell/NSIS/
Authenticode, native build/package output, macOS bundle, GitHub attestation, or
public Release evidence. The manual Windows install lifecycle is diagnostic
evidence outside this Release closure. Closure requires, in order:

1. a stable `vX.Y.Z` tag, annotated or lightweight, whose target is the
   intended source SHA (Cargo version on that commit equals the tag);
2. one serialized formal build workflow for that tag succeeds—either the
   original tag push or a later formal dispatch at that exact tag ref—including
   matching native compiles, signing/notarization, exact assets, disclosure,
   digests, metadata, and attestation;
3. a public, non-prerelease, Latest Release has those exact artifacts;
4. any later optional bookkeeping push is a new `main` HEAD and must satisfy
   its own CI requirements; it is not part of the release transaction.

If no published GitHub Release exists for `vX.Y.Z`, operators may force-update
the tag and start a new formal run. A draft tied to the old source then fails
closed instead of being silently removed. For the common same-tag/same-SHA
failure, do not move the tag: dispatch `mode=formal` at the existing tag ref;
an owned failed draft for that same source can be recovered automatically.

A successful `mode=preflight` dispatch from the trusted `main` workflow may be
run for any explicitly frozen repository candidate SHA to produce and attest
diagnostic installers, but formal closure neither requires nor infers success
from it.

`windows-11-arm` remains public preview and may block the run. Unsigned Windows
installers may trigger trust prompts; disclosure, SHA-256, and attestation make
the origin auditable but are not equivalent to Authenticode. The repository's
administrative branch-protection and provenance-workflow settings remain
outside runtime eligibility and are not represented as release guarantees.

## 11. Wrong vs Correct

Wrong: freeze live `main` HEAD and a prior exact-source `CI / Required` as
formal identity, refuse lightweight tags, cache `src-tauri/target`, or treat a
`notarytool wait` timeout as Apple rejection. Also wrong: require a tag move to
rerun the same formal tag/SHA, dispatch formal mode from a branch workflow ref,
or treat an arbitrary same-tag draft as workflow-owned.

```text
sourceSha = live main HEAD
require successful push CI on that SHA
reject tag objects that are commits (lightweight)
cache path: src-tauri/target
notarize app zip, then notarize DMG
xcrun notarytool wait "$id" --timeout 1800
# exit 124 / empty stdout => fail the Release job
bump X.Y.Z after every failed unpublished formal run
force-push the same tag just to rerun the same SHA
workflow_dispatch mode=formal from refs/heads/feature/test
delete any draft merely because tag_name matches
```

Correct: the remote tag's target commit is formal `sourceSha`. Annotated and
lightweight tags are both valid. Missing exact-source push CI is not a gate.
The same tag/SHA can be rerun with `workflow_dispatch(mode=formal)` at the tag
ref without a tag mutation. Operators may force-update `vX.Y.Z` while no
published Release exists, but a stale draft from another source fails closed.
Only a same-source failed draft with independently verified Actions
run/attempt/job provenance can be deleted and recreated. A published Release
is immutable.
Cache `~/.cargo/registry` and `~/.cargo/git` from `Cargo.lock`. Submit the
signed DMG once without `--wait`, poll `notarytool info` on that submission
id until `Accepted` / `Invalid` or the wait budget, then staple the DMG and
the original app from that ticket. Do not emit a ZIP.

## Scenario: Single DMG notarization poll

### 1. Scope / Trigger

- Trigger: Apple Developer ID notarization is an infra integration. First
  submissions for this team stayed `In Progress` past 30 and 60 minutes and
  later reached `Accepted`. `notarytool wait --timeout` writes JSON to stderr
  and exits 124, which `set -e` treats as failure even while Apple is still
  processing.
- Owner: `scripts/release/macos-developer-id.sh` plus the `build-macos` job in
  `.github/workflows/release.yml`.

### 2. Signatures

- `macos-developer-id.sh notarize-dmg <dmg>`
- `xcrun notarytool submit <dmg> --output-format json` (no `--wait`)
- `xcrun notarytool info <submission-id> --output-format json`
- `xcrun notarytool log <submission-id> <log-json>` on `Invalid` / `Rejected`
- `xcrun stapler staple <dmg>` then `macos-developer-id.sh staple-app <app>`

### 3. Contracts

- Request: one regular signed UDZO DMG; one Apple submission id.
- Response status values: `Accepted` (success), `In Progress` / `UNKNOWN`
  (keep polling), `Invalid` / `Rejected` (fail).
- Environment: required `FYAGENT_APPLE_CERTIFICATE_P12_BASE64`,
  `FYAGENT_APPLE_CERTIFICATE_PASSWORD`, `FYAGENT_APPLE_ID`,
  `FYAGENT_APPLE_APP_SPECIFIC_PASSWORD`. Optional
  `FYAGENT_NOTARY_WAIT_SECONDS` (default `18000`),
  `FYAGENT_NOTARY_POLL_SECONDS` (default `20`),
  `FYAGENT_NOTARY_HEARTBEAT_SECONDS` (default `120`).
- Job: `build-macos` `timeout-minutes: 360`.

### 4. Validation & Error Matrix

- Missing secrets or unsigned/non-regular DMG -> fail before submit.
- Submit JSON without a string `id` -> fail; do not poll.
- `notarytool wait` timeout / exit 124 -> must not be the wait path.
- `Invalid` / `Rejected` -> print `notarytool log`, fail, do not staple.
- Non-terminal after wait budget -> fail with id and last status; do not
  upload a second Apple job.
- `Accepted` -> staple DMG and the original app; do not emit a ZIP.

### 5. Good / Base / Bad Cases

- Good: `info` returns `Accepted` inside the budget; the DMG has a stapled
  ticket and the original app is stapled from the same ticket.
- Base: `info` stays `In Progress` for more than 60 minutes, then `Accepted`;
  the same submission id is reused.
- Bad: two serial Apple uploads; `wait --timeout 1800` twice; treating 124 as
  rejection; bumping the Cargo version solely because an unpublished tag's
  formal run timed out.

### 6. Tests Required

- `tests/releaseWorkflow.test.ts` asserts exactly one `notarytool submit`,
  presence of `notarytool info` and `notarytool log`, no `xcrun notarytool wait`
  invocation, `FYAGENT_NOTARY_WAIT_SECONDS`,
  `scripts/release/macos-developer-id.sh notarize-dmg`,
  `staple-app`, `timeout-minutes: 360` on `build-macos`, the DMG Applications
  symlink, styled layout scripts, and the absence of `macOS.zip`.
- Local tests do not call Apple; a successful unit run is not notarization
  evidence.

### 7. Wrong vs Correct

#### Wrong

```bash
xcrun notarytool wait "$id" --timeout 1800 --output-format json >"$out"
# timeout JSON on stderr, empty $out, exit 124, set -e kills the job
```

#### Correct

```bash
xcrun notarytool submit "$dmg" --output-format json   # capture id
xcrun notarytool info "$id" --output-format json      # poll until Accepted
```

## Scenario: Formal tag-target identity

### 1. Scope / Trigger

- Trigger: Formal publication identity is an infra contract. Historical
  squash-merged commits already present on `main` and unpublished notarization
  timeouts must not require a Cargo bump. Current mainline merge policy is
  owned by [GitHub Merge Governance](./github-merge-governance.md). The workflow
  must not freeze live `main` HEAD or a prior `CI / Required` run as
  `sourceSha`.
- Owner: `scripts/release/dev-release-eligibility.mjs` plus the `eligibility`
  job in `.github/workflows/release.yml`.

### 2. Signatures

- `workflow_dispatch.inputs.mode = preflight | formal`
- `workflow_dispatch.inputs.source_sha = <sha> | ""`
- `evaluateDevReleaseEligibility({ event.dispatchMode, remoteTag, ... })`
- `verifyRecoverableReleaseDraft(release, runAttempt, jobs, tag, sourceSha)`
- Frozen output keys: `appVersion`, `releaseTag`, `sourceSha`, `workflowSha`,
  `mode`, `ciRunId`, `ciRunAttempt`

### 3. Contracts

- Request: stable `vX.Y.Z` tag whose target commit is the intended source.
  Annotated and lightweight tags are both valid.
- Normal formal entry is a tag push. Manual formal retry is
  `workflow_dispatch(mode=formal)` at the **same tag ref** with empty
  `source_sha`; this is not a branch dispatch carrying an arbitrary tag input.
- Response: `sourceSha = tag target commit`, `workflowSha = sourceSha`,
  `ciRunId = null`, `ciRunAttempt = null`.
- Push and formal dispatch for the same tag share one
  `release-formal-vX.Y.Z` concurrency group with cancellation disabled.
- Operators may force-update unpublished `vX.Y.Z` onto a later SHA. The
  workflow never moves or deletes the tag. Published Releases are immutable.
- An exact-source draft may be deleted/recreated only after marker + exact
  originating Actions run attempt + publish job/transaction-step provenance
  all prove it is residue from a failed FyAgent Release transaction.

### 4. Validation & Error Matrix

- Tag target != frozen `sourceSha` during the run -> fail; do not retarget.
- Lightweight tag pointing at another commit -> fail.
- Formal dispatch from a branch ref, with non-empty `source_sha`, or whose
  workflow SHA differs from the tag source -> fail before native builds.
- Missing exact-source push CI -> continue; Release compile is the proof.
- Published GitHub Release already exists for the tag -> fail immutable.
- Same-source owned failed draft -> re-prove immediately before deleting that
  draft ID, require DELETE 204 and GET-by-ID 404, then start a fresh draft.
- Foreign/source-mismatched/success-origin/ambiguous draft -> fail closed.
- Failed unpublished formal run -> operators may force-update the same tag;
  do not bump `X.Y.Z` solely for that timeout. For the same SHA, prefer manual
  formal retry without moving the tag.

### 5. Good / Base / Bad Cases

- Good: annotated `v0.4.3` at the intended SHA; its tag-push formal build
  fails before publication; `gh workflow run release.yml --ref v0.4.3 -f
mode=formal` reruns the exact same tag/source without any tag mutation.
- Base: the previous same-source formal transaction left an owned draft; the
  retry proves its exact failed Actions origin, deletes only that draft,
  confirms the ID is gone, and creates a fresh transaction.
- Base: unpublished `v0.4.2` is force-updated onto a later SHA after a
  notarization timeout and no published Release exists; one new formal run
  starts. A draft bound to the old SHA is not auto-deleted.
- Bad: require live `main` HEAD + successful `CI / Required`; refuse
  lightweight tags; force-move a tag just to retry the same SHA; dispatch from
  a branch ref; delete a same-tag draft without provenance; overwrite a
  published release; bump solely because Apple stayed In Progress.

### 6. Tests Required

- `tests/devReleaseEligibility.test.ts` accepts a lightweight formal tag whose
  commit SHA is the frozen source, accepts formal dispatch at that tag, and
  rejects a tag/source/mode mismatch.
- `tests/releaseWorkflow.test.ts` and `tests/devReleaseRemote.test.ts` assert
  frozen `ciRunId` / `ciRunAttempt` are null in formal mode, formal
  push/dispatch share the formal path, preflight cannot publish, and
  concurrency keys by release tag rather than event source.
- `tests/releaseDraftOwnership.test.ts` rejects published, wrong-source,
  wrong-target/time-window, malformed-marker, wrong-repository/workflow/run, successful-origin,
  missing-publish-job, successful-transaction-step, and incomplete-job-set
  evidence; it accepts only a matching failed push/formal-dispatch origin.
- Local tests do not create tags or GitHub Releases.

### 7. Wrong vs Correct

#### Wrong

```text
sourceSha = live main HEAD
require successful push CI on that SHA
reject lightweight tags
bump X.Y.Z after every failed unpublished formal run
move the same tag to retrigger the same SHA
recover draft based only on tag_name / HTML marker
```

#### Correct

```text
sourceSha = remote tag target commit
ciRunId = null
formal retry = workflow_dispatch at refs/tags/vX.Y.Z
same tag + same SHA => no tag mutation
owned same-source failed draft => prove Actions run/attempt/job, delete draft ID, confirm 404, recreate
published vX.Y.Z => immutable failure
operators may force-update unpublished vX.Y.Z
the workflow never moves the tag
```
