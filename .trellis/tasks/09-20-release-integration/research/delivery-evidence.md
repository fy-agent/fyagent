# Research: delivery evidence

- Query: 核对 PR #192 的 GitHub review/check 状态，以及 FDE 分支 `codex/fde-delivery-integration` 的提交、任务断言和保存日志，识别交付缺口并列出最终必需检查。
- Scope: mixed
- Date: 2026-09-20

## Findings

### 1. PR #192 的已验证阻断

- GitHub PR #192（<https://github.com/fy-agent/fyagent/pull/192>）当前为 `OPEN`、`draft=true`，head 为 `6a4af16d15e8c29f3588eb77e478d4bc995d0705`，base 为 `main@64f4d8f`。`reviewDecision` 为空，reviews 和 issue comments 均为空；因此没有可报告的 GitHub reviewer 结论或未解决 review thread。
- Hosted checks 在该 head 上只有一个实际阻断：`Frontend Checks` 失败，随后 `CI / Required` 失败；其余 PR 依赖 job 均成功（`Classify Changes`、`Repository Contracts`、`Desktop Acceptance Contract`、`Backend Checks (Windows)`、`Windows Native Contracts (X64)`、`Windows Native Contracts (ARM64)`、`Backend Checks (macOS)`、`Commit Convention`）。`Commit Convention / Push` 和 `label` 是附加成功项，不是 main 的 required status。
- 失败日志来自 run `35452775621` / job `105922867093`：636 个浏览器测试通过，但 `tests/repositoryWorkstationPathContract.test.ts` 失败，报告具体位置为 `.trellis/tasks/archive/2026-09/09-19-subscription-cross-agent/verification.md:78` 和 `:79`。断言要求文档中的两个具体用户 home/workstation 路径改成语义占位符。`CI / Required` 的最终 gate 日志明确记录 `tests: failure`；这是 actionable blocker，修复后必须推新 head 并重新跑依赖检查。
- 该 finding 与当前任务 design 的约束一致：`.trellis/tasks/09-20-release-integration/design.md:18-20` 要求保留隐私契约、将具体 workstation 路径换为语义占位符；任务 acceptance 也要求失败被修复而不是绕过（`prd.md:17-19`）。

### 2. FDE branch 的远端交付状态

- `codex/fde-delivery-integration` 当前远端 head 确认为 `6f38c2136bfea551225a6c207400e88ad94683e5`，父提交为 `8deadf79`；该 head 只做了归档 review 文件从 `reviews/` 到 `research/` 的规范化重命名，FDE 产品实现位于其祖先提交链（包括 `c6cfbe94`、`cd8b4ced`、`d2a5dc7d` 等）。现已建立 draft PR #193（<https://github.com/fy-agent/fyagent/pull/193>），base 为 `main@64f4d8f`，head 为 `6f38c213`。
- PR #193 的 body 已明确顺序“本 PR → #192”、schema/migration 由 #192 补齐、FDE 不含订阅复用实现，并保留真实账号、客户系统、Windows、Apple 公证、正式发布和真实客户验收边界。当前无 reviews/comments，`reviewDecision` 为空；没有需要处理的 GitHub review finding。
- PR #193 的 hosted run `35493454323` 正在进行：`Classify Changes`、`Repository Contracts`、`Desktop Acceptance Contract`、`Commit Convention` 已成功；`Frontend Checks`、`Backend Checks (Windows)`、`Windows Native Contracts (X64)`、`Windows Native Contracts (ARM64)`、`Backend Checks (macOS)` 仍 pending，因此 `CI / Required` 尚未产生最终结果。`Commit Convention / Push` run `35452915192` 和 `label` 已成功。FDE PR 尚不能称为 hosted checks 通过，需由主控跟踪该 run 完成。
- FDE 归档任务的 `task.json` 在 `6f38c213` 中为 `status=completed`、`branch=codex/fde-delivery-integration`，但 `base_branch=codex/macos-045-audit`、`pr_url=null`、`commit=null`；这是归档时的历史元数据，现以 GitHub PR #193 的 main base 和精确 head 为交付读回。PR #193 body 已补齐本任务要求的范围、顺序与 UAT 边界。
- 保存的控制证据 `<workspace>/tmp/fde-workstreams-20260919/control-evidence/final-git-delivery.json` 记录 `localHead=remoteHead=6f38c213`、工作树干净、`postarchiveContracts=passed`、`mainModifiedByTask=false`。这证明分支同步与本地交付控制，不替代 GitHub PR/check 证据。

