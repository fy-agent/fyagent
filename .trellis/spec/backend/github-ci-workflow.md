# GitHub CI Workflow Contract

## 1. Scope and stable public result

This contract owns `.github/workflows/ci.yml`,
`.github/workflows/commit-convention-push.yml`,
`scripts/ci/classify-changes.mjs`, `scripts/ci/required-gate.mjs`,
`scripts/ci/verify-commit-messages.mjs`, and their fixture suites. It applies to
pull requests, merge queue candidates, lightweight branch-push commit policy,
and manual diagnostics.

This workflow owns scheduling and aggregation, not the underlying product
behavior. Windows installer/runtime review guidance is recorded in
[Windows Installer](./windows-installer.md) and
[Windows Runtime Security](./windows-runtime-security.md); executable authority
remains in the workflow, implementation, scripts, and tests.

The only stable aggregate result is:

```text
CI / Required
```

The workflow has no top-level `paths` or `paths-ignore` filter. The aggregate
job uses `if: always()`, so a docs-only change, classifier failure, cancelled
domain job, or timeout cannot make the required result disappear.

The Required CI workflow triggers are:

```yaml
pull_request:
merge_group:
  types: [checks_requested]
workflow_dispatch:
```

- PR and `merge_group` runs execute only affected domains;
- `workflow_dispatch` is the explicit full diagnostic path;
- ordinary branch pushes and `gh-readonly-queue/**` queue-ref pushes do not
  create `CI / Required` at all.

Branch pushes use the separate lightweight workflow:

```yaml
push:
  branches-ignore:
    - "gh-readonly-queue/**"
```

It checks only Conventional Commit history and emits `Commit Convention / Push`.
It does not classify domains, install frontend/Python/Rust dependencies, run
product tests, or emit `CI / Required`. Queue-ref pushes are excluded because
`merge_group` is the sole Required CI authority for Merge Queue candidates.

Repository branch protection, rulesets, merge methods, and the Trellis
merge-readiness lifecycle are outside this workflow and are owned by
[GitHub Merge Governance](./github-merge-governance.md). In particular, a
green `CI / Required` does not authorize merge before the task/spec/prearchive
and archive lifecycle is complete. Formal Release trust is the tagged source
SHA plus the Release workflow's own native compile, not an
administrator-enforced branch claim or a prior `CI / Required` run.

## 2. Explicit change classification

The repository-owned public CLI is:

```text
node scripts/ci/classify-changes.mjs --base <sha> --head <sha> --json [--summary-file <path>]
```

Both revisions must be full 40-character commit object IDs. The classifier
does not read `GITHUB_EVENT_NAME`, `GITHUB_REF`, or any other ambient GitHub
policy input. It obtains the changed path set through a NUL-delimited Git diff,
classifies both sides of rename/copy records, and emits exactly:

```json
{
  "domains": {
    "contracts": true,
    "frontend": false,
    "desktop": false,
    "backend": false,
    "windowsNative": false,
    "docsSpec": true
  },
  "unknownPaths": [],
  "forceFull": false
}
```

Path ownership exists only in the classifier module. Workflow YAML may map
domain booleans to jobs, but it must not duplicate repository path globs.
When `--summary-file` is provided, the same classifier appends Markdown
diagnostics for the changed paths, matched owner, requested domains, and any
path-derived Full CI reason. This diagnostic output never changes the stable
JSON plan consumed by Required CI.

Classification invariants:

- CI authority that can change Required CI scheduling, classification,
  aggregation, collected step outcomes, or CI toolchain admission sets
  `forceFull=true` and every domain true;
- global checked-in toolchain authority such as mise, Node/Python/Rust version
  roots, `supported-platform-check.mjs`, `toolchain-check.mjs`, and the shared
  task library remains Full CI;
