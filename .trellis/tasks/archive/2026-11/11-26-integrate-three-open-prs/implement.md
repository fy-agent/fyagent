# 三 PR 集成与验收执行手册

> **待审，不执行。** 只有用户明确批准本版方案后才调用 `task.py start`、继续合并、编写集成验收脚本、安装依赖或运行测试。要求补充细节、描述执行顺序、审阅文档都不构成批准。目前现场见 [paused-state.md](paused-state.md)，不自动清理或重建。

## 1. 复工校验、两次合并与定向移植

以下是批准后的 PowerShell 命令。各阶段分别执行并检查 `$LASTEXITCODE`；不要复制成跳过错误的批处理。两个目录用途不同：任务记录留在原 checkout，新工作树只用于集成和产物构建。

```powershell
$Repo = (Get-Location).Path  # 从原 checkout 根执行
$Review = Join-Path (Split-Path $Repo -Parent) 'fyagent-three-pr-review'
$Evidence = Join-Path (Split-Path $Repo -Parent) 'fyagent-three-pr-evidence'
$Base = '82d6204908ea28d2dfb8a8af61cc7b67b65ea161'
$Pr443 = '65d3c40b576cb5a53166c37970ff566a021b191e'
$Pr444 = '86e7ea784d809dc582c655803dc6e6165db12cdd'
$Pr198 = '367c62ec4d5a746aed4c739459fc3f38ecc80f5c'
git -C $Review status --porcelain=v1 --branch
git -C $Review rev-parse HEAD MERGE_HEAD
git -C $Review diff --name-only --diff-filter=U
git -C $Review diff --cached --stat
```

1. **#443 已有本地提交，不再合一次。** 用 `git -C $Review show -s --format='%H %P %s' HEAD` 确认 HEAD 与暂停记录相同，父提交正好是 `$Base` 和 `$Pr443`。确认 #444 的 `MERGE_HEAD` 等于 `$Pr444`；检查已暂存和未暂存区域，特别是已出现的 `workspace-transfer.toml` 改动。只在能逐项解释保留改动时继续；现场不符则停手交人核对。若获明确同意从头重走，使用新 worktree/分支，不覆盖现场，只对 `$Pr443`、`$Pr444` 执行 merge。
2. **完成 #444。** 按 [逐文件裁决](design.md#冲突逐文件裁决)处理四个路径；检查工作区迁移权限、命令和 typed port 集合。先 `git add -- <已审查路径>`，再确认 `git ls-files -u` 无输出、`git diff --cached --check` 成功，最后提交，提交信息注明 PR 号。不要再对处于合并中的 worktree 发起一次 merge。
3. **移植 #198 仍缺失的行为。** `$Pr198` 仅用于来源比对，不 merge、不全量 cherry-pick。按 [P198-D/E](pr198-review.md) 在当前实现做失败前/成功后回归，或证明现有代码已等价；只提交相应生产改动、测试与实际受影响的契约，注明原 PR 与 SHA。P198-A/B/C 保留现有实现，P198-F 不复制老摘要。每个行为记为主线已有、移植、或有证据不采纳。
4. **每次提交都验证交付清单。** 从实际 `git diff --cached --name-status`、未暂存/未跟踪路径及被引用运行依赖建立清单；检查新增与删除、配置/锁文件、测试/文档；若发现新增迁移，核对实际 SQL 与注册入口。凭据、私钥、缓存不入库。提交前用 `git write-tree` 保存暂存树 ID，提交后用 `git rev-parse 'HEAD^{tree}'` 比对；用 `git diff-tree -r --name-status HEAD^1 HEAD` 核对清单；有疑问以 `git ls-tree HEAD -- <路径>`、`git hash-object --path=<路径> <路径>` 对比归一化后的 blob。记录断点用的 commit、tree、两父 SHA、处理决定和测试 ID。
5. 基线与 #443/#444 三个 `git -C $Review merge-base --is-ancestor <固定 SHA> HEAD` 须退出 0；#198 以逐项行为/替代提交记录验证，不要求其旧 head 成为祖先。确认工作树干净、没有 `MERGE_HEAD`，且 `git -C $Review diff --check $Base HEAD` 成功。全套测试与编译绑定同一干净 HEAD；修正源码后旧日志标为 `SUPERSEDED`。

## 2. 助手先跑：命令行验收

这些桌面功能通过 Tauri IPC，不是 `fyagent.exe` 的子命令。命令行自动化从现有 fixture 进入 Rust 服务或渲染交互，不添加临时公网接口、不在日常账号中调用生产 IPC。发布给用户的验收脚本放在任务目录、日志在 `$Evidence`，不要求把它们混入产品提交。

主协调器依[低资源方案](resource-plan.md)监控 CPU、内存、commit 与磁盘，统一授予一个重任务槽；子代理并行分工读代码与分析本轮日志，不独立开重命令。Playwright 聚焦命令限定 `--workers=1`。编译每次结束必须做产物保全和清理，实际 WSL 测试结束必须走专用 VHDX 的压缩门禁。

