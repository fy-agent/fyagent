# Implementation Plan — macOS Agent install/update experience

## Execution policy

- 本任务保持 `planning`，只有用户明确批准后才运行 `python ./.trellis/scripts/task.py start 08-31-macos-agent-install-update-experience`。
- 一个分支串行推进。安装 owner、wire contract、system commit 与 UI projection 不拆成并行实现，避免出现两套临时 owner。
- 每个阶段按“characterization test → 最小重构 → 删除旧实现 → 独立评审”执行。
- 对 Apple entitlement、privileged helper：本任务 **不实现** 系统 `/Applications` 写入。证据不足或用户已把 helper 划到后续任务时，系统目标保持 disabled/manual，不用捷径冒充完成。
- Windows 只做回归检查，不修改功能语义。

## Phase 0 — Freeze evidence and owner map

### Work

1. 保存/补齐 characterization tests：
   - OpenCode Desktop 不在 registry，当前不可发现；
   - Codex equal-or-newer 会调用 launch；
   - fresh desktop install 默认 user Applications；
   - generic Agent job 无 bytes/terminal diagnostics；
   - Grok action 可在 UI 中失去失败终态；
   - generic Agent DMG 完整进入内存并二次写盘。
2. 为下载、bundle metadata、DMG transaction、launch、inventory、Grok process/package manager、speed projector 建立调用图，标记唯一 owner 与删除候选。
3. 冻结官方 product/source matrix：QoderWork、TRAE Work、WorkBuddy、Codex Desktop、OpenCode Desktop/CLI、Grok native/npm。
4. 验证当前 OpenCode 官方 release metadata、Apple Silicon/Intel exact DMG 入口、Bundle ID 与 asset 唯一匹配。
5. 固定任务 wire baseline：readiness v3、inventory v1、Agent action/job v2；决定 version bump 或 compatibility projection。
6. 初始化 signed HIL ledger，不用 mock 关闭权限/签名门禁。

### Deliverables

- characterization tests；
- 更新 `research/current-implementation-audit.md` 与 `acceptance-evidence.md`；
- final owner/deletion map；
- G1 reuse gate 与 G3 OpenCode source gate 初始结论。

### Exit review

- 每个计划新增模块都能指出替代/扩展的既有 owner；
- 若方案要求先复制 Codex downloader/transaction 再“以后合并”，退回设计；
- 若 OpenCode 问题被扩大为全盘扫描，退回设计。

## Phase 1 — One shared artifact transport

### Work

1. 从 Codex 下载 owner 抽取 crate-private shared core，或通过窄接口让 generic Agent 委托现有 core。
2. 保持 Codex dedicated product port 与 release/session authority，先迁移 Codex 调用并跑全回归。
3. 将 QoderWork、TRAE Work、WorkBuddy 的 macOS artifact transfer 迁移到 shared core。
4. 增加 OpenCode Desktop official source policy，但此阶段只完成 resolver/transport tests，不启用 UI action。
5. 删除 generic full-memory `fetch_artifact_bytes -> Vec<u8>` 生产路径和第二份完整 DMG 写入。
6. 让 protected finalized artifact 直接进入 existing managed DMG transaction。
7. 为 generic Agent job 发布 raw transfer telemetry：completed/total/attempt/sequence/timestamp。

### Likely files

- `src-tauri/src/codex_desktop/download.rs`
- `src-tauri/src/codex_desktop/runtime.rs`
- `src-tauri/src/agent_install/fetch.rs`
- `src-tauri/src/agent_install/desktop.rs`
- `src-tauri/src/agent_install/macos.rs`
- `src-tauri/src/agent_install/jobs.rs`
- `src-tauri/src/agent_install/types.rs`
- new crate-private shared artifact module（名称由 Phase 0 决定）

### Tests

- streaming memory behavior；
- known/unknown Content-Length；
- redirect host violation；
- retry/backoff/attempt reset；
- cancel before/during transfer；
- size cap；
- `.part`/temp cleanup；
- artifact revalidation；
- Codex no-regression；
- no second full artifact copy。

### Exit criteria

- Codex 与 managed desktop products 命中同一 transport/temp owner；
- generic DMG 内存不随 artifact 大小线性增长；
- transfer snapshot 有单调可测试的真实 bytes。

## Phase 2 — Shared bundle metadata, inventory and explicit launch

### Work

