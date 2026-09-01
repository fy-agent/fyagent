# 统一桌面 Agent 安装面并完成 Windows 适配与 Codex 日志治理

## 0. 任务状态

- 优先级：P0
- 状态：planning，仅产出任务规划；本次不启动实现
- 主平台：Windows 正式提权构建与开发构建
- 兼容平台：macOS 做统一安装策略收敛与现有桌面链路回归保护
- 执行方式：单任务串行推进，先证据与复用评审，再实现
- 核心约束：**不得为了赶进度新建下载器、包管理器、签名校验器、通用命令桥、更新守护进程、Windows 服务或第二套用户态运行时。**

## 1. 背景与已确认问题

截至提交前重新核验的 `dev/laiyongjie` 基线 `b1335f2f`，仓库并非“没有 Windows 能力”，而是已有安全底座与产品实现没有完全闭环：

1. Windows 正式构建以管理员权限运行。`services/tooling.rs` 为避免管理员进程读取或执行普通用户 CLI，当前对正式 Windows 构建整体关闭 CLI 探测与生命周期操作；因此开发态可工作的 Grok 发现、安装或更新，在正式安装态会直接不可用。
2. 仓库已有冻结的 Explorer 交互用户上下文、普通用户 `fyagent-user-helper`、带 nonce/job 的 one-shot 命名管道协议、Codex MSIX 安装链路、受保护下载产物、签名校验以及 QoderWork/TRAE Work/WorkBuddy 的 Windows EXE 安装链路。这些必须复用，不能另起炉灶。
3. 当前 Agent 产品契约已将 Claude 与 OpenCode 定义为 Desktop-only；但它们的 Windows 产品身份、发现规则、安装目标与源解析仍为空或明确 fail-closed，实际只完成了 macOS 路径。
4. QoderWork、TRAE Work、WorkBuddy 与 Codex Desktop 已有 Windows 代码，但既有任务保留了“真实 Windows x64 正式构建 HIL 未完成”的风险，不能把 mock、单元测试或 CI 当作完整可用证明。
5. OpenAI 于 2026-09-01 发布的新 ChatGPT 桌面应用已把 Chat、Work 与 Codex 合并，旧版还可能以 ChatGPT Classic 并存；现有 Codex 精确包身份必须在真实 Windows 上重新核验，但不得退化为显示名或进程名模糊匹配。
6. Codex session usage 同步把“父 rollout 尚未追到 child fork 时刻”的可恢复依赖状态逐文件记录为 `WARN`。同步器启动时执行一次，之后每 60 秒执行，同一批历史文件会反复进入该路径，造成大量无操作价值的日志污染。
7. 旧 Settings/Tooling 仍为 Claude、Gemini、OpenCode、OpenClaw、Hermes 等构造 npm、Shell、PowerShell 安装/更新与复制命令入口，形成与 Agent Desktop policy 冲突的第二套安装 owner。

本任务不是“再写一套 Windows 安装器”，而是把现有 owner、当前 Agent 产品合同和官方发行方式接成一个可证明的 Windows 闭环。

## 2. 目标

### 2.1 产品目标

| 产品 | 当前合法 surface | Windows 目标 | 更新要求 |
| --- | --- | --- | --- |
| QoderWork | Desktop | 发现、安装、启动、正式包实机回归；只修复真实失败 | 保持 `update=false`；本任务不得开启 |
| TRAE Work | Desktop | 发现、安装、启动、正式包实机回归；只修复真实失败 | 保持 `update=false`；本任务不得开启 |
| WorkBuddy | Desktop | 发现、安装、启动、正式包实机回归；评审 EXE 与 Store 身份 | 保持 `update=false`；本任务不得开启 |
| Codex | Desktop | 发现、安装、更新、启动完整回归；核验新 ChatGPT、升级后的 Codex 与 ChatGPT Classic 并存身份 | 只接受 HIL 证明的精确 package/AUMID 迁移集合；禁止显示名兼容和并排假更新 |
| Claude（内部 ID 仍为 `ClaudeCode`） | Desktop-only | 补齐 Windows 发现、安装、更新、启动 | 通过 G1 决策门确定官方用户安装器或 MSIX owner；不得形成双重更新 owner |
| OpenCode | Desktop-only | 补齐官方 Windows Desktop 的发现、安装、更新、启动 | x64 复用官方签名 EXE；ARM64 必须满足当前 release 资产、签名身份与原生 HIL 后才可开启 |
| Grok Build | CLI | 在正式提权构建中由普通 Explorer 用户完成发现、安装、更新 | 保持已安装 distribution owner；新装只使用经评审的官方 owner |

