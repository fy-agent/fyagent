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
    expect(
      within(connectedSection!).getByRole("button", { name: "切回官方" }),
    ).toBeVisible();
    expect(document.body.textContent).not.toMatch(
      /access[_ ]?token|refresh[_ ]?token|authorization[_ ]?code|secretRef/iu,
    );
  });

  it("connects matching software from the account detail with this account preselected", async () => {
    const user = userEvent.setup();
    const overview = managedAuthOverviewFixture();
    overview.connections[0] = {
      ...overview.connections[0],
      accountId: null,
      authStatus: "disconnected",
      requestMode: "none",
      requestProviderLabel: null,
      allowedActions: ["connect_account", "refresh"],
    };
    overview.accounts[0] = {
      ...overview.accounts[0],
      connectedConsumerCount: 1,
    };
    const applyConnectionAction = vi.fn(async () => {
      const next = managedAuthOverviewFixture();
      return mutationResultFixture(next);
    });
    renderPage(
      managedPorts({
        getOverview: vi.fn(async () => overview),
        applyConnectionAction,
      }),
    );

    const connectSection = (
      await screen.findByRole("heading", { name: "连接到软件" })
    ).closest("section");
    expect(connectSection).not.toBeNull();
    await user.click(
      within(connectSection!).getByRole("button", { name: "用此账号连接" }),
    );
    const dialog = screen.getByRole("dialog", { name: "连接 Codex 账号" });
    await user.click(within(dialog).getByRole("button", { name: "确认" }));

    expect(applyConnectionAction).toHaveBeenCalledWith({
      connectionId: CODEX_CONNECTION_ID,
      expectedRevision: CONNECTION_REVISION,
      action: "connect_account",
      accountId: OPENAI_ACCOUNT_ID,
    });
  });

  it("starts a Codex connection login when the saved account cannot connect yet", async () => {
    const user = userEvent.setup();
    const overview = managedAuthOverviewFixture();
    overview.connections[0] = {
      ...overview.connections[0],
      accountId: null,
      authStatus: "disconnected",
      requestMode: "none",
      requestProviderLabel: null,
      allowedActions: ["refresh"],
    };
    overview.accounts[0] = {
      ...overview.accounts[0],
      connectedConsumerCount: 1,
    };
    const startLogin = vi.fn(async () => deviceLoginSessionFixture());
    renderPage(
      managedPorts({
        getOverview: vi.fn(async () => overview),
        startLogin,
      }),
    );

    await user.click(await screen.findByRole("button", { name: "连接 Codex" }));
    const dialog = screen.getByRole("dialog", { name: "添加官方账号" });
    expect(within(dialog).getByLabelText("连接 Codex")).toBeChecked();
    await user.click(within(dialog).getByRole("button", { name: "下一步" }));
    await user.click(within(dialog).getByRole("button", { name: "继续" }));

    expect(startLogin).toHaveBeenCalledWith({
      provider: "openai",
      purpose: "connect_consumer",
      consumer: "codex",
      method: "browser_loopback",
      accountId: null,
    });
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

  it("does not treat an unbound Codex slot as a missing installation", async () => {
    const overview = managedAuthOverviewFixture();
    overview.connections[0] = {
      ...overview.connections[0],
      targetId: null,
      targetLabel: null,
    };
    renderPage(
      managedPorts({
        getOverview: vi.fn(async () => overview),
      }),
      "/auth?consumer=codex&view=connections",
    );

    expect(await screen.findByRole("heading", { name: "Codex" })).toBeVisible();
    expect(screen.getByText("OpenAI · person@example.com")).toBeVisible();
    expect(
      screen.queryByText("未检测到可管理的安装实例。账号页面不会自动安装软件。"),
    ).not.toBeInTheDocument();
  });

  it("does not claim Codex is using the saved account when native projection is unavailable", async () => {
    const overview = managedAuthOverviewFixture();
    overview.connections[0] = {
      ...overview.connections[0],
      reasonCodes: ["native_projection_unavailable"],
      requestMode: "third_party_api",
      requestProviderLabel: "custom",
    };
    renderPage(
      managedPorts({
        getOverview: vi.fn(async () => overview),
      }),
      "/auth?consumer=codex&view=connections",
    );

    const detail = await screen.findByRole("region", {
      name: "Codex 连接详情",
    });
    expect(within(detail).getAllByText("账号已保存").length).toBeGreaterThan(0);
    expect(screen.getByText("账号已保存，尚未写入软件")).toBeVisible();
    expect(within(detail).getByText("custom")).toBeVisible();
    expect(
      within(detail).getByText(
        "账号已保存在 FyAgent。还不能改写该软件的本地登录和模型来源，所以本机配置不会变。",
      ),
    ).toBeVisible();
    expect(within(detail).queryByText("已连接")).not.toBeInTheDocument();
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

  it("shows managed-auth reason copy when account removal returns a command error", async () => {
    const user = userEvent.setup();
    const removeAccount = vi.fn(async () => {
      throw {
        contractVersion: 1,
        reasonCode: "secret_unavailable",
      };
    });
    const ports = managedPorts({ removeAccount });
    renderPage(ports);

    await user.click(await screen.findByRole("button", { name: "移除账号" }));
    const dialog = await screen.findByRole("dialog", {
      name: "移除 person@example.com？",
    });
    await user.click(within(dialog).getByRole("button", { name: "移除账号" }));

    expect(
      await screen.findAllByText("系统凭据库暂时不可用。"),
    ).not.toHaveLength(0);
    expect(screen.queryByText("请稍后重试。")).not.toBeInTheDocument();
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

  it("shows recovery reasons with a refresh action instead of a generic unavailable banner", async () => {
    const user = userEvent.setup();
    const overview = managedAuthOverviewFixture();
    overview.reasonCodes = ["secret_unavailable", "migration_blocked"];
    overview.accounts[0] = {
      ...overview.accounts[0],
      health: "migration_blocked",
      reasonCodes: ["migration_blocked"],
    };
    overview.connections[0] = {
      ...overview.connections[0],
      authStatus: "pending_restart",
      pendingRestart: true,
      reasonCodes: ["pending_restart", "external_change_detected"],
      allowedActions: ["restart", "refresh", "open_consumer"],
    };
    const getOverview = vi.fn(async () => overview);
    renderPage(managedPorts({ getOverview }));

    expect(
      await screen.findByText("系统凭据库暂时不可用。"),
    ).toBeVisible();
    expect(
      screen.getAllByText("旧账号数据尚未完成安全迁移。").length,
    ).toBeGreaterThan(0);
    expect(
      screen.queryByText("部分账号状态暂时无法确认，请刷新后再进行危险操作。"),
    ).not.toBeInTheDocument();
    expect(screen.getAllByText("需要完成迁移").length).toBeGreaterThan(0);
    const accountDetail = screen.getByRole("region", {
      name: "person@example.com 账号详情",
    });
    expect(within(accountDetail).getAllByText("等待重启").length).toBeGreaterThan(
      0,
    );
    expect(
      screen.getAllByText(
        "检测到软件在 FyAgent 外部修改了登录信息，请刷新确认。",
      ).length,
    ).toBeGreaterThan(0);

    await user.click(screen.getByRole("button", { name: "刷新状态" }));
    await waitFor(() => expect(getOverview.mock.calls.length).toBeGreaterThan(1));
  });

  it("moves between account and connection tabs with the keyboard", async () => {
    const user = userEvent.setup();
    renderPage(managedPorts());

    const accountsTab = await screen.findByRole("tab", { name: /账号 2/ });
    await user.click(accountsTab);
    await user.keyboard("{ArrowRight}");

    const connectionsTab = screen.getByRole("tab", { name: /软件连接 4/ });
    expect(connectionsTab).toHaveFocus();
    expect(connectionsTab).toHaveAttribute("aria-selected", "true");
    expect(
      screen.getByRole("region", { name: "软件连接列表" }),
    ).toBeVisible();
  });

  it("closes the login dialog with Escape without leaving a second login owner", async () => {
    const user = userEvent.setup();
    renderPage(managedPorts());

    await user.click(await screen.findByRole("button", { name: "添加账号" }));
    expect(
      screen.getByRole("dialog", { name: "添加官方账号" }),
    ).toBeVisible();
    await user.keyboard("{Escape}");
    await waitFor(() => {
      expect(
        screen.queryByRole("dialog", { name: "添加官方账号" }),
      ).not.toBeInTheDocument();
      expect(screen.getByRole("button", { name: "添加账号" })).toHaveFocus();
    });
  });

  it("copies the device code without moving focus", async () => {
    const user = userEvent.setup();
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText },
    });
    renderPage(managedPorts(), "/auth?consumer=codex&view=connections");

    await user.click(await screen.findByRole("button", { name: "添加账号" }));
    const dialog = screen.getByRole("dialog", { name: "添加官方账号" });
    await user.click(within(dialog).getByLabelText("设备码登录"));
    await user.click(within(dialog).getByRole("button", { name: "下一步" }));
    await user.click(within(dialog).getByRole("button", { name: "继续" }));

    const copy = await within(dialog).findByRole("button", {
      name: "复制设备码",
    });
    copy.focus();
    await user.click(copy);
    expect(writeText).toHaveBeenCalledWith("ABCD-EFGH");
    expect(copy).toHaveFocus();
    expect(
      await within(dialog).findByRole("button", { name: "已复制设备码" }),
    ).toHaveFocus();
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
