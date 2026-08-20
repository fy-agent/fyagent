# GitHub governance research

## Current verified state

Verified on 2026-08-13 against `fy-agent/fyagent`:

- canonical repository: public organization repository, not a fork;
- default branch: `main`;
- allowed merge methods: merge commit, rebase, and squash;
- automatic source-branch deletion: disabled;
- rulesets endpoint: no repository rulesets;
- branch protection on `main`:
  - pull request required;
  - administrators are included;
  - zero approving reviews;
  - Code Owner review disabled;
  - last-push approval disabled;
  - no required status checks;
  - force pushes disabled;
  - deletion disabled;
  - linear history, conversation resolution, branch lock, and fork syncing
    disabled.

The baseline `main` SHA has a successful GitHub Actions check named exactly
`CI / Required`, owned by GitHub Actions app ID `15368`. The workflow and
`.trellis/spec/backend/github-ci-workflow.md` identify that exact name as the
only stable aggregate.

## User decision

The user approved the minimum live reconciliation:

- require only `CI / Required` on `main`;
- keep the required approval count at zero;
- do not require Code Owner review;
- preserve required PRs, administrator enforcement, and the force-push/deletion
  prohibitions;
- make no unrelated protection changes.

Use `strict: false` for the new required check. This is the minimum behavior
change and fastest merge path: the exact PR head must have a successful
aggregate, but the task does not introduce a separate “branch must be up to
date” policy that the user did not request.

## Safe optimistic update sequence

1. Create and push the focused branch; open a non-draft PR to `main` with
   `Closes #21`.
2. Wait for a successful `CI / Required` on the exact PR head before changing
   protection. This proves the context currently exists and avoids configuring
   a misspelled or absent gate.
3. Immediately refetch the full protection document, normalize the response
   into the endpoint's writable request schema, and compare every governed
   value with the planning snapshot. Stop on drift rather than overwriting an
   intervening maintainer change.
4. Save a non-secret before snapshot in task/PR evidence. GitHub does not
   document conditional unsafe requests for this endpoint, so GET -> PUT is an
   optimistic window, not atomic CAS.
5. Use GitHub's documented branch-protection endpoint with a full explicit
   payload because required-status-check protection is currently absent. Add
   only `required_status_checks = { strict: false, checks: [{ context:
   "CI / Required", app_id: 15368 }] }`; reproduce every other live writable
   value unchanged.
6. Refetch protection and assert the exact required context/provider plus all
   preserved settings. Recheck the PR merge state; do not merge while GitHub
   reports an unknown, blocked, or stale state.
7. Squash merge, read the PR's `merge_commit_sha`, verify it remains reachable
   from remote `main`, verify Issue #21 closes, and wait for that SHA's latest
   `.github/workflows/ci.yml` event=`push` attempt and GitHub Actions-owned
   `CI / Required` to succeed.

GitHub's current official REST documentation exposes `PATCH` for existing
required-status-check protection and full `PUT` branch protection updates. The
full update requires administration permission and returns validation errors
for invalid settings. The implementation should set the API version header
through `gh api` and inspect the response instead of trusting process exit
alone.

## Rollback

If the update response or post-update verification differs from the approved
shape, fetch live state again. Restore the normalized pre-update payload with
`required_status_checks: null` only if that fresh state exactly equals this
task's expected post-update state. If it does not, a concurrent actor may have
changed protection; do not perform a second overwrite. Stop for human
reconciliation. Verify any permitted rollback through a fresh GET. Do not
proceed to merge until either the approved state or the verified original state
is established.

Repository content rollback is a normal PR revert after merge. Never force-push
or delete `main`; do not use branch protection disablement as a merge shortcut.
Final Trellis archive/journal changes are local-only with `--no-commit` and are
not a second public governance PR without renewed user authorization.
