# 实施前继承文件保护基线

- 记录日期：2026-08-12
- 基线类型：工作树内容 Git blob hash（设计冻结前）
- 用途：这些文件是本任务现场已有且必须保留的有效成果；三个执行 Agent 与后续页面修复不得改写。最终验收重新计算并逐项比对。
- 证据等级：`code_audit`
- 本记录未运行测试、构建、服务或浏览器。

| 文件 | Git blob hash |
| --- | --- |
| `src/index.html` | `c5ff9932f285285559f2d4cb5ac20375e8973aa4` |
| `package.json` | `b81bef412f8c7de66613dfa24123f9f7dbe949f9` |
| `playwright.v2.config.ts` | `5ee748a99cf16254d1e14f57ce4955451a3544e6` |
| `src/v2/app/styles/tokens.css` | `7ec796965df53ffc4b2ef5c1a73a24cf100e1a17` |
| `src/v2/app/styles/globals.css` | `ffe6e3059e3a410ce46b6489960772ce7358a068` |
| `src/v2/app/styles/index.css` | `87b8ee6087029a10c0984c711811eaed2fb9823b` |
| `src/v2/app/styles/v4-shell.css` | `a5c31b254ccb81568b567fb778beebbdc909cef6` |
| `tests/v2/app/router-shell.test.tsx` | `abcb3d186514089d42520e33503b90c4c1e2416d` |
| `tests/v2-browser/shell.spec.ts` | `24f35fb5500907d91eb73dfec79f7de99386a9e9` |

这些 hash 只证明冻结时内容，不替代 baseline-to-HEAD、worktree、untracked 和逐提交文件清单审计。

## 最终保护复核（2026-08-13）

- `src/index.html`、`package.json`、Playwright 配置、四个 V2 style 文件和 `tests/v2/app/router-shell.test.tsx` 的最终 Git blob hash 与上表完全一致。
- `tests/v2-browser/shell.spec.ts` 最终 hash 为 `58ef77759606dd412d82d292e96f26aced43f172`。相对冻结 hash 只有 8 行新增、3 行删除：补 Memory 新合同要求的保存步骤与“同步预览任务 / 待执行 / 未写入”断言，并把页面健康监控移到 `file://` 重定向完成后。该变更属于既有 Prompt/Memory 集成用例校正，没有改变 Shell、导航或其他页面实现。
- 基线 `e33d37dd6f9d58c11207f843b5c33750a79dbb4a...HEAD` 对 `src-tauri/**`、Agent/models/skills/mcp 页面、`navigation.ts`、`router.tsx`、`widgets/app-shell/**` 和两个无关图片目录的累计 diff 均为空。
- 最终 worktree 的上述保护实现路径无修改；`docs/images/视觉-1/` 与 `docs/images/视觉/` 仍是原有未跟踪目录，未 stage、未删除、未覆盖。
- 逐提交 `git show --name-only` 已回读：Prompt、Memory、shared 三个独占提交只含各自 owner；后续提交按 builder、专属/既有集成测试、运行截图、继承 preview 配置/样式和生成 standalone 分离，没有纳入保护实现模块。
