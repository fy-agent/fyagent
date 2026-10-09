import { Button } from "../../../shared/ui/Button";

import { WarningIcon } from "@phosphor-icons/react/dist/csr/Warning";
import { CheckCircleIcon } from "@phosphor-icons/react/dist/csr/CheckCircle";
import { InfoIcon } from "@phosphor-icons/react/dist/csr/Info";
import { XCircleIcon } from "@phosphor-icons/react/dist/csr/XCircle";
import { ClockIcon } from "@phosphor-icons/react/dist/csr/Clock";
import { ArrowsClockwiseIcon } from "@phosphor-icons/react/dist/csr/ArrowsClockwise";

import {
  type RestoreAttempt,
  type RestoreStage,
} from "../../../shared/features/session-migration";
import {
  restoreAttemptFeedback,
  type SessionFailureFeedback,
} from "../failure-feedback";
import { FailureFeedback } from "./FailureFeedback";

export interface StatusBannerProps {
  stage?: RestoreStage;
  isIndeterminate?: boolean;
  hasIncompleteTurn?: boolean;
  isCapabilityVerified?: boolean;
  capabilityReason?: string;
  activeAttempt?: RestoreAttempt | null;
  structuredError?: SessionFailureFeedback | null;
  failureFeedback?: SessionFailureFeedback;
  onRetrySource?: () => void;
  onReviewRestore?: () => void;
  reviewingAttempts?: boolean;
  onVerifyReadback?: () => void;
  verifyingReadback?: boolean;
}

