# 本地执行记录

- 候选分支 `integration/three-open-prs`，独立 worktree `../fyagent-three-pr-review`。
- #443 本地合并 `57f8f969d98fc66d1993f1e59c237c05688007ba`；#444 本地合并 `ecd1b86e06c727feafdf4e24320a9b7264954220`；#198 的 Codex 就绪和标题定向移植 `040fa81a83bc52ebb5bf9e5c9cb8372edc2241bc`，提交树 `fc410219b2ba6a6a5d65ecb87f98d6a1e738824e`。没有整合 #198 的旧历史，按用户决定保留该 PR 开放。未推送或发 Issue。
- #444 静态核对为 430 个注册命令及唯一授权，六个新命令归工作区迁移权限，退役 Proxy 命令未恢复。移植补充了 Codex 原生探测、标题与对话框回归，但没有运行产品测试。
- 串行验收入口 `verify-integration.py`。首次启动前被继承的 Git 环境变量拦截；仅在重试的子进程去掉 `GIT_EDITOR`、`GIT_PAGER`、`GIT_TERMINAL_PROMPT` 后，守卫采样成功。证据 `../fyagent-three-pr-evidence/verify-d_my4ikx/{results.json,resources.csv}`。
- **E0-1 BLOCKED_RESOURCE**：入场采样可用 RAM 2,983,198,720 B（约 2.78 GiB），总计 15.63 GiB，要求至少 20%（约 3.13 GiB）；系统 commit 30,565,273,600 / 36,529,537,024 B（83.7%，要求低于 80%）。CPU 2%、磁盘约 60.0 GiB 空余，不是限制项。bootstrap 没启动，未产生编译缓存或 exe；无本轮 WSL 产品测试，故不压缩 VHDX。没有停止用户应用或共享 WSL。
- 解锁：在机器空闲且资源同时满足上述入场阈值时，从原 checkout 重跑：`env -u GIT_EDITOR -u GIT_PAGER -u GIT_TERMINAL_PROMPT python .trellis/tasks/11-26-integrate-three-open-prs/verify-integration.py --worktree ../fyagent-three-pr-review --evidence-dir ../fyagent-three-pr-evidence --expected-head 040fa81a83bc52ebb5bf9e5c9cb8372edc2241bc --build`（当前 shell 语法）。验收未通过前不标记候选可体验。
- H1–H6 未运行；WSL 压缩须先有专用发行版并完成测试。macOS 交接见 `macos-issue-draft.md`，仍未发布，需远端操作授权和确切候选证据。Trellis 保持进行中。

## 复跑记录

- 首次复跑在采样时遇到 Windows PowerShell 本地代码页 JSON 解码失败（`verify-dlwb80zf`）；验收脚本显式使用 UTF-8 输出后，下一轮完整采样成功（`verify-jkj9ir7k/resources.csv`）。候选源码未变。
- 最新入场采样：可用 RAM 约 5.85 GiB，已达物理内存下限；系统 commit 为 30,840,811,520 / 38,297,714,688 B（约 80.53%，仍高于 80%）；CPU 1%，构建卷约 59.5 GiB。验收仍在 `E0-1` 放行前停止；无产品测试、编译、exe 或本轮编译缓存。
- 未发现本任务遗留的测试或编译进程。现有 Ubuntu-24.04 有活动 WSL 实例，未证明由本任务独占，不强制停止；不关闭其他用户应用、不改页面文件。保存工作后正常退出闲置应用，建议释放约 1 GiB 提交内存留余量，再复跑。

## 已执行：保护 6075、验收与诊断构建

