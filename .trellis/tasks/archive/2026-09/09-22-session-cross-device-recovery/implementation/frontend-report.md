# 会话跨设备恢复（Session Cross-Device Recovery）前端与UI/UX实现报告

- **执行角色**：Antigravity (`gemini-3.8-flash-high`)，前端与UI/UX实施
- **任务目录**：`.trellis/tasks/09-22-session-cross-device-recovery`
- **基线与分支**：`origin/main c0b2ec21`，工作分支 `codex/session-cross-device-recovery-20260922`
- **交付日期**：2026-09-22
- **当前状态**：定向缺陷修复已完成，静态检查与模块单测全数通过，浏览器端E2E与实机验收留待QA与协调方

---

## 1. 本轮定向缺陷修复与纯 Helper 实现

针对协调者在复核中提出的 6 项确切缺陷，前端已全部完成定向修复，并将相关纯函数抽离至 `src/shared/features/session-migration.ts` 作为单一语义 Owner（避免页面组件重导出触发 Fast-Refresh 异常）：

### 1.1 恢复请求重试幂等与弹窗生命周期隔离 (`ImportPackageDialog.tsx`)
- **问题**：原实现每次点击确认均生成全新 `crypto.randomUUID()`，重试无法复用，违反后端幂等重试约束；且异步回调未隔离已关闭或新开弹窗。
- **修复**：
  - 引入 `computeRestoreBindingKey(params)` 纯函数：根据 `packagePath`、`contentDigest`、`targetProviderId`、`snapshotIds`、`targetWorkspace`、`requestKind` 计算确定性绑定指纹。
  - 在 `ImportPackageDialog` 中维护固定 `requestIdRef`：仅当用户实质修改绑定参数（更换包、切换目标软件、增删快照、修改本地目录、改变冲突策略）时重新生成 `requestId`；网络超时或异常重试时严格复用同一 `requestId`。
  - 恢复请求进行中（`isRestoring`）全量禁用关闭弹窗、切换软件、修改快照及修改目录。
  - 采用 `activeExecutionTokenRef` 隔离异步竞态：若用户已重置或发起新请求，陈旧的异步响应直接丢弃，不污染界面状态。

### 1.2 本地探针（Local Probe）门控与多来源快照过滤 (`ImportPackageDialog.tsx`)
- **问题**：原逻辑未对目标软件进行本地探针校验，直接取首个来源 Provider 却提交包内全部快照，多来源包发生交叉错配；非成功状态统一显示为绿色成功。
- **修复**：
  - 增加纯函数 `isProviderRestoreSupported(probe)`：严格校验本地探测状态（必须 `installed === true` 且 `writeSupported === true`）。若未安装或版本不受支持，确认按钮保持禁用，并展示具体中文拦截原因（如版本过低、未检测到客户端）。
  - 增加纯函数 `filterSnapshotsForProvider(sessions, providerId)`：解析包后提供目标软件下拉选择（限定仅可选包内实际包含的来源 Provider），快照列表动态按所选目标软件严格过滤，杜绝跨 Provider 快照错配。
  - 弹窗结果横幅严格细分真实阶段：
    - 全量干净成功（`nativeWritten` / `nativeReadbackVerified`）：绿色成功横幅。
    - 包含失败（`failed`）或需要对账（`needsReconciliation`）：红色警示横幅与明确排查提示。
    - 处于中间状态（`ambiguous` / `nativeWritePending`）：黄色待确认横幅，真实展示中文章态。

### 1.3 冻结批量导出与逐项审查阻断 (`ExportPreviewDialog.tsx` & `Page.tsx`)
- **问题**：多选时仅预览当前单个选中项，实际导出却包含全部勾选，未受审会话被静默导出；保存时重读活动状态导致竞态。
- **修复**：
  - 在 `Page.tsx` 中点击“批量导出”时即时冻结所选的会话项列表 `ExportTargetItem[]`（深拷贝形式），打开弹窗并以其为唯一输入源。
  - 在 `ExportPreviewDialog` 中引入 `validateBatchExportSessions` 审查机制：对冻结列表中的每一个会话逐项执行 `previewSessionMigration` 与 `canExportSession` 校验。
  - 只要有任意会话存在抽取失败、未决提问（`indeterminate`）或不可导出轮次，立即阻止整包导出，并明确列出违规会话名称及拦截原因。
  - 导出执行时严格基于冻结列表调用后端接口，不读取活动列表的后续勾选变动。

