# Root cause and reuse review

Date: 2026-09-01

## Windows inventory

仓库已有完整 owner：`agent_install/windows.rs` 负责 known roots、App Paths、Uninstall、32/64 Registry view、Win32 version、PE architecture、stable file identity、WinVerifyTrust 与 signer；`inventory.rs` 负责完整度、候选归一化、opaque target 和 fresh destination。

确认根因：inventory parent 以 query-value-only 权限打开后调用子键枚举。Microsoft 的 `RegEnumKeyEx` 合同要求 `KEY_ENUMERATE_SUB_KEYS`。修复 existing Registry adapter 的 query+enumerate 只读 capability；不引入 WinGet、PowerShell、全盘扫描或第二套 inventory。

Primary sources:

- https://learn.microsoft.com/windows/win32/api/winreg/nf-winreg-regenumkeyexw
- https://learn.microsoft.com/windows/win32/sysinfo/registry-key-security-and-access-rights

## Managed desktop update reuse

仓库已经有一套闭合 install/update transaction：first-party source adapter、opaque release capability、exact inventory target、streaming download、EXE/DMG verification、Windows helper 或 macOS staged replacement，以及 authoritative post-install readback。前端也只从 backend `allowedActions` 与 `update_available` 派生动作。

系统性阻断是 `lifecycle_policy.rs` 对 QoderWork、TRAE Work、WorkBuddy 设置 `update=false`。正确方案是开放 existing owner，而不是新增 updater 或猜 silent installer switches。

2026-09-01 重新查询官方 metadata，确认现有 parser 仍匹配 live schema：QoderWork `0.9.15`、TRAE Work CN `2.3.79533`、WorkBuddy Windows `5.4.7.37521366`。这些值仅为调研证据，不能成为 pinned runtime fallback。

## Cross MSVC Clippy

普通 Rust Windows target 不提供 Microsoft CRT/SDK 与 native C/C++ toolchain。采用维护中的 cargo-xwin/xwin 复用 sysroot、cache 和 Cargo/Clippy 接线；拒绝 GNU/Zig ABI、`cross` MSVC image、自写 SDK downloader 与 bootstrap 自动安装。

Primary sources:

- https://github.com/rust-cross/cargo-xwin
- https://github.com/Jake-Shadle/xwin

设计为 bootstrap advisory、strict read-only preflight 和 default-no Clippy 三层。advisory 不安装工具、不运行 Clippy、不进入 default check；strict/Clippy 不进入 bootstrap。

## Frontend reuse

仓库已有一枚 SelectionLens、一个 Motion owner、一个 spring token 与 SideNavigation state owner。外框来自 host/lens 双层材质，闪烁来自布局重测时 width/height spring。使用 position-only geometry 和 state-specific host material，不新增动画库、第二枚 lens 或页面状态机。

Motion layout guidance:

- https://motion.dev/docs/react-layout-animations

## Dependency and evidence decision

不新增生产依赖。所有 runtime/frontend 工作扩展现有 owner。Portable/macOS checks 能证明 policy、parser、argv、compile contract 与 UI；不能证明真实 Registry、Explorer SID、UAC/vendor UI、当前 Authenticode、自定义路径或桌面启动，这些仍是 Windows native HIL。
