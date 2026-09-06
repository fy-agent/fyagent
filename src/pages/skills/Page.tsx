import { useQueryClient } from "@tanstack/react-query";
import { useEffect, useMemo, useRef, useState } from "react";

import { getSkillTargetIcon } from "../../shared/assets/apps";
import {
  buildSkillSearchText,
  convergeSelection,
  errorMessage,
  isDiscoverableInstalled,
  runSequentialBulk,
  skillInstallDestination,
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
  useSkillHubSearch,
  useSkillUpdates,
  useUnmanagedSkills,
} from "../../shared/features/queries";
import {
  createSkillAssignments,
  SKILL_DISCOVERY_PAGE_SIZE,
  SKILLHUB_CATEGORY_ALL,
  SKILLHUB_CATEGORY_TABS,
  SKILLHUB_MARKET_OWNER,
  SKILLHUB_OFFICIAL_CATEGORIES,
  SKILL_TARGETS,
  type DiscoverableSkill,
  type InstalledSkill,
  type SkillBackupEntry,
  type SkillHubCategoryFilter,
  type SkillHubSkill,
  type SkillTargetId,
} from "../../shared/features/types";
import { Button } from "../../shared/ui/Button";
import { AnimatePresence } from "../../shared/ui/motion";
import type { DialogOriginRef } from "../../shared/ui/dialogOrigin";
import { useDialogState } from "../../shared/ui/useDialogState";
import { PopoverPrimitive } from "../../shared/ui/vendor";
import { ConfirmDialog, Dialog } from "../../shared/ui/Dialog";
import {
  Badge,
  Checkbox,
  EmptyState,
  InlineNotice,
  Spinner,
} from "../../shared/ui/primitives";
import { AssignmentPanel } from "../../shared/ui/AssignmentPanel";
import { InstallTargetDialog } from "../../shared/features/controls/InstallTargetDialog";
import { CopyablePath } from "../../shared/features/controls/CopyablePath";
import { ExternalLinkButton } from "../../shared/features/controls/ExternalLinkButton";
import { FeatureList, FeatureListItem } from "../../shared/ui/FeatureList";
import { FeaturePagination } from "../../shared/ui/FeaturePagination";
import { FeatureSearch } from "../../shared/ui/FeatureSearch";
import { FeatureTabPanel, FeatureTabs } from "../../shared/ui/FeatureTabs";
import { SplitPanes } from "../../shared/ui/split";

import "./page.css";

type SkillsTab = "installed" | "discovery";
type DialogName = "more" | "unmanaged" | "backups" | "settings" | null;

function formatSkillTimestamp(value: number): string {
  if (!Number.isFinite(value) || value <= 0) return "未知";
  return new Date(value * 1000).toLocaleString();
}

function githubRepoUrl(owner: string, name: string): string | null {
  if (owner.toLowerCase() === SKILLHUB_MARKET_OWNER) return null;
  if (!/^[\w.-]+$/.test(owner) || !/^[\w.-]+$/.test(name)) return null;
  return `https://github.com/${owner}/${name}`;
}

