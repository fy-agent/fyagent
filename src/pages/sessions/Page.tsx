import { useState, useMemo, useCallback, useEffect } from "react";
import { Link } from "react-router-dom";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { ArrowLeftIcon } from "@phosphor-icons/react/dist/csr/ArrowLeft";
import { PlayIcon } from "@phosphor-icons/react/dist/csr/Play";
import { DownloadSimpleIcon } from "@phosphor-icons/react/dist/csr/DownloadSimple";
import { UploadSimpleIcon } from "@phosphor-icons/react/dist/csr/UploadSimple";
import { FolderSimpleIcon } from "@phosphor-icons/react/dist/csr/FolderSimple";
import { InfoIcon } from "@phosphor-icons/react/dist/csr/Info";
import { CheckIcon } from "@phosphor-icons/react/dist/csr/Check";
import { CheckSquareIcon } from "@phosphor-icons/react/dist/csr/CheckSquare";
import { SquareIcon } from "@phosphor-icons/react/dist/csr/Square";
import { BookOpenIcon } from "@phosphor-icons/react/dist/csr/BookOpen";
import { DesktopIcon } from "@phosphor-icons/react/dist/csr/Desktop";
import { ClockCounterClockwiseIcon } from "@phosphor-icons/react/dist/csr/ClockCounterClockwise";
import { ChatsCircleIcon } from "@phosphor-icons/react/dist/csr/ChatsCircle";

import { useFeatures } from "../../shared/features/provider";
import { detectRuntime } from "../../shared/platform/runtime";
import { Button } from "../../shared/ui/Button";
import { FeatureSearch } from "../../shared/ui/FeatureSearch";
import { SplitPanes } from "../../shared/ui/split/SplitPanes";
import { useDialogState } from "../../shared/ui/useDialogState";
import type { DialogOriginRef } from "../../shared/ui/dialogOrigin";
import type {
  MigratableMessage,
  MigratableSession,
  RestoreAttempt,
  RestoreRequest,
  RestoreStage,
  SessionMessage,
  SessionMeta,
  LocalProviderProbe,
} from "../../shared/features/session-migration";
import {
  classifyRestoreResults,
  PROVIDER_LABELS,
  RESTORE_STAGE_LABELS,
  SUPPORTED_PROVIDER_IDS,
  getSessionStableKey,
  parseMigrationError,
} from "../../shared/features/session-migration";

import { StatusBanners } from "./components/StatusBanners";
import {
  ConversationStream,
  type ConversationTurn,
} from "./components/ConversationStream";
import {
  ExportPreviewDialog,
  type ExportTargetItem,
} from "./components/ExportPreviewDialog";
import { ImportPackageDialog } from "./components/ImportPackageDialog";
import { RemapWorkspaceDialog } from "./components/RemapWorkspaceDialog";
import { StartupGuideDialog } from "./components/StartupGuideDialog";
import { AttestationDialog } from "./components/AttestationDialog";

import "./page.css";

const EMPTY_MESSAGES: SessionMessage[] = [];

