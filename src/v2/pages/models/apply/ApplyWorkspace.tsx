import { useRef, useState } from "react";

import type {
  ChangeJobSnapshot,
  ChangePlan,
} from "../../../shared/features/change-plans";
import { Button } from "../../../shared/ui/primitives";
import {
  createApplyViewModel,
  type ApplyStepPresentation,
  type ApplyWorkspaceError,
} from "./view-model";
import "./apply-workspace.css";

export type ConfirmChangePlanInput = {
  readonly planId: string;
  readonly planDigest: string;
};

export type ApplyWorkspaceProps = {
  readonly plan: ChangePlan | null;
  readonly job: ChangeJobSnapshot | null;
  readonly busy: boolean;
  readonly error: ApplyWorkspaceError | null;
  readonly onConfirm: (input: ConfirmChangePlanInput) => void | Promise<void>;
  readonly onRegenerate: () => void;
  readonly onClose: () => void;
};

function ApplyStep({ step }: { readonly step: ApplyStepPresentation }) {
  return (
    <li
      className="fy-apply-step"
      data-status={step.status}
      aria-current={step.current ? "step" : undefined}
    >
      <span className="fy-apply-step-mark" aria-hidden />
      <span>
        <strong>{step.label}</strong>
        <small>{step.detail}</small>
      </span>
    </li>
  );
}

export function ApplyWorkspace({
  plan,
  job,
  busy,
  error,
  onConfirm,
  onRegenerate,
  onClose,
}: ApplyWorkspaceProps) {
  const planKey = plan ? `${plan.planId}:${plan.planDigest}` : null;
  const confirmLockRef = useRef<string | null>(null);
  const [lockedPlanKey, setLockedPlanKey] = useState<string | null>(null);
  const confirmLocked = planKey !== null && lockedPlanKey === planKey;
  const view = createApplyViewModel(plan, job, { busy, error });

  const handleConfirm = () => {
    if (
      !plan ||
      !planKey ||
      !view.canConfirm ||
      confirmLockRef.current === planKey
    )
      return;
    confirmLockRef.current = planKey;
    setLockedPlanKey(planKey);
    void onConfirm({
      planId: plan.planId,
      planDigest: plan.planDigest,
    });
  };

  return (
    <section
      className="fy-apply-workspace"
      data-mode={view.mode}
      data-tone={view.tone}
      aria-labelledby="fy-apply-title"
      aria-busy={busy}
    >
      <header className="fy-apply-header">
        <div>
          <p className="fy-apply-eyebrow">Change Plan</p>
          <h2 id="fy-apply-title">{view.title}</h2>
          <p>{view.description}</p>
        </div>
        <span className="fy-apply-status" data-tone={view.tone}>
          {view.statusLabel}
        </span>
      </header>

      <div className="fy-apply-grid">
        <section
          className="fy-apply-pane"
          aria-labelledby="fy-apply-plan-title"
        >
          <h3 id="fy-apply-plan-title">变更计划</h3>
          {plan ? (
            <dl className="fy-apply-plan-details">
              <div>
                <dt>目标 Provider</dt>
                <dd>{plan.targetProviderName}</dd>
              </div>
              <div>
                <dt>重启预期</dt>
                <dd>
                  {plan.restartExpectation === "recommended"
                    ? "建议重启 Codex"
                    : plan.restartExpectation === "not_required"
                      ? "无需重启"
                      : "尚未确认"}
                </dd>
              </div>
              <div>
                <dt>计划状态</dt>
                <dd>{plan.status === "ready" ? "可确认" : "不可再次使用"}</dd>
              </div>
            </dl>
          ) : (
            <p className="fy-apply-empty">尚未生成可应用的计划。</p>
          )}
        </section>

        <section
          className="fy-apply-pane"
          aria-labelledby="fy-apply-progress-title"
        >
          <h3 id="fy-apply-progress-title">执行进度</h3>
          {view.steps.length > 0 ? (
            <ol className="fy-apply-step-list" aria-label="Change Job 执行步骤">
              {view.steps.map((step) => (
                <ApplyStep key={step.key} step={step} />
              ))}
            </ol>
          ) : (
            <p className="fy-apply-empty">确认前不会执行任何配置写入。</p>
          )}
        </section>

        <section
          className="fy-apply-pane fy-apply-result"
          aria-labelledby="fy-apply-result-title"
        >
          <h3 id="fy-apply-result-title">回读结果</h3>
          <p className="fy-apply-live" aria-live="polite">
            {view.statusLabel}
          </p>
          {view.resources.length > 0 ? (
            <ul className="fy-apply-resource-list">
              {view.resources.map((resource) => (
                <li key={resource.key} data-tone={resource.tone}>
                  <span>{resource.label}</span>
                  <strong>{resource.statusLabel}</strong>
                </li>
              ))}
            </ul>
          ) : (
            <p className="fy-apply-empty">尚无真实回读结果。</p>
          )}
          {view.usageEvidenceCopy ? (
            <p className="fy-apply-usage">{view.usageEvidenceCopy}</p>
          ) : null}
        </section>
      </div>

      <footer className="fy-apply-actions">
        <Button onClick={onClose} disabled={busy}>
          关闭
        </Button>
        {view.canRegenerate ? (
          <Button className="fy-control-button-primary" onClick={onRegenerate}>
            重新生成计划
          </Button>
        ) : null}
        {!job && !view.canRegenerate ? (
          <Button
            className="fy-control-button-primary"
            onClick={handleConfirm}
            disabled={!view.canConfirm || confirmLocked}
          >
            {view.confirmLabel}
          </Button>
        ) : null}
      </footer>
    </section>
  );
}
