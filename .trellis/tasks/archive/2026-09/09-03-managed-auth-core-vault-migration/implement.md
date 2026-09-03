# Implement

## Checklist

1. [x] 冻结 schema/DTO/error 合同。
2. [x] 扩展 SecretPurpose 和生产组合根。
3. [x] 新增 auth metadata tables/DAO/service。
4. [x] 实现旧 store 读入、SecretRef 写入、metadata commit、readback、旧文件封存的可恢复迁移。
5. [x] 重定向兼容命令和 token resolver。
6. [x] 运行 backend/contract/native focused checks。


## Validation

- 所有项目环境和检查通过 `mise` 执行。
- 先运行本切片 focused checks，再运行父任务要求的相关 frontend/backend/contracts gates。
- 原生行为必须在匹配平台 HIL 中验证；mock/browser/cross-compile 不替代 native evidence。

## Rollback Point

若当前阶段无法满足父任务的单一 authority、SecretRef、refresh-owner 或 readback 不变量，停止该能力并回到设计，不增加第二实现或明文 fallback。
