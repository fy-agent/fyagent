# Technical Design — macOS privileged known-application commit helper

## 1. Decision summary

本设计经过 Apple API、开源组件、现有仓库、威胁模型和发布链评审后，采用以下默认方案：

1. **注册：**首版只使用 `SMJobBless`，通过 Blessed 封装，在 macOS 12 及以上保持一个生产路径。`SMAppService` 作为未来单独迁移任务，不在本任务形成双运行时分支。
2. **传输：**使用 SecureXPC 的 typed route、自动/显式 code-signing requirement 和文件描述符传输；不自行实现 XPC 编解码或 PID 鉴权。
3. **授权：**使用 Authorization Services 与 Authorized，为每次 mutation 创建/请求封闭 custom right；helper 在操作前重新验证并销毁权利。
4. **客户端形态：**XPC 客户端运行在已签名 FyAgent 主进程内，通过一个小型 Swift-to-C bridge 接入 Rust；不增加可被外部直接执行的通用 client sidecar。
5. **源能力：**Rust 打开并重新验证 source app 目录，向 Swift bridge 传递 directory FD；SecureXPC 以 `FileDescriptorForXPC`/等价能力交给 helper。协议不传源路径。
6. **目标 authority：**helper 只接收生成的 product/target-slot 枚举，在自己内部映射到 `/Applications` 固定直接子项。renderer、Rust 和 XPC wire 均不传目标路径。
7. **事务：**已有 Rust macOS transaction 继续拥有下载、mount、产品验证、job 和用户态流程；系统提交抽取为一个 `MacSystemCommitPort`。helper 用单请求执行 root-only staging/backup/commit/verify/rollback/recovery。
8. **策略单一来源：**产品 ID、目标 slot、basename、Bundle ID、版本来源/等价规则由一个 repo-owned closed policy source 生成 Rust 与 Swift 投影；禁止在 helper 手工维护第二张产品表。
9. **启用：**debug/mock 只能验证可移植逻辑。只有 formal Developer ID signed/notarized HIL 通过时，production inventory 才把系统目标投影为 eligible。

## 2. Why this design

### 2.1 Why SMJobBless for the first implementation

Apple 在 macOS 13+ 推荐 `SMAppService`，但项目最低系统是 12.0。`SMAppService` 的 LaunchDaemon 首次注册还需要管理员在 System Settings 中批准，交互和状态机与 `SMJobBless` 不同。首版同时维护两个方案会产生：

- 两套 bundle layout；
- 两套注册/升级/移除状态；
- 两套用户授权体验；
- 两套 HIL 和错误映射；
- 生产 fallback 中难以判断哪个 daemon 是 authority。

因此本任务选择一个保守的兼容路径：在 macOS 12+ 使用 Blessed/SMJobBless。其 deprecated 风险被隔离在 `MacPrivilegedHelperRegistrar` 后面，并记录未来迁移触发器：当 FyAgent 允许把最低系统提高到 13+，或 Apple 宣布实际移除 SMJobBless 时，单独迁移到 SMAppService，不能在本任务内做隐式双栈。

### 2.2 Why not copy Mist or sample helper operations

Mist 和 SwiftAuthorizationSample 证明了 Blessed/SecureXPC、嵌入位置、签名 requirement、helper 状态、更新和卸载能够组成真实产品，但它们的业务命令面适合各自项目/样例：

- Mist helper 接受路径并执行多类通用命令；
- SwiftAuthorizationSample 演示 allowlisted command execution 和 path-based helper update；
- 新的 SMAppService 示例中也常见 root shell command。

这些业务协议对 FyAgent 过宽。FyAgent 只复用安装、授权、XPC、requirement、版本与状态模式，重新定义一个更小的 known-application commit protocol。

### 2.3 Why a main-process Swift bridge

Rust/Tauri 不能直接 `cargo add` Swift Package。可选集成方式包括：

