# 优化任务运行器 Ctrl+C 终止与进程树清理机制

## Goal

彻底优化 `mise run dev` 等任务在收到 `Ctrl+C` 时的响应速度与进程树清理机制：
1. 解决前置构建（macOS Swift Helper 等）在收到 `Ctrl+C` 后仍继续跑后续流程乃至留下孤儿进程（占住 `.build-development` 锁）的问题；
2. 解决 `runForeground` 拦截 `SIGINT` 后连按第二次 `Ctrl+C` 被忽略、无法强制退出的问题；
3. 规范化中断退出码为标准 POSIX `130`（避免 pnpm 报 `ELIFECYCLE exit code 1` 假崩溃）；
4. 确保在发生信号中断时能够快速、彻底终止整个子进程树。

## Requirements

1. **第二次 Ctrl+C 强制退出（Force Kill & Immediate Exit）**：
   - 当交互式任务（`runForeground`）收到中断信号（`SIGINT`/`SIGTERM`）进入 shutdown 流程时，若在子进程退出前再次收到 `Ctrl+C`，不得静默忽略，应立即强杀进程树并以退出码 `130` 强制退出 Node 进程。
   - 增加安全兜底超时（如 3000ms），防止异常子进程忽略 `SIGTERM`/`SIGKILL` 导致终端永久挂起。
2. **前置构建脚本中断信号捕获（Trap SIGINT/SIGTERM）**：
   - `build-macos-privileged-helper.sh` 增加 `trap 'exit 130' INT TERM`，确保在 `if swift_build_product ...; then` 条件分支中被中断时立即终止，不得吞掉信号继续执行后续架构编译与打包，避免留下孤儿进程。
3. **同步运行器（`run`）信号响应与退出码对齐**：
   - 当 `run` (`spawnSync`) 被中断信号终止（`result.signal` 为 `SIGINT`/`SIGTERM` 等）时，标准退出（`SIGINT` → 130，`SIGTERM` → 143），不再抛出模糊的 `exited with null` 异常并退出为 1。
4. **规范化信号退出码映射**：
   - 提取并共享 `signalExitCode(signal)` 映射逻辑（`SIGINT` → 130，`SIGTERM` → 143，`SIGHUP` → 129 等），在子进程由信号杀死或父进程收到中断时，赋予标准退出码，避免终端包裹层（如 pnpm）报错。
5. **跨平台兼容与现有测试契约保持**：
   - 保持 macOS/Linux POSIX 进程组 `-pid` 信号机制与 Windows `taskkill.exe /pid /t /f` 语义一致；
   - 现有契约测试 `tests/miseTaskContract.test.ts` 及所有 tasks 契约测试全部保持绿色通过，并补充针对本次优化的单元测试用例。

## Acceptance Criteria

- [x] `build-macos-privileged-helper.sh` 包含 INT/TERM 信号 trap，在任何一步被 Ctrl+C 中断时立即以 130 退出，不继续执行后续构建；
- [x] `runForeground` 在首次 `SIGINT` 后若再次收到 `SIGINT`，立即终止进程树并以 130 退出；
- [x] `runForeground` 收到信号中断后，最终退出码为对应的 signalExitCode（Ctrl+C 为 130）；
- [x] `run()` 在子进程被信号中断时，返回或处理退出码为标准信号退出码；
- [x] 补充针对连续中断信号与 signalExitCode 的契约与单元测试；
- [x] `mise run check:contracts` 与 `mise run test:unit` 全部通过。
