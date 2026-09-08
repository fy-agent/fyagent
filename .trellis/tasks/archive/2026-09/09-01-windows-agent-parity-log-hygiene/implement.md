# 实施计划：统一桌面 Agent 安装面、Windows 适配与 Codex 日志治理

> 本任务串行执行。每一阶段的退出条件未满足前，不进入下一阶段；禁止在 helper protocol、inventory 与 lifecycle policy 上并行改动。当前任务保持 `planning`，本次提交不启动实现。

## Phase 0：证据、复现与复用门禁

- [ ] 在真实 Windows 11 x64 上安装当前正式 FyAgent，复现正式构建与开发构建差异。
- [ ] 逐产品记录当前 Windows inventory/readiness/action 状态与真实失败，不根据用户描述直接猜根因。
- [ ] 对 QoderWork/TRAE Work/WorkBuddy/Codex 跑现有链路，区分“代码缺口”与“尚未完成 HIL”。
- [ ] 捕获 Claude 官方用户安装器与 x64/arm64 MSIX 的 package identity、signer、scope、Cowork 能力与 update owner 行为。
- [ ] 捕获 OpenCode Windows x64 当前 stable Desktop 资产，以及 current release 是否真实提供 ARM64 Desktop 资产；记录 redirect、signer、ProductName、Registry/App Paths、scope 与 updater 行为。
- [ ] 捕获 Grok official native/npm owner 的 candidate 类别、版本输出、安装与更新行为。
- [ ] 对新 ChatGPT 干净安装、旧 Codex 官方升级、ChatGPT Classic 并存做 package/application ID/AUMID HIL，判断现有 exact owner 是否需要闭集迁移。
- [ ] 用用户提供场景复现 Codex deferred 日志；记录父子 size/mtime/时间线水位与每轮日志数量，删除或脱敏本地路径和 ID。
- [ ] 更新 `research/windows-hil-baseline.md` 与产品复用矩阵；记录 G1/G2/G3/G5 决策证据。
- [ ] 任一产品官方渠道、identity、scope 或 owner 无法证明时，明确保持 fail-closed，不进入猜测实现。

**退出条件**：计划写入代码的 identity/owner 均有官方来源与 HIL 证据；Claude package owner、OpenCode architecture、Codex physical identity 与 Grok fresh-install owner 均有书面决策。

## Phase 1：先更新合同与失败测试

- [ ] 更新 Windows runtime security spec：formal build 只允许 policy 授权的闭集 helper action，不再笼统描述为全部 CLI 永久不可用。
- [ ] 更新 external Agent spec：当前 7 产品 surface 与 Windows capability matrix。
- [ ] 更新 Codex installer/package spec：若 Claude 复用 MSIX，明确共享 owner 与 product identity 参数化边界；补充新 ChatGPT/Classic exact identity 门禁。
- [ ] 更新 frontend Agent model spec：新增 reason/actionability 时保持四语言与 DTO parity。
- [ ] 新增失败优先的 protocol、source、inventory、deployment、identity migration 与日志预算测试。
- [ ] 不先改 UI 文案掩盖后端不可用。
- [ ] 将 Settings/Tooling lifecycle action 精确收敛为仅 Grok Build；删除 macOS/Windows 的 non-Grok install/update/manual-command surface，同时保留有明确消费者的只读发现/配置行为。

**退出条件**：测试能稳定重现当前缺口，并明确禁止通用命令 helper、elevated fallback、模糊 identity、假成功、non-Grok CLI installer 与国产三项 update。

## Phase 2：扩展现有 ordinary-user helper

- [ ] 在 `fyagent-user-helper` 共享 protocol 中增加闭集 product/action/result；保持旧 action 兼容或显式升级 protocol version。
- [ ] CLI parser 拒绝未知 action、未知 product/owner、额外参数、非规范 job/nonce。
- [ ] 沿用 one-shot pipe、action binding、PID/SID/session 验证、frame/message/time budget 与 error redaction。
- [ ] 选择唯一代码 owner，抽取或共享 Grok Windows candidate、owner 与 version normalization；删除重复表，保持其他平台和业务所需的只读 Settings/Tooling 行为不变。
- [ ] helper runtime 实现固定 Grok observe/install/update；捕获输出仅用于本地限长解析，不经 IPC 返回。
- [ ] 主进程新增薄 coordinator；formal Windows 与 development Windows 使用同一语义路径，避免长期双实现。
- [ ] 添加并发、超时、子进程清理、helper crash、nonce/replay、message order 与 frame limit 测试。