- supported-platform digest inventories
  (`supported-platform-structure-assets.json` and
  `supported-platform-raster-assets.json`) reach contracts without Full CI.
  The Changes job already runs the live checker on every Required CI plan, so
  hash bookkeeping for an otherwise-narrow change must not force unrelated
  product domains;
- release authority (`.github/workflows/release.yml`, `scripts/release/**`, the
  release task/check helpers, and release/CI contract tests) reaches the
  lightweight contracts owner without automatically requesting unrelated
  frontend, desktop, backend, or Windows-native product jobs;
- commit-message policy (`commit-convention-push.yml` and
  `verify-commit-messages.mjs`) reaches contracts without becoming Full CI;
- repository-owned GitHub automation that does not schedule or aggregate
  Required CI (`labeler.yml` and `star-history.yml`) reaches contracts without
  becoming CI authority;
- tracked GitHub metadata/templates, optional agent/editor/Trellis governance,
  and repository-governance helpers reach contracts plus docs/spec where they
  are documentation/governance surfaces rather than product code;
- repository task scripts are classified by responsibility: frontend and
  backend/native wrappers reach their affected domains, release/docs/contract
  helpers reach contracts/docs as applicable, and ambiguous shared
  task/toolchain authorities stay Full CI;
- `package.json` and pnpm dependency roots widen contracts/frontend/desktop;
- every `src-tauri/**` change reaches contracts plus its backend/native owner,
  so version, release, manifest, NSIS, and desktop-security static suites cannot
  be skipped by a Rust-only plan;
- Cargo workspace and lock roots widen contracts/backend/windowsNative;
- Windows runtime, NSIS, Windows packaging, and Codex Windows ownership reach
  contracts plus `windowsNative`;
- docs and optional Trellis specs/tasks/journals reach docsSpec plus the
  lightweight contracts owner;
- the generated root `FyAgent-前端交互预览.html` is gitignored and not in the
  Git index; its retired path still classifies as contracts plus frontend so
  deletions and history diffs are not unknownPaths;
- retired session-memory trees (`.omo/`, `memory/`) and the retired sandbox
  packaging prefix still classify as contracts plus docsSpec so deletions
  versus `main` are not unknownPaths; the classifier must not spell the
  retired sandbox token as one contiguous source string;
- a new path without an owner is returned in sorted `unknownPaths`, printed as
  JSON, and makes the CLI fail;
- unknown paths are never silently treated as no-op or full CI;
- missing revisions, non-commit objects, malformed diff output, unsafe paths,
  duplicate flags, and option injection fail closed.

`star-history.yml` is repository automation rather than Required CI authority.
It calls the public SHA-pinned `xpzouying/star-history` Action instead of
re-cloning that generator. Hosted `api.star-history.com` README embeds are not
used: after GitHub's June 2026 stargazers restriction they render a placeholder
rather than a live chart, and the official workaround puts an encrypted
contents-write token in the public README. The Action publishes only the chart
files to the unprotected `star-history` data branch. New stars refresh the
chart through the `watch` `started` event. GitHub's `schedule` trigger is
best-effort and can skip slots, so it remains the periodic reconciliation path
(including unstars), every three hours at minute 17. Manual `workflow_dispatch`
remains available. Chart generation prefers the repository secret
`STAR_HISTORY_TOKEN` and falls back to `github.token`; the Action's git push
still uses the job-local `contents: write` built-in token. The job runs on
`ubuntu-24.04`; that hosted runner label is not a shipped-product surface.

The classifier's `forceFull` is path-derived only. Event policy is applied by
the Required workflow after classification:

- PR and merge-group candidates keep the classifier plan;
- manual diagnostics replace the plan with `forceFull=true` and every existing
  domain true;
- event forcing never clears `unknownPaths` or converts a classifier error to
  success.

## 3. Base and head identity

The classifier receives these explicit inputs:

