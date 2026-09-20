import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { AgentSourceLinks } from "@/pages/agents/AgentSourceLinks";
import { resolveAgentSourceLinks } from "@/shared/features/agents";
import { FeatureProvider } from "@/shared/features/provider";
import { createBrowserFeaturePorts } from "@/shared/platform/browser/features";
import type { AgentOfficialLink } from "@/shared/features/types";

describe("AgentSourceLinks", () => {
  const grokLinks: AgentOfficialLink[] = [
    {
      id: "product",
      label: "打开 Grok Build 官方页面",
      url: "https://x.ai/build",
    },
    {
      id: "docs",
      label: "打开 Grok Build 官方文档",
      url: "https://docs.x.ai/build/overview",
    },
    {
      id: "download",
      label: "Grok Build 源码与安装说明",
      url: "https://github.com/xai-org/grok-build/blob/main/README.md",
    },
    {
      id: "license",
      label: "开源许可证 (Apache-2.0)",
      url: "https://github.com/xai-org/grok-build/blob/main/LICENSE",
    },
  ];

  const codexLinks: AgentOfficialLink[] = [
    {
      id: "product",
      label: "打开 OpenAI Codex 官方主页",
      url: "https://openai.com/codex/",
    },
    {
      id: "desktop",
      label: "Codex Desktop 官方页面",
      url: "https://openai.com/codex/",
    },
    {
      id: "download",
      label: "Codex CLI 安装与使用说明",
      url: "https://help.openai.com/en/articles/11096431",
    },
    {
      id: "terms",
      label: "OpenAI 使用条款",
      url: "https://openai.com/policies/terms-of-use/",
    },
  ];

  function renderWithProviders(
    ui: React.ReactElement,
    openExternalSpy = vi.fn(async () => undefined),
  ) {
    const ports = createBrowserFeaturePorts();
    ports.settings.openExternal = openExternalSpy;
    return render(<FeatureProvider ports={ports}>{ui}</FeatureProvider>);
  }

  it("renders verified official links using ExternalLinkButton without duplicate static tables", async () => {
    const openSpy = vi.fn(async () => undefined);
    const user = userEvent.setup();
    renderWithProviders(<AgentSourceLinks catalogLinks={grokLinks} />, openSpy);

    expect(
      screen.getByRole("region", { name: "官方来源与许可链接" }),
    ).toBeInTheDocument();

    const productBtn = screen.getByRole("button", {
      name: /打开 Grok Build 官方页面/,
    });
    expect(productBtn).toBeInTheDocument();

    await user.click(productBtn);
    expect(openSpy).toHaveBeenCalledWith("https://x.ai/build");
  });

  it("distinguishes open source licenses from proprietary terms of service", () => {
    const grokResolved = resolveAgentSourceLinks(grokLinks);
    const licenseItem = grokResolved.find((l) => l.category === "license");
    expect(licenseItem?.isOssLicense).toBe(true);
    expect(licenseItem?.licenseName).toBe("Apache-2.0");
    expect(licenseItem?.badge).toBe("开源许可");

    const codexResolved = resolveAgentSourceLinks(codexLinks);
    const termsItem = codexResolved.find((l) => l.category === "terms");
    expect(termsItem?.isOssLicense).toBe(false);
    expect(termsItem?.badge).toBe("服务协议");
    expect(termsItem?.url).toBe("https://openai.com/policies/terms-of-use/");
  });

  it("exposes both desktop and CLI entry links for Codex", () => {
    renderWithProviders(<AgentSourceLinks catalogLinks={codexLinks} />);

    expect(
      screen.getByRole("button", { name: /Codex Desktop 官方页面/ }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: /Codex CLI 安装与使用说明/ }),
    ).toBeInTheDocument();
  });

  it("returns null when catalogLinks is empty or undefined", () => {
    const { container: c1 } = renderWithProviders(
      <AgentSourceLinks catalogLinks={[]} />,
    );
    expect(c1.querySelector(".fy-agent-source-links")).toBeNull();

    const { container: c2 } = renderWithProviders(<AgentSourceLinks />);
    expect(c2.querySelector(".fy-agent-source-links")).toBeNull();
  });
});
