# 统一 FyAgent 软件一键安装免内容校验策略 — 实施计划

## 0. 启动门禁

- [x] 用户审批最终 PRD/设计/实施摘要。
- [x] 运行 `task.py start`，确认任务进入 `in_progress`。
- [x] 由实现执行方加载 `implement.jsonl`；写代码前按 `trellis-before-dev` 读取相关规范全文。
- [x] 复核工作树，只允许任务规划文件和本任务实现变更；不覆盖用户改动。

## 1. 固化测试目标

- [x] 在现有 Rust/service/platform/TypeScript contract 测试中先加入或改写失败用例：上游 hash、size、Windows identity/publisher/version/minimum OS、macOS bundle/team/signature/version/architecture 漂移不得阻止平台安装调用。
- [x] 保留并补强反向用例：固定 endpoint、metadata 大小上限、取消、磁盘/短写、受保护临时文件、Windows ACL/no-follow/file-ID、helper 身份与协议、macOS 挂载/目标边界、安装后目标缺失。
- [x] 增加现有 CLI 工具安装入口审计测试，证明其继续委托包管理器/官方脚本并执行安装后版本探测。

## 2. 简化 release source 与描述模型

- [x] 将 `ReleaseDescriptor` 从内容证明改为流程描述：移除 expected SHA；把 size 改为非准入 hint；更新 release ID 构造和 DTO。
- [x] 将 `AgentsMirrorSource` 改为只读取固定 manifest endpoint，删除 checksum endpoint、checksum parser、manifest/hash 交叉验证和仅服务于这些校验的错误分支。
- [x] 保留 metadata body 上限、严格必需字段解析、固定平台/架构 endpoint、缓存、重试、取消以及对远端 URL/delta 的忽略。
- [x] 更新 source/types 单元测试和 fixture，证明内容字段变化/缺失不会成为准入门槛，而流程必需字段缺失仍准确失败。

## 3. 重构下载产物与状态机

- [x] 下载器不再比较远端 Content-Length、expected size 或 expected SHA；进度总量使用可选 hint/响应 Content-Length，不能参与成功判定。
- [x] 下载过程中计算实际大小和本地动态摘要，保留 flush/sync、原子 finalize、受保护重开和动态摘要一致性检查。
- [x] `DownloadedArtifact` 暴露实际大小/动态摘要给安全交接层，不再暴露 release expected 值。
- [x] 删除 checksum mismatch re-anchor 分支和下载内容验证 stage；服务状态机从安全下载完成进入平台准备/安装，安装后 verification 保留。
- [x] 更新 job、service、shared DTO、parser、fixture、Legacy/V2 controller 与进度测试。

## 4. Windows 安装链迁移

- [x] 移除父进程 MSIX 固定 Name/Publisher/Version/MinVersion/Architecture、签名文件和包结构准入；删除不再需要的 production manifest validator 或将必要读取收敛为无 allowlist 的操作性 locator 提取。
- [x] 用下载实际大小/动态摘要替换 Windows pin、PackageBridge、helper control 中来自 release metadata 的 expected size/hash。
- [x] 保留 PackageBridge 固定根、operation ACL、held handle、no-follow、file ID/link/reparse/placeholder、copy/flush/no-replace 和清理协议。
- [x] 保留固定 helper CLI、authenticated pipe、Shell SID 和 `AddPackageByUriAsync` 默认原生安装；不得添加任意 URL/path/scope/bypass 参数。
- [x] 让安装后查询使用当前操作动态获得的 package locator/identity，并绑定同一 Shell SID；移除下载内容与固定 Stable identity/publisher 的比较。
- [x] 调整本地发现/launch/restart 的 identity 使用：兼容现有 Stable 安装，但不让固定 identity 成为新下载包准入或当前 job 成功的前提。
- [x] 更新 Rust 平台测试、helper portable tests、`codexUserHelperContract`、`codexWindowsUserScopeContract` 和必要的 Windows native smoke/compile 测试。

