# 技术设计：统一桌面 Agent 安装面、Windows 适配与 Codex 日志治理

## 1. 设计原则

1. **主进程做策略，普通用户 helper 做用户态观察与执行。** 管理员权限不是绕过用户上下文问题的工具。
2. **闭集能力，不传命令。** IPC 表达产品语义，不表达 shell 语义。
3. **成功必须回读。** 安装器运行结束不是安装成功；远端存在新版本不是本地已更新。
4. **一个能力一个 owner。** 下载、MSIX、EXE、inventory、用户会话、前端投影均扩展现有 owner。
5. **对不确定性 fail-closed。** 未采到真实签名、package identity、安装位置或用户上下文时，不把猜测写进 allowlist。
6. **日志是诊断接口。** 预期暂态不应制造告警风暴，真实错误也不能被整体静默。
7. **安装 owner 必须唯一。** macOS/Windows 的 non-Grok Settings/Tooling install/update/manual-command surface 必须退场；有明确消费者的只读发现/配置能力才保留。

## 2. 现有架构与复用 owner

| 能力 | 现有 owner | 本任务处理 |
| --- | --- | --- |
| Agent 产品/surface/action policy | `agent_install/lifecycle_policy` | 复用，不创建 Windows 特供矩阵 |
| 冻结 Explorer 用户 SID/session/profile/PATH | `windows_runtime/`、`platform/windows/interactive_user.rs` | 原样复用；只补调用接口与测试 |
| 普通用户进程启动 | 现有 Explorer COM/process launch | 复用，不新增 token 降权方案 |
| helper 协议与 runtime | `src-tauri/user-helper/` | 扩展闭集 action/result，不新增 sidecar |
| helper 认证与回传 | job + nonce + one-shot named pipe + action binding | 保持不变量，新增动作走同一握手 |
| 下载、临时目录、取消、进度 | 现有 Agent/Codex fetch、job、`JobTempDir` | 复用，不再建 HTTP client/downloader |
| MSIX/AppX 验证与部署 | Codex Desktop Windows PackageManager/bridge/helper | Claude 若选择 MSIX，仅抽取或窄委托产品无关能力 |
| 厂商 EXE 验证与执行 | `agent_install/windows.rs` + `agent-exe-install` helper | OpenCode/Claude EXE 扩闭集产品枚举 |
| Desktop inventory | `agent_install/windows.rs` + `desktop.rs` | 补产品 descriptor 与 package adapter，不做全盘扫描 |
| Grok CLI owner/版本逻辑 | `services/tooling/grok.rs` 与现有 discovery | 抽取可共享纯规则，用户态执行落入 helper |
| 前端 Agent 状态 | 现有 Agent DTO/ports/components | 只扩 reason/action projection |
| Codex usage 同步 | `services/session_usage_codex.rs` | 类型化 pending reason、调度状态与日志预算 |

任何新增模块都必须说明为什么不能由上述 owner 完成。默认不新增 runtime 依赖。

## 3. 目标调用链

```text
UI Agent action
  -> thin Tauri command
  -> Agent lifecycle policy / selected stable target
  -> elevated coordinator
       |- Desktop: resolve source -> existing protected download -> verify package
       |- Grok: construct closed semantic helper request
  -> existing Explorer-user helper (one-shot, authenticated, bounded)
       |- verified package install OR fixed Grok observe/install/update
       |- no arbitrary command surface
  -> structured redacted result
  -> authoritative inventory/version rediscovery under frozen user
  -> readiness/action DTO refresh
```

任何步骤无法证明目标用户、产品 identity、selected target 或执行结果时，链路停止并返回闭集 reason code。

### 3.1 Stable lifecycle matrix

```text
qoderwork   -> desktop -> install, launch
trae-work   -> desktop -> install, launch
workbuddy   -> desktop -> install, launch
grokbuild   -> cli     -> install, update
codex       -> desktop -> install, update, launch
claude-code -> desktop -> install, update, launch
opencode    -> desktop -> install, update, launch
```

平台 capability 可以让 action 暂时不可用，但不能新增 policy 未允许的 action。稳定配置 ID 与物理安装组件分离：`claude-code` 可继续作为配置 ID，组件显示 `Claude Desktop`。