| 方式 | 优点 | 风险 | 决策 |
|---|---|---|---|
| 外部 Swift client CLI | 接入简单 | 额外可执行入口；任意同用户进程可启动；FD/授权生命周期复杂 | 拒绝 |
| Rust 手写 libxpc + Security 校验 | 单语言调用 | 重写 SecureXPC 已解决的 typed routing、audit-token 和签名校验 | 拒绝 |
| Swift client library + C ABI | XPC peer 是正式 FyAgent 主进程；复用 SecureXPC；Rust 边界窄 | 需要 universal library/FFI/signing | 采用 |
| 全 Swift 重写安装协调器 | Swift 生态直接 | 复制 Rust target/job/transaction/product owner | 拒绝 |

客户端 bridge 只暴露 crate-private C ABI，接收固定宽度/封闭字段和已经打开的 FD。它不接受 JSON、URL、文件路径、命令或 renderer payload。

## 3. Proposed repository shape

实施时可以按最新仓库结构调整文件名，但职责必须保持：

```text
src-tauri/
  macos-privileged-helper/
    Package.swift
    Package.resolved
    Sources/
      FyAgentPrivilegedProtocol/     # Swift closed DTO + SecureXPC routes
      FyAgentPrivilegedClientBridge/ # C ABI, Blessed, Authorized, SecureXPC client
      FyAgentPrivilegedHelper/       # root executable, SecureXPC server, transaction
    Resources/
      helper-info.plist
      helper-launchd.plist
    Tests/
  src/
    macos_system_commit/             # Rust MacSystemCommitPort + FFI adapter
  build.rs / release integration
```

这不是新的跨平台 helper 框架。Windows `user-helper` 继续独立；共享的是安全原则和测试思路，不共享平台实现。

## 4. Dependency decision

### 4.1 Selected components

| Component | Selected use | Initial exact candidate | Notes |
|---|---|---|---|
| Blessed | `SMJobBless` authorization/install/error assessment | tag `0.6.0` | HEAD 仅 README 漂移；MIT |
| SecureXPC | typed Mach service, peer validation, FD transfer | audited revision `1cece54562c7626d042f007d2f38cfe325565850` or a newer exact tagged release containing the same fixes | 0.8.0 已有 FD/SMAppService/SMJobBless support；post-0.8.0 commits improve hardened-runtime defaults and executable-path handling；MIT |
| Authorized | custom rights, Authorization Codable/external form | tag `1.0.0` | Blessed transitive dependency；本任务直接使用时显式声明；MIT |
| EmbeddedPropertyList | read embedded helper plists | tag `2.0.2` | Blessed transitive；MIT |
| Required | parse/evaluate signing requirements for diagnostics | tag `0.1.1` | Blessed transitive；MIT |
| SwiftAuthorizationSample | design/reference only | audited commit `85f45622f819ca5b5dcf8867801a6b5d3edf63b2` | 不作为 dependency，不复制 command surface；MIT |
| Mist | production packaging/reference only | audited commit `aed0e49a307d7630a139f8876a9b2651be79f4b8` | active macOS 12+ evidence；不复制业务 routes；MIT |

### 4.2 Pinning gate

实施开始时重新检查上游：

1. 若 SecureXPC 已发布包含已审查 post-0.8.0 fixes 的新 tag，优先 exact tag；
2. 否则固定上述 exact revision，并在 `Package.resolved` 锁定 revision；
3. 禁止 `branch: main`、`from:` 后不提交 resolved lock、floating binary 或 curl 下载源码；
4. 运行 license/provenance/advisory review；
5. 将许可证和版本写入项目既有 NOTICE/third-party inventory；
6. 对 `Blessed` 的 transitive packages 同样检查 resolved revision。

## 5. Ownership and data flow

```text
Renderer
  closed action + opaque inventory/target/revision
        |
        v
Rust Agent/Codex lifecycle owner
  - fresh inventory revalidation
  - release/source/download/mount
  - single app discovery
  - product/version/shape verification
  - job + cancellation + UI projection
        |
        v
MacBundleCommitCoordinator
  - chooses UserFilesystemCommit or MacSystemCommitPort
  - opens source directory capability
  - requests operation-specific Authorization right
        |
        v
Swift client bridge in FyAgent process
  - helper install/update/status
  - mutual SecureXPC requirement
  - transfers FD + Authorization + closed request
        |
        v
Root helper
  - authenticate client and Authorization right
  - resolve product/target slot internally
  - root-only staging/backup/atomic commit/verify/rollback
        |
        v
Rust fresh inventory readback
  - exact path/scope/product/version/no duplicate
```

