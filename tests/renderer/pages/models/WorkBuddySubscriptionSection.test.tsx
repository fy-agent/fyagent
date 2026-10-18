import { useQueryClient } from "@tanstack/react-query";
import { featureKeys } from "@/shared/features/queries";
import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { WorkBuddySubscriptionSection } from "@/pages/models/WorkBuddySubscriptionSection";
import { FeatureProvider } from "@/shared/features/provider";
import { createBrowserFeaturePorts } from "@/shared/platform/browser/features";
import {
  managedAuthOverviewFixture,
  OPENAI_ACCOUNT_ID,
  XAI_ACCOUNT_ID,
} from "../../fixtures/managedAuth";
import {
  changePlanWorkBuddySubscriptionWire as plan,
  changeJobWorkBuddyWire,
} from "../../fixtures/changePlans";

function ExpireAccount() {
  const client = useQueryClient();
  return (
    <button
      onClick={() => {
        const next = managedAuthOverviewFixture();
        next.accounts = next.accounts.map((account) => ({
          ...account,
          health: "requires_reauth",
        }));
        client.setQueryData(featureKeys.managedAuthOverview, next);
      }}
    >
      Expire account
    </button>
  );
}

function setup() {
  const ports = createBrowserFeaturePorts();
  ports.managedAuth.getOverview = vi.fn(async () =>
    managedAuthOverviewFixture(),
  );
  ports.providers.fetchXaiManagedModels = vi.fn(async () => ({
    models: ["grok-test"],
    truncated: false,
  }));
  ports.workbuddy.getSubscriptionSource = vi.fn(async () => ({
    revision: "before",
    bindingVersion: null,
    source: null,
  }));
  ports.changePlans.createWorkBuddySubscriptionPlan = vi.fn(async () => plan);
  const job = {
    ...changeJobWorkBuddyWire,
    status: "succeeded" as const,
    resultCode: "applied" as const,
    resources: [
      {
        kind: "work_buddy_subscription_binding" as const,
        status: "matched" as const,
        code: "target_matched",
      },
    ],
    planId: plan.planId,
  };
  ports.changePlans.applyChangePlan = vi.fn(async () => ({
    kind: "admitted" as const,
    job,
  }));
  ports.changePlans.getChangeJob = vi.fn(async () => job);
  const props = {
    active: true,
    disabled: false,
    writeTargets: [
      {
        path: "~/.workbuddy/models.json",
        backupPath: "~/.workbuddy/models.json.backup",
        exists: true,
      },
    ],
    onBeginWrite: vi.fn(() => true),
    onEndWrite: vi.fn(),
    onUnconfirmed: vi.fn(),
    onSaved: vi.fn(),
  };
  const view = render(
    <FeatureProvider ports={ports}>
      <ExpireAccount />
      <WorkBuddySubscriptionSection {...props} />
    </FeatureProvider>,
  );
  return { ports, props, view };
}
async function chooseOpenAI() {
  fireEvent.click(await screen.findByRole("radio", { name: /ChatGPT/ }));
  fireEvent.change(screen.getByLabelText("订阅模型 ID"), {
    target: { value: "gpt-test" },
  });
}
async function preview() {
  fireEvent.click(
    screen.getByRole("button", { name: "预览 WorkBuddy 订阅设置" }),
  );
  await screen.findByRole("region", { name: "保存 WorkBuddy 订阅设置" });
}

