# Prompt / Memory 前端执行计划

## 1. 闭环清单

本轮只按以下清单推进，不用架构清理或额外页面替代闭环：

- [x] 技术设计概要与详细设计概要完成。
- [x] 产品、架构、详细设计三次静态评审通过，严重意见关闭。
- [x] 设计阶段零测试、零构建、零浏览器/截图。
- [x] 任务状态从 `review` 准确切到 `in_progress`。
- [x] Prompt、Memory、共享合同/standalone 三线路并行实施且文件互斥。
- [x] 三线路分别取得自己的模块单测新鲜通过结果。
- [x] 主 Agent 核验三线路后才第一次运行完整集成。
- [x] lint、typecheck、完整 unit、browser、renderer build 全部通过。
- [x] standalone `file://` 与 Prompt/Memory 关键交互实际通过。
- [x] 四档 viewport 无关键不可达、横向溢出或遮挡。
- [x] 两张 1586×992 新截图标为 `runtime_screenshot`。
- [x] `src-tauri`、其他四个页面、导航、Shell、无关图片目录保持保护。
- [x] PRD、design、implement、spec、verification 与代码一致。
- [x] 任务回到 `review`，不归档。
- [x] 使用多个小提交并只推送 `codex/prompt-memory-frontend-refactor`。

## 2. 阶段门禁

### Gate A：产品设计静态评审

状态：已完成初审。初审发现 1 个 P0、5 个 P1、3 个 P2；冻结设计必须逐项承接。

允许：读文档、读源码、写设计/评审文档。

禁止：lint、typecheck、unit/integration test、Playwright、build、dev server、截图、pixel diff。

### Gate B：技术架构静态评审

状态：`ARCHITECTURE_REVIEW=PASS`，P0/P1 为 0；P2/P3 已进入详细设计。

门禁：不因 PASS 误称代码缺口已修复。

### Gate C：详细设计静态评审

状态：`DETAILED_DESIGN_REVIEW=PASS`；`DESIGN_FREEZE=2026-08-12`。

产物：

- `detailed-design-overview.md`
- `execution-plan.md`
- `reviews/detailed-design-review.md`

通过条件：

1. 文件 owner 互斥。
2. 所有 P0/P1 有状态转换与单测。
3. 集成严格排在三个模块完成之后。
4. 不存在后端、Shell、导航或其他页面改动。
5. `DESIGN_REVIEW=PASS`、`ARCHITECTURE_REVIEW=PASS`、`DETAILED_DESIGN_REVIEW=PASS`。

通过后更新 `design.md` / `implement.md` 为新文档入口，标记 `DESIGN_FREEZE=2026-08-12`，再进入实现。

## 3. 设计冻结提交与推送

冻结后先做一个文档提交：

```text
docs: freeze prompt memory frontend design
```

只显式 stage 本任务设计、评审、计划、PRD/任务元数据和必要 spec 摘要；排除源码、生成 HTML、截图和无关图片目录。推送当前分支，不创建/切换分支，不开 PR。

## 4. 实施前预检

设计冻结后、派发执行 Agent 前只做环境和所有权预检，不运行测试：

1. `git branch --show-current` 必须是 `codex/prompt-memory-frontend-refactor`。
2. `git diff --name-only -- src-tauri` 必须为空。
3. 记录 `git status --short`，确认两个无关图片目录未被修改/删除。
4. 确认依赖目录、锁定 Node/pnpm 和 Chromium 可用；只确认存在，不启动测试或服务。
5. 把 `task.json.status` 从 `review` 改为 `in_progress`。
6. 重新读取冻结的 `detailed-design-overview.md` 文件 owner 表。

## 5. 三线路并行实施

主 Agent 同时派发三个执行 Agent。共同提示必须包含：

- Active task 路径。
- 你不是唯一在仓库工作的 Agent。
- 只能修改明确独占文件。
- 不得回滚、覆盖、格式化或提交其他 Agent 修改。
- 遇到跨模块问题报告主 Agent，不越权改共享/其他页面。
- 不改 Agent 目录、模型、Skills、MCP、navigation、router、AppShell、Shell、`src-tauri`。
- 完成后只运行自己的模块单测；不得运行 lint/typecheck/full unit/browser/build/dev server。
- 返回修改文件、行为变化、命令和原始测试结果。

### 5.1 线路 1：Prompt

Owner：

