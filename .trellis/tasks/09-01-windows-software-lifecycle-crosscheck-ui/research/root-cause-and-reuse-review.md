# Root cause and reuse review

Date: 2026-09-01

## Windows inventory

仓库已有完整 owner：`agent_install/windows.rs` 负责已知路径、App Paths、Uninstall、32/64 registry view、Win32 version resource、PE architecture、Authenticode、signer 与稳定文件身份；`inventory.rs` 负责完整度、候选归一化、opaque target 和 fresh destination。

根因是 inventory parent 以 query-only 权限打开后调用子键枚举。修复应在 `windows_runtime/registry.rs` 的 access mask owner 中增加 query+enumerate 的只读 capability；不需要 WinGet、PowerShell、全盘扫描或第二套 inventory。

## Cross MSVC Clippy

普通 Rust Windows target 不包含 CRT/SDK 和 C/C++ 构建环境。评审结论：

- 采用 cargo-xwin/xwin：复用成熟的 CRT/SDK sysroot、缓存和 Cargo/Clippy 接线。
- 拒绝 GNU/Zig ABI：目标 ABI 不同。
- 拒绝自写 SDK downloader：重复许可、布局、缓存和工具链维护。
- 拒绝默认 bootstrap 安装：可选诊断不应拖重所有开发者环境。

因此 preflight 与可能写依赖缓存的 Clippy 分离；前者只读，后者 default-no。

## Frontend reuse

仓库已有 `SelectionLens`、共享 spring、`Collapsible` 与 SideNavigation 状态 owner。修复只增加一个窄 geometry mode 和状态化材质，不引入新动画库、第二个 slider 或页面本地状态机。

## Dependency decision

不新增生产依赖。cargo-xwin 是显式开发者前置；运行时、后端和前端都扩展现有 owner。
