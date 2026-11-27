# 主线四项遗留验收收尾执行结果（最终归档版）

## 1. 基线、隔离边界与双构建封存清单

- **基线源码**：`upstream/main` @ `ca59409a49acb381f6f3ac7be1a09709cd302faa`（独立工作树 `E:/github/fyagent-main-acceptance`，分支 `chore/11-27-main-acceptance-closeout`）。
- **主工作区与日常配置零污染核验**：
  - 主工作区 `E:/github/fyagent`（`fix/programdata-bridge-acl`）全程未切换、未修改。
  - 日常用户目录 `C:\Users\<username>` 下 `.config\opencode\opencode.json`、`.omp\agent\models.yml`、`.omp\agent\config.yml` 在桌面验收前后 SHA-256 100% 一致（零改动）。
- **本轮实测暴露并修复的两处 Windows Gemini 原生恢复阻塞缺陷**：
  1. `src-tauri/src/session_manager/migrate/native/mod.rs`：新增显式 `windows` / `macos` 正分支 `runtime_binary_candidates`（Windows 优先解析 `node.exe`），并在 Windows `installer_locations` 加入 `get_user_roaming_app_data_dir().join("npm")`。
  2. `src-tauri/src/session_manager/migrate/native/gemini.rs`：`NATIVE_BRIDGE` 新增 `win32` 正分支 `resolveEntry(rawArg)`，只读解析 `<= 64 KiB` 的 `.cmd` / `.bat` 包装器并提取受控 `@google/gemini-cli/bundle/gemini.js` 引用（展开前导 `%dp0%` / `%~dp0`）后调用 `fs.realpath`，保留 `@google/gemini-cli` `=0.46.0` 硬门禁；Windows 子进程增加 `CREATE_NO_WINDOW`；补充 Windows 集成单测 `windows_cmd_shim_and_node_exe_resolve_and_guard_the_fixed_bridge`（覆盖 `node.exe` 解析、伪造 `.cmd` 零副作用拒绝、合法 `.cmd` 写入与 `ReadbackVerdict::Visible`），同步更新 `scripts/tasks/supported-platform-structure-assets.json` 与 `.trellis/spec/backend/session-migration.md`。
- **封存可执行文件 SHA-256（封存后已即时清理 `src-tauri/target`、`user-helper/target` 与 `dist` 编译缓存）**：
  - **构建 S（标准构建，无 `test-hooks`）**：
    - `fyagent.exe`（71,463,424 B, `x64`）：`ff4ab9e732bf03a9e64634bb99b69286385e63d33ee75087abd9877aa0790bf6`
    - `fyagent-user-helper.exe`（1,024,512 B, `x64`）：`3fa038d88c27f7300a4a6e64d292e54924ff108a4d494e4af0db269156c105f3`
  - **构建 T（诊断隔离构建，`--features test-hooks`）**：
    - `fyagent.exe`（71,558,656 B, `x64`）：`b62a6a5cf07fb019af4ed307566eed296f49c5f0425d9aaf3b3b85133b8b13a4`
    - `fyagent-user-helper.exe`（1,024,512 B, `x64`）：`3fa038d88c27f7300a4a6e64d292e54924ff108a4d494e4af0db269156c105f3`

---

## 2. 四项遗留验收实测结论总览

| 编号 | 验收项 | 最终结论 | 隔离/构建层级 | 外置证据文件（`E:/github/fyagent-main-evidence/logs/`） |
| --- | --- | --- | --- | --- |
| **H1** | 系统托盘正常退出与退出码 `0` | **PASS** | 构建 S（标准无 `test-hooks`，交互式 Explorer 桌面） | `H1-tray-exit.json`, `H1-1-main-window.png` |
| **H4** | Keenable `https://api.keenable.ai/mcp` 线上连通 | **PASS** | 真实公网 Streamable HTTP + 真实 `@google/gemini-cli@0.46.0` | `H4-keenable-online.json` |
| **H5** | 桌面 GUI「全端体检与克隆」克隆与轮换 | **PASS** | 构建 T（`--features test-hooks` + `FYAGENT_TEST_HOME` 隔离桌面 GUI） | `H5-gui-clone-rotate.json`, `H5-1..4-*.png` |
| **H6** | Gemini `0.46.0` 实机恢复回读 + GUI 门控可访问性 + 真实模型续答 | **PASS**（Gemini 在线续答受本机无 Gemini 凭据限制，如实单列） | 构建 T 桌面 GUI + 真实 `@google/gemini-cli@0.46.0` + 真实 `codex@0.154.0` 在线模型 | `H6-gui-dialog-gates.json`, `H6-1..2-*.png`, `H6-gemini-cli-restore.json`, `H6-real-continuation.json` |