1. 从 Codex macOS bundle discipline 抽取 bounded `plutil -> JSON -> typed fields` reader。
2. generic managed desktop registry 改用 shared reader，删除手写 XML parser。
3. 保留 `/Applications`/`~/Applications` direct-child scanner 和 inventory normalization，不新增 Launch Services/global scanner。
4. 在 registry 增加 OpenCode Desktop policy：`opencode + desktop + ai.opencode.desktop`。
5. 增加 `cli | desktop` closed surface 到 backend domain、readiness、action request、job key 和 strict parser。
6. 保持 top-level 七产品 catalog；OpenCode CLI 继续复用 Tooling，Desktop 进入 managed registry。
7. 在现有 `platform::process_launch` macOS adapter 内实现 NSWorkspace application-open completion；删除命令行 `open` 细节但不新增第二个业务 launcher。
8. Codex equal-or-newer 返回 pure no-op/readback；install/update success 不调用 launch。

### Tests

- binary/XML plist；
- missing/wrong type/oversized output；
- same name wrong Bundle ID；
- symlink app/executable escape；
- `/Applications` and `~/Applications`；
- 0/1/multiple candidate；
- stale revision/target moved；
- OpenCode CLI-only/Desktop-only/both/neither；
- illegal product/surface combinations；
- explicit launch success/failure；
- Codex equal/newer no launch；
- renderer cannot send path/Bundle ID/args。

### Exit criteria

- 当前 `/Applications/OpenCode.app` 被 authoritative inventory 识别；
- 手写 plist parser 不再是生产 owner；
- 所有 desktop launch 只能由显式 action 触发；
- 按钮 backend allowed action 可表达，但 frontend 尚可留到 Phase 6。

## Phase 3 — Select and prove one `/Applications` system-commit adapter

**SKIPPED for this task (2026-08-31).** User deferred privileged helper / system Applications writes to a later Trellis task on the same branch. Do not implement Phase 3A/3B/3C production adapters. Keep `MacSystemApplications` disabled.

### Phase 3A — Apple native authorization spike

### Phase 3A — Apple native authorization spike

1. 为实际 Developer ID/signing pipeline 申请并验证 `com.apple.developer.security.privileged-file-operations` entitlement。
2. 在最小签名/公证 prototype 中接入 `NSWorkspace.requestAuthorization` + authorized `FileManager`。
3. 真实验证：
   - target absent fresh create；
   - exact replace existing app；
   - rollback/compensating restore；
   - cancel/deny/expired authorization；
   - Rust/Objective-C bridge；
   - macOS 12/current OS；
   - no user-scope fallback。
4. 记录 SDK/API 限制、entitlement provisioning 和 HIL。

**Decision A:** 六项全部通过则实现 `NativeAuthorizedSystemCommitAdapter`，跳过 Phase 3B。

### Phase 3B — Reviewed helper spike, only if A is insufficient

1. Dependency/security review：
   - `Blessed` for SMJobBless lifecycle（项目 min macOS 12）；
   - `SecureXPC` for typed authenticated XPC；
   - SwiftAuthorizationSample for signing/version/downgrade patterns；
   - Mist for real macOS 12+ packaging/reference。
2. 建立一个 Swift helper target 和共享 closed protocol；不复制业务 downloader/DMG resolver。
3. 最小 prototype：
   - app/helper Developer ID signing；
   - embedded helper + bless/install；
   - mutual code-sign requirement；
   - version/status route；
   - one harmless capability-bound operation；
   - replay/wrong signer/old version rejection；
   - notarized app HIL。
4. Prototype 通过后实现 `PrivilegedHelperSystemCommitAdapter`：
   - request only `operation_id + revision`；
   - backend-owned protected manifest/staging；
   - closed product → fixed `/Applications` basename；
   - no network/shell/arbitrary path/URL/command/delete；
   - exact commit/rollback/recovery result。
5. 删除未选择的 prototype/adapter；生产只能注册一个 `MacSystemCommitPort` implementation。

**Decision B:** helper 也不能安全通过时，system install action 保持 disabled/manual；不以 `~/Applications` 冒充成功。

### Tests/HIL

- capability expiry/replay；
- path containment/no symlink；
- product/destination closed enum；
- caller/server signing and downgrade；
- user cancel/auth deny；
- helper unavailable/old/communication failure（若适用）；
- source/target revision drift；
- no network/shell surface；
- macOS 12/current signed/notarized builds。

### Exit criteria

- G2 记录最终选择、证据和 rejected option；
- exactly one production system-commit adapter；
- `/Applications` action 仍 feature-disabled，直到 Phase 4 product transaction HIL 通过。

## Phase 4 — Exact destination desktop install/update

### Work

