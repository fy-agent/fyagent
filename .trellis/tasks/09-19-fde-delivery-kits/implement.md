## 已批准实施合同 v1（取代前文中的待定项）

主控于 2026-09-19 批准直接实现：纯文本 `.fyagent-kit.json` v1、native immutable library、项目页面板、导入不全局激活。协调来源：`/Users/serendipity/.codex/worktrees/fyagent-fde-control/fyagent/.trellis/tasks/09-19-fde-delivery-integration/coordination-contract.md` v1。

跨线统一 camelCase `projectId/projectRevision`；绑定为 `bindDeliveryKit(projectId, expectedRevision, kitId, kitVersion, manifestDigest, bindingIntentId)`；检查事实 `sourceClass=local_fixture`，无机器客户验收。先独立完成本机 validator/UI/port；项目及 evidence persistence 通过窄 adapter 接入，缺失明确 unavailable。包不写其他域表，原生文件库无需数据库迁移。共享注册修改列入交接，由 root 集成裁决。

实现边界：新增包域、原生服务/命令、独立 typed port、项目面板、合成内容和专属测试；保留现有全局配置/目录/凭据服务。前文仅规划的限制已被本次实施授权取代；保持不安装真实应用、不读取或修改真实客户数据/凭据。UI 采用现有 FyAgent 精确克制的开发工具样式和语义 tokens。

# 后续实施计划

当前只规划，保持任务 planning；主控继续派发后才进入实现。总体授权已具备，不向用户重复确认。实现模型按用户指定 GPT-6 快速模式由主控配置，本轮不切换执行模型。

## 顺序与并行条件

1. 主控选择 schema/入口/存储提案，分配共享注册文件；本线无跨线依赖的内容/schema/导入安全/演示可立即开工，不等项目和证据全部完成。
2. 新增本线 domain、原生服务、三包，冻结规范摘要的 Rust/TS fixture。先验证安全边界，再接 UI；预览零外部写入，应用仅入包库。
3. 经营周报确定性演示及正反样例，产出事实 DTO，不自行存证据记录，不把期望当结果。
4. 包 UI、依赖信息、导入/导出预览和回读；复用 CatalogMasterDetail/FeatureSearch/Dialog。UI 实施前读 `~/.codex/DESIGN.md` 及相关 visual/navigation/dialog specs，本轮不生成 UI。
5. 跨线接口冻结后接项目 binding/revision 和证据接收/回读；不可用明确 unavailable，无生产占位成功。
6. 主控统一注册 native 命令/ACL、feature ports、路由，集成共享文件后验证端到端最小切片。

## Ownership 建议

本线拟独占：`src/domain/delivery-kits/**`、`src/shared/features/delivery-kits.ts`、`src/shared/platform/tauri/feature-ports/delivery-kits.ts`、`src/shared/features/delivery-kits-ui/**`、`src-tauri/src/services/delivery_kits/**`、`src-tauri/src/commands/delivery_kits.rs`、`src-tauri/resources/delivery-kits/**` 和专属测试。

主控集成共享：`src/shared/features/ports.ts`、`types.ts`、`queries.ts`、`src/shared/platform/tauri/features.ts`、`src/app/primaryPages.tsx`、`src/shared/config/navigation.ts`、`src-tauri/src/lib.rs`、`commands/mod.rs`、`services/mod.rs`、Tauri permissions/capabilities 与架构 allowlist。

项目 schema/迁移、证据 schema/记录、SecretRef 服务不属于本线。现有 `presets.ts`、`catalog.ts` 默认只读；提升到 domain 或扩展 Skill 项目投影需要主控联合分配。

## 后续最小充分检查

- schema：三包 round-trip，未知字段/版本、重复 key、深度/大小、异内容冲突/重放、摘要跨端一致。
- native：取消、过期预览、并发、文件句柄失效、symlink/非法 ID/越界、写失败/重启回读；入库成功但绑定失败的部分状态。
- 安全：secret canary、SecretRef 不进入 DTO/日志/错误/导出；脚本拒绝；无自动网络/shell/安装/分配。
- 周报：金额/增长/达成、来源、缺失/重复/窗口/币种/除零；改 expected 不可欺骗宿主。
- 集成：项目 A/B、旧 revision、删除引用保护；证据写入失败不能显示已记录；合成样例与真实连接分开。
- UI：预览取消/冲突/重试、旧请求结果、空态、键盘焦点；浏览器 fixture 仅证明交互，临时 HOME 的 native 检查另行取证。

专属测试创建后，使用现有入口，例如：

```sh
rtk mise run test:unit -- tests/domain/delivery-kits.test.ts
rtk mise run rust:test -- delivery_kits
rtk mise run typecheck
rtk mise run check:contracts
```

本轮不运行上述产品测试，只做文档合同及 JSON 有效性核对。最终全量检查、构建与实机测试由主控按集成范围安排，本轮不替换 `/Applications/FyAgent.app`。

## 回退与最终证据

安全失败先阻断；绑定失败不清理他人包/共享资源。回退只恢复指定项目包版本绑定。最终交主控：源码版本、静态检查、临时包库 native 回读、合成样例输出、真实服务未验证项；不拿静态模板完整代替业务可运行证据。
