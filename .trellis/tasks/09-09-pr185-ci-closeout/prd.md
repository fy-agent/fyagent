# Repair PR 185 CI and close out Grok task records

## Goal

Restore the failing WebKit contrast contract without weakening gates, reconcile superseded task records, and deliver a replacement pull request to main.

## Requirements

- Preserve PR #185's managed Grok subscription behavior and original commit provenance.
- Repair the failed WebKit Agents-page readability check without lowering contrast thresholds, skipping tests, changing CI admission, or adding dependencies.
- Fix the Agent scan teardown/StrictMode races exposed by full frontend verification; do not suppress unhandled errors or cancel native jobs on route teardown.
- Reconcile the three historical child plans with the already-archived September 8 parent revision. Preserve historical acceptance criteria and explicitly distinguish superseded scope from verified implementation.
- Update the owning code specs before task archival and validate effective task context references afterwards.
- Deliver a replacement PR through normal required checks into main; only then close #185 and clean up demonstrably unused branches and this session's worktree.
- Preserve main, dev/laiyongjie, branches related to open PRs, concurrent work, and active repository infrastructure.

## Acceptance Criteria

- [ ] The original failure is evidenced and an unchanged-threshold regression passes in Chromium and WebKit.
- [ ] Frontend and repository contract checks pass; the replacement PR's required CI passes without overrides.
- [ ] Current specs describe the corrected behavior; all four original Grok task records have valid metadata, relationships and explicit dispositions.
- [ ] The closeout task is verified, committed and archived using the existing Trellis lifecycle.
- [ ] GitHub confirms the replacement PR is merged into main and #185 is closed with a replacement reference.
- [ ] Cleanup is verified against fresh PR/ref/worktree state, without deleting concurrent modifications or breaking active repository assets.

## Scope limits

- No changes to authentication protocols, paid-account entitlement behavior, WorkBuddy subscription scope, dependencies, release versions or required CI policy.
- Automated fixtures are not evidence of real Grok subscription consumption or native installed-CLI end-to-end acceptance.
- The star-history branch serves README assets and a scheduled workflow; absence of an open PR alone does not make it unused.
