import { useQueryClient } from "@tanstack/react-query";
import {
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { StrictMode, useState } from "react";
import { MemoryRouter, useLocation } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";

import { XaiSubscriptionSection } from "@/pages/models/XaiSubscriptionSection";
import type { FeaturePorts } from "@/shared/features/ports";
import { FeatureProvider } from "@/shared/features/provider";
import { featureKeys } from "@/shared/features/queries";
import { createBrowserFeaturePorts } from "@/shared/platform/browser/features";
import { TooltipProvider } from "@/shared/ui/primitives";
import {
  managedAuthOverviewFixture,
  OPENAI_ACCOUNT_ID,
  XAI_ACCOUNT_ID,
} from "../../fixtures/managedAuth";

const secondAccountId = `ma1:${"9".repeat(32)}`;
const providerId = "subscription-fixture-provider";
const modelIds = ["grok-fixture-1", "grok-fixture-2"];
const writeTargets = [
  {
    path: "~/.claude/settings.json",
    backupPath: "~/.claude/settings.json.backup",
    exists: true,
  },
];

function configuredPorts() {
  const ports = createBrowserFeaturePorts();
  const overview = managedAuthOverviewFixture();
  overview.accounts.push({
    ...overview.accounts[1],
    accountId: secondAccountId,
    login: "second@example.com",
    connectedConsumerCount: 0,
    isDefault: false,
  });
  ports.managedAuth.getOverview = vi.fn(async () => overview);
  ports.providers.fetchXaiManagedModels = vi.fn(async () => ({
    models: modelIds,
    truncated: false,
  }));
  ports.providers.bindManagedProxy = vi.fn(async (request) => ({
    providerId,
    providerName: "My SuperGrok",
    app: request.app,
    activated: request.app !== "codex",
    alreadyBound: false,
  }));
  ports.providers.getSummary = vi.fn(async () => ({
    providers: { [providerId]: { id: providerId, name: "My SuperGrok" } },
    currentId: providerId,
    writeTargets,
  }));
  ports.opencodeModels.bindManagedProxy = vi.fn(async () => ({
    providerId,
    providerName: "OpenCode subscription",
    app: "opencode" as const,
    activated: true,
    alreadyBound: false,
  }));
  ports.opencodeModels.getSnapshot = vi.fn(async () => ({
    providers: [
      {
        id: providerId,
        name: "OpenCode subscription",
        modelIds,
        editable: true,
      },
    ],
    selectedModel: `${providerId}/${modelIds[1]}`,
    revision: "revision-after",
    path: "~/.config/opencode/opencode.json",
    backupPath: "~/.config/opencode/opencode.json.fyagent.backup",
    exists: true,
  }));
  return ports;
}

function LocationAndAuthority() {
  const location = useLocation();
  const client = useQueryClient();
  return (
    <>
      <output data-testid="location">
        {location.pathname}
        {location.search}
      </output>
      <button
        data-testid="expire-account"
        onClick={() =>
          client.setQueryData(
            featureKeys.managedAuthOverview,
            (
              previous:
                | ReturnType<typeof managedAuthOverviewFixture>
                | undefined,
            ) =>
              previous && {
                ...previous,
                accounts: previous.accounts.map((account) =>
                  account.accountId === XAI_ACCOUNT_ID
                    ? { ...account, health: "requires_reauth" }
                    : account,
                ),
              },
          )
        }
      >
        Expire selected account
      </button>
    </>
  );
}

function renderSection(
  ports: FeaturePorts,
  app: "claude" | "codex" | "grokbuild" | "opencode" = "claude",
  active = true,
) {
  const onBegin = vi.fn(() => true);
  const onEnd = vi.fn();
  const onUnconfirmed = vi.fn();
  function Harness() {
    const [disabled, setDisabled] = useState(false);
    return (
      <>
        <XaiSubscriptionSection
          {...(app === "opencode"
            ? { app, expectedRevision: "revision-before" }
            : { app })}
          writeTargets={
            app === "opencode"
              ? [
                  {
                    path: "~/.config/opencode/opencode.json",
                    backupPath:
                      "~/.config/opencode/opencode.json.fyagent.backup",
                    exists: true,
                  },
                ]
              : writeTargets
          }
          onBeginWrite={onBegin}
          onEndWrite={onEnd}
          onUnconfirmed={() => {
            onUnconfirmed();
            setDisabled(true);
          }}
          active={active}
          disabled={disabled}
        />
        <LocationAndAuthority />
      </>
    );
  }
  const view = render(
    <StrictMode>
      <MemoryRouter
        initialEntries={[
          `/models?target=${app}&agentReturn=grokbuild&agentSection=models`,
        ]}
      >
        <TooltipProvider>
          <FeatureProvider ports={ports}>
            <Harness />
          </FeatureProvider>
        </TooltipProvider>
      </MemoryRouter>
    </StrictMode>,
  );
  return { ...view, onBegin, onEnd, onUnconfirmed };
}

async function selectAccountAndModel(
  user: ReturnType<typeof userEvent.setup>,
  account = "xai@example.com",
) {
  await user.click(
    await screen.findByRole("radio", { name: `Grok · ${account}` }),
  );
  await user.click(await screen.findByRole("button", { name: modelIds[1] }));
}

describe("Managed subscription selection and application", () => {
  it("binds OpenCode with the confirmed revision, rereads its own snapshot and never switches another target", async () => {
    const user = userEvent.setup();
    const ports = configuredPorts();
    renderSection(ports, "opencode");
    await selectAccountAndModel(user);
    await user.click(screen.getByRole("button", { name: "应用到 OpenCode" }));
    const dialog = await screen.findByRole("dialog");
    expect(
      within(dialog).getByText("~/.config/opencode/opencode.json"),
    ).toBeVisible();
    await user.click(within(dialog).getByRole("button", { name: "确认应用" }));
    expect(
      await screen.findByText("已将账号订阅应用到 OpenCode"),
    ).toBeVisible();
    expect(
      ports.opencodeModels.bindManagedProxy,
    ).toHaveBeenCalledExactlyOnceWith({
      accountId: XAI_ACCOUNT_ID,
      modelId: modelIds[1],
      expectedRevision: "revision-before",
    });
    expect(ports.opencodeModels.getSnapshot).toHaveBeenCalledOnce();
    expect(ports.managedAuth.getOverview).toHaveBeenCalledTimes(2);
    expect(ports.providers.bindManagedProxy).not.toHaveBeenCalled();
    expect(ports.providers.getSummary).not.toHaveBeenCalled();
  });

  it.each(["missing-provider", "different-default"])(
    "blocks OpenCode writes when saved model readback has %s",
    async (mismatch) => {
      const user = userEvent.setup();
      const ports = configuredPorts();
      const readSnapshot = ports.opencodeModels.getSnapshot;
      ports.opencodeModels.getSnapshot = vi.fn(async () => ({
        ...(await readSnapshot()),
        ...(mismatch === "missing-provider"
          ? { providers: [] }
          : { selectedModel: `${providerId}/other-model` }),
      }));
      const { onUnconfirmed } = renderSection(ports, "opencode");
      await selectAccountAndModel(user);
      await user.click(screen.getByRole("button", { name: "应用到 OpenCode" }));
      await user.click(
        within(await screen.findByRole("dialog")).getByRole("button", {
          name: "确认应用",
        }),
      );
      expect(
        await screen.findByText("设置已保存，当前状态待确认"),
      ).toBeVisible();
      expect(onUnconfirmed).toHaveBeenCalledOnce();
      expect(
        screen.getByRole("button", { name: "应用到 OpenCode" }),
      ).toBeDisabled();
      expect(
        screen.queryByText("已将账号订阅应用到 OpenCode"),
      ).not.toBeInTheDocument();
    },
  );

  it("uses an explicitly chosen account and suggested model without claiming entitlement, then rereads both owners", async () => {
    const user = userEvent.setup();
    const ports = configuredPorts();
    renderSection(ports);
    expect(
      await screen.findByRole("button", { name: "应用到 Claude Code" }),
    ).toBeDisabled();
    expect(ports.providers.fetchXaiManagedModels).not.toHaveBeenCalled();
    await selectAccountAndModel(user, "second@example.com");
    expect(screen.getByText(/并非你的账号模型名单/)).toBeVisible();
    expect(
      screen.queryByRole("button", { name: /Claude Desktop/ }),
    ).not.toBeInTheDocument();
    expect(ports.providers.fetchXaiManagedModels).toHaveBeenCalledWith(
      secondAccountId,
    );
    await user.click(
      screen.getByRole("button", { name: "应用到 Claude Code" }),
    );
    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByText(/second@example.com/)).toBeVisible();
    expect(within(dialog).getByText("~/.claude/settings.json")).toBeVisible();
    const confirmation = within(dialog).getByRole("button", {
      name: "确认应用",
    });
    fireEvent.click(confirmation);
    fireEvent.click(confirmation);
    expect(
      await screen.findByText("已将账号订阅应用到 Claude Code"),
    ).toBeVisible();
    expect(ports.providers.bindManagedProxy).toHaveBeenCalledExactlyOnceWith({
      app: "claude",
      accountId: secondAccountId,
      modelId: modelIds[1],
    });
    expect(ports.providers.getSummary).toHaveBeenCalledWith("claude");
    expect(ports.managedAuth.getOverview).toHaveBeenCalledTimes(2);
    expect(screen.getByText(/实际调用与额度使用以服务返回为准/)).toBeVisible();
  });

  it("clears the previous account's model choice and requires a new selection", async () => {
    const user = userEvent.setup();
    const ports = configuredPorts();
    renderSection(ports);
    await selectAccountAndModel(user);
    await user.click(
      screen.getByRole("radio", { name: "Grok · second@example.com" }),
    );
    await waitFor(() =>
      expect(ports.providers.fetchXaiManagedModels).toHaveBeenCalledTimes(2),
    );
    expect(
      screen.getByRole("button", { name: "应用到 Claude Code" }),
    ).toBeDisabled();
    expect(screen.queryByText(/已选模型：/)).not.toBeInTheDocument();
  });

  it("routes to the current account page with the existing Agent return context", async () => {
    const user = userEvent.setup();
    renderSection(configuredPorts());
    await user.click(screen.getByRole("button", { name: "管理订阅账号" }));
    expect(screen.getByTestId("location")).toHaveTextContent(
      "/auth?view=accounts&agentReturn=grokbuild&agentSection=models",
    );
  });

  it("saves a Codex draft and hands off to the existing source workspace without applying a plan", async () => {
    const user = userEvent.setup();
    const ports = configuredPorts();
    ports.changePlans.applyChangePlan = vi.fn();
    renderSection(ports, "codex");
    await selectAccountAndModel(user);
    await user.click(
      screen.getByRole("button", { name: "保存 Codex 订阅配置" }),
    );
    await user.click(
      within(await screen.findByRole("dialog")).getByRole("button", {
        name: "确认保存",
      }),
    );
    expect(await screen.findByText(/当前请求来源尚未切换/)).toBeVisible();
    expect(ports.providers.bindManagedProxy).toHaveBeenCalledWith({
      app: "codex",
      accountId: XAI_ACCOUNT_ID,
      modelId: modelIds[1],
    });
    expect(ports.changePlans.applyChangePlan).not.toHaveBeenCalled();
    await user.click(
      screen.getByRole("button", { name: "继续预览 Codex 配置" }),
    );
    expect(screen.getByTestId("location")).toHaveTextContent(
      "/auth?consumer=codex&view=connections&agentReturn=grokbuild&agentSection=models",
    );
  });

  it("blocks a selected account that expires while confirmation is open", async () => {
    const user = userEvent.setup();
    const ports = configuredPorts();
    renderSection(ports);
    await selectAccountAndModel(user);
    await user.click(
      screen.getByRole("button", { name: "应用到 Claude Code" }),
    );
    fireEvent.click(screen.getByTestId("expire-account"));
    expect(
      within(await screen.findByRole("dialog")).getByRole("button", {
        name: "确认应用",
      }),
    ).toBeDisabled();
    expect(ports.providers.bindManagedProxy).not.toHaveBeenCalled();
  });

  it("offers explicit manual input after discovery failure without replacing the selected account", async () => {
    const user = userEvent.setup();
    const ports = configuredPorts();
    ports.providers.fetchXaiManagedModels = vi.fn(async () => {
      throw new Error("SENTINEL-SECRET");
    });
    renderSection(ports);
    await user.click(
      await screen.findByRole("radio", { name: "Grok · xai@example.com" }),
    );
    const input = await screen.findByLabelText("订阅模型 ID");
    await user.type(input, "grok-manual-fixture");
    expect(
      screen.getByRole("button", { name: "应用到 Claude Code" }),
    ).toBeEnabled();
    expect(screen.queryByText(/SENTINEL-SECRET/)).not.toBeInTheDocument();
  });

  it.each(["account_unavailable", "apply_failed_rolled_back"])(
    "shows %s as a safe per-target failure",
    async (code) => {
      const user = userEvent.setup();
      const ports = configuredPorts();
      ports.providers.bindManagedProxy = vi.fn(async () => {
        throw { code };
      });
      const view = renderSection(ports);
      await selectAccountAndModel(user);
      await user.click(
        screen.getByRole("button", { name: "应用到 Claude Code" }),
      );
      await user.click(
        within(await screen.findByRole("dialog")).getByRole("button", {
          name: "确认应用",
        }),
      );
      expect(await screen.findByText("未能应用订阅配置")).toBeVisible();
      expect(view.onUnconfirmed).not.toHaveBeenCalled();
      expect(
        screen.queryByText("已将账号订阅应用到 Claude Code"),
      ).not.toBeInTheDocument();
    },
  );

  it("does not claim success when authoritative reread fails", async () => {
    const user = userEvent.setup();
    const ports = configuredPorts();
    ports.providers.getSummary = vi.fn(async () => {
      throw new Error("SENTINEL-SECRET");
    });
    const view = renderSection(ports);
    await selectAccountAndModel(user);
    await user.click(
      screen.getByRole("button", { name: "应用到 Claude Code" }),
    );
    await user.click(
      within(await screen.findByRole("dialog")).getByRole("button", {
        name: "确认应用",
      }),
    );
    expect(
      await screen.findByText("无法确认当前设置", {}, { timeout: 3000 }),
    ).toBeVisible();
    expect(view.onUnconfirmed).toHaveBeenCalledOnce();
    expect(
      screen.getByRole("button", { name: "应用到 Claude Code" }),
    ).toBeDisabled();
    expect(screen.queryByText(/SENTINEL-SECRET/)).not.toBeInTheDocument();
  });

  it("reports a mismatched authoritative reread even after the pending target unmounts", async () => {
    const user = userEvent.setup();
    const ports = configuredPorts();
    let finishBinding!: () => void;
    const binding = new Promise<void>((resolve) => {
      finishBinding = resolve;
    });
    const bind = ports.providers.bindManagedProxy;
    ports.providers.bindManagedProxy = vi.fn(async (request) => {
      await binding;
      return bind(request);
    });
    ports.providers.getSummary = vi.fn(async () => ({
      providers: {},
      currentId: "another-provider",
      writeTargets,
    }));
    const view = renderSection(ports);
    await selectAccountAndModel(user);
    await user.click(
      screen.getByRole("button", { name: "应用到 Claude Code" }),
    );
    await user.click(
      within(await screen.findByRole("dialog")).getByRole("button", {
        name: "确认应用",
      }),
    );
    expect(ports.providers.bindManagedProxy).toHaveBeenCalledOnce();
    view.unmount();
    finishBinding();
    await waitFor(() => expect(view.onUnconfirmed).toHaveBeenCalledOnce());
    expect(ports.providers.getSummary).toHaveBeenCalledWith("claude");
    expect(view.onEnd).toHaveBeenCalledOnce();
  });

  it("does not read managed accounts for an inactive panel", () => {
    const ports = configuredPorts();
    renderSection(ports, "claude", false);
    expect(ports.managedAuth.getOverview).not.toHaveBeenCalled();
  });

  it.each(["claude", "codex", "grokbuild"] as const)(
    "binds an explicit ChatGPT model to %s without sending its credential to an API-key catalog",
    async (app) => {
      const user = userEvent.setup();
      const ports = configuredPorts();
      renderSection(ports, app);
      await user.click(
        await screen.findByRole("radio", { name: "ChatGPT · Personal" }),
      );
      expect(ports.providers.fetchXaiManagedModels).not.toHaveBeenCalled();
      expect(
        screen.getByRole("button", { name: "查看模型选项" }),
      ).toBeDisabled();
      await user.type(
        await screen.findByLabelText("订阅模型 ID"),
        "chatgpt-fixture-model",
      );
      const label =
        app === "codex"
          ? "保存 Codex 订阅配置"
          : app === "claude"
            ? "应用到 Claude Code"
            : "应用到 Grok Build";
      await user.click(screen.getByRole("button", { name: label }));
      const dialog = await screen.findByRole("dialog");
      if (app !== "codex")
        expect(within(dialog).getByText(writeTargets[0].path)).toBeVisible();
      await user.click(
        within(dialog).getByRole("button", {
          name: app === "codex" ? "确认保存" : "确认应用",
        }),
      );
      await waitFor(() =>
        expect(
          ports.providers.bindManagedProxy,
        ).toHaveBeenCalledExactlyOnceWith({
          app,
          accountId: OPENAI_ACCOUNT_ID,
          modelId: "chatgpt-fixture-model",
        }),
      );
      await waitFor(() =>
        expect(ports.providers.getSummary).toHaveBeenCalledWith(app),
      );
      expect(ports.providers.fetchXaiManagedModels).not.toHaveBeenCalled();
    },
  );

  it("checks Grok Build's current source rather than treating a saved row as activation", async () => {
    const user = userEvent.setup();
    const ports = configuredPorts();
    ports.providers.getSummary = vi.fn(async () => ({
      providers: { [providerId]: { id: providerId, name: "Subscription" } },
      currentId: "another-provider",
      writeTargets,
    }));
    const view = renderSection(ports, "grokbuild");
    await selectAccountAndModel(user);
    await user.click(screen.getByRole("button", { name: "应用到 Grok Build" }));
    await user.click(
      within(await screen.findByRole("dialog")).getByRole("button", {
        name: "确认应用",
      }),
    );
    expect(await screen.findByText("设置已保存，当前状态待确认")).toBeVisible();
    expect(view.onUnconfirmed).toHaveBeenCalledOnce();
    expect(
      screen.queryByText("已将账号订阅应用到 Grok Build"),
    ).not.toBeInTheDocument();
  });
});
