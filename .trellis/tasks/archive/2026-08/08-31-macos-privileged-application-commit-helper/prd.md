# 实现 macOS `/Applications` 封闭特权提交 Helper

## 0. 任务状态与边界

- 状态：由 `task.py start` 转入实施；归档前若正式签名 HIL 不可用，系统目标仍保持禁用。
- 平台：仅 macOS；最低系统版本保持项目当前的 macOS 12.0。
- 开发形态：独立 Trellis 任务；不创建通用提权框架或产品私有 helper。实现可按 `research/implementation-seam.md` 并行分包，但共享同一 ABI 与产品表。
- 依赖关系：前置 `08-31-macos-agent-install-update-experience` 已归档，其 helper-facing 合同（下载、身份、inventory、普通用户事务、系统目标 `authorization_required`）以当前树为准。后续 `08-31-macos-agent-directory-install-policy` 拥有目录排序、国产仅安装、Claude/OpenCode Agent 安装面和 Claude Desktop source；本任务不得实现那些产品面。
- 交接：本任务交付 `MacSystemCommitPort`、封闭 helper 与签名 verifier。生产 `/Applications` action 在正式 HIL 前保持禁用，供后续 install 任务复用而非抢先启用。
- 发布门槛：没有经过 Developer ID 签名、公证和真实系统目录 HIL 时，生产中的 `/Applications` 一键目标必须继续保持 `authorization_required`，不得提前放开。

## 1. Goal

复用 Apple Service Management、Authorization Services 与经过审查的成熟 Swift 组件，为已经由 FyAgent 用户态流程验证的桌面应用提供一个最小、封闭、可审计、可回滚的系统级提交边界：

```text
已验证应用能力
  -> 用户明确授权
  -> helper 在 /Applications 的固定槽位完成 staging / replace / rollback
  -> 用户态重新读取权威 inventory
  -> succeeded / rollback_restored / recovery_required
```

helper 不是文件管理器、安装器下载器、脚本执行器或通用 root 服务。它只负责已知桌面产品在固定系统目录中的最后提交与必要恢复。

## 2. 用户价值

完成后，用户在 FyAgent 中选择系统级安装目标时，可以通过正常的 macOS 管理员授权把受支持桌面应用安装到 `/Applications`，而不是静默落到 `~/Applications`。已有系统级安装能够在原位置更新；失败时旧应用可恢复，结果和恢复状态可明确展示。

## 3. 产品范围

首期只允许当前后端已经拥有已验证 macOS app-bundle 身份策略的桌面产品：

- Codex / 新 ChatGPT 桌面端的稳定身份（`ChatGPT.app` 默认，历史 `Codex.app` 仅用于已存在槽位）；
- OpenCode Desktop；
- QoderWork CN；
- TRAE Work CN；
- WorkBuddy。

Claude Desktop 身份与镜像 source 属于后续 install-policy 任务，本任务不得预加产品行。产品 ID、Bundle ID、固定目标 basename、版本来源和等价规则必须从后端权威策略生成，Swift helper 不得手工复制第二张常量表。未知产品、未知目标槽位或仅有显示名称的产品一律拒绝。

## 4. Requirements

### R1 — 复用优先是硬性验收规则

按以下顺序决策：

1. 复用 FyAgent 已有的下载、DMG mount、应用身份、目标 authority、job、staging、replacement、rollback 和 inventory readback owner；
2. 复用 Apple Service Management、Authorization Services、XPC、Foundation 与 POSIX 文件能力；
3. 复用经过审查并固定版本的成熟开源组件处理 helper 安装、授权包装、XPC 编解码和连接身份校验；
4. 只为 FyAgent 的封闭业务操作编写最小适配层；
5. 只有系统 API 和成熟组件无法满足时，才允许实现最小的缺失文件事务逻辑，并记录不能复用的具体原因。

禁止：