| Event               | Base                    | Head                    | Event policy     |
| ------------------- | ----------------------- | ----------------------- | ---------------- |
| `pull_request`      | `pull_request.base.sha` | `pull_request.head.sha` | affected domains |
| `merge_group`       | `merge_group.base_sha`  | `merge_group.head_sha`  | affected domains |
| `workflow_dispatch` | `github.sha`            | `github.sha`            | full             |

PR checks intentionally classify the PR head against the explicit base rather
than inferring identity from a local merge commit. Merge queue checks use the
event's explicit group base/head pair. A missing PR or merge-group SHA is a
classifier job failure and therefore a failed `CI / Required`.

The lightweight push workflow independently compares event `before` to
`github.sha` for commit-message policy. A first push or an unreachable rewritten
`before` uses `head` as an empty comparison so the current head subject is still
validated without escalating into product CI.

## 4. Domain-to-job topology

```text
commit-convention
  ↓
changes
  ├─ contracts
  ├─ frontend
  ├─ desktop-acceptance-contract
  ├─ backend-windows
  ├─ windows-native-contracts (X64, ARM64)
  └─ backend-macos
         ↓
    CI / Required
```

`commit-convention` is the fast-fail gate. It runs with checkout plus Node.js
only, validates every commit subject in the explicit base/head comparison, and
validates the pull request title on `pull_request` events. When it fails, every
downstream job is skipped so the workflow stops before expensive domain work.
FyAgent imposes no repository-defined maximum character length on an otherwise
valid Conventional Commit subject or pull-request title. The gate still
requires a non-empty Conventional Commit structure and keeps the existing
merge/revert and GitHub squash-suffix handling.
GitHub merge subjects matching `Merge pull request #<n> from ...`,
`Merge branch ...`, or `Merge remote-tracking branch ...`, and revert subjects
matching `Revert "..."`, are merge-topology records and remain accepted by the
same policy.

Empty-comparison coverage must use an isolated git fixture, not the Actions
checkout HEAD: `pull_request` checkouts are merge commits whose subject is
`Merge <sha> into <sha>`, while this job compares `PR_BASE_SHA`..`PR_HEAD_SHA`.

The requested job mapping is exact:

| Job ID                        | Requested when               |
| ----------------------------- | ---------------------------- |
| `contracts`                   | `contracts \|\| docsSpec`    |
| `frontend`                    | `frontend`                   |
| `desktop-acceptance-contract` | `desktop`                    |
| `backend-windows`             | `backend \|\| windowsNative` |
| `windows-native-contracts`    | `windowsNative`              |
| `backend-macos`               | `backend`                    |

Every domain job needs `changes` and may run only after classifier success.
`changes` needs `commit-convention` and may run only after commit validation
success. Docs/spec-only changes therefore execute the repository contracts gate
but do not start frontend, Rust, macOS, or Windows-heavy jobs. The contracts
job runs the task, docs, Python lock, version, and release contract suite; it
does not require Trellis task state, overlay reconciliation, or hook execution.

## 5. Required aggregation and timeout evidence

`scripts/ci/required-gate.mjs` is the pure evaluator for the stable aggregate.
It receives:

1. exact `toJSON(needs)` results for `commit-convention`, `changes`, plus the
   six domain job IDs;
2. the exact classifier/event plan emitted by `changes`;
3. the current workflow run-attempt Jobs REST response.

The evaluator requires exact keys and booleans. Its result rules are:

- `commit-convention` must be successful;
- `changes` must be successful;
- a requested job must be successful;
- a non-requested job must be skipped;
- a requested skip is failure, not an optimization;
- an unrequested job that runs is policy drift and fails;
- failure, cancellation, timeout, missing results, unknown conclusions,
  incomplete pagination, and result/API disagreement all fail;
- matrix conclusions are aggregated without allowing one successful child to
  hide a failed, cancelled, or timed-out child.

