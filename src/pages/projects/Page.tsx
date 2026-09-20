import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useRef, useState, type ComponentType } from "react";
import type {
  Project,
  ProjectContext,
  ProjectDependencySnapshot,
  ResourceOption,
} from "../../domain/projects";
import { projectError } from "../../shared/features/projects";
import { useFeatures } from "../../shared/features/provider";
import { featureKeys } from "../../shared/features/queries";
import { CopyablePath } from "../../shared/features/controls/CopyablePath";
import { Button } from "../../shared/ui/Button";
import { ConfirmDialog } from "../../shared/ui/Dialog";
import { FeatureTabPanel, FeatureTabs } from "../../shared/ui/FeatureTabs";
import { EmptyState, InlineNotice } from "../../shared/ui/primitives";
import {
  usePrimaryBlocker,
  usePrimaryBlockerOrigin,
} from "../../shared/ui/PrimaryBlocker";
import { usePersistentSearchParams } from "../../shared/ui/usePersistentSearchParams";
import {
  PersistentSurface,
  usePersistentVisibility,
} from "../../shared/ui/PersistentSurface";
import "./projects.css";

export interface ProjectPanelProps {
  projectId: string;
  projectRevision: number;
  archived: boolean;
  kit: Project["kit"];
  disabled: boolean;
  onProjectChanged: () => Promise<void>;
}
export interface ProjectsPageProps {
  DeliveryKitPanel?: ComponentType<ProjectPanelProps>;
  VerificationPanel?: ComponentType<ProjectPanelProps>;
}
const states: Record<string, string> = {
  unverifiable: "版本尚无法确认",
  matched: "与保存版本一致",
  drifted: "来源已变化",
  missing: "来源已删除",
  unavailable: "暂不可用",
  revoked: "已撤销",
  purpose_mismatch: "用途不匹配",
};
const kinds: Record<string, string> = {
  provider: "模型配置",
  mcp: "MCP",
  skill: "Skill",
  prompt: "提示词",
  memory: "记忆",
};
const resourceKey = (r: ResourceOption) =>
  JSON.stringify([r.kind, r.agentId, r.rawId]);
type ProjectView = "prepare" | "delivery" | "verification";

