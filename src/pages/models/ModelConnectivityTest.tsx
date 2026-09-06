import { useMemo, useRef, useState } from "react";

import { classNames } from "../../shared/design-system/classNames";
import type { ModelProbeResult } from "../../shared/features/types";
import { Button } from "../../shared/ui/Button";
import { Dialog } from "../../shared/ui/Dialog";
import { FieldFeedback, type Notice } from "./feedback";
import { GroupedModelChips, ModelSearchField } from "./modelChips";
import { noticeFromModelProbe } from "./modelsShared";
import {
  classifyModelType,
  filterModelIds,
  groupModelIds,
} from "./workBuddyModels";

export function ModelConnectivityTest({
  modelIds,
  ownedByById,
  disabled = false,
  searchId,
  onPrepare,
  onProbe,
  onBusyChange,
  resetVersion,
}: {
  modelIds: readonly string[];
  ownedByById?: Readonly<Record<string, string>>;
  disabled?: boolean;
  searchId: string;
  onPrepare?: () => boolean;
  onProbe: (modelId: string) => Promise<ModelProbeResult>;
  onBusyChange?: (busy: boolean) => void;
  resetVersion?: string | number;
}) {
  const originRef = useRef<HTMLElement | null>(null);
  const [open, setOpen] = useState(false);
  const [search, setSearch] = useState("");
  const [groupFilter, setGroupFilter] = useState<string | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [probing, setProbing] = useState(false);
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
    if (disabled || probing) return;
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
    if (!selectedId || probing || disabled) return;
    setProbing(true);
    onBusyChange?.(true);
    setNotice(null);
    try {
      const result = await onProbe(selectedId);
      setNotice(noticeFromModelProbe(result));
    } catch (error) {
      setNotice({
        tone: "error",
        title: "连通测试失败",
        description:
          error instanceof Error && error.message.trim()
            ? error.message
            : "请检查地址、凭据、模型和服务状态后重试。",
      });
    } finally {
      setProbing(false);
      onBusyChange?.(false);
    }
  };

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
        description="测试会向所选模型发送一条简短请求，可能产生少量用量。完成后会显示响应或错误。"
        actions={
          <>
            <Button disabled={probing} onClick={() => closeDialog(false)}>
              关闭
            </Button>
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
          <button
            type="button"
            className={classNames(
              "fy-models-probe-filter",
              groupFilter === null && "fy-models-probe-filter-active",
            )}
            aria-pressed={groupFilter === null}
            onClick={() => setGroupFilter(null)}
          >
            全部
          </button>
          {groups.map((group) => (
            <button
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
            </button>
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
        <FieldFeedback id={`${searchId}-probe-dialog`} notice={notice} />
      </Dialog>
      <FieldFeedback
        id={`${searchId}-probe-result`}
        notice={open ? null : notice}
      />
    </div>
  );
}
