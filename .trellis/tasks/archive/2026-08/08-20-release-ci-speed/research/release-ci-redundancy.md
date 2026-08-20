# Research: Release / CI redundancy and speed

- **Query**: Why CI/release run twice; tag immutability; what actually takes time; safe speedups that are not huge `target/` caches; existing cache usage
- **Scope**: mixed (workflows, eligibility scripts, specs, hosted-run evidence, GitHub/Cargo docs)
- **Date**: 2026-08-20

Local worktree at research time: `2d42a674` (`Release 0.4.1`). Remote `origin/main` is `04bf9939` (`Release 0.4.2: notarize the macOS DMG once`). Dual app+DMG notarization is the local contract; 0.4.2 already dropped the serial app-zip wait on `main`.

## Findings

### Files Found

| File Path | Description |
|---|---|
| `.github/workflows/ci.yml` | PR / merge-group / `main`+`dev/laiyongjie` push / dispatch CI. Push and dispatch force every domain. |
| `.github/workflows/release.yml` | Dispatch preflight vs tag-push formal. Native rebuild, pin, Windows proof/sign/seal, attest, publish. |
| `.trellis/spec/backend/github-ci-workflow.md` | CI contract: `CI / Required`, event-forced full push CI, Rust/uv caches off. |
| `.trellis/spec/backend/github-release-workflow.md` | Release contract: preflight vs formal, annotated tag == live `main` HEAD, no tag move, no candidate-controlled caches. |
| `scripts/release/dev-release-eligibility.mjs` | Pure eligibility: annotated tag, live authority HEAD, latest exact-source **push** CI. |
| `scripts/release/verify-dev-release-remote.mjs` | Live GitHub collector; formal requires `git/tags/{sha}` (annotated object). |
| `scripts/release/macos-developer-id.sh` | `notarytool submit --wait --timeout 1800`; local tree notarizes app then DMG. |
| `tests/releaseWorkflow.test.ts` | Pins `cache: false` on Release pnpm/Rust; local tests still expect `notarize-app` + `notarize-dmg`. |
| `tests/ciWorkflow.test.ts` | Pins Rust `cache: false` and uv `enable-cache: false`. |
| `tests/devReleaseEligibility.test.ts` | Lightweight tag, moved branch, stale green CI all fail closed. |
| `tests/devReleaseRemote.test.ts` | Lightweight tag rejected before CI collection; branch move fails `remoteDev.headSha`. |
| `.github/workflows/labeler.yml` | Unrelated; not a `CI / Required` input. |

### 1. Why “CI” appears to run twice

There are **three different native-heavy machines**, not one workflow looping.

```text
PR / merge-group CI     (affected domains; often still full if src-tauri or control-plane)
        ↓ merge
main (or dev) push CI   (always forceFull=true)  ← eligibility identity
        ↓ annotated tag vX.Y.Z
Release formal          (tauri release build + sign/notarize)  ← rebuilds natives
```

Optional extra cycle:

```text
dev/laiyongjie push CI  →  workflow_dispatch preflight Release  (full native rebuild, unsigned Windows)
```

**A. Eligibility requires a successful push CI, then Release rebuilds everything.**

Formal frozen identity (spec §2, evaluator output) binds:

- `sourceSha` = live remote `main` HEAD
- `ciRunId` / `ciRunAttempt` = latest **exact-source `push` CI** whose `headBranch` is `main`
- then every `build-windows` / `build-macos` job checks out that SHA and compiles **again** in release profile

CI does **not** produce installers. Release does not consume CI `target/` or binaries. The Required job is identity, not an artifact handoff.

Evidence:

- `.trellis/spec/backend/github-release-workflow.md` lines 39–41, 73–76, 116–125, 429–442
- `scripts/release/dev-release-eligibility.mjs` `selectLatestSuccessfulCi` (filters `event === "push"`, `headBranch === authorityBranch`, latest run/attempt must be `completed`/`success`)
- `scripts/release/verify-dev-release-remote.mjs` `collectCiRuns` query: `branch`, `event=push`, `head_sha`
- `.github/workflows/release.yml` `build-windows` / `build-macos` start from a clean checkout of `source_sha` and `pnpm tauri build`