- 自行重写 `SMJobBless` 安装器；
- 自行发明通用 XPC RPC 框架；
- 复制 Mist、SwiftAuthorizationSample 或其他样例里的通用命令/任意路径业务接口；
- 在 helper 内重新实现下载、远端元数据、DMG 挂载或产品 release 解析；
- 使用 `sudo`、`osascript ... with administrator privileges`、`AuthorizationExecuteWithPrivileges`、setuid 程序或任意 shell 提权；
- 为每个产品建立一套 helper、协议或替换事务。

### R2 — 一个生产注册路径

- 项目最低支持 macOS 12，因此本任务不得只实现 macOS 13+ 的注册方案。
- 同一发行版只能启用一个生产 helper 注册/安装路径；不得把两个提权机制做成运行时 fallback 链。
- 注册失败、用户取消、helper 被禁用或版本不兼容时，系统目标保持不可执行，并给出稳定、可恢复的状态。
- 对未来迁移到新 Service Management API 保留窄接口边界，但迁移本身不是本任务的完成条件。

### R3 — helper 业务面必须封闭

helper 只允许以下语义：

- 查询 helper 协议、版本和健康状态；
- 提交一个已知产品到一个编译期/生成期固定的 `/Applications/<KnownName>.app` 槽位；
- 在同一次事务内恢复该槽位的已验证备份；
- 显式移除 FyAgent 自己的 helper 与对应注册项。

helper 请求和任何 renderer/Tauri IPC 均不得包含：

- 任意源路径或目标路径；
- URL、命令、可执行文件、工作目录或参数数组；
- 任意 copy/move/delete/chown/chmod 操作；
- 任意 Bundle ID、basename、Team ID 或 destination 字符串；
- hash 绕过、权限绕过、验证绕过或“force”字段；
- token、密码、Keychain locator 或远端凭据。

产品、目标槽位、操作类型、协议版本和错误类型均为封闭枚举。未知字段、未知枚举、保留位非零和超长值必须 fail closed。

### R4 — renderer 永远不能直接触达 root 能力

- renderer 继续只提交现有 Agent/Codex 封闭 action 与 opaque inventory/target/revision capability。
- Rust 后端完成目标重验证、应用验证、job 边界和用户授权协调后，才可调用 crate-private `MacSystemCommitPort`。
- Swift bridge、XPC 路由、文件描述符和授权 external form 不得暴露为 Tauri command 参数或前端类型。
- 不新增通用 shell、filesystem、sidecar 或 process Tauri 权限。

### R5 — 源对象必须是能力，不是字符串路径

- 用户态必须先完成下载、DMG 只读挂载、唯一 `.app` 发现、Bundle 身份/版本/可执行形态验证与 staging 准备。
- 进入特权边界时，源应用必须通过已经打开并重新检查的目录/文件能力交付；不能只把用户可写路径字符串交给 root helper。
- helper 在 mutation 前必须重新检查文件类型、文件身份、目录 containment、symlink/reparse 等价风险、产品身份和 source revision。
- 源能力变化、关闭、漂移、指向非目录或无法完整验证时，零目标写入。

### R6 — 目标必须由 helper 自己从封闭策略解析

- fresh system install 只允许产品默认的 `/Applications/<FixedName>.app`。
- system update 只允许 inventory 已选择、且属于该产品有限允许槽位集合的现有 `/Applications` 直接子项。
- helper 不能接受自定义目标；嵌套目录、外置卷、自定义系统路径和另一个 Applications scope 保持 unsupported/manual。
- 更新严格保持被选中的系统槽位，不得创建 `~/Applications` 副本。
- 已存在于 `~/Applications` 的应用继续由普通用户事务原位置更新；本任务不做静默迁移。

### R7 — 每次 mutation 都需要可验证的用户意图

