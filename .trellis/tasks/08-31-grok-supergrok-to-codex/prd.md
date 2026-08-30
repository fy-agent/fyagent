# Place SuperGrok into Claude Code, Claude Desktop, and Codex

先读父任务 [summary.md](../08-31-grok-first-class-iteration/summary.md)。用例：[use-cases.md](./use-cases.md)。本子任务不管三条登录路标，也不管 WorkBuddy。

目录名仍是 `08-31-grok-supergrok-to-codex`，名称是历史留下的，范围以标题和本文为准。

## Goal

人在认证中心用 SuperGrok 登录一次，就能把这颗脑子分别用到 Claude Code、Claude Desktop、Codex。每家先看要改什么，点头后再改，改完再检查。Desktop 不在新界面 Agent 目录里，亲测走旧界面。

## Confirmed facts

见 `research/current-supergrok-codex-path.md`。

- SuperGrok 扫码是共用认证中心。旧界面已经能绑：Claude Code / Claude Desktop 的 `xAI (Grok)`（`xai_oauth`），以及 Codex 的 `xAI (Grok) OAuth`。只记绑了哪个账号。
- 新界面 Change Plan 只收 API 钥匙，并且会拒绝 SuperGrok 这种登录。有没有账号，页面长得一样。
- Claude 没有 Change Plan 适配器。不要为 Claude 新开第四个执行器。Claude / Desktop 复用现有 Provider 绑定。
- Codex 还用原来的预览/确认/检查。只多开一扇该开的门：认证中心里已经有可用账号。不要第四套保存，不要把旧表单搬进新界面。

## Requirements

- R1. 不新做执行器。Codex 只在现有预览/确认上，允许已有 SuperGrok 托管账号通过。
- R2. Claude Code、Claude Desktop、Codex 各写各的。失败了不说别人也被改好了。
- R3. token 不进 Provider 行，不进预览单，不进前端。
- R4. 新界面能看见「没账号先去认证中心；有账号再绑 Claude Code / Codex」。Desktop 指到旧界面完成即可。
- R5. 不把旧认证中心整页搬进 `src/v2`。

## Acceptance Criteria

- [ ] 没账号时，新界面指向认证中心；Codex Change Plan 仍然拒绝。
- [ ] 有账号时，Claude Code、Claude Desktop、Codex 都能预览、确认、检查；预览单里没有钥匙。
- [ ] 一家失败不谎报其他工具。
- [ ] 回写 #42、#106，不关整张 #42。
- [ ] William 在 Windows 和 Mac mini 上用真实账号亲自走完这三家。密码不进仓库。

## Out of scope

- WorkBuddy（见 `08-31-grok-supergrok-to-workbuddy`）
- 登录路标文案
- ChatGPT 登录
- Qoder / TRAE 模型写入
- 安装升级、额度看板