### 3.2 Legacy Tooling convergence

```text
Tooling lifecycle
  -> Grok Build observe/install/update only
  -> other CLI tools: read-only discovery/configuration only when consumed
  -> no non-Grok install/update/manual command bundle

Agent lifecycle
  -> all six Desktop product lifecycle actions
```

前端删除 non-Grok 按钮、复制命令和远程脚本文案；后端 stale action 在 side effect 前拒绝。不能把旧 CLI 请求转发到 Desktop installer，也不能为了保留旧列表虚构 Desktop 产品。

## 4. 前置决策门

### G0：真实失败复现与基线冻结

在改代码前，使用与发布一致的 Windows x64 正式安装包复现：

- 每个产品的未安装、已安装、旧版本、用户级/机器级、多安装状态。
- 正式 `requireAdministrator` 与开发 `asInvoker` 的差异。
- 用户提供的 Codex deferred 日志场景，记录每轮数量、文件稳定性与父子时间线状态。

证据只保存产品/package identity、签名主体摘要、注册表键类别、scope、版本、退出分类与脱敏日志；不提交安装包、用户路径、rollout ID 或账户信息。

### G1：Claude Windows 分发与更新 owner

官方同时提供用户安装器与 x64/arm64 MSIX，但 scope 和功能语义不同。真实 Windows 评审必须记录：

- 用户安装器是否适配现有交互式 helper、安装 scope、完整 Claude/Cowork 功能、原地更新与可观察退出。
- MSIX 的 package family/name/publisher/architecture/application ID、per-user 注册与 machine provisioning 差异、Cowork 服务注册与自动更新行为。
- FyAgent 是只负责首次安装并把更新交给厂商，还是可以安全做显式更新；必须只有一个 update owner。

选择顺序：

1. 不损失用户可见功能；
2. 能复用现有 owner；
3. 能由官方 package identity、签名与 post-readback 证明；
4. 不需要静默修改系统功能或自动重启；
5. 两条路径都不能满足时，保持 manual/official-page fallback，而不是实现半可靠安装。

### G2：OpenCode Windows 身份与架构冻结

对当前 first-party release 逐架构捕获：

- stable endpoint 与 redirect host、最终格式、文件名与 architecture；
- Authenticode signer/certificate chain；
- ProductName/FileDescription/version、Uninstall/App Paths、默认/可选目录与 scope；
- 安装、更新、取消、并排安装与 updater 行为；
- GitHub latest stable version与下载入口是否一致。

上游 workflow 的 ARM64 构建只是候选证据。当前 release 资产、签名/PE identity、安装行为与 Windows ARM64 原生 HIL 任一缺失时，ARM64 继续 fail-closed。

### G3：Grok Windows distribution owner

评审官方 native 与 official npm 两种 owner：

- 新装默认 owner 必须明确，不能因本机碰巧存在 npm 就静默切换。
- 已安装 owner 通过候选来源与固定版本探针识别，更新保持 owner。
- host、固定 package/script identity、固定参数、超时与版本解析均为代码闭集。
- 不使用通用 WinGet 或通用 PowerShell/npm bridge。

### G4：既有 Windows Desktop 链路 HIL

QoderWork、TRAE Work、WorkBuddy、Codex 必须先跑现有实现，按真实失败做最小修复。不得因为任务范围大而重写已经存在的 source、inventory、download 或 helper；QoderWork/TRAE Work/WorkBuddy 的 `update=false` 始终保持，不由 HIL 重新开放。

### G5：Codex 对新 ChatGPT 桌面应用的精确身份兼容

OpenAI 当前官方迁移说明表明，新 ChatGPT 桌面应用包含 Codex，旧版还可能以 ChatGPT Classic 并存。实现前必须采集：

- 新 ChatGPT 干净安装的 package name/publisher/family/version/application ID/AUMID；
- 旧 Codex 通过官方正常更新后的 identity 变化；
- 新 ChatGPT 与 ChatGPT Classic 并存时的两个精确身份、启动 target 与 update owner；
- 当前 `codex_desktop` owner 的 inventory/deployment/launch 是否已经自然兼容。

只有 HIL 证明当前 exact identity 已变化且现有 owner 无法识别时，才允许加入小型、first-party、闭集的 migration set。禁止显示名、窗口标题、进程名或 contains 匹配；无法唯一判定时保持歧义/不可操作状态。

