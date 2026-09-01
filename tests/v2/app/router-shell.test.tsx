import type { CSSProperties, ReactNode } from "react";
import {
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
  { path: "/agents", label: "AI软件配置" },
  { path: "/models", label: "模型管理" },
  { path: "/skills", label: "Skills 管理" },
  { path: "/mcp", label: "MCP 管理" },
  { path: "/prompts", label: "提示词管理" },
  { path: "/memory", label: "记忆模块" },
] as const;

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
  ).toEqual(["brand"]);
  expect(screen.queryByTestId("tool-cluster")).not.toBeInTheDocument();
  for (const name of ["搜索", "设置", "账户"]) {
    expect(screen.queryByRole("button", { name })).not.toBeInTheDocument();
  }
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
        screen.getByRole("link", { name: "AI软件配置", current: "page" }),
      ).toHaveAttribute("href", "/agents");
      expect(screen.queryByTestId("liquid-glass-lens")).not.toBeInTheDocument();
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
      expect(within(activeLink).queryByTestId("liquid-glass-lens")).toBeNull();
      expect(
        within(navigation).queryByTestId("liquid-glass-lens"),
      ).not.toBeInTheDocument();
      expect(within(navigation).getByTestId("selection-lens")).toBeVisible();
      expect(within(navigation).getAllByTestId("selection-lens")).toHaveLength(
        1,
      );
      const content = screen.getByRole("main", { name: "内容" });
      expect(content).not.toBeEmptyDOMElement();
    },
  );

  it("renders all six product workspaces", async () => {
    const pageTestIds = new Map([
      ["/agents", "agents-page"],
      ["/models", "models-page"],
      ["/skills", "skills-page"],
      ["/mcp", "mcp-page"],
      ["/prompts", "prompts-page"],
      ["/memory", "memory-page"],
    ]);

    for (const path of navigationContract.map(({ path }) => path)) {
      const view = render(
        <RouterProvider
          router={createMemoryRouter(appRoutes, { initialEntries: [path] })}
        />,
      );
      const pageTestId = pageTestIds.get(path);
      expect(pageTestId).toBeDefined();
      expect(await screen.findByTestId(pageTestId!)).toBeVisible();
      expect(
        screen.getByRole("main", { name: "内容" }),
      ).not.toBeEmptyDOMElement();
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
    expect(
      navigation.querySelectorAll(
        ".fy-side-navigation-group > .fy-side-navigation-item, .fy-side-navigation-group > .fy-side-navigation-toggle",
      ),
    ).toHaveLength(3);
    expect(
      within(navigation).queryByRole("link", { name: "Agent 目录" }),
    ).not.toBeInTheDocument();
    expect(
      within(navigation).queryByRole("link", { name: /^记忆$/ }),
    ).not.toBeInTheDocument();
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
      within(navigation).getByRole("link", { name: "模型管理" }),
    ).toHaveFocus();
    await user.keyboard("{ArrowDown}");
    expect(
      within(navigation).getByRole("link", { name: "Skills 管理" }),
    ).toHaveFocus();
    await user.keyboard("{Escape}");
    expect(configurationToggle).toHaveFocus();
    expect(configurationToggle).toHaveAttribute("aria-expanded", "false");
  });

  it("does not expose unimplemented shell tools as clickable controls", () => {
    renderRoute("/models");

    for (const name of ["搜索", "设置", "账户"]) {
      expect(screen.queryByRole("button", { name })).not.toBeInTheDocument();
    }

    expectSystemOwnedChrome();
  });
});

describe("FyAgent V2 primary route lifecycle", () => {
  it("unmounts Models and discards unsaved route-local form state", async () => {
    const user = userEvent.setup();
    const router = renderRoute("/models?target=workbuddy");

    const url = await screen.findByLabelText("服务地址");
    await user.type(url, "https://keep.example/v1");
    await user.click(screen.getByRole("link", { name: "AI软件配置" }));
    await expectPath(router, "/agents");
    expect(screen.queryByTestId("models-page")).not.toBeInTheDocument();

    await user.click(screen.getByRole("link", { name: "模型管理" }));
    await expectPath(router, "/models");
    expect(await screen.findByTestId("models-page")).toBeVisible();
    expect(screen.queryByLabelText("服务地址")).not.toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "QoderWork CN" })).toBeVisible();
  });

  it("derives Agent configuration from the current URL after remount", async () => {
    const user = userEvent.setup();
    const ports = createBrowserFeaturePorts();
    ports.catalog.get = vi.fn(async () => ({
      contractVersion: 5 as const,
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
              id: "desktop" as const,
              label: "Claude Desktop",
              url: "https://claude.com/download",
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
      await user.click(screen.getByRole("link", { name: "模型管理" }));
      await expectPath(router, "/models");
      expect(screen.queryByTestId("agents-page")).not.toBeInTheDocument();

      await user.click(screen.getByRole("link", { name: "AI软件配置" }));
      await expectPath(router, "/agents");
      expect(
        await screen.findByRole("heading", { name: "Claude Code" }),
      ).toBeVisible();
      expect(router.state.location.search).toBe(
        "?target=claude-code&section=models",
      );
    } finally {
      createPorts.mockRestore();
    }
  });

  it("unmounts inactive product pages and restores their default local view", async () => {
    const user = userEvent.setup();
    const router = renderRoute("/skills");

    await user.click(await screen.findByRole("tab", { name: "发现" }));
    expect(screen.getByRole("tab", { name: "发现" })).toHaveAttribute(
      "aria-selected",
      "true",
    );
    await user.click(screen.getByRole("link", { name: "提示词管理" }));
    await expectPath(router, "/prompts");
    expect(screen.queryByTestId("skills-page")).not.toBeInTheDocument();

    await user.click(await screen.findByTestId("prompt-app-gemini"));
    expect(screen.getByTestId("prompt-app-gemini")).toHaveAttribute(
      "aria-current",
      "true",
    );
    await user.click(screen.getByRole("link", { name: "记忆模块" }));
    await expectPath(router, "/memory");
    expect(screen.queryByTestId("prompts-page")).not.toBeInTheDocument();

    await user.click(await screen.findByRole("tab", { name: "每日记忆" }));
    expect(screen.getByRole("tab", { name: "每日记忆" })).toHaveAttribute(
      "aria-selected",
      "true",
    );
    await user.click(screen.getByRole("link", { name: "MCP 管理" }));
    await expectPath(router, "/mcp");
    expect(screen.queryByTestId("memory-page")).not.toBeInTheDocument();
    expect(await screen.findByTestId("mcp-page")).toBeVisible();

    await user.click(screen.getByRole("link", { name: "Skills 管理" }));
    await expectPath(router, "/skills");
    expect(screen.queryByTestId("mcp-page")).not.toBeInTheDocument();
    expect(await screen.findByRole("tab", { name: "已安装" })).toHaveAttribute(
      "aria-selected",
      "true",
    );
    await user.click(screen.getByRole("link", { name: "提示词管理" }));
    expect(await screen.findByTestId("prompt-app-claude")).toHaveAttribute(
      "aria-current",
      "true",
    );
    await user.click(screen.getByRole("link", { name: "记忆模块" }));
    expect(await screen.findByRole("tab", { name: "长期记忆" })).toHaveAttribute(
      "aria-selected",
      "true",
    );
    await user.click(screen.getByRole("link", { name: "MCP 管理" }));
    expect(screen.getByTestId("mcp-page")).toBeVisible();
  });
});
