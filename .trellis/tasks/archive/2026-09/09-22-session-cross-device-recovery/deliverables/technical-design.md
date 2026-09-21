> 历史准备材料（superseded）：最终实现合同见 `.trellis/spec/backend/session-migration.md`，当前能力与验收见 `implementation/delivery-readiness.md`。下文的 src/v2、Codex inject-only 及早期 provider 缺口判断不作为当前产品事实。

# Session 跨设备恢复技术设计（v2，定向返工）

- **任务**：`.trellis/tasks/09-22-session-cross-device-recovery`
- **角色**：Cursor 高级技术设计执行者 ｜ **执行模型**：claude-opus-5-thinking-high
- **基线**：`codex/frontend-interaction-v3-1-20260826` @ `60d699fa`
- **范围**：仅设计文档。未修改生产源码，未提交，未运行任何 provider 写入。
- **v1 快照**：`evidence/technical-design-v1-reviewed.md`（Grok 4.7 评审所依据的版本）
- **返工依据**：`briefs/architecture-revision.md`、`design.md`「技术方案首稿必须修正」、`evidence/codex-probe/findings.md`、`evidence/codex-probe/request-capture/result.json`、`evidence/codex-probe/request-capture/isolated-home/sessions/2026/09/22/rollout-*.jsonl`、`decisions/receipt-storage-response.json`、`deliverables/grok47-final-review.md`

**证据纪律**：`文件:行号` = 本仓库已实现代码；`evidence/…` = 本轮落盘实测；`research/…` = 9-21 隔离实测。其余一律标未验证。reader 未读某字段 ≠ 磁盘没有该字段。Jev 输出是辅助语义裁定，不是正确性或验收证明。

---

## 0. 修订说明（v1 → v2）

| # | v1 的错误 | v2 的处置 | 依据 |
|---|---|---|---|
| R1 | 用"下一条 user 前最后一条 assistant"作为默认 final 规则，六家默认放行 `Positional` 导出 | **删除 Positional 等级**。只有版本限定 + fixture 实证的提取规则能标 final；无规则/缺信号 = `Unknown` = **任意轮阻断导出**，末轮无豁免 | `MessagePhase` schema 定义 `commentary` 就是中途播报；revision brief §1 |
| R2 | 把一问一答强塞进 `turns[]`，非末轮无答复一律判为坏数据 | **包体改为有序消息序列**，连续 user、中断后重发都能如实表达；"未完成"是派生事实不是错误 | revision brief §1 |
| R3 | 称 Codex 磁盘 rollout「无完成字段」、①–⑥全未验证、OpenCode 是唯一实测写入 | 用本轮 rollout 实测重写 §6.5 / §9.2 / §9.8：磁盘**确有** `phase` 与 `content_item_kinds`；注入/持久化/重启 resume/下轮请求均有证据；原生列表 API 为空、UI 未验；Hermes 同样有合成写入实证 | `evidence/codex-probe/**`、`research/…/hermes-synthetic-result.json` |
| R4 | `migration_key` 含 `source_session_id` 却声称目标 ID 变更后仍相等；对账拿目标新 ID 重算 | **拆成三件事**：来源身份（包内传递）、内容摘要（纯内容）、目标映射（仅回执）。对账走已知回执→目标 ID，永不反向重算来源键 | revision brief §3 |
| R5 | `migration_key` 单主键；"重试 = 新建另一份" | 主键改 `attempt_id`；另建**默认幂等 slot 唯一约束**（包快照 + 内容摘要 + 目标 store 身份）；另存副本是独立显式请求、独立映射，全部保留 | revision brief §4 |
| R6 | `imported` 一个阶段吞掉"写入"与"原生可见"；`advance_recovery_stage` 可被 renderer 推到任意阶段 | 六级分证 + `needs_reconciliation`/`ambiguous`；**删除 `advance_recovery_stage`**，用户确认只写 `user_attestation` 侧信息 | revision brief §5 |
| R7 | 身份字段"静默净化后继续算身份" | 身份字段非法 → **拒绝或仅做显示转义**，绝不净化后参与身份计算；正文永远逐字保真 | revision brief §7 |
| R8 | 写死 `SCHEMA_VERSION 20→21`；只靠 WebDAV skip 列表防止回执跨设备扩散 | 版本号实施时重读；补**设备/store 绑定失效**策略，覆盖普通 SQL 备份恢复 | `design.md`「回执存储裁定」、revision brief §6 |
| R9 | 错误码与测试合同分叉 | 统一 camelCase 单套；补 `nativeReadbackMismatch` / `reconciliationRequired` / `ambiguousNativeMatch` / `packageUnknownField` / `jsonTooDeep` | `grok47-final-review.md` §4、§具体改法 5 |

**未解决项**见 §12，不在本文档内假装收敛。

---

## 1. 现状事实基线

| 能力 | 位置 | 事实 |
|---|---|---|
| 七 provider 并行扫描 | `src-tauri/src/session_manager/mod.rs:58-94` | 每次全量重扫，无持久化 |
| 消息读取分派 | `mod.rs:96-116` | `opencode` / `hermes` 的 `sqlite:` 前缀走独立分支 |
| Tauri 命令 | `commands/session_manager.rs`，注册于 `lib.rs:2116-2120` | `list_sessions` / `get_session_messages` / `delete_sessions` / `launch_session_terminal` |
| 前端 v1 会话页 | `src/components/sessions/`，API `src/lib/api/sessions.ts` | 已接通。缺的是 v2 shell 一级入口（`src/v2/app/router.tsx:52` 仍是 `memory`） |

**与 final-only 冲突的硬事实：**

- `SessionMessage` 只有 `role`/`content`/`ts`（`mod.rs:30-37`）。七个 adapter 一致丢弃 message id、turn id、完成状态、模型元数据。
- `extract_text_from_item` 把 `tool_use` 合成 `[Tool: {name}]`（`utils.rs:88-95`）、把 `tool_result` 递归内联（`utils.rs:97-106`）；Codex adapter 另造 `role="assistant"` 的工具消息（`codex.rs:239-247`）。**展示 DTO 不可用于迁移导出。**
- `load_messages` 全系无大小上限（`codex.rs:205-269`、`opencode.rs:157-237`、`hermes.rs:199-200`）。
- `codex.rs:427-499` 已有一套"跳过 `# AGENTS.md` / `<environment_context>` / IDE 注入"的过滤逻辑，但**只用于推标题**，不用于消息读取。本设计把这个判据提升到提取层并用 `content_item_kinds` 加固（§6.4）。
- 配置备份不覆盖 provider 原生会话（`database/backup.rs`）。
- 终端拉起硬编码只支持 macOS（`terminal/mod.rs:27-29`）。Windows 侧"打开目标软件"为空白。

---

## 2. 架构选型

**候选 A（采用）**：有版本的单个 JSON 迁移包 + 复用现有 FyAgent SQLite 的一张小回执表。源列表仍走实时扫描，回执不存第二份正文。

**候选 B（否决）**：新建通用会话库 + 作业引擎（可借 `change_plans`/`change_jobs`，`database/schema.rs:1648-1700`）。

否决 B 的理由是证据而非偏好：

- FyAgent 会变成第二事实源，必须与七个私有存储双向对账。OpenCode 同一会话同时存在 JSON 与 SQLite 两份（`opencode.rs:26-49` 就是为此做的去重合并）；OpenClaw 的 sessionId 会因 daily/idle reset 变化。对账成本高且无产品收益。
- 作业引擎把一次用户显式触发的短操作变成异步作业，**增加**崩溃窗口而非减少。
- 内容边界只有用户原文与最终答复，无附件无工具日志，不需要对象存储或压缩归档。

**为什么回执放现有 SQLite 而不是 sidecar 文件**（`design.md`「回执存储裁定」，Jev 同向但不构成证明）：SQLite 买到的是本地行的原子更新、唯一约束闸和并发命令下的单行更新；sidecar 要自建锁与索引，且若落在会同步的应用数据里仍需另做一套排除，崩溃窗口并不更小。**买不到的是跨进程 exactly-once** —— 原生写入本来就不在本地事务内（§8）。

**不要同时保留 sidecar 与 SQLite 两套回执**：二者是同一回执的两个候选，叠用才是冗余。

---

## 3. 数据模型

### 3.1 包体：有序消息序列，不是问答对

