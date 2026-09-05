import { useId, useRef, useState } from "react";

import type { ChangeJobSnapshot, ChangePlan } from "../change-plans";
import { Button } from "../../ui/Button";
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
  const id = useId();
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
      aria-labelledby={`${id}-title`}
      aria-busy={busy}
    >
      <header className="fy-apply-header">
        <div>
          <p className="fy-apply-eyebrow">配置确认</p>
          <h2 id={`${id}-title`}>{view.title}</h2>
          <p>{view.description}</p>
        </div>
        <span className="fy-apply-status" data-tone={view.tone}>
          {view.statusLabel}
        </span>
      </header>

      {view.preview ? (
        <div className="fy-apply-preview" aria-label="配置更改预览">
          <section className="fy-apply-pane" aria-labelledby={`${id}-semantic`}>
            <h3 id={`${id}-semantic`}>将要更改</h3>
            <p>{view.preview.semantic.summary}</p>
            <dl className="fy-apply-plan-details">
              <div>
                <dt>操作</dt>
                <dd>{view.preview.semantic.operationLabel}</dd>
              </div>
              <div>
                <dt>目标</dt>
                <dd>{view.preview.semantic.targetName}</dd>
              </div>
              <div>
                <dt>确认状态</dt>
                <dd>{view.preview.semantic.confirmationLabel}</dd>
              </div>
            </dl>
          </section>
          <section className="fy-apply-pane" aria-labelledby={`${id}-risk`}>
            <h3 id={`${id}-risk`}>需要注意</h3>
            <dl className="fy-apply-plan-details">
              <div>
                <dt>应用后</dt>
                <dd>{view.preview.risk.restartLabel}</dd>
              </div>
            </dl>
            {view.preview.risk.empty ? (
              <p className="fy-apply-empty">没有其他需要确认的事项</p>
            ) : (
              <ul className="fy-apply-risk-list">
                {view.preview.risk.items.map((item) => (
                  <li key={item.key}>
                    {item.label}（{item.levelLabel}）
                  </li>
                ))}
              </ul>
            )}
          </section>
          <section className="fy-apply-pane" aria-labelledby={`${id}-scope`}>
            <h3 id={`${id}-scope`}>影响范围</h3>
            <dl className="fy-apply-plan-details">
              <div>
                <dt>会检查</dt>
                <dd>{view.preview.scope.readLabels.join("、")}</dd>
              </div>
              <div>
                <dt>会修改</dt>
                <dd>{view.preview.scope.writeLabels.join("、")}</dd>
              </div>
              <div>
                <dt>登录凭据</dt>
                <dd>{view.preview.scope.secretLabel}</dd>
              </div>
              <div>
                <dt>预览有效期</dt>
                <dd>{view.preview.scope.expiresLabel}</dd>
              </div>
            </dl>
          </section>
          <section className="fy-apply-pane" aria-labelledby={`${id}-recovery`}>
            <h3 id={`${id}-recovery`}>失败或中断时</h3>
            <dl className="fy-apply-plan-details">
              <div>
                <dt>保存失败</dt>
                <dd>{view.preview.recovery.rollbackLabel}</dd>
              </div>
              <div>
                <dt>操作中断</dt>
                <dd>{view.preview.recovery.interruptionLabel}</dd>
              </div>
            </dl>
          </section>
        </div>
      ) : (
        <p className="fy-apply-empty">还没有可确认的配置预览。</p>
      )}

      <div className="fy-apply-grid">
        <section className="fy-apply-pane" aria-labelledby={`${id}-progress`}>
          <h3 id={`${id}-progress`}>执行进度</h3>
          {view.steps.length > 0 ? (
            <ol className="fy-apply-step-list" aria-label="配置应用步骤">
              {view.steps.map((step) => (
                <ApplyStep key={step.key} step={step} />
              ))}
            </ol>
          ) : (
            <p className="fy-apply-empty">确认前不会修改配置。</p>
          )}
        </section>

        <section
          className="fy-apply-pane fy-apply-result"
          aria-labelledby={`${id}-result`}
        >
          <h3 id={`${id}-result`}>应用结果</h3>
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
            <p className="fy-apply-empty">应用后会在这里显示检查结果。</p>
          )}
          {view.partialTruth ? (
            <dl className="fy-apply-partial" aria-label="部分应用结果">
              <div>
                <dt>已完成</dt>
                <dd>{view.partialTruth.succeededCount}</dd>
              </div>
              <div>
                <dt>已恢复</dt>
                <dd>{view.partialTruth.compensatedCount}</dd>
              </div>
              <div>
                <dt>尚未确认</dt>
                <dd>{view.partialTruth.unverifiedCount}</dd>
              </div>
              <div>
                <dt>仍有更改</dt>
                <dd>
                  {view.partialTruth.remainingEffects.length > 0
                    ? view.partialTruth.remainingEffects.join("、")
                    : "没有检测到残留更改"}
                </dd>
              </div>
              <div>
                <dt>建议操作</dt>
                <dd>
                  {view.partialTruth.manualActions.length > 0
                    ? view.partialTruth.manualActions.join("、")
                    : "无需额外操作"}
                </dd>
              </div>
            </dl>
          ) : null}
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
            重新生成预览
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
