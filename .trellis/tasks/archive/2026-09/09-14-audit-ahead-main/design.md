# Design — Ahead-of-main audit and integration

## 1. Baseline and ownership

The immutable review baseline is the six commits in
`origin/main..dev/laiyongjie` captured on 2026-09-14. The review treats them as
two coherent workstreams rather than one feature:

1. **Official npm CLI lifecycle** — live package metadata, exact-version plans,
   PATH-default discovery, npm 12 script policy and Windows ordinary-user or
   development execution.
2. **Concise Renderer surfaces** — copy/layout ownership, warning guards,
   browser scroll/density evidence and repository-wide contract snapshots.

No history rewrite is required. Corrections land as new review commits so the
PR preserves the original implementation and the audit trail.

## 2. SPEC decomposition

Trellis injects at most 32768 bytes per referenced SPEC by default. Three
changed owner files exceed that limit and therefore cannot remain the only
source for sub-agent execution:

| Current owner | Problem | Target ownership |
| --- | --- | --- |
| `external-agent-lifecycle.md` | Core inventory/jobs and product/source/desktop identity are mixed; npm detail is duplicated | Keep inventory, capability and job orchestration in the original file. Move product source resolution, shared Claude/Grok npm admission, closed desktop identity and product-specific scan material to `external-agent-sources.md`. Keep only Claude-specific owner/prefix/execution rules in `claude-code-cli.md`. |
| `task-runner-contract.md` | Public task API, prearchive validation, supported-platform identity seals and host diagnostics are one 50 KiB document | Move supported-platform source/asset inventory and whole-repository snapshot rules to `supported-platform-governance.md`; leave a concise routing section in the task-runner owner. |
| `windows-runtime-security.md` | Shell-user authority and Agent-specific helper/registry scenarios are one 50 KiB document | Keep frozen Explorer-user/path/elevation fundamentals in the original file. Move trusted Agent EXE launch, closed CLI helper and inventory-parent registry scenarios to `windows-agent-runtime-security.md`. |

The original filenames stay valid entry points. Backend `index.md` names each
new semantic owner. Cross-references replace duplicated implementation detail.
Every resulting owner must remain below the per-file injection limit.

## 3. Review model

Review proceeds in four layers:

1. **Commit intent** — compare each commit message and task artifact with its
   actual diff.
2. **Executable contract** — trace live npm/package discovery and Renderer
   state/layout behavior through implementation and tests.
3. **Repository gates** — run formatter, type/lint, Rust, unit, browser,
   performance and contract checks using repository-owned commands.
4. **Remote integration** — create/update one PR, inspect exact failing jobs,
   make scoped fixes, then merge only when GitHub reports all required checks
   successful.

## 4. Compatibility and rollback

- No serialized DTO, command name, persisted schema or product capability is
  intentionally changed by the SPEC split.
- Spec moves use ordinary Markdown files and relative links; no redirect file
  is deleted.
- If a split causes context or link validation failure, restore the moved
  section to its prior owner and narrow the split rather than increasing
  `max_file_bytes`.
- CI fixes must preserve existing performance and security budgets. A failing
  native-platform check is repaired at the owning code/test boundary, not
  bypassed in workflow configuration.

## 5. Merge shape

Use a small number of coherent commits after the six baseline commits:

1. SPEC ownership/decomposition plus any directly required link/contract tests.
2. Code or test fixes found by the audit, if any.
3. Trellis archive commit.
4. Trellis journal commit.

The PR targets `main` from the long-lived `dev/laiyongjie` branch. Merge method
and queue usage follow `github-merge-governance.md` and current repository
settings observed through GitHub CLI.
