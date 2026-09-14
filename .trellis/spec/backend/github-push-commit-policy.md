# GitHub Branch-Push Commit Policy

## 1. Scope / Trigger

Read this contract before changing `.github/workflows/commit-convention-push.yml`
or the push-specific range passed to
`scripts/ci/verify-commit-messages.mjs`.

This is a lightweight branch-push commit-message policy. It is not a second
product CI authority, branch synchronization workflow, or merge-readiness
signal. [GitHub CI Workflow](./github-ci-workflow.md) owns PR/merge-group
classification and the stable `CI / Required` result; [GitHub Merge
Governance](./github-merge-governance.md) owns admission to `main`.

An abnormal history rewrite can leave `github.event.before` unreachable even
after checkout with full history because no ref points at the former tip. This
owner defines the defensive empty-comparison behavior without widening push
automation.

## 2. Signatures

```text
push workflow:
  resolve base_sha / head_sha
  -> node scripts/ci/verify-commit-messages.mjs
       --base <40-hex commit> --head <40-hex commit>

listCommitSubjectsInRange(base, head)
  -> [{ sha, parents: string[], subject }]
     # Git format: %H, %P, %s
```

Push trigger excludes `gh-readonly-queue/**` and emits only:

```text
Commit Convention / Push
```

It never emits `CI / Required` or invokes the domain classifier.

## 3. Contracts

- If push `before` is forty zeroes, set `base_sha=head_sha`.
- If `${base_sha}^{commit}` does not exist in the checked-out clone, set
  `base_sha=head_sha`. Log the defensive fallback without treating the
  unreachable object as a product-CI failure.
- The fallback validates the current head subject through an empty comparison;
  it does not fetch arbitrary history, run product domains, or repair/sync the
  branch.
- PR and merge-group missing SHAs remain fail-closed in Required CI. This push
  fallback must not leak into `.github/workflows/ci.yml`.
- Normal Conventional Commit types and PR-title rules remain unchanged.
- The explicit `merge: <nonempty description>` integration form is accepted
  only when the corresponding Git object has at least two distinct parents.
  It is not a general normal type, subject-only exemption, PR-title allowance,
  or hash allowlist.
- The verifier enumerates the complete side-branch commit range. A first-parent
  or no-merges filter must not hide invalid merged commits.
- Generated GitHub merge subjects and revert subjects remain separate existing
  rules. New integrations may use an ordinary valid conventional subject.

## 4. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| Push `before` is forty zeroes | Use head-to-head empty comparison; validate current head. |
| Push `before` is a 40-hex SHA but not a commit in clone | Log fallback, use head-to-head; no domain CI. |
| Normal reachable push range | Validate every commit in explicit base..head range. |
| PR/merge-group base/head SHA missing | Required CI classifier fails; no push fallback. |
| Multi-parent object has valid explicit integration subject | Accept topology-specific form. |
| Same subject is on single-parent commit or PR title | Reject. |
| Integration subject is empty/nonstandard | Reject. |
| Side-branch commit has invalid subject | Reject even when merge commit itself is valid. |
| Queue-ref push triggers this workflow | Contract regression; queue uses merge-group Required CI only. |
| Push workflow starts product domains or emits `CI / Required` | Contract regression. |

## 5. Good / Base / Bad Cases

- **Good:** ordinary branch push has a reachable base; only the pushed range is
  checked for Conventional Commit subjects.
- **Base:** force-update removes the previous tip; workflow confirms `before`
  is not a commit, compares head to itself and still validates current head.
- **Base:** a genuine two-parent historical integration uses the narrow
  explicit integration form while every merged side commit remains checked.
- **Bad:** use branch push as a second Required CI authority, start product
  jobs to enforce subject style, or fetch/mutate refs to manufacture a range.
- **Bad:** add `merge` to all normal types, accept it by subject alone, skip
  side-branch history, or disable convention checks for one old integration.

## 6. Tests Required

- `tests/githubWorkflowTriggers.test.ts` asserts push-only
  `git cat-file -e "${base_sha}^{commit}"`, head-to-head fallback,
  queue-ref exclusion and the absence of `CI / Required`/domain jobs.
- `tests/verifyCommitMessages.test.ts` builds temporary Git histories for real
  multi-parent integration objects, head-only comparison, single-parent
  impostors, PR titles, empty/nonstandard subjects and invalid side commits.
- Reachable-range fixtures prove every commit is enumerated without
  first-parent/no-merges suppression.
- Required CI tests independently prove PR/merge-group identity remains strict
  and cannot use this fallback.
- Local tests do not claim to reproduce GitHub's unreachable object storage;
  they prove the workflow command and verifier semantics separately.

## 7. Wrong vs Correct

### Wrong

```bash
node scripts/ci/verify-commit-messages.mjs \
  --base "$PUSH_BASE_SHA" --head "$head_sha"
# fatal: before SHA does not identify a commit object
```

```text
push -> classify domains -> CI / Required
```

### Correct

```bash
base_sha="$PUSH_BASE_SHA"
if ! git cat-file -e "${base_sha}^{commit}" 2>/dev/null; then
  base_sha="$head_sha"
fi
node scripts/ci/verify-commit-messages.mjs \
  --base "$base_sha" --head "$head_sha"
```

```text
push -> Commit Convention / Push only
PR/merge_group -> explicit base/head -> CI / Required
```
