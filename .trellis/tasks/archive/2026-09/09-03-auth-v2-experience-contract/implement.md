# Implement

## Checklist

1. [x] 读取父任务前端蓝图和 V2 相关 Spec。
2. [x] 先补严格 DTO/Port/fixtures 和失败测试。
3. [x] 新增导航、route loader、page shell、账号列表/详情/连接组件及登录会话对话框。
4. [x] 让 Agent 卡片仅显示摘要并链接中央页。
5. [x] 运行 mise 的 V2 lint/type/test/browser/build 检查。


## Validation

- 所有项目环境和检查通过 `mise` 执行。
- 先运行本切片 focused checks，再运行父任务要求的相关 frontend/backend/contracts gates。
- 原生行为必须在匹配平台 HIL 中验证；mock/browser/cross-compile 不替代 native evidence。

## Rollback Point

若当前阶段无法满足父任务的单一 authority、SecretRef、refresh-owner 或 readback 不变量，停止该能力并回到设计，不增加第二实现或明文 fallback。