```rust
// src-tauri/src/session_manager/migrate/model.rs

pub const PACKAGE_SCHEMA: &str = "fyagent.session.v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SessionPackage {
    pub schema: String,                  // 必须等于 PACKAGE_SCHEMA
    pub exported_at: i64,
    pub exporter: ExporterInfo,
    pub sessions: Vec<MigratableSession>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExporterInfo {
    pub app: String,                     // "fyagent"
    pub app_version: String,
    pub platform: String,                // "macos" | "windows" | "linux"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MigratableSession {
    pub snapshot_id: String,             // §4.4 来源 + 内容的语义快照
    /// 纯内容摘要。只由 messages 派生，见 §4.2
    pub content_digest: String,          // "fyc1:<64 hex>"
    /// 来源身份。原样传递，不参与 content_digest，见 §4.1
    pub origin: OriginIdentity,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_active_at: Option<i64>,
    /// 源工作目录的 basename，绝不写绝对路径。见 §7.5
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_label: Option<String>,
    /// **有序**消息序列。不成对、不补齐、不合并连续同角色消息。
    pub messages: Vec<MigratableMessage>,
    pub extraction: ExtractionReport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OriginIdentity {
    pub origin_id: String,               // §4.1 稳定来源标识；不是正文摘要
    pub provider_id: String,
    /// 来源软件的原生会话 ID。缺失时为 None —— 绝不用空串占位（§4.1）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    /// 仅当导出时真的探测到才写
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cli_version: Option<String>,
    /// 来源设备上该 provider 存储的身份（如 CODEX_HOME 的 installation_id）。
    /// 仅用于人工追溯与歧义提示，不参与任何自动合并。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub store_fingerprint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MigratableMessage {
    /// 从 0 起的稠密序号，等于在 messages 中的下标
    pub seq: u32,
    pub kind: MessageKind,
    /// 原文逐字。不摘要、不截断、不改写路径、不做任何净化。
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ts: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MessageKind {
    /// 经来源规则确认为真实用户输入（已排除运行时注入，§6.4）
    UserText,
    /// 经版本限定、fixture 实证的规则确认为该轮最终答复
    AssistantFinal,
}
// 包内只存在这两种。commentary / 工具 / 思考 / 附件 / 运行时注入一律不进包，
// 只在 ExtractionReport.omitted 里计数。
// 判不出来的不是"降级放行"，是导出失败（§6.3）。

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExtractionReport {
    /// 实际生效的提取规则标识 + 版本，如 "codex.rollout.phase-v1"
    pub rule_id: String,
    /// 该规则被 fixture 实证覆盖的 provider 版本区间，如 ["=0.154.0"]
    pub rule_verified_versions: Vec<String>,
    /// 没有后续 AssistantFinal 的 UserText 的 seq 列表。
    /// 派生事实，不是错误：连续追问、中断后重发都会落在这里。
    pub open_user_messages: Vec<u32>,
    pub omitted: OmittedCounts,
    pub source_path_family: PathFamily,   // posix | windows | unknown
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OmittedCounts {
    pub tool_events: u32,
    pub reasoning_blocks: u32,
    /// 被规则判定为 commentary / 中途播报的 assistant 消息
    pub commentary_messages: u32,
    pub attachments: u32,
    /// 运行时注入（开发者规则、AGENTS.md、environment_context 等）
    pub runtime_injections: u32,
    pub unknown_blocks: u32,
}
```

**没有 `finalConfidence` 字段了。** 包内出现 `AssistantFinal` 就代表"由 `rule_id` 在 `rule_verified_versions` 内实证判定"。判不出来的根本进不了包。这避免了 v1 里"写个 `positional` 然后照样导出"的漏洞。

### 3.2 恢复请求与回执

```rust
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreRequest {
    pub package_path: String,
    /// 每次明确用户操作生成；网络重试必须复用
    pub request_id: String,
    /// 只恢复这些 snapshot_id；空 = 全部
    pub snapshot_ids: Vec<String>,
    pub target_provider_id: String,
    /// 必须是用户通过文件选择器给出、且在目标机已存在的绝对目录
    pub target_workspace: String,
    pub request_kind: RestoreRequestKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RestoreRequestKind {
    /// 默认导入：占用幂等 slot。同一语义快照 + 同一本机目标 store 只成立一次
    DefaultImport,
    /// 另存副本：显式操作，独立 request_id/attempt_id，不占默认 slot；同一请求重试仍复用原行
    SaveAsNewCopy,
}
// 刻意没有 Overwrite。

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreAttempt {
    /// 主键。每次恢复尝试一行，永不复用。
    pub attempt_id: String,
    pub request_id: String,
    pub snapshot_id: String,
    pub request_kind: RestoreRequestKind,
    /// DefaultImport 才有值；SaveAsNewCopy 为 None
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idempotency_slot: Option<String>,
    pub content_digest: String,
    pub origin: OriginIdentity,
    pub target_provider_id: String,
    /// 目标 provider 存储身份，见 §4.3
    pub target_store_id: String,
    /// 写前预分配的 nonce；用于在目标不回 ID 时做对账锚点
    pub target_native_nonce: String,
    /// 目标返回的原生 ID。未返回时为 None，不猜。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_native_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_workspace: Option<String>,
    pub stage: RestoreStage,
    /// 同一行上的幂等重试次数（§8.4）。另存副本的同一 request_id 重试也复用行
    pub attempt_count: u32,
    /// 系统证据之外的用户自报，单独一栏，不参与 stage
    pub user_attestation: Option<UserAttestation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<MigrationError>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RestoreStage {
    /// 包已解析并通过 schema / 大小 / 提取规则校验
    PackageVerified,
    /// 已决定写、尚未确认结果。崩溃窗口在这里（§8）
    NativeWritePending,
    /// 目标写入调用返回成功
    NativeWritten,
    /// 用目标自己的读取通道确认历史可见（不是我们扫 JSONL）
    NativeReadbackVerified,
    /// 目标软件已被拉起并载入
    TargetOpened,
    /// 目标软件重启后仍可读回
    RestartReadbackVerified,
    /// 下一轮请求里确实带上了迁移的历史（请求级证据）
    NextTurnRequestVerified,
    /// 真实模型回复且引用了迁移事实
    NextTurnReplyVerified,
    /// 写入结果不明，必须先对账
    NeedsReconciliation,
    /// 多候选或无法定位，禁止自动动作
    Ambiguous,
    /// 有正面证据表明没有产生任何副作用
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserAttestation {
    pub attested_at: i64,
    /// 用户自报到达了哪一级。仅作展示与人工追溯。
    pub claimed_stage: RestoreStage,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}
```

**`stage` 只由系统证据推进。** `user_attestation` 是并列的一栏，永远不改 `stage`，UI 必须把两者分开呈现（"系统已验证到 X ／ 用户自报 Y"）。

### 3.3 能力矩阵与单次恢复结果分离

```rust
/// 工程发布口径：编译期常量表，来自研发验收，用户不必自己跑
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseCapability {
    pub provider_id: String,
    /// 提取规则是否已被 fixture 实证 —— 这是导出的**必要条件**
    pub extraction_rule: Option<ExtractionRuleInfo>,
    /// 写入路径标识；None = 本发布未验证任何写入路径
    pub write_strategy: Option<String>,
    /// 每一级各自已验证到哪些版本。空数组 = 该级从未验证
    pub verified_stages: Vec<(RestoreStage, Vec<String>)>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractionRuleInfo {
    pub rule_id: String,
    pub verified_versions: Vec<String>,
}

/// 本机运行期探测结果，与上面的发布矩阵**分开**
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalProviderProbe {
    pub provider_id: String,
    pub installed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detected_version: Option<String>,
    /// 本机版本是否落在 extraction_rule.verified_versions 内
    pub extraction_supported: bool,
    /// 本机版本是否落在 write_strategy 已验证区间内
    pub write_supported: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason_code: Option<String>,
}
```

**`extraction_supported` 为真只代表能安全导出，不代表能恢复。** 两个门槛独立（§7.6）。

### 3.4 TypeScript 镜像

追加到 `src/types.ts`（现有 `SessionMeta` 在 `src/types.ts:457-473`）。字段与 §3.1–3.3 一一对应，无额外字段、无遗漏。

