# Frontend Interaction V3 Technical Design

## 1. 设计结论

采用“保留六路由、重构壳层、在 `/agents` 使用 query + 页面局部扫描状态”的最小方案。资源管理页继续拥有自己的业务状态和平台写入逻辑；Agent 选配页只聚合并调用现有 owner，不创建第二套平行配置数据库。

## 2. 运行结构

```text
src/index.html -> src/v2/main.tsx
  -> AppShell
     -> TopBar (Brand + ToolCluster)
     -> ShellBody
        -> SideNavigation
        -> ContentViewport
           -> PersistentPrimaryOutlet
              -> /agents | /models | /skills | /mcp | /prompts | /memory
```

### 路由策略

- 保留 `/agents`、`/models`、`/skills`、`/mcp`、`/prompts`、`/memory`。
- `navigationItems` 从扁平展示数据升级为支持 group/child 的 typed config，但仍提供给 `PersistentPrimaryOutlet` 一份稳定的 route leaf 列表。
- `/agents` 使用稳定 query 合同保存可恢复的选择：`?target=<agentId>&section=models|skills|mcp|prompts`。没有 query 时显示软件目录。
- 扫描请求使用页面局部显式状态：
  - `directory.idle`
  - `directory.scanning`
  - `directory.empty`
  - `directory.error`
- selected Agent 和 section 不放入全局 store；由 Router query 派生。
- “进入管理”导航到既有 route；从管理页返回时 `/agents` keep-alive 保留 selected agent 与 tab。
- 不新增 pathname 或嵌套路由；query 只承担刷新恢复、返回和全局管理跳转衔接。

## 3. 壳层与组件责任

| 区域 | 现有 owner | V3 改动 |
|---|---|---|
| 顶栏 | `widgets/app-shell/TopBar.tsx` | 移除六等权主导航，只保留品牌与工具动作 |
| 左侧导航 | 新建 `widgets/app-shell/SideNavigation.tsx` | 三组导航、展开、active、keyboard、responsive |
| 内容 keep-alive | `app/PersistentPrimaryOutlet.tsx` | 继续挂载六页；改用 leaf route 解析 |
| Agent 页 | `pages/agents/Page.tsx` | 软件目录/扫描状态 + 单 Agent 四段配置壳层 |
| 资源控件 | shared UI | 复用 `FeatureTabs`、`FeatureSearch`、`FeatureList`、`FeaturePagination`、`SelectionLens` |
| 分配控件 | shared UI | 复用 `AssignmentPanel`、`InstallTargetDialog` |
| 管理布局 | shared UI | 复用 `CatalogMasterDetail`、`SplitPanes` |

新增组件必须是被两个以上页面/模块真实复用的稳定概念；仅为减少 JSX 行数不得提前抽象。

## 4. 数据流与能力真相

### Skills / MCP

沿用现有 per-Agent assignment ports 与 target IDs。Agent 选配页读取全局管理页的已安装资源及分配状态；toggle 后等待 authoritative readback 再更新成功反馈。optimistic UI 如使用，失败必须回滚。

### 模型

当前模型能力按应用分散，且并非所有 Agent 可写。V3 不新增通用 model assignment port，采用 capability-aware projection：

1. 从现有 quick setup、WorkBuddy/OpenCode 专用配置以及 Qoder/TRAE capability 读取 sanitized 状态。
2. 增加 route-local view model，显示已观测/已配置模型、能力状态和“进入模型管理”。
3. 已有 direct owner 的动作只能委托原 writer；不统一异构保存协议。
4. Qoder unsupported、TRAE read/assisted 等状态以 disabled/read-only 呈现。

禁止把 UI toggle 写入 local-only store 并展示为供应商配置成功。

### 提示词

现有 `PromptAppId` 支持集合继续是权威来源。支持的 Agent 使用现有 prompt port；Qoder/TRAE/WorkBuddy 等无 owner 的目标显示不可用说明，除非实现期找到并验证已有原生 owner。不得仅为原型一致而扩展伪后端。

### 记忆

仅做布局适配，读取/保存/打开工作区的现有 platform adapter 不变。

## 5. 扫描状态

- 扫描聚合现有 Agent Install Readiness port 的七个查询；settled 数量形成进度，不新增 native scan command。
- 状态包含 request id 或等价 stale-response guard，避免重复扫描的旧响应覆盖新结果。
- 页面卸载或重新进入时不得把已保存配置清空。
- 扫描失败保留上次成功结果，并提供可重试反馈。
- 当前 port 没有取消语义，因此页面不展示可成功的取消动作；`unknown` 独立于“未安装”。
- browser preview 只作为交互 fixture，不宣称真实机器扫描结果。

## 6. 视觉 token 与布局

- 继续使用 `src/v2/app/styles/tokens.css` 的 Blue Ambient / Clear Glass token。
- 只新增壳层必需的布局 token，例如侧栏宽度、壳层 gap、窄视口 rail 宽度。
- 桌面默认侧栏完整显示 group 与 child 文案；900px 附近允许压缩间距和宽度，但不切成无标签 icon rail。
- 原型图负责层级、位置与密度；文字清晰度、可访问性和真实状态优先于逐像素复刻。

## 7. 可访问性与交互约束

- 侧栏展开按钮使用 `aria-expanded`、`aria-controls`；当前路由使用 `aria-current="page"`。
- 四段页签使用既有可访问 tab/selection 语义。
- 所有 icon-only 动作有 label/tooltip；focus ring 不被玻璃面板裁切。
- loading/disabled 不只依靠颜色；错误反馈与对应动作邻近。
- 尊重 `prefers-reduced-motion`。

## 8. 规范修订

下列当前规范与已确认 V3 冲突，必须在同一实现任务中明确替换旧条款：

- `v2-shell.md`：顶部六个主导航 -> 左侧三组导航，路由本身保留。
- `v2-agent-models.md`：Agent directory 仅提供 direct jump、禁止详细 selector -> 允许 `/agents` 内四段 selector，但能力写入仍受现有平台契约约束。
- `v2-skills-mcp.md` 与 `v2-prompts-memory.md`：补充从 Agent 选配页进入全局管理与返回的关系，不改变原数据 owner。

旧结论在规范中应标记为 `SUPERSEDED_BY_FRONTEND_INTERACTION_V3`，避免后续 Agent 同时执行两套信息架构。

## 9. 验证设计

### 静态与组件

- typed navigation tree 与 leaf routes 单元测试。
- Agent 页面状态 reducer/view model 单元测试。
- 模型/提示词 capability matrix 测试，确保 unsupported 不出现可写成功态。
- 现有页面测试按新壳层更新，不删除有意义的业务断言。

### 浏览器交互

- 导航展开、六 route keep-alive、返回路径。
- 扫描状态转换、unknown、stale response，以及不存在假取消。
- 四段搜索、toggle/selection、管理跳转与失败回滚。
- 900px 与桌面宽度关键布局。

### 桌面与 Windows

- 本地 packaged app 覆盖 11 个冻结状态并截图。
- 平台写入动作以 sanitized readback 证明；不输出 secret。
- Windows 环境重复核心路径，单独保存 fresh receipt；签名/公证不属于本任务。

## 10. 回滚与提交策略

按可独立回滚的序列提交：

1. 任务/规范与壳层导航；
2. Agent 扫描与四段壳层；
3. Skills/MCP 真实分配接入；
4. 模型/提示词 capability-honest 接入；
5. 管理页/记忆视觉整合；
6. 测试、UAT 修复与证据文档。

无数据迁移。出现回归时回滚对应窄提交，而不是重置整条分支。
