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
import { Button } from "../../shared/ui/Button";
import { ConfirmDialog } from "../../shared/ui/Dialog";
import { EmptyState, InlineNotice } from "../../shared/ui/primitives";
import {
  usePrimaryBlocker,
  usePrimaryBlockerOrigin,
} from "../../shared/ui/PrimaryBlocker";
import { usePersistentSearchParams } from "../../shared/ui/usePersistentSearchParams";
import { usePersistentVisibility } from "../../shared/ui/PersistentSurface";
import "./projects.css";

export interface ProjectPanelProps {
  projectId: string;
  projectRevision: number;
  archived: boolean;
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
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [showArchived, setShowArchived] = useState(false);
  const selected = searchParams.get("project");
  const selectedProject = projects.data?.find((p) => p.projectId === selected);
  const context = useQuery({
    queryKey: featureKeys.projectContext(selected),
    queryFn: () => ports.projects.getContext(selected!),
    enabled: visible && !!selected,
    staleTime: Infinity,
  });
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
  const createCustomer = async () => {
    if (busy || !visible) return;
    setBusy(true);
    setError("");
    try {
      const c = await ports.projects.createCustomer(customerName);
      setCustomerName("");
      setCustomerId(c.customerId);
      await refresh();
    } catch (e) {
      setError(projectError(e));
    } finally {
      setBusy(false);
    }
  };
  const createProject = async () => {
    if (busy || !visible) return;
    setBusy(true);
    setError("");
    try {
      const p = await ports.projects.create(customerId, newProjectName);
      setNewProjectName("");
      await refresh();
      setSearchParams({ project: p.projectId });
    } catch (e) {
      setError(projectError(e));
    } finally {
      setBusy(false);
    }
  };
  return (
    <div
      className="fy-feature-page fy-projects-page"
      aria-label="客户项目"
      data-testid="projects-page"
    >
      <header className="fy-projects-header">
        <h1>客户项目</h1>
        <span>按客户整理资料与工具</span>
      </header>
      <div className="fy-projects-layout">
        <aside className="fy-projects-rail" aria-label="项目列表">
          <details>
            <summary>新建客户与项目</summary>
            <form
              onSubmit={(e) => {
                e.preventDefault();
                void createCustomer();
              }}
            >
              <label>
                客户名称
                <input
                  className="fy-control-input"
                  value={customerName}
                  maxLength={160}
                  onChange={(e) => setCustomerName(e.target.value)}
                />
              </label>
              <Button type="submit" disabled={busy || !customerName.trim()}>
                添加客户
              </Button>
            </form>
            <form
              onSubmit={(e) => {
                e.preventDefault();
                void createProject();
              }}
            >
              <label>
                所属客户
                <select
                  className="fy-control-input"
                  value={customerId}
                  onChange={(e) => setCustomerId(e.target.value)}
                >
                  <option value="">请选择客户</option>
                  {customers.data
                    ?.filter((c) => !c.archived)
                    .map((c) => (
                      <option key={c.customerId} value={c.customerId}>
                        {c.name}
                      </option>
                    ))}
                </select>
              </label>
              <label>
                项目名称
                <input
                  className="fy-control-input"
                  value={newProjectName}
                  maxLength={160}
                  onChange={(e) => setNewProjectName(e.target.value)}
                />
              </label>
              <Button
                type="submit"
                disabled={busy || !customerId || !newProjectName.trim()}
              >
                创建项目
              </Button>
            </form>
          </details>
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
            <p>先添加客户，再创建第一个项目。</p>
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
              title="选择一个项目"
              description="项目资料独立保存，选择项目不会更改正在使用的软件配置。"
            />
          ) : context.isError ? (
            <EmptyState
              title="无法读取项目工作说明"
              description="项目文件可能被移动或替换。原有项目记录已保留，请检查目录后重试。"
            >
              <Button onClick={() => void context.refetch()}>重试</Button>
            </EmptyState>
          ) : !context.data ||
            context.data.projectId !== selectedProject.projectId ||
            context.data.projectRevision !== selectedProject.projectRevision ? (
            <p role="status">正在读取工作说明…</p>
          ) : (
            <ProjectEditor
              key={`${selectedProject.projectId}:${selectedProject.projectRevision}`}
              project={selectedProject}
              context={context.data}
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
  context,
  onChanged,
  DeliveryKitPanel,
  VerificationPanel,
}: ProjectsPageProps & {
  project: Project;
  context: ProjectContext;
  onChanged: () => Promise<void>;
}) {
  const { ports } = useFeatures();
  const visible = usePersistentVisibility();
  const [name, setName] = useState(project.name);
  const [text, setText] = useState(context.content);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [status, setStatus] = useState("");
  const [resource, setResource] = useState("");
  const [credential, setCredential] = useState("");
  const [model, setModel] = useState("");
  const [archiveOpen, setArchiveOpen] = useState(false);
  const [reloadOpen, setReloadOpen] = useState(false);
  const reloadOrigin = useRef<HTMLElement | null>(null);
  const archiveOrigin = useRef<HTMLElement | null>(null);
  const dirty = name !== project.name || text !== context.content;
  const blocker = usePrimaryBlocker(dirty || busy);
  const navigationOrigin = usePrimaryBlockerOrigin();
  const resources = useQuery({
    queryKey: featureKeys.projectResourceOptions,
    queryFn: () => ports.projects.resourceOptions(),
    enabled: visible,
  });
  const credentials = useQuery({
    queryKey: featureKeys.projectCredentialOptions,
    queryFn: () => ports.projects.credentialOptions(),
    enabled: visible,
  });
  const selectedResource = resources.data?.find(
    (r) => resourceKey(r) === resource,
  );
  const dependencies = useQuery({
    queryKey: featureKeys.projectDependencies(project.projectId),
    queryFn: () => ports.projects.dependencySnapshot(project.projectId),
    enabled: visible,
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
    disabled: disabled || dirty,
    onProjectChanged: onChanged,
  };
  return (
    <>
      <div className="fy-projects-title">
        <h2>{project.name}</h2>
        <Button
          disabled={disabled || dirty}
          dialogOriginRef={archiveOrigin}
          onClick={() => setArchiveOpen(true)}
        >
          归档项目
        </Button>
      </div>
      {project.archived && (
        <InlineNotice>
          项目已归档。工作说明、文件和资源引用仍保留。
        </InlineNotice>
      )}
      {error && <InlineNotice tone="error">{error}</InlineNotice>}
      {status && <p role="status">{status}</p>}
      {(error || dirty) && (
        <Button
          disabled={busy || !visible}
          dialogOriginRef={reloadOrigin}
          onClick={() => setReloadOpen(true)}
        >
          重新载入项目
        </Button>
      )}
      <section>
        <h3>项目资料</h3>
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
        <Button
          disabled={
            disabled ||
            name === project.name ||
            !name.trim() ||
            text !== context.content
          }
          onClick={() =>
            void run(
              () => ports.projects.update(request, name, false),
              "项目名称已保存",
            )
          }
        >
          保存名称
        </Button>
        <label>
          工作说明
          <textarea
            className="fy-control-input fy-projects-context"
            value={text}
            disabled={disabled}
            onChange={(e) => setText(e.target.value)}
            placeholder="记录项目目标、授权资料、口径与待确认事项"
          />
        </label>
        <p className="fy-projects-hint">
          保存会为此项目生成新的工作说明文件，保留先前版本。不会写入其他软件的全局记忆。
        </p>
        <Button
          className="fy-control-button-primary"
          disabled={
            disabled ||
            name !== project.name ||
            (!dirty && context.state === "materialized")
          }
          onClick={() =>
            void run(
              () => ports.projects.writeContext(request, text),
              "工作说明已保存",
            )
          }
        >
          {busy ? "保存中…" : "保存工作说明"}
        </Button>
        {context.directory && (
          <p className="fy-projects-path">
            文件目录 <code>{context.directory}</code>
          </p>
        )}
      </section>
      <section>
        <h3>项目资源</h3>
        <p className="fy-projects-hint">
          引用现有资源，不会自动在软件中启用。资源版本暂无法完整确认，使用前请核对。
          全局记忆暂不支持项目绑定，请将本项目说明保存在上方资料中。
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
        <h3>账号引用</h3>
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
              </div>
              <Button
                disabled={disabled || dirty}
                onClick={() =>
                  void run(
                    () =>
                      ports.projects.removeCredential(request, c.credentialId),
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
        <h3>Codex 工作目录</h3>
        <p>生成独立的工作目录、只读配置和检查说明。账号和工具不会自动带入。</p>
        <Button
          disabled={disabled || dirty || context.state !== "materialized"}
          onClick={() =>
            void run(
              () => ports.projects.prepareCodex(request),
              "Codex 工作目录已生成",
            )
          }
        >
          生成 Codex 工作目录
        </Button>
        {context.codexInstructions && (
          <details>
            <summary>查看使用说明</summary>
            <pre className="fy-projects-instructions">
              {context.codexInstructions}
            </pre>
          </details>
        )}
      </section>
      <section>
        <h3>使用前检查</h3>
        {dependencies.data ? (
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
        <p>尚未启动项目 Agent。外部软件的认证、工具调用和权限需要另外确认。</p>
        <Button disabled>启动项目 Agent（暂不可用）</Button>
      </section>
      {DeliveryKitPanel && (
        <section aria-label="交付包">
          <DeliveryKitPanel {...panelProps} />
        </section>
      )}
      {VerificationPanel && (
        <section aria-label="验证与交接">
          <VerificationPanel {...panelProps} />
        </section>
      )}
      <ConfirmDialog
        open={visible && reloadOpen}
        title="重新载入项目？"
        description="未保存的草稿会被丢弃，请先复制需要保留的内容。"
        pending={busy}
        originRef={reloadOrigin}
        onCancel={() => setReloadOpen(false)}
        onConfirm={() => {
          setReloadOpen(false);
          setName(project.name);
          setText(context.content);
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
    </>
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
