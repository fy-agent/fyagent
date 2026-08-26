# Frontend Interaction V3.1 Technical Design

## 1. Change Boundary

最小行为差距是：当前 V3 页面保留真实端口与路由，但 UI 投影加入了原型外内容、错误的左右关系、过密容器和错误扫描结果语义。V3.1 在 renderer 层修正页面结构与投影，不扩展后端能力。

预计修改范围：

- `src/v2/pages/agents/**`：扫描状态、完成结果投影、Agent 四段页面结构与局部样式；
- `src/v2/pages/prompts/**`：页面 10 结构与局部样式；
- `src/v2/pages/memory/**`：页面 11 结构与局部样式；
- `src/v2/pages/models/**`、`skills/**`、`mcp/**`：只处理逐页对照确认的结构与视觉差异；
- `src/v2/shared/ui/**`：仅在两个以上目标页面确有共同原型结构时增加窄 variant；
- `tests/v2/**`、`tests/v2-browser/**`：同步新的可见行为与负面断言。

明确排除 `src-tauri/**`、数据库、API、依赖、全局设计体系和无关页面。

## 2. Source and Visual Direction

- 产品 archetype：AI / Developer Tool 桌面控制中心。
- 视觉基线：仓库 11 张高保真 PNG 与现有蓝色 liquid-glass token。
- token 继续使用当前 `--bg`、`--surface`、`--surface-2`、`--text`、`--text-muted`、`--accent`、`--border`、`--radius`、`--shadow` 体系；只增加有明确原型语义的局部 token。
- 原型控制页面组成与层级，现有业务端口控制数据真实性。

## 3. Runtime Structure

```text
AppShell
  -> SideNavigation
  -> PersistentPrimaryOutlet
     -> /agents
        -> AgentDirectory
           -> useAgentDirectoryScan
           -> completed-result projection
        -> AgentConfiguration
           -> full-width FeatureTabs variant
           -> Models | Skills | MCP | Prompts section
     -> /models | /skills | /mcp
     -> /prompts
     -> /memory
```

六个 route 与 keep-alive 行为保持不变。`/agents?target=<id>&section=<section>` 保持现有 query 合同。

## 4. Scan State and Projection

底层 `AgentInstallReadiness` 类型继续保留全部平台语义。页面增加显式投影函数或等价 view model：

```ts
type VisibleInstalledAgent = AgentInstallReadiness & {
  installState: "installed" | "installed_not_runnable";
};

type ScanProjection = {
  installed: VisibleInstalledAgent[];
  failedIds: AgentCatalogId[];
  hasTechnicalError: boolean;
};
```

投影规则：

1. `installed` 与 `installed_not_runnable` 进入完成列表；
2. `not_installed`、`unknown` 和 `unavailable` 从正常列表移除；
3. query rejection 与环境错误进入独立 error channel；
4. 已安装项采用明确优先级与 `AGENT_CATALOG_IDS` 稳定顺序；
5. scanning 阶段只展示原型所需反馈；
6. stale-response guard、最近一次安全结果和真实配置保持现有语义。

页面组件不再 map 全目录后逐卡显示状态。测试直接验证投影数组与 DOM 列表，避免 CSS 隐藏造成语义残留。

### Frozen UI States

- `idle`：只显示原型标题行、指定图标和左侧 `开始扫描`；结果区保持空白，页面不补写说明、状态卡或目录占位。
- `scanning`：按页面 02 显示 `扫描中…`、`正在扫描本机 AI 软件`、`已发现 N 个`、一条进度轨道、已经发现的安装项与剩余骨架；移除 `已完成 x / y` 与取消能力说明。
- `complete`：按页面 01 显示 `我的 AI 软件`、左侧扫描入口和已安装卡片；卡片不显示状态 badge、扫描时间、完整介绍或未安装目录项。
- `error`：使用现有 `InlineNotice` 的最小错误反馈与重试入口；错误项不进入正常卡片列表，成功的安装项仍按完成态投影。

