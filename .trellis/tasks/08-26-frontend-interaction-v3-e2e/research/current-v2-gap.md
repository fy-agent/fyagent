# Current V2 Gap Audit

## 审计边界

- 基线：`origin/main` at `91a66254a0f7190fbc500591d188f52cde74fc7e`。
- 设计：`docs/fyagent/design/frontend-interaction-v3/` 的 11 张已批准高保真原型。
- 产品来源：飞书讨论 `WXTZdpDUHo6Q2fxyjXSc3242n2a` revision 2。
- 证据级别：`code_audit`；尚非运行态或 Windows 验收。

## 已确认现状

1. 生产入口已经是 `src/index.html -> src/v2/main.tsx`。
2. 当前六个主 route 为 `/agents`、`/models`、`/skills`、`/mcp`、`/prompts`、`/memory`。
3. `PersistentPrimaryOutlet` 为六页提供 keep-alive；V3 不需要重写 router。
4. 当前 `TopBar + PrimaryNav` 展示六个等权顶部入口，与 V3 左侧三组导航冲突。
5. Models/Skills/MCP/Prompts/Memory 已有大量真实业务逻辑，07–11 应做重装壳与局部重排。
6. `FeatureTabs`、`FeatureSearch`、`FeatureList`、`FeaturePagination`、`AssignmentPanel`、`InstallTargetDialog`、`CatalogMasterDetail`、`SelectionLens` 可直接复用。
7. Skills 与 MCP 已有真实 per-Agent assignment owner。
8. Agent 页当前不具备批准稿要求的扫描状态与四段选配壳层。

## 最小新增面

- `SideNavigation` 与新 shell body layout。
- typed grouped navigation config，同时保留稳定 leaf route 列表。
- Agent directory 扫描状态机。
- `/agents?target=<agentId>&section=models|skills|mcp|prompts` + four-section configure shell。
- route-local 模型/提示词 capability view model；不新增通用 model assignment port。

## 能力不对称

### 模型

- 现有配置按应用分散，并非统一可写。
- Qoder 当前不支持该写入路径。
- TRAE 主要是读取/assisted/vendor UI 路径，不能按原型假装通用 toggle 后已写入。
- WorkBuddy/OpenCode 有各自专用模型配置；Claude/Codex/Grok 有 quick setup 路径。

### 提示词

- 当前 `PromptAppId` 只覆盖已有支持集合。
- Qoder/TRAE/WorkBuddy 等不能自动被视为有 prompt writer。

结论：设计稿定义交互骨架，但可写能力必须由当前平台契约裁决。unsupported 与 assisted-only 是应实现的产品状态，不是需要被隐藏的异常。

## 三线程收敛结论

三条独立 `gpt-5.6-sol / max` 只读线程分别完成代码差距、测试打包和复杂度审查，结论一致：

- 11 张图对应 6 个产品面、11 个状态，不应产生 11 个 route/page。
- 扫描复用 Agent Install Readiness 的七个 queries，以 settled 数量计算进度；`unknown` 不能写成“未安装”。
- 当前没有真实取消扫描语义；本轮只显示等待，不新建后台扫描、进度事件或取消协议。
- 保留六个 pathname，Agent/section 使用 query，避免全局 store 和 returnTo 注册表。
- Models 只做 capability-aware projection；Prompt 对 `promptAppId=null` 的 Agent 显示不支持。
- Memory 原型中的“复制”指复制内容；现有复制路径能力不能替代。
- 新测试收敛为 4–6 个聚焦契约与 1 条跨页面浏览器路径，运行面冻结后再制作截图/pixel evidence。

## 规范冲突

- `v2-shell.md` 仍规定顶部六主导航。
- `v2-agent-models.md` 仍限制 Agent directory 只做 direct capability jumps，且排除详细 selector。

这两条已被 2026-08-26 人类确认的 V3 产品决定替代，需在实现前同步修订并标记 superseded。

## 环境预检

- 独立 worktree 当前没有 `node_modules` 和 `.venv`。
- 仓库 env/system checks 会在 Python 依赖 `smol-toml` 导入前失败；这是 bootstrap 未完成，不是产品代码回归。
- 实现开始后先运行 `mise run bootstrap`，再以 fresh checks 建立基线。
- 当前 macOS 仅确认 Xcode Command Line Tools；Developer ID 签名/公证不在本任务范围。

## 风险排序

1. Blocking：把 unsupported 模型/提示词画成可写并返回假成功。
2. Blocking：把 readiness `unknown` 误报为“未安装”，或提供没有 cancellation contract 的假取消。
3. Major：修改 router 导致 keep-alive 和管理页状态丢失。
4. Major：重写管理页而不是复用，扩大回归面。
5. Major：在页面仍变化时反复制作正式证据，导致截图失效。
6. Minor：原型与真实字体渲染的像素差异；当前不构成产品阻塞。

## 推荐实现原则

- 先稳定壳层，再接四段真实能力，再做管理页视觉整合，最后冻结运行面并采集证据。
- 每次写入由原 owner 执行并 fresh readback。
- 不添加跨域通用 repository 或“万能资源”抽象。
- 测试聚焦状态转换、导航/返回、能力矩阵与写入失败回滚；避免为静态样式堆叠低价值快照。