**B. Tag push does not re-run `ci.yml`.**

`ci.yml` `on.push.branches` is only `main` and `dev/laiyongjie`. Tag `v*.*.*` triggers **only** `release.yml`. The “second CI” people wait on is therefore:

1. wait for `main` push `CI / Required` (eligibility), **then**
2. wait for the Release workflow’s own native jobs (rebuild)

**C. Same commit SHA on `dev/laiyongjie` cannot satisfy formal eligibility.**

Formal authority branch is `main` (`FORMAL_BRANCH` in `dev-release-eligibility.mjs`). A green `dev/laiyongjie` push run has `headBranch != main` and is ignored. Fast-forwarding `main` to that SHA still starts a **new** full push CI because event policy forces `forceFull=true` on every configured-branch push (`ci.yml` lines 70–74, 100–103; spec §2).

**D. Preflight is not a formal prerequisite, but it is a full native rebuild if used.**

Spec §1: dispatch “is not a release-closure prerequisite” and “can never create or update a GitHub Release”. Topology is mutually exclusive (`prove-windows-preflight` vs `sign-windows-formal`/`seal-windows-formal`). Running dispatch then tagging therefore does two complete Release native graphs.

Hosted proof that recent 0.4.0/0.4.1 tags did **not** use preflight; last successful dispatch was `31829707288` (2026-08-14, ~29 min). The painful double wait on 0.4.0 was **PR CI + main CI + formal Release**, not preflight.

**E. Inside one CI run, Windows compiles twice on `windows-2025`.**

On every `main`/`dev` push, both of these are requested:

- `backend-windows`: `cargo check` + Clippy + `cargo test` (debug, workspace)
- `windows-native-contracts` (X64): another Cargo compile of the desktop crate for the explicit-SID smoke

Plus `backend-macos` (fmt/check/clippy/test) in parallel with Release later compiling **release** universal (two Apple targets).

**F. Spec drift (CI vs Release).**

`github-ci-workflow.md` §1 still says “the current release trust is the exact successful **dev** push chain”. Implemented formal eligibility and `github-release-workflow.md` use **`main`**. Treat the Release spec + scripts as source of truth for publication.

### 2. Tag immutability gates

Formal admission requires all of these at once, then **again** at publish-start and immediately before the final PATCH:

| Gate | Where | Failure mode |
|---|---|---|
| YAML filter `v*.*.*` is only routing | `release.yml` `on.push.tags` | Eligibility still requires stable `vX.Y.Z` |
| Tag name == `v` + Cargo version | `release.yml` contract step; `validateCandidate` | Mismatch fails before builds |
| Ref object type must be `tag`, not `commit` | `validateRemoteTag`; remote collector `expectEqual(tagRefObject.type, "tag")` | Lightweight tags fail **before** CI collection |
| Annotated tag name exact; target type `commit`; target SHA == frozen `sourceSha` | `validateRemoteTag` | Tag pointing elsewhere fails |
| Live `main` HEAD == frozen `sourceSha` | `validateRemoteDev` + `expectEqual(remoteDevHeadSha, candidate.sourceSha)` | Any later `main` push during the ~30 min Release fails eligibility recheck |
| Workflow never moves/deletes the tag | spec §3, §8, failure matrix | Failed run is not repaired by retargeting `vX.Y.Z` |
| Existing draft/published Release for that tag | publish job | Refuses update/replace/delete |
| Latest exact-source `main` push CI must still be the frozen run/attempt | live `--expected` recheck | A newer failed/cancelled/in-progress attempt, or `main` moving, stops publication |

Tests:

- `tests/devReleaseEligibility.test.ts` “lightweight tag”, “tag target SHA”, “does not accept an old green commit after the dev branch moves”
- `tests/devReleaseRemote.test.ts` “rejects a lightweight formal tag before collecting CI evidence”, “rejects when the dev branch moves”

