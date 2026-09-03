# Implement

## Checklist

1. [ ] 复核前序子任务和剩余双 owner。
2. [ ] 迁移/删除旧 UI、command 和直接 manager 调用。
3. [ ] 完成恢复、安全、a11y、性能和整库测试。
4. [ ] 执行 native HIL，记录脱敏证据。
5. [ ] 更新 Specs，逐任务归档，提交归档/journal，确认 clean。


## Validation

- 所有项目环境和检查通过 `mise` 执行。
- 先运行本切片 focused checks，再运行父任务要求的相关 frontend/backend/contracts gates。
- 原生行为必须在匹配平台 HIL 中验证；mock/browser/cross-compile 不替代 native evidence。

## Rollback Point

若当前阶段无法满足父任务的单一 authority、SecretRef、refresh-owner 或 readback 不变量，停止该能力并回到设计，不增加第二实现或明文 fallback。
