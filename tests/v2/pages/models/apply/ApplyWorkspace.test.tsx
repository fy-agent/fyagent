import { fireEvent, render, screen } from "@testing-library/react";
import { StrictMode } from "react";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it, vi } from "vitest";

import { ApplyWorkspace } from "@/v2/pages/models/apply/ApplyWorkspace";
import { createApplyViewModel } from "@/v2/pages/models/apply/view-model";
import type {
  ChangeJobSnapshot,
  ChangePlan,
} from "@/v2/shared/features/change-plans";

const plan: ChangePlan = {
  planId: "plan-1",
  operation: "codex_provider_switch",
  targetProviderId: "provider-1",
  targetProviderName: "Provider One",
  planDigest: "a".repeat(64),
  baselineDigest: "b".repeat(64),
  dbBaselineProviderId: "provider-before",
  deviceBaselineProviderId: "provider-before",
  secretCapability: "no_new_credential_material",
  createdAt: 1_800_000_000,
  expiresAt: 1_800_000_900,
  status: "ready",
  adapter: {
    adapterId: "codex_provider_switch",
    adapterVersion: "1",
    operationType: "codex_provider_switch",
    phases: [
      "precheck",
      "snapshot",
      "managed_write",
      "readback",
      "finalize",
    ],
    readSet: [
      "provider_db_current",
      "device_current",
      "target_definition",
      "codex_live_projection",
    ],
    writeSet: [
      "provider_db_current",
      "device_current",
      "codex_live_projection",
    ],
    idempotencyScope: "plan",
    cancelMode: "before_managed_write",
    compensationMode: "writer_owned_rollback",
    faultPoints: [
      "before_managed_write",
      "after_managed_write_before_record",
    ],
  },
  currentProviderCode: "provider-before",
  targetProviderCode: "provider-1",
  restartExpectation: "not_required",
  risks: [],
  evidenceNote: "usage_not_observed",
};

function job(overrides: Partial<ChangeJobSnapshot> = {}): ChangeJobSnapshot {
  return {
    jobId: "job-1",
    executionId: "job-1",
    planId: plan.planId,
    idempotencyKey: plan.planId,
    targetProviderId: plan.targetProviderId,
    revision: 1,
    eventSeq: 4,
    status: "running",
    resultCode: "running",
    adapterErrorCode: null,
    steps: [
      { kind: "precheck", status: "succeeded", code: "ok" },
      { kind: "snapshot", status: "succeeded", code: "bound" },
      { kind: "managed_write", status: "running", code: "started" },
      { kind: "readback", status: "pending", code: "pending" },
      { kind: "finalize", status: "pending", code: "pending" },
    ],
    resources: [
      { kind: "provider_db_current", status: "pending", code: "pending" },
      { kind: "device_current", status: "pending", code: "pending" },
      { kind: "target_definition", status: "pending", code: "pending" },
      { kind: "codex_live_projection", status: "pending", code: "pending" },
    ],
    partialResult: {
      succeededSteps: ["precheck", "snapshot"],
      compensatedSteps: [],
      unverifiedSteps: [],
      remainingEffects: [],
      manualActions: [],
    },
    events: [
      {
        sequence: 1,
        phase: "precheck",
        reasonCode: "started",
        createdAt: 1_800_000_001,
      },
    ],
    restartRequirement: "not_required",
    usageEvidence: "not_observed",
    recoveryState: "not_needed",
    diagnosticCode: null,
    liveConfigChanged: false,
    createdAt: 1_800_000_001,
    updatedAt: 1_800_000_002,
    ...overrides,
  };
}

const baseProps = {
  plan,
  job: null,
  busy: false,
  error: null,
  onConfirm: vi.fn(),
  onRegenerate: vi.fn(),
  onClose: vi.fn(),
} as const;

