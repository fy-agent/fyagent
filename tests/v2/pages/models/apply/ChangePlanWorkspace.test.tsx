import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

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

  it("polls getChangeJob while the job is running and stops after a terminal snapshot", async () => {
    vi.useFakeTimers({ toFake: ["setInterval", "clearInterval"] });
    const running = {
      ...changeJobWire,
      status: "running" as const,
      resultCode: "running" as const,
      eventSeq: 4,
      targetProviderId: "targetA",
    };
    const succeeded = {
      ...running,
      status: "succeeded" as const,
      resultCode: "applied" as const,
      eventSeq: 9,
    };
    const get = vi
      .fn()
      .mockResolvedValueOnce(running)
      .mockResolvedValueOnce(running)
      .mockResolvedValueOnce(succeeded);
    const ports = {
      ...createBrowserFeaturePorts(),
      changePlans: {
        createCodexProviderSwitchPlan: vi.fn(async () => ({
          ...changePlanWire,
          planId: "plan-targetA",
          targetProviderId: "targetA",
        })),
        applyChangePlan: vi.fn(async () => ({
          kind: "admitted" as const,
          job: running,
        })),
        cancelChangeJob: vi.fn(),
        getChangeJob: get,
        listRecoverableChangeJobs: vi.fn(async () => []),
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
          }}
        />
      </FeatureProvider>,
    );

    fireEvent.click(screen.getByRole("button", { name: "生成切换计划" }));
    fireEvent.click(
      await screen.findByRole("button", { name: "确认应用" }),
    );
    await waitFor(() => expect(get).toHaveBeenCalledTimes(1));
    expect(screen.getAllByText("进行中").length).toBeGreaterThan(0);

    await act(async () => {
      await vi.advanceTimersByTimeAsync(1000);
    });
    await waitFor(() => expect(get).toHaveBeenCalledTimes(2));

    await act(async () => {
      await vi.advanceTimersByTimeAsync(1000);
    });
    await waitFor(() => expect(get).toHaveBeenCalledTimes(3));
    expect(await screen.findByText("后端事件序号 9")).toBeVisible();

    await act(async () => {
      await vi.advanceTimersByTimeAsync(1000);
    });
    expect(get).toHaveBeenCalledTimes(3);
  });
});

afterEach(() => {
  vi.useRealTimers();
});
