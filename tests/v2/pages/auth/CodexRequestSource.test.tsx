import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter, useLocation } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";

import { CodexRequestSource } from "@/v2/pages/auth/CodexRequestSource";
import { FeatureProvider } from "@/v2/shared/features/provider";
import type { ChangeJobSnapshot } from "@/v2/shared/features/change-plans";
import { createBrowserFeaturePorts } from "@/v2/shared/platform/browser/features";
import { changeJobWire, changePlanWire } from "../../fixtures/changePlans";

function LocationProbe() {
  const location = useLocation();
  return (
    <output data-testid="source-location">
      {location.pathname}
      {location.search}
    </output>
  );
}

function setup() {
  const ports = createBrowserFeaturePorts();
  let currentId = "before";
  const getSummary = vi.fn(async () => ({
    currentId,
    providers: {
      before: { id: "before", name: "Original API" },
      "provider-1": { id: "provider-1", name: "Provider One" },
    },
    writeTargets: [],
  }));
  ports.providers.getSummary = getSummary;
  ports.changePlans.createCodexProviderSwitchPlan = vi.fn(
    async () => changePlanWire,
  );
  const completed: ChangeJobSnapshot = {
    ...changeJobWire,
    status: "succeeded",
    resultCode: "applied",
    revision: 5,
    resources: changeJobWire.resources.map((resource) => ({
      ...resource,
      status: "matched",
    })),
  };
  ports.changePlans.applyChangePlan = vi.fn(async () => {
    currentId = "provider-1";
    return { kind: "admitted" as const, job: completed };
  });
  ports.changePlans.getChangeJob = vi.fn(async () => completed);
  ports.changePlans.listRecoverableChangeJobs = vi.fn(async () => []);
  const refreshOverview = vi.fn(async () => undefined);
  const onBusyChange = vi.fn();
  const renderSource = (active = true) => (
    <MemoryRouter
      initialEntries={[
        "/auth?consumer=codex&view=connections&agentReturn=codex&agentSection=skills",
      ]}
    >
      <FeatureProvider ports={ports}>
        <CodexRequestSource
          active={active}
          disabled={false}
          onBusyChange={onBusyChange}
          onRefreshOverview={refreshOverview}
        />
      </FeatureProvider>
      <LocationProbe />
    </MemoryRouter>
  );
  return { ports, getSummary, refreshOverview, onBusyChange, renderSource };
}

async function confirmSwitch() {
  fireEvent.click(await screen.findByRole("button", { name: "预览更改" }));
  const confirm = await screen.findByRole("button", { name: "应用更改" });
  fireEvent.click(confirm);
  fireEvent.click(confirm);
}

describe("central Codex request source", () => {
  it("switches once, rereads both owners and retains the result after currentId changes", async () => {
    const { ports, getSummary, refreshOverview, onBusyChange, renderSource } =
      setup();
    render(renderSource());
    await confirmSwitch();
    await waitFor(() => expect(onBusyChange).toHaveBeenLastCalledWith(false));
    expect(ports.changePlans.applyChangePlan).toHaveBeenCalledTimes(1);
    expect(ports.changePlans.applyChangePlan).toHaveBeenCalledWith({
      planId: changePlanWire.planId,
      planDigest: changePlanWire.planDigest,
    });
    expect(getSummary).toHaveBeenCalledTimes(2);
    expect(refreshOverview).toHaveBeenCalledTimes(1);
    expect(screen.getByRole("combobox", { name: "切换到" })).toHaveValue(
      "before",
    );
    expect(screen.getByRole("button", { name: "关闭" })).toBeEnabled();
    expect(onBusyChange).toHaveBeenCalledWith(true);
  });

  it("keeps writes locked on settlement failure, retries reads without repeating the write", async () => {
    const { ports, refreshOverview, onBusyChange, renderSource } = setup();
    refreshOverview.mockRejectedValueOnce(
      new Error("private upstream failure"),
    );
    render(renderSource());
    await confirmSwitch();
    const retry = await screen.findByRole("button", {
      name: "重新检查切换结果",
    });
    expect(screen.getByRole("button", { name: "预览更改" })).toBeDisabled();
    expect(onBusyChange).toHaveBeenLastCalledWith(true);
    expect(screen.queryByText("private upstream failure")).toBeNull();
    fireEvent.click(retry);
    await waitFor(() => expect(onBusyChange).toHaveBeenLastCalledWith(false));
    expect(ports.changePlans.applyChangePlan).toHaveBeenCalledTimes(1);
    expect(refreshOverview).toHaveBeenCalledTimes(2);
  });

  it("does not read on a hidden surface and keeps the closed Agent return tuple when editing", async () => {
    const { getSummary, renderSource } = setup();
    const view = render(renderSource(false));
    expect(getSummary).not.toHaveBeenCalled();
    view.rerender(renderSource());
    await screen.findByRole("combobox", { name: "切换到" });
    fireEvent.click(
      screen.getByRole("button", { name: "编辑 Codex 模型配置" }),
    );
    expect(screen.getByTestId("source-location")).toHaveTextContent(
      "/models?target=codex&agentReturn=codex&agentSection=skills",
    );
  });

  it("fails closed when apply admission is unknown rather than offering another write", async () => {
    const { ports, onBusyChange, renderSource } = setup();
    ports.changePlans.applyChangePlan = vi
      .fn()
      .mockRejectedValue(new Error("transport lost"));
    render(renderSource());
    await confirmSwitch();
    expect(await screen.findByText(/无法确认本次切换是否已开始/)).toBeVisible();
    expect(screen.getByRole("button", { name: "预览更改" })).toBeDisabled();
    expect(onBusyChange).toHaveBeenLastCalledWith(true);
    expect(ports.changePlans.applyChangePlan).toHaveBeenCalledTimes(1);
  });
});
