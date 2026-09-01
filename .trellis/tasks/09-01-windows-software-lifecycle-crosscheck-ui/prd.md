# 完善跨平台 AI 软件生命周期与开发检查

## 0. 范围

- 优先级：P0
- 类型：fullstack / developer-experience
- 产品平台：Windows 与 macOS；Linux 仅作为开发宿主。
- 本任务处理 AI 软件生命周期，不处理 FyAgent 自身安装或更新。
- Windows 原生 CI/HIL 仍是 Registry、UAC、vendor installer、签名、启动和自定义安装路径的最终证据；macOS cargo-xwin 只提供早期编译诊断。

## 1. 已确认问题

1. Windows App Paths/Uninstall inventory parent 以 query-only 权限打开后执行子键枚举，真实环境会失败并把完整状态降为 `unknown`。
2. QoderWork、TRAE Work、WorkBuddy 已有官方 source、下载、制品验证、精确目标、平台事务和后置回读，但 `lifecycle_policy.rs` 错误设置 `update=false`。
3. macOS 开发者缺少初始化阶段可见、但不污染普通 bootstrap 的 Windows-MSVC 前置检查。
4. 左侧导航 host 与 SelectionLens 同时绘制 active frame，折叠时形成双层外框；扫描重排时 width/height spring 造成边缘闪烁。

## 2. 需求

### R1. Windows inventory

- inventory parent 使用 query-value + enumerate-subkeys 的最小只读权限。
- 中间组件 traversal-only，枚举 child query-value-only；不得获得 create/set/delete/security-write。
- optional parent 缺失表示无记录；访问、枚举、链接、边界或 Shell-context 错误保持 incomplete/`unknown`。
- complete + no trusted candidate 投影为 `not_installed` 并恢复 reviewed fresh destination。
- 不新增 WinGet、PowerShell、进程名检测、全盘扫描或第二套 inventory owner。

### R2. AI 桌面软件一键安装与更新

- QoderWork、TRAE Work、WorkBuddy Desktop 支持 `install | update | launch`。
- update 只在唯一可信候选、candidate update-eligible、官方 source 可解析、远程版本不同、opaque target/release capability 完整时暴露。
- install 绑定 fresh destination；update 必须绑定 existing candidate。缺失、过期、变化或歧义目标在下载/写入前失败。
- macOS 复用 exact-path DMG staging/rollback；Windows 复用 verified EXE、closed helper selector、vendor UI/UAC 与 fresh inventory readback。
- 不猜 silent installer switch，不新增 vendor updater，不让 renderer 提供 URL、路径、命令、hash 或 bypass。

### R3. macOS Windows-MSVC 诊断

- `bootstrap` 运行 read-only `system:check:windows-msvc-cross:advisory`；缺少可选工具时报告全部问题但退出 0，非 macOS 明确 SKIP。
- strict `system:check:windows-msvc-cross` 缺项时退出非零且不安装任何内容。
- `rust:clippy:windows-msvc-cross` default-no，固定 cargo-xwin 版本、Windows x64 target、clang-cl、xwin toolset、workspace/all-targets/locked manifest 与 `-D warnings`。
- strict/Clippy 不进入 bootstrap；整个 family 不进入 default check、CI 或 Release gate。

### R4. 左侧导航

- 展开状态保留活动语义；折叠活动组只显示一层共享 SelectionLens。
- collapsed hover/active 不恢复 host frame；focus、ARIA、Router、键盘和 reduced-motion 不变。
- SideNavigation 使用 position-only geometry：位置 spring，尺寸立即同步；其他调用方默认完整 geometry 不变。
- 不新增第二个 lens、动画库或页面本地状态机。

### R5. 复用与规范

- 修复既有 Registry、Agent lifecycle、平台 transaction、SelectionLens 和 mise owner。
- 不新增生产依赖。
- 归档前更新 owning SPEC，并诚实记录 Windows native HIL 尚属 release evidence。

## 3. 非目标

- 不猜 Claude/OpenCode Windows identity、source、PFN/AUMID 或安装路径。
- 不修改 Codex 专用 Desktop/PackageManager owner。
- 不自动安装 LLVM、CMake、Ninja、Rust target、cargo-xwin 或系统包。
- 不把 cargo-xwin 成功描述为 Windows 原生验收。
- 不新增 WinGet/Chocolatey/Docker MSVC image/Zig GNU ABI/本地 SDK downloader。

## 4. 验收

- [x] Registry parent query+enumerate、child query-only、无 write capability。
- [x] complete/no candidate → `not_installed` + fresh destination；incomplete → `unknown`。
- [x] 三款 managed desktop policy 允许 install/update/launch。
- [x] newer source + single update-eligible target 才暴露 update；缺目标在下载前失败。
- [x] bootstrap advisory 成功报告缺项；strict/Clippy 维持严格边界。
- [x] SideNavigation 单层 lens 与 position-only geometry 有 unit/browser contract。
- [ ] focused/full prearchive gates 通过。
- [ ] owning SPEC、任务归档与 journal 提交完成。
- [ ] Windows 原生 HIL 作为后续 release evidence：真实已安装/未安装、UAC/vendor UI、自定义路径和启动。
