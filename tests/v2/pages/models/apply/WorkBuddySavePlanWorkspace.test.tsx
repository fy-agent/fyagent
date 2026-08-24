import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { WorkBuddySavePlanWorkspace } from "@/v2/pages/models/apply/WorkBuddySavePlanWorkspace";
import { FeatureProvider } from "@/v2/shared/features/provider";
import { createBrowserFeaturePorts } from "@/v2/shared/platform/browser/features";
import {
  changePlanWorkBuddyWire,
  changeJobWorkBuddyWire,
} from "../../../fixtures/changePlans";

describe("WorkBuddy save Change Plan workspace", () => {
  it("confirms with planId and digest only and has no cancel control", async () => {
    const apply = vi.fn(async (input: { planId: string; planDigest: string }) => {
      expect(Object.keys(input).sort()).toEqual(["planDigest", "planId"]);
      return {
        kind: "admitted" as const,
        job: {
          ...changeJobWorkBuddyWire,
          planId: changePlanWorkBuddyWire.planId,
          idempotencyKey: changePlanWorkBuddyWire.planId,
          targetProviderId: changePlanWorkBuddyWire.targetProviderId,
          status: "succeeded" as const,
          resultCode: "applied" as const,
        },
      };
    });
    const get = vi.fn(async () => ({
      ...changeJobWorkBuddyWire,
      planId: changePlanWorkBuddyWire.planId,
      idempotencyKey: changePlanWorkBuddyWire.planId,
      targetProviderId: changePlanWorkBuddyWire.targetProviderId,
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
    const create = vi.fn(async () => changePlanWorkBuddyWire);
    const onTerminal = vi.fn();
    const ports = {
      ...createBrowserFeaturePorts(),
      changePlans: {
        createCodexProviderSwitchPlan: vi.fn(),
        createCodexProviderUpsertPlan: vi.fn(),
        createWorkBuddySavePlan: create,
        applyChangePlan: apply,
        cancelChangeJob: vi.fn(),
        getChangeJob: get,
        listRecoverableChangeJobs: vi.fn(async () => []),
      },
    };

    render(
      <FeatureProvider ports={ports}>
        <WorkBuddySavePlanWorkspace
          active
          request={{
            baseUrl: "https://api.example.test/v1",
            apiKey: "secret",
            allowNoApiKey: false,
            selectedModelIds: ["model-a"],
            manualModelIds: [],
            removedModelIds: [],
            clearExistingApiKeys: false,
            expectedRevision: null,
          }}
          plan={changePlanWorkBuddyWire}
          previewError={null}
          onPlanChange={vi.fn()}
          onTerminal={onTerminal}
          onDismiss={vi.fn()}
        />
      </FeatureProvider>,
    );

    expect(screen.getByText("WorkBuddy 模型保存并应用")).toBeVisible();
    expect(screen.queryByRole("button", { name: "取消" })).toBeNull();
    expect(document.body).not.toHaveTextContent("secret");
    fireEvent.click(screen.getByRole("button", { name: "确认应用" }));
    await waitFor(() => expect(apply).toHaveBeenCalledTimes(1));
    expect(apply).toHaveBeenCalledWith({
      planId: changePlanWorkBuddyWire.planId,
      planDigest: changePlanWorkBuddyWire.planDigest,
    });
    await waitFor(() => expect(onTerminal).toHaveBeenCalled());
    expect(create).not.toHaveBeenCalled();
  });
});
