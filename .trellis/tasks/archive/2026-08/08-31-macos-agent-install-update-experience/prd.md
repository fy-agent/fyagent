# 完善 macOS Agent 安装、更新、发现与启动体验

## 0. 任务状态

- 状态：实现与 owning spec 已对齐（job v3 `transfer`、OpenCode `surfaces`、系统 `/Applications` 提交延期）；准备归档
- 平台范围：仅 macOS；Windows 的安装、发现、Helper、IPC 与界面行为不在本任务中调整。
- 最低系统版本：维持项目现有的 macOS 12.0。
- **范围裁剪（2026-08-31，用户确认）**：写入系统 `/Applications`（privileged helper / native authorization / system-commit adapter）**不在本任务实现**。后续由同一分支上的独立 Trellis 任务处理；本任务不得预建 helper、不得用 sudo/AppleScript 顶替，也不得把装到 `~/Applications` 宣称成系统安装成功。
- 本任务归档不依赖 helper 任务。

## 1. 目标

在**不新增第二套下载器、DMG 安装事务、应用扫描器、启动器或任意提权执行器**的前提下，统一修复 FyAgent 在 macOS 上的以下问题：

1. Codex Desktop 安装/更新完成或“已是最新”时不再隐式启动；所有桌面应用由用户显式点击 **“打开软件”** 启动。
2. OpenCode 保持一个产品身份，但分别展示 CLI 与 Desktop 两个独立生命周期；已安装的 OpenCode Desktop 能被扫描，并具备一键安装、更新和启动能力。
3. QoderWork、TRAE Work、WorkBuddy、Codex Desktop、OpenCode Desktop 的 **用户目录** 一键安装/原位置更新可用；**系统 `/Applications` 写入**留给后续 helper 任务。已安装在 `/Applications` 的应用仍须能被发现并「打开软件」。
4. Grok Build 保留并识别 native/internal 与官方 npm 两种分发 owner：native 复用 xAI updater/installer 及其官方 GCS 备用源，npm 复用现有包管理 owner；首次安装由用户明确选择，更新不得自动跨 owner。
5. 通用 Agent 下载恢复真实字节进度和下载速度；百分比统一最多保留一位小数。
6. 将当前重复或较弱的实现收敛到既有 Codex 下载、bundle 解析、DMG 替换、回滚与进程启动 owner。系统目录 privileged commit **不在本任务落地**。

## 2. 名称澄清

- 用户口述的“OpenCore”按项目上下文、本机应用和官方产品资料确认是 **OpenCode**，本任务不涉及 OpenCore Legacy Patcher。
- 用户口述的“Grow Build”按项目目录与现有产品 ID 确认是 **Grok Build**。
- 用户看到的 Codex 红色警告没有保存原始文本、错误码或日志，因此本任务不猜测具体根因；只处理已经由代码证明存在的隐式启动副作用，并完善可复现、可诊断的显式启动路径。

## 3. 已确认的仓库事实

1. `agent_install` 的 macOS 桌面产品注册表当前只包含 QoderWork、TRAE Work、WorkBuddy；OpenCode 只进入 CLI Tooling 路径。
2. 现有 macOS 桌面扫描已经浅层扫描 `/Applications` 与 `~/Applications`，并以 bundle identifier 确认产品身份；OpenCode 未被发现的直接原因是没有桌面产品策略，而不是必须引入新的全盘扫描器。
3. 通用桌面扫描当前手写 XML `Info.plist` 字符串查找；Codex owner 已经通过系统 `plutil` 做有界、结构化读取，并兼容 binary/XML plist。
4. 现有桌面启动 owner 已能对 inventory 绑定的候选执行 macOS `open`；缺口是产品接线、显式按钮和结果诊断。实现应在既有 `process_launch` owner 内用 Apple 原生 application-open adapter 替换命令行细节，而不是新建第二套 launcher。
5. `DesktopDeploymentTarget::Fresh(MacUserApplications)` 当前默认写入 `~/Applications`；`MacSystemApplications` 只返回 `authorization_required`，没有可执行授权 adapter。
6. 通用 Agent job snapshot 没有下载字节、总量或速度字段；前端 hook 固定返回 `percent: null`。
7. 通用 macOS Agent 下载会把完整 DMG 收入内存；Codex 已有受控流式下载、`.part`、取消、重试、进度、受保护临时目录和可靠 DMG 替换/回滚事务。
8. Codex Desktop 的 equal-or-newer 分支当前会直接调用 launch，并以 `succeed_after_launch` 结束作业。
9. Grok Build 已有成熟的 executable discovery、安装形态识别、锚定更新、PATH 修复、官方 installer 与 npm 执行基础；当前问题在于多路径被串成自动 fallback 且终态不可见。本任务保留 Tooling owner，拆除自动跨 owner 切换并补齐持久作业、来源与版本回读。
10. 项目当前只有 Developer ID + hardened runtime + notarization 流水线，没有 privileged-file-operations entitlement，也没有 macOS privileged helper。

