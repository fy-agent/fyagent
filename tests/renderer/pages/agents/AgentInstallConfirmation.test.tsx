import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { AgentInstallConfirmation } from "@/pages/agents/AgentInstallConfirmation";
import type { AgentLifecycleActionView } from "@/pages/agents/useAgentLifecycleAction";
import type { AgentInstallPreflight } from "@/shared/features/agent-install-readiness";
import { installPreflightFixture } from "../../../fixtures/agentInstallPreflight";

function show(preflight: AgentInstallPreflight) {
  const confirm = vi.fn();
  const dismissPreflight = vi.fn();
  render(
    <AgentInstallConfirmation
      name="QoderWork"
      lifecycle={
        {
          preflight,
          confirm,
          dismissPreflight,
        } as unknown as AgentLifecycleActionView
      }
    />,
  );
  return { confirm, dismissPreflight };
}

describe("Agent install space confirmation", () => {
  it("discloses the download-cap budget and requires explicit confirmation", () => {
    const actions = show(
      installPreflightFixture({ agentId: "qoderwork", action: "install" }),
    );
    expect(screen.getByText(/需预留 6.0 GiB/)).toHaveTextContent(
      "最少可用 10.0 GiB",
    );
    expect(screen.getByText(/按下载器的 2 GiB 上限/)).toHaveTextContent(
      "不是厂商精确安装需求",
    );
    expect(actions.confirm).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "确认安装" }));
    expect(actions.confirm).toHaveBeenCalledOnce();
  });

  it("shows exact-source estimated budget separately from available space", () => {
    show({
      ...installPreflightFixture({ agentId: "qoderwork", action: "install" }),
      artifactSizeBytes: 1024 ** 3,
      requiredBytes: 3 * 1024 ** 3,
      spaceBudgetBasis: "source_size",
    });
    expect(screen.getByText(/需预留 3.0 GiB/)).toBeVisible();
    expect(screen.getByText(/按此安装包元数据大小的 3 倍/)).toHaveTextContent(
      "确认时会重新检查",
    );
  });

  it("does not claim an unknown CLI budget is sufficient", () => {
    show({
      ...installPreflightFixture({
        agentId: "claude-code",
        surface: "cli",
        action: "install",
      }),
      downloadUrl: null,
      requiredBytes: null,
      runtime: "node_npm",
      spaceBudgetBasis: "cli_unknown",
    });
    expect(screen.getByText(/安装预留预算尚未核定/)).toBeVisible();
    expect(screen.getByText(/可用空间数值不能证明容量足够/)).toBeVisible();
    expect(screen.queryByText(/需预留 6.0/)).not.toBeInTheDocument();
  });

  it("shows package reserve budget for npm CLI installation", () => {
    show({
      ...installPreflightFixture({
        agentId: "claude-code",
        surface: "cli",
        action: "install",
      }),
      downloadUrl: null,
      artifactSizeBytes: 217880104,
      requiredBytes: 217880104 * 3,
      runtime: "node_npm",
      spaceBudgetBasis: "package_reserve",
    });
    expect(screen.getByText(/需预留 0.6 GiB/)).toBeVisible();
    expect(
      screen.getByText(/按已核实 npm 包及依赖大小的 3 倍预留 FyAgent 保守预算/),
    ).toHaveTextContent("这是预留空间，不是厂商保证的完整峰值");
  });
});
