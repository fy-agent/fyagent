import {
  useState,
  useMemo,
  useCallback,
  useEffect,
  useRef,
  type MouseEvent,
} from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { DesktopIcon } from "@phosphor-icons/react/dist/csr/Desktop";
import { DownloadSimpleIcon } from "@phosphor-icons/react/dist/csr/DownloadSimple";
import { UploadSimpleIcon } from "@phosphor-icons/react/dist/csr/UploadSimple";

import { useFeatures } from "../../shared/features/provider";
import { detectRuntime } from "../../shared/platform/runtime";
import { Button } from "../../shared/ui/Button";
import { SplitPanes } from "../../shared/ui/split/SplitPanes";
import { useDialogState } from "../../shared/ui/useDialogState";
import { usePersistentVisibility } from "../../shared/ui/PersistentSurface";
import type { DialogOriginRef } from "../../shared/ui/dialogOrigin";
import type {
  LocalProviderProbe,
  MigratableMessage,
  MigratableSession,
  RestoreAttempt,
  RestoreRequest,
  RestoreStage,
  SessionMessage,
  SessionMeta,
} from "../../shared/features/session-migration";
import {
  classifyRestoreResults,
  getSessionStableKey,
  isProviderRestoreSupported,
  parseMigrationError,
  RESTORE_STAGE_LABELS,
  SUPPORTED_PROVIDER_IDS,
} from "../../shared/features/session-migration";

import {
  readFailureFeedback,
  restoreAttemptFeedback,
  type SessionFailureFeedback,
} from "./failure-feedback";
import type { ConversationTurn } from "./components/ConversationStream";
import {
  ExportPreviewDialog,
  type ExportTargetItem,
} from "./components/ExportPreviewDialog";
import { ImportPackageDialog } from "./components/ImportPackageDialog";
import { RemapWorkspaceDialog } from "./components/RemapWorkspaceDialog";
import { StartupGuideDialog } from "./components/StartupGuideDialog";
import { AttestationDialog } from "./components/AttestationDialog";
import { SessionsDetailPane } from "./components/SessionsDetailPane";
import { SessionsMasterPane } from "./components/SessionsMasterPane";

import "./page.css";

const EMPTY_MESSAGES: SessionMessage[] = [];

function getAttemptSeverityRank(stage: RestoreStage): number {
  switch (stage) {
    case "needsReconciliation":
      return 1;
    case "failed":
      return 2;
    case "ambiguous":
      return 3;
    case "nativeWritePending":
      return 4;
    case "packageVerified":
      return 5;
    case "nativeWritten":
      return 6;
    case "nativeReadbackVerified":
      return 7;
    default:
      return 99;
  }
}

