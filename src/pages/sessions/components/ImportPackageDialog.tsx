import { useState, useRef, useId } from "react";
import { FolderOpenIcon } from "@phosphor-icons/react/dist/csr/FolderOpen";
import { CheckCircleIcon } from "@phosphor-icons/react/dist/csr/CheckCircle";
import { WarningIcon } from "@phosphor-icons/react/dist/csr/Warning";
import { XCircleIcon } from "@phosphor-icons/react/dist/csr/XCircle";

import { Dialog } from "../../../shared/ui/Dialog";
import { Button } from "../../../shared/ui/Button";
import type { DialogOriginRef } from "../../../shared/ui/dialogOrigin";
import type {
  LocalProviderProbe,
  ReadSessionPackageResult,
  RestoreAttempt,
  RestoreRequest,
  RestoreRequestKind,
} from "../../../shared/features/session-migration";
import {
  classifyRestoreResults,
  computeRestoreBindingKey,
  isProviderRestoreSupported,
  parseMigrationError,
  PROVIDER_LABELS,
  RESTORE_STAGE_LABELS,
} from "../../../shared/features/session-migration";

export interface ImportPackageDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onReadPackage: (path: string) => Promise<ReadSessionPackageResult>;
  onRestore: (request: RestoreRequest) => Promise<RestoreAttempt[]>;
  onPickPackageFile: () => Promise<string | null>;
  onPickDirectory: () => Promise<string | null>;
  localProbes: Record<string, LocalProviderProbe>;
  initialTargetWorkspace?: string;
  originRef: DialogOriginRef | undefined;
  onImportSuccess?: (attempts: RestoreAttempt[]) => void;
}

