import { CheckIcon } from "@phosphor-icons/react/dist/csr/Check";
import { FolderSimpleIcon } from "@phosphor-icons/react/dist/csr/FolderSimple";
import { InfoIcon } from "@phosphor-icons/react/dist/csr/Info";
import { PlayIcon } from "@phosphor-icons/react/dist/csr/Play";

import { Button } from "../../../shared/ui/Button";
import type { DialogOriginRef } from "../../../shared/ui/dialogOrigin";
import {
  PROVIDER_LABELS,
  RESTORE_STAGE_LABELS,
  type LocalProviderProbe,
  type MigratableSession,
  type RestoreAttempt,
  type RestoreStage,
  type SessionMessage,
  type SessionMeta,
} from "../../../shared/features/session-migration";
import type { SessionFailureFeedback } from "../failure-feedback";
import {
  ConversationStream,
  type ConversationTurn,
} from "./ConversationStream";
import { SessionEvidenceDetails } from "./SessionEvidenceDetails";
import { SessionStages } from "./SessionStages";
import { StatusBanners } from "./StatusBanners";

export interface SessionsDetailPaneProps {
  viewMode: "sessions" | "attempts";
  selectedSession: SessionMeta | null;
  activeAttempt: RestoreAttempt | null;
  activeProviderProbe: LocalProviderProbe | undefined;
  canOpenTarget: boolean;
  isCapabilityVerified: boolean;
  capabilityReason?: string;
  targetWorkspace: string;
  sourceRead: "pending" | "available" | "failed";
  loadingDetail: boolean;
  migratableSession: MigratableSession | null;
  structuredError: SessionFailureFeedback | null;
  hasIndeterminate: boolean;
  hasIncomplete: boolean;
  conversationTurns: ConversationTurn[];
  rawMessages: SessionMessage[];
  readbackError: { attemptId: string; feedback: SessionFailureFeedback } | null;
  reviewingAttempts: boolean;
  verifyingReadback: boolean;
  remapOriginRef: DialogOriginRef;
  guideOriginRef: DialogOriginRef;
  attestationOriginRef: DialogOriginRef;
  onOpenRemap: () => void;
  onOpenGuide: () => void;
  onOpenAttestation: () => void;
  onResumeInTarget: () => void;
  onRetrySource: () => void;
  onReviewRestoreWithFeedback: (attempt: RestoreAttempt | null) => void;
  onVerifyReadback: () => void;
}

