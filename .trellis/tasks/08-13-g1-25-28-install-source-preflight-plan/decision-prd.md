# G1-04/G1-05/G1-06/G1-07 组级决策 PRD（问题清单版）

## 一句话组级结论
把安装链路拆成“来源信誉层、包完整性层、环境可装层、安装计划层”四个独立契约，前端只展示可读状态和决策依据，不用任何单一绿灯掩盖来源合规与可安装条件。

## 一、问题清单（已证据化）

### P0-问题 A：来源与许可被混为“可安装”（#25）
- **现状**：
  - Issue #25 目标是展示官方来源、许可与镜像来路；仍未明确收敛到现有状态契约。
  - 当前 代码中 `TrustedDownloadEndpoint` 仅固定官方端点，且 `StartInstallRequest` 仅传 `expected_release_id`（未传 URL 或签名白名单对象）；表面上可防止手工篡改路径，但并未把“可否缓存/可否再分发”落库到可读可回滚字段。
- **待定**：
  - 是否把“许可与镜像授权”作为独立 `source_trust_state` 与 `distribution_allowed` 字段持久化并纳入 UI 判定？
- **阻塞项**：
  - `Issue #25` 需先定义数据模型与可信源标签词典。

### P0-问题 B：包完整性来源分层不清（#26）
- **现状**：
  - #26 已有补充，明确“FyAgent 计算 hash ≠ 供应商签名/撤回证明”；代码侧存在下载后 hash 验证与签名验证流程入口（`run_install_flow`）。
  - 但 `RunInstall` 流程目前以 release metadata 驱动，缺少“vendor manifest / platform signature / withdrawal record”三类可回读字段的完整分层。
- **待定**：
  - 是否要把“vendor_manifest、platform_signature、revocation_record”作为 `PackageTrustLevel` 的组成部分，还是仍以“hash通过”作为入场门槛。

### P0-问题 C：环境预检能力未能表达 `unknown`（#27）
- **现状**：
  - #27 补充强调预检结果应该是 `pass/fail/warning/unknown`，且 `unknown` 不能自动降级为 pass。
  - 当前安装前链路已有 preflight 阶段，但缺少一套与 Issue 要求一致的 error-code 分层（OS/arch/权限/网络/账号/远程）规范。
- **待定**：
  - 预检失败时是否要求“阻断安装并可手动导向修复建议”，还是“允许继续但高亮警告”。

### P0-问题 D：不可静默变化的安装计划未落位（#28）
- **现状**：
  - #28 要求 `plan` 锁定 `agentId/version/source/hash/actions-summary`；任一变化需重预览。
  - 代码已有 `RestartPlan` 与 `plan_revision`，并包含去重与 reason 标注，但目前并非“安装执行前一致性契约的用户可读层”展示。
- **待定**：
  - 是把执行期 `plan_revision` 与安装计划 hash 统一为同一 `snapshot_id`，还是保持分层（前端 snapshot 与后端 plan_revision）并通过 `reconfirm_id` 串联。

### 额外：已识别的 stale-reference
- 当前 #25-#28 的正文与 25/26/27/28 之间关系链没有直接引用 #35/#41/#49/#50/#51 的旧方向。
- 需要“清理清单”中的条目：
  1. 若有后续实现提交草稿中仍写“可见字段可表示可复用 model 请求”等 wording，应回退到 `UNVERIFIED` 或 `NEEDS_RUNTIME_CHECK`；
  2. 避免把未联网模型预检、人工挑战、或 WorkBuddy 内部人工验证结果写成“已完成”证据；
  3. 在 UI 文案中移除“静默成功/自动修复”的措辞。

## 二、Issue 依赖链（1-hop）与流程顺序（可执行顺序表）

```text
#22（目录事实）
   → #25（来源/许可）
     → #26（hash/签名/撤回）
       → #28（不可静默变化的安装计划）
   → #27（环境预检）
     → #28（不可静默变化的安装计划）

#28（输出）
   → #29（普通用户主程序+窄权限 helper）

#26（额外阻塞）→ #70、#74（安全来源/隔离区完整性专题）
#27（额外阻塞）→ #31（多份安装与版本策略）
#28 还依赖 #57（用户确认与漂移重做）
```

## 三、前后端/数据契约分离（四层模型）

1. **Catalog/source metadata 层（#25）**
   - 输入：官方入口、发布方实体、许可边界、镜像授权。
   - 输出：`official_landing_url`、`resolved_download_host`、`package_source_kind`、`redistribution_allowed`、`legal_scope`。

2. **Package trust 层（#26）**
   - 输入：vendor manifest、platform signature、签名者、撤回记录、计算 hash。
   - 输出：`package_trust_level` 与 `verification_summary`。

