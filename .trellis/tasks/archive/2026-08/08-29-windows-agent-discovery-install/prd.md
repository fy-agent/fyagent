# Stage 3 — Windows Agent 发现与一键安装

## Goal

基于 Stage 1 的安装目标合同，为 Windows 上的 QoderWork CN、TRAE Work CN 和 WorkBuddy 提供：

- 多来源、可解释、可去重的安装 inventory；
- user/system/custom/packaged 安装的显式候选；
- 受控下载、签名验证和 interactive-user 安装器启动；
- UAC/安装器 UI/取消/失败的真实状态；
- 安装后重新枚举，不以进程启动或退出码冒充成功。

## Requirements

### 1. Windows installation inventory

建立一个共享 `WindowsInstalledAppInventory` owner，按 Stage 1 证据接口提供候选。至少组合：

1. interactive Shell user 的 Uninstall registry；
2. machine Uninstall registry；
3. 32 位和 64 位 registry view；
4. per-user / machine App Paths；
5. PackageManager/MSIX identity（适用时）；
6. 产品审查过的 known roots；
7. 文件 identity/version readback。

约束：

- 当前进程可能 elevated 且不是 Explorer 用户，per-user 读取必须绑定冻结的 interactive-user SID，而不是直接使用进程 HKCU。
- Registry location 是代码内闭集，沿用/扩展 `windows_runtime::registry` 的 component-by-component、symbolic-link-safe 只读遍历；不得让 renderer 或 registry 内容提供下一段任意 key path。
- 明确访问 WOW64 32/64 view；一个 view 失败不能把另一个 view 的结果误写为完整 inventory。
- App Paths 和 Uninstall 记录是证据，不是唯一真相。缺失文件、错误 product identity、错误 signer 或 stale record 返回 non-actionable observation。
- MSIX 仅在产品存在受审查的 package identity 时使用 PackageManager。
  2026-08-30 的三款当前来源均为 EXE，未发现可绑定的 PFN/AUMID，因此
  不得为了“覆盖 MSIX”而猜测 PackageManager 身份；Codex 现有 MSIX
  inventory/helper 回归保持独立绿色。未来产品出现真实 MSIX 来源时再接入。
- Known paths 是最后一层受控证据，不进行全盘扫描、WMI 产品枚举或递归搜索所有驱动器。
- EXE identity/version 使用 Windows Version APIs（GetFileVersionInfo/VerQueryValue 或现有可信封装），不继续把 PE 首尾 UTF-16 字符串扫描作为生产权威。
- 多来源指向同一 canonical executable/package 时合并；多份真实安装全部保留。

### 2. Product/source descriptors

- `ResolvedDesktopSource` 或后继描述符必须明确：package format、architecture、installer scope、interaction mode、expected product identity 和 source provenance。
- 当前 QoderWork resolver 指向 `Setup-User-x64.exe`，只能声明 user-scope interactive installer；不能在 UI 中冒充 System Installer。
- 若后续加入 Qoder System Installer，必须有独立受审查 endpoint/source ID/scope/elevation 合同。
- WorkBuddy 官方安装向导允许选择安装路径，因此默认属于 `vendor_interactive_chooses_destination`；FyAgent 不得发明 undocumented silent switches。
- TRAE Work 也只能使用官方当前 resolver 已证明的 installer mode/scope；未知字段保持 unknown/manual。
- 不支持的 architecture 明确 disabled；不能用 x64 安装包冒充 arm64 支持。

### 3. Download and installer admission

- 复用 Codex Desktop 的 bounded streaming download、private temp/job、retained artifact capability 和 cancellation；不得把大型 EXE 全量读入 `Vec<u8>` 再交给一个可变路径。
- 下载 URL/redirect 必须保持产品 allowlist、HTTPS、无 userinfo/query credential 和 bounded redirect。
- 使用 WinVerifyTrust 或现有可信封装验证本地 PE Authenticode；expected signer/publisher 必须通过真实官方 installer/HIL 研究冻结。
- Registry `Publisher`、文件 description 或下载 host 不能单独代替签名验证。
- 签名缺失、链不可信、publisher 不符、文件被替换或 retained capability 失效时零执行。
- 不把远端 metadata hash 当作唯一 admission；若使用 hash，必须绑定本次实际下载并与签名/来源政策共同使用。

### 4. Closed installer execution

- 扩展现有 Windows interactive-user/helper 边界，而不是新增任意 `ShellExecute(path,args)` IPC。
- Renderer 只提交 Stage 1 target 和 release IDs；backend 将它们解析为固定 product/package capability。
- Helper 协议只能执行闭集 installer action，持有 parent 传递的受保护 package bridge/pin；不得接受 renderer path、command、working directory、verb 或自由参数。
- EXE installer 通过 interactive Explorer/ShellExecuteEx 或经审查的等价路径启动；需要 elevation 时由 Windows UAC 决定，用户取消映射为专用 reason。
- 若使用 ShellExecuteEx，要求可观察 process handle；无法获得 process/terminal outcome 时返回 unknown/incomplete，不能立即 success。
- 当前产品默认以 vendor UI 模式运行。UI 文案应为“打开安装向导并等待完成”，不是“后台全自动安装”。
- Silent args 只有在官方文档、版本兼容性和真实 HIL 证明后才能作为产品闭集策略加入。
- Vendor installer 启动后，FyAgent 不强杀安装器；“取消等待”和“取消安装”语义必须区分。commit/外部进程启动后取消按钮只能在有可靠取消合同的 adapter 中出现。

