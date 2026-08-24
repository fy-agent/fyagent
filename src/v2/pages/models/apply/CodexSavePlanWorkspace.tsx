import { useCallback, useEffect, useRef, useState } from "react";

import type {
  ChangeJobSnapshot,
  ChangePlan,
  ChangePlanErrorCode,
} from "../../../shared/features/change-plans";
import type { ProviderQuickSetupRequest } from "../../../shared/features/models";
import { useFeatures } from "../../../shared/features/provider";
import { ApplyWorkspace } from "./ApplyWorkspace";
import {
  changePlanErrorCode,
  isActiveJobStatus,
  JOB_REFRESH_INTERVAL_MS,
} from "./changePlanErrors";

export function CodexSavePlanWorkspace({
  active,
  request,
  plan,
  previewError,
  onPlanChange,
  onTerminal,
  onDismiss,
}: {
  active: boolean;
  request: ProviderQuickSetupRequest | null;
  plan: ChangePlan | null;
  previewError: { code: ChangePlanErrorCode; message?: string } | null;
  onPlanChange: (plan: ChangePlan | null) => void;
  onTerminal: (job: ChangeJobSnapshot) => void;
  onDismiss: () => void;
}) {
  const { ports } = useFeatures();
  const [job, setJob] = useState<ChangeJobSnapshot | null>(null);
  const [error, setError] = useState<{
    code: ChangePlanErrorCode;
    message?: string;
  } | null>(null);
  const [busy, setBusy] = useState(false);
  const requestRevision = useRef(0);
  const terminalNotified = useRef<string | null>(null);
  const displayError = error ?? previewError;

  const notifyTerminal = useCallback(
    (nextJob: ChangeJobSnapshot) => {
      if (isActiveJobStatus(nextJob.status)) return;
      if (terminalNotified.current === nextJob.jobId) return;
      terminalNotified.current = nextJob.jobId;
      onTerminal(nextJob);
    },
    [onTerminal],
  );

  const createPlan = async () => {
    if (!request || busy) return;
    const revision = ++requestRevision.current;
    setBusy(true);
    setJob(null);
    setError(null);
    terminalNotified.current = null;
    try {
      const nextPlan =
        await ports.changePlans.createCodexProviderUpsertPlan(request);
      if (requestRevision.current !== revision) return;
      onPlanChange(nextPlan);
    } catch (cause) {
      if (requestRevision.current !== revision) return;
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
      onPlanChange(
        plan?.planId === input.planId && plan
          ? { ...plan, status: "consumed" }
          : plan,
      );
      setJob(outcome.job);
      notifyTerminal(outcome.job);
      const refreshed = await ports.changePlans.getChangeJob(outcome.job.jobId);
      if (requestRevision.current !== revision) return;
      setJob(refreshed);
      notifyTerminal(refreshed);
    } catch (cause) {
      if (requestRevision.current === revision)
        setError({ code: changePlanErrorCode(cause) });
    } finally {
      if (requestRevision.current === revision) setBusy(false);
    }
  };

  useEffect(() => {
    const jobId = job?.jobId;
    const status = job?.status;
    if (busy || !jobId || !status || !isActiveJobStatus(status)) return;
    const revision = requestRevision.current;
    let disposed = false;
    const timer = window.setInterval(() => {
      void (async () => {
        try {
          const refreshed = await ports.changePlans.getChangeJob(jobId);
          if (disposed || requestRevision.current !== revision) return;
          setJob(refreshed);
          notifyTerminal(refreshed);
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
  }, [busy, job?.jobId, job?.status, notifyTerminal, ports.changePlans]);

  if (!active || (!request && !plan && !displayError)) return null;

  return (
    <section
      className="fy-models-section"
      aria-label="Change Plan Provider 保存"
    >
      <h3>保存并设为当前配置</h3>
      <p className="fy-models-muted">
        先生成零写入预览，再单次确认。确认只发送计划身份，不会再次提交密钥。
      </p>
      {plan || job || displayError ? (
        <ApplyWorkspace
          plan={plan}
          job={job}
          busy={busy}
          error={displayError}
          onConfirm={applyPlan}
          onRegenerate={() => void createPlan()}
          onClose={() => {
            requestRevision.current += 1;
            setJob(null);
            setError(null);
            setBusy(false);
            onDismiss();
          }}
        />
      ) : null}
    </section>
  );
}
