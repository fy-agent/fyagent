# Prompt / Memory 前端详细设计概要

## 1. 文档状态与设计锁

- 阶段：设计冻结完成；实现与运行验收已通过
- 上游：`prd.md`、`technical-design-overview.md`
- 静态评审：`reviews/product-design-review.md`、`reviews/technical-architecture-review.md`
- 证据等级：`code_audit`
- 状态：`DESIGN_FREEZE=2026-08-12`；`DETAILED_DESIGN_REVIEW=PASS`

本文件定义实现所需的文件所有权、类型、状态转换、交互和测试。严重评审意见关闭并记录后，本文与 `technical-design-overview.md` 一起冻结；执行 Agent 只实现冻结合同，不重新解释产品范围。

## 2. 产品与视觉锁

### 2.1 产品模型

- Prompt 是可组合、可复用、长期生效的规则库；不是单次聊天 Prompt，也不是 Codex 设置页。
- Memory 是 Agent 已留下内容的来源浏览、长期草稿提炼和同步任务预览；不是实时文件管理器。
- Prompt 多规则可同时启用，每条规则可多选目标。
- Memory 顶层固定为长期记忆、每日记录、会话记录；只有 4 个经过验证的目标组可生成同步预览任务。
- 所有写操作停留在 React state；`preview` 与 durable result 严格分开。

### 2.2 视觉方向与 token

产品 archetype 锁定为 `Developer Tool / AI Product`。保留现有深蓝、紧凑、三栏开发工具界面，不重做 Shell。

使用现有 token，不修改 `tokens.css`：

| 语义 | Token / 当前值 |
| --- | --- |
| 背景 | `--fy-bg: #172d43` |
| 主文字 | `--fy-text: #f5f8fc` |
| 次级文字 | `--fy-text-secondary: #d1dde8` |
| 弱文字 | `--fy-text-tertiary: #a9bdcf` |
| 选中 | `--fy-selected: rgba(25, 103, 181, 0.76)` |
| 焦点 | `--fy-focus: #70beff` |
| 边框 | `--fy-border: rgba(206, 224, 240, 0.18)` |
| 圆角 | 页面既有 9/10/11/16px 层级 |
| 字体 | 系统 UI 字体；正文编辑区使用既有 mono stack |

新增可见 prototype 状态、路径状态和 pending task 只使用现有文字、边框、warning/success token；不增加全局颜色或新的视觉语言。

## 3. 文件所有权与预计改动

### 3.1 执行线路 1：Prompt 独占

| 文件 | 预计改动 |
| --- | --- |
| `src/v2/pages/prompts/Page.tsx` | 显式 saved baseline、未保存新建项、page-level `useBlocker`、canonical 资源计数、prototype 可见文案、放弃/保存/启用转换 |
| `src/v2/pages/prompts/page.css` | prototype 状态文本和必要的紧凑响应式样式；不改全局 token |
| `src/v2/pages/prompts/prototype.ts` | 仅在冻结类型需要时同步 seed；保留 9 条匿名规则和 2 条默认启用 |
| `tests/v2/pages/prompts/Page.test.tsx` | data router harness、saved baseline、路由 guard、多规则/多目标/缺失文件等聚焦测试 |

### 3.2 执行线路 2：Memory 独占

| 文件 | 预计改动 |
| --- | --- |
| `src/v2/pages/memory/prototype.ts` | 新增 resource/local/provenance/revision/preview task 类型；去掉无证据的 durable“已同步”；Daily/Session 的原型编辑策略锁为只读 |
| `src/v2/pages/memory/Page.tsx` | 标题/正文/目标 saved baseline、未保存提炼项、page-level `useBlocker`、只读策略、provenance 展示、路径状态、逐目标 pending task、revision 失效规则 |
| `src/v2/pages/memory/page.css` | prototype 状态、provenance、路径状态和 task 状态样式；不改全局 token |
| `tests/v2/pages/memory/Page.test.tsx` | data router harness、Daily/Session 只读、提炼来源链、首次保存、目标 dirty、pending task、revision 失效和路由 guard |

### 3.3 执行线路 3：共享合同与 standalone 独占

