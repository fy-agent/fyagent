import * as z from "zod/mini";

// ─── Core Enums & Types ──────────────────────────────────────────

export type MessageKind = "userText" | "assistantFinal";
export const messageKindSchema = z.enum(["userText", "assistantFinal"]);

export type PathFamily = "posix" | "windows" | "unknown";
export const pathFamilySchema = z.enum(["posix", "windows", "unknown"]);

export type RestoreRequestKind = "defaultImport" | "saveAsNewCopy";
export const restoreRequestKindSchema = z.enum([
  "defaultImport",
  "saveAsNewCopy",
]);

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

export const restoreStageSchema = z.enum([
  "packageVerified",
  "nativeWritePending",
  "nativeWritten",
  "nativeReadbackVerified",
  "targetOpened",
  "restartReadbackVerified",
  "nextTurnRequestVerified",
  "nextTurnReplyVerified",
  "needsReconciliation",
  "ambiguous",
  "failed",
]);

// ─── Identity & Messages ─────────────────────────────────────────

export interface OriginIdentity {
  originId: string;
  providerId: string;
  sessionId?: string;
  cliVersion?: string;
  storeFingerprint?: string;
}

export const originIdentitySchema = z.strictObject({
  originId: z.string(),
  providerId: z.string(),
  sessionId: z.optional(z.string()),
  cliVersion: z.optional(z.string()),
  storeFingerprint: z.optional(z.string()),
});

export interface MigratableMessage {
  seq: number;
  kind: MessageKind;
  text: string;
  ts?: number;
}

export const migratableMessageSchema = z.strictObject({
  seq: z.number(),
  kind: messageKindSchema,
  text: z.string(),
  ts: z.optional(z.number()),
});

export interface OmittedCounts {
  toolEvents: number;
  reasoningBlocks: number;
  commentaryMessages: number;
  attachments: number;
  runtimeInjections: number;
  unknownBlocks: number;
}

export const omittedCountsSchema = z.strictObject({
  toolEvents: z.number(),
  reasoningBlocks: z.number(),
  commentaryMessages: z.number(),
  attachments: z.number(),
  runtimeInjections: z.number(),
  unknownBlocks: z.number(),
});

export interface ExtractionReport {
  ruleId: string;
  ruleVerifiedVersions: string[];
  openUserMessages: number[];
  omitted: OmittedCounts;
  sourcePathFamily: PathFamily;
}

export const extractionReportSchema = z.strictObject({
  ruleId: z.string(),
  ruleVerifiedVersions: z.array(z.string()),
  openUserMessages: z.array(z.number()),
  omitted: omittedCountsSchema,
  sourcePathFamily: pathFamilySchema,
});

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

export const migratableSessionSchema = z.strictObject({
  snapshotId: z.string(),
  contentDigest: z.string(),
  origin: originIdentitySchema,
  title: z.optional(z.string()),
  createdAt: z.optional(z.number()),
  lastActiveAt: z.optional(z.number()),
  workspaceLabel: z.optional(z.string()),
  messages: z.array(migratableMessageSchema),
  extraction: extractionReportSchema,
});

export interface SessionPackage {
  schema: string;
  exportedAt: number;
  exporter: {
    app: string;
    appVersion: string;
    platform: string;
  };
  sessions: MigratableSession[];
}

export const sessionPackageSchema = z.strictObject({
  schema: z.literal("fyagent.session.v1"),
  exportedAt: z.number(),
  exporter: z.strictObject({
    app: z.string(),
    appVersion: z.string(),
    platform: z.string(),
  }),
  sessions: z.array(migratableSessionSchema),
});

export interface UserAttestation {
  attestedAt: number;
  claimedStage: RestoreStage;
  note?: string;
}

export const userAttestationSchema = z.strictObject({
  attestedAt: z.number(),
  claimedStage: restoreStageSchema,
  note: z.optional(z.string()),
});

export interface MigrationErrorPayload {
  code: string;
  detail?: Record<string, unknown>;
}

export const migrationErrorPayloadSchema = z.strictObject({
  code: z.string(),
  detail: z.optional(z.record(z.string(), z.unknown())),
});