成功只有在最后的用户态权威 readback 完成后才能发布。

## 6. Closed contracts

Exact names may adapt, but wire shape and forbidden fields are stable.

### 6.1 Helper status

```text
HelperStatusRequest { protocolVersion }

HelperStatusReply {
  protocolVersion,
  helperVersion,
  minimumClientVersion,
  state: ready | update_required | incompatible | recovery_required,
  activeRecoveryReceipt?
}
```

不返回 installed path、requirement 字符串、PID 或本地用户名。

### 6.2 System commit request

```text
KnownApplicationCommitRequest {
  protocolVersion,
  operationId,              // canonical UUID
  issuedAtMonotonicClass,   // bounded freshness token, not wall-clock authority alone
  action: fresh_install | update_existing,
  product: codex_desktop | opencode_desktop |
           qoderwork | trae_work | workbuddy,
  targetSlot: closed generated enum,
  expectedTargetRevision,
  expectedSourceRevision,
  sourceDirectory: FileDescriptorForXPC,
  authorization: Authorization,
  reserved: all zero
}
```

`targetSlot` 例如区分 Codex 的允许 basename 迁移形态，但每个值都由统一产品策略生成并在 helper 内解析；它不是路径。

### 6.3 Result

```text
KnownApplicationCommitResult {
  protocolVersion,
  operationId,
  stage,
  outcome: committed | rollback_restored | recovery_required,
  installedIdentityRevision?,
  reasonCode?,
  receiptId?
}
```

诊断字符串限长、脱敏，可选；路径永不跨 wire。

### 6.4 Helper removal

```text
RemoveHelperRequest {
  protocolVersion,
  operationId,
  authorization,
  reserved: zero
}
```

只允许 helper 解除自己的 launchd registration、删除自己的固定 helper/plist 和自己创建的 root-private recovery records。不能接受任何目标参数。

## 7. Authorization model

### 7.1 Rights

建议至少两个独立 right：

```text
com.fyagent.desktop.system-application.commit
com.fyagent.desktop.privileged-helper.remove
```

rollback 是 commit 事务内部的补偿步骤，不另行要求用户授权；否则失败窗口无法自动恢复。

### 7.2 Flow

1. App 确保 right definition 使用系统 `authenticate-admin` 规则，并提供本地化说明。
2. 紧邻 helper call 前创建新 Authorization session，请求对应 right，允许系统交互。
3. App 通过 SecureXPC 传输 Authorization external form；不把 bytes 暴露给 Rust log/renderer。
4. Helper 重新构造 Authorization，使用相同 right 且不允许 helper-side UI，再次检查。
5. Helper 执行或拒绝 mutation。
6. Helper/bridge 在终态调用 destroy-rights，并丢弃 request。

Authorization Services 控制认证方法和 credential cache。FyAgent 的保证是“一次 mutation 一次 fresh request/recheck，不保存 blanket capability”，不是“每次强制出现密码输入框”。

## 8. Process identity model

### 8.1 Installation requirements

- App `Info.plist` 中只有一个 helper label 的 `SMPrivilegedExecutables` entry。
- Helper embedded info 中 `SMAuthorizedClients` requirement 必须包含：
  - `anchor apple generic`；
  - FyAgent app signing identifier `com.fyagent.desktop`；
  - Team identifier `HY446996QX`；
  - minimum safe app `CFBundleVersion`。
- App 对 helper 的 requirement 包含 helper identifier、Team ID 和期望版本策略。
- requirement 从 signed fixture/单一配置生成并由测试解析；不在多个 plist/script 手工漂移。

### 8.2 Runtime requirements

