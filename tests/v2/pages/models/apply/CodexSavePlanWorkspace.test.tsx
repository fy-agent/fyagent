import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { CodexSavePlanWorkspace } from "@/v2/pages/models/apply/CodexSavePlanWorkspace";
import { FeatureProvider } from "@/v2/shared/features/provider";
import { createBrowserFeaturePorts } from "@/v2/shared/platform/browser/features";
import {
  changePlanUpsertWire,
  changeJobWire,
} from "../../../fixtures/changePlans";

describe("Codex save Change Plan workspace", () => {
  it("confirms with planId and digest only and has no cancel control", async () => {
    const apply = vi.fn(async (input: { planId: string; planDigest: string }) => {
      expect(Object.keys(input).sort()).toEqual(["planDigest", "planId"]);
      return {
        kind: "admitted" as const,
        job: {
          ...changeJobWire,
          planId: changePlanUpsertWire.planId,
          idempotencyKey: changePlanUpsertWire.planId,
          targetProviderId: changePlanUpsertWire.targetProviderId,
          status: "succeeded" as const,
          resultCode: "applied" as const,
        },
      };
    });
    const get = vi.fn(async () => ({
      ...changeJobWire,
      planId: changePlanUpsertWire.planId,
      idempotencyKey: changePlanUpsertWire.planId,
      targetProviderId: changePlanUpsertWire.targetProviderId,
      status: "succeeded" as const,
      resultCode: "applied" as const,
      eventSeq: 5,
      events: [
        { sequence: 1, phase: "precheck" as const, reasonCode: "ok", createdAt: 1 },
        { sequence: 2, phase: "snapshot" as const, reasonCode: "ok", createdAt: 2 },
        { sequence: 3, phase: "managed_write" as const, reasonCode: "ok", createdAt: 3 },
        { sequence: 4, phase: "readback" as const, reasonCode: "ok", createdAt: 4 },
        { sequence: 5, phase: "finalize" as const, reasonCode: "ok", createdAt: 5 },
      ],
    }));
    const create = vi.fn(async () => changePlanUpsertWire);
    const onTerminal = vi.fn();
    const ports = {
      ...createBrowserFeaturePorts(),
      changePlans: {
        createCodexProviderSwitchPlan: vi.fn(),
        createCodexProviderUpsertPlan: create,
        createWorkBuddySavePlan: vi.fn(),
        applyChangePlan: apply,
        cancelChangeJob: vi.fn(),
        getChangeJob: get,
        listRecoverableChangeJobs: vi.fn(async () => []),
      },
    };

    render(
      <FeatureProvider ports={ports}>
        <CodexSavePlanWorkspace
          active
          request={{
            name: "Gateway",
            baseUrl: "https://codex.example/v1",
            apiKey: "secret",
            modelId: "gpt-5",
          }}
          plan={changePlanUpsertWire}
          previewError={null}
          onPlanChange={vi.fn()}
          onTerminal={onTerminal}
          onDismiss={vi.fn()}
        />
      </FeatureProvider>,
    );

    expect(screen.getByText("Codex Provider 保存并设为当前")).toBeVisible();
    expect(screen.queryByRole("button", { name: "取消" })).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "确认应用" }));
    await waitFor(() => expect(apply).toHaveBeenCalledTimes(1));
    expect(apply).toHaveBeenCalledWith({
      planId: changePlanUpsertWire.planId,
      planDigest: changePlanUpsertWire.planDigest,
    });
    await waitFor(() => expect(onTerminal).toHaveBeenCalled());
    expect(create).not.toHaveBeenCalled();
  });

  it("regenerates a stale plan instead of retrying the old digest", async () => {
    const regenerated = {
      ...changePlanUpsertWire,
      planId: "plan-upsert-2",
      planDigest: "b".repeat(64),
    };
    const create = vi.fn(async () => regenerated);
    const apply = vi.fn(async () => ({
      kind: "rejected" as const,
      errorCode: "stale" as const,
    }));
    const onPlanChange = vi.fn();
    const ports = {
      ...createBrowserFeaturePorts(),
      changePlans: {
        createCodexProviderSwitchPlan: vi.fn(),
        createCodexProviderUpsertPlan: create,
        createWorkBuddySavePlan: vi.fn(),
        applyChangePlan: apply,
        cancelChangeJob: vi.fn(),
        getChangeJob: vi.fn(),
        listRecoverableChangeJobs: vi.fn(async () => []),
      },
    };

    render(
      <FeatureProvider ports={ports}>
        <CodexSavePlanWorkspace
          active
          request={{
            name: "Gateway",
            baseUrl: "https://codex.example/v1",
            apiKey: "secret",
            modelId: "gpt-5",
          }}
          plan={changePlanUpsertWire}
          previewError={null}
          onPlanChange={onPlanChange}
          onTerminal={vi.fn()}
          onDismiss={vi.fn()}
        />
      </FeatureProvider>,
    );

    fireEvent.click(screen.getByRole("button", { name: "确认应用" }));
    expect(
      await screen.findByRole("button", { name: "重新生成计划" }),
    ).toBeVisible();
    fireEvent.click(screen.getByRole("button", { name: "重新生成计划" }));
    await waitFor(() => expect(create).toHaveBeenCalledTimes(1));
    expect(onPlanChange).toHaveBeenCalledWith(regenerated);
    expect(apply).toHaveBeenCalledTimes(1);
    expect(apply).toHaveBeenCalledWith({
      planId: changePlanUpsertWire.planId,
      planDigest: changePlanUpsertWire.planDigest,
    });
  });
});
