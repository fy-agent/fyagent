# 主线四项遗留验收收尾（H1/H4/H5/H6）

## Goal

基于 GitHub 最新 `main` 分支（`ca59409a49acb381f6f3ac7be1a09709cd302faa`，已合入 PR #445），在独立工作树与受控隔离环境中完成前序任务 `11-26-integrate-three-open-prs` 尚未收尾的四项桌面/线上/实机验收（托盘正常退出、Keenable 线上连通、GUI 克隆与轮换、Gemini 0.46.0 实机恢复与真实模型续答），产出可复核、已脱敏的证据记录并按 Trellis 规范闭环。

## Background & Confirmed Facts

- **基线源码**：`upstream/main` 当前 HEAD 为 `ca59409a49acb381f6f3ac7be1a09709cd302faa`。主工作区 `E:/github/fyagent` 位于未完成的 `fix/programdata-bridge-acl` 分支且含未提交改动，本任务全程在独立工作树 `E:/github/fyagent-main-acceptance`（分支 `chore/11-27-main-acceptance-closeout`）与外置证据目录 `E:/github/fyagent-main-evidence` 执行。
- **Windows 路径隔离边界**：`src-tauri/src/config.rs:53-56` 与 `121-139` 规定 Windows 标准构建忽略 `FYAGENT_TEST_HOME`，始终冻结当前 Explorer 用户目录；仅测试或 `--features test-hooks` 构建允许通过 `FYAGENT_TEST_HOME` 重定向用户主目录与本地数据目录。
- **专项代码锚点**：
  - 托盘退出：`src-tauri/src/tray.rs:834-837` 注册菜单项 `"quit"`，`src-tauri/src/tray.rs:957-960` 调用 `app.exit(0)`。
  - Keenable 目录项：`src/pages/mcp/catalog.ts:1057-1091` 定义 `id: "keenable"`，协议为 Streamable HTTP，地址 `https://api.keenable.ai/mcp`，请求头 `X-Keenable-Title: FyAgent`，API Key 可选。
  - GUI 克隆与轮换：`src/pages/models/Page.tsx:215-245`「全端体检与克隆」按钮打开 `src/pages/models/ModelHealthDialog.tsx`（`overview` / `clone` / `rotate` 视图），对应后端 `src-tauri/src/services/provider/model_overview.rs:874-1160`。
  - 会话恢复与续答：`src-tauri/src/session_manager/migrate/native/gemini.rs:21-24` 限定 `@google/gemini-cli` 精确版本 `=0.46.0`；`src-tauri/src/session_manager/migrate/native/codex.rs:26` 限定 Codex 版本 `0.154.0`。
- **用户已确认决策**：
  - 新建今日 Trellis 任务并按正式流程推进。
  - 采用方案 A：H1 使用无 `test-hooks` 的标准 Windows x64 构建验收；H5 与 H6 使用 `--features test-hooks` 桌面构建配合 `FYAGENT_TEST_HOME` 临时隔离目录执行真实桌面 GUI 验收，零触碰日常配置。
  - 批准 H4 向 `https://api.keenable.ai/mcp` 发送固定公开词条验证线上连通，批准 H6 在隔离目录中执行 Gemini `0.46.0` 实机恢复及真实模型续答验证。

## Requirements