## 5. helper 协议设计

### 5.1 闭集请求

沿用现有 protocol version、job id、nonce、pipe、Hello/Started/Progress/Success/Error 顺序与帧预算。新增能力表达为语义枚举，例如：

```text
HelperAction
  - CodexMsixInstall                         // existing
  - AgentExeInstall { product }              // existing + closed products
  - AgentPackageInstall { product, kind }    // only if shared MSIX extraction is justified
  - ToolOperation {
      tool: GrokBuild,
      action: Observe | Install | Update,
      expected_owner: None | Native | Npm
    }
```

这是合同示意，不强制具体 Rust 类型名。禁止字段：任意 command、args、URL、script、cwd、path、environment、shell type、redirect target。

### 5.2 结构化响应

```text
ToolOperationResult {
  detected: bool,
  normalized_version: Option<BoundedVersion>,
  owner: None | Native | Npm,
  outcome: Observed | Installed | Updated | NoChange,
  reason: Option<ClosedReasonCode>
}
```

- 不回传绝对路径、原始 stdout/stderr、完整命令行或安装脚本正文。
- helper 内可捕获限长输出用于解析；解析完成后丢弃，不进入 IPC 或生产日志。
- 输出、帧数、消息数、执行时长与子进程树均有硬上限。

### 5.3 身份与通道不变量

- 父进程使用冻结的 Explorer 用户启动 helper。
- helper 验证父进程、受保护 artifact/package bridge、pipe owner 与 action binding。
- 父进程验证 helper PID、SID/session 与 handshake nonce。
- 每个 job 只连接一次；未知 action、额外参数、错误顺序、重放或超长帧均拒绝。
- 保持全局与产品级 in-flight 限制，避免两个安装器或两个 owner 同时修改同一产品。
- helper 失败时不回退到管理员上下文执行。

## 6. Desktop 产品实现

### 6.1 Source resolver

- 每个产品只使用固定官方 metadata/stable endpoint 与 host allowlist。
- 远端 metadata 只能提供 schema 允许的版本或 asset 证据；远端任意 URL、文件名、命令或 hash 字段不能直接成为执行权限。
- 下载继续使用现有 redirect allowlist、流式大小上限、取消与受保护 job 目录。
- OpenCode x64 增加经核验的 Windows Desktop stable alias；ARM64 alias 只在当前 release 与原生 HIL同时证明后加入。
- Claude 根据 G1 选择的官方 owner 建闭集 descriptor，不从只含 macOS 的镜像 schema 猜 Windows 分支。

### 6.2 Artifact 与 package 验证

- EXE/NSIS：现有 PE 解析、architecture、ProductName/ProductVersion、WinVerifyTrust 与 HIL 冻结 signer/product identity。
- MSIX：现有 manifest、publisher、package identity、architecture、签名与受保护 bridge 验证。
- 验证前后都重新检查 artifact identity，防止下载后替换。
- “来自官方域名”不能替代本地签名与 identity 验证。

### 6.3 Inventory

- 组合 current interactive-user PackageManager、shell-user/machine Uninstall 32/64 views、App Paths 32/64 views、受限 known paths 与文件/package identity。
- 所有 user-scoped adapter 使用冻结的 Explorer 用户，而不是当前管理员 profile。
- adapter access denied、部分 hive 不可用或 package enumeration 失败必须保留为 incomplete evidence。
- 候选按 canonical file/package identity 去重，但保留并行安装和 scope 差异。
- AppExecutionAlias、PATH、配置目录只产生 candidate，不直接产生 trusted target。

### 6.4 安装与更新

- Fresh install 前冻结完整 baseline inventory。
- Update 前冻结用户选择的 stable target、scope、owner、version 与 file/package identity。
- helper 完成后重新建立完整 inventory：
  - Fresh install：必须出现唯一符合预期 scope/identity 的新 target；
  - Update：原 target 的版本或经过 G5 审查的 exact identity 必须发生可信变化；
  - 多目标、新目标与旧目标并排、target 消失、scope 漂移或 inventory incomplete 均失败。
- installer exit code 只用于错误分类，不是成功证明。
- 不自动删除用户原有安装；多安装由 selection/conflict 状态处理。