```typescript
export type MessageKind = "userText" | "assistantFinal";
export type PathFamily = "posix" | "windows" | "unknown";
export type RestoreRequestKind = "defaultImport" | "saveAsNewCopy";
export type RestoreStage =
  | "packageVerified"
  | "nativeWritePending"
  | "nativeWritten"
  | "nativeReadbackVerified"
  | "targetOpened"
  | "restartReadbackVerified"
  | "nextTurnRequestVerified"
  | "nextTurnReplyVerified"
  | "needsReconciliation"
  | "ambiguous"
  | "failed";

export interface OriginIdentity {
  originId: string;
  providerId: string;
  sessionId?: string;
  cliVersion?: string;
  storeFingerprint?: string;
}

export interface MigratableMessage {
  seq: number;
  kind: MessageKind;
  text: string;
  ts?: number;
}

export interface OmittedCounts {
  toolEvents: number;
  reasoningBlocks: number;
  commentaryMessages: number;
  attachments: number;
  runtimeInjections: number;
  unknownBlocks: number;
}

export interface ExtractionReport {
  ruleId: string;
  ruleVerifiedVersions: string[];
  openUserMessages: number[];
  omitted: OmittedCounts;
  sourcePathFamily: PathFamily;
}

export interface MigratableSession {
  snapshotId: string;
  contentDigest: string;
  origin: OriginIdentity;
  title?: string;
  createdAt?: number;
  lastActiveAt?: number;
  workspaceLabel?: string;
  messages: MigratableMessage[];
  extraction: ExtractionReport;
}

export interface SessionPackage {
  schema: string;
  exportedAt: number;
  exporter: { app: string; appVersion: string; platform: string };
  sessions: MigratableSession[];
}

export interface UserAttestation {
  attestedAt: number;
  claimedStage: RestoreStage;
  note?: string;
}

export interface RestoreRequest {
  packagePath: string;
  requestId: string;
  snapshotIds: string[];
  targetProviderId: string;
  targetWorkspace: string;
  requestKind: RestoreRequestKind;
}

export interface RestoreAttempt {
  attemptId: string;
  requestId: string;
  snapshotId: string;
  requestKind: RestoreRequestKind;
  idempotencySlot?: string;
  contentDigest: string;
  origin: OriginIdentity;
  targetProviderId: string;
  targetStoreId: string;
  targetNativeNonce: string;
  targetNativeId?: string;
  targetWorkspace?: string;
  stage: RestoreStage;
  attemptCount: number;
  userAttestation: UserAttestation | null;
  lastError?: MigrationErrorPayload;
  createdAt: number;
  updatedAt: number;
}

export interface MigrationErrorPayload {
  code: string;          // §10 的 camelCase 码，单套
  detail?: Record<string, unknown>;
}

export interface ExtractionRuleInfo {
  ruleId: string;
  verifiedVersions: string[];
}

export interface ReleaseCapability {
  providerId: string;
  extractionRule: ExtractionRuleInfo | null;
  writeStrategy: string | null;
  verifiedStages: Array<[RestoreStage, string[]]>;
}

export interface LocalProviderProbe {
  providerId: string;
  installed: boolean;
  detectedVersion?: string;
  extractionSupported: boolean;
  writeSupported: boolean;
  reasonCode?: string;
}
```

---

## 4. 三个分离的身份

v1 把三件事揉成一个 `migration_key`，导致两个自相矛盾：包含来源 ID 却声称目标 ID 变更后仍相等；对账时拿目标新 ID 重算来源键必然永不匹配。v2 彻底拆开。

### 4.1 来源身份：与内容摘要分离

`originId` 是包内稳定来源标识，原样传递到回执；目标新 ID 不得替换它。来源 CLI 版本、显示标题和导出时间不参与身份。

- 对能可靠识别的原生会话：由 provider、来源 store 的本地稳定命名空间、原生 session ID 通过长度前缀哈希生成 originId。命名空间由 FyAgent 在来源机本地保存，不使用绝对路径或机器秘密；没有可靠命名空间时使用下一项。
- 无可靠原生 ID / store 身份：为这次来源实例分配随机 UUID，并持久保存在包中。复制或转交同一包保留 UUID。无法可靠关联的另一次导出可以产生新的来源身份，宁可提示潜在重复，也不把不同来源同正文自动合并。
- 回执存在且原生目标 ID 与当前内容核对通过时，重导出可以沿用已记录来源；映射丢失不能凭正文猜血统。
- `sessionId` 可缺省，不能用空串、文件名、首条消息或时间戳伪造。`storeFingerprint` 仅诊断；任何原生文件名回退须标清其来源。

### 4.2 内容摘要 `content_digest`——纯内容

```
input =
    b"fyagent.session.content.v1\x00"
  + for each message in messages (按 seq 升序):
        kind_byte            // b'U' = userText, b'A' = assistantFinal
      + u64_le(text.len())   // **UTF-8 字节长度**，不是字符数
      + text.as_bytes()

content_digest = "fyc1:" + hex(sha256(input))
```

排除：provider id、来源会话 ID、路径、时间戳、标题、CLI 版本、平台、导出时间、`omitted` 计数。

**为什么空 assistant 与缺 assistant 不会碰撞**：包里每条消息都有自己的 `kind_byte + 长度 + 正文` 三元组。文本为空的 `AssistantFinal` 产生 `b'A' + 0u64`（9 字节）；根本没有该条 assistant 则一个字节都不产生。两者字节流不同。同理，`[U("ab")]` 与 `[U("a"), U("b")]` 也因长度前缀而不同。

**"有无 final / 未完成的区别"如何体现**：`open_user_messages` 是从 `messages` 序列派生的（某个 `UserText` 之后到下一个 `UserText` 之前没有 `AssistantFinal`），所以它已经被消息序列本身编码，不必也不应单独进哈希。

### 4.3 目标映射——只在回执里

- `target_store_id`：目标机上该 provider 存储的身份。Codex 用 `CODEX_HOME/installation_id`（该文件存在，见 `evidence/codex-probe/isolated-home/installation_id`）；OpenCode 用 `get_opencode_db_path()` 的 canonical 路径（`opencode.rs:90-93`）；其余 provider 在其规则落地时逐一定义。目标 store 的可靠身份尚需验证；路径本身不能证明存储实例未被替换。无法确认时返回 `targetStoreUnidentified`，默认导入和另存副本均禁止原生写入。
- `target_native_id`：目标返回的新 ID。
- **对账方向是单向的**：已知回执 → 目标 ID → 去目标查。**永不**从目标新 ID 反算来源身份或内容键。
- 映射缺失时不得臆造血统。v1 那个用首条 user 正文拼出来的 `lineage_key` 已删除——它会把"同一开场白的两次不同对话"错误地认成同源。同源关系只能由可信来源标识及已核验映射提供，不能靠正文匹配建立。

### 4.4 语义快照、默认幂等与请求重试

所有拼接字段使用 UTF-8 字节长度前缀（u64 little-endian），各哈希有独立固定域：

```
snapshotId = H("fyagent.snapshot.v1", origin.originId, contentDigest)
idempotencySlot = H("fyagent.slot.v1", snapshotId,
                    targetProviderId, targetStoreId, currentDeviceBinding)
```

后端重新计算并校验 snapshotId，不能信任包自报值。包文件原始 SHA 只作证据，不参与幂等：相同来源、相同正文重新导出，即使 exportedAt/JSON 排版改变，也应命中同一 slot；不同来源相同正文必须保留两个快照。

默认导入占用唯一 slot；来源新增消息后成为新快照；同一来源在当前目标已有不同快照时，默认导入返回 `sourceSnapshotConflict` 并提示显式另存，不自动向旧原生会话追加或覆盖。只有默认重复导入且目标映射、原生正文仍有效，才返回已存在结果。目标被删除、替换或改变后不能沿用旧成功状态，进入核验或待对账。

另存副本不占默认 slot，但必须携带稳定 requestId。每次用户明确要求新副本生成新 requestId；同一请求的超时重发、双击去重和重试必须复用 requestId。表内唯一 `(device_binding, request_id, snapshot_id)` 阻止一个请求产生两份。重复键还必须比对请求种类、目标 provider/store/workspace；相同键不同参数以 `identityFieldInvalid` 拒绝，不能默默更换目标。原生写入仅由成功取得该行写权限的执行者发起；用事务内条件更新竞争 pending 状态，不依赖先查再插。

---

## 5. 命令边界

新文件 `src-tauri/src/commands/session_migration.rs`，在 `lib.rs` 的 `invoke_handler` 追加注册（现有会话命令在 `lib.rs:2116-2120`）。

| 命令 | 入参 | 出参 | 副作用 |
|---|---|---|---|
| `preview_session_migration` | `providerId, sourcePath` | `MigratableSession` \| `MigrationError` | 只读。跑提取规则 |
| `export_session_package` | `items[], targetPath` | `{ path, sessionCount, byteLen, packageFileDigest }` | 写包文件 |
| `read_session_package` | `path` | `SessionPackage` + 每个 `contentDigest` 的既有 attempt 列表 | 只读 |
| `probe_local_provider` | `providerId` | `LocalProviderProbe` | 跑 `--version`/`--help` 子进程 |
| `get_release_capability_matrix` | — | `Vec<ReleaseCapability>` | 只读常量表 |
| `restore_session_package` | `RestoreRequest` | `Vec<RestoreAttempt>` | 写目标原生会话 + 写回执 |
| `verify_native_readback` | `attemptId` | `RestoreAttempt` | **系统驱动**：用目标自己的读取通道复核，据此推进 stage |
| `list_restore_attempts` | `filter?` | `Vec<RestoreAttempt>` | 只读 |
| `reconcile_restore_attempts` | — | `Vec<RestoreAttempt>` | 对账（§8.3） |
| `record_user_attestation` | `attemptId, claimedStage, note?` | `RestoreAttempt` | **只写 `user_attestation` 栏**，绝不改 `stage` |