- SecureXPC server 使用与 `SMAuthorizedClients` 同等或更严格的 client requirement。
- Client 显式要求 helper Team ID、identifier、hardened runtime 和 protocol/version compatibility。
- 不使用 `xpc_connection_get_pid` / `NSXPCConnection.processIdentifier` 作为 authority。
- helper 检查 `getppid() == 1` 可作为运行形态 sanity check，但不能替代签名校验。

### 8.3 Downgrade policy

- Helper `CFBundleVersion` 与 formal app release version 单调绑定；helper 变更必须伴随 FyAgent version bump。
- 不使用样例的 source-hash auto-increment，避免构建修改工作区和不可复现版本。
- 新 helper 的 `SMAuthorizedClients` minimum version 阻止旧 app 连接。
- 更新 helper 时，当前 app 必须同时满足 installed 与 bundled helper requirements。
- equal/lower helper 不覆盖；开发期同版本测试通过显式 removal/独立 development version，不放宽 production downgrade rule。

## 9. Product policy single source

产品策略需要为 Rust 与 Swift 生成两个 projection：

```text
KnownSystemApplicationPolicy {
  productId,
  allowedTargetSlots [{ id, basename, freshDefault }],
  expectedBundleId,
  versionSource,
  versionEquivalence,
  allowedActions
}
```

要求：

- 源文件位于后端权威层，不进入 renderer；
- 生成器输出 deterministic Rust/Swift fixtures，并有 drift test；
- helper 不接收这些字符串作为 request；
- implementation 开始时先对当前任务已经形成的 product registry 做 reuse audit，优先让它成为/生成该 source，而不是新建并行表；
- ChatGPT/Codex 允许 basename 只能来自已有 application-identity contract，禁止 name matching。

## 10. Source capability and TOCTOU defense

### 10.1 Client preparation

Rust 在用户态：

1. 从 downloader-owned artifact capability 开始；
2. 只读 mount DMG；
3. 找到唯一 direct `.app`；
4. structured plist/product-specific reader 验证身份、版本、可执行 shape；
5. 将 source app 准备为受控只读目录；
6. 使用 `open(..., O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC)`/等价安全 API 获取 FD；
7. `fstat`、稳定 file identity、source revision 与 product policy 绑定；
8. bridge duplicate FD 后再进入 XPC；字符串 path 不进入 request。

### 10.2 Helper revalidation

Helper：

- 确认接收对象是 directory FD；
- 通过 `fstat`/`openat`/`fstatat` 只沿固定相对组件读取 `Contents/Info.plist`、版本文件和 executable；
- 拒绝 symlink、目录逃逸、特殊文件、hard-link 异常、身份漂移、源/策略不匹配；
- mutation 前再检查 source revision；
- FD 生命周期覆盖整个 copy，客户端路径删除/替换不改变已打开对象。

### 10.3 Copy implementation gate

优先评估 Apple 原生 `copyfile`/clone 文件能力能否在 directory-FD/no-follow 约束下递归复制 `.app` 并保留必要 metadata。若不能证明安全：

- 采用最小的 fd-relative `openat` recursion，只支持普通目录、普通文件和受审查 symlink policy；
- 不发展成通用 copier；
- 对 bundle 实际文件类型、ACL/xattr/resource fork、sparse/clone、错误恢复建立 fixture/HIL；
- 不调用 `ditto`、shell 或外部程序作为 root fallback。

该 gate 必须在实现阶段留下 ADR 和测试证据，不能凭 API 名称假定满足。

## 11. Root transaction

### 11.1 Generated paths

Helper 只在 `/Applications` 目标父目录创建：

```text
.fyagent-system-stage-<operation-uuid>.app
.fyagent-system-backup-<operation-uuid>.backup
```

并在 root-private 固定目录保存一个最小 transaction receipt：

```text
/Library/Application Support/FyAgent/SystemCommit/v1/<operation-uuid>.receipt
```

receipt 只含封闭枚举、target slot、阶段、生成名和 identity revisions；不含任意路径、用户路径或 authorization bytes。目录 root:wheel、最小权限、no symlink。

### 11.2 Sequence