GitHub's `needs.<job>.result` exposes only `success`, `failure`, `cancelled`,
and `skipped`; it cannot distinguish a timeout from ordinary failure. The
Required job therefore has job-local `actions: read`, reads the exact current
run and attempt through the Jobs REST API, and uses its explicit `timed_out`
conclusion. The token is scoped only to the collection step. The evaluator
runs in the following step without a token environment.

The API response must be complete: `total_count` equals the received jobs
array length. The current topology is below one page; a future topology that
exceeds the bound must add complete pagination in the same change.

## 6. Concurrency and reruns

- Required CI includes `github.event_name` in its concurrency key. PR and
  merge-group runs therefore cannot cancel each other even if a future workflow
  change accidentally reuses the same ref text.
- PR and merge-group groups cancel stale in-progress runs when a newer candidate
  for the same PR/ref arrives.
- the lightweight push workflow has its own workflow/ref concurrency namespace
  and may cancel only an older commit-policy run for that pushed branch.
- each manual diagnostic run uses its own run ID/attempt group and sets
  `cancel-in-progress=false`; two manual diagnostics do not cancel each other.
- a native GitHub rerun remains the same run identity with a later attempt and
  keeps the original `GITHUB_SHA`/`GITHUB_REF` semantics.
- formal Release eligibility does not require a successful exact-source CI
  rerun. Preflight is independently dispatched from the trusted `main`
  workflow with an explicit immutable candidate SHA.

## 7. Job and toolchain contracts

All direct third-party Actions use reviewed full 40-character commit SHAs.
Workflow permissions default to `contents: read`; no CI job has write
permission or accesses repository secrets. Every checkout sets
`persist-credentials: false`.

CI does not invoke mise. Version and toolchain facts come from the standard
repository files through the repository-owned verifier:

- `.node-version` for Node;
- `packageManager` in `package.json` for pnpm;
- `rust-toolchain.toml` for Rust;
- `mise.lock` for the reviewed uv lock value;
- `.python-version` for managed Python.

The verifier executes ordinary tool commands directly on the admitted POSIX
CI hosts (`darwin` and `linux`). It imports that closed host predicate from the
dependency-free `scripts/tasks/platform.mjs` bootstrap owner rather than from
the package-dependent task library, because several native CI jobs resolve
toolchain facts before `pnpm install`. On Windows, only pnpm's batch shim is
routed through the selected `ComSpec` with a closed token grammar; native
executables still run directly. Any other host platform fails closed. The
verifier and its unit test are explicit development-host surfaces in the
supported-platform inventory and do not declare a shipped Linux product
target.

The workflow does not duplicate literal Node, pnpm, Rust, uv, Python, or
application versions. Rust setup disables its implicit cache. Backend and
Windows-native jobs may restore a lockfile-keyed `~/.cargo/registry` and
`~/.cargo/git` cache; they never cache `src-tauri/target` and must not set
`RUSTC_WRAPPER` or sccache in repository Cargo config. uv setup pins
the resolved reviewed version and disables cache. pnpm installation uses the
frozen lockfile. The frontend full unit suite excludes only the four
host-mise integration suites (`developmentEnvironment`, `miseTaskContract`,
`systemCheck`, and `taskDocs`); the contracts job owns their
pure/static contracts, and the local canonical check owns the real mise
boundary. Generated task documentation is verified by `task-docs.mjs check`
inside `release-check.mjs --ci`. Maintained-document `mise run` membership
and standalone setup belong to `docs-contract-check.mjs` on the local
`tasks:validate` path. Neither CI job freezes protocol names or toolchain
versions by scanning README or spec Markdown.

