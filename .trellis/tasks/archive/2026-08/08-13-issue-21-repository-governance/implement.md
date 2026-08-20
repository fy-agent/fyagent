# Implementation plan: Issue 21 repository governance

## Phase A — pre-development and source lock

- [ ] Wait for the user to explicitly approve the final planning summary.
- [ ] Run `task.py start` and verify the task status becomes `in_progress`.
      Do not edit repository deliverables or dispatch an implementer before
      both gates succeed.
- [ ] Load Phase 2.1 context and dispatch the Trellis implementer with this
      task's curated context.
- [ ] Fetch `origin`, verify the branch is still based on the current remote
      `main`, and inspect any new Issue #21 comments before editing.
- [ ] Record the current branch-protection and GitHub security-analysis
      snapshots without secrets; stop if they differ materially from planning.
- [ ] Confirm the worktree contains only this task's planning artifacts.

Completion: exact source, requirement, and external-policy inputs are current;
no unreviewed drift exists.

## Phase B — repository content and executable contract

- [ ] Replace the six concrete user-profile paths with the four approved
      semantic placeholders. Preserve surrounding provenance and conclusions.
- [ ] Update `README.md`, `README_EN.md`, and `README_JA.md` with semantically
      aligned scope, architecture, onboarding, validation/evidence, and
      WorkBuddy text.
- [ ] Update both language halves of `CONTRIBUTING.md` with canonical source,
      maintainer/fork/CC-Switch remote roles, and branch -> PR -> exact-head
      Required CI -> squash flow.
- [ ] Correct `.github/CODEOWNERS` comments while preserving mappings.
- [ ] Add the dated, sanitized repository-governance audit under
      `docs/fyagent/audits/`.
- [ ] Add the dependency-free `scripts/audit/repository-governance-scan.mjs`
      helper plus synthetic tests proving current/history blob enumeration,
      binary/deleted/pathless handling, fail-closed errors, safe paths, and
      complete suppression of raw secret candidates.
- [ ] Extend `tests/currentDocsContract.test.ts` to cover every tracked
      Markdown profile path and the durable README/contribution/CODEOWNERS
      concepts. Do not create a second runner or dependency.

Completion: the diff contains only Issue #21/governance-owned files and every
new durable statement has executable or cited repository evidence.

## Phase C — safe scan evidence and local validation

- [ ] Run the tracked Markdown scan; verify zero concrete user-profile paths
      and preservation of approved localized/demo examples.
- [ ] Capture the exact audit-helper, Git, GitHub CLI, Node, mise, and GitHub
      REST API versions in the sanitized audit record.
- [ ] Run current-tree and reachable-history account/local-ID probes. Classify
      canonical owners, CODEOWNERS, licenses, Git attribution, historical URLs,
      placeholders, and actual workstation identifiers separately.
- [ ] Use the audited helper to enumerate and read unique current and all-ref
      reachable blobs in memory, including deleted/renamed/pathless/binary
      blobs. Emit only category, sanitized path, OID, and count; fail closed on
      any incomplete/object-read/parser condition. Stop and privately escalate
      any plausible live secret without writing it to audit/PR/task/log output.
- [ ] Run current-tree and reachable-history object-size inventories; verify
      the 10 MiB review count and reviewed top-blob list.
- [ ] Query live GitHub security-analysis state and record its timestamp and
      limitations in the audit.
- [ ] Run focused tests:
      `mise run test:unit tests/currentDocsContract.test.ts`.
- [ ] Run `mise run release:check`, then canonical `mise run check`.
- [ ] Run Trellis check via a separate check sub-agent, including docs quality,
      code quality, and security/diff views. Fix validated findings and repeat
      affected checks.

Completion: focused and full local gates pass; tool versions and preliminary
scan methodology are verified; no plausible secret or unresolved review
finding remains. Final candidate scans still occur after staging in Phase D.

## Phase D — commit, PR, and exact-head remote gate

- [ ] Re-fetch `origin/main`, reconcile safely if it advanced, rerun affected
      scans/tests, and inspect the final diff.
