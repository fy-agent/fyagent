# Implement — SuperGrok to Claude / Desktop / Codex

先读 [design.md](./design.md) 和 [use-cases.md](./use-cases.md)。对齐源：父任务 [summary.md](../08-31-grok-first-class-iteration/summary.md)。

依赖：登录窗口的路标不要把扫码说成 `grok login`。本窗口可以先改准入，但新界面文案要和登录窗口一致。

## 开工顺序

1. 读 `research/current-supergrok-codex-path.md` 全文。
2. Claude Code / Desktop：确认现有 `xai_oauth` 绑定还能走通。新界面只补「没账号去认证中心」。Desktop 不要塞进 Agent 目录。
3. Codex：改 `prove_codex_target_credential_capability` 的窄口——没账号继续拒绝，有账号可以预览，单子里没有钥匙。
4. 补 Codex 新界面能看见的路。不 import v1 表单。
5. 改自动检查：不要再断言「凡是 SuperGrok 一律拒绝」。三家失败互不连坐。
6. 亲测 UC-P1–P4。回写 #42 / #106，不关整张 #42。

## 会碰到的文件（先读再改）

| 文件 | 为什么 |
|---|---|
| `src-tauri/src/services/change_plan/service.rs` | Codex 凭证门；窄口只放行已有 `xai_oauth` 托管账号 |
| `src-tauri/src/commands/change_plan.rs` | 现有 create/apply；不要新命令类型 |
| `src/config/codexProviderPresets.ts` | `xAI (Grok) OAuth` 预设，不要和 API Key 那条搞混 |
| `src/config/claudeProviderPresets.ts` | Claude Code 的 `xAI (Grok)` = OAuth |
| `src/config/claudeDesktopProviderPresets.ts` | Desktop 绑定 |
| `src/components/providers/forms/ProviderForm.tsx` | V1 绑定 `authBinding` |
| `src/v2/pages/models/Page.tsx` | Codex / Claude 新界面入口 |
| `src/v2/pages/models/apply/CodexSavePlanWorkspace.tsx` | 预览/确认/检查 |
| `tests/v2/app/architecture.test.ts` | 禁止 v1 认证表单进 `src/v2` |

## 自动检查

- Change Plan：`service.rs` 里现有「托管绑定被拒」的测试要改成「没账号拒绝 / 有账号放行且计划无密钥」
- Codex 预设：`tests/config/xaiOauthProviderPresets.test.ts` 不要把 API Key 预设当成扫码
- V2 架构：`tests/v2/app/architecture.test.ts`
- 单目标：一家失败不得出现其他 Agent 的成功 apply

## 亲测

`hil-matrix.md` 的 AT6–AT8、H5–H7、H9。不要替 WorkBuddy 窗口勾 H8。

## 回滚

Codex 窄口和 Claude 绑定分开撤。