| 文件 | 预计改动 |
| --- | --- |
| `src/v2/shared/config/agentTargets.ts` | 显式 Prompt path state、Prompt canonical key、Memory 资格字段、派生 writable ids、canonical 分组、无效 lookup 显式返回 |
| `tests/v2/shared/config/agentTargets.test.ts`（新增） | 7 个 Prompt 资源 / 8 个实例 / 4 个 Memory 目标、重复选择去重、OpenClaw 合并、无效 lookup |
| `scripts/build-v2-preview.mjs` | 从 `dist/index.html` 解析并内联当前 production entry graph 的全部直接 script/stylesheet；不再按文件体积猜入口 |
| `tests/v2/scripts/build-v2-preview.test.ts`（新增） | builder 纯函数/临时目录模块测试：无 entry fail-fast、多 stylesheet 顺序、path escape 拒绝、direct entry 全量内联 |

以下已有改动由本轮保留，但三个执行 Agent 不再修改：

- `src/index.html`
- `package.json`
- `playwright.v2.config.ts`
- `src/v2/app/styles/tokens.css`
- `src/v2/app/styles/globals.css`
- `src/v2/app/styles/index.css`
- `src/v2/app/styles/v4-shell.css`
- `tests/v2/app/router-shell.test.tsx`
- `tests/v2-browser/shell.spec.ts`

### 3.4 主 Agent 独占

| 文件 | 预计改动 |
| --- | --- |
| `tests/v2-browser/prompt-memory.spec.ts`（新增） | 模块完成后新增跨路由 dirty guard、Prompt/Memory 关键交互和四视口运行验收，不修改通用 shell spec |
| `FyAgent-前端交互预览.html` | 仅由最终 `build:renderer` 重新生成 |
| `research/prompt-cross-agent-1586x992.png` | 最终重新生成 `runtime_screenshot` |
| `research/memory-cross-agent-1586x992.png` | 最终重新生成 `runtime_screenshot` |
| `research/verification.md` | 最终只记录本轮新鲜命令、数量和边界 |
| 本任务设计/评审/计划/`task.json` | 状态、决策、实际结果同步 |
| `.trellis/spec/frontend/v2-shell.md` | 最终只同步 Prompt/Memory executable contract 与测试结果，不改导航/Shell/其他页面合同 |

## 4. 共享 TypeScript 合同

### 4.1 目标资源

```ts
export type PromptPathState = "exists" | "create-on-enable";

export type MemorySyncEligibility =
  | "source-only"
  | "verified-rule-bridge"
  | "verified-native";

export interface AgentTargetDefinition {
  id: AgentTargetId;
  toolId: AgentToolId;
  name: string;
  scopeLabel: string;
  instanceNames: readonly string[];
  promptFile: string;
  promptPath: string;
  promptCanonicalResourceKey: string;
  promptPathState: PromptPathState;
  memoryDestination: string;
  memorySyncEligibility: MemorySyncEligibility;
  detected: boolean;
}
```

字段规则：

- `promptCanonicalResourceKey` 只代表 Prompt instruction resource，绝不用于把 Memory 的 `MEMORY.md` / `USER.md` 当成同一个文件。
- Claude：`verified-rule-bridge`。
- 两个 OpenClaw workspace、Hermes：`verified-native`。
- Codex、Gemini、OpenCode：`source-only`。
- Gemini/OpenCode Prompt path state 是 `create-on-enable`；其余为 `exists`。

### 4.2 Canonical Prompt 分组

```ts
export interface CanonicalPromptTargetGroup {
  key: string;
  primaryTargetId: AgentTargetId;
  targetIds: AgentTargetId[];
  instanceNames: string[];
}

export function groupPromptTargetsByCanonicalResource(
  targetIds: readonly AgentTargetId[],
): CanonicalPromptTargetGroup[];

export function agentTargetById(
  id: AgentTargetId,
): AgentTargetDefinition | undefined;
```

算法：

1. 按输入顺序 lookup，跳过无效/重复 ID。
2. 按 `promptCanonicalResourceKey` 建 `Map`，首次出现决定 group 顺序和 primary target。
3. 对每组 `targetIds` 与 `instanceNames` 去重并保序。
4. 目标文件数取 group 数；覆盖实例数取所有 group 的实例并集。
5. 无效 lookup 不静默回退 Codex；页面显示“未知来源”或忽略无效选择，测试直接验证 `undefined`。

`memoryWritableTargetIds` 从资格字段派生，不再维护第二份手工名单。

## 5. Prompt 详细设计

### 5.1 页面局部状态

