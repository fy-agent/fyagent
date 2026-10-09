import { useRef, useState } from "react";
import { SKILL_TARGETS, type SkillTargetId } from "../directory";
import type {
  BulkAssignmentItem,
  BulkAssignmentPlan,
  BulkAssignmentResult,
} from "../bulk-assignment";
import { Button } from "../../ui/Button";
import { Dialog } from "../../ui/Dialog";
import type { DialogOriginRef } from "../../ui/dialogOrigin";
import { Checkbox, InlineNotice } from "../../ui/primitives";
import { FeatureSearch } from "../../ui/FeatureSearch";

const resultLabels = {
  confirmed: "完成：分配状态已读回",
  failed: "失败：可能存在部分写入，请刷新后重新预览",
  drift: "未执行：来源或分配状态已变化，请重新预览",
  read_only: "未执行：链接资源或目标只读",
};

export function BulkAssignmentDialog({
  items,
  kind,
  originRef,
  busy,
  onClose,
  onExecute,
}: {
  items: BulkAssignmentItem[];
  kind: "Skills" | "MCP";
  originRef: DialogOriginRef;
  busy: boolean;
  onClose(): void;
  onExecute(
    plan: BulkAssignmentPlan,
    onResult: (result: BulkAssignmentResult) => void,
  ): Promise<void>;
}) {
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [search, setSearch] = useState("");
  const [target, setTarget] = useState<SkillTargetId>(SKILL_TARGETS[0].id);
  const [enabled, setEnabled] = useState(true);
  const [plan, setPlan] = useState<BulkAssignmentPlan | null>(null);
  const [results, setResults] = useState<BulkAssignmentResult[]>([]);
  const [running, setRunning] = useState(false);
  const [error, setError] = useState(false);
  const pending = useRef(false);
  const locked = busy || running;
  const visible = items.filter((item) =>
    `${item.name} ${item.id}`
      .toLocaleLowerCase()
      .includes(search.trim().toLocaleLowerCase()),
  );
  const chosen = items.filter((item) => selected.has(item.id));
  const retry = () => {
    if (pending.current) return;
    setSelected(
      new Set(
        (plan?.items ?? [])
          .filter(
            (item) =>
              !results.some(
                (result) =>
                  result.id === item.id && result.status === "confirmed",
              ),
          )
          .map((item) => item.id),
      ),
    );
    setPlan(null);
    setResults([]);
    setError(false);
  };
  return (
    <Dialog
      open
      originRef={originRef}
      title={`${kind} 批量分配`}
      size="comfortable"
      description="只调整所选资源在一个软件中的分配。既有保护性写入仍由原生服务执行；结果不代表外部软件已加载。"
      onOpenChange={(open) => {
        if (!open && !locked && !pending.current) onClose();
      }}
      actions={
        <>
          <Button
            disabled={locked}
            onClick={() => {
              if (!pending.current) onClose();
            }}
          >
            关闭
          </Button>
          {plan ? (
            <>
              <Button
                disabled={locked}
                onClick={() => {
                  if (pending.current) return;
                  setPlan(null);
                  setResults([]);
                  setError(false);
                }}
              >
                重新选择
              </Button>
              {results.length > 0 || error ? (
                <Button disabled={locked} onClick={retry}>
                  选择未完成项重新预览
                </Button>
              ) : (
                <Button
                  className="fy-control-button-primary"
                  disabled={locked || !plan.items.length}
                  onClick={async () => {
                    if (pending.current || locked) return;
                    pending.current = true;
                    setRunning(true);
                    try {
                      const observed: BulkAssignmentResult[] = [];
                      await onExecute(plan, (result) => {
                        observed.push(result);
                        setResults((current) => [...current, result]);
                      });
                      if (observed.length !== plan.items.length) setError(true);
                    } catch {
                      setError(true);
                    } finally {
                      pending.current = false;
                      setRunning(false);
                    }
                  }}
                >
                  确认执行 · {plan.items.length}
                </Button>
              )}
            </>
          ) : (
            <Button
              disabled={locked || !chosen.length}
              onClick={() => {
                setPlan({
                  target,
                  enabled,
                  items: chosen.map((item) => ({
                    ...item,
                    apps: { ...item.apps },
                  })),
                });
              }}
            >
              预览所选 · {chosen.length}
            </Button>
          )}
        </>
      }
    >
      {plan ? (
        <>
          <p>
            目标：
            {
              SKILL_TARGETS.find((entry) => entry.id === plan.target)?.label
            } · {plan.enabled ? "启用" : "停用"} · {plan.items.length} 项
          </p>
          {error && (
            <InlineNotice tone="error">
              读取或执行失败；不能确认完成。请刷新后重新预览。
            </InlineNotice>
          )}
          <div
            className="fy-feature-list"
            aria-label="批量预览与结果"
            aria-live="polite"
          >
            {plan.items.map((item) => (
              <article key={item.id} className="fy-feature-card">
                <strong>{item.name}</strong>
                <p>{item.id}</p>
                <p>
                  {results.find((result) => result.id === item.id)
                    ? resultLabels[
                        results.find((result) => result.id === item.id)!.status
                      ]
                    : item.readOnlyTargets?.includes(plan.target)
                      ? "只读：执行时将拒绝"
                      : "等待确认"}
                </p>
              </article>
            ))}
          </div>
        </>
      ) : (
        <>
          <label className="fy-control-field">
            目标软件
            <select
              className="fy-control-select"
              disabled={locked}
              value={target}
              onChange={(event) =>
                setTarget(event.target.value as SkillTargetId)
              }
            >
              {SKILL_TARGETS.map((entry) => (
                <option key={entry.id} value={entry.id}>
                  {entry.label}
                </option>
              ))}
            </select>
          </label>
          <label className="fy-control-field">
            分配操作
            <select
              className="fy-control-select"
              disabled={locked}
              value={enabled ? "enable" : "disable"}
              onChange={(event) => setEnabled(event.target.value === "enable")}
            >
              <option value="enable">启用</option>
              <option value="disable">停用</option>
            </select>
          </label>
          <FeatureSearch
            value={search}
            onValueChange={setSearch}
            disabled={locked}
            ariaLabel="筛选批量资源"
            placeholder="按名称筛选"
          />
          <p>
            已选 {selected.size} 项 · 当前可用 {chosen.length} 项 · 筛选结果{" "}
            {visible.length} 项
          </p>
          <Button disabled={locked} onClick={() => setSelected(new Set())}>
            清空选择
          </Button>
          <Button
            disabled={locked || !visible.length}
            onClick={() =>
              setSelected(
                (current) =>
                  new Set([...current, ...visible.map((item) => item.id)]),
              )
            }
          >
            选择筛选结果
          </Button>
          <div className="fy-feature-list">
            {visible.map((item) => (
              <Checkbox
                key={item.id}
                label={`选择 ${item.name}`}
                checked={selected.has(item.id)}
                disabled={locked}
                onCheckedChange={(checked) =>
                  setSelected((current) => {
                    const next = new Set(current);
                    if (checked) next.add(item.id);
                    else next.delete(item.id);
                    return next;
                  })
                }
              />
            ))}
            {!visible.length && <p>没有筛选结果，已选项目仍保留。</p>}
          </div>
        </>
      )}
    </Dialog>
  );
}
