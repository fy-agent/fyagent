# Technical Design — macOS Agent install/update experience

## 1. Design goals

本设计只解决 macOS 上已经验证的生命周期缺口，并把实现收敛到现有 owner。核心原则：

1. **复用现有 owner，而不是复制代码。** Codex 继续拥有 dedicated product port，但其通用下载、临时文件、DMG 事务与进度能力可以下沉为 crate-private shared core。
2. **只扩展现有 inventory。** OpenCode Desktop 加入现有 managed desktop registry；不新增全盘扫描器、renderer registry 或第二个应用目录。
3. **一个操作只有一个 owner。** 下载、bundle metadata、DMG transaction、launch、system commit、Grok distribution 和前端 speed projection 各自只有一个权威边界。
4. **失败保持原状态。** 权限、来源、版本、目标或 post-install readback 无法证明时 fail closed；不得静默写入 `~/Applications`、切换 Grok 分发模式或打开应用。
5. **真实可观测性优先。** 只有真实 byte telemetry 才显示百分比/速度；外部 installer 没有 byte 协议时展示阶段、耗时与安全日志摘要。

## 2. Final ownership map

| Concern | Current problem | Final owner |
| --- | --- | --- |
| Desktop artifact transport | Codex streaming 与 generic Agent `Vec<u8>` 两套实现 | `desktop_artifact` shared core，Codex/managed Agent 委托 |
| DMG preparation/replace/rollback | Agent 先内存下载、再写第二份 DMG | 现有 managed exact DMG transaction，直接接收 shared artifact |
| Desktop discovery | 现有 known roots scanner 缺 OpenCode policy | 现有 Agent installation inventory + managed desktop registry |
| Bundle metadata | generic Agent 手写 XML parser | Codex 已有 bounded `plutil -> JSON -> typed fields` owner 下沉复用 |
| Desktop launch | install/update 可隐式 launch；macOS 用命令行 `open` | 现有 `platform::process_launch` owner，macOS 内部改用 `NSWorkspace` completion adapter |
| OpenCode state | 一个产品被当作 CLI-only | 一个 catalog product，`cli`/`desktop` 两个 closed surface |
| `/Applications` commit | 仅返回 authorization required | 一个 `MacSystemCommitPort`；Apple 原生授权优先，必要时 reviewed helper |
| Grok lifecycle | native/installer/npm 被串成模糊 fallback，终态易丢失 | 现有 Tooling owner + distribution-bound plan/job |
| Download UX | Agent 无 bytes，多个 formatter | raw transfer snapshot + 一个 shared TypeScript projector |

## 3. Shared desktop artifact core

### 3.1 Boundary

从 Codex download implementation 中抽取产品无关能力，或保留底层模块并暴露窄接口。产品 source resolver 仍在各自 domain，shared core 不解析 renderer URL，也不决定产品 fallback。

```text
ArtifactRequest
  product_policy_id
  release_capability
  ordered backend-owned endpoints
  redirect/host policy
  artifact format and architecture class
  max size / timeout / retry policy

ArtifactProgress
  attempt / max_attempts
  completed_bytes
  total_bytes?
  sequence
  observed_at

DownloadedArtifact
  protected job-local file
  observed byte count
  source category
  revalidate()
```

### 3.2 Preserved properties

必须保留 Codex 已有：

- `reqwest` streaming；
- bounded redirect 与 host allowlist；
- `.part` → finalized file；
- cancellation 与 retry；
- size cap；
- protected temp directory/file revalidation；
- known-only cleanup；
- progress throttling；
- release/session binding。

禁止为了通用化降低 Codex metadata pinning、release identity、single-flight、post-install readback 或取消语义。

### 3.3 Migration

- generic Agent 不再返回完整 DMG `Vec<u8>`；
- 不再把同一 artifact 写成第二份临时 DMG；
- shared artifact 直接交给现有 managed DMG transaction；
- QoderWork、TRAE Work、WorkBuddy、OpenCode Desktop 与 Codex 共享 transport/temp primitives，但 Codex 仍保留 dedicated product port。

## 4. Managed desktop inventory and bundle metadata

### 4.1 Discovery scope

本任务不扩大扫描面。继续使用现有权威 roots：

- `/Applications`
- `~/Applications`

