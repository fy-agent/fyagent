import {
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { AgentInstallReadinessSection } from "@/v2/pages/agents/AgentInstallReadinessSection";
import type {
  AgentActionJobSnapshot,
  AgentActionJobStage,
  AgentActionResult,
  AgentInstallationInventory,
  AgentInstallReadiness,
  AgentInstallReadinessPort,
} from "@/v2/shared/features/agent-install-readiness";

function readiness(agentId: "qoderwork" | "codex"): AgentInstallReadiness {
  const codex = agentId === "codex";
  return {
    contractVersion: 3,
    agentId,
    reviewedAt: "2026-08-29",
    installState: "unknown",
    inventoryState: "unknown",
    requiresTargetSelection: false,
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

function inventory(
  agentId: "qoderwork" | "codex",
  installed = false,
): AgentInstallationInventory {
  return {
    contractVersion: 1,
    inventoryId: `i1:${"a".repeat(32)}`,
    agentId,
    state: installed ? "single" : "not_observed",
    candidates: installed
      ? [
          {
            candidateId: `c1:${"b".repeat(32)}`,
            candidateRevision: `r1:${"c".repeat(64)}`,
            agentId,
            scope: "current_user",
            owner: "vendor_installer",
            packageKind: "app_bundle",
            localVersion: "0.9.12",
            launchEligible: true,
            installEligible: false,
            updateEligible: true,
            reasonCodes: [],
            evidenceCodes: ["bundle_identity"],
            locationLabel: "~/Applications/QoderWork CN.app",
          },
        ]
      : [],
    freshDestinations:
      agentId === "qoderwork" && !installed
        ? [
            {
              destinationId: `d1:${"d".repeat(32)}`,
              destinationRevision: `r1:${"e".repeat(64)}`,
              scope: "current_user",
              owner: "vendor_installer",
              packageKind: "app_bundle",
              requiresElevation: false,
              writable: true,
              eligible: true,
              reasonCodes: [],
              locationLabel: "~/Applications",
            },
          ]
        : [],
    reasonCodes: [],
  };
}

function portFor(data: AgentInstallReadiness): AgentInstallReadinessPort {
  return {
    get: vi.fn(async () => data),
    getInventory: vi.fn(async () =>
      inventory(data.agentId as "qoderwork" | "codex"),
    ),
    startAction: vi.fn(),
    cancelAction: vi.fn(),
    getActionJob: vi.fn(),
  };
}

describe("AgentInstallReadinessSection", () => {
  it("renders lifecycle actions without reusing legacy auth actions", async () => {
    const port = portFor(readiness("qoderwork"));
    render(<AgentInstallReadinessSection agentId="qoderwork" port={port} />);

    const region = screen.getByRole("region", { name: "安装方式" });
    expect(
      await within(region).findByRole("button", { name: "安装" }),
    ).toBeVisible();
    expect(
      within(region).queryByRole("button", { name: "登录" }),
    ).not.toBeInTheDocument();
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
          getInventory: async () => {
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

  it("shows native job stage and waits for succeeded instead of a 32s failure", async () => {
    const available: AgentInstallReadiness = {
      ...readiness("qoderwork"),
      installState: "installed",
      updateState: "update_available",
      localVersion: "0.9.12",
      remoteVersion: "0.9.15",
      releaseId: `v1:${"a".repeat(64)}`,
      allowedActions: ["update"],
    };
    const current: AgentInstallReadiness = {
      ...available,
      updateState: "up_to_date",
      localVersion: "0.9.15",
      allowedActions: ["launch", "auth_login"],
    };
    let stage: AgentActionJobStage = "downloading";
    const port: AgentInstallReadinessPort = {
      get: vi.fn(async () => (stage === "succeeded" ? current : available)),
      getInventory: vi.fn(async () => inventory("qoderwork", true)),
      startAction: vi.fn(
        async (): Promise<AgentActionResult> => ({
          contractVersion: 2,
          agentId: "qoderwork",
          action: "update",
          jobId: "job-1",
          stage: "checking",
          reasonCode: null,
        }),
      ),
      cancelAction: vi.fn(
        async (): Promise<AgentActionJobSnapshot> => ({
          contractVersion: 2,
          jobId: "job-1",
          agentId: "qoderwork",
          action: "update",
          stage: "cancelled",
          cancellable: false,
          reasonCode: "cancelled",
        }),
      ),
      getActionJob: vi.fn(
        async (): Promise<AgentActionJobSnapshot> => ({
          contractVersion: 2,
          jobId: "job-1",
          agentId: "qoderwork",
          action: "update",
          stage,
          cancellable: true,
          reasonCode: null,
        }),
      ),
    };
    render(<AgentInstallReadinessSection agentId="qoderwork" port={port} />);
    fireEvent.click(
      await screen.findByRole("button", { name: "更新到最新版" }),
    );
    expect(await screen.findByText("正在下载安装包")).toBeVisible();
    stage = "succeeded";
    await waitFor(
      () => {
        expect(
          screen.getByText("操作已完成。下面是再次读取的状态，不是推断。"),
        ).toBeVisible();
      },
      { timeout: 3000 },
    );
    expect(
      screen.queryByText("操作未能完成。此区域不会推断安装成功。"),
    ).not.toBeInTheDocument();
  });
});
