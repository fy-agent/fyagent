# 提示词与记忆前端重规划

## Goal

在 `dev/laiyongjie` 基线上，把提示词、记忆两页从「能用的原生 CRUD 壳」收到和 Agent 目录 / Skills / MCP 同一代信息架构。用户按应用或来源找内容、在中间栏改、在检视里看当前文件或资源元数据，而不是对着 `<select>`、编辑弹窗和永远展开的第三栏。现有 native port、单应用启用、四个长期文件和每日记忆的产品边界不变。

## Confirmed facts

- 合并基线是 `origin/dev/laiyongjie` @ `59acaceb`。`origin/dev/xk` @ `a8348ff9` 已是祖先，不必再合徐坤分支。
- 工作只发生在 `cursor/prompt-memory-frontend-align-06e7`。不在 `dev/laiyongjie` 上 commit / push。
- 两页已经接 `FeaturePorts`，已经用 `SplitPanes`、`SelectionLens`、`usePrimaryBlocker`。缺的是目录轨、就地编辑和检视折叠。
- `CatalogMasterDetail`、`ExternalLinkButton`、keep-alive、MCP「已安装 / 发现」都在基线里，可直接复用。
- 合同仍是 `.trellis/spec/frontend/v2-prompts-memory.md`：七个 `PromptAppId`、四个 `MemoryDocumentId`、OpenClaw `YYYY-MM-DD.md`、浏览器 native-only、禁止会话扫描和跨工具同步 UI。
- `supportedAppIconById` 没有 `openclaw`。应用轨需要补 OpenClaw 品牌或文档化的字母回退，不能空图标。

## Requirements

### 共享信息架构

- R1. 两页都是「目录轨 + 工作区」。工作区内部是「列表 + 就地编辑」。检视不得再作为永远展开的第三主栏。
- R2. 搜索只过滤当前工作区列表，放在工作区顶部，不和应用/来源轨抢同一条 toolbar。
- R3. 应用、来源、页签、列表选中一律走 `SelectionLens` 轨。禁止原生 `<select>` 充当主切换器。
- R4. 创建和编辑在中间栏完成。`Dialog` 只留给确认（放弃未保存、删除、启用项不可删的说明）。每页只保留一个实心主动作。
- R5. 空集合、无搜索结果、桌面能力不可用必须分开，并给出一个合法下一步。
- R6. 脏草稿在切换应用/文档/页签/条目/离页前确认。继续用共享 `ConfirmDialog` 和 `usePrimaryBlocker`，不用 `window.confirm`。
- R7. 页面 CSS 只做命名空间下的编辑高度与滚动。遵守 `11f81b77`：分栏子节点在 pane 内滚动，禁止无 overflow 的 `height: 100%`。不得改 `--fy-catalog-rail-width`，不得另起主题。
- R8. HTTP(S) 外链走 `ExternalLinkButton`。打开本机目录仍走 `ports.memory.openOpenClawDirectory`。
- R9. 不改 `FeaturePorts`、Rust command、ACL、查询 key 语义。浏览器继续 native-only，不种演示数据。

### 提示词页

- R10. 左轨用 `CatalogMasterDetail` 列出七个应用（Claude 默认），有图标、名称、该应用已启用条数。切应用等于切 query，不是「当前 Agent 设置胶囊」。
- R11. 工作区列表行至少：名称、描述或「暂无描述」、启用状态。选中是编辑对象，Switch 是启用，互不替代。
- R12. 中间栏就地编辑名称、描述、正文；保存后权威回读。保存不自动启用。
- R13. 「当前使用的内容」改为编辑头下的可折叠检视或页签，默认折叠。它仍只读，不是第二份可写正文。
- R14. 主动作「新建提示词」打开中间栏空草稿，不进 Dialog。「从文件导入」保留为次动作。
- R15. 继续遵守单应用单条启用、启用项先停用再删、写锁、回读失败警告。

### 记忆页

- R16. 页签仍是长期记忆 / 每日记忆。文案改为准确范围：长期覆盖 OpenClaw 与 Hermes；每日只覆盖 OpenClaw。
- R17. 长期左轨按来源分组（OpenClaw、Hermes），下列四个固定资源。禁止再写死「长期记忆 · 4」这种无信息计数。
- R18. 中间栏就地编辑；尚未创建的 OpenClaw 文件保持「保存才创建」。Hermes 启用开关和字符上限进编辑头，不放第三栏。
- R19. 删除「记忆信息」常驻第三栏。路径做成可复制文本；「打开 OpenClaw 工作区 / 记忆目录」留在编辑头或折叠检视。
- R20. 每日：搜索在工作区顶部；列表 + 就地编辑两栏。说明文字并进编辑头，不再单独占一栏。
- R21. 不恢复会话记录、提炼广播、跨工具同步目标或原型「已同步」。

## Out of scope

- 在 `dev/laiyongjie` 上直接开发或 push。
- 再单独 merge `dev/xk`。
- 新增 Tauri command、扩 Prompt 应用、扩 Memory 资源、接会话/跨工具同步。
- 把产品退回 08-12 跨 Agent 规则库原型。
- 做提示词发现市场。
- 改 Agent / Models / Skills / MCP 的用户可见行为（除非两页复用已有组件时的非行为重构，本轮不做）。
- 改顶栏导航、默认 `#/models`、keep-alive 策略。

## Acceptance Criteria

- [x] AC1. 功能分支是 `origin/dev/laiyongjie` 的后代；PR base 是 `dev/laiyongjie`；没有任何 commit 被推到 `dev/laiyongjie`。
- [x] AC2. 提示词：七应用目录轨，无 `<select>`；中间就地编辑；live file 可折叠；新建/导入/启用/删除/脏确认/写锁/回读警告仍在。
- [x] AC3. 记忆：来源分组轨；无「记忆信息」第三栏；Hermes 开关和上限在编辑头；每日两栏；页头文案不再暗示 Hermes 有每日文件。
- [x] AC4. 两页搜索都在工作区内；空 / 无结果 / native-only 仍可区分。
- [x] AC5. 900×600、1152×640、1232×700、1440×900 无横向溢出、主操作可见；分栏内容不画出 pane。Linux Playwright 已过；Windows 125%/150% 与 `mise run check` 见 `handoff.md`。
- [x] AC6. 不改 `src-tauri`，不改 port 签名，浏览器无种子数据。
- [x] AC7. 本环境已跑 `lint:v2`、`typecheck:v2`、`test:v2`、`test:v2:browser`、`build:renderer`、`format:check`。完整 `mise run check` 需 Windows/macOS，见 `handoff.md`。
- [x] AC8. `v2-prompts-memory.md` 补上目录轨、就地编辑、折叠检视，且不放松现有 command 边界。

## Interaction defects this sprint must close

- 点开提示词后先看到应用 / ID / 时间定义列表，正文缩在只读框里，再点「编辑」才进 Dialog。人类阅读路径过长。
- 三栏默认宽度加 `min-height: 330/450` 的 textarea，常见窗口下正文被挤没，必须手动拉分隔条才看得到。这是错误交互，不是可接受的进阶调节。
- 记忆第三栏「记忆信息 / 使用说明」和中间栏重复，进一步抢走正文宽度。

## Open questions

无阻断问题。产品对象沿用基线 native 合同；今晚只补信息架构与上述阅读/分栏缺陷。