### 2.2 工程目标

- 正式提权主进程只负责策略、调度、下载与验证；普通用户观察/执行由现有 `fyagent-user-helper` 承担。
- 安装或更新成功必须由安装后的注册表、包清单、可执行文件身份与版本探针重新发现证明，不能只依据安装器退出码、文件存在或远端版本号。
- UI 复用现有 Agent inventory/readiness/action DTO；仅在确有新状态时扩展闭集 reason code，不创建 Windows 特供的平行前端流程。
- 可恢复 Codex fork 依赖不再逐文件输出 `WARN` 或每轮重复 `INFO`；真正的数据损坏、I/O、数据库或一致性错误仍保留可操作告警。
- macOS 与 Windows 的旧 Settings/Tooling 安装面必须统一收敛：除 Grok Build 外，删除/拒绝所有公开 CLI 安装、更新、复制命令和远程脚本入口；其他业务确实需要的只读 CLI 发现与配置能力按消费者保留。

## 3. 强制复用与“禁止造轮子”门禁

任何实现代码开始前，执行者必须在 task `research/` 中补齐并更新复用审计，按以下顺序决策：

1. **现有 FyAgent owner**：优先扩展 `agent_install`、`codex_desktop`、`windows_runtime`、`fyagent-user-helper`、下载/签名/inventory 与前端 DTO。
2. **操作系统或框架能力**：优先使用 Win32、AppX/MSIX、注册表、Authenticode、Tauri 与现有 `windows-rs` 能力。
3. **项目已采用依赖**：先确认现有 Rust/Node 依赖是否已覆盖需要的接口。
4. **成熟维护中的开源实现**：只有前三层不能满足时才评审；必须记录许可证、维护状态、依赖成本、安全面与弃用风险。
5. **最薄产品适配层**：最后才允许写产品特定 glue；不得复制整个安装/更新框架。

明确禁止：

- 新建第二个 downloader、缓存系统、签名校验器、包管理器、安装事务、update daemon、Windows service 或用户态 runtime。
- 新建接受 `command`、`args`、`cwd`、`url`、`verb`、脚本正文、任意路径或任意环境变量的通用 helper/IPC。
- 从管理员主进程直接执行用户 profile 中的 CLI，或在普通用户 helper 失败时回退为管理员执行。
- 解析 WinGet、PowerShell、npm、安装器等面向人的本地化表格文本作为安装状态权威。
- 仅凭路径、文件名、目录存在、进程退出码 0、远端版本号或窗口/进程显示名宣告安装、更新或身份匹配成功。
- 为了让 Claude Cowork 可用而静默开启 Windows 可选功能、修改启动项或自动重启设备。
- 因日志太多而全局关闭 `WARN`、屏蔽整个模块或吞掉真正错误。

## 4. 功能需求

### R0. 跨平台安装面统一

- macOS 与 Windows 共享同一个产品/surface/action policy：QoderWork、TRAE Work、WorkBuddy、Codex、Claude Desktop、OpenCode Desktop 为 Desktop；只有 Grok Build 为 CLI。
- renderer 只呈现 backend `allowedActions`；旧 CLI 或非法 update 请求必须在网络、目标查询、下载、helper、文件系统和进程启动之前零副作用拒绝。
- non-Grok Tooling installer 的退场同时覆盖前端按钮/复制命令/文案和后端 direct IPC，不能只隐藏 UI。
- QoderWork、TRAE Work、WorkBuddy 始终保持 install + launch、无 FyAgent update。

### R1. Windows 运行上下文

- 正式 Windows 构建继续维持 `requireAdministrator` 与冻结 Explorer 用户的安全模型。
- 所有用户态发现、版本探针与 Grok 生命周期执行必须绑定启动时冻结的 SID、session、profile、LocalAppData、RoamingAppData 与 PATH；运行中 shell 用户变化不得静默切换目标用户。
- 无 Explorer、无可验证 helper、session/SID 漂移、helper 超时、UAC 取消、管道握手失败时均 fail-closed，并返回稳定、可本地化的 reason code。
- 不得存在 elevated fallback。

