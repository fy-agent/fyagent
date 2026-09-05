import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import type {
  ChangePlan,
  ChangePlanErrorCode,
  ChangeJobSnapshot,
} from "../change-plans";
import { useFeatures } from "../provider";
import { useRecoverableChangeJobs } from "../queries";
import type { ProviderSummaryMap } from "../models";
import { Button } from "../../ui/Button";
import { InlineNotice } from "../../ui/primitives";
import { ApplyWorkspace } from "./ApplyWorkspace";
import { changePlanErrorCode, isActiveJobStatus } from "./changePlanErrors";
import { useChangeJob } from "./useChangeJob";
import { hasUnconfirmedAuthority } from "./view-model";

export function ChangePlanWorkspace({
  active,
  providers,
  currentId,
  disabled = false,
  onBusyChange,
  onTerminal,
}: {
  active: boolean;
  providers: ProviderSummaryMap;
  currentId: string;
  disabled?: boolean;
  onBusyChange?: (busy: boolean) => void;
  onTerminal?: (job: ChangeJobSnapshot) => Promise<void>;
}) {
  const { ports } = useFeatures();
  const targets = useMemo(
    () =>
      Object.values(providers).filter((provider) => provider.id !== currentId),
    [currentId, providers],
  );
  const [targetId, setTargetId] = useState("");
  const [plan, setPlan] = useState<ChangePlan | null>(null);
  const [error, setError] = useState<{
    code: ChangePlanErrorCode;
    message?: string;
  } | null>(null);
  const [busy, setBusy] = useState(false);
  const busyRef = useRef(false);
  const requestRevision = useRef(0);
  const [writePending, setWritePending] = useState(false);
  const [settlementError, setSettlementError] = useState(false);
  const [admissionUnknown, setAdmissionUnknown] = useState(false);
  const terminalSeen = useRef<string | null>(null);
  const {
    job,
    error: readError,
    setJob,
    refetch,
  } = useChangeJob(ports.changePlans, active && !busy);
  const displayError = error ?? readError;
  const unresolved =
    writePending ||
    admissionUnknown ||
    Boolean(readError || settlementError || (error && job));
  const locked = busy || disabled || unresolved;

  useEffect(() => {
    onBusyChange?.(unresolved);
  }, [unresolved, onBusyChange]);

  const reconcile = useCallback(
    async (next: ChangeJobSnapshot) => {
      if (isActiveJobStatus(next.status)) return;
      const revision = requestRevision.current;
      try {
        await onTerminal?.(next);
        if (revision !== requestRevision.current) return;
        const unconfirmed = hasUnconfirmedAuthority(next);
        setSettlementError(unconfirmed);
        setWritePending(unconfirmed);
        if (!unconfirmed) setError(null);
      } catch {
        if (revision === requestRevision.current) setSettlementError(true);
      }
    },
    [onTerminal],
  );

  useEffect(() => {
    if (busy || !job || isActiveJobStatus(job.status)) return;
    const key = `${job.jobId}:${job.revision}`;
    if (terminalSeen.current === key) return;
    terminalSeen.current = key;
    void reconcile(job);
  }, [busy, job, reconcile]);
  useEffect(
    () => () => {
      requestRevision.current += 1;
    },
    [],
  );
  const recoverableJobs = useRecoverableChangeJobs(active);
  const effectiveTargetId = targets.some((target) => target.id === targetId)
    ? targetId
    : (targets[0]?.id ?? "");
  // The admitted operation owns its identity even after readback changes currentId.
  const visiblePlan = plan;
  const visibleJob = job;

  const selectTarget = (nextTargetId: string) => {
    if (locked) return;
    requestRevision.current += 1;
    setTargetId(nextTargetId);
    setPlan(null);
    setJob(null);
    setError(null);
  };

  const createPlan = async () => {
    if (!active || !effectiveTargetId || locked || busyRef.current) return;
    busyRef.current = true;
    const revision = ++requestRevision.current;
    setBusy(true);
    setPlan(null);
    setJob(null);
    setError(null);
    setSettlementError(false);
    terminalSeen.current = null;
    try {
      const nextPlan =
        await ports.changePlans.createCodexProviderSwitchPlan(
          effectiveTargetId,
        );
      if (requestRevision.current === revision) setPlan(nextPlan);
    } catch (cause) {
      if (requestRevision.current === revision)
        setError({ code: changePlanErrorCode(cause) });
    } finally {
      if (requestRevision.current === revision) {
        busyRef.current = false;
        setBusy(false);
      }
    }
  };

  const applyPlan = async (input: {
    readonly planId: string;
    readonly planDigest: string;
  }) => {
    if (!active || locked || busyRef.current) return;
    busyRef.current = true;
    const revision = ++requestRevision.current;
    setBusy(true);
    setError(null);
    setWritePending(true);
    onBusyChange?.(true);
    let admitted = false;
    try {
      const outcome = await ports.changePlans.applyChangePlan(input);
      if (requestRevision.current !== revision) return;
      if (outcome.kind === "rejected") {
        setError({ code: outcome.errorCode });
        setWritePending(false);
        return;
      }
      admitted = true;
      setPlan((current) =>
        current?.planId === input.planId
          ? { ...current, status: "consumed" }
          : current,
      );
      setJob(outcome.job);
      const refreshed = await ports.changePlans.getChangeJob(outcome.job.jobId);
      if (requestRevision.current === revision) setJob(refreshed);
    } catch (cause) {
      if (requestRevision.current === revision) {
        setError({ code: changePlanErrorCode(cause) });
        if (!admitted) setAdmissionUnknown(true);
      }
    } finally {
      if (requestRevision.current === revision) {
        busyRef.current = false;
        setBusy(false);
      }
    }
  };

  return (
    <section className="fy-source-switch" aria-label="切换已保存配置">
      <h3>切换已保存配置</h3>
      <p>选择 Codex 已保存的配置，检查后再切换。官方账号登录单独保留。</p>
      {(recoverableJobs.data?.length ?? 0) > 0 ? (
        <InlineNotice tone="warning">
          上次有 {recoverableJobs.data?.length ?? 0} 个配置操作未完成。FyAgent
          已检查当前设置，不会自动重复修改。
        </InlineNotice>
      ) : null}
      {targets.length > 0 ? (
        <div className="fy-source-switch-fields">
          <label className="fy-control-field">
            <span>切换到</span>
            <select
              value={effectiveTargetId}
              onChange={(event) => selectTarget(event.target.value)}
              className="fy-control-select"
              disabled={locked}
            >
              {targets.map((target) => (
                <option key={target.id} value={target.id}>
                  {target.name}
                </option>
              ))}
            </select>
          </label>
          <Button
            disabled={!effectiveTargetId || locked}
            onClick={() => void createPlan()}
          >
            {busy && !visiblePlan ? "正在生成预览…" : "预览更改"}
          </Button>
        </div>
      ) : (
        <InlineNotice tone="info">
          没有其他已保存配置。可在模型管理中添加。
        </InlineNotice>
      )}

      {admissionUnknown ? (
        <InlineNotice tone="warning">
          无法确认本次切换是否已开始，暂时停止后续更改。请重新打开 FyAgent
          检查配置。
        </InlineNotice>
      ) : null}
      {(readError || settlementError || (error && job)) && !admissionUnknown ? (
        <InlineNotice tone="warning">
          <p>暂时无法确认当前配置。重新检查前不能继续切换。</p>
          <Button
            disabled={busy || !active}
            onClick={() => {
              void (async () => {
                if (busyRef.current) return;
                busyRef.current = true;
                setBusy(true);
                const revision = requestRevision.current;
                try {
                  const result = await refetch();
                  if (revision !== requestRevision.current) return;
                  if (result.error || !result.data) return;
                  setError(null);
                  await reconcile(result.data);
                } finally {
                  if (revision === requestRevision.current) {
                    busyRef.current = false;
                    setBusy(false);
                  }
                }
              })();
            }}
          >
            重新检查切换结果
          </Button>
        </InlineNotice>
      ) : null}

      {visiblePlan || visibleJob || displayError ? (
        <ApplyWorkspace
          plan={visiblePlan}
          job={visibleJob}
          busy={locked}
          error={displayError}
          onConfirm={applyPlan}
          onRegenerate={() => void createPlan()}
          onClose={() => {
            if (locked) return;
            requestRevision.current += 1;
            busyRef.current = false;
            setPlan(null);
            setJob(null);
            setError(null);
            setBusy(false);
            terminalSeen.current = null;
          }}
        />
      ) : null}
    </section>
  );
}