export interface RestoreRequest {
  packagePath: string;
  requestId: string;
  snapshotIds: string[];
  targetProviderId: string;
  targetWorkspace: string;
  requestKind: RestoreRequestKind;
}

export const restoreRequestSchema = z.strictObject({
  packagePath: z.string(),
  requestId: z.string(),
  snapshotIds: z.array(z.string()),
  targetProviderId: z.string(),
  targetWorkspace: z.string(),
  requestKind: restoreRequestKindSchema,
});

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

export const restoreAttemptSchema = z.strictObject({
  attemptId: z.string(),
  requestId: z.string(),
  snapshotId: z.string(),
  requestKind: restoreRequestKindSchema,
  idempotencySlot: z.optional(z.string()),
  contentDigest: z.string(),
  origin: originIdentitySchema,
  targetProviderId: z.string(),
  targetStoreId: z.string(),
  targetNativeNonce: z.string(),
  targetNativeId: z.optional(z.string()),
  targetWorkspace: z.optional(z.string()),
  stage: restoreStageSchema,
  attemptCount: z.number(),
  userAttestation: z.nullable(userAttestationSchema),
  lastError: z.optional(migrationErrorPayloadSchema),
  createdAt: z.number(),
  updatedAt: z.number(),
});

export interface ExtractionRuleInfo {
  ruleId: string;
  verifiedVersions: string[];
}

export const extractionRuleInfoSchema = z.strictObject({
  ruleId: z.string(),
  verifiedVersions: z.array(z.string()),
});

export interface ReleaseCapability {
  providerId: string;
  extractionRule: ExtractionRuleInfo | null;
  writeStrategy: string | null;
  verifiedStages: Array<[RestoreStage, string[]]>;
}

export const releaseCapabilitySchema = z.strictObject({
  providerId: z.string(),
  extractionRule: z.nullable(extractionRuleInfoSchema),
  writeStrategy: z.nullable(z.string()),
  verifiedStages: z.array(z.tuple([restoreStageSchema, z.array(z.string())])),
});

export interface LocalProviderProbe {
  providerId: string;
  installed: boolean;
  detectedVersion?: string;
  extractionSupported: boolean;
  writeSupported: boolean;
  reasonCode?: string;
}

export const localProviderProbeSchema = z.strictObject({
  providerId: z.string(),
  installed: z.boolean(),
  detectedVersion: z.optional(z.string()),
  extractionSupported: z.boolean(),
  writeSupported: z.boolean(),
  reasonCode: z.optional(z.string()),
});

export interface SessionPackageExportResult {
  path: string;
  sessionCount: number;
  byteLen: number;
  packageFileDigest: string;
}

export const sessionPackageExportResultSchema = z.strictObject({
  path: z.string(),
  sessionCount: z.number(),
  byteLen: z.number(),
  packageFileDigest: z.string(),
});

export interface ReadSessionPackageResult {
  package: SessionPackage;
  attempts: RestoreAttempt[];
}

export const readSessionPackageResultSchema = z.strictObject({
  package: sessionPackageSchema,
  attempts: z.array(restoreAttemptSchema),
});

// ─── Browsing Existing Sessions ──────────────────────────────────

export interface SessionMeta {
  providerId: string;
  sessionId: string;
  title?: string;
  summary?: string;
  projectDir?: string | null;
  createdAt?: number;
  lastActiveAt?: number;
  sourcePath?: string;
  resumeCommand?: string;
}

export const sessionMetaSchema = z.strictObject({
  providerId: z.string(),
  sessionId: z.string(),
  title: z.optional(z.string()),
  summary: z.optional(z.string()),
  projectDir: z.optional(z.nullable(z.string())),
  createdAt: z.optional(z.number()),
  lastActiveAt: z.optional(z.number()),
  sourcePath: z.optional(z.string()),
  resumeCommand: z.optional(z.string()),
});

export interface SessionMessage {
  role: string;
  content: string;
  ts?: number;
}

export const sessionMessageSchema = z.strictObject({
  role: z.string(),
  content: z.string(),
  ts: z.optional(z.number()),
});

// ─── Feature Port Interface ──────────────────────────────────────

