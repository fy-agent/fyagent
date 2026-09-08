# 实现 OpenCode Desktop Provider 账号管理

## Goal

让 OpenCode Desktop 在没有系统 opencode CLI 时仍可管理 Provider 登录与账号连接。

## Background

- 本任务是父任务 `.trellis/tasks/09-03-unified-agent-auth-management` 的可独立验收切片。
- 产品、协议、复用、许可证和安全基线以父任务 `prd.md`、`design.md` 与 `research/` 为准；发现外部事实变化时先更新 research，不在代码中猜测。

## Requirements

- 依据官方 Auth schema 观察 Provider 连接和 credential 类型。
- 通过 Desktop inventory 绑定用户选择的 OpenCode 数据目录/实例。
- 支持 OpenAI/xAI/Copilot 等 FyAgent-managed session 的用途隔离连接投影。
- 支持断开、冲突检测、外部修改、revision/CAS、原子写入和 readback。
- 前端展示 provider、账号、credential source 和 restart requirement。

## Acceptance Criteria

- [x] Desktop 已安装但 CLI 缺失时仍可读取和管理支持的 Provider。
- [x] 未知/环境变量/配置来源 Provider 不被误删或覆盖。
- [x] 写入保留未知条目、权限和其他 Provider，失败不制造乐观成功。
- [x] 同一 OpenAI/xAI identity 默认使用 OpenCode 独立 Credential Session。
- [ ] 真实 Desktop HIL 覆盖连接、刷新、断开、外部变更和重启。

## Out of Scope

- 探测 Desktop 私有随机 sidecar 端口/密码。
- 依赖 PATH 中的 opencode CLI 作为 Desktop Auth 前置。
- 未经评审把任意 Provider plugin auth 复制进 FyAgent。

## Open Questions

无阻断性产品问题；技术不确定项必须通过第一方源码、官方文档、当前仓库和 native HIL 收敛。