详细证据见：

- `research/current-implementation-audit.md`
- `research/repository-gap-analysis.md`
- `research/upstream-distribution-and-macos.md`
- `research/macos-authorization-options.md`
- `research/reuse-options.md`
- `research/source-and-network-strategy.md`
- `research/spec-constraints.md`

## 4. 稳定产品决策

### D1 — 复用优先是硬性验收规则

实现选择顺序固定为：

1. 复用 FyAgent 现有权威 owner；
2. 复用项目已采用的系统能力、框架或依赖；
3. 采用维护状态、许可证和安全边界经过审查的成熟开源组件；
4. 仅在前三项存在明确能力缺口时增加最薄 adapter；
5. 自研完整实现是最后选择，必须在任务 research 中记录候选、拒绝原因和新增维护成本。

以下行为直接判定为任务未完成：

- 新建第二套 HTTP 下载、重试、重定向、临时文件、取消或速度算法；
- 新建第二套 `hdiutil + copy + replace + rollback` DMG 事务；
- 新建 Launch Services 全局扫描器来解决已知根目录中的 OpenCode；
- 新建通用 root copy、任意 shell、任意 URL、任意路径或任意 bundle ID IPC；
- 硬编码未经审查的 GitHub 代理、匿名 CDN 或第三方二进制镜像；
- 仅在页面调用 `toFixed(1)`，而不补齐真实进度数据链路。

### D2 — OpenCode 是一个产品、两个安装面

OpenCode 继续使用唯一的 `AgentCatalogId::OpenCode`、品牌、配置、Provider 和 catalog 排序，但拥有两个独立安装面：

- `cli`：现有命令行工具的发现、版本、安装与更新；
- `desktop`：macOS `.app` 的发现、版本、安装、更新与启动。

两个安装面必须有独立状态、动作、错误和作业键；任一安装面已安装不得推导另一安装面已安装。不得创建第二个 OpenCode 产品 ID、复制配置入口或复制 Provider 能力。

### D3 — 本任务只交付用户目录安装；系统 Applications 写入延期

适用桌面产品：QoderWork、TRAE Work、WorkBuddy、Codex Desktop、OpenCode Desktop。

- 本任务首次一键安装保持现有可执行目标：`~/Applications/<Product>.app`（`MacUserApplications`）。
- `MacSystemApplications` 保持 `authorization_required` / 不可执行；UI 不得把它画成已可用的一键系统安装。
- 已经位于 `~/Applications` 的安装仍原位置更新，不自动迁移到 `/Applications`。
- 已经位于 `/Applications` 的安装：**发现、版本、打开软件**在本任务内完成；**原位置替换/回写**留给 helper 任务（当前 executor 对系统目标返回授权不足即可）。
- 两处同时存在时必须由 inventory target capability 显式选择，不按扫描顺序猜测。
- 不得把用户目录安装成功显示成「已安装到系统应用程序文件夹」。

### D4 — 安装/更新与启动解耦

- 安装、更新、检查更新、版本相等 no-op 和安装成功后的刷新均不自动启动第三方应用。
- 有唯一或已选择的可信桌面候选时显示 **“打开软件”**。
- 启动只接受 backend 发出的 opaque candidate/target，不接受 renderer 路径、bundle ID、命令或参数。
- CLI 安装面不显示“打开软件”。

