import { useQueryClient } from "@tanstack/react-query";
import { useMemo, useRef, useState } from "react";

import { getSupportedAppIcon } from "../../shared/assets/apps";
import {
  buildMcpSearchText,
  convergeSelection,
  errorMessage,
  mcpInstallDirectory,
  overlayKnownMcpFields,
  parseAdvancedServerJson,
  parseKeyValueLines,
  runSequentialBulk,
  sanitizeMcpConfigurationError,
  UserFacingError,
} from "../../shared/features/helpers";
import { redactMcpArgs, redactMcpUrl } from "../../shared/features/mcpSecurity";
import { mcpPresets } from "../../shared/features/presets";
import { useFeatures } from "../../shared/features/provider";
import { featureKeys, useMcpServers } from "../../shared/features/queries";
import { useWideFeatureLayout } from "../../shared/features/responsive";
import {
  createMcpAssignments,
  MCP_TARGETS,
  type McpServer,
  type McpServerSpec,
  type McpTargetId,
} from "../../shared/features/types";
import {
  Badge,
  Button,
  ConfirmDialog,
  Dialog,
  EmptyState,
  InlineNotice,
  Input,
  Spinner,
} from "../../shared/ui/primitives";
import { AssignmentPanel } from "../../shared/ui/AssignmentPanel";
import { CopyablePath } from "../../shared/ui/CopyablePath";
import { ExternalLinkButton } from "../../shared/ui/ExternalLinkButton";
import { FeatureList, FeatureListItem } from "../../shared/ui/FeatureList";
import { FeatureSearch } from "../../shared/ui/FeatureSearch";
import { FeatureTabs } from "../../shared/ui/FeatureTabs";
import { SplitPanes } from "../../shared/ui/split";
import { findCatalogItem, MCP_PROVENANCE_LABEL } from "./catalog";
import { DEFAULT_NEW_APPS } from "./constants";
import { McpDiscovery } from "./Discovery";
import "./page.css";

function transportOf(server: McpServer): "stdio" | "http" | "sse" {
  if (server.server.type === "http" || server.server.type === "sse") {
    return server.server.type;
  }
  return "stdio";
}

function assignedMcpTargets(server: McpServer) {
  return MCP_TARGETS.filter((target) => Boolean(server.apps[target.id]));
}

const INSTALLED_SPLIT_LABELS = ["调整列表与详情的宽度", "调整详情与分配的宽度"];
const WORKBUDDY_TRUST_TITLE = "需要在 WorkBuddy 中信任 MCP";
const WORKBUDDY_TRUST_DESCRIPTION =
  "请到「连接器 → 自定义连接器」中信任该 MCP 后才能使用。";

