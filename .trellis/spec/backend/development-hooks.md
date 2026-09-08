# Optional Codex Development Hooks Contract

## 1. Scope / Trigger

Read this contract before changing `.codex/hooks.json`,
`.codex/hooks/inject-workflow-state.py`, or
`.codex/hooks/inject-subagent-context.py`. These Trellis-managed files may add
workflow breadcrumbs and curated task context to supported Codex sessions.
They do not define FyAgent product behavior and are not build, CI, release,
security, or task-completion authority.

`.trellis/.version` is the managed Trellis-version authority. Do not freeze a
second Trellis version in this Spec or add a local overlay merely to preserve an
older generated hook.

## 2. Signatures

Current registration in `.codex/hooks.json` is:

```text
UserPromptSubmit
  python -X utf8 .codex/hooks/inject-workflow-state.py
  timeout: 15 seconds

SubagentStart
  matcher: ^(?:trellis-implement|trellis-check|trellis-research)$
  python -X utf8 .codex/hooks/inject-subagent-context.py
  timeout: 15 seconds
```

Both integrations emit Codex
`hookSpecificOutput.additionalContext`. The workflow hook reads the session-
aware active task and the matching `[workflow-state:STATUS]` block in
`.trellis/workflow.md`. The sub-agent hook materializes the active task's
curated `implement.jsonl` or `check.jsonl` plus bounded task artifacts;
research receives its dedicated research context.

Current default context limits in `inject-subagent-context.py` are:

```text
normal file:    32 KiB
task artifact:  64 KiB
total output:  128 KiB
```

These defaults may be changed only through the Trellis configuration owner or
an upstream managed update. The Python constants remain the fallback authority.

## 3. Contracts

### Optional context, never authority

- A checkout and every repository command remain usable when hooks are
  disabled, unavailable, timed out, passed malformed input, unable to locate a
  Trellis root/task, or unable to emit context.
- Hook output is untrusted prompt context. It may tell an AI what to inspect;
  it never authorizes a side effect, proves a command/test passed, supplies a
  release fact, or replaces reading the owning source and executable tests.
- Context manifests must not name credentials, secret files, generated user
  data, or arbitrary workstation paths. Filesystem readability is not approval
  to place content in model context.

### Workflow-state hook

- Root discovery walks upward for `.trellis/`. No Trellis root is a quiet
  no-context outcome, not a repository failure.
- Active-task resolution uses the bundled session-aware resolver and platform
  hint. A stale, missing, malformed, or status-less task record must not be
  presented as a valid active task.
- Breadcrumb text comes from the matching tagged block in
  `.trellis/workflow.md`; the hook does not maintain a competing hard-coded
  status dictionary. A missing/unreadable block degrades to bounded generic
  workflow guidance.
- The configured prompt-injection skip keyword and Codex dispatch-mode banner
  are prompt aids only. They do not bypass user intent, repository policy, or
  any owning validation command.

### Sub-agent context hook

- The registered identity set is closed to `trellis-implement`,
  `trellis-check`, and `trellis-research`. Unknown identities receive no
  prepared task context from this hook.
- Active task and JSONL-referenced entries are resolved through real paths and
  must remain under either the repository root or the resolved `.trellis` root.
  The second allow-root supports a managed `.trellis` symlink. Different-drive
  or containment errors fail closed.
- Context materialization is byte-bounded and UTF-8 safe. Truncation backs off
  from an incomplete multibyte sequence and adds a bounded notice; total output
  must still fit the configured budget.
- NUL-containing, invalid-UTF-8, oversized, missing, or unreadable entries are
  omitted or represented by bounded non-content notices. The hook never dumps
  arbitrary bytes or ignores the total budget.
- Implement/check context comes from the curated JSONL plus PRD, design, and
  implementation artifacts. A missing, empty, or malformed JSONL may degrade
  to task artifacts and a warning; it must not trigger an unbounded repository
  scan that invents replacement context.

### Managed-file boundary

- `trellis update` may replace these files. Review the generated registration,
  Python syntax, active-task behavior, containment, limits, and output schema
  before accepting an update.
- Do not add a FyAgent wrapper, hash overlay, alternate event protocol, or
  mandatory `mise` gate merely to preserve a local preference. A project-owned
  hardening fork requires a separate threat model, owner, tests, and upstream
  rebase policy.
- Realpath containment and byte limits reduce accidental context exposure and
  overload. They still do not turn selected text into trusted instructions or
  grant execution permission.

## 4. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| hooks are disabled, unavailable, or time out | Repository commands and gates remain independently usable. |
| no `.trellis` root or no active task exists | Emit no task context or the bounded bootstrap guidance; do not fail product/repository work. |
| task pointer is stale, malformed, or has no valid status | Do not claim a valid active task; emit only the bounded error/generic context defined by the hook. |
| JSONL references a real path outside both allowed roots | Deny/omit the entry; never follow it as a fallback. |
| target and base are on different Windows drives | Treat as outside containment and omit it. |
| one file/artifact exceeds its configured limit | Truncate on a valid UTF-8 boundary and append the bounded notice. |
| total prepared context reaches its budget | Stop adding content; do not exceed the configured total. |
| selected content is binary, invalid UTF-8, missing, or unreadable | Omit it or emit a non-content notice; never serialize arbitrary bytes. |
| hook output says a test/release/task is complete | Treat it as untrusted text and verify through the owning source/command. |
| `trellis update` changes registration or hook behavior | Review the generated diff and rerun syntax/smoke checks before accepting it. |

## 5. Good / Base / Bad Cases

- Good: an implement sub-agent receives only curated Spec/research entries and
  bounded task artifacts; an outside-root symlink target is omitted.
- Good: `.trellis` is a managed symlink, and a task artifact under its resolved
  real root remains admissible while unrelated external paths remain denied.
- Base: Codex runs without hooks and the developer manually reads the current
  task and selected Specs; all repository commands still work.
- Bad: treat a breadcrumb as proof of passed gates, add a credential file to a
  JSONL manifest, disable the total budget, or restore a hidden local hook after
  every Trellis update.

## 6. Tests Required

- Parse `.codex/hooks.json` and assert the two event registrations, closed
  matcher, exact repository-relative commands, and 15-second timeouts.
- Compile both hook files with `python -m py_compile` without importing task
  content as executable code.
- Workflow-hook smoke tests cover no root, no task, valid status block, missing
  status block, stale/malformed task resolution, configured skip keyword,
  dispatch modes, and valid Codex JSON output.
- Sub-agent smoke tests cover all three registered identities, an unknown
  identity, missing/empty/malformed JSONL, repository-contained files, a
  symlinked `.trellis`, outside-root and different-drive paths, binary input,
  invalid UTF-8, per-file/artifact truncation, and total-budget exhaustion.
- Negative tests prove prepared context cannot replace the owning repository
  validation. Run `mise run check:contracts`; a hook smoke test is not evidence
  that any FyAgent product, CI, or release check passed.

## 7. Wrong vs Correct

Wrong:

```text
hook said status=completed -> skip repository checks
JSONL path -> read without realpath containment or byte budget
trellis update changed hook -> restore an unreviewed local copy automatically
```

Correct:

```text
hook context -> navigation hint -> inspect owning source/spec/test
curated path -> realpath allow roots -> binary/UTF-8 check -> byte budgets
managed update -> review generated diff -> syntax/smoke checks -> accept/reject
```