The always-running Changes job executes the durable supported-platform surface
checker directly after checkout and Node setup, alongside the change plan and
before diagnostic aggregation. This makes every Required CI plan scan the complete checked-out current
tree rather than relying on conditional domain jobs or checker unit tests.
GitHub-hosted Linux runner labels are not a product surface. Portable
control-plane and contract jobs—including commit policy, change
classification, repository contracts, frontend checks, mock-only desktop
acceptance, the Required aggregate, Labeler, and Star History—use the pinned
`ubuntu-24.04` label. Native product evidence remains on `windows-2025`,
`windows-11-vs2026-arm`, and `macos-15`; a portable job never substitutes for
matching-host compile/runtime/package evidence. CI never receives the
task-specific prearchive exclusion; after the lifecycle
task is archived, the canonical archive boundary applies and any new
first-party support surface fails Required CI. The Repository Contracts plan
also runs the same checker through `release-check.mjs --ci` as defense in
depth when that domain is selected.

Backend jobs run locked Cargo check, Clippy with warnings denied, and tests on
Windows and macOS. macOS additionally owns `cargo fmt --check`. The Windows
backend uses the test manifest and
the native x64 `windows-2025` runner. Before any Windows backend Cargo command
can compile the desktop crate, the job invokes the repository-owned
`scripts/prepare-windows-user-helper.mjs` with the exact x64 target and debug
profile. The dependency-free script first delegates workspace membership,
package-version inheritance, and lockfile validation to the `check` command of
`scripts/version.mjs` through the current Node executable, then builds and
atomically stages the matching helper executable. The desktop build remains
fail closed when its declared Tauri sidecar is absent; CI must not substitute an
empty placeholder or make the resource optional.

The Windows backend also owns the matching-host Credential Manager evidence
for the private SecretRef core. After the ordinary workspace Rust tests, it
runs the ignored `secret_service_contract::native_os_backend_crud_readback`
integration test with `FYAGENT_NATIVE_SECRET_TEST=1`, captures the Cargo output,
and requires both exit status zero and an exact `1 passed; 0 failed` result.
Merely compiling the Windows leaf or selecting a filter that executes zero
tests is not native credential-store evidence. The step is part of the job's
collected required outcomes, so a failed create/read/replace/delete/cleanup
round trip fails `CI / Required`.

Every CI job treats checkout as the hard prerequisite, then collects the raw
outcome of every repository-owned setup or validation step that follows it.
Every later step uses the explicit condition
`!cancelled() && steps.checkout.outcome == 'success'`: checkout failure stops
the job, cancellation stops expensive work, and an ordinary validation failure
does not hide later independent diagnostics. Collected steps must not use
`continue-on-error`; otherwise a setup action's post-job failure could be
rewritten after the evaluator has already run. A final
`evaluate-step-outcomes.mjs` step checks each exact expected step ID's raw
`outcome` and returns nonzero if any step failed, was skipped, was cancelled, or
did not report a valid result. It must not use `conclusion`, and a missing
result must fail closed. Checkout failure, setup-action post failure, runner
loss, and job timeout remain native job failures and are still rejected by
`CI / Required`.

If the changed-path classifier job finishes with `failure` rather than
`cancelled`, every conditional CI domain runs instead of trusting absent or
partial classifier outputs. This preserves fail-closed coverage while still
collecting frontend, contract, backend, desktop, and native diagnostics from
their independent checkouts. A cancelled classifier run remains cancelled and
does not start replacement diagnostics.

Within Cargo, check and Clippy use `--keep-going` so all still-buildable
dependency-graph branches are attempted before the command returns failure.
This does not claim that a target whose dependency failed can run. Rust tests
use `--no-fail-fast`, which continues across test executables after an
executable fails. Backend `cargo test` steps set `RUST_TEST_THREADS=1` because
`FYAGENT_TEST_HOME` is process-global and parallel unit tests otherwise race
on the override. Backend test commands alone enable `fyagent/test-hooks` so
integration-test fixtures can bind Windows user paths to their explicit
`FYAGENT_TEST_HOME`; check, Clippy, native contract compilation, and production
builds retain the frozen Explorer-user fail-closed boundary. The workflow
therefore exposes all diagnostics that remain
executable in the current job, then fails once at the collection boundary; it
never turns a failed check into a green job.

