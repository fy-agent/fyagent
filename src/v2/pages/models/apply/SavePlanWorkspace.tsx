import { useCallback, useEffect, useRef, useState } from "react";

import type {
  ChangeJobSnapshot,
  ChangePlan,
  ChangePlanErrorCode,
} from "../../../shared/features/change-plans";
import { useFeatures } from "../../../shared/features/provider";
import { usePersistentVisibility } from "../../../shared/ui/PersistentSurface";
import { ApplyWorkspace } from "./ApplyWorkspace";
import { changePlanErrorCode, isActiveJobStatus } from "./changePlanErrors";
import { useChangeJob } from "./useChangeJob";

export interface SavePlanWorkspaceProps<Request> {
  active: boolean;
  request: Request | null;
  plan: ChangePlan | null;
  previewError: { code: ChangePlanErrorCode; message?: string } | null;
  onPlanChange: (plan: ChangePlan | null) => void;
  onTerminal: (job: ChangeJobSnapshot) => void;
  onDismiss: () => void;
}

export function SavePlanWorkspace<Request>({
  active,
  request,
  plan,
  previewError,
  onPlanChange,
  onTerminal,
  onDismiss,
  create,
  label,
  title,
  description,
}: SavePlanWorkspaceProps<Request> & {
  create: (request: Request) => Promise<ChangePlan>;
  label: string;
  title: string;
  description: string;
}) {
  const { ports } = useFeatures();
  const visible = usePersistentVisibility();
  const [error, setError] = useState<{ code: ChangePlanErrorCode } | null>(
    null,
  );
  const [busy, setBusy] = useState(false);
  const busyRef = useRef(false);
  const requestRevision = useRef(0);
  const terminalNotified = useRef<string | null>(null);
  const {
    job,
    error: readError,
    setJob,
  } = useChangeJob(ports.changePlans, active && !busy);
  const displayError = error ?? readError ?? previewError;

  useEffect(
    () => () => {
      requestRevision.current += 1;
    },
    [],
  );

  const notifyTerminal = useCallback(
    (next: ChangeJobSnapshot) => {
      if (
        isActiveJobStatus(next.status) ||
        terminalNotified.current === next.jobId
      )
        return;
      terminalNotified.current = next.jobId;
      onTerminal(next);
    },
    [onTerminal],
  );

  useEffect(() => {
    if (job) notifyTerminal(job);
  }, [job, notifyTerminal]);

  const begin = () => {
    if (!active || !visible || busyRef.current) return null;
    busyRef.current = true;
    setBusy(true);
    setError(null);
    return ++requestRevision.current;
  };
  const finish = (revision: number) => {
    if (requestRevision.current !== revision) return;
    busyRef.current = false;
    setBusy(false);
  };

  const createPlan = async () => {
    if (!request) return;
    const revision = begin();
    if (revision === null) return;
    setJob(null);
    terminalNotified.current = null;
    try {
      const next = await create(request);
      if (requestRevision.current === revision) onPlanChange(next);
    } catch (cause) {
      if (requestRevision.current === revision)
        setError({ code: changePlanErrorCode(cause) });
    } finally {
      finish(revision);
    }
  };

  const applyPlan = async (input: {
    readonly planId: string;
    readonly planDigest: string;
  }) => {
    const revision = begin();
    if (revision === null) return;
    try {
      const outcome = await ports.changePlans.applyChangePlan(input);
      if (requestRevision.current !== revision) return;
      if (outcome.kind === "rejected") {
        setError({ code: outcome.errorCode });
        return;
      }
      onPlanChange(
        plan?.planId === input.planId ? { ...plan, status: "consumed" } : plan,
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
      finish(revision);
    }
  };

  if (!active || (!request && !plan && !displayError)) return null;
  return (
    <section className="fy-models-section" aria-label={label}>
      <h3>{title}</h3>
      <p className="fy-models-muted">{description}</p>
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
            busyRef.current = false;
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