export function ProjectsPage({
  DeliveryKitPanel,
  VerificationPanel,
}: ProjectsPageProps = {}) {
  const { ports } = useFeatures();
  const queryClient = useQueryClient();
  const { visible, searchParams, setSearchParams } =
    usePersistentSearchParams();
  const customers = useQuery({
    queryKey: featureKeys.projectCustomers,
    queryFn: () => ports.projects.listCustomers(),
    enabled: visible,
  });
  const projects = useQuery({
    queryKey: featureKeys.projects,
    queryFn: () => ports.projects.list(),
    enabled: visible,
    staleTime: Infinity,
  });
  const [customerName, setCustomerName] = useState("");
  const [newProjectName, setNewProjectName] = useState("");
  const [customerId, setCustomerId] = useState("");
  const [creating, setCreating] = useState(false);
  const [newCustomer, setNewCustomer] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [showArchived, setShowArchived] = useState(false);
  const selected = searchParams.get("project");
  const selectedProject = projects.data?.find((p) => p.projectId === selected);
  const activeCustomers = customers.data?.filter((c) => !c.archived) ?? [];
  const creatingCustomer = newCustomer || activeCustomers.length === 0;
  const context = useQuery({
    queryKey: featureKeys.projectContext(selected),
    queryFn: () => ports.projects.getContext(selected!),
    retry: false,
    enabled: visible && !!selected,
    staleTime: Infinity,
  });
  const selectedContext =
    context.data?.projectId === selected ? context.data : undefined;
  const refresh = async () => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: featureKeys.projects }),
      queryClient.invalidateQueries({ queryKey: featureKeys.projectCustomers }),
      queryClient.invalidateQueries({
        queryKey: featureKeys.projectContext(selected),
      }),
      queryClient.invalidateQueries({
        queryKey: featureKeys.projectDependencies(selected),
      }),
    ]);
  };
  const createProject = async () => {
    if (busy || !visible) return;
    setBusy(true);
    setError("");
    try {
      let ownerId = customerId;
      if (creatingCustomer) {
        const c = await ports.projects.createCustomer(customerName);
        ownerId = c.customerId;
        setCustomerName("");
        setCustomerId(ownerId);
        setNewCustomer(false);
        await refresh();
      }
      const p = await ports.projects.create(ownerId, newProjectName);
      setNewProjectName("");
      await refresh();
      setCreating(false);
      setSearchParams({ project: p.projectId });
    } catch (e) {
      setError(projectError(e));
    } finally {
      setBusy(false);
    }
  };
  return (
    <div
      className="fy-feature-page fy-feature-workspace fy-projects-page"
      aria-label="客户项目"
      data-testid="projects-page"
    >
      <header className="fy-projects-header">
        <h1>客户项目</h1>
        <span>整理项目目标、交付方案与验证结果</span>
      </header>
      <div className="fy-projects-layout">
        <aside className="fy-projects-rail" aria-label="项目列表">
          <Button
            className="fy-control-button-primary fy-projects-create"
            aria-expanded={creating}
            aria-controls="project-create-form"
            onClick={() => setCreating((open) => !open)}
          >
            新建项目
          </Button>
          {creating && (
            <form
              id="project-create-form"
              className="fy-projects-create-form"
              aria-label="新建项目"
              onSubmit={(e) => {
                e.preventDefault();
                void createProject();
              }}
            >
              {activeCustomers.length > 0 && (
                <label>
                  所属客户
                  <select
                    className="fy-control-input"
                    value={newCustomer ? "new" : customerId}
                    disabled={busy}
                    onChange={(e) => {
                      setNewCustomer(e.target.value === "new");
                      setCustomerId(
                        e.target.value === "new" ? "" : e.target.value,
                      );
                    }}
                  >
                    <option value="">请选择客户</option>
                    {activeCustomers.map((c) => (
                      <option key={c.customerId} value={c.customerId}>
                        {c.name}
                      </option>
                    ))}
                    <option value="new">新建客户…</option>
                  </select>
                </label>
              )}
              {creatingCustomer && (
                <label>
                  客户名称
                  <input
                    className="fy-control-input"
                    value={customerName}
                    maxLength={160}
                    disabled={busy}
                    onChange={(e) => setCustomerName(e.target.value)}
                    placeholder="例如：星河商贸"
                  />
                </label>
              )}
              <label>
                项目名称
                <input
                  className="fy-control-input"
                  value={newProjectName}
                  maxLength={160}
                  disabled={busy}
                  onChange={(e) => setNewProjectName(e.target.value)}
                  placeholder="例如：经营周报"
                />
              </label>
              <Button
                type="submit"
                disabled={
                  busy ||
                  customers.isPending ||
                  customers.isError ||
                  (creatingCustomer ? !customerName.trim() : !customerId) ||
                  !newProjectName.trim()
                }
              >
                创建项目
              </Button>
            </form>
          )}
          <label className="fy-projects-check">
            <input
              type="checkbox"
              checked={showArchived}
              onChange={(e) => setShowArchived(e.target.checked)}
            />
            显示已归档
          </label>
          {error && <InlineNotice tone="error">{error}</InlineNotice>}
          {projects.isError || customers.isError ? (
            <EmptyState title="无法读取项目" description="请在桌面应用中重试。">
              <Button onClick={() => void refresh()}>重试</Button>
            </EmptyState>
          ) : projects.isPending ? (
            <p role="status">正在读取项目…</p>
          ) : projects.data?.length === 0 ? (
            <p className="fy-projects-hint">
              还没有项目。从一个具体的客户需求开始。
            </p>
          ) : null}
          <ul className="fy-projects-list">
            {projects.data
              ?.filter((p) => showArchived || !p.archived)
              .map((p) => (
                <li key={p.projectId}>
                  <Button
                    aria-pressed={selected === p.projectId}
                    onClick={() => setSearchParams({ project: p.projectId })}
                  >
                    <strong>{p.name}</strong>
                    <span>
                      {
                        customers.data?.find(
                          (c) => c.customerId === p.customerId,
                        )?.name
                      }
                      {p.archived ? " · 已归档" : ""}
                    </span>
                  </Button>
                </li>
              ))}
          </ul>
          <details>
            <summary>管理客户</summary>
            {customers.data?.map((c) => (
              <CustomerEditor
                key={`${c.customerId}:${c.revision}`}
                customer={c}
                onChanged={refresh}
              />
            ))}
          </details>
        </aside>
        <main className="fy-projects-detail" aria-label="项目详情">
          {!selectedProject ? (
            <EmptyState
              title={
                projects.data?.length ? "选择要继续的项目" : "开始一个客户项目"
              }
              description="记录客户要解决的问题，为项目选择交付方案，再保存验证结果与交接说明。"
            >
              <Button onClick={() => setCreating(true)}>
                {projects.data?.length ? "新建另一个项目" : "创建第一个项目"}
              </Button>
            </EmptyState>
          ) : (
            <ProjectEditor
              key={selectedProject.projectId}
              project={selectedProject}
              customerName={
                customers.data?.find(
                  (c) => c.customerId === selectedProject.customerId,
                )?.name ?? "客户"
              }
              context={
                context.isError || !selectedContext
                  ? {
                      projectId: selectedProject.projectId,
                      projectRevision: selectedProject.projectRevision,
                      content: selectedContext?.content ?? "",
                      directory: null,
                      state: "unavailable",
                      codexInstructions: null,
                    }
                  : selectedContext
              }
              contextLoading={context.isPending}
              onChanged={refresh}
              DeliveryKitPanel={DeliveryKitPanel}
              VerificationPanel={VerificationPanel}
            />
          )}
        </main>
      </div>
    </div>
  );
}