The repository-contract runner applies the same rule inside its single CI
step: version, lockfile, dependency, task-document, NSIS, and contract-test
diagnostics all run before it returns an aggregate failure. Composite package
scripts must not hide independent CI diagnostics behind shell `&&`; the
desktop acceptance job runs its mock test suite and its mock-contract verifier
as separately collected steps. Dependent operations may still stop locally
when their prerequisite is absent, but that failure must not suppress an
independent later diagnostic.

## 8. Matching-architecture Windows native contract

`windows-native-contracts` is a fail-fast-disabled two-entry matrix:

| Runner                  | GitHub architecture | Rust host                 | Managed Python platform |
| ----------------------- | ------------------- | ------------------------- | ----------------------- |
| `windows-2025`          | `X64`               | `x86_64-pc-windows-msvc`  | `win-amd64`             |
| `windows-11-vs2026-arm` | `ARM64`             | `aarch64-pc-windows-msvc` | `win-arm64`             |

Before Cargo compilation, each child requires exact `RUNNER_ARCH`, exactly one
`rustc -vV` host line, and equality with the matrix host. After that check and
before the explicit-SID test builds the desktop crate, the child invokes the
same dependency-free helper preparation script with `matrix.rust_host` and the
debug profile. The native job does not install pnpm or frontend dependencies.
Cargo receives that host through explicit `--target`; an x64 compatibility
process cannot satisfy the ARM64 contract. The exact test name is:

```text
codex_desktop::platform::windows::deployment::tests::native_explicit_sid_main_query_smoke
```

The command requires exactly one passed test. It exercises the real WinRT
explicit-SID `PackageTypes.Main` adapter and malformed-SID HRESULT propagation
without Store, network, a real Codex package, or a multi-account VM.

The same matrix installs the exact managed Python implementation/platform and
proves the native Python platform through the locked uv environment. Zero
tests, wrong architecture, missing managed Python, timeout, or preview runner
unavailability fails the matrix. Cross-compilation and structural inspection
are not substitutes for either native runner.

## 9. Desktop acceptance and Labeler boundaries

The desktop acceptance job remains mock-only. It runs the mock acceptance
contract, native Fetch/MSW/Tauri mock, and visual baseline preflight; it does
not start a native window or claim hardware-in-the-loop evidence.

`.github/workflows/labeler.yml` remains a separate trusted-base workflow. It
uses `pull_request_target` plus numeric manual replay, does not checkout or run
pull-request code, and has only `contents: read` and `pull-requests: write`.
Labeler is not a CI dependency and cannot satisfy `CI / Required`.
The `frontend` label owns frontend source/tooling and concrete frontend test
directories; it must not use a blanket `tests/**` rule that labels release or
CI-contract-only PRs as frontend.

## 10. Validation and evidence boundary

Required automated fixtures cover:

- docs/spec, frontend, desktop, backend, Windows native, dependency-root,
  typed control-plane ownership, release-only versus CI-authority escalation,
  multi-path union, rename/delete, unknown paths, and the retired generated
  standalone preview path;
- valid long Conventional Commit subjects and PR titles without a repository
  maximum-length gate, while malformed/empty values still fail structurally;
- classifier Markdown diagnostics while preserving the exact stable JSON plan;
- malformed/missing/non-commit base/head revisions and option injection;
- PR, merge-group, lightweight push-policy, and manual event wiring;
- manual-only event-forced full CI;
- absence of Required CI `push`, absence of queue-ref push policy, and event
  identity in Required CI concurrency;
- legal skip versus required skip, unexpected execution, failure,
  cancellation, timeout, classifier failure, incomplete API evidence, and
  result/conclusion mismatch;
- exact per-job collected step IDs, raw-outcome aggregation, cancellation
  boundaries, Cargo keep-going, and Rust test no-fail-fast semantics;
- immutable Action pins, minimal permissions, exact runners/toolchains, and
  the x64/ARM64 explicit-SID smoke wiring.

