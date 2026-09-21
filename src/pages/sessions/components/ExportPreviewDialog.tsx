import { useState, useEffect, useRef } from "react";
import { WarningIcon } from "@phosphor-icons/react/dist/csr/Warning";
import { DownloadSimpleIcon } from "@phosphor-icons/react/dist/csr/DownloadSimple";
import { FolderOpenIcon } from "@phosphor-icons/react/dist/csr/FolderOpen";
import { Spinner } from "../../../shared/ui/primitives";

import { Dialog } from "../../../shared/ui/Dialog";
import { Button } from "../../../shared/ui/Button";
import type { DialogOriginRef } from "../../../shared/ui/dialogOrigin";
import {
  buildPureTextPreview,
  canExportSession,
  parseMigrationError,
  PROVIDER_LABELS,
  type MigratableSession,
  type SessionPackageExportResult,
} from "../../../shared/features/session-migration";

export interface ExportTargetItem {
  providerId: string;
  sourcePath: string;
  sessionId: string;
  title?: string;
}

export interface ExportPreviewDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  targets: ExportTargetItem[];
  onPreviewSession: (
    providerId: string,
    sourcePath: string,
  ) => Promise<MigratableSession>;
  onExport: (
    items: Array<{ providerId: string; sourcePath: string }>,
    targetPath: string,
  ) => Promise<SessionPackageExportResult>;
  onPickExportPath: (defaultName: string) => Promise<string | null>;
  originRef: DialogOriginRef | undefined;
}

