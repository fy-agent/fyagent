# 低资源验收与清理（仅批准后运行）

## 调度：测试可分工，重任务不可抢跑

主协调器独占执行队列、工作树写入和清理权限。子代理按三组分工：A 审查 ACL/目录/UI 并解读 T1/T3/T7 证据；B 核对传输、模型和 WSL/SSH 前提，解读 T2/T4/H2/H3；C 复核 #198 取舍、回归和 macOS 交接。每组只写自己的证据条目，提交 `ID / SHA / RUNNING、PASS、FAIL、BLOCKED / 日志位置 / 下一步`；不能自行启动 Cargo、浏览器、WSL、清理或改源。主协调器收集退出事件和真实日志，不以子代理宣称通过替代命令结果。

实际进程默认 **一个重任务槽**：bootstrap、Cargo 检查/测试、Playwright、整套 `mise run check`、Windows 编译、原生 WSL/SSH 测试、VHDX 压缩彼此不重叠。只读审查可以并行，初始最多两个子代理，加开的第三组排队；资源预警时停止派发，只保留协调器和正在运行的唯一命令。Playwright 聚焦命令追加 `--workers=1`；Rust 以子进程环境 `CARGO_BUILD_JOBS=1`、`RUST_TEST_THREADS=1` 起步，先核对最终树任务封装是否接受/继承，不改全局环境、`NODE_OPTIONS` 或受保护编译器/目标设置，不为降并发绕开 canonical `mise` 门禁。Vitest 当前配置已封顶两个 worker；不与重任务并行。

## 监控与中止规则

每次开跑先记录 60 秒空载的可用物理内存、系统 commit 已用/上限、总 CPU、构建卷与目标 VHDX 卷剩余空间。每 5 秒采样到外置 `resources.csv`，含 UTC、步骤、根 PID、子进程树的工作集与私有内存、系统 CPU、内存、commit、空闲磁盘；机器有运行 WSL 时另记录 WSL 相关占用，区分于本任务 PID。用 Windows 性能计数器/CIM（如 `Win32_PerfFormattedData_PerfOS_Memory` 的 `AvailableMBytes`、`CommittedBytes`、`CommitLimit` 及 processor `_Total`），不依赖本地化的计数器路径。未建立采样或无法追踪本轮进程树时，不派重任务。阈值为保守初始值，按实测向更保守调整，不能把阈值当作 OOM 保证：

| 条件 | 处置 |
| --- | --- |
| 开重任务前：可用内存至少 `max(3 GiB, 20% 物理内存)` 且 commit 使用率低于 80%，构建卷至少有 30 GiB 空闲，并满足产物复制需要。 | 不足则保持队列暂停；记录具体前提，不改页面文件，不杀用户程序。 |
| 至少三次采样内存低于开跑线、commit ≥ 80%，或 CPU 连续 60 秒 ≥ 90%。 | 不再开新任务，记录峰值；CPU 单独高不是 OOM，也不强停有进展的编译。 |
| 可用内存持续两次低于 `max(1.5 GiB, 10%)`、commit 持续两次 ≥ 90%、构建卷低于 10 GiB，或明确 OOM/磁盘满。 | 中止本轮重命令；记录峰值和退出原因。先正常取消；有需要只终止已记录并复核身份的本轮拥有进程树，不按 `cargo`/`node` 名称扫杀，不停别的 WSL/IDE。压缩已经开始时不按普通构建杀树，跟随工具正常结束或交给操作者处理。 |
| 采样失联或多进程共享缓存无法证明所有权。 | 停止派发，核对进程与日志；不得在文件仍打开时删除缓存。降到单作业后仍超线就标 `BLOCKED_RESOURCE`，不自动反复重跑。 |

命令前后记录退出码；最终保存各步骤峰值、告警/中止时间、是否存在孤儿进程。监视器只保留本轮生命周期，测试结束即停。**释放 RAM 靠停进程和结束本轮服务；删除编译缓存与压缩 VHDX 释放的是磁盘。**

