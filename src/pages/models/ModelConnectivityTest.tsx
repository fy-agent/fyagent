import { useEffect, useMemo, useRef, useState } from "react";

import { classNames } from "../../shared/design-system/classNames";
import type {
  ModelProbeResult,
  ModelProbeSnapshot,
} from "../../shared/features/models";
import { Button, PressableButton } from "../../shared/ui/Button";
import { Dialog } from "../../shared/ui/Dialog";
import { usePersistentVisibility } from "../../shared/ui/PersistentSurface";
import { FieldFeedback, type Notice } from "./feedback";
import { GroupedModelChips, ModelSearchField } from "./modelChips";
import { noticeFromModelProbe } from "./modelsShared";
import {
  classifyModelType,
  filterModelIds,
  groupModelIds,
} from "./workBuddyModels";

function latestProbeSnapshot(
  previous: ModelProbeSnapshot | null,
  snapshot: ModelProbeSnapshot,
): ModelProbeSnapshot {
  if (
    previous?.requestId === snapshot.requestId &&
    (previous.requestCount > snapshot.requestCount ||
      previous.phase === "completed" ||
      previous.phase === "cancelled" ||
      (previous.phase === "cancelling" &&
        (snapshot.phase === "running" || snapshot.phase === "retrying")))
  )
    return previous;
  return snapshot;
}