```ts
interface PromptDraftState {
  value: PromptPrototypeItem;
  baseline: PromptPrototypeItem | null;
  hasSavedBaseline: boolean;
}

type PromptPageState = {
  items: PromptPrototypeItem[];
  selectedId: string;
  draft: PromptDraftState;
  transientNewId: string | null;
  query: string;
  feedback: string;
};
```

`items` 是前端已保存快照。新建条目为了在左栏可见，可以作为 transient row 放入 `items`，但 `transientNewId` 与 `baseline=null` 使它首次保存前始终 dirty。

### 5.2 Dirty 判定

```text
isDirty =
  !draft.hasSavedBaseline
  OR name/description/content/enabled 与 baseline 不同
  OR targetIds 集合与 baseline 不同
```

- target IDs 比较忽略顺序，仅比较集合。
- `query`、选中态和 feedback 不计 dirty。
- 保存成功后 baseline 使用保存后的深拷贝，`transientNewId=null`。

### 5.3 Prompt 状态转换

| 事件 | 前置 | 转换 | 反馈 |
| --- | --- | --- | --- |
| `SELECT(id)` | id 存在；无 dirty 或确认放弃 | 若当前是未保存新建项则移除；载入目标 baseline | 清空 |
| `CREATE` | 无 dirty 或确认放弃 | 移除旧 transient；新建 disabled/无目标项；baseline=null | “先填写内容并选择注入目标” |
| `EDIT(field)` | 当前存在 | 仅改 draft | 清空 |
| `TOGGLE_TARGET(id)` | 不是已启用规则的最后目标 | 改 draft targetIds | 清空 |
| `SAVE` | 名称非空；enabled 时至少一目标 | insert/update items；写 baseline；清 transient | “已保存到前端预览；未写入本机文件” |
| `TOGGLE_ENABLED(id)` | saved item；启用时 saved targets 非空 | 原子更新 saved item.enabled；若为当前项，只同步 `draft.value.enabled` 与 `baseline.enabled`，保留 draft/baseline 其他字段 | “已加入/移出前端组合” |
| `DISCARD` | dirty | 新建项移除；已有项恢复 baseline | 无 |
| `ROUTE_LEAVE` | pathname 改变且 dirty | confirm=false: blocker.reset；true: blocker.proceed | 浏览器确认框 |

开关规则：

- 不允许用未保存 target 草稿绕过启用校验；启用读取最后一次 saved targets。
- 未保存新建项必须先保存，才可启用。
- 启用/停用一条规则不修改其他规则。
- 开关是 saved rule 的即时前端提交，不单独制造 dirty。若名称/正文/目标已经 dirty，开关后这些草稿仍保留；随后放弃只恢复未保存字段，并保留已经提交的 enabled 状态。

### 5.4 Prompt 组件责任

| 区域 | 输入 | 输出 |
| --- | --- | --- |
| Header | prototype 状态、create handler | 新建事件；持续可见“前端原型 · 未读写本机文件” |
| Library | filtered saved/transient items、selectedId | select、toggle enabled |
| Editor | draft value、dirty、validation | edit、save |
| Target inspector | shared catalog、draft targetIds | toggle target；canonical 文件/实例计数 |
| Page guard | `isDirty`、current/next pathname | `reset` 或 `proceed` |

不拆新组件文件；现有单页规模内用局部渲染片段与纯 helper 即可，避免并发新增抽象。

### 5.5 Prompt 显示逻辑

- `promptPathState=exists` → “已存在”。
- `create-on-enable` → “启用时创建”。
- 页面说明：“在前端预览中组合长期规则；本轮不会写入本机 Agent 文件”。
- 托管区块说明使用未来条件句：“接入真实同步后，同一路径只执行一次，并保护托管区块外内容”。
- 搜索结果为空 → “没有匹配的提示词”；不改变 selected/draft。

## 6. Memory 详细设计

### 6.1 类型

```ts
export type MemoryResourceState =
  | "exists"
  | "missing"
  | "frontend-draft";

export type MemoryLocalState =
  | "source"
  | "saved-preview"
  | "changes-pending"
  | "managed-by-prompts";

export interface MemoryProvenance {
  sourceItemId: string;
  sourceTargetId: AgentTargetId;
  sourceToolId: AgentToolId;
  sourcePath: string;
  sourceUpdatedAt: string;
  capturedAt: string;
  sourceSummary: string;
}

export interface MemoryPreviewTargetTask {
  targetId: AgentTargetId;
  sourceRevision: number;
  previewState: "pending";
  durableState: "not-run";
  createdAt: string;
  error: null;
}
```

