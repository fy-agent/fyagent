# Trellis spec inventory

## Baseline

- Scope: `.trellis/spec/**/*.md`
- Files: **41**
- Lines before changes: **14,883**
- Backend: 24 files / 9,065 lines
- Frontend: 14 files / 5,263 lines
- Guides: 3 files / 555 lines
- Broken relative Markdown links found: 0
- Index omissions found: 0
- Exact duplicated substantial paragraphs across files found: 0

“无完全重复段落”不代表没有重复责任；主要问题是同一规则被不同措辞重复说明、
长期合同夹带历史快照，以及顶部 override 覆盖旧正文。

## Outcome after review

- Files after changes: **41**（未删除任何 contract）
- Lines after changes: **13,887**
- Changed specs: **21**
- Reviewed and retained without edits: **20**
- Net reduction: **996 lines**（926 additions / 1,922 deletions）

精简集中在索引、thinking guides、通用 reuse、开发环境和上游同步；安全、权限、
TOCTOU、secret、签名、回滚、原生平台与发布事务的 owning contracts 保持深度。

## Disposition

| Spec | Disposition | Reason |
| --- | --- | --- |
| `backend/application-brand-assets.md` | Update | 保留资产身份与验证，移除日期化 clean-break 叙述。 |
| `backend/application-identity.md` | Update | 保留身份/许可边界，将一次性迁移措辞改为稳定合同。 |
| `backend/change-plan-executor.md` | Retain | 跨层执行、幂等、补偿与部分结果合同不可概括替代。 |
| `backend/codex-desktop-installer.md` | Retain | MSIX、helper、bridge、签名与 TOCTOU 安全闭包。 |
| `backend/codex-provider-configuration.md` | Retain | Provider 写入、凭据身份、原生投影与迁移失败矩阵。 |
| `backend/deeplink-import-security.md` | Retain | 不可信输入、能力边界和导入安全合同。 |
| `backend/development-environment.md` | Condense | 删除工具版本与实现快照复制，保留权威来源、bootstrap、host 边界和门禁。 |
| `backend/development-hooks.md` | Update | Trellis 版本从 `.trellis/.version` 读取，不在 spec 固定当前值。 |
| `backend/external-agent-p0.md` | Update | 保留大型安全合同，仅清理日期化研究叙述和瞬时事实。 |
| `backend/fyagent-version-contract.md` | Retain | 应用版本、资产命名和一致性门禁的唯一 owner。 |
| `backend/github-ci-workflow.md` | Update | 保留 CI 分类、聚合、runner 和失败证据边界；移除日期化第三方嵌入叙述。 |
| `backend/github-merge-governance.md` | Retain | Merge Queue、任务生命周期与治理事务边界。 |
| `backend/github-release-workflow.md` | Update | 保留发布事务，工具版本改为与权威文件精确一致而非重复值。 |
| `backend/index.md` | Rewrite | 只承担阅读顺序、路由与维护规则。 |
| `backend/macos-dmg-layout.md` | Update | 保留 DMG 文件系统、Finder 布局和验证合同；Python 版本改读权威文件。 |
| `backend/main-window-layout.md` | Retain | 原生窗口几何与 renderer chrome 的跨层边界。 |
| `backend/modular-boundaries.md` | Retain | 后端模块职责与依赖方向稳定。 |
| `backend/reuse.md` | Retain | 后端复用优先级、封装和安全约束已集中且无功能快照。 |
| `backend/secretref-backend.md` | Retain | secret 存储、引用、DTO 和原生证据边界。 |
| `backend/task-runner-contract.md` | Retain | 本地任务 API、参数传输、host guard 和副作用策略。 |
| `backend/upstream-sync.md` | Condense | 保留不可变 tag/双亲 ancestry 合同；具体 v3.19.2 SHA 链接 provenance ledger。 |
| `backend/windows-installer.md` | Retain | NSIS、有界清理、签名、卸载和原生证据合同。 |
| `backend/windows-runtime-security.md` | Retain | elevated Bob / Explorer Alice、HKU、COM 与 helper 封闭边界。 |
| `backend/workbuddy-configuration.md` | Update | 保留 revision/backup/overwrite 安全合同，修正已过期的 Models keep-alive 生命周期。 |
| `frontend/component-guidelines.md` | Retain | 通用组件接口和可访问性边界稳定。 |
| `frontend/directory-structure.md` | Retain | 前端目录职责和 import 方向稳定。 |
| `frontend/hook-guidelines.md` | Retain | hook 所有权、生命周期和返回形状规则稳定。 |
| `frontend/index.md` | Rewrite | 明确基础规范、V2 合同和阅读顺序，不复制功能行为。 |
| `frontend/modular-boundaries.md` | Retain | renderer/host、V1/V2、feature/platform 边界。 |
| `frontend/quality-guidelines.md` | Update | 保留测试分层；DEP0040 精确版本/祖先链改由 executable checker/tests 唯一拥有。 |
| `frontend/reuse.md` | Condense | 删除历史 commit 和各 V2 功能合同复述，保留 owner registry 与放置规则。 |
| `frontend/state-management.md` | Retain | server/local/URL/secret 状态 owner 规则稳定。 |
| `frontend/type-safety.md` | Retain | unknown 边界、DTO parser 和禁止局部 cast 的规则稳定。 |
| `frontend/user-facing-copy.md` | Retain | 用户文案、证据强度和错误消息边界。 |
| `frontend/v2-agent-models.md` | Update | 合入当前 catalog-first 四分区配置壳，删除旧 direct-jump/installed-only 冲突条款。 |
| `frontend/v2-prompts-memory.md` | Update | 将 Agents prompts 委托与 reread 规则直接写入合同。 |
| `frontend/v2-shell.md` | Update | 将 V3 左侧导航、active-route-only 和 selection owner 合入正文。 |
| `frontend/v2-skills-mcp.md` | Update | 合入 Agents 委托，删除固定 handler 数量等瞬时事实。 |
| `guides/code-reuse-thinking-guide.md` | Condense | 只保留搜索/依赖/共享 owner 决策顺序，细节链接 owning specs。 |
| `guides/cross-layer-thinking-guide.md` | Condense | 只保留数据流、边界、authority、错误与 round-trip 检查。 |
| `guides/index.md` | Rewrite | 定义 thinking guide 角色和进入 code spec 的条件。 |