### 6.5 启动

- EXE 通过现有 trusted process launch 在冻结 Explorer 用户中启动。
- MSIX 通过精确 package/application identity 启动；Codex/ChatGPT/Classic 按 G5 冻结的 exact AUMID 选择。
- 启动前再次确认 selected target 仍属于当前 inventory；变化则返回 `TargetChanged`。
- 不从 PATH、窗口标题、进程显示名猜测目标。

## 7. Grok 普通用户发现与生命周期

### 7.1 规则复用

现有非正式 Windows tooling 已包含 PATH/profile/npm/Volta/pnpm/Scoop/nvm 等候选逻辑。实现必须选择单一代码 owner：

1. 把纯 candidate、owner 判定、version normalization 抽到现有 helper crate 的共享无 runtime 模块；或
2. 在现有 backend owner 下建立纯数据模块供主 crate/helper 共用。

禁止主进程与 helper 复制两套候选表。抽取不得改变 macOS/Linux 和其他业务所需的只读发现/配置行为；除 Grok 外的 Tooling lifecycle action 与公开安装命令必须移除。

### 7.2 Observe

- helper 在冻结用户 profile/PATH 与有限 known directories 中枚举闭集 candidate。
- 对 candidate 执行固定 `--version` 探针，隐藏窗口、限时、限输出；只接受可规范化版本。
- PATH/alias 只提供候选，最终以可执行身份与成功探针为准。
- 多 owner/多版本冲突返回选择或不可操作状态，不任意取第一个。

### 7.3 Install/Update

- Native owner 只运行代码内固定的官方流程；Npm owner 只运行固定 package name 与固定 action。
- 执行前验证 owner 所需宿主确实属于冻结用户且探针可用。
- 完成后重新 observe；只有 owner 符合预期且版本达到可信变化才成功。
- 网络、宿主缺失、超时、输出异常等映射为闭集 reason，不回传原始输出。

## 8. Codex deferred 状态与日志预算

### 8.1 类型化状态

把当前字符串原因拆成至少以下闭集语义：

- `MissingParent`：父文件尚未发现，可恢复。
- `ParentTimelineNotCaughtUp`：父文件存在但最大事件时间早于 child fork，是否继续高频重试由父文件稳定性决定。
- `StableForkGap`：父子 evidence 长期未变化且仍不满足，稳定挂起等待 fingerprint 变化。
- `MalformedTimeline` / `InvariantViolation`：不可恢复或需要人工关注。
- `IoFailure` / `DatabaseFailure`：基础设施错误。

具体类型名可调整，但禁止通过本地化 message 文本反向分类。

### 8.2 Retry 与 diagnostic state 分离

fingerprint 至少绑定规范化文件 identity、reason、父/子 size + mtime 或等价内容水位；不得包含用户可识别路径。

- retry scheduling state 与 last emitted diagnostic state 分开保存。
- `MissingParent` 与仍增长的 `ParentTimelineNotCaughtUp` 保持有界重试，但不逐文件告警。
- 父子都稳定后进入稳定挂起或指数退避，避免每分钟重复全量工作。
- 任一相关 fingerprint 变化时立即允许重新评估。
- 进程重启后可以重新评估，但不能恢复 N 条逐文件 WARN 风暴。

### 8.3 日志策略

- 预期 deferred：默认无生产日志；调试模式每轮最多一条 `DEBUG` 聚合 `{missing_parent, catching_up, stable_gap, total}`。
- 真正异常：`WARN`/`ERROR`，同 fingerprint 去重；reason 或 evidence 变化后可再次输出。
- 所有日志只显示计数、闭集 reason 与最多一个不可逆短 fingerprint；不显示路径、rollout ID、文件名或内容。
- 扩展现有 sync summary 作为唯一聚合 owner，不在 `mark_deferred` 内逐文件 `warn!`。

### 8.4 正确性不变量

- deferred child cursor 不推进。
- parent 更新即使 child stamp 不变也能触发必要重试。
- replay prefix 与 child suffix 保持去重。
- parent 补齐后 usage 全部恢复且只插入一次。
- rebuild 与 incremental sync 总量一致。
- 日志抑制不改变 parse、cursor、transaction 或 usage 语义。