export function ExportPreviewDialog({
  open,
  onOpenChange,
  targets,
  onPreviewSession,
  onExport,
  onPickExportPath,
  originRef,
}: ExportPreviewDialogProps) {
  const [loadingState, setLoadingState] = useState<
    "loading" | "error" | "ready"
  >("loading");
  const [progressText, setProgressText] = useState("正在读取与审核…");
  const [previewedSessions, setPreviewedSessions] = useState<
    MigratableSession[]
  >([]);
  const [blockedItems, setBlockedItems] = useState<
    Array<{ target: ExportTargetItem; reason: string }>
  >([]);
  const [loadErrors, setLoadErrors] = useState<
    Array<{ target: ExportTargetItem; error: string }>
  >([]);

  const [targetPath, setTargetPath] = useState("");
  const [isExporting, setIsExporting] = useState(false);
  const [exportError, setExportError] = useState<string | null>(null);

  const [validatedFn, setValidatedFn] = useState<
    typeof onPreviewSession | null
  >(null);
  const [validatedTargets, setValidatedTargets] = useState<
    ExportTargetItem[] | null
  >(null);

  const isRevalidating =
    validatedFn !== onPreviewSession || validatedTargets !== targets;
  const effectiveLoadingState = isRevalidating ? "loading" : loadingState;

  const activeTokenRef = useRef(0);

  // Load and strictly validate all frozen targets upon mount/targets change
  useEffect(() => {
    if (!open || targets.length === 0) return;

    const token = ++activeTokenRef.current;

    const runValidation = async () => {
      setProgressText(`正在读取与审核 (1/${targets.length})…`);
      setLoadingState("loading");
      setBlockedItems([]);
      setLoadErrors([]);
      setPreviewedSessions([]);

      const loaded: MigratableSession[] = [];
      const blocked: Array<{ target: ExportTargetItem; reason: string }> = [];
      const errors: Array<{ target: ExportTargetItem; error: string }> = [];

      for (let i = 0; i < targets.length; i++) {
        if (token !== activeTokenRef.current) return;
        const target = targets[i];
        setProgressText(`正在读取与审核 (${i + 1}/${targets.length})…`);

        try {
          const migratable = await onPreviewSession(
            target.providerId,
            target.sourcePath,
          );
          const check = canExportSession(migratable);
          if (!check.allowed) {
            blocked.push({
              target,
              reason: check.reason || "包含待判定或不合规轮次",
            });
          } else {
            loaded.push(migratable);
          }
        } catch (err) {
          const parsed = parseMigrationError(err);
          errors.push({
            target,
            error: parsed.message,
          });
        }
      }

      if (token !== activeTokenRef.current) return;

      setValidatedFn(() => onPreviewSession);
      setValidatedTargets(targets);

      if (blocked.length > 0 || errors.length > 0) {
        setBlockedItems(blocked);
        setLoadErrors(errors);
        setLoadingState("error");
      } else {
        setPreviewedSessions(loaded);
        setLoadingState("ready");
      }
    };

    void runValidation();
  }, [open, targets, onPreviewSession]);

  if (targets.length === 0) return null;

  const dateStr = new Date().toISOString().slice(0, 10);
  const defaultFileName =
    targets.length > 1
      ? `fyagent-sessions-batch-${dateStr}.json`
      : `fyagent-session-${targets[0].providerId}-${dateStr}.json`;

  const handlePickExportPath = async () => {
    if (isExporting) return;
    try {
      const chosen = await onPickExportPath(defaultFileName);
      if (chosen) {
        setTargetPath(chosen);
        setExportError(null);
      }
    } catch (err) {
      const parsed = parseMigrationError(err);
      setExportError(parsed.message);
    }
  };

  const handleConfirmExport = async () => {
    if (!targetPath.trim() || isExporting || effectiveLoadingState !== "ready")
      return;
    setExportError(null);
    setIsExporting(true);
    try {
      const exportPayload = targets.map((t) => ({
        providerId: t.providerId,
        sourcePath: t.sourcePath,
      }));
      await onExport(exportPayload, targetPath.trim());
      onOpenChange(false);
    } catch (err) {
      const parsed = parseMigrationError(err);
      setExportError(parsed.message);
    } finally {
      setIsExporting(false);
    }
  };

  const totalMessages = previewedSessions.reduce(
    (sum, s) => sum + s.messages.length,
    0,
  );
  const totalOmittedTools = previewedSessions.reduce(
    (sum, s) => sum + s.extraction.omitted.toolEvents,
    0,
  );
  const totalOmittedReasoning = previewedSessions.reduce(
    (sum, s) => sum + s.extraction.omitted.reasoningBlocks,
    0,
  );

  const samplePreview =
    previewedSessions.length > 0
      ? buildPureTextPreview(previewedSessions[0])
      : "";

  return (
    <Dialog
      open={open}
      originRef={originRef}
      onOpenChange={(next) => {
        if (!isExporting) onOpenChange(next);
      }}
      size="wide"
      title={
        effectiveLoadingState === "error"
          ? "导出被安全阻断"
          : targets.length > 1
            ? `批量导出迁移包 (${targets.length} 个会话)`
            : "导出迁移包 (纯文本 Q&A 问答)"
      }
      description={
        effectiveLoadingState === "error"
          ? "检测到未通过合规审查的会话，为保障还原确定性已阻止整包导出。"
          : "仅导出用户提问与最终答复原文，严格剔除工具调用与思考过程。"
      }
      actions={
        effectiveLoadingState === "error" ? (
          <Button onClick={() => onOpenChange(false)}>关闭</Button>
        ) : (
          <>
            <Button disabled={isExporting} onClick={() => onOpenChange(false)}>
              取消
            </Button>
            <Button
              className="fy-control-button-primary"
              disabled={
                effectiveLoadingState !== "ready" ||
                isExporting ||
                !targetPath.trim()
              }
              onClick={() => void handleConfirmExport()}
            >
              <DownloadSimpleIcon size={14} weight="bold" />
              <span>{isExporting ? "正在导出…" : "确认导出迁移包"}</span>
            </Button>
          </>
        )
      }
    >
      <div className="fy-export-preview-body">
        {/* 状态 1：加载全量预览 */}
        {effectiveLoadingState === "loading" && (
          <div
            style={{
              padding: "36px 0",
              display: "flex",
              flexDirection: "column",
              alignItems: "center",
              gap: "12px",
            }}
          >
            <Spinner label={progressText} />
            <span style={{ fontSize: "14px", color: "var(--fy-text-primary)" }}>
              {progressText}
            </span>
            <span
              style={{ fontSize: "12px", color: "var(--fy-text-secondary)" }}
            >
              正在对冻结的 {targets.length} 个会话逐项执行 Q&A
              问答提取与合规性审查…
            </span>
          </div>
        )}

        {/* 状态 2：存在未决或解析失败内容，阻断整包导出 */}
        {effectiveLoadingState === "error" && (
          <div className="fy-export-blocked-alert" role="alert">
            <div className="fy-alert-header">
              <WarningIcon size={24} weight="fill" />
              <div className="fy-alert-title">
                整包导出已阻断：包含未通过审查的会话
              </div>
            </div>
            <div className="fy-alert-detail">
              依据跨设备恢复安全契约，迁移包严禁包含未判定终态内容或损坏会话。请排除以下会话后重试：
            </div>

            {blockedItems.length > 0 && (
              <div
                style={{
                  display: "flex",
                  flexDirection: "column",
                  gap: "6px",
                  marginTop: "8px",
                }}
              >
                <strong>合规未通过会话：</strong>
                {blockedItems.map((b) => (
                  <div
                    key={b.target.sessionId}
                    style={{
                      fontSize: "12px",
                      padding: "6px 10px",
                      background: "rgba(0,0,0,0.2)",
                      borderRadius: "4px",
                    }}
                  >
                    • [
                    {PROVIDER_LABELS[b.target.providerId] ??
                      b.target.providerId}
                    ] {b.target.title || b.target.sessionId} — 原因: {b.reason}
                  </div>
                ))}
              </div>
            )}

            {loadErrors.length > 0 && (
              <div
                style={{
                  display: "flex",
                  flexDirection: "column",
                  gap: "6px",
                  marginTop: "8px",
                }}
              >
                <strong>读取失败会话：</strong>
                {loadErrors.map((e) => (
                  <div
                    key={e.target.sessionId}
                    style={{
                      fontSize: "12px",
                      padding: "6px 10px",
                      background: "rgba(0,0,0,0.2)",
                      borderRadius: "4px",
                    }}
                  >
                    • [
                    {PROVIDER_LABELS[e.target.providerId] ??
                      e.target.providerId}
                    ] {e.target.title || e.target.sessionId} — 错误: {e.error}
                  </div>
                ))}
              </div>
            )}
          </div>
        )}

        {/* 状态 3：全组预览与合规审查完全通过 */}
        {effectiveLoadingState === "ready" && (
          <>
            <div className="fy-export-stats-grid">
              <div className="fy-stat-card">
                <span className="fy-stat-label">导出快照数</span>
                <span className="fy-stat-value">
                  {previewedSessions.length} 个
                </span>
              </div>
              <div className="fy-stat-card">
                <span className="fy-stat-label">问答消息总数</span>
                <span className="fy-stat-value">{totalMessages} 条</span>
              </div>
              <div className="fy-stat-card">
                <span className="fy-stat-label">剔除工具事件</span>
                <span className="fy-stat-value">{totalOmittedTools} 条</span>
              </div>
              <div className="fy-stat-card">
                <span className="fy-stat-label">剔除思考过程</span>
                <span className="fy-stat-value">
                  {totalOmittedReasoning} 块
                </span>
              </div>
            </div>

            <div className="fy-export-field-group">
              <label htmlFor="export-path-input" className="fy-field-label">
                保存文件路径 (.json)：
              </label>
              <div className="fy-input-with-button">
                <input
                  id="export-path-input"
                  type="text"
                  className="fy-input-text"
                  placeholder="/path/to/exported-package.json"
                  value={targetPath}
                  disabled={isExporting}
                  onChange={(e) => setTargetPath(e.target.value)}
                />
                <Button
                  type="button"
                  disabled={isExporting}
                  onClick={() => void handlePickExportPath()}
                >
                  <FolderOpenIcon size={16} />
                  <span>选择保存位置</span>
                </Button>
              </div>
              <span className="fy-field-hint">
                点击“选择保存位置”打开原生保存文件窗口，或直接粘贴目标绝对路径。
              </span>
            </div>

            <div className="fy-export-field-group">
              <label className="fy-field-label">
                问答纯文本预览 (首个会话样例)：
              </label>
              <div className="fy-pure-text-box">{samplePreview}</div>
            </div>

            {exportError && (
              <div className="fy-field-error" role="alert">
                {exportError}
              </div>
            )}
          </>
        )}
      </div>
    </Dialog>
  );
}