## 5. macOS 安装链迁移

- [x] 保留受控 DMG attach/detach和安装目标发现，删除 `hdiutil verify`、`codesign`、`spctl`、Team ID、Bundle ID、版本、架构、最低系统版本与 release 的准入比较。
- [x] 将“唯一顶层 `.app`”保留为操作性定位规则；错误语义改为无法定位安装目标，而非身份/签名不匹配。
- [x] 保留挂载边界、symlink/escape 防护、目标目录安全、运行中应用处理、原子替换、备份回滚与清理。
- [x] 安装后从实际目标动态读取身份/版本/路径并验证存在、可读取、可启动；runtime/restart 绑定实际路径/动态 identity。
- [x] 更新 macOS fake/fixture 测试，明确当前 Windows 主机不构成真实 macOS HIL。

## 6. 错误、UI、文档和全局契约

- [x] 删除或重定义仅服务于下载内容校验的稳定错误码、suggested action、四语言文案和复制详情；保留 transport、native install、result verification 和 runtime errors。
- [x] Legacy 与 V2 UI 不再显示“校验下载包”阶段；安装后验证仍准确显示。
- [x] 更新 `.trellis/spec/backend/codex-desktop-installer.md`，写入平台无关的“软件安装包不做 FyAgent 内容准入”规则与保留的操作安全边界。
- [x] 更新维护文档、用户手册及相关 contract tests，公开说明 FyAgent 委托来源/原生安装器且不验证发布者、签名、hash 或包身份。
- [x] 明确 Skills、插件、MCP、配置包、同步、release/CI 验证未改变。

## 7. 质量检查与真实验证

- [x] 运行受影响的 targeted Rust tests、user-helper tests、Vitest/contract tests，先快速收敛失败。
- [x] 运行 `mise run format:check`、`mise run typecheck`、`mise run test:unit`。
- [x] 运行 `mise run rust:fmt:check`、`mise run rust:clippy`、`mise run rust:test`，以及仓库定义的 Windows helper/native contract 检查。
- [x] 运行 Trellis `trellis-check` 全范围检查；核实所有 CRITICAL/WARNING 都回到实际数据源、调用链和测试。
- [x] Windows 原生安装 HIL 本次未执行；已将真实 MSIX、UAC/helper、PackageManager、ACL/file-ID/PackageBridge 现场行为明确记录为未验证剩余风险，未用 mock/编译结果代替。
- [x] 不把 Windows 本机结果外推为 macOS/Apple Silicon 真实安装成功。

## 8. 收尾、回滚与归档

- [x] 对照 PRD 每条验收标准检查 diff、测试和 HIL/剩余风险证据。
- [x] 运行 `trellis-update-spec`，只更新长期有效的安装器责任规范，不记录临时调试过程。
- [x] 确认没有持久化 schema、Skills/MCP/配置内容校验或无关重构变化；如出现则回滚该部分。
- [x] 创建聚焦代码变更的 commit，并记录验证结果。
- [x] 运行 Trellis finish/archive 流程，将任务归档；归档前不宣称任务完成。

## 回滚点

1. **source/model**：若 manifest-only 解析无法提供流程必需 locator，回滚该阶段并重新设计 locator，不恢复内容准入作为快捷方式。
2. **Windows helper/bridge**：任何 ACL、SID、pipe、file-ID 或同一对象保护退化时，停止并回滚 Windows 阶段；不得以“已取消校验”为由删除操作安全。
3. **macOS replace**：任何目标边界或回滚能力退化时，停止并回滚 macOS 阶段。
4. **DTO/UI**：Rust/TypeScript fixture 不一致时回滚未同步的 wire 改动，禁止以宽松 parser 掩盖 drift。
5. **整体**：任务不做数据库迁移；代码提交可整体 revert，临时下载与 PackageBridge 使用现有安全清理路径。