`MemoryPrototypeItem` 在现有字段上调整：

```ts
interface MemoryPrototypeItem {
  // identity/source/display fields 保留
  writable: boolean;              // 来源能力事实
  editableInPrototype: boolean;   // 本轮页面策略
  resourceState: MemoryResourceState;
  localState: MemoryLocalState;
  revision: number;
  provenance: MemoryProvenance | null;
  syncTargetIds: AgentTargetId[]; // 最后一次保存的选择
  previewTasks: MemoryPreviewTargetTask[];
  owner: "memory" | "prompts";
}
```

- 删除 prototype 级 durable `已同步` 表达。
- owner=prompts 时 localState 固定 `managed-by-prompts`。
- 有已保存目标但未执行真实同步时仍是 `saved-preview` / 待同步语义，不得显示已同步。

### 6.2 页面局部状态

```ts
type MemoryPageState = {
  category: MemoryCategory;
  itemsByCategory: Record<MemoryCategory, MemoryPrototypeItem[]>;
  selectedIds: Record<MemoryCategory, string>;
  queries: Record<MemoryCategory, string>;
  draftTitle: string;
  draftContent: string;
  draftTargetIds: AgentTargetId[];
  baseline: MemoryPrototypeItem | null;
  transientPromotedId: string | null;
  feedback: string;
};
```

### 6.3 可编辑判定与 dirty

```text
editorReadOnly =
  category !== longTerm
  OR owner === prompts
  OR editableInPrototype === false

isDirty =
  transientPromotedId === selectedItem.id
  OR title/content 与 baseline 不同
  OR draftTargetIds 与 baseline.syncTargetIds 不同
```

- `writable` 只描述来源能力，不直接开放编辑器。
- Daily/Session 本轮 `editableInPrototype=false`，textarea readOnly，保存按钮显示“只读来源”。
- Prompt-owned 永远只读。

### 6.4 Memory 状态转换

| 事件 | 前置 | 转换 | 反馈 |
| --- | --- | --- | --- |
| `SELECT_SOURCE(id)` | 无 dirty 或确认放弃 | 移除未保存提炼项或恢复 baseline；载入来源 | 清空 |
| `SWITCH_CATEGORY` | 无 dirty 或确认放弃 | 保留各分类 query/selected；载入目标 baseline | 清空 |
| `EDIT` | 可编辑长期条目 | 改 draft，local view 为 changes pending | 清空 |
| `TOGGLE_TARGET` | 长期 memory-owned | 改 draft targets，计入 dirty | 清空 |
| `SAVE` | 可编辑、标题非空且 `isDirty=true` | revision + 1；保存内容/目标；清空旧 previewTasks；写 baseline；清 transient | “已保存到前端预览；尚未写入本机文件” |
| `SAVE`（无变化） | `isDirty=false` | 按钮 disabled / handler no-op；不增 revision、不清 tasks、不改 baseline | 无 |
| `PROMOTE` | category=daily/sessions；无 dirty | 原子创建 longTerm transient：切到 longTerm、选中新 id、baseline=null、revision=0、resourceState=frontend-draft、localState=changes-pending、syncTargetIds=[]、写完整 provenance；原来源不变 | “已生成长期记忆草稿；原始记录保持不变” |
| `PREVIEW_SYNC` | memory-owned；has saved baseline；无 dirty；saved targets >=1 | 为每个 saved target 建 pending task，sourceRevision=当前 revision；不改 durable 状态 | “前端预览：已生成 N 个待执行任务；未写入本机文件” |
| `SCAN_PREVIEW` | 无 dirty 或确认放弃 | 重载当前 baseline；不访问磁盘 | “模拟扫描：6 个工具、8 个 Agent 实例” |
| `ROUTE_LEAVE` | pathname 改变且 dirty | cancel reset；confirm proceed | 浏览器确认框 |

### 6.5 Revision 与 preview task 失效

