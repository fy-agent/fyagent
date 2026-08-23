import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import type {
  ChangeJobSnapshot,
  ChangePlan,
} from "../../../shared/features/change-plan";
import { isTerminalChangeJob } from "../../../shared/features/change-plan";
import type { ChangePlanPort } from "../../../shared/features/ports";
import {
  Button,
  Dialog,
  InlineNotice,
  Spinner,
} from "../../../shared/ui/primitives";
import {
  canRequestCancellation,
  JOB_STATUS_LABELS,
  manualRecoveryCopy,
  planErrorCopy,
  RESOURCE_LABELS,
  RESOURCE_STATUS_LABELS,
  RESTART_LABELS,
  resultCopy,
  STEP_LABELS,
  STEP_STATUS_LABELS,
  unknownPlanErrorCopy,
} from "./presentation";
import "./change-plan.css";

type SafeProvider = { id: string; name: string };

export type ChangePlanSwitchProps = {
  active: boolean;
  currentProviderId: string;
  providers: Record<string, SafeProvider>;
  port: ChangePlanPort;
  externalPlan?: ChangePlan | null;
  onExternalPlanConsumed?: () => void;
  onTerminal: () => void | Promise<void>;
};

type BusyState = "planning" | "applying" | "cancelling" | null;
type UiNotice = { tone: "info" | "warning" | "error"; text: string };

function PreviewSection({
  title,
  children,
}: {
  title: string;
  children: React.ReactNode;
}) {
  return (
    <section className="fy-change-plan-preview-section">
      <h4>{title}</h4>
      {children}
    </section>
  );
}

function ChangePlanPreview({ plan }: { plan: ChangePlan }) {
  const upsert = plan.operation === "codex_provider_upsert_and_switch";
  const readLabels = plan.adapter.readSet.map((kind) => RESOURCE_LABELS[kind]);
  const writeLabels = plan.adapter.writeSet.map(
    (kind) => RESOURCE_LABELS[kind],
  );
  return (
    <div className="fy-change-plan-preview" data-testid="change-plan-preview">
      <PreviewSection title="语义变化">
        <p>
          {upsert ? "保存" : "将 Codex 当前配置切换到"}{" "}
          <strong>{plan.targetProviderName}</strong>
          {upsert ? "，并设为当前配置。" : "。"}
        </p>
        <p>不会在应用流程中发送模型请求或主动验证网络。</p>
        <ol>
          {plan.businessSteps.map((step) => (
            <li key={step}>
              {step === "save_provider" ? "保存 Provider" : "设为当前 Provider"}
            </li>
          ))}
        </ol>
      </PreviewSection>
      {plan.credential ? (
        <PreviewSection title="凭据边界">
          <p>
            API Key 将保存到系统钥匙串（{plan.credential.secretRefDisplay}
            ）；数据库、事件和日志不保存明文或摘要。
          </p>
          <p>
            应用时仍会把明文投影到 Codex 自己的 auth/config 文件，这是 Codex CLI
            当前运行所需的本机边界。
          </p>
        </PreviewSection>
      ) : null}
      <PreviewSection title="风险与重启">
        <p>{RESTART_LABELS[plan.restartExpectation]}</p>
        <p>
          风险项：
          {plan.risks.length > 0
            ? `${plan.risks.length} 项本机配置变更`
            : "无额外风险项"}
        </p>
      </PreviewSection>
      <PreviewSection title="前置条件与范围">
        <p>执行前会重新核对当前基线；一旦漂移，旧计划不会写入。</p>
        <p>读取：{readLabels.join("、")}</p>
        <p>写入：{writeLabels.join("、")}</p>
      </PreviewSection>
      <PreviewSection title="恢复方式">
        <p>失败补偿由现有 Provider writer 管理，并以真实回读结果为准。</p>
        <p>无法证明结果时会停止自动处理并要求人工检查，不会重放写入。</p>
      </PreviewSection>
    </div>
  );
}