## 9. 前端与 reason code

- 后端继续返回结构化 inventory/readiness/action，前端不维护第二套产品 allowlist。
- 新 reason 只表达用户可行动差异，例如 `InteractiveUserUnavailable`、`HelperUnavailable`、`TargetSelectionRequired`、`PrerequisiteRequired`、`PlatformUnsupported`、`InstallationVerificationFailed`。
- Claude 系统前置条件只检测并给出 reason/official-page action，不自动修改 Windows 功能。
- action 完成后统一刷新 inventory，不做乐观版本更新。
- 诊断只显示脱敏 code/fingerprint，不显示绝对路径、installer stderr、package family、SID 或 session 标识。

## 10. 测试设计

### 10.1 纯逻辑与合同测试

- lifecycle surface/action matrix 与 stale request fail-closed。
- Source parser：平台、architecture、host、schema、redirect、version、恶意字段。
- Inventory：0/1/多 target、user/machine scope、损坏注册表、错误产品名、伪造文件、alias-only、partial adapter failure。
- Deployment readback：fresh、same-target update、reviewed exact migration、side-by-side、no-change、incomplete baseline。
- Protocol：action binding、nonce、order、frame limit、extra args、replay、unknown product/owner。
- Grok：candidate priority、owner preservation、version parse、host missing、timeout、output limit。
- Logging：120 轮重复状态、parent catch-up、stable gap、fingerprint change、restart first pass、true corruption dedup。

### 10.2 Native CI

复用现有 Windows x64/ARM64 native jobs，不新建平行 workflow。必要时只扩充现有 change classifier，使相关 owner 变更触发已有 job。

### 10.3 HIL

至少在 Windows 11 x64 正式安装包执行：

- clean standard-user Explorer + FyAgent administrator process。
- administrator interactive session。
- 每个产品 absent、installed、install、launch；允许 update 的产品再做旧版到新版。
- UAC cancel、offline、wrong signature/package identity、helper kill、installer timeout。
- 用户目录含空格与非 ASCII、alias disabled、empty PATH。
- 0/1/多安装、user/machine scope 并存、stale registry/remnant。
- 新 ChatGPT clean install、旧 Codex upgrade、ChatGPT Classic coexistence。
- Codex deferred 数据集重复 120 轮与 parent catch-up recovery。

ARM64 声明必须有当前 native artifact 与真实 Windows ARM64 host HIL；cross-compile、emulation、workflow matrix 只作为辅助证据。

## 11. 被拒绝方案

| 方案 | 拒绝原因 |
| --- | --- |
| 在管理员主进程直接运行用户 CLI | 用户 profile/PATH/owner 错误，扩大权限与供应链风险 |
| 新建通用 `run-command` helper | 把受限 sidecar 变成任意代码执行器，破坏当前安全模型 |
| 为每个产品复制 downloader/installer | 重复基础设施，行为与安全检查必然漂移 |
| 所有产品统一改用 WinGet | package owner、可用性、scope、输出与 updater 语义不统一，形成第二套 lifecycle |
| 解析 WinGet/PowerShell/npm 人类可读输出 | 本地化且不是稳定机器合同，不能成为 inventory 权威 |
| 强制 Claude 使用 per-user MSIX | 可能牺牲 Cowork 服务注册或功能，必须由 G1 证据决定 |
| 自动开启 Virtual Machine Platform 或重启 | 超出一键安装授权边界，系统影响过大 |
| 只把 Codex `WARN` 改为 `DEBUG` | 只能遮噪，不能解决重复解析、每轮 INFO 与永久 pending |
| 全局静默 session usage 日志 | 会吞掉真实解析、I/O、数据库和一致性错误 |
| 恢复 Claude/OpenCode Agent CLI surface | 与当前 Agent 产品 policy 冲突，制造重复 surface |
| 保留重复的 non-Grok Settings/Tooling installer | 与统一 desktop-only policy 冲突，形成第二套安装 owner；应移除 lifecycle action，同时保留有消费者的只读配置能力 |
| 用 `ChatGPT`/`Codex` 显示名或进程名宽松兼容 | 会把 ChatGPT Classic、历史 Codex 或伪造进程误认为目标；必须使用 exact package/AUMID |