```text
validate client + authorization
validate request + source FD + target revision
create/fsync receipt: preparing
copy source FD -> generated same-volume stage
reinspect stage via fixed product policy
fsync stage + parent
receipt: ready_to_commit

if update:
  revalidate exact target
  rename target -> generated backup
  receipt: backup_created

rename stage -> fixed target
receipt: replacement_committed
verify target identity/version/source equivalence

if verification fails:
  remove only exact expected replacement
  restore exact expected backup
  reverify restored target

on success:
  remove backup
  remove receipt
  fsync parent
return committed
```

Fresh install failure removes only an exact transaction-owned replacement。任何目标/backup identity drift 都停止并保留 recovery receipt。

### 11.3 Crash recovery

Helper 启动/每次请求前检查 bounded receipt directory：

- `preparing` / `ready_to_commit`：清理确属 receipt 的 stage；目标不动；
- `backup_created`：若 target absent，恢复已验证 backup；
- `replacement_committed`：验证 target；成功则清 backup，失败则尝试恢复；
- 多 receipt、未知版本、路径/identity drift：进入 `recovery_required`，拒绝新 commit；
- 恢复逻辑只能操作 generated names + fixed target slot。

故障注入必须覆盖每个 receipt phase。

## 12. Helper lifecycle

### 12.1 Install/update

- 调用 system commit 前先查询 bundled/installed helper compatibility。
- missing 或 bundled newer：向用户说明并通过 Blessed 请求 admin authorization；取消不启动 app commit。
- installed newer：拒绝 downgrade，要求更新 FyAgent。
- installed incompatible/tampered：不自动覆盖，显示 recovery/removal guidance。
- helper update 成功后重新连接并验证 version/protocol/signature。

### 12.2 Health

状态不能只看 `/Library/PrivilegedHelperTools/<label>` 是否存在。至少结合：

- bundled helper metadata/signature；
- installed helper static code/signature/version；
- authenticated XPC health reply；
- recovery receipt state；
- launchd connection可达性。

不要求 renderer 知道物理路径。

### 12.3 Removal

- Settings/诊断中提供显式动作；
- 需要独立 admin right；
- helper 只解除自己的固定 job、删除自己的固定 binary/plist/receipt directory；
- 先拒绝 active transaction；
- 返回后客户端确认连接失效、固定 artifacts 消失；
- app 已被直接拖入废纸篓时 helper 可能残留，但 requirement 使其他客户端无法使用；文档说明显式移除流程。

## 13. Rust/Swift boundary

Rust facade：

```text
MacSystemCommitPort
  helper_status() -> HelperStatus
  ensure_helper_ready(user_intent) -> HelperReady
  commit_known_application(AuthorizedSystemCommit) -> SystemCommitOutcome
  remove_helper(user_intent) -> RemoveOutcome
```

`AuthorizedSystemCommit` 由 Rust 内部构造，不能 `Serialize` 到 IPC。建议使用固定 C ABI struct：

- version + size；
- closed numeric enums；
- UUID bytes；
- revision bytes；
- source FD；
- reserved zero；
- callback/owned result buffer。

Swift bridge 负责 Authorization 对象与 SecureXPC。Rust 不解析 AuthorizationExternalForm，也不把它写入日志。

## 14. Job and UI integration

沿用现有 Agent/Codex job，不创建 helper 专用 renderer job，也不做 Agent 目录/Claude 安装 UX（后续 install-policy 任务）。

Job stage 复用现有 closed 集合：`checking`、`awaiting_user`、`staging`、`installing`、`verifying_installation`、`cancelled`、`failed`、`incomplete`。Helper 特有失败使用新的 reason code，而不是再加一套 stage。

UI/文案最小集：

- 系统目标在正式 HIL 前继续 `authorization_required`，不自动选择用户目录；
- 用户取消 helper/commit 授权显示 cancelled/对应 helper reason，不改装目录；
- `rollback_restored` 告知新版本未生效、旧版本已恢复；
- `recovery_required` 禁止继续重试；
- debug/unsigned build 不得伪装成签名 HIL。
- helper 显式卸载是 backend closed route，不在本任务做新 Settings 产品页。

