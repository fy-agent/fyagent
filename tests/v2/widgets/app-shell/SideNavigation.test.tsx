import { render, screen, waitFor, within } from "@testing-library/react";
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
  it("derives the stable seven route leaves from three typed groups", () => {
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
        items: [
          { id: "agents", label: "AI软件配置" },
          { id: "auth", label: "账号与认证" },
        ],
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
      { id: "auth", path: "/auth" },
      { id: "models", path: "/models" },
      { id: "skills", path: "/skills" },
      { id: "mcp", path: "/mcp" },
      { id: "prompts", path: "/prompts" },
      { id: "memory", path: "/memory" },
    ]);
  });

  it("renders exactly four approved top-level controls without duplicate copy", () => {
    renderNavigation();

    const navigation = screen.getByRole("navigation", { name: "主导航" });
    const topLevelControls = navigation.querySelectorAll<HTMLElement>(
      ".fy-side-navigation-group > .fy-side-navigation-item, .fy-side-navigation-group > .fy-side-navigation-toggle",
    );

    expect(
      Array.from(topLevelControls, (control) => control.textContent?.trim()),
    ).toEqual(["AI软件配置", "账号与认证", "配置管理", "记忆模块"]);
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

  it("derives the Agent return URL from a closed management query", async () => {
    const user = userEvent.setup();
    render(
      <MemoryRouter
        initialEntries={["/models?agentReturn=workbuddy&agentSection=mcp"]}
      >
        <SideNavigation />
      </MemoryRouter>,
    );

    const navigation = screen.getByRole("navigation", { name: "主导航" });
    const agents = within(navigation).getByRole("link", {
      name: "AI软件配置",
    });
    const models = within(navigation).getByRole("link", {
      name: "模型管理",
    });

    expect(models).toHaveAttribute("aria-current", "page");
    expect(agents).toHaveAttribute(
      "href",
      "/agents?target=workbuddy&section=mcp",
    );

    await user.click(agents);
    expect(agents).toHaveAttribute("aria-current", "page");
  });

  it("retains a validated Agents query after navigating to another primary route", async () => {
    const user = userEvent.setup();
    render(
      <MemoryRouter initialEntries={["/agents?target=workbuddy&section=mcp"]}>
        <SideNavigation />
      </MemoryRouter>,
    );

    const navigation = screen.getByRole("navigation", { name: "主导航" });
    const agents = within(navigation).getByRole("link", {
      name: "AI软件配置",
    });

    expect(agents).toHaveAttribute("href", "/agents");
    const mcp = within(navigation).getByRole("link", { name: "MCP 管理" });
    expect(mcp).toHaveAttribute(
      "href",
      "/mcp?agentReturn=workbuddy&agentSection=mcp",
    );
    await user.click(mcp);
    expect(mcp).toHaveAttribute("aria-current", "page");
    await waitFor(() =>
      expect(agents).toHaveAttribute(
        "href",
        "/agents?target=workbuddy&section=mcp",
      ),
    );
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
    expect(toggle).toHaveAttribute("data-selection-material", "context-frame");
    expect(within(navigation).getAllByTestId("selection-lens")).toHaveLength(1);
    expect(within(navigation).queryByTestId("liquid-glass-lens")).toBeNull();

    await user.click(toggle);

    expect(toggle).toHaveAttribute("aria-expanded", "false");
    expect(toggle).toHaveClass("fy-side-navigation-toggle-active");
    expect(toggle).toHaveAttribute("data-collapsed-active", "true");
    expect(toggle).toHaveAttribute("data-selection-material", "text-only");
    expect(items).not.toBeVisible();
    expect(modelsLink).toHaveAttribute("aria-current", "page");
    expect(within(navigation).getAllByTestId("selection-lens")).toHaveLength(1);
    expect(within(navigation).queryByTestId("liquid-glass-lens")).toBeNull();
  });

  it("keeps selected navigation hosts semantic while the shared lens owns the only frame", () => {
    renderNavigation("/agents");

    const navigation = screen.getByRole("navigation", { name: "主导航" });
    const agents = within(navigation).getByRole("link", {
      name: "AI软件配置",
    });
    const lens = within(navigation).getByTestId("selection-lens");

    expect(agents).toHaveAttribute("aria-current", "page");
    expect(agents).toHaveAttribute("data-selection-material", "text-only");
    expect(lens).toHaveAttribute("data-selection-material", "frame");
    expect(lens).toHaveAttribute("data-selection-lens-geometry", "position");
    expect(within(navigation).getAllByTestId("selection-lens")).toHaveLength(1);
  });

  it("keeps one memory lens after collapsing then expanding configuration", async () => {
    const user = userEvent.setup();
    renderNavigation("/memory");

    const navigation = screen.getByRole("navigation", { name: "主导航" });
    const toggle = within(navigation).getByRole("button", {
      name: "配置管理",
    });
    const memory = within(navigation).getByRole("link", { name: "记忆模块" });

    expect(memory).toHaveAttribute("aria-current", "page");
    expect(within(navigation).getAllByTestId("selection-lens")).toHaveLength(1);

    await user.click(toggle);
    expect(toggle).toHaveAttribute("aria-expanded", "false");
    expect(memory).toHaveAttribute("aria-current", "page");
    expect(within(navigation).getAllByTestId("selection-lens")).toHaveLength(1);

    await user.click(toggle);
    expect(toggle).toHaveAttribute("aria-expanded", "true");
    expect(memory).toHaveAttribute("aria-current", "page");
    expect(within(navigation).getAllByTestId("selection-lens")).toHaveLength(1);
    expect(memory.querySelector("[data-selection-lens-target]")).not.toBeNull();
  });

  it("supports expand, collapse, and vertical keyboard focus", async () => {
    const user = userEvent.setup();
    renderNavigation("/agents");

    const navigation = screen.getByRole("navigation", { name: "主导航" });
    const toggle = within(navigation).getByRole("button", { name: "配置管理" });
    const items = screen.getByTestId("configuration-management-items");

    expect(toggle).toHaveAttribute("aria-expanded", "true");
    expect(items).toBeVisible();

    await user.click(toggle);
    expect(toggle).toHaveAttribute("aria-expanded", "false");
    expect(items).not.toBeVisible();
    expect(items).toHaveAttribute("hidden");
    expect(
      within(navigation).queryByRole("link", { name: "模型管理" }),
    ).not.toBeInTheDocument();

    toggle.focus();
    await user.keyboard("{ArrowRight}");
    expect(toggle).toHaveAttribute("aria-expanded", "true");
    await user.keyboard("{ArrowRight}");
    expect(
      within(navigation).getByRole("link", { name: "模型管理" }),
    ).toHaveFocus();
    await user.keyboard("{ArrowDown}");
    expect(
      within(navigation).getByRole("link", { name: "Skills 管理" }),
    ).toHaveFocus();
    await user.keyboard("{Escape}");
    expect(toggle).toHaveFocus();
    expect(toggle).toHaveAttribute("aria-expanded", "false");
  });

  it("omits closing and closed leaves from arrow and tab order", async () => {
    const user = userEvent.setup();
    renderNavigation("/agents");

    const navigation = screen.getByRole("navigation", { name: "主导航" });
    const toggle = within(navigation).getByRole("button", { name: "配置管理" });
    const memory = within(navigation).getByRole("link", { name: "记忆模块" });
    const agents = within(navigation).getByRole("link", { name: "AI软件配置" });

    await user.click(toggle);
    expect(toggle).toHaveAttribute("aria-expanded", "false");

    toggle.focus();
    await user.keyboard("{ArrowDown}");
    expect(memory).toHaveFocus();
    await user.keyboard("{ArrowUp}");
    expect(toggle).toHaveFocus();
    await user.keyboard("{Home}");
    expect(agents).toHaveFocus();
    await user.keyboard("{End}");
    expect(memory).toHaveFocus();

    toggle.focus();
    await user.tab();
    expect(memory).toHaveFocus();
  });

  it("collapses instantly when the user prefers reduced motion", async () => {
    const originalMatchMedia = window.matchMedia;
    window.matchMedia = ((query: string) => ({
      matches: query.includes("prefers-reduced-motion"),
      media: query,
      onchange: null,
      addEventListener() {},
      removeEventListener() {},
      addListener() {},
      removeListener() {},
      dispatchEvent() {
        return false;
      },
    })) as typeof window.matchMedia;

    try {
      const user = userEvent.setup();
      renderNavigation("/agents");

      const toggle = screen.getByRole("button", { name: "配置管理" });
      const items = screen.getByTestId("configuration-management-items");

      await user.click(toggle);

      expect(toggle).toHaveAttribute("aria-expanded", "false");
      expect(items).not.toBeVisible();
      expect(items).toHaveAttribute("hidden");
      expect(
        screen.queryByRole("link", { name: "模型管理" }),
      ).not.toBeInTheDocument();
    } finally {
      window.matchMedia = originalMatchMedia;
    }
  });
});
