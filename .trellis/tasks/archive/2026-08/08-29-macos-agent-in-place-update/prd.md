# Stage 2 — macOS 原位置安装与更新

## Goal

基于 Stage 1 的安装目标权威合同，为 QoderWork CN、TRAE Work CN 和 WorkBuddy 建立一个可恢复的 macOS DMG 部署路径：

- 更新严格作用于用户选择的现有 `.app` 候选；
- `/Applications` 保持 `/Applications`，`~/Applications` 保持 `~/Applications`；
- 首次安装只使用 backend 返回的明确目的地；
- 权限失败、运行中、身份漂移或验证失败时保留旧应用；
- 所有产品执行相同强度的安装后回读。

## Requirements

### 1. Target semantics

- 本任务硬依赖 Stage 1 的 `inventoryId + targetId + expectedTargetRevision`。
- `update` 必须引用一个当前仍然 trusted 的 macOS app-bundle candidate；不得根据 bundle 文件名、扫描顺序或默认根重新选择。
- 目标是选中候选的**精确父目录和 bundle 路径**。即使下载包内应用名称变化，也不得未审查地重命名或迁移现有安装。
- 多候选、候选消失、revision/identity/scope 漂移时零写入并返回刷新/重新选择。
- `install` 使用 Stage 1 返回的 fresh destination。对标准 DMG 产品，系统 Applications 和用户 Applications 是不同选项；产品政策可以推荐系统位置，但不允许代码无条件写用户目录。

### 2. No silent scope fallback

- 更新系统级候选时，权限失败必须返回 `authorization_required` / `permission_denied`，旧应用保持不变。
- 更新不得把系统目标静默降级到 `~/Applications`，也不得在用户目标失败时改写 `/Applications`。
- 首次安装只有在用户显式选择另一个 destination 后才能改变 scope；一次失败不能自动改变选择。
- 若项目尚无经审查的 macOS elevation/authorization adapter，系统目标保持不可自动执行并提供明确人工路径；不得通过任意 `sudo`/AppleScript shell 拼接补洞。

### 3. Reuse the existing replacement transaction

- 复用/泛化 `codex_desktop/platform/macos/dmg.rs` 的同卷 staging、backup、rename、验证和恢复思想；不得继续维护 `agent_install/desktop.rs` 中第二套简单 `hdiutil + ditto` 直拷贝流水线。
- 共享 owner 至少负责：
  - 受控 job 目录和 DMG mount/detach；
  - mount 内唯一 `.app` 发现；
  - 下载 bundle identity 预验证；
  - 目标父目录和 generated path confinement；
  - staging bundle 复制与验证；
  - 旧目标备份、commit、安装后验证；
  - 失败回滚与 generated artifact 清理；
  - commit point 前取消和 commit point 后不可取消语义。
- Codex Desktop 继续作为回归金样；抽取不能改变其现有 stable bundle、权限 fallback 或 restart 语义。

### 4. Product-specific policy

- QoderWork CN：下载 bundle ID 必须为 `com.qoder.work.cn`；本机回读使用可信 plist 版本。
- TRAE Work CN：bundle ID 必须为 `cn.trae.solo.app`；可比较版本优先使用 `Contents/Resources/app/product.json` 的 `tronBuildVersion`。
- WorkBuddy：bundle ID 必须为 `com.workbuddy.workbuddy`；本机 marketing version 与远端长版本使用现有审查过的等价规则。
- Product policy 只提供 identity/version equivalence/source descriptor；不得各自实现文件替换事务。

### 5. Running application and restart

- 安装前使用 closed bundle identity 检查目标应用是否运行；不得按窗口标题或进程显示名强制关闭。
- 若没有可信的协调关闭能力，要求用户先退出并重新检查；不能在无法确认的状态下 force kill。
- 如果安全部署允许替换运行中 bundle，也必须明确告知“下次启动生效”并在部署后验证磁盘目标，不得把正在运行的旧进程版本当作新安装回读。
- 安装成功后由用户选择是否启动；启动必须针对刚验证的候选路径。

### 6. Authoritative verification

- 复制/rename/命令退出成功不能单独成为 `succeeded`。
- fresh inventory 必须证明：
  - 选定位置存在且是普通 app bundle；
  - bundle ID 与产品 identity 一致；
  - 版本满足目标 release 的产品比较规则，或在 versionless latest 情况下至少完成身份/可运行形状验证；
  - 没有在另一个 scope 产生未声明副本；
  - launch target 仍绑定该候选。
- QoderWork、TRAE Work、WorkBuddy 使用同一个 verifier contract；不得只验证 TRAE。

### 7. UI and shared components

- 复用 Stage 1 shared target picker；页面不再根据 `installed` bool 推断目标。
- 操作前明确显示位置标签、scope、当前版本、目标版本/最新版语义、是否需退出/授权。
- 复用共享 lifecycle status/notice；只有出现第二个真实消费者时才抽新状态组件。
- 成功文案必须引用权威回读；权限、取消、回滚、恢复失败使用不同 closed reason。

### 8. Same-domain defect policy

测试中发现 mount 清理、bundle identity、候选 location、running check、rollback、重复副本或生命周期进度缺陷时，在本任务内修复并加回归。Windows、Auth 和无关 V2 页面问题不得扩入。

## Non-goals

- 不实现 Windows 安装。
- 不改变下载源/版本 resolver 的既有产品政策，除非修复本任务验证所必需的同域错误。
- 不引入通用 privileged shell、任意路径复制或 Finder automation。
- 不承诺静默安装到系统目录；系统写权限必须通过经审查的 OS/应用授权边界。
- 不把首次安装的显式 user-scope 选择视为错误；错误的是更新过程中未经用户同意改变 scope。

## Acceptance Criteria

- [ ] `/Applications/Product.app` 更新后仍只有该系统位置被更新，`~/Applications` 不产生新副本。
- [ ] `~/Applications/Product.app` 更新后仍在原用户位置，系统目录不产生新副本。
- [ ] 系统和用户两份候选同时存在时必须选择；未选择/过期选择零写入。
- [ ] 系统目标权限失败保留旧 bundle 和版本，不自动 fallback 到用户目录。
- [ ] Fresh install 只使用用户选中的 backend destination；失败后 scope 不改变。
- [ ] Staging/backup 均在受控位置，拒绝 symlink/replacement drift/非预期文件类型。
- [ ] Staged bundle 在 commit 前完成 identity/shape 验证。
- [ ] Commit 后验证失败时恢复旧 bundle；若恢复无法证明完成，返回明确 recovery-required 状态并保留证据路径的 backend-private ledger。
- [ ] 取消仅在 commit point 前生效；commit point 后 UI 不再显示可取消。
- [ ] QoderWork、TRAE Work、WorkBuddy 都经过相同 verifier，且测试证明不会由 copy success 直接进入 succeeded。
- [ ] 安装/更新成功后 fresh inventory 的 candidate ID/版本/location 与选定目标一致。
- [ ] 运行中应用的处理基于 bundle identity，不能按名称 force kill；不支持协调关闭时给出可操作提示。
- [ ] Codex Desktop macOS installer 的完整回归保持通过。
- [ ] 在 disposable macOS profile/HIL 中覆盖 system、user、多副本、权限拒绝、运行中、取消、验证失败、回滚成功/失败和重新启动。

## Dependency

Stage 1 target-authority contract must be merged and reviewed before this task starts implementation.
