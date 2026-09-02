import {
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { AgentInstallReadinessSection } from "@/v2/pages/agents/AgentInstallReadinessSection";
import {
  AGENT_ACTION_CONTRACT_VERSION,
  AGENT_INSTALL_READINESS_CONTRACT_VERSION,
  type AgentActionJobSnapshot,
  type AgentActionJobStage,
  type AgentActionResult,
  type AgentInstallationInventory,
  type AgentInstallReadiness,
  type AgentInstallReadinessPort,
  type AgentSurfaceReadiness,
} from "@/v2/shared/features/agent-install-readiness";

function readiness(
  agentId: "qoderwork" | "codex" | "opencode" | "grokbuild" | "claude-code",
): AgentInstallReadiness {
  const codex = agentId === "codex";
  const grok = agentId === "grokbuild";
  return {
    contractVersion: AGENT_INSTALL_READINESS_CONTRACT_VERSION,
    agentId,
    reviewedAt: "2026-08-29",
    installState: "unknown",
    inventoryState: "unknown",
    requiresTargetSelection: false,
    updateState: codex ? "unknown" : "latest_unknown",
    releaseId: null,
    localVersion: null,
    remoteVersion: null,
    authOwnership: codex
      ? "fyagent_managed"
      : agentId === "opencode"
        ? "provider_owned"
        : "agent_owned",
    authState: "unknown",
    sourceKind: codex
      ? "codex_desktop"
      : grok
        ? "cli_tooling"
        : "managed_desktop",
    allowedActions: codex ? [] : ["install", "auth_login"],
    reasonCodes: codex
      ? ["managed_by_codex_desktop", "auth_state_unknown"]
      : ["auth_state_unknown"],
  };
}

function inventory(
  agentId: "qoderwork" | "codex" | "opencode" | "grokbuild" | "claude-code",
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

function surfaceReadiness(
  surface: "cli" | "desktop",
  overrides: Partial<AgentSurfaceReadiness> = {},
): AgentSurfaceReadiness {
  return {
    surface,
    installState: "not_installed",
    inventoryState: "not_observed",
    requiresTargetSelection: false,
    updateState: "latest_unknown",
    releaseId: null,
    localVersion: null,
    remoteVersion: null,
    sourceKind: surface === "cli" ? "cli_tooling" : "managed_desktop",
    allowedActions: ["install"],
    reasonCodes: [],
    ...overrides,
  };
}

function desktopReadiness(
  agentId: "opencode" | "claude-code",
  overrides: Partial<AgentSurfaceReadiness> = {},
): AgentInstallReadiness {
  const desktop = surfaceReadiness("desktop", overrides);
  return {
    ...readiness(agentId),
    installState: desktop.installState,
    inventoryState: desktop.inventoryState,
    requiresTargetSelection: desktop.requiresTargetSelection,
    updateState: desktop.updateState,
    releaseId: desktop.releaseId,
    localVersion: desktop.localVersion,
    remoteVersion: desktop.remoteVersion,
    sourceKind: "managed_desktop",
    allowedActions: desktop.allowedActions,
    reasonCodes: desktop.reasonCodes,
  };
}

function portFor(data: AgentInstallReadiness): AgentInstallReadinessPort {
  return {
    get: vi.fn(async () => data),
    getInventory: vi.fn(async (agentId) =>
      inventory(agentId as "qoderwork" | "codex" | "opencode"),
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

    const region = screen.getByRole("region", { name: "安装与更新" });
    expect(
      await within(region).findByRole("button", { name: "安装" }),
    ).toBeVisible();
    expect(
      within(region).queryByRole("button", { name: "登录" }),
    ).not.toBeInTheDocument();
    expect(within(region).getAllByText("暂时无法确认").length).toBeGreaterThan(
      0,
    );
    expect(port.get).toHaveBeenCalledWith("qoderwork");
  });

  it("redirects Codex conceptually to the existing installer without adding an action", async () => {
    render(
      <AgentInstallReadinessSection
        agentId="codex"
        port={portFor(readiness("codex"))}
      />,
    );
    const region = screen.getByRole("region", { name: "安装与更新" });
    expect(
      await within(region).findByText(
        "Codex Desktop 的安装和更新请在现有安装器中完成。",
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
      await screen.findByText("暂时无法检查安装状态。请重新打开此页面。"),
    ).toBeVisible();
  });

  it("shows native job stage and waits for succeeded instead of a 32s failure", async () => {
    const available: AgentInstallReadiness = {
      ...readiness("opencode"),
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
      getInventory: vi.fn(async () => inventory("opencode", true)),
      startAction: vi.fn(
        async (): Promise<AgentActionResult> => ({
          contractVersion: AGENT_ACTION_CONTRACT_VERSION,
          agentId: "opencode",
          action: "update",
          jobId: "job-1",
          stage: "checking",
          reasonCode: null,
        }),
      ),
      cancelAction: vi.fn(
        async (): Promise<AgentActionJobSnapshot> => ({
          contractVersion: AGENT_ACTION_CONTRACT_VERSION,
          jobId: "job-1",
          agentId: "opencode",
          action: "update",
          stage: "cancelled",
          cancellable: false,
          reasonCode: "cancelled",
          transfer: null,
        }),
      ),
      getActionJob: vi.fn(
        async (): Promise<AgentActionJobSnapshot> => ({
          contractVersion: AGENT_ACTION_CONTRACT_VERSION,
          jobId: "job-1",
          agentId: "opencode",
          action: "update",
          stage,
          cancellable: true,
          reasonCode: null,
          transfer: null,
        }),
      ),
    };
    render(<AgentInstallReadinessSection agentId="opencode" port={port} />);
    fireEvent.click(
      await screen.findByRole("button", { name: "更新当前位置" }),
    );
    expect(await screen.findByText("正在下载安装包")).toBeVisible();
    stage = "succeeded";
    await waitFor(
      () => {
        expect(screen.getByText("操作已完成，安装状态已更新。")).toBeVisible();
      },
      { timeout: 3000 },
    );
    expect(
      screen.queryByText("无法确认操作结果。请刷新安装状态后再试。"),
    ).not.toBeInTheDocument();
    expect(port.startAction).toHaveBeenCalledTimes(1);
    expect(port.startAction).toHaveBeenCalledWith(
      expect.objectContaining({ action: "update" }),
    );
    expect(port.startAction).not.toHaveBeenCalledWith(
      expect.objectContaining({ action: "launch" }),
    );
    expect(screen.getByRole("button", { name: "打开软件" })).toBeVisible();
  });

  it("labels desktop launch exactly 打开软件 on a single OpenCode desktop component", async () => {
    const data = desktopReadiness("opencode", {
      installState: "installed",
      inventoryState: "single",
      updateState: "up_to_date",
      localVersion: "1.18.19",
      allowedActions: ["launch"],
    });
    const desktopInventory = inventory("opencode", true);
    desktopInventory.candidates[0].localVersion = "1.18.19";
    const port: AgentInstallReadinessPort = {
      get: vi.fn(async () => data),
      getInventory: vi.fn(async () => desktopInventory),
      startAction: vi.fn(
        async (): Promise<AgentActionResult> => ({
          contractVersion: AGENT_ACTION_CONTRACT_VERSION,
          agentId: "opencode",
          action: "launch",
          jobId: null,
          stage: "succeeded",
          reasonCode: null,
        }),
      ),
      cancelAction: vi.fn(),
      getActionJob: vi.fn(),
    };
    render(<AgentInstallReadinessSection agentId="opencode" port={port} />);

    const region = await screen.findByRole("region", { name: "安装与更新" });
    expect(
      within(region).queryByRole("heading", { name: "命令行" }),
    ).not.toBeInTheDocument();
    expect(
      within(region).queryByRole("heading", { name: "桌面应用" }),
    ).not.toBeInTheDocument();
    const launch = within(region).getByRole("button", { name: "打开软件" });
    expect(launch).toBeVisible();
    expect(region.querySelector('[data-surface="cli"]')).toBeNull();
    expect(region.querySelector('[data-surface="desktop"]')).not.toBeNull();
    fireEvent.click(launch);
    await waitFor(() =>
      expect(port.startAction).toHaveBeenCalledWith(
        expect.objectContaining({
          agentId: "opencode",
          action: "launch",
        }),
      ),
    );
    expect(port.startAction).not.toHaveBeenCalledWith(
      expect.objectContaining({ action: "install" }),
    );
    expect(port.startAction).not.toHaveBeenCalledWith(
      expect.objectContaining({ surface: "cli" }),
    );
    expect(port.getInventory).toHaveBeenCalledWith("opencode");
    expect(port.getInventory).not.toHaveBeenCalledWith("opencode", "cli");
  });

  it("labels the Claude physical component Claude Desktop", async () => {
    const port = portFor(
      desktopReadiness("claude-code", {
        installState: "not_installed",
        allowedActions: ["install"],
      }),
    );
    render(<AgentInstallReadinessSection agentId="claude-code" port={port} />);
    const region = await screen.findByRole("region", { name: "安装与更新" });
    expect(
      within(region).getByRole("heading", { name: "Claude Desktop" }),
    ).toBeVisible();
    expect(
      within(region).queryByRole("heading", { name: "命令行" }),
    ).not.toBeInTheDocument();
    expect(within(region).getByRole("button", { name: "安装" })).toBeVisible();
  });

  it.each([
    {
      name: "not installed",
      installState: "not_installed" as const,
      allowedActions: ["install" as const],
      expectLaunch: false,
    },
    {
      name: "installed",
      installState: "installed" as const,
      inventoryState: "single" as const,
      updateState: "up_to_date" as const,
      localVersion: "1.18.19",
      allowedActions: ["launch" as const],
      expectLaunch: true,
    },
  ])("keeps OpenCode on one desktop row when $name", async (fixture) => {
    const port = portFor(
      desktopReadiness("opencode", {
        installState: fixture.installState,
        inventoryState: fixture.inventoryState,
        updateState: fixture.updateState,
        localVersion: fixture.localVersion,
        allowedActions: fixture.allowedActions,
      }),
    );
    render(<AgentInstallReadinessSection agentId="opencode" port={port} />);
    const region = await screen.findByRole("region", { name: "安装与更新" });
    expect(
      within(region).queryByRole("heading", { name: "命令行" }),
    ).not.toBeInTheDocument();
    expect(region.querySelector('[data-surface="cli"]')).toBeNull();
    const desktop = region.querySelector(
      '[data-surface="desktop"]',
    ) as HTMLElement;
    expect(desktop).not.toBeNull();
    if (fixture.expectLaunch) {
      expect(
        within(desktop).getByRole("button", { name: "打开软件" }),
      ).toBeVisible();
    } else {
      expect(
        within(desktop).queryByRole("button", { name: "打开软件" }),
      ).not.toBeInTheDocument();
    }
  });

  it("keeps a disabled system Applications destination visible and does not treat it as one-click", async () => {
    const available: AgentInstallReadiness = {
      ...readiness("qoderwork"),
      installState: "not_installed",
      allowedActions: ["install"],
    };
    const dests = inventory("qoderwork");
    dests.freshDestinations = [
      dests.freshDestinations[0],
      {
        destinationId: `d1:${"f".repeat(32)}`,
        destinationRevision: `r1:${"e".repeat(64)}`,
        scope: "all_users",
        owner: "vendor_installer",
        packageKind: "app_bundle",
        requiresElevation: true,
        writable: false,
        eligible: false,
        reasonCodes: ["authorization_required"],
        locationLabel: "系统应用程序文件夹",
      },
    ];
    const port: AgentInstallReadinessPort = {
      get: vi.fn(async () => available),
      getInventory: vi.fn(async () => dests),
      startAction: vi.fn(),
      cancelAction: vi.fn(),
      getActionJob: vi.fn(),
    };
    render(<AgentInstallReadinessSection agentId="qoderwork" port={port} />);
    fireEvent.click(await screen.findByRole("button", { name: "安装" }));
    expect(await screen.findByText("系统应用程序文件夹")).toBeVisible();
    expect(screen.getByText("所有用户")).toBeVisible();
    expect(
      screen.getByText(/系统应用程序文件夹目前不可用于一键安装/),
    ).toBeVisible();
    expect(
      screen.getByRole("radio", { name: /系统应用程序文件夹/ }),
    ).toBeDisabled();
    expect(port.startAction).not.toHaveBeenCalled();
  });

  it("shows real transfer copy while downloading and never invents 0 B/s", async () => {
    const available: AgentInstallReadiness = {
      ...readiness("qoderwork"),
      installState: "not_installed",
      allowedActions: ["install"],
    };
    let stage: AgentActionJobStage = "downloading";
    const port: AgentInstallReadinessPort = {
      get: vi.fn(async () => available),
      getInventory: vi.fn(async () => inventory("qoderwork")),
      startAction: vi.fn(
        async (): Promise<AgentActionResult> => ({
          contractVersion: AGENT_ACTION_CONTRACT_VERSION,
          agentId: "qoderwork",
          action: "install",
          jobId: "job-1",
          stage: "checking",
          reasonCode: null,
        }),
      ),
      cancelAction: vi.fn(),
      getActionJob: vi.fn(
        async (): Promise<AgentActionJobSnapshot> => ({
          contractVersion: AGENT_ACTION_CONTRACT_VERSION,
          jobId: "job-1",
          agentId: "qoderwork",
          action: "install",
          stage,
          cancellable: true,
          reasonCode: null,
          transfer: {
            phase: "download",
            completedBytes: 3744,
            totalBytes: 10_000,
            attempt: 1,
            maxAttempts: 3,
            sequence: 1,
            observedAt: "2026-08-14T00:00:01.000Z",
          },
        }),
      ),
    };
    render(<AgentInstallReadinessSection agentId="qoderwork" port={port} />);
    fireEvent.click(await screen.findByRole("button", { name: "安装" }));
    expect(await screen.findByText("下载中 37.4%")).toBeVisible();
    expect(screen.queryByText(/0 B\/s/)).not.toBeInTheDocument();
    stage = "succeeded";
  });

  it("shows Grok official npm as an explicit choice and does not auto-run it", async () => {
    const grokTooling = {
      getSnapshot: vi.fn(async () => ({
        localVersion: null,
        latestVersion: "1.0.6",
        distributionOwner: null,
        latestSource: "native_internal" as const,
        installedButBroken: false,
        error: null,
      })),
      installOfficialNpm: vi.fn(async () => undefined),
    };
    render(
      <AgentInstallReadinessSection
        agentId="grokbuild"
        port={portFor(readiness("grokbuild"))}
        grokTooling={grokTooling}
      />,
    );
    expect(await screen.findByText("使用官方 npm 方式")).toBeVisible();
    expect(screen.getByText("官方命令行最新")).toBeVisible();
    expect(grokTooling.installOfficialNpm).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "使用官方 npm 方式" }));
    await waitFor(() =>
      expect(grokTooling.installOfficialNpm).toHaveBeenCalledTimes(1),
    );
    expect(
      screen.queryByRole("button", { name: "打开软件" }),
    ).not.toBeInTheDocument();
  });

  it("offers 改用官方 npm 方式 only after a native failure and does not auto-run it", async () => {
    const installed: AgentInstallReadiness = {
      ...readiness("grokbuild"),
      installState: "installed",
      updateState: "update_available",
      localVersion: "1.0.5",
      remoteVersion: "1.0.6",
      allowedActions: ["update"],
    };
    const grokTooling = {
      getSnapshot: vi.fn(async () => ({
        localVersion: "1.0.5",
        latestVersion: "1.0.6",
        distributionOwner: "native_internal" as const,
        latestSource: "native_internal" as const,
        installedButBroken: false,
        error: null,
      })),
      installOfficialNpm: vi.fn(async () => undefined),
    };
    const port: AgentInstallReadinessPort = {
      get: vi.fn(async () => installed),
      getInventory: vi.fn(async () => inventory("grokbuild")),
      startAction: vi.fn(async () => {
        const error = new Error("failed") as Error & {
          reasonCode: "source_not_verified";
        };
        error.reasonCode = "source_not_verified";
        throw error;
      }),
      cancelAction: vi.fn(),
      getActionJob: vi.fn(),
    };
    render(
      <AgentInstallReadinessSection
        agentId="grokbuild"
        port={port}
        grokTooling={grokTooling}
      />,
    );
    expect(await screen.findByText("官方命令行")).toBeVisible();
    expect(
      screen.queryByRole("button", { name: "改用官方 npm 方式" }),
    ).not.toBeInTheDocument();
    fireEvent.click(
      await screen.findByRole("button", { name: "更新到最新版" }),
    );
    expect(
      await screen.findByRole("button", { name: "改用官方 npm 方式" }),
    ).toBeVisible();
    expect(grokTooling.installOfficialNpm).not.toHaveBeenCalled();
    expect(
      screen.queryByRole("button", { name: "打开软件" }),
    ).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "改用官方 npm 方式" }));
    await waitFor(() =>
      expect(grokTooling.installOfficialNpm).toHaveBeenCalledTimes(1),
    );
  });
});