function ServerDetail({
  server,
  busy,
  onToggle,
  onEdit,
  onDelete,
  showAssignment,
}: {
  server: McpServer;
  busy: boolean;
  onToggle: (app: McpTargetId, enabled: boolean) => void;
  onEdit: () => void;
  onDelete: () => void;
  showAssignment: boolean;
}) {
  const spec = server.server;
  const transport = transportOf(server);
  const assigned = assignedMcpTargets(server);
  const catalogItem = findCatalogItem(server.id);
  const sourceLabel = catalogItem ? "精选目录" : "手动添加";
  const installDirectory = mcpInstallDirectory(spec);
  const description =
    server.description?.trim() || catalogItem?.description || "暂无说明";
  const homepage = server.homepage || catalogItem?.homepage;
  const docs = server.docs || catalogItem?.docs;

  return (
    <section
      className="fy-feature-panel fy-feature-detail fy-feature-detail-scroll"
      aria-label="MCP 详情"
    >
      <div className="fy-feature-detail-header">
        <div className="fy-feature-detail-title">
          <h2>{server.name}</h2>
          <Badge tone="accent">{transport}</Badge>
          <Badge tone={catalogItem ? "accent" : "neutral"}>{sourceLabel}</Badge>
        </div>
        <p className="fy-feature-intro">{description}</p>
        <div className="fy-feature-actions">
          <Button onClick={onEdit} disabled={busy}>
            编辑
          </Button>
          <Button
            className="fy-control-button-danger"
            onClick={onDelete}
            disabled={busy}
          >
            删除
          </Button>
        </div>
      </div>
      <div className="fy-feature-info-grid">
        <section className="fy-feature-info-card" aria-label="安装来源">
          <h3>安装来源</h3>
          <p className="fy-feature-info-lead">
            {catalogItem
              ? "此 MCP 来自内置精选目录。安装后写入统一配置，并可继续在编辑窗口调整。"
              : "此 MCP 由手动添加或从现有 Agent 配置导入，没有绑定精选目录条目。"}
          </p>
          <dl className="fy-feature-definition">
            <dt>来源类型</dt>
            <dd>{sourceLabel}</dd>
            {catalogItem && (
              <>
                <dt>发布方</dt>
                <dd>{catalogItem.publisher}</dd>
                <dt>来源标识</dt>
                <dd>{MCP_PROVENANCE_LABEL[catalogItem.provenance]}</dd>
              </>
            )}
            <dt>ID</dt>
            <dd>
              <code className="fy-feature-code">{server.id}</code>
            </dd>
            <dt>安装目录</dt>
            <dd>
              {installDirectory ? (
                <CopyablePath revealValue={false} value={installDirectory} />
              ) : (
                "无本地安装目录"
              )}
            </dd>
          </dl>
          {(homepage || docs) && (
            <div className="fy-feature-actions">
              {homepage && (
                <ExternalLinkButton url={homepage}>主页</ExternalLinkButton>
              )}
              {docs && <ExternalLinkButton url={docs}>文档</ExternalLinkButton>}
            </div>
          )}
        </section>
        <section className="fy-feature-info-card" aria-label="当前分配">
          <h3>当前分配</h3>
          <p className="fy-feature-info-lead">
            {assigned.length > 0
              ? `已启用 ${assigned.length} 个应用。需要增减时，使用应用分配开关。`
              : "尚未分配到任何应用。启用后，对应软件才能加载此 MCP。"}
          </p>
          {assigned.length > 0 && (
            <ul className="fy-feature-app-chips">
              {assigned.map((app) => (
                <li key={app.id} className="fy-feature-app-chip">
                  <img
                    className="fy-feature-assignment-icon"
                    src={getSupportedAppIcon(app.id)}
                    alt=""
                    aria-hidden="true"
                  />
                  {app.label}
                </li>
              ))}
            </ul>
          )}
        </section>
        <section
          className="fy-feature-info-card fy-feature-info-span"
          aria-label="安装信息"
        >
          <h3>安装信息</h3>
          <dl className="fy-feature-definition">
            <dt>传输类型</dt>
            <dd>{transport}</dd>
            {spec.command && (
              <>
                <dt>命令</dt>
                <dd>
                  <code className="fy-feature-code">{spec.command}</code>
                </dd>
              </>
            )}
            {spec.args && spec.args.length > 0 && (
              <>
                <dt>参数</dt>
                <dd>
                  {redactMcpArgs(spec.args).map((argument, index) => (
                    <code
                      className="fy-feature-code"
                      key={`${argument}-${index}`}
                    >
                      {argument}
                    </code>
                  ))}
                </dd>
              </>
            )}
            {spec.cwd && spec.cwd.trim() !== installDirectory && (
              <>
                <dt>工作目录</dt>
                <dd>
                  <code className="fy-feature-code">{spec.cwd}</code>
                </dd>
              </>
            )}
            {spec.url && (
              <>
                <dt>URL</dt>
                <dd>
                  <code className="fy-feature-code">
                    {redactMcpUrl(spec.url)}
                  </code>
                </dd>
              </>
            )}
            {spec.env && (
              <>
                <dt>环境变量</dt>
                <dd>{Object.keys(spec.env).length} 项（仅在编辑时显示）</dd>
              </>
            )}
            {spec.headers && (
              <>
                <dt>请求头</dt>
                <dd>{Object.keys(spec.headers).length} 项（仅在编辑时显示）</dd>
              </>
            )}
          </dl>
        </section>
      </div>
      {showAssignment && (
        <div className="fy-feature-inline-assignment">
          <AssignmentPanel
            apps={server.apps}
            disabled={busy}
            labelSuffix="MCP 分配"
            onToggle={onToggle}
            targets={MCP_TARGETS}
          />
        </div>
      )}
    </section>
  );
}