### 5. Post-install and update semantics

- Installer exit 0 只是信号；最终必须刷新 Stage 1 inventory。
- 安装成功要求出现一个与产品 identity、目标 release、scope/interaction 结果一致的 trusted candidate，并且可执行文件/包 identity 可重新打开验证。
- Exit 0 但没有目标候选、出现多个新候选、版本不符或旧目标消失时返回 incomplete/verification failed。
- 对 vendor-owned interactive installer，FyAgent 无法保证原子回滚时必须在预览中诚实标注；不能复用 macOS managed rollback 文案。
- Update 只有在产品 installer 能够可靠绑定/保持选中候选时才标记 managed update。否则降级为 assisted vendor update，并在结束后明确比较原 candidate 与新 inventory。
- 系统安装、user 安装和 custom 安装不能因同名自动合并或覆盖。

### 6. Shared frontend/public components

- 复用 Stage 1 target picker 和 shared lifecycle status surface。
- 如 vendor installer 引入 `awaiting_user/external_process_running` 阶段，扩展一个共享 install job stage view；不要在三个 Agent 卡片中复制进度/取消/重试 JSX。
- 状态组件只渲染 typed state。下载安装、签名、helper、UAC 和 inventory 逻辑属于 backend。
- Windows candidate 来源/冲突以用户可理解的 scope、version、location label 和 disabled reason 展示，不暴露 registry key、SID、raw path 或 signer diagnostics。

### 7. Same-domain defect policy

测试中发现 Windows detection precedence、stale registry、PATH shadow、parallel install、UAC、helper、download、signature、installer wait 或 post-install readback 缺陷时，在本任务内修复。FyAgent 自身 Authenticode 发布仍由 #68 跟踪。

## Non-goals

- 不做全盘扫描、Win32_Product/WMI repair-triggering 枚举或任意 package manager 执行。
- 不支持未出现在 verified source descriptor 中的 MSI/EXE/MSIX 格式；架构可扩展，但本任务只实现当前产品真实格式。
- 不发明 NSIS/Inno/Electron silent switches。
- 不读取 vendor credential/profile 文件来辅助发现。
- 不把 FyAgent 自身安装器签名、更新器和发布渠道并入本任务。
- 不在没有 arm64 官方来源时声明 Windows arm64 Agent 安装支持。

## Acceptance Criteria

- [x] Inventory 覆盖 interactive-user/machine Uninstall、32/64 registry views、App Paths 和 known paths，并保留来源与 partial-failure 状态；当前三款 EXE 明确记录 PackageManager 不适用，Codex MSIX 回归保持绿色。
- [x] Elevated FyAgent 仍读取 Explorer 用户的 per-user 安装，不误读管理员进程 HKCU。
- [x] Registry symbolic link、oversized/unexpected values、控制字符和非绝对路径 fail closed。
- [x] 同一 EXE 的 registry + App Paths + known-path 证据合并；不同 scope/custom path 候选不合并。
- [x] Stale registry、missing executable、product mismatch、version resource malformed 和 signer mismatch 都不可执行。
- [x] Production identity 使用 Win32 version/signature API，不依赖 PE UTF-16 字符串窗口扫描。
- [x] Qoder User installer 不显示为 system installer；unsupported architecture 明确 disabled。
- [x] 下载使用 retained file capability，替换同路径文件/符号链接/bridge drift 会在执行前拒绝。
- [x] Authenticode trust/publisher 不匹配零执行，错误输出不泄露用户路径或证书原始信息。
- [x] Installer 由冻结 interactive user 启动；上下文漂移、UAC cancel、helper identity mismatch 和无 process handle 使用不同 closed reason。
- [x] Vendor UI 模式显示 awaiting-user/external-process-running，不在启动瞬间 success。
- [x] Exit 0 后必须 inventory readback；未发现、版本不符、多个新候选或 scope 漂移不报告成功。
- [x] User/system/custom 与 parallel install fixtures 都要求显式目标选择。
- [ ] Windows x64 HIL 覆盖 QoderWork、TRAE Work、WorkBuddy 的未安装、已有安装、更新、取消、UAC 拒绝、自定义目录、stale registry 和卸载后残留。
- [x] 对每个产品/architecture 给出 `managed | assisted | manual | unsupported` 结论，不以另一个产品的证据代替。
- [x] Stage 1、Codex Desktop Windows、interactive-user、registry 和 helper 安全回归全部通过。

## Dependencies

- Stage 1 target-authority contract.
- #68 remains a separate release dependency for FyAgent's own Windows artifacts.
