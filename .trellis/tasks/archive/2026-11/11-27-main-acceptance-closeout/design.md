# 主线四项遗留验收收尾技术设计

## 1. 架构与环境边界

| 边界对象 | 路径 / 标识 | 职责与约束 |
| --- | --- | --- |
| 原开发工作区（只读保护） | `E:/github/fyagent` (`fix/programdata-bridge-acl`) | 保持现状，禁止切换分支、暂存、重置或覆盖任何文件。 |
| 验收独立工作树 | `E:/github/fyagent-main-acceptance` (`chore/11-27-main-acceptance-closeout`) | 基于 `upstream/main` (`ca59409a49acb381f6f3ac7be1a09709cd302faa`) 创建；承载本 Trellis 任务文档与构建入口。 |
| 外置证据与隔离运行根 | `E:/github/fyagent-main-evidence` | 存放构建产物封存件、SHA-256 清单、局部 CLI 安装、隔离 `FYAGENT_TEST_HOME`、日志与截图，不入 Git 库。 |

## 2. 双构建配置与隔离机制（方案 A）

根据 `src-tauri/src/config.rs:53-56` 与 `121-139`，Windows 标准构建强制读取启动时冻结的 Explorer 用户目录，仅 `--features test-hooks` 允许 `FYAGENT_TEST_HOME` 覆盖。为同时满足「H1 真实正式行为」与「H5/H6 零污染日常配置」两个硬约束，采用双构建配置：

1. **构建 S（标准构建，无 `test-hooks`）**：
   - 用途：专用于 **H1 托盘正常退出**。仅执行启动、主窗观察、关闭主窗驻留托盘、右键托盘菜单点击「退出」，不触发任何配置保存或迁移操作。
   - 封存路径：`E:/github/fyagent-main-evidence/build-standard/`（含 `fyagent.exe`、`fyagent-user-helper.exe` 与 `manifest.json`）。
2. **构建 T（隔离钩子构建，`--features test-hooks`）**：
   - 用途：专用于 **H5 GUI 克隆/轮换** 与 **H6 会话导入/Gemini 实机恢复 GUI 验证**。
   - 运行时注入环境变量：`FYAGENT_TEST_HOME=E:/github/fyagent-main-evidence/isolated-home`、`GEMINI_CLI_HOME=E:/github/fyagent-main-evidence/isolated-gemini`、`CODEX_HOME=E:/github/fyagent-main-evidence/isolated-codex` 及前置局部 CLI 的 `PATH`。
   - 证据披露：在 `execution-result.md` 中明确标注构建 T 开启了 `test-hooks` 特性以启用 `FYAGENT_TEST_HOME` 路径隔离，不混同于标准安装包签名产物。

### 低内存单槽构建参数

本机物理内存 15.63 GiB、当前提交内存水位约 80%–83%。两次 Cargo 构建严格串行（绝不同时运行），采用低内存诊断编译参数（`CARGO_BUILD_JOBS=1`、关闭 LTO 与调试符号以降低 `link.exe` 峰值内存），每次封存二进制后立即清理 `target` 中间产物以回收磁盘与页面缓存。

## 3. 分项验证设计

### 3.1 H1：托盘正常退出与退出码

- **调用链**：桌面进程启动 → WebView2 主窗渲染 → 主窗收到关闭事件隐藏至系统托盘 → 系统托盘图标（`TRAY_ID`）菜单触发 `"quit"` 事件 → `tray::handle_tray_menu_event` 调用 `app.exit(0)` → 进程退出。
- **验证与采集**：
  - PowerShell 启动构建 S：`Start-Process -FilePath <standard/fyagent.exe> -PassThru`，记录 `StartTime`、`Id`。
  - 核实主窗口句柄可见后，向主窗口发送关闭请求，等待 2 秒验证 `$proc.HasExited -eq $false`。
  - 通过 Windows UIAutomation / Win32 托盘区域交互（展开 Windows 11 隐藏图标溢出浮层 `TopLevelWindowForOverflowXamlIsland`，定位 `FyAgent` 托盘按钮弹出原生菜单并点击「退出」；若受桌面焦点限制则协同人工右键点击一次「退出」）。
  - `$proc.WaitForExit(15000)`，断言 `HasExited == true` 且 `ExitCode == 0`，并枚举进程表确认无同名残留进程。