### R2. Windows Desktop Agent 发现

- 复用现有 Windows inventory，组合使用注册表 Uninstall/App Paths、AppX/MSIX inventory、受限已知目录与文件身份；每个产品只提供闭集身份描述。
- Claude 与 OpenCode 的产品名、package identity、相对 EXE、发布者/签名主体、默认 scope 只能来自官方包与真实 Windows HIL 证据，不得猜测或照搬第三方截图。
- Codex 必须分别核验新 ChatGPT 干净安装、旧 Codex 官方升级以及与 ChatGPT Classic 并存的状态；只允许闭集 package name/publisher/family/application ID/AUMID 身份，不得以 `ChatGPT`、`Codex` 显示名、窗口标题或进程名授权操作。
- 同一产品存在 0、1、多份安装时必须区分：未安装、唯一可信目标、需要用户选择、证据不完整或冲突。
- 配置目录、session 目录、AppExecutionAlias 或 PATH 命中只能作为候选线索，不能单独作为 Desktop 安装证据。
- inventory adapter 访问失败或证据不完整不能被映射为“未安装”。

### R3. Windows Desktop 安装与更新

- 全部复用现有“源解析 → 受保护下载 → 产物重验证 → Authenticode/AppX 身份验证 → 普通用户 helper/既有包桥 → 安装后重新发现”管线。
- Claude 设置 G1 决策门：
  - 对比官方用户友好安装器与官方 x64/arm64 MSIX，记录功能完整性、scope、Cowork 服务注册、更新 owner、包身份、卸载与回滚行为。
  - 若 per-user MSIX 会牺牲当前产品承诺的功能，不得为了代码复用强行选择；应复用现有 EXE helper 运行经过验证的官方安装器。
  - 若选择 MSIX，必须复用 Codex 已有 AppX/MSIX 下载、验证、bridge/helper 与显式 SID inventory，不得使用新的 PowerShell 字符串拼接方案。
  - 必须明确唯一更新 owner，不能让 FyAgent、厂商 updater、Store 或 MDM 互相覆盖。
- OpenCode Windows x64 使用上游官方 stable Desktop 资产与固定仓库版本证据，复用现有 EXE helper。ARM64 仅在当前 first-party release 真实提供对应资产、签名/身份通过且完成 Windows ARM64 原生 HIL 后开启；workflow 配置或 x64 模拟运行不能替代。
- Codex 设置 G5 身份门：先用新 ChatGPT 干净安装、旧 Codex 正常升级与 ChatGPT Classic 并存三组 HIL 冻结精确 identity；只有现有 owner 无法识别真实目标且证据闭合时，才加入小型 first-party migration set。
- 更新操作必须绑定用户确认的现有 stable target。完成后同一目标的版本或经过审查的 exact identity 必须发生符合预期的变化；不得通过新装到另一目录伪装为更新。
- QoderWork/TRAE Work/WorkBuddy 当前 `update=false` policy 不得被顺手改开。无官方行为与真实 HIL 证明时，UI 必须诚实保持不可更新。

### R4. Grok Build CLI 普通用户链路

- 只为当前 Agent policy 合法的 Grok CLI lifecycle 接入普通用户 helper；不得借此重新开放 Claude/OpenCode Agent CLI surface。
- helper 请求采用闭集 `{ tool, action, expected_owner? }` 语义；首期 `tool` 只允许 Grok Build，`action` 只允许 `observe/install/update`。
- helper 内复用或抽取现有 Windows candidate、owner 与版本规则，执行固定版本探针与固定上游动作；不得在主进程和 helper 各复制一套候选路径表。
- 响应只返回结构化、限长、脱敏结果，例如 detected、normalized version、owner、outcome 与 reason；不得回传原始 stdout/stderr、绝对路径或命令行。
- 已安装 Grok 必须保持 owner（例如 official native 或 official npm）进行更新；新装 owner 选择必须来自已评审的官方渠道，禁止隐式跨 owner 迁移。
- Settings/Tooling 中所有 non-Grok 公开 install/update action、复制命令与远程脚本入口都属于本任务收敛范围；只读版本/路径/配置能力仅在有明确业务消费者时保留。

