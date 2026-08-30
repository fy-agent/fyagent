# Design — Grok login trichotomy

先读父任务 [summary.md](../08-31-grok-first-class-iteration/summary.md)。事实和行号见 `research/current-login-surfaces.md`。用例见 [use-cases.md](./use-cases.md)。

## 边界

本子任务只立三条登录路标。不写 Claude / Desktop / Codex / WorkBuddy，不新做「查 Grok 登没登」。

三条路已经存在，只是散在三处。不要合成一个控件。

| 路 | 现有主人 | 这轮改什么 |
|---|---|---|
| 官方 `grok login` / `logout` | V2 Agent 配置页 `AgentAuthStatusPanel` → `start_agent_auth_session` → `launch_auth_action(GrokBuild)` | 文案点名终端命令；终点仍是 `handoff_complete` + `handoff_only` |
| SuperGrok 扫码 | v1 认证中心 `AuthCenterPanel` / `XaiOAuthSection` → `auth_start_login("xai_oauth")` | Agent / Codex 认证区指路到认证中心；不在 Agent 页启动扫码 |
| API 钥匙 | V2 模型页 Quick Setup `fyagent-v2-quick-setup-grokbuild` | **默认不改**。这里不要出现 `grok login` |

## 合同（不得破）

- Grok 官方登录：**禁止**出现「已验证」「已登录」「认证结果已验证」。权威是 `unverified`。
- Claude 的 `claude auth status` 验证环保持原样。
- Codex Agent 认证保持 `fyagent_managed`，没有登录按钮。
- 禁止读/写 `~/.grok/auth.json` 来证明已登录。额度查询可以继续读，登录成功不能靠它。
- 禁止从 Agent 配置页调用 `auth_start_login`。
- 禁止把 v1 `AuthCenterPanel` / `XaiOAuthSection` 进口到 `src/v2`。
- 没改模型草稿则 #141 B7 标 `not touched`。默认不要动 `ProviderPanel` / `quickSetup.ts`。

## 数据流

1. 人在 Grok Agent 配置页点登录 → 终端跑 `grok login` → 会话立刻 `handoff_complete`。
2. 人要扫码 → 被指到 v1 设置「认证」页的 `xAI (Grok OAuth)`。
3. 人要填钥匙 → 还在模型页，和上面两路无关。

## 兼容

ChatGPT 登录（`codex_oauth`）不动。Grok 安装/升级不动。

## 回滚

只撤文案和指路。不要动 `auth_sessions.rs` 的 handoff 短路径，除非测试证明字改了但状态机坏了。
