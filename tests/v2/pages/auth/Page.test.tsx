import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter, useLocation } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";

import { AuthPage } from "@/v2/pages/auth/Page";
import type { FeaturePorts } from "@/v2/shared/features/ports";
import { FeatureProvider } from "@/v2/shared/features/provider";
import { createBrowserFeaturePorts } from "@/v2/shared/platform/browser/features";
import { TooltipProvider } from "@/v2/shared/ui/primitives";
import {
  ACCOUNT_REVISION,
  CODEX_CONNECTION_ID,
  CONNECTION_REVISION,
  OPENAI_ACCOUNT_ID,
  PREVIEW_ID,
  deviceLoginSessionFixture,
  managedAuthOverviewFixture,
  mutationResultFixture,
  removalPreviewFixture,
} from "../../fixtures/managedAuth";

function LocationProbe() {
  const location = useLocation();
  return (
    <output data-testid="test-location">
      {location.pathname}
      {location.search}
    </output>
  );
}

function managedPorts(
  overrides: Partial<FeaturePorts["managedAuth"]> = {},
): FeaturePorts {
  const ports = createBrowserFeaturePorts();
  ports.managedAuth = {
    getOverview: vi.fn(async () => managedAuthOverviewFixture()),
    startLogin: vi.fn(async () => deviceLoginSessionFixture()),
    getLoginSession: vi.fn(async () => deviceLoginSessionFixture()),
    cancelLogin: vi.fn(async () =>
      deviceLoginSessionFixture({
        stage: "cancelled",
        canCancel: false,
        reasonCode: "cancelled",
        terminal: true,
      }),
    ),
    reopenLogin: vi.fn(async () => deviceLoginSessionFixture()),
    switchLoginMethod: vi.fn(async () => deviceLoginSessionFixture()),
    setDefaultAccount: vi.fn(async () => mutationResultFixture()),
    previewAccountRemoval: vi.fn(async () => removalPreviewFixture()),
    removeAccount: vi.fn(async () => mutationResultFixture()),
    applyConnectionAction: vi.fn(async () => mutationResultFixture()),
    ...overrides,
  };
  return ports;
}

function renderPage(ports: FeaturePorts, initialEntry = "/auth") {
  return render(
    <MemoryRouter initialEntries={[initialEntry]}>
      <TooltipProvider delayDuration={0} skipDelayDuration={0}>
        <FeatureProvider ports={ports}>
          <AuthPage />
        </FeatureProvider>
      </TooltipProvider>
      <LocationProbe />
    </MemoryRouter>,
  );
}

function withoutOpenAiAccount() {
  const next = managedAuthOverviewFixture();
  next.accounts = next.accounts.filter(
    (account) => account.accountId !== OPENAI_ACCOUNT_ID,
  );
  next.connections = next.connections.filter(
    (connection) => connection.accountId !== OPENAI_ACCOUNT_ID,
  );
  return next;
}

