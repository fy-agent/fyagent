# Place SuperGrok into WorkBuddy models

先读父任务 [summary.md](../08-31-grok-first-class-iteration/summary.md)。用例：[use-cases.md](./use-cases.md)。本子任务不管三条登录路标，也不管 Claude / Desktop / Codex 的 Provider 绑定。

## Goal

人在认证中心用 SuperGrok 登录一次，就能在 WorkBuddy 里用上这颗脑子：先看要改什么，点头后再改，改完再检查。走 WorkBuddy 自己的保存，不是 Codex 那扇门。

## Confirmed facts

见 `research/current-workbuddy-save-path.md`。

- WorkBuddy 可以自己换模型。保存走 `create_workbuddy_save_plan` / `workbuddy_models_save`，请求是地址 + 钥匙 + 模型 ID。
- 现在没有 `xai_oauth` 预设。钥匙会写进 WorkBuddy 自己的 `models.json`，不走 Provider 行。
- Qoder 不能配第三方模型；TRAE 不能代写模型。这两家不在本任务。

## Requirements

- R1. 不新做执行器。只走现有 WorkBuddy Change Plan。
- R2. 已扫码 SuperGrok 的，先用这份账号拉模型名单，不要再扫一次。能少填一把钥匙就少填。
- R3. 不把 OAuth 刷新令牌抄进 `models.json`，不进预览单，不进前端。
- R4. 失败了不说 Claude / Codex 也被改好了。
- R5. 不把 WorkBuddy 改成 `AppType` 或 Provider。

## Acceptance Criteria

- [ ] 没账号时指向认证中心，不假装已经写进 WorkBuddy。
- [ ] 有账号时能预览、确认、检查；预览走 `workbuddy_models_save`，单子里没有刷新令牌。
- [ ] 回写 #42、#106，不关整张 #42。
- [ ] William 在 Windows 和 Mac mini 上用真实账号亲自走完。密码不进仓库。

## Out of scope

- Claude Code / Claude Desktop / Codex 写入
- 登录路标文案
- ChatGPT 登录
- Qoder / TRAE / OpenCode 另做 SuperGrok 扫码
- 安装升级、额度看板
