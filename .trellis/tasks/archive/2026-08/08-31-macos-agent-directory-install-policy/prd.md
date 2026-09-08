# 收敛 macOS Agent 目录排序与一键安装策略

## 0. 任务状态

- 状态：`planning`
- 优先级：P0
- 实施范围：macOS 一键安装、更新、扫描排序和相关共享产品合同。
- Windows：不新增或验收 Windows 桌面安装器；产品级“无更新/无 CLI 安装面”决策不得在后续 Windows 任务中被重新引入。
- 写集：本轮规划只允许修改本任务目录。现有实现和其他 Trellis 任务只读。
- 启动门禁：用户已批准与 helper 任务并行实施。写集仍须避开 helper 独占文件；`/Applications` 生产启用仍等 helper HIL。

## 1. Goal

把 Agent 软件目录收敛为一个基于真实扫描结果、产品策略明确、安装来源可解释的 macOS 一键安装入口：

1. 扫描完成后，已安装软件优先于未安装软件；已安装软件中，QoderWork、TRAE Work、WorkBuddy 优先；
2. QoderWork、TRAE Work、WorkBuddy 只提供 FyAgent 一键安装和已安装后的打开/配置，不提供 FyAgent 一键更新；
3. OpenCode 与 Claude 不再把 CLI 作为 Agent 一键安装面；
4. OpenCode Desktop 和 Claude Desktop 复用现有桌面安装基础设施，提供一键安装、一键更新和“打开软件”；
5. Claude Desktop 使用经过审查的国内网络友好固定镜像端点，但不把下载可达性描述为服务地区可用性，也不引入任意镜像/URL 能力；
6. 不新建第二套下载器、DMG 安装事务、进度状态机、版本比较、target authority 或应用启动器。

## 2. Confirmed facts

### 2.1 用户产品决策

- QoderWork、TRAE Work、WorkBuddy 是本任务定义的“国产优先”三项。
- 上述三项只需要一键安装，不需要 FyAgent 一键更新。
- Agent 目录排序必须由扫描结果驱动，而不是永远使用固定产品顺序。
- OpenCode CLI 和 Claude Code CLI 不再作为 Agent 目录的一键下载/安装形态。
- OpenCode Desktop 和 Claude Desktop 需要一键更新。
- 先完成 macOS；Windows 桌面安装适配在 macOS 验证稳定后另行规划。

### 2.2 当前仓库事实

- `PRODUCT_DIRECTORY` 与后端 Agent Catalog 当前都有固定七项顺序，`AgentDirectory` 直接按输入数组渲染，没有扫描后排序 owner。
- 扫描状态已经区分 `installed`、`installed_not_runnable`、`not_installed`、`unknown`、`unavailable`、technical error，并保留后续刷新失败前的上一次成功结果。
- OpenCode 当前同时声明 `cli` 和 `desktop`；Claude Code 当前只声明 `cli`。
- QoderWork、TRAE Work、WorkBuddy 当前的 desktop readiness 和 action dispatcher 允许 `update`。
- OpenCode Desktop 已有官方双架构 DMG 固定端点和桌面源适配器。
- 通用 managed-desktop 路径已经复用 Codex 的流式下载、DMG 只读挂载、唯一顶层 `.app`、同卷 staging、替换、回滚和安装后回读能力。
- `/Applications` 系统提交由独立 helper 任务负责；本任务不得复制提权实现。

### 2.3 Claude Desktop 外部证据

- Anthropic 当前把 Chat、Cowork 和 Claude Code 放在同一个 Claude Desktop 应用中；因此移除 Agent CLI 安装面后，桌面应用是本任务可管理的真实软件形态。
- Anthropic 官方提供 macOS DMG/PKG，并说明 universal 构建同时支持 Intel 与 Apple Silicon。
- 2026-08-31 对当前镜像 DMG 的只读检查确认：顶层应用为 `Claude.app`，Bundle ID 为 `com.anthropic.claudefordesktop`，版本源位于 Info.plist，最低系统为 macOS 12，包含 `x86_64` 与 `arm64`，Developer ID 和 Gatekeeper/公证检查通过。
- `Wangnov/claude-app-mirror` 是 MIT 许可的窄镜像项目：同步官方当前安装包到 GitHub Releases 与 Cloudflare R2，不构建、不修改、不重打包 Claude。
- Anthropic 当前公开的 Claude 可访问地区列表未列出中国大陆。镜像只能改善安装包传输，不能被宣传为绕过账号、服务或地区政策。

