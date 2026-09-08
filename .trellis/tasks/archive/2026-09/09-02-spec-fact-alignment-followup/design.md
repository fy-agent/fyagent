# Spec 事实对齐跟进设计

## 1. Goal

在不修改产品代码的前提下，把上一轮拆分后的 Skills、MCP、Agent Auth、
Assignment、Agent Directory 与 Models SPEC 校准到当前 `HEAD` 的真实 Port、
DTO、调用顺序、失败状态和测试边界。文档必须既能指导安全实现，也不能把尚未
存在的运行时校验、原子回滚、聚合 Port 或敏感值隔离描述成既成事实。

## 2. Authority order

发生冲突时按以下顺序取证：

1. 当前生产源码中的命令、Port、服务和持久化顺序；
2. 可执行测试所断言的命令名、payload、解析和页面状态；
3. 聚焦 SPEC；
4. 兼容路由和归档任务记录。

公开 Trellis 资料只用于校验 SPEC 的组织原则；FYAgent 的具体合同只由本仓库
事实决定。

## 3. Correction boundaries

- `backend/skill-management.md`：区分外部非法目录输入与历史脏行卸载清理；
  保留真实的 filesystem/SQLite 非原子顺序。
- `backend/mcp-management.md`：明确 `McpServer.server` 的 IPC 类型是
  `serde_json::Value`，统一 upsert 当前没有数据库写入前的集中校验；区分
  普通展示、编辑态和外部 preflight 的敏感字段边界。
- `frontend/v2-assignments.md`：区分 TypeScript 闭集与运行时解析；共享
  authoritative helper 的 `false`/readback 语义保持唯一权威。
- `frontend/v2-skills.md`：描述管理页真实的 query invalidation/readback，且
  不虚构对 `toggleApp(): boolean` 的 `false` 分支处理。
- `frontend/v2-mcp.md`：保留原始 env/header 编辑态事实，并把 native
  validation 的真实时序与失败后持久化可能性写清楚。
- `frontend/v2-agent-auth.md`：严格 DTO 解析与请求/响应 Agent 绑定检查分开；
  不宣称 `startSession` 已完成尚未实现的二次绑定校验。
- `frontend/v2-agent-directory.md`：修正 lifecycle capability 的真实字段名。
- `frontend/v2-models.md`：按 `providers`、`workbuddy`、`opencodeModels`、
  `traeWork`、`changePlans` 五个真实 Port 和各自写入协议组织合同，不保留
  不存在的统一 `ModelPorts`。
- 其他被全库扫描命中的旧错误码、路径与“文件 + 子命令/配置段”记法，仅做
  最小事实修正，不扩展其功能范围。

## 4. Non-goals

- 不通过修改源码“让代码适配文档”；本任务只修文档事实。
- 不重新拆分已完成的信息架构，不扩大兼容路由。
- 不把已识别的产品加固机会伪装成当前合同；它们以“当前限制 / 变更要求”
  明示。
- 不修改依赖、测试、构建、CI、发布或用户文档。

## 5. Verification

1. 每个变更断言都能定位到具体源码或测试。
2. 变更后的跨层/基础设施 SPEC 均保留七段式结构。
3. 全库相对链接、索引可达性和代码路径检查通过。
4. 运行聚焦 V2 测试、`git diff --check`、`mise run check:contracts` 和精确
   prearchive gate。
5. 最终差异只包含 SPEC、任务记录与既有归档任务清单勾选。
