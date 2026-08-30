# Clarify Grok login trichotomy

先读父任务 [summary.md](../08-31-grok-first-class-iteration/summary.md)。用例：[use-cases.md](./use-cases.md)。本子任务只立登录路标，不把 SuperGrok 写进 Claude / Codex / WorkBuddy。

## Goal

人能分清三条登录路。官方登录过期去终端跑 `grok login`。扫码去认证中心。API 钥匙留在模型页。打开官方入口，不等于已经登录。

## Confirmed facts

见 `research/current-login-surfaces.md`。

- 官方登录/退出只从新界面 Agent 配置页的认证按钮开始，结果只能是「交给官方了」。
- 扫码只在旧认证中心。新界面模型页没有扫码，也没有 `grok login`。
- 没有可复查的 Grok 登录状态命令。不能用 `~/.grok/auth.json` 证明已登录。
- 不改模型草稿则 #141 B7 标 `not touched`。

## Requirements

- R1. 三条路的名称、下一步、失败指回不互相抢。
- R2. 官方登录终点仍是「门打开了」，不是「已验证」。
- R3. 复用现有 `grok login` 交接和认证中心扫码，不新做一套登录。
- R4. 默认不改模型草稿。

## Acceptance Criteria

- [ ] 三条路的招牌各说各的。
- [ ] 官方登录不出现「认证结果已验证」。
- [ ] Claude 能验证的路还在。
- [ ] 回写 #43、#106，不关整张 #43。
- [ ] William 在 Windows 和 Mac mini 上亲自走完三条路。密码不进仓库。

## Out of scope

- SuperGrok 写进 Claude / Desktop / Codex / WorkBuddy
- ChatGPT 登录
- 安装升级、额度看板