describe("Apply view model", () => {
  it("keeps preview neutral and never treats it as an apply result", () => {
    const view = createApplyViewModel(plan, null, {
      busy: false,
      error: null,
      nowMs: plan.createdAt * 1000,
    });

    expect(view.mode).toBe("preview");
    expect(view.tone).toBe("neutral");
    expect(view.canConfirm).toBe(true);
    expect(view.usageEvidenceCopy).toBeNull();
  });

  it.each([
    ["planned", "planned"],
    ["running", "running"],
    ["succeeded", "applied"],
    ["warning", "applied_with_warning"],
    ["failed", "writer_failed_baseline_restored"],
  ] as const)(
    "maps real job status %s without throwing",
    (status, resultCode) => {
      const view = createApplyViewModel(
        plan,
        job({ status, resultCode, resources: [] }),
        { busy: false, error: null },
      );

      expect(view.statusLabel).not.toBe("");
    },
  );

  it.each(["succeeded", "warning"] as const)(
    "states that %s has no real usage evidence",
    (status) => {
      const view = createApplyViewModel(
        plan,
        job({
          status,
          resultCode:
            status === "succeeded" ? "applied" : "applied_with_warning",
          resources: [
            {
              kind: "provider_db_current",
              status: "matched",
              code: "matched",
            },
          ],
        }),
        { busy: false, error: null },
      );

      expect(view.usageEvidenceCopy).toContain("尚无真实使用证据");
    },
  );

  it.each(["mismatched", "unavailable"] as const)(
    "keeps %s readback non-green",
    (status) => {
      const view = createApplyViewModel(
        plan,
        job({
          status: "succeeded",
          resultCode: "applied",
          resources: [
            { kind: "device_current", status, code: "not_confirmed" },
          ],
        }),
        { busy: false, error: null },
      );

      expect(view.tone).not.toBe("success");
      expect(view.mode).toBe("recovery");
    },
  );

  it("keeps a confirmed writer failure non-green", () => {
    const view = createApplyViewModel(
      plan,
      job({
        status: "failed",
        resultCode: "writer_failed_baseline_restored",
        recoveryState: "succeeded",
        resources: [
          {
            kind: "provider_db_current",
            status: "mismatched",
            code: "target_not_current",
          },
          {
            kind: "device_current",
            status: "mismatched",
            code: "target_not_current",
          },
          {
            kind: "target_definition",
            status: "matched",
            code: "definition_matched",
          },
          {
            kind: "codex_live_projection",
            status: "mismatched",
            code: "live_mismatched",
          },
        ],
      }),
      { busy: false, error: null },
    );

    expect(view.mode).toBe("failed");
    expect(view.tone).toBe("danger");
    expect(view.title).toContain("原基线已确认");
  });

  it("renders pre-write cancellation and interruption without inventing recovery uncertainty", () => {
    const cancelled = createApplyViewModel(
      plan,
      job({
        status: "cancelled",
        resultCode: "cancelled_before_write",
        recoveryState: "not_needed",
        resources: [],
      }),
      { busy: false, error: null },
    );
    expect(cancelled.mode).toBe("failed");
    expect(cancelled.tone).toBe("neutral");
    expect(cancelled.title).toContain("已取消");

    const interrupted = createApplyViewModel(
      plan,
      job({
        status: "failed",
        resultCode: "interrupted_before_write",
        recoveryState: "succeeded",
        resources: [
          {
            kind: "provider_db_current",
            status: "mismatched",
            code: "target_not_current",
          },
        ],
      }),
      { busy: false, error: null },
    );
    expect(interrupted.mode).toBe("failed");
    expect(interrupted.title).toContain("写入前中断");
  });

  it("renders recovered target and compensation from authoritative job fields", () => {
    const recovered = createApplyViewModel(
      plan,
      job({
        status: "warning",
        resultCode: "recovered_target_reached",
        recoveryState: "not_needed",
        resources: [],
      }),
      { busy: false, error: null },
    );
    expect(recovered.mode).toBe("warning");
    expect(recovered.title).toContain("恢复回读确认");

    const compensated = createApplyViewModel(
      plan,
      job({
        steps: [
          { kind: "precheck", status: "succeeded", code: "ok" },
          { kind: "snapshot", status: "succeeded", code: "bound" },
          {
            kind: "managed_write",
            status: "compensated",
            code: "writer_owned_rollback_confirmed",
          },
          { kind: "readback", status: "succeeded", code: "baseline_restored" },
          { kind: "finalize", status: "succeeded", code: "finalized" },
        ],
      }),
      { busy: false, error: null },
    );
    expect(
      compensated.steps.find((step) => step.key.startsWith("managed_write")),
    ).toMatchObject({ detail: "已补偿", status: "succeeded" });
  });

  it.each(["expired", "stale", "consumed", "invalid_digest"])(
    "offers regeneration only for %s",
    (code) => {
      const view = createApplyViewModel(plan, null, {
        busy: false,
        error: { code },
        nowMs: plan.createdAt * 1000,
      });

      expect(view.canConfirm).toBe(false);
      expect(view.canRegenerate).toBe(true);
      expect(view.mode).toBe("regenerate");
    },
  );

  it("disables confirmation when credential admission is blocked", () => {
    const view = createApplyViewModel(null, null, {
      busy: false,
      error: { code: "secret_dependency_unavailable" },
    });

    expect(view.canConfirm).toBe(false);
    expect(view.canRegenerate).toBe(true);
    expect(view.mode).toBe("blocked");
  });
});

