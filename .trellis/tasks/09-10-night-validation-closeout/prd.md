# Nightly validation and integration preparation

The user requested the current local branch state, completion of tonight's
functional testing and merge-conflict preparation, and a separate coordination
session for currently actionable work. The active product delivery is #52 / PR
#187, based on main `11c13339`, with published head `398628bc`.

## Acceptance

- Reproduce and repair the three post-archive contract failures in PR #187.
- Run complete local checks and production-boot/browser functional coverage on
  the final repair. Keep native, hosted CI and supplemental performance evidence
  distinct; the two previously measured animation failures remain explicit.
- Refresh Git references, classify local branches, and perform a non-destructive
  merge-tree check for the current delivery against main. Preserve every existing
  worktree and avoid reviving superseded branches.
- Deliver the narrow repair to the existing working PR and independently read
  back its head/checks. Do not merge main, release, or close issues.
- Prepare a coordination task with dependencies, existing thread references,
  executable next steps and actual session-creation status. If the current tool
  surface cannot create a separate Codex session, record that blocker accurately.

## Scope

Correct stale reviewed-raster counts and concrete workstation paths in #52's
archived metadata. Preserve the strict inventory, privacy rules and budgets.
No product behavior, runtime credential, user configuration or dependency change.
