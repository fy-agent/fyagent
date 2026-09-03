# Implement

## Checklist

1. [x] 复核前序子任务和剩余双 owner。
2. [ ] 迁移/删除旧 UI、command 和直接 manager 调用。
3. [ ] 完成恢复、安全、a11y、性能和整库测试。
4. [ ] 执行 native HIL，记录脱敏证据。
5. [ ] 更新 Specs，逐任务归档，提交归档/journal，确认 clean。

已完成的第一刀：`copilot_get_token*` 对 renderer 永久 fail-closed；leftover
Settings 认证页改为兼容说明，不再作为第二套登录 owner；Managed Auth 相关
Rust cfg 已写入 supported-platform allowance，非 macOS/Windows 浏览器打开
保持 `Unsupported`。V1 Provider 表单里的 Copilot/Codex/xAI 区块、JSON
manager 兼容路径、故障恢复 UX 和 native HIL 仍未做。


## Validation

- 所有项目环境和检查通过 `mise` 执行。
- 先运行本切片 focused checks，再运行父任务要求的相关 frontend/backend/contracts gates。
- 原生行为必须在匹配平台 HIL 中验证；mock/browser/cross-compile 不替代 native evidence。

## Rollback Point

若当前阶段无法满足父任务的单一 authority、SecretRef、refresh-owner 或 readback 不变量，停止该能力并回到设计，不增加第二实现或明文 fallback。
