import { Button } from "../../../shared/ui/Button";

import { WarningIcon } from "@phosphor-icons/react/dist/csr/Warning";
import { CheckCircleIcon } from "@phosphor-icons/react/dist/csr/CheckCircle";
import { InfoIcon } from "@phosphor-icons/react/dist/csr/Info";
import { XCircleIcon } from "@phosphor-icons/react/dist/csr/XCircle";
import { ClockIcon } from "@phosphor-icons/react/dist/csr/Clock";
import { ArrowsClockwiseIcon } from "@phosphor-icons/react/dist/csr/ArrowsClockwise";

import {
  parseMigrationError,
  type RestoreAttempt,
  type RestoreStage,
} from "../../../shared/features/session-migration";

export interface StatusBannerProps {
  stage?: RestoreStage;
  isIndeterminate?: boolean;
  hasIncompleteTurn?: boolean;
  isCapabilityVerified?: boolean;
  capabilityReason?: string;
  isCodexProbeWarning?: boolean;
  activeAttempt?: RestoreAttempt | null;
  structuredError?: { code: string; message: string } | null;
  onOpenAttestationModal?: () => void;
  onVerifyReadback?: () => void;
  verifyingReadback?: boolean;
}

export function StatusBanners({
  stage,
  isIndeterminate,
  hasIncompleteTurn,
  isCapabilityVerified = true,
  capabilityReason,
  isCodexProbeWarning,
  activeAttempt,
  structuredError,
  onOpenAttestationModal,
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
        <div className="fy-status-banner banner-danger" role="alert">
          <div className="fy-status-banner-icon">
            <XCircleIcon size={20} weight="fill" />
          </div>
          <div className="fy-status-banner-content">
            <div className="fy-status-banner-title">
              会话处理错误 · {structuredError.code}
            </div>
            <div className="fy-status-banner-desc">
              {structuredError.message}
            </div>
          </div>
        </div>
      )}

      {/* 1. 最终答复待判定：最高优先级阻断提示 */}
      {isIndeterminate && (
        <div className="fy-status-banner banner-danger" role="alert">
          <div className="fy-status-banner-icon">
            <XCircleIcon size={20} weight="fill" />
          </div>
          <div className="fy-status-banner-content">
            <div className="fy-status-banner-title">
              最终答复待判定 · 强行阻断导出与恢复写入
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

      {/* 4. Codex 0.154.0 探针警告 */}
      {isCodexProbeWarning && (
        <div className="fy-status-banner banner-info" role="status">
          <div className="fy-status-banner-icon">
            <InfoIcon size={20} weight="bold" />
          </div>
          <div className="fy-status-banner-content">
            <div className="fy-status-banner-title">Codex 版本支持</div>
            <div className="fy-status-banner-desc">
              当前已验证 Codex 0.154.0
              的历史恢复。请核对完整历史后再发送新消息。
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
            {/* 用户主观自报提示与按钮 */}
            {activeAttempt?.userAttestation ? (
              <div className="fy-user-attestation-notice">
                <InfoIcon size={14} />
                <span>
                  用户自报标记：已手动确认续聊（
                  {activeAttempt.userAttestation.note || "无备注"}
                  ，记录于{" "}
                  {new Date(
                    activeAttempt.userAttestation.attestedAt,
                  ).toLocaleTimeString()}
                  ）。注：此为主观标记，不替代系统读回客观证据。
                </span>
              </div>
            ) : (
              onOpenAttestationModal && (
                <div className="fy-banner-action-row" style={{ marginTop: 6 }}>
                  <Button
                    type="button"
                    className="fy-control-button-subtle"
                    onClick={onOpenAttestationModal}
                  >
                    <span>标记：我已手动续聊</span>
                  </Button>
                </div>
              )
            )}
          </div>
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
              目标软件已完整读取这段会话的原文。你可以打开会话并发送新消息，继续对话。
            </div>
          </div>
        </div>
      )}

      {/* 7. 写入失败提示 */}
      {stage === "failed" && (
        <div className="fy-status-banner banner-danger" role="alert">
          <div className="fy-status-banner-icon">
            <XCircleIcon size={20} weight="fill" />
          </div>
          <div className="fy-status-banner-content">
            <div className="fy-status-banner-title">会话恢复写入失败</div>
            <div className="fy-status-banner-desc">
              写入目标软件本地存储时发生错误
              {activeAttempt?.lastError
                ? `（原因: ${parseMigrationError(activeAttempt.lastError).message}）`
                : ""}
              。请检查本地客户端状态与目录权限。
            </div>
          </div>
        </div>
      )}

      {/* 8. 对账警告 */}
      {stage === "needsReconciliation" && (
        <div className="fy-status-banner banner-danger" role="alert">
          <div className="fy-status-banner-icon">
            <WarningIcon size={20} weight="fill" />
          </div>
          <div className="fy-status-banner-content">
            <div className="fy-status-banner-title">写入结果尚未确认</div>
            <div className="fy-status-banner-desc">
              目前无法确认目标软件是否完整保存了会话，禁止盲目重试。请先核对恢复记录与目标软件的会话列表。
            </div>
          </div>
        </div>
      )}

      {/* 9. 状态待确认 */}
      {stage === "ambiguous" && (
        <div className="fy-status-banner banner-warning" role="status">
          <div className="fy-status-banner-icon">
            <WarningIcon size={20} weight="bold" />
          </div>
          <div className="fy-status-banner-content">
            <div className="fy-status-banner-title">恢复状态待确认</div>
            <div className="fy-status-banner-desc">
              恢复状态未完全确认，请勿重复提交，建议前往目标客户端核对或查看历史记录。
            </div>
          </div>
        </div>
      )}

      {/* 10. 恢复处理中 */}
      {stage === "nativeWritePending" && (
        <div className="fy-status-banner banner-info" role="status">
          <div className="fy-status-banner-icon">
            <ClockIcon size={20} weight="bold" />
          </div>
          <div className="fy-status-banner-content">
            <div className="fy-status-banner-title">恢复处理中</div>
            <div className="fy-status-banner-desc">
              恢复操作尚未完成，请查看恢复记录，不要重复创建副本。
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