以上四个状态先写入逐页差异表，再允许产品代码修改。

## 5. Page Structure

### Pages 01-02

- `AgentDirectory` header 收敛为原型中的图标、`我的 AI 软件` 和左侧扫描入口。
- 完成态渲染已安装集合；扫描态渲染原型长条、禁用动作与骨架。
- 删除卡片上的原型外状态、完整介绍、扫描时间和额外说明，保留原型中的软件信息与配置入口。

### Pages 03-06

- `AgentConfiguration` identity row、返回入口和 tabs 按原型重排。
- tabs variant 填满可用宽度，四段等分并保持响应式。
- section header 只输出原型中的标题与管理入口；description 改为可选或移除。
- 模型段只读投影现有 capability query：WorkBuddy 读取 status/model IDs，OpenCode 读取 snapshot，TRAE Work 保持 assisted，Qoderwork 保持 unsupported，Provider 类目标读取 summary；该段只提供搜索、选中详情和管理路由，DOM 中不出现模型 Switch。
- WorkBuddy/OpenCode 的 `saveModels`、Provider quick setup、TRAE Work validation 与任何模型 mutation 均留在原 owner，禁止降格为 Agent 页单项开关。
- Skills 与 MCP 继续复用真实 query、toggle owner 与 authoritative readback，改为原型通栏结构。
- 提示词段只在 `promptAppId` 存在时读取列表、搜索、展示只读正文，并执行 `enable + refetch`；回读目标 `enabled === true` 后提交成功状态。
- 提示词导入、新建、编辑、保存、删除、停用、live file、dirty guard 与 write lock 只归 Page 10；`promptAppId=null` 时不发 query。
- 删除内部 capability 说明和 UI 外泄语义；不可闭环的原型交互不进入 DOM。

### Pages 07-09

- 使用现有管理页与共享 chassis。
- 只对逐页差异清单确认的标题、操作位置、宽度、密度、列表和分配区做窄改动。

### Page 10

- 保留 `PromptsPort`、query partition、write lock、dirty guard 和 reread。
- 用原型三列关系组织应用栏、提示词库与编辑区，操作与搜索位置对齐原型。
- 页面 CSS 只负责命名空间内宽度、滚动和响应式。

### Page 11

- 保留 `MemoryPort`、四个长期资源、每日文件与目录动作。
- 按原型组织页签、左侧资源区、编辑头和主体；复制当前正文。
- 页面 CSS 只负责命名空间内宽度、滚动和响应式。

### Required Geometry Checks

- 页面 03：四个 tab 在同一横向容器内等分，可用宽度覆盖右侧内容区；返回入口位于原型右上角。
- 页面 03：模型列表不存在 `role=switch`；Qoderwork 不发模型 query，TRAE Work 只读 assisted，所有目标零模型 mutation。
- 页面 06：有 `promptAppId` 的启用动作完成 refetch；无 `promptAppId` 的目标不调用 `prompts.getAll`；CRUD 入口仅保留 `进入提示词管理`。
- 页面 10：应用栏、提示词库、编辑区保持原型顺序；搜索和顶部动作的 bounding box 与目标列对齐。
- 页面 11：资源列表在左、编辑区在右；页签、编辑头动作与 textarea 边界顺序固定。
- Browser tests 在目标视口读取 bounding boxes，验证 pane 数量、顺序、宽度占比、越界和主动作位置。

## 6. Shared Component Policy

- 先复用 `FeatureTabs`、`FeatureList`、`FeatureSearch`、`SplitPanes`、`CatalogMasterDetail` 与 primitives。
- 原型需要新的横向 tabs 形态时，增加语义明确的 class/variant，保持旧调用方视觉稳定。
- 只有两个以上页面需要同一稳定结构时才新增 shared component。
- 禁止页面级覆盖穿透其他模块，禁止全局 CSS 补丁。

## 7. A-to-A Execution and Grok Review Contract

实施链路固定为：