只枚举普通、非 symlink、直接子级 `.app`。原因：本次 OpenCode 漏检已确认是 product policy 缺失，而非 Launch Services 或路径问题；新增全盘/外置卷扫描会增加误识别、性能和 target-authority 风险。

### 4.2 One registry, multiple product policies

`DESKTOP_PRODUCTS`/等价 registry 增加 OpenCode Desktop policy：

```text
catalog product: opencode
surface: desktop
bundle id: ai.opencode.desktop
canonical basename: OpenCode.app
supported platform: macOS
source policy: official OpenCode desktop release
```

目录名只用于展示与 fresh target basename，不作为身份依据。现有 QoderWork、TRAE Work、WorkBuddy policy 保持一个 registry owner。

### 4.3 Shared structured plist reader

将 Codex 已有 bounded `plutil` 读取逻辑抽取成 crate-private macOS bundle metadata owner：

```text
read_bundle_metadata(path)
  -> bundle_identifier
  -> short_version?
  -> bundle_version?
  -> executable_name/path shape
```

要求：

- 支持 binary/XML plist；
- 输出大小有界；
- typed parser 拒绝缺失/错误类型；
- path canonicalization、regular/no-symlink、bundle/executable containment 复用现有事务规则；
- 删除 generic Agent 手写 `<key>/<string>` 扫描。

不引入第二个 plist crate，也不在生产代码中调用 `defaults`、`PlistBuddy` 或 Spotlight CLI。

### 4.4 Candidate semantics

候选继续由 inventory 生成 opaque target + revision：

- 0：未安装；
- 1：可直接绑定；
- >1：要求用户选择；
- 执行动作前重新枚举和验证；
- target 消失、移动、scope/owner/identity 变化返回 stale/changed reason，不猜选另一个候选。

## 5. Explicit desktop launch

### 5.1 Product behavior

- 安装、更新、检查、equal-or-newer no-op 和 post-install refresh 均不得 launch；
- desktop surface 有唯一/已选择的 `launch_eligible` candidate 时展示 **“打开软件”**；
- CLI surface 不展示该动作；
- Codex equal-or-newer 分支返回 `AlreadyCurrent`/等价 readback，不调用 launch。

### 5.2 One launch owner

保留 `platform::process_launch` 作为业务 owner，在其 macOS implementation 内把命令行 `open` 替换为 `NSWorkspace.openApplication(... completionHandler:)` 或当前 SDK 对应的 typed application API。

边界：

- renderer 只发送 backend-issued candidate capability；
- backend launch 前重新验证 candidate/revision；
- native adapter 接收验证后的 path，不接受 renderer path、Bundle ID、arguments 或 environment；
- completion error 映射到稳定 reason code；
- diagnostic 记录 OS error category，不记录完整用户路径；
- Tauri opener 继续负责普通 URL/目录，不承担 application lifecycle。

这不是第二个 launcher，而是现有 owner 的 macOS 实现替换。

## 6. Product surface contract

### 6.1 Domain model

保留七个 top-level Agent catalog ID，引入 closed surface：

```text
AgentSurface = Cli | Desktop
SurfaceKey = (AgentCatalogId, AgentSurface)
```

建议的 versioned readiness/job shape：

```ts
interface AgentProductReadiness {
  agentId: AgentCatalogId;
  surfaces: AgentSurfaceReadiness[];
}

interface AgentSurfaceReadiness {
  surface: "cli" | "desktop";
  localVersion?: string;
  latestVersion?: string;
  installations: InstallationSummary[];
  selectedInstallationKey?: string;
  distributionOwner?: "native_internal" | "official_npm";
  allowedActions: AgentAction[];
  reasonCodes: AgentReasonCode[];
  sourceStatus?: SourceStatus;
}
```

严格 parser 拒绝未知 surface、非法产品/surface 组合和额外危险字段。renderer 不提交 URL、path、command、scope、bundle ID、distribution override 或 validation bypass。

### 6.2 OpenCode

- `cli`：继续由 Tooling domain 观察和执行；
- `desktop`：使用 managed desktop registry、official DMG source、shared artifact core、existing DMG transaction、explicit launch；
- 两个 surface 的状态、action、job key、error 独立；
- UI 在同一产品区域展示两个 section，不复制产品配置/Provider 入口。