### R5. 前端与状态投影

- 继续使用当前 7 个 Agent 产品和既有 surface policy，不新增 Windows 特供产品或重复卡片。
- 操作按钮只由后端 readiness/actionability 决定；发现不完整、目标歧义、helper 不可用、平台不支持时不得显示可执行假象。
- 安装/更新完成后强制刷新 inventory；只在权威回读成功后显示完成，不在前端乐观修改版本或状态。
- 新 reason code 必须同时补齐 TypeScript 类型、中文/英文/日文/繁中文案与 parity 测试。
- UI 不显示安装器原始输出、本地绝对路径、SID、package family、rollout ID 或 session 内容。

### R6. Codex deferred 日志治理

- 将“父 rollout 仍未覆盖 child fork 时间”建模为闭集、可判断的 pending reason，避免依赖中文或英文错误字符串反向分类。
- 将“是否重试”与“是否已经输出诊断”分离；清理 retry 计算结果不得同时清除 diagnostic fingerprint。
- 对仍可能增长的父文件保持可恢复重试；对长期稳定、父子均无变化的 fork gap 采用有界退避或稳定挂起，避免每 60 秒重复解析与告警。
- 正常 deferred 不逐文件输出 `WARN`。默认生产日志应静默；调试模式每轮最多输出一条脱敏、按 reason 聚合的 `DEBUG` 摘要。
- 真正的解析损坏、不可恢复时间线冲突、I/O、数据库写入或不变量破坏继续输出 `WARN`/`ERROR`，但同一未变化文件与同一 reason 必须按 fingerprint 去重。
- 日志不得包含完整用户目录、session 文件名、rollout ID、token、提示词或会话正文。
- 降噪不得改变使用量语义：父链未满足前不能提前导入 child；父文件补齐后必须能恢复并且只导入一次。

### R7. 文档与规范

实现完成前必须更新以下事实合同：

- Windows 正式构建不再是笼统的“所有 CLI 永久不可用”，而是“只有经过闭集 helper policy 允许的 Agent 操作可用”。
- Agent 产品 surface、Windows package owner、发现证据、安装/更新成功定义与不支持矩阵。
- helper IPC 安全边界与禁止自由参数。
- Codex/新 ChatGPT/ChatGPT Classic 的精确身份和条件迁移合同。
- Codex session deferred 的日志级别、聚合、重试与去重合同。

## 5. 非目标

- 不恢复 Claude Code CLI 或 OpenCode CLI 作为 Agent 产品 surface。
- 不删除 Provider、Skills、MCP、模型、会话或配置业务仍需要的只读 CLI 能力；但所有 non-Grok 公开 install/update/manual-command surface 必须退场。
- 不把稳定 Agent 产品 ID `Codex` 改名为 ChatGPT；只处理物理 Windows 应用身份的精确兼容。
- 不实现卸载、登录/Auth、账户迁移、配置迁移或 WSL 自动安装。
- 不修复上游 Claude/OpenCode/Grok 自身运行缺陷。
- 不静默安装 Virtual Machine Platform、WSL 或其他系统组件，不自动重启。
- 不声明无当前官方包、签名身份或原生 HIL 的 Windows ARM64 产品已受支持。
- 不重写已经工作的 macOS DMG/helper 链路；只移除与统一 desktop-only 策略冲突的 non-Grok CLI 安装面，并完成行为保持的回归。

## 6. 验收标准

### A. 架构与复用

- [ ] `research/` 中存在逐项复用审计；每个新增模块都说明为什么不能由现有 owner、OS 能力或已采用依赖完成。
- [ ] 仓库不存在第二套下载、签名、MSIX、EXE 安装、Explorer 用户上下文、job 或 update owner。
- [ ] helper/IPC schema 不接受自由命令、参数、URL、路径、cwd、脚本或原始环境。
- [ ] 正式提权主进程不直接执行普通用户 Grok CLI，helper 失败也无 elevated fallback。
- [ ] macOS/Windows 的 non-Grok Settings/Tooling install/update/manual-command surface 已全部退场；有明确消费者的只读发现/配置能力仍正常。

### B. 产品矩阵