export interface SessionMigrationPort {
  listSessions(): Promise<SessionMeta[]>;
  getSessionMessages(
    providerId: string,
    sourcePath: string,
  ): Promise<SessionMessage[]>;
  previewSessionMigration(
    providerId: string,
    sourcePath: string,
  ): Promise<MigratableSession>;
  exportSessionPackage(
    items: Array<{ providerId: string; sourcePath: string }>,
    targetPath: string,
  ): Promise<SessionPackageExportResult>;
  readSessionPackage(path: string): Promise<ReadSessionPackageResult>;
  probeLocalProvider(providerId: string): Promise<LocalProviderProbe>;
  getReleaseCapabilityMatrix(): Promise<ReleaseCapability[]>;
  restoreSessionPackage(request: RestoreRequest): Promise<RestoreAttempt[]>;
  verifyNativeReadback(attemptId: string): Promise<RestoreAttempt>;
  listRestoreAttempts(): Promise<RestoreAttempt[]>;
  reconcileRestoreAttempts(): Promise<RestoreAttempt[]>;
  recordUserAttestation(
    attemptId: string,
    claimedStage: RestoreStage,
    note?: string,
  ): Promise<RestoreAttempt>;
  openRestoredSession(attemptId: string): Promise<boolean>;
  pickDirectory(defaultPath?: string): Promise<string | null>;
  pickPackageFile(): Promise<string | null>;
  pickExportPath(defaultName: string): Promise<string | null>;
}

// ─── Provider Definitions & Constants ────────────────────────────

export const SUPPORTED_PROVIDER_IDS = [
  "codex",
  "opencode",
  "hermes",
  "gemini",
  "claude",
  "grokbuild",
  "openclaw",
] as const;

export type SupportedProviderId = (typeof SUPPORTED_PROVIDER_IDS)[number];

export const PROVIDER_LABELS: Record<string, string> = {
  codex: "Codex",
  opencode: "OpenCode",
  hermes: "Hermes",
  gemini: "Gemini",
  claude: "Claude Code",
  grokbuild: "Grok",
  openclaw: "OpenClaw",
};

export const RESTORE_STAGE_LABELS: Record<RestoreStage, string> = {
  packageVerified: "迁移包已核验",
  nativeWritePending: "等待写入目标软件",
  nativeWritten: "已写入目标存储 · 待读回验证",
  nativeReadbackVerified: "目标存储读回核验通过",
  targetOpened: "已在目标软件中拉起",
  restartReadbackVerified: "目标软件重启读回验证通过",
  nextTurnRequestVerified: "下轮请求历史核验通过",
  nextTurnReplyVerified: "真实模型续聊回复验证通过",
  needsReconciliation: "写入结果尚未确认",
  ambiguous: "状态歧义",
  failed: "恢复失败",
};

// ─── Helpers: Strict Final-Only & Verification Validation ────────

export interface CanExportResult {
  allowed: boolean;
  reason?: string;
}

/**
 * Performs structural validation on an extracted migratable session.
 * Checks for at least one user prompt. Indeterminate or unfinalized responses
 * are caught and blocked during preview extraction RPC or error payload handling;
 * trailing unanswered user messages are preserved per contract.
 */
export function canExportSession(session: MigratableSession): CanExportResult {
  // Ensure there are at least user messages
  const userMessages = session.messages.filter((m) => m.kind === "userText");
  if (userMessages.length === 0) {
    return {
      allowed: false,
      reason: "会话中未检测到用户提示词，无法构成有效问答包。",
    };
  }

  return { allowed: true };
}

/**
 * Builds a clean, human-readable pure text preview of the user prompts and
 * assistant final answers for UI preview, omitting any tool logs or runtime details.
 * Consecutive user messages are rendered sequentially in distinct incomplete turns.
 */
