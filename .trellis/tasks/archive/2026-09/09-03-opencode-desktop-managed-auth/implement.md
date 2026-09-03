# Implement

## Checklist

1. [x] 冻结当前 OpenCode 官方 schema/路径/权限事实。
2. [x] 实现 capability/observation 和严格 parser。
3. [x] 实现 connection transaction、CAS/readback/rollback。
4. [x] 接入中央 UI 和 Agent 摘要。
5. [ ] 删除 Desktop Auth 对 CLI 的依赖并运行跨平台测试/HIL。

PATH CLI 依赖已从观察/断开路径移除。条目 5 仍未勾选：真实 Desktop HIL
（连接、刷新、断开、外部变更、重启）未做；
`OPENCODE_EXTERNAL_WRITE_HOT_RELOAD_PROVEN` 保持 `false`。


## Validation

- 所有项目环境和检查通过 `mise` 执行。
- 先运行本切片 focused checks，再运行父任务要求的相关 frontend/backend/contracts gates。
- 原生行为必须在匹配平台 HIL 中验证；mock/browser/cross-compile 不替代 native evidence。

## Rollback Point

若当前阶段无法满足父任务的单一 authority、SecretRef、refresh-owner 或 readback 不变量，停止该能力并回到设计，不增加第二实现或明文 fallback。
