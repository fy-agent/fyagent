# 实现 OpenAI 官方登录与 Codex 无损切换

## Goal

实现 OpenAI 官方账号登录和 Codex 原生连接，同时确保官方登录在第三方 Provider 切换中永久保留。

## Background

- 本任务是父任务 `.trellis/tasks/09-03-unified-agent-auth-management` 的可独立验收切片。
- 产品、协议、复用、许可证和安全基线以父任务 `prd.md`、`design.md` 与 `research/` 为准；发现外部事实变化时先更新 research，不在代码中猜测。

## Requirements

- 实现官方 browser-loopback Authorization Code + PKCE，Device Code 为受控回退。
- 管理 OpenAI identity 和用途隔离的 Credential Session。
- 实现 Codex file/keyring/auto/ephemeral 能力观察和安全连接事务。
- 切换官方/第三方 Provider 时永不破坏官方凭据；切回无需有效用户重新登录。
- 处理 Codex 外部刷新、generation reconciliation、restart/readback。

## Acceptance Criteria

- [x] browser flow 校验 loopback host/path/state/PKCE、端口和超时，且日志不含 code/token。
- [x] Device Code 回退可取消、可恢复并遵守服务端轮询间隔。
- [x] Codex 官方连接和第三方 Provider 状态独立展示与存储。
- [x] 所有第三方切换路径均不能删除/覆盖官方 Auth。
- [ ] 真实 Codex HIL 覆盖登录、刷新、切换第三方、切回、重启和外部 token 更新。生产 file/keyring 投影保持 fail-closed。

## Out of Scope

- 复用 cockpit-tools 受限许可证源码。
- 伪装 keyring/auto 为 file 写入。

## Open Questions

无阻断性产品问题；技术不确定项必须通过第一方源码、官方文档、当前仓库和 native HIL 收敛。