describe("AuthPage", () => {
  it("keeps account identity, software connection and current request source visually separate", async () => {
    renderPage(managedPorts());

    expect(
      await screen.findByRole("heading", { name: "账号与认证" }),
    ).toBeVisible();
    expect(
      screen.getByRole("heading", { name: "person@example.com" }),
    ).toBeVisible();
    const accountDetail = screen.getByRole("region", {
      name: "person@example.com 账号详情",
    });
    expect(accountDetail).toHaveTextContent("OpenAI · ChatGPT Plus");
    const connectedSection = screen
      .getByRole("heading", { name: "已连接软件" })
      .closest("section");
    expect(connectedSection).not.toBeNull();
    expect(
      within(connectedSection!).getByRole("heading", { name: "Codex" }),
    ).toBeVisible();
    expect(within(connectedSection!).getByText("DeepSeek API")).toBeVisible();
    expect(within(connectedSection!).getByText("已保留")).toBeVisible();
    expect(
      within(connectedSection!).getByText("由 Codex 自动续期"),
    ).toBeVisible();
    expect(document.body.textContent).not.toMatch(
      /access[_ ]?token|refresh[_ ]?token|authorization[_ ]?code|secretRef/iu,
    );
  });

  it("opens the consumer deep link and preserves the three-state explanation", async () => {
    renderPage(
      managedPorts(),
      "/auth?consumer=codex&view=connections&agentReturn=codex&agentSection=models",
    );

    expect(await screen.findByRole("heading", { name: "Codex" })).toBeVisible();
    expect(screen.getByText("OpenAI · person@example.com")).toBeVisible();
    const detail = screen.getByRole("region", { name: "Codex 连接详情" });
    expect(within(detail).getByText("DeepSeek API")).toBeVisible();
    expect(within(detail).getByText("已保留")).toBeVisible();
    expect(within(detail).getByText("由 Codex 自动续期")).toBeVisible();
    expect(screen.getByTestId("test-location")).toHaveTextContent(
      "agentReturn=codex",
    );
  });

  it("starts the selected official device-code flow without exposing callback data", async () => {
    const user = userEvent.setup();
    const startLogin = vi.fn(async () => deviceLoginSessionFixture());
    const ports = managedPorts({ startLogin });
    renderPage(ports, "/auth?consumer=codex&view=connections");

    await user.click(await screen.findByRole("button", { name: "添加账号" }));
    const dialog = screen.getByRole("dialog", { name: "添加官方账号" });
    await user.click(within(dialog).getByLabelText("设备码登录"));
    await user.click(within(dialog).getByRole("button", { name: "下一步" }));
    expect(dialog).toHaveTextContent("auth.openai.com / chatgpt.com");
    await user.click(within(dialog).getByRole("button", { name: "继续" }));

    expect(startLogin).toHaveBeenCalledWith({
      provider: "openai",
      purpose: "connect_consumer",
      consumer: "codex",
      method: "device_code",
      accountId: null,
    });
    expect(await within(dialog).findByText("ABCD-EFGH")).toBeVisible();
    expect(
      within(dialog).getByRole("region", { name: "设备码登录" }),
    ).toHaveTextContent("auth.openai.com");
    expect(dialog.textContent).not.toContain("localhost");
    expect(dialog.textContent).not.toContain("authorization_code");
  });

  it("recovers an active backend login session without starting another", async () => {
    const overview = managedAuthOverviewFixture();
    overview.activeSessions = [deviceLoginSessionFixture()];
    const startLogin = vi.fn();
    const ports = managedPorts({
      getOverview: vi.fn(async () => overview),
      startLogin,
    });
    renderPage(ports);

    expect(await screen.findByText("等待你完成官方登录")).toBeVisible();
    await userEvent.click(screen.getByRole("button", { name: "继续登录" }));
    expect(screen.getByRole("dialog", { name: "添加官方账号" })).toBeVisible();
    expect(screen.getByText("ABCD-EFGH")).toBeVisible();
    expect(startLogin).not.toHaveBeenCalled();
  });

  it("previews impact before removing an account and commits only the reviewed preview", async () => {
    const user = userEvent.setup();
    const previewAccountRemoval = vi.fn(async () => removalPreviewFixture());
    const removeAccount = vi.fn(async () =>
      mutationResultFixture(withoutOpenAiAccount()),
    );
    const ports = managedPorts({ previewAccountRemoval, removeAccount });
    renderPage(ports);

    await user.click(await screen.findByRole("button", { name: "移除账号" }));
    const dialog = await screen.findByRole("dialog", {
      name: "移除 person@example.com？",
    });
    expect(await within(dialog).findByText("将断开")).toBeVisible();
    expect(within(dialog).getByText("FyAgent Local Proxy")).toBeVisible();
    expect(within(dialog).getByText("不会改变")).toBeVisible();
    await user.click(within(dialog).getByRole("button", { name: "移除账号" }));

    expect(previewAccountRemoval).toHaveBeenCalledWith(
      OPENAI_ACCOUNT_ID,
      ACCOUNT_REVISION,
    );
    expect(removeAccount).toHaveBeenCalledWith(
      PREVIEW_ID,
      OPENAI_ACCOUNT_ID,
      ACCOUNT_REVISION,
    );
    await waitFor(() =>
      expect(screen.queryByText("person@example.com")).not.toBeInTheDocument(),
    );
  });

  it("confirms switching Codex back to the official account separately from its current provider", async () => {
    const user = userEvent.setup();
    const applyConnectionAction = vi.fn(async () => {
      const overview = managedAuthOverviewFixture();
      overview.connections[0] = {
        ...overview.connections[0],
        requestMode: "official_subscription",
        requestProviderLabel: "OpenAI 官方订阅",
      };
      return mutationResultFixture(overview);
    });
    renderPage(
      managedPorts({ applyConnectionAction }),
      "/auth?consumer=codex&view=connections",
    );

    await user.click(await screen.findByRole("button", { name: "切回官方" }));
    const dialog = screen.getByRole("dialog", {
      name: "切回 Codex 官方模式？",
    });
    expect(dialog).toHaveTextContent("DeepSeek API");
    await user.click(within(dialog).getByRole("button", { name: "切换" }));

    expect(applyConnectionAction).toHaveBeenCalledWith({
      connectionId: CODEX_CONNECTION_ID,
      expectedRevision: CONNECTION_REVISION,
      action: "switch_to_official",
      accountId: null,
    });
    await waitFor(() => {
      const detail = screen.getByRole("region", {
        name: "Codex 连接详情",
      });
      expect(detail).toHaveTextContent("OpenAI 官方订阅");
    });
  });

  it("keeps the browser-only state explicit instead of seeding fake accounts", async () => {
    renderPage(createBrowserFeaturePorts());

    expect(
      await screen.findByRole(
        "heading",
        { name: "无法加载账号与认证" },
        { timeout: 4_000 },
      ),
    ).toBeVisible();
    expect(
      screen.getByText("此功能仅在 FyAgent 桌面应用中可用。"),
    ).toBeVisible();
    expect(screen.queryByText("person@example.com")).not.toBeInTheDocument();
  });
});
