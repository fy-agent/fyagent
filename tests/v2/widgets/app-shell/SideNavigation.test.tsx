import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";

vi.mock("@samasante/liquid-glass", () => ({
  Glass: ({
    children,
    className,
    "data-testid": testId,
  }: {
    children?: React.ReactNode;
    className?: string;
    "data-testid"?: string;
  }) => (
    <span className={className} data-testid={testId}>
      {children}
    </span>
  ),
}));

import {
  navigationGroups,
  navigationItems,
} from "@/v2/shared/config/navigation";
import { SideNavigation } from "@/v2/widgets/app-shell/SideNavigation";

function renderNavigation(initialEntry = "/agents") {
  return render(
    <MemoryRouter initialEntries={[initialEntry]}>
      <SideNavigation />
    </MemoryRouter>,
  );
}

describe("SideNavigation", () => {
  it("derives the stable six route leaves from three typed groups", () => {
    expect(
      navigationGroups.map(({ id, label, collapsible, items }) => ({
        id,
        label,
        collapsible,
        items: items.map((item) => ({ id: item.id, label: item.label })),
      })),
    ).toEqual([
      {
        id: "agent-configuration",
        label: "AI软件配置",
        collapsible: false,
        items: [{ id: "agents", label: "AI软件配置" }],
      },
      {
        id: "configuration-management",
        label: "配置管理",
        collapsible: true,
        items: [
          { id: "models", label: "模型管理" },
          { id: "skills", label: "Skills 管理" },
          { id: "mcp", label: "MCP 管理" },
          { id: "prompts", label: "提示词管理" },
        ],
      },
      {
        id: "memory",
        label: "记忆模块",
        collapsible: false,
        items: [{ id: "memory", label: "记忆模块" }],
      },
    ]);
    expect(navigationItems.map(({ id, path }) => ({ id, path }))).toEqual([
      { id: "agents", path: "/agents" },
      { id: "models", path: "/models" },
      { id: "skills", path: "/skills" },
      { id: "mcp", path: "/mcp" },
      { id: "prompts", path: "/prompts" },
      { id: "memory", path: "/memory" },
    ]);
  });

  it("renders exactly three approved top-level controls without duplicate copy", () => {
    renderNavigation();

    const navigation = screen.getByRole("navigation", { name: "主导航" });
    const topLevelControls = navigation.querySelectorAll<HTMLElement>(
      ".fy-side-navigation-group > .fy-side-navigation-item, .fy-side-navigation-group > .fy-side-navigation-toggle",
    );

    expect(
      Array.from(topLevelControls, (control) => control.textContent?.trim()),
    ).toEqual(["AI软件配置", "配置管理", "记忆模块"]);
    expect(
      within(navigation).getByRole("link", { name: "AI软件配置" }),
    ).toHaveAttribute("href", "/agents");
    expect(
      within(navigation).getByRole("link", { name: "记忆模块" }),
    ).toHaveAttribute("href", "/memory");
    expect(
      within(navigation).queryByRole("link", { name: "Agent 目录" }),
    ).not.toBeInTheDocument();
    expect(
      within(navigation).queryByRole("link", { name: /^记忆$/ }),
    ).not.toBeInTheDocument();
  });

  it("keeps one active lens when an active configuration group collapses", async () => {
    const user = userEvent.setup();
    renderNavigation("/models");

    const navigation = screen.getByRole("navigation", { name: "主导航" });
    const toggle = within(navigation).getByRole("button", {
      name: "配置管理",
    });
    const items = screen.getByTestId("configuration-management-items");
    const modelsLink = within(items).getByText("模型管理").closest("a");

    expect(modelsLink).toHaveAttribute("aria-current", "page");
    expect(within(navigation).getAllByTestId("selection-lens")).toHaveLength(1);
    expect(within(navigation).queryByTestId("liquid-glass-lens")).toBeNull();

    await user.click(toggle);

    expect(toggle).toHaveAttribute("aria-expanded", "false");
    expect(toggle).toHaveClass("fy-side-navigation-toggle-active");
    expect(items).not.toBeVisible();
    expect(modelsLink).toHaveAttribute("aria-current", "page");
    expect(within(navigation).getAllByTestId("selection-lens")).toHaveLength(1);
    expect(within(navigation).queryByTestId("liquid-glass-lens")).toBeNull();
  });
});
