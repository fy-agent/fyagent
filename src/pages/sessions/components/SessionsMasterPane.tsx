import type { MouseEvent } from "react";
import { Link } from "react-router-dom";
import { BookOpenIcon } from "@phosphor-icons/react/dist/csr/BookOpen";
import { ChatsCircleIcon } from "@phosphor-icons/react/dist/csr/ChatsCircle";
import { CheckSquareIcon } from "@phosphor-icons/react/dist/csr/CheckSquare";
import { ClockCounterClockwiseIcon } from "@phosphor-icons/react/dist/csr/ClockCounterClockwise";
import { SquareIcon } from "@phosphor-icons/react/dist/csr/Square";

import { Button } from "../../../shared/ui/Button";
import { FeatureSearch } from "../../../shared/ui/FeatureSearch";
import {
  getSessionStableKey,
  parseMigrationError,
  PROVIDER_LABELS,
  RESTORE_STAGE_LABELS,
  SUPPORTED_PROVIDER_IDS,
  type RestoreAttempt,
  type RestoreStage,
  type SessionMeta,
} from "../../../shared/features/session-migration";

export interface SessionsMasterPaneProps {
  viewMode: "sessions" | "attempts";
  onChangeViewMode: (mode: "sessions" | "attempts") => void;
  sessions: SessionMeta[];
  filteredSessions: SessionMeta[];
  loadingSessions: boolean;
  sessionsError: boolean;
  sessionsLoadError: unknown;
  onRefetchSessions: () => void;
  providerFilter: string;
  onChangeProviderFilter: (providerId: string) => void;
  searchQuery: string;
  onChangeSearchQuery: (query: string) => void;
  activeStableKey: string | null;
  onSelectSessionKey: (key: string) => void;
  selectedKeys: Set<string>;
  onToggleSelectSession: (key: string, event: MouseEvent) => void;
  onSelectAllFiltered: () => void;
  onClearSelection: () => void;
  attempts: RestoreAttempt[];
  attemptsError: boolean;
  attemptsLoadError: unknown;
  onRefetchAttempts: () => void;
  activeAttemptId: string | undefined;
  onSelectAttemptId: (attemptId: string) => void;
}

export function SessionsMasterPane({
  viewMode,
  onChangeViewMode,
  sessions,
  filteredSessions,
  loadingSessions,
  sessionsError,
  sessionsLoadError,
  onRefetchSessions,
  providerFilter,
  onChangeProviderFilter,
  searchQuery,
  onChangeSearchQuery,
  activeStableKey,
  onSelectSessionKey,
  selectedKeys,
  onToggleSelectSession,
  onSelectAllFiltered,
  onClearSelection,
  attempts,
  attemptsError,
  attemptsLoadError,
  onRefetchAttempts,
  activeAttemptId,
  onSelectAttemptId,
}: SessionsMasterPaneProps) {
  return (
    <aside className="fy-sessions-master-pane" aria-label="会话列表">
      <div className="fy-sessions-mode-bar" role="tablist">
        <button
          type="button"
          className={`fy-mode-tab ${viewMode === "sessions" ? "active" : ""}`}
          onClick={() => onChangeViewMode("sessions")}
        >
          <ChatsCircleIcon size={14} />
          <span>本地会话 ({sessions.length})</span>
        </button>
        <button
          type="button"
          className={`fy-mode-tab ${viewMode === "attempts" ? "active" : ""}`}
          onClick={() => onChangeViewMode("attempts")}
        >
          <ClockCounterClockwiseIcon size={14} />
          <span>恢复记录 ({attempts.length})</span>
        </button>
      </div>

      {viewMode === "sessions" ? (
        <>
          <div className="fy-sessions-master-header">
            <div
              className="fy-provider-chips-row"
              role="tablist"
              aria-label="按AI软件筛选"
            >
              <button
                type="button"
                className={`fy-provider-chip ${providerFilter === "all" ? "active" : ""}`}
                onClick={() => onChangeProviderFilter("all")}
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
                    onClick={() => onChangeProviderFilter(pId)}
                  >
                    {PROVIDER_LABELS[pId] ?? pId}
                    {count > 0 ? ` (${count})` : ""}
                  </button>
                );
              })}
            </div>

            <FeatureSearch
              value={searchQuery}
              onValueChange={onChangeSearchQuery}
              placeholder="搜索会话标题、摘要、源ID..."
              ariaLabel="搜索会话"
            />

            {selectedKeys.size > 0 && (
              <div className="fy-selection-toolbar">
                <span>已选择 {selectedKeys.size} 个会话</span>
                <div style={{ display: "flex", gap: "8px" }}>
                  <button
                    type="button"
                    className="fy-link-btn"
                    onClick={onSelectAllFiltered}
                  >
                    全选
                  </button>
                  <button
                    type="button"
                    className="fy-link-btn"
                    onClick={onClearSelection}
                  >
                    取消
                  </button>
                </div>
              </div>
            )}
          </div>

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
                  onClick={onRefetchSessions}
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
                    onClick={() => onSelectSessionKey(stableKey)}
                    onKeyDown={(e) => {
                      if (e.key === "Enter" || e.key === " ") {
                        e.preventDefault();
                        onSelectSessionKey(stableKey);
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
                          onClick={(e) => onToggleSelectSession(stableKey, e)}
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
                      {s.title || s.summary || "未命名会话"}
                    </div>
                    <div className="fy-session-item-meta">
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
                onClick={onRefetchAttempts}
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
              const isSelected = a.attemptId === activeAttemptId;
              return (
                <div
                  key={a.attemptId}
                  role="listitem"
                  tabIndex={0}
                  className={`fy-attempt-item-card ${isSelected ? "selected" : ""}`}
                  onClick={() => onSelectAttemptId(a.attemptId)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" || e.key === " ") {
                      e.preventDefault();
                      onSelectAttemptId(a.attemptId);
                    }
                  }}
                >
                  <div className="fy-attempt-item-top">
                    <span className="fy-session-item-provider-tag">
                      {PROVIDER_LABELS[a.targetProviderId] ??
                        a.targetProviderId}
                    </span>
                    <span className="fy-attempt-stage-badge">
                      {RESTORE_STAGE_LABELS[a.stage as RestoreStage] ?? a.stage}
                    </span>
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
          className="fy-sessions-memory-link"
          style={{
            fontWeight: 600,
            textDecoration: "none",
            fontSize: "12px",
          }}
        >
          记忆模块 →
        </Link>
      </div>
    </aside>
  );
}