function CustomerEditor({
  customer,
  onChanged,
}: {
  customer: {
    customerId: string;
    name: string;
    revision: number;
    archived: boolean;
  };
  onChanged: () => Promise<void>;
}) {
  const { ports } = useFeatures();
  const visible = usePersistentVisibility();
  const [name, setName] = useState(customer.name);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  async function save(archived: boolean) {
    if (busy || !visible) return;
    setBusy(true);
    setError("");
    try {
      await ports.projects.updateCustomer(
        customer.customerId,
        customer.revision,
        name,
        archived,
      );
      await onChanged();
    } catch (e) {
      setError(projectError(e));
    } finally {
      setBusy(false);
    }
  }
  return (
    <div className="fy-projects-customer">
      <label>
        客户名称
        <input
          className="fy-control-input"
          value={name}
          onChange={(e) => setName(e.target.value)}
          maxLength={160}
        />
      </label>
      <div className="fy-projects-actions">
        <Button
          disabled={busy || !name.trim() || name === customer.name}
          onClick={() => void save(customer.archived)}
        >
          保存名称
        </Button>
        <Button disabled={busy} onClick={() => void save(!customer.archived)}>
          {customer.archived ? "恢复客户" : "归档客户"}
        </Button>
      </div>
      {error && <p role="alert">{error}</p>}
    </div>
  );
}

