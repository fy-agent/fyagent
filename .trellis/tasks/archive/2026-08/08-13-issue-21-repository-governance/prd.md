# Issue 21 repository governance

## Goal

Close [GitHub Issue #21](https://github.com/fy-agent/fyagent/issues/21), including
the maintainer's 2026-08-13 governance addendum, through one focused and
evidence-backed pull request. The result must remove workstation-specific
metadata from the current public tree, make the repository's current purpose,
architecture, development and validation boundaries, canonical GitHub source,
contribution flow, and governance scan results explicit, then pass the real
pull-request gate, merge to `main`, and archive this Trellis task.

## Background and confirmed facts

- The verified planning base is
  `origin/main@9be29455a081d3ff0bc761465672727d09ffb3e6`; the task branch is
  `codex/issue-21-repository-governance` and targets `main`.
- `fy-agent/fyagent` is the public canonical organization repository and is
  not a fork. This checkout's only remote is `origin`, with fetch and push both
  targeting the canonical repository.
- The current public Markdown tree contains six real Windows user-profile
  paths: the five references named by Issue #21 plus one committed Trellis
  archive reference. Generic examples such as `C:\Users\<username>\...`,
  localized angle-bracket placeholders and neutral `example/profile/...`
  paths are documentation examples and are not personal identifiers.
- The three root README files already describe the product goal, current
  capability boundary, active-development status, and release-trust boundary.
  They do not yet provide a useful runtime architecture overview; their first
  checkout flow omits `mise run system:check`; and they do not clearly separate
  current-host validation, the remote `CI / Required` merge authority, and
  formal Release evidence.
- `CONTRIBUTING.md` says `CI / Required` is the multi-platform merge authority.
  GitHub currently requires pull requests on `main`, enforces that rule for
  administrators, allows squash/merge/rebase, and forbids force-push and branch
  deletion, but it currently requires neither a status check nor an approval.
- `.github/CODEOWNERS` claims that Code Owner review is required even though
  the live branch protection has that option disabled. Enabling the current
  owner rule without verifying reviewer availability could block all merges.
- Repository history is public evidence and will retain pre-fix content unless
  rewritten. This task must report that boundary; it must not rewrite public
  Git history to hide already-published strings.
- The user approved making `CI / Required` the only platform-enforced required
  check while retaining zero required approvals and no Code Owner enforcement.

## Requirements

### R1. Redact current public workstation paths

- Replace all six real Windows user-profile paths in tracked Markdown with
  stable descriptive placeholders.
- Preserve every provenance role, SHA-256 value, evidence classification, and
  audit conclusion surrounding those paths.
- Preserve legitimate generic and localized path examples unchanged.
- Treat the single archived Trellis edit as a narrowly scoped privacy
  redaction required by Issue #21; do not otherwise rewrite archived task
  content or conclusions.
- Add or extend an executable repository check so a future real
  `C:\Users\<name>\...` Markdown path fails while approved angle-bracket and
  explicit demo examples remain valid.

### R2. Complete the public README contract

- Keep `README.md`, `README_EN.md`, and `README_JA.md` semantically aligned.
- Preserve the verified product goal, current-versus-vision boundary, current
  release status, and signing/notarization limitations.
- Add a concise, factual architecture path covering the React/Vite renderer,
  Tauri IPC boundary, Rust commands/services, and local SQLite/configuration
  and proxy surfaces, with a link to maintained development documentation.
- Correct first-checkout development instructions to require `mise >= 2026.8.0`
  and the sequence `mise trust` -> `mise run bootstrap` ->
  `mise run system:check` -> `mise run dev`.
- Separate interactive development, optional current-host build evidence,
  current-host `mise run check`, remote `CI / Required`, and formal Release
  evidence. Do not imply native runtime, installer lifecycle, signing,
  notarization, or published Release evidence that was not executed.
- Describe supported configuration domains and the independent WorkBuddy entry
  without overstating current product capability.

### R3. Record canonical repository and contribution topology

- Update maintained contributor documentation in English and Chinese to state
  that `fy-agent/fyagent` is the canonical source of truth.
- Distinguish a maintainer checkout that may use canonical `origin` from an
  external contributor fork where the personal fork is `origin` and the
  canonical repository is a fetch source such as `upstream`.
- Do not mutate contributors' remotes, claim that a personal fork exists, or
  confuse the canonical repository with the separately controlled CC Switch
  upstream used by repository maintenance tasks.
- Document the actual branch -> focused PR -> `CI / Required` -> squash merge
  flow and retain the no-force-push/no-direct-main expectation.

### R4. Produce safe, reproducible governance scan evidence

- Record the exact source SHA, date, scope, commands/tool versions, sanitized
  findings, and result for account/local identifier, workstation path, secret,
  and large-file scans.
- Distinguish the current tracked tree from reachable Git history and normal
  attribution metadata. Never print, commit, or quote a discovered secret.
- Report findings by category, path, object ID or count only when safe. A
  plausible live secret is a stop condition requiring private remediation;
  it must not be merged as public evidence.
- Use a repository-owned, dependency-free audit helper that reads current-tree
  and all-reachable-history blob contents in memory, fails closed on unreadable
  objects, and emits only category, sanitized path, object ID, and count. Its
  synthetic tests must prove that raw candidates never reach stdout or JSON.
- Scan both current tracked files and reachable history for unexpectedly large
  blobs, while treating intentionally tracked/LFS-governed assets according to
  repository policy. Do not rewrite history in this task.
- Prefer existing repository and GitHub capabilities; add no dependency solely
  for this audit. Any new helper must be repository-owned, deterministic, and
  covered by executable tests.

### R5. Reconcile documented and live review policy

- Remove or correct the false `.github/CODEOWNERS` assertion that Code Owner
  approval is currently enforced.
- Preserve the owner mapping unless repository evidence demonstrates that a
  different owner is authorized and available.
- Keep the written merge rule and live GitHub protection state consistent with
  the approved policy: `CI / Required` is the only required status check,
  strict up-to-date enforcement remains off, approvals remain at zero, and
  Code Owner review remains advisory rather than required.

### R6. Validate, publish, merge, and close the evidence loop

- Run the applicable focused checks and the canonical current-host
  `mise run check`; record any platform/evidence limits precisely.
- Review the complete diff for public wording, secret exposure, unrelated
  changes, and rollback safety before committing.
- Commit using the repository's Conventional Commit style, push only the task
  branch, create a non-draft PR targeting `main`, and link Issue #21 so the
  successful merge closes it.
- Wait in the foreground for the PR's exact-head `CI / Required` result. Do not
  merge a failed, cancelled, stale, absent, or ambiguous result.
- Use squash merge as the fastest repository-supported method after all chosen
  gates pass. Verify the merged PR, resulting remote `main` SHA, closed Issue,
  post-merge `main` CI state, and the final governance policy.
- Finish and archive this Trellis task only after the repository and GitHub
  acceptance criteria are satisfied. Do not describe archive metadata as
  product or CI evidence.
- Treat final Trellis finish/archive as local administrative state. Use
  `archive --no-commit` after remote acceptance and do not push a second
  archive/journal PR without new authorization.

## Acceptance Criteria

- [ ] AC1: The current `main` tree contains no real Windows username in public
      Markdown; the six identified references use stable placeholders and all
      surrounding provenance, hashes, roles, and conclusions remain intact.
- [ ] AC2: Generic user-path examples continue to pass an executable negative
      and positive regression contract.
- [ ] AC3: All three root READMEs consistently cover goal, current scope,
      architecture, first checkout, local/remote validation, current status,
      and evidence limitations using facts supported by current code and docs.
- [ ] AC4: Maintained contributor documentation identifies
      `fy-agent/fyagent` as canonical, explains maintainer versus fork remotes,
      and defines the branch -> PR -> Required CI -> squash merge path without
      mutating a user's local remotes.
- [ ] AC5: A committed, sanitized audit record provides reproducible current
      tree and history results for account/local IDs, secrets, and large files;
      no secret material is exposed by the record or command output, and a
      synthetic executable test proves raw candidates are suppressed.
- [ ] AC6: `.github/CODEOWNERS`, contributor documentation, and live `main`
      protection no longer make contradictory enforcement claims.
- [ ] AC7: Applicable focused checks and `mise run check` pass on the current
      host, and the exact PR head obtains a successful `CI / Required` result.
- [ ] AC8: The branch is squash-merged into `main`, the merge and post-merge
      state are verified, Issue #21 is closed, and this task is finished and
      archived locally with `--no-commit` and its validation and residual-risk
      record. No second administrative archive PR is implied.

## Out of scope

- Product runtime behavior, public APIs, persistence schemas, installer or
  Release workflow changes, new dependencies, release tags, or a new release.
- Git history rewriting, force-pushing `main`, deleting published evidence, or
  attempting to remove ordinary Git author/committer attribution.
- Changing the CC Switch upstream synchronization contract or mutating a
  contributor's local Git remotes.
- Enabling a mandatory human approval or changing the CODEOWNERS identity
  without a separately verified and authorized reviewer policy.
- Remediating a plausible live secret in public; such a finding pauses this
  task for private rotation/removal instructions.

## Key decisions

- Deliver the Issue and its governance addendum through one focused task and
  one PR; the documentation, scan evidence, policy reconciliation, and merge
  proof share one acceptance boundary and do not justify multiple PRs.
- Require `CI / Required` through live branch protection after the exact PR
  head has produced that successful context. Keep `strict: false`, approval
  count zero, and Code Owner enforcement disabled for the minimum approved
  behavior change and fastest policy-compliant merge.
- Add no dependency and no new broad secret-scanning or blob-size CI product.
  Commit reproducible, sanitized audit evidence and extend the existing public
  documentation contract only where an executable regression is justified.
- Do not rewrite Git history. Public history remains a stated evidence and
  residual-privacy boundary; the current tree is remediated.
- After the one public governance PR and remote acceptance, finish/archive the
  Trellis task locally with no auto-commit. Administrative archive/journal
  changes remain local unless the user later authorizes a separate PR.
