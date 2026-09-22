# 客户项目模块移除：范围研究

## 结论与版本定位

当前工作树 `/Users/<username>/fyagent` 位于 `codex/frontend-interaction-v3-1-20260826`，该分支远端跟踪已失效且存在大量既有 dirty changes；本研究未修改这些内容。当前根目录对 `src/` 的初步搜索无客户项目命中，是因为该模块实际位于另一条已注册 worktree：

- `/Users/<username>/.codex/worktrees/fyagent-fde-projects/fyagent`，`codex/fde-project-isolation`，`aa22316e`（实现合同及定向验证交接）。
- 模块实现起点为 `d43b0388`/`230f80eb`；前端接线为 `398dc192`；后续历史中还存在 `0fec6c75`、`cd8b4ced`、`c6cfbe94`，分别扩展恢复/工作区/验证与浏览器布局。`git worktree list` 还显示 `codex/fde-delivery-integration` 等相关 worktree，故删除规划应按完整提交族检查，而不能只按当前分支搜索。

后续提交已将模块扩大到交付包与验证工作区；已直接读取 `cd8b4ced` 的文档和 `c6cfbe94` 的浏览器测试，但未把这些提交检出为工作树。未能证明它们是否已合并到用户指定的目标发布分支，规划阶段应把“是否在目标版本”列为验收前检查项。

## 可删除的产品入口与前端边界

模块的独立入口和接线为：

- `src/pages/projects/Page.tsx:1-356`：客户/项目列表、归档、项目资料、资源/凭据绑定、Codex 准备、交付包和依赖快照插槽；样式为 `src/pages/projects/projects.css`。
- `src/domain/projects/index.ts:1-143`：Customer、Project、ProjectContext、ProjectDependencySnapshot、资源/凭据/kit DTO 与 strict schema。
- `src/shared/features/projects.ts:1-81`：`ProjectsPort` 全部 API 与错误码；`src/shared/platform/tauri/feature-ports/projects.ts:1-139` 为 18 个 Tauri 命令的 renderer 适配。
- `src/shared/config/navigation.ts:1-64`：`projects` id、`/projects` 路径和“客户项目”导航项；`src/app/primaryPages.tsx:23-68`：懒加载入口。
- `src/shared/features/ports.ts:1-2,257-260`：`FeaturePorts.projects` 聚合字段；`src/shared/platform/tauri/features.ts:1-37` 与 `src/shared/platform/browser/features.ts:9-30` 分别注册 native 和 browser fallback。

前端删除清单应覆盖上述文件及所有 import/类型聚合，而不是只删页面。检查 `src/app/ProjectsWorkspace.tsx`（后续提交 `cd8b4ced` 引入）以及 `src/shared/features/delivery-kits-ui/ProjectDeliveryKitsPanel.tsx`、`src/shared/features/verification/VerificationPanel.tsx` 的 `projectId/projectRevision` props；这些属于后来与项目页耦合的交付/验证工作区，若用户要求“完全移除关联内容”，需一并移除项目专属分支，同时保留仍被其他模块使用的通用验证/交付能力。

## Native 后端、IPC 与存储

- `src-tauri/src/commands/projects.rs:1-166` 暴露客户 CRUD、项目 CRUD/归档、资源/凭据绑定、context 读写、Codex 准备、delivery kit 绑定和依赖快照；命令名集中在 `projects_*`。
- `src-tauri/src/services/projects/mod.rs:1-329` 是 `ProjectsService`、revision CAS、归档约束、context generation 发布与 kit authority；`files.rs:1-180` 管理不可变 generation 目录和 `context.md`；`resources.rs:1-112` 从既有 provider/prompt/MCP/skill 与 managed-auth 只读投影选项；`tests.rs` 为领域测试。
- `src-tauri/src/database/dao/projects.rs:10-29,31-207` 创建 DAO 与四张本地域表：`fde_customers`、`fde_projects`、`fde_project_kit_intents`、`fde_project_context_versions`。项目完整聚合 JSON 存在 `fde_projects.document`，context 文件根目录在 `src-tauri/src/lib.rs:1753-1757` 注入为 `get_app_config_dir()/projects`。
- `src-tauri/src/database/mod.rs:54-56` 将 schema 版本定为 22；`src-tauri/src/database/schema.rs:335-339,551-559` 在初始化及 21→22 迁移中创建这些表。完整移除必须设计旧数据库的 22→后续版本迁移：删除/归档四表、索引及项目目录，不能仅停止创建新表。
- `src-tauri/src/services/mod.rs`、`src-tauri/src/database/dao/mod.rs`、`src-tauri/src/commands/mod.rs` 为模块声明；`src-tauri/src/lib.rs:1753-1757,2144-2161` 注入 service 并注册全部命令；`src-tauri/permissions/projects.toml:1-4` 是专属 ACL，`src-tauri/capabilities/default.json` 需删除对应 capability 引用。

