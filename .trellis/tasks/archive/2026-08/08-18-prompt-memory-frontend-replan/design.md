# 提示词与记忆前端重规划 — 设计

## Boundaries

- 可写：`src/v2/pages/prompts/**`、`src/v2/pages/memory/**`、必要时 `src/v2/shared/assets/apps` 的 OpenClaw 图标映射、对应 `tests/v2/**`、`tests/v2-browser/**`、`.trellis/spec/frontend/v2-prompts-memory.md`。
- 复用、不改行为：`CatalogMasterDetail`、`SplitPanes`、`SelectionLens`、`ExternalLinkButton`、`usePrimaryBlocker`、`FeaturePorts.prompts` / `memory`。
- 不改：`src-tauri/**`、port 签名、查询 key、MCP/Skills/Agents/Models 页面、router、TopBar。
- Git：从 `origin/dev/laiyongjie` 切出；PR 合回 `dev/laiyongjie`；工作分支不 track 该远端。

## Page maps

### 提示词

```
┌ header: 提示词 · 按应用管理可启用的提示词     [从文件导入] [新建提示词] ┐
├─────────────────────────────────────────────────────────────────────┤
│ CatalogRail          │ 工作区                                       │
│ Claude · 1 条启用    │ 搜索名称、描述、内容、ID                     │
│ Codex · 0            │ ┌ 列表 ────────┬ 就地编辑 ─────────────────┐ │
│ Gemini               │ │ 名称         │ 标题 / 启用 / 保存        │ │
│ Grok Build           │ │ 描述 · 启用  │ 正文（占满剩余高度）      │ │
│ OpenCode             │ │              │ 名称+描述（次行）         │ │
│ OpenClaw             │ │              │ 折叠：当前使用的内容      │ │
│ Hermes               │ └──────────────┴───────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────┘
```

| 区域 | 职责 | 禁止 |
| --- | --- | --- |
| 应用轨 | 切换 `PromptAppId`，带动 `usePrompts` / `usePromptLiveFile` | `<select>`、Codex 胶囊、把轨理解成「当前 Agent 设置」 |
| 列表 | 选中编辑对象 | 用 Switch 代替选中 |
| 编辑 | 草稿；保存后权威回读 | Dialog 编辑；保存即启用 |
| 检视 | 只读 live file，默认折叠 | 常驻第三栏再放一份 textarea |

紧凑宽度走现有 `SPLIT_STACK_QUERY`：轨在上，列表/编辑上下叠。检视保持折叠。

OpenClaw 图标：若没有现成 asset，用字母标记 + `BrandIconFrame` 的 surface 背景，并在 spec 记一笔；不要从 MCP/Agent 图标里误借。

### 记忆

```
┌ header: 记忆 · 管理 OpenClaw / Hermes 的长期文件，以及 OpenClaw 每日记录 ┐
├ 长期记忆 | 每日记忆                                                     ┤
├─────────────────────────────────────────────────────────────────────────┤
│ 来源轨                 │ 工作区                                         │
│ OpenClaw               │ 长期：无搜索（四条固定）                       │
│  MEMORY.md  尚未创建   │ 每日：搜索文件名/正文                          │
│  USER.md    可编辑     │ ┌ 列表 ──┬ 就地编辑 + 头信息 ───────────────┐ │
│ Hermes                 │ │        │ Hermes 开关 / 字符数 / 打开目录  │ │
│  MEMORY.md             │ │        │ 可复制路径                       │ │
│  USER.md               │ └────────┴──────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────────┘
```

长期不再使用三栏。每日从三栏收成两栏。说明、限额、打开目录进编辑头。

## Contracts

保持 `v2-prompts-memory.md` 的 port 与失败矩阵，只追加 IA：

```ts
// 页面结构，不进 wire DTO
type PromptWorkspace = {
  rail: PromptAppId;
  list: "library";
  editor: "inline";
  inspector: "collapsed-live-file";
};

type MemoryWorkspace = {
  tab: "long-term" | "daily";
  rail: "openclaw" | "hermes" | "daily-files";
  editor: "inline";
  inspector: "none"; // 元数据进编辑头
};
```

页头固定句：

- 提示词：按应用管理可启用的提示词，并查看该应用当前使用的内容。
- 记忆：管理 OpenClaw 与 Hermes 的长期文件，以及 OpenClaw 的每日记录。

## State

- 应用/文档/页签/选中仍是页面 local state；query 继续按现有 key 分区。
- 提示词 `isDirty`：名称、描述、正文。目标集合本轮仍由后端「单条启用」表达，不引入多目标草稿。
- 记忆 `isDirty`：当前资源正文。切文档/页签走现有 `requestTransition`。
- 写锁、权威回读、回读失败警告、启用项禁删，原样保留。
- 新建提示词：中间栏 `mode: "new"`，不先写入列表；取消或确认放弃后丢掉。

## Compatibility

- 浏览器 native-only、无种子数据。
- 不改 standalone 是否生成的 CI 约定（基线已 gitignore 预览 HTML）。
- 回滚：还原两页、测试和 spec IA 段；shared 底盘保持基线原样。

## Risks

- OpenClaw 无现成品牌图。必须有可见回退，否则目录轨会缺一项。
- 就地编辑加长中间栏后，容易再画出 pane。验收盯 `11f81b77` 的 scrollport 合同。
- 评审可能把 08-12 跨 Agent 原型当成「还没做完」。PR 写清：基线产品是 native 边界，今晚只升级 IA。