### D5 — Grok Build 按分发 owner 复用上游语义

- `native/internal` 与 `official_npm` 是两个独立、官方支持的分发 owner；inventory 必须先识别 owner，再决定可用动作。
- native fresh install 复用 xAI 官方 installer；native update 优先复用已安装且锚定的 `grok update --check/--version`，不得复制架构识别、channel、artifact 命名、目录布局、symlink、配置写入和版本自检算法。
- native 路径中的 `x.ai` 主源与 `storage.googleapis.com` 备用源由 xAI 官方 updater/installer 管理；FyAgent 只负责受控执行、终态、来源类别和安装后回读。
- 官方 npm 包 `@xai-official/grok` 复用现有包管理 owner 和用户/企业 npm registry；它是首次安装时的显式替代选项，或经用户明确批准的迁移目标，不是 native 失败后的自动 fallback。
- 本次调研没有发现独立的“中国大陆官方镜像”，因此任务不承诺或内置不存在的官方镜像。

### D6 — `/Applications` system-commit / helper **本任务不实现**

- G1 证据见 `research/g1-authorization-spike.md`：当前 Developer ID 包无 privileged-file-operations；SDK 授权 FileManager 不能 fresh-create 缺失的 `.app`。
- 用户确认：系统根目录 Applications 需要 helper，本任务允许不实现。后续独立任务再做 Blessed/SecureXPC 或其它封闭 helper。
- 本任务保持系统目标 disabled/manual；禁止 sudo、AppleScript admin、通用 root XPC、静默 `~/Applications` 冒充系统成功。
- 下列设计（Gate A 原生授权、Gate B helper 协议）仅作后续任务输入，不是本任务交付物。

<details>
<summary>后续 helper 任务的原 D6 约束（本任务不执行）</summary>

- 先用签名/公证原型验证 Apple `NSWorkspace.requestAuthorization`、`com.apple.developer.security.privileged-file-operations` 与 authorized `FileManager` 是否能同时完成“目标不存在时首次提交”和“既有 bundle 原位置替换/回滚”。
- 原生授权若覆盖全部事务，直接作为唯一 system-commit adapter，不增加 helper。
- 若原生 API 无法完成 fresh create 或无法维持事务不变量，则在同一 port 下采用一个封闭 helper；项目最低系统仍为 macOS 12，因此首选复用 Blessed（SMJobBless）+ SecureXPC，并以 SwiftAuthorizationSample 与 Mist 的签名/打包实践作为参考。不得同时保留两套生产提交路径。
- helper 只负责 `/Applications` 中已知产品事务的最终提交、回滚和必要清理；下载、远端元数据、产品选择、页面输入和任意命令不进入 root 进程。
- helper 与客户端必须通过 code-signing requirement 相互认证，并具有版本/防降级约束；request 只能引用 backend 生成的短期 operation capability，不接受 renderer 原始路径、URL、命令或任意产品字符串。
- 若原生授权与成熟 helper 两条路都无法在 Developer ID/notarized 包中安全落地，系统目标保持不可用并明确记录阻塞；不得降级为 `sudo`、AppleScript、静默用户目录安装或自研通用 XPC。

</details>

## 5. 功能需求

### R1 — 平台与兼容性

- 仅修改 macOS 路径；Windows 行为、wire 和测试结果保持兼容。
- 维持 macOS 12.0 最低版本。
- 任何新增 Swift/Rust/macOS API 都要在 macOS 12 和当前受支持 macOS 上完成可用性审查。
- 原生授权结论必须来自签名 Developer ID app 的 HIL；开发态可执行文件或纯 mock 不能被描述为授权已经可用。

### R2 — 安装面合同