### 6.3 Codex

- 保持 dedicated desktop installer port；
- 只下沉通用 artifact/temp/progress/DMG primitives；
- no-op 与 install success 不隐式 launch；
- explicit launch 文案统一为“打开软件”。

## 7. OpenCode official source policy

### 7.1 Verified inputs

官方 download 页面目前提供独立 macOS Apple Silicon/Intel DMG；已验证稳定入口：

- `https://opencode.ai/download/stable/darwin-aarch64-dmg`
- `https://opencode.ai/download/stable/darwin-x64-dmg`

本机已安装 bundle 为 `/Applications/OpenCode.app`，Bundle ID `ai.opencode.desktop`。这些是设计证据，版本常量不得硬编码。

### 7.2 Resolver

优先使用官方 repository/release metadata 冻结 release identity 与 exact asset；稳定入口只在能够与 frozen release 安全绑定、或作为官方 HIL 入口时使用。resolver 必须：

- 固定 official metadata/asset/redirect hosts；
- 按 macOS + arm64/x64 + DMG 唯一匹配；
- missing/ambiguous asset fail closed；
- 不自动切换到 GitHub proxy、匿名 CDN 或 Homebrew command；
- 安装后以本地 bundle identity/version/executable readback 闭环。

遵循现有 executable-installer non-admission spec：不新增远端 digest/size/Team ID/signature 比较作为下载内容 admission gate。

## 8. System `/Applications` commit

**本任务不实现。** 2026-08-31 用户确认写入系统 Applications 需要 helper，改由后续独立 Trellis 任务交付。本节保留为后续任务的设计输入。本任务生产行为：`MacSystemApplications` 保持 `authorization_required`；无 helper、无 sudo、无 AppleScript admin。

### 8.1 Single port

### 8.1 Single port

```text
MacSystemCommitPort
  preflight(capability)
  commit(operation_capability)
  rollback(operation_capability)
  status()
```

Desktop coordinator、inventory 与 UI 只依赖该 port，不知道最终是 Apple native authorization 还是 helper。生产构建最终只能启用一个 adapter。

### 8.2 Gate A — Apple native authorization first

签名/公证 prototype 验证：

1. `com.apple.developer.security.privileged-file-operations` entitlement 是否能为实际 Developer ID 获批和保留；
2. `NSWorkspace.requestAuthorization` + authorized `FileManager` 能否在目标不存在时完成 fresh create；
3. 能否 exact replace 既有 `.app` 并保持 rollback；
4. cancel/deny/expired authorization 是否保持旧目标和 staging；
5. Rust/Objective-C bridge 是否能保持 closed capability boundary；
6. macOS 12 与当前系统真实 HIL。

若六项全部通过，采用 native adapter，不引入 helper。

### 8.3 Gate B — Reviewed helper only when required

若 Gate A 无法覆盖 fresh create 或事务不变量，采用一个 helper adapter：

- 因项目最低 macOS 12，优先评估 `Blessed` 的 SMJobBless 管理；
- XPC 复用 `SecureXPC`；
- `SwiftAuthorizationSample` 提供签名/版本/防降级参考；
- `Mist` 提供真实 macOS 12+ 应用集成和发布参考；
- 不复制这些项目的业务操作，只复用 helper install/XPC/signing primitives。

Helper contract：

```text
query_helper_status()
commit_known_application(operation_id, revision)
rollback_known_application(operation_id, revision)
```

约束：

- renderer 无法创建 operation；
- operation ID 解析到 backend 生成、短期、单次使用的 protected manifest/staging；
- closed product enum 决定固定 `/Applications/<basename>.app`；
- helper 验证 caller/server code-signing requirements、版本、防降级、重放、containment、no-symlink、source/target revision；
- helper 不联网、不解析 remote metadata、不接受 shell/command/URL/任意 destination/任意 copy-delete；
- 日志与 reply 脱敏；
- 失败调用 existing transaction rollback/recovery semantics。

若 Gate A/B 均失败，system target 保持 disabled/manual。不得用 `sudo`、AppleScript、AuthorizationExecuteWithPrivileges 或静默 user-scope fallback。

### 8.4 Destination rules

