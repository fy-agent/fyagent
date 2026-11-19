import { useQueryClient } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { FeatureProvider } from "@/shared/features/provider";
import { featureKeys } from "@/shared/features/queries";
import { createBrowserFeaturePorts } from "@/shared/platform/browser/features";
import CodexReloadBanner from "@/widgets/app-shell/CodexReloadBanner";

function TriggerCodexMutation() {
  const queryClient = useQueryClient();
  return (
    <button
      type="button"
      onClick={() => {
        queryClient.setQueryData(featureKeys.providerMode("codex"), {
          mode: "direct",
          attached: false,
          routeProviderId: null,
          directProviderId: "quick-setup-codex",
        });
      }}
    >
      模拟保存 Codex 配置
    </button>
  );
}

describe("CodexReloadBanner", () => {
  it("shows reload guidance when Codex clients are stale, restarts daemon on click, and re-checks after Codex query updates", async () => {
    const user = userEvent.setup();
    const ports = createBrowserFeaturePorts();

    let staleState = { daemon: true, others: true };
    ports.providers.getCodexReloadStatus = vi.fn(async () => staleState);
    ports.providers.restartCodexDaemon = vi.fn(async () => {
      staleState = { daemon: false, others: true };
      return "restarted" as const;
    });

    render(
      <FeatureProvider ports={ports}>
        <CodexReloadBanner />
        <TriggerCodexMutation />
      </FeatureProvider>,
    );

    const banner = await screen.findByTestId("codex-reload-banner");
    expect(banner).toHaveTextContent("一键重启命令行服务");
    expect(banner).toHaveTextContent("macOS 按 ⌘Q，Windows 从托盘退出");

    await user.click(
      screen.getByRole("button", { name: "一键重启命令行服务" }),
    );
    await waitFor(() =>
      expect(ports.providers.restartCodexDaemon).toHaveBeenCalledOnce(),
    );
    expect(banner).toHaveTextContent("已重启 Codex 命令行后台服务");

    // Dismiss hides the banner
    await user.click(screen.getByRole("button", { name: "知道了" }));
    expect(
      screen.queryByTestId("codex-reload-banner"),
    ).not.toBeInTheDocument();

    // Updating a Codex query key inside FeatureProvider's QueryClient undismisses and re-evaluates stale status
    await user.click(
      screen.getByRole("button", { name: "模拟保存 Codex 配置" }),
    );

    expect(
      await screen.findByTestId("codex-reload-banner"),
    ).toHaveTextContent("Codex 桌面端或插件仍在使用旧模型配置");
  });
});
