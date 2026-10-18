import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { WorkBuddySubscriptionPlanWorkspace } from "@/pages/models/apply/WorkBuddySubscriptionPlanWorkspace";
import { FeatureProvider } from "@/shared/features/provider";
import { createBrowserFeaturePorts } from "@/shared/platform/browser/features";
import { changePlanWorkBuddySubscriptionWire as plan } from "../../../fixtures/changePlans";

function setup(canApply = true) {
  const ports = createBrowserFeaturePorts();
  ports.changePlans.applyChangePlan = vi.fn(async () => ({
    kind: "rejected" as const,
    errorCode: "stale" as const,
  }));
  ports.changePlans.createWorkBuddySubscriptionPlan = vi.fn(async () => plan);
  const props = {
    active: true,
    canApply,
    request: {
      accountId: "explicit-account",
      modelId: "explicit-model",
      expectedRevision: null,
      expectedBindingVersion: null,
    },
    plan,
    previewError: null,
    onPlanChange: vi.fn(),
    onTerminal: vi.fn(),
    onDismiss: vi.fn(),
    onApplyError: vi.fn(),
  };
  render(
    <FeatureProvider ports={ports}>
      <WorkBuddySubscriptionPlanWorkspace {...props} />
    </FeatureProvider>,
  );
  return { ports, props };
}
describe("WorkBuddy subscription plan workspace", () => {
  it("discloses WorkBuddy routing and closes without writes", () => {
    const { ports, props } = setup();
    expect(
      screen.getByText(/保存 WorkBuddy 模型设置，服务地址为/),
    ).toHaveTextContent(plan.targetProviderName);
    expect(document.body).not.toHaveTextContent("将 Codex 当前 Provider 切换");
    fireEvent.click(screen.getByRole("button", { name: "关闭" }));
    expect(props.onDismiss).toHaveBeenCalledOnce();
    expect(ports.changePlans.applyChangePlan).not.toHaveBeenCalled();
    expect(
      ports.changePlans.createWorkBuddySubscriptionPlan,
    ).not.toHaveBeenCalled();
  });
  it("blocks confirmation when account becomes unavailable but allows closing", () => {
    const { ports, props } = setup(false);
    expect(screen.getByRole("button", { name: "应用更改" })).toBeDisabled();
    fireEvent.click(screen.getByRole("button", { name: "应用更改" }));
    fireEvent.click(screen.getByRole("button", { name: "关闭" }));
    expect(ports.changePlans.applyChangePlan).not.toHaveBeenCalled();
    expect(props.onDismiss).toHaveBeenCalledOnce();
  });
  it("submits only opaque plan identity once and reports stale without success", async () => {
    const { ports, props } = setup();
    const confirm = screen.getByRole("button", { name: "应用更改" });
    await act(async () => {
      fireEvent.click(confirm);
      fireEvent.click(confirm);
    });
    await waitFor(() => expect(props.onApplyError).toHaveBeenCalledOnce());
    expect(ports.changePlans.applyChangePlan).toHaveBeenCalledExactlyOnceWith({
      planId: plan.planId,
      planDigest: plan.planDigest,
    });
    expect(props.onTerminal).not.toHaveBeenCalled();
    expect(document.body).not.toHaveTextContent("已保存");
  });
});