1. Desktop coordinator 接入 selected `MacSystemCommitPort`，不复制 DMG transaction。
2. Fresh automatic destination 改为 `/Applications/<fixed basename>.app`。
3. 现有 user/system candidate update 绑定 exact path/scope/revision；不迁移。
4. System commit 前 revalidate candidate、running state、transaction staging 和 authorization/helper status。
5. Post-install 必须重新 inventory readback：path、scope、Bundle ID、version shape、launch eligibility。
6. Readback failure 使用 existing rollback；无法证明恢复进入 recovery-required。
7. 逐产品接入：
   - QoderWork；
   - TRAE Work；
   - WorkBuddy；
   - OpenCode Desktop；
   - Codex dedicated port 对接同一 system-commit capability。
8. 移除 fresh user-scope 默认；历史 `~/Applications` 只保留 exact update。

### Tests/HIL

- five products fresh `/Applications` install；
- no `~/Applications` duplicate；
- system/user exact update；
- multiple candidate selection；
- app running；
- target drift after preflight；
- cancel before commit；
- failure during commit；
- post-readback failure rollback/recovery；
- signed Apple Silicon + Intel evidence。

### Exit criteria

- system action only enabled after signed HIL；
- no silent destination fallback；
- updates preserve exact location；
- install/update never launches app。

## Phase 5 — Grok distribution-aware persistent jobs

### Work

1. Extend Tooling observation to publish `NativeInternal | OfficialNpm` owner without exposing sensitive config/path。
2. Replace immediate lifecycle result with persistent Agent job snapshot/terminal result while reusing the existing process/package-manager executor。
3. Native check/update：
   - anchored executable；
   - `grok update --check`；
   - freeze version；
   - `grok update --version <V>`；
   - controlled env/proxy；
   - captured bounded stdout/stderr/exit/timeout；
   - post-observe executable/version/owner/layout。
4. Native fresh install：
   - fixed xAI official installer action；
   - no `curl | bash` from renderer；fetch script through constrained backend transport to protected temp, then execute locally with fixed argument shape；
   - official installer owns x.ai/GCS/arch/Rosetta/layout/self-check。
5. Official npm fresh/update：
   - explicit source selection；
   - anchored npm/package manager；
   - official `@xai-official/grok@<version>`；
   - reuse configured registry；
   - post-observe owner/version/PATH。
6. Remove shell-composed `native || installer || npm` automatic chain。
7. Native failure terminal may offer a new explicit npm action, but never auto-execute it。
8. Installer/updater no byte protocol → indeterminate progress + stage/elapsed/redacted log summary。

### Tests

- detect native/npm/ambiguous owner；
- native check/update success；
- x.ai success；
- x.ai blocked + official GCS fallback；
- both blocked terminal error；
- official npm explicit fresh/update；
- native failure does not call npm；
- npm owner does not call native updater；
- proxy/registry config；
- exit nonzero/timeout/cancel；
- stdout/stderr truncation/redaction；
- failed update preserves previous binary/symlink/owner；
- Rosetta source behavior HIL；
- mainland network HIL。

### Exit criteria

- “正在安装后消失”不再出现；每个 path 有持久 terminal snapshot；
- owner and source category 可解释；
- no random mirror/no automatic owner transition。

## Phase 6 — Frontend surfaces, progress and copy

### Work

1. OpenCode 同一卡片展示“命令行”与“桌面应用”两个 section，状态/action/job 独立。
2. 所有 desktop surface launch 文案严格为 **“打开软件”**；CLI 不显示。
3. install/update/no-op completion 不调用 launch command。
4. 从 Codex `snapshots.ts` 抽取 shared transfer projector；Codex/Agent 页面共用。
5. percent 一位小数；bytes/speed 共用 formatter；unknown total/unknown speed/terminal stale speed 正确处理。
6. Persist terminal failed/cancelled/rollback/recovery state，不在 refresh 后无提示消失。
7. System install UI 显示 `/Applications` 与管理员授权；adapter unavailable 时 disabled/manual，不显示内部 helper/XPC 术语。
8. Grok source picker/owner label：native（推荐）与 official npm（明确选择）；不显示为“镜像自动降级”。
9. 复用现有 Button、notice、lifecycle status、target picker；只有第二消费者和清晰 owner 时才抽 component。

### Tests

- exact “打开软件”；
- no implicit launch invocation；
- OpenCode four surface combinations；
- 37.44 → `37.4%`；
- speed update/stale/reset/terminal；
- unknown Content-Length；
- terminal persistence；
- system authorization states；
- Grok source selection and no automatic switch；
- small window/Chinese/accessibility/browser review。

## Phase 7 — Specs, deletion and three review rounds

### Spec updates

至少更新：

