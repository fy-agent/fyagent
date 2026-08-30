# Implement — Grok login trichotomy

先读 [design.md](./design.md) 和 [use-cases.md](./use-cases.md)。父任务 [summary.md](../08-31-grok-first-class-iteration/summary.md) 是对齐源。

依赖：无。投放窗口可以并行读材料，但不要在本窗口的路标立好前，把 Codex 文案改成叫人去跑 `grok login`。

## 开工顺序

1. 读 `research/current-login-surfaces.md` 全文，不要凭记忆改状态机。
2. 改 V2 Grok 认证区文案：下一步写明终端 `grok login` / `grok logout`。锁住「已交给官方认证入口」，禁止「认证结果已验证」。
3. 给 SuperGrok 扫码一个指向认证中心的下一步。不要在 Agent 页启动设备码。
4. 打开模型页 Grok Quick Setup，确认没有 `grok login` 说明书。默认不改这个文件。
5. 用下面的自动检查钉住 UC-L1–L4。没改草稿就把 #141 B7 标成没碰。
6. 回写 #43 / #106，不关整张 #43。

## 会碰到的文件（先读再改）

| 文件 | 为什么 |
|---|---|
| `src/v2/pages/agents/AgentAuthStatusPanel.tsx` | Grok / Claude / Codex 认证文案和按钮 |
| `tests/v2/pages/agents/AgentAuthStatusPanel.test.tsx` | 禁止 Grok「认证结果已验证」；Claude 仍要能验证 |
| `tests/v2-browser/agents-v3.spec.ts` | 浏览器层 Grok 不得出现「认证结果已验证」 |
| `src-tauri/src/agent_install/auth_actions.rs` | Grok `HandoffComplete`；不要改成 verified |
| `src-tauri/src/agent_install/auth_sessions.rs` | handoff 短路径；默认不改 |
| `src/components/settings/AuthCenterPanel.tsx` | 扫码主人；只指路，不重做 |

不要进口：`src/v2` 不得 import `AuthCenterPanel` / `XaiOAuthSection`（`tests/v2/app/architecture.test.ts`）。

## 自动检查

优先跑现有认证测试，不要一上来跑整仓：

- `tests/v2/pages/agents/AgentAuthStatusPanel.test.tsx`
- `tests/v2/features/agent-auth.test.ts`
- `tests/v2-browser/agents-v3.spec.ts` 里 Grok / Claude 认证断言

改完再按仓库惯例补 `mise run check` 里和本窗口相关的项。

## 亲测

父任务 `research/hil-matrix.md` 的 AT1–AT5、H1–H4。本窗口不跑 H5–H8。

## 回滚

只还原认证文案。不要整段撤 Claude 验证合同。