export function buildPureTextPreview(session: MigratableSession): string {
  const lines: string[] = [];
  lines.push(`会话快照: ${session.snapshotId}`);
  lines.push(
    `来源软件: ${PROVIDER_LABELS[session.origin.providerId] ?? session.origin.providerId}`,
  );
  lines.push(`内容摘要: ${session.contentDigest}`);
  if (session.workspaceLabel) {
    lines.push(`工作区标签: ${session.workspaceLabel}`);
  }
  lines.push(`提取规则: ${session.extraction.ruleId}`);
  lines.push(
    `已剔除工具事件: ${session.extraction.omitted.toolEvents} 条, 思考块: ${session.extraction.omitted.reasoningBlocks} 个`,
  );
  lines.push("─".repeat(50));
  lines.push("");

  let currentTurn = 1;
  let pendingUserMsg: MigratableMessage | undefined;

  for (const msg of session.messages) {
    if (msg.kind === "userText") {
      if (pendingUserMsg) {
        lines.push(`【第 ${currentTurn} 轮 · 用户原始提示词（未完成轮次）】`);
        lines.push(pendingUserMsg.text);
        lines.push("");
        lines.push("─".repeat(40));
        lines.push("");
        currentTurn += 1;
      }
      pendingUserMsg = msg;
    } else if (msg.kind === "assistantFinal") {
      if (pendingUserMsg) {
        lines.push(`【第 ${currentTurn} 轮 · 用户原始提示词】`);
        lines.push(pendingUserMsg.text);
        lines.push("");
      }
      lines.push(`【第 ${currentTurn} 轮 · AI最终答复原文】`);
      lines.push(msg.text);
      lines.push("");
      lines.push("─".repeat(40));
      lines.push("");
      currentTurn += 1;
      pendingUserMsg = undefined;
    }
  }

  if (pendingUserMsg) {
    lines.push(`【第 ${currentTurn} 轮 · 用户原始提示词（未完成轮次）】`);
    lines.push(pendingUserMsg.text);
    lines.push("");
    lines.push("─".repeat(40));
    lines.push("");
  }

  return lines.join("\n");
}

/**
 * Generates a stable key for a session that avoids collisions across different
 * providers or source paths sharing identical session IDs.
 */
export function getSessionStableKey(session: {
  providerId: string;
  sourcePath?: string;
  sessionId: string;
}): string {
  return `${session.providerId}::${session.sourcePath || ""}::${session.sessionId}`;
}

/**
 * Computes a deterministic binding key for a restore confirmation action.
 * If user parameters do not change, the exact same requestId must be reused for idempotency.
 * Note: packageDigest is excluded because selected snapshotIds already uniquely cover content.
 */
export function computeRestoreBindingKey(params: {
  packagePath: string;
  packageDigest?: string;
  targetProviderId: string;
  snapshotIds: string[];
  targetWorkspace: string;
  requestKind: RestoreRequestKind;
}): string {
  const sortedIds = [...params.snapshotIds].sort().join(",");
  return [
    params.packagePath.trim(),
    params.targetProviderId,
    sortedIds,
    params.targetWorkspace.trim(),
    params.requestKind,
  ].join("::");
}

export interface ParsedMigrationError {
  code: string;
  message: string;
  detail?: string;
}

const ERROR_CODE_MESSAGES: Record<string, string> = {
  providerVersionUnsupported: "当前客户端版本尚未支持恢复迁移",
  provider_version_unsupported: "当前客户端版本尚未支持恢复迁移",
  unsupportedProvider: "不支持的客户端软件",
  unsupported_provider: "不支持的客户端软件",
  sessionNotFound: "未找到指定的本地会话",
  session_not_found: "未找到指定的本地会话",
  packageCorrupt: "迁移包数据损坏或格式不正确",
  invalidPackage: "迁移包校验失败，缺少必要字段",
  package_corrupted: "迁移包数据损坏",
  nativeWriteFailed: "写入目标软件本地存储失败",
  native_write_failed: "写入目标软件本地存储失败",
  readbackMismatch: "目标存储读回数据与迁移包不一致",
  readback_mismatch: "目标存储读回数据与迁移包不一致",
  idempotencyConflict: "当前恢复请求正在处理或已存在相同绑定",
  idempotency_conflict: "当前恢复请求正在处理或已存在相同绑定",
  needsReconciliation: "写入结果尚未确认，请核对恢复记录",
  needs_reconciliation: "写入结果尚未确认，请核对恢复记录",
  database_locked: "目标数据库正在被占用，请先关闭目标客户端",
  permission_denied: "文件系统权限不足，无法读写目标路径",
  workspace_not_found: "指定的工作区目录不存在",
  probeError: "探测本地客户端状态异常，请检查客户端配置",
  probe_error: "探测本地客户端状态异常，请检查客户端配置",
  probeFailed: "探测本地客户端状态失败",
  probe_failed: "探测本地客户端状态失败",
  providerNotInstalled: "本地未安装该客户端",
  provider_not_installed: "本地未安装该客户端",
};