- `src/v2/pages/prompts/Page.tsx`
- `src/v2/pages/prompts/page.css`
- `src/v2/pages/prompts/prototype.ts`
- `tests/v2/pages/prompts/Page.test.tsx`

实现顺序：

1. data router test harness 与新增失败断言。
2. saved baseline / transient new draft。
3. saved rule 开关的原子 baseline 语义：clean 切换保持 clean；dirty 切换不吞草稿，放弃保留已提交开关。
4. page-level `useBlocker`。
5. canonical resource count 和 prototype 可见文案。
6. 聚焦单测。

唯一允许命令：

```bash
pnpm test:v2 -- tests/v2/pages/prompts/Page.test.tsx
```

### 5.2 线路 2：Memory

Owner：

- `src/v2/pages/memory/Page.tsx`
- `src/v2/pages/memory/page.css`
- `src/v2/pages/memory/prototype.ts`
- `tests/v2/pages/memory/Page.test.tsx`

实现顺序：

1. 新类型与现有 seed 迁移，移除假“已同步”。
2. data router test harness 与失败断言。
3. Daily/Session 只读与 saved baseline/target dirty。
4. provenance、原子切到 longTerm 的 transient promotion（初始目标为空）。
5. clean save no-op、per-category query/selection 保持、revision/tasks。
6. page-level `useBlocker`、路径/可见 prototype 状态。
7. 聚焦单测。

唯一允许命令：

```bash
pnpm test:v2 -- tests/v2/pages/memory/Page.test.tsx
```

### 5.3 线路 3：共享合同与 standalone

Owner：

- `src/v2/shared/config/agentTargets.ts`
- `tests/v2/shared/config/agentTargets.test.ts`
- `scripts/build-v2-preview.mjs`
- `tests/v2/scripts/build-v2-preview.test.ts`

不再修改 `src/index.html`、`package.json`、Playwright 配置、通用 tests 或 Shell styles。

实现顺序：

1. target 显式字段和 derived eligibility。
2. canonical Prompt grouping 与 invalid lookup。
3. 共享合同测试。
4. standalone entry parsing 与可测试函数边界（不运行 build）。
5. standalone 的 fail-fast、多 stylesheet 顺序、全部直接 module entry 与 path escape 模块测试。
6. 聚焦单测。

唯一允许命令：

```bash
pnpm test:v2 -- tests/v2/shared/config/agentTargets.test.ts tests/v2/scripts/build-v2-preview.test.ts
```

### 5.4 并行依赖处理

Prompt/Memory 可以按冻结签名编码，但共享文件可能仍在写入。页面执行 Agent 在共享线路完成前不得为了临时编译错误越权修改 `agentTargets.ts`；等待主 Agent 通知共享合同稳定后再跑自己的单测。

## 6. 子模块验收与小提交

主 Agent 等三个 Agent 全部返回后：

1. 用命令核验每个 owner 文件存在和 diff。
2. 读取关键状态转换，不接受仅“测试通过”的声明。
3. 核验三条单测命令和真实数量。
4. 确认没有执行完整集成/浏览器/build。
5. 发现模块问题，回派原 owner 修复并重跑其模块单测。

三个模块都绿后，按显式 path 分三个提交并逐次推送当前分支：

```text
feat: harden shared agent target contracts
feat: complete prompt prototype state flow
feat: complete memory prototype state flow
```

即使工作区仍有其他模块未提交修改，也不得使用 `git add -A`；每次只 stage 当前提交 owner 文件。

## 7. 首次完整集成

只有在以下证据同时成立后执行：

- Prompt Agent 完成 + Prompt 单测通过。
- Memory Agent 完成 + Memory 单测通过。
- Shared Agent 完成 + shared 单测通过。
- 主 Agent 静态核验关键文件和 owner 无越界。

然后按顺序运行：

```bash
pnpm lint:v2
pnpm typecheck:v2
pnpm test:v2
pnpm build:renderer
pnpm test:v2:browser
```

`test:v2:browser` 会再次 build；这是浏览器验收入口的既有合同，不用旧 build 结果替代。

## 8. 运行验收

### 8.1 四档视口

Playwright 配置自动覆盖：

- 900×600
- 1152×640
- 1232×700
- 1440×900

验收：