function ProjectEditor({
  project,
  customerName,
  context,
  contextLoading,
  onChanged,
  DeliveryKitPanel,
  VerificationPanel,
}: ProjectsPageProps & {
  project: Project;
  customerName: string;
  context: ProjectContext;
  contextLoading: boolean;
  onChanged: () => Promise<void>;
}) {
  const { ports } = useFeatures();
  const visible = usePersistentVisibility();
  const [nameDraft, setName] = useState<string | null>(null);
  const [textDraft, setText] = useState<string | null>(null);
  const name = nameDraft ?? project.name;
  const text = textDraft ?? context.content;
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [status, setStatus] = useState("");
  const [resource, setResource] = useState("");
  const [credential, setCredential] = useState("");
  const [model, setModel] = useState("");
  const [archiveOpen, setArchiveOpen] = useState(false);
  const [reloadOpen, setReloadOpen] = useState(false);
  const [recoveryOpen, setRecoveryOpen] = useState(false);
  const [view, setView] = useState<ProjectView>("prepare");
  const [visitedViews, setVisitedViews] = useState<ReadonlySet<ProjectView>>(
    () => new Set(["prepare"]),
  );
  const contextInput = useRef<HTMLTextAreaElement>(null);
  const tabsId = `project-work-${project.projectId}`;
  function changeView(next: ProjectView) {
    setView(next);
    setVisitedViews((visited) => new Set([...visited, next]));
  }
  const recoveryOrigin = useRef<HTMLElement | null>(null);
  const reloadOrigin = useRef<HTMLElement | null>(null);
  const archiveOrigin = useRef<HTMLElement | null>(null);
  const dirty = name !== project.name || text !== context.content;
  const blocker = usePrimaryBlocker(dirty || busy);
  const navigationOrigin = usePrimaryBlockerOrigin();
  const resources = useQuery({
    queryKey: featureKeys.projectResourceOptions,
    queryFn: () => ports.projects.resourceOptions(),
    enabled: visible && view === "prepare",
  });
  const credentials = useQuery({
    queryKey: featureKeys.projectCredentialOptions,
    queryFn: () => ports.projects.credentialOptions(),
    enabled: visible && view === "prepare",
  });
  const selectedResource = resources.data?.find(
    (r) => resourceKey(r) === resource,
  );
  const dependencies = useQuery({
    queryKey: featureKeys.projectDependencies(project.projectId),
    queryFn: () => ports.projects.dependencySnapshot(project.projectId),
    enabled: visible && view === "prepare",
  });
  const request = {
    projectId: project.projectId,
    expectedRevision: project.projectRevision,
  };
  const disabled = busy || project.archived || !visible;
  async function run(action: () => Promise<unknown>, message: string) {
    if (disabled) return;
    setBusy(true);
    setError("");
    setStatus("");
    try {
      await action();
      setStatus(message);
      await onChanged();
    } catch (e) {
      setError(projectError(e));
    } finally {
      setBusy(false);
    }
  }
  const panelProps: ProjectPanelProps = {
    projectId: project.projectId,
    projectRevision: project.projectRevision,
    archived: project.archived,
    kit: project.kit,
    disabled: disabled || dirty,
    onProjectChanged: onChanged,
  };
  return (
    <div className="fy-projects-workspace">
      <div className="fy-projects-title">
        <div>
          <p className="fy-projects-customer-name">{customerName}</p>
          <h2>{project.name}</h2>
          <p className="fy-projects-hint">
            {contextLoading
              ? "正在读取项目资料"
              : context.state === "materialized"
                ? "已保存工作说明"
                : context.state === "not_created"
                  ? "先写清项目目标与交付要求"
                  : "工作说明需要重新检查"}
            {project.kit ? " · 已选择交付方案" : " · 尚未选择交付方案"}
          </p>
        </div>
        {!project.archived && (
          <Button
            disabled={busy || contextLoading}
            onClick={() => {
              if (dirty || context.state !== "materialized") {
                changeView("prepare");
                if (view === "prepare") contextInput.current?.focus();
              } else changeView(project.kit ? "verification" : "delivery");
            }}
          >
            {dirty || context.state !== "materialized"
              ? "完善项目说明"
              : project.kit
                ? "查看验证与交接"
                : "选择交付方案"}
          </Button>
        )}
      </div>
      {project.archived && (
        <InlineNotice>
          项目已归档。工作说明、文件和资源引用仍保留。
        </InlineNotice>
      )}
      {error && <InlineNotice tone="error">{error}</InlineNotice>}
      {status && <p role="status">{status}</p>}
      {(error || dirty) && (
        <div className="fy-projects-draft-notice">
          {dirty && <span>项目资料有未保存的修改</span>}
          <Button
            disabled={busy || !visible}
            dialogOriginRef={reloadOrigin}
            onClick={() => setReloadOpen(true)}
          >
            重新载入项目
          </Button>
        </div>
      )}
      <FeatureTabs
        id={tabsId}
        label="项目工作区"
        value={view}
        onChange={changeView}
        options={[
          { id: "prepare", label: "项目准备" },
          { id: "delivery", label: "交付方案" },
          { id: "verification", label: "验证与交接" },
        ]}
      />
      <FeatureTabPanel
        tabsId={tabsId}
        value="prepare"
        active={view === "prepare"}
        layout="workspace"
        className="fy-projects-panel"
      >
        <section className="fy-projects-goal">
          <h3>这次要交付什么</h3>
          {!contextLoading && context.state === "unavailable" && (
            <InlineNotice tone="error">
              工作说明文件无法读取或已在外部修改。请先保留外部内容，再重新建立工作说明。
              <Button
                disabled={busy || !visible}
                onClick={() => void onChanged()}
              >
                重试读取
              </Button>
            </InlineNotice>
          )}
          {contextLoading && <p role="status">正在读取工作说明…</p>}
          <label>
            工作说明
            <textarea
              ref={contextInput}
              className="fy-control-input fy-projects-context"
              value={text}
              disabled={disabled || contextLoading}
              onChange={(e) => setText(e.target.value)}
              placeholder="客户要解决什么问题？使用哪些已获授权的资料？交付什么成果、如何验收？"
            />
          </label>
          <p className="fy-projects-hint">
            这份说明仅用于本项目，保存时保留先前版本。
          </p>
          <Button
            className="fy-control-button-primary"
            disabled={
              disabled ||
              contextLoading ||
              context.state === "unavailable" ||
              (text === context.content && context.state === "materialized")
            }
            onClick={() =>
              void run(async () => {
                await ports.projects.writeContext(
                  { ...request, expectedRevision: context.projectRevision },
                  text,
                );
                setText(null);
              }, "工作说明已保存")
            }
          >
            {busy ? "保存中…" : "保存工作说明"}
          </Button>
          {!contextLoading && context.state === "unavailable" && (
            <Button
              disabled={disabled}
              dialogOriginRef={recoveryOrigin}
              onClick={() => setRecoveryOpen(true)}
            >
              重新建立工作说明
            </Button>
          )}
        </section>
        <details className="fy-projects-optional">
          <summary>
            资源与账号{" "}
            <span>
              可选 · {project.resources.length} 项资源，
              {project.credentials.length} 个账号
            </span>
          </summary>
          {dirty && (
            <p className="fy-projects-hint">
              先保存项目资料，再调整资源与账号。
            </p>
          )}
          <section>
            <h3>为项目保留所需资源</h3>
            <p className="fy-projects-hint">
              关联已保存的模型、MCP、Skill
              或提示词，方便后续核对。引用不会自动在软件中启用。
            </p>
            {resources.isError ? (
              <p role="alert">无法读取资源，请刷新重试。</p>
            ) : (
              <label>
                选择资源
                <select
                  className="fy-control-input"
                  value={resource}
                  disabled={disabled || dirty}
                  onChange={(e) => setResource(e.target.value)}
                >
                  <option value="">请选择</option>
                  {resources.data?.map((r) => (
                    <option key={resourceKey(r)} value={resourceKey(r)}>
                      {r.agentId} · {kinds[r.kind]} · {r.label}
                    </option>
                  ))}
                </select>
              </label>
            )}
            {selectedResource?.kind === "provider" && (
              <label>
                模型名称
                <input
                  className="fy-control-input"
                  value={model}
                  onChange={(e) => setModel(e.target.value)}
                  maxLength={256}
                  disabled={disabled || dirty}
                />
              </label>
            )}
            <Button
              disabled={disabled || dirty || !selectedResource}
              onClick={() => {
                const r = selectedResource;
                if (r)
                  void run(
                    () =>
                      ports.projects.bindResource(
                        request,
                        r,
                        r.kind === "provider" ? model || null : null,
                      ),
                    "资源引用已保存",
                  );
              }}
            >
              添加引用
            </Button>
            <ul className="fy-projects-resources">
              {project.resources.map((r) => (
                <li key={`${r.kind}:${r.agentId}:${r.rawId}`}>
                  <div>
                    <strong>
                      {resources.data?.find(
                        (x) =>
                          x.kind === r.kind &&
                          x.agentId === r.agentId &&
                          x.rawId === r.rawId,
                      )?.label ?? "来源不可用"}
                    </strong>
                    <span>
                      {r.agentId} · {kinds[r.kind]}
                      {r.model ? ` · ${r.model}` : ""}
                    </span>
                    <span>
                      {dependencies.isError
                        ? "状态暂无法确认，请重新检查"
                        : states[
                            dependencies.data?.resources.find(
                              (item) =>
                                item.resource.kind === r.kind &&
                                item.resource.agentId === r.agentId &&
                                item.resource.rawId === r.rawId,
                            )?.state ?? "unverifiable"
                          ]}
                    </span>
                  </div>
                  <Button
                    disabled={disabled || dirty}
                    onClick={() =>
                      void run(
                        () => ports.projects.removeResource(request, r),
                        "引用已移除",
                      )
                    }
                  >
                    移除引用
                  </Button>
                </li>
              ))}
            </ul>
          </section>
          <section>
            <h3>记录项目使用的账号</h3>
            <p className="fy-projects-hint">
              仅保存账号用途，不复制凭据；尚未检查远端认证。
            </p>
            {credentials.isError ? (
              <p role="alert">无法读取账号，请重试。</p>
            ) : (
              <label>
                选择账号
                <select
                  className="fy-control-input"
                  value={credential}
                  disabled={disabled || dirty}
                  onChange={(e) => setCredential(e.target.value)}
                >
                  <option value="">请选择</option>
                  {credentials.data?.map((c) => (
                    <option
                      key={c.credentialId}
                      value={c.credentialId}
                      disabled={c.state !== "unverifiable"}
                    >
                      {c.label} · {c.consumer}
                    </option>
                  ))}
                </select>
              </label>
            )}
            <Button
              disabled={disabled || dirty || !credential}
              onClick={() => {
                const c = credentials.data?.find(
                  (c) => c.credentialId === credential,
                );
                if (c)
                  void run(
                    () => ports.projects.bindCredential(request, c),
                    "账号引用已保存",
                  );
              }}
            >
              绑定账号用途
            </Button>
            <ul className="fy-projects-resources">
              {project.credentials.map((c) => (
                <li key={c.credentialId}>
                  <div>
                    <strong>
                      {credentials.data?.find(
                        (x) => x.credentialId === c.credentialId,
                      )?.label ?? "来源不可用"}
                    </strong>
                    <span>{c.consumer}</span>
                    <span>
                      {dependencies.isError
                        ? "状态暂无法确认，请重新检查"
                        : states[
                            dependencies.data?.credentials.find(
                              (item) =>
                                item.binding.credentialId === c.credentialId,
                            )?.state ?? "unverifiable"
                          ]}
                    </span>
                  </div>
                  <Button
                    disabled={disabled || dirty}
                    onClick={() =>
                      void run(
                        () =>
                          ports.projects.removeCredential(
                            request,
                            c.credentialId,
                          ),
                        "账号引用已移除",
                      )
                    }
                  >
                    移除引用
                  </Button>
                </li>
              ))}
            </ul>
          </section>
          <section>
            <h3>检查资源是否仍可用</h3>
            {dependencies.data && !dependencies.isError ? (
              <DependencySummary snapshot={dependencies.data} />
            ) : (
              <p>
                {dependencies.isError
                  ? "无法核对项目，请刷新重试。"
                  : "正在核对项目…"}
              </p>
            )}
            <Button
              disabled={busy || dirty || !visible}
              onClick={() => void dependencies.refetch()}
            >
              重新检查引用
            </Button>
          </section>
        </details>
        <section>
          <h3>Codex 工作目录</h3>
          <p className="fy-projects-hint">
            {context.state !== "materialized"
              ? "先保存工作说明，再生成供 Codex 使用的独立目录。"
              : project.codexPrepared
                ? "在 Codex 中选择下方目录继续工作。账号与工具需在 Codex 中另行配置。"
                : "把工作说明放进独立目录，随后在 Codex 中选择这个目录开展工作。"}
          </p>
          <Button
            disabled={disabled || dirty || context.state !== "materialized"}
            onClick={() =>
              void run(
                () => ports.projects.prepareCodex(request),
                "Codex 工作目录已生成",
              )
            }
          >
            {project.codexPrepared ? "重新生成工作目录" : "生成 Codex 工作目录"}
          </Button>
          {context.directory && (
            <CopyablePath
              value={context.directory}
              label={project.codexPrepared ? "工作目录" : "资料目录"}
            />
          )}
          {context.codexInstructions && (
            <details>
              <summary>查看使用说明</summary>
              <pre className="fy-projects-instructions">
                {context.codexInstructions}
              </pre>
            </details>
          )}
        </section>
        <details className="fy-projects-settings">
          <summary>项目设置</summary>
          <label>
            项目名称
            <input
              className="fy-control-input"
              value={name}
              maxLength={160}
              disabled={disabled}
              onChange={(e) => setName(e.target.value)}
            />
          </label>
          <div className="fy-projects-actions">
            <Button
              disabled={disabled || name === project.name || !name.trim()}
              onClick={() =>
                void run(async () => {
                  await ports.projects.update(request, name, false);
                  setName(null);
                }, "项目名称已保存")
              }
            >
              保存名称
            </Button>
            <Button
              disabled={disabled || dirty}
              dialogOriginRef={archiveOrigin}
              onClick={() => setArchiveOpen(true)}
            >
              归档项目
            </Button>
          </div>
        </details>
      </FeatureTabPanel>
      <FeatureTabPanel
        tabsId={tabsId}
        value="delivery"
        active={view === "delivery"}
        layout="workspace"
        className="fy-projects-panel"
      >
        <PersistentSurface active={view === "delivery"}>
          {DeliveryKitPanel && visitedViews.has("delivery") && (
            <DeliveryKitPanel {...panelProps} />
          )}
        </PersistentSurface>
      </FeatureTabPanel>
      <FeatureTabPanel
        tabsId={tabsId}
        value="verification"
        active={view === "verification"}
        layout="workspace"
        className="fy-projects-panel"
      >
        <PersistentSurface active={view === "verification"}>
          {VerificationPanel && visitedViews.has("verification") && (
            <VerificationPanel {...panelProps} />
          )}
        </PersistentSurface>
      </FeatureTabPanel>
      <ConfirmDialog
        open={visible && recoveryOpen}
        title="重新建立工作说明？"
        description="将以上方内容建立新版本，旧文件保留。请先复制需要保留的外部修改。"
        pending={busy}
        originRef={recoveryOrigin}
        onCancel={() => setRecoveryOpen(false)}
        onConfirm={() => {
          setRecoveryOpen(false);
          void run(async () => {
            await ports.projects.writeContext(request, text, true);
            setText(null);
          }, "工作说明已重新建立");
        }}
      />
      <ConfirmDialog
        open={visible && reloadOpen}
        title="重新载入项目？"
        description="未保存的草稿会被丢弃，请先复制需要保留的内容。"
        pending={busy}
        originRef={reloadOrigin}
        onCancel={() => setReloadOpen(false)}
        onConfirm={() => {
          setReloadOpen(false);
          setName(null);
          setText(null);
          setError("");
          void onChanged();
        }}
      />
      <ConfirmDialog
        open={visible && blocker.state === "blocked"}
        title="放弃未保存的修改？"
        description="继续后会离开当前项目，未保存的工作说明或名称将丢失。"
        pending={busy}
        originRef={navigationOrigin}
        onCancel={() => blocker.state === "blocked" && blocker.reset()}
        onConfirm={() => blocker.state === "blocked" && blocker.proceed()}
      />
      <ConfirmDialog
        open={visible && archiveOpen}
        title="归档此项目？"
        description="归档后不能继续修改。资料和文件会保留，不会删除账号或全局资源。"
        pending={busy}
        originRef={archiveOrigin}
        onCancel={() => setArchiveOpen(false)}
        onConfirm={() => {
          setArchiveOpen(false);
          void run(
            () => ports.projects.update(request, project.name, true),
            "项目已归档",
          );
        }}
      />
    </div>
  );
}

function DependencySummary({
  snapshot,
}: {
  snapshot: ProjectDependencySnapshot;
}) {
  const missing = snapshot.resources.filter(
    (r) => r.state === "missing",
  ).length;
  return (
    <div className="fy-projects-observation">
      <p>
        {snapshot.contextState === "materialized"
          ? "工作说明文件可读取"
          : snapshot.contextState === "not_created"
            ? "尚未保存工作说明"
            : "工作说明文件不可用"}
      </p>
      <p>
        {missing
          ? `${missing} 项资源已不存在，请重新选择。`
          : `已关联 ${snapshot.resources.length} 项资源。`}
      </p>
      {snapshot.credentials.map((c) => (
        <p key={c.binding.credentialId}>
          {c.binding.consumer}：{states[c.state]}
        </p>
      ))}
      <p className="fy-projects-hint">
        检查时间 {new Date(snapshot.observedAt).toLocaleString()}
      </p>
    </div>
  );
}
