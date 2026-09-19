import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { StrictMode, useState, type ReactNode } from "react";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";

import { OpenCodeModelsPanel } from "@/pages/models/OpenCodeModelsPanel";
import { OpenCodeSubscriptionRestore } from "@/pages/models/OpenCodeSubscriptionRestore";
import type { OpenCodeProviderSnapshot } from "@/shared/features/models";
import type { FeaturePorts } from "@/shared/features/ports";
import { FeatureProvider } from "@/shared/features/provider";
import { featureKeys } from "@/shared/features/queries";
import { createBrowserFeaturePorts } from "@/shared/platform/browser/features";
import { TooltipProvider } from "@/shared/ui/primitives";
import { managedAuthOverviewFixture } from "../../fixtures/managedAuth";

const managedProvider = {
  id: "fyagent-openai-opencode-fixture",
  name: "ChatGPT subscription",
  modelIds: ["subscription-only-model"],
};
const apiProvider = {
  id: "ordinary-api",
  name: "Ordinary API",
  modelIds: ["ordinary-api-model"],
};
const writeTargets = [
  {
    path: "~/.config/opencode/opencode.json",
    backupPath: "~/.config/opencode/opencode.json.backup",
    exists: true,
  },
];

function snapshot(providers: OpenCodeProviderSnapshot[] = []) {
  return {
    providers,
    selectedModel: null,
    revision: "restored-revision",
    ...writeTargets[0],
  };
}

function deferred<T>() {
  let resolve: (value: T) => void = () => {
    throw new Error("Deferred value is not initialized");
  };
  let reject: (reason: unknown) => void = () => {
    throw new Error("Deferred value is not initialized");
  };
  const promise = new Promise<T>((resolvePromise, rejectPromise) => {
    resolve = resolvePromise;
    reject = rejectPromise;
  });
  return { promise, resolve, reject };
}

function configuredPorts() {
  const ports = createBrowserFeaturePorts();
  ports.opencodeModels.restoreManagedProxy = vi.fn(async () => undefined);
  ports.opencodeModels.getSnapshot = vi.fn(async () => snapshot([apiProvider]));
  ports.opencodeModels.saveModels = vi.fn();
  ports.managedAuth.getOverview = vi.fn(async () =>
    managedAuthOverviewFixture(),
  );
  return ports;
}

function renderWithPorts(
  children: ReactNode,
  ports: FeaturePorts,
  queries = new QueryClient({
    defaultOptions: { queries: { retry: false, staleTime: Infinity } },
  }),
) {
  const view = render(
    <StrictMode>
      <MemoryRouter>
        <TooltipProvider>
          <FeatureProvider ports={ports}>
            <QueryClientProvider client={queries}>
              {children}
            </QueryClientProvider>
          </FeatureProvider>
        </TooltipProvider>
      </MemoryRouter>
    </StrictMode>,
  );
  return { ...view, queries };
}

function renderRestore(ports: FeaturePorts, allowWrite = true) {
  const onBegin = vi.fn(() => allowWrite);
  const onEnd = vi.fn();
  const onUnconfirmed = vi.fn();
  const onRestored = vi.fn();
  function Harness() {
    const [blocked, setBlocked] = useState(false);
    return (
      <OpenCodeSubscriptionRestore
        disabled={blocked}
        writeTargets={writeTargets}
        onBeginWrite={onBegin}
        onEndWrite={onEnd}
        onRestored={onRestored}
        onUnconfirmed={() => {
          onUnconfirmed();
          setBlocked(true);
        }}
      />
    );
  }
  return {
    ...renderWithPorts(<Harness />, ports),
    onBegin,
    onEnd,
    onUnconfirmed,
    onRestored,
  };
}

async function confirmRestore(user: ReturnType<typeof userEvent.setup>) {
  await user.click(screen.getByRole("button", { name: "恢复之前的模型配置" }));
  await user.click(
    within(await screen.findByRole("dialog")).getByRole("button", {
      name: "确认恢复",
    }),
  );
}