### 3.2 H4：Keenable 线上连通

- **目录契约**（`src/pages/mcp/catalog.ts:1057-1091`）：
  - `url`: `https://api.keenable.ai/mcp`
  - `headers`: `{"X-Keenable-Title": "FyAgent"}`（无 Key 时不生成 `Authorization`）。
- **两层实测**：
  1. **协议层直连验证**：按 MCP Streamable HTTP 规范（`Accept: application/json, text/event-stream`，携带 `X-Keenable-Title: FyAgent` 及协商返回的 `Mcp-Session-Id`）依次执行 `initialize`、`notifications/initialized`、`tools/list` 及一次固定公开技术词条查询（如查询 `"Tauri v2 tray menu"`），记录状态码、工具名称集合与响应摘要。
  2. **客户端集成验证**：将同一无 Key 配置写入隔离客户端的 `mcpServers` / `mcp_servers` 配置，通过真实 CLI 列举或调用 Keenable 工具，验证真实客户端对 Streamable HTTP 配置的兼容性。

### 3.3 H5：GUI「全端体检与克隆」克隆与轮换

- **合成基线构造**（位于 `$FYAGENT_TEST_HOME`）：
  - OpenCode 配置文件与 omp 配置文件各预置：1 个待操作测试模型项（假密钥 `sk-fyagent-baseline-0001`）、1 个无关自定义 provider、顶层无关自定义键值，以及独立的空 MCP/Skills 占位配置，计算初始 SHA-256。
- **桌面 GUI 交互路径**：
  - 启动构建 T（绑定 `FYAGENT_TEST_HOME`），进入「模型」页点击「全端体检与克隆」打开 `ModelHealthDialog`。
  - **阶段 1（取消不变性）**：分别进入 `clone` 视图生成预览后取消、进入 `rotate` 视图生成预览后取消，回读两份目标文件确认字节级 SHA-256 与基线完全一致。
  - **阶段 2（GUI 克隆）**：选择源配置克隆至已准备的 OpenCode / omp 目标并确认应用；校验目标文件新增/更新选定模型项、旁路 `.fyagent.backup` 字节等于基线、无关 provider 与自定义字段保持不变。
  - **阶段 3（GUI 轮换）**：切换至 `rotate` 视图，输入新合成假密钥（`sk-fyagent-rotated-0002`，不联网），勾选测试目标预览并确认应用；校验仅选定目标的密钥字段变更，模型列表、无关 provider 及其他域文件未被改动。

### 3.4 H6：Gemini 0.46.0 实机恢复、Windows 阻塞缺陷修复与真实模型续答

- **实测发现的 Windows 原生恢复两处阻塞缺陷与最小修复设计**：
  1. `src-tauri/src/session_manager/migrate/native/mod.rs`：`resolve_cli` 原本仅拼接无扩展名 `dir.join(binary)`，在 Windows 上无法命中 `node.exe`，且 `installer_locations` 未包含 Windows npm 全局默认目录 `Roaming\npm`。修复为通过显式 `windows` / `macos` 平台分支的 `runtime_binary_candidates` 优先匹配 `node.exe`，并在 Windows 安装目录候选列表加入 `get_user_roaming_app_data_dir().join("npm")`。
  2. `src-tauri/src/session_manager/migrate/native/gemini.rs`：`NATIVE_BRIDGE` 原本直接对 `entryArg` 调用 `fs.realpath`，在 Windows 下命中 `gemini.cmd` 批处理包装器而非 `bundle/gemini.js`，导致向上读取 `../package.json` 失败或误判 `versionMismatch`。修复为在 `win32` 且入口为 `.cmd` / `.bat` 时只读解析上限 64 KiB 的包装脚本，仅提取指向 `@google/gemini-cli/bundle/gemini.js` 的受控引用（展开前导 `%dp0%` / `%~dp0`）后再执行 `fs.realpath` 与严格包名/版本 `0.46.0` 门禁校验；同时在 Windows 子进程添加 `CREATE_NO_WINDOW` 标志并补充 `windows_cmd_shim_and_node_exe_resolve_and_guard_the_fixed_bridge` 单元测试，同步更新 `scripts/tasks/supported-platform-structure-assets.json` 审查摘要。