## 备份、同步、导入导出与数据保留

`src-tauri/src/database/backup.rs:75-115` 明确把四张 `fde_*` 表同时列入 `SYNC_SKIP_TABLES` 和 `SYNC_PRESERVE_TABLES`：WebDAV/S3 同步不上传项目数据，但导入远端配置时从本机快照恢复。普通完整 SQL 导出仍由 `src-tauri/src/commands/import_export.rs:19-60` 调用 `Database::export_sql/import_sql`，因此会包含项目表；数据库导入前还会生成备份（`backup.rs:175-205` 附近）。此外，context 文件在应用配置目录下，不属于 SQLite 表，普通 SQL 迁移不会处理它。

数据保留建议：

1. 删除前先提供一次明确的本地导出/备份提示，保留现有数据库备份文件和项目目录的可恢复副本，避免把客户资料静默丢失。
2. 新版本迁移应从 SQLite 删除四张 `fde_*` 表及索引，并删除/隔离 `get_app_config_dir()/projects` 下的 generation 文件；迁移完成后应使旧项目数据不可再被产品读取。
3. 完整 SQL 导入对旧备份要有兼容策略：忽略/丢弃 `fde_*` 表并记录一次迁移提示，不能因旧备份含项目表而恢复已移除域。WebDAV/S3 的 preserve/skip 列表应移除这些表名，避免继续保留死数据；但不要触碰其他本地专属表。
4. 若产品政策要求客户项目资料可审计，建议在迁移前把导出文件和 context 目录标记为“旧客户项目归档”，由用户自行保存；当前代码未提供独立项目导出命令，只有完整配置 SQL 导出和后续交付/验证 Markdown/JSON 导出（后续提交文档 `cd8b4ced:docs/user-manual/zh/4-extensions/4.7-fde-projects.md:40-42`）。

## 测试、文档与引用

直接可见的测试引用包括：`tests/renderer/pages/projects/Page.test.tsx`、`tests/renderer/platform/projects.test.ts`、`tests/renderer/platform/tauriAclContract.test.ts:128-130`、`tests/renderer/app/router-shell.test.tsx:30-33,134-138`、`tests/renderer/app/architecture.test.ts`，以及 native `src-tauri/src/services/projects/tests.rs` 和 `src-tauri/src/database/backup.rs:1906-1923`。后续提交又增加 `tests/browser/projects-layout.spec.ts`（`c6cfbe94`，完整页面布局/滚动验收）及交付/验证相关测试，需按提交历史复核。

文档与任务引用：`.trellis/tasks/09-19-fde-project-isolation/{prd.md,design.md,implement.md,implementation-ready.json,implementation-status.json}` 记录合同、命令和表结构；`.trellis/tasks/09-19-fde-delivery-integration/` 的设计/评审文件和后续文档 `docs/user-manual/zh/4-extensions/4.7-fde-projects.md:1-42` 介绍用户功能。后者仅存在于后续提交读取结果中，当前 `fyagent-fde-projects` 工作树未落盘该文件，目标版本是否包含它未知。

## 与共享能力的边界

项目域只引用共享 provider、prompt、MCP、skill 和 managed-auth 的只读元数据（`src-tauri/src/services/projects/resources.rs:7-110`），不应删除这些共享资源及其普通配置页。它还复用通用 SQLite backup/import、Tauri feature-port 聚合、交付包/验证组件；规划时应拆出“移除项目专属 wiring”和“保留共享能力”的边界。`ProjectKitReader` 当前为 `UnavailableKitReader`（`src-tauri/src/services/projects/mod.rs:33-42`），属于项目专属 authority，应随域移除；delivery kit/verification 的通用服务需按引用图决定是否保留。

未知项：未在本次只读范围内核对正式发布分支的合并状态、用户设备上的实际 SQLite 文件数量、`get_app_config_dir()/projects` 是否已有真实客户资料，以及所有后续交付/验证提交是否已进入目标版本。实施前必须在目标分支复核 `git log --all -- <path>`、构建入口与迁移测试，并由用户确认数据保留/导出政策。
