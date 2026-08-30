# Design — SuperGrok to Claude Code, Claude Desktop, and Codex

先读父任务 [summary.md](../08-31-grok-first-class-iteration/summary.md)。事实和行号见 `research/current-supergrok-codex-path.md`。用例见 [use-cases.md](./use-cases.md)。

本子任务不管登录路标，不管 WorkBuddy。目录名是历史留下的，范围以标题为准。

## 共用前提

登录还在认证中心。token 在 `xai_oauth_auth.json`。Provider 行只记：

- `meta.providerType = "xai_oauth"`
- `meta.authBinding = { source: "managed_account", authProvider: "xai_oauth", accountId }`

禁止把 token 写进 Provider、预览单、前端。禁止新 OAuth。禁止第四个 Change Plan 执行器。禁止把 v1 表单进口 `src/v2`。

三家**各写各的**。不要合成一张多目标计划。

## Claude Code

现有主人：`claudeProviderPresets` 里名为 `xAI (Grok)` 的 `xai_oauth` 预设 + `ProviderForm` 绑定（`ProviderForm.tsx` 约 1176–1587）。

- 没有 Claude Change Plan 适配器。不要新开。
- 新界面要能看见：没账号 → 去认证中心；有账号 → 走现有绑定/预览。不要整页搬 `XaiOAuthSection`。
- 失败不得声称 Codex / Desktop / WorkBuddy 已改好。

## Claude Desktop

现有主人：`claudeDesktopProviderPresets` + `ClaudeDesktopProviderForm`（约 619–624 挂 `XaiOAuthSection`）。

- 不在 V2 Agent 目录里。不要硬造目录页。
- 亲测和写入都走旧界面。新界面最多一句路标。

## Codex

现有主人：Change Plan `codex_provider_switch` / `codex_provider_upsert_and_switch`。

今天会拒绝 SuperGrok：`prove_codex_target_credential_capability`（`service.rs` 约 1593–1646）看到 `ManagedAccount` 或任何 `provider_type` 就 `SecretDependencyUnavailable`。Quick Setup DTO 只有 API 钥匙。V2 有没有 xAI 账号，页面长得一样。

这轮只开窄口：

1. 认证中心已有可用 `xai_oauth` 账号时，允许预览。计划里仍然没有钥匙。
2. 新界面能看见：没账号先去认证中心；有账号再预览 Codex。
3. 能切换已经绑好的旧记录就切换。不要悄悄盖掉 `fyagent-v2-quick-setup-codex` 那条 API 钥匙槽。
4. 仍走 `apply_change_plan(planId, planDigest)`。不要第四个 adapter。

## 兼容

Claude 官方登录验证环不动。Codex Agent 认证继续 `fyagent_managed`。ChatGPT `codex_oauth` 不动。

## 回滚

三家可以单独关。关 Codex 窄口时，恢复「托管账号一律拒绝」，不要误伤 API 钥匙预览。