- Prompt/Memory 主操作可点击。
- dirty route guard 的取消/确认均可操作。
- Daily/Session 只读、提炼、保存、目标多选、pending task 可操作。
- standalone `file://` 可直接打开并切两页。
- standalone 生成 HTML 不残留指向本地 `dist/assets` 的 entry script/stylesheet。
- 无页面级横向溢出、按钮遮挡、不可达表单或 console/page error。

### 8.2 1586×992 截图

在完成所有修复后重新截取：

- `research/prompt-cross-agent-1586x992.png`
- `research/memory-cross-agent-1586x992.png`

证据只标 `runtime_screenshot`。未做 automated diff，不标 `pixel_diff` 或 1:1。

## 9. 失败修复循环

1. 保存原始失败命令与最小错误证据。
2. 定位 owning module；不跨模块顺手重构。
3. 先重跑 owning module 单测。
4. 再重跑首次失败的完整命令。
5. browser/layout 问题只改对应 page CSS；不改 Shell/token 兜底。
6. standalone 问题只改 builder/source；不手工修生成 HTML。
7. 同类问题连续出现时记录根因，再决定是否更新 spec。

## 10. 最终 Trellis check 与保护审计

代码和运行验收绿后使用 `trellis-check` 做全范围审查并直接修复可确认问题。最后运行：

```bash
python3 ./.trellis/scripts/task.py validate .trellis/tasks/08-12-prompt-memory-frontend-refactor
git diff --check
git diff --name-only e33d37dd6f9d58c11207f843b5c33750a79dbb4a...HEAD -- src-tauri
git diff --name-only -- src-tauri
```

保护范围均执行两层审计：`e33d37dd6f9d58c11207f843b5c33750a79dbb4a...HEAD` 检查已经提交并推送的累计 diff，普通 `git diff` / `git status --short -- <paths>` 检查未提交与 untracked 状态。不能用仅检查 worktree 的空 diff 证明整个分支未误纳。

零改动路径至少包括：`src-tauri/**`、`src/v2/pages/{agents,models,skills,mcp}/**`、`src/v2/shared/config/navigation.ts`、`src/v2/app/router.tsx`、`src/v2/widgets/app-shell/**`。`docs/images/视觉-1/` 与 `docs/images/视觉/` 还要分别检查 baseline-to-HEAD、worktree 与 untracked 状态，保持原样且不 stage。

每个小提交后用 `git show --pretty='' --name-only <commit>` 回读实际文件清单，必须等于该提交声明的显式 owner/文档集合。设计冻结前已经存在且要求保留的 `src/index.html`、V2 styles、通用测试和 preview 配置已记录在 `research/pre-implementation-protected-hashes.md`；执行 Agent 与后续修复不得改动这些冻结内容，最终再次比对 hash。

附加静态搜索：

- 退役 Codex 单应用/单启用/旧 Memory tab 文案。
- prototype 中 durable“已同步”假语义。
- `src/v2` legacy import 或 pages 直接 Tauri import。
- 本机用户名、绝对私人路径、凭据、私人 Prompt/Memory 正文。
- Agent/models/skills/mcp/navigation/router/AppShell/Shell 新增 diff。
- 无关图片目录删除/覆盖。

## 11. 文档回写与最终提交

最终回写：

- `prd.md`：只勾选本轮新鲜证明的验收项。
- `design.md`：权威入口 + 冻结方案摘要。
- `implement.md`：实际批次、命令和结果入口。
- `technical-design-overview.md` / `detailed-design-overview.md`：状态改为通过/冻结。
- 三份 review：关闭状态与证据。
- `research/verification.md`：替换旧结果为本轮新鲜结果。
- `.trellis/spec/frontend/v2-shell.md`：同步 executable contract，不碰其他模块合同。
- `task.json`：状态回到 `review`，不 completed、不 archive。

最后提交并推送：

```text
test: verify prompt memory frontend integration
docs: record prompt memory frontend acceptance
```

不创建 PR，除非用户另行要求。

## 12. 停止条件

只有以下情况允许中途停下向用户请求决定：

- 新事实要求改产品范围、后端合同或数据模型。
- 必须修改 navigation/router/AppShell/Shell 或其他人负责的四个页面才能继续。
- 需要真实文件写入、账户权限或外部生产操作。
- 新鲜验证失败无法在当前 owning scope 内修复。

普通 review 意见、单测失败、lint/typecheck/browser 回归和文档漂移由主 Agent继续修复，不作为确认点。
