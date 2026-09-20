import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { StrictMode, useState, type ReactNode } from "react";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";

import { ProviderSubscriptionRestore } from "@/pages/models/ProviderSubscriptionRestore";
import { XaiSubscriptionSection } from "@/pages/models/XaiSubscriptionSection";
import type {
  ProviderAppId,
  ProviderProxyRestorePreview,
  ProviderSummaryQueryData,
} from "@/shared/features/models";
import type { FeaturePorts } from "@/shared/features/ports";
import { FeatureProvider } from "@/shared/features/provider";
import { featureKeys } from "@/shared/features/queries";
import { createBrowserFeaturePorts } from "@/shared/platform/browser/features";
import { TooltipProvider } from "@/shared/ui/primitives";
import { managedAuthOverviewFixture } from "../../fixtures/managedAuth";

const targets = [
  { path: "native-config.toml", exists: true },
  { path: "native-catalog.json", exists: true },
];
const liveSummary = (app: ProviderAppId): ProviderSummaryQueryData => ({
  providers: {},
  currentId: "",
  writeTargets: [],
  live: {
    target: app,
    state: "configured",
    exists: true,
    connection: {
      baseUrl: "https://original.example.test/v1",
      modelId: "original-model",
      protocol: app === "claude" ? "anthropic" : "responses",
    },
  },
});

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((done, fail) => {
    resolve = done;
    reject = fail;
  });
  return { promise, resolve, reject };
}

function configuredPorts() {
  const ports = createBrowserFeaturePorts();
  const enabled = new Map<ProviderAppId, boolean>();
  ports.providers.getProxyRestorePreview = vi.fn(
    async (app): Promise<ProviderProxyRestorePreview> => ({
      app,
      enabled: enabled.get(app) ?? true,
      canRestore: enabled.get(app) ?? true,
      targets,
    }),
  );
  ports.providers.restoreManagedProxy = vi.fn(async (app) => {
    enabled.set(app, false);
  });
  ports.providers.getSummary = vi.fn(async (app) => liveSummary(app));
  ports.managedAuth.getOverview = vi.fn(async () =>
    managedAuthOverviewFixture(),
  );
  return ports;
}

function renderWithPorts(children: ReactNode, ports: FeaturePorts) {
  const queries = new QueryClient({
    defaultOptions: { queries: { retry: false, staleTime: Infinity } },
  });
  const result = render(
    <StrictMode>
      <TooltipProvider>
        <FeatureProvider ports={ports}>
          <QueryClientProvider client={queries}>{children}</QueryClientProvider>
        </FeatureProvider>
      </TooltipProvider>
    </StrictMode>,
  );
  return { ...result, queries };
}

function renderRestore(
  ports: FeaturePorts,
  app: ProviderAppId = "codex",
  allowWrite = true,
  active = true,
) {
  const onBegin = vi.fn(() => allowWrite);
  const onEnd = vi.fn();
  const onUnconfirmed = vi.fn();
  const onRecoveryConfirmed = vi.fn();
  function Harness() {
    const [blocked, setBlocked] = useState(false);
    return (
      <>
        <button disabled={blocked}>普通配置写入</button>
        <ProviderSubscriptionRestore
          app={app}
          active={active}
          disabled={false}
          onBeginWrite={onBegin}
          onEndWrite={onEnd}
          onUnconfirmed={() => {
            onUnconfirmed();
            setBlocked(true);
          }}
          onRecoveryConfirmed={(target) => {
            onRecoveryConfirmed(target);
            if (target === app) setBlocked(false);
          }}
        />
      </>
    );
  }
  return {
    ...renderWithPorts(<Harness />, ports),
    onBegin,
    onEnd,
    onUnconfirmed,
    onRecoveryConfirmed,
  };
}

async function open(
  user: ReturnType<typeof userEvent.setup>,
  target = "Codex",
) {
  const button = await screen.findByRole("button", {
    name: `退出 ${target} 代理并恢复配置`,
  });
  await waitFor(() => expect(button).toBeEnabled());
  await user.click(button);
  return screen.findByRole("dialog", { name: `退出 ${target} 代理` });
}

async function confirm(
  user: ReturnType<typeof userEvent.setup>,
  target = "Codex",
) {
  const dialog = await open(user, target);
  await user.click(
    within(dialog).getByRole("button", { name: "确认退出并恢复" }),
  );
}