- [ ] 在干净 Windows 11 x64 正式安装包环境中，当前 7 个 Agent 产品均显示真实、可解释的状态；不支持项明确 fail-closed。
- [ ] QoderWork、TRAE Work、WorkBuddy、Codex、Claude、OpenCode 的唯一可信安装均能被识别；仅有配置目录、alias、错误签名或伪造 EXE 时不得误识别。
- [ ] 当前 policy 允许安装的产品可完成一键安装，并通过安装后 inventory 回读证明。
- [ ] 当前 policy 允许更新的产品可在同一 target 完成更新并回读新版本或审查过的 exact identity；旧版本不变、并排新装、inventory 不完整均判失败。
- [ ] Desktop 启动发生在冻结的 Explorer 用户会话，不以管理员桌面或错误用户启动。
- [ ] Grok 在正式 Windows release 中可由普通用户 helper 完成 observe/install/update，owner 不被隐式替换。
- [ ] Claude/OpenCode Agent CLI surface 仍被 policy 拒绝，没有因复用旧 tooling 映射而重新暴露。
- [ ] QoderWork、TRAE Work、WorkBuddy 的 UI 不显示更新，direct update 请求在任何 side effect 前稳定拒绝。
- [ ] 六个 Desktop 产品在 macOS/Windows 均只有一个 Desktop lifecycle component；只有 Grok Build 显示 CLI lifecycle。
- [ ] Codex 覆盖新 ChatGPT 干净安装、旧 Codex 升级和 ChatGPT Classic 并存；目标选择基于精确 package/AUMID，Classic 或同名进程不得被误认。

### C. 边界场景

- [ ] 覆盖标准用户登录 + 管理员启动 FyAgent、管理员用户登录、无 Explorer、SID/session 漂移、UAC 取消、helper 超时/崩溃、管道伪造/重放、安装器非零退出。
- [ ] 覆盖中文/空格用户目录、非 ASCII 显示名、禁用 AppExecutionAlias、空 PATH、损坏注册表项、0/1/多安装、用户级与机器级并存。
- [ ] 覆盖断网、下载中断、非 allowlist redirect、签名错误、package identity/架构不符、安装后无权威回读。
- [ ] ARM64 对每个产品按当前官方产物、签名身份与原生 HIL 逐项声明；未证明的产品稳定返回不支持。

### D. 日志

- [ ] 同一未变化的可恢复 parent/child fork gap 连续运行等价于 120 次 60 秒同步：逐文件 `WARN` 为 0，重复 `INFO` 为 0；调试模式每轮最多 1 条限长脱敏聚合，日志量不随 deferred 文件数线性增长。
- [ ] 父文件补齐后 child 能恢复导入且只导入一次。
- [ ] 父子文件长期稳定但不满足 fork 时不会每分钟重复解析或告警；相关 fingerprint 变化后仍会重新评估。
- [ ] 同一未变化的真实损坏只产生一次 fingerprint 告警；文件证据或 reason 变化后可以再次告警。
- [ ] 相关生产日志不含完整绝对路径、rollout ID、session 文件名或会话内容。

### E. 自动化与实机证明

- [ ] Windows x64 与 ARM64 native CI 的 workspace check、clippy、tests 通过；产品级 ARM64 支持仍按官方证据单独判断。
- [ ] helper protocol/action/order/nonce/replay/限长/超时测试覆盖新增动作。
- [ ] Agent source、inventory、target selection、更新回读、错误投影和多语言前端测试通过。
- [ ] Codex deferred 分类、退避/稳定挂起、去重、恢复与日志预算测试通过。
- [ ] macOS 共享路径回归测试通过。
- [ ] **真实 Windows x64 正式安装包 HIL 是任务完成硬门禁**：干净安装、预装旧版、更新、启动、取消、失败恢复均留存脱敏证据；未执行时不得声明完成或归档为“已支持”。
- [ ] Windows ARM64 只对有当前官方 ARM64 产品包且完成原生 HIL 的链路做支持声明；其余产品保持明确不支持。

## 7. 完成定义

只有在以下条件全部满足后才能归档：代码与规范一致、强制测试通过、真实 Windows x64 正式包 HIL 完成、所有未支持项在 UI/DTO 中诚实表达、日志污染被可重复测试证明已消除、没有新增通用执行能力或重复基础设施、两端 non-Grok 公开 CLI 安装面已退场且所需只读配置能力未回退。
