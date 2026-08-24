import { useEffect, useMemo, useRef, useState } from "react";

import type {
  ChangeJobSnapshot,
  ChangePlan,
  ChangePlanErrorCode,
} from "../../../shared/features/change-plans";
import { useFeatures } from "../../../shared/features/provider";
import { useRecoverableChangeJobs } from "../../../shared/features/queries";
import type { ProviderSummaryMap } from "../../../shared/features/types";
import { Button, InlineNotice } from "../../../shared/ui/primitives";
import { ApplyWorkspace } from "./ApplyWorkspace";
import {
  changePlanErrorCode,
  isActiveJobStatus,
  JOB_REFRESH_INTERVAL_MS,
} from "./changePlanErrors";

export function ChangePlanWorkspace({
  active,
  providers,
  currentId,
}: {
  active: boolean;
  providers: ProviderSummaryMap;
  currentId: string;
}) {
  const { ports } = useFeatures();
  const targets = useMemo(
    () =>
      Object.values(providers).filter((provider) => provider.id !== currentId),
    [currentId, providers],
  );
  const [targetId, setTargetId] = useState("");
  const [plan, setPlan] = useState<ChangePlan | null>(null);
  const [job, setJob] = useState<ChangeJobSnapshot | null>(null);
  const [error, setError] = useState<{
    code: ChangePlanErrorCode;
    message?: string;
  } | null>(null);
  const [busy, setBusy] = useState(false);
  const requestRevision = useRef(0);
  const recoverableJobs = useRecoverableChangeJobs(active);
  const effectiveTargetId = targets.some((target) => target.id === targetId)
    ? targetId
    : (targets[0]?.id ?? "");
  const visiblePlan =
    plan?.targetProviderId === effectiveTargetId ? plan : null;
  const visibleJob = job?.targetProviderId === effectiveTargetId ? job : null;

  const selectTarget = (nextTargetId: string) => {
    requestRevision.current += 1;
    setTargetId(nextTargetId);
    setPlan(null);
    setJob(null);
    setError(null);
  };

  const createPlan = async () => {
    if (!effectiveTargetId || busy) return;
    const revision = ++requestRevision.current;
    setBusy(true);
    setPlan(null);
    setJob(null);
    setError(null);
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
      if (requestRevision.current === revision) setBusy(false);
    }
  };

  const applyPlan = async (input: {
    readonly planId: string;
    readonly planDigest: string;
  }) => {
    if (busy) return;
    const revision = ++requestRevision.current;
    setBusy(true);
    setError(null);
    try {
      const outcome = await ports.changePlans.applyChangePlan(input);
      if (requestRevision.current !== revision) return;
      if (outcome.kind === "rejected") {
        setError({ code: outcome.errorCode });
        return;
      }
      setPlan((current) =>
        current?.planId === input.planId
          ? { ...current, status: "consumed" }
          : current,
      );
      setJob(outcome.job);
      const refreshed = await ports.changePlans.getChangeJob(outcome.job.jobId);
      if (requestRevision.current === revision) setJob(refreshed);
    } catch (cause) {
      if (requestRevision.current === revision)
        setError({ code: changePlanErrorCode(cause) });
    } finally {
      if (requestRevision.current === revision) setBusy(false);
    }
  };

  useEffect(() => {
    const jobId = visibleJob?.jobId;
    const status = visibleJob?.status;
    if (busy || !jobId || !status || !isActiveJobStatus(status)) return;
    const revision = requestRevision.current;
    let disposed = false;
    const timer = window.setInterval(() => {
      void (async () => {
        try {
          const refreshed = await ports.changePlans.getChangeJob(jobId);
          if (disposed || requestRevision.current !== revision) return;
          setJob(refreshed);
        } catch (cause) {
          if (disposed || requestRevision.current !== revision) return;
          setError({ code: changePlanErrorCode(cause) });
          window.clearInterval(timer);
        }
      })();
    }, JOB_REFRESH_INTERVAL_MS);
    return () => {
      disposed = true;
      window.clearInterval(timer);
    };
  }, [busy, ports.changePlans, visibleJob?.jobId, visibleJob?.status]);

  return (
    <section
      className="fy-models-section"
      aria-label="Change Plan Provider 切换"
    >
      <h3>切换已保存的 Provider</h3>
      <p className="fy-models-muted">
        先生成零写入预览，再单次确认应用。当前 Provider 不会列为目标。
      </p>
      {(recoverableJobs.data?.length ?? 0) > 0 ? (
        <InlineNotice tone="warning">
          检测到 {recoverableJobs.data?.length ?? 0} 个可恢复 Change
          Job，已执行只读回读；不会重放写入。
        </InlineNotice>
      ) : null}
      {targets.length > 0 ? (
        <div className="fy-models-inline-fields">
          <label className="fy-control-field">
            <span>目标 Provider</span>
            <select
              value={effectiveTargetId}
              onChange={(event) => selectTarget(event.target.value)}
              disabled={busy}
            >
              {targets.map((target) => (
                <option key={target.id} value={target.id}>
                  {target.name}
                </option>
              ))}
            </select>
          </label>
          <Button
            disabled={!effectiveTargetId || busy}
            onClick={() => void createPlan()}
          >
            {busy && !visiblePlan ? "正在生成…" : "生成切换计划"}
          </Button>
        </div>
      ) : (
        <InlineNotice tone="info">没有可切换的已保存 Provider。</InlineNotice>
      )}

      {visiblePlan || visibleJob || error ? (
        <ApplyWorkspace
          plan={visiblePlan}
          job={visibleJob}
          busy={busy}
          error={error}
          onConfirm={applyPlan}
          onRegenerate={() => void createPlan()}
          onClose={() => {
            requestRevision.current += 1;
            setPlan(null);
            setJob(null);
            setError(null);
            setBusy(false);
          }}
        />
      ) : null}
    </section>
  );
}
