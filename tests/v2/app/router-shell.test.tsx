import type { CSSProperties, ReactNode } from "react";
import {
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { createMemoryRouter, RouterProvider } from "react-router-dom";
import type { RouteObject } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";

import { appRoutes } from "@/v2/app/router";
import { createBrowserFeaturePorts } from "@/v2/shared/platform/browser/features";
import * as featureFactory from "@/v2/shared/platform/features";

vi.mock("@samasante/liquid-glass", () => ({
  Glass: ({
    children,
    className,
    style,
    "data-testid": testId,
  }: {
    children?: ReactNode;
    className?: string;
    style?: CSSProperties;
    "data-testid"?: string;
  }) => (
    <span className={className} style={style} data-testid={testId}>
      {children}
    </span>
  ),
}));

const navigationContract = [
  { path: "/agents", label: "Agent 目录" },
  { path: "/models", label: "模型" },
  { path: "/skills", label: "Skills" },
  { path: "/mcp", label: "MCP" },
  { path: "/prompts", label: "提示词" },
  { path: "/memory", label: "记忆" },
] as const;

const toolNames = ["搜索", "设置", "账户"] as const;
const windowControlNames = ["最小化", "最大化/还原", "关闭"] as const;

type TestRouter = ReturnType<typeof createMemoryRouter>;

function renderRoute(initialEntry: string): TestRouter {
  const router = createMemoryRouter(appRoutes, {
    initialEntries: [initialEntry],
  });

  render(<RouterProvider router={router} />);
  return router;
}

async function expectPath(router: TestRouter, pathname: string): Promise<void> {
  await waitFor(() => {
    expect(router.state.location.pathname).toBe(pathname);
  });
}

function expectSystemOwnedChrome(): void {
  const topBar = screen.getByTestId("top-bar");

  expect(
    Array.from(
      topBar.querySelectorAll(
        '[data-testid="brand"], [data-testid="tool-cluster"]',
      ),
    ).map((element) => element.getAttribute("data-testid")),
  ).toEqual(["brand", "tool-cluster"]);
  expect(topBar.querySelector('[data-testid="side-navigation"]')).toBeNull();
  expect(screen.getByTestId("side-navigation")).toBeVisible();
  expect(document.querySelector("[data-tauri-drag-region]")).toBeNull();
  expect(screen.queryByTestId("titlebar-drag-region")).not.toBeInTheDocument();
  expect(screen.queryByTestId("window-controls")).not.toBeInTheDocument();
  for (const name of windowControlNames) {
    expect(screen.queryByRole("button", { name })).not.toBeInTheDocument();
  }
}

describe("FyAgent V2 routing", () => {
  it.each(["/", "/route-that-does-not-exist"])(
    "redirects %s to the agents route",
    async (initialEntry) => {
      const router = renderRoute(initialEntry);

      await expectPath(router, "/agents");
      const navigation = screen.getByRole("navigation", { name: "主导航" });
      expect(
        screen.getByRole("link", { name: "Agent 目录", current: "page" }),
      ).toHaveAttribute("href", "/agents");
      expect(screen.getAllByTestId("liquid-glass-lens")).toHaveLength(1);
      expect(within(navigation).getAllByTestId("selection-lens")).toHaveLength(
        1,
      );
    },
  );

  it.each(navigationContract)(
    "makes $path reachable and derives selection from router location",
    async ({ path, label }) => {
      const router = renderRoute(path);

      await expectPath(router, path);
      const navigation = screen.getByRole("navigation", { name: "主导航" });
      const activeLink = within(navigation).getByRole("link", {
        name: label,
      });
      const selectedLinks = within(navigation)
        .getAllByRole("link")
        .filter((link) => link.getAttribute("aria-current") === "page");

      expect(activeLink).toHaveAttribute("aria-current", "page");
      expect(selectedLinks).toEqual([activeLink]);
      expect(within(activeLink).getByTestId("liquid-glass-lens")).toBeVisible();
      expect(
        within(navigation).getAllByTestId("liquid-glass-lens"),
      ).toHaveLength(1);
      expect(within(navigation).getByTestId("selection-lens")).toBeVisible();
      expect(within(navigation).getAllByTestId("selection-lens")).toHaveLength(
        1,
      );
      const content = screen.getByRole("main", { name: "内容" });
      expect(content).not.toBeEmptyDOMElement();
    },
  );

  it("renders all six product workspaces", () => {
    for (const path of navigationContract.map(({ path }) => path)) {
      const view = render(
        <RouterProvider
          router={createMemoryRouter(appRoutes, { initialEntries: [path] })}
        />,
      );
      expect(
        screen.getByRole("main", { name: "内容" }),
      ).not.toBeEmptyDOMElement();
      if (path === "/prompts") {
        expect(screen.getByTestId("prompts-page")).toBeVisible();
      }
      if (path === "/memory") {
        expect(screen.getByTestId("memory-page")).toBeVisible();
      }
      view.unmount();
    }
  });

  it("keeps the system-chrome shell available when a child route fails", async () => {
    const [rootRoute] = appRoutes;
    const [contentBoundary] = rootRoute.children ?? [];
    const failingRoute: RouteObject = {
      path: "failure",
      loader: () => {
        throw new Error("Route failed");
      },
      element: <div />,
      hydrateFallbackElement: <div />,
    };
    expect(rootRoute?.errorElement).toBeUndefined();
    expect(contentBoundary?.errorElement).toBeTruthy();
    const routes: RouteObject[] = [
      {
        path: "/",
        element: rootRoute?.element,
        children: [
          {
            errorElement: contentBoundary?.errorElement,
            children: [failingRoute],
          },
        ],
      },
    ];
    const router = createMemoryRouter(routes, {
      initialEntries: ["/failure"],
    });

    render(<RouterProvider router={router} />);

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "页面暂时无法打开",
    );
    expect(screen.getByTestId("top-bar")).toBeVisible();
    expectSystemOwnedChrome();
  });
});

