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

      {view.preview ? (
        <div className="fy-apply-preview" aria-label="变更计划预览">
          <section
            className="fy-apply-pane"
            aria-labelledby="fy-apply-semantic-title"
          >
            <h3 id="fy-apply-semantic-title">语义变化</h3>
            <p>{view.preview.semantic.summary}</p>
            <dl className="fy-apply-plan-details">
              <div>
                <dt>操作</dt>
                <dd>{view.preview.semantic.operationLabel}</dd>
              </div>
              <div>
                <dt>当前 Provider</dt>
                <dd>{view.preview.semantic.currentCode}</dd>
              </div>
              <div>
                <dt>目标 Provider</dt>
                <dd>
                  {view.preview.semantic.targetName}（
                  {view.preview.semantic.targetCode}）
                </dd>
              </div>
              <div>
                <dt>计划状态</dt>
                <dd>{view.preview.semantic.planStatusLabel}</dd>
              </div>
            </dl>
          </section>
          <section
            className="fy-apply-pane"
            aria-labelledby="fy-apply-risk-title"
          >
            <h3 id="fy-apply-risk-title">风险与重启</h3>
            <dl className="fy-apply-plan-details">
              <div>
                <dt>重启预期</dt>
                <dd>{view.preview.risk.restartLabel}</dd>
              </div>
            </dl>
            {view.preview.risk.empty ? (
              <p className="fy-apply-empty">无额外风险项</p>
            ) : (
              <ul className="fy-apply-risk-list">
                {view.preview.risk.items.map((item) => (
                  <li key={item.code}>
                    {item.code}（{item.severity}）
                  </li>
                ))}
              </ul>
            )}
          </section>
          <section
            className="fy-apply-pane"
            aria-labelledby="fy-apply-scope-title"
          >
            <h3 id="fy-apply-scope-title">前置条件与范围</h3>
            <dl className="fy-apply-plan-details">
              <div>
                <dt>读取范围</dt>
                <dd>{view.preview.scope.readLabels.join("、")}</dd>
              </div>
              <div>
                <dt>写入范围</dt>
                <dd>{view.preview.scope.writeLabels.join("、")}</dd>
              </div>
              <div>
                <dt>凭据条件</dt>
                <dd>{view.preview.scope.secretLabel}</dd>
              </div>
              <div>
                <dt>数据库基线</dt>
                <dd>{view.preview.scope.dbBaselineLabel}</dd>
              </div>
              <div>
                <dt>设备基线</dt>
                <dd>{view.preview.scope.deviceBaselineLabel}</dd>
              </div>
              <div>
                <dt>过期时间</dt>
                <dd>{view.preview.scope.expiresLabel}</dd>
              </div>
            </dl>
          </section>
          <section
            className="fy-apply-pane"
            aria-labelledby="fy-apply-recovery-title"
          >
            <h3 id="fy-apply-recovery-title">恢复方式</h3>
            <dl className="fy-apply-plan-details">
              <div>
                <dt>证据</dt>
                <dd>{view.preview.recovery.evidenceLabel}</dd>
              </div>
              <div>
                <dt>补偿</dt>
                <dd>{view.preview.recovery.compensationLabel}</dd>
              </div>
              <div>
                <dt>回读</dt>
                <dd>{view.preview.recovery.readbackLabel}</dd>
              </div>
            </dl>
          </section>
        </div>
      ) : (
        <p className="fy-apply-empty">尚未生成可应用的计划。</p>
      )}

      <div className="fy-apply-grid">
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
          {view.eventSeq !== null ? (
            <p className="fy-apply-event-seq">后端事件序号 {view.eventSeq}</p>
          ) : null}
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
          {view.partialTruth ? (
            <dl className="fy-apply-partial" aria-label="部分执行结果">
              <div>
                <dt>已成功步骤</dt>
                <dd>{view.partialTruth.succeededCount}</dd>
              </div>
              <div>
                <dt>已补偿步骤</dt>
                <dd>{view.partialTruth.compensatedCount}</dd>
              </div>
              <div>
                <dt>未确认步骤</dt>
                <dd>{view.partialTruth.unverifiedCount}</dd>
              </div>
              <div>
                <dt>剩余效果</dt>
                <dd>
                  {view.partialTruth.remainingEffects.length > 0
                    ? view.partialTruth.remainingEffects.join("、")
                    : "无剩余写入效果"}
                </dd>
              </div>
              <div>
                <dt>人工动作</dt>
                <dd>
                  {view.partialTruth.manualActions.length > 0
                    ? view.partialTruth.manualActions.join("、")
                    : "无需额外人工动作"}
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
