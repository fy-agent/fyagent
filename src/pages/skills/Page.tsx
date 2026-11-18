import { useQueryClient } from "@tanstack/react-query";
import { useMemo, useRef, useState } from "react";

import {
  buildSkillSearchText,
  convergeSelection,
  errorMessage,
  runSequentialBulk,
  skillInstallDestination,
  UserFacingError,
} from "../../shared/features/helpers";
import { useFeatures } from "../../shared/features/provider";
import { skillUpdateErrorMessage } from "../../shared/features/skills";
import { useWideFeatureLayout } from "../../shared/features/responsive";
import {
  useInstalledSkills,
  useSkillUpdates,
} from "../../shared/features/queries";
import {
  SKILL_TARGETS,
  type InstalledSkill,
  type SkillBackupEntry,
  type SkillTargetId,
} from "../../shared/features/types";
import { Button } from "../../shared/ui/Button";
import { AnimatePresence } from "../../shared/ui/motion";
import { useDialogState } from "../../shared/ui/useDialogState";
import { PopoverPrimitive } from "../../shared/ui/vendor";
import { ConfirmDialog } from "../../shared/ui/Dialog";
import { EmptyState, InlineNotice, Spinner } from "../../shared/ui/primitives";
import { AssignmentPanel } from "../../shared/ui/AssignmentPanel";
import { BulkAssignmentDialog } from "../../shared/features/controls/BulkAssignmentDialog";
import {
  executeBulkAssignment,
  type BulkAssignmentItem,
  type BulkAssignmentPlan,
  type BulkAssignmentResult,
} from "../../shared/features/bulk-assignment";
import { InstallTargetDialog } from "../../shared/features/controls/InstallTargetDialog";
import { ExternalLinkButton } from "../../shared/features/controls/ExternalLinkButton";
import { FeatureList, FeatureListItem } from "../../shared/ui/FeatureList";
import { FeatureSearch } from "../../shared/ui/FeatureSearch";
import { FeatureTabPanel, FeatureTabs } from "../../shared/ui/FeatureTabs";
import { SplitPanes } from "../../shared/ui/split";
import { DETAIL_PANE_SIZING } from "../../shared/ui/split/sizing";
import { AuxiliaryDialogs } from "./AuxiliaryDialogs";
import { SkillDetail } from "./SkillDetail";
import { SkillDiscovery } from "./SkillDiscovery";
import {
  githubRepoUrl,
  INSTALLED_SPLIT_LABELS,
  invalidations,
  type DialogName,
  type SkillsTab,
} from "./skillsShared";

import "./page.css";

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
    | { kind: "updateAll" }
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
  const applicableUpdates = useMemo(
    () =>
      updates.filter((update) =>
        installed.some((item) => item.id === update.id),
      ),
    [updates, installed],
  );
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
        applicableUpdates.map((item) => item.id),
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
          {applicableUpdates.length > 0 && (
            <Button
              className="fy-control-button-primary"
              dialogOriginRef={dialogOriginRef}
              onClick={() => setConfirm({ kind: "updateAll" })}
              disabled={busy}
            >
              更新全部 · {applicableUpdates.length}
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
                    <SkillDetail
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
        <SkillDiscovery
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
            : confirm?.kind === "backup"
              ? `删除 ${confirm.backup.skill.name} 的备份`
              : confirm?.kind === "updateAll"
                ? `更新 ${applicableUpdates.length} 个 Skill？`
                : "确认操作"
        }
        description={
          confirm?.kind === "uninstall"
            ? "将从管理列表及已启用的应用中移除。"
            : confirm?.kind === "backup"
              ? "删除后无法从该备份恢复。"
              : confirm?.kind === "updateAll"
                ? "更新会用仓库里的内容替换本地目录。本地改过的地方只保留在备份里。"
                : ""
        }
        danger={confirm?.kind !== "updateAll"}
        pending={busy}
        onCancel={() => setConfirm(null)}
        onConfirm={async () => {
          const action = confirm;
          if (!action) return;
          if (action.kind === "uninstall")
            await write("Skill 已卸载", async () => {
              await ports.skills.uninstall(action.skill.id);
            });
          else if (action.kind === "backup")
            await write("备份已删除", async () => {
              await ports.skills.deleteBackup(action.backup.backupId);
            });
          else if (action.kind === "updateAll") {
            setConfirm(null);
            await updateAll();
            return;
          }
          setConfirm(null);
        }}
      >
        {confirm?.kind === "updateAll" && (
          <ul aria-label="将要更新的 Skill" className="fy-skills-update-list">
            {applicableUpdates.map((update) => {
              const skill = installed.find((item) => item.id === update.id);
              const name = skill?.name ?? update.name;
              const repo =
                skill?.repoOwner && skill?.repoName
                  ? `${skill.repoOwner}/${skill.repoName}`
                  : null;
              const repoUrl =
                skill?.readmeUrl ??
                (skill?.repoOwner && skill?.repoName
                  ? githubRepoUrl(skill.repoOwner, skill.repoName)
                  : null);
              return (
                <li key={update.id} className="fy-skills-update-item">
                  <span className="fy-skills-update-name">{name}</span>
                  {repo &&
                    (repoUrl ? (
                      <ExternalLinkButton
                        url={repoUrl}
                        className="fy-skills-update-repo"
                      >
                        {repo}
                      </ExternalLinkButton>
                    ) : (
                      <span className="fy-skills-update-repo">{repo}</span>
                    ))}
                </li>
              );
            })}
          </ul>
        )}
      </ConfirmDialog>
    </div>
  );
}
