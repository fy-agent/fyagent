# 建立统一 Managed Auth Core、SecretRef 与迁移基础

## Goal

建立唯一 Managed Auth 后端 owner，安全持久化账号元数据和 Credential Session，并迁移现有明文 OAuth store。

## Background

- 本任务是父任务 `.trellis/tasks/09-03-unified-agent-auth-management` 的可独立验收切片。
- 产品、协议、复用、许可证和安全基线以父任务 `prd.md`、`design.md` 与 `research/` 为准；发现外部事实变化时先更新 research，不在代码中猜测。

## Requirements

- 新增统一 domain/service、闭集 identity/session/connection/login job 类型。
- 激活并扩展现有 SecretRef 作为 refresh/access/id token bundle 的 OS-native 存储。
- 新增 SQLite metadata 表、schema migration、事务和恢复状态。
- 迁移现有 codex_oauth_auth.json、xai_oauth_auth.json 与 Copilot metadata；兼容旧命令但不保留第二 authority。
- 实现单一 refresh owner、generation/CAS、per-session lock 和 token redaction。

## Acceptance Criteria

- [ ] refresh/access/id token 不再持久化到普通 JSON/SQLite/DTO。
- [ ] 迁移幂等、可中断恢复、失败不删除旧 store；完成且 readback 后才清理/封存旧文件。
- [ ] 每条 refresh-token lineage 同时只有一个 owner，旧 generation 不能覆盖新 generation。
- [ ] 命令层薄，Proxy 和 Agent Auth 只能通过统一 service。
- [ ] 数据库、SecretRef 和迁移测试通过；native secret HIL 缺失时保持 fail-closed。

## Out of Scope

- Codex/Grok/OpenCode 原生投影的完整产品连接。
- 绕过 macOS Keychain entitlement 或 Windows Credential Manager 的明文 fallback。

## Open Questions

无阻断性产品问题；技术不确定项必须通过第一方源码、官方文档、当前仓库和 native HIL 收敛。
