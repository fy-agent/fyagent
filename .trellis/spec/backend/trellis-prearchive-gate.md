# Trellis Direct-Session Prearchive Gate

## 1. Scope / Trigger

Read this contract before changing `check:prearchive`,
`check:contracts:prearchive`, `scripts/tasks/prearchive-check.mjs`, or the
private active-task exclusion accepted by the supported-platform checker.

This gate exists only for the narrow interval in which one directly active,
in-progress Trellis task still contains tracked planning markers that will be
moved by `task.py archive`. It does not define the ordinary `check`,
`check:contracts`, CI, or post-archive behavior. The public task surface and
argument transport are owned by
[Repository Task Runner](./task-runner-contract.md); the downstream inventory
checker is owned by
[Supported-Platform Governance](./supported-platform-governance.md).

## 2. Signatures

```text
mise run check:prearchive --exclude-active-task <path>
mise run check:contracts:prearchive --exclude-active-task <path>

scripts/tasks/prearchive-check.mjs
  mode = check | check:contracts
  excludeActiveTask = repository-relative canonical task path

FYAGENT_SUPPORTED_PLATFORM_ACTIVE_TASK=<validated path>
  # private child-process transport only
```

Accepted path shape:

```text
.trellis/tasks/MM-DD-<id>
```

The leaf accepts exactly one input channel: direct CLI, mise usage, or the
private environment variable. It never infers an active task from a glob or
from repository state alone.

## 3. Contracts

- The wrapper accepts one repository-relative direct child below the canonical
  `.trellis/tasks` root. It rejects parent traversal, backslashes, nesting,
  archive paths, wildcards, symlinks, non-direct realpaths, non-directories,
  and missing/symlinked/non-regular `task.json` files.
- `<id>` is derived from the canonical directory name. `task.json.id` and
  `task.json.name` must both equal it, and `status` must be `in_progress`.
- The same canonical path must be Trellis's current pointer with
  `stale=false` and a direct `source="session:<id>"`. A fallback pointer,
  another task, or a stale record cannot authorize exclusion.
- Only after all identity checks pass may the wrapper transport the path
  through `FYAGENT_SUPPORTED_PLATFORM_ACTIVE_TASK` to the one checker leaf.
  Callers may not preseed it, combine it with a CLI/usage path, or make the
  variable a CI input.
- The wrapper selects only `check` or `check:contracts`; it never forwards the
  exclusion to unrelated leaves or retries a failed check without it.
- Canonical `check`, `check:contracts`, `supported-platform:check`, CI, and
  post-archive validation always run without an exclusion.
- The exclusion is identity-specific, not semantic. It suppresses only the
  directly active task directory during its own archive transition; all other
  repository files and task records remain scanned.

## 4. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| `--exclude-active-task` missing or duplicated | Fail before nested check. |
| Wrapper mode is not `check` or `check:contracts` | Fail closed. |
| Path is absolute, nested, archived, traversing, wildcarded or contains backslashes | Reject before filesystem scan. |
| Task directory or `task.json` is a symlink/non-regular escape | Reject before nested check. |
| Directory-derived ID differs from `task.json.id` or `.name` | Reject. |
| Task status is not `in_progress` | Reject. |
| Current pointer is missing, stale, fallback, another task or another session | Reject. |
| Caller preseeds the private env variable or supplies more than one channel | Reject; do not choose a winner. |
| Nested check exits nonzero | Propagate the failure; never retry broader or omit the platform check. |
| Ordinary check/CI attempts an active-task exclusion | Contract regression; canonical scan must remain unfiltered. |

## 5. Good / Base / Bad Cases

- **Good:** two differently named fixture tasks each validate when that exact
  path is the direct current in-progress task for its own session.
- **Base:** canonical `mise run check` executes with no private entry and scans
  all active/archive paths normally.
- **Base:** after `task.py archive`, post-archive contracts run without an
  exclusion and validate the moved task in its durable location.
- **Bad:** hard-code a historical task ID, accept `session-fallback`, skip every
  `.trellis/tasks/**` path, or let CI provide the private variable.
- **Bad:** when prearchive fails, rerun canonical checks while silently omitting
  the offending task.

## 6. Tests Required

- Pure path tests cover two valid task identities plus malformed date/ID,
  traversal, backslash, archive, nesting, wildcard, directory symlink,
  `task.json` symlink and realpath escape.
- Metadata tests cover `id`, `name`, `status`, missing and non-regular
  `task.json` failures.
- Session tests cover direct ownership, stale pointers, fallback pointers,
  wrong task and wrong session.
- Input-channel tests reject caller-preseeded private state and every
  CLI/usage/environment duplication.
- Integration evidence records one real prearchive composite from the directly
  bound session, archives the task, then runs the canonical post-archive gate
  without an exclusion.
- CI/workflow tests prove the private environment entry is never supplied by
  hosted automation.

## 7. Wrong vs Correct

### Wrong

```text
check:prearchive -> skip .trellis/tasks/**
check failed -> rerun check without supported-platform scanner
CI -> FYAGENT_SUPPORTED_PLATFORM_ACTIVE_TASK=<branch task>
```

### Correct

```text
derive one .trellis/tasks/MM-DD-id path
  -> prove realpath/file type
  -> prove task.json id/name/status
  -> prove direct current session ownership
  -> privately pass to exactly one supported-platform leaf
  -> archive
  -> run canonical post-archive checks with no exclusion
```