What GitHub itself allows vs what this repo allows:

- Git can force-update a tag unless a ruleset forbids it. This project **treats that as a failure**, not a recovery tool (`tag replacement` in the failure matrix).
- A **rerun of the same Release run** is allowed while that SHA is still live `main` HEAD (CI spec §6; eligibility selects latest attempt). That is the supported retry. Moving `v0.4.1` to a fix commit is not; 0.4.1 timeout was followed by a **new version** `v0.4.2`.

Operational rigidity: you must tag the **current** `main` HEAD, then keep `main` still for the whole formal graph. A docs-only follow-up push during notarization fails the pre-PATCH recheck even if artifacts are already built.

### 3. What actually takes time (hosted evidence 2026-08-20)

Eligibility, Windows signing, sealing, verify, attest, and publish are **not** the wall clock.

#### CI `32378172410` (`main` push, success, ~17 min)

| Job | Minutes | Notes |
|---|---:|---|
| Classify Changes | 0.4 | |
| Desktop Acceptance Contract | 0.8 | |
| Repository Contracts | 1.4 | pnpm cache on |
| Frontend Checks | 2.1 | pnpm cache on |
| Windows Native Contracts (X64) | 6.5 | Rust cache off |
| Windows Native Contracts (ARM64) | 7.8 | preview runner |
| Backend Checks (macOS) | 8.2 | check+clippy+test |
| **Backend Checks (Windows)** | **16.3** | **critical path** |
| CI / Required | 0.2 | aggregation only |

PR runs the same day were ~14 min. Merge still starts a second full run (~14–17 min) because push forces every domain.

#### Release `32349268303` (`v0.4.0`, success, unsigned macOS era, ~32 min)

| Job | Minutes | Notes |
|---|---:|---|
| Eligibility | 0.4 | API collect + freeze |
| Windows x64 NSIS | 21.1 | `pnpm tauri build` + NSIS |
| Windows ARM64 NSIS | 20.8 | parallel |
| **macOS universal** | **27.8** | **critical path** |
| Pin trusted inputs | 0.5 | waits for **both** Windows and macOS |
| Windows formal sign (each) | 0.5–0.7 | no recompile |
| Fresh seal (each) | 0.5–0.9 | no recompile |
| Verify / attest / publish | 0.5 / 0.5 / 0.9 | |

Windows finished ~7 min before macOS; sign/seal could not start until pin, so the topology serializes Windows post-processing behind macOS.

#### Release `32369906972` (`v0.4.1`, failure, ~58 min)

| Job | Minutes | Notes |
|---|---:|---|
| Windows x64 / ARM64 | 19.5 / 21.2 | success; unused after macOS fail |
| **macOS Developer ID + notarize** | **57.5 fail** | build ~27 min, then `notarytool` |
| Pin / sign / seal / publish | skipped | |

Failed step log (`Seal, notarize, and verify the Developer ID app`):

- notarize step began `2026-08-20T13:06:17Z`
- `{"message":"Timeout of 1800 second(s) was reached before processing completed."}` at `13:36:28Z`
- `macos-developer-id.sh` uses `xcrun notarytool submit … --wait --timeout 1800`
- Local tree then still had a **second** DMG notarization wait (never reached)

`origin/main` `04bf9939` already changes this to one DMG submission + staple-from-ticket. That removes a serial 0–30 min wait; it does **not** remove the ~20–28 min cold `tauri` compiles.

#### Preflight `31829707288` (2026-08-14, success, ~29 min)

Same native shape as formal minus sign/seal/publish: macOS ~26 min critical path; unsigned Windows sealing ~0.5 min. Confirms preflight cost ≈ formal native cost when Apple notarization is not in the path.

#### Time ranking (what to optimize)

1. **Release native compiles** (Windows ~20 min each, macOS universal ~28 min) — Release wall clock
2. **Apple notarization wait** (0–30 min per `notarytool --wait`; 0.4.1 hit the cap on the **app** zip)
3. **CI Windows backend** (~16 min) — CI wall clock
4. **Duplicate full CI on merge** (~14 min extra after a green PR)
5. **Pin-after-both-natives** (~7 min Windows idle on v0.4.0)
6. Eligibility / Authenticode / attest / publish — seconds to ~1 min each

