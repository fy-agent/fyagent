# 交接：Grok 一等公民（明天 Mac 继续）

写给明天的 William。密码、验证码、邮箱、账号名、token 不要写进仓库或结果表。

## 分支

- 仓库：`fy-agent/fyagent`（远端 `origin`）
- 分支：`feat/grok-first-class-iteration`
- 从 `main` / `79092221` 切出，**不要**推 `main`
- 明天若分支名或目标仓库不对，直接改；这份说明跟着分支走

对齐源：本目录 [`summary.md`](./summary.md)。亲测表：[`research/hil-matrix.md`](./research/hil-matrix.md)。

## 现在程序里有什么

三条登录已经分开，不要混：

| 路 | 人怎么走 | 对的样子 |
|---|---|---|
| 官方 Grok | 新界面 → AI软件配置 → Grok Build → 模型 →「登录」 | 终端 `grok login` / `grok logout`。只说门打开了，**不说**已登录。 |
| SuperGrok 扫码 | 同一页点「打开认证中心扫 SuperGrok」 | 弹出旧认证中心，在 **xAI (Grok OAuth)** 扫一次。这把钥匙给后面几家共用。 |
| API 钥匙 | 侧栏模型管理 → Grok Build，或 Grok 页「配置 API 钥匙」 | 只填钥匙。没有 `grok login` 说明书。 |

SuperGrok 扫完之后，分别去各家模型页写入（每家一张单，不要混）：

1. Claude Code：模型页 →「绑定到 Claude Code」→ 先看 → 确认
2. Claude Desktop：同一页「绑定到 Claude Desktop」（目录没有单独一页是正常的）
3. Codex：模型页 →「创建 SuperGrok Provider」→ 先看 → 确认创建 → 再确认切换预览
4. WorkBuddy：模型页 →「用 SuperGrok 拉名单」→ 走它自己的保存，不是 Codex 那扇门

官方 `grok login` **不会**写进 Codex。只用 Grok Build 的人，终端自己跑 `grok login` 就可以。

## 今天修过的交互

- 认证状态只出现在「模型」分段。Skill / MCP / 提示词顶上不再钉认证条。
- Grok / Codex 不再显示「刷新状态」。刷新不会再把交接成功条清掉、看起来像退登。
- Grok 页主按钮改成「配置 API 钥匙」，并加了去 Codex / Claude / WorkBuddy 绑定 SuperGrok 的门。
- 「打开认证中心」会弹出旧认证中心（不把扫码搬进新界面，也不新做一套 OAuth）。

## 明天 Mac 怎么走

1. checkout `feat/grok-first-class-iteration`（或你改过的分支名）。
2. 不要装、不要升级 Grok。不要做 ChatGPT 登录。不要写 Qoder / TRAE。
3. 按 [`research/hil-matrix.md`](./research/hil-matrix.md) 走完 H1–H9。
4. 结果只写屏幕事实。Windows 结果表还在本机：`C:\Users\wq241\Downloads\FYAGENT-GROK-HIL-WINDOWS-结果.md`。Mac 可写私人交接仓 `results/mac.md`。
5. 少一家、少一台电脑，都不算完。不要关 GitHub #42 / #43。

Windows 上官方登录（H1）已经走过一轮，当时「刷新状态」会像退登、「进入模型管理」会掉进 API Key 页；这两处今天已改，Mac 请按新交互测。SuperGrok → 四家写入（H3–H8）Windows 还没走完。

## 明确不要做

- 不把官方登录显示成「已验证 / 已登录」
- 不读 `~/.grok/auth.json` 假装已登录
- 不把旧认证中心整页搬进 `src/v2`
- 不提交 `.qoder/`、截图、`MEMORY.md`、密钥
