# 校准归档后 Spec 接口事实

## Goal

复核 comprehensive-spec-refresh 归档后的新拆分文档，修正 Skills、MCP、Agent Auth、Assignment 与 Agent Directory 的真实 Port、路径、读回、敏感值和非原子写入语义，并完成全库 Spec 验证。

## Requirements

- Treat the checked-out source and tests as authority. Do not preserve a Spec
  statement merely because it sounds safer or more complete.
- Correct the focused contracts for native Skill/MCP ordering, V2 Agent Auth,
  shared Assignment behavior, Agent Directory Port paths, V2 Skills/MCP
  management behavior, and the per-product V2 Models Port/write protocols.
- Keep the information architecture and compatibility routers introduced by
  `comprehensive-spec-refresh`; this task fixes facts rather than reopening the
  full split decision.
- State current limitations explicitly, including compile-time-only simple
  Tauri adapters, query-invalidation readback, optional Skill backup results,
  raw MCP env/header values in edit/query state, and non-atomic filesystem/
  database boundaries.
- Do not change product source code, tests, dependencies, workflows, or release
  configuration.
- Preserve the unrelated archived-task checklist update already present in the
  working tree and commit it with the lifecycle correction.

## Acceptance Criteria

- [x] Every changed code-spec retains all seven mandatory sections required by
  `trellis-update-spec`.
- [x] All referenced source/test/spec paths exist, and all relative Markdown
  links resolve.
- [x] The backend Skill/MCP contracts match real command names, target IDs,
  DTOs, and mutation ordering.
- [x] The frontend Auth/Assignment/Skills/MCP contracts match real Port names,
  method return types, page ownership, query/readback behavior, and sensitive
  value exposure; Models records the real Provider, WorkBuddy, OpenCode, TRAE,
  and Change Plan boundaries rather than a fictional aggregate Port.
- [x] All spec documents are reachable from the root or layer indexes; no stale
  monolith target or duplicate semantic owner remains.
- [x] `mise run check:contracts`, focused V2 tests, `git diff --check`, and the
  exact prearchive gate pass.
- [x] Final pre-commit diff contains documentation/task artifacts only; work
  commit and independent archive are completed in the finish phase.

## Notes

- Keep `prd.md` focused on requirements, constraints, and acceptance criteria.
- Lightweight tasks can remain PRD-only.
- For complex tasks, add `design.md` for technical design and `implement.md` for execution planning before `task.py start`.