CI `cargo test`/`check` (debug) and Release `tauri build` (release, NSIS / universal app) are **different artifacts**. Speeding CI does not shorten Release compiles unless artifacts are reused or compiles are skipped.

### 4. Existing cache usage

No `actions/cache`, Swatinem `rust-cache`, or `sccache` steps exist under `.github/workflows/`.

| Location | Cache setting | Effect |
|---|---|---|
| CI `contracts` / `frontend` / `desktop-acceptance-contract` `setup-node` | `cache: pnpm` + `cache-dependency-path: pnpm-lock.yaml` | **Only enabled caches** |
| CI `backend-windows` / `backend-macos` / `windows-native` `setup-node` | no pnpm cache | frontend not installed there |
| CI all `setup-rust-toolchain` | `cache: false` (4 jobs; `tests/ciWorkflow.test.ts`) | no `target/` restore |
| CI `setup-uv` | `enable-cache: false` | |
| Release `pnpm/action-setup` (Windows + macOS builds) | `cache: false` (`tests/releaseWorkflow.test.ts`) | |
| Release `setup-rust-toolchain` | `cache: false` | |
| Release `setup-node` | no `cache: pnpm` | still `pnpm install --frozen-lockfile` on native jobs |

`actions-rust-lang/setup-rust-toolchain` with `cache: false` still injects (visible in the v0.4.1 macOS job env):

- `CARGO_INCREMENTAL: 0`
- `CARGO_REGISTRIES_CRATES_IO_PROTOCOL: sparse`
- `CARGO_UNSTABLE_SPARSE_REGISTRY: true`

There is **no** `.cargo/config.toml` in the repo. Cargo 1.70+ (this repo pins 1.97.1) already defaults crates.io to sparse. “Turn on cargo sparse” is already true in hosted jobs.

Release spec §5: required jobs must not “restore **candidate-controlled** release caches”. Tests encode `cache: false` as a contract, not a comment.

