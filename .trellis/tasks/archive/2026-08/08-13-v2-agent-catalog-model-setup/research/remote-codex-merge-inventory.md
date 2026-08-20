# Remote Codex merge inventory

## Pinned scope

The user clarified that this delivery must merge every remote `codex/*` branch
visible **now** into the local `dev/laiyongjie` branch. The scope was refreshed
with `git fetch origin --no-tags` and pinned at
`2026-08-13T16:36:31.6712368+08:00`:

| Remote ref | Pinned commit | Tip subject |
| --- | --- | --- |
| `origin/codex/github-brand-community-polish` | `fbb0b339640c213f39dba92b6827ed87eff4361a` | `chore: record journal` |
| `origin/codex/issue-21-repository-governance` | `43145063d8003b636030a5bf5c2191020cc27944` | `fix(ci): classify audit scripts` |
| `origin/codex/prompt-memory-frontend-refactor` | `afc317a7be3883a4a438d48fad5fb8b6d1f1f5ab` | `fix(dev): restore mise workflow closure` |
| `origin/codex/restore-for-you-agent` | `3bd86e50d901300c76283754003a29dfa9868911` | `docs: restore For You Agent identity` |
| `origin/codex/前端设计` | `fd54598ff16cbf4628495e0d7fadad53d4c168cc` | `docs: archive FyAgent control plane prototypes` |

Later creation or movement of a remote branch is outside this fixed snapshot
unless the user explicitly expands the scope again. Acceptance uses the pinned
commit IDs, not the potentially moving remote-tracking names:

```powershell
git merge-base --is-ancestor <pinned-commit> HEAD
```

All five checks must exit successfully after integration.

## Pre-merge topology audit

The following read-only audit was refreshed against local feature-baseline
`b780b9ae426be9152c27c56e009b46d0262748da`. Counts are diagnostic only; the
pinned tips above are the immutable acceptance authority.

| Pinned tip | Merge base with baseline | Unique remote commits | Ancestor before merge | Principal overlap / resolution boundary |
| --- | --- | ---: | --- | --- |
| `fbb0b339640c213f39dba92b6827ed87eff4361a` | `f424ceff8f085673d00b8fd191045cb965987408` | 3 | no | GitHub community templates and archived brand task. Its tree equals `e8e578fcfee346947546926fe406a11557f26970` (`05811745ea9a50057ddd4b17ad1511bdbdfeb609`); preserve ancestry without replaying that stale snapshot over current files. |
| `43145063d8003b636030a5bf5c2191020cc27944` | `f424ceff8f085673d00b8fd191045cb965987408` | 5 | no | Governance templates, `CODEOWNERS`, contribution/docs contracts, audit scripts, and Trellis task records. Preserve current supported-platform/security behavior when conflicts occur. |
| `afc317a7be3883a4a438d48fad5fb8b6d1f1f5ab` | `e33d37dd6f9d58c11207f843b5c33750a79dbb4a` | 19 | no | V2 shell/navigation/styles/tests, Prompt/Memory pages, frontend specs, generated preview, and task records. Integrate Prompt/Memory while retaining this task's Agent/Models ports, pages, security, and tests. |
| `3bd86e50d901300c76283754003a29dfa9868911` | `f424ceff8f085673d00b8fd191045cb965987408` | 2 | no | For You Agent identity/community history. Its tree equals `8c8ca4c2eea69889cbdf53d9c983218806e93a4e` (`0a279a20383e9cf9c3ddcf271b2c02f8cdbfc4e6`); preserve ancestry without restoring the stale public snapshot over current files. |
| `fd54598ff16cbf4628495e0d7fadad53d4c168cc` | `5fbf862118874ba953b890bc5a579bf3b46a2658` | 1 | no | Historical visual-review binaries and archived task evidence. Keep it separate from canonical runtime icons and regenerate inventories from the resolved final tree. |

This table is not a completion claim. After every merge, recompute ancestry
against final `HEAD`; do not carry these pre-merge `no` values into final
evidence as though integration had already happened.

## Integration boundaries

- Commit and verify the current Agent/Models/backend/Y-icon write set before
  any merge; never merge into the dirty implementation tree.
- Preserve full ancestry. Do not rebase, squash, force-update, delete, rename,
  or push a remote branch.
- Resolve the final V2 shell as six non-empty pages: this task owns the native
  Agent/Models contracts; the prompt-memory branch owns its bounded frontend
  prototypes; Skills/MCP remain regression-safe.
- Preserve the latest Provider atomicity and credential-isolation fixes. Do not
  select an older remote side wholesale for shared shell, security, or build
  files.
- Regenerate structure/raster/generated preview outputs from the resolved final
  tree instead of selecting a stale side during conflict resolution.
- The merged history contains remote task records in `planning`, `review`, and
  `in_progress` states. Preserve their truthful historical states unless their
  own completion gates are independently satisfied; do not archive them merely
  to make the active-task list shorter.
- The already-public remote history contains absolute personal paths. Correct
  such paths in the final tree where they remain user-visible, but do not
  rewrite the pinned commits because ancestry preservation is part of this
  explicit merge request.

## Required final ancestry proof

Run these exact commands after all conflict resolution and derived-output
regeneration; every command must return exit code `0`:

```powershell
git merge-base --is-ancestor fbb0b339640c213f39dba92b6827ed87eff4361a HEAD
git merge-base --is-ancestor 43145063d8003b636030a5bf5c2191020cc27944 HEAD
git merge-base --is-ancestor afc317a7be3883a4a438d48fad5fb8b6d1f1f5ab HEAD
git merge-base --is-ancestor 3bd86e50d901300c76283754003a29dfa9868911 HEAD
git merge-base --is-ancestor fd54598ff16cbf4628495e0d7fadad53d4c168cc HEAD
```

Record the final `HEAD` and exit codes in task validation evidence. Do not mark
the merge acceptance item complete from branch names, merge messages, tree
similarity, or pre-merge checks alone.