export function McpPage() {
  const queryClient = useQueryClient();
  const { ports, notify, installTarget, setInstallTarget } = useFeatures();
  const wideLayout = useWideFeatureLayout();
  const query = useMcpServers();
  const servers = useMemo(() => Object.values(query.data ?? {}), [query.data]);
  const [tab, setTab] = useState<"installed" | "discovery">("installed");
  const [search, setSearch] = useState("");
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [editing, setEditing] = useState<McpServer | "new" | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<McpServer | null>(null);
  const [busy, setBusy] = useState(false);
  const [workbuddyTrustOpen, setWorkbuddyTrustOpen] = useState(false);
  const [progress, setProgress] = useState<{
    done: number;
    total: number;
  } | null>(null);
  const writeLock = useRef(false);
  const filtered = useMemo(() => {
    const value = search.trim().toLocaleLowerCase();
    return value
      ? servers.filter((server) => buildMcpSearchText(server).includes(value))
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
  ) => {
    if (writeLock.current) return false;
    writeLock.current = true;
    setBusy(true);
    try {
      await operation();
      notify({ tone: "success", title });
      onSuccess?.();
      return true;
    } catch (error) {
      notify({
        tone: "error",
        title: `${title}失败`,
        description: sanitizeMcpConfigurationError(error),
      });
      return false;
    } finally {
      await refresh();
      setProgress(null);
      setBusy(false);
      writeLock.current = false;
    }
  };
  const noteWorkBuddyTrust = () => setWorkbuddyTrustOpen(true);
  const toggle = (server: McpServer, app: McpTargetId, enabled: boolean) =>
    write(
      "分配已更新",
      async () => {
        await ports.mcp.toggleApp(server.id, app, enabled);
      },
      () => {
        if (app === "workbuddy" && enabled) noteWorkBuddyTrust();
      },
    );
  const bulkAssign = (app: McpTargetId, enabled: boolean) =>
    write(
      "批量分配完成",
      async () => {
        const ids = servers
          .filter((server) => Boolean(server.apps[app]) !== enabled)
          .map((server) => server.id);
        const result = await runSequentialBulk(
          ids,
          (id) => ports.mcp.toggleApp(id, app, enabled),
          (done, total) => setProgress({ done, total }),
        );
        if (result.failures.length)
          throw new UserFacingError(
            `${result.failures.length} 项失败，${result.successes.length} 项成功`,
          );
      },
      () => {
        if (app === "workbuddy" && enabled) noteWorkBuddyTrust();
      },
    );
  const importExisting = () =>
    write("MCP 导入", async () => {
      const count = await ports.mcp.importFromApps();
      notify({
        tone: "info",
        title: count === 0 ? "没有发现可导入的 MCP" : `已导入 ${count} 个 MCP`,
      });
    });
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
          <Button disabled={busy} onClick={() => void importExisting()}>
            导入现有
          </Button>
          <Button
            className="fy-control-button-primary"
            disabled={busy}
            onClick={() => setEditing("new")}
          >
            添加 MCP
          </Button>
        </div>
      </header>
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
      {tab === "discovery" ? (
        query.isLoading ? (
          <EmptyState title="正在加载 MCP" description="正在读取安装状态">
            <Spinner />
          </EmptyState>
        ) : (
          <div className="fy-feature-workspace">
            <McpDiscovery
              servers={servers}
              busy={busy}
              defaultTarget={installTarget}
              onPickTarget={setInstallTarget}
              onInstall={async (server) =>
                write(
                  "MCP 已安装",
                  async () => {
                    await ports.mcp.upsert(server);
                    setSelectedId(server.id);
                  },
                  () => {
                    if (server.apps.workbuddy) noteWorkBuddyTrust();
                  },
                )
              }
              onViewInstalled={(id) => {
                setSelectedId(id);
                setTab("installed");
              }}
            />
          </div>
        )
      ) : query.isLoading ? (
        <EmptyState title="正在加载 MCP" description="正在读取 MCP 服务">
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
            <>
              <Button onClick={() => void importExisting()}>导入现有</Button>{" "}
              <Button
                className="fy-control-button-primary"
                onClick={() => setEditing("new")}
              >
                添加 MCP
              </Button>{" "}
              <Button onClick={() => setTab("discovery")}>浏览发现</Button>
            </>
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
              description="为保护敏感信息，密钥和请求头不会参与搜索。"
            />
          ) : (
            <SplitPanes separatorLabels={INSTALLED_SPLIT_LABELS}>
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
                        {server.description ||
                          server.tags?.join(" · ") ||
                          "暂无说明"}{" "}
                        · {transportOf(server)} ·{" "}
                        {
                          MCP_TARGETS.filter((app) => server.apps[app.id])
                            .length
                        }{" "}
                        Agent
                      </span>
                    </FeatureListItem>
                  ))}
                </FeatureList>
              </section>
              {selected && (
                <ServerDetail
                  server={selected}
                  busy={busy}
                  onToggle={(app, enabled) => toggle(selected, app, enabled)}
                  onEdit={() => setEditing(selected)}
                  onDelete={() => setDeleteTarget(selected)}
                  showAssignment={!wideLayout}
                />
              )}
              {selected && wideLayout && (
                <section className="fy-feature-panel fy-feature-assign-scroll">
                  <AssignmentPanel
                    apps={selected.apps}
                    disabled={busy}
                    labelSuffix="MCP 分配"
                    onToggle={(app, enabled) => toggle(selected, app, enabled)}
                    targets={MCP_TARGETS}
                  />
                  <hr />
                  <h3>全量分配</h3>
                  {MCP_TARGETS.map((app) => (
                    <div key={app.id} className="fy-feature-assignment">
                      <span>{app.label}</span>
                      <span>
                        <Button
                          disabled={busy}
                          onClick={() => bulkAssign(app.id, true)}
                        >
                          全开
                        </Button>{" "}
                        <Button
                          disabled={busy}
                          onClick={() => bulkAssign(app.id, false)}
                        >
                          全关
                        </Button>
                      </span>
                    </div>
                  ))}
                </section>
              )}
            </SplitPanes>
          )}
        </div>
      )}
      {editing !== null && (
        <McpEditor
          key={editing === "new" ? "new" : editing.id}
          initial={editing === "new" ? null : editing}
          existingIds={new Set(servers.map((server) => server.id))}
          busy={busy}
          onClose={() => setEditing(null)}
          onSave={(server) => {
            const wasAssigned =
              editing !== "new" && Boolean(editing.apps.workbuddy);
            void write(
              editing === "new" ? "MCP 已添加" : "MCP 已更新",
              async () => {
                await ports.mcp.upsert(server);
                setEditing(null);
              },
              () => {
                if (server.apps.workbuddy && !wasAssigned) noteWorkBuddyTrust();
              },
            );
          }}
        />
      )}
      <Dialog
        open={workbuddyTrustOpen}
        title={WORKBUDDY_TRUST_TITLE}
        description={WORKBUDDY_TRUST_DESCRIPTION}
        onOpenChange={(open) => {
          if (!open) setWorkbuddyTrustOpen(false);
        }}
        actions={
          <Button
            className="fy-control-button-primary"
            onClick={() => setWorkbuddyTrustOpen(false)}
          >
            知道了
          </Button>
        }
      >
        <p>
          WorkBuddy 官方限制第三方 MCP 必须在安装后手动信任授权才能正常使用。
        </p>
      </Dialog>
      <ConfirmDialog
        open={deleteTarget !== null}
        title={`删除 ${deleteTarget?.name ?? "MCP"}`}
        description="将从管理列表及已启用的应用中删除。"
        pending={busy}
        onCancel={() => setDeleteTarget(null)}
        onConfirm={async () => {
          const target = deleteTarget;
          if (target)
            await write("MCP 已删除", async () => {
              await ports.mcp.delete(target.id);
            });
          setDeleteTarget(null);
        }}
      />
    </div>
  );
}