describe("Provider subscription exit", () => {
  it("discloses only native restoration targets and cancellation writes nothing", async () => {
    const user = userEvent.setup();
    const ports = configuredPorts();
    const { onBegin } = renderRestore(ports);
    const dialog = await open(user);
    for (const target of targets)
      expect(within(dialog).getByText(target.path)).toBeVisible();
    expect(dialog).toHaveTextContent("使用接管前保存的配置恢复");
    expect(dialog).not.toHaveTextContent("backup");
    expect(dialog).toHaveTextContent("重新打开软件或新建会话");
    await user.click(within(dialog).getByRole("button", { name: "取消" }));
    expect(ports.providers.restoreManagedProxy).not.toHaveBeenCalled();
    expect(ports.providers.getSummary).not.toHaveBeenCalled();
    expect(ports.managedAuth.getOverview).not.toHaveBeenCalled();
    expect(onBegin).not.toHaveBeenCalled();
  });

  it.each([
    ["claude", "Claude Code"],
    ["codex", "Codex"],
    ["grokbuild", "Grok Build"],
  ] as const)(
    "restores %s only after fresh preview and awaits all readbacks",
    async (app, label) => {
      const user = userEvent.setup();
      const ports = configuredPorts();
      const overview =
        deferred<ReturnType<typeof managedAuthOverviewFixture>>();
      ports.managedAuth.getOverview = vi.fn(() => overview.promise);
      const { onBegin, onEnd, onUnconfirmed, onRecoveryConfirmed, queries } =
        renderRestore(ports, app);
      await confirm(user, label);
      await waitFor(() =>
        expect(ports.managedAuth.getOverview).toHaveBeenCalledOnce(),
      );
      expect(
        ports.providers.restoreManagedProxy,
      ).toHaveBeenCalledExactlyOnceWith(app);
      expect(ports.providers.getSummary).toHaveBeenCalledExactlyOnceWith(app);
      expect(onBegin).toHaveBeenCalledOnce();
      expect(onEnd).not.toHaveBeenCalled();
      expect(onRecoveryConfirmed).not.toHaveBeenCalled();
      expect(screen.queryByText(/已退出.*并回读/)).not.toBeInTheDocument();
      await act(async () => overview.resolve(managedAuthOverviewFixture()));
      await waitFor(() => expect(onEnd).toHaveBeenCalledOnce());
      expect(screen.getByRole("status")).toHaveTextContent(
        `已退出 ${label} 代理并回读`,
      );
      expect(queries.getQueryData(featureKeys.providerSummary(app))).toEqual(
        liveSummary(app),
      );
      expect(onUnconfirmed).not.toHaveBeenCalled();
      expect(onRecoveryConfirmed).toHaveBeenCalledExactlyOnceWith(app);
      expect(
        vi
          .mocked(ports.providers.getProxyRestorePreview)
          .mock.calls.every(([target]) => target === app),
      ).toBe(true);
    },
  );

  it("repeated confirmation cannot admit another restore while the first preview recheck waits", async () => {
    const user = userEvent.setup();
    const ports = configuredPorts();
    const waitPreview = deferred<ProviderProxyRestorePreview>();
    const initial = {
      app: "codex" as const,
      enabled: true,
      canRestore: true,
      targets,
    };
    ports.providers.getProxyRestorePreview = vi
      .fn()
      .mockResolvedValueOnce(initial)
      .mockImplementationOnce(() => waitPreview.promise)
      .mockResolvedValue({ ...initial, enabled: false, canRestore: false });
    const { onBegin } = renderRestore(ports);
    const dialog = await open(user);
    const button = within(dialog).getByRole("button", {
      name: "确认退出并恢复",
    });
    act(() => {
      fireEvent.click(button);
      fireEvent.click(button);
    });
    expect(onBegin).toHaveBeenCalledOnce();
    expect(ports.providers.restoreManagedProxy).not.toHaveBeenCalled();
    await act(async () => waitPreview.resolve(initial));
    await waitFor(() =>
      expect(ports.providers.restoreManagedProxy).toHaveBeenCalledOnce(),
    );
  });

  it.each([
    { app: "claude" as const },
    { enabled: false, canRestore: false },
    { canRestore: false },
    { targets: [{ path: "another-native-target", exists: true }] },
    { targets: [{ ...targets[0], exists: false }, targets[1]] },
  ])(
    "changed preview %j requires another preview and causes no mutation",
    async (change) => {
      const user = userEvent.setup();
      const ports = configuredPorts();
      ports.providers.getProxyRestorePreview = vi
        .fn()
        .mockResolvedValueOnce({
          app: "codex",
          enabled: true,
          canRestore: true,
          targets,
        })
        .mockResolvedValue({
          app: "codex",
          enabled: true,
          canRestore: true,
          targets,
          ...change,
        });
      const { onUnconfirmed, onEnd } = renderRestore(ports);
      await confirm(user);
      await waitFor(() => expect(onEnd).toHaveBeenCalledOnce());
      expect(screen.getByText(/恢复条件已变化/)).toBeVisible();
      expect(ports.providers.restoreManagedProxy).not.toHaveBeenCalled();
      expect(onUnconfirmed).not.toHaveBeenCalled();
    },
  );

  it("a native conflict keeps safe guidance and blocks the failed target", async () => {
    const user = userEvent.setup();
    const ports = configuredPorts();
    ports.providers.restoreManagedProxy = vi.fn(async () => {
      throw new Error("fixture-private-backup");
    });
    const { onUnconfirmed, onRecoveryConfirmed } = renderRestore(ports);
    await confirm(user);
    await waitFor(() => expect(onUnconfirmed).toHaveBeenCalledOnce());
    expect(screen.getByRole("status")).toHaveTextContent(
      "未能确认配置已完整恢复",
    );
    expect(screen.getByText(targets[0].path)).toBeVisible();
    expect(screen.getByText(targets[1].path)).toBeVisible();
    expect(
      screen.queryByText(/fixture-private-backup/),
    ).not.toBeInTheDocument();
    expect(screen.queryByText(/已退出.*并回读/)).not.toBeInTheDocument();
    expect(ports.providers.getSummary).not.toHaveBeenCalled();
    expect(onRecoveryConfirmed).not.toHaveBeenCalled();
  });

  it.each([
    "missing-live",
    "unreadable-live",
    "wrong-live-target",
    "summary-error",
    "overview-error",
    "still-enabled",
  ])("does not claim success after %s readback", async (failure) => {
    const user = userEvent.setup();
    const ports = configuredPorts();
    const summary = liveSummary("codex");
    if (failure === "missing-live") delete summary.live;
    if (failure === "unreadable-live")
      summary.live = {
        target: "codex",
        state: "unreadable",
        exists: null,
        connection: null,
      };
    if (failure === "wrong-live-target")
      summary.live = {
        target: "claude",
        state: "missing",
        exists: false,
        connection: null,
      };
    ports.providers.getSummary = vi.fn(async () => {
      if (failure === "summary-error") throw new Error("read");
      return summary;
    });
    if (failure === "overview-error")
      ports.managedAuth.getOverview = vi.fn(async () => {
        throw new Error("read");
      });
    if (failure === "still-enabled")
      ports.providers.restoreManagedProxy = vi.fn(async () => undefined);
    const { onUnconfirmed, onRecoveryConfirmed } = renderRestore(ports);
    await confirm(user);
    await waitFor(() => expect(onUnconfirmed).toHaveBeenCalledOnce());
    expect(screen.getByRole("status")).toHaveTextContent(
      "未能确认配置已完整恢复",
    );
    expect(screen.queryByText(/已退出.*并回读/)).not.toBeInTheDocument();
    expect(onRecoveryConfirmed).not.toHaveBeenCalled();
  });

  it("permits only a fresh same-target recovery retry while ordinary writes stay blocked", async () => {
    const user = userEvent.setup();
    const ports = configuredPorts();
    const restore = ports.providers.restoreManagedProxy;
    ports.providers.restoreManagedProxy = vi
      .fn<FeaturePorts["providers"]["restoreManagedProxy"]>()
      .mockRejectedValueOnce(new Error("external edit"))
      .mockImplementation(restore);
    const { onUnconfirmed, onRecoveryConfirmed } = renderRestore(ports);
    await confirm(user);
    await waitFor(() => expect(onUnconfirmed).toHaveBeenCalledOnce());
    expect(screen.getByRole("button", { name: "普通配置写入" })).toBeDisabled();
    expect(
      screen.queryByRole("button", { name: "退出 Codex 代理并恢复配置" }),
    ).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "重新检查恢复状态" }));
    await waitFor(() =>
      expect(
        screen.getByRole("button", { name: "退出 Codex 代理并恢复配置" }),
      ).toBeEnabled(),
    );
    expect(screen.getByRole("button", { name: "普通配置写入" })).toBeDisabled();
    expect(onRecoveryConfirmed).not.toHaveBeenCalled();
    await confirm(user);
    await waitFor(() =>
      expect(onRecoveryConfirmed).toHaveBeenCalledExactlyOnceWith("codex"),
    );
    expect(screen.getByRole("button", { name: "普通配置写入" })).toBeEnabled();
    expect(vi.mocked(ports.providers.restoreManagedProxy).mock.calls).toEqual([
      ["codex"],
      ["codex"],
    ]);
  });

  it("rechecks an already-exited target without another mutation and awaits all readbacks", async () => {
    const user = userEvent.setup();
    const ports = configuredPorts();
    ports.providers.getSummary = vi
      .fn<FeaturePorts["providers"]["getSummary"]>()
      .mockRejectedValueOnce(new Error("read interrupted"))
      .mockResolvedValue(liveSummary("codex"));
    const { onUnconfirmed, onRecoveryConfirmed } = renderRestore(ports);
    await confirm(user);
    await waitFor(() => expect(onUnconfirmed).toHaveBeenCalledOnce());
    const overview = deferred<ReturnType<typeof managedAuthOverviewFixture>>();
    ports.managedAuth.getOverview = vi.fn(() => overview.promise);
    const button = screen.getByRole("button", { name: "重新检查恢复状态" });
    act(() => {
      fireEvent.click(button);
      fireEvent.click(button);
    });
    await waitFor(() =>
      expect(ports.managedAuth.getOverview).toHaveBeenCalledOnce(),
    );
    expect(screen.getByRole("button", { name: "普通配置写入" })).toBeDisabled();
    expect(onRecoveryConfirmed).not.toHaveBeenCalled();
    expect(ports.providers.restoreManagedProxy).toHaveBeenCalledOnce();
    await act(async () => overview.resolve(managedAuthOverviewFixture()));
    await waitFor(() =>
      expect(onRecoveryConfirmed).toHaveBeenCalledExactlyOnceWith("codex"),
    );
    expect(screen.getByRole("button", { name: "普通配置写入" })).toBeEnabled();
    expect(ports.providers.restoreManagedProxy).toHaveBeenCalledOnce();
  });

  it("keeps the target blocked when retry readback remains unknown", async () => {
    const user = userEvent.setup();
    const ports = configuredPorts();
    ports.providers.getSummary = vi.fn(async () => ({
      providers: {},
      currentId: "",
      writeTargets: [],
    }));
    const { onUnconfirmed, onRecoveryConfirmed } = renderRestore(ports);
    await confirm(user);
    await waitFor(() => expect(onUnconfirmed).toHaveBeenCalledOnce());
    await user.click(screen.getByRole("button", { name: "重新检查恢复状态" }));
    await waitFor(() => expect(onUnconfirmed).toHaveBeenCalledTimes(2));
    expect(onRecoveryConfirmed).not.toHaveBeenCalled();
    expect(screen.getByRole("button", { name: "普通配置写入" })).toBeDisabled();
    expect(ports.providers.restoreManagedProxy).toHaveBeenCalledOnce();
  });

  it("uses the independent recovery callbacks without reopening subscription binding on failure", async () => {
    const user = userEvent.setup();
    const ports = configuredPorts();
    vi.spyOn(ports.providers, "bindManagedProxy");
    const restore = ports.providers.restoreManagedProxy;
    ports.providers.restoreManagedProxy = vi
      .fn<FeaturePorts["providers"]["restoreManagedProxy"]>()
      .mockRejectedValueOnce(new Error("conflict"))
      .mockImplementation(restore);
    const onBeginWrite = vi.fn(() => false);
    const onBeginRecovery = vi.fn(() => true);
    const onRecoveryConfirmed = vi.fn();
    function Harness() {
      const [blocked, setBlocked] = useState(false);
      return (
        <XaiSubscriptionSection
          app="codex"
          active
          disabled={blocked}
          recoveryDisabled={false}
          writeTargets={[]}
          onBeginWrite={onBeginWrite}
          onBeginRecovery={onBeginRecovery}
          onEndWrite={() => {}}
          onUnconfirmed={() => setBlocked(true)}
          onRecoveryConfirmed={(app) => {
            onRecoveryConfirmed(app);
            if (app === "codex") setBlocked(false);
          }}
        />
      );
    }
    renderWithPorts(
      <MemoryRouter>
        <Harness />
      </MemoryRouter>,
      ports,
    );
    await confirm(user);
    await waitFor(() =>
      expect(
        screen.getByRole("button", { name: "管理订阅账号" }),
      ).toBeDisabled(),
    );
    expect(onBeginRecovery).toHaveBeenCalledOnce();
    expect(onBeginWrite).not.toHaveBeenCalled();
    await user.click(screen.getByRole("button", { name: "重新检查恢复状态" }));
    await waitFor(() =>
      expect(
        screen.getByRole("button", { name: "退出 Codex 代理并恢复配置" }),
      ).toBeEnabled(),
    );
    expect(screen.getByRole("button", { name: "管理订阅账号" })).toBeDisabled();
    await confirm(user);
    await waitFor(() =>
      expect(onRecoveryConfirmed).toHaveBeenCalledExactlyOnceWith("codex"),
    );
    expect(screen.getByRole("button", { name: "管理订阅账号" })).toBeEnabled();
    expect(onBeginRecovery).toHaveBeenCalledTimes(3);
    expect(onBeginWrite).not.toHaveBeenCalled();
    expect(ports.providers.bindManagedProxy).not.toHaveBeenCalled();
  });

  it.each(["missing", "not_configured"] as const)(
    "accepts a confirmed %s preimage without guessing a model",
    async (state) => {
      const user = userEvent.setup();
      const ports = configuredPorts();
      ports.providers.getSummary = vi.fn(
        async (): Promise<ProviderSummaryQueryData> => ({
          providers: {},
          currentId: "",
          writeTargets: [],
          live:
            state === "missing"
              ? { target: "codex", state, exists: false, connection: null }
              : { target: "codex", state, exists: true, connection: null },
        }),
      );
      const { onUnconfirmed } = renderRestore(ports);
      await confirm(user);
      expect(await screen.findByText(/已退出 Codex 代理并回读/)).toBeVisible();
      expect(onUnconfirmed).not.toHaveBeenCalled();
    },
  );

  it("withholds confirmation when the native restore proof is unavailable", async () => {
    const ports = configuredPorts();
    ports.providers.getProxyRestorePreview = vi.fn(async (app) => ({
      app,
      enabled: true,
      canRestore: false,
      targets,
    }));
    renderRestore(ports);
    expect(
      await screen.findByRole("button", { name: "退出 Codex 代理并恢复配置" }),
    ).toBeDisabled();
    expect(ports.providers.restoreManagedProxy).not.toHaveBeenCalled();
  });

  it("does not observe or mutate an inactive target", () => {
    const ports = configuredPorts();
    renderRestore(ports, "codex", true, false);
    expect(ports.providers.getProxyRestorePreview).not.toHaveBeenCalled();
    expect(ports.providers.restoreManagedProxy).not.toHaveBeenCalled();
  });

  it("respects the enclosing target writer lock", async () => {
    const user = userEvent.setup();
    const ports = configuredPorts();
    const { onBegin, onEnd } = renderRestore(ports, "codex", false);
    await confirm(user);
    expect(onBegin).toHaveBeenCalledOnce();
    expect(ports.providers.restoreManagedProxy).not.toHaveBeenCalled();
    expect(onEnd).not.toHaveBeenCalled();
  });

  it("retains the target block and releases its parent lock after a late failure on unmount", async () => {
    const user = userEvent.setup();
    const ports = configuredPorts();
    const restore = deferred<void>();
    ports.providers.restoreManagedProxy = vi.fn(() => restore.promise);
    const { unmount, onUnconfirmed, onRecoveryConfirmed, onEnd } =
      renderRestore(ports);
    await confirm(user);
    await waitFor(() =>
      expect(ports.providers.restoreManagedProxy).toHaveBeenCalledOnce(),
    );
    unmount();
    await act(async () => restore.reject(new Error("late recovery conflict")));
    await waitFor(() => expect(onEnd).toHaveBeenCalledOnce());
    expect(onUnconfirmed).toHaveBeenCalledOnce();
    expect(onRecoveryConfirmed).not.toHaveBeenCalled();
  });

  it("retains target A's success when target B fails, without another A write", async () => {
    const user = userEvent.setup();
    const ports = configuredPorts();
    const restore = ports.providers.restoreManagedProxy;
    ports.providers.restoreManagedProxy = vi.fn(async (app) => {
      if (app === "grokbuild") throw new Error("conflict");
      await restore(app);
    });
    renderWithPorts(
      <>
        {(["claude", "grokbuild"] as const).map((app) => (
          <ProviderSubscriptionRestore
            key={app}
            app={app}
            active
            disabled={false}
            onBeginWrite={() => true}
            onEndWrite={() => {}}
            onUnconfirmed={() => {}}
          />
        ))}
      </>,
      ports,
    );
    await confirm(user, "Claude Code");
    expect(
      await screen.findByText(/已退出 Claude Code 代理并回读/),
    ).toBeVisible();
    await confirm(user, "Grok Build");
    expect(await screen.findByText(/未能确认配置已完整恢复/)).toBeVisible();
    expect(screen.getByText(/已退出 Claude Code 代理并回读/)).toBeVisible();
    expect(vi.mocked(ports.providers.restoreManagedProxy).mock.calls).toEqual([
      ["claude"],
      ["grokbuild"],
    ]);
  });
});
