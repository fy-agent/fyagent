import { useQueryClient } from "@tanstack/react-query";
import { useEffect, useMemo, useRef, useState } from "react";

import { getSkillTargetIcon } from "../../shared/assets/apps";
import {
  buildSkillSearchText,
  convergeSelection,
  errorMessage,
  isDiscoverableInstalled,
  runSequentialBulk,
  skillInstallPath,
  supportedFoundIn,
  UserFacingError,
} from "../../shared/features/helpers";
import { useFeatures } from "../../shared/features/provider";
import { useWideFeatureLayout } from "../../shared/features/responsive";
import {
  featureKeys,
  useFeatureSettings,
  useInstalledSkills,
  useSkillBackups,
  useSkillDiscoveryPage,
  useSkillRepos,
  useSkillsShSearch,
  useSkillUpdates,
  useUnmanagedSkills,
} from "../../shared/features/queries";
import {
  createSkillAssignments,
  SKILL_DISCOVERY_PAGE_SIZE,
  SKILL_TARGETS,
  type DiscoverableSkill,
  type InstalledSkill,
  type SkillBackupEntry,
  type SkillDiscoveryStatus,
  type SkillRepo,
  type SkillTargetId,
} from "../../shared/features/types";
import {
  Badge,
  Button,
  Checkbox,
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
import { FeaturePagination } from "../../shared/ui/FeaturePagination";
import { FeatureSearch } from "../../shared/ui/FeatureSearch";
import { FeatureTabs } from "../../shared/ui/FeatureTabs";
import { SplitPanes } from "../../shared/ui/split";

import "./page.css";

type SkillsTab = "installed" | "discovery";
type DialogName =
  | "more"
  | "unmanaged"
  | "backups"
  | "repos"
  | "settings"
  | null;

function formatSkillTimestamp(value: number): string {
  if (!Number.isFinite(value) || value <= 0) return "未知";
  return new Date(value * 1000).toLocaleString();
}

function githubRepoUrl(owner: string, name: string): string | null {
  if (!/^[\w.-]+$/.test(owner) || !/^[\w.-]+$/.test(name)) return null;
  return `https://github.com/${owner}/${name}`;
}

function skillRepoKey(skill: { repoOwner: string; repoName: string }): string {
  return `${skill.repoOwner}/${skill.repoName}`;
}

function skillDirectoryNote(skill: {
  name: string;
  directory: string;
}): string {
  return skill.directory && skill.directory !== skill.name
    ? skill.directory
    : "";
}

function skillCardBody(skill: DiscoverableSkill, hideRepo: boolean): string {
  const description = skill.description.trim();
  if (description) return description;
  const directoryNote = skillDirectoryNote(skill);
  if (directoryNote) return directoryNote;
  return hideRepo ? "" : `来自 ${skillRepoKey(skill)}`;
}

function skillDetailBody(skill: DiscoverableSkill): string {
  return skillCardBody(skill, false) || "暂无说明";
}

function skillDetailMeta(
  skill: DiscoverableSkill & { installs?: number },
): string {
  return [
    skillRepoKey(skill),
    typeof skill.installs === "number"
      ? `${skill.installs.toLocaleString()} 次安装`
      : null,
  ]
    .filter(Boolean)
    .join(" · ");
}

function skillDocsAction(
  readmeUrl?: string,
  repoUrl?: string | null,
): { url: string; label: "说明" | "仓库" } | null {
  if (readmeUrl) {
    return {
      url: readmeUrl,
      label: /SKILL\.md|\/blob\//i.test(readmeUrl) ? "说明" : "仓库",
    };
  }
  return repoUrl ? { url: repoUrl, label: "仓库" } : null;
}

function assignedSkillTargets(skill: InstalledSkill) {
  return SKILL_TARGETS.filter((target) => Boolean(skill.apps[target.id]));
}

const INSTALLED_SPLIT_LABELS = ["调整列表与详情的宽度", "调整详情与分配的宽度"];

const invalidations = [
  featureKeys.skills,
  featureKeys.skillBackups,
  featureKeys.skillDiscovery,
  featureKeys.skillRepos,
  featureKeys.skillUnmanaged,
];

function Detail({
  skill,
  update,
  busy,
  onToggle,
  onUpdate,
  onUninstall,
  showAssignment,
}: {
  skill: InstalledSkill;
  update?: { remoteHash: string };
  busy: boolean;
  onToggle: (app: SkillTargetId, enabled: boolean) => void;
  onUpdate: () => void;
  onUninstall: () => void;
  showAssignment: boolean;
}) {
  const assigned = assignedSkillTargets(skill);
  const repo =
    skill.repoOwner && skill.repoName
      ? `${skill.repoOwner}/${skill.repoName}`
      : null;
  const repoUrl =
    skill.repoOwner && skill.repoName
      ? githubRepoUrl(skill.repoOwner, skill.repoName)
      : null;
  const sourceLabel = repo ? "GitHub 仓库" : "本地导入";
  const description = skill.description?.trim() || "暂无说明";

  return (
    <section
      className="fy-feature-panel fy-feature-detail fy-feature-detail-scroll"
      aria-label="Skill 详情"
    >
      <div className="fy-feature-detail-header">
        <div className="fy-feature-detail-title">
          <h2>{skill.name}</h2>
          {update && <Badge tone="warning">有更新</Badge>}
          <Badge tone={repo ? "accent" : "neutral"}>{sourceLabel}</Badge>
        </div>
        <p className="fy-feature-intro">{description}</p>
        <div className="fy-feature-actions">
          {update && (
            <Button
              className="fy-control-button-primary"
              disabled={busy}
              onClick={onUpdate}
            >
              更新
            </Button>
          )}
          <Button
            className="fy-control-button-danger"
            disabled={busy}
            onClick={onUninstall}
          >
            卸载
          </Button>
        </div>
      </div>
      <div className="fy-feature-info-grid">
        <section className="fy-feature-info-card" aria-label="下载来源">
          <h3>下载来源</h3>
          <p className="fy-feature-info-lead">
            {repo
              ? "此 Skill 来自 GitHub 仓库，安装后保存在本地目录，并可按仓库检查更新。"
              : "此 Skill 来自本地导入或 ZIP 安装，当前没有绑定远程仓库。"}
          </p>
          <dl className="fy-feature-definition">
            <dt>来源类型</dt>
            <dd>{sourceLabel}</dd>
            {repo && (
              <>
                <dt>仓库</dt>
                <dd>{repo}</dd>
              </>
            )}
            {skill.repoBranch && (
              <>
                <dt>分支</dt>
                <dd>{skill.repoBranch}</dd>
              </>
            )}
            <dt>安装目录</dt>
            <dd>
              <CopyablePath value={skillInstallPath(skill)} />
            </dd>
          </dl>
          {(repoUrl || skill.readmeUrl) && (
            <div className="fy-feature-actions">
              {repoUrl && (
                <ExternalLinkButton url={repoUrl}>打开仓库</ExternalLinkButton>
              )}
              {skill.readmeUrl && (
                <ExternalLinkButton url={skill.readmeUrl}>
                  查看说明
                </ExternalLinkButton>
              )}
            </div>
          )}
        </section>
        <section className="fy-feature-info-card" aria-label="当前分配">
          <h3>当前分配</h3>
          <p className="fy-feature-info-lead">
            {assigned.length > 0
              ? `已启用 ${assigned.length} 个应用。需要增减时，使用应用分配开关。`
              : "尚未分配到任何应用。启用后，对应软件才能加载此 Skill。"}
          </p>
          {assigned.length > 0 && (
            <ul className="fy-feature-app-chips">
              {assigned.map((app) => (
                <li key={app.id} className="fy-feature-app-chip">
                  <img
                    className="fy-feature-assignment-icon"
                    src={getSkillTargetIcon(app.id)}
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
            {skill.installedAt > 0 && (
              <>
                <dt>安装时间</dt>
                <dd>{formatSkillTimestamp(skill.installedAt)}</dd>
              </>
            )}
            <dt>最近更新</dt>
            <dd>{formatSkillTimestamp(skill.updatedAt)}</dd>
          </dl>
        </section>
      </div>
      {showAssignment && (
        <div className="fy-feature-inline-assignment">
          <AssignmentPanel
            apps={skill.apps}
            disabled={busy}
            labelSuffix="Skill 分配"
            onToggle={onToggle}
            targets={SKILL_TARGETS}
          />
        </div>
      )}
    </section>
  );
}

export function SkillsPage() {
  const queryClient = useQueryClient();
  const { ports, installTarget, setInstallTarget, notify } = useFeatures();
  const wideLayout = useWideFeatureLayout();
  const [tab, setTab] = useState<SkillsTab>("installed");
  const [search, setSearch] = useState("");
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [dialog, setDialog] = useState<DialogName>(null);
  const [confirm, setConfirm] = useState<
    | { kind: "uninstall"; skill: InstalledSkill }
    | { kind: "repo"; repo: SkillRepo }
    | { kind: "backup"; backup: SkillBackupEntry }
    | null
  >(null);
  const [busy, setBusy] = useState(false);
  const [progress, setProgress] = useState<{
    done: number;
    total: number;
  } | null>(null);
  const writeLock = useRef(false);
  const installedQuery = useInstalledSkills();
  const updatesQuery = useSkillUpdates(false);
  const installed = useMemo(
    () => installedQuery.data ?? [],
    [installedQuery.data],
  );
  const updates = useMemo(() => updatesQuery.data ?? [], [updatesQuery.data]);
  const updatesById = useMemo(
    () => new Map(updates.map((item) => [item.id, item])),
    [updates],
  );
  const filtered = useMemo(() => {
    const query = search.trim().toLocaleLowerCase();
    return query
      ? installed.filter((skill) => buildSkillSearchText(skill).includes(query))
      : installed;
  }, [installed, search]);
  const convergedId = convergeSelection(filtered, selectedId);
  const selected = filtered.find((skill) => skill.id === convergedId) ?? null;

  const refreshAll = async () => {
    await Promise.all([
      ...invalidations.map((queryKey) =>
        queryClient.invalidateQueries({ queryKey }),
      ),
      ...(updatesQuery.data === undefined ? [] : [updatesQuery.refetch()]),
    ]);
  };
  const write = async (title: string, operation: () => Promise<void>) => {
    if (writeLock.current) return;
    writeLock.current = true;
    setBusy(true);
    try {
      await operation();
      notify({ tone: "success", title });
    } catch (error) {
      notify({
        tone: "error",
        title: `${title}失败`,
        description: errorMessage(error),
      });
    } finally {
      await refreshAll();
      setProgress(null);
      setBusy(false);
      writeLock.current = false;
    }
  };
  const toggle = (
    skill: InstalledSkill,
    app: SkillTargetId,
    enabled: boolean,
  ) =>
    write("分配已更新", async () => {
      await ports.skills.toggleApp(skill.id, app, enabled);
    });
  const checkUpdates = async () => {
    try {
      const result = await updatesQuery.refetch({ throwOnError: true });
      notify({
        tone: "info",
        title: result.data?.length
          ? `发现 ${result.data.length} 个更新`
          : "所有 Skills 均为最新",
      });
    } catch (error) {
      notify({
        tone: "error",
        title: "检查更新失败",
        description: errorMessage(error),
      });
    }
  };
  const updateAll = () =>
    write("批量更新完成", async () => {
      const result = await runSequentialBulk(
        updates.map((item) => item.id),
        ports.skills.update,
        (done, total) => setProgress({ done, total }),
      );
      if (result.failures.length)
        throw new UserFacingError(
          `${result.failures.length} 项失败，${result.successes.length} 项成功`,
        );
    });
  const bulkAssign = (app: SkillTargetId, enabled: boolean) =>
    write("批量分配完成", async () => {
      const ids = installed
        .filter((skill) => Boolean(skill.apps[app]) !== enabled)
        .map((skill) => skill.id);
      const result = await runSequentialBulk(
        ids,
        (id) => ports.skills.toggleApp(id, app, enabled),
        (done, total) => setProgress({ done, total }),
      );
      if (result.failures.length)
        throw new UserFacingError(
          `${result.failures.length} 项失败，${result.successes.length} 项成功`,
        );
    });
  const pickAndInstallZip = async () => {
    if (writeLock.current) return;
    writeLock.current = true;
    setBusy(true);
    let path: string | null;
    try {
      path = await ports.skills.pickZip();
    } catch (error) {
      notify({
        tone: "error",
        title: "ZIP 选择失败",
        description: errorMessage(error),
      });
      return;
    } finally {
      setBusy(false);
      writeLock.current = false;
    }
    if (!path) return;
    await write("ZIP 安装完成", async () => {
      await ports.skills.installFromZip(path, installTarget);
    });
  };

  return (
    <div
      className={`fy-feature-page fy-split-page fy-skills-page${tab === "discovery" ? " fy-skills-page-discovery" : ""}`}
      data-testid="skills-page"
    >
      <header className="fy-feature-header">
        <div className="fy-feature-heading">
          <h1>Skills</h1>
          <p>
            {tab === "discovery"
              ? "从仓库或 skills.sh 浏览可安装的 Skills。"
              : "安装、更新并分配 Skills 到所选应用。"}
          </p>
        </div>
        <div className="fy-feature-actions">
          {tab === "discovery" && (
            <>
              <p className="fy-feature-description">
                将安装到{" "}
                {SKILL_TARGETS.find((app) => app.id === installTarget)?.label ??
                  "Claude Code"}
              </p>
              <FeatureTabs
                id="skills-install-target"
                className="fy-feature-target-tabs"
                label="安装目标"
                value={installTarget}
                onChange={setInstallTarget}
                options={SKILL_TARGETS.map((app) => ({
                  id: app.id,
                  label: (
                    <>
                      <img
                        className="fy-feature-assignment-icon"
                        src={getSkillTargetIcon(app.id)}
                        alt=""
                        aria-hidden="true"
                      />
                      <span>{app.label}</span>
                    </>
                  ),
                }))}
              />
              <Button onClick={() => setDialog("repos")}>管理仓库</Button>
            </>
          )}
          {tab === "installed" && (
            <>
              <Button
                onClick={checkUpdates}
                disabled={busy || updatesQuery.isFetching}
              >
                检查更新
              </Button>
              {updates.length > 0 && (
                <Button
                  className="fy-control-button-primary"
                  onClick={updateAll}
                  disabled={busy}
                >
                  更新全部 · {updates.length}
                </Button>
              )}
              <div className="fy-feature-menu">
                <Button
                  aria-expanded={dialog === "more"}
                  onClick={() => setDialog(dialog === "more" ? null : "more")}
                >
                  更多
                </Button>
                {dialog === "more" && (
                  <div className="fy-feature-menu-popover">
                    <Button onClick={() => setDialog("unmanaged")}>
                      导入本地 Skill
                    </Button>
                    <Button
                      disabled={busy}
                      onClick={() => void pickAndInstallZip()}
                    >
                      从 ZIP 安装
                    </Button>
                    <Button onClick={() => setDialog("backups")}>
                      备份恢复
                    </Button>
                    <Button onClick={() => setDialog("settings")}>
                      Skill 设置
                    </Button>
                  </div>
                )}
              </div>
            </>
          )}
        </div>
      </header>
      <FeatureTabs
        id="skills-view-tabs"
        label="Skills 视图"
        value={tab}
        onChange={setTab}
        options={[
          { id: "installed", label: "已安装" },
          { id: "discovery", label: "发现" },
        ]}
      />
      {progress && (
        <>
          <div
            className="fy-feature-progress"
            aria-label={`进度 ${progress.done}/${progress.total}`}
          >
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
      {tab === "installed" ? (
        <>
          {installedQuery.error && installedQuery.data !== undefined && (
            <InlineNotice tone="error">
              刷新失败，正在显示上一次成功加载的数据：
              {errorMessage(installedQuery.error)}
            </InlineNotice>
          )}
          {installedQuery.isLoading ? (
            <EmptyState
              title="正在加载 Skills"
              description="正在读取已安装的 Skills"
            >
              <Spinner />
            </EmptyState>
          ) : installedQuery.error && installedQuery.data === undefined ? (
            <EmptyState
              title="无法加载 Skills"
              description={errorMessage(installedQuery.error)}
              actions={
                <Button onClick={() => void installedQuery.refetch()}>
                  重试
                </Button>
              }
            />
          ) : installed.length === 0 ? (
            <EmptyState
              title="还没有安装 Skill"
              description="从发现页、ZIP 或已安装的应用中导入第一个 Skill。"
              actions={
                <Button
                  className="fy-control-button-primary"
                  onClick={() => setTab("discovery")}
                >
                  浏览发现
                </Button>
              }
            />
          ) : (
            <div className="fy-feature-workspace">
              <div className="fy-feature-toolbar">
                <FeatureSearch
                  ariaLabel="搜索已安装 Skills"
                  placeholder="搜索名称、说明或仓库"
                  value={search}
                  onValueChange={setSearch}
                />
              </div>
              {filtered.length === 0 ? (
                <EmptyState
                  title="没有匹配的 Skill"
                  description="请调整搜索关键词"
                />
              ) : (
                <SplitPanes separatorLabels={INSTALLED_SPLIT_LABELS}>
                  <section
                    className="fy-feature-panel fy-feature-list-panel"
                    aria-label="已安装 Skills 列表"
                  >
                    <h2>已安装 · {installed.length}</h2>
                    <FeatureList id="skills-installed-list">
                      {filtered.map((skill) => (
                        <FeatureListItem
                          key={skill.id}
                          selected={skill.id === selected?.id}
                          title={skill.name}
                          onSelect={() => setSelectedId(skill.id)}
                        >
                          <span>{skill.description || "暂无说明"}</span>
                        </FeatureListItem>
                      ))}
                    </FeatureList>
                  </section>
                  {selected && (
                    <Detail
                      key={selected.id}
                      skill={selected}
                      update={updatesById.get(selected.id)}
                      busy={busy}
                      onToggle={(app, enabled) =>
                        toggle(selected, app, enabled)
                      }
                      onUpdate={() =>
                        void write("Skill 更新完成", async () => {
                          await ports.skills.update(selected.id);
                        })
                      }
                      onUninstall={() =>
                        setConfirm({ kind: "uninstall", skill: selected })
                      }
                      showAssignment={!wideLayout}
                    />
                  )}
                  {selected && wideLayout && (
                    <section className="fy-feature-panel fy-feature-assign-scroll">
                      <AssignmentPanel
                        apps={selected.apps}
                        disabled={busy}
                        labelSuffix="Skill 分配"
                        onToggle={(app, enabled) =>
                          toggle(selected, app, enabled)
                        }
                        targets={SKILL_TARGETS}
                      />
                      <hr />
                      <h3>全量分配</h3>
                      {SKILL_TARGETS.map((app) => (
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
        </>
      ) : (
        <Discovery
          installTarget={installTarget}
          busy={busy}
          setDialog={setDialog}
          onInstall={(skill) =>
            write(`${skill.name} 已安装`, async () => {
              await ports.skills.install(skill, installTarget);
            })
          }
        />
      )}
      <AuxiliaryDialogs
        key={dialog ?? "closed"}
        name={dialog}
        close={() => setDialog(null)}
        installTarget={installTarget}
        busy={busy}
        write={write}
        setConfirm={setConfirm}
      />
      <ConfirmDialog
        open={confirm !== null}
        title={
          confirm?.kind === "uninstall"
            ? `卸载 ${confirm.skill.name}`
            : confirm?.kind === "repo"
              ? `移除仓库 ${confirm.repo.owner}/${confirm.repo.name}`
              : confirm
                ? `删除 ${confirm.backup.skill.name} 的备份`
                : "确认操作"
        }
        description={
          confirm?.kind === "uninstall"
            ? "将从管理列表及已启用的应用中移除，并创建可恢复备份。"
            : confirm?.kind === "repo"
              ? "仅移除仓库来源，不会卸载已安装 Skills。"
              : "删除后无法从该备份恢复。"
        }
        pending={busy}
        onCancel={() => setConfirm(null)}
        onConfirm={async () => {
          const action = confirm;
          if (!action) return;
          if (action.kind === "uninstall")
            await write("Skill 已卸载", async () => {
              await ports.skills.uninstall(action.skill.id);
            });
          else if (action.kind === "repo")
            await write("仓库已移除", async () => {
              await ports.skills.removeRepo(
                action.repo.owner,
                action.repo.name,
              );
              await queryClient.invalidateQueries({
                queryKey: featureKeys.skillRepos,
              });
            });
          else
            await write("备份已删除", async () => {
              await ports.skills.deleteBackup(action.backup.backupId);
            });
          setConfirm(null);
        }}
      />
    </div>
  );
}

function groupSkillsByRepo<T extends { repoOwner: string; repoName: string }>(
  skills: readonly T[],
): Array<[string, T[]]> {
  const groups = new Map<string, T[]>();
  for (const skill of skills) {
    const key = skillRepoKey(skill);
    const current = groups.get(key) ?? [];
    current.push(skill);
    groups.set(key, current);
  }
  return [...groups.entries()];
}

function DiscoveryCard({
  busy,
  hideRepo,
  installLabel,
  isInstalled,
  skill,
  onInstall,
  onOpenDetail,
}: {
  busy: boolean;
  hideRepo: boolean;
  installLabel: string;
  isInstalled: boolean;
  skill: DiscoverableSkill & { installs?: number };
  onInstall: (skill: DiscoverableSkill) => Promise<void>;
  onOpenDetail: (skill: DiscoverableSkill & { installs?: number }) => void;
}) {
  const repo = skillRepoKey(skill);
  const repoUrl = githubRepoUrl(skill.repoOwner, skill.repoName);
  const body = skillCardBody(skill, hideRepo);
  const docs = skillDocsAction(skill.readmeUrl, repoUrl);
  const meta = [
    hideRepo ? null : repo,
    typeof skill.installs === "number"
      ? `${skill.installs.toLocaleString()} 次安装`
      : null,
  ]
    .filter(Boolean)
    .join(" · ");

  return (
    <article className="fy-feature-card">
      <header className="fy-feature-card-meta">
        <h3>{skill.name}</h3>
        {isInstalled && <Badge tone="accent">已安装</Badge>}
      </header>
      {body ? <p className="fy-feature-card-body">{body}</p> : null}
      {meta ? <p className="fy-feature-card-note">{meta}</p> : null}
      <footer>
        <Button
          className="fy-control-button-primary"
          disabled={busy || isInstalled}
          onClick={() => void onInstall(skill)}
        >
          {isInstalled ? "已安装" : `安装到 ${installLabel}`}
        </Button>
        <Button onClick={() => onOpenDetail(skill)}>详情</Button>
        {docs ? (
          <ExternalLinkButton url={docs.url}>{docs.label}</ExternalLinkButton>
        ) : null}
      </footer>
    </article>
  );
}

function Discovery({
  installTarget,
  busy,
  setDialog,
  onInstall,
}: {
  installTarget: SkillTargetId;
  busy: boolean;
  setDialog: (name: DialogName) => void;
  onInstall: (skill: DiscoverableSkill) => Promise<void>;
}) {
  const [source, setSource] = useState<"repos" | "skillssh">("repos");
  const [search, setSearch] = useState("");
  const [debouncedSearch, setDebouncedSearch] = useState("");
  const [repoFilter, setRepoFilter] = useState("all");
  const [status, setStatus] = useState<SkillDiscoveryStatus>("all");
  const [skillsShInput, setSkillsShInput] = useState("");
  const [skillsShQuery, setSkillsShQuery] = useState("");
  const [page, setPage] = useState(1);
  const [detailSkill, setDetailSkill] = useState<
    (DiscoverableSkill & { installs?: number }) | null
  >(null);
  const resultsTop = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const timer = window.setTimeout(() => {
      setDebouncedSearch(search);
    }, 300);
    return () => window.clearTimeout(timer);
  }, [search]);
  const discoveryQuery = debouncedSearch.trim();
  const repos = useSkillRepos();
  const installed = useInstalledSkills();
  const enabledRepos = useMemo(
    () => (repos.data ?? []).filter((repo) => repo.enabled),
    [repos.data],
  );
  const repoOptions = useMemo(
    () =>
      enabledRepos.map((repo) => ({
        id: `${repo.owner}/${repo.name}`,
        label: `${repo.owner}/${repo.name}`,
      })),
    [enabledRepos],
  );
  const activeRepoFilter =
    repoFilter !== "all" && repoOptions.some((repo) => repo.id === repoFilter)
      ? repoFilter
      : "all";
  const discovery = useSkillDiscoveryPage(
    discoveryQuery,
    activeRepoFilter === "all" ? undefined : activeRepoFilter,
    status,
    page,
    source === "repos",
  );
  const skillsSh = useSkillsShSearch(skillsShQuery, page);
  const installedItems = useMemo(() => installed.data ?? [], [installed.data]);
  const repoSkills = discovery.data?.skills ?? [];
  const repoTotalCount = discovery.data?.totalCount ?? repoSkills.length;
  const skillsShTotalCount = skillsSh.data?.totalCount ?? 0;
  const totalCount = source === "repos" ? repoTotalCount : skillsShTotalCount;
  const totalPages = Math.max(
    1,
    Math.ceil(totalCount / SKILL_DISCOVERY_PAGE_SIZE),
  );
  const skills =
    source === "repos"
      ? repoSkills
      : (skillsSh.data?.skills ?? []).map((skill) => ({
          ...skill,
          description: "",
        }));
  const installLabel =
    SKILL_TARGETS.find((app) => app.id === installTarget)?.label ??
    "Claude Code";
  const groupedSkills = groupSkillsByRepo(skills);
  const resultSummary =
    source === "repos"
      ? `${skills.length} / ${repoTotalCount} 个 Skill · 将安装到 ${installLabel}`
      : `skills.sh · ${skills.length} / ${skillsSh.data?.totalCount ?? skills.length} · 将安装到 ${installLabel}`;
  const goToPage = (next: number) => {
    setPage(next);
    resultsTop.current?.scrollIntoView?.({ block: "start" });
  };
  return (
    <section className="fy-feature-workspace" ref={resultsTop}>
      {source === "repos" ? (
        <div className="fy-feature-toolbar">
          <FeatureSearch
            ariaLabel="搜索仓库 Skills"
            placeholder="搜索 Skill 或仓库"
            value={search}
            onValueChange={(value) => {
              setSearch(value);
              setPage(1);
            }}
          />
        </div>
      ) : (
        <form
          className="fy-feature-toolbar"
          onSubmit={(event) => {
            event.preventDefault();
            const query = skillsShInput.trim();
            if (query.length >= 2) {
              setSkillsShQuery(query);
              setPage(1);
            }
          }}
        >
          <FeatureSearch
            ariaLabel="搜索 skills.sh"
            placeholder="至少输入 2 个字符"
            value={skillsShInput}
            onValueChange={setSkillsShInput}
          />
          <Button type="submit" disabled={skillsShInput.trim().length < 2}>
            搜索
          </Button>
        </form>
      )}
      <div className="fy-feature-toolbar">
        <FeatureTabs
          id="skills-discovery-source-tabs"
          label="发现来源"
          value={source}
          onChange={(value) => {
            setSource(value);
            setPage(1);
          }}
          options={[
            { id: "repos", label: "仓库" },
            { id: "skillssh", label: "skills.sh" },
          ]}
        />
        {source === "repos" && (
          <FeatureTabs
            id="skills-install-status"
            label="安装状态"
            value={status}
            onChange={(value) => {
              setStatus(value);
              setPage(1);
            }}
            options={[
              { id: "all", label: "全部状态" },
              { id: "uninstalled", label: "未安装" },
              { id: "installed", label: "已安装" },
            ]}
          />
        )}
      </div>
      {source === "repos" && repoOptions.length > 1 && (
        <FeatureTabs
          id="skills-repo-filter"
          label="仓库筛选"
          value={activeRepoFilter}
          onChange={(value) => {
            setRepoFilter(value);
            setPage(1);
          }}
          options={[{ id: "all", label: "全部仓库" }, ...repoOptions]}
        />
      )}
      {installed.error && installed.data !== undefined && (
        <InlineNotice tone="error">
          已安装 Skills 刷新失败，正在显示上一次成功数据：
          {errorMessage(installed.error)}
        </InlineNotice>
      )}
      {source === "repos" &&
        discovery.error &&
        discovery.data !== undefined && (
          <InlineNotice tone="error">
            仓库 Skills 刷新失败，正在显示上一次成功数据：
            {errorMessage(discovery.error)}
          </InlineNotice>
        )}
      {source === "repos" && repos.error && repos.data !== undefined && (
        <InlineNotice tone="error">
          仓库配置刷新失败，正在显示上一次成功数据：
          {errorMessage(repos.error)}
        </InlineNotice>
      )}
      {source === "skillssh" &&
        skillsSh.error &&
        skillsSh.data !== undefined && (
          <InlineNotice tone="error">
            skills.sh 刷新失败，正在显示上一次成功数据：
            {errorMessage(skillsSh.error)}
          </InlineNotice>
        )}
      {installed.error && installed.data === undefined ? (
        <EmptyState
          title="无法加载已安装 Skills"
          description={errorMessage(installed.error)}
          actions={
            <Button onClick={() => void installed.refetch()}>重试</Button>
          }
        />
      ) : source === "repos" && repos.error && repos.data === undefined ? (
        <EmptyState
          title="无法加载仓库配置"
          description={errorMessage(repos.error)}
          actions={<Button onClick={() => void repos.refetch()}>重试</Button>}
        />
      ) : source === "repos" &&
        discovery.error &&
        discovery.data === undefined ? (
        <EmptyState
          title="无法加载仓库 Skills"
          description={errorMessage(discovery.error)}
          actions={
            <Button onClick={() => void discovery.refetch()}>重试</Button>
          }
        />
      ) : source === "skillssh" &&
        skillsSh.error &&
        skillsSh.data === undefined ? (
        <EmptyState
          title="skills.sh 搜索失败"
          description={errorMessage(skillsSh.error)}
          actions={
            <Button onClick={() => void skillsSh.refetch()}>重试</Button>
          }
        />
      ) : (installed.data === undefined && installed.isPending) ||
        (source === "repos" &&
          ((discovery.data === undefined && discovery.isPending) ||
            (repos.data === undefined && repos.isPending))) ||
        (source === "skillssh" &&
          skillsShQuery.length >= 2 &&
          skillsSh.data === undefined &&
          skillsSh.isPending) ? (
        <EmptyState title="正在加载发现内容" description="请稍候">
          <Spinner />
        </EmptyState>
      ) : source === "repos" &&
        repos.data !== undefined &&
        repos.data.length === 0 ? (
        <EmptyState
          title="尚未配置仓库"
          description="留在仓库来源，可添加仓库或明确切换到 skills.sh"
          actions={
            <>
              <Button onClick={() => setDialog("repos")}>添加仓库</Button>{" "}
              <Button
                onClick={() => {
                  setSource("skillssh");
                  setPage(1);
                }}
              >
                切换到 skills.sh
              </Button>
            </>
          }
        />
      ) : skills.length === 0 ? (
        <EmptyState
          title="没有发现结果"
          description={
            source === "skillssh" && skillsShQuery.length < 2
              ? "输入至少两个字符开始搜索"
              : "当前来源或筛选条件下没有结果"
          }
        />
      ) : (
        <div className="fy-feature-discovery-scroll" aria-label="可发现 Skills">
          <p className="fy-feature-description">{resultSummary}</p>
          {groupedSkills.map(([repo, items]) => {
            const showHeading = items.length > 1;
            return (
              <section key={repo} aria-label={repo}>
                {showHeading ? (
                  <h3>
                    {repo} · {items.length}
                  </h3>
                ) : null}
                <div className="fy-feature-grid">
                  {items.map((skill) => (
                    <DiscoveryCard
                      key={skill.key}
                      busy={busy}
                      hideRepo={showHeading}
                      installLabel={installLabel}
                      isInstalled={isDiscoverableInstalled(
                        skill,
                        installedItems,
                      )}
                      skill={skill}
                      onInstall={onInstall}
                      onOpenDetail={setDetailSkill}
                    />
                  ))}
                </div>
              </section>
            );
          })}
        </div>
      )}
      <FeaturePagination
        page={page}
        totalPages={totalPages}
        ariaLabel={
          source === "skillssh" ? "skills.sh 分页" : "仓库 Skills 分页"
        }
        onPageChange={goToPage}
      />
      {detailSkill ? (
        <Dialog
          open
          title={detailSkill.name}
          description={skillDetailMeta(detailSkill) || "Skill 详情"}
          onOpenChange={(open) => {
            if (!open) setDetailSkill(null);
          }}
          actions={<Button onClick={() => setDetailSkill(null)}>关闭</Button>}
        >
          <p className="fy-feature-intro">{skillDetailBody(detailSkill)}</p>
        </Dialog>
      ) : null}
    </section>
  );
}

function AuxiliaryDialogs({
  name,
  close,
  installTarget,
  busy,
  write,
  setConfirm,
}: {
  name: DialogName;
  close: () => void;
  installTarget: SkillTargetId;
  busy: boolean;
  write: (title: string, operation: () => Promise<void>) => Promise<void>;
  setConfirm: (
    value:
      | { kind: "repo"; repo: SkillRepo }
      | { kind: "backup"; backup: SkillBackupEntry }
      | null,
  ) => void;
}) {
  const queryClient = useQueryClient();
  const { ports } = useFeatures();
  const unmanaged = useUnmanagedSkills(name === "unmanaged");
  const backups = useSkillBackups(name === "backups");
  const repos = useSkillRepos();
  const settings = useFeatureSettings(name === "settings");
  const [selected, setSelected] = useState<Set<string> | null>(null);
  const [repoValue, setRepoValue] = useState("");
  const [branch, setBranch] = useState("main");
  const [syncMethod, setSyncMethod] = useState<
    "auto" | "symlink" | "copy" | null
  >(null);
  const [importApps, setImportApps] = useState<
    Record<string, ReturnType<typeof createSkillAssignments>>
  >({});
  const [migrationTarget, setMigrationTarget] = useState<
    "fyagent" | "unified" | null
  >(null);
  const [migrationResult, setMigrationResult] = useState<{
    migratedCount: number;
    skippedCount: number;
    errors: string[];
  } | null>(null);
  const installed = useInstalledSkills();
  const selectedDirectories =
    selected ?? new Set((unmanaged.data ?? []).map((skill) => skill.directory));
  const selectedSyncMethod =
    syncMethod ?? settings.data?.skillSyncMethod ?? "auto";
  const parseRepo = (): SkillRepo | null => {
    const normalized = repoValue
      .trim()
      .replace(/^https:\/\/github\.com\//i, "")
      .replace(/\.git$/i, "")
      .replace(/\/$/, "");
    const [owner, repo, ...rest] = normalized.split("/");
    return owner && repo && rest.length === 0
      ? { owner, name: repo, branch: branch.trim() || "main", enabled: true }
      : null;
  };
  if (!name || name === "more") return null;
  if (name === "unmanaged")
    return (
      <Dialog
        open
        title="导入本地 Skills"
        description="选择要管理的 Skills。系统会根据支持情况预设可用应用，你仍可逐项调整。"
        onOpenChange={(open) => !open && !busy && close()}
        actions={
          <>
            <Button onClick={close} disabled={busy}>
              取消
            </Button>
            <Button
              className="fy-control-button-primary"
              disabled={busy || selectedDirectories.size === 0}
              onClick={() =>
                void write("导入完成", async () => {
                  await ports.skills.importFromApps(
                    (unmanaged.data ?? [])
                      .filter((skill) =>
                        selectedDirectories.has(skill.directory),
                      )
                      .map((skill) => ({
                        directory: skill.directory,
                        apps:
                          importApps[skill.directory] ??
                          createSkillAssignments(
                            supportedFoundIn(skill.foundIn),
                          ),
                      })),
                  );
                  close();
                })
              }
            >
              导入所选 · {selectedDirectories.size}
            </Button>
          </>
        }
      >
        <div className="fy-feature-list">
          {unmanaged.error && unmanaged.data !== undefined && (
            <InlineNotice tone="error">
              扫描刷新失败，正在显示上一次成功数据：
              {errorMessage(unmanaged.error)}
            </InlineNotice>
          )}
          {unmanaged.data === undefined && unmanaged.isLoading ? (
            <Spinner />
          ) : unmanaged.error && unmanaged.data === undefined ? (
            <InlineNotice tone="error">
              扫描失败：{errorMessage(unmanaged.error)}
            </InlineNotice>
          ) : (unmanaged.data ?? []).length === 0 ? (
            <p>没有发现未管理的 Skills。</p>
          ) : (
            unmanaged.data?.map((skill) => (
              <article key={skill.directory} className="fy-feature-card">
                <label className="fy-feature-assignment">
                  <Checkbox
                    label={`选择 ${skill.name}`}
                    checked={selectedDirectories.has(skill.directory)}
                    onCheckedChange={(checked) =>
                      setSelected((current) => {
                        const next = new Set(current ?? selectedDirectories);
                        if (checked) next.add(skill.directory);
                        else next.delete(skill.directory);
                        return next;
                      })
                    }
                  />
                  <strong>{skill.name}</strong>
                </label>
                <div className="fy-feature-check-grid">
                  {SKILL_TARGETS.map((app) => (
                    <label key={app.id} className="fy-feature-check">
                      <Checkbox
                        label={`${skill.name} 分配到 ${app.label}`}
                        checked={Boolean(
                          (importApps[skill.directory] ??
                            createSkillAssignments(
                              supportedFoundIn(skill.foundIn),
                            ))[app.id],
                        )}
                        onCheckedChange={(checked) =>
                          setImportApps((current) => ({
                            ...current,
                            [skill.directory]: {
                              ...(current[skill.directory] ??
                                createSkillAssignments(
                                  supportedFoundIn(skill.foundIn),
                                )),
                              [app.id]: checked,
                            },
                          }))
                        }
                        disabled={!selectedDirectories.has(skill.directory)}
                      />
                      {app.label}
                    </label>
                  ))}
                </div>
              </article>
            ))
          )}
        </div>
      </Dialog>
    );
  if (name === "backups")
    return (
      <Dialog
        open
        title="备份恢复"
        description={`恢复时安装到 ${SKILL_TARGETS.find((app) => app.id === installTarget)?.label}`}
        onOpenChange={(open) => !open && !busy && close()}
        actions={
          <Button onClick={close} disabled={busy}>
            关闭
          </Button>
        }
      >
        <div className="fy-feature-list">
          {backups.error && backups.data !== undefined && (
            <InlineNotice tone="error">
              备份刷新失败，正在显示上一次成功数据：
              {errorMessage(backups.error)}
            </InlineNotice>
          )}
          {backups.data === undefined && backups.isLoading ? (
            <Spinner />
          ) : backups.error && backups.data === undefined ? (
            <InlineNotice tone="error">
              备份加载失败：{errorMessage(backups.error)}
            </InlineNotice>
          ) : (backups.data ?? []).length === 0 ? (
            <p>当前没有可恢复的备份。</p>
          ) : (
            backups.data?.map((backup) => (
              <article key={backup.backupId} className="fy-feature-card">
                <h3>{backup.skill.name}</h3>
                <p>{new Date(backup.createdAt * 1000).toLocaleString()}</p>
                <footer>
                  <Button
                    disabled={busy}
                    onClick={() =>
                      void write("备份已恢复", async () => {
                        await ports.skills.restoreBackup(
                          backup.backupId,
                          installTarget,
                        );
                        close();
                      })
                    }
                  >
                    恢复
                  </Button>
                  <Button
                    className="fy-control-button-danger"
                    disabled={busy}
                    onClick={() => setConfirm({ kind: "backup", backup })}
                  >
                    删除
                  </Button>
                </footer>
              </article>
            ))
          )}
        </div>
      </Dialog>
    );
  if (name === "repos")
    return (
      <Dialog
        open
        title="仓库管理"
        description="填写 GitHub 仓库地址，支持简写或完整链接；移除仓库不会卸载已安装的 Skills。"
        onOpenChange={(open) => !open && !busy && close()}
        actions={
          <Button onClick={close} disabled={busy}>
            关闭
          </Button>
        }
      >
        <div className="fy-feature-form-grid">
          <label className="fy-control-field">
            GitHub 仓库
            <Input
              value={repoValue}
              onChange={(event) => setRepoValue(event.target.value)}
              placeholder="owner/repo"
            />
          </label>
          <label className="fy-control-field">
            分支
            <Input
              value={branch}
              onChange={(event) => setBranch(event.target.value)}
            />
          </label>
          <Button
            className="fy-control-button-primary fy-feature-form-span"
            disabled={busy || !parseRepo()}
            onClick={() => {
              const repo = parseRepo();
              if (repo)
                void write("仓库已添加", async () => {
                  await ports.skills.addRepo(repo);
                  await queryClient.invalidateQueries({
                    queryKey: featureKeys.skillRepos,
                  });
                  setRepoValue("");
                });
            }}
          >
            添加仓库
          </Button>
        </div>
        <div className="fy-feature-list">
          {repos.error && (
            <InlineNotice tone="error">
              仓库加载失败：{errorMessage(repos.error)}
            </InlineNotice>
          )}
          {(repos.data ?? []).map((repo) => (
            <div
              className="fy-feature-assignment"
              key={`${repo.owner}/${repo.name}`}
            >
              <span>
                {repo.owner}/{repo.name} · {repo.branch}
              </span>
              <Button
                className="fy-control-button-danger"
                disabled={busy}
                onClick={() => setConfirm({ kind: "repo", repo })}
              >
                移除
              </Button>
            </div>
          ))}
        </div>
      </Dialog>
    );
  return (
    <>
      <Dialog
        open
        title="Skill 设置"
        description="选择同步方式，并将已安装的 Skills 迁移到所选位置。"
        onOpenChange={(open) => !open && !busy && close()}
        actions={
          <Button onClick={close} disabled={busy}>
            关闭
          </Button>
        }
      >
        <div className="fy-feature-form-grid">
          <label className="fy-control-field">
            同步方式
            <select
              className="fy-control-select"
              value={selectedSyncMethod}
              onChange={(event) =>
                setSyncMethod(event.target.value as typeof syncMethod)
              }
            >
              <option value="auto">自动</option>
              <option value="symlink">符号链接</option>
              <option value="copy">复制</option>
            </select>
          </label>
          <Button
            disabled={busy || !settings.data}
            onClick={() =>
              void write("同步设置已保存", async () => {
                const fresh = await ports.settings.get();
                await ports.settings.save({
                  ...fresh,
                  skillSyncMethod: selectedSyncMethod,
                });
                await queryClient.invalidateQueries({
                  queryKey: featureKeys.settings,
                });
              })
            }
          >
            保存同步方式
          </Button>
          <Button disabled={busy} onClick={() => setMigrationTarget("fyagent")}>
            迁移到 FyAgent
          </Button>
          <Button disabled={busy} onClick={() => setMigrationTarget("unified")}>
            迁移到统一目录
          </Button>
        </div>
        {migrationResult && (
          <InlineNotice
            tone={migrationResult.errors.length ? "warning" : "info"}
          >
            已迁移 {migrationResult.migratedCount}，跳过{" "}
            {migrationResult.skippedCount}
            {migrationResult.errors.length > 0 && (
              <p>部分 Skills 未能迁移，请稍后重试。</p>
            )}
          </InlineNotice>
        )}
      </Dialog>
      <ConfirmDialog
        open={migrationTarget !== null}
        title="确认迁移 Skill 存储"
        description={
          (installed.data?.length ?? 0) > 0
            ? `当前有 ${installed.data?.length ?? 0} 个已安装 Skill。迁移期间请勿关闭应用。`
            : "当前没有已安装 Skill，仍会更新存储位置。"
        }
        pending={busy}
        onCancel={() => setMigrationTarget(null)}
        onConfirm={async () => {
          const target = migrationTarget;
          if (target)
            await write("存储迁移完成", async () => {
              setMigrationResult(await ports.skills.migrateStorage(target));
            });
          setMigrationTarget(null);
        }}
      />
    </>
  );
}