- 6075 是 `Ubuntu-24.04` 内的 Vite（PID 348914），由 Windows `wslrelay.exe`（PID 33768）转发。只对无已建立连接的 6073 前端进程组发了 TERM，另正常停止四个独立测试容器：`novelist-pr53-redis`、`novelist-pr53-postgres`、`novelist-pr48-retry-isolated`、`novelist-pr48-isolated-8dcb6bcb`；没有删除容器或卷。6074、应用后端和数据库保留。最终 Windows/WSL 两侧都确认原 PID 仍监听 6075。未关闭或压缩共享 WSL。
- 实测通过：环境检查；ACL 与平台合同 33 项（`verify-a5zhsgwp/T1-1.log`）；迁移对话框 5 项（`verify-eo9x0cl7/T2-1.log`）；会话 fixtures 79 项（`verify-8stt4btf`）；MCP 目录 14 项（`verify-n_ep9t_2`）；Chromium mock IPC 迁移/SSH/WSL 交互 1 项（`verify-diz5umn6`）。ACL/迁移对话框原始 Vitest 退出 0，旧日志解析一度误报；已分别按原始输出核对。浏览器缺组件后安装锁定 Chromium 并重跑通过。
- 默认 release 和两次低内存尝试被资源保护取消。随后依用户要求单独运行诊断构建（一个 Cargo 作业、关闭 LTO、`opt-level=0`、`codegen-units=1`，内存仅采样告警）：`mise run build:binary` 实际退出 0，日志给出 Windows x64 exe；监控器把主进程退出后的短暂子进程残留判为失败，随后确认 Job 已排空。归档时构建复用的 helper 保留旧 mtime；以本次准备日志、helper target/暂存/主程序旁三处 SHA-256 一致及 x64 PE 头复核，手工归档。不是默认优化或签名产物，也不是完整 CLI 门禁通过。
- 产物：`../fyagent-three-pr-evidence/verify-pz9kmccg/artifact/`，同目录保留 `fyagent.exe` 与 `fyagent-user-helper.exe`，确切 SHA-256 与构建参数见 `manifest.json`。主程序 `4821c7143818dbcbde483de74b0db397d75a7d7daaa92e655064b774bc6f7ccf`。封存后清理独立 worktree 的两个 Cargo target、`dist` 与暂存 helper，净回收约 11.77 GiB；见同次证据目录 `manual-cleanup.json`。工作树干净。
- 仍未通过完整 Rust fixture/`mise run check`，未在隔离 Windows 用户/VM 启动桌面程序，未做产品级 WSL、SSH、线上 Keenable 或 macOS 验收。不得从默认账户启动替代隔离冒烟，不以浏览器 mock 声称真实传输。#198 保持开放，未推送、未发 Issue。
- `mise run check:frontend` 退出 0：类型、lint、格式检查通过；256 文件、3,435 测试通过、2 跳过；桌面 mock 7 项通过，视觉预检通过（未捕获原生桌面）。原始日志 `artifact://211`。
- 定向 Rust 稳定复跑：关闭测试调试信息和增量编译，单作业、持续监控；`verify-8fh8b6_i` 的 T5 退出 0，79 项会话 fixture 和 3 项 Codex 原生回归均通过。

## 当前交付快照

- 最终源码 `abbcb56726162e0e6fa33cc00490a0deafee16e0` / tree `f52e16a2877b3408d7606533e5f252fc7e95c911`。修复 Rust 格式和 Windows 下闲置 SSH 测试包装，保持 macOS 测试调用同一实现；修复模型 fixture 使用真实临时路径并初始化 Windows 测试上下文，未改生产模型写入规则。模型保留和回滚各 1 项、传输 10 项、Rust ACL 1 项在 `2fe1f52f` 执行通过；最终提交只新增这些测试源码的平台清单摘要，未重跑这些 Rust 用例。最终提交的 `check:contracts` 通过（`artifact://251`）。
- 最新 Windows x64 诊断版：`../fyagent-three-pr-evidence/verify-9swv4g55/artifact/`。`mise run build:binary` 退出 0；监控器仍报告子进程退出时序故障，确认本轮 Job 清空后，以实际构建日志、确切源码快照、x64 PE 头、helper 源/中转/交付校验和手工封存。主程序 SHA-256 `6191377cedd248a5b7964530b9938c4525c64b213660977a979545424da7d241`。旧快照诊断版不是当前交付。最终 release 缓存、前端 dist 和暂存 helper 清理回收约 3.70 GiB，调试缓存未尽删。没有启动桌面程序。
- `mise run check` 在此前快照已通过 Rust Clippy 后进入全量测试； Windows symlink/junction fixture 报权限 1314 或找不到 junction helper。OMP fixture 问题已定向修复并通过；整套 Rust 未在具备对应权限/工具的 Windows 环境复跑。未执行产品级 WSL/SSH 或 macOS 验收。
- 6075 在定向清理后两侧复核仍由原进程监听（见上）；最终只读复查时 Ubuntu-24.04 已是 Stopped、Windows 无 6075 监听。未执行 WSL shutdown/terminate、未停 6075 进程、未改其端口，也未擅自重启服务；停止原因未查明。

