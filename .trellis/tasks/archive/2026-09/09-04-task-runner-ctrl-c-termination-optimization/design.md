# 任务运行器 Ctrl+C 终止优化技术设计

## 1. 现状分析与根因

1. **Bash 脚本在 `if` 语句内忽略 `set -e`**：
   `scripts/release/build-macos-privileged-helper.sh` 在 `copy_or_lipo_product` 中使用了：
   ```bash
   if swift_build_product "$product" --arch arm64 --arch x86_64; then ...
   ```
   在 bash 规范中，`set -e` 在 `if` 条件中被挂起。当用户在第一轮 universal build 按 `Ctrl+C` 时，子命令返回 130，bash 认为只是 `if` 条件失败，继续执行后方的单架构构建 `swift_build_product --arch arm64` 与 `x86_64`，并产生锁占用与日志泄漏。
2. **`runForeground` 忽略连击 `Ctrl+C` 且退出码写死为 1**：
   在 `scripts/tasks/lib.mjs` 中，`shutdown` 函数通过 `if (shuttingDown) return;` 阻断了后续所有信号监听。如果子进程退出较慢或卡顿，用户按第二次 `Ctrl+C` 毫无反应。同时，子进程因 signal 退出时直接赋值 `process.exitCode = 1`，导致终端包装器（如 pnpm）将正常中断误报为运行异常。

## 2. 模块边界与接口设计

### 2.1 信号退出码映射函数 `signalExitCode`
位于 `scripts/tasks/lib.mjs`：
```javascript
export function signalExitCode(signal) {
  switch (signal) {
    case "SIGHUP": return 129;
    case "SIGINT": return 130;
    case "SIGQUIT": return 131;
    case "SIGKILL": return 137;
    case "SIGTERM": return 143;
    default: return 1;
  }
}
```

### 2.2 `runForeground` 信号处理增强
- 记录 `let receivedSignal = null;`
- 首次信号到达：
  - `receivedSignal = signal;`
  - `shuttingDown = true;`
  - 触发 `killProcessTree(treePid, platform, options.runner, options.posixKill);`
  - 启动兜底定时器（如 3000ms），超时后强制 `killProcessTree` 并以 `signalExitCode(signal)` 退出；
- 二次信号到达（用户连按 `Ctrl+C`）：
  - 检测到 `shuttingDown === true`；
  - 立即再次调用 `killProcessTree`；
  - 直接执行 `process.exit(signalExitCode(signal));`，杜绝卡死在 Node 事件循环。
- `child.on('exit', (status, signal) => ...)`：
  - 清理定时器与信号监听；
  - 正确计算退出码：优先取 `receivedSignal` 或 `signal` 对应的 `signalExitCode`，若为正常退出则取 `status ?? 0`；
  - 触发 `process.exit(exitCode)` 或允许进程优雅结束。

### 2.3 `run`（同步执行器）信号增强
- 检查 `result.signal`：
  - 若为 `SIGINT`，直接以 `process.exit(130)` 退出；
  - 若为 `SIGTERM`，直接以 `process.exit(143)` 退出；
  - 若有其他信号且未指定 `allowFailure`，报错信息中明确指出 `terminated by ${result.signal}` 而非 `exited with null`。

### 2.4 `build-macos-privileged-helper.sh` 脚本 trap
在脚本开头加入：
```bash
trap 'exit 130' INT TERM
```
无论在脚本的任何执行阶段（包括 `if` 条件内、subshell、循环或 lipo），一旦收到中断信号，脚本立即以 130 退出，绝不执行后序构建。

## 3. 风险与回滚方案

- 风险极低：均为任务运行器内部辅助脚本及构建脚本，不涉及业务代码与运行时发布包；
- 若出现非预期行为，可通过 git revert 快速回滚。
