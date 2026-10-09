import { CaretDownIcon } from "@phosphor-icons/react/dist/csr/CaretDown";
import { useQueryClient } from "@tanstack/react-query";
import { useEffect, useMemo, useRef, useState } from "react";

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
import { skillUpdateErrorMessage } from "../../shared/features/skills";
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
import {
  Collapsible,
  CollapsibleCaret,
  CollapsibleContent,
  CollapsibleTrigger,
} from "../../shared/ui/Collapsible";
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
import { BulkAssignmentDialog } from "../../shared/features/controls/BulkAssignmentDialog";
import {
  executeBulkAssignment,
  type BulkAssignmentItem,
  type BulkAssignmentPlan,
  type BulkAssignmentResult,
} from "../../shared/features/bulk-assignment";
import { InstallTargetDialog } from "../../shared/features/controls/InstallTargetDialog";
import { CopyablePath } from "../../shared/features/controls/CopyablePath";
import { ExternalLinkButton } from "../../shared/features/controls/ExternalLinkButton";
import { FeatureList, FeatureListItem } from "../../shared/ui/FeatureList";
import { FeaturePagination } from "../../shared/ui/FeaturePagination";
import { FeatureSearch } from "../../shared/ui/FeatureSearch";
import { FeatureTabPanel, FeatureTabs } from "../../shared/ui/FeatureTabs";
import { SplitPanes } from "../../shared/ui/split";
import { DETAIL_PANE_SIZING } from "../../shared/ui/split/sizing";

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
  const [installationOpen, setInstallationOpen] = useState(false);
  const repo =
    skill.repoOwner && skill.repoName
      ? `${skill.repoOwner}/${skill.repoName}`
      : null;
  const market = isMarketSkill(skill);
  const repoUrl =
    skill.repoOwner && skill.repoName
      ? githubRepoUrl(skill.repoOwner, skill.repoName)
      : null;
  const sourceLabel = market ? "Skill 市场" : repo ? "GitHub 仓库" : "本地导入";
  const description = skill.description?.trim();

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
        {description && <p className="fy-feature-intro">{description}</p>}
        {skill.readOnly || skill.readOnlyTargets?.length ? (
          <InlineNotice tone="warning">
            检测到链接目录：{skill.readOnly ? "此 Skill 来源只读。" : ""}
            {skill.readOnlyTargets?.length
              ? `只读目标：${skill.readOnlyTargets.map((id) => SKILL_TARGETS.find((target) => target.id === id)?.label ?? id).join("、")}。`
              : ""}
            链接及其目标不会写入或删除；请在外部调整链接后刷新。
          </InlineNotice>
        ) : null}
        <div className="fy-feature-actions">
          {update && (
            <Button
              className="fy-control-button-primary"
              disabled={
                busy || skill.readOnly || Boolean(skill.readOnlyTargets?.length)
              }
              onClick={onUpdate}
            >
              更新
            </Button>
          )}
          <Button
            className="fy-control-button-danger"
            disabled={
              busy || skill.readOnly || Boolean(skill.readOnlyTargets?.length)
            }
            onClick={onUninstall}
            dialogOriginRef={originRef}
          >
            卸载
          </Button>
        </div>
      </div>
      <div className="fy-feature-info-grid">
        <Collapsible
          open={installationOpen}
          onOpenChange={setInstallationOpen}
          asChild
        >
          <section className="fy-feature-info-card" aria-label="安装信息">
            <h3>
              <CollapsibleTrigger asChild>
                <Button>
                  安装信息
                  <CollapsibleCaret open={installationOpen}>
                    <CaretDownIcon size={16} />
                  </CollapsibleCaret>
                </Button>
              </CollapsibleTrigger>
            </h3>
            <CollapsibleContent open={installationOpen}>
              <dl className="fy-feature-definition">
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
                {skill.installedAt > 0 && (
                  <>
                    <dt>安装时间</dt>
                    <dd>{formatSkillTimestamp(skill.installedAt)}</dd>
                  </>
                )}
                <dt>最近更新</dt>
                <dd>{formatSkillTimestamp(skill.updatedAt)}</dd>
              </dl>
              {(repoUrl || skill.readmeUrl) && (
                <div className="fy-feature-actions">
                  {repoUrl && (
                    <ExternalLinkButton url={repoUrl}>
                      打开仓库
                    </ExternalLinkButton>
                  )}
                  {skill.readmeUrl && (
                    <ExternalLinkButton url={skill.readmeUrl}>
                      查看说明
                    </ExternalLinkButton>
                  )}
                </div>
              )}
            </CollapsibleContent>
          </section>
        </Collapsible>
      </div>
      {showAssignment && (
        <div className="fy-feature-inline-assignment">
          <AssignmentPanel
            apps={skill.apps}
            disabled={busy}
            disabledTargets={SKILL_TARGETS.filter((target) =>
              skill.readOnlyTargets?.includes(target.id),
            ).map((target) => target.id)}
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
  const menuTriggerRef = useRef<HTMLButtonElement>(null);
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
  const [bulkOpen, setBulkOpen] = useState(false);
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
      await refreshAll();
      notify({ tone: "success", title });
    } catch (error) {
      try {
        await refreshAll();
      } catch {
        /* Keep the original failure visible. */
      }
      notify({
        tone: "error",
        title: `${title}失败`,
        description: skillUpdateErrorMessage(error) ?? errorMessage(error),
      });
    } finally {
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
      const accepted = await ports.skills.toggleApp(skill.id, app, enabled);
      const observed = (await ports.skills.getInstalled()).find(
        (entry) => entry.id === skill.id,
      );
      if (accepted === false || observed?.apps[app] !== enabled)
        throw new UserFacingError(
          "分配未确认，可能存在部分写入。请刷新后重试。",
        );
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
        async (id) => {
          try {
            return await ports.skills.update(id);
          } catch (error) {
            const message = skillUpdateErrorMessage(error);
            if (message) throw new UserFacingError(message);
            throw error;
          }
        },
        (done, total) => setProgress({ done, total }),
      );
      if (result.failures.length) {
        const details = result.failures
          .map((failure) => failure.error)
          .filter((message) => message !== "请稍后重试。")
          .join(" ");
        throw new UserFacingError(
          `${result.failures.length} 项失败，${result.successes.length} 项成功${details ? `。${details}` : ""}`,
        );
      }
    });
  const bulkItem = (skill: InstalledSkill): BulkAssignmentItem => ({
    id: skill.id,
    name: skill.name,
    apps: skill.apps,
    identity: JSON.stringify({
      ...skill,
      apps: undefined,
      readOnly: undefined,
      readOnlyTargets: undefined,
    }),
    // Native first assignment adopts observed entries: path/hash/times become SSOT metadata.
    allowAdoption: skill.contentHash == null,
    readbackIdentity: JSON.stringify({
      ...skill,
      apps: undefined,
      readOnly: undefined,
      readOnlyTargets: undefined,
      path: undefined,
      contentHash: undefined,
      installedAt: undefined,
      updatedAt: undefined,
    }),
    // These observation-only fields are supplied by the I17 command wrapper.
    readOnly: (skill as InstalledSkill & { readOnly?: boolean }).readOnly,
    readOnlyTargets: (skill as InstalledSkill & { readOnlyTargets?: string[] })
      .readOnlyTargets,
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
      await executeBulkAssignment(
        plan,
        async () => (await ports.skills.getInstalled()).map(bulkItem),
        (id, target, enabled) => ports.skills.toggleApp(id, target, enabled),
        onResult,
      );
    } finally {
      try {
        await refreshAll();
      } finally {
        setBusy(false);
        writeLock.current = false;
      }
    }
  };
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
      className="fy-feature-page fy-split-page fy-skills-page"
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
            disabled={busy || !installed.length}
            dialogOriginRef={dialogOriginRef}
            onClick={() => setBulkOpen(true)}
          >
            批量分配
          </Button>
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
              <Button ref={menuTriggerRef}>更多</Button>
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
                  dialogReturnRef={menuTriggerRef}
                  onClick={() => setDialog("unmanaged")}
                >
                  导入本地 Skill
                </Button>
                <Button
                  disabled={busy}
                  onClick={() => void pickAndInstallZip()}
                  dialogOriginRef={dialogOriginRef}
                  dialogReturnRef={menuTriggerRef}
                >
                  从 ZIP 安装
                </Button>
                <Button
                  dialogOriginRef={dialogOriginRef}
                  dialogReturnRef={menuTriggerRef}
                  onClick={() => setDialog("backups")}
                >
                  备份恢复
                </Button>
                <Button
                  dialogOriginRef={dialogOriginRef}
                  dialogReturnRef={menuTriggerRef}
                  onClick={() => setDialog("settings")}
                >
                  Skill 设置
                </Button>
              </PopoverPrimitive.Content>
            </PopoverPrimitive.Portal>
          </PopoverPrimitive.Root>
        </div>
      </header>
      <p className="fy-feature-note" role="note">
        链接目录中的 Skills 可以读取；链接目录及其目标只读，不会写入或删除。
        如需安装、更新或调整分配，请使用普通目录。
      </p>
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
      <AnimatePresence>
        {bulkOpen && (
          <BulkAssignmentDialog
            key="skills-bulk"
            kind="Skills"
            originRef={dialogOriginRef}
            busy={busy}
            items={installed.map(bulkItem)}
            onClose={() => setBulkOpen(false)}
            onExecute={executeAssignment}
          />
        )}
      </AnimatePresence>
      <FeatureTabPanel
        tabsId="skills-view-tabs"
        value="installed"
        layout="workspace"
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
            <EmptyState title="正在加载 Skills">
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
                <SplitPanes
                  {...DETAIL_PANE_SIZING}
                  separatorLabels={INSTALLED_SPLIT_LABELS}
                >
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
                          {skill.description && (
                            <span>{skill.description}</span>
                          )}
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
                        disabledTargets={SKILL_TARGETS.filter((target) =>
                          selected.readOnlyTargets?.includes(target.id),
                        ).map((target) => target.id)}
                        labelSuffix="Skill 分配"
                        onToggle={(app, enabled) =>
                          toggle(selected, app, enabled)
                        }
                        targets={SKILL_TARGETS}
                      />
                      <hr />
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
        layout="workspace"
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
            onViewInstalled={(id) => {
              setDialog(null);
              setTab("installed");
              setSearch("");
              setSelectedId(id ?? null);
            }}
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
            ? "将从管理列表及已启用的应用中移除。"
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
        <EmptyState title="正在加载发现内容">
          <Spinner />
        </EmptyState>
      ) : skills.length === 0 ? (
        <EmptyState title="没有发现结果" description="试试其他关键词或分类。" />
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
            description={skillDetailMeta(detailSkill) || undefined}
            onOpenChange={(open) => {
              if (!open) setDetailSkill(null);
            }}
            actions={<Button onClick={() => setDetailSkill(null)}>关闭</Button>}
          >
            {skillCardBody(detailSkill) && (
              <p className="fy-feature-intro">{skillCardBody(detailSkill)}</p>
            )}
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
  onViewInstalled,
  setConfirm,
}: {
  originRef?: DialogOriginRef;
  name: DialogName;
  close: () => void;
  installTarget: SkillTargetId;
  busy: boolean;
  write: (title: string, operation: () => Promise<void>) => Promise<void>;
  onViewInstalled(id?: string): void;
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
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [importSearch, setImportSearch] = useState("");
  const [importPreview, setImportPreview] = useState<Array<{
    directory: string;
    name: string;
    identity: string;
    apps: ReturnType<typeof createSkillAssignments>;
  }> | null>(null);
  const [importBlocked, setImportBlocked] = useState<Set<string>>(new Set());
  const importPending = useRef(false);
  const [importRunning, setImportRunning] = useState(false);
  const [importResults, setImportResults] = useState<
    Array<{
      directory: string;
      name: string;
      complete: boolean;
      installedId?: string;
      unconfirmed?: boolean;
    }>
  >([]);
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
  const selectedDirectories = selected;
  const selectedAvailable = (unmanaged.data ?? []).filter(
    (skill) =>
      selected.has(skill.directory) && !importBlocked.has(skill.directory),
  );
  const visibleUnmanaged = (unmanaged.data ?? []).filter((skill) =>
    `${skill.name} ${skill.directory}`
      .toLocaleLowerCase()
      .includes(importSearch.trim().toLocaleLowerCase()),
  );
  const selectedSyncMethod =
    syncMethod ?? settings.data?.skillSyncMethod ?? "auto";
  if (!name || name === "more") return null;
  const importLocked = busy || importRunning;
  if (name === "unmanaged")
    return (
      <Dialog
        originRef={originRef}
        open
        title="导入本地 Skills"
        description="选择 Skills 和要启用的软件。链接来源可复制到普通受管目录，原链接及其目标保持只读。"
        onOpenChange={(open) =>
          !open && !importLocked && !importPending.current && close()
        }
        actions={
          <>
            <Button
              onClick={() => {
                if (!importPending.current) close();
              }}
              disabled={importLocked}
            >
              取消
            </Button>
            {importPreview ? (
              <>
                <Button
                  disabled={importLocked}
                  onClick={() => {
                    setImportPreview(null);
                    setImportResults([]);
                  }}
                >
                  重新选择
                </Button>
                {importResults.some(
                  (item) =>
                    !item.complete && !item.installedId && !item.unconfirmed,
                ) && (
                  <Button
                    disabled={importLocked}
                    onClick={() => {
                      setSelected(
                        new Set(
                          importResults
                            .filter(
                              (item) =>
                                !item.complete &&
                                !item.installedId &&
                                !item.unconfirmed,
                            )
                            .map((item) => item.directory),
                        ),
                      );
                      setImportPreview(null);
                      setImportResults([]);
                    }}
                  >
                    选择未完成项重新预览
                  </Button>
                )}
                <Button
                  className="fy-control-button-primary"
                  disabled={
                    importLocked ||
                    !importPreview.length ||
                    importResults.length > 0
                  }
                  onClick={async () => {
                    if (importPending.current || importLocked) return;
                    importPending.current = true;
                    setImportRunning(true);
                    try {
                      await write("导入完成", async () => {
                        try {
                          const fresh = await ports.skills.scanUnmanaged();
                          if (
                            importPreview.some(
                              (item) =>
                                JSON.stringify(
                                  fresh.find(
                                    (entry) =>
                                      entry.directory === item.directory,
                                  ),
                                ) !== item.identity,
                            )
                          )
                            throw new UserFacingError(
                              "来源已变化，导入未执行。请重新选择并预览。",
                            );
                          const results: Array<{
                            directory: string;
                            name: string;
                            complete: boolean;
                            installedId?: string;
                            unconfirmed?: boolean;
                          }> = [];
                          for (const item of importPreview) {
                            let complete = false;
                            let installedId: string | undefined;
                            let unconfirmed = false;
                            try {
                              const currentSource = (
                                await ports.skills.scanUnmanaged()
                              ).find(
                                (entry) => entry.directory === item.directory,
                              );
                              if (
                                JSON.stringify(currentSource) !== item.identity
                              )
                                throw new UserFacingError(
                                  "此来源已变化，请重新预览。",
                                );
                              const imported =
                                await ports.skills.importFromApps([
                                  {
                                    directory: item.directory,
                                    apps: item.apps,
                                  },
                                ]);
                              const observed = (
                                await ports.skills.getInstalled()
                              ).find(
                                (entry) => entry.directory === item.directory,
                              );
                              installedId = observed?.id;
                              complete =
                                imported.some(
                                  (entry) => entry.directory === item.directory,
                                ) &&
                                Boolean(observed) &&
                                SKILL_TARGETS.every(
                                  (target) =>
                                    observed?.apps[target.id] ===
                                    item.apps[target.id],
                                );
                            } catch {
                              try {
                                installedId = (
                                  await ports.skills.getInstalled()
                                ).find(
                                  (entry) => entry.directory === item.directory,
                                )?.id;
                              } catch {
                                unconfirmed = true;
                              }
                            }
                            if (complete || installedId || unconfirmed)
                              setImportBlocked(
                                (current) =>
                                  new Set([...current, item.directory]),
                              );
                            results.push({
                              directory: item.directory,
                              name: item.name,
                              complete,
                              installedId,
                              unconfirmed,
                            });
                            setImportResults([...results]);
                          }
                          if (results.some((item) => !item.complete))
                            throw new UserFacingError(
                              "部分导入未完成。请查看逐项结果：已入库项到已安装页重新预览分配；无法确认的项先刷新查看，勿重复导入。",
                            );
                        } catch (error) {
                          setImportResults((current) =>
                            current.length
                              ? current
                              : importPreview.map((item) => ({
                                  directory: item.directory,
                                  name: item.name,
                                  complete: false,
                                })),
                          );
                          throw error;
                        }
                      });
                    } finally {
                      importPending.current = false;
                      setImportRunning(false);
                    }
                  }}
                >
                  确认导入 · {importPreview.length}
                </Button>
              </>
            ) : (
              <Button
                className="fy-control-button-primary"
                disabled={
                  importLocked ||
                  unmanaged.isFetching ||
                  !selectedAvailable.length ||
                  Boolean(unmanaged.error)
                }
                onClick={() =>
                  setImportPreview(
                    selectedAvailable.map((skill) => ({
                      directory: skill.directory,
                      name: skill.name,
                      identity: JSON.stringify(skill),
                      apps: {
                        ...(importApps[skill.directory] ??
                          createSkillAssignments(
                            supportedFoundIn(skill.foundIn),
                          )),
                      },
                    })),
                  )
                }
              >
                预览导入 · {selectedAvailable.length}
              </Button>
            )}
          </>
        }
      >
        {importPreview ? (
          <div
            className="fy-feature-list"
            aria-label="导入预览与结果"
            aria-live="polite"
          >
            <p>
              将导入 {importPreview.length} 项到 Skills
              库；原目录保持不变，分配到以下所选软件。
            </p>
            {importPreview.map((item) => (
              <article key={item.directory} className="fy-feature-card">
                <strong>{item.name}</strong>
                <p>{item.directory}</p>
                <p>
                  {SKILL_TARGETS.filter((target) => item.apps[target.id])
                    .map((target) => target.label)
                    .join("、") || "不分配到软件"}
                </p>
                {(() => {
                  const result = importResults.find(
                    (entry) => entry.directory === item.directory,
                  );
                  return (
                    <>
                      <p>
                        {!result
                          ? "等待确认"
                          : result.complete
                            ? "导入完成"
                            : result.installedId
                              ? "已入库，目标分配待确认或未完成；请在已安装页重新预览分配。"
                              : result.unconfirmed
                                ? "无法确认是否已入库；请刷新并查看已安装列表，避免重复导入。"
                                : "导入未完成；可重新选择仍未管理的项目并预览重试。"}
                      </p>
                      {result && !result.complete && (
                        <Button
                          disabled={importLocked}
                          onClick={async () => {
                            if (importPending.current) return;
                            importPending.current = true;
                            setImportRunning(true);
                            try {
                              await queryClient.invalidateQueries({
                                queryKey: featureKeys.skills,
                              });
                            } catch {
                              /* The installed page exposes its query error. */
                            } finally {
                              importPending.current = false;
                              setImportRunning(false);
                              onViewInstalled(result.installedId);
                            }
                          }}
                        >
                          {result.installedId
                            ? `查看已安装 ${item.name}`
                            : "刷新并查看已安装"}
                        </Button>
                      )}
                    </>
                  );
                })()}
              </article>
            ))}
          </div>
        ) : (
          <>
            <FeatureSearch
              value={importSearch}
              onValueChange={setImportSearch}
              disabled={importLocked}
              ariaLabel="筛选本地 Skills"
              placeholder="按名称或目录筛选"
            />
            <p>
              已选 {selected.size} 项 · 当前可用 {selectedAvailable.length} 项 ·
              筛选结果 {visibleUnmanaged.length} 项
            </p>
            <Button
              disabled={importLocked}
              onClick={() => setSelected(new Set())}
            >
              清空选择
            </Button>
            <Button
              disabled={importLocked || !visibleUnmanaged.length}
              onClick={() =>
                setSelected(
                  (current) =>
                    new Set([
                      ...current,
                      ...visibleUnmanaged.map((skill) => skill.directory),
                    ]),
                )
              }
            >
              选择筛选结果
            </Button>
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
              ) : visibleUnmanaged.length === 0 ? (
                <p>没有筛选结果，已选项目仍保留。</p>
              ) : (
                visibleUnmanaged.map((skill) => (
                  <article key={skill.directory} className="fy-feature-card">
                    <label className="fy-feature-assignment">
                      <Checkbox
                        label={`选择 ${skill.name}`}
                        disabled={
                          importLocked || importBlocked.has(skill.directory)
                        }
                        checked={selectedDirectories.has(skill.directory)}
                        onCheckedChange={(checked) =>
                          setSelected((current) => {
                            const next = new Set(current);
                            if (checked) next.add(skill.directory);
                            else next.delete(skill.directory);
                            return next;
                          })
                        }
                      />
                      <strong>{skill.name}</strong>
                    </label>
                    {importBlocked.has(skill.directory) && (
                      <InlineNotice tone="warning">
                        此项已导入或结果未确认；请查看已安装列表后处理分配，勿重复导入。
                      </InlineNotice>
                    )}
                    {skill.readOnly && (
                      <InlineNotice tone="warning">
                        检测到链接来源，只读取并复制到普通受管目录；原目录不会修改或删除。
                      </InlineNotice>
                    )}
                    <AssignmentPanel
                      apps={
                        importApps[skill.directory] ??
                        createSkillAssignments(supportedFoundIn(skill.foundIn))
                      }
                      disabled={
                        importLocked ||
                        importBlocked.has(skill.directory) ||
                        !selectedDirectories.has(skill.directory)
                      }
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
          </>
        )}
      </Dialog>
    );
  if (name === "backups")
    return (
      <Dialog
        originRef={originRef}
        open
        title="备份恢复"
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