## High-risk contracts intentionally retained

以下文档即使较长，也不能用短摘要替代：Change Plan、Codex installer/provider、
External Agent、CI/Release、Task Runner、Windows Installer/Runtime、SecretRef、
WorkBuddy，以及对应 V2 功能合同中的 secret、readback、rollback、native-only、
auth 和 installer 失败矩阵。它们的长度来自风险与平台差异，而不是单纯重复。

## Review findings

1. **V3 override 收敛**：`v2-shell`、`v2-agent-models`、`v2-skills-mcp`、
   `v2-prompts-memory` 的日期化顶部补丁已直接合入正文；旧 direct-jump、
   installed-only 和顶部导航规则已删除。
2. **生命周期冲突修复**：现行 `PersistentPrimaryOutlet` 只渲染 active route，
   Models target 切换/离开路由会 unmount 并清理未保存 secret。该规则已同步到
   Shell、Agent/Models 和 WorkBuddy contracts。
3. **版本权威收敛**：Node/pnpm/Rust/Python/mise/Trellis 的具体值由各自配置文件
   和 lock 唯一拥有；generic specs 只要求精确一致。
4. **上游证据外置**：长期 spec 不再复制 v3.19.2 SHA。只读 `ls-remote` 与临时
   ref 核验了 ledger 的 annotated tag/peeled commit；two-parent merge、parent
   order、merge-base 和 ancestry 也匹配，临时 ref 已清理。
5. **未机械裁剪高风险合同**：Codex installer/provider、Windows、Release、
   External Agent、SecretRef、Change Plan 等仅在确认瞬时事实或冲突时小改。

## Validation evidence so far

- Round 1 structural audit：41/41 index coverage、无 broken relative links、
  无日期化 override/SUPERSEDES、无私有路径/分支标记、无 reviewed upstream SHA
  泄漏、无 trailing whitespace/额外 EOF 空行；`git diff --check` 通过。
- `mise run check:contracts:prearchive --exclude-active-task <task>`：
  docs/task/version/release/platform contracts 通过；聚焦测试 33 files，
  594 passed / 1 skipped；Native Fetch probe 4 passed。
- `mise run test:v2`：59 files / 418 tests 全部通过，覆盖 Agent catalog/config、
  route unmount、WorkBuddy draft/secret cleanup、Skills/MCP/Prompts authoritative
  readback。
- `mise run check:prearchive --exclude-active-task <task>`：完整门禁通过；包括
  TypeScript、Prettier、172 files / 1,555 passed / 1 skipped 的前端单元测试、
  i18n、desktop mock/visual preflight、Rust fmt/check/clippy、2,945 passed /
  5 ignored 的 Rust 主单元套件及全部集成套件、80 个 mise task 合同、文档、
  lock、supported-platform、version 与 Release 检查。Release 聚焦合同测试仍为
  33 files / 594 passed / 1 skipped，Native Fetch 4 passed。