describe("ApplyWorkspace", () => {
  it("does not submit while rendering a preview", () => {
    const onConfirm = vi.fn();
    render(<ApplyWorkspace {...baseProps} onConfirm={onConfirm} />);

    expect(onConfirm).not.toHaveBeenCalled();
    expect(screen.getByRole("button", { name: "确认应用" })).toBeEnabled();
  });

  it("submits only planId and planDigest once under StrictMode and repeat clicks", () => {
    const onConfirm = vi.fn((input: { planId: string; planDigest: string }) => {
      void input;
      return new Promise<void>(() => {});
    });
    render(
      <StrictMode>
        <ApplyWorkspace {...baseProps} onConfirm={onConfirm} />
      </StrictMode>,
    );

    const confirm = screen.getByRole("button", { name: "确认应用" });
    fireEvent.click(confirm);
    fireEvent.click(confirm);

    expect(onConfirm).toHaveBeenCalledTimes(1);
    expect(onConfirm).toHaveBeenCalledWith({
      planId: plan.planId,
      planDigest: plan.planDigest,
    });
    expect(Object.keys(onConfirm.mock.calls[0][0])).toEqual([
      "planId",
      "planDigest",
    ]);
  });

  it("renders only regenerate and close actions for a stale plan", () => {
    render(<ApplyWorkspace {...baseProps} error={{ code: "stale" }} />);

    expect(
      screen.queryByRole("button", { name: "确认应用" }),
    ).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "重新生成计划" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "关闭" })).toBeEnabled();
  });

  it("announces live job status and honest usage evidence", () => {
    render(
      <ApplyWorkspace
        {...baseProps}
        job={job({
          status: "warning",
          resultCode: "applied_with_warning",
          resources: [
            {
              kind: "provider_db_current",
              status: "matched",
              code: "matched",
            },
          ],
        })}
      />,
    );

    expect(
      screen.getByText("尚无真实使用证据。", { exact: false }),
    ).toBeVisible();
    expect(
      screen.getByText("需留意", { selector: ".fy-apply-live" }),
    ).toHaveAttribute("aria-live", "polite");
  });

  it("keeps prohibited prototype controls and data sources out of product code", () => {
    const sourceDir = resolve("src/v2/pages/models/apply");
    const sources = ["ApplyWorkspace.tsx", "view-model.ts", "index.ts"]
      .map((name) => readFileSync(resolve(sourceDir, name), "utf8"))
      .join("\n");

    expect(sources).not.toMatch(
      /\bscenario\b|\bfake\b|\bcancel\b|\bbackup\b|\brestore\b/i,
    );
  });
});