describe("FyAgent V2 shell accessibility", () => {
  it("exposes the grouped labels and landmarks in document tab order", async () => {
    const user = userEvent.setup();
    renderRoute("/models");

    const brand = screen.getByTestId("brand");
    const navigation = screen.getByRole("navigation", { name: "主导航" });
    const routeLinks = within(navigation).getAllByRole("link");
    const toolButtons = toolNames.map((name) =>
      screen.getByRole("button", { name }),
    );
    const configurationToggle = within(navigation).getByRole("button", {
      name: "配置管理",
    });

    expect(brand).toHaveAccessibleName("FyAgent");
    expect(screen.getByRole("main", { name: "内容" })).toBeVisible();
    expect(routeLinks.map((link) => link.textContent?.trim())).toEqual(
      navigationContract.map(({ label }) => label),
    );
    expectSystemOwnedChrome();

    const expectedTabOrder = [
      ...toolButtons,
      routeLinks[0],
      configurationToggle,
      ...routeLinks.slice(1),
    ];
    for (const control of expectedTabOrder) {
      await user.tab();
      expect(control).toHaveFocus();
    }
  });

  it("supports configuration expansion and vertical keyboard focus", async () => {
    const user = userEvent.setup();
    renderRoute("/agents");

    const navigation = screen.getByRole("navigation", { name: "主导航" });
    const configurationToggle = within(navigation).getByRole("button", {
      name: "配置管理",
    });
    const configurationItems = screen.getByTestId(
      "configuration-management-items",
    );

    expect(
      within(navigation).getByText("AI软件配置", { exact: true }),
    ).toBeVisible();
    expect(
      within(navigation).getByText("记忆模块", { exact: true }),
    ).toBeVisible();
    expect(configurationToggle).toHaveAttribute("aria-expanded", "true");
    expect(configurationItems).toBeVisible();

    await user.click(configurationToggle);
    expect(configurationToggle).toHaveAttribute("aria-expanded", "false");
    expect(configurationItems).not.toBeVisible();

    configurationToggle.focus();
    await user.keyboard("{ArrowRight}");
    expect(configurationToggle).toHaveAttribute("aria-expanded", "true");
    await user.keyboard("{ArrowRight}");
    expect(
      within(navigation).getByRole("link", { name: "模型" }),
    ).toHaveFocus();
    await user.keyboard("{ArrowDown}");
    expect(
      within(navigation).getByRole("link", { name: "Skills" }),
    ).toHaveFocus();
    await user.keyboard("{Escape}");
    expect(configurationToggle).toHaveFocus();
    expect(configurationToggle).toHaveAttribute("aria-expanded", "false");
  });

  it("keeps inert shell tools safely clickable", () => {
    renderRoute("/models");

    const buttons = toolNames.map((name) =>
      screen.getByRole("button", { name }),
    );

    for (const button of buttons) {
      expect(button).toBeEnabled();
      expect(button).toHaveAccessibleName();
      expect(() => fireEvent.click(button)).not.toThrow();
    }

    expectSystemOwnedChrome();
  });
});