export function SessionsDetailPane({
  viewMode,
  selectedSession,
  activeAttempt,
  activeProviderProbe,
  canOpenTarget,
  isCapabilityVerified,
  capabilityReason,
  targetWorkspace,
  sourceRead,
  loadingDetail,
  migratableSession,
  structuredError,
  hasIndeterminate,
  hasIncomplete,
  conversationTurns,
  rawMessages,
  readbackError,
  reviewingAttempts,
  verifyingReadback,
  remapOriginRef,
  guideOriginRef,
  attestationOriginRef,
  onOpenRemap,
  onOpenGuide,
  onOpenAttestation,
  onResumeInTarget,
  onRetrySource,
  onReviewRestoreWithFeedback,
  onVerifyReadback,
}: SessionsDetailPaneProps) {
  return (
    <main className="fy-sessions-detail-pane" aria-label="会话详情">
      {viewMode === "attempts" && activeAttempt ? (
        <div className="fy-attempt-detail-view">
          <div className="fy-sessions-detail-header">
            <div className="fy-detail-top-row">
              <div className="fy-detail-title-group">
                <h1 className="fy-detail-title">
                  恢复记录：
                  {PROVIDER_LABELS[activeAttempt.targetProviderId] ??
                    activeAttempt.targetProviderId}{" "}
                </h1>
                <div className="fy-detail-meta-tags">
                  <span className="fy-session-item-provider-tag">
                    {PROVIDER_LABELS[activeAttempt.targetProviderId] ??
                      activeAttempt.targetProviderId}
                  </span>
                  <span>
                    当前阶段:{" "}
                    {RESTORE_STAGE_LABELS[
                      activeAttempt.stage as RestoreStage
                    ] ?? activeAttempt.stage}
                  </span>
                  <span>•</span>
                  <span>
                    创建于: {new Date(activeAttempt.createdAt).toLocaleString()}
                  </span>
                </div>
              </div>

              <div className="fy-detail-actions">
                <Button
                  className="fy-control-button-primary"
                  disabled={!canOpenTarget}
                  onClick={onResumeInTarget}
                >
                  <PlayIcon size={14} weight="fill" />
                  <span>
                    {activeAttempt ? "打开目标会话" : "查看目标启动说明"}
                  </span>
                </Button>
              </div>
            </div>

            <div className="fy-workspace-bar">
              <span
                style={{
                  color: "var(--fy-text-tertiary)",
                  fontWeight: 550,
                  whiteSpace: "nowrap",
                }}
              >
                目标工作区路径:
              </span>
              <span className="fy-workspace-path">
                {activeAttempt.targetWorkspace}
              </span>
            </div>
          </div>

          <div className="fy-sessions-stream-container">
            <SessionStages
              probe={activeProviderProbe}
              attempt={activeAttempt}
            />
            <StatusBanners
              stage={activeAttempt.stage}
              activeAttempt={activeAttempt}
              failureFeedback={
                readbackError?.attemptId === activeAttempt.attemptId
                  ? readbackError.feedback
                  : undefined
              }
              onReviewRestore={() => onReviewRestoreWithFeedback(activeAttempt)}
              reviewingAttempts={reviewingAttempts}
              isCapabilityVerified={isCapabilityVerified}
              capabilityReason={capabilityReason}
              onVerifyReadback={onVerifyReadback}
              verifyingReadback={verifyingReadback}
            />
          </div>

          <footer className="fy-sessions-bottom-bar">
            <div className="fy-bottom-info">
              <SessionEvidenceDetails attempt={activeAttempt} />
            </div>
            <div className="fy-bottom-actions">
              <Button
                className="fy-control-button-primary"
                dialogOriginRef={attestationOriginRef}
                onClick={onOpenAttestation}
              >
                <CheckIcon size={16} weight="bold" />
                <span>标记：我已手动续聊</span>
              </Button>
            </div>
          </footer>
        </div>
      ) : selectedSession ? (
        <>
          <div className="fy-sessions-detail-header">
            <div className="fy-detail-top-row">
              <div className="fy-detail-title-group">
                <h1 className="fy-detail-title">
                  {selectedSession.title ||
                    selectedSession.summary ||
                    "未命名会话"}
                </h1>
                <div className="fy-detail-meta-tags">
                  <span className="fy-session-item-provider-tag">
                    {PROVIDER_LABELS[selectedSession.providerId] ??
                      selectedSession.providerId}
                  </span>
                  <span>
                    共{" "}
                    {conversationTurns.length > 0
                      ? `${conversationTurns.length} 轮问答`
                      : `${rawMessages.length} 条原始消息`}
                  </span>
                  {selectedSession.lastActiveAt && (
                    <>
                      <span>•</span>
                      <span>
                        {new Date(
                          selectedSession.lastActiveAt,
                        ).toLocaleString()}
                      </span>
                    </>
                  )}
                </div>
              </div>

              <div className="fy-detail-actions">
                <Button dialogOriginRef={remapOriginRef} onClick={onOpenRemap}>
                  <FolderSimpleIcon size={16} />
                  <span>绑定本机目录</span>
                </Button>
                <Button
                  className="fy-control-button-primary"
                  disabled={!canOpenTarget}
                  onClick={onResumeInTarget}
                >
                  <PlayIcon size={14} weight="fill" />
                  <span>
                    {activeAttempt ? "打开目标会话" : "查看目标启动说明"}
                  </span>
                </Button>
              </div>
            </div>

            <div className="fy-workspace-bar">
              <div
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: "8px",
                  overflow: "hidden",
                }}
              >
                <span
                  style={{
                    color: "var(--fy-text-tertiary)",
                    fontWeight: 550,
                    whiteSpace: "nowrap",
                  }}
                >
                  目标本地工作区:
                </span>
                <span className="fy-workspace-path">
                  {targetWorkspace || "尚未指定本地工程目录"}
                </span>
              </div>
              {migratableSession?.workspaceLabel && (
                <span
                  style={{
                    fontSize: "11px",
                    padding: "1px 6px",
                    borderRadius: "4px",
                    background: "var(--fy-surface-inset)",
                    color: "var(--fy-text-tertiary)",
                  }}
                >
                  原标签: {migratableSession.workspaceLabel}
                </span>
              )}
              <span className="fy-workspace-fidelity-hint">
                （对话正文内的历史路径与代码保持原文不变）
              </span>
            </div>
          </div>

          <div className="fy-sessions-stream-container">
            <SessionStages
              sourceRead={sourceRead}
              extraction={
                loadingDetail
                  ? "pending"
                  : migratableSession
                    ? "available"
                    : "unavailable"
              }
              probe={activeProviderProbe}
              attempt={activeAttempt}
            />
            <StatusBanners
              stage={activeAttempt?.stage}
              isIndeterminate={hasIndeterminate}
              hasIncompleteTurn={hasIncomplete}
              isCapabilityVerified={isCapabilityVerified}
              capabilityReason={capabilityReason}
              activeAttempt={activeAttempt}
              structuredError={structuredError}
              onRetrySource={onRetrySource}
              failureFeedback={
                readbackError?.attemptId === activeAttempt?.attemptId
                  ? readbackError?.feedback
                  : undefined
              }
              onReviewRestore={() => onReviewRestoreWithFeedback(activeAttempt)}
              reviewingAttempts={reviewingAttempts}
              onVerifyReadback={onVerifyReadback}
              verifyingReadback={verifyingReadback}
            />

            {loadingDetail ? (
              <div
                style={{
                  padding: "32px",
                  textAlign: "center",
                  color: "var(--fy-text-tertiary)",
                }}
              >
                正在读取会话问答流…
              </div>
            ) : (
              <ConversationStream
                turns={conversationTurns}
                rawMessages={rawMessages}
                isRawFallback={!migratableSession}
              />
            )}
          </div>

          <footer className="fy-sessions-bottom-bar">
            <div className="fy-bottom-info">
              <div>
                <span>
                  导出保留用户提问与最终答复原文，不包含工具日志、思考过程及工作区文件。
                </span>
                <SessionEvidenceDetails
                  session={selectedSession}
                  preview={migratableSession}
                  attempt={activeAttempt}
                />
              </div>
            </div>
            <div className="fy-bottom-actions">
              <Button dialogOriginRef={guideOriginRef} onClick={onOpenGuide}>
                <InfoIcon size={16} />
                <span>查看启动说明</span>
              </Button>
              <Button
                className="fy-control-button-primary"
                dialogOriginRef={attestationOriginRef}
                disabled={!activeAttempt}
                onClick={onOpenAttestation}
              >
                <CheckIcon size={16} weight="bold" />
                <span>标记：我已手动续聊</span>
              </Button>
            </div>
          </footer>
        </>
      ) : (
        <div className="fy-sessions-empty-state">
          请从左侧选择一个会话以查看内容与恢复状态
        </div>
      )}
    </main>
  );
}
