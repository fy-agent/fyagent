import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import {
  AlertTriangle,
  CheckCircle2,
  Circle,
  Loader2,
  ShieldCheck,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import type {
  ApplyChangePlanOutcome,
  ChangeJobSnapshot,
  ChangePlan,
} from "@/lib/api/change-plan";
import {
  isTerminalChangeJob,
  useApplyChangePlan,
  useChangeJob,
  useCreateCodexProviderSwitchPlan,
} from "@/lib/query/change-plan";

interface ChangePlanFlowProps {
  open: boolean;
  targetProviderId: string | null;
  onOpenChange: (open: boolean) => void;
  onTerminal?: (job: ChangeJobSnapshot) => void | Promise<void>;
}

function isStaleOutcome(outcome?: ApplyChangePlanOutcome): boolean {
  return (
    outcome?.kind === "rejected" &&
    ["stale", "expired", "consumed", "invalid_digest"].includes(
      outcome.errorCode ?? "unknown",
    )
  );
}

function resultKey(status: ChangeJobSnapshot["status"]): string {
  switch (status) {
    case "succeeded":
      return "changePlan.result.succeeded";
    case "warning":
      return "changePlan.result.warning";
    case "failed":
      return "changePlan.result.failed";
    default:
      return "changePlan.result.unknown";
  }
}

function stepKey(kind: ChangeJobSnapshot["steps"][number]["kind"]): string {
  switch (kind) {
    case "precheck":
      return "changePlan.step.precheck";
    case "apply":
      return "changePlan.step.apply";
    case "readback":
      return "changePlan.step.readback";
    case "reconcile":
      return "changePlan.step.reconcile";
    default:
      return "changePlan.step.unknown";
  }
}

function resourceKey(
  kind: ChangeJobSnapshot["resources"][number]["kind"],
): string {
  switch (kind) {
    case "provider_db_current":
      return "changePlan.resource.provider_db_current";
    case "device_current":
      return "changePlan.resource.device_current";
    case "target_definition":
      return "changePlan.resource.target_definition";
    case "codex_live_projection":
      return "changePlan.resource.codex_live_projection";
    default:
      return "changePlan.resource.unknown";
  }
}

function resourceStatusKey(
  status: ChangeJobSnapshot["resources"][number]["status"],
): string {
  switch (status) {
    case "pending":
      return "changePlan.resourceStatus.pending";
    case "matched":
      return "changePlan.resourceStatus.matched";
    case "mismatched":
      return "changePlan.resourceStatus.mismatched";
    case "unavailable":
      return "changePlan.resourceStatus.unavailable";
    default:
      return "changePlan.resourceStatus.unknown";
  }
}

export function ChangePlanFlow({
  open,
  targetProviderId,
  onOpenChange,
  onTerminal,
}: ChangePlanFlowProps) {
  const { t } = useTranslation();
  const createPlan = useCreateCodexProviderSwitchPlan();
  const applyPlan = useApplyChangePlan();
  const [plan, setPlan] = useState<ChangePlan>();
  const [outcome, setOutcome] = useState<ApplyChangePlanOutcome>();
  const [jobId, setJobId] = useState<string>();
  const confirmRef = useRef<HTMLButtonElement>(null);
  const notifiedTerminalRef = useRef<string>();
  const jobQuery = useChangeJob(jobId);
  const job = jobQuery.data ?? outcome?.job;

  useEffect(() => {
    if (!open || !targetProviderId) return;
    setPlan(undefined);
    setOutcome(undefined);
    setJobId(undefined);
    createPlan
      .mutateAsync(targetProviderId)
      .then(setPlan)
      .catch(() => undefined);
    // mutateAsync is stable for the lifetime of this hook instance.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, targetProviderId]);

  useEffect(() => {
    if (!job || !isTerminalChangeJob(job)) return;
    const key = `${job.jobId}:${job.revision}`;
    if (notifiedTerminalRef.current === key) return;
    notifiedTerminalRef.current = key;
    void onTerminal?.(job);
  }, [job, onTerminal]);

  const expired = !!plan && plan.expiresAt * 1000 <= Date.now();
  const stale = expired || isStaleOutcome(outcome);
  const running = applyPlan.isPending || (!!job && !isTerminalChangeJob(job));
  const loadFailed = createPlan.isError && !plan;

  useEffect(() => {
    if (open && plan && !job && !stale) confirmRef.current?.focus();
  }, [job, open, plan, stale]);

  const title = useMemo(() => {
    if (running) return t("changePlan.runningTitle");
    if (stale) return t("changePlan.staleTitle");
    if (job) return t(resultKey(job.status));
    return t("changePlan.previewTitle");
  }, [job, running, stale, t]);

  const handleApply = async () => {
    if (!plan || expired) return;
    try {
      const next = await applyPlan.mutateAsync({
        planId: plan.planId,
        planDigest: plan.planDigest,
      });
      setOutcome(next);
      if (next.job) setJobId(next.job.jobId);
    } catch {
      // Mutation state owns the safe, localized failure surface.
    }
  };

  const handleReplan = async () => {
    if (!targetProviderId) return;
    setOutcome(undefined);
    setJobId(undefined);
    try {
      setPlan(await createPlan.mutateAsync(targetProviderId));
    } catch {
      // Mutation state owns the safe, localized failure surface.
    }
  };

  const canClose = !running;

  return (
    <Dialog open={open} onOpenChange={(next) => canClose && onOpenChange(next)}>
      <DialogContent aria-busy={running}>
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
          <DialogDescription>{t("changePlan.description")}</DialogDescription>
        </DialogHeader>

        <div className="space-y-4 overflow-y-auto px-6 py-5">
          {createPlan.isPending && !plan && (
            <StatusLine icon={<Loader2 className="h-4 w-4 animate-spin" />}>
              {t("changePlan.loading")}
            </StatusLine>
          )}

          {loadFailed && (
            <Notice tone="danger" title={t("changePlan.unsupportedTitle")}>
              {t("changePlan.unsupportedDescription")}
            </Notice>
          )}

          {plan && !job && !stale && (
            <>
              <div className="rounded-lg border border-border-default bg-muted/20 p-4">
                <p className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
                  {t("changePlan.targetLabel")}
                </p>
                <p className="mt-1 font-semibold">{plan.targetProviderName}</p>
                <p className="mt-3 text-sm text-muted-foreground">
                  {t("changePlan.previewBody")}
                </p>
              </div>
              <Notice tone="neutral" title={t("changePlan.evidenceTitle")}>
                {t("changePlan.evidenceNotObserved")}
              </Notice>
            </>
          )}

          {stale && (
            <Notice tone="warning" title={t("changePlan.staleTitle")}>
              {t("changePlan.staleDescription")}
            </Notice>
          )}

          {job && (
            <>
              <ol className="space-y-2" aria-label={t("changePlan.stepsLabel")}>
                {job.steps.map((step) => (
                  <li
                    key={step.kind}
                    className="flex items-center gap-2 text-sm"
                  >
                    {step.status === "running" ? (
                      <Loader2 className="h-4 w-4 animate-spin text-primary" />
                    ) : step.status === "succeeded" ? (
                      <CheckCircle2 className="h-4 w-4 text-emerald-600" />
                    ) : step.status === "failed" ? (
                      <AlertTriangle className="h-4 w-4 text-destructive" />
                    ) : (
                      <Circle className="h-4 w-4 text-muted-foreground" />
                    )}
                    <span>{t(stepKey(step.kind))}</span>
                  </li>
                ))}
              </ol>

              {isTerminalChangeJob(job) && (
                <div className="space-y-2">
                  {job.resources.map((resource) => (
                    <div
                      key={resource.kind}
                      className="flex items-center justify-between gap-3 rounded-md border border-border-default px-3 py-2 text-sm"
                    >
                      <span>{t(resourceKey(resource.kind))}</span>
                      <span
                        className={
                          resource.status === "matched"
                            ? "text-emerald-600"
                            : "text-destructive"
                        }
                      >
                        {t(resourceStatusKey(resource.status))}
                      </span>
                    </div>
                  ))}
                </div>
              )}

              {job.restartRequirement === "recommended" && (
                <Notice tone="warning" title={t("changePlan.restartTitle")}>
                  {t("changePlan.restartDescription")}
                </Notice>
              )}
              {job.recoveryState === "recovery_required" && (
                <Notice tone="danger" title={t("changePlan.recoveryTitle")}>
                  {t("changePlan.recoveryDescription")}
                </Notice>
              )}
              <Notice tone="neutral" title={t("changePlan.evidenceTitle")}>
                {t("changePlan.evidenceNotObserved")}
              </Notice>
            </>
          )}

          {applyPlan.isError && !outcome && (
            <Notice tone="danger" title={t("changePlan.applyFailedTitle")}>
              {t("changePlan.applyFailedDescription")}
            </Notice>
          )}
        </div>

        <DialogFooter>
          {stale ? (
            <Button
              onClick={() => void handleReplan()}
              disabled={createPlan.isPending}
              autoFocus
            >
              {t("changePlan.replan")}
            </Button>
          ) : job && isTerminalChangeJob(job) ? (
            <Button onClick={() => onOpenChange(false)} autoFocus>
              {t("changePlan.close")}
            </Button>
          ) : (
            <>
              <Button
                variant="outline"
                onClick={() => onOpenChange(false)}
                disabled={running}
              >
                {t("changePlan.cancel")}
              </Button>
              <Button
                ref={confirmRef}
                onClick={() => void handleApply()}
                disabled={!plan || running || loadFailed}
                autoFocus
              >
                {running ? t("changePlan.applying") : t("changePlan.confirm")}
              </Button>
            </>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function StatusLine({
  icon,
  children,
}: {
  icon: ReactNode;
  children: ReactNode;
}) {
  return (
    <div className="flex items-center gap-2 text-sm text-muted-foreground">
      {icon}
      {children}
    </div>
  );
}

function Notice({
  tone,
  title,
  children,
}: {
  tone: "neutral" | "warning" | "danger";
  title: string;
  children: ReactNode;
}) {
  const classes =
    tone === "danger"
      ? "border-destructive/40 bg-destructive/5"
      : tone === "warning"
        ? "border-amber-500/40 bg-amber-500/5"
        : "border-border-default bg-muted/20";
  return (
    <div className={`rounded-lg border p-3 ${classes}`}>
      <div className="flex items-center gap-2 text-sm font-medium">
        {tone === "neutral" ? (
          <ShieldCheck className="h-4 w-4" />
        ) : (
          <AlertTriangle className="h-4 w-4" />
        )}
        {title}
      </div>
      <p className="mt-1 text-sm text-muted-foreground">{children}</p>
    </div>
  );
}