**`advance_recovery_stage` 已删除。** v1 那个接口让 renderer 把任意 attempt 推到任意阶段，等于把客户端自报冒充系统证据。

### 5.1 安全边界

现有 `launch_session_terminal` 接受 renderer 传来的任意 shell 字符串，这在 `commands/session_manager.rs:28-61` 被论证为"已知并接受的风险"，前提是 renderer 可信。

恢复链路**不能沿用**这个前提，因为它新增了一条以前不存在的输入通道：**来自另一台设备的文件**。因此：

1. `restore_session_package` 不接受任何命令字符串，只接受 `contentDigest` + `providerId` + 用户经 `tauri-plugin-dialog`（`lib.rs:898` 已注册）选出的绝对目录。
2. 需要拉起目标软件时，由后端从**目标返回的原生 ID** 生成命令，复用 `terminal::session_resume_argument` 白名单（`terminal/mod.rs:368-376`：首字符为字母数字或 `_`，其余仅 `[A-Za-z0-9_.-]`）。校验对象是目标产生的 ID，不是包里的字符串。
3. 包里的字符串按 §7.4 处理：身份字段非法则拒绝，不做"净化后继续用"。

---

## 6. final-only 提取

### 6.1 为什么必须新写

`providers/utils.rs` 是展示向契约（`utils.rs:88-106`），改它会破坏现有 v1 会话页。新增独立模块，`utils.rs` 一行不改：

```
src-tauri/src/session_manager/migrate/extract/
  mod.rs          // 规则注册表、RawMessage、提取主循环
  strict_text.rs  // extract_text_strict，与 utils::extract_text 平行
  rules/codex.rs rules/claude.rs rules/opencode.rs rules/openclaw.rs
  rules/gemini.rs rules/grokbuild.rs rules/hermes.rs
```

### 6.2 strict 文本抽取

```rust
pub struct StrictText { pub text: String, pub omitted: OmittedCounts }

/// 与 utils::extract_text 的差异（按块粒度丢弃，不是整条消息）：
///   tool_use / tool_call / function_call            -> 丢，tool_events += 1
///   tool_result / function_call_output              -> 丢，tool_events += 1
///   thinking / reasoning / redacted_thinking        -> 丢，reasoning_blocks += 1
///   image / file / document / attachment            -> 丢，attachments += 1
///   text / output_text / input_text                 -> 原文保留
///   其余未知块                                        -> 丢，unknown_blocks += 1
/// 保留片段按源顺序以 "\n" 连接。除此之外不做任何改写。
pub fn extract_text_strict(content: &Value) -> StrictText;
```

Claude 的一条 assistant 消息可同时含 `text` 与 `tool_use` 块（`claude.rs:76` 现在把两者合并）；strict 保留 `text` 块、丢 `tool_use` 块，而不是整条丢弃。

### 6.3 提取主循环：没有默认放行

```
messages = []
for raw in provider_rule.raw_stream():        // 有序
    match provider_rule.classify(raw):
      RealUser            => messages.push(UserText{ text, ts })
      FinalAnswer         => messages.push(AssistantFinal{ text, ts })
      Commentary          => omitted.commentary_messages += 1
      RuntimeInjection    => omitted.runtime_injections   += 1
      ToolEvent           => omitted.tool_events          += 1
      Reasoning           => omitted.reasoning_blocks     += 1
      Attachment          => omitted.attachments          += 1
      Indeterminate(why)  => return Err(finalAnswerIndeterminate { seq, why })   // ← 立即失败
```

四条不变式：

1. **`classify` 必须由 provider 专属、版本限定、fixture 实证的规则给出**。没有规则 = 整会话导出失败（`extractionRuleUnavailable`），不是降级放行。
2. **`Indeterminate` 在任意位置都阻断导出**，末轮不豁免。v1 的"末轮 unknown 可以放行"已删除。
3. **不强塞一问一答**。连续多条 `UserText`（追问、中断后重发）原样保留顺序；末尾的 `UserText` 没有答复也原样保留。这些都是真实场景，不是坏数据。
4. **`open_user_messages` 是派生的**，导出时一并计算写入报告，供目标侧与 UI 使用，不影响成败判定。

"没有 assistant 跟随"与"有 assistant 但判不出是不是 final"是两件完全不同的事：前者是合法的未完成状态，后者是失败。v1 把它们混成了同一个规则。

### 6.4 Codex 规则 `codex.rollout.phase-v1`（唯一已实证的规则）

**证据**：本轮隔离实测的 rollout
`evidence/codex-probe/request-capture/isolated-home/sessions/2026/09/22/rollout-2026-09-22T01-16-15-01a0c4f7-e678-7063-8763-390ed30a27e1.jsonl`
每行形如 `{type:"response_item", payload:{type:"message", role, phase?, content, internal_chat_message_metadata_passthrough:{turn_id, content_item_kinds}}}`。

实测到的行：

| role | phase | content_item_kinds | 判定 |
|---|---|---|---|
| `developer` | — | `["host_skills.instructions","permissions.instructions","collaboration_mode.instructions"]` | RuntimeInjection |
| `developer` | — | `["multi_agent.role_instructions"]` | RuntimeInjection |
| `developer` | — | `["multi_agent.mode_instructions"]` | RuntimeInjection |
| **`user`** | — | `["agents_md.instructions","environments.environment_context"]` | **RuntimeInjection**（role 是 user，但不是用户写的） |
| `user` | — | `["unknown"]` | 我们注入的迁移消息 |
| `assistant` | **`final_answer`** | `["unknown"]` | 我们注入的迁移消息 |
| `user` | — | `["user.text"]` | 真实用户输入 |

**Schema 定义**（`evidence/codex-schema/codex_app_server_protocol.v2.schemas.json` 的 `MessagePhase`）：

> Classifies an assistant message as interim commentary or final answer text. **Providers do not emit this consistently, so callers must treat `None` as "phase unknown"** and keep compatibility behavior for legacy models.
> - `commentary`：Mid-turn assistant text (for example preamble/progress narration).
> - `final_answer`：The assistant's terminal answer text for the current turn.

这直接推翻 v1 的两个说法：磁盘**有**完成字段；而且官方明文说 `None` 是"phase 未知"，**不是**"取最后一条"。

规则定义：

```
classify(payload) =
  if payload.type != "message"                               -> skip（非消息记录）
  if role == "developer"                                     -> RuntimeInjection
  kinds = internal_chat_message_metadata_passthrough.content_item_kinds

  if role == "user":
      if kinds is missing                                    -> Indeterminate("user kinds missing")
      if kinds ⊆ {"user.text"}                               -> RealUser
      if kinds ∩ RUNTIME_KINDS ≠ ∅                           -> RuntimeInjection
      else                                                   -> Indeterminate("unrecognized user kinds")

  if role == "assistant":
      if phase == "commentary"                               -> Commentary
      if phase == "final_answer" and 所有 content 块为 output_text
                                                             -> FinalAnswer
      if phase == "final_answer" but 含非 output_text 块      -> Indeterminate("non-text final block")
      if phase is null                                       -> Indeterminate("phase unknown")

RUNTIME_KINDS = { "agents_md.instructions", "environments.environment_context",
                  "host_skills.instructions", "permissions.instructions",
                  "collaboration_mode.instructions", "multi_agent.role_instructions",
                  "multi_agent.mode_instructions" }
```

三点必须写明：

- **`phase` 缺失不走 positional 兜底。** 官方明说 provider 不一致地发这个字段，旧模型可能没有。缺 `phase` → `Indeterminate` → 阻断。要放行必须先有覆盖该版本的 fixture 实证，再把版本写进 `rule_verified_versions`。本轮实证仅覆盖 codex-cli `0.154.0`。
- **`content_item_kinds == ["unknown"]` 不算干净正文。** 实测显示我们自己 `inject_items` 注入的消息就被标成 `unknown`。因此**由本功能恢复出来的 Codex thread 不能用本规则再次导出**——这是已知的往返退化，必须在 UI 明示，不能靠"反正是我们写的"来豁免。
- `RUNTIME_KINDS` 是**实测观察到的集合**，不是官方穷举清单。遇到未收录的 kind 一律 `Indeterminate`，宁可阻断也不放行。`codex.rs:427-499` 已有的 `# AGENTS.md` / `<environment_context>` 文本判据可作为同向交叉校验，但**不作为主判据**——它是字符串启发式，`content_item_kinds` 才是结构化事实。

