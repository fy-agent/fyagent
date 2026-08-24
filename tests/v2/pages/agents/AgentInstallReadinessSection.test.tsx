import { render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { AgentInstallReadinessSection } from "@/v2/pages/agents/AgentInstallReadinessSection";
import type { AgentInstallReadiness } from "@/v2/shared/features/agent-install-readiness";

function readiness(agentId: "qoderwork" | "codex"): AgentInstallReadiness {
  const codex = agentId === "codex";
  return {
    contractVersion: 1,
    agentId,
    reviewedAt: "2026-08-24",
    automation: {
      state: "unavailable",
      reasonCode: codex ? "managed_by_codex_desktop" : "official_guide_only",
    },
    source: {
      state: "unknown",
      reasonCode: "source_review_not_refreshed",
      installMode: codex ? "managed_package" : "official_guide",
      licenseScope: "unconfirmed",
      distributionState: "unconfirmed",
      checkedAt: null,
    },
    integrity: {
      state: "unknown",
      summaryCode: "integrity_not_checked",
      checkedAt: null,
    },
    preflight: {
      state: "unknown",
      reasonCode: "preflight_not_run",
      checks: [],
      checkedAt: null,
    },
    plan: {
      state: "unknown",
      reasonCode: "plan_not_created",
      snapshotId: null,
      snapshotStale: null,
    },
  };
}

describe("AgentInstallReadinessSection", () => {
  it("shows a compact non-positive official-guide summary without actions", async () => {
    const load = vi.fn(async () => readiness("qoderwork"));
    render(<AgentInstallReadinessSection agentId="qoderwork" load={load} />);

    const region = screen.getByRole("region", { name: "安装方式" });
    expect(
      await within(region).findByText(
        "当前仅提供官方指引，通用自动安装尚不可用。",
      ),
    ).toBeVisible();
    expect(within(region).getByText("不可用")).toBeVisible();
    expect(within(region).getAllByText("未确认")).toHaveLength(4);
    expect(within(region).queryByRole("button")).not.toBeInTheDocument();
    expect(load).toHaveBeenCalledWith("qoderwork");
  });

  it("redirects Codex conceptually to the existing installer without adding an action", async () => {
    render(
      <AgentInstallReadinessSection
        agentId="codex"
        load={async () => readiness("codex")}
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
        agentId="workbuddy"
        load={async () => {
          throw new Error("native only");
        }}
      />,
    );
    const region = screen.getByRole("region", { name: "安装方式" });
    expect(
      await within(region).findByText(
        "当前无法读取安装准备度。此区域不会推断安装可用性。",
      ),
    ).toBeVisible();
    expect(region).not.toHaveTextContent("已确认");
    expect(within(region).queryByRole("button")).not.toBeInTheDocument();
  });
});
