# 项目草稿与工作说明恢复修复

## 修改边界

修复独立阶段审查的两个 P2；保持现有页面、两个保存入口和文件 generation/CAS 机制，不引入通用表单框架。仅改本次分配的 Page、renderer test、projects service/command，并新增独立原生恢复测试模块；没有提交，没有更改其他 worker 的 DAO、resources 或 tests.rs。

## 结果

- 项目编辑器按 projectId 保持挂载，两个字段各自保存可空草稿覆盖值。名称/说明可同时编辑；保存一个只清理该字段草稿，另一个跨刷新保留。普通说明写入使用读到的 context revision，避免拿新项目 revision 覆盖旧说明缓存。
- getContext 失败仅使工作说明区退化。项目元数据、归档、引用与交付/验证插槽保持可见。初次载入或重试也不卸载编辑器；按 projectId 校验缓存归属。文件读取不自动重试，用户可显式重读。
- 工作说明不可用时提供“重新建立工作说明”，确认后才传 recover=true。说明清楚指出以当前文本建立新版本、保留旧文件。
- ProjectsService::write_context 保留普通调用，委托新增 write_context_with_recovery。仅显式 recover 跳过旧 generation 的成功回读要求；归档/CAS/长度校验、全新 UUID 目录、nofollow 文件写入、digest 保存均保持。旧文件不修改；项目目录自身被替换仍拒绝。
- dependency_snapshot：版本相等的 Skill 仍为 Unverifiable；版本不同仍为 Drifted，以免将 DB 代际误称磁盘内容已确认。

## 接口与 root 接线

现有 projects_write_context 增加 `recover: Option<bool>`，缺省 false；无新 command/permission/ACL。TS port 的 `writeContext(request, content, recover?: boolean)` 与 invoke `{recover}` 由 root 按协调消息负责；本次全局 typecheck 已通过，说明该接口现已接上。

## 检查

- 最后修改后 `mise run test:unit -- tests/renderer/pages/projects/Page.test.tsx`：6/6 通过。新增覆盖同时编辑保存保留另一草稿、保存失败保留两字段、损坏说明时归档入口可用、恢复取消不写且确认后才发送 recover=true。
- `mise run typecheck`：通过。第一次检查发现测试 mock 返回 state 被推断为 string，已限定字面量后通过。
- 指定 Page/test 的 ESLint：通过。
- 指定文件格式化与 `git diff --check`：通过；Rust 使用 skip_children=true 仅格式化本次文件，没有重写他人模块。
- 新增 `project_recovery_tests.rs` 两项原生测试：损坏旧 generation 保留/陈旧 revision 拒绝/归档拒绝；generation symlink 不跟随且 project symlink 拒绝。**按 root 指令未执行原生编译，等待统一集成过滤测试**。
- 不属于原生 UI UAT、真实客户数据或 CLI 推理测试；root 继续独立复核及整体验收。

## 变更文件

- src/pages/projects/Page.tsx
- tests/renderer/pages/projects/Page.test.tsx
- src-tauri/src/services/projects/mod.rs
- src-tauri/src/services/projects/project_recovery_tests.rs（新增）
- src-tauri/src/commands/projects.rs