Local static and hermetic tests prove workflow structure and evaluator logic.
They do not prove a hosted runner exists or that native code executed. A
successful PR/merge-group `CI / Required` remains hosted proof for the domains
requested by that exact comparison; when Windows-native is requested it
includes the x64 and ARM64 matrix children. It is not a formal Release
eligibility gate. `windows-11-vs2026-arm` is the explicit GA Visual Studio
2026 image rather than the migrating ARM alias. Runner
unavailability blocks that CI job and is never converted into a reduced or
cross-built run.

## 11. Wrong and correct patterns

Wrong:

```yaml
on:
  pull_request:
    paths: ["src/**"]
```

```js
unknownPaths.length > 0 ? forceFull() : skipEverything();
```

```yaml
if: ${{ needs.frontend.result != 'failure' }}
```

Wrong: restore a Rust build-artifact cache or sccache wrapper.

```yaml
- uses: actions-rust-lang/setup-rust-toolchain@...
  with:
    cache: true
- uses: actions/cache@...
  with:
    path: src-tauri/target
```

Correct:

```text
explicit base/head -> repository classifier -> requested domains
requested success + authorized skip + REST conclusion -> CI / Required
tag target SHA + successful Release compile -> formal publication eligibility
setup-rust-toolchain cache: false
actions/cache path: ~/.cargo/registry + ~/.cargo/git
key: Cargo.lock + runner OS/arch
never src-tauri/target, never RUSTC_WRAPPER / sccache
```

## Scenario: Push before SHA missing after history rewrite

### 1. Scope / Trigger

- Trigger: lightweight branch-push commit policy still needs a comparison range.
  An abnormal history rewrite or force-update can leave `github.event.before`
  pointing to a commit that `actions/checkout` `fetch-depth: 0` does not clone
  once no ref points at it. This is defensive commit-policy behavior, not a
  branch synchronization contract; branch maintenance is outside Required CI.
- Owner: `.github/workflows/commit-convention-push.yml` before
  `scripts/ci/verify-commit-messages.mjs`.

### 2. Signatures

- Workflow resolves `base_sha` / `head_sha`, then
  `node scripts/ci/verify-commit-messages.mjs --base <sha> --head <sha>`.

### 3. Contracts

- `push` event `before` that is forty zeroes -> `base_sha = head_sha`.
- `push` event `before` that is not `${base_sha}^{commit}` in the clone ->
  `base_sha = head_sha` (empty comparison).
- this fallback never invokes the domain classifier or `CI / Required`.

### 4. Validation & Error Matrix

- Forty-zero or unreachable push `before` -> empty comparison and current-head
  commit subject validation only.
- Missing PR/merge-group SHA remains a Required classifier failure in
  `.github/workflows/ci.yml`.

### 5. Good / Base / Bad Cases

- Good: ordinary push; `before` is an ancestor still fetched by complete
  history; only the pushed commit range is checked for Conventional Commits.
- Base: force-update drops the previous tip; workflow logs that `before` is
  not a commit in the clone and validates `head` against `head`.
- Bad: use branch push as a second Required CI authority or start product-domain
  jobs merely to enforce commit-message policy.

### 6. Tests Required

- `tests/githubWorkflowTriggers.test.ts` asserts the push-only `git cat-file -e`
  fallback, queue-ref exclusion, and absence of `CI / Required` in the push
  workflow.
- Local tests do not clone GitHub's unreachable `before` objects.

### 7. Wrong vs Correct

#### Wrong

```bash
node scripts/ci/verify-commit-messages.mjs --base "$PUSH_BASE_SHA" --head "$head_sha"
# git cat-file: before SHA does not identify a commit object
```

#### Correct

```bash
if ! git cat-file -e "${base_sha}^{commit}" 2>/dev/null; then
  base_sha="$head_sha"
fi
node scripts/ci/verify-commit-messages.mjs --base "$base_sha" --head "$head_sha"
```
