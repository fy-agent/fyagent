import { useQueryClient } from "@tanstack/react-query";
import { useMemo, useRef, useState } from "react";

import {
  buildMcpSearchText,
  convergeSelection,
  errorMessage,
  sanitizeMcpConfigurationError,
  UserFacingError,
} from "../../shared/features/helpers";
import {
  MCP_IMPORT_SOURCES,
  type McpImportReport,
  type McpImportSourceId,
} from "../../shared/features/mcp";
import { useFeatures } from "../../shared/features/provider";
import { featureKeys, useMcpServers } from "../../shared/features/queries";
import { useWideFeatureLayout } from "../../shared/features/responsive";
import {
  MCP_TARGETS,
  type McpServer,
  type McpTargetId,
} from "../../shared/features/types";
import { Button } from "../../shared/ui/Button";
import { AnimatePresence } from "../../shared/ui/motion";
import {
  captureDialogOrigin,
  type DialogOriginRef,
} from "../../shared/ui/dialogOrigin";
import { useDialogState } from "../../shared/ui/useDialogState";
import { ConfirmDialog, Dialog } from "../../shared/ui/Dialog";
import {
  Checkbox,
  EmptyState,
  InlineNotice,
  Spinner,
} from "../../shared/ui/primitives";
import { AssignmentPanel } from "../../shared/ui/AssignmentPanel";
import { BulkAssignmentDialog } from "../../shared/features/controls/BulkAssignmentDialog";
import {
  executeBulkAssignment,
  type BulkAssignmentItem,
  type BulkAssignmentPlan,
  type BulkAssignmentResult,
} from "../../shared/features/bulk-assignment";
import { FeatureList, FeatureListItem } from "../../shared/ui/FeatureList";
import { FeatureSearch } from "../../shared/ui/FeatureSearch";
import { FeatureTabPanel, FeatureTabs } from "../../shared/ui/FeatureTabs";
import { SplitPanes } from "../../shared/ui/split";
import { DETAIL_PANE_SIZING } from "../../shared/ui/split/sizing";
import { WorkBuddyTrustDialog } from "../../shared/ui/WorkBuddyTrustDialog";
import { McpDiscovery } from "./Discovery";
import { InspectorDialog } from "./InspectorDialog";
import { McpEditor } from "./McpEditor";
import { ServerDetail } from "./ServerDetail";
import { transportOf } from "./constants";
import "./page.css";

const INSTALLED_SPLIT_LABELS = ["调整列表与详情的宽度", "调整详情与分配的宽度"];

