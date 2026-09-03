# Implement

## Checklist

1. [x] 复核前序子任务和剩余双 owner。
2. [x] leftover `auth_*` 与 leftover Copilot 登录/轮询/删除/设默认/注销 IPC 永久 fail-closed；leftover Provider OAuth 区块改为只读 picker。未迁移 JSON manager 仍可作为 list/status 只读回退。
3. [ ] 完成恢复、安全、a11y、性能和整库测试。
4. [ ] 执行 native HIL，记录脱敏证据。
5. [x] owning specs 已按 leftover fail-closed 更新。任务归档、提交和工作树清洁仍未做。

已完成：`copilot_get_token*` 对 renderer 永久 fail-closed；leftover
Settings 认证页是兼容说明；leftover `auth_start_login` /
`auth_poll_for_account` / `auth_remove_account` / `auth_set_default_account`
/ `auth_logout` / `auth_cancel_login` 以及 leftover
`copilot_start_device_flow` / `copilot_poll_for_auth` /
`copilot_poll_for_account` / `copilot_remove_account` /
`copilot_set_default_account` / `copilot_logout` 返回
`legacy_auth_mutation_disabled`；Provider 表单 Copilot/Codex/xAI 区块不再
登录、轮询或删除账号。Managed Auth 相关 Rust cfg 已写入
supported-platform allowance。

仍未做：未迁移 JSON manager 的密封与删除、故障恢复 UX、macOS/Windows
native HIL、任务归档与提交。


## Validation

- 所有项目环境和检查通过 `mise` 执行。
- 先运行本切片 focused checks，再运行父任务要求的相关 frontend/backend/contracts gates。
- 原生行为必须在匹配平台 HIL 中验证；mock/browser/cross-compile 不替代 native evidence。

## Rollback Point

若当前阶段无法满足父任务的单一 authority、SecretRef、refresh-owner 或 readback 不变量，停止该能力并回到设计，不增加第二实现或明文 fallback。