1. Antigravity / Gemini 3.7 持有全部前端页面、局部 UI variant、状态投影与相关前端测试的实施和 UI 缺陷修复责任。
2. Cursor / Grok 4.6 持有后端与共享组件边界、技术调研、复杂度挑战和强制评审责任；当前不计划后端扩展，必要的后端或共享组件改动必须先形成最短合同和明确 owner。
3. Codex / `gpt-5.6-sol` / `max` 持有任务、分支、调度、监工、验证、桌面运行、截图、透明资产与交接责任；Codex 不修改页面 JSX/CSS，不与 Gemini 争抢前端 owner。
4. Cursor / Grok 4.6 对每个冻结波次和最终 diff 执行强制门槛；CLI 身份不可用时使用已授权的 Grok Build / `grok-4.6`，并记录实际路由。
5. Grok Bot App 在桌面运行核验阶段担任观察员和监工；桌面操作、截图与透明底图资产仍由 Codex 执行。
6. Codex 汇总 Gemini 与 Grok 结论，对冲突按最新用户原文、原型、真实端口合同和代码证据裁决，并负责最终交接口径。

### Stage Lock State Machine

```text
planning
  -- Grok Planning PASS + user approval --> wave_a
wave_a
  -- Grok Gate A PASS --> wave_b
wave_b
  -- Grok Final PASS --> codex_verification
codex_verification
  -- fresh engineering + runtime evidence --> handoff_ready
```

`CHANGES_REQUIRED` 不前进状态，直接回到当前阶段 owner 修订；新 diff 使旧 verdict 失效。Gate A 前不得启动 Wave B，Final Gate 前不得运行最终验证、冻结候选、制作完成截图或形成对外完成口径。

每个实施波次结束后生成只读 review packet：

- 当前 branch 和 base commit；
- `git diff --stat` 与 `git diff --name-only`；
- 原型编号到修改文件映射；
- 原型外标题/副标题/解释、填充卡片/分组/标签/入口、“未确认”及同类状态的负面扫描结果；
- 左右关系、横向长条宽度与 Prompts/Memory pane 结构的目标视口截图或几何证据；
- 相关测试命令与 exit code；
- 未决缺口；
- Gemini、Grok 与 Codex 的实际模型/通道路由。

review packet 完整性是 Gate A 与 Final Gate 的开闸前提；缺任一项直接 `CHANGES_REQUIRED`。packet 中的目标视口截图或几何仅作为过程审查证据，不是完成截图、候选截图或对外完成口径。

Grok 输出固定结构：

```text
Verdict: PASS | CHANGES_REQUIRED
Blocking:
Major:
Complexity:
Evidence:
Required fixes:
```

任一用户指定必查项存在即为 `CHANGES_REQUIRED`；带 Blocking/Major 的条件通过无效。UI、布局、样式、文案、前端状态和 UI 测试问题全部退回 Gemini；Codex 自有的调度、桌面、截图、透明资产、证据、交接和消息问题退回 Codex；后端、共享组件边界、数据语义与复杂度问题由 Grok 给出精确合同和 owner。Codex 不接管页面代码。每次修订都重新进入同一 Grok 门槛，直至明确 `PASS`。

## 8. Validation and Evidence

证据层次：

1. `code_audit`：文件、测试与 Grok review；
2. `runtime_screenshot`：冻结表面后的本地运行核验；
3. `pixel_diff`：本轮未设数值阈值，仅在用户另行要求时加入；
4. merge、release、production：全部排除。

最终截图与报告只在完整 diff 通过 Grok Final Gate 后生成；Grok 通过前不得冻结候选或形成高保真完成口径。本轮没有对外发送授权。

## 9. Compatibility and Rollback

- 无数据迁移、无 API 迁移、无依赖变更。
- 路由和端口保持兼容。
- 按页面波次提交可独立回滚的窄变更；出现回归时回退对应波次，不重置整个分支。
- 旧 V3 分支保持完整历史，V3.1 分支承载全部新改动。