3. **Environment fact 层（#27）**
   - 输入：OS/arch、磁盘、权限、账号/地区/远端能力。
   - 输出：`preflight_codes[]`（含 pass/fail/warning/unknown）。

4. **Install plan 层（#28）**
   - 输入：前三层输出 + 实际执行路径摘要。
   - 输出：`plan_revision/snapshot_id` 与 “不可静默变化”检查表。

**当前代码对接线索（未改造前提下）**：
- `src-tauri/src/services/codex_desktop/mod.rs`：`check_latest`、`start_install`、`run_install_flow` 已有 release-id 锁定与预检、下载校验、安装入场路径。
- `src-tauri/src/codex_desktop/restart_plan.rs`：`RestartPlan` 已具备 `plan_revision` 与目标归并逻辑，具备与“不可静默变化”衔接的天然承载位。
- `src-tauri/src/codex_desktop/types.rs`：`TrustedDownloadEndpoint` 固定源、`StartInstallRequest` 仅用 `expected_release_id`，说明前端尚未拿到完整 source/trust snapshot。
- `src/lib/hooks/useCodexDesktopInstaller.ts`：目前前端只拿 `releaseId` 驱动 startInstall，符合“不在前端传 URL”边界。

## 四、可选方案对比（组级）

### 方案一（推荐）：层级契约收敛
- 做法：保留现有安装执行路径，新增“来源/许可层”和“安装计划快照层”两个新 API 字段与前端渲染层；保留 #26/#27 的 preflight 检查码分层。
- 优点：改动集中、对现网回归小、能兼容现有 `expected_release_id` + `RestartPlan`。
- 风险：新增数据模型字段需要一次统一 migration 和旧数据回填。

### 方案二：先做 UI 层重写（不改后端契约）
- 做法：后端保持现状，仅靠 UI 文案解释来源/签名/预检不足。
- 优点：短周期。
- 风险：仍会有“绿色幻觉”；不满足本组“不可静默变化与可回读证据”要求。

### 方案三：先暂停前端，仅上后台审计
- 做法：只加服务端字段与日志，不改现有 UI。
- 优点：内部证据先行。
- 风险：用户仍无法判断状态，无法作为发布决策直接采用。

**推荐：方案一。**

## 五、PRD 交付目标（本组）

### 推荐保留/合并
- 保留 #25 #26 #27 #28 作为主链；不拆分。

### 建议收窄
- #25：`来源展示` 保持“官方来源可核验”，但默认不承诺“可托管/可镜像”除非 `legal_scope` 明确。
- #26：默认状态值增加 `integrity_available` 与 `integrity_unknown` 两档，避免统一“通过”。
- #27：新增 `unknown` 支持，并强制带 `checked_at + source`。
- #28：把“执行计划 + hash 锁”改为“用户可复核 snapshot”。

### 建议改名（可选）
- #25 -> `official-source-and-rights contract`
- #26 -> `package integrity and revocation contract`
- #27 -> `environment preflight contract`
- #28 -> `install snapshot + non-silent change contract`

## 六、PRD 任务分配（执行者级别）

> 说明：该组下游任务仅允许 **赖永杰** 单人执行，不再多人并行分派。

### 负责人（单一）
- Owner：`赖永杰`
- 任务：
  - 批准并落地 `InstallPlan` 四层契约定义与字段字典；
  - 按 `#25`/`#26`/`#27`/`#28` 顺序实施；
  - 统一在 GitHub Issue 与 Git 仓库提交记录同步进度与证据。

## 七、交付后建议启动的任务（你说的“分给执行者去做”）

1. 实现故事：catalog metadata schema v1（#25）
2. 实现故事：integrity graph + signer/callback 缓存策略（#26）
3. 实现故事：environment preflight 码表 + unknown 规范（#27）
4. 实现故事：安装计划快照与变更重确认（#28）

每项均给出验收项：
- 能读（Read）：字段可展示可回读；
- 能拒（Reject）：变更后要求重确认；
- 能查（Trace）：错误码可指向来源。

## 八、未决主会话确认点（需要你拍板）
1. #25/#26/#27/#28 的字段命名是否采用 `*_state/*_code` 统一前缀？
2. #28 的“重确认”触发阈值是否包括 signer 变更与 remote preflight 的 warning。
3. 对“可缓存/可镜像”是否允许先进入“待授权灰色可下载”而非“封禁”样式。

## 九、提交与同步（回传）

这份 PRD 已形成链路决策文档，请按以下执行方式推进：
  1) 同步发布到 GitHub（注释/任务备注）；
  2) 任务下发与验收只在 Git 里走 Issue / PR / 提交链路，不再外部多角色分发。