**退出条件**：正式 Windows Grok 全链路不再依赖管理员直接执行用户 CLI，protocol 中没有自由命令能力。

## Phase 3：补齐 Claude/OpenCode Desktop Windows

### Claude

- [ ] 根据 G1 选择并实现唯一 package owner。
- [ ] 若选择 MSIX：抽取或窄委托现有 Codex AppX/MSIX owner，复用 protected package bridge、explicit SID inventory、signature 与 package identity 验证。
- [ ] 若选择 EXE：扩展现有闭集 `agent-exe-install` product enum，复用 Authenticode、artifact pin、ShellExecuteEx 与 post-readback。
- [ ] 只填入 HIL 证明的 Windows identity、scope、launch target 与 version readback；不得猜测。
- [ ] 明确 vendor auto-updater、Store/MDM 与 FyAgent action 的唯一 owner，避免重复注册或并排安装。
- [ ] Cowork/Virtual Machine Platform 等前置条件只检测并返回 reason/official-page action，不自动修改系统或重启。

### OpenCode

- [ ] Source resolver 增加经过核验的 Windows x64 Desktop stable alias，保持 fixed owner、host allowlist、architecture token 与 release version freeze。
- [ ] ARM64 alias 只在 current release 真实存在 asset、signature/PE identity 通过且 native HIL 完成后加入。
- [ ] 扩展现有 EXE helper 的闭集 product enum，冻结 HIL signer/product identity。
- [ ] inventory 覆盖 Uninstall/App Paths/limited known paths，拒绝 alias-only、配置目录与伪造 EXE。
- [ ] fresh install 与 same-target update 都通过完整 baseline/post-readback 验证。

**退出条件**：Claude/OpenCode 在 Windows x64 的发现、安装、允许的更新与启动均由 authoritative readback 证明；未证明的 architecture 明确 fail-closed。

## Phase 4：既有 Windows 产品真实闭环

- [ ] QoderWork/TRAE Work/WorkBuddy/Codex 逐项执行 native HIL，按失败证据做最小修复。
- [ ] 不重写已通过的 source/inventory/helper；优先修正 identity、scope、post-readback、bootstrapper completion 或 reason projection。
- [ ] Qoder/TRAE/WorkBuddy 的 `update=false` policy 始终保持；本任务不得因 HIL 通过而开放 update。
- [ ] WorkBuddy 比较现有 signed EXE 与 Store package 的 exact identity/update/scope；只选择证据更完整的 owner，Store page 不能被报告为自动安装成功。
- [ ] Codex 回归 MSIX install/update/launch、explicit SID inventory、user/machine package 并存、cancel 与 failure。
- [ ] 覆盖新 ChatGPT clean install、旧 Codex upgrade、ChatGPT Classic coexistence；只有需要时加入 HIL 证明的 exact identity set，并验证 Classic 不会被误启动。
- [ ] 覆盖 0/1/multiple installations 与 selected-target stale protection。

**退出条件**：已有 Windows 代码不再只有 mock/CI 证明，真实 x64 formal-build 行为与 policy 一致。

## Phase 5：Codex session deferred 治理

- [ ] 把 parent timeline 未追到 fork 的字符串错误改为 typed pending reason。
- [ ] 分离 retry scheduling state 与 diagnostic emission state；retryable pending 重算不能清除 warning fingerprint。
- [ ] 定义不含用户路径的 fingerprint，区分仍增长、长期稳定与真实损坏。
- [ ] 复用现有 pending/cache/sync summary owner，实现 bounded retry 或 stable suspension；相关 evidence 变化时恢复评估。
- [ ] 删除 `mark_deferred` 的 per-file `warn!`；预期状态默认静默，debug 模式每轮最多一条 aggregate。
- [ ] 修正 deferred 存在时每轮重复 INFO 的放大路径。
- [ ] 真正异常按 fingerprint 去重并保留 `WARN`/`ERROR`。
- [ ] 加入 120 轮日志预算、restart first pass、parent catch-up、stable gap、fingerprint change 与 true corruption 测试。
- [ ] 验证 usage 不提前、不重复、不丢失导入，cursor/replay prefix/dedup 不变量保持。

