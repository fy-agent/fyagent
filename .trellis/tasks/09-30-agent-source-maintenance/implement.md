# 执行计划与状态

## 已完成的准备工作

- [x] 获得用户对任务创建、维护实施和版本交付的明确授权。
- [x] 查询现有 open mirror issue，结果为空；尝试创建 Issue，403 被拒绝。
- [x] 重新读取 main 基线、必需 CI 检查和 release contract。
- [x] 尝试创建维护分支，403 被拒绝；未更改远端。
- [x] 核实 Hermes 官方日期 tag 下的项目产品版本及 OpenCode 官网渠道。
- [x] 构建 Hermes 候选代码及 6 项 Rust 回归测试。
- [x] 验证 baseline 与 GitHub blob 一致；`git apply --check` 通过。

## A. 任务落库和授权环境

- [x] 确认当前 GitHub 连接/本地已授权 gh 可写目标仓库，不索要聊天中的 token。
- [x] 重读 main、工作树、活动 Trellis 任务和环境限制，不覆盖他人未提交修改。
- [x] 在实际仓库创建或导入此任务，按仓库 Trellis 流程登记会话、上下文与 task status。
- [x] 在独立分支导入候选 patch；源码 blob 不匹配则重做合并，禁止强制覆盖。

## B. Hermes

- [x] 先明确 latest 字段是产品 stable 还是安装渠道 latest，审查对应 UI 提示。
- [x] 运行新增 Rust 回归测试，补足超时、重定向、404 和流式超限的传输测试。
- [x] 审查候选修复，必要时完成可信安装 owner 判定和来源展示。
- [x] 更新 existing external-agent-sources owner contract，移除“Hermes 仅 PyPI”的旧约束。
- [x] 联网读取当前 stable tag 下项目元数据并与已安装 CLI 版本对照。

## C. OpenCode 与 Claude/Grok

- [x] 查明新旧 OpenCode CLI 官方包身份、稳定渠道及已安装来源识别。
- [x] 添加渠道分离测试，再实现版本查询；不改变 Desktop 的下载 authority。
- [x] 给镜像降级添加诚实的来源/最新性状态；覆盖官方失败、tag 落后、exact 包缺失和 hash 不一致。
- [x] 重新审查 #198，验证其 Claude ownership 修复与本轮变更兼容，避免重复修复。

## D. 全产品在线与原生验收

- [x] 逐项完成 prd 的 10 产品/12 表面矩阵，每个支持架构保存证据。
- [ ] 新安装、已有安装升级、多 owner、取消/失败恢复、重扫和启动。
- [x] 对不支持安装的 CLI 标为不适用；对网络错误标为未验证。
- [ ] Windows x64、Windows ARM64 和 macOS 的支持目标分别验收，禁用平台不计失败。

## E. 提交与发布

- [ ] 在授权环境先运行 `mise run check:backend`，随后 `mise run check:contracts` 和 `mise run check`。
- [ ] 新增 Rust 测试筛选命令可用 `cargo test --manifest-path src-tauri/Cargo.toml --lib services::tooling::versions::tests::hermes_`，仅在仓库支持且已授权的宿主/VM中执行。
- [x] 确定未占用的补丁版本，调用现有版本工具；更新变更记录和安装支持差异。
- [ ] 提交 PR、实际等待并读取必需 CI 结果；遵循 merge governance 合并。
- [ ] 记录最终 source SHA，走 release contract 所定义的稳定 tag 发布路径。
- [ ] 校验正式发布资产、摘要、signing-status 和 provenance；重新下载并抽验。
- [ ] 写入真实 commit、PR URL、CI run、release URL 和逐项验收结果，再归档任务。

状态说明（2026-09-30）：
- “运行新增 Rust 回归测试”：测试已写入，但主 crate 在 Linux 开发主机本就编译不过，只能由 CI 的 Windows/macOS runner 执行，结果以 PR 的 CI 为准。
- “逐项完成矩阵”：元数据与下载别名已逐项实测（research/source-matrix-20260930.md）；D 组的真机安装/升级/启动验收因没有 Windows/macOS 真机未做，保持未完成。