### 3. FDE 本地证据的可用边界和需要避免的误读

- `<workspace>/tmp/fde-workstreams-20260919/control-evidence/integration-backend-closure.log` 是可引用的后续闭合日志：Rust 主包 `3344 passed; 0 failed; 5 ignored`，配套合同日志 `<workspace>/tmp/fde-workstreams-20260919/control-evidence/postarchive-contracts-final.log` 记录合同测试 `34 files / 650 passed / 1 skipped` 以及 native-fetch `4 passed`。这些是本机固定工作树证据，PR #193 body 当前以摘要方式引用了本地验证；仍应把 hosted CI 结果作为独立证据。
- 同目录的 `integration-backend-final.log` 仍记录 `3340 passed; 3 failed; 5 ignored`；`integration-contracts-prearchive.log` 也记录过 8 个失败。它们属于历史迭代证据，不能作为最终通过结论；当前 PR body 使用 `3670 passed` 的汇总时，应以 closure/final 日志与对应 commit 解释其来源。
- FDE 原生 handoff `<workspace>/tmp/fde-workstreams-20260919/native-uat/native-evidence-readback.json` 保留了 `local_fixture` 的 failed 记录和 `native_remote` 的 unknown 记录；`native-uat-progress.json` 的剩余项为空，但这只能说明本机验收演练按产品规则收敛，不能升级为真实客户验收或真实外部服务成功。归档 acceptance matrix 也明确客户验收仅本机演练，Windows、Developer ID 正式公证、真实客户系统仍未验证。
- 本地 evidence 目录在仓库外，没有随 `6f38c213` 进入 Git；因此 PR review 无法直接读取这些日志。PR #193 已提供归档任务和手册链接，但 exact local log readback 仍属于主控交付材料，不能替代 GitHub checks。

### 4. 当前仓库的精确 required-check 清单

- `main` branch protection API 当前唯一 required status context 是 `CI / Required`，`strict=false`，且该 check 绑定 GitHub Actions app；这与 `.trellis/spec/backend/github-merge-governance.md:69-94` 和 `.trellis/spec/backend/github-ci-workflow.md:19-41` 一致。
- `.github/workflows/ci.yml:required` 的 `needs` 明确要求以下依赖 job 在同一次 PR/merge-group run 中被评估：`Commit Convention`、`Classify Changes`、`Repository Contracts`、`Frontend Checks`、`Desktop Acceptance Contract`、`Backend Checks (Windows)`、`Windows Native Contracts`、`Backend Checks (macOS)`。Required gate 本身再读取这些结果；当前 PR #192 的展示名中 Windows native 被拆成 `Windows Native Contracts (X64)` 与 `(ARM64)`，两者都必须保持通过。对本次 FDE+subscription 合并，因跨 frontend/backend/native/contracts，最终 hosted 清单应至少包含：`CI / Required`、`Classify Changes`、`Repository Contracts`、`Frontend Checks`、`Desktop Acceptance Contract`、`Backend Checks (Windows)`、`Windows Native Contracts (X64)`、`Windows Native Contracts (ARM64)`、`Backend Checks (macOS)`、`Commit Convention`；`Commit Convention / Push`、`label` 可记录但不是 Required gate。
- `main` active ruleset `main-merge-queue`（API id `21271876`）当前为 `MERGE`、`ALLGREEN`，`max_entries_to_build=2`、`max_entries_to_merge=1`、`min_entries_to_merge=1`、wait `0`、timeout `30` 分钟；workflow 的 `merge_group: checks_requested` 仍存在。两条 PR 都应在各自最终 head 上先得到 PR checks，再按仓库 queue/merge-commit 策略处理；不要把当前 PR head 的绿/红结果转移到新 head。

## Caveats / Not Found

- PR #193 已存在，但当前没有 review thread，且 hosted domain checks 尚未全部结束；没有 GitHub review finding 可闭合，也不能把 pending run 视为通过。
- PR #192 当前 failure 的具体路径 finding 已从 GitHub annotation 直接读取；本报告没有重复进行代码修复，也没有修改工作树。
- 本地 FDE 日志来自 `<workspace>/tmp/fde-workstreams-20260919/`，其内容可能随本机临时目录变化；Git commit/remote API、PR check API 和保存日志的交叉证据分别记录，不能互相替代。
- 未检查真实账号订阅 entitlement、客户系统、Windows 实机或正式 Apple 公证；任务和 FDE acceptance 中这些边界仍应原样保留。
