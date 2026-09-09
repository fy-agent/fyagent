# 原分支计划留档

当前执行以父任务 2026-09-08 修订为准；本页为历史内容。

## prd.md

# Finish Grok login and SuperGrok placement into supported tools

先读 [summary.md](./summary.md)。用例总表：[use-cases.md](./use-cases.md)。亲测勾选：[research/hil-matrix.md](./research/hil-matrix.md)。

## Goal

分清三种 Grok 登录；扫码一次 SuperGrok，能用到 Claude Code、Claude Desktop、Codex 和 WorkBuddy。William 在 Windows 和 Mac mini 上亲自走完才算完成。

## Background

- 意图：[Discussion #106](https://github.com/fy-agent/fyagent/discussions/106)。登录回写 [#43](https://github.com/fy-agent/fyagent/issues/43)，投放回写 [#42](https://github.com/fy-agent/fyagent/issues/42)。
- 2026-08-31：William 决定关联投放一起做，不拆成「先只做 Codex」。
- 子任务：登录路标；Claude/Desktop/Codex 投放；WorkBuddy 投放。

## Confirmed facts

登录（`08-31-grok-login-trichotomy/research/current-login-surfaces.md`）：

- 三条路散在三处。官方登录只交接，不验证。没有 Grok 登录状态命令。不能用 `~/.grok/auth.json` 证明已登录。
- ChatGPT 登录是 `codex_oauth`，和 SuperGrok 扫码不是一把钥匙。

投放（`08-31-grok-supergrok-to-codex/research/current-supergrok-codex-path.md`）：

- SuperGrok 扫码是共用认证中心。旧界面已能绑 Claude Code / Claude Desktop / Codex 的 `xai_oauth` 预设。
- 新界面 Change Plan / Quick Setup 只认 API 钥匙，会拒绝托管扫码。有没有账号，页面长得一样。
- Claude Desktop 不在新界面 Agent 目录里，亲测走旧界面。

WorkBuddy：

- 目录允许自己换模型。保存走自己的 Change Plan（地址 + 钥匙 + 模型名），不是 Provider Quick Setup。
- 现在没有 `xai_oauth` 预设。Qoder 不能配第三方模型；TRAE 不能代写模型。

## Requirements

- R1. 三种登录路标分开。官方登录不说已登录。
- R2. SuperGrok 扫码仍只在认证中心。
- R3. 同一份已登录 SuperGrok，能分别写进 Claude Code、Claude Desktop、Codex。每家一张独立预览/保存。失败不连累别人。
- R4. 同一份脑子能写进 WorkBuddy。优先用已扫码账号，不要无故再要一把钥匙。走 WorkBuddy 自己的保存。
- R5. Qoder / TRAE 不写第三方模型。ChatGPT 登录这轮不做。
- R6. 双机亲测全部路径。密码不进仓库。

## Acceptance Criteria

- [ ] AC1. 人能分清官方登录、扫码、API 钥匙。
- [ ] AC2. 官方登录不出现「已验证」。Claude 原来能验证的路还在。
- [ ] AC3. SuperGrok → Claude Code、Claude Desktop、Codex 都能先看再改再检查（Desktop 可在旧界面完成）。
- [ ] AC4. SuperGrok → WorkBuddy 能保存并回读。
- [ ] AC5. 一家失败不谎报另一家成功。
- [ ] AC6. 不关 #42 / #43 整张工单。#141 B7 按有没有改草稿标记。
- [ ] AC7. Windows 和 Mac mini 都按 `research/hil-matrix.md` 走完。

## Out of scope

- 安装升级 Grok（#31、#32）
- 新界面额度看板
- ChatGPT 登录
- Qoder / TRAE 模型写入
- 总门卫
- 写 `~/.grok/auth.json` 冒充登录


## design.md

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


## implement.md

# Implement — Grok first-class iteration

先读 [summary.md](./summary.md) 和 [use-cases.md](./use-cases.md)。父任务不改产品代码。

下游开工读序（每个子任务都要齐）：`summary.md` → 子任务 `prd.md` → `design.md` → `implement.md` → `use-cases.md` → 该任务 `research/` → `implement.jsonl` 里的 spec。

不要从空白开始，也不要只读父任务摘要就改代码。

## Feature inventory

| 编号 | 人能做成的事 | 谁做 | 回写 |
|---|---|---|---|
| F1 | Grok 官方登录/退出找得到，并且不说已经登录 | 登录窗口 | #43 |
| F2 | SuperGrok 扫码的下一步指向认证中心 | 登录窗口 | #43 |
| F3 | 模型页继续只填 API 钥匙 | 登录窗口 | #43 |
| F4 | SuperGrok 能进 Claude Code、Claude Desktop、Codex：每家先看、再改、再检查（Desktop 可在旧界面完成） | 投放窗口 | #42 / #41 / #63 |
| F5 | SuperGrok 能进 WorkBuddy：先看、再改、再检查 | WorkBuddy 窗口 | #42 |
| F6 | 名单上有名字，不等于已经完全支持 | 各窗口改字时都遵守 | #22 / #106 |

## Change inventory

| 编号 | 要改 | 不要改 |
|---|---|---|
| C1 | Grok 认证区的字，点名去终端跑 `grok login` | Claude 那种「查一下真的登了」 |
| C2 | 扫码的下一步指到认证中心 | 新做一套登录；把旧设置页搬进新界面 |
| C3 | Claude Code / Desktop：复用已有 `xai_oauth` 绑定；每家独立保存 | Claude Change Plan 新执行器；和 Codex 写一张单 |
| C4 | 现有 Codex 预览：认证中心已有 SuperGrok 账号时放行；新界面能看见这条路 | 第四套保存；预览单里放钥匙；不打招呼就盖掉原来的 API 钥匙槽 |
| C5 | WorkBuddy 自己的保存预览能用已扫码账号拉模型 | 走 Codex upsert；把刷新令牌抄进 `models.json` |
| C6 | 用自动检查把 F1–F5 钉住 | 没必要就别动模型草稿 |

## Ordered work

1. 先立登录路标，免得投放还在叫人去跑 `grok login`。
2. 再开 Claude / Desktop / Codex：每家独立预览和保存。Codex 先改「准不准预览」，再补新界面能看见的路。
3. 再开 WorkBuddy：走它自己的 Change Plan。
4. 总控把三条线接成一次能走完。
5. William 在两台电脑上按 `research/hil-matrix.md` 亲测。
6. 回写 GitHub，不关整张 #42 / #43。

## Validation

- 各窗口自己的程序检查。
- 总控：`research/hil-matrix.md` 两台电脑都打勾。
- 密码不进仓库。

## Risky files

- `src/v2/pages/agents/AgentAuthStatusPanel.tsx`
- Grok 登录交接（先看，不要轻易改短路径）
- Codex 预览是否放行（`prove_codex_target_credential_capability` 一带）
- Claude / Desktop 的 `xai_oauth` 预设和 `ProviderForm` 绑定
- WorkBuddy `create_workbuddy_save_plan` / `models.json` 写入
- 认证中心现有扫码界面（尽量只指路，不重做）

## Rollback points

- 登录路标撤了，不影响投放写入。
- Claude / Desktop / Codex / WorkBuddy 可以单独关一扇门，不要互相连坐。
