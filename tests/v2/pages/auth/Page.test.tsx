import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";

import { AuthPage } from "@/v2/pages/auth/Page";
import type { FeaturePorts } from "@/v2/shared/features/ports";
import { FeatureProvider } from "@/v2/shared/features/provider";
import { createBrowserFeaturePorts } from "@/v2/shared/platform/browser/features";
import { managedAuthOverviewFixture } from "../../fixtures/managedAuth";

function renderPage(ports: FeaturePorts, entry = "/auth") {
  return render(
    <MemoryRouter initialEntries={[entry]}>
      <FeatureProvider ports={ports}>
        <AuthPage />
      </FeatureProvider>
    </MemoryRouter>,
  );
}

function fixturePorts(): FeaturePorts {
  const ports = createBrowserFeaturePorts();
  const overview = managedAuthOverviewFixture();
  ports.managedAuth = {
    getOverview: vi.fn(async () => overview),
    getActiveLoginSession: vi.fn(async () => null),
    startLogin: vi.fn(),
    getLoginSession: vi.fn(),
    reopenLogin: vi.fn(),
    switchLoginMethod: vi.fn(),
    cancelLogin: vi.fn(),
    previewAccountRemoval: vi.fn(),
    mutateAccount: vi.fn(),
    mutateConnection: vi.fn(),
  };
  return ports;
}

describe("Managed Auth page", () => {
  it("renders account identity separately from software connections and request source", async () => {
    const user = userEvent.setup();
    renderPage(fixturePorts());

    expect(await screen.findByRole("heading", { name: "账号与认证" })).toBeVisible();
    const tabs = screen.getByRole("tablist", { name: "账号与认证视图" });
    expect(within(tabs).getByRole("tab", { name: /账号/u })).toBeVisible();
    const connections = within(tabs).getByRole("tab", { name: /软件连接/u });
    await user.click(connections);
    expect(connections).toHaveAttribute("aria-selected", "true");
    expect(screen.getByText(/当前模型来源/u)).toBeVisible();
    expect(screen.getByText(/账号连接|Provider 连接/u)).toBeVisible();
  });

  it("renders a controlled desktop-only state in a browser instead of seeded login data", async () => {
    renderPage(createBrowserFeaturePorts());

    expect(await screen.findByText(/仅在 FyAgent 桌面应用中可用/u)).toBeVisible();
    expect(screen.queryByText(/已登录/u)).not.toBeInTheDocument();
    expect(screen.queryByText(/已连接/u)).not.toBeInTheDocument();
  });

  it("accepts only closed consumer selection from the route", async () => {
    renderPage(fixturePorts(), "/auth?view=connections&consumer=codex");

    const tabs = await screen.findByRole("tablist", { name: "账号与认证视图" });
    expect(within(tabs).getByRole("tab", { name: /软件连接/u })).toHaveAttribute(
      "aria-selected",
      "true",
    );
    expect(document.body.textContent).not.toMatch(/refresh[_ -]?token|secretRef|authorization code/iu);
  });
});