export function SessionsPage() {
  const { isNative } = detectRuntime();
  const { ports, notify } = useFeatures();
  const queryClient = useQueryClient();
  const visible = usePersistentVisibility();
  const pageRef = useRef<HTMLDivElement>(null);

  const [viewMode, setViewMode] = useState<"sessions" | "attempts">("sessions");
  const [selectedSessionKey, setSelectedSessionKey] = useState<string | null>(
    null,
  );
  const [selectedAttemptId, setSelectedAttemptId] = useState<string | null>(
    null,
  );
  const [providerFilter, setProviderFilter] = useState<string>("all");
  const [searchQuery, setSearchQuery] = useState("");
  const [selectedKeys, setSelectedKeys] = useState<Set<string>>(new Set());
  const [customWorkspaces, setCustomWorkspaces] = useState<
    Record<string, string>
  >({});

  const {
    data: sessions = [],
    isLoading: loadingSessions,
    isError: sessionsError,
    error: sessionsLoadError,
    refetch: refetchSessions,
  } = useQuery<SessionMeta[]>({
    queryKey: ["sessions-list"],
    queryFn: async () => ports.sessions.listSessions(),
    enabled: isNative && visible,
  });

  const {
    data: attempts = [],
    isError: attemptsError,
    error: attemptsLoadError,
    refetch: refetchAttempts,
  } = useQuery<RestoreAttempt[]>({
    queryKey: ["sessions-attempts"],
    queryFn: async () => ports.sessions.listRestoreAttempts(),
    enabled: isNative && visible,
  });

  const { data: localProbes = {}, refetch: refetchLocalProbes } = useQuery<
    Record<string, LocalProviderProbe>
  >({
    queryKey: ["sessions-local-probes"],
    queryFn: async () => {
      const result: Record<string, LocalProviderProbe> = {};
      await Promise.all(
        SUPPORTED_PROVIDER_IDS.map(async (pId) => {
          try {
            result[pId] = await ports.sessions.probeLocalProvider(pId);
          } catch (err) {
            const parsed = parseMigrationError(err);
            result[pId] = {
              providerId: pId,
              installed: false,
              extractionSupported: false,
              writeSupported: false,
              reasonCode:
                parsed.code !== "unknown" ? parsed.code : "probe_error",
            };
          }
        }),
      );
      return result;
    },
    enabled: isNative && visible,
  });

  const selectedSession = useMemo<SessionMeta | null>(() => {
    if (sessions.length === 0 || !selectedSessionKey) return null;
    return (
      sessions.find(
        (s: SessionMeta) => getSessionStableKey(s) === selectedSessionKey,
      ) ?? null
    );
  }, [sessions, selectedSessionKey]);

  const activeStableKey = selectedSession
    ? getSessionStableKey(selectedSession)
    : null;

  const selectedAttempt = useMemo<RestoreAttempt | null>(() => {
    if (attempts.length === 0 || !selectedAttemptId) return null;
    return (
      attempts.find((a: RestoreAttempt) => a.attemptId === selectedAttemptId) ??
      null
    );
  }, [attempts, selectedAttemptId]);

  const { data: sessionDetailData, isLoading: loadingDetail } = useQuery({
    queryKey: [
      "session-detail",
      selectedSession?.providerId,
      selectedSession?.sourcePath,
      selectedSession?.sessionId,
    ],
    queryFn: async () => {
      if (!selectedSession) {
        return {
          rawMessages: EMPTY_MESSAGES,
          sourceRead: "pending" as const,
          migratable: null,
          error: null,
        };
      }
      const sourcePath = selectedSession.sourcePath || "";
      let rawMsgs: SessionMessage[];
      let sourceRead: "available" | "failed" = "available";
      let migratable: MigratableSession | null = null;
      let errPayload: SessionFailureFeedback | null = null;

      try {
        rawMsgs = await ports.sessions.getSessionMessages(
          selectedSession.providerId,
          sourcePath,
        );
      } catch (err) {
        rawMsgs = EMPTY_MESSAGES;
        sourceRead = "failed";
        errPayload = readFailureFeedback(err, "source");
      }

      try {
        migratable = await ports.sessions.previewSessionMigration(
          selectedSession.providerId,
          sourcePath,
        );
      } catch (err) {
        if (!errPayload) {
          errPayload = readFailureFeedback(err, "extraction");
        }
      }

      return {
        rawMessages: rawMsgs,
        sourceRead,
        migratable,
        error: errPayload,
      };
    },
    enabled: isNative && visible && Boolean(selectedSession),
  });

  const rawMessages = sessionDetailData?.rawMessages ?? EMPTY_MESSAGES;
  const migratableSession = sessionDetailData?.migratable ?? null;
  const structuredError = sessionDetailData?.error ?? null;

  const defaultWorkspaceFallback = useMemo(() => {
    return sessions.find((s: SessionMeta) => s.projectDir)?.projectDir ?? "";
  }, [sessions]);

  const targetWorkspace = activeStableKey
    ? (customWorkspaces[activeStableKey] ??
      selectedSession?.projectDir ??
      defaultWorkspaceFallback)
    : (customWorkspaces["default"] ?? defaultWorkspaceFallback);

  const [exportTargets, setExportTargets, exportSessionKey] =
    useDialogState<ExportTargetItem[]>();
  const [importOpen, setImportOpen] = useState(false);
  const [importProbes, setImportProbes] = useState<Record<
    string,
    LocalProviderProbe
  > | null>(null);
  const importProbeGeneration = useRef(0);
  const [remapOpen, setRemapOpen, remapSessionKey] = useDialogState<boolean>();
  const [guideOpen, setGuideOpen, guideSessionKey] = useDialogState<boolean>();
  const [attestationOpen, setAttestationOpen, attestationSessionKey] =
    useDialogState<boolean>();

  const [exportOriginRef] = useState<DialogOriginRef>({ current: null });
  const [importOriginRef] = useState<DialogOriginRef>({ current: null });
  const [remapOriginRef] = useState<DialogOriginRef>({ current: null });
  const [guideOriginRef] = useState<DialogOriginRef>({ current: null });
  const [attestationOriginRef] = useState<DialogOriginRef>({ current: null });

  const [verifyingReadback, setVerifyingReadback] = useState(false);
  const [readbackError, setReadbackError] = useState<{
    attemptId: string;
    feedback: SessionFailureFeedback;
  } | null>(null);
  const [reviewingAttempts, setReviewingAttempts] = useState(false);

  const refreshSessions = useCallback(async () => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: ["sessions-list"] }),
      queryClient.invalidateQueries({ queryKey: ["session-detail"] }),
    ]);
  }, [queryClient]);

  const refreshRestoreAttempts = useCallback(async () => {
    await queryClient.cancelQueries(
      { queryKey: ["sessions-attempts"], exact: true },
      { revert: true },
    );
    return queryClient.fetchQuery<RestoreAttempt[]>({
      queryKey: ["sessions-attempts"],
      queryFn: () => ports.sessions.listRestoreAttempts(),
      staleTime: 0,
    });
  }, [ports.sessions, queryClient]);

  const filteredSessions = useMemo(() => {
    return sessions.filter((s: SessionMeta) => {
      if (providerFilter !== "all" && s.providerId !== providerFilter) {
        return false;
      }
      if (!searchQuery.trim()) return true;
      const q = searchQuery.toLowerCase();
      const title = (s.title || "").toLowerCase();
      const summary = (s.summary || "").toLowerCase();
      const sId = s.sessionId.toLowerCase();
      const pId = s.providerId.toLowerCase();
      const dir = (s.projectDir || "").toLowerCase();
      return (
        title.includes(q) ||
        summary.includes(q) ||
        sId.includes(q) ||
        pId.includes(q) ||
        dir.includes(q)
      );
    });
  }, [sessions, providerFilter, searchQuery]);

  const activeAttempt = useMemo<RestoreAttempt | null>(() => {
    if (viewMode === "attempts") {
      return selectedAttempt;
    }
    if (!selectedSession) return null;

    const targetMatches = attempts.filter(
      (a: RestoreAttempt) =>
        a.targetProviderId === selectedSession.providerId &&
        a.targetNativeId === selectedSession.sessionId,
    );
    if (targetMatches.length > 0) {
      return targetMatches
        .slice()
        .sort(
          (a, b) =>
            getAttemptSeverityRank(a.stage) - getAttemptSeverityRank(b.stage),
        )[0];
    }

    if (migratableSession) {
      const originMatches = attempts.filter(
        (a: RestoreAttempt) =>
          a.origin.originId === migratableSession.origin.originId &&
          a.snapshotId === migratableSession.snapshotId &&
          a.targetProviderId === selectedSession.providerId,
      );
      if (originMatches.length > 0) {
        return originMatches
          .slice()
          .sort(
            (a, b) =>
              getAttemptSeverityRank(a.stage) - getAttemptSeverityRank(b.stage),
          )[0];
      }
    }

    return null;
  }, [viewMode, selectedAttempt, selectedSession, attempts, migratableSession]);

  const activeProviderId =
    viewMode === "attempts"
      ? selectedAttempt?.targetProviderId
      : selectedSession?.providerId;
  const activeProviderProbe = activeProviderId
    ? localProbes[activeProviderId]
    : undefined;

  const restoreSupport = isProviderRestoreSupported(activeProviderProbe);
  const isCapabilityVerified = restoreSupport.supported;

  const canOpenTarget =
    isCapabilityVerified &&
    (!activeAttempt ||
      [
        "nativeReadbackVerified",
        "targetOpened",
        "restartReadbackVerified",
        "nextTurnRequestVerified",
        "nextTurnReplyVerified",
      ].includes(activeAttempt.stage));

  const capabilityReason = restoreSupport.reason;

  const conversationTurns = useMemo<ConversationTurn[]>(() => {
    if (!migratableSession) return [];

    const turns: ConversationTurn[] = [];
    let currentTurn = 1;
    let pendingUserMsg: MigratableMessage | undefined;

    for (const msg of migratableSession.messages) {
      if (msg.kind === "userText") {
        if (pendingUserMsg) {
          turns.push({
            turnNumber: currentTurn,
            userMessage: pendingUserMsg,
            isIndeterminate: false,
            isIncomplete: true,
          });
          currentTurn += 1;
        }
        pendingUserMsg = msg;
      } else if (msg.kind === "assistantFinal") {
        turns.push({
          turnNumber: currentTurn,
          userMessage: pendingUserMsg,
          assistantMessage: msg,
          isIndeterminate: false,
          isIncomplete: false,
        });
        currentTurn += 1;
        pendingUserMsg = undefined;
      }
    }

    if (pendingUserMsg) {
      turns.push({
        turnNumber: currentTurn,
        userMessage: pendingUserMsg,
        isIndeterminate: false,
        isIncomplete: true,
      });
    }

    return turns;
  }, [migratableSession]);

  const hasIndeterminate = Boolean(
    structuredError?.code === "finalAnswerIndeterminate" ||
      structuredError?.code?.toLowerCase().includes("indeterminate"),
  );

  const hasIncomplete = useMemo(
    () => conversationTurns.some((t) => t.isIncomplete),
    [conversationTurns],
  );

  const handleOpenExport = useCallback(() => {
    const chosenSessions =
      selectedKeys.size > 0
        ? sessions.filter((s: SessionMeta) =>
            selectedKeys.has(getSessionStableKey(s)),
          )
        : selectedSession
          ? [selectedSession]
          : [];

    const targets: ExportTargetItem[] = chosenSessions.map((s) => ({
      providerId: s.providerId,
      sourcePath: s.sourcePath || "",
      sessionId: s.sessionId,
      title: s.title || s.sessionId,
    }));

    if (targets.length === 0) {
      notify({
        tone: "info",
        title: "请先选择会话",
        description: "未选中任何有效会话以供导出。",
      });
      return;
    }

    setExportTargets(targets);
  }, [selectedKeys, sessions, selectedSession, notify, setExportTargets]);

  const handleOpenImport = useCallback(() => {
    const generation = ++importProbeGeneration.current;
    setImportProbes(null);
    setImportOpen(true);
    // Do not share an in-flight probe that started before this opening.
    void queryClient
      .cancelQueries({ queryKey: ["sessions-local-probes"], exact: true })
      .then(() => {
        if (generation !== importProbeGeneration.current) return;
        return refetchLocalProbes();
      })
      .then((result) => {
        if (generation === importProbeGeneration.current) {
          setImportProbes(result?.isSuccess ? result.data : {});
        }
      });
  }, [queryClient, refetchLocalProbes]);

  const hasOpenDialog = Boolean(
    exportTargets || importOpen || remapOpen || guideOpen || attestationOpen,
  );
  useEffect(() => {
    if (!visible || hasOpenDialog) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      if (
        e.defaultPrevented ||
        e.isComposing ||
        e.repeat ||
        e.altKey ||
        e.shiftKey ||
        !(e.metaKey || e.ctrlKey)
      ) {
        return;
      }
      const key = e.key.toLowerCase();
      if (key !== "e" && key !== "i") return;

      const page = pageRef.current;
      if (!page || page.closest("[hidden], [inert]")) return;

      for (const context of [e.target, document.activeElement]) {
        if (
          context === null ||
          context === window ||
          context === document ||
          context === document.body
        ) {
          continue;
        }
        if (!(context instanceof Node) || !page.contains(context)) return;
        const element =
          context instanceof Element ? context : context.parentElement;
        if (
          element?.closest("input, textarea, select") ||
          element?.closest('[contenteditable]:not([contenteditable="false"])')
        ) {
          return;
        }
      }

      const dialogs = document.querySelectorAll(
        '[role="dialog"], [role="alertdialog"]',
      );
      if (
        Array.from(dialogs).some(
          (dialog) =>
            dialog.getAttribute("data-state") !== "closed" &&
            !dialog.closest('[hidden], [inert], [aria-hidden="true"]'),
        )
      ) {
        return;
      }

      if (key === "e") {
        e.preventDefault();
        handleOpenExport();
      } else {
        e.preventDefault();
        handleOpenImport();
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [visible, hasOpenDialog, handleOpenExport, handleOpenImport]);

  const toggleSelectSession = (stableKey: string, e: MouseEvent) => {
    e.stopPropagation();
    setSelectedKeys((prev) => {
      const next = new Set(prev);
      if (next.has(stableKey)) {
        next.delete(stableKey);
      } else {
        next.add(stableKey);
      }
      return next;
    });
  };

  const selectAllFiltered = () => {
    setSelectedKeys(
      new Set(filteredSessions.map((s: SessionMeta) => getSessionStableKey(s))),
    );
  };

  const clearSelection = () => {
    setSelectedKeys(new Set());
  };

  const handleExport = async (
    itemsToExport: Array<{ providerId: string; sourcePath: string }>,
    targetPath: string,
  ) => {
    if (itemsToExport.length === 0) {
      return {
        path: targetPath,
        sessionCount: 0,
        byteLen: 0,
        packageFileDigest: "",
      };
    }

    const result = await ports.sessions.exportSessionPackage(
      itemsToExport,
      targetPath,
    );
    notify({
      tone: "success",
      title: "导出迁移包成功",
      description: `文件保存至: ${result.path} (${result.sessionCount} 个会话, ${result.byteLen} 字节)`,
    });
    void refreshSessions();
    return result;
  };

  const handlePreviewSession = useCallback(
    (pId: string, sPath: string) =>
      ports.sessions.previewSessionMigration(pId, sPath),
    [ports.sessions],
  );

  const handlePickExportPath = useCallback(
    (defaultName: string) => ports.sessions.pickExportPath(defaultName),
    [ports.sessions],
  );

  const handleRestore = async (request: RestoreRequest) => {
    let res: RestoreAttempt[];
    try {
      res = await ports.sessions.restoreSessionPackage(request);
    } finally {
      await refreshRestoreAttempts().catch(() => undefined);
      void refreshSessions();
    }
    const classification = classifyRestoreResults(res);

    if (
      classification.kind === "empty" ||
      classification.kind === "needsReconciliation" ||
      classification.kind === "failed"
    ) {
      notify({
        tone: "error",
        title: classification.summaryTitle,
        description: classification.summaryDescription,
      });
    } else if (
      classification.kind === "ambiguous" ||
      classification.kind === "pending"
    ) {
      notify({
        tone: "info",
        title: classification.summaryTitle,
        description: classification.summaryDescription,
      });
    } else if (classification.isAllCleanSuccess) {
      notify({
        tone: "success",
        title: "会话恢复写入完成",
        description: `已处理 ${res.length} 个会话记录，已写入本地存储。`,
      });
    }

    return res;
  };

  const handleVerifyReadback = async () => {
    if (!activeAttempt) return;
    const attempt = activeAttempt;
    setReadbackError(null);
    setVerifyingReadback(true);
    try {
      const updated = await ports.sessions.verifyNativeReadback(
        activeAttempt.attemptId,
      );
      if (updated.stage === "nativeReadbackVerified") {
        notify({
          tone: "success",
          title: "读回核验成功",
          description: "已确认目标软件本地存储中包含该会话记录。",
        });
      } else if (updated.stage === "needsReconciliation") {
        notify({
          tone: "error",
          title: "读回状态不一致",
          description: "本地读回核对未完全匹配，需要对账，请勿重复写入。",
        });
      } else if (updated.stage === "failed") {
        notify({
          tone: "error",
          title: "读回核验失败",
          description: restoreAttemptFeedback(updated).writeSummary,
        });
      } else {
        notify({
          tone: "info",
          title: "读回核验状态更新",
          description: `当前恢复阶段: ${RESTORE_STAGE_LABELS[updated.stage as RestoreStage] ?? updated.stage}`,
        });
      }
      void refreshSessions();
    } catch (err) {
      const feedback = {
        ...restoreAttemptFeedback(attempt),
        phase: "目标读回调用失败",
        message: parseMigrationError(err).message,
        nextStep: "重新执行系统读回核验或核对恢复回执，不再次写入。",
      };
      setReadbackError({ attemptId: attempt.attemptId, feedback });
      notify({
        tone: "error",
        title: "读回核验发生错误",
        description: feedback.message,
      });
    } finally {
      await refreshRestoreAttempts().catch(() => undefined);
      setVerifyingReadback(false);
    }
  };

  const handleReviewRestore = async () => {
    setReviewingAttempts(true);
    try {
      try {
        await ports.sessions.reconcileRestoreAttempts();
      } catch (error) {
        await refreshRestoreAttempts().catch(() => undefined);
        throw error;
      }
      const receipts = await refreshRestoreAttempts();
      return receipts;
    } finally {
      setReviewingAttempts(false);
    }
  };

  const handleRecordAttestation = async (
    attemptId: string,
    claimedStage: RestoreStage,
    note?: string,
  ) => {
    try {
      await ports.sessions.recordUserAttestation(attemptId, claimedStage, note);
    } finally {
      await refreshRestoreAttempts().catch(() => undefined);
    }
    notify({
      tone: "success",
      title: "已记录手动续聊标记",
      description: "已添加您的手动确认标记。系统状态保持客观记录不变。",
    });
  };

  const handleResumeInTarget = async () => {
    if (!activeAttempt) {
      setGuideOpen(true);
      return;
    }
    try {
      const opened = await ports.sessions.openRestoredSession(
        activeAttempt.attemptId,
      );
      if (opened) {
        notify({
          tone: "success",
          title: "已请求打开目标软件",
          description: "已向目标客户端发起唤起请求。请在目标客户端中继续对话。",
        });
      } else {
        notify({
          tone: "info",
          title: "未能直接唤起客户端",
          description: "已为您打开对应软件的手动续聊指引。",
        });
        setGuideOpen(true);
      }
    } catch (err) {
      notify({
        tone: "error",
        title: "拉起目标客户端失败",
        description: parseMigrationError(err).message,
      });
      setGuideOpen(true);
    }
  };

  if (!isNative) {
    return (
      <div
        ref={pageRef}
        className="fy-sessions-page"
        data-testid="sessions-page"
      >
        <div className="fy-browser-native-blocker" role="status">
          <div className="fy-blocker-card">
            <DesktopIcon size={44} weight="duotone" />
            <h2>会话跨设备恢复需在桌面端运行</h2>
            <p>
              请在 FyAgent 桌面客户端打开会话中心，读取本机会话并创建恢复包。
            </p>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div ref={pageRef} className="fy-sessions-page" data-testid="sessions-page">
      <header className="fy-sessions-topbar">
        <div className="fy-sessions-breadcrumb">
          <span>会话中心</span>
          <span>/</span>
          <span className="fy-crumb-current">跨设备会话恢复与浏览</span>
        </div>

        <div className="fy-sessions-top-actions">
          <Button dialogOriginRef={importOriginRef} onClick={handleOpenImport}>
            <UploadSimpleIcon size={16} />
            <span>导入会话包 (⌘I)</span>
          </Button>

          {selectedKeys.size > 1 ? (
            <Button
              className="fy-control-button-primary"
              dialogOriginRef={exportOriginRef}
              onClick={handleOpenExport}
            >
              <DownloadSimpleIcon size={16} weight="bold" />
              <span>批量导出 ({selectedKeys.size} 个会话)</span>
            </Button>
          ) : (
            <Button
              className="fy-control-button-primary"
              dialogOriginRef={exportOriginRef}
              disabled={!selectedSession}
              onClick={handleOpenExport}
            >
              <DownloadSimpleIcon size={16} weight="bold" />
              <span>导出迁移包 (⌘E)</span>
            </Button>
          )}
        </div>
      </header>

      <SplitPanes
        className="fy-sessions-split-layout"
        minWidths={[280, 480]}
        defaultWidths={[340, 700]}
        flexiblePane={1}
        separatorLabels={["调整会话列表与详情宽度"]}
      >
        <SessionsMasterPane
          viewMode={viewMode}
          onChangeViewMode={setViewMode}
          sessions={sessions}
          filteredSessions={filteredSessions}
          loadingSessions={loadingSessions}
          sessionsError={sessionsError}
          sessionsLoadError={sessionsLoadError}
          onRefetchSessions={() => void refetchSessions()}
          providerFilter={providerFilter}
          onChangeProviderFilter={setProviderFilter}
          searchQuery={searchQuery}
          onChangeSearchQuery={setSearchQuery}
          activeStableKey={activeStableKey}
          onSelectSessionKey={setSelectedSessionKey}
          selectedKeys={selectedKeys}
          onToggleSelectSession={toggleSelectSession}
          onSelectAllFiltered={selectAllFiltered}
          onClearSelection={clearSelection}
          attempts={attempts}
          attemptsError={attemptsError}
          attemptsLoadError={attemptsLoadError}
          onRefetchAttempts={() => void refetchAttempts()}
          activeAttemptId={activeAttempt?.attemptId}
          onSelectAttemptId={setSelectedAttemptId}
        />

        <SessionsDetailPane
          viewMode={viewMode}
          selectedSession={selectedSession}
          activeAttempt={activeAttempt}
          activeProviderProbe={activeProviderProbe}
          canOpenTarget={canOpenTarget}
          isCapabilityVerified={isCapabilityVerified}
          capabilityReason={capabilityReason}
          targetWorkspace={targetWorkspace}
          sourceRead={sessionDetailData?.sourceRead ?? "pending"}
          loadingDetail={loadingDetail}
          migratableSession={migratableSession}
          structuredError={structuredError}
          hasIndeterminate={hasIndeterminate}
          hasIncomplete={hasIncomplete}
          conversationTurns={conversationTurns}
          rawMessages={rawMessages}
          readbackError={readbackError}
          reviewingAttempts={reviewingAttempts}
          verifyingReadback={verifyingReadback}
          remapOriginRef={remapOriginRef}
          guideOriginRef={guideOriginRef}
          attestationOriginRef={attestationOriginRef}
          onOpenRemap={() => setRemapOpen(true)}
          onOpenGuide={() => setGuideOpen(true)}
          onOpenAttestation={() => setAttestationOpen(true)}
          onResumeInTarget={() => void handleResumeInTarget()}
          onRetrySource={() => {
            if (!selectedSession) return;
            void queryClient.invalidateQueries({
              queryKey: [
                "session-detail",
                selectedSession.providerId,
                selectedSession.sourcePath,
                selectedSession.sessionId,
              ],
            });
          }}
          onReviewRestoreWithFeedback={(attempt) =>
            void handleReviewRestore().catch((error) => {
              if (attempt) {
                setReadbackError({
                  attemptId: attempt.attemptId,
                  feedback: {
                    ...restoreAttemptFeedback(attempt),
                    phase: "恢复回执核对失败",
                    message: parseMigrationError(error).message,
                  },
                });
              }
            })
          }
          onVerifyReadback={() => void handleVerifyReadback()}
        />
      </SplitPanes>

      {exportTargets && (
        <ExportPreviewDialog
          key={exportSessionKey}
          open={Boolean(exportTargets)}
          onOpenChange={(open) => {
            if (!open) setExportTargets(null);
          }}
          targets={exportTargets}
          onPreviewSession={handlePreviewSession}
          onExport={handleExport}
          onPickExportPath={handlePickExportPath}
          originRef={exportOriginRef}
        />
      )}

      <ImportPackageDialog
        open={importOpen}
        onOpenChange={setImportOpen}
        onReadPackage={(path) => ports.sessions.readSessionPackage(path)}
        onRestore={handleRestore}
        onReviewRestore={handleReviewRestore}
        onVerifyReadback={async (attemptId) => {
          try {
            return await ports.sessions.verifyNativeReadback(attemptId);
          } finally {
            await refreshRestoreAttempts().catch(() => undefined);
            void refreshSessions();
          }
        }}
        onPickPackageFile={() => ports.sessions.pickPackageFile()}
        onPickDirectory={() => ports.sessions.pickDirectory()}
        localProbes={importProbes}
        initialTargetWorkspace={targetWorkspace}
        originRef={importOriginRef}
        onImportSuccess={() => void refreshSessions()}
      />

      {remapOpen && (
        <RemapWorkspaceDialog
          key={remapSessionKey}
          open={Boolean(remapOpen)}
          onOpenChange={(open) => {
            if (!open) setRemapOpen(null);
          }}
          currentPath={targetWorkspace}
          onSave={async (newPath) => {
            if (activeStableKey) {
              setCustomWorkspaces((prev) => ({
                ...prev,
                [activeStableKey]: newPath,
              }));
            }
            notify({
              tone: "success",
              title: "已更新工作区绑定",
              description: `目标本地工作区已更新为: ${newPath}`,
            });
          }}
          onPickDirectory={() => ports.sessions.pickDirectory()}
          originRef={remapOriginRef}
        />
      )}

      {guideOpen && (
        <StartupGuideDialog
          key={guideSessionKey}
          open={Boolean(guideOpen)}
          onOpenChange={(open) => {
            if (!open) setGuideOpen(null);
          }}
          providerId={
            activeAttempt
              ? activeAttempt.targetProviderId
              : (selectedSession?.providerId ?? "codex")
          }
          nativeSessionId={
            activeAttempt?.targetNativeId || selectedSession?.sessionId
          }
          originRef={guideOriginRef}
        />
      )}

      {attestationOpen && (
        <AttestationDialog
          key={attestationSessionKey}
          open={Boolean(attestationOpen)}
          onOpenChange={(open) => {
            if (!open) setAttestationOpen(null);
          }}
          attempt={activeAttempt}
          onRecord={handleRecordAttestation}
          originRef={attestationOriginRef}
        />
      )}
    </div>
  );
}
