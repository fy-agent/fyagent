# UAT Execution Plan

## Closure Checklist

- [x] Freeze installed-app and repository baselines.
- [x] Enumerate routes, secondary entries, dialogs, and state families from code and runtime.
- [x] Create a privacy-safe local evidence store and verified application-data backup.
- [x] Execute full-page visual and functional UAT, including safe negative paths and resize checks.
- [x] Produce sanitized evidence index, coverage matrix, scores, functional matrix, findings, gaps, verdict, and retest plan.
- [x] Add the `platform=macOS` declaration and Windows/AIMaster reuse handoff without performing remote Windows operations.
- [x] Validate task/artifacts, check for leaked sensitive strings or raw evidence, commit, push, and create PR.

## Ordered Execution

1. Record app metadata, signature/Gatekeeper/process state and exact `origin/main` identity.
2. Inspect V2 composition, routes, feature facades, and relevant tests/specs; generate a candidate surface/state inventory.
3. Use Computer Use to reconcile the candidate inventory with the running app.
4. Create an untracked evidence directory and checksum manifest. Back up `~/.fyagent` before any persistent-risk interaction.
5. Traverse every runtime surface. For each state, capture screenshot/accessibility evidence, score visual quality, and exercise safe controls.
6. For Prompts and Memory, use read-only/cancel/validation first; perform an isolated reversible write only if backup, target isolation, rollback, and readback are all established.
7. Reconcile evidence into the final report and issue register. Mark each claim with the strongest evidence grade actually reached.
8. Run task validation, Markdown/link checks, secret/path scan, Git diff/status review, then commit and push.
9. Create a PR to `main` and verify the remote PR metadata/readback.

## Validation

- `python3 ./.trellis/scripts/task.py validate 08-24-installed-fyagent-full-uat`
- Markdown link and required-section checks for the report/evidence index.
- Secret/private-path scan over staged files.
- `git diff --check` and exact staged-file review.
- Relevant existing unit/static checks if the UAT audit identifies executable report tooling; no unrelated full product suite is used as UAT evidence.

## Risk and Stop Conditions

- Stop only for a real permission/safety/user-decision/environment blocker that cannot be self-resolved.
- Do not install, update, merge, release, or modify product code.
- Do not keep retrying a provider/native action that could duplicate an external write.
- Do not publish raw screenshots if privacy cannot be established.