export function McpPage({
  creationTarget,
}: { creationTarget?: McpTargetId } = {}) {
  const dialogOriginRef = useRef<HTMLElement | null>(null);
  const queryClient = useQueryClient();
  const { ports, notify, installTarget, setInstallTarget } = useFeatures();
  const wideLayout = useWideFeatureLayout();
  const query = useMcpServers();
  const servers = useMemo(() => Object.values(query.data ?? {}), [query.data]);
  const [tab, setTab] = useState<"installed" | "discovery">("installed");
  const [search, setSearch] = useState("");
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [editing, setEditing, editingKey] = useDialogState<McpServer | "new">();
  const [inspecting, setInspecting, inspectingKey] =
    useDialogState<McpServer>();
  const [deleteTarget, setDeleteTarget] = useState<McpServer | null>(null);
  const [deleteError, setDeleteError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [bulkOpen, setBulkOpen] = useState(false);
  const [bulkTrustNeeded, setBulkTrustNeeded] = useState(false);
  const [importOpen, setImportOpen] = useState(false);
  const [importSources, setImportSources] = useState<McpImportSourceId[]>([]);
  const [importReport, setImportReport] = useState<McpImportReport | null>(
    null,
  );
  const [workbuddyTrustOpen, setWorkbuddyTrustOpen] = useState(false);
  const [trustOrigin, setTrustOrigin] = useState<DialogOriginRef>({
    current: null,
  });
  const [progress, setProgress] = useState<{
    done: number;
    total: number;
  } | null>(null);
  const writeLock = useRef(false);
  const filtered = useMemo(() => {
    const value = search.trim().toLocaleLowerCase();
    return value
      ? servers.filter((server) =>
          [
            buildMcpSearchText(server),
            ...(server.sources ?? []).map(
              (id) =>
                MCP_IMPORT_SOURCES.find((source) => source.id === id)?.label ??
                id,
            ),
          ]
            .join(" ")
            .toLocaleLowerCase()
            .includes(value),
        )
      : servers;
  }, [search, servers]);
  const convergedId = convergeSelection(filtered, selectedId);
  const selected = filtered.find((server) => server.id === convergedId) ?? null;
  const refresh = () =>
    queryClient.invalidateQueries({ queryKey: featureKeys.mcp });
  const write = async (
    title: string,
    operation: () => Promise<void>,
    onSuccess?: () => void,
    notifySuccess = true,
    onFailure?: (message: string) => void,
  ) => {
    if (writeLock.current) return false;
    writeLock.current = true;
    setBusy(true);
    try {
      await operation();
      await refresh();
      if (notifySuccess) notify({ tone: "success", title });
      onSuccess?.();
      return true;
    } catch (error) {
      try {
        await refresh();
      } catch {
        /* Keep the original failure visible. */
      }
      const message = sanitizeMcpConfigurationError(error);
      onFailure?.(message);
      // The editor callback owns visible inline feedback; a duplicate toast
      // would cover its footer actions on compact windows.
      if (!onFailure)
        notify({ tone: "error", title: `${title}失败`, description: message });
      return false;
    } finally {
      setProgress(null);
      setBusy(false);
      writeLock.current = false;
    }
  };
  const noteWorkBuddyTrust = (origin: DialogOriginRef = dialogOriginRef) => {
    // Follow-up notices adopt the initiating operation's source, not another
    // page-level action. The next Dialog consumes its own one-use capture.
    setTrustOrigin({ ...origin });
    delete origin.snapshot;
    delete origin.returnTarget;
    setWorkbuddyTrustOpen(true);
  };
  const toggle = (server: McpServer, app: McpTargetId, enabled: boolean) =>
    write(
      "分配已更新",
      async () => {
        await ports.mcp.toggleApp(server.id, app, enabled);
        const observed = (await ports.mcp.getAll())[server.id];
        if (observed?.apps[app] !== enabled)
          throw new UserFacingError(
            "分配未确认，可能存在部分写入。请刷新后重试。",
          );
      },
      () => {
        if (app === "workbuddy" && enabled) noteWorkBuddyTrust();
      },
    );
  const bulkItem = (server: McpServer): BulkAssignmentItem => ({
    id: server.id,
    name: server.name,
    apps: server.apps,
    // Kept in memory for drift comparison; never rendered, logged or exported.
    identity: JSON.stringify({ ...server, apps: undefined }),
  });
  const executeAssignment = async (
    plan: BulkAssignmentPlan,
    onResult: (result: BulkAssignmentResult) => void,
  ) => {
    if (writeLock.current)
      throw new UserFacingError("另一项操作仍在执行，请稍后重新预览。");
    writeLock.current = true;
    setBusy(true);
    try {
      const results = await executeBulkAssignment(
        plan,
        async () => Object.values(await ports.mcp.getAll()).map(bulkItem),
        (id, target, enabled) => ports.mcp.toggleApp(id, target, enabled),
        onResult,
      );
      if (
        plan.target === "workbuddy" &&
        plan.enabled &&
        results.some(
          (result) =>
            result.status === "confirmed" &&
            plan.items.find((item) => item.id === result.id)?.apps[
              plan.target
            ] !== plan.enabled,
        )
      )
        setBulkTrustNeeded(true);
    } finally {
      try {
        await refresh();
      } finally {
        setBusy(false);
        writeLock.current = false;
      }
    }
  };
  const importExisting = () => {
    const sources = [...importSources];
    setImportOpen(false);
    return write(
      "MCP 导入",
      async () => {
        const report = await ports.mcp.importFromApps(sources);
        setImportReport(report);
        const failures = report.sources.filter(
          (source) => source.failureCode !== null,
        ).length;
        notify({
          tone: failures || report.projectionFailed ? "error" : "info",
          title: failures
            ? report.projectionFailed
              ? `MCP 导入完成，${failures} 个来源失败，${report.projectionFailed} 项工具配置写入失败`
              : `MCP 导入完成，${failures} 个来源失败`
            : report.projectionFailed
              ? "MCP 导入完成，部分工具配置写入失败"
              : "MCP 导入结果已更新",
        });
      },
      undefined,
      false,
    );
  };
  return (
    <div
      className="fy-feature-page fy-split-page fy-mcp-page"
      data-testid="mcp-page"
      aria-label="MCP"
    >
      <header className="fy-feature-header">
        <h1 className="fy-mcp-page-title">MCP 管理</h1>
        <FeatureTabs
          id="mcp-view-tabs"
          label="MCP 视图"
          value={tab}
          onChange={setTab}
          options={[
            { id: "installed", label: "已安装" },
            { id: "discovery", label: "发现" },
          ]}
        />
        <div className="fy-feature-actions">
          <Button
            disabled={busy || !servers.length}
            dialogOriginRef={dialogOriginRef}
            onClick={() => setBulkOpen(true)}
          >
            批量分配
          </Button>
          <Button
            disabled={busy}
            dialogOriginRef={dialogOriginRef}
            onClick={() => {
              setImportSources([]);
              setImportOpen(true);
            }}
          >
            导入现有
          </Button>
          <Button
            className="fy-control-button-primary"
            disabled={busy}
            onClick={() => setEditing("new")}
            dialogOriginRef={dialogOriginRef}
          >
            添加 MCP
          </Button>
        </div>
      </header>
      {importReport && (
        <section
          className="fy-mcp-import-results"
          aria-label="MCP 导入结果"
          aria-live="polite"
          tabIndex={0}
        >
          {importReport.sources.map((source) => (
            <p key={source.source}>
              <strong>
                {
                  MCP_IMPORT_SOURCES.find((item) => item.id === source.source)
                    ?.label
                }
              </strong>
              ：
              {source.failureCode
                ? "来源读取或配置冲突校验失败，本来源未写入；检查配置后重新导入。"
                : `新增 ${source.added} · 分配状态变化 ${source.assignmentChanged} · 未变化 ${source.unchanged} · 来源停用，未收录 ${source.disabledSkipped}`}
            </p>
          ))}
          {importReport.projectionFailed > 0 && (
            <>
              <p>
                <strong>
                  工具配置写入失败 {importReport.projectionFailed} 项
                </strong>
                ，已收录的数据仍保留；请检查工具配置后重试。
              </p>
              {importReport.projectionFailures.map((failure, index) => (
                <p key={`${failure.target}-${index}`}>
                  <strong>
                    {
                      MCP_IMPORT_SOURCES.find(
                        (item) => item.id === failure.target,
                      )?.label
                    }
                  </strong>
                  ：
                  {
                    {
                      invalid_config: "配置格式或服务定义无效",
                      io_failed: "配置文件读写失败",
                      projection_failed: "工具配置同步失败",
                    }[failure.reason]
                  }
                </p>
              ))}
            </>
          )}
          <p className="fy-feature-description">
            来源收录与工具配置写入分别报告；分配状态不代表连接或实际运行已经验证。
          </p>
        </section>
      )}
      {progress && (
        <>
          <div className="fy-feature-progress">
            <span
              style={{
                width: `${progress.total ? (progress.done / progress.total) * 100 : 0}%`,
              }}
            />
          </div>
          <p className="fy-feature-description">
            正在处理 {progress.done}/{progress.total}
          </p>
        </>
      )}
      {query.error && query.data !== undefined && (
        <InlineNotice tone="error">
          刷新失败，正在显示上一次成功数据：{errorMessage(query.error)}
        </InlineNotice>
      )}
      <AnimatePresence>
        {bulkOpen && (
          <BulkAssignmentDialog
            key="mcp-bulk"
            kind="MCP"
            originRef={dialogOriginRef}
            busy={busy}
            items={servers.map(bulkItem)}
            onClose={() => {
              setBulkOpen(false);
              if (bulkTrustNeeded) {
                setBulkTrustNeeded(false);
                noteWorkBuddyTrust();
              }
            }}
            onExecute={executeAssignment}
          />
        )}
      </AnimatePresence>
      <FeatureTabPanel
        tabsId="mcp-view-tabs"
        value="discovery"
        layout="workspace"
        active={tab === "discovery"}
        unmountOnExit
      >
        {query.isLoading ? (
          <EmptyState title="正在加载 MCP">
            <Spinner />
          </EmptyState>
        ) : (
          <div className="fy-feature-workspace">
            <McpDiscovery
              servers={servers}
              busy={busy}
              defaultTarget={installTarget}
              onPickTarget={setInstallTarget}
              onInstall={async (server, origin) =>
                write(
                  "MCP 已安装",
                  async () => {
                    await ports.mcp.upsert(server);
                    setSelectedId(server.id);
                  },
                  () => {
                    if (server.apps.workbuddy) noteWorkBuddyTrust(origin);
                  },
                )
              }
              onViewInstalled={(id) => {
                setSelectedId(id);
                setTab("installed");
              }}
            />
          </div>
        )}
      </FeatureTabPanel>
      <FeatureTabPanel
        tabsId="mcp-view-tabs"
        value="installed"
        layout="workspace"
        active={tab === "installed"}
        unmountOnExit
      >
        {query.isLoading ? (
          <EmptyState title="正在加载 MCP">
            <Spinner />
          </EmptyState>
        ) : query.error && query.data === undefined ? (
          <EmptyState
            title="无法加载 MCP"
            description={errorMessage(query.error)}
            actions={<Button onClick={() => void query.refetch()}>重试</Button>}
          />
        ) : servers.length === 0 ? (
          <EmptyState
            title="还没有 MCP 服务"
            description="添加新的 MCP，从现有 Agent 配置导入，或到发现页浏览精选"
            actions={
              <Button onClick={() => setTab("discovery")}>浏览发现</Button>
            }
          />
        ) : (
          <div className="fy-feature-workspace">
            <div className="fy-feature-toolbar">
              <FeatureSearch
                ariaLabel="搜索 MCP"
                placeholder="搜索名称、命令、标签或来源"
                value={search}
                onValueChange={setSearch}
              />
            </div>
            {filtered.length === 0 ? (
              <EmptyState
                title="没有匹配的 MCP"
                description="请尝试名称、命令、标签或来源。"
              />
            ) : (
              <SplitPanes
                {...DETAIL_PANE_SIZING}
                separatorLabels={INSTALLED_SPLIT_LABELS}
              >
                <section
                  className="fy-feature-panel fy-feature-list-panel"
                  aria-label="MCP 列表"
                >
                  <h2>已安装 · {servers.length}</h2>
                  <FeatureList id="mcp-server-list">
                    {filtered.map((server) => (
                      <FeatureListItem
                        key={server.id}
                        selected={server.id === selected?.id}
                        title={server.name}
                        onSelect={() => setSelectedId(server.id)}
                      >
                        <span>
                          {[
                            server.description || server.tags?.join(" · "),
                            transportOf(server),
                            `${MCP_IMPORT_SOURCES.filter((source) => server.apps[source.id]).length} 个分配`,
                          ]
                            .filter(Boolean)
                            .join(" · ")}
                        </span>
                      </FeatureListItem>
                    ))}
                  </FeatureList>
                </section>
                {selected && (
                  <ServerDetail
                    key={selected.id}
                    originRef={dialogOriginRef}
                    server={selected}
                    busy={busy}
                    onToggle={(app, enabled) => toggle(selected, app, enabled)}
                    onEdit={() => setEditing(selected)}
                    onInspect={() => setInspecting(selected)}
                    onDelete={() => {
                      setDeleteError(null);
                      setDeleteTarget(selected);
                    }}
                    showAssignment={!wideLayout}
                  />
                )}
                {selected && wideLayout && (
                  <section className="fy-feature-panel fy-feature-assign-scroll">
                    <AssignmentPanel
                      dialogOriginRef={dialogOriginRef}
                      apps={selected.apps}
                      disabled={busy}
                      labelSuffix="MCP 分配"
                      onToggle={(app, enabled) =>
                        toggle(selected, app, enabled)
                      }
                      targets={MCP_TARGETS}
                    />
                    <hr />
                  </section>
                )}
              </SplitPanes>
            )}
          </div>
        )}
      </FeatureTabPanel>
      {inspecting && (
        <InspectorDialog
          key={inspectingKey}
          server={inspecting}
          originRef={dialogOriginRef}
          onClose={() => setInspecting(null)}
        />
      )}
      <Dialog
        open={importOpen}
        originRef={dialogOriginRef}
        onOpenChange={setImportOpen}
        title="选择 MCP 导入来源"
        description="只读取所选来源并收录到 FyAgent；保留来源停用状态。此步骤不会写入 Agent 配置或测试连接。"
        actions={
          <>
            <Button onClick={() => setImportOpen(false)}>取消</Button>
            <Button
              className="fy-control-button-primary"
              disabled={busy || importSources.length === 0}
              onClick={() => void importExisting()}
            >
              开始导入
            </Button>
          </>
        }
      >
        <div className="fy-mcp-import-source-list">
          {MCP_IMPORT_SOURCES.map((source) => (
            <label className="fy-mcp-import-source" key={source.id}>
              <Checkbox
                label={source.label}
                checked={importSources.includes(source.id)}
                onCheckedChange={(checked) => {
                  setImportSources((current) =>
                    checked
                      ? [...current, source.id]
                      : current.filter((id) => id !== source.id),
                  );
                }}
              />
              <span>{source.label}</span>
            </label>
          ))}
        </div>
      </Dialog>
      <AnimatePresence>
        {editing !== null && (
          <McpEditor
            key={editingKey}
            originRef={dialogOriginRef}
            initial={editing === "new" ? null : editing}
            creationTarget={creationTarget}
            existingIds={new Set(servers.map((server) => server.id))}
            busy={busy}
            onClose={() => setEditing(null)}
            onSave={async (server, event) => {
              const wasAssigned =
                editing !== "new" && Boolean(editing.apps.workbuddy);
              if (server.apps.workbuddy && !wasAssigned) {
                const source: DialogOriginRef = dialogOriginRef;
                captureDialogOrigin(
                  source,
                  event.currentTarget,
                  source.returnTarget ?? source.current,
                );
              }
              let failure =
                "MCP 操作未确认完成，请核对管理列表和目标配置后再继续。";
              const saved = await write(
                editing === "new" ? "MCP 已添加" : "MCP 已更新",
                async () => {
                  await ports.mcp.upsert(server);
                },
                () => {
                  setEditing(null);
                  if (server.apps.workbuddy && !wasAssigned)
                    noteWorkBuddyTrust();
                },
                true,
                (message) => {
                  failure = message;
                },
              );
              return saved
                ? null
                : `${failure}。管理库或部分目标可能已更改；草稿已保留，请先核对管理列表和目标配置，再决定是否重试。`;
            }}
          />
        )}
      </AnimatePresence>
      <WorkBuddyTrustDialog
        originRef={trustOrigin}
        open={workbuddyTrustOpen}
        onOpenChange={(open) => {
          if (!open) setWorkbuddyTrustOpen(false);
        }}
      />
      <ConfirmDialog
        originRef={dialogOriginRef}
        open={deleteTarget !== null}
        title={`删除 ${deleteTarget?.name ?? "MCP"}`}
        description={deleteError ?? "将从管理列表及已启用的应用中删除。"}
        pending={busy}
        onCancel={() => setDeleteTarget(null)}
        onConfirm={async () => {
          const target = deleteTarget;
          if (!target) return;
          const removed = await write("MCP 已删除", async () => {
            await ports.mcp.delete(target.id);
          });
          if (removed) setDeleteTarget(null);
          else
            setDeleteError(
              "删除未确认完成，部分目标可能已更改。请先核对管理列表和目标配置，再决定是否重试或取消。",
            );
        }}
      />
    </div>
  );
}
