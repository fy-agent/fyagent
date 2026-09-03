# Implement

## Checklist

1. [ ] 刷新 xAI/Grok 官方证据。
2. [ ] 迁移现有 xAI manager 到 ManagedAuth adapter。
3. [ ] 实现 Grok consumer capability probe、projection/helper、readback。
4. [ ] 接入 UI 与 Agent observation。
5. [ ] 执行并发/刷新/外部修改测试及 native HIL。


## Validation

- 所有项目环境和检查通过 `mise` 执行。
- 先运行本切片 focused checks，再运行父任务要求的相关 frontend/backend/contracts gates。
- 原生行为必须在匹配平台 HIL 中验证；mock/browser/cross-compile 不替代 native evidence。

## Rollback Point

若当前阶段无法满足父任务的单一 authority、SecretRef、refresh-owner 或 readback 不变量，停止该能力并回到设计，不增加第二实现或明文 fallback。
