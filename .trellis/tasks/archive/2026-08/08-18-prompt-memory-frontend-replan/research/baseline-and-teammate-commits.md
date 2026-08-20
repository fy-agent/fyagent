# 基线与相关提交评估

## Git 工作方式

| 角色 | 引用 | 用途 |
| --- | --- | --- |
| 合并基线 | `origin/dev/laiyongjie` @ `59acaceb` | 功能分支从这里切出，PR 也合回这里 |
| 工作分支 | `cursor/prompt-memory-frontend-align-06e7` | 只在这条分支上开发；不设置、不推送到 `origin/dev/laiyongjie` |
| 徐坤线 | `origin/dev/xk` @ `a8348ff9` | 已是基线祖先，不必再单独 merge |

禁止：在 `dev/laiyongjie` 上直接 commit / push。  
要求：后续合并以 `dev/laiyongjie` 为 base。

`origin/dev/xk` ⊂ `origin/dev/laiyongjie`（`merge-base --is-ancestor` 为真）。徐坤的 MCP 发现与中国精选已经在基线里，评估时按基线上的 SHA 看即可。

## 基线里两页的现状

提示词 `src/v2/pages/prompts/Page.tsx`：

- 已接 `FeaturePorts.prompts`，七个应用，单应用单条启用。
- 顶栏 toolbar 用原生 `<select>` 切应用。
- 三栏：列表 / 只读详情 / 当前 live file；编辑走 Dialog。
- 已用 `SplitPanes`、`SelectionLens`、`usePrimaryBlocker`。

记忆 `src/v2/pages/memory/Page.tsx`：

- 已接 `FeaturePorts.memory`：OpenClaw/Hermes 四个长期文件 + OpenClaw 每日。
- 页签只有「长期记忆 / 每日记忆」。合同禁止会话扫描和跨工具同步 UI。
- 三栏：四个扁平资源 / 编辑器 / 「记忆信息」定义列表（和中间栏重复）。
- 标题写「OpenClaw 与 Hermes」，但每日只属于 OpenClaw。

这就是「规划太粗糙」的现场：底盘已经跟上 Agent/MCP，信息架构还停在 CRUD 壳。

## 赖永杰线上、今晚必须吃进评估的提交

基线作者以 `pythonrust` / `Kafu` 为主。相关范围：

| SHA | 日期 | 提交 | 评估 |
| --- | --- | --- | --- |
| `59acaceb` | 08-18 | route HTTP(S) jumps through one ExternalLinkButton | 两页若出现外链，必须走 `ExternalLinkButton`，不要再写一套 `openExternal` |
| `11f81b77` | 08-18 | keep split-pane content inside catalog-style scrollports | 分栏子节点在 pane 内滚动；page CSS 禁止 `height:100%` 且无 overflow |
| `feb0b8dc` | 08-18 | share resizable split panes across V2 feature pages | 两页已用；继续复用，不要退回页面私有栅格 |
| `497e6e62` | 08-18 | keep catalog geometry after MCP workspace CSS | 改 Memory/Prompt workspace 时不得改坏 `--fy-catalog-rail-width` |
| `4b5bde0a` | 08-18 | Agent/Models catalog panes scroll and resize independently | 应用/来源轨应改用 `CatalogMasterDetail`，与 Agent 同几何 |
| `5f933dc8` | 08-18 | keep all six primary pages alive | 已在基线。隐藏页不得再挂 `useBlocker`；继续用 `usePrimaryBlocker` |
| `10a8c84b` `0fa755fd` `9bb391d0` `606c35bd` | 08-17/18 | SelectionLens | 应用轨、页签、列表继续用它，禁止 `<select>` 当主切换 |
| `8ddef12d` | 08-15 | connect prompts and memory management | 产品边界仍是现有 command；今晚只改 IA，不扩 port |

不纳入本任务 diff：Codex 安装器、Windows helper、MSVC、WorkBuddy 模型写、Qoder/TRAE。它们已在基线，原样保留。

## 徐坤线上、已在基线里的相关提交

| SHA | 日期 | 提交 | 评估 |
| --- | --- | --- | --- |
| `9ae4fea6` | 08-18 | curated discovery and copyable install paths | 管理/检视分模式；路径可复制；搜索放在 workspace |
| `a8348ff9` | 08-18 | expand China catalog and split install vs configure | 启用（激活）和编辑（配置）拆开 |
| `aeaeeb55` | 08-18 | merge MCP catalog discovery from xk | 合入记录；本任务不再 merge `dev/xk` |

徐坤任务写明不改 Prompts/Memory。我们只借模式，不改 MCP 页，也不做提示词市场。

## 对照结论

| 维度 | 基线提示词/记忆 | Agent / MCP 已做到 | 今晚要对齐到 |
| --- | --- | --- | --- |
| 产品对象 | 七应用 Prompt；四长期 + OpenClaw 每日 | 各页自己的 bounded port | 保持现有 port，不回到 08-12 跨 Agent 原型 |
| 切换器 | `<select>` / 扁平四文件 | CatalogRail + 品牌图标 | 应用/来源改目录轨 |
| 编辑 | Dialog；中间栏只读 | 详情栏就地做事 | 中间栏就地编辑 |
| 第三栏 | live file / 记忆信息常驻 | 检视可收或并进详情头 | 检视折叠，不与编辑抢宽度 |
| 搜索 | 提示词在页级 toolbar | workspace 内搜索 | 搜索移进工作区 |
| 外链 | 记忆「打开目录」走 port | `ExternalLinkButton` | HTTP 一律走共享按钮 |
| 分栏 | 已用 SplitPanes | 同底盘 + scrollport 合同 | 遵守 `11f81b77`，不新写栅格 |