- 安装 helper 的授权不能自动等同于之后所有 root mutation 的授权。
- fresh install、system update 和显式 helper removal 在调用 helper 前都必须取得对应的、短生命周期的 Authorization Services 权利。
- helper 必须在执行 mutation 的紧邻位置重新验证授权，而不是只相信客户端声称“用户已同意”。
- 授权 external form 只能在经过身份验证的 XPC 请求内短暂传输，不记录、不缓存、不写日志；终态后销毁权利。
- 用户取消、认证失败、授权过期或权利定义漂移时，零目标写入，并返回独立 reason code。
- 不承诺每次必然出现密码框；具体认证交互与凭据缓存由 macOS Security Server 决定，但 FyAgent 不持有长期 blanket authorization。

### R8 — 双向进程身份验证

- helper 必须只接受满足 FyAgent 正式签名 requirement、Bundle identifier、Team identifier、helper 协议版本和最低安全版本约束的客户端。
- 客户端也必须验证连接到的是预期 helper，而不是只信 Mach service 名称。
- 不得使用 PID 作为身份 authority；PID 只可用于诊断且不得出现在公开 DTO。
- ad-hoc、未签名、错误 Team、错误 identifier、旧于最低安全版本、被替换的 helper 或客户端全部拒绝。
- helper 安装 requirement 与 XPC communication requirement 是两个独立门禁，均必须测试。

### R9 — 单请求事务、原位置更新与可恢复性

- 一个系统提交必须作为一项完整 helper 事务执行，不能由 renderer 或 Rust 连续发出任意文件操作拼接。
- helper 在 `/Applications` 所在卷创建自己命名、自己拥有、可验证的 staging/backup 路径；名称不能来自产品包或 renderer。
- 更新顺序必须覆盖：目标重验证、同卷 staging、staging 重验证、旧目标备份、原子 commit、安装后验证、backup 清理。
- 任一步失败时恢复旧目标；恢复已证明时返回 `rollback_restored`，无法证明时返回 `recovery_required`，绝不返回绿色成功。
- helper 或主应用在 commit window 崩溃后，下一次健康检查/请求必须能够识别本 helper 的未完成事务，并只恢复/清理确属自己的生成路径。
- helper 不得删除身份已经漂移的 replacement、backup 或目标。

### R10 — root 进程最小权限与最小依赖

root helper：

- 不联网，不解析远端 metadata，不读取浏览器/Keychain/TCC 用户数据；
- 不启动 GUI，不显示自己的认证窗口；
- 不执行 shell、`Process`、`system`、`popen` 或任意外部程序；
- 不加载项目无关插件，不提供脚本扩展；
- 不写 `/Applications` 与自己 root-private 状态目录之外的业务文件；
- 使用 bounded input、bounded diagnostics、idle/on-demand 生命周期和结构化日志；
- 日志不输出用户路径、AuthorizationExternalForm、完整签名 requirement、源文件描述符或应用私有内容。

### R11 — helper 生命周期

- 明确表达：missing、bundled newer、compatible、incompatible、disabled/unreachable、tampered、recovery required。
- helper 版本必须单调，并与 FyAgent 发布版本/安全版本策略绑定；禁止自动生成不可复现的源文件版本。
- 新 helper 不得被旧 app 使用；旧 helper 的更新不得允许降级或同版本覆盖。
- helper 更新失败不得破坏已安装的可用 helper；需要用户重新授权时明确提示。
- 提供显式、封闭、经授权的 helper removal；删除 app 前未移除 helper时，残留 helper仍须因客户端 requirement 而不可被其他进程利用。

### R12 — 签名、打包、公证与供应链

- helper 和客户端 bridge 必须构建为项目支持的 universal 架构，并在主 app 之前按 inside-out 顺序签名。
- App 的 `SMPrivilegedExecutables`、helper 的 `SMAuthorizedClients`、嵌入的 info/launchd plist、Mach service label、helper 路径、版本和签名 requirement 必须由单一配置源生成或验证，不能散落手写。
- 开源 Swift 包必须固定 exact tag/commit 和 resolved revision；禁止 floating branch、未审查 binary artifact 或在线下载预编译 helper。
- 许可证、NOTICE、源代码归属和依赖清单进入项目现有合规流程。
- 当前正式 macOS 流水线仍只提交一次最终 DMG 公证；不得为 helper 建立旁路发布或第二份未绑定证据。
- preflight 构建可以证明结构和架构，不能宣称 Developer ID/SMJobBless HIL 已通过。

