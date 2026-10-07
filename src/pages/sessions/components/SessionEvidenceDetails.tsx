import type {
  MigratableSession,
  RestoreAttempt,
  SessionMeta,
} from "../../../shared/features/session-migration";
import { RESTORE_STAGE_LABELS } from "../../../shared/features/session-migration";

export function SessionEvidenceDetails({
  session,
  preview,
  attempt,
}: {
  session?: SessionMeta;
  preview?: MigratableSession | null;
  attempt?: RestoreAttempt | null;
}) {
  return (
    <details className="fy-session-evidence-details">
      <summary>来源与验证详情</summary>
      <div className="fy-summary-details">
        {session && <div>源会话 ID：{session.sessionId}</div>}
        {preview && (
          <>
            <div>来源 ID：{preview.origin.originId}</div>
            <div>快照 ID：{preview.snapshotId}</div>
            <div>内容 digest：{preview.contentDigest}</div>
            <div>提取规则：{preview.extraction.ruleId}</div>
          </>
        )}
        {attempt && (
          <>
            <div>恢复尝试 ID：{attempt.attemptId}</div>
            <div>来源 ID：{attempt.origin.originId}</div>
            <div>快照 ID：{attempt.snapshotId}</div>
            <div>内容 digest：{attempt.contentDigest}</div>
            <div>目标原生 ID：{attempt.targetNativeId || "待分配"}</div>
            <div>系统回执阶段：{RESTORE_STAGE_LABELS[attempt.stage]}</div>
          </>
        )}
        <div>
          验证边界：目标写入、原生读回、打开软件、重启读回、下一轮请求与真实模型回复分别记录。
          用户自报不能替代系统回执，也不能证明端到端或跨平台验证通过。
        </div>
      </div>
    </details>
  );
}