- 每次真正保存发生内容、标题或目标变化时 `revision += 1`。
- clean save 始终 disabled/no-op，不递增 revision，也不清空仍绑定当前 revision 的 tasks。
- 保存新 revision 时清空旧 `previewTasks`，避免旧任务冒充当前内容。
- 只编辑未保存 draft 不清空 saved item 的任务；UI 在 dirty 时隐藏/禁用生成按钮并提示先保存，旧任务标记为基于 revision N。
- 放弃 draft 恢复 baseline，不改变其 revision 和 tasks。
- 对同一 saved revision 重新生成任务时用当前 target IDs 完整替换旧 tasks，避免重复。

### 6.6 提炼 provenance

从 selected daily/session 生成：

```ts
provenance = {
  sourceItemId: selected.id,
  sourceTargetId: selected.sourceTargetId,
  sourceToolId: selected.toolId,
  sourcePath: selected.path,
  sourceUpdatedAt: selected.updatedAt,
  capturedAt: "刚刚",
  sourceSummary: selected.purpose,
};
```

- 新草稿自己的 `path` 是“FyAgent 前端草稿”，`resourceState=frontend-draft`；不覆盖 provenance 的原路径/时间。
- 右栏新增“提炼自”区域，显示来源标题、path、sourceUpdatedAt；不创建真实跳转或文件打开动作。
- 原 daily/session item 引用和内容不变。

### 6.7 Memory 组件责任与显示逻辑

| 区域 | 责任 |
| --- | --- |
| Header | 标题、模拟扫描、可见 prototype 状态 |
| Tabs | 分类切换；dirty guard；标准 tab 语义 |
| Source library | 分类查询、来源/格式/数量、本地状态 |
| Editor | 长期条目编辑或 Daily/Session/Prompt-owned 只读预览 |
| Inspector details | 工具、来源、存储、位置、来源能力、本轮操作、路径状态、数量、更新时间 |
| Provenance | 仅提炼草稿/条目显示来源链 |
| Sync targets | 仅 memory-owned longTerm 显示 4 个 verified group |
| Preview tasks | 每目标显示“待执行 · 未写入”，注明 source revision |

路径状态：

- `exists` → “已存在”。
- `missing` → “未发现”。
- `frontend-draft` → “前端草稿 · 未创建文件”。

来源能力与本轮操作分开：例如 Daily 可以显示来源支持读写，但“本轮操作：只读提炼”，避免能力事实和当前产品策略混淆。

## 7. Route dirty guard 合同

React Router 7.18.2 的准确页面用法：

```ts
const shouldBlockNavigation = useCallback<BlockerFunction>(
  ({ currentLocation, nextLocation }) =>
    isDirty && currentLocation.pathname !== nextLocation.pathname,
  [isDirty],
);
const blocker = useBlocker(shouldBlockNavigation);
```

当 `blocker.state === "blocked"`：

- `window.confirm` 返回 false → `blocker.reset()`。
- 返回 true → `blocker.proceed()`。

限制：

- 每页只注册一个 blocker；两个页面不会同时挂载。
- 只覆盖 SPA pathname 变化，不覆盖刷新、关窗、文件协议外链或系统窗口关闭。
- 不修改 PrimaryNav、router、AppShell，也不新增 shared hook。
- 页面测试必须用 `createMemoryRouter + RouterProvider`，不能用直接 render 或仅 `MemoryRouter`。

## 8. 用户反馈、空状态和错误状态

### 8.1 本轮可触发

| 情况 | 反馈/行为 |
| --- | --- |
| Prompt 名称为空 | 保存 disabled 或本地字段校验；不改 saved state |
| Prompt 启用无 saved target | “请先选择并保存至少一个注入目标” |
| 已启用 Prompt 移除最后目标 | 拒绝并提示先停用 |
| Memory 同步无 saved target | “请先选择并保存至少一个同步目标” |
| Memory 新提炼/dirty 后同步 | “请先保存当前修改，再生成同步预览” |
| 搜索无结果 | 列表内明确空状态，不清选中项 |
| dirty 内部或路由离开 | 单次确认；取消不改变任何 saved state |
| invalid target lookup | 显示“未知来源”或忽略选择；不回退 Codex |
| prototype scan/sync | 明确“模拟/前端预览/未写入” |

### 8.2 未来状态位置

loading、partial、permission denied、conflict、failed、unsupported 只在未来 data source projection 中出现；本轮不制造假失败按钮或假后端 toast。

## 9. Standalone 详细设计

### 9.1 当前承诺