详细证据见：

- `research/current-implementation-audit.md`
- `research/upstream-source-evidence.md`
- `research/reuse-decision.md`

## 3. Product lifecycle matrix

| Product ID | Agent 安装形态 | 未安装 | 已安装 | FyAgent 更新 | 打开软件 | 说明 |
| --- | --- | --- | --- | --- | --- | --- |
| `qoderwork` | desktop | 一键安装 | 显示已安装 | 禁止 | 允许 | 保留官方 source，仅用于首次安装 |
| `trae-work` | desktop | 一键安装 | 显示已安装 | 禁止 | 允许 | 保留官方 source，仅用于首次安装 |
| `workbuddy` | desktop | 一键安装 | 显示已安装 | 禁止 | 允许 | vendor `/v2/update` 可继续作为安装 release metadata，不等于允许 FyAgent 更新动作 |
| `grokbuild` | cli | 保持现状 | 保持现状 | 保持现状 | 不适用 | 不在本任务重构 |
| `codex` | desktop | 保持专用安装器 | 保持现状 | 保持现状 | 允许 | 继续作为共享基础设施黄金回归样例 |
| `claude-code` | desktop | 一键安装 Claude Desktop | 显示 Claude Desktop | 允许 | 允许 | 产品 ID 保持，Agent 生命周期不再提供 CLI |
| `opencode` | desktop | 一键安装 OpenCode Desktop | 显示 OpenCode Desktop | 允许 | 允许 | Agent 生命周期不再提供 CLI |

“禁止更新”只表示 FyAgent 不提供对应的一键更新动作，不禁止厂商应用自身的自动更新能力。

## 4. Requirements

### R1 — macOS-first and compatibility

- 本任务实现与 HIL 只覆盖 macOS。
- 不新增 Windows source、MSIX/EXE installer、Registry/App Paths 或 Windows helper 行为。
- shared catalog/surface 产品合同可以反映“无 CLI 安装面、国产三项无 update”这一长期决策，但除这些明确决策外不得改变 Windows 行为。
- 项目最低 macOS 版本继续为 12.0。

### R2 — Stable scan-driven ordering

- 后端 Catalog 和 `PRODUCT_DIRECTORY` 的 canonical order 继续作为稳定 tie-breaker；不得为了 Agent 页面排序而修改全局产品目录顺序。
- 初次扫描未完成时保持 canonical order，避免卡片随每一项异步返回而跳动。
- 初次扫描完成后按以下 bucket 稳定排序：
  1. 已安装的国产三项；
  2. 其他已安装项；
  3. 未能可靠判定的项；
  4. 已确认未安装的项。
- `installed_not_runnable` 仍属于“已安装”。
- `unknown`、`unavailable`、当前扫描 technical error、当前扫描没有有效结果都属于“未能可靠判定”，不得伪装成未安装。
- 后续重新扫描期间冻结上一轮已提交顺序；整轮扫描完成后一次性重新排序。
- 生命周期动作成功并完成权威 readiness reread 后，如果当前没有扫描进行，可立即重新投影顺序，使刚安装的软件进入已安装分组。
- 每个 bucket 内保持 canonical order；国产优先只作用于“已安装”bucket，未安装项不得仅因国产身份被提前。
- 排序变化不能丢失当前键盘焦点、链接语义或 card key。

### R3 — One semantic owner for domestic priority

- 国产优先信息必须放进现有共享产品目录元数据，不允许页面再维护第二个 `Set`/数组。
- 后端不需要因为 UI 排序新增地域判断；生命周期动作权限由独立 product policy owner 管理。
- 名称、描述或目录顺序不得被用来推断国产身份。

### R4 — QoderWork/TRAE Work/WorkBuddy are install-only