### 1.4 Provider ID 统一为 `grokbuild`
- **问题**：前端部分代码写为 `"grok"`，与后端 `src-tauri/src/session_manager/mod.rs` 中的 `"grokbuild"` 不符，导致探测假未安装及无法筛选。
- **修复**：
  - 在 `SUPPORTED_PROVIDER_IDS`、`PROVIDER_LABELS` 及 `StartupGuideDialog.tsx` 中全面统一使用 `"grokbuild"`。

### 1.5 结构化错误解析与真实失败态展示 (`session-migration.ts` & `Page.tsx`)
- **问题**：Native 返回的错误常为 JSON 字符串，直接展示代码内部 JSON；列表与记录加载失败时 fallback 为兜底空数组，产生“空数据”假象。
- **修复**：
  - 在 `src/shared/features/session-migration.ts` 中实现统一解析函数 `parseMigrationError(err)`：支持解析 Tauri IPC 抛出的 JSON 字符串（提取 `code`、`message`、`detail`），映射为用户友好的中文提示文案，不暴露内部黑话。
  - `Page.tsx` 中针对会话列表加载失败（`sessionsError`）、恢复记录加载失败（`attemptsError`）与会话详情失败（`sessionDetailData.error`），分别提供明确的“重新加载”与错误提示面板，不再以空列表掩盖真实失败。
  - `handleRestore` 恢复结果处理：仅当全部会话均为干净成功时触发“会话恢复写入完成”成功提示；对 `ambiguous` 提示“恢复操作待确认”，对 `needsReconciliation` 提示“恢复状态需要对账”，不报虚假成功。

### 1.6 原生文件与保存路径选择器 (`SessionMigrationPort`)
- **问题**：导入弹窗曾使用目录选择按钮，无法直选 JSON 文件；导出保存路径未提供原生保存对话框。
- **修复**：
  - 在 `SessionMigrationPort` 中新增：
    - `pickPackageFile(): Promise<string | null>`
    - `pickExportPath(defaultName: string): Promise<string | null>`
  - 在 `src/shared/platform/tauri/feature-ports/sessionMigration.ts` 中桥接 `pick_session_package_file` 与 `pick_session_package_export_path`，并通过 zod 严格校验返回类型。
  - 在 `src/shared/platform/tauri/features.ts` 中补充桥接；在 `src/shared/platform/browser/features.ts` 中补充 `rejectNativeOnly`。
  - 弹窗按钮文案严格使用：“选择文件”、“选择保存位置”。

---

## 2. 导出纯 Helper 清单 (`src/shared/features/session-migration.ts`)

| 纯 Helper 函数 | 职责说明 |
| :--- | :--- |
| `getSessionStableKey(session)` | 生成会话全局唯一稳定键：`${providerId}::${sourcePath}::${sessionId}` |
| `computeRestoreBindingKey(params)` | 计算恢复请求确定性绑定指纹，支撑重试复用同一 `requestId` |
| `parseMigrationError(error)` | 结构化错误统一解析器，从 Error/JSON 字符串解析代码与中文信息 |
| `isProviderRestoreSupported(probe)` | 校验本地探针是否支持恢复写入，提供中文拦截原因 |
| `filterSnapshotsForProvider(sessions, providerId)` | 根据目标 Provider 严格过滤包内匹配的快照集合 |
| `validateBatchExportSessions(targets, previewSession)` | 批量导出预校验，拦截未决/异常会话并列出明确阻断原因 |

---

## 3. 本地验证与真实检查证据

所有检查均在隔离工作树通过 `rtk` 与 `mise run` 执行，真实结果如下：