### 6.5 七 provider 的规则状态

判定口径：**Verified** = 规则已由实测产物 fixture 证实且限定到具体版本；**Candidate** = 格式里有候选字段但尚无真实产物 fixture；**None** = 尚无规则，导出被阻断。

| provider | 读取（已实现） | 候选完成信号 | 规则状态 | 导出 |
|---|---|---|---|---|
| **Codex 0.154.0** | `codex.rs:205-269`（丢弃 `phase` 与 `content_item_kinds`） | `payload.phase` ∈ {commentary, final_answer}；`content_item_kinds` 区分运行时注入 | **Verified** `codex.rollout.phase-v1`，限 `=0.154.0` | ✅ 放行 |
| **OpenCode 1.18.30** | `opencode.rs:157-237`（只读 `role`）/ SQLite `opencode.rs:255-273` | message info 的 `finish` 与 `time.completed` | **Candidate** —— 只在**我们自己写的**合成 fixture 里出现过（`research/…/check_opencode_import.py:40-43`），未在真实 opencode 产物上核验字段必现 | ⛔ 阻断，待真实产物 fixture |
| **Claude 2.1.220** | `claude.rs:33-86` | `message.stop_reason`（reader 未读，仅格式已知）；`isSidechain`/`parentUuid` 可剔子代理 | **None** | ⛔ |
| **OpenClaw 2026.7.1-2** | `openclaw.rs:92-119`，已有 `[message_id: ...]` 剥离（`openclaw.rs:22-28`） | 无已知字段 | **None** | ⛔ |
| **Gemini 0.59.0** | `gemini.rs:70-98`，`info`/`error` 已跳过 | 无已知字段 | **None** | ⛔ |
| **Grok Build 1.0.34** | `grokbuild.rs:64-88`，已跳过 `reasoning` | 无；且官方称 `updates.jsonl` 才是恢复权威日志，我们读的 `chat_history.jsonl` 是另一份视图 | **None** | ⛔ |
| **Hermes v0.20.5** | `hermes.rs:199-200` `SELECT role, content, created_at` | 官方文档称 messages 表另有 `tool_calls`/`tool_name`（我们的 SELECT 没取）；可用 `PRAGMA table_info` 探测，`hermes.rs:151-152` 已有先例 | **None** | ⛔ |

**这些"None"全都是"reader 未读 / 尚无 fixture"，不是"格式里没有"。** 对每一家都应按 Codex 的做法做一次隔离产物实测再定规则。v1 把六家预先钉成 `Positional` 并放行导出，是本次返工的头号修正项。

---

## 7. 包、大小、路径、版本

### 7.1 大小与结构上限

| 层级 | 上限 | 超限 |
|---|---|---|
| 读源累计字节 | 256 MiB | `sourceTooLarge` |
| 单条消息正文 | 4 MiB | `messageTooLarge` |
| 单会话消息条数 | 4000 | `tooManyMessages` |
| 单会话正文总量 | 32 MiB | `sessionTooLarge` |
| 单包会话数 | 200 | `tooManySessions` |
| 包文件字节 | 128 MiB | `packageTooLarge`（先看 `metadata().len()` 再解析） |
| JSON 嵌套深度 | 64 | `jsonTooDeep` |

**一律失败，不截断**——截断即篡改原文。

### 7.2 磁盘写入

用 `config::atomic_write`（`src-tauri/src/config.rs:390`，临时文件 + rename），避免半个包被当成完整包。不压缩、不打 zip：内容只有文本，单 JSON 文件便于用户自行打开核对"里面确实没有工具日志"，这个可审计性比体积重要。

### 7.3 包版本门控

- `schema != "fyagent.session.v1"` → `packageSchemaUnsupported`，拒绝解析。
- 出现未知字段 → `packageUnknownField`（`deny_unknown_fields` 触发后映射到独立错误码，而不是笼统的 malformed）。未来的 v2 包在 v1 应用上会在 `schema` 这一步被明确拒绝，不会被宽容解析成残包。
- `exporter.appVersion` / `platform` 只用于诊断展示，不参与门控。

### 7.4 字符串处理：身份字段不做静默净化

分成三类，规则不同：

| 类别 | 字段 | 规则 |
|---|---|---|
| **正文** | `messages[].text` | **逐字保真**。不 trim、不折叠空行、不重写路径、不转义。渲染侧负责安全 |
| **身份** | `origin.sessionId`、`origin.storeFingerprint`、`contentDigest` | **不净化**。含 C0 控制字符、双向覆写（`U+202A`–`U+202E`）、超长（>200 字符）→ 整个会话 `identityFieldInvalid` 拒绝导入。**绝不**净化后继续参与身份计算或 slot 计算 |
| **展示元数据** | `title`、`workspaceLabel`、`origin.cliVersion` | 原值保留在包里；**仅在渲染时**做转义与长度截断。不改写包内值，不参与任何哈希。`workspaceLabel` 额外要求不含 `/` `\` `:`，违反则该字段按缺失处理并提示 |

v1 的"落地前统一净化"会让两份本应不同的身份在净化后相等，制造假重复。

### 7.5 目录元数据映射

**A. 包内不存绝对路径。** 源目录各家来源不同：Codex `session_meta.payload.cwd`（`codex.rs:328-333`）、Claude 顶层 `cwd`（`claude.rs:148-152`）、OpenCode `directory`（`opencode.rs:507`）、Hermes `cwd`/`directory`（`hermes.rs:118-123`）、Grok `info.cwd`（`grokbuild.rs:189`）、Gemini 读 `.project_root`（`gemini.rs:38-39`）。全部不进包，只留 basename 作 `workspaceLabel`，形态记进 `sourcePathFamily`。

**B. 目标目录由用户在目标机指定。** `target_workspace` 必须是用户选出的、在目标机已存在的绝对目录；后端 canonicalize 后校验存在且是目录，否则 `targetDirectoryNotFound`。**不自动创建目录**（创建目录属于迁移工作文件，在排除范围内）。

**C. 正文里的旧路径保留原样。** 已实测现象：OpenCode 导入后会话目录被改写为目标目录，但 assistant 正文仍含旧 Windows 路径（`opencode-synthetic-result.json` 的 `directory_remapped_to_import_cwd: true` 与 `assistant_path_still_contains_source_cwd: true`）。产品决定不改写正文（用户明确要求原文），改为按 `sourcePathFamily` 在 UI 提示"正文含另一操作系统的路径"。

Gemini 额外：会话按 `tmp/<project_hash>/chats/` 组织且 hash 绑定项目根路径，目标机必须重算或重新映射——这是 Gemini 的专属未知项。

### 7.6 能力门槛：提取与恢复是两道独立的门

```
能导出（extraction gate）  ⟸  本机版本 ∈ extraction_rule.verified_versions
能恢复（write gate）       ⟸  目标版本 ∈ write_strategy 已验证区间
能声称"已恢复"             ⟸  stage 达到 nativeReadbackVerified 及以上
```

**通过第一道门不蕴含第二道。** Codex 当前提取规则已实证（可导出），但写入路径的原生可见性仍未通过（§9.2），所以 Codex **不能**标为"支持恢复"。

版本探测走 `probe_local_provider`：跑 `<cli> --version` 与 `--help`，5 秒超时，结果与编译期 `ReleaseCapability` 表比对。**必须探测安装版而非查文档表**：`research/…/local-cli-evidence.json` 记录 Grok 官网列了 `import` 而本机 1.0.34 help 没有；照文档承诺会直接产生假支持。

---

## 8. 崩溃窗口、对账与重试

### 8.1 窗口在哪里

**原生写入不在 FyAgent 的 SQLite 事务内**——它是子进程（`opencode import`、`hermes sessions import`）或外部协议调用（Codex app-server）。SQLite 能买到的是本地行的原子更新与唯一约束闸，**买不到跨进程 exactly-once**。文案里不得出现 exactly-once。

### 8.2 写前保存 + 预分配

```
1. 校验 request_id 和 snapshot_id；计算默认 slot；INSERT attempt(request_id, snapshot_id, attempt_id, slot, stage=packageVerified, nonce=<预分配>)
   —— 请求唯一键冲突返回原尝试；默认 slot 冲突关联原尝试并核验目标。禁止重复调用写入
2. UPDATE stage = nativeWritePending                    // 单独提交
3. 若目标支持先拿 ID（Codex）：
      3a. thread/start  -> 得到 threadId
      3b. UPDATE target_native_id = threadId            // **先落回执，再产生内容副作用**
      3c. thread/inject_items
   若目标不支持先拿 ID（OpenCode / Hermes 子进程导入）：
      3'. 把预分配的 nonce 写进可被目标保留的字段（如 session title 后缀 / slug），
          作为对账锚点；执行导入；从 stdout 解析返回的 ID
