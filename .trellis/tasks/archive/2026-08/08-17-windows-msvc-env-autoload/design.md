# Windows MSVC 编译环境自动接入 — 技术设计

## 1. 架构与边界

新增一个单一职责模块 `scripts/tasks/windows-msvc-env.mjs`，向两个既有入口提供能力：

```
scripts/tasks/windows-msvc-env.mjs   (新增，纯函数 + 可注入 spawn)
        ▲                    ▲
        │ import             │ import
        │                    │
scripts/tasks/host-native.mjs        scripts/tasks/system-check.mjs
  executeTauriTask                     REQUIREMENTS.win32 的 MSVC 诊断
  executeCargoTask
```

职责分离：

- `system:check`：**静态诊断**——用 vswhere 定位 VS 2022 并校验 VC 工具组件存在，FAIL 给出可操作 hint。不实际编译、不加载环境。
- `host-native`：**运行时加载**——在真正要编译的 `pnpm tauri` / `cargo` 子进程前，通过 `VsDevCmd.bat` 实际解析 MSVC/SDK 环境并注入子进程 env。

两者共享 `windows-msvc-env.mjs` 的 vswhere 定位与组件校验逻辑，避免重复。

## 2. 模块契约（windows-msvc-env.mjs）

全部为纯函数 + 可注入依赖，仅用 Node 内置（`node:child_process`、`node:path`、`node:fs`、`node:process`），不引入 npm 依赖（满足 task-runner 契约的"只用 Node built-ins"约束）。

```js
export const VCTOOLS_COMPONENT = "Microsoft.VisualStudio.Component.VC.Tools.x86.x64";

// 架构映射：process.arch -> { arch, hostArch }
export function msvcArchitecture(architecture) {
  switch (architecture) {
    case "x64":   return { arch: "x64",   hostArch: "x64" };
    case "arm64": return { arch: "arm64", hostArch: "arm64" };
    default: throw new Error(`Unsupported MSVC host architecture: ${architecture}`);
  }
}

// vswhere.exe 候选绝对路径（x64 系统 + arm64 系统的 Installer 目录）
export function vswhereCandidates() { /* string[] */ }

// 定位 VS 2022（含 BuildTools）并校验 VC 工具组件；返回 { installationPath, vcToolsInstalled }
export function findVsInstallation({ spawn = spawnSync } = {}) { /* ... */ }

// 主入口：win32 时定位 VS 并通过 VsDevCmd 解析环境；非 win32 返回 null
export function resolveMsvcEnvironment({
  platform = process.platform,
  architecture = process.arch,
  nodeExecutable = process.execPath,
  spawn = spawnSync,
} = {}) { /* 返回 Record<string,string> | null */ }

// 可操作报错文本
export function msvcRequirementHint() { /* string */ }
```

### 2.1 vswhere 定位

- 候选路径（按顺序尝试，取存在者）：
  - `C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe`（x64 系统）
  - `C:\Program Files\Microsoft Visual Studio\Installer\vswhere.exe`（arm64 系统）
- 查询参数（argv 数组，无 shell）：
  `vswhere.exe -latest -version "[17.0,18.0)" -products * -requires <VCTOOLS_COMPONENT> -property installationPath`
- 输出 trim 后非空 → VS 存在且组件满足；空 → 报 `msvcRequirementHint()`。

### 2.2 VsDevCmd 环境解析

- `VsDevCmd.bat` 路径 = `<installationPath>\Common7\Tools\VsDevCmd.bat`。
- 用 `spawn("cmd.exe", ["/d", "/s", "/c", command], { shell: false, windowsVerbatimArguments: true, encoding: "utf8", windowsHide: true })` 直接调用 `cmd.exe` 可执行文件（**不是** `shell: true`）。
- `command` 手动构造为：
  `call "<VsDevCmd>" -no_logo -arch=<arch> -host_arch=<hostArch> >nul && "<nodeExecutable>" -e "process.stdout.write(JSON.stringify(process.env))"`
  - `process.stdout.write(JSON.stringify(process.env))` 不含内层双引号，避免命令字符串引号嵌套。
  - `windowsVerbatimArguments: true` 使 Node 原样 join argv，配合 `cmd /d /s /c` 的引号规则，保证带空格的 VS/Node 路径正确。
- 解析 stdout 为 JSON → 得到完整环境（含 `INCLUDE`/`LIB`/`LIBPATH`/`PATH` 等）。
- 校验：JSON 解析失败或关键变量（`INCLUDE`、`LIB`）缺失 → 报可操作错误。

## 3. host-native.mjs 接入

