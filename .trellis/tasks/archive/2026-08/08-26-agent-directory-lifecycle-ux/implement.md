# Implementation Plan — Single Branch / Serial Execution

## Phase 0 — Start Gate

1. 只使用当前任务：`.trellis/tasks/08-26-agent-directory-lifecycle-ux`。
2. 后续开发只使用一个开发分支；不要创建子任务或并行 Worktree。
3. 开始修改产品代码前检查当前 working tree，识别已有 V3 Agent/Shell 改动并保护它们，不覆盖用户未授权变更。
4. 读取本任务 `prd.md`、`design.md`、`research/planning-evidence.md`、`research/execution-context.md` 以及 `implement.jsonl` / `check.jsonl`。
5. 默认直接采用已经完成的调研和复用方案；只有代码现状与规划冲突、出现真实能力缺口或新的兼容/安全问题时，才做针对性补充调研。

## Phase 1 — Sidebar Collapse Motion

1. 确认 SelectionLens、SideNavigation 和 V2 shared UI 中没有新的等价 owner。
2. 优先复用现有 Radix Collapsible + Motion；不新增动画/Disclosure 依赖。
3. 建立或扩展最小 shared motion/collapsible owner，使 SelectionLens 与折叠内容复用同一 spring source。
4. 将「配置管理」展开/收起接入该 owner；保留 Router active state、`aria-expanded`、ArrowRight / ArrowLeft / Escape / Home / End。
5. caret 与内容运动保持同源，不保留互相冲突的独立 easing；`prefers-reduced-motion` 直接或近即时切换。
6. 运行 focused SideNavigation / architecture tests 和 `mise run typecheck:v2`。

**阶段检查点**：没有新增依赖、没有第二套 motion token、键盘/ARIA/reduced-motion 不回归后再继续。

## Phase 2 — Progressive Directory Scan

1. 在现有 `useAgentDirectoryScan` 上最小演进，保留 request-id/stale guard、retained results、single-flight 和 TanStack Query owner。
2. 让 catalog 决定 7 行是否存在；scan state 只决定每行当前 readiness/pending/error presentation。
3. 加入首次进入目录的 auto-start gate；注意 V2 keep-alive，避免每次重新显示 `/agents` 都无条件重扫。
4. 保留 manual rescan；单项 readiness settle 后立即更新该行，不等待全部 promise 完成。
5. 覆盖 partial failure、all failure、rescan retained result、stale previous request completion。
6. 运行 focused Agent page/hook Vitest 和 `mise run typecheck:v2`。

**阶段检查点**：首屏目录不再依赖扫描结果，技术失败不会被转换为 not-installed，首次自动扫描只触发一次。

## Phase 3 — Lifecycle Actions / Progress Reuse

1. 检查是否已有 generic Agent action hook；若没有，从 `AgentInstallReadinessSection` 抽取现有 start → poll → terminal → authoritative refresh runner，不复制代码。
2. generic path 只根据当前 readiness 的 `allowedActions` 暴露 install/update，并继续使用 `FeaturePorts.agentInstallReadiness`。
3. immediate CLI action 使用 pending → authoritative refresh；有 jobId 的动作展示真实 stage 并处理 terminal/failure/timeout/cancel。
4. Codex 直接复用 `useCodexDesktopInstaller` / `FeaturePorts.codexDesktop`，目录只消费需要的最小投影；不复制 percent、speed、subscription、downloader 或 release validation。
5. 不扩大 backend DTO，不为了 UI 伪造 generic numeric progress。
6. 写 focused tests：install/update、job stage、immediate action、failure/reason、timeout、refresh、Codex owner honesty。
7. 运行相关 Vitest 和 `mise run typecheck:v2`。

**阶段检查点**：generic/Codex owner 边界清楚，动作终态后一定权威回读，没有 optimistic installed flag。

## Phase 4 — Agent Directory Composition

1. 将 `AgentDirectory` 改为 catalog-driven full list，并接入 per-row scan projection。
2. 集中派生 configure gate，避免 JSX 多处重复判断：首次未确认 / not-installed / unknown / unavailable / 当前动作中不可配置；已确认存在时按 PRD 兼容语义开放。
3. 在「进行配置」左侧接 lifecycle slot：扫描中 / 一键安装 / 一键更新 / action stage/progress / failure state。
4. 未安装 + backend 允许 install 才显示「一键安装」；已安装 + update_available + backend 允许 update 才显示「一键更新」。不支持的平台不显示 fake action。
5. 调整 `Page.css` action group、responsive 和状态 presentation；继续使用现有 design tokens/primitives，不引入第二设计体系。
6. 安装/更新成功后只依赖刷新后的 readiness/Codex local state重渲染，不写本地“安装成功”事实。
7. 更新 Agent page unit tests，删除旧的 idle 空列表 / installed-only 断言，加入完整状态矩阵。

**阶段检查点**：7 行首屏存在、逐项扫描、configure gate、install/update 条件、busy/progress/readback 都能在 unit tests 中独立证明。

## Phase 5 — Browser Integration / SPEC / Final Gate

1. 更新 `tests/v2-browser/agents-v3.spec.ts`：首屏全量、自动 scan、渐进 settle、install/update/readback、partial failure/rescan，并保留 Skills/MCP/Models capability honesty。
2. 联合运行 SideNavigation/browser shell，确认 collapse animation 不影响 route、focus、active SelectionLens 和 reduced-motion。
3. 检查 responsive / overflow / error recovery。
4. 更新 `.trellis/spec/frontend/v2-agent-models.md`，必要时更新 `v2-shell.md` / reuse spec；只记录最终稳定规则，不固化临时 hook 名称或本次串行施工步骤。
5. 最少运行：

```bash
mise run typecheck:v2
mise run test:v2
mise run test:v2:browser
mise run build:renderer
```

6. 根据当时仓库 mandatory gate 补 `mise run check` 或等价 required checks。

## Final Review Checklist

- 没有第二 catalog / installer / query / motion owner。
- 没有新增不必要依赖。
- 不支持的平台没有 fake 一键安装。
- Generic job 没有 fake percentage。
- Codex 没有复制 downloader/progress engine。
- action success 后有 authoritative readback。
- first scan 不空白，rescan 不闪空。
- reduced-motion / keyboard / ARIA 正确。
- 新 shared code 有真实复用价值，没有为了阶段拆分制造一次性 abstraction。
- 全部工作在一个分支串行完成，没有 Worktree/子任务残留。

## Execution Flexibility

阶段顺序是推荐的风险控制路径，不是死板的代码合同。执行方如果发现最新代码已经有等价 helper/component，直接复用并缩小改动；如果两个相邻阶段在当前代码中天然应该一起改，可以合并实现并在阶段检查点统一验证。只有新增依赖、扩大 wire/backend contract、改变安全边界或用户可见语义时，才需要回到规划评审。