- 引入封闭的 `cli | desktop` 安装面概念，命名可在实现期微调，但语义必须稳定。
- 安装面参与 readiness、inventory、action request、job snapshot、query key 与 UI projection；页面不得通过产品名称猜测。
- 只有一个安装面的产品保持紧凑展示；OpenCode 在同一产品区域展示两个独立 section。
- strict TypeScript/Rust parser 必须拒绝未知安装面、非法产品/安装面组合和多余危险字段。
- renderer 不得提交 URL、文件路径、命令、安装 scope、bundle identifier 或验证绕过字段。

### R3 — OpenCode Desktop 发现

- 将 OpenCode Desktop 加入现有 managed desktop registry，而不是新建扫描器。
- 官方本地身份固定为 bundle identifier `ai.opencode.desktop`；目录名只用于展示/目标 basename，不作为身份。
- 继续扫描 `/Applications` 与 `~/Applications` 的普通、非 symlink、直接子级 `.app` 候选。
- 将通用扫描的 plist 读取收敛到 Codex 已有 `plutil -> bounded JSON -> typed fields` owner，支持 binary/XML plist，删除手写 XML 字符串查找。
- 候选需要记录路径、scope、版本、candidate revision、launch/update capability。
- 0/1/多候选、读取失败、身份不匹配和候选漂移必须分别表达；不得退回应用名匹配。
- 自定义目录、嵌套目录和外置卷扫描不在本任务范围；以后需要时另建需求，不借本问题扩大扫描面。

### R4 — OpenCode Desktop 官方来源与生命周期

- Apple Silicon 使用 OpenCode 官方 macOS Apple Silicon DMG；Intel 使用官方 macOS Intel DMG。
- 版本和 release identity 动态解析，不把调研时版本写入常量。
- 复用 managed desktop source descriptor、共享流式 downloader 和现有 macOS DMG transaction。
- 本任务 fresh install 目标为 `~/Applications/OpenCode.app`（或产品 policy 确认的用户目录 basename）。系统 `/Applications/OpenCode.app` 的写入留给 helper 任务。
- 用户目录 update 绑定既有 candidate，严格保留其路径与 scope。系统目录 candidate 只发现/启动，不在本任务内替换。
- 安装后必须重新扫描并证明目标路径、scope、bundle ID、版本形状和 launch eligibility，才能进入 succeeded。

### R5 — 系统目录安装与原位置更新（本任务延期）

本条整体交给后续 helper 任务。本任务只要求：系统目标保持 `authorization_required`；用户目录 fresh/update 仍走现有同卷事务；不得用用户目录成功冒充系统安装。

原需求原文保留供后续任务：

<details>
<summary>R5 原文（本任务不验收）</summary>

- fresh system install 前显示“将安装到系统的应用程序文件夹，需要管理员允许”。
- 用户取消授权、system-commit adapter 不可用或验证失败时（helper 路径还包括安装、版本、XPC peer 失败）：
  - 旧安装不变；
  - 不创建 `~/Applications` 副本；
  - job 进入明确终态；
  - mount、download part、staging 按事务所有权清理。
- update system candidate 时请求/使用受控系统提交能力并原位置替换。
- update user candidate 时不调用 privileged system-commit adapter，继续使用现有同卷事务原位置替换。
- commit 前重新验证 candidate revision、目标身份、运行状态和事务路径；进入目标变更后取消按钮不可用。
- post-install readback 失败时执行既有 rollback；无法证明恢复时进入 recovery-required，不显示绿色成功。

</details>

### R6 — system-commit adapter 边界（本任务延期）

本条整体交给后续 helper 任务。本任务不实现 `MacSystemCommitPort` 生产 adapter。

<details>
<summary>R6 原文（本任务不验收）</summary>

- Phase 0 先完成 Apple 原生授权 signed spike；原型必须证明 entitlement provisioning、fresh create、exact replacement、rollback、取消/拒绝和 macOS 12/current HIL。
- 若原生能力完整，adapter 只接受 coordinator 生成的 capability-bound operation，不暴露通用 FileManager。
- 若原生能力不足，才进入 helper spike：嵌入 helper、bless/install、版本查询、可信 XPC、一个无害封闭请求、升级/拒绝旧客户端和 notarized app HIL；原型通过后才接入应用提交。
- helper 公共协议只能表达封闭操作，例如：`commit_known_application(operation_id, revision)` 与 `query_helper_status`。
- `operation_id` 只索引 backend 建立的短期 protected manifest；renderer 无法创建或读取 manifest。
- system-commit adapter 禁止接受：
  - renderer/插件提供的 path、URL、bundle ID、destination 或 command；
  - shell、可执行文件和参数；
  - 任意 copy/move/delete；
  - hash/signature/identity bypass；
  - 任意产品字符串。