function ChangeJobWorkspace({
  job,
  busy,
  onCancel,
  onDismiss,
}: {
  job: ChangeJobSnapshot;
  busy: BusyState;
  onCancel: () => void;
  onDismiss: () => void;
}) {
  const terminal = isTerminalChangeJob(job.status);
  const manualActions = manualRecoveryCopy(job);
  return (
    <section
      className="fy-change-job-workspace"
      data-testid="change-job-workspace"
      data-status={job.status}
      aria-labelledby="fy-change-job-title"
    >
      <header className="fy-change-job-heading">
        <div>
          <span className="fy-change-job-kicker">本机 Change Plan</span>
          <h3 id="fy-change-job-title">配置应用进度</h3>
        </div>
        <strong className="fy-change-job-status" data-status={job.status}>
          {JOB_STATUS_LABELS[job.status]}
        </strong>
      </header>

      <ol className="fy-change-job-steps" aria-label="配置应用步骤">
        {job.steps.map((step) => (
          <li
            key={step.kind}
            data-status={step.status}
            aria-current={step.status === "running" ? "step" : undefined}
          >
            <span className="fy-change-job-step-mark" aria-hidden />
            <strong>{STEP_LABELS[step.kind]}</strong>
            <span>{STEP_STATUS_LABELS[step.status]}</span>
          </li>
        ))}
      </ol>

      <div className="fy-change-job-outcome" aria-live="polite">
        <strong>{resultCopy(job.resultCode)}</strong>
        <span>{RESTART_LABELS[job.restartRequirement]}</span>
        <span>
          {job.status === "succeeded" || job.status === "warning"
            ? "真实使用证据：配置已应用，尚无真实使用证据"
            : "真实使用证据：尚无真实使用证据；本机结果不代表 Agent 已真实使用"}
        </span>
      </div>

      <div className="fy-change-job-resources" aria-label="本机回读结果">
        {job.resources.map((resource) => (
          <div key={resource.kind} data-status={resource.status}>
            <span>{RESOURCE_LABELS[resource.kind]}</span>
            <strong>{RESOURCE_STATUS_LABELS[resource.status]}</strong>
          </div>
        ))}
      </div>

      {job.partialResult ? (
        <InlineNotice
          tone={job.recoveryState === "recovery_required" ? "error" : "warning"}
        >
          <strong>部分结果</strong>
          <span>
            已完成 {job.partialResult.succeededSteps.length} 步，已恢复{" "}
            {job.partialResult.compensatedSteps.length} 步， 未确认{" "}
            {job.partialResult.unverifiedSteps.length} 步。
          </span>
          {manualActions.map((action) => (
            <span key={action}>{action}</span>
          ))}
        </InlineNotice>
      ) : null}

      <div className="fy-change-job-actions">
        {canRequestCancellation(job) ? (
          <Button disabled={busy !== null} onClick={onCancel}>
            {busy === "cancelling" ? "正在请求取消…" : "写入前取消"}
          </Button>
        ) : null}
        {terminal ? <Button onClick={onDismiss}>收起结果</Button> : null}
      </div>
    </section>
  );
}