## Windows 全量复验通过

- 在用户启用开发人员模式后，普通会话实测文件和目录 symlink 均可创建并回读；此前报 1314 的 Skills 父目录用例通过。Junction 定向复验 3 项通过；PowerShell 路径只在测试进程设置。
- 固定 `abbcb56726162e0e6fa33cc00490a0deafee16e0` 运行 `mise run check`，退出 0；前端 3435 项通过、2 跳过，Rust fmt/check/Clippy/全套测试及契约门通过。完整日志独立存放于 [windows-full-check.log](../../../../fyagent-three-pr-evidence/verify-9swv4g55/windows-full-check.log)，SHA-256 `e810071676551266311692d2e9146561c70fe9dc21fbf8e1a5713140f0651fc6`。上节的全套 Rust 未复跑状态由本结果取代。
- 续跑检查确认候选工作树干净、HEAD/tree 与产物清单一致，两个 EXE 的 SHA-256 均匹配。H1–H6 仍无隔离 Windows 桌面或专用 WSL 发行版证据；不能在当前真实账户启动或使用共享 Ubuntu 代替。H7 仍为未发布的 macOS 交接；不归档、不推送、不关闭 #198。

### 续跑检查点

H1 的编译产物已就绪：重新校验两个 EXE 的 SHA-256，均与清单一致。当前只观察到一个 Explorer 会话、一个启用的本地账户，未发现正在运行或已可用的隔离 Windows VM／Sandbox；WSL 只有已停止的共享 Ubuntu。H1 `BLOCKED`：由 Windows 验收人提供一个已登录的普通测试账户（独立 Explorer）或可操作的 Windows x64 VM。将两个 EXE 放在同一目录，先验哈希，再按 [H1](manual-acceptance.md#h1--windows-x64-启动) 启动、观察主窗口与四个入口并退出。没有重复编译，也没有在日常账户启动。H2–H6 按各自隔离/目标/授权条件执行；维护 VHDX 仅限完成测试后的专用发行版。

### 日常环境 H1 实测

原隔离要求来自本任务设计和人工清单，不是软件运行限制。按用户最新决定，H1 改用日常普通账户，仅启动、导航和关闭窗口，不执行实际写入。Windows 11 x64 build 22621，开发人员模式开启。校验同一候选的两个 EXE 后启动，主窗可见；实际打开模型管理、MCP 管理、会话中心和环境迁移面板，观察到 SSH、WSL 入口。关闭主窗后窗口隐藏、进程仍运行；仅终止本次启动的实例，未验证托盘正常退出。记为启动/导航通过，正常退出未验证。

脱敏结果已发布至 [PR #444](https://github.com/fy-agent/fyagent/pull/444#issuecomment-7200109019)，明确是本地集成 SHA 的结果，不是 #444 单独 HEAD 的结果。未推送、未关闭 #198、未执行真实迁移或 macOS 验收。

## 实际写入验收

候选产品源码仍为 `abbcb56726162e0e6fa33cc00490a0deafee16e0`。为 H2/H3/H6 临时加载三项 opt-in 测试（无生产逻辑变化），测试补丁、日志及 SHA-256 封存在 [native-hil-run](../../../../fyagent-three-pr-evidence/native-hil-run/manifest.json)；验收后已从候选工作树反向撤除测试补丁，未带入候选提交。

- H2：专用 Alpine WSL2 普通用户和合成提示词；生产 WSL 预览／应用原生服务通过创建、覆盖、预览不写入、目标文件及覆盖备份逐字节和 SHA-256 回读。未使用共享 Ubuntu 或传输真实配置。
- H3：Windows OpenSSH 到环回测试服务，固定宿主键与专用身份，生产 SSH 预览／应用原生服务通过同一套真实写入与备份回读；Cargo 在 PATH 前插入 DLL 路径导致首次测试夹具门禁失败，使用同一测试 EXE 加独立 PATH 后通过。未验证桌面 SSH 档案重启流程或外部主机。
- H5：同一产品提交的批量模型测试 4 项通过，实际写入临时 OpenCode/omp 文件，核对无关字段保留与失败回滚；未写入日常配置，也不等于 GUI 克隆/轮换实测。
- H6：官方 Windows x64 Codex 0.154.0，以 Codex 自己生成的安装身份和无凭据临时 store，实际写入合成 rollout、调用真实 `thread/read` 验证并得到 `NativeReadbackVerified`；未宣称客户端人工打开、真实模型续答或 Gemini 原生验收通过。
- HIL 结束时仅停止专用 Alpine（环回 SSH 随之停止）；原 Ubuntu 未启动、未改。VHDX 压缩仍需具备权限的维护步骤。H1 托盘正常退出、H4 Keenable 线上可达性、H7 macOS 及上述专项未覆盖项仍不能标为通过。

## 远端交付检查点

- [候选 PR #445](https://github.com/fy-agent/fyagent/pull/445) 已发布为草稿；不分别合入 #443/#444，#198 继续开放。远端第一次提交检查拒绝两条本地 merge 的标题；已只修正提交标题及由此变化的父 SHA，保留原始 PR heads 和各提交的 tree。新 HEAD `22b4ba42c02e64a4ba46f8a02c2ee3dafb66a280`，源码树仍为 `f52e16a2877b3408d7606533e5f252fc7e95c911`。本地 commit convention 和远端该检查通过；已比较完整远端树的 3653 个路径/模式/对象 ID，均与本地提交匹配。
- 本地的 `mise run check`、诊断构建和 HIL 证据仍标注实际测试提交 `abbcb567`，利用同一源码树建立对应关系，不宣称新 SHA 上重新运行了本地测试。原 #444 的 Windows Clippy 失败与集成分支已修复的 SSH 测试包装函数一致，不能沿用原 PR 的 CI 当作 #445 的结果。
- [macOS 验收 Issue #446](https://github.com/fy-agent/fyagent/issues/446) 已 @nonghua123，等待原生结果。剩余合入门禁：macOS 交接 → Trellis prearchive → 归档 → post-archive 校验 → 精确 PR HEAD/Required CI → Merge Queue → 回读 main。期间保持任务 `in_progress`，不以草稿或本地通过代替合入。

- 本轮回收了环回 SSH 临时密钥和客户端材料，确认监听停止。专用测试 VHDX 已离线（约 80 MiB），当前会话非管理员且无 `Optimize-VHD`，标记 `BLOCKED_WSL_COMPACT`；未触碰共享 Ubuntu。托盘退出补测未能在当前自动化会话找到可调用的 FyAgent 托盘控件，已清理本轮实例，正常退出维持未验证。
- 已在候选工作树单独绑定 Trellis 会话，`task.py current --source` 返回直接 `session:`；`task.py validate` 和 `mise run check:contracts:prearchive --exclude-active-task .trellis/tasks/11-26-integrate-three-open-prs` 通过。归档命令未执行。

- 完整 `mise run check:prearchive --exclude-active-task .trellis/tasks/11-26-integrate-three-open-prs` 在候选 `22b4ba42` 上退出 0；[完整日志](../../../../fyagent-three-pr-evidence/native-hil-run/prearchive-full.log)，SHA-256 `4f16ba204b73bed0f1b7504a4bb469903c2689ab37fdd60e17c819e8f94edc43`。仅排除直接绑定的当前 Trellis 任务目录。
- #445 当前 HEAD 的 [CI / Required](https://github.com/fy-agent/fyagent/actions/runs/45935583031/job/140307540608) 已通过，含 Windows/macOS 后端与 Windows x64/ARM64 原生契约；这是 PR CI，不是 Merge Queue 的 merge-group 结果。#446 仍无人工回复。此前约定没有 macOS 原生结果不自动入队，若接受延期需明确记录维护者决定；未转 Ready、未启用 auto-merge、未合入 main、未归档。

- 用户明确接受 macOS 真机验收延期，授权合并 #445；由用户之后通知 `nonghua123` 通过新 PR 完成，#446 保留跟踪。本轮不把 macOS CI 视为真机验收。