- [ ] Finalize every intended repository and task file, then present the
      Trellis-required one-shot commit plan and obtain explicit commit
      confirmation.
- [ ] Stage exactly the approved files, derive the candidate index tree with
      `git write-tree`, and run path/account/secret/blob scans against that
      exact tree plus all reachable history. If any staged file changes, restage
      and repeat every affected scan.
- [ ] Validate Trellis context/artifacts and record implementation/check
      evidence without inventing remote results. Re-stage and repeat the
      candidate scans if this writes a committed file.
- [ ] Commit using the approved focused Conventional Commit plan. Verify the
      commit tree equals the scanned index tree, then run a read-only exact-head
      consistency scan; record the result in the PR/session, not a new
      self-referential commit.
- [ ] Push `codex/issue-21-repository-governance` to `origin`.
- [ ] Create a non-draft PR targeting `main` with `Closes #21`, the exact final
      head SHA, scan summary, local tests, branch-protection change, risks,
      limits, and rollback.
- [ ] Bind the remote run to PR number, exact head SHA,
      `.github/workflows/ci.yml`, event `pull_request`, latest attempt, check
      name `CI / Required`, and GitHub Actions app ID `15368`. Keep the watcher
      in the foreground until it succeeds. Treat
      failure/cancellation/absence/staleness/provider mismatch as a blocker.

Completion: the exact pushed head is reviewable and has one successful stable
aggregate check.

## Phase E — branch protection and merge

- [ ] Fetch a fresh full protection snapshot, normalize it into the writable
      request schema, and compare every approved field with the expected
      pre-change state. Acknowledge the subsequent update is optimistic, not an
      atomic CAS; unsafe methods have no documented ETag precondition here.
- [ ] Prepare/review the full GitHub API payload. Add only non-strict
      `CI / Required` bound to GitHub Actions app ID `15368`; preserve
      PR/admin/approval/force/delete and all other settings exactly.
- [ ] Apply the branch-protection update, fetch it again, and assert every
      target field. On mismatch, refetch: roll back only if the live state is
      still exactly this task's expected post-state. Otherwise do not overwrite
      possible concurrent maintenance; stop for human reconciliation.
- [ ] Re-read PR mergeability, unresolved conversations, exact head SHA, and
      `CI / Required` immediately before merge.
- [ ] Squash merge through the PR without direct `main` pushes or protection
      bypass.

Completion: PR reports merged through the approved path and live protection
contains exactly the approved required check.

## Phase F — post-merge acceptance and task archival

- [ ] Read the merged PR's `merge_commit_sha`, verify it is reachable from
      remote `main`, and record whether it is still the tip without requiring
      that no later merge has occurred.
- [ ] Verify Issue #21 is closed and the PR metadata/commit are public.
- [ ] Wait for that exact merge SHA's latest-attempt `push` run of
      `.github/workflows/ci.yml` and GitHub Actions-owned `CI / Required` to
      complete successfully; do not substitute the PR run.
- [ ] Re-run remote/current-tree privacy checks and re-fetch live protection.
- [ ] Record final verified evidence, residual limits (history retention,
      GitHub secret scanning disabled unless independently changed, no HIL),
      and rollback in the task/session record.
- [ ] Run the Trellis spec-update decision. Update a durable spec only if the
      implementation establishes a reusable contract not already owned by
      executable tests and maintained docs.
- [ ] Finish the task and archive
      `08-13-issue-21-repository-governance --no-commit` locally only after all
      acceptance criteria pass. Do not push a second administrative archive or
      journal PR without renewed authorization.

Completion: repository, GitHub policy, PR, Issue, exact `main` CI, and local
Trellis archive state are all verified; the final report explicitly says the
archive diff was not published to remote `main`; no required work remains.

## Rollback points

- Content before merge: task branch/PR only; revise or close.
- Branch protection: restore the normalized pre-change payload only while a
  fresh GET still equals this task's expected post-state; otherwise stop rather
  than overwriting concurrent maintenance.
- Content after merge: normal revert PR; never force-push `main`.
- Secret finding: stop publication, report only category/path privately, and
  wait for explicit credential-rotation/removal authority.