- 三项未安装且 source/target 可用时，backend `allowedActions` 只允许 `install`。
- 三项已安装时，`allowedActions` 不得包含 `update`；可包含符合现有权威条件的 `launch`。
- 三项的 `updateState` 对 FyAgent 必须是 `unavailable`，不得显示“检查更新”“有新版本”“已是最新”或一键更新按钮。
- normalized inventory candidate 的 update eligibility 必须为 false。
- 直接调用 `start_agent_action(action=update)` 必须在 target revalidation、网络请求、下载和文件变更之前失败。
- 使用稳定、语义准确的 `action_not_supported` reason code；不得使用 `source_not_verified`、`surface_not_supported` 或 `executor_not_implemented` 冒充产品策略拒绝。
- 已安装三项的 readiness 不应只为更新比较而访问远端 metadata；未安装时仍可解析 release/source 以支持首次安装。
- 现有 source resolver、下载器和安装事务不得删除，因为它们仍是首次安装所需能力。

### R5 — Remove Agent CLI install surfaces without deleting product domains

- `opencode` 的合法/default Agent surface 改为 desktop only。
- `claude-code` 的合法/default Agent surface 改为 desktop only。
- Agent Catalog 官方链接删除 OpenCode CLI 与 Claude Code CLI 安装入口，保留产品/桌面官方入口。
- Agent readiness、inventory、target picker、action dispatcher、auth handoff 中不得继续把这两个产品路由到 CLI installer。
- 删除 Agent 页面中的 CLI/desktop 双行投影和 CLI-specific primary action。
- 本任务不删除 OpenCode/Claude 在 Provider、Skills、MCP、模型配置、会话、用户已有 CLI 探测等其他领域的稳定身份；需要删除底层 Tooling 支持时必须另有独立证据和任务。

### R6 — Claude Desktop managed source

- 保持 `AgentCatalogId::ClaudeCode` 和现有配置/分配 ID，避免跨领域迁移；安装面与用户可见 component label 使用 `Claude Desktop`。
- 新增一个受控 managed-desktop product policy：
  - Bundle ID：`com.anthropic.claudefordesktop`；
  - canonical app basename：`Claude.app`；
  - version source：Info.plist；
  - package format：DMG；
  - host artifact：universal macOS DMG。
- metadata 只从固定 `https://claudeapp.agentsmirror.com/latest/manifest` 获取；artifact 只从固定 `https://claudeapp.agentsmirror.com/latest/mac` 获取。
- manifest parser 只接受有界 schema v2 和精确 `sources.macos.universal` 分支，验证 `platform=darwin`、`arch=universal`、`format=dmg`、一致且有界的 version，以及可选 `contentLength` hint。
- manifest 中的 `url`、`redirect`、文件名、hash、ETag 和 Last-Modified 不得变成下载 capability；下载 URL 由 Rust 固定枚举决定。
- 当前镜像为经过本任务审查的单一产品 source，不得推广为通用代理、任意 host 或用户可输入 URL。
- source 不可用、schema 漂移或 post-install version 不匹配时 fail closed，并提供 Anthropic 官方下载页入口。
- 安装包可下载不等于 Claude 服务在用户地区可用。产品文案不得声称镜像解决登录、账号或地区限制。

### R7 — Claude Desktop install and update

- 未安装时使用共享 managed-desktop job 执行一键安装。
- 已安装时比较本地 Info.plist version 与镜像 manifest version，只在有新版本或 latest 状态无法可靠比较但用户明确执行时提供更新。
- 更新绑定当前选中的 opaque inventory target，保持 `/Applications` 或 `~/Applications` 的原位置。
- `/Applications` commit 必须委托独立 macOS privileged helper owner；本任务不得实现 sudo、AppleScript admin 或第二个 helper。
- 安装/更新后必须通过现有 inventory 重新发现并读取真实本地版本，成功后不自动启动。
- 只有用户点击“打开软件”才启动 Claude Desktop。

### R8 — OpenCode Desktop update

- 继续使用官方、架构唯一的 stable DMG endpoint和现有 managed-desktop transaction。
- 优先复用现有 GitHub latest-version HTTP owner或抽取其窄适配，读取 `anomalyco/opencode` 最新稳定 tag，避免为 update availability 另写一套 GitHub 客户端。
- artifact 下载仍使用固定 OpenCode stable endpoint；metadata 与下载之间发生版本漂移时，mounted bundle version mismatch 必须返回 refresh/retry，而不是安装未知版本。
- 不调用、复制或嵌入 OpenCode 自身 Electron updater；FyAgent 的 job、target authority、下载、回滚和 helper 合同仍是唯一 owner。
- 已安装 OpenCode Desktop 在远端版本更高时提供一键更新；更新成功后显示实际版本并保留“打开软件”。