describe("FyAgent V2 primary page persistence", () => {
  it("keeps Models page form content after visiting another primary route", async () => {
    const user = userEvent.setup();
    const router = renderRoute("/models?target=workbuddy");

    const url = await screen.findByLabelText("服务地址");
    await user.type(url, "https://keep.example/v1");
    await user.click(screen.getByRole("link", { name: "Agent 目录" }));
    await expectPath(router, "/agents");
    expect(screen.getByTestId("models-page")).not.toBeVisible();

    await user.click(screen.getByRole("link", { name: "模型" }));
    await expectPath(router, "/models");
    expect(await screen.findByLabelText("服务地址")).toHaveValue(
      "https://keep.example/v1",
    );
    expect(screen.getByRole("heading", { name: "WorkBuddy" })).toBeVisible();
  });

  it("keeps Agent catalog selection after visiting another primary route", async () => {
    const user = userEvent.setup();
    const ports = createBrowserFeaturePorts();
    ports.catalog.get = vi.fn(async () => ({
      contractVersion: 4 as const,
      reviewedAt: "2026-08-18",
      agents: [
        {
          id: "qoderwork" as const,
          variantId: "qoderwork-cn" as const,
          displayName: "QoderWork CN",
          description: "QoderWork",
          officialLinks: [
            {
              id: "product" as const,
              label: "打开 QoderWork CN 官方页面",
              url: "https://qoder.com.cn/qoderwork",
            },
          ],
          capabilities: [],
        },
        {
          id: "claude-code" as const,
          variantId: "claude-code" as const,
          displayName: "Claude Code",
          description: "Claude Code",
          officialLinks: [
            {
              id: "cli" as const,
              label: "Claude Code CLI",
              url: "https://docs.anthropic.com/en/docs/claude-code/getting-started",
            },
          ],
          capabilities: [],
        },
      ],
    }));
    const createPorts = vi
      .spyOn(featureFactory, "createFeaturePorts")
      .mockReturnValue(ports);
    try {
      const router = renderRoute("/agents?target=claude-code");

      expect(
        await screen.findByRole("heading", { name: "Claude Code" }),
      ).toBeVisible();
      await user.click(screen.getByRole("link", { name: "模型" }));
      await expectPath(router, "/models");
      expect(screen.getByTestId("agents-page")).not.toBeVisible();

      await user.click(screen.getByRole("link", { name: "Agent 目录" }));
      await expectPath(router, "/agents");
      expect(
        await screen.findByRole("heading", { name: "Claude Code" }),
      ).toBeVisible();
      expect(router.state.location.search).toBe("?target=claude-code");
    } finally {
      createPorts.mockRestore();
    }
  });

  it("keeps Skills, Prompts, Memory, and MCP after leaving", async () => {
    const user = userEvent.setup();
    const router = renderRoute("/skills");

    await user.click(screen.getByRole("tab", { name: "发现" }));
    expect(screen.getByRole("tab", { name: "发现" })).toHaveAttribute(
      "aria-selected",
      "true",
    );
    await user.click(screen.getByRole("link", { name: "提示词" }));
    await expectPath(router, "/prompts");
    expect(screen.getByTestId("skills-page")).not.toBeVisible();

    await user.click(screen.getByTestId("prompt-app-gemini"));
    expect(screen.getByTestId("prompt-app-gemini")).toHaveAttribute(
      "aria-current",
      "true",
    );
    await user.click(screen.getByRole("link", { name: "记忆" }));
    await expectPath(router, "/memory");
    expect(screen.getByTestId("prompts-page")).not.toBeVisible();

    await user.click(screen.getByRole("tab", { name: "每日记忆" }));
    expect(screen.getByRole("tab", { name: "每日记忆" })).toHaveAttribute(
      "aria-selected",
      "true",
    );
    await user.click(screen.getByRole("link", { name: "MCP" }));
    await expectPath(router, "/mcp");
    expect(screen.getByTestId("memory-page")).not.toBeVisible();
    expect(screen.getByTestId("mcp-page")).toBeVisible();

    await user.click(screen.getByRole("link", { name: "Skills" }));
    await expectPath(router, "/skills");
    expect(screen.getByTestId("mcp-page")).not.toBeVisible();
    expect(screen.getByRole("tab", { name: "发现" })).toHaveAttribute(
      "aria-selected",
      "true",
    );
    await user.click(screen.getByRole("link", { name: "提示词" }));
    expect(screen.getByTestId("prompt-app-gemini")).toHaveAttribute(
      "aria-current",
      "true",
    );
    await user.click(screen.getByRole("link", { name: "记忆" }));
    expect(screen.getByRole("tab", { name: "每日记忆" })).toHaveAttribute(
      "aria-selected",
      "true",
    );
    await user.click(screen.getByRole("link", { name: "MCP" }));
    expect(screen.getByTestId("mcp-page")).toBeVisible();
  });
});
