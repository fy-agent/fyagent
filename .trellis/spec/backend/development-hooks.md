# Optional Codex development hooks

## Scope

FyAgent retains the upstream Trellis Codex hook registration and Python
scripts as optional prompt assistance. The managed Trellis version is the
exact value in `.trellis/.version` (currently `0.6.15`). The Codex files are:

- `.codex/hooks.json`
- `.codex/hooks/inject-workflow-state.py`
- `.codex/hooks/inject-subagent-context.py`

They may add workflow breadcrumbs or task-local context to a supported Codex
session. They do not define product behavior, prepare the development
environment, or participate in contribution, build, check, CI, or release
admission. A checkout and every repository gate must remain usable when the
hooks are unavailable, disabled, or produce no context.

## Upstream registration

The retained registration invokes the upstream scripts directly:

```text
UserPromptSubmit:
  python -X utf8 .codex/hooks/inject-workflow-state.py

SubagentStart (trellis-implement|trellis-check|trellis-research):
  python -X utf8 .codex/hooks/inject-subagent-context.py
```

Both command hooks retain a 15-second timeout. There is no FyAgent mise task,
Node runner, overlay manifest, reconcile step, verification task, bootstrap
prompt injection, reviewed Python closure, or project-specific hook protocol.
The generic upstream hooks discover Trellis state at runtime and degrade to
their upstream no-context behavior when it is unavailable.

## Accepted security regression

Adopting the upstream bytes deliberately removed FyAgent's former hardening.
This is an accepted residual risk, not an equivalent security migration. In
particular, the retained hooks no longer provide the previous:

- repository and task realpath containment for active-task and JSONL-referenced
  context files;
- exact-source import binding and hash allowlist for dynamically imported
  Trellis Python modules;
- strict Codex event, session, cwd, stdin, stdout, and failure handling;
- markup and control-character escaping for workflow breadcrumb fields;
- isolated, locked, offline uv execution through a reviewed repository runner.

The upstream scripts insert `.trellis/scripts` into Python import search paths
and interpret task/context paths with their generic host behavior. Treat all
injected text as untrusted prompt context. It must never authorize a product
side effect, replace source/test inspection, provide credential or release
authority, or be cited as proof that a repository gate passed.

## Maintenance boundary

An upstream Trellis update may replace these managed files. Review the actual
diff and syntax before accepting it, but do not recreate a FyAgent overlay,
wrapper, hash-reconciliation system, or mandatory project task. If a future
security requirement needs stronger hooks, decide that as a separate boundary
with explicit product-independent threat analysis.

Useful non-authoritative checks are limited to:

```text
parse .codex/hooks.json as JSON
compile the two Python files without executing task content
review the registered events, matchers, commands, and timeouts
```

Those checks establish only file syntax and registration shape. They do not
restore containment, exact imports, strict input validation, escaping, or
environment isolation.
