import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { CodexInstallConfirmation } from "@/shared/codex-desktop/CodexInstallConfirmation";
import type { CodexDesktopInstallerViewModel } from "@/shared/codex-desktop/useCodexDesktopInstaller";
import { codexInstallPreflightFixture } from "../../../fixtures/codexInstallPreflight";

function show(platform: "macos" | "windows", updating = false) {
  const confirmInstall = vi.fn();
  const dismissPreflight = vi.fn();
  const selectStagingDirectory = vi.fn();
  const preflight = {
    ...codexInstallPreflightFixture("release"),
    platform,
    updating,
    targetLabel: platform === "macos" ? "~/Applications" : "Windows 应用目录",
  };
  render(
    <CodexInstallConfirmation
      installer={
        {
          preflight,
          confirmInstall,
          dismissPreflight,
          selectStagingDirectory,
        } as unknown as CodexDesktopInstallerViewModel
      }
    />,
  );
  return { confirmInstall, dismissPreflight, selectStagingDirectory };
}

describe("Codex installation destination disclosure", () => {
  it("explains that macOS fresh installs cannot choose another directory", () => {
    const actions = show("macos");
    expect(
      screen.getByText(/当前安装方式暂不支持选择其他目录/),
    ).toHaveTextContent("~/Applications");
    expect(
      screen.queryByRole("button", { name: /选择.*目录/ }),
    ).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "暂不开始" }));
    expect(actions.dismissPreflight).toHaveBeenCalledOnce();
    expect(actions.confirmInstall).not.toHaveBeenCalled();
  });

  it("explains that macOS updates retain their existing location", () => {
    show("macos", true);
    expect(
      screen.getByText(/更新保留现有位置，暂不支持更改目录/),
    ).toBeVisible();
  });

  it("separates selectable download staging from the OS-managed MSIX destination", () => {
    const actions = show("windows");
    expect(screen.getByText(/Windows 管理 MSIX 的安装位置/)).toBeVisible();
    expect(screen.getByText(/仅下载暂存使用/)).toBeVisible();
    const picker = screen.getByRole("button", { name: "选择下载临时目录" });
    fireEvent.click(picker);
    expect(actions.selectStagingDirectory).toHaveBeenCalledOnce();
    expect(
      screen.queryByRole("button", { name: /选择安装位置/ }),
    ).not.toBeInTheDocument();
  });
});