### 3.1 注入点与签名

```js
import { resolveMsvcEnvironment as loadMsvcEnvironment } from "./windows-msvc-env.mjs";

export function executeTauriTask({
  // ...现有参数
  loadMsvcEnvironment: loadMsvcEnvironmentFn = loadMsvcEnvironment,
}) { ... }

export function executeCargoTask({
  // ...现有参数
  loadMsvcEnvironment: loadMsvcEnvironmentFn = loadMsvcEnvironment,
}) { ... }
```

### 3.2 执行顺序（关键不变量）

`executeTauriTask`（dev/build/build:binary/build:debug）：
```
assertTauriRequest → assertNoCargoToolchainConfig → resolve rustc/rustdoc
→ probe rustc/rustdoc -vV → planTauriTask → [win32] resolve MSVC env → runCommand(pnpm tauri ...)
```

`executeCargoTask`（check/clippy/test）：
```
assertCargoRequest → validateCargoConfig → resolve rustc/rustdoc → resolve runner
→ probe rustc/rustdoc -vV → planCargoTask
→ [win32] runCommand(node, prepare-windows-user-helper)   ← 纯 Node，不需要 MSVC
→ [win32] resolve MSVC env                                 ← 仅此处加载
→ runCommand(cargo ...)
```

要点：
- MSVC 解析位于 rustc/rustdoc 身份校验**之后**（身份不符时在加载前即失败）。
- 位于 helper prepare **之后**（helper 失败时不会浪费加载）。
- 位于最终 cargo/tauri **之前**。
- macOS 路径完全不调用 `loadMsvcEnvironmentFn`。

### 3.3 环境合并

```js
let commandEnvironment = plan.environment; // ownedCargoEnvironment 输出
if (platform === "win32") {
  const msvcEnvironment = loadMsvcEnvironmentFn({ platform, architecture }) ?? {};
  commandEnvironment = { ...commandEnvironment, ...msvcEnvironment };
}
runCommand(plan.command, plan.args, { env: commandEnvironment });
```

- `ownedCargoEnvironment` 已固定 RUSTC/RUSTDOC、清空 wrapper/flags，MSVC 增量只补 `INCLUDE`/`LIB`/`LIBPATH`/`PATH` 等，不覆盖工具链变量。
- `lib.mjs` 的 `run` 用 `{ ...process.env, ...options.env }`，最终子进程 PATH 是 `process.env.PATH` 与 VsDevCmd 解析出的 PATH 的合并结果（VsDevCmd 的 PATH 是继承当前 PATH 后 prepend VS 目录的超集，故合并安全）。

## 4. system-check.mjs 改动

- win32 `commands` 把 `["where.exe", ["cl.exe"], hint]` 替换为 vswhere 诊断项：
  `["vswhere.exe", ["-latest", "-version", "[17.0,18.0)", "-products", "*", "-requires", VCTOOLS_COMPONENT, "-property", "installationPath"], "Install Visual Studio 2022 Build Tools with the \"Desktop development with C++\" workload."]`
- `probe` 对 `vswhere.exe` 用 `findVsInstallation` 的绝对路径解析（因为 vswhere 不在 PATH）。
- 保持 `--describe-platform` 输出的 `[command, args, hint]` 结构不变，且不含 `sudo`/`apt`/`brew`/`winget`/`choco`、args 不含 `install`/`add`（满足现有契约测试）。
- 仍不引入 `execSync`/`execFileSync`（沿用 lib.mjs `run`）。

## 5. 兼容性、回滚与风险

- **兼容性**：macOS 路径零变化；`host-native.mjs` 新增的是带默认值的可选参数，向后兼容。
- **回滚**：删除/回退 `windows-msvc-env.mjs` 与 `host-native.mjs`、`system-check.mjs` 的对应改动即可恢复旧行为，无状态副作用。
- **cmd.exe 契约例外**：本任务在 `task-runner-contract.md` 中记录——VS 官方唯一加载机制是 `cmd.exe` + `VsDevCmd.bat`，属于受控例外；使用 `shell: false` + argv 数组，不使用 `shell: true`，不拼接 shell 命令字符串。
- **Windows 引号/编码**：`windowsVerbatimArguments: true` + 手动构造命令 + Node dump JSON，规避 `set` 文本编码歧义；非 ASCII 路径由 JSON.stringify 的 UTF-8 输出保证。
- **证据边界**：本机（macOS）只能跑纯逻辑单测，真实 Windows 环境加载/编译需 Windows native Actions runner 证明；本任务在纯测试中注入 mock spawn，不依赖真实 VS。