- fresh automatic target：`/Applications/<Product>.app`；
- existing `/Applications` candidate：exact-location update；
- existing `~/Applications` candidate：exact-location update，不迁移；
- no silent scope fallback；
- multi-candidate 必须选择；
- commit 前重验证 revision/running state；
- install success 只在 post-install inventory readback 后发布。

## 9. Grok Build distribution-aware lifecycle

### 9.1 Owners

```text
GrokDistributionOwner
  NativeInternal
  OfficialNpm
```

Inventory 通过锚定 executable、symlink/layout 和非敏感 config 观察 owner。现有本机是 `NativeInternal`，更新失败必须保持该 owner。

### 9.2 Native/internal plan

- latest/check：复用锚定 executable 的官方 `grok update --check`，或现有 Tooling 中与 official stable channel 对齐的 resolver；
- update：调用锚定 executable 的 `grok update --version <frozen-version>`；
- fresh install：复用 xAI official installer；
- native updater/installer 自身拥有 x.ai primary、official GCS fallback、architecture/Rosetta、artifact/compression、internal layout、symlink/config/PATH 与 executable self-check；
- FyAgent 不复制这些算法，只提供 typed fixed action、受控 environment、timeout/cancel、stdout/stderr capture、terminal job 与 post-observation。

禁止 `grok update || installer || npm install` 这类 shell-composed chain。native 内部的 x.ai → GCS 是同一 owner 的官方 transport fallback，不是分发切换。

### 9.3 Official npm plan

- 首次安装时作为用户显式选择的官方替代路径；
- 使用现有 anchored package-manager adapter 与 `@xai-official/grok@<frozen-version>`；
- latest 来自当前配置 registry 的官方 package metadata；
- npm-owned update 只走 npm owner；
- 不调用 native updater、不重写 `~/.grok` internal layout；
- package manager executable、prefix、权限和 post-install command resolution 必须被 preflight/回读。

Native 失败后 UI 可以提供“改用官方 npm 方式”的新动作，但原 job 先进入失败终态，并要求用户明确选择；不得自动迁移 owner。

### 9.4 China network behavior

- Native：x.ai primary → xAI-declared GCS fallback，尊重 `HTTPS_PROXY`/`HTTP_PROXY`/`NO_PROXY`；
- Npm：复用用户/企业 npm registry 配置；
- 不存在已验证的独立大陆官方镜像；
- 不硬编码 GitHub proxy、匿名镜像或把 credential 发送给 artifact mirror；
- HIL 分别记录 source/owner，不把“最终安装成功”当成来源证据。

### 9.5 Job and progress

Grok action 必须进入持久 job：

```text
checking -> preflight -> executing -> verifying -> terminal
```

保存 bounded/redacted output、exit status、timeout、owner、source category、attempt 和 post-readback。官方 updater/installer 没有稳定 byte protocol时使用 indeterminate progress，不解析 stderr 猜百分比/速度。

## 10. Transfer telemetry and frontend projection

### 10.1 Backend snapshot

通用 action job versioned extension：

```text
transfer.phase
transfer.completedBytes
transfer.totalBytes?
transfer.attempt
transfer.maxAttempts
transfer.sequence
transfer.observedAt
```

约束：

- completed 在同一 attempt 单调不减；
- total 为正且不得小于 completed；
- retry 通过 attempt 变化显式 reset；
- unknown total → percent null；
- terminal snapshot 保留最终 byte count，但不保留陈旧 speed；
- old wire version 通过明确 compatibility projection 或 version bump 处理。

### 10.2 Shared TypeScript projector

从 Codex `snapshots.ts` 提取：

```text
projectTransfer(previous, snapshot)
  percentLabel?     // exactly one decimal at most
  transferredLabel
  speedLabel?
  indeterminate
  freshness
```

Codex 与 Agent UI 共用同一算法。速度用真实 byte delta + monotonic time；阶段变化、retry、长时间无样本、terminal 时 reset/hide。页面不得直接输出 raw float 或复制 `formatBytes`/speed state。

## 11. State, concurrency and errors

### 11.1 Single-flight

- 每个 `(product, surface)` 同时一个写 job；
- 同 surface 写 job 运行时 launch disabled；
- duplicate request 返回现有 job；
- plan/inventory revision 在执行前重新验证；
- commit point 前可取消，目标开始变更后不可取消；
- app restart 不恢复半完成下载，temp/part/staging known-only cleanup。

