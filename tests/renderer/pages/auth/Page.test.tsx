import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter, useLocation } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";

import { AuthPage } from "@/pages/auth/Page";
import type { AgentAuthPort } from "@/shared/features/agent-auth";
import type { ManagedAuthConnectionActionRequest } from "@/shared/features/managed-auth";
import type { FeaturePorts } from "@/shared/features/ports";
import { FeatureProvider } from "@/shared/features/provider";
import { createBrowserFeaturePorts } from "@/shared/platform/browser/features";
import { TooltipProvider } from "@/shared/ui/primitives";
import {
  ACCOUNT_REVISION,
  CODEX_CONNECTION_ID,
  CONNECTION_REVISION,
  OPENAI_ACCOUNT_ID,
  XAI_ACCOUNT_ID,
  GROK_CONNECTION_ID,
  OPENCODE_CONNECTION_ID,
  PREVIEW_ID,
  deviceLoginSessionFixture,
  managedAuthOverviewFixture,
  mutationResultFixture,
  removalPreviewFixture,
  connectionPreviewFixture,
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
    previewConnectionAction: vi.fn(async (request) =>
      connectionPreviewFixture(request),
    ),
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
  it("offers Grok official CLI handoff without claiming login or invoking xAI OAuth", async () => {
    const user = userEvent.setup();
    const ports = managedPorts();
    const observation = {
      contractVersion: 1 as const,
      agentId: "grokbuild" as const,
      kind: "handoff_only" as const,
      ownership: "agent_owned" as const,
      authority: "unverified" as const,
      allowedIntents: ["login", "logout"] as ("login" | "logout")[],
      checkedAt: "2026-09-20T00:00:00Z",
      reasonCodes: ["handoff_only"] as "handoff_only"[],
    };
    ports.agentAuth = {
      getObservation: vi.fn(async () => observation),
      getActiveSession: vi.fn(async () => null),
      startSession: vi.fn<AgentAuthPort["startSession"]>(async (request) => ({
        contractVersion: 1,
        sessionId: "123e4567-e89b-42d3-a456-426614174000",
        agentId: "grokbuild",
        intent: request.intent,
        stage: "handoff_complete",
        canStopWaiting: false,
        outcome: "handoff_only",
        observation,
        reasonCode: "handoff_only",
      })),
      getSession: vi.fn(),
      stopWaiting: vi.fn(),
    };
    renderPage(ports, "/auth?consumer=grokbuild&view=connections");
    const login = await screen.findByRole("button", {
      name: "打开官方 CLI 登录",
    });
    await waitFor(() => expect(login).toBeEnabled());
    await user.click(login);
    expect(
      await screen.findByText("已打开官方 CLI 登录入口，请在终端完成登录。"),
    ).toBeVisible();
    expect(ports.agentAuth.startSession).toHaveBeenCalledWith({
      agentId: "grokbuild",
      intent: "login",
    });
    expect(ports.managedAuth.startLogin).not.toHaveBeenCalled();
    await user.click(screen.getByRole("button", { name: "退出官方 CLI 登录" }));
    const dialog = screen.getByRole("dialog", {
      name: "退出 Grok 官方 CLI 登录",
    });
    expect(ports.agentAuth.startSession).toHaveBeenCalledTimes(1);
    await user.click(
      within(dialog).getByRole("button", { name: "确认退出官方 CLI" }),
    );
    expect(
      await screen.findByText(
        "已向官方 CLI 发出退出操作，请在官方终端确认结果。",
      ),
    ).toBeVisible();
    expect(ports.agentAuth.startSession).toHaveBeenLastCalledWith({
      agentId: "grokbuild",
      intent: "logout",
    });
  });

  it("does not present a gated Grok connection as an account reuse or device-code target", async () => {
    const user = userEvent.setup();
    const overview = managedAuthOverviewFixture();
    const grok = overview.connections.find(
      (item) => item.connectionId === GROK_CONNECTION_ID,
    )!;
    Object.assign(grok, {
      accountId: null,
      authStatus: "disconnected",
      allowedActions: ["refresh"],
      reasonCodes: ["native_projection_unavailable"],
    });
    renderPage(
      managedPorts({ getOverview: vi.fn(async () => overview) }),
      `/auth?account=${XAI_ACCOUNT_ID}`,
    );
    const detail = await screen.findByRole("region", {
      name: "xai@example.com 账号详情",
    });
    expect(
      within(detail).queryByRole("button", { name: "连接 Grok Build" }),
    ).not.toBeInTheDocument();
    await user.click(within(detail).getByRole("button", { name: "重新登录" }));
    expect(screen.getByRole("dialog")).toHaveTextContent("xAI 设备码账号");
    expect(screen.getByRole("dialog")).not.toHaveTextContent("用于 Grok Build");
  });

  it("preserves target A success when B fails and retries B with a fresh preview", async () => {
    const user = userEvent.setup();
    let overview = managedAuthOverviewFixture();
    overview.connections = overview.connections
      .filter((item) =>
        [CODEX_CONNECTION_ID, OPENCODE_CONNECTION_ID].includes(
          item.connectionId,
        ),
      )
      .map((item) => ({
        ...item,
        provider: "openai",
        accountId: null,
        authStatus: "disconnected",
        allowedActions: ["connect_account", "refresh"],
        reasonCodes: [],
      }));
    let previewSequence = 0;
    const previewConnectionAction = vi.fn(
      async (request: ManagedAuthConnectionActionRequest) => ({
        ...connectionPreviewFixture(request),
        previewId: `323e4567-e89b-42d3-a456-${String(++previewSequence).padStart(12, "0")}`,
      }),
    );
    let failB = true;
    const applyConnectionAction = vi.fn<
      FeaturePorts["managedAuth"]["applyConnectionAction"]
    >(async (request) => {
      if (request.connectionId === OPENCODE_CONNECTION_ID && failB) {
        failB = false;
        throw { contractVersion: 1, reasonCode: "external_change_detected" };
      }
      overview = {
        ...overview,
        connections: overview.connections.map((connection) =>
          connection.connectionId === request.connectionId
            ? {
                ...connection,
                accountId: OPENAI_ACCOUNT_ID,
                authStatus: "connected",
                allowedActions: ["refresh"],
              }
            : connection,
        ),
      };
      return mutationResultFixture(overview);
    });
    const ports = managedPorts({
      getOverview: vi.fn(async () => overview),
      applyConnectionAction,
      previewConnectionAction,
    });
    ports.configRecovery.list = vi.fn(async () => []);
    renderPage(ports);
    const connect = async (name: string) => {
      const card = (await screen.findByRole("heading", { name })).closest(
        "article",
      )!;
      await user.click(
        within(card).getByRole("button", { name: "用此账号连接" }),
      );
      const dialog = screen.getByRole("dialog");
      await waitFor(() =>
        expect(
          within(dialog).getByRole("button", { name: "确认" }),
        ).toBeEnabled(),
      );
      await user.click(within(dialog).getByRole("button", { name: "确认" }));
      await waitFor(() =>
        expect(screen.queryByRole("dialog")).not.toBeInTheDocument(),
      );
    };
    await connect("Codex");
    await connect("OpenCode Desktop");
    const a = screen.getByRole("article", { name: "Codex · OpenAI 操作结果" });
    const b = screen.getByRole("article", {
      name: "OpenCode Desktop · OpenAI 操作结果",
    });
    expect(a).toHaveTextContent("此连接操作已完成并回读。");
    expect(b).toHaveTextContent("检测到软件在 FyAgent 外部修改了登录信息");
    const firstB = applyConnectionAction.mock.calls.find(
      ([request]) => request.connectionId === OPENCODE_CONNECTION_ID,
    )![1];
    await user.click(within(b).getByRole("button", { name: "重试此连接" }));
    const retry = screen.getByRole("dialog");
    await waitFor(() =>
      expect(within(retry).getByRole("button", { name: "确认" })).toBeEnabled(),
    );
    await user.click(within(retry).getByRole("button", { name: "确认" }));
    await waitFor(() =>
      expect(b).toHaveTextContent("此连接操作已完成并回读。"),
    );
    expect(
      applyConnectionAction.mock.calls.filter(
        ([request]) => request.connectionId === CODEX_CONNECTION_ID,
      ),
    ).toHaveLength(1);
    const bCalls = applyConnectionAction.mock.calls.filter(
      ([request]) => request.connectionId === OPENCODE_CONNECTION_ID,
    );
    expect(bCalls).toHaveLength(2);
    expect(bCalls[1][1]).not.toBe(firstB);
    expect(a).toHaveTextContent("此连接操作已完成并回读。");
    await user.click(within(b).getByRole("button", { name: "撤回文件修改" }));
    await waitFor(() =>
      expect(ports.configRecovery.list).toHaveBeenCalledWith(["opencode_auth"]),
    );
  });

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
    expect(
      within(connectedSection!).getAllByText("Codex", { exact: true }),
    ).toHaveLength(1);
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

  it("retains a connection target that differs from the software name", async () => {
    const overview = managedAuthOverviewFixture();
    overview.connections[0] = {
      ...overview.connections[0],
      targetLabel: "Codex · 工作配置",
    };
    renderPage(managedPorts({ getOverview: vi.fn(async () => overview) }));
    const detail = await screen.findByRole("region", {
      name: "person@example.com 账号详情",
    });
    expect(
      within(detail).getByText("Codex · 工作配置", { exact: true }),
    ).toBeVisible();
    expect(
      within(detail).getByRole("heading", { name: "Codex" }),
    ).toBeVisible();
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

    expect(applyConnectionAction).toHaveBeenCalledWith(
      {
        connectionId: CODEX_CONNECTION_ID,
        expectedRevision: CONNECTION_REVISION,
        action: "connect_account",
        accountId: OPENAI_ACCOUNT_ID,
      },
      "323e4567-e89b-42d3-a456-426614174000",
    );
  });

  it("lets a saved-only Codex account connect even when the card still names that account", async () => {
    const user = userEvent.setup();
    const overview = managedAuthOverviewFixture();
    overview.connections[0] = {
      ...overview.connections[0],
      accountId: OPENAI_ACCOUNT_ID,
      authStatus: "disconnected",
      requestMode: "official_subscription",
      requestProviderLabel: "openai",
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

    expect(applyConnectionAction).toHaveBeenCalledWith(
      {
        connectionId: CODEX_CONNECTION_ID,
        expectedRevision: CONNECTION_REVISION,
        action: "connect_account",
        accountId: OPENAI_ACCOUNT_ID,
      },
      "323e4567-e89b-42d3-a456-426614174000",
    );
  });

  it("lets a disconnected Codex slot restore the unofficial model source", async () => {
    const user = userEvent.setup();
    const overview = managedAuthOverviewFixture();
    overview.connections[0] = {
      ...overview.connections[0],
      accountId: OPENAI_ACCOUNT_ID,
      authStatus: "disconnected",
      requestMode: "official_subscription",
      requestProviderLabel: "openai",
      allowedActions: ["connect_account", "disconnect", "refresh"],
    };
    overview.accounts[0] = {
      ...overview.accounts[0],
      connectedConsumerCount: 1,
    };
    const applyConnectionAction = vi.fn(async () => {
      const next = managedAuthOverviewFixture();
      return mutationResultFixture(next);
    });
    const previewConnectionAction = vi.fn(async (request) => ({
      ...connectionPreviewFixture(request),
      writeTargets:
        request.action === "disconnect"
          ? [
              {
                path: "~/.codex/config.toml",
                backupPath: "~/.codex/config.toml.fyagent.backup",
                exists: true,
              },
            ]
          : connectionPreviewFixture(request).writeTargets,
      preservedPaths:
        request.action === "disconnect"
          ? ["~/.codex/auth.json"]
          : connectionPreviewFixture(request).preservedPaths,
    }));
    renderPage(
      managedPorts({
        getOverview: vi.fn(async () => overview),
        applyConnectionAction,
        previewConnectionAction,
      }),
    );

    const connectSection = (
      await screen.findByRole("heading", { name: "连接到软件" })
    ).closest("section");
    expect(connectSection).not.toBeNull();
    await user.click(
      within(connectSection!).getByRole("button", {
        name: "恢复第三方模型来源",
      }),
    );
    const dialog = screen.getByRole("dialog", {
      name: "恢复 Codex 的第三方模型来源？",
    });
    expect(await within(dialog).findByText("将修改")).toBeVisible();
    expect(within(dialog).getByText("~/.codex/config.toml")).toBeVisible();
    await user.click(within(dialog).getByRole("button", { name: "恢复" }));

    expect(applyConnectionAction).toHaveBeenCalledWith(
      {
        connectionId: CODEX_CONNECTION_ID,
        expectedRevision: CONNECTION_REVISION,
        action: "disconnect",
        accountId: null,
      },
      "323e4567-e89b-42d3-a456-426614174000",
    );
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
      screen.queryByText(
        "未检测到可管理的安装实例。账号页面不会自动安装软件。",
      ),
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
        "FyAgent 暂时不能改写该软件的本地登录。已保存的账号不代表软件已登录，请使用官方登录入口。",
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
    let currentId = "third-party";
    const applyConnectionAction = vi.fn(async () => {
      currentId = "official";
      const overview = managedAuthOverviewFixture();
      overview.connections[0] = {
        ...overview.connections[0],
        requestMode: "official_subscription",
        requestProviderLabel: "OpenAI 官方订阅",
      };
      return mutationResultFixture(overview);
    });
    const ports = managedPorts({ applyConnectionAction });
    ports.providers.getSummary = vi.fn(async () => ({
      currentId,
      providers: {
        official: { id: "official", name: "Official" },
        "third-party": { id: "third-party", name: "Third-party API" },
      },
      writeTargets: [],
    }));
    ports.changePlans.listRecoverableChangeJobs = vi.fn(async () => []);
    renderPage(ports, "/auth?consumer=codex&view=connections");
    expect(await screen.findByRole("combobox", { name: "切换到" })).toHaveValue(
      "official",
    );

    await user.click(await screen.findByRole("button", { name: "切回官方" }));
    const dialog = screen.getByRole("dialog", {
      name: "切回 Codex 官方模式？",
    });
    expect(dialog).toHaveTextContent("DeepSeek API");
    await user.click(within(dialog).getByRole("button", { name: "切换" }));

    expect(applyConnectionAction).toHaveBeenCalledWith(
      {
        connectionId: CODEX_CONNECTION_ID,
        expectedRevision: CONNECTION_REVISION,
        action: "switch_to_official",
        accountId: null,
      },
      "323e4567-e89b-42d3-a456-426614174000",
    );
    await waitFor(() => {
      const detail = screen.getByRole("region", {
        name: "Codex 连接详情",
      });
      expect(detail).toHaveTextContent("OpenAI 官方订阅");
    });
    await waitFor(() =>
      expect(ports.providers.getSummary).toHaveBeenCalledTimes(2),
    );
    expect(screen.getByRole("combobox", { name: "切换到" })).toHaveValue(
      "third-party",
    );
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

    expect(await screen.findByText("系统凭据库暂时不可用。")).toBeVisible();
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
    expect(
      within(accountDetail).getAllByText("等待重启").length,
    ).toBeGreaterThan(0);
    expect(
      screen.getAllByText(
        "检测到软件在 FyAgent 外部修改了登录信息，请刷新确认。",
      ).length,
    ).toBeGreaterThan(0);

    await user.click(screen.getByRole("button", { name: "刷新状态" }));
    await waitFor(() =>
      expect(getOverview.mock.calls.length).toBeGreaterThan(1),
    );
  });

  it("moves between account and connection tabs with the keyboard", async () => {
    const user = userEvent.setup();
    renderPage(managedPorts());

    const accountsTab = await screen.findByRole("tab", { name: /账号 2/ });
    await user.click(accountsTab);
    await user.keyboard("{ArrowRight}");

    const connectionsTab = screen.getByRole("tab", { name: /软件连接 4/ });
    expect(connectionsTab).toHaveFocus();
    // Radix moves focus synchronously; selected state is owned by a separate
    // Router update. Await that commit rather than racing an effect under load.
    await waitFor(() => {
      expect(connectionsTab).toHaveAttribute("aria-selected", "true");
      expect(
        screen.getByRole("region", { name: "软件连接列表" }),
      ).toBeVisible();
    });
  });

  it("closes the login dialog with Escape without leaving a second login owner", async () => {
    const user = userEvent.setup();
    renderPage(managedPorts());

    await user.click(await screen.findByRole("button", { name: "添加账号" }));
    expect(screen.getByRole("dialog", { name: "添加官方账号" })).toBeVisible();
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