type Mode = "quick" | "advanced";

function McpEditor({
  initial,
  existingIds,
  busy,
  onClose,
  onSave,
}: {
  initial: McpServer | null;
  existingIds: Set<string>;
  busy: boolean;
  onClose: () => void;
  onSave: (server: McpServer) => void;
}) {
  const spec = initial?.server ?? {};
  const [id, setId] = useState(initial?.id ?? "");
  const [name, setName] = useState(initial?.name ?? "");
  const [description, setDescription] = useState(initial?.description ?? "");
  const [tags, setTags] = useState((initial?.tags ?? []).join(", "));
  const [homepage, setHomepage] = useState(initial?.homepage ?? "");
  const [docs, setDocs] = useState(initial?.docs ?? "");
  const [transport, setTransport] = useState<"stdio" | "http" | "sse">(
    spec.type === "http" || spec.type === "sse" ? spec.type : "stdio",
  );
  const [command, setCommand] = useState(spec.command ?? "");
  const [args, setArgs] = useState((spec.args ?? []).join("\n"));
  const [cwd, setCwd] = useState(spec.cwd ?? "");
  const [url, setUrl] = useState(spec.url ?? "");
  const [env, setEnv] = useState(
    Object.entries(spec.env ?? {})
      .map(([key, value]) => `${key}=${value}`)
      .join("\n"),
  );
  const [headers, setHeaders] = useState(
    Object.entries(spec.headers ?? {})
      .map(([key, value]) => `${key}: ${value}`)
      .join("\n"),
  );
  const [apps, setApps] = useState(() =>
    initial ? { ...initial.apps } : createMcpAssignments(DEFAULT_NEW_APPS),
  );
  const [mode, setMode] = useState<Mode>("quick");
  const [advanced, setAdvanced] = useState(JSON.stringify(spec, null, 2));
  const [errors, setErrors] = useState<string[]>([]);
  const [preset, setPreset] = useState("custom");
  const original = useRef<McpServer | null>(
    initial ? structuredClone(initial) : null,
  );
  const draft = useRef<McpServerSpec>(structuredClone(spec));
  const applyPreset = (presetId: string) => {
    setPreset(presetId);
    if (presetId === "custom") return;
    const value = mcpPresets.find((item) => item.id === presetId);
    if (!value) return;
    setId(value.id);
    setName(value.name);
    setTags((value.tags ?? []).join(", "));
    setHomepage(value.homepage ?? "");
    setDocs(value.docs ?? "");
    setTransport(
      value.server.type === "http" || value.server.type === "sse"
        ? value.server.type
        : "stdio",
    );
    setCommand(value.server.command ?? "");
    setArgs((value.server.args ?? []).join("\n"));
    setCwd(value.server.cwd ?? "");
    setUrl(value.server.url ?? "");
    setEnv("");
    setHeaders("");
    draft.current = structuredClone(value.server);
    setAdvanced(JSON.stringify(value.server, null, 2));
  };
  const applySpecToQuickForm = (value: McpServerSpec) => {
    setTransport(
      value.type === "http" || value.type === "sse" ? value.type : "stdio",
    );
    setCommand(value.command ?? "");
    setArgs((value.args ?? []).join("\n"));
    setCwd(value.cwd ?? "");
    setUrl(value.url ?? "");
    setEnv(
      Object.entries(value.env ?? {})
        .map(([key, item]) => `${key}=${item}`)
        .join("\n"),
    );
    setHeaders(
      Object.entries(value.headers ?? {})
        .map(([key, item]) => `${key}: ${item}`)
        .join("\n"),
    );
  };
  const quickSpec = (): McpServerSpec => {
    const envResult = parseKeyValueLines(env, "env");
    const headersResult = parseKeyValueLines(headers, "headers");
    if (envResult.errors.length || headersResult.errors.length)
      throw new UserFacingError(
        [
          ...envResult.errors.map((item) => `环境变量：${item}`),
          ...headersResult.errors.map((item) => `请求头：${item}`),
        ].join("；"),
      );
    if (transport === "stdio") {
      if (!command.trim()) throw new UserFacingError("请填写启动命令。");
      return {
        type: "stdio",
        command: command.trim(),
        ...(args.trim() ? { args: args.split(/\r?\n/).filter(Boolean) } : {}),
        ...(cwd.trim() ? { cwd: cwd.trim() } : {}),
        ...(Object.keys(envResult.value).length
          ? { env: envResult.value }
          : {}),
      };
    }
    if (!url.trim()) throw new UserFacingError("请填写连接地址。");
    try {
      new URL(url.trim());
    } catch {
      throw new UserFacingError("连接地址格式无效。");
    }
    return {
      type: transport,
      url: url.trim(),
      ...(Object.keys(headersResult.value).length
        ? { headers: headersResult.value }
        : {}),
    };
  };
  const switchMode = (next: Mode) => {
    try {
      if (next === "advanced") {
        draft.current = overlayKnownMcpFields(draft.current, quickSpec());
        setAdvanced(JSON.stringify(draft.current, null, 2));
      } else {
        draft.current = parseAdvancedServerJson(advanced);
        applySpecToQuickForm(draft.current);
      }
      setMode(next);
      setErrors([]);
    } catch (error) {
      setErrors([errorMessage(error)]);
    }
  };
  const submit = () => {
    const nextErrors: string[] = [];
    const trimmedId = id.trim();
    if (!trimmedId) nextErrors.push("ID 为必填项");
    if (!initial && existingIds.has(trimmedId)) nextErrors.push("ID 已存在");
    if (!name.trim()) nextErrors.push("名称为必填项");
    let spec: McpServerSpec | null = null;
    try {
      spec =
        mode === "advanced"
          ? parseAdvancedServerJson(advanced)
          : overlayKnownMcpFields(draft.current, quickSpec());
    } catch (error) {
      nextErrors.push(errorMessage(error));
    }
    if (nextErrors.length || !spec) {
      setErrors(nextErrors);
      return;
    }
    const base = original.current ?? {};
    onSave({
      ...base,
      id: initial?.id ?? trimmedId,
      name: name.trim(),
      server: spec,
      apps: { ...(initial?.apps ?? {}), ...apps },
      description: description.trim() || undefined,
      tags: tags
        .split(",")
        .map((item) => item.trim())
        .filter(Boolean),
      homepage: homepage.trim() || undefined,
      docs: docs.trim() || undefined,
    } as McpServer);
  };
  return (
    <Dialog
      open
      onOpenChange={(next) => !next && !busy && onClose()}
      title={initial ? `编辑 ${initial.name}` : "添加 MCP"}
      description="可使用表单或 JSON 编辑服务配置。敏感信息仅在此窗口显示。"
      large
      actions={
        <>
          <Button onClick={onClose} disabled={busy}>
            取消
          </Button>
          <Button
            className="fy-control-button-primary"
            onClick={submit}
            disabled={busy}
          >
            {busy ? "保存中…" : "保存"}
          </Button>
        </>
      }
    >
      {errors.length > 0 && (
        <InlineNotice tone="error">
          <ul>
            {errors.map((error) => (
              <li key={error}>{error}</li>
            ))}
          </ul>
        </InlineNotice>
      )}
      <div className="fy-feature-form-grid">
        {!initial && (
          <label className="fy-control-field">
            模板
            <select
              className="fy-control-select"
              value={preset}
              onChange={(event) => applyPreset(event.target.value)}
            >
              <option value="custom">自定义</option>
              {mcpPresets.map((item) => (
                <option key={item.id} value={item.id}>
                  {item.name}
                </option>
              ))}
            </select>
          </label>
        )}
        <label className="fy-control-field">
          ID
          <Input
            value={id}
            onChange={(event) => setId(event.target.value)}
            disabled={Boolean(initial)}
          />
        </label>
        <label className="fy-control-field">
          名称
          <Input
            value={name}
            onChange={(event) => setName(event.target.value)}
          />
        </label>
        <label className="fy-control-field">
          描述
          <Input
            value={description}
            onChange={(event) => setDescription(event.target.value)}
          />
        </label>
        <label className="fy-control-field">
          标签（逗号分隔）
          <Input
            value={tags}
            onChange={(event) => setTags(event.target.value)}
          />
        </label>
        <label className="fy-control-field">
          主页
          <Input
            value={homepage}
            onChange={(event) => setHomepage(event.target.value)}
          />
        </label>
        <label className="fy-control-field">
          文档
          <Input
            value={docs}
            onChange={(event) => setDocs(event.target.value)}
          />
        </label>
        <FeatureTabs
          id="mcp-editor-mode-tabs"
          className="fy-feature-form-span"
          label="编辑模式"
          value={mode}
          onChange={switchMode}
          options={[
            { id: "quick", label: "快速配置" },
            { id: "advanced", label: "JSON 编辑" },
          ]}
        />
        {mode === "quick" ? (
          <>
            <label className="fy-control-field">
              传输类型
              <select
                className="fy-control-select"
                value={transport}
                onChange={(event) =>
                  setTransport(event.target.value as typeof transport)
                }
              >
                <option value="stdio">stdio</option>
                <option value="http">http</option>
                <option value="sse">sse</option>
              </select>
            </label>
            {transport === "stdio" ? (
              <>
                <label className="fy-control-field">
                  命令
                  <Input
                    value={command}
                    onChange={(event) => setCommand(event.target.value)}
                  />
                </label>
                <label className="fy-control-field fy-feature-form-span">
                  参数（每行一个）
                  <textarea
                    className="fy-control-textarea"
                    rows={4}
                    value={args}
                    onChange={(event) => setArgs(event.target.value)}
                  />
                </label>
                <label className="fy-control-field">
                  工作目录
                  <Input
                    value={cwd}
                    onChange={(event) => setCwd(event.target.value)}
                  />
                </label>
                <label className="fy-control-field fy-feature-form-span">
                  环境变量（KEY=VALUE）
                  <textarea
                    className="fy-control-textarea"
                    rows={4}
                    value={env}
                    onChange={(event) => setEnv(event.target.value)}
                  />
                </label>
              </>
            ) : (
              <>
                <label className="fy-control-field fy-feature-form-span">
                  URL
                  <Input
                    value={url}
                    onChange={(event) => setUrl(event.target.value)}
                  />
                </label>
                <label className="fy-control-field fy-feature-form-span">
                  请求头（Name: Value 或 Name=Value）
                  <textarea
                    className="fy-control-textarea"
                    rows={4}
                    value={headers}
                    onChange={(event) => setHeaders(event.target.value)}
                  />
                </label>
              </>
            )}
          </>
        ) : (
          <label className="fy-control-field fy-feature-form-span">
            单个服务配置（JSON）
            <textarea
              className="fy-control-textarea"
              rows={14}
              value={advanced}
              onChange={(event) => setAdvanced(event.target.value)}
              spellCheck={false}
            />
          </label>
        )}
        <div className="fy-feature-form-span">
          <AssignmentPanel
            apps={apps}
            disabled={busy}
            labelSuffix="MCP 分配"
            onToggle={(app, enabled) =>
              setApps((current) => ({ ...current, [app]: enabled }))
            }
            targets={MCP_TARGETS}
          />
        </div>
      </div>
    </Dialog>
  );
}