GitHub Actions cache default quota is **10 GB / repo** ([dependency caching](https://docs.github.com/en/actions/reference/workflows-and-actions/dependency-caching)). Hosted runners guarantee **14 GB SSD** ([runner reference](https://docs.github.com/en/actions/reference/runners/github-hosted-runners)). `windows-2025` no longer exposes a large `D:` workspace disk ([runner-images#12609](https://github.com/actions/runner-images/issues/12609)); restoring a Tauri `target/` (especially universal two-arch) is the fill-SSD failure mode the user named.

### 5. Options that are not huge `target/` caches (ranked by risk)

Risk = contract / supply-chain / identity breakage, not implementation size. Expected win is wall-clock vs v0.4.0-class runs.

#### R0 — Operational, no spec change (lowest)

| Option | Expected win | Notes |
|---|---|---|
| Do not dispatch preflight before every formal | ~30 min per avoided Release graph | Spec already: preflight is optional diagnostic, not a gate |
| Do not treat PR green as a substitute for waiting, but **do** start the tag only after `main` CI — avoid a third native graph | avoids dispatch+formal | 0.4.0 already skipped preflight |
| Rerun the failed Release run instead of moving the tag, **only if** `main` HEAD is unchanged and no Release exists | recovers notarize flakes | Supported; v0.4.1 could not be retargeted |

#### R1 — Low: drop work that is already redundant with the contract

| Option | Expected win | Contract impact |
|---|---|---|
| Keep `origin/main` 0.4.2 “notarize DMG once” (already on remote) | removes a serial 0–30 min `notarytool` wait | Already specified on `04bf9939`; local tree still dual-notarize |
| Stop hoping cargo sparse / `CARGO_INCREMENTAL=0` will help | ~0 | Already set by `setup-rust-toolchain` |
| Make push CI skip domains the PR just proved | **cannot** feed eligibility | Eligibility requires **this SHA’s** `push` + `headBranch=main` `CI / Required`, not the PR event |

#### R2 — Low–medium: small caches / no `target/`

| Option | Expected win | Why not `target/` | Contract impact |
|---|---|---|---|
| Cache **only** `~/.cargo/registry` (+ maybe `~/.cargo/git`), keyed on `src-tauri/Cargo.lock`, never `src-tauri/target` | crate download minutes, not compile | Hundreds of MB vs multi-GB `target/` | CI spec “Rust setup disables its implicit cache”; Release spec forbids candidate-controlled **release** caches. A lockfile-keyed registry cache on `push` (not `pull_request`) is the least-poisonous reading, still a spec+test change |
| Enable `cache: pnpm` on Release native jobs (CI frontend already has it) | `pnpm install` seconds–low minutes | Node store, not Rust | Release tests require pnpm `cache: false` today |
| `CARGO_HOME` registry cache on CI backend jobs only | small | same | CI tests require rust-action `cache: false`; a **separate** `actions/cache` path would be a new pin (full SHA) |

Do **not**: `setup-rust-toolchain` `cache: true`, Swatinem `rust-cache` default (`~/.cargo` + `./target`), or `actions/cache` of `src-tauri/target`. GitHub 10 GB quota + 14 GB runner SSD + universal two-arch artifacts match the user’s fill-disk constraint. [Depot on sccache vs `target/` blobs](https://depot.dev/blog/sccache-in-github-actions); [Swatinem/rust-cache](https://github.com/swatinem/rust-cache) still snapshots `./target`.

#### R3 — Medium: sccache (object cache, not directory blob)

| Option | Expected win | Caveats |
|---|---|---|
| `RUSTC_WRAPPER=sccache` + [Mozilla-Actions/sccache-action](https://github.com/Mozilla-Actions/sccache-action) using GHA backend | dependency crate compiles; **not** final linked bins | Still counts against 10 GB GHA cache; [sccache#2566](https://github.com/mozilla/sccache/issues/2566) cache-thrash warning; mozilla README: `bin` / `cdylib` / `proc-macro` crates that invoke the linker are **not** cached — the Tauri app binary is a `bin`; third-party Action needs a reviewed full SHA; local `host-native` **rejects compiler wrappers** (`.trellis/spec/backend/development-environment.md` §5) so wrapper must stay GHA-env-only, never `.cargo/config.toml` |

sccache is a plausible **CI backend** experiment. Using it on **Release** collides harder with “no candidate-controlled release caches” and with the desire that published bytes come from a clean compile of frozen SHA.

#### R4 — Medium: skip duplicate jobs without changing trust

| Option | Expected win | Caveats |
|---|---|---|
| Narrow `windows-native-contracts` so it does not rebuild the whole desktop crate when `backend-windows` already compiled (e.g. reuse workspace artifacts **within the same job only** — they cannot share `target/` across jobs) | little unless jobs are merged | Merging X64 native smoke into `backend-windows` saves one cold compile on `windows-2025` (~6 min overlap, not full 16). ARM64 native job remains |
| Split `pin-release-build-inputs` so Windows sign/seal can start when Windows raw artifacts exist, without waiting for macOS | ~7 min on v0.4.0-shaped runs | Spec §4: one pin job waits for **all** native builds and uploads one `trusted-build-inputs` ID. Splitting is a trust-topology change (two pins, two IDs) |
| Merge preflight **code path** into formal (delete dispatch) | none if dispatch unused; removes a foot-gun | Large YAML/test/spec deletion; loses unsigned diagnostic on `dev/laiyongjie` |

#### R5 — Medium–high: reuse artifacts across workflows

| Option | Expected win | Why it is not a drop-in |
|---|---|---|
| Reuse preflight artifacts as formal inputs | ~20–28 min native if SHA identical | Spec: preflight is not authority; Windows preflight is **unsigned**; pin artifact IDs are **per run**; macOS signing/notarization happens **inside** `build-macos` of that run; attestation subjects are this run’s bytes; formal still needs Authenticode transform + fresh seal |
| Feed CI binaries into Release | CI never builds release NSIS/DMG | Would mean adding packaging to CI (secrets on CI, or unsigned-only) and a new digest pin across workflows. Cross-workflow artifact trust is exactly what pin-by-original-ID inside **one** Release run is designed to avoid |
| Skip requiring prior `CI / Required` identity | ~14–17 min wait before tagging | Current trust root **instead of** branch protection (spec §1: `main` HEAD + exact-source CI, not rulesets). Dropping it is a product/security decision, not a cache tweak |

#### R6 — High: relax tag / HEAD immutability

| Option | Expected win | Breakage |
|---|---|---|
| Allow lightweight tags | easier local `git tag` | Collector refuses `refObject.type=commit`; tests cover this |
| Allow moving `vX.Y.Z` to a new SHA after a failed notarize | retry without bumping version | Frozen tag object SHA + target SHA + “never move the tag” + existing-Release refusal. GitHub Releases are tag-name keyed; moving tags is a known supply-chain foot-gun |
| Allow publish while live `main` has moved past the tag | `main` can receive fixes during notarization | Live recheck compares `main` HEAD to frozen SHA **twice**; this is what stops “tag an old commit after main moved” |
| Infer formal from preflight success | skip one native graph | Spec §10: “formal closure neither requires nor infers success from” preflight; different authority branch |

A **version bump** (`v0.4.1` → `v0.4.2`) is the currently supported escape hatch when a tag is burned or notarization cannot be retried on the same SHA.

### Related Specs

- `.trellis/spec/backend/github-release-workflow.md` — two modes, frozen identity, annotated tag, no caches, no tag repair, pin topology, dual (local) vs single (0.4.2 remote) notarization
- `.trellis/spec/backend/github-ci-workflow.md` — force-full push CI, Rust/uv caches off, Required aggregation; **stale “dev push chain” sentence vs implemented `main` formal**
- `.trellis/spec/backend/development-environment.md` §5 — local host-native forbids `RUSTC_WRAPPER` / compiler wrappers (sccache must not land in repo Cargo config)

### External References

- [GitHub Actions cache usage limits (default 10 GB / repo)](https://docs.github.com/en/actions/reference/workflows-and-actions/dependency-caching)
- [GitHub-hosted runner SSD (14 GB guaranteed)](https://docs.github.com/en/actions/reference/runners/github-hosted-runners)
- [windows-2025 D: drive removal / less free disk](https://github.com/actions/runner-images/issues/12609)
- [Cargo sparse protocol default since 1.70](https://blog.rust-lang.org/2023/06/01/Rust-1.70.0/)
- [mozilla/sccache README — linker-invoking bins not cached](https://github.com/mozilla/sccache/blob/main/README.md)
- [sccache-action GHA backend](https://github.com/Mozilla-Actions/sccache-action)
- Hosted runs: [CI 32378172410](https://github.com/fy-agent/fyagent/actions/runs/32378172410), [Release v0.4.0 32349268303](https://github.com/fy-agent/fyagent/actions/runs/32349268303), [Release v0.4.1 fail 32369906972](https://github.com/fy-agent/fyagent/actions/runs/32369906972)

## Caveats / Not Found

- This worktree is **one commit behind** `origin/main` 0.4.2 notarize-once. Implement against whichever SHA is the task base.
- Job minutes are from `gh run view --json jobs` (start/complete timestamps), not step-level flame graphs. `pnpm tauri build` vs notarize split for v0.4.1 is inferred from the notarize step start vs job start.
- No in-repo measurement of `src-tauri/target` bytes on `windows-2025` / `macos-15`. Disk-fill risk is from GitHub’s 14 GB floor + public Windows 2025 disk reports, not a local `du`.
- Whether org cache quota was raised above 10 GB was not queried.
- Tag **rulesets** / branch protection were not read; eligibility explicitly does not use them.
- `labeler.yml` and visual preflight (`pnpm test:desktop:visual:preflight`) are unrelated to Release preflight mode.
