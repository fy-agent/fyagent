import type {
  LocalProviderProbe,
  RestoreAttempt,
} from "../../../shared/features/session-migration";
import { restoreAttemptFeedback } from "../failure-feedback";
import {
  isProviderRestoreSupported,
  RESTORE_STAGE_LABELS,
} from "../../../shared/features/session-migration";

export function SessionStages({
  sourceRead,
  extraction,
  probe,
  attempt,
}: {
  sourceRead?: "pending" | "available" | "failed";
  extraction?: "pending" | "available" | "unavailable";
  probe?: LocalProviderProbe;
  attempt?: RestoreAttempt | null;
}) {
  const writer = isProviderRestoreSupported(probe);
  const stage = attempt?.stage;
  const targetWritten =
    stage !== undefined &&
    [
      "nativeWritten",
      "nativeReadbackVerified",
      "targetOpened",
      "restartReadbackVerified",
      "nextTurnRequestVerified",
      "nextTurnReplyVerified",
    ].includes(stage);
  const targetReadback =
    stage !== undefined &&
    [
      "nativeReadbackVerified",
      "targetOpened",
      "restartReadbackVerified",
      "nextTurnRequestVerified",
      "nextTurnReplyVerified",
    ].includes(stage);
  // Only persisted system stages count. A user claim never advances a fact.
  const nativeLoop =
    stage === "nextTurnReplyVerified"
      ? "系统已验证真实模型回复"
      : stage === "nextTurnRequestVerified"
        ? "系统已验证下一轮请求；真实回复尚未核验"
        : stage === "restartReadbackVerified"
          ? "系统已验证重启后读回；下一轮请求与回复尚未核验"
          : stage === "targetOpened"
            ? "系统回执记录目标已打开；重启与续聊尚未核验"
            : "软件实际打开、重启和真实续聊闭环尚未核验";
  return (
    <div
      className="fy-status-banner banner-info"
      role="region"
      aria-label="会话能力与恢复阶段"
    >
      <div className="fy-status-banner-content">
        <dl>
          {sourceRead && (
            <>
              <dt>源读取</dt>
              <dd>
                {sourceRead === "available"
                  ? "原文已读取"
                  : sourceRead === "failed"
                    ? "读取失败"
                    : "正在读取"}
              </dd>
              <dt>提取与导出</dt>
              <dd>
                {extraction === "available"
                  ? "问答预览可用，导出时核验所选会话"
                  : extraction === "unavailable"
                    ? "提取不可用；可用原文仍可查看"
                    : "等待提取预览"}
              </dd>
            </>
          )}
          <dt>目标写入能力</dt>
          <dd>
            {writer.supported
              ? "本机版本支持写入恢复"
              : "本机写入能力尚未确认或不受支持"}
            {probe?.detectedVersion ? `（${probe.detectedVersion}）` : ""}
          </dd>
          <dt>包导入</dt>
          <dd>
            {attempt ? "会话包已核验并建立恢复回执" : "尚无匹配的包导入回执"}
          </dd>
          <dt>目标写入</dt>
          <dd>
            {targetWritten
              ? "系统已确认写入目标存储"
              : attempt
                ? restoreAttemptFeedback(attempt).writeSummary
                : "尚未确认目标写入"}
          </dd>
          <dt>目标读回</dt>
          <dd>
            {targetReadback ? "系统已核验目标本地历史" : "目标读回尚未核验通过"}
          </dd>
          <dt>实机闭环</dt>
          <dd>{nativeLoop}</dd>
          <dt>系统恢复确认</dt>
          <dd>
            {attempt ? RESTORE_STAGE_LABELS[attempt.stage] : "尚无匹配恢复回执"}
          </dd>
        </dl>
      </div>
    </div>
  );
}