- `.trellis/spec/backend/external-agent-p0.md`
- `.trellis/spec/backend/codex-desktop-installer.md`
- `.trellis/spec/backend/macos-dmg-layout.md`
- `.trellis/spec/backend/reuse.md`
- `.trellis/spec/backend/modular-boundaries.md`
- `.trellis/spec/frontend/v2-agent-models.md`
- `.trellis/spec/frontend/reuse.md`
- `.trellis/spec/frontend/quality-guidelines.md`
- `.trellis/spec/frontend/user-facing-copy.md`

写清：surface contract、wire version、shared artifact owner、bundle metadata owner、launch owner、selected system-commit adapter、Grok distribution owner、shared transfer projector。

### Review round 1 — Architecture/reuse

- no second downloader/DMG transaction/parser/launcher/speed owner；
- Codex dedicated port preserved；
- OpenCode uses existing catalog/inventory；
- exactly one system-commit adapter；
- OSS dependency license/version/maintenance/build integration recorded。

### Review round 2 — Security/failure

- strict IPC fields；
- source/redirect policy；
- target revision/containment；
- authorization/helper peer/replay/downgrade；
- Grok owner preservation；
- log redaction；
- rollback/recovery/cancel semantics。

### Review round 3 — UX/HIL

- explicit launch only；
- one-decimal/real speed；
- terminal state visible；
- system destination truthful；
- OpenCode current app recognized；
- mainland network and signed hardware matrix complete。

## Negative scans

Final names may vary, but equivalent scans are mandatory：

```bash
rg -n 'fetch_artifact_bytes\(|collect_body\(|download_macos_dmg_bytes' src-tauri/src/agent_install
rg -n 'plist_string|<key>.*CFBundle' src-tauri/src/agent_install
rg -n 'Command::new\("open"\)' src-tauri/src
rg -n 'grok update.*\|\||@xai-official/grok.*fallback|fallback.*@xai-official/grok' src-tauri/src
rg -n 'sudo|with administrator privileges|AuthorizationExecuteWithPrivileges' src-tauri/src
rg -n 'toFixed\(|formatBytes|bytesPerSecond' src | sort
rg -n 'url:|path:|command:|bundleId:|destination:' src/v2/shared/platform src-tauri/src/agent_install
```

每个命中必须证明属于唯一 owner/测试 fixture；无法解释则继续收敛。

## Validation commands

先以 `mise tasks` 校验真实 task 名称，至少执行等价项：

```bash
mise run rust:fmt:check
mise run rust:clippy
mise run rust:test
mise run typecheck:v2
mise run test:v2
mise run test:v2:browser
mise run check:contracts
mise run check
python ./.trellis/scripts/task.py validate 08-31-macos-agent-install-update-experience
git diff --check
```

## Required HIL matrix

| Case | Apple Silicon | Intel | macOS 12 | Current macOS | Mainland network | Signed/notarized |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| OpenCode discovery/explicit launch | required | required/runner | required | required | optional | required |
| Codex install/update/no auto-launch | required | required/runner | required | required | required | required |
| Qoder/Trae/WorkBuddy system install | required | required/runner | required | required | required | required |
| exact-location update/rollback | required | required/runner | required | required | optional | required |
| selected system-commit adapter | required | required/runner | required | required | n/a | required |
| Grok native x.ai/GCS | required | required/runner | required | required | required | recommended |
| Grok explicit official npm | required | required/runner | required | required | required | recommended |
| transfer progress/speed/cancel | required | required/runner | required | required | required | recommended |

Intel 运行证据可来自受信 HIL runner；仅 cross-compile 不算运行证据。

## Stop conditions

出现以下情况时停止对应能力并回到 design review，不使用隐式 fallback：

- shared extraction 削弱 Codex release/session/security invariants；
- OpenCode official asset 无法唯一绑定 platform/arch/release；
- Apple native authorization 与 reviewed helper 均无法在 signed build 安全工作；
- helper 需要 generic shell/path/URL API；
- Grok owner 无法可靠观察或更新需要自动跨 owner；
- 只能通过解析日志猜下载百分比/速度；
- Windows contract 出现非预期改变；
- signed HIL 与 mock/unit result 冲突。

UI 在阻塞时保持 disabled/manual，并记录准确 reason；不得把用户目录安装、随机镜像或 npm 迁移包装为成功。

## Definition of done

- PRD 所有适用 acceptance criteria 有自动化或 signed HIL evidence；
- G1–G6 有明确 Passed/Blocked/Not-applicable 结果；
- selected system-commit adapter 是唯一生产 owner；
- old duplicate/fragile implementations 被删除；
- source/owner/destination/launch/progress behavior 与 specs 一致；
- task validation、full checks、negative scans、`git diff --check` 通过；
- task 尚未因 planning 文档本身而启动或修改产品代码。