## 每次 Windows 编译结束必做

无论成功、失败或人为中止，先等子进程树结束并封存日志；成功时先把本次 `fyagent.exe` 及实际要求的 sidecar/资源复制到独立交付目录，检查复制前后哈希一致，再清理。失败时不得留旧 exe 冒充新产物。记录目标卷起止空闲字节和实际删除路径。

允许清理的对象仅是本次运行创建、经构建输出确认且位于集成 worktree 内的生成目录/文件：常见为 `src-tauri/target`、`src-tauri/user-helper/target`、`dist`、本轮生成的 helper 暂存二进制与浏览器输出，实际以日志和前后清单判定。不清理开始前已存在而无法分离归属的内容，不用 `git clean -xfd`；不动 pnpm store、Cargo 全局 registry/git 缓存、Rust 工具链、`.venv`、`node_modules`、用户 `%TEMP%`、原工作区或共享 target。删除前校验 `Resolve-Path`、路径归属、Git 无跟踪、无 junction/symlink/reparse 重定向且无本轮使用进程。失败或不能证明归属则登记 `CLEANUP_BLOCKED`，保留证据。Rust fixture 的多次 Cargo 调用属于一个门禁批次：不中途删除共用中间产物迫使重编；批次中止或最后一项后，清理只对本轮独占输出。用户要求的体验编译每次均单独封存和清理。

## WSL 测试尾声：只压缩本轮专用发行版

1. 先保存 H2/H3 产物回读、哈希、备份证据到 Windows 外置目录；关闭本轮测试 SSH 服务与 shell；只按本轮清单删除可重建 fixture（不删日常 Linux 数据）。确认以后仍需的测试数据已备份，且复制可读。若空间不足以做必要备份，先停，不能冒险压缩。
2. 确认目标是 WSL2 专用测试发行版而非现有 `Ubuntu-24.04`、Docker 或共享发行版。记录 `wsl --list --verbose`（只读命令）和测试发行版与 VHDX 的唯一映射：优先用创建测试发行版时登记的实际导入路径，必要时参考 [Microsoft 的发行版磁盘定位](https://learn.microsoft.com/en-us/windows/wsl/disk-space#how-to-locate-the-vhdx-file-and-disk-path-for-your-linux-distribution)。记录目标文件大小／实际占盘口径与卷剩余空间。若测试只是在共享发行版中放了路径隔离 fixture，则清理该 fixture，压缩标为人工维护，不触碰共享 VHDX。
3. 在目标发行版内先按权限与文件系统能力作 `sync`，可用时对该测试根文件系统执行受控 `fstrim`；没有支持不做写零填盘。由操作者确认无其他工作后，仅 `wsl --terminate <测试发行版>`，核对目标 `Stopped` 且 VHDX 已脱离/无占用。默认不运行 `wsl --shutdown`；若必须停止整个 WSL VM 才能脱离，或工具需提权，交由操作者在明确获准的维护窗口执行。磁盘路径不唯一、不是动态 VHDX、还在挂载、备份未完成，一律停止。
4. 操作者使用受支持的离线工具。可选 Windows 管理员 DiskPart：`select vdisk file="<已确认的测试 VHDX>"`、`detail vdisk` 核对身份，再 `compact vdisk`（仅动态 VHDX，且 detached 或只读挂载）；不 attach 可写，不 resize/unregister，不全局改 `.wslconfig`。参考 [Microsoft compact vdisk](https://learn.microsoft.com/en-us/windows-server/administration/windows-commands/compact-vdisk)。
5. 记录压缩前后同口径测量、宿主卷释放字节、退出状态；重启**同一个测试发行版**，用普通测试用户核对启动及预留见证文件哈希，再正常结束本轮服务。成功但未显著变小可如实记录；锁占用、需管理员或权限/路径未确认标 `BLOCKED_WSL_COMPACT`，不自动换目标。压缩是维护结果，与功能验证分别计分。
