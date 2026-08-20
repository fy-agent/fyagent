# Windows MSVC 编译环境自动接入

## Goal

让 Windows 开发者无需手动打开 VS 2022 Developer PowerShell、也无需把 MSVC/SDK 变量永久写进系统 PATH，就能用仓库统一命令完成本地原生开发与构建。启动器在需要编译时按需、仅对子进程加载 VS 2022 MSVC/SDK 环境，并把错误的 `system:check` 诊断修正为可操作的 vswhere 诊断。

## 背景与确认事实

- 仓库用 mise 统一管理 Node（24.19.0）、pnpm（10.12.3）、Rust（1.97.1）、uv。这些工具已在 PATH 上，不依赖 VS 环境。
- MSVC（`cl.exe`/`link.exe`）、`INCLUDE`/`LIB`/`LIBPATH`、Windows SDK 属于系统级组件，**不可**由 mise 安装，也不应永久写进系统 PATH（会被 VS/SDK 更新打破、多版本冲突）。
- 当前普通 PowerShell 未加载 VS 临时开发环境，导致 `where cl.exe` 失败；但设备已装 VS 2022 Build Tools 17.14 + MSVC x64 工具集 14.44.35207 + Windows SDK，用 `VsDevCmd.bat` 临时加载后 `mise run system:check` 全通过。
- 现有 `scripts/tasks/host-native.mjs` 是唯一的原生编译/构建入口（`pnpm dev`/`pnpm build` → `host-native.mjs`；`rust:check`/`clippy`/`test` → `rust.mjs` → `executeCargoTask`），它固定当前宿主 target、校验 rustc/rustdoc 身份、拒绝调用者注入 target/linker/wrapper/runner。
- 现有 `scripts/tasks/system-check.mjs` 在 win32 下用裸 `where.exe cl.exe` 探测，是错误 FAIL 的来源。
- spec `backend/development-environment.md` 与 `backend/task-runner-contract.md` 是本次变更的权威边界；`task-runner-contract.md` 已声明"不引入 cmd.exe / shell: true / 命令字符串拼接"，本任务需为该契约明确一个受控例外（VS 官方唯一加载机制是 `cmd.exe` + `VsDevCmd.bat`）。

## Requirements

- R1：新增 `scripts/tasks/windows-msvc-env.mjs`，通过官方 `vswhere.exe` 定位 VS 2022（含 Build Tools），校验 `Microsoft.VisualStudio.Component.VC.Tools.x86.x64` 组件存在。
- R2：同一模块通过 `VsDevCmd.bat -no_logo -arch=<arch> -host_arch=<hostArch>` 仅为当前子进程解析 MSVC/SDK 环境（`INCLUDE`/`LIB`/`LIBPATH`/`PATH` 等），返回 env 对象，**不修改 `process.env`、不写系统/用户环境、不写磁盘/注册表**。
- R3：`host-native.mjs` 的 `executeTauriTask` 与 `executeCargoTask` 在 `platform === "win32"` 时，于 rustc/rustdoc 身份校验之后、最终 `cargo`/`pnpm tauri` 子进程启动之前，解析一次 MSVC 环境并合并进子进程 env。
- R4：`arch`/`host_arch` 由 `process.arch` 推导（x64→x64/x64，arm64→arm64/arm64），不得硬编码。
- R5：`system-check.mjs` 的 win32 检查把 `where.exe cl.exe` 替换为 vswhere 静态诊断（定位 VS 2022 + VC 工具组件），FAIL 时给出"安装 Desktop development with C++ 工作负载"的可操作 hint。
- R6：找不到 VS 或组件缺失时，报错/提示必须明确指向安装 `Desktop development with C++ / MSVC x64-x86 build tools`，而非只报"找不到 cl.exe"。
- R7：不破坏现有安全边界：`assertNoCallerTargetOverride`、`ownedCargoEnvironment`（固定 RUSTC/RUSTDOC、清空 wrapper/flags）、`assertNoCargoToolchainConfig` 全部保留；MSVC 环境是纯增量，绝不覆盖 rustc/target/linker/runner。
- R8：`rust:fmt` / `rust:fmt:check` 不加载 MSVC（rustfmt 不编译）。

## Acceptance Criteria

- [ ] `scripts/tasks/windows-msvc-env.mjs` 存在，导出纯函数（架构映射、vswhere 候选路径、env 解析、错误信息），可注入 spawn，测试可在非 Windows 主机上运行。
- [ ] `executeTauriTask` / `executeCargoTask` 仅在 `platform === "win32"` 时调用 MSVC 解析，且发生在 rustc/rustdoc 校验之后、最终子进程之前；macOS 路径不触发、调用序列不变。
- [ ] Windows Rust 任务的调用序列为：validate cargo-config → resolve rustc/rustdoc/runner → probe rustc/rustdoc → helper prepare → resolve MSVC env → run cargo。
- [ ] `system:check` 在 Windows 下用 vswhere 诊断 MSVC，`--describe-platform` 输出仍为 `[command, args, hint]` 三元组且不含 elevation / package-manager 命令。
- [ ] `mise run check` 全绿；`developmentEnvironment.test.ts`、`miseTaskContract.test.ts`、`systemCheck.test.ts`、`localBuildBoundary.test.ts` 及新增 MSVC 测试通过。
- [ ] 开发者在 Windows 上的命令边界保持不变：`mise trust` → `mise run bootstrap` → `mise run system:check` → `mise run dev`。
- [ ] spec `backend/development-environment.md` 与 `backend/task-runner-contract.md` 更新，记录 MSVC 环境加载边界与 cmd.exe 受控例外。

## Out of Scope

- 不让 mise 安装/管理 Visual Studio 或 Windows SDK（系统级、带组件选择与提权边界）。
- 不永久修改系统/用户 PATH、INCLUDE、LIB 等环境变量。
- 不支持非宿主 OS/架构的交叉编译（继续由 native GitHub Actions 承担）。
- 不改变 `pnpm tauri` 低层 leaf 的定位；不拦截手写 `cargo`/`rustc` 命令。
- 不锁定具体 MSVC 工具集版本（如 `-vcvars_ver=14.44.35207`）；只要求组件存在，由 VS 自选默认工具集。

## Key Decisions

- MSVC 环境加载是 `task-runner-contract.md` 中"不引入 cmd.exe / shell: true / 命令字符串拼接"的**唯一受控例外**：用 `shell: false` 直接 spawn `cmd.exe` 可执行文件 + argv 数组，不使用 `shell: true`，不做 shell 命令字符串拼接。
- 环境加载通过 Node 子进程 dump `process.env` 为 JSON 来解析，避免解析 `set` 文本的编码/引号歧义。
- 环境只作用于当前 spawn 的子进程，进程内解析结果仅缓存一次，不落地。

## Risks

- Windows cmd.exe 引号/编码处理需在实现时重点验证（含非 ASCII 路径）；本机（macOS）无法做真实 Windows 运行验证，需依赖 Windows native Actions runner 作为证据闭环。
- Windows x64/ARM64 的 `-host_arch` 组合行为需在 Windows runner 上验证。
