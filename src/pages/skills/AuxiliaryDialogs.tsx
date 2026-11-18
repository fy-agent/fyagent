import { useQueryClient } from "@tanstack/react-query";
import { useRef, useState } from "react";

import {
  errorMessage,
  supportedFoundIn,
  UserFacingError,
} from "../../shared/features/helpers";
import { useFeatures } from "../../shared/features/provider";
import {
  featureKeys,
  useFeatureSettings,
  useInstalledSkills,
  useSkillBackups,
  useUnmanagedSkills,
} from "../../shared/features/queries";
import {
  createSkillAssignments,
  SKILL_TARGETS,
  type SkillBackupEntry,
  type SkillTargetId,
} from "../../shared/features/types";
import { Button } from "../../shared/ui/Button";
import type { DialogOriginRef } from "../../shared/ui/dialogOrigin";
import { ConfirmDialog, Dialog } from "../../shared/ui/Dialog";
import { Checkbox, InlineNotice, Spinner } from "../../shared/ui/primitives";
import { AssignmentPanel } from "../../shared/ui/AssignmentPanel";
import { FeatureSearch } from "../../shared/ui/FeatureSearch";
import type { DialogName } from "./skillsShared";

export function AuxiliaryDialogs({
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
