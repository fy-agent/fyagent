# Local commit and archive plan

Authorization: the user's subsequent request to resolve all remaining work
approves the local work commit and archive, including the reported baseline
contract and test-warning repairs. No remote publication is authorized.

## Work commit

Proposed message:

```text
refactor(ui): simplify secondary pages and close validation gaps
```

Include the original 41 tracked UI/SPEC/test paths below, the fourteen explicitly
listed completion-repair paths (including performance and native-test isolation),
and this task's planning/research/review artifacts.
The previous UI modifications were reviewed as this same task; the completion
repairs are described separately in research/validation-repairs.md. No unrelated
dirty paths were found. Recheck before staging and stop for newly appearing,
unrecognized changes.

```text
.trellis/spec/frontend/mcp.md
.trellis/spec/frontend/prompts-memory.md
.trellis/spec/frontend/skills.md
.trellis/spec/frontend/surfaces-responsive.md
.trellis/spec/frontend/user-facing-copy.md
.trellis/spec/frontend/visual-language.md
src/app/styles/features.css
src/app/styles/tokens.css
src/pages/agents/AgentAssignmentSections.tsx
src/pages/agents/AgentAuthStatusPanel.tsx
src/pages/agents/AgentPromptsSection.tsx
src/pages/agents/Page.tsx
src/pages/auth/AccountView.tsx
src/pages/auth/CodexRequestSource.tsx
src/pages/auth/ConnectionsView.tsx
src/pages/auth/LoginDialog.tsx
src/pages/auth/Page.tsx
src/pages/auth/page.css
src/pages/health/Page.tsx
src/pages/health/page.css
src/pages/mcp/Page.tsx
src/pages/memory/Page.tsx
src/pages/memory/page.css
src/pages/models/ModelConnectivityTest.tsx
src/pages/models/OpenCodeModelsPanel.tsx
src/pages/models/Page.tsx
src/pages/models/QoderModelsPanel.tsx
src/pages/models/TraeModelsPanel.tsx
src/pages/models/XaiSubscriptionSection.tsx
src/pages/prompts/Page.tsx
src/pages/skills/Page.tsx
src/shared/ui/WorkBuddyTrustDialog.tsx
tests/browser/auth.spec.ts
tests/browser/responsive-density.spec.ts
tests/browser/scroll-ownership.spec.ts
tests/renderer/app/userFacingCopy.test.ts
tests/renderer/features/featurePages.test.tsx
tests/renderer/pages/auth/Page.test.tsx
tests/renderer/pages/health/Page.test.tsx
tests/renderer/pages/memory/Page.test.tsx
tests/renderer/pages/models/Page.test.tsx
.trellis/tasks/09-14-concise-renderer/
```

Completion-repair additions:

```text
.trellis/spec/backend/claude-code-cli.md
.trellis/spec/backend/task-runner-contract.md
.trellis/spec/frontend/quality-guidelines.md
scripts/tasks/supported-platform-structure-assets.json
src-tauri/src/services/tooling/grok_npm.rs
src-tauri/src/services/tooling/versions.rs
tests/codexWindowsUserScopeContract.test.ts
tests/renderer/app/setup.ts
tests/renderer/app/actWarningGuard.test.ts
tests/renderer/pages/agents/Page.test.tsx
config/playwright.performance.config.ts
tests/architecture/rootGovernance.test.ts
.trellis/spec/backend/proxy-runtime.md
src-tauri/src/services/provider/mod.rs
```

Do not include generated logs, screenshots, `node_modules`, dependency/API/data
changes or unrelated work. The two mechanical native changes above are the
entire native behavior-preserving compile patch. The final native gate also
required a test-only correction in provider/mod.rs: use the existing ephemeral
listener configuration, assert the returned port, and stop that test listener.
No production Provider behavior is modified.

## Bookkeeping after the work commit

Use the existing `fyagent-concise-renderer-20260914` context identity. Preserve
work-commit → archive-commit → journal-commit order; no amend and no push.
The installed archive command moves the directory but does not rewrite its
JSONL references, so use its supported `--no-commit` option, repair only this
task's relocated context paths, stage the move, and run canonical postarchive
checks without an exclusion before making the archive commit. Record the work
commit in task metadata and pass only work-commit hashes to the journal command.

The previous proposal to archive with known failing aggregates is superseded.
Only actual final gate results in verification.md authorize completion; native
Windows/live-account/installer/signing evidence is still not inferred from
portable or host-only tests.