- production router 当前只有 DEV UI Lab 使用 lazy import；Prompt/Memory 和其余 production 页面均为静态 entry graph。
- 本轮 builder 解析 `dist/index.html` 中全部带 `src` 的 module entry script 与 `rel=stylesheet` 直链，按 HTML 顺序内联。
- 对 entry bundle 中当前 Vite `new URL(asset, import.meta.url)` 资产继续 data URL 内联。
- 生成后不得留下指向 `dist/assets` 的 entry script/stylesheet 请求。

### 9.2 不扩大承诺

- 不修改 router 消除未来 dynamic imports。
- 若将来 production route 出现 lazy chunk，需新增递归 import graph 内联或改成可携带资源包；本轮不声称支持任意未知 dynamic graph。
- 不手工编辑 standalone 生成物。

### 9.3 Fail-fast

- `dist/index.html` 没有 entry script 时抛出明确错误。
- stylesheet 可为 0 或多个；多个按顺序合并。
- 解析到的本地 entry path 必须留在 `dist` 内，禁止 `../` 逃逸。
- final build/readback 负责验证 `file://` 实际打开。

### 9.4 可测试边界

- `scripts/build-v2-preview.mjs` 导出不触发构建的解析、路径解析与 build 函数；只有被 Node 作为入口执行时才写真实 `dist` / standalone 文件。
- 模块测试使用临时目录和最小 `index.html`/asset fixtures，不运行 Vite build，也不改真实 `dist` 或生成物。
- 模块测试必须证明：没有 module entry 明确失败；多个 stylesheet 保持 HTML 顺序；全部直接 module scripts 被内联；本地引用逃出 `dist` 时拒绝。

## 10. 单元测试设计

### 10.1 Prompt 模块命令

```bash
pnpm test:v2 -- tests/v2/pages/prompts/Page.test.tsx
```

至少覆盖：

1. 9 条 grounded rules、2 条默认启用、7 个 Prompt 资源/8 实例、2 个 create-on-enable。
2. 启用第三条不关闭另外两条。
3. 目标多选、保存、搜索；canonical 文件计数正确。
4. 已启用规则不能移除最后目标；无 saved target 不能启用。
5. clean 当前项开关即时提交后不 dirty、不触发离开确认。
6. 其他字段已 dirty 时切换开关不丢草稿；放弃草稿后仍保留已提交开关。
7. 新建项首次保存前始终 dirty；确认放弃后不留空 transient row。
8. 编辑后内部选择的取消/确认分支。
9. 路由离开的取消/确认分支；保存后离开不询问。
10. 可见 prototype 文案不暗示真实写入。

### 10.2 Memory 模块命令

```bash
pnpm test:v2 -- tests/v2/pages/memory/Page.test.tsx
```

至少覆盖：

1. 三类来源、6 工具/8 实例说明、4 个 verified target、prototype 状态。
2. 长期条目修改和目标修改都计 dirty，保存后 revision 增加。
3. 生成逐目标 pending tasks；durable 显示未执行；不存在“已同步”。
4. 新 revision 保存使旧 tasks 失效；放弃 draft 保留原 revision/tasks；clean save 不增 revision、不清 tasks。
5. Daily 与 Session textarea 只读，只能提炼。
6. 提炼原子切到 longTerm 并选中新 unsaved draft，初始目标为空、baseline=null、changes-pending；保留 source ID/target/tool/path/time/summary，原来源不变。
7. 提炼草稿未保存不能生成任务；保存后至少一个目标才能生成。
8. Prompt-owned 只读并指向 Prompt；exists/frontend-draft 路径状态可见。
9. 分类/来源/扫描/提炼 dirty guard。
10. 路由离开的取消/确认分支；保存后离开不询问。
11. 分类往返后每类 query/selected item 独立保留；无结果空状态不清除 saved/selected state。

### 10.3 共享合同 / standalone 模块命令

```bash
pnpm test:v2 -- tests/v2/shared/config/agentTargets.test.ts tests/v2/scripts/build-v2-preview.test.ts
```

至少覆盖：

1. 7 个唯一 Prompt canonical resources 覆盖 8 个唯一实例。
2. 重复 target ID 输入只生成一个 group。
3. OpenClaw 默认 group 同时覆盖 main/utility，群聊独立。
4. 4 个 Memory 目标资格正确，Codex/Gemini/OpenCode 排除。
5. Gemini/OpenCode 为 create-on-enable。
6. invalid ID lookup 不回退 Codex。
7. standalone 无 entry fail-fast，多 stylesheet 按 HTML 顺序内联。
8. standalone 内联全部直接 module entry scripts，拒绝逃出 `dist` 的本地路径。

