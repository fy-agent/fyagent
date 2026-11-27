# 主线四项遗留验收收尾执行手册

> **门禁声明**：仅在用户明确批准 Phase 1 规划摘要后，才运行 `task.py start` 进入执行阶段。所有重任务（前端编译、Rust 编译、浏览器或桌面实例）严格单槽串行。

## 1. 上下文激活与环境预检（Step 0）

在独立工作树 `E:/github/fyagent-main-acceptance` 中执行：

```powershell
$env:TRELLIS_CONTEXT_ID = 'agy-13141689-2db0-4c81-a59a-cee92dcc3009'
Set-Location E:\github\fyagent-main-acceptance
python ./.trellis/scripts/task.py validate .trellis/tasks/11-27-main-acceptance-closeout
python ./.trellis/scripts/task.py start .trellis/tasks/11-27-main-acceptance-closeout
```

1. 校验 Git 快照与原工作区零改动：`git rev-parse HEAD` 必须等于 `ca59409a49acb381f6f3ac7be1a09709cd302faa`；检查 `E:/github/fyagent` 状态未被触碰。
2. 采样当前可用物理内存、系统提交内存占比与磁盘剩余空间，写入 `E:/github/fyagent-main-evidence/resources-preflight.json`。
3. 执行 `mise run system:check` 确认工具链与 WebView2 环境就绪。

## 2. 构建标准二进制与执行 H1 托盘正常退出（Step 1–2）

1. **低内存单槽构建标准版（构建 S）**：
   - 先构建前端静态资源（`mise exec -- pnpm build`），再以单作业低内存参数编译无 `test-hooks` 的 `fyagent.exe` 与 `fyagent-user-helper.exe`。
   - 将两个可执行文件复制到 `E:/github/fyagent-main-evidence/build-standard/`，计算并写入 SHA-256 清单 `manifest.json`。
2. **执行 H1 实测**：
   - 确认当前无运行中的 `fyagent` 进程。
   - `Start-Process -FilePath E:\github\fyagent-main-evidence\build-standard\fyagent.exe -PassThru` 启动进程，记录 PID 与主窗出现时间。
   - 关闭主窗口，等待并核实主窗隐藏但进程仍在运行（`HasExited == false`）。
   - 通过托盘自动化脚本（或协同人工右键点击托盘菜单「退出」）触发 `"quit"` 菜单项。
   - 调用 `WaitForExit(15000)` 核实 `HasExited == true` 且 `ExitCode == 0`，确认无残留 `fyagent` / `fyagent-user-helper` 进程，保存 `H1-tray-exit.json` 日志。

## 3. 执行 H4 Keenable 线上连通验收（Step 3）

1. 核对 `keenable` 无 Key 配置输出（仅含 `url: "https://api.keenable.ai/mcp"` 与 `X-Keenable-Title: "FyAgent"`，无 `Authorization`）。
2. 运行协议连通验证脚本访问 `https://api.keenable.ai/mcp`：
   - 依次执行 MCP Streamable HTTP `initialize`、`notifications/initialized`、`tools/list` 与固定公开词条（`"Rust Tauri tray menu documentation"`）工具调用。
   - 记录 HTTP 状态码、响应头会话标识存在性（脱敏）、工具列表与返回内容摘要长度到 `H4-keenable-online.json`。
3. 在隔离客户端配置中挂载该无 Key Keenable MCP 项，通过真实 CLI 验证客户端侧 MCP 识别与连通性。

## 4. 构建 `test-hooks` 隔离二进制与准备局部 CLI（Step 4）

1. 编译启用 `--features test-hooks` 的 `fyagent.exe`（复用已构建的前端 `dist` 与 `fyagent-user-helper.exe`），封存至 `E:/github/fyagent-main-evidence/build-test-hooks/` 并记录 SHA-256。
2. **立即清理构建缓存**：删除 `E:/github/fyagent-main-acceptance/src-tauri/target` 与 `dist` 构建产物，记录回收磁盘空间。
3. 在 `E:/github/fyagent-main-evidence/isolated-clis/` 局部安装 `@google/gemini-cli@0.46.0` 与 `@openai/codex@0.154.0`，核验各自 `--version` 精确匹配 `0.46.0` 与 `0.154.0`。