### R9 — Backend-enforced lifecycle policy

- 新增或收敛一个 crate-private `AgentLifecyclePolicy` owner，用一个封闭表/函数同时驱动 legal surfaces、readiness actions、inventory eligibility、source resolution条件和 action dispatch。
- 不允许在 `types.rs`、`inventory.rs`、`mod.rs`、前端页面分别复制产品 allowlist 后各自漂移。
- renderer 只呈现 backend `allowedActions`；隐藏按钮不是安全边界。
- action request 继续只接受 product ID、closed action、surface 和 opaque target/release capabilities；不新增 URL、路径、命令或 bypass 字段。
- malformed/旧 surface、国产三项 update、OpenCode/Claude CLI action 均必须被 backend 以稳定 reason code 拒绝，且零副作用。

### R10 — Reuse-first implementation

按以下顺序决策：

1. 现有 FyAgent owner；
2. 当前仓库已采用的框架、crate 和 shared component；
3. 已审查的上游官方 API/开源实现；
4. 一个窄的 FyAgent adapter；
5. 前四项确实不适用时才写最小本地实现，并记录原因。

明确禁止：

- 第二套 HTTP downloader、retry、redirect、cache 或 cancellation；
- 第二套 DMG mount/copy/replace/rollback；
- 复制 Codex installer 后改产品名；
- 复制 OpenCode Electron updater；
- 把 Claude mirror shell/CI 脚本放进客户端；
- 通用 mirror/proxy、任意 URL 或 renderer source selection；
- page-local 产品优先级表；
- 前端自行推导 update eligibility。

### R11 — UX and copy

- QoderWork、TRAE Work、WorkBuddy 未安装时显示“一键安装”；已安装时不出现“一键更新”。
- Claude/OpenCode 只显示一个 desktop component，不显示 CLI component。
- Claude desktop component 的可见名称明确为“Claude Desktop”，避免把 `.app` 误称为 CLI。
- 安装/更新进度复用当前共享百分比、速度、阶段和持久终态。
- source 失败文案简短说明“暂时无法获取安装包”，并提供官方页面，不展示内部 URL、完整路径或诊断堆栈。
- 列表排序更新应在扫描完成后一次发生，不在逐项扫描时持续跳动。

### R12 — Specs and evidence

实现完成后更新至少以下 owning contract：

- Agent Catalog/surface/product action matrix；
- executable software installer 的 Claude/OpenCode source 与 managed product policy；
- frontend Agent directory 的动态排序合同；
- macOS helper 依赖与 `/Applications` HIL 边界。

一次性版本、镜像当前 manifest、下载大小、观察到的签名和 review commit 只留在本任务 research，不复制进长期 spec。

## 5. Non-goals

- 实现 Windows Claude/OpenCode desktop 安装器。
- 恢复或新增 OpenCode/Claude CLI 一键安装。
- 删除用户机器上已经存在的 OpenCode/Claude CLI。
- 删除 Provider、Skills、MCP、模型、会话领域中的 OpenCode/Claude 标识。
- 为 QoderWork、TRAE Work、WorkBuddy 实现 FyAgent 更新或迁移。
- 实现厂商应用自身的自动更新配置。
- 创建 FyAgent 自有 Claude 镜像服务或通用下载代理。
- 绕过 Anthropic 地区、账号、登录或服务政策。
- 在本任务中实现 `/Applications` privileged helper。
- 修改正在实施的前置任务或覆盖其工作树改动。

## 6. Acceptance criteria

### Ordering

- [ ] 初次扫描过程中维持 canonical order，不随单项返回跳动。
- [ ] 扫描完成后，所有 `installed`/`installed_not_runnable` 项位于 unresolved 和 `not_installed` 之前。
- [ ] 已安装 QoderWork、TRAE Work、WorkBuddy 位于其他已安装项之前，三项内部保持 canonical order。
- [ ] 未安装国产项不会越过已安装非国产项。
- [ ] 当前扫描失败但保留旧 installed readiness 的项被视为 unresolved 排序，同时旧配置入口仍按既有 stale-data 合同保持可用。
- [ ] 重扫期间顺序冻结；完成后一次更新。
- [ ] 安装成功后的权威 reread 会把该卡片移动到已安装分组，焦点和链接仍有效。

