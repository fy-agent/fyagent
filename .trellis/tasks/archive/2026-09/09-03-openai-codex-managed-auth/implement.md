# Implement

## Checklist

1. [x] 刷新 OpenAI 官方登录源码证据。
2. [x] 实现 backend-owned login session 与 loopback server。
3. [x] 接入统一 vault/repository。
4. [x] 实现 Codex consumer observation；file projection 保持 fail-closed。
5. [x] 修改 Provider switch invariant 和设置迁移。
6. [x] 接入 V2 login commands；focused mise 通过。Native Codex HIL 未做，能力保持 closed。


## Validation

- 所有项目环境和检查通过 `mise` 执行。
- 先运行本切片 focused checks，再运行父任务要求的相关 frontend/backend/contracts gates。
- 原生行为必须在匹配平台 HIL 中验证；mock/browser/cross-compile 不替代 native evidence。

## Rollback Point

若当前阶段无法满足父任务的单一 authority、SecretRef、refresh-owner 或 readback 不变量，停止该能力并回到设计，不增加第二实现或明文 fallback。