每个执行 Agent 只能运行自己的上述命令；不得运行 `pnpm test:v2` 全集、browser、build、lint 或 typecheck。

## 11. 集成测试设计（全部模块完成后）

主 Agent 先核验三个模块的文件和单测真实输出，再第一次执行集成。

### 11.1 静态/单元集成

- `pnpm lint:v2`
- `pnpm typecheck:v2`
- `pnpm test:v2`
- `pnpm build:renderer`

### 11.2 浏览器

- `pnpm test:v2:browser`
- 新 `prompt-memory.spec.ts` 在四档 viewport 自动运行：
  - Prompt 编辑后点 Memory，取消保持 Prompt，确认后进入 Memory。
  - Prompt 多规则与目标保存可点。
  - Memory Daily/Session 只读、提炼、保存、目标多选和 pending tasks 可点。
  - 页面无横向溢出；主操作和反馈可见；console/page error 为空。
- standalone 模块测试验证 builder 纯逻辑；最终 browser/readback 验证生成 HTML 的 `file://` 路径、无 console/page error，并静态断言没有指向本地 `dist/assets` 的 entry script/stylesheet 残留。

### 11.3 失败修复顺序

1. 定位到 owning module。
2. 修复 owning files；先重跑该模块单测。
3. 模块绿后重跑受影响集成命令。
4. browser/build 失败不得用放宽断言或伪造 fixture 掩盖。

## 12. 验收证据类型

| 证据 | 等级 | 能证明 | 不能证明 |
| --- | --- | --- | --- |
| 设计/源码/测试静态审查 | `code_audit` | 合同、边界、断言存在 | 运行行为 |
| 模块 Vitest 输出 | `unit_test` | 单模块状态转换 | 完整路由/布局 |
| 完整 lint/typecheck/unit/build | `integration_test` | 跨模块编译与回归 | 视觉可达性 |
| Playwright 四视口 | `browser_runtime` | 点击、路由、几何、console | 原生 Tauri/真实文件写入 |
| 1586×992 页面截图 | `runtime_screenshot` | 本轮实际渲染快照 | 1:1 pixel diff |
| 基线到 `HEAD` 与 worktree 的 `src-tauri` diff 都为空 | `code_audit` | 多次提交后仍无后端 diff | 远端后端运行态 |

未运行 automated image comparison 时，严禁标记 `pixel_diff` 或“1:1 像素通过”。

## 13. 明确不修改

实现与最终修复均不得修改：

- `src-tauri/**`
- 数据库、Rust command、真实文件写入逻辑
- `src/v2/pages/agents/**`
- `src/v2/pages/models/**`
- `src/v2/pages/skills/**`
- `src/v2/pages/mcp/**`
- `src/v2/shared/config/navigation.ts`
- `src/v2/app/router.tsx`
- `src/v2/widgets/app-shell/**`
- `docs/images/视觉-1/**`
- `docs/images/视觉/**`

不新增全局 store、Context service、假 adapter、后端 mock 层、通用表单框架或跨页组件库。

## 14. 设计评审意见关闭映射

| 评审项 | 冻结落点 |
| --- | --- |
| P0 同步预览误标已同步 | 6.1、6.4：pending preview / durable not-run |
| P1 逐目标/partial 缺失 | 6.1、6.7：每目标 task；未来 durable result 独立 |
| P1 路由 dirty guard | 7：各 Page.tsx 单 blocker |
| P1 新建/提炼无 baseline | 5.1-5.3、6.2-6.4 |
| P1 provenance 丢失 | 6.6 |
| P1 Daily 可编辑 | 6.3-6.4 |
| P2 Memory path state | 6.7 |
| P2 prototype 文案 | 5.5、6.7、8 |
| P2 canonical 只靠人工 | 4.2 |
| 架构 P2 Prompt/Memory canonical 混淆 | 4.1-4.2：字段只服务 Prompt |
| 架构 P2 task revision | 6.5 |
| 架构 P2 standalone entry graph | 9 |
| 架构 P2 文档/ownership | 3、13；冻结前同步入口文档 |
| 架构 P3 invalid lookup | 4.2 |

以上均有明确 owner、状态转换和单元测试。详细设计评审通过后才允许开始代码实现。