/**
 * Parses structured error payloads or JSON strings from native Tauri IPC
 * into clean, user-friendly Chinese messages without raw internal JSON or jargon.
 */
export function parseMigrationError(err: unknown): ParsedMigrationError {
  if (!err) {
    return { code: "unknown", message: "未知错误" };
  }

  let rawObj: unknown = err;

  if (typeof err === "string") {
    const trimmed = err.trim();
    if (
      (trimmed.startsWith("{") && trimmed.endsWith("}")) ||
      (trimmed.startsWith("[") && trimmed.endsWith("]"))
    ) {
      try {
        rawObj = JSON.parse(trimmed);
      } catch {
        // Not valid JSON, keep as raw string
      }
    }
  } else if (err instanceof Error) {
    const trimmed = err.message.trim();
    if (trimmed.startsWith("{") && trimmed.endsWith("}")) {
      try {
        rawObj = JSON.parse(trimmed);
      } catch {
        rawObj = { message: err.message };
      }
    } else {
      rawObj = { message: err.message };
    }
  }

  if (typeof rawObj === "object" && rawObj !== null) {
    const rec = rawObj as Record<string, unknown>;
    const code =
      typeof rec.code === "string"
        ? rec.code
        : typeof rec.reasonCode === "string"
          ? rec.reasonCode
          : typeof rec.error === "string"
            ? rec.error
            : "error";

    let detailStr: string | undefined;
    if (typeof rec.detail === "string") {
      detailStr = rec.detail;
    } else if (typeof rec.detail === "object" && rec.detail !== null) {
      detailStr = JSON.stringify(rec.detail);
    }

    let message =
      typeof rec.message === "string"
        ? rec.message
        : typeof rec.msg === "string"
          ? rec.msg
          : typeof rec.reason === "string"
            ? rec.reason
            : "";

    if (ERROR_CODE_MESSAGES[code]) {
      message = ERROR_CODE_MESSAGES[code];
    } else if (!message || message === code) {
      message = `操作遇到问题 (${code})`;
    }

    return {
      code,
      message,
      detail: detailStr,
    };
  }

  const str = String(err);
  return {
    code: "unknown",
    message: str.length > 200 ? str.slice(0, 200) + "…" : str,
  };
}

/**
 * Pure check whether a local provider probe allows session restore.
 */
export function isProviderRestoreSupported(probe?: LocalProviderProbe): {
  supported: boolean;
  reason?: string;
} {
  if (!probe) {
    return { supported: false, reason: "尚未探测到该客户端的安装状态" };
  }
  if (probe.reasonCode && probe.reasonCode !== "providerNotInstalled") {
    const parsed = parseMigrationError({ code: probe.reasonCode });
    if (parsed.message && parsed.message !== "未知错误") {
      return { supported: false, reason: parsed.message };
    }
  }
  if (!probe.installed) {
    return { supported: false, reason: "本地未安装该客户端" };
  }
  if (!probe.writeSupported) {
    return {
      supported: false,
      reason: probe.reasonCode
        ? parseMigrationError({ code: probe.reasonCode }).message
        : "当前版本尚未支持会话写入恢复",
    };
  }
  return { supported: true };
}

export type RestoreResultKind =
  | "none"
  | "empty"
  | "cleanSuccess"
  | "needsReconciliation"
  | "failed"
  | "ambiguous"
  | "pending";

export interface RestoreClassification {
  kind: RestoreResultKind;
  isAllCleanSuccess: boolean;
  hasFailed: boolean;
  hasReconciliation: boolean;
  hasAmbiguous: boolean;
  hasPending: boolean;
  bannerTone: "success" | "error" | "warning" | "info";
  summaryTitle: string;
  summaryDescription: string;
}

/**
 * Classifies a set of restore attempts into unified status categories,
 * ensuring empty receipts fail-closed rather than falsely reporting success.
 */