```bash
# 1. 静态代码质量检查 (ESLint)
rtk mise run lint
# 结果: 0 errors, 0 warnings (PASS)

# 2. TypeScript 严格类型检查 (tsc --noEmit)
rtk mise run typecheck
# 结果: 0 errors (PASS)

# 3. 会话迁移模块全量单测 (Vitest)
rtk mise run test:unit tests/session-migration/
# 结果: 6 test files passed, 38/38 tests passed (100% PASS)
#   - tests/session-migration/schema.test.ts (12 passed)
#   - tests/session-migration/fixture-contract.test.ts (4 passed)
#   - tests/session-migration/tauri-port.test.ts (4 passed)
#   - tests/session-migration/pure-helpers.test.ts (5 passed)
#   - tests/session-migration/import-dialog-behavior.test.tsx (10 passed)
#   - tests/session-migration/page-behavior.test.tsx (3 passed)

# 4. 导航既有单测回归 (Vitest)
rtk mise run test:unit tests/renderer/widgets/app-shell/SideNavigation.test.tsx
# 结果: 1 test file passed, 10/10 tests passed (PASS)

# 5. 代码格式检查 (Prettier)
rtk pnpm prettier --check 'src/pages/sessions/**/*' 'src/shared/features/session-migration.ts'
# 结果: All matched files use Prettier code style! (PASS)
```

---

## 4. 交付文件清单

- `src/pages/sessions/Page.tsx`：冻结批量导出、稳定键、精准恢复匹配、状态隔离与兜底工作区。
- `src/pages/sessions/page.css`：现代暗色控制中心样式，弹窗与列表状态样式。
- `src/pages/sessions/components/ExportPreviewDialog.tsx`：冻结列表审查、原生文件保存、合规阻断。
- `src/pages/sessions/components/ImportPackageDialog.tsx`：幂等 requestId、探针门控、快照按软件过滤、原生文件选择。
- `src/pages/sessions/components/ConversationStream.tsx`：Q&A 问答流，保留完整轮次。
- `src/pages/sessions/components/StatusBanners.tsx`：状态横幅与结构化错误反馈。
- `src/pages/sessions/components/AttestationDialog.tsx`：主观自报续聊确认。
- `src/pages/sessions/components/StartupGuideDialog.tsx`：原生软件只读启动指引（统一 grokbuild）。
- `src/pages/sessions/components/RemapWorkspaceDialog.tsx`：工作区重映射保真提示。
- `src/shared/features/session-migration.ts`：Zod 契约、类型定义、6 个核心纯 Helper 函数。
- `src/shared/features/ports.ts`：新增 `pickPackageFile` 与 `pickExportPath` 接口。
- `src/shared/platform/tauri/feature-ports/sessionMigration.ts`：Tauri IPC 真实桥接与类型校验。
- `src/shared/platform/tauri/features.ts`：Tauri 特性桥接注入。
- `src/shared/platform/browser/features.ts`：浏览器降级与防护绑定。

---

## 5. 已实现内容 vs 未验收事项划分

| 模块 / 环节 | 当前状态 | 验收主体与方式 |
| :--- | :--- | :--- |
| UI 组件、交互状态流与 Dialog 规范 | **已实现，代码与单测通过** | 前端自检 (Vitest DOM 测试) |
| 重试幂等、快照过滤、探针门控逻辑 | **已实现，纯函数与组件测试通过** | 前端自检 (纯 Helper 测试与组件测试) |
| 批量导出冻结预校验机制 | **已实现，模拟校验通过** | 前端自检 (`page-behavior.test.tsx`) |
| 跨平台/真实浏览器端 E2E | **未验收** | QA 独立负责（`tests/browser/session-migration.spec.ts`） |
| 后端真实 Tauri 命令与数据库落盘 | **未验收（前端依赖后端协同）** | 后端与协调方集成验收 |
| Windows/多端原生实机环境联调 | **未验收** | 用户已确认暂无 Windows 物理机，略过实机验收 |