| ID | 在 `$Review` 内执行 | 必须观察到的结果 |
| --- | --- | --- |
| E0 环境 | 审阅最终配置与执行脚本；若未信任，由操作者执行 `mise trust`；再跑 `mise run bootstrap`、`mise run system:check` | 锁定工具/依赖、MSVC、WebView2 可用。暂停前的通过结果不归到最终 HEAD。 |
| T1 权限与清理 | `mise run test:unit tests/renderer/platform/tauriAclContract.test.ts tests/remainingPlatformSurface.test.ts`；`mise run rust:test application_acl_covers_every_registered_command_without_remote_access` | Tauri handler、权限和前端边界一致；新命令是本地权限，退役 Proxy 不可达。 |
| T2 WSL/SSH fixture | `mise run test:unit tests/renderer/features/workspace-transfer/WorkspaceTransferDialog.test.tsx`；`mise run rust:test remote_push::` | 档案保存/回读/删除、自定义路径、非法端口、WSL 列举/预览/应用状态、脚本备份/权限和不展示伪凭据。明确列出实际运行的用例；macOS 专属 SSH 端到端用例不是 Windows 证据。 |
| T3 Keenable | `mise run test:unit tests/renderer/features/mcpCatalog.test.ts` | 官方 HTTP 目录项；无 Key 不发 Authorization；有测试 Key 生成 Bearer 头且不进搜索文本。不得联网验证真实服务，除非单独满足 H4。 |
| T4 模型边界 | `mise run rust:test batch_setup_preserves_unrelated_user_fields_in_opencode_and_omp` | 用临时 home：改选定 provider 项，保留无关 provider、字段。用原断言；不以模型界面文案代替文件回读。 |
| T5 会话 | `mise run test:unit tests/session-migration`；分别 `mise run rust:test codex_probe_requires_native_initialization_without_mutating_store`、`mise run rust:test parse_session_uses_real_prompt_after_structured_context`、`mise run rust:test display_title_skips_mixed_unknown_and_malformed_provenance` | 不写入未初始化 store；打开导入重新 probe；Gemini 已验 writer 不被提取门误禁；中文错误焦点/可见性；标题为真实提示。若当前 commit 有等价测试，记录真实替代名称和断言，不接受零匹配。 |
| T6 Claude 归属 | `mise run rust:test homebrew_`，核对日志里 `user-helper` 的 `homebrew_bin_can_resolve_to_same_prefix_official_npm_package` 和 `homebrew_label_does_not_admit_other_owners_or_prefixes` 是否执行 | 同 prefix 官方 npm 路径为 npm，Cellar/Caskroom、异 prefix 及外部管理器不算 npm。Windows 只证明 fixture；未编入本目标时标记未运行，转 H7。 |
| T7 浏览器交互 | `mise exec -- pnpm exec playwright test --config config/playwright.config.ts --project chromium-1232x700 --workers=1 tests/browser/agents-models.spec.ts --grep 'TopBar migration and SSH push dialog'` | mock IPC 下的浏览器交互、可访问角色与预览流程；Chromium 缺失时审查后 `mise exec -- pnpm exec playwright install chromium` 再跑；保留失败 trace，不当原生证据。 |
| T8 总门禁 | `mise run check` | 环境、前端、后端、契约全部成功才记总门通过。运行某个聚焦步骤失败仍记录已能运行的独立项；不掩盖总门失败。 |

脚本规格：给定 worktree 与外置证据目录；开跑/收尾校验 HEAD、tree 和干净状态；每步留命令、实际退出码、命中测试名/计数、UTC 时间及日志文件。Rust 原生命令一次一个过滤式（`mise run rust:test` 不接受多个过滤式）；将 `0 tests`、被平台排除、提前跳过分别写入结果；工作区套件中其他包报告 `0 tests` 不影响确实命中预期包/测试的步骤。模拟数据仅用假配置、假凭据、临时目录/内存库；不继承测试结果中不可披露的账号数据。脚本遇到身份不符、零测试、必需检查失败或无法拿到本次构建产物，应非零退出并指明失败步骤。GUI 实测与外部连接单独交证据，不因脚本退出 0 自动变成通过。

脚本拟定入口（批准后实现）：`python <任务目录>/verify-integration.py --worktree <集成目录> --evidence-dir <外置证据目录>`。首个阶段输出 `source.json`（HEAD、tree、固定祖先），之后每项一个独立日志与 `results.json` 条目（ID、输入、命令、退出码、实际测试、证据类型、耗时、失败原因）。默认按 E0 → T1–T8 → 编译运行；`--only <ID>` 用于复跑指定项，不覆盖原日志，不把单项标成总门通过。独立阻塞不阻止其他可安全执行的项，但任何必需项未通过，脚本总结果不得为成功。