export function ModelConnectivityTest({
  modelIds,
  ownedByById,
  disabled = false,
  searchId,
  onPrepare,
  onProbe,
  onStatus,
  onCancel,
  onBusyChange,
  resetVersion,
}: {
  modelIds: readonly string[];
  ownedByById?: Readonly<Record<string, string>>;
  disabled?: boolean;
  searchId: string;
  onPrepare?: () => boolean;
  onProbe: (modelId: string, requestId: string) => Promise<ModelProbeResult>;
  onStatus?: (requestId: string) => Promise<ModelProbeSnapshot>;
  onCancel?: (requestId: string) => Promise<ModelProbeSnapshot>;
  onBusyChange?: (busy: boolean) => void;
  resetVersion?: string | number;
}) {
  const visible = usePersistentVisibility();
  const originRef = useRef<HTMLElement | null>(null);
  const [open, setOpen] = useState(false);
  const [search, setSearch] = useState("");
  const [groupFilter, setGroupFilter] = useState<string | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [probing, setProbing] = useState(false);
  const [activeVersion, setActiveVersion] =
    useState<typeof resetVersion>(undefined);
  const [cancelling, setCancelling] = useState(false);
  const [progress, setProgress] = useState<ModelProbeSnapshot | null>(null);
  const activeRef = useRef<{
    requestId: string;
    version: typeof resetVersion;
    cancelPending: boolean;
  } | null>(null);
  const mountedRef = useRef(true);
  const cancelRef = useRef(onCancel);
  useEffect(() => {
    cancelRef.current = onCancel;
  }, [onCancel]);
  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      const active = activeRef.current;
      activeRef.current = null;
      if (active)
        void cancelRef.current?.(active.requestId).catch(() => undefined);
    };
  }, []);
  useEffect(() => {
    const active = activeRef.current;
    if (!visible || !probing || !onStatus || !active) return;
    let disposed = false;
    let timer: ReturnType<typeof setTimeout> | undefined;
    const poll = async () => {
      try {
        const snapshot = await onStatus(active.requestId);
        if (
          !disposed &&
          activeRef.current === active &&
          snapshot.requestId === active.requestId
        ) {
          setProgress((previous) => latestProbeSnapshot(previous, snapshot));
        }
      } catch {
        // Registration may still be queued. A failed read never fabricates a terminal state.
      }
      if (!disposed && activeRef.current === active)
        timer = setTimeout(() => void poll(), 500);
    };
    timer = setTimeout(() => void poll(), 100);
    return () => {
      disposed = true;
      if (timer !== undefined) clearTimeout(timer);
    };
  }, [probing, onStatus, visible]);
  const [noticeState, setNoticeState] = useState<{
    notice: Notice;
    version: string | number | undefined;
  } | null>(null);
  const notice =
    noticeState && noticeState.version === resetVersion
      ? noticeState.notice
      : null;
  const setNotice = (next: Notice | null) => {
    setNoticeState(
      next === null ? null : { notice: next, version: resetVersion },
    );
  };

  const groups = useMemo(() => groupModelIds(modelIds), [modelIds]);
  const groupedIds = useMemo(() => {
    if (!groupFilter) return [...modelIds];
    return modelIds.filter((id) => classifyModelType(id) === groupFilter);
  }, [groupFilter, modelIds]);
  const visibleIds = useMemo(
    () => filterModelIds(groupedIds, search),
    [groupedIds, search],
  );

  if (modelIds.length === 0) return null;

  const resetPicker = () => {
    setSearch("");
    setGroupFilter(null);
    setSelectedId(null);
  };

  const openDialog = () => {
    if (!visible || disabled || activeRef.current) return;
    if (onPrepare && !onPrepare()) return;
    resetPicker();
    setOpen(true);
  };

  const closeDialog = (nextOpen: boolean) => {
    if (probing) return;
    setOpen(nextOpen);
    if (!nextOpen) resetPicker();
  };

  const runProbe = async () => {
    if (!visible || !selectedId || activeRef.current || disabled) return;
    const active = {
      requestId: crypto.randomUUID(),
      version: resetVersion,
      cancelPending: false,
    };
    activeRef.current = active;
    setActiveVersion(active.version);
    setProbing(true);
    setCancelling(false);
    setProgress(null);
    onBusyChange?.(true);
    setNotice(null);
    try {
      const result = await onProbe(selectedId, active.requestId);
      if (!mountedRef.current || activeRef.current !== active) return;
      if (result.requestId !== active.requestId) {
        throw new Error("模型测试结果标识不匹配，请重新测试。");
      }
      setNoticeState({
        notice: noticeFromModelProbe(result),
        version: active.version,
      });
    } catch (error) {
      if (!mountedRef.current || activeRef.current !== active) return;
      setNoticeState({
        notice: {
          tone: "error",
          title: "连通测试失败",
          description:
            error instanceof Error && error.message.trim()
              ? error.message
              : "请检查地址、凭据、模型和服务状态后重试。",
        },
        version: active.version,
      });
    } finally {
      if (mountedRef.current && activeRef.current === active) {
        activeRef.current = null;
        setProbing(false);
        setCancelling(false);
        setProgress(null);
        onBusyChange?.(false);
      }
    }
  };

  const cancelProbe = async () => {
    const active = activeRef.current;
    if (!active || !onCancel || active.cancelPending) return;
    active.cancelPending = true;
    setCancelling(true);
    try {
      const snapshot = await onCancel(active.requestId);
      if (
        mountedRef.current &&
        activeRef.current === active &&
        snapshot.requestId === active.requestId
      ) {
        setProgress((previous) => latestProbeSnapshot(previous, snapshot));
        // Await the original native probe result; cancel acknowledgement alone is not completion.
      }
    } catch {
      if (mountedRef.current && activeRef.current === active) {
        active.cancelPending = false;
        setCancelling(false);
        setNoticeState({
          notice: {
            tone: "warning",
            title: "取消尚未确认",
            description: "后台测试仍可能运行，可再次取消；请等待测试终态。",
          },
          version: active.version,
        });
      }
    }
  };

  const progressVisible = probing && activeVersion === resetVersion;

  return (
    <div className="fy-models-probe">
      <Button
        dialogOriginRef={originRef}
        disabled={disabled || probing}
        onClick={openDialog}
      >
        {probing ? "测试中…" : "测试连通"}
      </Button>
      <Dialog
        open={open}
        originRef={originRef}
        onOpenChange={closeDialog}
        size="wide"
        title="选择要测试的模型"
        description="使用约 1024 token 的估算兼容输入；每次测试先发 1 次请求，仅请求超时自动重试 1 次，最多 2 次。每次请求都可能产生用量；这不是实际费用测量。"
        actions={
          <>
            <Button disabled={probing} onClick={() => closeDialog(false)}>
              关闭
            </Button>
            {probing && onCancel ? (
              <Button disabled={cancelling} onClick={() => void cancelProbe()}>
                {cancelling ? "正在取消…" : "取消测试"}
              </Button>
            ) : null}
            <Button
              className="fy-control-button-primary"
              disabled={probing || !selectedId}
              onClick={() => void runProbe()}
            >
              {probing ? "测试中…" : "开始测试"}
            </Button>
          </>
        }
      >
        <ModelSearchField
          id={searchId}
          label="搜索模型"
          value={search}
          onChange={setSearch}
        />
        <div
          className="fy-models-probe-filters"
          role="toolbar"
          aria-label="按分组过滤"
        >
          <PressableButton
            type="button"
            className={classNames(
              "fy-models-probe-filter",
              groupFilter === null && "fy-models-probe-filter-active",
            )}
            aria-pressed={groupFilter === null}
            onClick={() => setGroupFilter(null)}
          >
            全部
          </PressableButton>
          {groups.map((group) => (
            <PressableButton
              key={group.type}
              type="button"
              className={classNames(
                "fy-models-probe-filter",
                groupFilter === group.type && "fy-models-probe-filter-active",
              )}
              aria-pressed={groupFilter === group.type}
              onClick={() => setGroupFilter(group.type)}
            >
              {group.type}
              <span className="fy-models-group-count">{group.ids.length}</span>
            </PressableButton>
          ))}
        </div>
        <div className="fy-models-probe-list">
          <GroupedModelChips
            ids={visibleIds}
            selectedId={selectedId ?? undefined}
            onSelect={setSelectedId}
            ownedByById={ownedByById}
            emptyLabel={
              search.trim() || groupFilter
                ? "没有匹配的模型 ID"
                : "没有可测试的模型"
            }
          />
        </div>
        {progressVisible ? (
          <p role="status">
            {cancelling || progress?.phase === "cancelling"
              ? "正在取消，等待后台终态…"
              : progress?.phase === "retrying"
                ? "首次请求超时，正在进行第 2 次请求；可能产生额外用量。"
                : "正在测试，等待模型首块响应…"}
            {progress
              ? ` 已发起 ${progress.requestCount} 次请求。`
              : " 请求次数等待后台确认。"}
          </p>
        ) : null}
        <FieldFeedback id={`${searchId}-probe-dialog`} notice={notice} />
      </Dialog>
      <FieldFeedback
        id={`${searchId}-probe-result`}
        notice={open ? null : notice}
      />
    </div>
  );
}