### R13 — 真实 HIL 才能启用

完成声明必须包含正式签名/公证构建的真实验收：

- macOS 12 与当前支持 macOS；
- Apple Silicon，并对 universal x86_64 slice 完成构建/签名验证；有可用 Intel 机器时执行真实 Intel HIL；
- fresh `/Applications` install；
- existing `/Applications` update；
- 用户取消/错误凭据；
- helper missing、older、newer、tampered、disabled；
- app/helper wrong Team/identifier/版本；
- application running、target drift、source drift；
- verification failure、rollback success、rollback uncertainty；
- helper 在关键 commit 阶段被终止后的恢复；
- helper update 与 explicit removal；
- 重新读取 Agent/Codex inventory 后路径、scope、版本和重复副本检查。

若签名证书、macOS 12 机器或正式公证环境不可用，任务可以完成代码和可移植测试，但不得归档为“系统一键安装已完成”，也不得打开生产 action。

### R14 — 与现有工作解耦

- 实施开始时重新读取届时的 `agent_install`、Codex macOS transaction、产品策略、release 脚本和 V2 action DTO；不以本任务创建时的行号/临时结构为永久事实。
- 只在 helper-facing 合同稳定后接入，避免与当前正在修改的 lifecycle 代码交叉覆盖。
- Windows `fyagent-user-helper` 的 closed-action、安全审计和测试思路可以参考；其普通用户、Windows pipe、PackageBridge 和安装实现不得复制到 macOS root helper。
- Windows 代码、CI 语义和用户 helper 行为保持不变。

## 5. Non-goals

- 通用 root 文件管理器、命令执行器、包管理器或脚本宿主。
- 在 root helper 内下载、检查更新、挂载 DMG、访问产品官网或解析远端 release。
- 把任意本地 `.app` 拖入 FyAgent 后以 root 安装。
- 自动迁移 `~/Applications` 到 `/Applications`。
- 管理 `/System/Applications`、`/Library` 任意内容、LaunchAgents、网络扩展、系统扩展或其他守护进程。
- Windows helper、Windows installer 或跨平台 helper 框架重构。
- 只支持 macOS 13+ 的注册实现，或在本任务同时维护 SMJobBless 与 SMAppService 双生产路径。
- 用 helper 解决第三方应用自身登录、启动、更新源或签名问题。
- 未经 HIL 就默认启用系统目标。
- Agent 目录排序、国产三项“仅安装不更新”、OpenCode/Claude CLI 安装面移除、Claude Desktop 镜像 source，或任何后续 install-policy 任务将改写的产品 UX。
- 为 helper 新建独立 Settings 产品页或第二套 renderer job。

## 6. Acceptance Criteria

### Reuse and architecture

- [ ] helper 安装复用经过审查并固定版本的成熟 Service Management wrapper；没有手写 SMJobBless 状态机。
- [ ] XPC 复用 typed、身份验证明确的成熟实现；没有项目内第二套通用 RPC 编解码器。
- [ ] Authorization Services 使用成熟 wrapper/Apple API；没有密码处理或自制认证 UI。
- [ ] 下载、DMG、应用身份、target authority、job、普通用户事务和 inventory readback 继续由现有 owner 管理。
- [ ] Mist/样例只作为安装、签名、状态和升级参考；其任意路径/命令业务协议没有进入 FyAgent。
- [ ] 没有新增产品私有 helper 或按产品复制事务。

### Closed privilege boundary

