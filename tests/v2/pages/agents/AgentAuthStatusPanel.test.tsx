import { fireEvent, render, screen, waitFor } from "@testing-library/react";
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

function renderPanel(
  agentId: AgentAuthObservation["agentId"],
  port: AgentAuthPort,
  enabled = true,
) {
  const ports: FeaturePorts = createBrowserFeaturePorts();
  ports.agentAuth = port;
  const result = render(
    <FeatureProvider ports={ports}>
      <AgentAuthStatusPanel agentId={agentId} enabled={enabled} />
    </FeatureProvider>,
  );
  return {
    ...result,
    rerenderPanel(nextEnabled: boolean) {
      result.rerender(
        <FeatureProvider ports={ports}>
          <AgentAuthStatusPanel agentId={agentId} enabled={nextEnabled} />
        </FeatureProvider>,
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

    expect(await screen.findByText("已验证退出")).toBeVisible();
    fireEvent.click(screen.getByRole("button", { name: "登录" }));
    expect(await screen.findByText("等待你完成官方认证")).toBeVisible();
    expect(
      await screen.findByText("认证结果已验证", {}, { timeout: 2500 }),
    ).toBeVisible();
    expect(port.startSession).toHaveBeenCalledWith({
      agentId: "claude-code",
      intent: "login",
    });
    expect(port.getSession).toHaveBeenCalledWith(SESSION_ID);

    fireEvent.click(screen.getByRole("button", { name: "刷新状态" }));
    await waitFor(() =>
      expect(screen.queryByText("认证结果已验证")).not.toBeInTheDocument(),
    );
    expect(screen.getByText("已验证退出")).toBeVisible();
  });

  it("disconnects one OpenCode provider instead of claiming a global logout", async () => {
    const before: AgentAuthObservation = {
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
    const after: AgentAuthObservation = {
      ...before,
      state: "empty",
      providers: [],
    };
    const startSession = vi.fn(async () =>
      session(after, {
        agentId: "opencode",
        intent: "logout",
        stage: "verified",
        outcome: "verified_provider_change",
      }),
    );
    renderPanel("opencode", {
      getObservation: vi.fn(async () => before),
      getActiveSession: vi.fn(async () => null),
      startSession,
      getSession: vi.fn(),
      stopWaiting: vi.fn(),
    });

    expect(await screen.findByText("已连接 1 个 Provider")).toBeVisible();
    fireEvent.click(screen.getByRole("button", { name: "断开" }));
    await waitFor(() =>
      expect(startSession).toHaveBeenCalledWith({
        agentId: "opencode",
        intent: "logout",
        providerId: PROVIDER_ID,
      }),
    );
    expect(await screen.findByText("认证结果已验证")).toBeVisible();
  });

  it("renders Grok as handoff-only and never as verified", async () => {
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
    renderPanel("grokbuild", {
      getObservation: vi.fn(async () => observation),
      getActiveSession: vi.fn(async () => null),
      startSession: vi.fn(async () =>
        session(observation, {
          agentId: "grokbuild",
          stage: "handoff_complete",
          outcome: "handoff_only",
          reasonCode: "handoff_only",
        }),
      ),
      getSession: vi.fn(),
      stopWaiting: vi.fn(),
    });

    expect(
      await screen.findByText("仅支持打开官方认证入口（终端 grok login）"),
    ).toBeVisible();
    expect(screen.getByText(/终端里自己运行 grok login/)).toBeVisible();
    expect(screen.getByText(/不会写进 Codex/)).toBeVisible();
    expect(screen.getByText(/SuperGrok 扫码请去认证中心/)).toBeVisible();
    expect(screen.queryByText("认证结果已验证")).not.toBeInTheDocument();
    expect(screen.queryByText("已登录", { exact: true })).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "刷新状态" }),
    ).not.toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "打开认证中心" }),
    ).toBeVisible();

    fireEvent.click(screen.getByRole("button", { name: "登录" }));
    expect(await screen.findByText("已交给官方认证入口")).toBeVisible();
    expect(screen.getByText(/请完成 grok login/)).toBeVisible();
    expect(screen.queryByText("认证结果已验证")).not.toBeInTheDocument();
    expect(screen.queryByText("已登录", { exact: true })).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "刷新状态" }),
    ).not.toBeInTheDocument();
  });

  it("keeps Codex delegated to the existing Auth Center", async () => {
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
    renderPanel("codex", {
      getObservation: vi.fn(async () => observation),
      getActiveSession: vi.fn(async () => null),
      startSession: vi.fn(),
      getSession: vi.fn(),
      stopWaiting: vi.fn(),
    });

    expect(await screen.findByText("由 FyAgent 认证中心管理")).toBeVisible();
    expect(screen.getByText(/SuperGrok 扫码也在认证中心/)).toBeVisible();
    expect(screen.getByText(/不要去终端跑 grok login/)).toBeVisible();
    expect(
      screen.getByRole("button", { name: "打开认证中心" }),
    ).toBeVisible();
    expect(
      screen.queryByRole("button", { name: "刷新状态" }),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "登录" }),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "连接 Provider" }),
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
      await screen.findByText("认证结果已验证", {}, { timeout: 2500 }),
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