对命令行能直接观察的环境，助手批准后先完成：`wsl.exe --list --verbose`、专用发行版的普通用户/工具检查；若有批准的 SSH 测试别名，使用 H3 的严格主机校验命令测试认证与远端依赖。已具备隔离桌面和测试数据时，助手也可按 H1–H6 安全执行实际产品路径；只交接缺前提、需人工交互或需另一平台的项。命令行执行 WSL shell/SSH 握手只证明前提，不能替代产品预览、落盘和回读。

## 3. 编译与交付核验

T1–T8 必需项过门后运行 `mise run build:binary`。若门禁失败而确需诊断编译，可独立尝试，但**不标记为可验收体验版**。先记录构建起始时间，取构建日志中的目标和输出路径；预期项目产物名为 `fyagent.exe`，以实际日志为准。确认本次可执行文件确实落盘、时间/构建日志匹配且 Windows x64，计算 `Get-FileHash -Algorithm SHA256 -LiteralPath <本次exe绝对路径>`。如果运行还需要并列产物，也列入清单并逐一哈希。证据目录保存：

- `source`：仓库、确切分支/HEAD/tree、三项祖先证明、merge 父 SHA、#198 行为映射与移植提交、相对基线的 diff、逐文件裁决。
- `logs`：逐项退出码、测试计数、编译日志、原生与 mock 的证据类型、资源采样与峰值、产物封存/缓存清理及 WSL 压缩前后记录。
- `artifact`：产物路径、架构、版本、大小、SHA-256；运行前人工重新计算。不是签名安装包，不覆盖现有安装。
- `blockers`：验收 ID、当前状态、命令/错误与证据链接、安全前提、下一责任人、通过口径。没有 exe 时明确填「未产出」而不是填预计路径。

同一机器可在 PowerShell 用 `git -C $Repo log --first-parent --oneline $Base..integration/three-open-prs` 审查本地历史，或用 `git -C $Repo diff $Base <交付SHA>` 查看完整变化；此分支不推送。原生启动、核对文件与外部连通步骤见 [人工执行手册](manual-acceptance.md)。

## 4. 停点与回退

| 情形 | 处置 |
| --- | --- |
| 审批未给、暂停现场与记录不符 | 不执行。保留分支、索引和 worktree，提交人工差异清单。 |
| 新冲突涉及权限、目标路径或会话写入，无法保全双方约束 | 停在该 merge，提供三方差异、冲突文件和待裁决项。不要改用宽松 gate、猜测目标或隐藏失败。 |
| bootstrap/浏览器缓存/依赖不可用 | 附工具、版本和首个报错。可独立执行不依赖该前提的步骤；不绕过 lockfile、不清空全局缓存。 |
| 构建或产品测试失败 | 保留日志和输入 SHA；修复后重新验证受影响项及总门。同一错误不能用 mock 或旧产物覆盖。 |
| 真实环境缺失 | 按 H 编号记录 `BLOCKED` 与接手步骤；不在日常 Windows/WSL home 或真实远程主机补跑。 |

只有用户明确允许放弃暂停合并时，才在独立 worktree 执行 `git merge --abort`；会丢弃该次合并和部分冲突处理，不会删除已提交的 #443。清理整个 worktree、日志、测试账户或 VM 是另一步，不作为本次规划的自动动作。

## 5. macOS 交接与远端合并（另行授权）

1. 本机不跑 macOS，却不删共享或 macOS 代码。Windows 候选定版后，用 [Issue 草稿](macos-issue-draft.md) 准备确切 PR/HEAD 与脱敏日志；获发布授权后创建或更新 Issue、`@nonghua123`，回填 Issue URL。通知发出标 `PENDING_MACOS`，不是测试通过。
2. 推荐维护者以同一候选的 macOS 实测结论作为进队条件。本地 Windows 不因等待 macOS 停止交付可体验候选。macOS FAIL 或缺必需结果时不自动队列；需要有意接受延期风险时由维护者明确决定，仍不得绕过任何仓库保护。
3. **推荐单一候选 PR** 对 `main` 承载已审查的 #443/#444 集成与 #198 定向修复，不同时让 #443/#444 独立入队。提交前从实际改动和被引用依赖生成交付清单，合入前由获授权的维护者完成 [仓库合并治理](../../spec/backend/github-merge-governance.md#3-contracts) 的 SPEC、Trellis 验证、prearchive、archive、合约回读和基线漂移检查。推送后读回确切远端 HEAD、完整文件树（API 截断则补取）和必需 blob；只让对应候选进 Merge Queue 验 `CI / Required`，禁止直接推 main 或管理员绕过。候选 HEAD 变动则重新验证受影响项；若原 PR 先被合入，先重算候选差异和证据，再决定进队。
4. 获准远端合并后回读最终 `main` merge SHA，核对 #443/#444 贡献和 #198 定向修复已落地。按用户决定，#198 保持开放；此前的条件关闭建议不再是执行项。