---

## 3. 分项核验明细

### 3.1 H1：系统托盘正常退出（PASS）
- 在正常交互式 Explorer 桌面启动标准构建 `fyagent.exe`（`PID=55500`），主窗于 `656 ms` 渲染可见。
- 向主窗发送 `WM_CLOSE` 后，窗口隐藏驻留系统托盘（`windowVisible: false`, `processAlive: true`）。
- 通过系统托盘点击「退出」触发 `"quit"` 菜单项，进程正常退出，实测 **`ExitCode == 0`**，退出后复查 `fyagent` / `fyagent-user-helper` 残留进程数为 **`0`**。
- 附加验证：在非交互式后台桌面站（`GetShellWindow() == 0`）启动时，进程按设计命中 `WIN_INTERACTIVE_USER_UNAVAILABLE`（`ExitCode == 1`）fail-closed 保护退出。

### 3.2 H4：Keenable 线上连通（PASS）
- 核验无 Key 配置仅含 `url: "https://api.keenable.ai/mcp"` 与请求头 `X-Keenable-Title: FyAgent`（无 `Authorization` 头）。
- 线上协议直连实测：
  - `initialize`：HTTP `200`（1,144 ms），返回 `keenable-mcp-server` `0.2.1`（协议版本 `2025-03-26`）；
  - `notifications/initialized`：HTTP `202`（267 ms）；
  - `tools/list`：HTTP `200`（853 ms），枚举 3 个工具：`search_web_pages`、`fetch_page_content`、`get_billing`；
  - `tools/call`（固定公开技术词条 `"Tauri v2 system tray menu documentation"`）：HTTP `200`（447 ms），`isError: false`，返回正文 `22,389` 字符。
- 真实客户端挂载实测：在隔离目录挂载 FyAgent 投影的 `httpUrl: "https://api.keenable.ai/mcp"` 后执行真实 `@google/gemini-cli@0.46.0` `mcp list`，输出 `✓ keenable: https://api.keenable.ai/mcp (http) - Connected`。

### 3.3 H5：GUI「全端体检与克隆」克隆与轮换（PASS）
- 标注边界：采用 `--features test-hooks` 构建配合 `FYAGENT_TEST_HOME` 隔离目录执行真实桌面 WebView2 GUI 操作，零触碰日常配置。
- 四阶段配置文件 SHA-256 实测记录：
  - **基线（Baseline）**：
    - `opencode.json`：`838a06514553271e03ae6eae5026ef5d18548f825ff4eb30d39dabd8d7dd78a8`
    - `omp/models.yml`：`4a008da1c78f6aadbad6228fa6222bd36f983e62b98c4c4973f7809a64eac6c2`
    - `omp/config.yml`：`c0e7aefadf7b9ec737e00413a332b0e3a20aa6b0047a6f1c70af0618c612d0bc`
    - `mcp-sentinel.json`：`833e3a03ee367064da0d3ac0ab27f909e25469701d5281cb4872059d75f33730`
  - **克隆预览取消后 / 轮换预览取消后**：四份文件 SHA-256 与基线 **100% 相同**（`cloneCancelUnchanged: true`, `rotateCancelUnchanged: true`）。
  - **GUI 克隆应用后（OpenCode → omp）**：
    - `omp/models.yml` 更新为 `2578aafa20e74e7e29716c3b90b9facf4a62218a8ae063503dcbdc1b0d448eb2`，旁路备份 `models.yml.fyagent.backup` 哈希精确等于基线 `4a008da1...`；
    - `omp/config.yml` 更新为 `573328c208147806ee7d85bc94fe4c6906018e12ead050753bf4f938476b31b7`，旁路备份 `config.yml.fyagent.backup` 哈希精确等于基线 `c0e7aefa...`；
    - 手写注释（`# keep-my-custom-comment`、`# keep-config-comment`）、无关 provider（`hand-edited`）及 `telemetry: false` 完整保留。
  - **GUI Key 一键轮换应用后（合成测试假密钥，不联网）**：
    - `opencode.json` 更新为 `03f907497e7729c8b20db9464d52251d339b598c1f7f63a45c5e0b38a50bcc95`，`omp/models.yml` 更新为 `b2182ea5ef4c348e9b4f67ff413ebb4acd3d5f94a9dea63bf1400eb54241e7a4`；
    - 仅选定目标密钥字段更新，`theme: "ocean-custom"`、`userManualFlag: true`、`my-local-ollama`、模型列表、YAML 注释及旁路 `mcp-sentinel.json`（哈希保持 `833e3a03...`）完全未受影响。