describe("OpenCode subscription restoration", () => {
  it("discloses the target and makes no restore or read call before confirmation", async () => {
    const user = userEvent.setup();
    const ports = configuredPorts();
    const { onBegin } = renderRestore(ports);
    await user.click(
      screen.getByRole("button", { name: "恢复之前的模型配置" }),
    );
    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByText(writeTargets[0].path)).toBeVisible();
    expect(within(dialog).getByText(writeTargets[0].backupPath)).toBeVisible();
    await user.click(within(dialog).getByRole("button", { name: "取消" }));
    expect(ports.opencodeModels.restoreManagedProxy).not.toHaveBeenCalled();
    expect(ports.opencodeModels.getSnapshot).not.toHaveBeenCalled();
    expect(ports.managedAuth.getOverview).not.toHaveBeenCalled();
    expect(onBegin).not.toHaveBeenCalled();
  });

  it("restores through the dedicated port and waits for fresh reads from both owners", async () => {
    const user = userEvent.setup();
    const ports = configuredPorts();
    const native = deferred<void>();
    const overview = deferred<ReturnType<typeof managedAuthOverviewFixture>>();
    ports.opencodeModels.restoreManagedProxy = vi.fn(() => native.promise);
    ports.managedAuth.getOverview = vi.fn(() => overview.promise);
    const { queries, onBegin, onEnd, onUnconfirmed, onRestored } =
      renderRestore(ports);
    queries.setQueryData(
      featureKeys.openCodeModelSnapshot,
      snapshot([managedProvider]),
    );
    queries.setQueryData(
      featureKeys.managedAuthOverview,
      managedAuthOverviewFixture(),
    );
    await confirmRestore(user);
    expect(
      ports.opencodeModels.restoreManagedProxy,
    ).toHaveBeenCalledExactlyOnceWith();
    expect(onBegin).toHaveBeenCalledOnce();
    expect(ports.opencodeModels.getSnapshot).not.toHaveBeenCalled();
    expect(onEnd).not.toHaveBeenCalled();
    await act(async () => native.resolve());
    await waitFor(() =>
      expect(ports.managedAuth.getOverview).toHaveBeenCalledOnce(),
    );
    expect(ports.opencodeModels.getSnapshot).toHaveBeenCalledOnce();
    expect(onEnd).not.toHaveBeenCalled();
    expect(onRestored).not.toHaveBeenCalled();
    await act(async () => overview.resolve(managedAuthOverviewFixture()));
    await waitFor(() => expect(onEnd).toHaveBeenCalledOnce());
    expect(queries.getQueryData(featureKeys.openCodeModelSnapshot)).toEqual(
      snapshot([apiProvider]),
    );
    expect(onUnconfirmed).not.toHaveBeenCalled();
    expect(onRestored).toHaveBeenCalledOnce();
    expect(ports.opencodeModels.saveModels).not.toHaveBeenCalled();
  });

  it.each(["openai", "xai"])(
    "blocks writes when the %s managed projection remains after restore",
    async (source) => {
      const user = userEvent.setup();
      const ports = configuredPorts();
      ports.opencodeModels.getSnapshot = vi.fn(async () =>
        snapshot([
          { ...managedProvider, id: `fyagent-${source}-opencode-fixture` },
        ]),
      );
      const { onEnd, onUnconfirmed } = renderRestore(ports);
      await confirmRestore(user);
      await waitFor(() => expect(onUnconfirmed).toHaveBeenCalledOnce());
      expect(onEnd).toHaveBeenCalledOnce();
      expect(
        screen.getByRole("button", { name: "恢复之前的模型配置" }),
      ).toBeDisabled();
    },
  );

  it.each(["configuration", "overview"])(
    "blocks writes if the %s readback fails",
    async (owner) => {
      const user = userEvent.setup();
      const ports = configuredPorts();
      if (owner === "configuration") {
        ports.opencodeModels.getSnapshot = vi
          .fn()
          .mockRejectedValue(new Error("fixture read failed"));
      } else {
        ports.managedAuth.getOverview = vi
          .fn()
          .mockRejectedValue(new Error("fixture read failed"));
      }
      const { onEnd, onUnconfirmed } = renderRestore(ports);
      await confirmRestore(user);
      await waitFor(() => expect(onUnconfirmed).toHaveBeenCalledOnce());
      expect(ports.opencodeModels.restoreManagedProxy).toHaveBeenCalledOnce();
      expect(ports.opencodeModels.getSnapshot).toHaveBeenCalledOnce();
      expect(ports.managedAuth.getOverview).toHaveBeenCalledOnce();
      expect(onEnd).toHaveBeenCalledOnce();
      expect(
        screen.getByRole("button", { name: "恢复之前的模型配置" }),
      ).toBeDisabled();
    },
  );

  it("still notifies the parent when an in-flight native restore fails after unmount", async () => {
    const user = userEvent.setup();
    const ports = configuredPorts();
    const native = deferred<void>();
    ports.opencodeModels.restoreManagedProxy = vi.fn(() => native.promise);
    const { unmount, onEnd, onUnconfirmed } = renderRestore(ports);
    await confirmRestore(user);
    unmount();
    await act(async () => native.reject(new Error("fixture native failure")));
    await waitFor(() => expect(onUnconfirmed).toHaveBeenCalledOnce());
    expect(onEnd).toHaveBeenCalledOnce();
    expect(ports.opencodeModels.getSnapshot).not.toHaveBeenCalled();
    expect(ports.managedAuth.getOverview).not.toHaveBeenCalled();
  });

  it("does not enter the writer when another operation owns the parent lock", async () => {
    const user = userEvent.setup();
    const ports = configuredPorts();
    const { onBegin, onEnd, onUnconfirmed } = renderRestore(ports, false);
    await confirmRestore(user);
    expect(onBegin).toHaveBeenCalledOnce();
    expect(ports.opencodeModels.restoreManagedProxy).not.toHaveBeenCalled();
    expect(onEnd).not.toHaveBeenCalled();
    expect(onUnconfirmed).not.toHaveBeenCalled();
  });

  it("still blocks the parent if overview fails after restored configuration unmounts the control", async () => {
    const user = userEvent.setup();
    const ports = configuredPorts();
    const overview = deferred<ReturnType<typeof managedAuthOverviewFixture>>();
    ports.managedAuth.getOverview = vi.fn(() => overview.promise);
    const { unmount, onEnd, onUnconfirmed, onRestored } = renderRestore(ports);
    await confirmRestore(user);
    await waitFor(() =>
      expect(ports.opencodeModels.getSnapshot).toHaveBeenCalledOnce(),
    );
    // The Panel removes this control as soon as its snapshot has no managed
    // provider; the second owner read can still be pending at that point.
    unmount();
    await act(async () =>
      overview.reject(new Error("fixture overview failure")),
    );
    await waitFor(() => expect(onUnconfirmed).toHaveBeenCalledOnce());
    expect(onEnd).toHaveBeenCalledOnce();
    expect(onRestored).not.toHaveBeenCalled();
  });

  it.each([false, true])(
    "excludes managed models from API Key editing and blocks saving (ordinary provider present: %s)",
    async (withApiProvider) => {
      const user = userEvent.setup();
      const ports = configuredPorts();
      ports.opencodeModels.getSnapshot = vi.fn(async () =>
        snapshot(
          withApiProvider ? [managedProvider, apiProvider] : [managedProvider],
        ),
      );
      renderWithPorts(
        <OpenCodeModelsPanel
          active
          writesBlocked={false}
          onBlockWrites={vi.fn()}
        />,
        ports,
      );
      expect(
        await screen.findByRole("button", { name: "恢复之前的模型配置" }),
      ).toBeEnabled();
      expect(screen.getByRole("button", { name: "保存并应用" })).toBeDisabled();
      expect(screen.getByLabelText("供应商名称")).toHaveValue(
        withApiProvider ? apiProvider.name : "",
      );
      const existing = screen.getByTestId("opencode-model-ids");
      await user.click(within(existing).getByRole("button"));
      expect(
        within(existing).queryByText(managedProvider.modelIds[0]),
      ).not.toBeInTheDocument();
      if (withApiProvider) {
        expect(
          within(existing).getByText(apiProvider.modelIds[0]),
        ).toBeVisible();
      } else {
        expect(
          within(existing).getByText("还没有找到已配置的模型 ID"),
        ).toBeVisible();
      }
      expect(ports.opencodeModels.saveModels).not.toHaveBeenCalled();
    },
  );
});
