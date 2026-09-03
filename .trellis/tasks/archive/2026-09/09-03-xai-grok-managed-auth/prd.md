# 实现 xAI 官方登录与 Grok Build 账号连接

## Goal

把 Grok Build 从 handoff-only 升级为可观察、可管理、可恢复的 xAI 官方账号连接。

## Background

- 本任务是父任务 `.trellis/tasks/09-03-unified-agent-auth-management` 的可独立验收切片。
- 产品、协议、复用、许可证和安全基线以父任务 `prd.md`、`design.md` 与 `research/` 为准；发现外部事实变化时先更新 research，不在代码中猜测。

## Requirements

- 复用现有 xAI OIDC discovery、Device Code、refresh rotation 和 reauth 状态。
- 将 xAI token material 迁入统一 vault，建立 identity/session/connection。
- 实现 Grok auth.json/官方 helper capability 的安全观察、连接、断开、外部更新合并。
- 支持 GROK_HOME 与多个安装/配置目标的明确选择。
- 前端显示账号、连接、续期 owner 和失效状态。

## Acceptance Criteria

- [x] xAI Device Code 遵守 discovery、issuer/endpoint allowlist、slow_down/expiry/cancel。
- [ ] Grok 连接写入或 helper 绑定后必须 readback；失败可回滚或报告恢复需要。
- [x] Grok/FyAgent Proxy 默认使用用途隔离 session。
- [ ] 外部 Grok 刷新不会被旧 FyAgent generation 覆盖。
- [ ] macOS/Windows Grok HIL 证明登录、刷新、断开和重新连接。

自动化目前只证明 Device Code 协议与用途隔离。Grok helper /
`GROK_FILE_PROJECTION_PRODUCTION_ENABLED` 保持 `false`，因此写入后
readback、外部 Grok 刷新与 generation 对账、以及 macOS/Windows Grok Auth
HIL 均未完成，不得勾选。Agent 侧 Grok 观察仍为 `handoff_only`。
CLI 安装成功不是登录证据。`refresh_owner` 不允许 `shared`。

## Out of Scope

- 把 CLI 安装成功当作登录或中国大陆网络可用证据。
- 共享同一 refresh-token lineage 给 FyAgent Proxy 与 Grok。

## Open Questions

无阻断性产品问题；技术不确定项必须通过第一方源码、官方文档、当前仓库和 native HIL 收敛。
