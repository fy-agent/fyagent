# Design — Grok login and SuperGrok placement

先读 [summary.md](./summary.md)。这里只写怎么接现有零件，不另起炉灶。

## Architecture

不新开登录系统，不新开第四套保存。

```text
新界面 Agent（Grok）
  → 打开终端 grok login / logout
  → 只说「门打开了」，不说「已经登进去」

旧认证中心（SuperGrok 扫码）
  → 账号存在 FyAgent 自己的保险柜（xai_oauth）
  → 这把钥匙给下面几家共用，每家各自写入

新界面模型（Grok）
  → 只填 API 钥匙
  → 不讲 grok login，不讲扫码
```

| 地方 | 这轮做什么 | 不要做什么 |
|---|---|---|
| 新界面 Agent | 官方登录的路标说清楚 | 假装已经登录；去翻 Grok 秘密文件 |
| 认证中心 | SuperGrok 扫码仍只在这里 | 搬进新界面；新做一套 OAuth |
| Claude Code | 已有账号能绑上去；新界面能看见这条路 | 和 Codex 写进同一张预览单 |
| Claude Desktop | 旧界面走通绑定；目录没有单独一页就不要硬造 | 假装它在新界面 Agent 目录里 |
| Codex | 现有 Change Plan 开窄口：已有托管账号才放行 | 第四套保存；预览单里放钥匙 |
| WorkBuddy | 走它自己的 Change Plan | 走 Codex upsert；把刷新令牌抄进 models.json |
| 新界面模型 | API 钥匙保持原样 | 把官方登录说明书贴过来 |

## Data flow

1. 官方登录：界面只说「给 Grok 登录或退出」。程序打开官方命令，立刻结束。不像 Claude 那样再查一遍「真的登进去了没有」。
2. 扫码：人在认证中心登完。钥匙放在 `xai_oauth_auth.json`，不写进 Grok 官方那个秘密文件。
3. Claude Code / Claude Desktop：旧界面已经能用 `xAI (Grok)` 的 `xai_oauth` 预设绑定。这轮复用这套绑定，每家一张独立保存。Claude 没有 Change Plan 适配器，不要为它新开第四个执行器。Desktop 亲测走旧界面。
4. Codex：旧界面已经能绑。新界面 Change Plan 今天会拒绝 SuperGrok。这轮仍用这一套预览，只允许「认证中心里已经有这个账号」。预览单里仍然不能出现钥匙。不要把旧表单搬进新界面。
5. WorkBuddy：走 `create_workbuddy_save_plan`。已扫码的，先用这份账号拉模型名单，不要再扫一次。WorkBuddy 自己的文件只认地址和钥匙：不要把 OAuth 刷新令牌抄进去。能少填一把钥匙就少填；做不到就老实说卡在文件格式，不要谎报已经写进去。

调研：

- `../08-31-grok-login-trichotomy/research/current-login-surfaces.md`
- `../08-31-grok-supergrok-to-codex/research/current-supergrok-codex-path.md`
- `../08-31-grok-supergrok-to-workbuddy/research/current-workbuddy-save-path.md`

## Compatibility

- Claude「能查到是否登录」的路不变。
- Codex「去认证中心管账号」的说法不变。
- 不关 #42 / #43 整张工单。
- 不装、不升级 Grok。
- ChatGPT 登录（`codex_oauth`）这轮不动。
- 额度查询继续可以读 Grok 秘密文件；登录成功不能靠它。
- 没改模型草稿，#141 B7 就标「这轮没碰」。

## Tradeoffs

- 新界面没有认证中心这一页。扫码用路标指回去，不整页搬迁。
- 官方登录没有「查一下登没登」的命令。双机亲测看的是门开对了、字写对了、人能在终端做完，不是软件显示「已登录」。
- Claude / Desktop 继续走已有 Provider 绑定，不新造 Claude Change Plan。
- Codex 要在现有预览上开窄门。
- WorkBuddy 和 Codex 不是同一扇门。关联的是同一把扫码钥匙，不是同一段写入代码。

## Rollback

登录路标、Claude/Desktop/Codex 写入、WorkBuddy 写入可以分开撤。不要把别人已经做完的登录合同整段撤掉。