export function ImportPackageDialog({
  open,
  onOpenChange,
  onReadPackage,
  onRestore,
  onPickPackageFile,
  onPickDirectory,
  localProbes,
  initialTargetWorkspace,
  originRef,
  onImportSuccess,
}: ImportPackageDialogProps) {
  const dialogId = useId();
  const [packagePath, setPackagePath] = useState("");
  const [isReading, setIsReading] = useState(false);
  const [readResult, setReadResult] = useState<ReadSessionPackageResult | null>(
    null,
  );
  const [readError, setReadError] = useState<string | null>(null);

  // Target provider and snapshots selection
  const [selectedTargetProviderId, setSelectedTargetProviderId] =
    useState<string>("");
  const [selectedSnapshotIds, setSelectedSnapshotIds] = useState<Set<string>>(
    new Set(),
  );

  // Target workspace & duplicate conflict policy
  const [targetWorkspace, setTargetWorkspace] = useState(
    initialTargetWorkspace || "",
  );
  const [conflictOption, setConflictOption] =
    useState<RestoreRequestKind>("defaultImport");

  // Restore execution state & idempotency tracking
  const [isRestoring, setIsRestoring] = useState(false);
  const [restoreError, setRestoreError] = useState<string | null>(null);
  const [restoreResult, setRestoreResult] = useState<RestoreAttempt[] | null>(
    null,
  );

  // Fixed requestId per confirmed binding parameters (for retry idempotency)
  const bindingKeyRef = useRef<string>("");
  const requestIdRef = useRef<string>("");
  const activeExecutionTokenRef = useRef<number>(0);

  // When target provider selection changes, update snapshot selection
  const handleTargetProviderChange = (newProvider: string) => {
    if (isRestoring || !readResult) return;
    setSelectedTargetProviderId(newProvider);
    const matchingSnapshots = readResult.package.sessions
      .filter((s) => s.origin.providerId === newProvider)
      .map((s) => s.snapshotId);
    setSelectedSnapshotIds(new Set(matchingSnapshots));
  };

  const handleToggleSnapshot = (snapshotId: string) => {
    if (isRestoring) return;
    setSelectedSnapshotIds((prev) => {
      const next = new Set(prev);
      if (next.has(snapshotId)) {
        next.delete(snapshotId);
      } else {
        next.add(snapshotId);
      }
      return next;
    });
  };

  const handlePickPackageFile = async () => {
    if (isRestoring) return;
    try {
      const file = await onPickPackageFile();
      if (file) {
        setPackagePath(file);
        setReadError(null);
      }
    } catch (err) {
      const parsed = parseMigrationError(err);
      setReadError(parsed.message);
    }
  };

  const handleReadPackage = async () => {
    if (isRestoring) return;
    if (!packagePath.trim()) {
      setReadError("请选择或输入迁移包文件路径");
      return;
    }
    setReadError(null);
    setIsReading(true);
    try {
      const res = await onReadPackage(packagePath.trim());
      setReadResult(res);

      const distinct = Array.from(
        new Set(res.package.sessions.map((s) => s.origin.providerId)),
      );
      const initialProvider = distinct[0] || "codex";
      setSelectedTargetProviderId(initialProvider);

      const matchingSnapshots = res.package.sessions
        .filter((s) => s.origin.providerId === initialProvider)
        .map((s) => s.snapshotId);
      setSelectedSnapshotIds(new Set(matchingSnapshots));
    } catch (err) {
      const parsed = parseMigrationError(err);
      setReadError(parsed.message);
      setReadResult(null);
      setSelectedTargetProviderId("");
      setSelectedSnapshotIds(new Set());
    } finally {
      setIsReading(false);
    }
  };

  const handlePickFolder = async () => {
    if (isRestoring) return;
    try {
      const dir = await onPickDirectory();
      if (dir) {
        setTargetWorkspace(dir);
      }
    } catch (err) {
      const parsed = parseMigrationError(err);
      setRestoreError(parsed.message);
    }
  };

  // Check target provider capability via local probe
  const targetProbe = localProbes[selectedTargetProviderId];
  const probeSupport = isProviderRestoreSupported(targetProbe);

  // Available source providers in this package
  const availableSourceProviders = readResult
    ? Array.from(
        new Set(readResult.package.sessions.map((s) => s.origin.providerId)),
      )
    : [];

  // Filter sessions matching selected provider
  const candidateSessions = readResult
    ? readResult.package.sessions.filter(
        (s) => s.origin.providerId === selectedTargetProviderId,
      )
    : [];

  const handleExecuteRestore = async () => {
    if (!readResult || isRestoring) return;
    if (!probeSupport.supported) {
      setRestoreError(probeSupport.reason || "目标软件不支持恢复写入");
      return;
    }
    if (selectedSnapshotIds.size === 0) {
      setRestoreError("请至少选择一个要恢复的会话快照");
      return;
    }
    if (!targetWorkspace.trim()) {
      setRestoreError("请选择当前机器上的工作区绝对目录");
      return;
    }

    setRestoreError(null);
    setIsRestoring(true);

    const executionToken = ++activeExecutionTokenRef.current;

    // Idempotency: compute deterministic binding key
    const bindingParams = {
      packagePath: packagePath.trim(),
      targetProviderId: selectedTargetProviderId,
      snapshotIds: Array.from(selectedSnapshotIds),
      targetWorkspace: targetWorkspace.trim(),
      requestKind: conflictOption,
    };
    const currentBindingKey = computeRestoreBindingKey(bindingParams);

    // Reuse existing requestId if binding parameters haven't changed
    let requestId = requestIdRef.current;
    if (currentBindingKey !== bindingKeyRef.current || !requestId) {
      requestId = crypto.randomUUID();
      requestIdRef.current = requestId;
      bindingKeyRef.current = currentBindingKey;
    }

    try {
      const request: RestoreRequest = {
        packagePath: packagePath.trim(),
        requestId,
        snapshotIds: Array.from(selectedSnapshotIds),
        targetProviderId: selectedTargetProviderId,
        targetWorkspace: targetWorkspace.trim(),
        requestKind: conflictOption,
      };

      const attempts = await onRestore(request);

      // Protect against stale async updates if dialog was reset or re-executed
      if (executionToken !== activeExecutionTokenRef.current) return;

      setRestoreResult(attempts);
      onImportSuccess?.(attempts);
    } catch (err) {
      if (executionToken !== activeExecutionTokenRef.current) return;
      const parsed = parseMigrationError(err);
      setRestoreError(parsed.message);
    } finally {
      if (executionToken === activeExecutionTokenRef.current) {
        setIsRestoring(false);
      }
    }
  };

  const handleReset = () => {
    activeExecutionTokenRef.current++;
    setReadResult(null);
    setRestoreResult(null);
    setReadError(null);
    setRestoreError(null);
    setPackagePath("");
    setSelectedTargetProviderId("");
    setSelectedSnapshotIds(new Set());
    bindingKeyRef.current = "";
    requestIdRef.current = "";
    setIsRestoring(false);
  };

  // Safe dialog close: forbidden during active restore request
  const handleSafeOpenChange = (next: boolean) => {
    if (isRestoring) return;
    if (!next) handleReset();
    onOpenChange(next);
  };

  // Determine result banner presentation using shared classifier
  const classification = classifyRestoreResults(restoreResult);

  return (
    <Dialog
      open={open}
      originRef={originRef}
      onOpenChange={handleSafeOpenChange}
      size="wide"
      title="导入跨设备会话包"
      description="解析会话迁移文件，选择目标客户端并安全恢复至本地存储。"
      actions={
        restoreResult ? (
          <>
            <Button
              onClick={() => {
                setRestoreResult(null);
                setRestoreError(null);
              }}
            >
              重新配置
            </Button>
            <Button
              className="fy-control-button-primary"
              onClick={() => {
                handleReset();
                onOpenChange(false);
              }}
            >
              完成
            </Button>
          </>
        ) : !readResult ? (
          <>
            <Button disabled={isReading} onClick={() => onOpenChange(false)}>
              取消
            </Button>
            <Button
              className="fy-control-button-primary"
              disabled={isReading || !packagePath.trim()}
              onClick={() => void handleReadPackage()}
            >
              {isReading ? "正在解析…" : "解析会话包"}
            </Button>
          </>
        ) : (
          <>
            <Button
              disabled={isRestoring}
              onClick={() => {
                setReadResult(null);
                setRestoreError(null);
              }}
            >
              上一步
            </Button>
            <Button
              className="fy-control-button-primary"
              disabled={
                isRestoring ||
                !probeSupport.supported ||
                selectedSnapshotIds.size === 0 ||
                !targetWorkspace.trim()
              }
              onClick={() => void handleExecuteRestore()}
            >
              {isRestoring ? "正在写入…" : "确认恢复至目标软件"}
            </Button>
          </>
        )
      }
    >
      <div className="fy-import-dialog-body" id={`import-dialog-${dialogId}`}>
        {/* 第一阶段：输入与解析会话包 */}
        {!readResult && !restoreResult && (
          <div className="fy-import-step-read">
            <div className="fy-import-field-group">
              <label htmlFor="package-path-input" className="fy-field-label">
                迁移包文件 (.json)：
              </label>
              <div className="fy-input-with-button">
                <input
                  id="package-path-input"
                  type="text"
                  className="fy-input-text"
                  placeholder="/path/to/session-package.json"
                  value={packagePath}
                  disabled={isReading}
                  onChange={(e) => setPackagePath(e.target.value)}
                />
                <Button
                  type="button"
                  disabled={isReading}
                  onClick={() => void handlePickPackageFile()}
                >
                  <FolderOpenIcon size={16} />
                  <span>选择文件</span>
                </Button>
              </div>
              <span className="fy-field-hint">
                请指定由 FyAgent 导出的标准 JSON 会话迁移包文件绝对路径。
              </span>
            </div>

            {readError && (
              <div className="fy-field-error" role="alert">
                {readError}
              </div>
            )}
          </div>
        )}

        {/* 第二阶段：展示包详情、选择目标软件、勾选快照、目录绑定与冲突处置 */}
        {readResult && !restoreResult && (
          <div className="fy-import-step-configure">
            {/* 包摘要信息卡片 */}
            <div className="fy-package-summary-card">
              <div className="fy-summary-header">
                <strong>会话包解析成功</strong>
                <span className="fy-summary-tag">
                  共 {readResult.package.sessions.length} 个会话快照
                </span>
              </div>
              <div className="fy-summary-details">
                <div>
                  导出方：{readResult.package.exporter.app} (
                  {readResult.package.exporter.platform})
                </div>
                <div>
                  包含来源：
                  {availableSourceProviders
                    .map((p) => PROVIDER_LABELS[p] ?? p)
                    .join("、")}
                </div>
                <div>
                  导出时间：
                  {new Date(readResult.package.exportedAt).toLocaleString()}
                </div>
              </div>
            </div>

            {/* 目标软件选择（严格与包内来源一一对应） */}
            <div className="fy-import-field-group">
              <label
                htmlFor="target-provider-select"
                className="fy-field-label"
              >
                恢复至目标软件（与包内来源一一对应）：
              </label>
              <select
                id="target-provider-select"
                className="fy-input-text"
                value={selectedTargetProviderId}
                disabled={isRestoring}
                onChange={(e) => handleTargetProviderChange(e.target.value)}
              >
                {availableSourceProviders.map((pId) => (
                  <option key={pId} value={pId}>
                    {PROVIDER_LABELS[pId] ?? pId}
                  </option>
                ))}
              </select>
              <span className="fy-field-hint">
                为防止跨软件数据错配，恢复写入仅支持还原至与会话来源相同的客户端。
              </span>
            </div>

            {/* 本地探针验证门控横幅 */}
            {!probeSupport.supported ? (
              <div className="fy-duplicate-conflict-box" role="alert">
                <div className="fy-conflict-header">
                  <WarningIcon size={18} weight="bold" />
                  <strong>目标软件不可用或未经验证</strong>
                </div>
                <p className="fy-conflict-desc">
                  {probeSupport.reason ||
                    "当前客户端未安装或版本不受支持，禁止执行写入恢复。"}
                </p>
              </div>
            ) : (
              <div className="fy-no-overwrite-guarantee">
                <CheckCircleIcon
                  size={14}
                  weight="bold"
                  color="var(--fy-success-text)"
                />
                <span>
                  客户端探测就绪：
                  {PROVIDER_LABELS[selectedTargetProviderId] ??
                    selectedTargetProviderId}{" "}
                  已安装且支持写入恢复。
                </span>
              </div>
            )}

            {/* 会话选择与摘要预览 */}
            <div className="fy-import-field-group">
              <label className="fy-field-label">
                选择要恢复的会话 (已选 {selectedSnapshotIds.size} /{" "}
                {candidateSessions.length})：
              </label>
              <div className="fy-pure-text-box" style={{ maxHeight: "150px" }}>
                {candidateSessions.map((session) => {
                  const isChecked = selectedSnapshotIds.has(session.snapshotId);
                  return (
                    <label
                      key={session.snapshotId}
                      style={{
                        display: "flex",
                        alignItems: "center",
                        gap: "8px",
                        padding: "4px 0",
                        cursor: isRestoring ? "not-allowed" : "pointer",
                      }}
                    >
                      <input
                        type="checkbox"
                        checked={isChecked}
                        disabled={isRestoring}
                        onChange={() =>
                          handleToggleSnapshot(session.snapshotId)
                        }
                      />
                      <span>
                        <strong>{session.title || "未命名会话"}</strong>{" "}
                        <span style={{ opacity: 0.7, fontSize: "11px" }}>
                          ({session.messages.length} 条消息
                          {session.workspaceLabel
                            ? ` · 标签: ${session.workspaceLabel}`
                            : ""}
                          )
                        </span>
                      </span>
                    </label>
                  );
                })}
              </div>
            </div>

            {/* 目标工作区目录选择 */}
            <div className="fy-import-field-group">
              <label
                htmlFor="target-workspace-input"
                className="fy-field-label"
              >
                绑定目标机器工作区目录：
              </label>
              <div className="fy-input-with-button">
                <input
                  id="target-workspace-input"
                  type="text"
                  className="fy-input-text"
                  placeholder="/Users/username/work/project"
                  value={targetWorkspace}
                  disabled={isRestoring}
                  onChange={(e) => setTargetWorkspace(e.target.value)}
                />
                <Button
                  type="button"
                  disabled={isRestoring}
                  onClick={() => void handlePickFolder()}
                >
                  <FolderOpenIcon size={16} />
                  <span>选择目录</span>
                </Button>
              </div>
              <span className="fy-field-hint">
                跨设备迁移时路径往往不同。请选择当前电脑上对应的实际工程文件夹。
              </span>
            </div>

            {/* 重复冲突处置（无覆盖选项） */}
            <div className="fy-conflict-section">
              <div className="fy-conflict-title">
                <span>重复冲突处置策略：</span>
              </div>

              <div
                className="fy-conflict-radio-group"
                role="radiogroup"
                aria-label="冲突策略"
              >
                <label
                  className={`fy-conflict-option ${conflictOption === "defaultImport" ? "selected" : ""}`}
                >
                  <input
                    type="radio"
                    name="conflict-policy"
                    value="defaultImport"
                    disabled={isRestoring}
                    checked={conflictOption === "defaultImport"}
                    onChange={() => setConflictOption("defaultImport")}
                  />
                  <div className="fy-option-content">
                    <strong>
                      默认策略：沿用已有恢复记录，不重复写入 (推荐)
                    </strong>
                    <p>
                      如果目标本地已恢复过该会话，不进行重复写入，保护已有记录。
                    </p>
                  </div>
                </label>

                <label
                  className={`fy-conflict-option ${conflictOption === "saveAsNewCopy" ? "selected" : ""}`}
                >
                  <input
                    type="radio"
                    name="conflict-policy"
                    value="saveAsNewCopy"
                    disabled={isRestoring}
                    checked={conflictOption === "saveAsNewCopy"}
                    onChange={() => setConflictOption("saveAsNewCopy")}
                  />
                  <div className="fy-option-content">
                    <strong>另存为新副本：作为新会话独立保存</strong>
                    <p>
                      分配全新的本地独立会话 ID 并完整写入，不修改已有会话内容。
                    </p>
                  </div>
                </label>
              </div>

              <div className="fy-no-overwrite-guarantee">
                <CheckCircleIcon
                  size={14}
                  weight="bold"
                  color="var(--fy-success-text)"
                />
                <span>
                  安全保证：系统绝不提供“覆盖（Overwrite）”选项，杜绝误操作导致本地历史会话被篡改。
                </span>
              </div>
            </div>

            {restoreError && (
              <div className="fy-field-error" role="alert">
                {restoreError}
              </div>
            )}
          </div>
        )}

        {/* 第三阶段：恢复结果真实阶段反馈 */}
        {restoreResult && (
          <div className="fy-import-step-result">
            <div className={`fy-result-banner ${classification.bannerTone}`}>
              {classification.bannerTone === "success" ? (
                <CheckCircleIcon size={24} weight="fill" />
              ) : classification.bannerTone === "error" ? (
                <XCircleIcon size={24} weight="fill" />
              ) : (
                <WarningIcon size={24} weight="fill" />
              )}
              <div>
                <strong>{classification.summaryTitle}</strong>
                <p>{classification.summaryDescription}</p>
              </div>
            </div>

            <div className="fy-result-attempts-list">
              {restoreResult.map((attempt) => {
                const stageLabel =
                  RESTORE_STAGE_LABELS[attempt.stage] ?? attempt.stage;
                const attemptError = attempt.lastError
                  ? parseMigrationError(attempt.lastError).message
                  : null;

                return (
                  <div
                    className="fy-result-attempt-card"
                    key={attempt.attemptId}
                  >
                    <div className="fy-attempt-header">
                      <span className="fy-session-item-provider-tag">
                        {PROVIDER_LABELS[attempt.targetProviderId] ??
                          attempt.targetProviderId}
                      </span>
                      <span className="fy-attempt-stage-pill">
                        {stageLabel}
                      </span>
                    </div>
                    <div className="fy-attempt-meta">
                      <div>本地 ID: {attempt.targetNativeId || "待分配"}</div>
                      <div>工作区: {attempt.targetWorkspace || "默认"}</div>
                      {attemptError && (
                        <div style={{ color: "var(--fy-danger-text)" }}>
                          失败原因: {attemptError}
                        </div>
                      )}
                    </div>
                  </div>
                );
              })}
            </div>
          </div>
        )}
      </div>
    </Dialog>
  );
}