export function classifyRestoreResults(
  attempts: RestoreAttempt[] | null | undefined,
): RestoreClassification {
  if (!attempts) {
    return {
      kind: "none",
      isAllCleanSuccess: false,
      hasFailed: false,
      hasReconciliation: false,
      hasAmbiguous: false,
      hasPending: false,
      bannerTone: "info",
      summaryTitle: "",
      summaryDescription: "",
    };
  }

  if (attempts.length === 0) {
    return {
      kind: "empty",
      isAllCleanSuccess: false,
      hasFailed: true,
      hasReconciliation: false,
      hasAmbiguous: false,
      hasPending: false,
      bannerTone: "error",
      summaryTitle: "未收到恢复回执",
      summaryDescription:
        "未收到恢复记录，当前结果无法确认。请刷新恢复记录后核对，不要重复创建副本。",
    };
  }

  const hasReconciliation = attempts.some(
    (a) => a.stage === "needsReconciliation",
  );
  const hasFailed = attempts.some((a) => a.stage === "failed");
  const hasAmbiguous = attempts.some((a) => a.stage === "ambiguous");
  const hasPending = attempts.some(
    (a) =>
      a.stage === "nativeWritePending" ||
      a.stage === "packageVerified" ||
      a.stage === "nativeWritten",
  );

  const isAllCleanSuccess =
    !hasReconciliation &&
    !hasFailed &&
    !hasAmbiguous &&
    !hasPending &&
    attempts.length > 0 &&
    attempts.every(
      (a) =>
        a.stage === "nativeReadbackVerified" ||
        a.stage === "targetOpened" ||
        a.stage === "restartReadbackVerified" ||
        a.stage === "nextTurnRequestVerified" ||
        a.stage === "nextTurnReplyVerified",
    );

  if (isAllCleanSuccess) {
    return {
      kind: "cleanSuccess",
      isAllCleanSuccess: true,
      hasFailed: false,
      hasReconciliation: false,
      hasAmbiguous: false,
      hasPending: false,
      bannerTone: "success",
      summaryTitle: "恢复执行完成",
      summaryDescription: `已在目标软件中核对 ${attempts.length} 个会话的完整历史，可以打开后继续对话。`,
    };
  }

  if (hasReconciliation) {
    return {
      kind: "needsReconciliation",
      isAllCleanSuccess: false,
      hasFailed: false,
      hasReconciliation: true,
      hasAmbiguous,
      hasPending,
      bannerTone: "error",
      summaryTitle: "写入结果尚未确认",
      summaryDescription:
        "目前无法确认目标软件是否完整保存了会话，禁止盲目重试。请先核对恢复记录。",
    };
  }

  if (hasFailed) {
    return {
      kind: "failed",
      isAllCleanSuccess: false,
      hasFailed: true,
      hasReconciliation: false,
      hasAmbiguous,
      hasPending,
      bannerTone: "error",
      summaryTitle: "恢复未完全成功",
      summaryDescription:
        "部分会话写入目标客户端失败，请检查工作区目录权限或客户端日志。",
    };
  }

  if (hasAmbiguous) {
    return {
      kind: "ambiguous",
      isAllCleanSuccess: false,
      hasFailed: false,
      hasReconciliation: false,
      hasAmbiguous: true,
      hasPending,
      bannerTone: "warning",
      summaryTitle: "恢复操作待确认",
      summaryDescription:
        "恢复状态未完全确认，请勿重复提交，建议前往目标客户端核对或查看历史记录。",
    };
  }

  return {
    kind: "pending",
    isAllCleanSuccess: false,
    hasFailed: false,
    hasReconciliation: false,
    hasAmbiguous: false,
    hasPending: true,
    bannerTone: "warning",
    summaryTitle: attempts.some((a) => a.stage === "nativeWritten")
      ? "已写入，待读回验证"
      : "恢复处理中",
    summaryDescription: attempts.some((a) => a.stage === "nativeWritten")
      ? "目标软件尚未确认完整历史。请在恢复记录中核验后继续对话。"
      : "恢复操作尚未完成，请查看恢复记录，不要重复创建副本。",
  };
}
