import {
  parseMigrationError,
  RESTORE_STAGE_LABELS,
  type RestoreAttempt,
  type RestoreRequest,
} from "../../shared/features/session-migration";

export interface SessionFailureFeedback {
  code?: string;
  phase: string;
  message: string;
  writeSummary: string;
  nextStep: string;
}

const SOURCE_CODES = new Set([
  "sourceUnreadable",
  "sourceTooLarge",
  "sessionNotFound",
  "session_not_found",
]);
const EXTRACTION_CODES = new Set([
  "extractionRuleUnavailable",
  "extractionRuleVersionMismatch",
  "finalAnswerIndeterminate",
  "runtimeInjectionUnclassified",
]);

// The invocation boundary proves whether the target writer was called. Error
// codes select copy within read-only operations; they never prove write safety.
export function readFailureFeedback(
  error: unknown,
  operation: "source" | "package" | "extraction" | "selection",
): SessionFailureFeedback {
  const parsed = parseMigrationError(error);
  return {
    code: parsed.code,
    phase:
      operation === "selection"
        ? "文件或目录选择"
        : SOURCE_CODES.has(parsed.code) || operation === "source"
          ? "源读取"
          : EXTRACTION_CODES.has(parsed.code) || operation === "extraction"
            ? "会话提取"
            : "迁移包校验",
    message: parsed.message,
    writeSummary: "未调用目标恢复，未写入目标会话。",
    nextStep: "草稿已保留。检查文件、编码或客户端版本，修正后重试同一动作。",
  };
}

export function unresolvedRestoreFeedback(
  error: unknown,
): SessionFailureFeedback {
  return {
    phase: "目标恢复调用 · 未收到完整结果",
    message: parseMigrationError(error).message,
    writeSummary: "本次选择的目标会话是否写入尚未确认，可能存在部分写入。",
    nextStep:
      "草稿与本次请求已保留。先核对恢复回执；结果未确认时禁止再次写入或另存副本。",
  };
}

// Persisted local system stage is the authority. Failed is emitted only after
// positive no-side-effect evidence. Its historical failure phase is unavailable;
// an allocated ID, lastError or user attestation does not supply that phase.
export function restoreAttemptFeedback(
  attempt: RestoreAttempt,
): SessionFailureFeedback {
  const message = attempt.lastError
    ? parseMigrationError(attempt.lastError).message
    : "";
  switch (attempt.stage) {
    case "packageVerified":
      return {
        phase: "迁移包已核验",
        message,
        writeSummary: "回执记录尚未进入目标写入。",
        nextStep: "核对恢复记录后再继续，不重复创建副本。",
      };
    case "nativeWritten":
      return {
        phase: "目标读回",
        message,
        writeSummary: "已写入目标会话，完整历史尚未通过目标读回。",
        nextStep: "重新执行系统读回核验，不再次写入。",
      };
    case "nativeReadbackVerified":
    case "targetOpened":
    case "restartReadbackVerified":
    case "nextTurnRequestVerified":
    case "nextTurnReplyVerified":
      return {
        phase: RESTORE_STAGE_LABELS[attempt.stage],
        message,
        writeSummary: "已写入，目标完整历史已通过系统读回。",
        nextStep: "可打开目标会话继续；用户确认与系统阶段分别记录。",
      };
    case "failed":
      return {
        phase: "恢复失败 · 失败阶段未记录",
        message,
        writeSummary: "本机权威回执已确认该会话没有写入副作用。",
        nextStep:
          "完整选中集合均已确认未写入后，可重试同一请求；不重写已完成或未确认的会话。",
      };
    case "nativeWritePending":
      return {
        phase: "目标写入处理中",
        message,
        writeSummary: "已进入目标写入调用，结果尚未确认。",
        nextStep: "等待并核对恢复回执，不重复提交。",
      };
    case "ambiguous":
    case "needsReconciliation":
      return {
        phase: "恢复结果核对",
        message,
        writeSummary: "目标会话是否完整写入尚未确认，可能存在部分写入。",
        nextStep: "先核对恢复回执与目标会话列表，禁止盲目重试。",
      };
  }
}

export function hasUnconfirmedRestore(attempts: RestoreAttempt[]): boolean {
  return (
    attempts.length === 0 ||
    attempts.some((attempt) =>
      [
        "ambiguous",
        "needsReconciliation",
        "nativeWritePending",
        "packageVerified",
        "nativeWritten",
      ].includes(attempt.stage),
    )
  );
}

export function canRetryRestoreRequest(
  attempts: RestoreAttempt[],
  request: RestoreRequest | null,
): boolean {
  if (!request) return false;
  const { requestId, snapshotIds, targetProviderId, requestKind } = request;
  // Workspace normalization is native-owned. Replay the frozen original
  // request; the persisted fingerprint validates its canonical workspace.
  return (
    Boolean(requestId) &&
    snapshotIds.length > 0 &&
    attempts.length > 0 &&
    attempts.every(
      (attempt) =>
        attempt.requestId === requestId &&
        attempt.targetProviderId === targetProviderId &&
        attempt.requestKind === requestKind &&
        snapshotIds.includes(attempt.snapshotId) &&
        attempt.stage === "failed",
    ) &&
    snapshotIds.every((id) =>
      attempts.some((attempt) => attempt.snapshotId === id),
    )
  );
}