- adapter 重新验证 operation 时效/重放、closed product enum、目标恰位于 `/Applications`、固定 basename、regular/no-symlink/containment、source identity、target revision 和事务生成路径；helper 路径还必须验证双向 code signing。
- privileged 进程（若存在）不联网、不解析远端元数据、不读取 Downloads/Desktop 等 TCC 目录，不运行 GUI。
- adapter 返回封闭结果和脱敏诊断，不返回任意 stdout/stderr 或用户绝对路径。

</details>

### R7 — 共享下载与 DMG 事务

- 从 Codex 下载 owner 中抽取或通过窄 adapter 复用：受控 HTTPS、redirect/host policy、timeout、retry、取消、大小上限、流式落盘、`.part`、flush/sync、atomic finalize、protected job directory 和 known-only cleanup。
- 通用 macOS Agent 不再把完整 DMG 收入 `Vec<u8>`，也不再写第二份完整临时副本。
- QoderWork、TRAE Work、WorkBuddy、OpenCode Desktop 和 Codex Desktop 共用一个 managed DMG preparation/replacement owner。
- `Content-Length` 只用于进度和大小上限，不成为新的远端完整性 admission gate。
- 不新增远端 hash、Team ID、Gatekeeper、notarization、签名或 publisher comparison 作为第三方应用 admission gate；继续遵循现有 owning spec 的 bundle identity/version 与本地事务验证合同。

### R8 — 真实下载进度与速度

- 通用 Agent job snapshot 增加封闭 transfer projection，至少包含：
  - `downloadedBytes`；
  - 可选 `totalBytes`；
  - 可选 `bytesPerSecond`；
  - 足以拒绝陈旧速度的 sequence/freshness 信息。
- 下载速度复用 Codex 已有字节增量 + 单调时间状态机，必要时抽取为 shared formatter/state owner；不得在 Agent 页面重写。
- 有总量时百分比 clamp 到 `0..100`，UI 最多显示一位小数。
- 无 `Content-Length` 时显示不定进度、已下载量和可用速度，不伪造百分比。
- 速度未知、为零、已陈旧或离开 downloading 阶段时隐藏，不显示虚假 `0 B/s`。
- terminal snapshot 不保留陈旧速度。
- 官方外部 installer 没有真实字节遥测时，只显示阶段、耗时和安全日志摘要；不得伪造速度。

### R9 — 显式“打开软件”

- 所有桌面安装面在存在唯一或已选择、`launch_eligible` 的可信 candidate 时显示 **“打开软件”**。
- 安装、更新与 no-op 均不调用 launch。
- Codex equal-or-newer 分支改为“已是最新/无需更新”的纯 readback 结果，不再调用 `platform.launch` 或 `succeed_after_launch`。
- 启动复用现有 inventory-bound `platform::process_launch` owner；macOS adapter 使用 Apple 原生 application-open completion API，保持 exact candidate 和 pre-launch refresh/revision 校验，不另建 launcher。
- 启动失败、目标消失和多候选未选择返回封闭 reason code；日志可记录 OS 类别，但不泄露用户目录。

### R10 — Grok Build 官方来源、版本与终态

- 继续复用 `services::tooling` 的 Grok executable discovery、native/npm layout 识别、`GROK_BIN_DIR`、PATH 修复和 anchored update。
- source policy 以 xAI 官方 installer 与官方 enterprise 文档为证据：
  1. native/internal installer 从 `x.ai` 获取；
  2. installer 自身在主源不可达时使用官方 GCS 备用源；
  3. 官方 npm 包 `@xai-official/grok` 使用用户/企业已有 npm registry，是独立分发模式。
