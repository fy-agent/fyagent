# 完善跨平台 AI 软件生命周期与开发检查

## 0. 状态与范围

- 优先级：P0
- 类型：fullstack / developer-experience
- 主问题：Windows 桌面 Agent inventory 的注册表父键权限不足，可能把完整的未安装状态错误降级为 `unknown`；macOS 缺少受控的 Windows MSVC Clippy 早期诊断；左侧导航折叠与 Agent 扫描期间存在重复选中材质和尺寸拖影。
- 证据边界：Windows 原生 CI/HIL 仍是 Windows 注册表、安装器、UAC、启动和发布行为的权威。macOS cargo-xwin 只提供编译诊断。
- 产品边界：QoderWork、TRAE Work、WorkBuddy 保持 `install + launch`，不开放 FyAgent `update`；Codex 保持专用桌面 owner；Claude/OpenCode Windows 身份没有闭合证据时继续 fail-closed。

## 1. 需求

### R1. Windows inventory 最小权限修复

- Uninstall/App Paths 父键使用只读 query + enumerate 权限，不得获得 create/set 权限。
- 枚举后的子键继续 query-only，并保持逐组件 registry-link 拒绝。
- 可选父键不存在属于“无记录”；访问、枚举、边界或 Shell 用户上下文错误仍使 aggregate incomplete/`unknown`，不能伪报未安装。
- 完整且无可信候选时投影 `not_installed`，并保留现有 fresh-install destination。
- 不新增扫描器、包管理器、下载器、签名器或安装状态机。

### R2. 显式 Windows MSVC 交叉诊断

- 新增只读 `system:check:windows-msvc-cross`，仅在 macOS x64/arm64 上检查固定前置条件。
- 新增 default-no、`dependency-environment` 的 `rust:clippy:windows-msvc-cross`。
- target 固定 `x86_64-pc-windows-msvc`；cargo-xwin 固定评审版本；最终 argv 固定 workspace/all-targets/locked manifest 与 `-D warnings`。
- 拒绝任意透传参数以及 caller Rust/C/CMake/xwin/Cargo-config 覆盖。
- preflight 不安装或下载任何内容；Clippy 只在显式确认后允许 cargo-xwin 下载/cache CRT/SDK。
- 两个任务都不进入 bootstrap、`check`、`check:backend`、CI 或 Release gate。

### R3. 左侧导航稳定性

- 展开状态保留配置组的上下文选中材质。
- 活动配置叶折叠后，toggle 清除自身 border/background/shadow，只保留一枚共享 SelectionLens；文字与 caret 使用更弱色阶。
- SideNavigation 使用 position-only geometry：`left/top` 保持可中断 spring，`width/height` 直接同步。
- 其他 SelectionLens 调用方默认 full geometry 行为不变。
- Router、ARIA、键盘、reduced-motion、overlay identity 不回退。

### R4. SPEC 与诚实状态

- 更新 task-runner、development-environment、external-agent、V2 shell owning specs。
- 归档记录区分自动化验证与未执行的真实 Windows HIL。
- 不扩展产品身份或更新策略。

## 2. 非目标

- 不启用 QoderWork、TRAE Work、WorkBuddy 的 FyAgent update。
- 不为 Claude/OpenCode 猜测 Windows ProductName、signer、PFN/AUMID 或安装路径。
- 不把 CRT/SDK 或系统工具安装放进 bootstrap/default check。
- 不修改 Codex PackageManager/MSIX owner。
- 不用 WinGet、PowerShell、全盘扫描或第二枚 slider 掩盖根因。

## 3. 验收标准

### A. Windows inventory

- [x] 父键精确 query + enumerate，无 create/set。
- [x] 子键 query-only，registry-link 安全合同保持。
- [x] 单元测试覆盖 enumerable parent、complete/no-candidate 与 incomplete/unknown。
- [x] 三款国内产品 update policy 保持关闭。

### B. 交叉诊断

- [x] strict preflight 覆盖 cargo-xwin、Clippy、Rust target、LLVM/LLD/llvm-lib、CMake、Ninja。
- [x] 固定 host/target/version/argv；拒绝参数、环境和 Cargo-config 覆盖。
- [x] preflight 无安装/下载；Clippy default-no 并提示许可边界。
- [x] bootstrap/default check DAG 不包含 cross tasks。

### C. 前端

- [x] collapsed active toggle 只有一枚 frame owner。
- [x] SideNavigation 使用 position-only geometry，宽高直接同步。
- [x] unit/V2/browser 测试覆盖展开、折叠、ARIA、键盘、reduced-motion 与扫描几何稳定性。

### D. 质量与归档

- [ ] 完整 prearchive gate 通过。
- [x] owning SPEC 已更新。
- [x] task context JSONL 校验通过。
- [ ] work commit → archive commit → journal commit 顺序完成。

## 4. 未执行的原生证据

- 本机未执行真实 Windows x64/ARM64 安装器、UAC、注册表和桌面启动 HIL。
- macOS cargo-xwin 结果不得描述为 Windows 原生支持证明。
