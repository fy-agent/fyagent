# 仓库卫生：大文件、冗余契约测试与工程化拆分

## Goal

去掉文档模板契约测试屎山；安全范围内停止追踪无用文件；不为堆测试数量而保留 tautology。

## Requirements

- 删除或大幅收缩 `tests/currentDocsContract.test.ts` 对 README/spec「必须包含字符串 X」的断言。文档生成物权威留在 `docs-contract-check.mjs` / `task-docs.mjs check`。
- `tests/taskDocs.test.ts` 不再重复 byte-for-byte `mise-tasks.md` compare（checker 已做）。
- 去掉与 `currentDocsContract` 重复的 `Windows-Portable` 文档扫描（保留 `desktopSecurityBoundary` 里对代码/脚本的断言即可）。
- `.gitignore` 加入 `.tmp/`。
- 不 `git filter-repo`。不拆 `proxy.rs` 等未在本次写集中的巨石模块。
- 若某营销 sample PNG 已 `status: superseded` 且仅被文档契约钉住，可在同步更新 raster JSON / 文档索引后 `git rm`；不要误删 README 当前截图或 brand source。

## Acceptance Criteria

- [x] CI 不再靠 Markdown substring 冻结协议名/工具链版本
- [x] `mise run tasks:docs:check` 与 `docs-contract-check` 仍能抓住过期 `mise-tasks.md`
- [x] `.tmp/` 被忽略
- [x] 没有历史改写
