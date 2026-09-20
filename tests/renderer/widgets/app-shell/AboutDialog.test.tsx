import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { FeatureProvider } from "@/shared/features/provider";
import { createBrowserFeaturePorts } from "@/shared/platform/browser/features";
import { TopBar } from "@/widgets/app-shell/TopBar";

function fixture(readVersion = vi.fn(async () => "9.8.7")) {
  const ports = createBrowserFeaturePorts();
  ports.settings.getAppVersion = readVersion;
  const openExternal = vi.fn(async () => {});
  ports.settings.openExternal = openExternal;
  render(
    <FeatureProvider ports={ports}>
      <TopBar />
    </FeatureProvider>,
  );
  return { readVersion, openExternal, user: userEvent.setup() };
}

describe("About dialog", () => {
  it("loads the actual version on opening, opens only the selected link, and restores keyboard focus", async () => {
    const { readVersion, openExternal, user } = fixture();
    expect(readVersion).not.toHaveBeenCalled();
    const trigger = screen.getByRole("button", { name: "关于 FyAgent" });
    await user.click(trigger);
    const dialog = await screen.findByRole("dialog", { name: "关于 FyAgent" });
    expect(await within(dialog).findByText("9.8.7")).toBeVisible();
    expect(readVersion).toHaveBeenCalledTimes(1);
    expect(openExternal).not.toHaveBeenCalled();
    const details = within(dialog).getByText("发布与许可").closest("details");
    expect(details).not.toHaveAttribute("open");
    await user.click(within(dialog).getByRole("button", { name: "查看更新" }));
    expect(openExternal).toHaveBeenLastCalledWith(
      "https://github.com/fy-agent/fyagent/releases",
    );
    await user.click(
      within(dialog).getByRole("button", { name: "帮助与反馈" }),
    );
    expect(openExternal).toHaveBeenLastCalledWith(
      "https://github.com/fy-agent/fyagent/issues",
    );
    await user.click(within(dialog).getByText("发布与许可"));
    expect(details).toHaveAttribute("open");
    await user.click(within(dialog).getByRole("button", { name: "软件许可" }));
    expect(openExternal).toHaveBeenLastCalledWith(
      "https://github.com/fy-agent/fyagent/blob/main/LICENSING.md",
    );
    await user.keyboard("{Escape}");
    await waitFor(() =>
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument(),
    );
    await waitFor(() => expect(trigger).toHaveFocus());
  });

  it("shows a bounded unavailable state and retries without exposing diagnostics", async () => {
    const readVersion = vi
      .fn()
      .mockRejectedValueOnce(new Error("private diagnostic"))
      .mockResolvedValue("8.7.6");
    const { user, openExternal } = fixture(readVersion);
    await user.click(screen.getByRole("button", { name: "关于 FyAgent" }));
    expect(await screen.findByText("版本信息暂不可用")).toBeVisible();
    expect(screen.queryByText("private diagnostic")).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "重新读取" }));
    expect(await screen.findByText("8.7.6")).toBeVisible();
    expect(readVersion).toHaveBeenCalledTimes(2);
    expect(openExternal).not.toHaveBeenCalled();
    await user.click(screen.getByRole("button", { name: "关闭" }));
    await waitFor(() =>
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument(),
    );
  });
});
