import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter, useLocation } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";

import { AgentAuthStatusPanel } from "@/v2/pages/agents/AgentAuthStatusPanel";
import type {
  AgentAuthObservation,
  AgentAuthPort,
  AgentAuthSessionSnapshot,
} from "@/v2/shared/features/agent-auth";
import type { FeaturePorts } from "@/v2/shared/features/ports";
import { FeatureProvider } from "@/v2/shared/features/provider";
import { createBrowserFeaturePorts } from "@/v2/shared/platform/browser/features";

const SESSION_ID = "123e4567-e89b-12d3-a456-426614174000";
const PROVIDER_ID = `p1:${"a".repeat(32)}`;

function account(state: "logged_in" | "logged_out"): AgentAuthObservation {
  return {
    kind: "account",
    contractVersion: 1,
    agentId: "claude-code",
    ownership: "agent_owned",
    authority: "verified",
    state,
    allowedIntents: ["login", "logout"],
    checkedAt: "2026-08-30T00:00:00Z",
    reasonCodes: [],
  };
}

function session(
  observation: AgentAuthObservation,
  overrides: Partial<AgentAuthSessionSnapshot> = {},
): AgentAuthSessionSnapshot {
  return {
    contractVersion: 1,
    sessionId: SESSION_ID,
    agentId: observation.agentId,
    intent: "login",
    stage: "preparing",
    canStopWaiting: false,
    outcome: null,
    observation,
    reasonCode: null,
    ...overrides,
  };
}

function LocationProbe() {
  const location = useLocation();
  return (
    <output data-testid="test-location">
      {location.pathname}
      {location.search}
    </output>
  );
}

function renderPanel(
  agentId: AgentAuthObservation["agentId"],
  port: AgentAuthPort,
  enabled = true,
) {
  const ports: FeaturePorts = createBrowserFeaturePorts();
  ports.agentAuth = port;
  const initialEntry = `/agents?target=${agentId}&section=models`;
  const result = render(
    <MemoryRouter initialEntries={[initialEntry]}>
      <FeatureProvider ports={ports}>
        <AgentAuthStatusPanel agentId={agentId} enabled={enabled} />
      </FeatureProvider>
      <LocationProbe />
    </MemoryRouter>,
  );
  return {
    ...result,
    rerenderPanel(nextEnabled: boolean) {
      result.rerender(
        <MemoryRouter initialEntries={[initialEntry]}>
          <FeatureProvider ports={ports}>
            <AgentAuthStatusPanel agentId={agentId} enabled={nextEnabled} />
          </FeatureProvider>
          <LocationProbe />
        </MemoryRouter>,
      );
    },
  };
}