describe("WorkBuddy subscription selection and confirmation", () => {
  it("requires explicit selections and closes preview with zero writes", async () => {
    const { ports, props } = setup();
    await screen.findByRole("radio", { name: /ChatGPT/ });
    for (const radio of screen.getAllByRole("radio"))
      expect(radio).not.toBeChecked();
    expect(
      screen.getByRole("button", { name: "预览 WorkBuddy 订阅设置" }),
    ).toBeDisabled();
    await chooseOpenAI();
    await preview();
    expect(
      ports.changePlans.createWorkBuddySubscriptionPlan,
    ).toHaveBeenCalledWith({
      accountId: OPENAI_ACCOUNT_ID,
      modelId: "gpt-test",
      expectedRevision: "before",
      expectedBindingVersion: null,
    });
    expect(screen.getByText(/服务地址为 http:/)).toHaveTextContent(
      plan.targetProviderName,
    );
    expect(document.body).toHaveTextContent("~/.workbuddy/models.json");
    expect(document.body).toHaveTextContent("同名模型");
    fireEvent.click(screen.getByRole("button", { name: "关闭" }));
    expect(ports.changePlans.applyChangePlan).not.toHaveBeenCalled();
    expect(props.onEndWrite).toHaveBeenCalledOnce();
    expect(screen.getByLabelText("订阅模型 ID")).toHaveValue("gpt-test");
  });
  it("rejects late catalog responses after changing account", async () => {
    const { ports } = setup();
    let resolve!: (value: { models: string[]; truncated: boolean }) => void;
    vi.mocked(ports.providers.fetchXaiManagedModels).mockReturnValue(
      new Promise((r) => {
        resolve = r;
      }),
    );
    fireEvent.click(await screen.findByRole("radio", { name: /xAI/ }));
    await chooseOpenAI();
    await act(async () =>
      resolve({ models: ["old-account-model"], truncated: false }),
    );
    expect(document.body).not.toHaveTextContent("old-account-model");
    expect(screen.getByLabelText("订阅模型 ID")).toHaveValue("gpt-test");
    expect(ports.providers.fetchXaiManagedModels).toHaveBeenCalledWith(
      XAI_ACCOUNT_ID,
    );
  });
  it("allows manual xAI input after catalog failure without leaking diagnostics", async () => {
    const { ports } = setup();
    vi.mocked(ports.providers.fetchXaiManagedModels).mockRejectedValue(
      new Error("oauth-secret-sentinel"),
    );
    fireEvent.click(await screen.findByRole("radio", { name: /xAI/ }));
    await screen.findByText("无法加载模型选项");
    fireEvent.change(screen.getByLabelText("订阅模型 ID"), {
      target: { value: "grok-manual" },
    });
    await preview();
    expect(
      ports.changePlans.createWorkBuddySubscriptionPlan,
    ).toHaveBeenCalledWith(
      expect.objectContaining({
        accountId: XAI_ACCOUNT_ID,
        modelId: "grok-manual",
      }),
    );
    expect(document.body).not.toHaveTextContent("oauth-secret-sentinel");
  });
  it("serializes repeated preview and confirm clicks, then requires fresh matching source", async () => {
    const { ports, props } = setup();
    await chooseOpenAI();
    fireEvent.click(
      screen.getByRole("button", { name: "预览 WorkBuddy 订阅设置" }),
    );
    fireEvent.click(
      screen.getByRole("button", { name: "预览 WorkBuddy 订阅设置" }),
    );
    await screen.findByRole("button", { name: "应用更改" });
    expect(
      ports.changePlans.createWorkBuddySubscriptionPlan,
    ).toHaveBeenCalledOnce();
    vi.mocked(ports.workbuddy.getSubscriptionSource).mockResolvedValue({
      revision: "after",
      bindingVersion: 1,
      source: {
        accountId: OPENAI_ACCOUNT_ID,
        modelId: "gpt-test",
        provider: "openai",
        bindingVersion: 1,
        localBaseUrl: plan.targetProviderName,
      },
    });
    const confirm = screen.getByRole("button", { name: "应用更改" });
    await act(async () => {
      fireEvent.click(confirm);
      fireEvent.click(confirm);
    });
    await screen.findByText("WorkBuddy 订阅配置已保存");
    expect(ports.changePlans.applyChangePlan).toHaveBeenCalledExactlyOnceWith({
      planId: plan.planId,
      planDigest: plan.planDigest,
    });
    expect(ports.workbuddy.getSubscriptionSource).toHaveBeenCalledTimes(2);
    expect(props.onSaved).toHaveBeenCalledOnce();
    expect(props.onUnconfirmed).not.toHaveBeenCalled();
  });
  it.each([
    "read-error",
    "wrong-account",
    "wrong-route",
    "wrong-model",
    "wrong-provider",
    "old-version",
    "failed-job",
    "stale",
  ])("blocks rewrites and never claims saved on %s", async (failure) => {
    const { ports, props } = setup();
    await chooseOpenAI();
    await preview();
    const source = {
      accountId: OPENAI_ACCOUNT_ID,
      modelId: "gpt-test",
      provider: "openai" as const,
      bindingVersion: 1,
      localBaseUrl: plan.targetProviderName,
    };
    const result = { revision: "after", bindingVersion: 1, source };
    if (failure === "read-error")
      vi.mocked(ports.workbuddy.getSubscriptionSource).mockRejectedValue(
        new Error("secret-sentinel"),
      );
    else
      vi.mocked(ports.workbuddy.getSubscriptionSource).mockResolvedValue({
        ...result,
        source: {
          ...source,
          ...(failure === "wrong-account" ? { accountId: XAI_ACCOUNT_ID } : {}),
          ...(failure === "wrong-route"
            ? { localBaseUrl: "http://127.0.0.1:1/wrong" }
            : {}),
          ...(failure === "wrong-model" ? { modelId: "wrong" } : {}),
          ...(failure === "wrong-provider" ? { provider: "xai" as const } : {}),
          ...(failure === "old-version" ? { bindingVersion: 0 } : {}),
        },
      });
    if (failure === "failed-job")
      vi.mocked(ports.changePlans.applyChangePlan).mockResolvedValue({
        kind: "admitted",
        job: { ...changeJobWorkBuddyWire, status: "failed" },
      });
    if (failure === "stale")
      vi.mocked(ports.changePlans.applyChangePlan).mockResolvedValue({
        kind: "rejected",
        errorCode: "stale",
      });
    fireEvent.click(screen.getByRole("button", { name: "应用更改" }));
    await screen.findByText("当前配置待确认");
    expect(
      screen.queryByText("WorkBuddy 订阅配置已保存"),
    ).not.toBeInTheDocument();
    expect(props.onUnconfirmed).toHaveBeenCalled();
    expect(props.onSaved).not.toHaveBeenCalled();
    expect(document.body).not.toHaveTextContent("secret-sentinel");
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "关闭" })).toBeEnabled(),
    );
    fireEvent.click(screen.getByRole("button", { name: "关闭" }));
    expect(props.onEndWrite).not.toHaveBeenCalled();
    expect(
      screen.getByRole("button", { name: "预览 WorkBuddy 订阅设置" }),
    ).toBeDisabled();
    expect(ports.changePlans.applyChangePlan).toHaveBeenCalledOnce();
  });
  it("rejects the current native label-only plan instead of inventing its service address", async () => {
    const { ports, props } = setup();
    vi.mocked(
      ports.changePlans.createWorkBuddySubscriptionPlan,
    ).mockResolvedValue({
      ...plan,
      targetProviderName: "WorkBuddy subscription",
    });
    await chooseOpenAI();
    fireEvent.click(
      screen.getByRole("button", { name: "预览 WorkBuddy 订阅设置" }),
    );
    await screen.findByText("无法预览订阅设置");
    expect(
      screen.queryByRole("button", { name: "应用更改" }),
    ).not.toBeInTheDocument();
    expect(ports.changePlans.applyChangePlan).not.toHaveBeenCalled();
    expect(props.onEndWrite).toHaveBeenCalledOnce();
  });
  it("revokes confirmation when the selected account expires during preview", async () => {
    const { ports } = setup();
    await chooseOpenAI();
    await preview();
    fireEvent.click(screen.getByRole("button", { name: "Expire account" }));
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "应用更改" })).toBeDisabled(),
    );
    fireEvent.click(screen.getByRole("button", { name: "应用更改" }));
    expect(ports.changePlans.applyChangePlan).not.toHaveBeenCalled();
  });
  it("never lets late source readback override a job refresh failure", async () => {
    const { ports, props } = setup();
    await chooseOpenAI();
    await preview();
    let resolve!: (
      result: Awaited<ReturnType<typeof ports.workbuddy.getSubscriptionSource>>,
    ) => void;
    vi.mocked(ports.workbuddy.getSubscriptionSource).mockReturnValue(
      new Promise((r) => {
        resolve = r;
      }),
    );
    vi.mocked(ports.changePlans.getChangeJob).mockRejectedValue(
      new Error("unavailable"),
    );
    fireEvent.click(screen.getByRole("button", { name: "应用更改" }));
    await screen.findByText("当前配置待确认");
    await act(async () =>
      resolve({
        revision: "after",
        bindingVersion: 1,
        source: {
          accountId: OPENAI_ACCOUNT_ID,
          modelId: "gpt-test",
          provider: "openai",
          bindingVersion: 1,
          localBaseUrl: plan.targetProviderName,
        },
      }),
    );
    expect(
      screen.queryByText("WorkBuddy 订阅配置已保存"),
    ).not.toBeInTheDocument();
    expect(props.onSaved).not.toHaveBeenCalled();
    expect(props.onUnconfirmed).toHaveBeenCalled();
  });
  it("does not publish a late preview after unmount", async () => {
    const { ports, props, view } = setup();
    let resolve!: (value: typeof plan) => void;
    vi.mocked(
      ports.changePlans.createWorkBuddySubscriptionPlan,
    ).mockReturnValue(
      new Promise((r) => {
        resolve = r;
      }),
    );
    await chooseOpenAI();
    fireEvent.click(
      screen.getByRole("button", { name: "预览 WorkBuddy 订阅设置" }),
    );
    await waitFor(() =>
      expect(
        ports.changePlans.createWorkBuddySubscriptionPlan,
      ).toHaveBeenCalled(),
    );
    view.unmount();
    await act(async () => resolve(plan));
    expect(ports.changePlans.applyChangePlan).not.toHaveBeenCalled();
    expect(props.onSaved).not.toHaveBeenCalled();
  });
});