### 11.2 Stable reason categories

名称可按现有 enum 调整，语义至少覆盖：

- `surface_not_supported`
- `application_selection_required`
- `bundle_identity_mismatch`
- `target_changed`
- `application_running`
- `application_launch_failed`
- `system_install_authorization_required`
- `system_install_authorization_denied`
- `system_commit_unavailable`
- `system_commit_peer_rejected`
- `official_source_unreachable`
- `official_fallback_unreachable`
- `distribution_owner_mismatch`
- `external_installer_failed`
- `external_installer_timed_out`
- `post_install_not_observed`
- `rollback_restored`
- `recovery_required`

UI 只消费 reason projection；日志记录 product/surface/stage/owner/source/exit category，禁止 token、npm credential、URL query、完整 home/temp path 和任意 shell text。

## 12. Security boundaries

- Renderer：只能选择 backend `allowedActions`、surface、release capability、完整 opaque target triplet；
- Product policy：唯一拥有 official hosts、Bundle ID、asset classifier、fixed destination；
- Downloader：只执行 constrained request；
- Inventory：唯一拥有 target normalization/revision；
- System commit：只消费 coordinator capability；
- Grok executor：固定 action/argument shape，不接受 renderer command/env；
- Launch：只消费 verified candidate。

Required negative tests：

- IPC 注入 URL/path/Bundle ID/command/env/scope/owner 被 strict parser 拒绝；
- malicious redirect 越界被拒绝；
- same name/wrong Bundle ID、binary plist、symlink app、target drift；
- authorization deny 不写 user scope；
- helper route（若存在）拒绝 replay/path escape/wrong signer；
- native Grok failure 不调用 npm；npm owner 不调用 native updater；
- logs 不含秘密/完整路径；
- no invented percent/speed。

## 13. Deletion and convergence map

完成后删除或停止成为 owner：

- generic full-memory DMG `Vec<u8>` path；
- second full artifact temp write；
- hand-written plist XML parser；
- command-line `open` implementation（业务 launch owner 保留）；
- OpenCode CLI-only readiness assumptions；
- Codex equal/newer implicit launch；
- shell-composed Grok multi-owner fallback；
- generic job transient terminal clearing；
- page-local percent/bytes/speed formatter copies；
- 未被选中的 system-commit prototype/adapters。

Windows-only code通过 platform boundary 保留，不借本任务重构。

## 14. Verification architecture

### Automated

- Rust: source policy、bundle parser、candidate/revision、surface contract、progress monotonicity、Grok owner plan/error mapping；
- shared downloader: retry、redirect、cancel、size cap、unknown length、cleanup、Codex regression；
- DMG transaction: fresh/update exact target、rollback/recovery；
- process launch adapter: success/failure/stale target；
- system commit: capability validation、deny/cancel、adapter selection、helper peer/replay tests when applicable；
- frontend: OpenCode surfaces、exact “打开软件”、no implicit launch、one-decimal percent、speed freshness、terminal persistence；
- architecture scans: duplicate downloader/parser/launcher/speed owner、arbitrary IPC、auto Grok owner switch。

### Signed HIL

- Apple Silicon；Intel device or trusted HIL runner；
- macOS 12 + current supported macOS；
- OpenCode `/Applications` discovery/version/explicit launch；
- Codex install/update/no-op without auto launch；
- five desktop products fresh `/Applications` install、system/user exact update、cancel、rollback；
- selected system-commit adapter signed/notarized behavior；
- Grok native x.ai/GCS、explicit npm、proxy/mainland network、failure preserves owner；
- download percent/speed/cancel/unknown Content-Length；
- multiple candidate and target drift。

Evidence appended to `research/acceptance-evidence.md`; design assumptions cannot close a gate.

## 15. Rollout and rollback

- New surface/job contract uses version bump or rigorously tested compatibility projection；
- system `/Applications` action remains disabled until the selected adapter passes signed HIL；
- Grok owner-aware job can be feature-gated during HIL, but old auto cross-owner fallback must not remain as hidden production fallback；
- failure rolls back to disabled/manual official action, not unsafe shell, unknown mirror or user-scope success；
- before merge remove dead prototypes, update owning specs and run full contract checks。