- **隔离工具链准备**：
  - 在 `E:/github/fyagent-main-evidence/isolated-clis/` 使用独立前缀安装 `@google/gemini-cli@0.46.0` 与 `@openai/codex@0.154.0`，不改动系统全局包目录。
- **Gemini 门控与原生恢复验证**：
  1. **门控负向用例**：先在不提供 `gemini@0.46.0`（或提供非 `0.46.0` 版本）的 `PATH` 下打开「会话 → 导入会话包」探测，验证 Gemini `writeSupported: false`，按钮禁用且错误提示准确。
  2. **Gemini 0.46.0 正向恢复**：切换至包含 `@google/gemini-cli@0.46.0` 的 `PATH` 与隔离 `GEMINI_CLI_HOME`，在 GUI「会话 → 导入会话包」触发重新探测（确认提取门状态不误锁已支持的写入门）。导入包含结构化上下文噪声与真实用户提示词、且末轮带有良性口令锚点（如 `ANCHOR-ORCHID-1127`）的合成 `fyagent.session.v1` 会话包：
     - 核对界面展示真实提示标题而非结构化附件内容；
     - 执行恢复并触发原生回读，验证达到 `nativeReadbackVerified`（底层经 `@google/gemini-cli@0.46.0` 自带 `loadConversationRecord` 与 `convertSessionToClientHistory` 校验一致）。
  3. **中文错误可访问性**：在导入对话框加载负向 fixture 包（或不可恢复输入），验证中文错误区域具备键盘焦点可达性、滚动可见且不展示成功横幅。
- **真实模型续答（Real Model Continuation）**：
  - 针对已通过 `nativeReadbackVerified` 恢复到隔离存储的会话：
    - **Codex `0.154.0` 续答**：在隔离 `CODEX_HOME` 下完成 Codex `0.154.0` 原生初始化与同一记忆锚点会话包恢复后，仅在子进程内存中透传本机已就绪的 `OPENAI_API_KEY`（及必要的模型路由配置副本），执行 `codex exec resume <restored-session-id> "请只回复上一轮约定的验收代号"`，验证真实模型读取恢复历史并返回 `ANCHOR-ORCHID-1127`。
    - **Gemini `0.46.0` 续答**：检查本机是否存在可用的 Gemini 认证材料；若存在则在隔离 `GEMINI_CLI_HOME` 临时注入并执行 `gemini --resume <restored-session-id> -p "请只回复上一轮约定的验收代号"` 验证真实模型回复；若本机未配置任何 Google/Gemini 有效凭据，则保留已恢复的隔离会话现场并提供单行续答命令供用户填入临时 Key 即刻补验，在报告中严格区分「Gemini `0.46.0` 实机恢复与原生回读（PASS）」与「Gemini 在线模型续答凭据前提」。

## 4. 安全、回滚与清理边界

- 任何测试密钥使用明显合成前缀且绝不发起外网请求；真实模型续答仅在隔离子进程使用已授权凭据发送固定良性口令问答，禁止在日志、截图或 Markdown 报告中打印凭据、请求头令牌或真实用户历史会话。
- 测试完成后立即删除外置隔离目录中的任何临时认证配置副本及 Cargo `target` 构建缓存，仅保留脱敏日志、哈希清单与必要截图。
- 本任务产物仅包含 `.trellis/tasks/11-27-main-acceptance-closeout/` 下的规划与执行记录文档，全文规避平台扫描器禁用的退役内核/子系统词汇，确保归档前后 `mise run supported-platform:check` 均零告警通过。