**退出条件**：用户提供的日志污染不可复现，真实异常仍可诊断，parent 补齐后 usage 只恢复一次。

## Phase 6：前端与可操作状态

- [ ] 复用现有 Agent DTO/readiness/action flow；后端回读前不做乐观成功。
- [ ] 补齐必要 reason code 与 zh/en/ja/zh-TW 文案、类型和 parity 测试。
- [ ] multiple targets、helper unavailable、system prerequisite、unsupported architecture、installation verification failure 均显示明确状态。
- [ ] 不显示绝对路径、installer output、SID、package family、rollout/session 标识。
- [ ] Claude/OpenCode Agent CLI 仍无 action；Settings/Tooling 中除 Grok 外也不存在公开 install/update/manual-command action，有明确消费者的只读能力仍正常。

**退出条件**：UI 与 backend capability matrix 一致，没有“按钮能点但 formal build 必失败”的状态。

## Phase 7：自动化验证

在仓库锁定工具链环境中执行并记录；不得以本机全局工具替代项目命令：

```bash
pnpm install --frozen-lockfile
pnpm typecheck
pnpm typecheck:v2
pnpm lint:v2
pnpm test:unit
pnpm test:v2
pnpm test:i18n
pnpm build:renderer
node scripts/release/verify-windows-nsis-contract.mjs
cargo fmt --all --check --manifest-path src-tauri/Cargo.toml
cargo check --workspace --all-targets --keep-going --locked --manifest-path src-tauri/Cargo.toml
cargo clippy --workspace --all-targets --keep-going --locked --manifest-path src-tauri/Cargo.toml -- -D warnings
cargo test --workspace --features fyagent/test-hooks --locked --manifest-path src-tauri/Cargo.toml --no-fail-fast
```

- [ ] 现有 Windows native x64/ARM64 CI jobs 全部触发并通过。
- [ ] 不新增仅为绕过现有失败的平行 workflow。
- [ ] macOS backend/frontend 与 privileged helper 共享路径回归通过。
- [ ] 检查 production logs/error DTO 不泄露用户路径、rollout ID、raw output 或 token。

## Phase 8：正式 Windows HIL 与发布态验证

- [ ] 构建与正式发布一致的 Windows x64 NSIS 安装包，验证 manifest 为 `requireAdministrator` 且 helper 被正确打包/验证。
- [ ] 在 clean Windows 11 x64 standard-user 与 administrator session 完成所有产品矩阵和边界场景。
- [ ] 用旧版本产品完成允许的 update 场景，证明 same-target change 与无 side-by-side fake success。
- [ ] 验证 no Explorer、user switch、UAC cancel、helper kill、network failure、wrong signature、wrong architecture 与 installer timeout。
- [ ] 完成新 ChatGPT/Codex/ChatGPT Classic exact identity matrix。
- [ ] 执行 Codex 120 轮日志预算与 parent catch-up recovery。
- [ ] 对 current official ARM64 package 的产品执行 Windows ARM64 native HIL；其他产品验证 fail-closed。
- [ ] 将脱敏证据写入 `research/acceptance-evidence.md`；不得提交安装包、账户信息或用户 session 数据。

**硬门禁**：Windows x64 formal-build HIL 未完成时，任务保持 blocked/in-progress，不能以 CI 通过替代并归档。

## Phase 9：规范收口与归档

- [ ] 根据最终实现更新相关 specs：删除已失真的“formal Windows 全部不可用”描述，并固化“六项 Desktop、仅 Grok CLI、non-Grok Tooling installer 退场、国产三项无 update”。
- [ ] 在 task notes 记录实际支持/不支持矩阵、HIL 环境、package/update owner 与残余风险。
- [ ] 完成至少三轮评审：安全边界、复用/重复代码、产品 policy 与 Windows identity；日志预算与 correctness；native evidence 与 scope control。
- [ ] 运行 Trellis context、task docs、JSON/JSONL、focused/full tests 与 packaging checks。
- [ ] 提交、推送、PR/合并按项目流程处理。
- [ ] 仅在完成定义全部满足后归档任务。