describe("AgentAuthStatusPanel", () => {
  it("keeps Claude awaiting until an authoritative reread verifies login", async () => {
    const before = account("logged_out");
    const verified = account("logged_in");
    const port: AgentAuthPort = {
      getObservation: vi.fn(async () => before),
      getActiveSession: vi.fn(async () => null),
      startSession: vi.fn(async () =>
        session(before, {
          stage: "awaiting_user",
          canStopWaiting: true,
        }),
      ),
      getSession: vi.fn(async () =>
        session(verified, {
          stage: "verified",
          outcome: "verified_logged_in",
        }),
      ),
      stopWaiting: vi.fn(),
    };
    renderPanel("claude-code", port);

    expect(await screen.findByText("未登录")).toBeVisible();
    fireEvent.click(screen.getByRole("button", { name: "登录" }));
    expect(await screen.findByText("等待你完成官方认证")).toBeVisible();
    expect(
      await screen.findByText("登录状态已更新", {}, { timeout: 2500 }),
    ).toBeVisible();
    expect(port.startSession).toHaveBeenCalledWith({
      agentId: "claude-code",
      intent: "login",
    });
    expect(port.getSession).toHaveBeenCalledWith(SESSION_ID);

    fireEvent.click(screen.getByRole("button", { name: "刷新状态" }));
    await waitFor(() =>
      expect(screen.queryByText("登录状态已更新")).not.toBeInTheDocument(),
    );
    expect(screen.getByText("未登录")).toBeVisible();
  });

  it("routes OpenCode provider management to the central page", async () => {
    const observation: AgentAuthObservation = {
      kind: "provider_connections",
      contractVersion: 1,
      agentId: "opencode",
      ownership: "provider_owned",
      authority: "verified",
      state: "configured",
      providers: [{ providerId: PROVIDER_ID, label: "OpenAI" }],
      allowedIntents: ["connect_provider", "logout"],
      checkedAt: "2026-08-30T00:00:00Z",
      reasonCodes: [],
    };
    const startSession = vi.fn();
    const getActiveSession = vi.fn(async () => null);
    renderPanel("opencode", {
      getObservation: vi.fn(async () => observation),
      getActiveSession,
      startSession,
      getSession: vi.fn(),
      stopWaiting: vi.fn(),
    });

    expect(await screen.findByText("已连接 1 个 Provider")).toBeVisible();
    fireEvent.click(screen.getByRole("button", { name: "管理连接" }));
    expect(screen.getByTestId("test-location")).toHaveTextContent(
      "/auth?consumer=opencode&view=connections&agentReturn=opencode&agentSection=models",
    );
    expect(startSession).not.toHaveBeenCalled();
    expect(getActiveSession).not.toHaveBeenCalled();
  });

  it("routes Grok login management centrally without claiming verification", async () => {
    const observation: AgentAuthObservation = {
      kind: "handoff_only",
      contractVersion: 1,
      agentId: "grokbuild",
      ownership: "agent_owned",
      authority: "unverified",
      allowedIntents: ["login", "logout"],
      checkedAt: "2026-08-30T00:00:00Z",
      reasonCodes: ["handoff_only"],
    };
    const startSession = vi.fn();
    renderPanel("grokbuild", {
      getObservation: vi.fn(async () => observation),
      getActiveSession: vi.fn(async () => null),
      startSession,
      getSession: vi.fn(),
      stopWaiting: vi.fn(),
    });

    expect(await screen.findByText("请在官方应用中登录")).toBeVisible();
    fireEvent.click(screen.getByRole("button", { name: "管理登录" }));
    expect(screen.getByTestId("test-location")).toHaveTextContent(
      "/auth?consumer=grokbuild&view=connections&agentReturn=grokbuild&agentSection=models",
    );
    expect(startSession).not.toHaveBeenCalled();
    expect(screen.queryByText("登录状态已更新")).not.toBeInTheDocument();
  });

  it("routes Codex to the central account and connection page", async () => {
    const observation: AgentAuthObservation = {
      kind: "fyagent_managed",
      contractVersion: 1,
      agentId: "codex",
      ownership: "fyagent_managed",
      authority: "verified",
      destination: "auth_center",
      allowedIntents: [],
      checkedAt: "2026-08-30T00:00:00Z",
      reasonCodes: ["managed_by_auth_center"],
    };
    const startSession = vi.fn();
    renderPanel("codex", {
      getObservation: vi.fn(async () => observation),
      getActiveSession: vi.fn(async () => null),
      startSession,
      getSession: vi.fn(),
      stopWaiting: vi.fn(),
    });

    expect(await screen.findByText("请在“账号与认证”中管理")).toBeVisible();
    fireEvent.click(screen.getByRole("button", { name: "管理账号" }));
    expect(screen.getByTestId("test-location")).toHaveTextContent(
      "/auth?consumer=codex&view=connections&agentReturn=codex&agentSection=models",
    );
    expect(startSession).not.toHaveBeenCalled();
    expect(
      screen.queryByRole("button", { name: "登录" }),
    ).not.toBeInTheDocument();
  });

  it("recovers an active backend session after the renderer remounts", async () => {
    const before = account("logged_out");
    const verified = account("logged_in");
    const active = session(before, {
      stage: "awaiting_user",
      canStopWaiting: true,
    });
    const getActiveSession = vi.fn(async () => active);
    const getSession = vi.fn(async () =>
      session(verified, {
        stage: "verified",
        outcome: "verified_logged_in",
      }),
    );
    renderPanel("claude-code", {
      getObservation: vi.fn(async () => before),
      getActiveSession,
      startSession: vi.fn(),
      getSession,
      stopWaiting: vi.fn(),
    });

    expect(await screen.findByText("等待你完成官方认证")).toBeVisible();
    expect(
      await screen.findByText("登录状态已更新", {}, { timeout: 2500 }),
    ).toBeVisible();
    expect(getActiveSession).toHaveBeenCalledWith("claude-code");
    expect(getSession).toHaveBeenCalledWith(SESSION_ID);
  });

  it("starts recovery when directory authority becomes enabled", async () => {
    const before = account("logged_out");
    const getActiveSession = vi.fn(async () => null);
    const view = renderPanel(
      "claude-code",
      {
        getObservation: vi.fn(async () => before),
        getActiveSession,
        startSession: vi.fn(),
        getSession: vi.fn(),
        stopWaiting: vi.fn(),
      },
      false,
    );

    expect(getActiveSession).not.toHaveBeenCalled();
    view.rerenderPanel(true);

    await waitFor(() =>
      expect(getActiveSession).toHaveBeenCalledWith("claude-code"),
    );
  });
});