export function StatusBanners({
  stage,
  isIndeterminate,
  hasIncompleteTurn,
  isCapabilityVerified = true,
  capabilityReason,
  activeAttempt,
  structuredError,
  failureFeedback,
  onRetrySource,
  onReviewRestore,
  reviewingAttempts,
  onVerifyReadback,
  verifyingReadback,
}: StatusBannerProps) {
  return (
    <div
      className="fy-session-status-banners"
      role="region"
      aria-label="会话状态与验证提示"
    >
      {/* 0. 结构化错误提示 */}
      {structuredError && (
        <div>
          <FailureFeedback feedback={structuredError} />
          {onRetrySource && (
            <Button type="button" onClick={onRetrySource}>
              重新读取与提取
            </Button>
          )}
        </div>
      )}
      {failureFeedback && (
        <FailureFeedback
          feedback={failureFeedback}
          onReview={onReviewRestore}
          reviewing={reviewingAttempts}
        />
      )}

      {/* 1. 最终答复待判定：最高优先级阻断提示 */}
      {isIndeterminate && (
        <div className="fy-status-banner banner-danger" role="alert">
          <div className="fy-status-banner-icon">
            <XCircleIcon size={20} weight="fill" />
          </div>
          <div className="fy-status-banner-content">
            <div className="fy-status-banner-title">
              最终答复待判定 · 此来源暂不可导出
            </div>
            <div className="fy-status-banner-desc">
              这段会话中有答复尚未完成或状态不明确，无法可靠确定最终答复原文。请等待答复完成后重新预览。
            </div>
          </div>
        </div>
      )}

      {/* 2. 未完成轮次提示 */}
      {!isIndeterminate && hasIncompleteTurn && (
        <div className="fy-status-banner banner-warning" role="status">
          <div className="fy-status-banner-icon">
            <ClockIcon size={20} weight="bold" />
          </div>
          <div className="fy-status-banner-content">
            <div className="fy-status-banner-title">
              包含未完成轮次 · 仅保留用户提示词原文
            </div>
            <div className="fy-status-banner-desc">
              会话中部分轮次未获得模型最终答复。目标软件不会自动代跑旧指令或继续生成，迁移后请在目标客户端中主动发送新消息。
            </div>
          </div>
        </div>
      )}

      {/* 3. 软件恢复支持未验证 */}
      {!isCapabilityVerified && (
        <div className="fy-status-banner banner-amber" role="status">
          <div className="fy-status-banner-icon">
            <WarningIcon size={20} weight="bold" />
          </div>
          <div className="fy-status-banner-content">
            <div className="fy-status-banner-title">
              当前版本的恢复能力尚未验证
            </div>
            <div className="fy-status-banner-desc">
              当前无法将会话恢复到这个软件版本
              {capabilityReason ? `（原因: ${capabilityReason}）` : ""}。
              请使用同一软件的已支持版本。
            </div>
          </div>
        </div>
      )}

      {/* 5. 写入目标存储完成待验证 */}
      {stage === "nativeWritten" && (
        <div className="fy-status-banner banner-warning" role="status">
          <div className="fy-status-banner-icon">
            <CheckCircleIcon size={20} weight="bold" />
          </div>
          <div className="fy-status-banner-content">
            <div className="fy-status-banner-title">
              已写入目标存储 · 待读回验证
            </div>
            <div className="fy-status-banner-desc">
              已写入目标软件，尚未确认它能完整读取历史。请先核验会话内容。
            </div>
            {onVerifyReadback && (
              <div className="fy-banner-action-row">
                <Button
                  type="button"
                  className="fy-control-button-subtle"
                  disabled={verifyingReadback}
                  onClick={onVerifyReadback}
                >
                  <ArrowsClockwiseIcon
                    size={14}
                    className={verifyingReadback ? "spinning" : ""}
                  />
                  <span>
                    {verifyingReadback ? "正在核验…" : "系统读回核验"}
                  </span>
                </Button>
              </div>
            )}
          </div>
        </div>
      )}

      {activeAttempt?.userAttestation && (
        <div className="fy-user-attestation-notice" role="note">
          <InfoIcon size={14} />
          <span>
            用户自报标记：已手动确认续聊， 记录于{" "}
            {new Date(
              activeAttempt.userAttestation.attestedAt,
            ).toLocaleString()}
            。 系统阶段保持回执记录；此标记不替代系统读回或真实回复证据。
          </span>
          {activeAttempt.userAttestation.note && (
            <details>
              <summary>用户备注</summary>
              <div>{activeAttempt.userAttestation.note}</div>
            </details>
          )}
        </div>
      )}

      {/* 6. 系统读回核验成功 */}
      {stage === "nativeReadbackVerified" && (
        <div className="fy-status-banner banner-success" role="status">
          <div className="fy-status-banner-icon">
            <CheckCircleIcon size={20} weight="fill" />
          </div>
          <div className="fy-status-banner-content">
            <div className="fy-status-banner-title">目标历史核验通过</div>
            <div className="fy-status-banner-desc">
              系统已通过目标原生读回核验这段会话的原文。打开软件、重启和真实续聊分别核验。
            </div>
          </div>
        </div>
      )}

      {activeAttempt &&
        [
          "failed",
          "needsReconciliation",
          "ambiguous",
          "nativeWritePending",
          "packageVerified",
        ].includes(activeAttempt.stage) && (
          <div className="fy-status-banner banner-danger" role="alert">
            <div className="fy-status-banner-content">
              <div className="fy-status-banner-title">
                {stage === "needsReconciliation"
                  ? "写入结果尚未确认"
                  : stage === "ambiguous"
                    ? "恢复状态待确认"
                    : stage === "failed"
                      ? "会话恢复未完成"
                      : "恢复处理中"}
              </div>
              <div className="fy-status-banner-desc">
                <div>阶段：{restoreAttemptFeedback(activeAttempt).phase}</div>
                <div>{restoreAttemptFeedback(activeAttempt).message}</div>
                <div>{restoreAttemptFeedback(activeAttempt).writeSummary}</div>
                <div>{restoreAttemptFeedback(activeAttempt).nextStep}</div>
              </div>
              {onReviewRestore && (
                <div className="fy-banner-action-row">
                  <Button
                    type="button"
                    disabled={reviewingAttempts}
                    onClick={onReviewRestore}
                  >
                    {reviewingAttempts ? "正在核对…" : "核对恢复回执"}
                  </Button>
                </div>
              )}
            </div>
          </div>
        )}
    </div>
  );
}