- fresh install：preflight 分别给出 `native/internal` 与 `official_npm` 的可用性、版本和网络前提；默认推荐 native，但必须由用户明确选择。native 失败后该 job 进入终态，并提供“改用官方 npm 方式”的新动作，不在原 job 内自动切换。
- native/internal update：使用锚定的 `grok update --check`/`grok update --version <V>`；仅在该 owner 的官方流程内允许 x.ai → GCS 回退，不调用 npm。
- npm install/update：继续使用锚定 npm executable 与官方包，不调用 native updater猜测布局。
- latest/version resolver 按 owner 分开：native 读取 xAI stable channel，npm 读取当前 registry 的官方包版本；两者发布时间不同不是自动失败理由，UI 不把一个 owner 的 latest 冒充另一个 owner 的 latest。
- 每次安装/更新结束后强制重新发现并执行 bounded `grok --version`；命令启动、terminal 打开或退出码 0 均不能单独证明成功。
- 捕获并限长/脱敏 official installer stdout、stderr、exit code、timeout、source category 与 attempt；不能停留在“正在安装”后消失。
- 尊重标准 `HTTPS_PROXY`、`HTTP_PROXY`、`NO_PROXY` 和 npm registry 配置。
- 所有官方路径失败时保留原版本和原安装模式，返回 `source_exhausted`/等价闭合错误，不尝试随机镜像。

### R11 — 状态、取消与错误

- 通用状态至少覆盖：checking、preflight、downloading、staging、awaiting authorization、installing/committing、verifying、succeeded、failed、cancelled、recovery required。
- commit point 前允许取消；目标开始变更后不可取消。
- 至少区分：
  - 用户取消授权；
  - system-commit adapter 不可用；若采用 helper，再区分版本不兼容/peer 拒绝；
  - 应用运行中；
  - target revision 漂移；
  - 主源失败/备用成功；
  - 所有来源耗尽；
  - 版本来源不一致；
  - 启动失败；
  - 安装后未发现；
  - rollback 已恢复；
  - recovery required。
- 日志仅记录 closed product/surface/stage/source/reason、operation ID 摘要和系统错误类别；禁止 token、npm credential、完整 URL query、用户主目录、临时路径或任意 shell 文本。

### R12 — 前端交互

- OpenCode 卡片在同一产品区域明确区分“命令行”和“桌面应用”，两个 section 有独立版本、状态和动作。
- 桌面应用动作顺序：状态/位置 → 安装或更新 → 打开软件。
- 本任务不展示可用的系统 `/Applications` 一键安装；用户目录 candidate 显示「更新当前位置」或等价文案。系统目标若出现，只能是不可用/需后续授权，不得像已接通 helper。
- 进度示例：
  - `下载中 42.7% · 8.3 MB/s`
  - `已下载 126 MB · 8.3 MB/s`
- 复用现有 Button、notice、lifecycle status、target picker 和 shared byte formatter；只有明确存在第二消费者时才新增 shared component。
- 用户可见文案描述实际动作，不暴露 helper、XPC、candidate revision、transaction 等内部术语。

### R13 — Spec 与 owner 收敛

完成实现后更新相关 backend/frontend owning specs，至少覆盖：

- Agent 安装面合同；
- managed desktop product registry；
- shared artifact transport 与 progress；
- macOS bundle metadata owner；
- managed DMG transaction；
- `/Applications` privileged commit：**记录为延期**（系统目标保持 disabled），不把未实现的 helper 写进生产 spec；
- explicit launch；
- Grok source/mode policy；
- UI progress formatting。

新增 shared owner 后删除或委托旧 owner，并通过负向扫描证明没有遗留重复实现。

## 6. 非目标

