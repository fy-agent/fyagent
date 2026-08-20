# 技术设计：Agent 目录交互、官方链接与 Codex 安装接入

## 1. 设计目标与边界

本任务在一个集成任务内完成五个相互关联的交付：Agent 图标背景、目录官方动作、Codex 安装器 V2 接入、双栏高度、Skills/MCP 分配图标。它们共享 Agent 目录合同、V2 FeaturePorts、同一详情页面及相同浏览器/原生验收，因此不拆成父子任务；实施时按无冲突文件所有权并行，再做一次集成审查。

必须保持的边界：

- Agent/Models 使用同一五候选目录；Skills/MCP 使用既有六应用分配集合。
- Codex 安装器的 URL、路径、包身份、校验与权限决定继续完全由 Rust 后端拥有。
- V2 页面不导入旧版 `src/components/**`、`src/hooks/**`、`src/lib/**`、旧样式或直接 Tauri API。
- Browser preview 不产生权威安装或外链成功。
- 不新增依赖、持久化迁移或其他 Agent 自动安装能力。

## 2. 版本化 Agent 目录合同 v2

### 2.1 Wire shape

将 v1 的单一 `officialUrl` 替换为结构化链接，并把合同版本提升为 2：

```ts
type AgentOfficialLinkId = "product" | "cli" | "desktop";

type AgentOfficialLink = {
  id: AgentOfficialLinkId;
  label: string;
  url: string;
};

type AgentCatalogEntry = {
  id: AgentCatalogId;
  displayName: string;
  description: string;
  officialLinks: AgentOfficialLink[];
  status:
    | "pending_verification"
    | "manual_install"
    | "managed_install";
  actions: AgentCatalogActions;
  evidenceLabel: string;
};

type AgentCatalogResult = {
  contractVersion: 2;
  reviewedAt: string;
  agents: AgentCatalogEntry[];
};
```

精确链接矩阵：

| Agent | links | browse | install |
| --- | --- | --- | --- |
| QoderWork | `product` -> 经核验 Qoder CN 页面 | available | assisted |
| TRAE Work | `product` -> `https://work.trae.cn/` | available | assisted |
| WorkBuddy | `product` -> `https://www.workbuddy.cn/` | available | assisted |
| Codex | 空数组 | not_supported | available |
| Claude Code | `cli` -> Anthropic CLI 设置文档；`desktop` -> `https://claude.com/download` | available | assisted |

Rust 在构造目录时拥有 URL、label、能力和状态事实。V2 Tauri adapter 对合同版本、精确字段、已知 ID、绝对 HTTPS、非空标签、条目顺序及 Codex 零链接做运行时解析；页面只渲染已解析结构，不维护第二份 URL 常量。Models 页从与目标行为匹配的 `product` 链接打开 QoderWork/TRAE 指引，不依赖数组第一个元素的隐式顺序。

### 2.2 Compatibility

这是内部、payload-free Tauri read contract 的原子升级，没有持久化数据或外部客户端兼容负担。Rust、TypeScript、fake-Tauri fixture、Agent/Models consumers 和精确合同测试在一个提交中更新；任一 v1 payload 进入 v2 parser 时明确失败为目录不可用，不静默猜测字段。

## 3. 官方链接动作与原生打开

数据流：

```text
Rust Agent catalog v2
  -> V2 Tauri runtime parser
  -> typed AgentCatalogEntry.officialLinks
  -> Agent detail semantic buttons
  -> SettingsPort.openExternal(url)
  -> open_external
  -> interactive user's system shell/browser
```

- QoderWork/TRAE/WorkBuddy 渲染一个 catalog-owned label/button。
- Claude 渲染 CLI 与 Desktop 两个按钮；每个按钮有独立 pending ID，避免一个请求把无关按钮语义混淆。
- Codex 不渲染该区域的外链按钮。
- 页面保留一次一个 open 调用的锁；失败使用固定、无 URL 回显的提示。
- 先用 fake-Tauri 冻结 exact command/payload，再在真实 Tauri 中点击至少一个官方动作。如果原生命令返回错误，读取结构化错误/本机日志，修复 `process_launch` 的最小根因；不得退回 `window.open`、从提升进程直接启动浏览器或放宽 HTTP(S)/交互用户证明。

## 4. Codex 安装器共享核心与 V2 适配

### 4.1 为什么不直接复制或导入旧组件

现有 Hook 超过 1000 行，包含 JobSnapshot 接纳顺序、下载速度样本、Query 缓存、事件恢复、状态/操作推导和错误行为。复制会产生第二套安全相关状态机；直接从 V2 导入旧 Hook/组件则违反 V2 shell 合同和直接 Tauri import 边界。

### 4.2 中立共享纯核心

新增窄范围的 `src/shared/codex-desktop/**`，只承载两个 renderer 都需要的纯合同：

- Rust 镜像 DTO 与安全辅助类型；
- platform version 显示/比较；
- local/remote version state 推导；
- installer view state、primary action 与禁用条件；
- JobSnapshot 单调接纳规则；
- 下载阶段相邻样本速度/进度推导；
- fixed error/view projections that do not translate or invoke side effects.

该目录不得导入 Tauri、React UI、legacy component、i18n、toast 或 platform code。原 `src/types/codexDesktop.ts` 和必要的旧纯模块保留兼容 re-export，旧 Hook 改为消费共享纯核心并通过既有测试证明无行为变化。V2 shell/architecture test 只允许这一精确 neutral shared boundary，不开放任意 legacy import。