### Domestic install-only policy

- [ ] 三项未安装时可一键安装；已安装时不展示 update 状态或按钮。
- [ ] 三项 readiness `updateState=unavailable`，candidate `updateEligible=false`，`allowedActions` 无 `update`。
- [ ] 直接提交三项 update action 返回 `action_not_supported`，并证明没有 metadata、download、helper 或 filesystem side effect。
- [ ] 三项已经安装时，readiness 不为更新比较访问远端 source。
- [ ] 三项首次安装继续复用各自现有官方 source 和共享 installer。

### OpenCode

- [ ] OpenCode Agent lifecycle 只接受/呈现 desktop surface；CLI surface/request 被稳定拒绝。
- [ ] Agent Catalog 不再提供 OpenCode CLI 安装链接，仍保留产品/桌面入口。
- [ ] 未安装 OpenCode Desktop 可一键安装；版本较旧时可一键更新；已安装时可“打开软件”。
- [ ] update 使用 official latest metadata + fixed stable DMG + mounted version readback，不调用上游 Electron updater。
- [ ] OpenCode Provider、模型、Skills、MCP 和 session 的现有产品身份未被破坏。

### Claude Desktop

- [ ] Claude Agent lifecycle 只接受/呈现 desktop surface；CLI install/update action 被稳定拒绝。
- [ ] Agent Catalog 删除 CLI 下载链接，desktop 入口指向 Anthropic 官方下载页。
- [ ] 固定 mirror manifest 与 `/latest/mac` 能解析出一个有界 universal macOS release；远端 `url` 不进入下载 capability。
- [ ] 当前受控 fixture/真实 DMG 被识别为 `Claude.app`、Bundle ID `com.anthropic.claudefordesktop`，并从 Info.plist读取版本。
- [ ] Claude Desktop 未安装时可一键安装，版本较旧时可一键更新，安装后可“打开软件”。
- [ ] source/schema/version mismatch、断网、取消和安装失败都有持久终态，不把下载可达性描述为服务可用性。

### Reuse and regression

- [ ] Codex、Claude、OpenCode 和现有 managed desktop 共用现有 downloader/job/progress/DMG transaction。
- [ ] 产品动作权限只有一个 backend owner；国产优先只有一个 frontend product-metadata owner。
- [ ] 没有新增通用 URL/path/command/mirror IPC，没有复制 updater 或 downloader。
- [ ] 相关 Rust、TypeScript、architecture、contract 和 browser tests 通过。
- [ ] macOS Apple Silicon 完成真实 install/update/launch HIL；Intel 资产选择至少有 fixture/contract 证据，无法执行的 Intel HIL明确记录。
- [ ] `/Applications` HIL 只有在独立 helper 任务签名、公证和回滚验收通过后才可标记完成。
- [ ] 除明确的产品级 CLI/update 移除外，Windows 测试保持原有行为；没有新增 Windows 桌面安装能力。

## 7. Planning gates and dependencies

1. **Predecessor interface gate**：实施前重新读取 `08-31-macos-agent-install-update-experience` 的最终 source/surface/job 接口，不基于规划时的未提交文件名机械修改。
2. **System commit gate**：`/Applications` install/update 验收依赖 `08-31-macos-privileged-application-commit-helper`；helper 未通过时保持 `authorization_required`，不得静默改装到用户目录并宣称系统安装成功。
3. **Claude source gate**：实施时重新拉取固定 mirror manifest，确认 schema、版本和 DMG identity；本任务记录的 `1.40609.0` 只是一时证据，不是 pin。
4. **OpenCode metadata gate**：确认最新稳定 GitHub release/tag 与 stable DMG endpoint 的 mounted version 一致；漂移必须刷新重试。
5. **Final approval gate**：本规划任务保持 `planning`，只有用户在看到最终摘要后另行明确批准，才允许进入实现。

## 8. Blocking open questions

无。实现文件边界可能因前置任务继续变化，已明确要求在实施 Phase 0 重新审计；这不改变本文产品行为。