- [ ] renderer/Tauri/helper wire 负向扫描确认不存在 path、URL、command、argument vector、destination、copy/delete、hash bypass 和 generic filesystem 字段。
- [ ] 未知产品、目标槽位、operation、协议版本、额外字段和非零 reserved 字段在 root mutation 前被拒绝。
- [ ] 源通过重新验证的 opened capability/FD 交付；路径替换和 TOCTOU 测试不能改变 helper 实际读取对象。
- [ ] 目标只由 helper 的生成策略解析到 `/Applications` 固定直接子项。
- [ ] root helper 无网络、shell、外部进程、TCC/凭据读取和通用文件操作能力。

### Authorization and identity

- [ ] helper install/update、每次系统 app mutation、helper removal 均有与语义匹配的授权门禁。
- [ ] helper 在 mutation 紧邻位置重新验证 AuthorizationExternalForm，并在终态销毁权利。
- [ ] 双向 code-signing requirement 测试覆盖正确/错误 Team、identifier、版本、ad-hoc、tampered 和 PID reuse。
- [ ] helper 安装 requirement 不能被误认为 XPC 通信 requirement；二者均有独立测试。
- [ ] helper 版本升级与 app 最低安全版本防止降级使用。

### Transaction correctness

- [ ] 受支持产品 fresh install 最终只出现在其固定 `/Applications` 槽位，`~/Applications` 不产生副本。
- [ ] 已选择的系统级安装在精确槽位更新；其他候选和 scope 不被修改。
- [ ] commit 前取消/拒绝为零目标写入；commit 后没有虚假的取消能力。
- [ ] staging、backup、target 均进行身份和 file-kind 复核；symlink、replacement drift、错误产品和版本异常被拒绝。
- [ ] post-install readback 失败会恢复并重新验证旧应用，或返回 `recovery_required`。
- [ ] kill/crash 故障注入后仅恢复/清理 helper 自己的生成路径，旧应用不丢失。
- [ ] copy/rename 成功不能单独成为 succeeded；必须通过 helper 本地验证与用户态 fresh inventory。

### Lifecycle, release and UX

- [ ] helper missing/upgrade required/ready/disabled/tampered/recovery-required 状态可被稳定展示和诊断。
- [ ] 用户取消授权、Bless 失败、XPC 拒绝、版本冲突、回滚成功与恢复不确定使用不同 reason code。
- [ ] helper/client/主 app universal 架构、嵌入路径、plist、requirements、inside-out 签名、hardened runtime、公证 ticket 和 sealed resources 均有自动 verifier。
- [ ] 开源依赖使用 exact pin、锁文件、许可证和供应链检查，不从 floating branch 或预编译二进制加载。
- [ ] macOS 12 与当前系统的正式签名 HIL 完成前，系统 action 仍为 `authorization_required`。
- [ ] HIL 通过后才启用受支持产品系统目标；失败时没有静默 `~/Applications` fallback。
- [ ] Windows 测试和行为保持原样。

## 7. Completion gate

本任务只有同时满足以下条件才能归档为完成：

1. 开源依赖和 Apple API 决策记录通过复用、安全、许可证与维护性评审；
2. helper 协议和 root 文件事务通过至少两轮独立安全审查及负向测试；
3. release/signing verifier 在正式构建中证明 nested helper/client/main app 的完整链条；
4. 正式签名、公证的 macOS 12 与当前系统 HIL 证明 fresh install、update、rollback、crash recovery、helper update/removal；
5. Agent/Codex 权威 inventory 重新读取确认系统目标结果；
6. 相关 Trellis spec 已更新，旧的 `authorization_required` 仅在能力不可用时保留，且没有第二套 owner。

任何一个门禁缺失，都只能报告为“实现/测试部分完成，系统 action 仍禁用”，不能宣称 `/Applications` 一键安装已经交付。缺少签名/公证/macOS 12 HIL 时，仍可在系统目标保持禁用的前提下归档本实现任务。