- **R1（H1 托盘正常退出）**：使用基于 `ca59409a` 构建的标准 Windows x64 `fyagent.exe`（附带并列 `fyagent-user-helper.exe`），验证启动主窗、关闭主窗后进程驻留托盘、通过系统托盘菜单触发「退出」后进程正常退出且退出码为 `0`、无残留子进程。
- **R2（H4 Keenable 线上连通）**：在不携带私钥或本地敏感信息的前提下，使用无 Key 配置对 `https://api.keenable.ai/mcp` 执行真实 Streamable HTTP 握手、工具枚举（`tools/list`）与固定公开词条搜索调用，并在隔离客户端配置中核验挂载连通结果。
- **R3（H5 GUI 克隆与轮换）**：在 `FYAGENT_TEST_HOME` 隔离目录下为 OpenCode 与 omp 预置包含无关 provider、自定义字段及合成假密钥的基线配置；通过桌面 GUI「模型 → 全端体检与克隆」依次验证：克隆/轮换预览取消后文件哈希不变、克隆应用仅修改目标模型并生成 `.fyagent.backup`、轮换应用仅更新目标密钥字段且保留全部无关配置。
- **R4（H6 Gemini 实机恢复与真实模型续答）**：在局部隔离目录安装 `@google/gemini-cli@0.46.0`（及按需 `@openai/codex@0.154.0`），验证版本/未安装禁用门、Gemini 提取不可用不阻塞恢复、含结构化上下文会话包的真实提示标题、可访问中文错误反馈、Gemini 原生恢复与 `NativeReadbackVerified` 回读；随后在隔离会话存储中注入授权测试凭据，通过真实 CLI 恢复导入会话并发起追问，验证真实模型基于已恢复上下文完成续答。
- **R5（证据诚实性与资源治理）**：重编译与重测试严格单任务串行，监控内存与提交水位；每次编译封存二进制 SHA-256 后立即回收独占构建缓存；测试用临时凭据副本仅存于外置隔离目录并在验证结束后立即擦除；报告中严格区分标准构建与 `test-hooks` 构建、`PASS` / `FAIL` / `BLOCKED` 状态，绝不输出任何密钥明文。

## Acceptance Criteria

- [ ] **AC1（对应 R1）**：记录标准构建 `fyagent.exe` 与 `fyagent-user-helper.exe` 的 SHA-256；实测记录包含启动 PID、主窗显示、关窗后进程存活、点击托盘菜单「退出」后进程自行退出及 `ExitCode == 0`。
- [ ] **AC2（对应 R2）**：记录无 Key Keenable 配置结构（无 `Authorization` 头）及对 `https://api.keenable.ai/mcp` 的真实连接结果（HTTP 状态码、工具清单、固定公开词条响应摘要；若远端限流或异常则按 DNS/TLS/429/协议准确分类记录）。
- [ ] **AC3（对应 R3）**：明确标注 `test-hooks` + `FYAGENT_TEST_HOME` 隔离边界；给出 OpenCode 与 omp 配置文件在「基线 → 预览取消 → GUI 克隆应用 → GUI 轮换应用」四阶段的 SHA-256、`.fyagent.backup` 校验及无关字段保留的结构化核对结论，日常用户目录零变更。
- [ ] **AC4（对应 R4）**：记录 `@google/gemini-cli@0.46.0` 门控测试、标题与中文错误可访问性核验、`NativeReadbackVerified` 原生回读证据，以及恢复会话后真实模型续答命中预设良性记忆口令的脱敏输出（若任一客户端缺少对应厂商可用凭据，按单端如实记录 `PASS` / `BLOCKED` 及解除条件）。
- [ ] **AC5（对应 R5）**：完成任务目录 `execution-result.md` 归档级证据写入，外置临时凭据与构建缓存已清理，`task.py validate`、预归档门禁与平台扫描门禁全部通过，原工作区 `E:/github/fyagent` 未被修改。

## Out of Scope

- 不修改或提交主工作区 `E:/github/fyagent`（`fix/programdata-bridge-acl`）的任何未提交改动。
- 不向日常 Windows 用户主目录（`C:\Users\<username>`）写入测试模型配置、假密钥或测试会话。
- 不在本机声称完成 macOS 真机验收（仍由 Issue #446 跟踪）。
- 经实测暴露并获用户批准，本任务包含对 `src-tauri/src/session_manager/migrate/native/mod.rs` 与 `gemini.rs` 两处 Windows 原生恢复阻塞缺陷（`node.exe` 解析与 `.cmd` 启动器目标解析）的最小边界修复及回归测试，不扩大其他业务域代码改动。