## 5. 执行 H5 GUI「全端体检与克隆」克隆与轮换验收（Step 5）

1. 在 `E:/github/fyagent-main-evidence/isolated-home` 下生成 OpenCode 与 omp 合成基线配置（含待测模型项、无关自定义 provider 与无关字段），记录基线哈希。
2. 以 `FYAGENT_TEST_HOME=E:\github\fyagent-main-evidence\isolated-home` 启动构建 T，通过桌面 GUI（辅以可复核的窗口截图与文件哈希快照）：
   - 打开「模型 → 全端体检与克隆」；
   - 执行克隆预览 → 取消、轮换预览 → 取消，校验两份配置文件 SHA-256 未变；
   - 执行克隆预览 → 应用，校验目标模型项写入、`.fyagent.backup` 等于基线、无关 provider 与自定义字段完整保留；
   - 执行轮换预览 → 应用（使用合成假密钥），校验仅目标密钥字段更新、其余结构完全未变。
3. 核验日常用户目录 `C:\Users\<username>` 下对应配置文件的修改时间或哈希未被触碰，保存 `H5-gui-clone-rotate.json`。

## 6. 执行 H6 Gemini 0.46.0 实机恢复与真实模型续答验收（Step 6）

1. **门控与可访问性检查**：
   - 在未暴露 `gemini@0.46.0` 时通过 GUI「会话 → 导入会话包」验证 Gemini 恢复禁用门；
   - 加载负向不可恢复包，验证中文错误提示可由键盘聚焦、滚动可见且无假成功状态。
2. **Gemini 0.46.0 实机恢复与原生回读**：
   - 暴露局部 `@google/gemini-cli@0.46.0` 与隔离 `GEMINI_CLI_HOME`，初始化目标目录后在 GUI「会话 → 导入会话包」重新探测；
   - 导入含结构化上下文与末轮记忆锚点（`ANCHOR-ORCHID-1127`）的合成 `fyagent.session.v1` 会话包，核验展示标题取自真实提示词；
   - 执行恢复并完成原生回读校验，确认获得 `nativeReadbackVerified`，并用官方 `@google/gemini-cli@0.46.0` 核实会话文件可读。
3. **真实模型续答验证**：
   - 在隔离 `CODEX_HOME` 下对 `@openai/codex@0.154.0` 恢复同一锚点会话包并达 `nativeReadbackVerified`，在独立子进程注入本机授权凭据运行 `codex exec resume <session-id>` 追问验收代号，验证真实模型回复 `ANCHOR-ORCHID-1127`。
   - 若本机具备可用 Gemini 认证（或用户提供临时 `GEMINI_API_KEY`），在隔离 `GEMINI_CLI_HOME` 下执行 `gemini --resume <session-id>` 完成 Gemini 在线续答验证；否则如实记录 Gemini 原生恢复 `PASS` 与在线续答凭据阻塞条件。
   - 验证结束后立即擦除隔离目录中任何临时凭据材料。

## 7. 收尾校验、预归档与提交门禁（Step 7）

1. 将四项实测命令、构建 SHA-256、阶段哈希对比与脱敏结论完整写入 `.trellis/tasks/11-27-main-acceptance-closeout/execution-result.md`。
2. 复核是否产生需沉淀至 `.trellis/spec/` 的新契约结论（Phase 3.3）。
3. 运行门禁命令验证：
   ```powershell
   $env:TRELLIS_CONTEXT_ID = 'agy-13141689-2db0-4c81-a59a-cee92dcc3009'
   python ./.trellis/scripts/task.py validate .trellis/tasks/11-27-main-acceptance-closeout
   mise run check:contracts:prearchive --exclude-active-task .trellis/tasks/11-27-main-acceptance-closeout
   mise run supported-platform:check
   ```
4. 按 `.trellis/workflow.md` Phase 3.4 汇总变更文件与提交计划，经用户一次性确认后提交并完成收尾。
