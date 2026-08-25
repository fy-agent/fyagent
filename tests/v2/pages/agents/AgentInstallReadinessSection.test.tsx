import { render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { AgentInstallReadinessSection } from "@/v2/pages/agents/AgentInstallReadinessSection";
import type {
  AgentInstallReadiness,
  AgentInstallReadinessPort,
} from "@/v2/shared/features/agent-install-readiness";

function readiness(agentId: "qoderwork" | "codex"): AgentInstallReadiness {
  const codex = agentId === "codex";
  return {
    contractVersion: 2,
    agentId,
    reviewedAt: "2026-08-25",
    installState: "unknown",
    updateState: codex ? "unknown" : "latest_unknown",
    releaseId: null,
    localVersion: null,
    remoteVersion: null,
    authOwnership: codex ? "fyagent_managed" : "agent_owned",
    authState: "unknown",
    sourceKind: codex ? "codex_desktop" : "managed_desktop",
    allowedActions: codex ? [] : ["install", "auth_login"],
    reasonCodes: codex
      ? ["managed_by_codex_desktop", "auth_state_unknown"]
      : ["auth_state_unknown"],
  };
}

function portFor(data: AgentInstallReadiness): AgentInstallReadinessPort {
  return {
    get: vi.fn(async () => data),
    startAction: vi.fn(),
    cancelAction: vi.fn(),
    getActionJob: vi.fn(),
  };
}

describe("AgentInstallReadinessSection", () => {
  it("renders backend-allowed actions without reconstructing URLs", async () => {
    const port = portFor(readiness("qoderwork"));
    render(<AgentInstallReadinessSection agentId="qoderwork" port={port} />);

    const region = screen.getByRole("region", { name: "安装方式" });
    expect(await within(region).findByRole("button", { name: "安装" })).toBeVisible();
    expect(within(region).getByRole("button", { name: "登录" })).toBeVisible();
    expect(within(region).getAllByText("未确认").length).toBeGreaterThan(0);
    expect(port.get).toHaveBeenCalledWith("qoderwork");
  });

  it("redirects Codex conceptually to the existing installer without adding an action", async () => {
    render(
      <AgentInstallReadinessSection
        agentId="codex"
        port={portFor(readiness("codex"))}
      />,
    );
    const region = screen.getByRole("region", { name: "安装方式" });
    expect(
      await within(region).findByText(
        "安装与更新由现有 Codex Desktop 安装器管理。",
      ),
    ).toBeVisible();
    expect(within(region).queryByRole("button")).not.toBeInTheDocument();
  });

  it("fails closed when the loader is unavailable", async () => {
    render(
      <AgentInstallReadinessSection
        agentId="qoderwork"
        port={{
          get: async () => {
            throw new Error("offline");
          },
          startAction: async () => {
            throw new Error("offline");
          },
          cancelAction: async () => {
            throw new Error("offline");
          },
          getActionJob: async () => {
            throw new Error("offline");
          },
        }}
      />,
    );
    expect(
      await screen.findByText(
        "当前无法读取安装准备度。此区域不会推断安装可用性。",
      ),
    ).toBeVisible();
  });
});
