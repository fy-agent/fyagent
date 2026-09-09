# 用例 — SuperGrok → Claude Code / Desktop / Codex

双机勾选表在父任务 `research/hil-matrix.md`。

## UC-P0 没账号先指路

- 对应 AT6、AT7
- 人：认证中心没有 SuperGrok，却想投到 Claude Code 或 Codex
- 期望：新界面指向认证中心；Codex Change Plan 仍然 `SecretDependencyUnavailable`；不搬旧表单

## UC-P1 Claude Code

- 对应 H5
- 人：已登录 SuperGrok → 选 Claude Code → 先看 → 确认 → 检查
- 期望：只改 Claude Code；预览/回读里没有钥匙；失败不谎报 Codex / WorkBuddy

## UC-P2 Claude Desktop

- 对应 H6
- 人：已登录 SuperGrok → 旧界面选 Claude Desktop → 先看 → 确认 → 检查
- 期望：能走通；不要要求它出现在 V2 Agent 目录里

## UC-P3 Codex

- 对应 H7
- 人：已登录 SuperGrok → 选 Codex → 先看 → 确认 → 检查
- 期望：走现有 Change Plan；预览单没有钥匙；不盖掉原来的 API 钥匙槽除非预览里写明

## UC-P4 一家失败不连坐

- 对应 AT8、H9
- 人：故意取消或失败其中一家
- 期望：界面和 job 都不说另外两家或 WorkBuddy 已改好

## 本窗口不做

登录三条路（H1–H4）。WorkBuddy（H8）。ChatGPT 登录。Qoder / TRAE。