## 15. Signing and bundle integration

### 15.1 Bundle layout

```text
FyAgent.app/
  Contents/
    MacOS/fyagent
    Frameworks/<Swift client bridge dylib/framework>
    Library/LaunchServices/com.fyagent.desktop.system-commit-helper
    Info.plist  # SMPrivilegedExecutables
```

SMJobBless helper是 command-line executable，embedded info/launchd plist通过 linker section/受审查构建步骤写入。

### 15.2 Build

- Swift package/toolchain由项目正式 macOS runner的 Xcode/Swift 版本构建；
- helper/client分别构建 arm64 + x86_64 并合并/产出 universal；
- deterministic script 验证 Package.resolved、symbols、architecture、embedded plists；
- Tauri app build 后、formal app signing 前将 nested artifacts 放入最终 bundle；
- preflight 可 ad-hoc/unsigned structure-check，但不能运行 SMJobBless acceptance。

### 15.3 Inside-out signing

formal pipeline：

1. 签 client framework/dylib；
2. 签 helper executable（hardened runtime、timestamp、正确 identifier）；
3. 验证 helper embedded info/launchd plist、CFBundleVersion、requirements、universal slices；
4. 签主 `FyAgent.app`；
5. `codesign --verify --deep --strict`，并做显式 nested checks；
6. 构建现有 styled DMG；
7. 按现有流程只提交该 DMG 公证并 staple DMG/app；
8. 最终 mount 后重新验证 nested helper/client/main app signatures。

现有 `sign-app` 只签主 app，不足以证明 helper；本任务必须扩展而不是建立旁路签名脚本。

## 16. Rollout

### Phase states

```text
compiled_only
  -> portable_tests_passed
  -> signed_dev_hil_passed
  -> formal_notarized_hil_passed
  -> system_destination_enabled
```

feature enablement 由 backend capability 决定：

- helper未打包/未正式签名/协议不兼容/HIL gate未启用：system destination仍 disabled；
- helper ready + formal capability enabled：system destination eligible；
- 运行时 helper失败：返回明确 reason，不 fallback。

回滚版本可以关闭 capability，使 system target重新变为 manual/authorization_required；不会自动删除 helper或迁移应用。

## 17. Compatibility and migration

- macOS 12：Blessed/SMJobBless正式路径。
- macOS 13+：本任务仍走同一路径，避免双状态机；记录 deprecated telemetry/日志，不调用 SMAppService fallback。
- future SMAppService migration：新任务替换 `MacPrivilegedHelperRegistrar` 与 bundle layout，保持 `MacSystemCommitPort`、协议业务语义和测试向量；迁移完成后删除 SMJobBless，不并存。
- Windows：无变化。

## 18. Failure mapping

至少需要以下 closed reasons，名称可按现有 enum 收敛：

```text
helper_not_packaged
helper_signature_invalid
helper_install_authorization_cancelled
helper_install_failed
helper_update_required
helper_downgrade_rejected
helper_protocol_incompatible
helper_peer_rejected
operation_authorization_cancelled
operation_authorization_invalid
source_capability_invalid
source_changed
target_slot_invalid
target_changed
application_running
permission_denied
commit_failed
rollback_restored
recovery_required
helper_removal_failed
```

BlessError、SecureXPC error、Authorization OSStatus 和 POSIX errors在 Swift/Rust adapter 边界映射；raw requirement、path、auth bytes、Mach port和完整 OS diagnostic不进入 IPC。

## 19. Validation strategy

详见 `implement.md`。设计级硬门禁：

- dependency/provenance review；
- protocol exact-key/forbidden-field tests；
- generated product policy drift tests；
- source FD TOCTOU tests；
- fake filesystem transaction/recovery tests；
- XPC identity and Authorization tests；
- release bundle/signature/verifier tests；
- signed/notarized HIL。

Portable tests不能替代 SMJobBless、Authorization dialog、launchd root daemon、Developer ID requirement、公证或真实 `/Applications` 原子替换。
