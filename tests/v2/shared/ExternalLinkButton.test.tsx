import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { FeatureProvider } from "@/v2/shared/features/provider";
import { createBrowserFeaturePorts } from "@/v2/shared/platform/browser/features";
import { ExternalLinkButton } from "@/v2/shared/ui/ExternalLinkButton";

describe("ExternalLinkButton", () => {
  it("opens through settings.openExternal and holds one in-flight lock", async () => {
    const user = userEvent.setup();
    let releaseOpen!: () => void;
    const opening = new Promise<void>((resolve) => {
      releaseOpen = resolve;
    });
    const ports = createBrowserFeaturePorts();
    ports.settings.openExternal = vi.fn(() => opening);

    render(
      <FeatureProvider ports={ports}>
        <ExternalLinkButton url="https://example.test/docs">
          文档
        </ExternalLinkButton>
        <ExternalLinkButton url="https://example.test/home">
          主页
        </ExternalLinkButton>
      </FeatureProvider>,
    );

    const docs = screen.getByRole("button", { name: "文档" });
    const home = screen.getByRole("button", { name: "主页" });
    await user.click(docs);

    expect(docs).toHaveTextContent("正在打开…");
    expect(docs).toHaveAttribute("aria-busy", "true");
    expect(home).toBeDisabled();
    expect(ports.settings.openExternal).toHaveBeenCalledTimes(1);
    expect(ports.settings.openExternal).toHaveBeenCalledWith(
      "https://example.test/docs",
    );

    await user.click(home);
    expect(ports.settings.openExternal).toHaveBeenCalledTimes(1);

    releaseOpen();
    await waitFor(() => expect(home).toBeEnabled());
    expect(docs).toHaveTextContent("文档");
  });

  it("toasts a fixed title without echoing the url", async () => {
    const user = userEvent.setup();
    const ports = createBrowserFeaturePorts();
    ports.settings.openExternal = vi.fn(async () => {
      throw new Error("https://secret.example/docs leaked");
    });

    render(
      <FeatureProvider ports={ports}>
        <ExternalLinkButton
          url="https://secret.example/docs"
          errorTitle="无法打开官方入口"
        >
          文档
        </ExternalLinkButton>
      </FeatureProvider>,
    );

    await user.click(screen.getByRole("button", { name: "文档" }));
    expect(await screen.findByText("无法打开官方入口")).toBeVisible();
    expect(screen.getByText("请稍后重试。")).toBeVisible();
    expect(screen.queryByText(/secret\.example/)).not.toBeInTheDocument();
  });
});