- Windows 安装、UAC、Registry、MSIX、EXE、Windows user-helper 或 Windows UI 改造。
- 全盘、Launch Services、自定义目录、嵌套目录或外置卷应用发现。
- 自动把 `~/Applications` 历史安装迁移到 `/Applications`。
- 为任意第三方应用提供通用安装器、通用 root 文件管理器或任意命令执行器。
- 引入 Homebrew 作为 OpenCode Desktop 必需依赖。
- 引入 Sparkle 或让第三方应用使用 FyAgent 自更新框架。
- 建设 FyAgent 自有 Grok/OpenCode 镜像服务。
- 把未经保存的 Codex 红色警告描述成已经确定的某个错误。
- 增加第三方应用远端 hash/签名/Team ID/Gatekeeper admission gate。
- 为官方外部 installer 伪造字节进度。
- **本任务不实现** macOS privileged helper、Blessed/SMJobBless、SecureXPC、Apple privileged-file-operations 生产路径，以及向 `/Applications` 的一键写入/替换。后续独立任务处理。

## 7. 验收标准

### 7.1 OpenCode 与发现

- [ ] OpenCode 仍是一个 catalog 产品，并同时展示 CLI 与 Desktop 两个独立安装面。
- [ ] `/Applications/OpenCode.app` 且 bundle ID 为 `ai.opencode.desktop` 时被识别，显示版本和“打开软件”。
- [ ] `~/Applications/OpenCode.app` 历史安装也可识别；两处同时存在时要求选择目标。
- [ ] CLI-only、Desktop-only、两者都有、两者都无四种状态均有测试。
- [ ] binary/XML plist 都能读取；错误 bundle ID、symlink app、损坏 plist 不会被标记为可信安装。
- [ ] 未引入 Launch Services 全局扫描器。

### 7.2 安装位置与事务

本任务验收（用户目录 + 发现）：

- [ ] QoderWork、TRAE Work、WorkBuddy、Codex Desktop、OpenCode Desktop **用户目录** fresh install 可用（`~/Applications`）。
- [ ] `~/Applications` candidate 更新后仍在原路径。
- [ ] `/Applications` 已有安装可被发现（OpenCode 等），但不在本任务内执行系统目录替换。
- [ ] 多 candidate 时要求选择；用户目录事务在 candidate 消失、revision 漂移、应用运行中时零错误写入。
- [ ] 用户目录 post-install verification 失败会恢复旧应用；无法证明恢复时进入 recovery-required。

以下留给 helper 任务，本任务不勾选为完成：

- [ ] N/A 本任务：five products fresh install 以 `/Applications` 为自动目标。
- [ ] N/A 本任务：`/Applications` candidate 更新后仍在原路径（写入）。

### 7.3 system-commit authorization

**本任务不实现。** 验收仅：

- [x] 系统目标保持禁用/`authorization_required`，不以用户目录或 mock 伪装成系统安装完成。
- [x] 未引入 sudo / AppleScript admin / 通用 privileged executor。
- [x] G1 原生授权 spike 已记录且 **未** 标 VERIFIED（`research/g1-authorization-spike.md`）。
- [ ] 其余 signed spike / helper adapter / HIL 条目 **延期到 helper 任务**，不得在本任务勾选通过。

### 7.4 启动

- [ ] 所有可信桌面 candidate 提供“打开软件”；CLI 不显示该按钮。
- [ ] 检查、安装、更新、no-op 与成功刷新均不会自动打开应用。
- [ ] Codex equal-or-newer 分支不再调用 launch。
- [ ] renderer 无法提交任意 `.app` 路径；启动失败和目标漂移有稳定终态。

### 7.5 下载体验

- [ ] 通用 Agent DMG 不再完整载入内存或写第二份完整 DMG。
- [ ] job snapshot 真实携带 downloaded/total/speed；前端不再固定 `percent: null`。
- [ ] 百分比最多一位小数并限定在 `0..100`。
- [ ] downloading 阶段显示真实速度；未知或陈旧速度隐藏。
- [ ] 无总量时显示不定进度与真实已下载量，不伪造百分比。
- [ ] Codex 原有下载、取消、重试、速度、临时目录和 DMG 回归测试保持通过。

### 7.6 Grok Build