export function SessionsPage() {
  const { isNative } = detectRuntime();
  const { ports, notify } = useFeatures();
  const queryClient = useQueryClient();

  // Navigation & view mode
  const [viewMode, setViewMode] = useState<"sessions" | "attempts">("sessions");
  const [selectedSessionKey, setSelectedSessionKey] = useState<string | null>(
    null,
  );
  const [selectedAttemptId, setSelectedAttemptId] = useState<string | null>(
    null,
  );
  const [providerFilter, setProviderFilter] = useState<string>("all");
  const [searchQuery, setSearchQuery] = useState("");

  // Multi-select for batch export
  const [selectedKeys, setSelectedKeys] = useState<Set<string>>(new Set());

  // Per-session custom workspace mappings
  const [customWorkspaces, setCustomWorkspaces] = useState<
    Record<string, string>
  >({});

  // ─── Queries ──────────────────────────────────────────────────────
  const {
    data: sessions = [],
    isLoading: loadingSessions,
    isError: sessionsError,
    error: sessionsLoadError,
    refetch: refetchSessions,
  } = useQuery<SessionMeta[]>({
    queryKey: ["sessions-list"],
    queryFn: async () => ports.sessions.listSessions(),
    enabled: isNative,
  });

  const {
    data: attempts = [],
    isError: attemptsError,
    error: attemptsLoadError,
    refetch: refetchAttempts,
  } = useQuery<RestoreAttempt[]>({
    queryKey: ["sessions-attempts"],
    queryFn: async () => ports.sessions.listRestoreAttempts(),
    enabled: isNative,
  });

  // Probe all 7 providers to get real runtime capability
  const { data: localProbes = {} } = useQuery<
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
                parsed.message && parsed.message !== "未知错误"
                  ? parsed.message
                  : parsed.code !== "unknown"
                    ? parsed.code
                    : "probe_error",
            };
          }
        }),
      );
      return result;
    },
    enabled: isNative,
  });

  // ─── Selected Session Resolution ──────────────────────────────────
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

  // Selected attempt resolution (when inspecting attempts list)
  const selectedAttempt = useMemo<RestoreAttempt | null>(() => {
    if (attempts.length === 0 || !selectedAttemptId) return null;
    return (
      attempts.find((a: RestoreAttempt) => a.attemptId === selectedAttemptId) ??
      null
    );
  }, [attempts, selectedAttemptId]);

  // ─── Detail Query (with Structured Error preservation) ───────────
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
          migratable: null,
          error: null,
        };
      }
      const sourcePath = selectedSession.sourcePath || "";
      let rawMsgs: SessionMessage[];
      let migratable: MigratableSession | null = null;
      let errPayload: {
        code: string;
        message: string;
        detail?: string;
      } | null = null;

      try {
        rawMsgs = await ports.sessions.getSessionMessages(
          selectedSession.providerId,
          sourcePath,
        );
      } catch (err) {
        rawMsgs = EMPTY_MESSAGES;
        errPayload = parseMigrationError(err);
      }

      try {
        migratable = await ports.sessions.previewSessionMigration(
          selectedSession.providerId,
          sourcePath,
        );
      } catch (err) {
        if (!errPayload) {
          errPayload = parseMigrationError(err);
        }
      }

      return { rawMessages: rawMsgs, migratable, error: errPayload };
    },
    enabled: isNative && Boolean(selectedSession),
  });

  const rawMessages = sessionDetailData?.rawMessages ?? EMPTY_MESSAGES;
  const migratableSession = sessionDetailData?.migratable ?? null;
  const structuredError = sessionDetailData?.error ?? null;

  // Fallback workspace from existing sessions if no specific session is focused
  const defaultWorkspaceFallback = useMemo(() => {
    return sessions.find((s: SessionMeta) => s.projectDir)?.projectDir ?? "";
  }, [sessions]);

  // Target workspace is scoped per session, with selectedSession.projectDir or fallback
  const targetWorkspace = activeStableKey
    ? (customWorkspaces[activeStableKey] ??
      selectedSession?.projectDir ??
      defaultWorkspaceFallback)
    : (customWorkspaces["default"] ?? defaultWorkspaceFallback);

  // ─── Modals & Dialogs (useDialogState for lifecycle isolation) ────
  const [exportTargets, setExportTargets, exportSessionKey] =
    useDialogState<ExportTargetItem[]>();
  const [importOpen, setImportOpen, importSessionKey] =
    useDialogState<boolean>();
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

  const refreshSessions = useCallback(async () => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: ["sessions-list"] }),
      queryClient.invalidateQueries({ queryKey: ["sessions-attempts"] }),
      queryClient.invalidateQueries({ queryKey: ["session-detail"] }),
    ]);
  }, [queryClient]);

  // ─── Filtered Sessions ───────────────────────────────────────────
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

  // ─── Active Attempt Matching (Strict Semantics) ───────────────────
  // Prioritizes unresolved receipt stages over successful ones so issues are never masked
  const getAttemptSeverityRank = (stage: RestoreStage): number => {
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
  };

  // Origin side: exact originId + snapshotId + targetProviderId
  // Target side: targetProviderId + targetNativeId matching selectedSession
  const activeAttempt = useMemo<RestoreAttempt | null>(() => {
    if (viewMode === "attempts") {
      return selectedAttempt;
    }
    if (!selectedSession) return null;

    // First check target side: imported session matching targetNativeId
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

    // Next check origin side: exported from this origin session
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

  // ─── Capability Verification from Local Probe ────────────────────
  const activeProviderId =
    viewMode === "attempts"
      ? selectedAttempt?.targetProviderId
      : selectedSession?.providerId;
  const activeProviderProbe = activeProviderId
    ? localProbes[activeProviderId]
    : undefined;

  const isCapabilityVerified = Boolean(
    activeProviderProbe?.installed && activeProviderProbe?.writeSupported,
  );

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

  const capabilityReason = activeProviderProbe
    ? activeProviderProbe.reasonCode &&
      activeProviderProbe.reasonCode !== "providerNotInstalled"
      ? parseMigrationError({
          code: activeProviderProbe.reasonCode,
          message: activeProviderProbe.reasonCode,
        }).message
      : !activeProviderProbe.installed
        ? "本地未安装该 AI 软件"
        : !activeProviderProbe.writeSupported
          ? "软件当前版本暂未通过恢复验证"
          : undefined
    : "软件运行环境待探测";

  // ─── Turns Projection (No Dropped Messages & Real Roles) ─────────
  const conversationTurns = useMemo<ConversationTurn[]>(() => {
    if (!migratableSession) return [];

    const turns: ConversationTurn[] = [];
    let currentTurn = 1;
    let pendingUserMsg: MigratableMessage | undefined;

    for (const msg of migratableSession.messages) {
      if (msg.kind === "userText") {
        if (pendingUserMsg) {
          // Genuinely unanswered user message followed by another prompt: incomplete turn
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

    // Trailing unclosed user message
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

  // ─── Export Dialog Controller ────────────────────────────────────
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
    void queryClient.invalidateQueries({ queryKey: ["sessions-local-probes"] });
    setImportOpen(true);
  }, [queryClient, setImportOpen]);

  // ─── Keyboard Shortcuts ──────────────────────────────────────────
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "e") {
        e.preventDefault();
        handleOpenExport();
      } else if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "i") {
        e.preventDefault();
        handleOpenImport();
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [handleOpenExport, handleOpenImport]);

  // ─── Multi-Selection Helpers ─────────────────────────────────────
  const toggleSelectSession = (stableKey: string, e: React.MouseEvent) => {
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

  // ─── Action Handlers ─────────────────────────────────────────────
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
    const res = await ports.sessions.restoreSessionPackage(request);
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

    void refreshSessions();
    return res;
  };

  const handleVerifyReadback = async () => {
    if (!activeAttempt) return;
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
          description: "本地存储未检测到该会话。",
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
      notify({
        tone: "error",
        title: "读回核验发生错误",
        description: err instanceof Error ? err.message : String(err),
      });
    } finally {
      setVerifyingReadback(false);
    }
  };

  const handleRecordAttestation = async (
    attemptId: string,
    claimedStage: RestoreStage,
    note?: string,
  ) => {
    await ports.sessions.recordUserAttestation(attemptId, claimedStage, note);
    notify({
      tone: "success",
      title: "已记录手动续聊标记",
      description: "已添加您的手动确认标记。系统状态保持客观记录不变。",
    });
    await queryClient.invalidateQueries({ queryKey: ["sessions-attempts"] });
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
        description: err instanceof Error ? err.message : String(err),
      });
      setGuideOpen(true);
    }
  };

  // ─── Browser Native-Only Fallback ────────────────────────────────
  if (!isNative) {
    return (
      <div className="fy-sessions-page" data-testid="sessions-page">
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
    <div className="fy-sessions-page" data-testid="sessions-page">
      {/* 顶部栏 */}
      <header className="fy-sessions-topbar">
        <div className="fy-sessions-breadcrumb">
          <span>会话中心</span>
          <span>/</span>
          <span className="fy-crumb-current">跨设备会话恢复与浏览</span>
          <span
            style={{
              marginLeft: "8px",
              fontSize: "11px",
              padding: "2px 8px",
              borderRadius: "999px",
              background: "var(--fy-surface-inset)",
              color: "var(--fy-text-tertiary)",
              border: "1px solid var(--fy-border)",
            }}
          >
            按软件与版本检查
          </span>
        </div>

        <div className="fy-sessions-top-actions">
          {selectedSessionKey && (
            <Button
              className="fy-btn-back-mobile"
              style={{ display: "none" }}
              onClick={() => setSelectedSessionKey(null)}
            >
              <ArrowLeftIcon size={14} />
              <span>返回列表</span>
            </Button>
          )}

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

      {/* 双栏工作台：复用 SplitPanes */}
      <SplitPanes
        className="fy-sessions-split-layout"
        minWidths={[280, 480]}
        defaultWidths={[340, 700]}
        flexiblePane={1}
        separatorLabels={["调整会话列表与详情宽度"]}
      >
        {/* 左栏：Master 列表与搜索筛选 */}
        <aside className="fy-sessions-master-pane" aria-label="会话列表">
          {/* 模式切换选项卡：本地会话 vs 恢复记录 */}
          <div className="fy-sessions-mode-bar" role="tablist">
            <button
              type="button"
              className={`fy-mode-tab ${viewMode === "sessions" ? "active" : ""}`}
              onClick={() => setViewMode("sessions")}
            >
              <ChatsCircleIcon size={14} />
              <span>本地会话 ({sessions.length})</span>
            </button>
            <button
              type="button"
              className={`fy-mode-tab ${viewMode === "attempts" ? "active" : ""}`}
              onClick={() => setViewMode("attempts")}
            >
              <ClockCounterClockwiseIcon size={14} />
              <span>恢复记录 ({attempts.length})</span>
            </button>
          </div>

          {viewMode === "sessions" ? (
            <>
              <div className="fy-sessions-master-header">
                {/* 7 软件芯片过滤器 */}
                <div
                  className="fy-provider-chips-row"
                  role="tablist"
                  aria-label="按AI软件筛选"
                >
                  <button
                    type="button"
                    className={`fy-provider-chip ${providerFilter === "all" ? "active" : ""}`}
                    onClick={() => setProviderFilter("all")}
                  >
                    全部 ({sessions.length})
                  </button>
                  {SUPPORTED_PROVIDER_IDS.map((pId: string) => {
                    const count = sessions.filter(
                      (s: SessionMeta) => s.providerId === pId,
                    ).length;
                    return (
                      <button
                        key={pId}
                        type="button"
                        className={`fy-provider-chip ${providerFilter === pId ? "active" : ""}`}
                        onClick={() => setProviderFilter(pId)}
                      >
                        {PROVIDER_LABELS[pId] ?? pId}
                        {count > 0 ? ` (${count})` : ""}
                      </button>
                    );
                  })}
                </div>

                {/* 搜索框：复用 FeatureSearch 原件 */}
                <FeatureSearch
                  value={searchQuery}
                  onValueChange={setSearchQuery}
                  placeholder="搜索会话标题、摘要、源ID..."
                  ariaLabel="搜索会话"
                />

                {/* 多选状态条 */}
                {selectedKeys.size > 0 && (
                  <div className="fy-selection-toolbar">
                    <span>已选择 {selectedKeys.size} 个会话</span>
                    <div style={{ display: "flex", gap: "8px" }}>
                      <button
                        type="button"
                        className="fy-link-btn"
                        onClick={selectAllFiltered}
                      >
                        全选
                      </button>
                      <button
                        type="button"
                        className="fy-link-btn"
                        onClick={clearSelection}
                      >
                        取消
                      </button>
                    </div>
                  </div>
                )}
              </div>

              {/* 会话列表项 */}
              <div className="fy-sessions-list" role="list">
                {sessionsError ? (
                  <div
                    style={{
                      padding: "24px 16px",
                      textAlign: "center",
                      display: "flex",
                      flexDirection: "column",
                      alignItems: "center",
                      gap: "10px",
                    }}
                    role="alert"
                  >
                    <strong
                      style={{
                        color: "var(--fy-danger-text)",
                        fontSize: "13px",
                      }}
                    >
                      本地会话扫描失败
                    </strong>
                    <span
                      style={{
                        fontSize: "12px",
                        color: "var(--fy-text-secondary)",
                      }}
                    >
                      {parseMigrationError(sessionsLoadError).message}
                    </span>
                    <Button
                      type="button"
                      className="fy-btn-subtle"
                      onClick={() => void refetchSessions()}
                    >
                      重新扫描
                    </Button>
                  </div>
                ) : loadingSessions && sessions.length === 0 ? (
                  <div
                    style={{
                      padding: "24px",
                      textAlign: "center",
                      color: "var(--fy-text-tertiary)",
                      fontSize: "13px",
                    }}
                  >
                    正在扫描本地会话…
                  </div>
                ) : filteredSessions.length === 0 ? (
                  <div
                    style={{
                      padding: "24px",
                      textAlign: "center",
                      color: "var(--fy-text-tertiary)",
                      fontSize: "13px",
                    }}
                  >
                    {searchQuery ? "未找到匹配会话" : "暂无本地会话"}
                  </div>
                ) : (
                  filteredSessions.map((s: SessionMeta) => {
                    const stableKey = getSessionStableKey(s);
                    const isSelected = stableKey === activeStableKey;
                    const isChecked = selectedKeys.has(stableKey);
                    return (
                      <div
                        key={stableKey}
                        role="listitem"
                        tabIndex={0}
                        className={`fy-session-item-card ${isSelected ? "selected" : ""}`}
                        onClick={() => setSelectedSessionKey(stableKey)}
                        onKeyDown={(e) => {
                          if (e.key === "Enter" || e.key === " ") {
                            e.preventDefault();
                            setSelectedSessionKey(stableKey);
                          }
                        }}
                      >
                        <div className="fy-session-item-top">
                          <div
                            style={{
                              display: "flex",
                              alignItems: "center",
                              gap: "6px",
                            }}
                          >
                            <button
                              type="button"
                              className="fy-checkbox-btn"
                              aria-label={isChecked ? "取消勾选" : "勾选以导出"}
                              onClick={(e) => toggleSelectSession(stableKey, e)}
                            >
                              {isChecked ? (
                                <CheckSquareIcon
                                  size={16}
                                  weight="fill"
                                  color="var(--fy-brand-blue)"
                                />
                              ) : (
                                <SquareIcon size={16} />
                              )}
                            </button>
                            <span className="fy-session-item-provider-tag">
                              {PROVIDER_LABELS[s.providerId] ?? s.providerId}
                            </span>
                          </div>
                          {s.lastActiveAt && (
                            <span className="fy-session-item-time">
                              {new Date(s.lastActiveAt).toLocaleDateString()}
                            </span>
                          )}
                        </div>
                        <div
                          className="fy-session-item-title"
                          title={s.title || s.sessionId}
                        >
                          {s.title || s.summary || s.sessionId}
                        </div>
                        <div className="fy-session-item-meta">
                          <span
                            className="fy-session-item-id-snippet"
                            title={s.sessionId}
                          >
                            ID:{" "}
                            {s.sessionId.length > 14
                              ? `${s.sessionId.slice(0, 12)}…`
                              : s.sessionId}
                          </span>
                          {s.projectDir && (
                            <span
                              title={s.projectDir}
                              style={{
                                overflow: "hidden",
                                textOverflow: "ellipsis",
                                whiteSpace: "nowrap",
                              }}
                            >
                              • {s.projectDir.split("/").pop()}
                            </span>
                          )}
                        </div>
                      </div>
                    );
                  })
                )}
              </div>
            </>
          ) : (
            /* 恢复记录独立视图 (解决部分 scanner 尚未展示 native 新导入的问题) */
            <div className="fy-attempts-view" role="list">
              {attemptsError ? (
                <div
                  style={{
                    padding: "32px 16px",
                    textAlign: "center",
                    display: "flex",
                    flexDirection: "column",
                    alignItems: "center",
                    gap: "10px",
                  }}
                  role="alert"
                >
                  <strong
                    style={{ color: "var(--fy-danger-text)", fontSize: "13px" }}
                  >
                    恢复记录加载失败
                  </strong>
                  <span
                    style={{
                      fontSize: "12px",
                      color: "var(--fy-text-secondary)",
                    }}
                  >
                    {parseMigrationError(attemptsLoadError).message}
                  </span>
                  <Button
                    type="button"
                    className="fy-btn-subtle"
                    onClick={() => void refetchAttempts()}
                  >
                    重新加载
                  </Button>
                </div>
              ) : attempts.length === 0 ? (
                <div
                  style={{
                    padding: "32px 16px",
                    textAlign: "center",
                    color: "var(--fy-text-tertiary)",
                    fontSize: "13px",
                  }}
                >
                  暂无跨设备恢复记录。
                  <br />
                  使用顶部“导入会话包”恢复其他设备的会话。
                </div>
              ) : (
                attempts.map((a: RestoreAttempt) => {
                  const isSelected = a.attemptId === activeAttempt?.attemptId;
                  return (
                    <div
                      key={a.attemptId}
                      role="listitem"
                      tabIndex={0}
                      className={`fy-attempt-item-card ${isSelected ? "selected" : ""}`}
                      onClick={() => setSelectedAttemptId(a.attemptId)}
                      onKeyDown={(e) => {
                        if (e.key === "Enter" || e.key === " ") {
                          e.preventDefault();
                          setSelectedAttemptId(a.attemptId);
                        }
                      }}
                    >
                      <div className="fy-attempt-item-top">
                        <span className="fy-session-item-provider-tag">
                          {PROVIDER_LABELS[a.targetProviderId] ??
                            a.targetProviderId}
                        </span>
                        <span className="fy-attempt-stage-badge">
                          {RESTORE_STAGE_LABELS[a.stage as RestoreStage] ??
                            a.stage}
                        </span>
                      </div>
                      <div className="fy-attempt-item-native-id">
                        本地会话 ID: {a.targetNativeId || "待分配"}
                      </div>
                      <div className="fy-attempt-item-workspace">
                        工作区: {a.targetWorkspace}
                      </div>
                      <div className="fy-attempt-item-time">
                        {new Date(a.createdAt).toLocaleString()}
                      </div>
                    </div>
                  );
                })
              )}
            </div>
          )}

          {/* 底部辅助入口：记忆模块跳转 */}
          <div
            style={{
              padding: "10px 16px",
              borderTop: "1px solid var(--fy-divider)",
              background: "var(--fy-surface-soft)",
              fontSize: "12px",
              display: "flex",
              alignItems: "center",
              justifyContent: "space-between",
            }}
          >
            <div
              style={{
                display: "flex",
                alignItems: "center",
                gap: "6px",
                color: "var(--fy-text-secondary)",
              }}
            >
              <BookOpenIcon size={16} />
              <span>原各软件本地记忆文件</span>
            </div>
            <Link
              to="/memory"
              style={{
                color: "var(--fy-brand-blue)",
                fontWeight: 600,
                textDecoration: "none",
                fontSize: "12px",
              }}
            >
              记忆模块 →
            </Link>
          </div>
        </aside>

        {/* 右栏：Detail 面板 */}
        <main className="fy-sessions-detail-pane" aria-label="会话详情">
          {viewMode === "attempts" && activeAttempt ? (
            /* 恢复记录详情展示 */
            <div className="fy-attempt-detail-view">
              <div className="fy-sessions-detail-header">
                <div className="fy-detail-top-row">
                  <div className="fy-detail-title-group">
                    <h1 className="fy-detail-title">
                      恢复记录：
                      {PROVIDER_LABELS[activeAttempt.targetProviderId] ??
                        activeAttempt.targetProviderId}{" "}
                      -{" "}
                      {activeAttempt.targetNativeId ||
                        activeAttempt.attemptId.slice(0, 8)}
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
                        创建于:{" "}
                        {new Date(activeAttempt.createdAt).toLocaleString()}
                      </span>
                    </div>
                  </div>

                  <div className="fy-detail-actions">
                    <Button
                      className="fy-control-button-primary"
                      disabled={!canOpenTarget}
                      onClick={() => void handleResumeInTarget()}
                    >
                      <PlayIcon size={14} weight="fill" />
                      <span>在目标软件中恢复</span>
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
                <StatusBanners
                  stage={activeAttempt.stage}
                  activeAttempt={activeAttempt}
                  isCapabilityVerified={isCapabilityVerified}
                  capabilityReason={capabilityReason}
                  onVerifyReadback={() => void handleVerifyReadback()}
                  verifyingReadback={verifyingReadback}
                  onOpenAttestationModal={() => setAttestationOpen(true)}
                />
              </div>

              <footer className="fy-sessions-bottom-bar">
                <div className="fy-bottom-info">
                  <span>
                    尝试 ID: {activeAttempt.attemptId} • 快照 ID:{" "}
                    {activeAttempt.snapshotId.slice(0, 16)}…
                  </span>
                </div>
                <div className="fy-bottom-actions">
                  <Button
                    className="fy-control-button-primary"
                    dialogOriginRef={attestationOriginRef}
                    onClick={() => setAttestationOpen(true)}
                  >
                    <CheckIcon size={16} weight="bold" />
                    <span>标记：我已手动续聊</span>
                  </Button>
                </div>
              </footer>
            </div>
          ) : selectedSession ? (
            /* 本地会话详情展示 */
            <>
              {/* 详情头部 */}
              <div className="fy-sessions-detail-header">
                <div className="fy-detail-top-row">
                  <div className="fy-detail-title-group">
                    <h1 className="fy-detail-title">
                      {selectedSession.title ||
                        selectedSession.summary ||
                        selectedSession.sessionId}
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
                      <span>•</span>
                      <span
                        style={{
                          fontFamily: "var(--font-mono)",
                          fontSize: "11px",
                        }}
                      >
                        源ID: {selectedSession.sessionId}
                      </span>
                    </div>
                  </div>

                  <div className="fy-detail-actions">
                    <Button
                      dialogOriginRef={remapOriginRef}
                      onClick={() => setRemapOpen(true)}
                    >
                      <FolderSimpleIcon size={16} />
                      <span>绑定本机目录</span>
                    </Button>
                    <Button
                      className="fy-control-button-primary"
                      disabled={!canOpenTarget}
                      onClick={() => void handleResumeInTarget()}
                    >
                      <PlayIcon size={14} weight="fill" />
                      <span>在目标软件中恢复</span>
                    </Button>
                  </div>
                </div>

                {/* 目标工作区路径映射条 */}
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

              {/* 消息滚动区 */}
              <div className="fy-sessions-stream-container">
                {/* 动态状态横幅 */}
                <StatusBanners
                  stage={activeAttempt?.stage}
                  isIndeterminate={hasIndeterminate}
                  hasIncompleteTurn={hasIncomplete}
                  isCapabilityVerified={isCapabilityVerified}
                  capabilityReason={capabilityReason}
                  isCodexProbeWarning={selectedSession.providerId === "codex"}
                  activeAttempt={activeAttempt}
                  structuredError={structuredError}
                  onVerifyReadback={() => void handleVerifyReadback()}
                  verifyingReadback={verifyingReadback}
                  onOpenAttestationModal={() => setAttestationOpen(true)}
                />

                {/* 问答流 */}
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

              {/* 底部操作底栏 */}
              <footer className="fy-sessions-bottom-bar">
                <div className="fy-bottom-info">
                  <span>
                    包含用户原始提问与最终答复；不包含工具日志、思考过程及工作区文件。
                  </span>
                </div>
                <div className="fy-bottom-actions">
                  <Button
                    dialogOriginRef={guideOriginRef}
                    onClick={() => setGuideOpen(true)}
                  >
                    <InfoIcon size={16} />
                    <span>查看启动说明</span>
                  </Button>
                  <Button
                    className="fy-control-button-primary"
                    dialogOriginRef={attestationOriginRef}
                    disabled={!activeAttempt}
                    onClick={() => setAttestationOpen(true)}
                  >
                    <CheckIcon size={16} weight="bold" />
                    <span>标记：我已手动续聊</span>
                  </Button>
                </div>
              </footer>
            </>
          ) : (
            <div
              style={{
                margin: "auto",
                textAlign: "center",
                color: "var(--fy-text-tertiary)",
              }}
            >
              请从左侧选择一个会话以查看内容与恢复状态
            </div>
          )}
        </main>
      </SplitPanes>

      {/* ─── 弹窗组件 (遵循 useDialogState 生命周期规范) ─────────────── */}
      {/* 1. 导出预览弹窗 (支持批量导出，逐项严格合规校验) */}
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

      {/* 2. 导入包与冲突处理弹窗 (重试幂等与本地探针门控) */}
      {importOpen && (
        <ImportPackageDialog
          key={importSessionKey}
          open={Boolean(importOpen)}
          onOpenChange={(open) => {
            if (!open) setImportOpen(null);
          }}
          onReadPackage={(path) => ports.sessions.readSessionPackage(path)}
          onRestore={handleRestore}
          onPickPackageFile={() => ports.sessions.pickPackageFile()}
          onPickDirectory={() => ports.sessions.pickDirectory()}
          localProbes={localProbes}
          initialTargetWorkspace={targetWorkspace}
          originRef={importOriginRef}
          onImportSuccess={() => void refreshSessions()}
        />
      )}

      {/* 3. 本地工作区重映射弹窗 */}
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

      {/* 4. 目标软件启动说明弹窗 */}
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

      {/* 5. 用户手动自报记录弹窗 */}
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

export default SessionsPage;