### 4.3 V2 CodexDesktopPort

在 `FeaturePorts` 增加：

```ts
interface CodexDesktopPort {
  getLocalStatus(): Promise<LocalInstallStatus>;
  checkLatest(force: boolean): Promise<RemoteReleaseStatus>;
  getJob(): Promise<JobSnapshot | null>;
  startInstall(expectedReleaseId: string): Promise<JobSnapshot>;
  cancelInstall(jobId: string): Promise<JobSnapshot>;
  launch(): Promise<void>;
  openLogDirectory(): Promise<void>;
  subscribeJobUpdates(
    onSnapshot: (snapshot: JobSnapshot) => void,
  ): Promise<() => void>;
}
```

Tauri adapter 使用原有七个固定命令和 `codex-desktop-installer://job-updated` 事件。Start 只发送 `{ expectedReleaseId }`；不得加入 URL、path、hash、scope 或 bypass。Browser adapter 的所有读写/订阅均返回明确 native-only 不可用，不提供 production-looking fixture；丰富 fixture 只存在于测试注入的 FeaturePorts。

### 4.4 V2 controller 与详情视图

V2 controller 使用现有 FeatureProvider 的 QueryClient，调用 port 并消费共享纯状态规则：

1. 并行读取 local/latest/job；事件订阅建立后再恢复一次 job snapshot，按共享单调规则合并。
2. 根据共享规则显示 checking、unsupported、ambiguous、ready install/update/launch、remote unavailable、job stages、succeeded/failed/cancelled。
3. install/update 前只使用已验证 remote `releaseId`；METADATA_CHANGED 要求刷新后由用户重新触发。
4. job working 时锁定冲突动作；只在后端 `cancellable` 时显示取消。
5. 仅 downloading/download 阶段显示 byte pair 与 bytes/s；安装原生进度不标记为字节。
6. 终态 invalidate/reread local、remote、job；错误只显示既有 redacted DTO 字段。
7. 页面卸载时取消事件监听；React StrictMode 不得留下重复监听。

视图使用 V2 primitives/tokens 和中文短文案，不复用 legacy Tailwind card。它提供 refresh、primary action、可选 launch、cancel、copy safe details（若 V2 已有受控 clipboard 入口，否则本任务不新增 clipboard 能力）与 open logs。具体操作以现有安装器合同和平台适用性为准，不能为了视觉一致虚构可用按钮。

## 5. Agent 图标与双栏布局

- 列表/详情 `<img>` 保持同一 `getAgentIcon` 来源和现有 alt 语义。
- 从 `.fy-agent-selector-icon`、`.fy-agent-detail-icon` 移除人为浅白 background；同时移除仅用于白色卡底的边框/内阴影，保留透明 padding、object-fit、圆角几何和 TRAE native-size 规则。
- `.fy-agent-layout` 设置交叉轴 start alignment，使两个 panel 使用自身内容高度。保持现有 760px 单栏断点；不添加固定高度或新的嵌套滚动容器。

## 6. Skills/MCP 六应用图标

新增 V2-owned typed app asset map：

```ts
const supportedAppIconById: Record<SupportedAppId, string> = { ... };
getSupportedAppIcon(id: SupportedAppId): string;
```

- Claude/Codex 复用 V2 已有审查资产；Gemini/Grok Build/OpenCode/Hermes 使用仓库已有本地资产字节的 V2-owned copy，不运行时引用 legacy 路径、不下载远程图片。
- `AssignmentPanel` 在文字前渲染 `alt="" aria-hidden="true"` 的装饰图标；Switch 的 accessible label 保持现有 `${app.label} ${labelSuffix}`。
- 同一 shared panel 同时覆盖 Skills 与 MCP，响应式仍只有一个六开关语义面板。
- 类型测试要求 `Record<SupportedAppId, string>` 完整；资产 decode/path 测试和 browser test 阻止缺图、破图或重复 panel。

## 7. Test data and validation flow

```text
Rust exact catalog tests
  -> V2 runtime parser/port tests
  -> pure shared installer state tests
  -> Agent + Assignment component tests
  -> fake-Tauri exact IPC/event tests
  -> four-viewport Playwright geometry
  -> renderer/Rust full focused gates
  -> real Tauri browser-launch observation
```

Mocks prove only renderer/IPC behavior。真实系统浏览器由原生 HIL 证明；真实 Codex 安装只有在存在安全、明确、可恢复的测试状态时才执行，否则保留为未验证风险，不能从 mock、Rust unit 或本地状态读取推导成功。

## 8. Rollback and stopping conditions

- 目录 v2、共享 installer core、V2 installer UI、图标/布局分别形成可审查 diff/commit rollback point；不使用破坏性 Git 操作。
- 若共享核心迁移导致 legacy installer 测试行为变化，先恢复兼容后再接 V2，不维护两套有分歧的状态规则。
- 若真实 external launch 失败且根因需要放宽交互用户身份、URL 校验或执行任意命令，停止并保留受控错误，而不是添加不安全 fallback。
- 若真实安装会覆盖用户现有 Codex Desktop、需要提升新权限或缺少可恢复测试状态，不执行该 HIL；报告未验证边界。
- 未通过 exact contract、secret-negative、single-listener/snapshot-order 或四视口 gate 时不得归档任务。
