# Design

Keep executable checkers:

- `scripts/tasks/docs-contract-check.mjs`
- `scripts/tasks/task-docs.mjs check`

Delete the Vitest layer that re-reads the same Markdown. Update `release-check.mjs` CI-safe list to drop `currentDocsContract.test.ts`. Update specs that name that file as required (`task-runner-contract.md`, `github-ci-workflow.md`) to name the checker instead.

`taskDocs.test.ts` keeps parser fixture tests of the checker; drop the live byte-compare `it`.

`.gitignore`: `.tmp/`.
