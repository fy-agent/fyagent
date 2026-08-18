import { CheckCircleIcon } from "@phosphor-icons/react/dist/csr/CheckCircle";
import { CircleIcon } from "@phosphor-icons/react/dist/csr/Circle";
import { CircleNotchIcon } from "@phosphor-icons/react/dist/csr/CircleNotch";
import { MinusCircleIcon } from "@phosphor-icons/react/dist/csr/MinusCircle";
import { WarningCircleIcon } from "@phosphor-icons/react/dist/csr/WarningCircle";
import { XCircleIcon } from "@phosphor-icons/react/dist/csr/XCircle";

import { applyFixtures, type ApplyScenario } from "./fixtures";
import {
  assertNever,
  createApplyViewModel,
  type ApplyPresentation,
  type ApplyStepPresentation,
  type ApplyStepStatus,
} from "./view-model";
import "./apply-workspace.css";

export type ApplyWorkspaceProps = {
  readonly scenario?: ApplyScenario;
};

function StepStatusIcon({ status }: { readonly status: ApplyStepStatus }) {
  switch (status) {
    case "succeeded":
      return <CheckCircleIcon className="fy-apply-icon" size={18} weight="fill" aria-hidden />;
    case "running":
      return (
        <CircleNotchIcon
          className="fy-apply-icon fy-apply-spin"
          size={18}
          weight="bold"
          aria-hidden
        />
      );
    case "warning":
      return <WarningCircleIcon className="fy-apply-icon" size={18} weight="fill" aria-hidden />;
    case "failed":
      return <XCircleIcon className="fy-apply-icon" size={18} weight="fill" aria-hidden />;
    case "cancelled":
      return <MinusCircleIcon className="fy-apply-icon" size={18} weight="fill" aria-hidden />;
    case "planned":
    case "not_started":
      return <CircleIcon className="fy-apply-icon" size={18} weight="regular" aria-hidden />;
    default:
      return assertNever(status);
  }
}

function PlanPane({ view }: { readonly view: ApplyPresentation }) {
  return (
    <section className="fy-apply-pane fy-apply-plan" data-testid="apply-plan">
      <h2>已确认变更</h2>
      <div className="fy-apply-plan-meta">
        <strong>{view.plan.actionLabel}</strong>
        <span>{`逻辑资源 ${String(view.plan.resourceCount)} 项`}</span>
        <em data-valid={view.plan.baselineValid ? "true" : "false"}>
          {view.plan.baselineValid ? "基线有效" : "基线已失效"}
        </em>
      </div>
    </section>
  );
}

function TimelinePane({ view }: { readonly view: ApplyPresentation }) {
  return (
    <section className="fy-apply-pane fy-apply-timeline" data-testid="apply-timeline">
      <h2>执行进度</h2>
      <ol className="fy-apply-step-list" aria-label="应用步骤">
        {view.steps.map((step) => (
          <StepRow key={step.stepId} step={step} />
        ))}
      </ol>
    </section>
  );
}

function StepRow({ step }: { readonly step: ApplyStepPresentation }) {
  return (
    <li
      className="fy-apply-step"
      data-status={step.status}
      aria-current={step.current ? "step" : undefined}
    >
      <StepStatusIcon status={step.status} />
      <strong className="fy-apply-step-copy">{step.label}</strong>
      <em>{step.statusLabel}</em>
    </li>
  );
}

function OutcomePane({ view }: { readonly view: ApplyPresentation }) {
  return (
    <section className="fy-apply-pane fy-apply-outcome" data-testid="apply-outcome">
      <h2 className="fy-visually-hidden">应用结果</h2>
      <p className="fy-visually-hidden" aria-live="polite">
        {view.title}
      </p>
      <h3 className="fy-apply-outcome-title">{view.title}</h3>
      <p className="fy-apply-outcome-copy">{view.subtitle}</p>
      <p className="fy-apply-effect">{`效果：${view.effectLabel}`}</p>
      {view.backupAvailable ? <p className="fy-apply-backup">备份可用</p> : null}
      {view.failureLabel ? <p className="fy-apply-failure">{view.failureLabel}</p> : null}
      {view.notices.map((notice) => (
        <p key={notice} className="fy-apply-notice">
          {notice}
        </p>
      ))}
      {view.evidence.length > 0 ? (
        <ul className="fy-apply-evidence">
          {view.evidence.map((item) => (
            <li key={item}>{item}</li>
          ))}
        </ul>
      ) : null}
      {view.observedUsageCopy ? (
        <p className="fy-apply-usage">{view.observedUsageCopy}</p>
      ) : null}
      <div className="fy-apply-actions">
        {view.actions.map((action) => (
          <button
            key={action.kind}
            className="fy-apply-action"
            type="button"
            data-primary={action.primary ? "true" : "false"}
          >
            {action.label}
          </button>
        ))}
      </div>
    </section>
  );
}

export function ApplyWorkspace({
  scenario = "succeeded",
}: ApplyWorkspaceProps) {
  const view = createApplyViewModel(applyFixtures[scenario]);

  return (
    <section
      className="fy-apply-workspace"
      aria-labelledby="fy-apply-title"
      data-testid="apply-workspace"
      data-data-source="prototype"
      data-status={view.status}
      data-effect={view.effect}
    >
      <header className="fy-apply-header">
        <div className="fy-apply-title-group">
          <h1 id="fy-apply-title">应用配置</h1>
          <div className="fy-apply-context-row">
            <strong className="fy-apply-prototype-status">前端原型 · 模拟数据</strong>
            <span className="fy-apply-outbound-note">不发送测试请求</span>
          </div>
        </div>
        <span className="fy-apply-status-badge" data-status={view.status}>
          {view.statusLabel}
        </span>
      </header>
      <div className="fy-apply-grid">
        <PlanPane view={view} />
        <TimelinePane view={view} />
        <OutcomePane view={view} />
      </div>
    </section>
  );
}