- [ ] 检查更新显示本地版本、目标版本、当前分发模式和来源状态。
- [ ] xAI 主源可用时完成官方流程并回读版本。
- [ ] 模拟 xAI 不可达时，官方 installer 使用 GCS fallback，并在诊断中记录来源降级。
- [ ] fresh install 可显式选择官方 npm 方式；native 失败只提供新的显式切换动作，不在同一 job 自动改变 owner。
- [ ] native update 失败不会静默改成 npm 模式。
- [ ] official installer 非零退出、timeout、stderr、所有来源失败均形成持久终态。
- [ ] 所有失败保留原可运行版本、symlink 和安装模式。
- [ ] 中国大陆网络环境完成主源/官方 fallback/npm 配置的真实验收记录；不宣称不存在的大陆官方镜像。

### 7.7 工程与安全

- [ ] Codex 与 managed Agent 复用一个 artifact transport、一个 bundle metadata owner 和一个 macOS DMG replacement transaction。
- [ ] 没有新增第二个 downloader、scanner、launcher、speed formatter 或通用 privileged executor。
- [ ] 新增开源依赖有许可证、维护状态、固定版本、供应链与构建/发布集成记录。
- [ ] Rust/TypeScript wire contract 同步升级，strict parser 拒绝未知字段和非法组合。
- [ ] Windows 契约和测试保持原结果。
- [ ] backend/frontend specs 更新为最终行为，任务计划性内容不进入永久规范。

## 8. 最低测试矩阵

| 维度 | 必须覆盖 |
| --- | --- |
| 系统 | macOS 12；当前受支持 macOS；Apple Silicon 必测；Intel 至少 source/arch 单测，具备设备时做 HIL |
| 产品 | QoderWork、TRAE Work、WorkBuddy、Codex Desktop、OpenCode Desktop、OpenCode CLI、Grok Build CLI |
| 位置 | 仅 `/Applications`、仅 `~/Applications`、两处同时存在、均不存在 |
| 权限 | 本任务：系统目标保持禁用。原生授权/helper 矩阵延期 |
| 事务 | 本任务：user-scope fresh/update、cancel、verify/rollback。system update 延期 |
| 运行状态 | 应用已退出、应用运行中、preflight 后目标被外部替换 |
| 网络 | native 主源正常、主源失败/官方 GCS fallback 成功、显式 npm fresh install、全部失败、代理、未知 Content-Length |
| UI | OpenCode 双安装面、单/多 candidate、真实/不定进度、速度陈旧、显式启动、启动失败 |

## 9. 实施前硬门禁

1. **G1 — Reuse gate**：完成 owner/开源候选矩阵；无法证明现有能力不足时不得新增模块或依赖。
2. **G2 — System-commit selection gate（延期）**：本任务不选择、不落地生产 adapter。系统目标保持禁用。后续 helper 任务再做 Gate A/B。
3. **G3 — OpenCode source gate**：确认当前官方 Apple Silicon/Intel DMG 入口、bundle ID 与架构唯一匹配。
4. **G4 — Grok official behavior gate**：以当前 official installer fixture 验证 channel、主源/GCS fallback、目录布局、安装模式和 `--version` 回读；未证明必要前不得复制 installer 算法。
5. **G5 — Shared owner gate**：证明 managed Agent 调用 Codex 的下载、bundle 和 DMG owner，而不是复制代码。
6. **G6 — Signed product HIL gate（系统安装延期）**：本任务不宣称系统安装完成；用户目录与发现/启动可用自动化证据归档。

## 10. 交付物

- 版本化的 Agent surface/readiness/action/job progress 合同。
- OpenCode Desktop product policy、发现、官方来源、安装、更新和显式启动接线。
- shared artifact transport、bundle metadata reader、DMG transaction 与 progress projection。
- **不交付** system-commit adapter / privileged helper；规格中写明系统目标保持 disabled，留给后续任务。
- Codex 显式启动/no-auto-launch 行为。
- Grok Build 官方来源/模式/终态/版本回读改进。
- Agent 页面双安装面、系统授权、真实进度、速度和“打开软件”交互。
- 单元、契约、架构、浏览器和原生 macOS HIL 证据。
- 更新后的 backend/frontend owning specs 与最终 reuse decision。