### 3.4 H6：Gemini 0.46.0 实机恢复回读、桌面门控与真实模型续答（PASS）
1. **桌面 GUI 可访问中文错误与门控实测**（`H6-1-accessible-chinese-error.png`、`H6-2-gemini-disabled-gate.png`、`H6-gui-dialog-gates.json`）：
   - 导入负向包触发 `role="alert"` 中文错误卡片（`迁移包校验` / `迁移包版本不受支持，请使用当前支持的会话包` / `未调用目标恢复，未写入目标会话。`），自动获得键盘焦点外框、视口内完整可见、含「返回编辑位置」操作按钮且无假成功横幅。
   - 未暴露 `gemini@0.46.0` 时，解析合法 Gemini 会话包正确显示包摘要与会话信息，同时「确认恢复至目标软件」按钮严格禁用。
   - 在 `FYAGENT_TEST_HOME\AppData\Roaming\npm` 暴露 `gemini.cmd`（`0.46.0`）与 `node.exe` 并重开导入弹窗后，`probe_local_provider("gemini")` 实时探测通过（证明 Gemini 源提取门未开放不会误锁已验证的目标写入门），「确认恢复至目标软件」按钮正常解锁。（注：独立 `--features test-hooks` 进程点击写入会按 `identity.rs:298-324` 的 `LOCAL_STATE_DIR_OVERRIDE` 线程局部测试守卫在 `claim_batch` 前 fail-closed 拦截，`session_restore_attempts` 保持空表零脏数据。）
2. **Gemini `0.46.0` 实机原生恢复与官方 CLI 回读（PASS）**（`H6-gemini-cli-restore.json` + `windows_cmd_shim_and_node_exe_resolve_and_guard_the_fixed_bridge`）：
   - 修复后的 `NATIVE_BRIDGE` 与 `GeminiWriter` 对真实 `@google/gemini-cli@0.46.0`（Windows `gemini.cmd` 包装器）完成 `probe -> ready`、`write -> written`（会话 ID `00112233-4455-4677-8899-aabbccddeeff`）、`read -> visible`（`ReadbackVerdict::Visible`，消息内容与顺序逐字匹配）；
   - 官方 `gemini.cmd --list-sessions` 成功读出已恢复会话：`1. Gemini 实机恢复验收测试 (Just now) [00112233-4455-4677-8899-aabbccddeeff]`。
3. **真实模型续答实测（Codex `0.154.0` 在线续答 PASS；Gemini 在线续答受本机无凭据限制如实披露）**（`H6-real-continuation.json`）：
   - **Codex `0.154.0`**：在隔离 `CODEX_HOME` 核实初始无 `installation_id`、首次运行后自然生成；写入含良性口令锚点 `ANCHOR-ORCHID-1127` 的会话 `79955eb8-1de6-4ef2-b1c9-b5c47f475a51`，通过官方 `codex app-server --stdio` `thread/read` 核验 `turnCount: 1`；随后执行 `codex exec resume 79955eb8-1de6-4ef2-b1c9-b5c47f475a51` 调用真实在线模型 `gpt-6-luna` 追问上一轮口令，耗时 `7,943 ms`、退出码 `0`，真实模型精确回复 **`ANCHOR-ORCHID-1127`**，续答后 `thread/read` 核验 `turnCount: 2`（`persistedNewTurn: true`）。隔离目录中的临时凭据配置已在验证后立即擦除。
   - **Gemini `0.46.0` 在线续答前提说明**：本机未配置 `GEMINI_API_KEY`、`GOOGLE_GENAI_USE_VERTEXAI` 或 Google OAuth 登录态，故不声称完成 Gemini 在线模型回复，仅声称完成 Gemini `0.46.0` 实机恢复与原生回读（`PASS`）及 Codex `0.154.0` 真实模型续答（`PASS`）。

---

## 4. 质量与治理门禁核验记录

在 `E:/github/fyagent-main-acceptance` 已通过以下全部校验：
- `python ./.trellis/scripts/task.py validate .trellis/tasks/11-27-main-acceptance-closeout` → `PASS`
- `mise run rust:fmt:check` & `mise run rust:clippy` → `PASS`
- `mise run rust:test windows_cmd_shim_and_node_exe_resolve_and_guard_the_fixed_bridge` → `PASS`
- `mise run format:check` & `mise run lint` & `mise run typecheck` → `PASS`
- `mise run supported-platform:check` → `PASS`
- `mise run check:contracts:prearchive --exclude-active-task .trellis/tasks/11-27-main-acceptance-closeout` → `PASS`
