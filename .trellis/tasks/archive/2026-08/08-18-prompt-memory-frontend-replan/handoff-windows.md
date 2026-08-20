# Windows 测试机任务

这台机器负责官方门禁和三页桌面。Linux 云环境过不了 `host-native.mjs guard`，不能替代这里。

拉代码：

```powershell
git fetch origin cursor/prompt-memory-frontend-align-06e7
git checkout cursor/prompt-memory-frontend-align-06e7
git pull origin cursor/prompt-memory-frontend-align-06e7
```

## 1. 环境

1. `mise --version` ≥ 2026.8.6，并 `mise trust` 本仓库。
2. `node -v` 对齐 `.node-version`（24）。`pnpm -v` 对齐 `package.json#packageManager`。
3. 本机原有的 Tauri / WebView2 依赖不要拆掉。

## 2. 必须先跑

在仓库根目录：

```powershell
mise run check
```

失败就停。看是宿主、Rust 还是前端。不要拿 Linux 上 `pnpm test:unit` 的失败去改平台合同。

check 通过后再补：

```powershell
mise run lint:v2
mise run typecheck:v2
mise run test:v2
mise run test:v2:browser
mise run build:renderer
```

## 3. 用编出来的桌面应用看三页

不要只看浏览器预览。125% 和 150% 缩放各看一遍。

### 提示词 `#/prompts`

按 `handoff-prompts.md` 第 1–11 条走。缩放后正文和主按钮仍在，不要横向滚动。

### 记忆 `#/memory`

1. 页头只承诺：长期 = OpenClaw + Hermes；每日 = 只有 OpenClaw。
2. 长期左轨按 OpenClaw / Hermes 分组。组里只看到 `MEMORY.md` / `USER.md`，不要「长期记忆 · 4」。
3. 点开直接看正文。没有「记忆信息 / 使用说明」第三栏。
4. Hermes 开关和字符上限在编辑头。超限仍可保存。
5. OpenClaw 未创建文件：保存才创建。
6. 「打开 OpenClaw 工作区」和每日「打开记忆目录」打开本机目录。
7. 每日：搜索在工作区顶部，列表 + 编辑两栏。

### Skills 发现 `#/skills` → 发现

1. 搜索在第一行。安装目标在页头，旁边有「将安装到 {应用}」。不要 `<select>`。
2. 卡片是网格，不是左右详情。
3. 卡片能看到：名称、已安装、说明或「来自 owner/repo」、`owner/repo · N 次安装`、`安装到 …`。
4. README 叫「说明」，纯仓库叫「仓库」。点开会进系统浏览器。
5. 窗口缩窄后安装目标可以只剩图标，卡片区仍是主体。
6. 结果行也要写「将安装到 {当前应用}」。

## 4. 确认没伤到别的

- Agents / Models 目录、外链、Provider / WorkBuddy
- Skills 已安装三栏、分配、卸载
- MCP 已安装 / 发现，密钥不进普通 UI
- 顶栏、默认 `#/models`、keep-alive
- 最大化后窗口不跳；125%/150% 分栏内容不画出 pane

## 5. 回报

按页写「过 / 不过 + 窗口尺寸 + 缩放 + 一句现象」。失败带：

- 路由
- 操作序列
- 期望 vs 实际
- `mise run check` 失败的任务名

结果回 PR #111。合并后先不要关这个 PR。