4. UPDATE stage = nativeWritten, target_native_id = <返回值>
5. verify_native_readback -> stage = nativeReadbackVerified
```

崩溃点与处置：

| 崩溃位置 | 回执状态 | 处置 |
|---|---|---|
| 1 之前 | 无行 | 无副作用，用户重来即可 |
| 2 与 3a 之间 | `nativeWritePending`，无 ID | **`ambiguous`**：`thread/start` 可能已经建了线程。不盲重试，先对账 |
| 3a 与 3b 之间 | `nativeWritePending`，无 ID | 同上 `ambiguous` |
| 3b 与 3c 之间 | `nativeWritePending`，**有 ID** | 可定位：按 ID 查目标，确认是否已注入；只有原生权威证据证明未注入才可续做；列表为空不等于未注入 |
| 3' 执行中 | `nativeWritePending`，无 ID，有 nonce | 按 nonce 对账 |
| 4 与 5 之间 | `nativeWritten` | 直接跑 `verify_native_readback` |

**`thread/start` 返回 ID 之前崩溃就是 `ambiguous`，禁止盲重试**——这是 revision brief §4 的硬要求。

### 8.3 对账

`reconcile_restore_attempts` 对每条 `nativeWritePending` / `ambiguous`：

1. **有 `target_native_id`** → 按 ID 直接查目标。这是唯一的强相关证据路径。
2. **无 ID，有 nonce 且目标保留了 nonce** → 按 nonce 定位。
3. **无 ID 无可用 nonce** → 只能用包内 final-only 摘要在目标侧做内容比对，且必须先排除目标运行时注入（Codex 会自动加入 `agents_md.instructions` / `environments.environment_context` 等，见 §6.4 实测），否则必然对不上。

判定：

- **恰好 1 个候选，且强相关证据（ID 或 nonce）+ 内容比对同时成立** → 晋升。
- **仅内容匹配**或**仅时间窗匹配** → **不是证明**，保持 `needsReconciliation`。
- **0 个候选** → 保持 `needsReconciliation`（不是 `failed`：目标可能只是不可见，见 §9.2 Codex 的列表 API 为空）。
- **多个候选** → `ambiguous`，全部候选一并记录，禁止自动动作。

已知漏检（结果是 unknown，不是"新建副本"）：Hermes SQLite scan 有 `LIMIT 500`（`hermes.rs:84`）；OpenClaw 的 sessionId 会因 reset 变化。

### 8.4 失败判定与重试

**非 0 退出码不等于"安全失败"。** 子进程可能在失败前已部分写入。只有在能正面证明没有副作用时才置 `failed`：

- 可执行文件不存在 / 无法启动；
- 参数解析阶段即退出，且退出码与已知的"未做任何 IO"错误码匹配；
- 协议层在任何写方法发出之前就断开。

除此之外（包括超时、被杀、stderr 无法解析、退出码未知）一律 `needsReconciliation`。

重试规则：

| 情形 | 动作 |
|---|---|
| `failed`（已证明无副作用） | 允许幂等重试，复用同一 slot 与 attempt 行，`attempt` 计数 +1 |
| `needsReconciliation` / `ambiguous` | **禁止再次调用原生写入**，先对账 |
| `nativeReadbackVerified` 之后用户想再要一份 | 必须显式发起 `SaveAsNewCopy`：新 `request_id` 与 `attempt_id`、不占默认 slot、新映射；同请求重发返回同一行，**旧映射全部保留** |

**"重试 = 默认新建另一份"已删除**（v1 §8.3 的错误）。另存副本是独立的显式用户操作，不是失败恢复的默认路径。所有副本映射永久保留，任何路径都不覆盖既有 `target_native_id`。

### 8.5 回执存储

复用现有 FyAgent SQLite 的一张小表，**不存第二份正文**：

```sql
CREATE TABLE IF NOT EXISTS session_restore_attempts (
  attempt_id           TEXT PRIMARY KEY,
  request_id           TEXT NOT NULL,
  snapshot_id          TEXT NOT NULL,
  origin_id            TEXT NOT NULL,
  request_kind         TEXT NOT NULL,
  idempotency_slot     TEXT,
  content_digest       TEXT NOT NULL,
  origin_provider_id   TEXT NOT NULL,
  origin_session_id    TEXT,
  origin_cli_version   TEXT,
  origin_store_fp      TEXT,
  target_provider_id   TEXT NOT NULL,
  target_store_id      TEXT NOT NULL,
  target_native_nonce  TEXT NOT NULL,
  target_native_id     TEXT,
  target_workspace     TEXT,
  stage                TEXT NOT NULL,
  attempt_count        INTEGER NOT NULL DEFAULT 1,
  user_attested_at     INTEGER,
  user_claimed_stage   TEXT,
  user_note            TEXT,
  last_error_code      TEXT,
  last_error_detail    TEXT,
  device_binding       TEXT NOT NULL,      -- 见下
  created_at           INTEGER NOT NULL,
  updated_at           INTEGER NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS uq_sra_request
  ON session_restore_attempts(device_binding, request_id, snapshot_id);
CREATE UNIQUE INDEX IF NOT EXISTS uq_sra_slot
  ON session_restore_attempts(idempotency_slot) WHERE idempotency_slot IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_sra_digest ON session_restore_attempts(content_digest);
CREATE INDEX IF NOT EXISTS idx_sra_native ON session_restore_attempts(target_provider_id, target_native_id);
```

**迁移版本号实施时重读**（`database/mod.rs:56` 的 `SCHEMA_VERSION` 当前是 20，但不在本文档里钉死"20→21"——期间可能有其他在途迁移占号）。

**设备/store 绑定失效策略**（不能只改 WebDAV）：

- `device_binding` = 本机安装标识 + 目标 store 身份的哈希，写入时固化并参与 slot。安装标识不得随配置包/SQL/WebDAV 复制到另一机器。
- 读取回执时**逐行校验** `device_binding` 与当前机器是否一致；不一致的行一律视为**失效**，不参与幂等判定、不参与对账、UI 标为"来自其他设备的回执"。
- WebDAV 同步侧仍需把该表加入 `database/backup.rs:76-97` 的 `SYNC_SKIP_TABLES` 与 `SYNC_PRESERVE_TABLES`。
- **普通 SQL 备份/恢复同样会复制回执**（`backup.rs` 的全量 `export_sql_string` 不跳过任何表），实施时普通备份导出也排除该表，导入保留当前本机回执；对旧备份意外带入的行仍做绑定校验。新设备 slot 含本机绑定，不能被旧行唯一索引误占。

---

## 9. 七 provider 的证据等级

六级分证：①包验证 ②目标写入 ③原生历史可见（用目标自己的读取通道） ④打开 ⑤重启后读回 ⑥下一轮（请求级 / 真实回复分开记）。

### 9.1 Codex 0.154.0 —— 提取规则已实证；写入有协议与请求级证据，原生可见性未通过

**有证据**（`evidence/codex-probe/findings.md`、`request-capture/result.json`，隔离 `CODEX_HOME`、合成四条问答、`model_called: false`、`real_user_store_touched: false`）：

- `thread/start` 返回 `thread_id = 01a0c4f7-e678-7063-8763-390ed30a27e1`；`thread/inject_items` 返回 `{}`（成功）。
- 四条原文落盘 rollout，角色、顺序、正文保真，assistant 两条带 `phase: "final_answer"`。
- 重启 app-server 后 `thread/resume` 成功。
- 向**本机模拟端点**发起下一轮，捕获到 1 个请求，`all_four_texts_in_next_request: true`；请求 `input` 里按 `user/assistant/user/assistant` 保留四条原文（`request-capture/captured-request.json`）。

**无证据 / 反面证据**：

- `thread/read` 的 `turns: []`，`historyMode: "paginated"`。
- 重启后 `thread/items/list` 与 `thread/turns/list` 的 `data: []`。
- `turn/start` 的 `itemsView: "notLoaded"`。
- legacy 模式亦未解决：`legacy/result.json` 记 `thread/items/list is not supported yet`（JSON-RPC `-32601`）。
- 桌面 UI 展示、真实模型回复、Mac/Windows 双向：全未测。

**结论**：①②⑥（请求级）有证据；③**明确未通过**（列表为空）；④⑤⑥（真实回复）未验证。

**"我们能扫到 JSONL" 不能顶替 ③。** `scan_sessions`（`mod.rs:58-94`）读的是磁盘文件，不是目标自己的展示通道；把它当作原生可见会把"文件在那里"误报成"用户能看见"。`verify_native_readback` 对 Codex 必须查 `thread/items/list` 或 `thread/turns/list`；目前它们为空，所以 Codex 的 ③ 记为 **blocked**，不是 pending。

**工程缺口**：全仓检索 `app.?server|jsonrpc|json_rpc` 在 `src-tauri/src` 零匹配。需从零实现 stdio JSON-RPC 客户端。另注意 `ThreadResumeParams.history` 标 `[UNSTABLE] FOR CODEX CLOUD - DO NOT USE`，禁止使用。

### 9.2 OpenCode 1.18.30 —— 写入已实证，提取规则尚未实证

- **写**：`research/…/opencode-synthetic-result.json`：`opencode import --pure` 退出码 0，export 往返 4 条消息角色/文本/顺序保真，`only_text_parts: true`。隔离 XDG 目录，未读真实会话，未调模型。→ ①② 有证据。
- **提取**：⛔。fixture 里的 `finish`/`time.completed` 是**我们自己写进去的**（`check_opencode_import.py:40-43`），不证明真实 opencode 会话必有这些字段。规则状态 Candidate，导出阻断。
- **缺口**：③④⑤⑥未验证；正文旧路径残留（按 §7.5-C 处理）；重复导入的 ID 冲突由 §4.4 的 slot + 目标生成新 ID 规避。
- **已验证的最小原生字段**（来自通过的 fixture）：session `info` 需 `id/slug/projectID/directory/title/version/time{created,updated}`；message `info` 需 `id/sessionID/role/time.created`，assistant 另需 `parentID/modelID/providerID/mode/path{cwd,root}/cost/tokens/finish/time.completed`；part 需 `id/sessionID/messageID/type:"text"/text`。**assistant 的 `path` 要填目标机路径**——这是唯一需要主动写入路径的地方。

### 9.3 Hermes v0.20.5 —— 同样有合成写入实证

- **写**：`research/…/hermes-synthetic-result.json`：合成 Codex 格式 → 安装版 `hermes sessions import` → 写入会话库 → 按续聊结构读回 4 条 → 离线追加后 5 条。未调模型，隔离存储已清理。→ ①② 有证据。
- **关键约束**：`native_importer_final_only: false`、`native_importer_retains_commentary_and_tool_name_marker: true`。**Hermes 自己的导入器会带入助手进度与工具名标记**，所以必须先用我们的规则完成 final-only 过滤，再合成 Codex 格式喂给它，绝不能把源软件原始日志直接转交。
- **提取**：⛔ 无规则。
- **缺口**：③④⑤⑥未验证。`hermes` 顶层 `import` 是全量备份恢复，**不在 final-only 范围**（`local-cli-evidence.json` 已注明），不要误用。
- `resume_command` 恒为 `None`（`hermes.rs:146`、`427`），④"打开"需另找入口。

### 9.4 Gemini 0.59.0

写入唯一证据是本机 help 里 `--session-file` 这个 flag 存在（`local-cli-evidence.json`）；JSON schema 未知、是否接受跨 project 未知。官方另明确自动保存的历史包含工具输入/输出，所以原生文件天然不是 final-only。①–⑥全未验证；提取规则 None。下一步是 schema 发现实测。

### 9.5 Claude 2.1.220

**无公开导入契约**：`/export` 是纯文本输出，`/import [codex|gemini|cursor]` 导入的是配置而非会话（`research/…/codex-claude.md`）；`--teleport` 要求同一 claude.ai 账号 + 同仓库 + 干净 git 状态。可能路径是构造精简原生 `.jsonl`——我们的**读取**能力已证实（`claude.rs` 全套），但读得懂不等于写得进。①–⑥全未验证；提取规则 None。**不按文档缺失断言"技术上不可能"。**

### 9.6 Grok Build 1.0.34

本机 help **未列 `import`**（`local-cli-evidence.json`），尽管 xAI 网页 reference 列了 → 必须以安装版探测为准。另外官方称 `updates.jsonl` 是恢复权威日志（ACP 事件序列），我们读的 `chat_history.jsonl` 是另一份视图；过滤 `updates.jsonl` 会破坏事件序列。①–⑥全未验证；提取规则 None。

### 9.7 OpenClaw 2026.7.1-2

本机受检命令只有 `sessions list/cleanup/compact/export-trajectory/tail`，无通用 import。会话归属 Gateway + agentId + controller ownership；官方 Session Share 明确只读、不允许 continuation（`codex-claude.md`）——**只读共享不算恢复**。`sessionId` 会因 daily/idle reset 变化。①–⑥全未验证；提取规则 None。

### 9.8 汇总

| provider | 提取规则 | ①包验证 | ②目标写入 | ③原生可见 | ④打开 | ⑤重启读回 | ⑥下轮请求 | ⑥下轮真实回复 |
|---|---|---|---|---|---|---|---|---|
| Codex 0.154.0 | **Verified**（`=0.154.0`） | ✅ | ✅ | **⛔ blocked**（列表为空） | — | ✅ resume 成功 | ✅ 模拟端点请求级 | — |
| OpenCode 1.18.30 | Candidate | ✅ | ✅ | — | — | — | — | — |
| Hermes v0.20.5 | None | ✅ | ✅ | — | — | — | — | — |
| Gemini 0.59.0 | None | — | — | — | — | — | — | — |
| Claude 2.1.220 | None | — | — | — | — | — | — | — |
| Grok 1.0.34 | None | — | — | — | — | — | — | — |
| OpenClaw 2026.7.1-2 | None | — | — | — | — | — | — | — |

✅ = 本轮或 9-21 有落盘实测；**⛔ blocked** = 实测为反面结果；— = 未验证（**不等于不支持**）。

**七个软件都在产品范围内。** 实施顺序不是永久缩小范围。当前无一家达到"支持恢复"的门槛（§7.6 要求 `nativeReadbackVerified`）。

---

## 10. 错误语义（单套 camelCase）

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "code", content = "detail")]
pub enum MigrationError {
    // —— 源读取 ——
    sourceUnreadable        { provider_id: String, reason: String },
    sourceTooLarge          { read_bytes: u64, limit: u64 },

    // —— 提取 ——
    extractionRuleUnavailable { provider_id: String, detected_version: Option<String> },
    extractionRuleVersionMismatch { provider_id: String, detected: Option<String>, verified: Vec<String> },
    finalAnswerIndeterminate  { seq: u32, reason: String },
    runtimeInjectionUnclassified { seq: u32, kinds: Vec<String> },

    // —— 包 ——
    packageSchemaUnsupported { found: String, supported: Vec<String> },
    packageUnknownField      { pointer: String, field: String },
    packageMalformed         { reason: String },
    packageTooLarge          { bytes: u64, limit: u64 },
    jsonTooDeep              { depth: u32, limit: u32 },
    messageTooLarge          { seq: u32, bytes: u64, limit: u64 },
    tooManyMessages          { count: u32, limit: u32 },
    sessionTooLarge          { bytes: u64, limit: u64 },
    tooManySessions          { count: u32, limit: u32 },
    identityFieldInvalid     { field: String, reason: String },
    packageWriteFailed       { path: String, reason: String },

    // —— 目标能力 ——
    providerNotInstalled     { provider_id: String },
    providerVersionUnsupported { provider_id: String, detected: Option<String>, verified: Vec<String> },
    capabilityProbeFailed    { provider_id: String, reason: String },
    targetStoreUnidentified  { provider_id: String },

    // —— 目标写入 ——
    targetDirectoryNotFound  { path: String },
    nativeImportFailed       { provider_id: String, exit_code: Option<i32>, stderr_tail: String },
    nativeProtocolFailed     { provider_id: String, method: String, reason: String },
    nativeReadbackMismatch   { attempt_id: String, expected_digest: String, observed: String },

    // —— 回执与对账 ——
    sourceSnapshotConflict  { origin_id: String, existing_attempt_id: String },
    idempotencySlotTaken     { existing_attempt_id: String, existing_stage: RestoreStage },
    reconciliationRequired   { attempt_id: String, reason: String },
    ambiguousNativeMatch     { attempt_id: String, candidates: Vec<String> },
}
```

不变式：

1. **绝不返回"成功但 0 条消息"**。任何导致零消息的路径都映射到上面某个码。
2. `code` 是稳定机器码，UI 文案按 code 映射；`detail` 仅诊断展示，不参与逻辑。
3. `stderr_tail` 截尾 2 KiB，返回前剔除疑似令牌（`sk-`、`ghp_`、`Bearer ` 之后的连续非空白串）。
4. `providerVersionUnsupported` 必须同时带 `detected` 与 `verified`，让用户看到"你装的是什么"和"我们验过什么"。
5. 提取类错误发生在**写包之前**，坏数据不落盘。
6. `nativeImportFailed` **不自动等于 `failed` 阶段**——阶段由 §8.4 的副作用证据决定。

---

## 11. 可分工文件范围与依赖

一文件一 writer。

| 块 | 文件 | 内容 | 依赖 |
|---|---|---|---|
| **A** | `session_manager/migrate/{mod,model}.rs` | DTO、`MigrationError`、`content_digest` / `idempotency_slot` 计算 + 单测 | — |
| **B** | `migrate/extract/{mod,strict_text}.rs` | `RawMessage`、`extract_text_strict`、提取主循环、规则注册表 | A |
| **B-codex** | `migrate/extract/rules/codex.rs` | `codex.rollout.phase-v1` + fixture 单测（用 `evidence/codex-probe/**` 的 rollout 做 golden） | B |
| **B-others** | `migrate/extract/rules/{opencode,claude,openclaw,gemini,grokbuild,hermes}.rs` | 每家先做隔离产物实测 → 再定规则。**六个文件互不依赖，可并行** | B + 各自实测 |
| **C** | `migrate/package.rs` | 读写、schema 门控、大小/深度限制、身份字段拒绝、`atomic_write` | A |
| **D** | `migrate/receipt.rs` + `database/schema.rs`（新表与迁移）+ `database/backup.rs`（两个 skip 列表） | 回执 DAO、slot 唯一约束、`device_binding` 行级失效。**既有文件需排队；版本号实施时重读** | A |
| **E** | `migrate/capability.rs` | `ReleaseCapability` 常量表 + `probe_local_provider` | — |
| **F** | `migrate/writers/opencode.rs` | 原生 JSON 合成 + `opencode import` 子进程 | A, C, E |
| **G1** | `src-tauri/src/codex_app_server/{mod,transport,protocol}.rs` | 全新 stdio JSON-RPC 客户端 | — |
| **G2** | `migrate/writers/codex.rs` | `thread/start` → 落回执 → `inject_items` → `turn/start` | A, E, G1 |
| **G3** | `migrate/verify/codex.rs` | `verify_native_readback`：查 `thread/items/list` / `turns/list`。**当前实测为空，本块的首要产出是把 blocked 变成可解或确认为产品限制** | G1 |
| **H** | `migrate/writers/hermes.rs` | strict 结果 → Codex 格式日志 → `hermes sessions import` | A, B-codex, E |
| **I** | `migrate/reconcile.rs` | 对账（§8.3） | A, B*, D |
| **J** | `commands/session_migration.rs` + `lib.rs`（仅追加注册） | 11 个命令 | A–F, I |
| **K** | `src/types.ts`（追加）+ `src/lib/api/sessionMigration.ts` | TS DTO + invoke 封装 | J 的签名 |
| **L** | `src/v2/pages/sessions/*` + `src/v2/shared/config/navigation.ts` + `src/v2/app/router.tsx` | v2 一级入口与页面 | K、ux-spec |

**关键依赖与风险：**

1. **G1 是最长路径**，全仓无任何 JSON-RPC 客户端。与 A/B/F 同时开工，不要串行。
2. **G3 的结论会决定 Codex 能否声称恢复**。在 ③ 从 blocked 变绿之前，任何"Codex 已支持"的表述都不成立。
3. **B-others 的前置是实测，不是编码**。每家先跑一次隔离产物探针拿到真实字段，再写规则。绕过这一步就会重演 v1 的 Positional 错误。
4. **L 撞当前 dirty 区**：`src/v2/app/router.tsx`、`navigation.ts`、`SideNavigation.tsx` 都在既有界面工作的 modified 列表里，需排在其后或同一 writer 承接。
5. **D 触碰 `schema.rs` / `backup.rs`**，高频公共文件，单独排期，版本号实施时重读。
6. **Windows 侧"打开目标软件"是空白**（`terminal/mod.rs:27-29` 只支持 macOS）。④ 在补齐前无法完成，Mac/Windows 双向因此受阻。应单列前置任务。

**建议顺序**：

```
第 1 波（并行）：A | E | G1
第 2 波（并行）：B → B-codex | C | D
第 3 波（并行）：G2 → G3 | F | I
第 4 波        ：J → K → L
第 5 波（并行）：B-others 六家（各自先做隔离产物实测）| H
```

---

## 12. 未解决项

按 revision brief 的要求，这些**不在本文档内假装收敛**：

1. **Codex 原生历史可见（③）当前实测为空**。`thread/read.turns`、`items/list`、`turns/list` 全为 `[]`，`historyMode: "paginated"`，legacy 模式报 `-32601`。是否存在 paginated 模式下的正确拉取方式、是否需要非默认 `historyMode`、桌面 UI 是否另有通道——全部未知。G3 未出结论前，Codex 不得标为可恢复。
2. **本功能产出的 Codex thread 无法用本规则再次导出**。注入的消息 `content_item_kinds` 为 `["unknown"]`（实测）。是否可在注入时携带合法 kinds、或是否需要一条专门的"FyAgent 迁移消息"识别规则，未解决。
3. **`RUNTIME_KINDS` 不是官方穷举清单**，只是本轮实测观察集合。遇到未收录 kind 会阻断导出。需要官方清单或更多产物样本。
4. **六家的提取规则全部缺失**，包括已能写入的 OpenCode 与 Hermes。在各自实测前不得放行导出。
5. **目标 store 身份**只对 Codex（`installation_id`）与 OpenCode（db 路径）有明确定义，其余五家未定义；未定义者不能进入任何原生写入路径；这是待补齐的验证条件，不是产品永久缩减。
6. **`target_native_nonce` 的承载方式**对子进程导入型 provider（OpenCode / Hermes）尚未验证：把 nonce 放进 title/slug 是否会被目标保留、是否会污染用户可见标题，需实测。
7. **Windows 端打开目标软件**无任何实现，Mac/Windows 双向验收受阻。
8. **`SCHEMA_VERSION` 具体号位**留到实施时读取，本文档不钉死。
9. **QA 合同对齐**：协调方已落实任意歧义阻断、真实缺答复保留、禁止自动追加、来源/内容/目标分离，并新增语义快照与同请求副本幂等检查。用例仍为 not_run；隔离合同实验不等于生产实现测试。

---

## 13. 本设计明确不做的事

- 不改 `providers/utils.rs`、不改任何现有 `providers/*.rs` 的读取语义、不改 `get_session_messages` 的返回契约。
- 不用 `ThreadResumeParams.history`（schema 明确 `DO NOT USE`）。
- 不用 `hermes` 顶层 `import`（全量备份，超出 final-only 范围）。
- 不把 `opencode --sanitize` 当作 final-only 导出。
- 不做 sidecar 回执文件（与 SQLite 表叠用即冗余）。
- 不做第二套会话库、作业引擎、跨设备身份服务、压缩归档、云同步、插件框架。
- 不提供 `Overwrite`；不覆盖任何既有 `target_native_id` 映射。
- 不删除/重命名/转换任何 `MEMORY.md`、`USER.md`、日常记忆文件。
- 不把"可查看 / 可复制"、"协议调用成功"、"我们能扫到磁盘文件"、"用户自报成功"中的任何一项标为已恢复。
- 不在任何路径上用模型补写缺失的最终答复。
- 不迁移认证、工具状态、附件、工作目录文件。
- 不宣称 exactly-once。

---

## 14. 自检

- 交付文件已写盘：`.trellis/tasks/09-22-session-cross-device-recovery/deliverables/technical-design.md`。v1 快照保留于 `evidence/technical-design-v1-reviewed.md`。
- 本轮未修改任何 `src/` 或 `src-tauri/` 文件，未 `git add` / `git commit`，未运行任何 provider 的导入/导出/删除命令，未读取真实用户会话内容，未新派 Agent，未重新全仓探索。
- revision brief 八条逐条落点：①→§3.1/§6.3/§6.5；②→§6.4/§9.1/§9.3/§9.8；③→§4.1/§4.2/§4.3；④→§3.2/§4.4/§8.2/§8.4/§8.5；⑤→§3.2/§5/§3.3；⑥→§2/§8.5；⑦→§7.4/§7.6；⑧→§0/§3.4（Rust↔TS 逐字段对齐）/§12。
- `grok47-final-review.md` 的严重项 1–4 与"具体改法"1–6 已逐条纳入；其中 QA 侧两处行为期望冲突不在本角色文件内单方裁定，已记入 §12.9。
- 错误码单套 camelCase，§10 与 §3.4 的 `MigrationErrorPayload.code`、§7.1 的上限表一致。


## 15. 协调方集成修订

GPT-6 在执行者修订完成后接管本文件，修正来源缺失时错误合并、原始包 hash 幂等、同请求另存副本重复、设备备份唯一索引冲突，并补充 Rust/TS/SQL 合同。v2 原稿保存在 evidence/technical-design-v2-before-integration.md。此为设计与参考验证，不代表生产代码已实现。
