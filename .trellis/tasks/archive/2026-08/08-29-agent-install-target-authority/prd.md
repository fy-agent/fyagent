# Stage 1 — 安装目标权威合同

## Goal

在不暴露任意路径能力、不创建第二个 Agent Catalog 的前提下，把当前“一个 Agent 只有一个 installed bool”的模型升级为：

- 可列出多份已安装候选；
- 可区分 user/system/custom scope 和安装所有者；
- 可列出首次安装允许的目标位置；
- 可让用户选择本次管理目标；
- 可在执行前拒绝过期候选、位置漂移和隐式跨 scope；
- 可由 macOS、Windows、启动、更新和后续验证共同复用。

本阶段只建立领域合同、聚合/选择政策、IPC 和共享 UI 基础，不实现完整的 macOS 替换或 Windows EXE/MSI 执行；平台执行分别由 Stage 2/3 完成。

## Requirements

### 1. Installation inventory

- 新增一个只读、无网络的安装 inventory façade，按 canonical `AgentCatalogId` 返回当前宿主的候选快照。
- 一个候选至少表达：
  - backend 生成的 opaque `candidateId`；
  - `candidateRevision`；
  - canonical Agent ID；
  - `scope = current_user | all_users | custom | unknown`；
  - `owner = vendor_installer | package_manager | fyagent | unknown`；
  - `packageKind = app_bundle | exe | msi | msix | unknown`；
  - 本机版本（若有可信证据）；
  - launch/install/update eligibility；
  - closed evidence/reason codes；
  - 隐私安全的 `locationLabel`。
- 原始绝对路径、registry handle、AUMID/PFN、bundle path、signer fingerprint 和内部 identity key 保持 backend-private。
- `locationLabel` 只用于显示：已知根使用 `/Applications`、`~/Applications`、`%LOCALAPPDATA%`、`%PROGRAMFILES%` 等符号别名；自定义路径必须边界化和去身份化。需要定位文件时使用受控的“在 Finder/Explorer 中显示”动作，而不是把路径作为后续命令输入。
- 同一物理安装由多条证据发现时要合并并保留 provenance；不同安装不得因显示名或版本相同而合并。
- stale registry、路径存在但 identity 不匹配、证据冲突等条目可以显示为不可执行候选，但不得被选为安装/更新目标。

### 2. Fresh-install destinations

- Inventory 同时返回由 backend 产品/平台政策生成的首次安装目标选项，例如 system Applications、user Applications、vendor installer choice。
- 目标选项使用 opaque `destinationId + destinationRevision`；renderer 不能提交一个任意目录。
- 可用目标必须明确 `scope`、是否需要 elevation、当前是否可写以及原因码。
- 没有产品/格式证据时，不凭跨产品习惯猜默认目录。

### 3. Selection and stale protection

- `update` 必须绑定一个已有候选；`install` 必须绑定一个 backend-projected fresh destination，或明确由 vendor installer 自己拥有目标选择。
- `launch` 在只有一个 trusted candidate 时可以兼容旧调用；多候选时必须返回 `target_selection_required`，不能按枚举顺序启动。
- 新动作请求通过 `inventoryId + targetId + expectedTargetRevision + expectedReleaseId?` 绑定之前展示给用户的快照。
- 后端在任何写入或启动前重新枚举并验证 identity、scope、owner 和 revision；漂移返回 `refresh_required` / `target_changed`，不尝试“最接近”的候选。
- 本阶段的“选择”默认是本次操作选择，不强制建立永久默认项。若实现持久偏好，只能保存稳定 backend identity，并且每次使用前重新验证；不能保存 renderer 路径。

### 4. Summary/readiness compatibility

- `AgentInstallReadinessDto` 继续提供目录卡片所需摘要，但不得把 multiple/ambiguous 压成 installed + 一个版本。
- 摘要至少区分：`not_observed | single | multiple | unsupported | unknown`，并提供是否需要目标选择。
- Contract version 必须显式升级；Rust 与 TypeScript parser 同步 fail closed。
- 旧调用在唯一 trusted candidate 时可兼容；任何多候选、冲突或过期状态必须拒绝隐式执行。

### 5. Shared architecture

- 将候选领域、快照、revision、去重和选择政策放在一个 crate-scoped shared owner，平台 adapter 只负责产出证据。
- 优先复用或泛化 Codex Desktop 已有 `TrustedInstallationCandidate`、scope、candidate inspection 和 prepared-package identity；不得复制一套近似类型后让两者长期分叉。
- 为前端新增或扩展一个 shared `InstallTargetPicker` / `LifecycleTargetDialog`，由 Agent 目录和后续生命周期页面复用。组件只处理 typed targets、disabled reasons 和选择，不拥有安装业务。
- 为 inventory query keys、parser 和 target-selection view model 建立单一 shared owner；页面不得自行拼 DTO 或筛选 winner。

### 6. Same-domain defect policy

实现/测试中发现候选去重、状态摘要、目标选择、受控 reveal 或现有唯一候选兼容问题时，在本任务内修复并加测试。平台部署、Auth、Prompt/Memory 或无关页面缺陷不扩入本任务。

## Non-goals

- 不执行 macOS bundle 替换或 Windows EXE/MSI/MSIX 安装。
- 不扫描整块磁盘，不接受用户输入路径，不新增通用文件浏览/执行命令。
- 不推断认证、模型、额度或真实请求可用性。
- 不把 installer registration、文件存在或 PATH 单独升级为 trusted candidate。
- 不修改 FyAgent 自身安装器/更新器。

## Acceptance Criteria

- [ ] 新 inventory DTO、closed enums 和请求 DTO 在 Rust/TypeScript 中 exact-key、exact-version、`deny_unknown_fields`/严格 parser 一致。
- [ ] Renderer 请求中不存在 URL、path、command、installer args、token、hash、signer 或 bypass 字段。
- [ ] 一个 Agent 的两份 trusted 安装会返回两个候选；状态为 `multiple`，不自动选择第一项。
- [ ] 同一物理安装由两条证据发现时合并为一个候选并保留 evidence codes。
- [ ] stale registration、identity mismatch 和 conflicting evidence 可见但不可执行。
- [ ] known system/user roots 使用隐私安全 label；原始路径不会进入日志、错误、URL、localStorage 或普通 analytics。
- [ ] Fresh install 只能选择 backend 返回的 destination；renderer 不能构造 custom path。
- [ ] Update 缺少 target、target revision 漂移、inventory 过期或 candidate 消失时零写入并要求刷新。
- [ ] 唯一 trusted candidate 的旧 launch 路径保持兼容；多候选 launch 明确要求选择。
- [ ] `AgentInstallReadinessDto` 不再把 multiple/unknown 渲染成确定单版本成功状态。
- [ ] Shared target picker 同时覆盖 candidate 和 fresh destination，具备键盘、disabled reason、loading/error/refresh 状态，并被至少两个真实入口复用或在本阶段保留为一个入口加已确定的 Stage 2/3 第二消费者。
- [ ] 架构测试阻止产品 adapter 自行实现 target selection/dedup，也阻止页面直接调用新的 Tauri 命令。
- [ ] #31、#47、#101 中多安装与“保持并回读”要求可逐项映射到测试或后续 Stage 2/3 验收项。

## Dependencies

- Reuses completed `08-25-agent-install-auth` action façade and current Agent Catalog.
- Must complete before Stage 2 and Stage 3 implementation starts.