function isMarketSkill(skill: { repoOwner?: string }): boolean {
  return (skill.repoOwner ?? "").toLowerCase() === SKILLHUB_MARKET_OWNER;
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

function skillCardBody(skill: DiscoverableSkill): string {
  const description = skill.description.trim();
  if (description) return description;
  return skillDirectoryNote(skill);
}

function skillDetailBody(skill: DiscoverableSkill): string {
  return skillCardBody(skill) || "暂无说明";
}

type DiscoverySkill = DiscoverableSkill &
  Partial<
    Pick<
      SkillHubSkill,
      | "slug"
      | "version"
      | "ownerName"
      | "installs"
      | "downloads"
      | "homepageUrl"
      | "category"
    >
  >;

function skillHubCategoryLabel(key: string | undefined): string {
  if (!key) return "";
  return (
    SKILLHUB_OFFICIAL_CATEGORIES.find((item) => item.key === key)?.name ?? ""
  );
}

function skillDetailMeta(skill: DiscoverySkill): string {
  if (isMarketSkill(skill)) {
    return [
      "Skill 市场",
      skillHubCategoryLabel(skill.category),
      skill.slug ?? skill.repoName,
      skill.ownerName,
      skill.version ? `v${skill.version}` : null,
      typeof skill.installs === "number"
        ? `${skill.installs.toLocaleString()} 次安装`
        : null,
    ]
      .filter(Boolean)
      .join(" · ");
  }
  return skillRepoKey(skill);
}

function skillDocsAction(
  skill: DiscoverySkill,
): { url: string; label: "说明" | "仓库" | "主页" } | null {
  if (isMarketSkill(skill)) {
    const url = skill.homepageUrl ?? skill.readmeUrl;
    return url ? { url, label: "主页" } : null;
  }
  const repoUrl = githubRepoUrl(skill.repoOwner, skill.repoName);
  if (skill.readmeUrl) {
    return {
      url: skill.readmeUrl,
      label: /SKILL\.md|\/blob\//i.test(skill.readmeUrl) ? "说明" : "仓库",
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
  featureKeys.skillUnmanaged,
];

function Detail({
  originRef,
  skill,
  update,
  busy,
  onToggle,
  onUpdate,
  onUninstall,
  showAssignment,
}: {
  originRef?: DialogOriginRef;
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
  const market = isMarketSkill(skill);
  const repoUrl =
    skill.repoOwner && skill.repoName
      ? githubRepoUrl(skill.repoOwner, skill.repoName)
      : null;
  const sourceLabel = market
    ? "从 Skill 市场安装"
    : repo
      ? "GitHub 仓库"
      : "本地导入";
  const sourceLead = market
    ? "从 Skill 市场安装，保存在本地目录。"
    : repo
      ? "来自 GitHub 仓库，保存在本地目录。"
      : "来自本地导入或 ZIP 安装。";
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
            dialogOriginRef={originRef}
          >
            卸载
          </Button>
        </div>
      </div>
      <div className="fy-feature-info-grid">
        <section className="fy-feature-info-card" aria-label="下载来源">
          <h3>下载来源</h3>
          <p className="fy-feature-info-lead">{sourceLead}</p>
          <dl className="fy-feature-definition">
            <dt>来源类型</dt>
            <dd>{sourceLabel}</dd>
            {repo && !market && (
              <>
                <dt>仓库</dt>
                <dd>{repo}</dd>
              </>
            )}
            {skill.repoBranch && !market && (
              <>
                <dt>分支</dt>
                <dd>{skill.repoBranch}</dd>
              </>
            )}
            <dt>安装目录</dt>
            <dd>
              <CopyablePath
                revealValue={false}
                value={skillInstallPath(skill)}
              />
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
              ? `已启用 ${assigned.length} 个应用。`
              : "尚未分配到任何应用。"}
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
  const dialogOriginRef = useRef<HTMLElement | null>(null);
  const queryClient = useQueryClient();
  const { ports, installTarget, setInstallTarget, notify } = useFeatures();
  const wideLayout = useWideFeatureLayout();
  const [tab, setTab] = useState<SkillsTab>("installed");
  const [search, setSearch] = useState("");
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [dialog, setDialog, dialogKey] =
    useDialogState<Exclude<DialogName, null>>();
  const [confirm, setConfirm] = useState<
    | { kind: "uninstall"; skill: InstalledSkill }
    | { kind: "backup"; backup: SkillBackupEntry }
    | null
  >(null);
  const [busy, setBusy] = useState(false);
  const [pendingZipPath, setPendingZipPath, zipKey] = useDialogState<string>();
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
    setPendingZipPath(path);
  };

  return (
    <div
      className={`fy-feature-page fy-split-page fy-skills-page${tab === "discovery" ? " fy-skills-page-discovery" : ""}`}
      data-testid="skills-page"
      aria-label="Skills"
    >
      <header className="fy-feature-header">
        <h1 className="fy-skills-page-title">Skills 管理</h1>
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
        <div className="fy-feature-actions">
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
          <PopoverPrimitive.Root
            open={dialog === "more"}
            onOpenChange={(open) =>
              setDialog((current) =>
                open ? "more" : current === "more" ? null : current,
              )
            }
          >
            <PopoverPrimitive.Trigger asChild>
              <Button>更多</Button>
            </PopoverPrimitive.Trigger>
            <PopoverPrimitive.Portal>
              <PopoverPrimitive.Content
                className="fy-feature-menu-popover"
                align="end"
                sideOffset={8}
                aria-label="更多 Skill 操作"
              >
                <Button
                  dialogOriginRef={dialogOriginRef}
                  onClick={() => setDialog("unmanaged")}
                >
                  导入本地 Skill
                </Button>
                <Button
                  disabled={busy}
                  onClick={() => void pickAndInstallZip()}
                  dialogOriginRef={dialogOriginRef}
                >
                  从 ZIP 安装
                </Button>
                <Button
                  dialogOriginRef={dialogOriginRef}
                  onClick={() => setDialog("backups")}
                >
                  备份恢复
                </Button>
                <Button
                  dialogOriginRef={dialogOriginRef}
                  onClick={() => setDialog("settings")}
                >
                  Skill 设置
                </Button>
              </PopoverPrimitive.Content>
            </PopoverPrimitive.Portal>
          </PopoverPrimitive.Root>
        </div>
      </header>
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
      <FeatureTabPanel
        tabsId="skills-view-tabs"
        value="installed"
        active={tab === "installed"}
        unmountOnExit
      >
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
                      originRef={dialogOriginRef}
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
      </FeatureTabPanel>
      <FeatureTabPanel
        tabsId="skills-view-tabs"
        value="discovery"
        active={tab === "discovery"}
        unmountOnExit
      >
        <Discovery
          busy={busy}
          defaultTarget={installTarget}
          onInstall={(skill, target) => {
            setInstallTarget(target);
            return write(`${skill.name} 已安装`, async () => {
              const slug = skill.slug ?? skill.repoName;
              await ports.skills.installSkillHub(slug, target);
            });
          }}
        />
      </FeatureTabPanel>
      <AnimatePresence>
        {pendingZipPath ? (
          <InstallTargetDialog
            key={zipKey}
            originRef={dialogOriginRef}
            title="从 ZIP 安装"
            busy={busy}
            defaultTarget={installTarget}
            pathForTarget={(target) => skillInstallDestination(target)}
            pathNote="具体文件夹名由 ZIP 内的 Skill 决定。"
            onCancel={() => setPendingZipPath(null)}
            onConfirm={(target) => {
              const path = pendingZipPath;
              setPendingZipPath(null);
              setInstallTarget(target);
              void write("ZIP 安装完成", async () => {
                await ports.skills.installFromZip(path, target);
              });
            }}
          />
        ) : null}
      </AnimatePresence>
      <AnimatePresence>
        {dialog && dialog !== "more" && (
          <AuxiliaryDialogs
            key={dialogKey}
            originRef={dialogOriginRef}
            name={dialog}
            close={() => setDialog(null)}
            installTarget={installTarget}
            busy={busy}
            write={write}
            setConfirm={setConfirm}
          />
        )}
      </AnimatePresence>
      <ConfirmDialog
        originRef={dialogOriginRef}
        open={confirm !== null}
        title={
          confirm?.kind === "uninstall"
            ? `卸载 ${confirm.skill.name}`
            : confirm
              ? `删除 ${confirm.backup.skill.name} 的备份`
              : "确认操作"
        }
        description={
          confirm?.kind === "uninstall"
            ? "将从管理列表及已启用的应用中移除，并创建可恢复备份。"
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

function DiscoveryCard({
  originRef,
  busy,
  isInstalled,
  skill,
  onInstall,
  onOpenDetail,
}: {
  originRef?: DialogOriginRef;
  busy: boolean;
  isInstalled: boolean;
  skill: DiscoverySkill;
  onInstall: (skill: DiscoverySkill) => void;
  onOpenDetail: (skill: DiscoverySkill) => void;
}) {
  const body = skillCardBody(skill);
  const docs = skillDocsAction(skill);
  const meta = isMarketSkill(skill)
    ? [
        skillHubCategoryLabel(skill.category),
        skill.version ? `v${skill.version}` : null,
        skill.ownerName,
      ]
        .filter(Boolean)
        .join(" · ")
    : "";

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
          onClick={() => onInstall(skill)}
          dialogOriginRef={originRef}
        >
          {isInstalled ? "已安装" : "安装"}
        </Button>
        <Button dialogOriginRef={originRef} onClick={() => onOpenDetail(skill)}>
          详情
        </Button>
        {docs ? (
          <ExternalLinkButton url={docs.url}>{docs.label}</ExternalLinkButton>
        ) : null}
      </footer>
    </article>
  );
}

function Discovery({
  busy,
  defaultTarget,
  onInstall,
}: {
  busy: boolean;
  defaultTarget: SkillTargetId;
  onInstall: (skill: DiscoverySkill, target: SkillTargetId) => Promise<void>;
}) {
  const originRef = useRef<HTMLElement | null>(null);
  const [search, setSearch] = useState("");
  const [debouncedSearch, setDebouncedSearch] = useState("");
  const [category, setCategory] = useState<SkillHubCategoryFilter>(
    SKILLHUB_CATEGORY_ALL,
  );
  const [page, setPage] = useState(1);
  const [detailSkill, setDetailSkill, detailKey] =
    useDialogState<DiscoverySkill>();
  const [pendingSkill, setPendingSkill, installKey] =
    useDialogState<DiscoverySkill>();
  const resultsTop = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const timer = window.setTimeout(() => {
      setDebouncedSearch(search);
    }, 300);
    return () => window.clearTimeout(timer);
  }, [search]);
  const discoveryQuery = debouncedSearch.trim();
  const installed = useInstalledSkills();
  const market = useSkillHubSearch(discoveryQuery, page, category, true);
  const installedItems = useMemo(() => installed.data ?? [], [installed.data]);
  const skills: DiscoverySkill[] = market.data?.skills ?? [];
  const totalCount = market.data?.totalCount ?? skills.length;
  const totalPages = Math.max(
    1,
    Math.ceil(totalCount / SKILL_DISCOVERY_PAGE_SIZE),
  );
  const currentPage = Math.min(page, totalPages);
  const goToPage = (next: number) => {
    setPage(next);
    resultsTop.current?.scrollIntoView?.({ block: "start" });
  };
  return (
    <section className="fy-feature-workspace" ref={resultsTop}>
      <div className="fy-feature-toolbar">
        <FeatureSearch
          ariaLabel="搜索 Skill 市场"
          placeholder="搜索 Skill 名称或用途"
          value={search}
          onValueChange={(value) => {
            setSearch(value);
            setPage(1);
          }}
        />
        <FeatureTabs
          id="skills-discovery-categories"
          label="分类筛选"
          value={category}
          onChange={(value) => {
            setCategory(value);
            setPage(1);
          }}
          options={SKILLHUB_CATEGORY_TABS}
        />
      </div>
      {installed.error && installed.data !== undefined && (
        <InlineNotice tone="error">
          已安装 Skills 刷新失败，正在显示上一次成功数据：
          {errorMessage(installed.error)}
        </InlineNotice>
      )}
      {market.error && market.data !== undefined && (
        <InlineNotice tone="error">
          Skill 市场刷新失败，正在显示上一次成功数据：
          {errorMessage(market.error)}
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
      ) : market.error && market.data === undefined ? (
        <EmptyState
          title="Skill 市场搜索失败"
          description={errorMessage(market.error)}
          actions={<Button onClick={() => void market.refetch()}>重试</Button>}
        />
      ) : (installed.data === undefined && installed.isPending) ||
        (market.data === undefined && market.isPending) ? (
        <EmptyState title="正在加载发现内容" description="请稍候">
          <Spinner />
        </EmptyState>
      ) : skills.length === 0 ? (
        <EmptyState title="没有发现结果" description="当前搜索条件下没有结果" />
      ) : (
        <div className="fy-feature-discovery-scroll" aria-label="可发现 Skills">
          <div className="fy-feature-grid">
            {skills.map((skill) => (
              <DiscoveryCard
                originRef={originRef}
                key={skill.key}
                busy={busy}
                isInstalled={isDiscoverableInstalled(skill, installedItems)}
                skill={skill}
                onInstall={(next) => {
                  setPendingSkill(next);
                }}
                onOpenDetail={setDetailSkill}
              />
            ))}
          </div>
        </div>
      )}
      <FeaturePagination
        page={currentPage}
        totalPages={totalPages}
        ariaLabel="Skill 市场分页"
        onPageChange={goToPage}
      />
      <AnimatePresence>
        {detailSkill ? (
          <Dialog
            key={detailKey}
            originRef={originRef}
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
      </AnimatePresence>
      <AnimatePresence>
        {pendingSkill ? (
          <InstallTargetDialog
            key={installKey}
            originRef={originRef}
            title={`安装 ${pendingSkill.name}`}
            busy={busy}
            defaultTarget={defaultTarget}
            pathForTarget={(target) =>
              skillInstallDestination(
                target,
                pendingSkill.directory ||
                  pendingSkill.slug ||
                  pendingSkill.repoName,
              )
            }
            onCancel={() => setPendingSkill(null)}
            onConfirm={(target) => {
              const skill = pendingSkill;
              setPendingSkill(null);
              void onInstall(skill, target);
            }}
          />
        ) : null}
      </AnimatePresence>
    </section>
  );
}

function AuxiliaryDialogs({
  originRef,
  name,
  close,
  installTarget,
  busy,
  write,
  setConfirm,
}: {
  originRef?: DialogOriginRef;
  name: DialogName;
  close: () => void;
  installTarget: SkillTargetId;
  busy: boolean;
  write: (title: string, operation: () => Promise<void>) => Promise<void>;
  setConfirm: (
    value: { kind: "backup"; backup: SkillBackupEntry } | null,
  ) => void;
}) {
  const migrationOriginRef = useRef<HTMLElement | null>(null);
  const queryClient = useQueryClient();
  const { ports, setInstallTarget } = useFeatures();
  const unmanaged = useUnmanagedSkills(name === "unmanaged");
  const backups = useSkillBackups(name === "backups");
  const settings = useFeatureSettings(name === "settings");
  const [selected, setSelected] = useState<Set<string> | null>(null);
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
  const [restoreTarget, setRestoreTarget] =
    useState<SkillTargetId>(installTarget);
  const installed = useInstalledSkills();
  const selectedDirectories =
    selected ?? new Set((unmanaged.data ?? []).map((skill) => skill.directory));
  const selectedSyncMethod =
    syncMethod ?? settings.data?.skillSyncMethod ?? "auto";
  if (!name || name === "more") return null;
  if (name === "unmanaged")
    return (
      <Dialog
        originRef={originRef}
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
                <AssignmentPanel
                  apps={
                    importApps[skill.directory] ??
                    createSkillAssignments(supportedFoundIn(skill.foundIn))
                  }
                  disabled={busy || !selectedDirectories.has(skill.directory)}
                  labelSuffix="Skill 分配"
                  onToggle={(app, enabled) =>
                    setImportApps((current) => ({
                      ...current,
                      [skill.directory]: {
                        ...(current[skill.directory] ??
                          createSkillAssignments(
                            supportedFoundIn(skill.foundIn),
                          )),
                        [app]: enabled,
                      },
                    }))
                  }
                  targets={SKILL_TARGETS}
                />
              </article>
            ))
          )}
        </div>
      </Dialog>
    );
  if (name === "backups")
    return (
      <Dialog
        originRef={originRef}
        open
        title="备份恢复"
        description="选择要恢复到的应用。"
        onOpenChange={(open) => !open && !busy && close()}
        actions={
          <Button onClick={close} disabled={busy}>
            关闭
          </Button>
        }
      >
        <AssignmentPanel
          mode="radio"
          ariaLabel="恢复目标"
          disabled={busy}
          onChange={setRestoreTarget}
          targets={SKILL_TARGETS}
          value={restoreTarget}
        />
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
                        setInstallTarget(restoreTarget);
                        await ports.skills.restoreBackup(
                          backup.backupId,
                          restoreTarget,
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
                    dialogOriginRef={originRef}
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
  return (
    <>
      <Dialog
        originRef={originRef}
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
          <Button
            dialogOriginRef={migrationOriginRef}
            disabled={busy}
            onClick={() => setMigrationTarget("fyagent")}
          >
            迁移到 FyAgent
          </Button>
          <Button
            dialogOriginRef={migrationOriginRef}
            disabled={busy}
            onClick={() => setMigrationTarget("unified")}
          >
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
        originRef={migrationOriginRef}
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
