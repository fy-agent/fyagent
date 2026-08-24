import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { ChangePlanWorkspace } from "@/v2/pages/models/apply/ChangePlanWorkspace";
import { FeatureProvider } from "@/v2/shared/features/provider";
import { createBrowserFeaturePorts } from "@/v2/shared/platform/browser/features";
import { changeJobWire, changePlanWire } from "../../../fixtures/changePlans";

describe("Models Change Plan connection", () => {
  it("uses only saved non-current Codex targets and clears a stale plan on target change", async () => {
    const create = vi.fn(async (targetProviderId: string) => ({
      ...changePlanWire,
      planId: `plan-${targetProviderId}`,
      targetProviderId,
      targetProviderName: targetProviderId,
    }));
    const apply = vi.fn(async () => ({
      kind: "admitted" as const,
      job: changeJobWire,
    }));
    const get = vi.fn(async () => changeJobWire);
    const cancel = vi.fn(async () => ({
      accepted: false,
      code: "not_active" as const,
      jobId: "job-1",
    }));
    const list = vi.fn(async () => []);
    const ports = {
      ...createBrowserFeaturePorts(),
      changePlans: {
        createCodexProviderSwitchPlan: create,
        applyChangePlan: apply,
        cancelChangeJob: cancel,
        getChangeJob: get,
        listRecoverableChangeJobs: list,
      },
    };

    render(
      <FeatureProvider ports={ports}>
        <ChangePlanWorkspace
          active
          currentId="current"
          providers={{
            current: { id: "current", name: "Current" },
            targetA: { id: "targetA", name: "Target A" },
            targetB: { id: "targetB", name: "Target B" },
          }}
        />
      </FeatureProvider>,
    );

    const target = screen.getByRole("combobox", { name: "目标 Provider" });
    expect(target).not.toHaveTextContent("Current");
    fireEvent.click(screen.getByRole("button", { name: "生成切换计划" }));
    expect(
      await screen.findByRole("button", { name: "确认应用" }),
    ).toBeEnabled();
    expect(create).toHaveBeenLastCalledWith("targetA");

    fireEvent.change(target, { target: { value: "targetB" } });
    expect(screen.queryByRole("button", { name: "确认应用" })).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "生成切换计划" }));
    expect(
      await screen.findByRole("button", { name: "确认应用" }),
    ).toBeEnabled();
    expect(create).toHaveBeenLastCalledWith("targetB");

    fireEvent.click(screen.getByRole("button", { name: "确认应用" }));
    await waitFor(() => expect(get).toHaveBeenCalledWith("job-1"));
    expect(screen.getByText("不可再次使用")).toBeVisible();
    expect(screen.queryByText("可确认")).toBeNull();
    expect(apply).toHaveBeenCalledWith({
      planId: "plan-targetB",
      planDigest: changePlanWire.planDigest,
    });
    expect(list).toHaveBeenCalledOnce();
  });
});