export function ChangePlanSwitch({
  active,
  currentProviderId,
  providers,
  port,
  externalPlan = null,
  onExternalPlanConsumed,
  onTerminal,
}: ChangePlanSwitchProps) {
  const [plan, setPlan] = useState<ChangePlan | null>(null);
  const [previewOpen, setPreviewOpen] = useState(false);
  const [job, setJob] = useState<ChangeJobSnapshot | null>(null);
  const [busy, setBusy] = useState<BusyState>(null);
  const [notice, setNotice] = useState<UiNotice | null>(null);
  const mountedRef = useRef(true);
  const planRef = useRef<ChangePlan | null>(null);
  const jobRef = useRef<ChangeJobSnapshot | null>(null);
  const terminalReportedRef = useRef<string | null>(null);
  const recoveryLoadedRef = useRef(false);
  const externalPlanHandledRef = useRef<string | null>(null);

  useEffect(() => {
    if (!active || !externalPlan) return;
    if (externalPlanHandledRef.current === externalPlan.planId) return;
    externalPlanHandledRef.current = externalPlan.planId;
    planRef.current = externalPlan;
    setPlan(externalPlan);
    setNotice(null);
    setPreviewOpen(true);
  }, [active, externalPlan]);

  const acceptSnapshot = useCallback((snapshot: ChangeJobSnapshot) => {
    if (!mountedRef.current) return;
    const expectedPlanId = planRef.current?.planId;
    if (expectedPlanId && snapshot.planId !== expectedPlanId) return;
    setJob((current) => {
      if (current && current.jobId !== snapshot.jobId) return current;
      if (
        current?.jobId === snapshot.jobId &&
        snapshot.eventSeq < current.eventSeq
      )
        return current;
      jobRef.current = snapshot;
      return snapshot;
    });
  }, []);

  useEffect(() => {
    mountedRef.current = true;
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void port
      .subscribeJobUpdates((event) => {
        if (disposed) return;
        const current = jobRef.current;
        if (!current || current.jobId !== event.jobId) return;
        if (event.eventSeq <= current.eventSeq) return;
        void port
          .getJob(event.jobId)
          .then(acceptSnapshot)
          .catch(() => undefined);
      })
      .then((cleanup) => {
        if (disposed) cleanup();
        else unlisten = cleanup;
      })
      .catch(() => undefined);
    return () => {
      disposed = true;
      mountedRef.current = false;
      unlisten?.();
    };
  }, [acceptSnapshot, port]);

  useEffect(() => {
    if (!active || recoveryLoadedRef.current) return;
    recoveryLoadedRef.current = true;
    void port
      .listRecoverableJobs()
      .then(
        (jobs) =>
          jobs.sort((left, right) => right.updatedAt - left.updatedAt)[0],
      )
      .then((candidate) => candidate && acceptSnapshot(candidate))
      .catch(() => undefined);
  }, [acceptSnapshot, active, port, providers]);

  useEffect(() => {
    if (!job || isTerminalChangeJob(job.status)) return;
    const timer = window.setInterval(() => {
      void port
        .getJob(job.jobId)
        .then(acceptSnapshot)
        .catch(() => undefined);
    }, 800);
    return () => window.clearInterval(timer);
  }, [acceptSnapshot, job, port]);

  useEffect(() => {
    if (!job || !isTerminalChangeJob(job.status)) return;
    if (terminalReportedRef.current === job.jobId) return;
    terminalReportedRef.current = job.jobId;
    void onTerminal();
  }, [job, onTerminal]);

  const options = useMemo(
    () =>
      Object.values(providers).sort((left, right) =>
        left.name.localeCompare(right.name, "zh-CN"),
      ),
    [providers],
  );
  const currentName = providers[currentProviderId]?.name ?? "尚未设置";

  const createPlan = async (provider: SafeProvider) => {
    if (busy !== null || provider.id === currentProviderId) return;
    setBusy("planning");
    setNotice(null);
    try {
      const next = await port.createCodexProviderSwitchPlan(provider.id);
      if (!mountedRef.current) return;
      planRef.current = next;
      setPlan(next);
      setPreviewOpen(true);
    } catch (error) {
      if (mountedRef.current)
        setNotice({ tone: "error", text: unknownPlanErrorCopy(error) });
    } finally {
      if (mountedRef.current) setBusy(null);
    }
  };

  const applyPlan = async () => {
    const approved = planRef.current;
    if (!approved || busy !== null) return;
    setPreviewOpen(false);
    setBusy("applying");
    setNotice({ tone: "info", text: "已确认计划，等待后端执行快照。" });
    try {
      const outcome = await port.apply(approved.planId, approved.planDigest);
      if (!mountedRef.current) return;
      if (outcome.kind === "rejected") {
        setNotice({ tone: "warning", text: planErrorCopy(outcome.errorCode) });
        if (
          ["expired", "stale", "invalid_digest", "plan_not_found"].includes(
            outcome.errorCode,
          )
        ) {
          planRef.current = null;
          setPlan(null);
        }
        return;
      }
      acceptSnapshot(outcome.job);
      setNotice(null);
    } catch (error) {
      if (mountedRef.current)
        setNotice({ tone: "error", text: unknownPlanErrorCopy(error) });
    } finally {
      if (mountedRef.current) setBusy(null);
    }
  };

  const cancelJob = async () => {
    const current = jobRef.current;
    if (!current || busy !== null) return;
    setBusy("cancelling");
    try {
      const outcome = await port.cancelJob(current.jobId);
      if (!mountedRef.current) return;
      setNotice({
        tone: outcome.accepted ? "info" : "warning",
        text: outcome.accepted
          ? "取消请求已在首笔写入前接受。"
          : outcome.code === "commit_point_passed"
            ? "写入临界点已经通过，将继续等待真实回读结果。"
            : "当前任务不能再取消，请以最终回读结果为准。",
      });
      const refreshed = await port.getJob(current.jobId);
      acceptSnapshot(refreshed);
    } catch {
      if (mountedRef.current)
        setNotice({
          tone: "error",
          text: "无法确认取消结果，请继续等待任务回读。",
        });
    } finally {
      if (mountedRef.current) setBusy(null);
    }
  };

  const dismissJob = () => {
    jobRef.current = null;
    planRef.current = null;
    setJob(null);
    setPlan(null);
    setNotice(null);
    onExternalPlanConsumed?.();
  };

  return (
    <section
      className="fy-change-plan-switch"
      aria-labelledby="fy-change-plan-switch-title"
    >
      <div className="fy-change-plan-switch-heading">
        <div>
          <h3 id="fy-change-plan-switch-title">切换已有 Codex 配置</h3>
          <p>先生成无副作用计划，确认一次后再写入并真实回读。</p>
        </div>
        <span>当前：{currentName}</span>
      </div>

      {options.length === 0 ? (
        <InlineNotice>暂无可用于切换的已有 Provider。</InlineNotice>
      ) : (
        <div className="fy-change-plan-provider-list">
          {options.map((provider) => {
            const current = provider.id === currentProviderId;
            return (
              <div key={provider.id} data-current={current ? "true" : "false"}>
                <span>
                  <strong>{provider.name}</strong>
                  {current ? <small>当前配置</small> : null}
                </span>
                <Button
                  disabled={current || busy !== null}
                  onClick={() => void createPlan(provider)}
                >
                  {busy === "planning" && !current
                    ? "正在生成…"
                    : current
                      ? "正在使用"
                      : "预览切换"}
                </Button>
              </div>
            );
          })}
        </div>
      )}

      {notice ? (
        <InlineNotice tone={notice.tone}>{notice.text}</InlineNotice>
      ) : null}
      {busy === "applying" && !job ? (
        <Spinner label="等待配置应用快照" />
      ) : null}
      {job ? (
        <ChangeJobWorkspace
          job={job}
          busy={busy}
          onCancel={() => void cancelJob()}
          onDismiss={dismissJob}
        />
      ) : null}

      <Dialog
        open={previewOpen && plan !== null}
        onOpenChange={(open) => busy === null && setPreviewOpen(open)}
        title={
          plan?.operation === "codex_provider_upsert_and_switch"
            ? "确认保存 Codex 配置"
            : "确认 Codex 配置切换"
        }
        description="确认的是这一张已绑定当前基线的计划；配置漂移或过期后不会继续写入。"
        large
        actions={
          <>
            <Button
              disabled={busy !== null}
              onClick={() => setPreviewOpen(false)}
            >
              返回检查
            </Button>
            <Button
              className="fy-control-button-primary"
              disabled={busy !== null || plan === null}
              onClick={() => void applyPlan()}
            >
              确认并应用一次
            </Button>
          </>
        }
      >
        {plan ? <ChangePlanPreview plan={plan} /> : null}
      </Dialog>
    </section>
  );
}
