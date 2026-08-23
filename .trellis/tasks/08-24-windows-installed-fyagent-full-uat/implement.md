# Windows UAT Execution Plan

## Closure Checklist

- [ ] Fetch and check out this branch in a dedicated Windows worktree without disturbing existing changes.
- [ ] Read `AGENTS.md`, `.trellis/workflow.md`, this task, and the specs named in the JSONL manifests.
- [ ] Post and commit the machine receipt; claim the task with actor/time/progress metadata.
- [ ] Freeze installed Windows FyAgent and repository baselines.
- [ ] Build the complete runtime surface, DPI/window, Agent-tool, functional-layer, and failure-path matrices.
- [ ] Create private raw evidence and a verified isolated profile/rollback path.
- [ ] Execute full-page visual and functional UAT, including both macOS-origin P1 hypotheses as Windows retests.
- [ ] Produce sanitized report, evidence index, issue register, verdict, gaps, owners, and retest plan.
- [ ] Run fresh validations, review the exact staged files, commit, push, create PR, and report CI/blockers.

## Ordered Execution

1. Preflight local repo, installed app, process, evidence-capture capability, display/DPI controls, profile-copy path, and required permissions before deep testing.
2. Claim the task and create the machine receipt artifact. Commit/push this first milestone so the Mac supervisor can verify real machine acceptance.
3. Reconcile code-declared routes/contracts with actual runtime pages and dialogs.
4. Back up and copy the FyAgent profile; verify fingerprints and rollback before any write test.
5. Traverse every page and state at 100/125/150% DPI, collecting screenshots and structured observations only after each tested surface is stable.
6. Exercise safe functional and negative paths; immediately record the strongest C/R/P/A layer and authoritative readback when available.
7. Inventory actual Windows Agent tools and execute the two isolated P1 retests.
8. Reconcile all evidence into the report, evidence index, issue register, release verdict, and retest conditions.
9. Run task validation, Markdown/link checks, secret/private-path scan, `git diff --check`, and exact staged-file review. Add relevant static/unit checks only when they support the report; do not present them as runtime UAT.
10. Commit/push the final artifacts and create a PR to `main` requesting 赖永杰 review and merge.

## Stop Conditions

Continue until closure. Stop only for a user decision, permission/safety boundary, or fresh environment failure that cannot be resolved safely. Report the exact blocker, commands/evidence already tried, affected acceptance criteria, and the smallest user action needed. Do not stop at planning, installation, static review, or a stage summary.
