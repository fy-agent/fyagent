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
import { describe, expect, it } from "vitest";

import { appRoutes } from "@/v2/app/router";

const navigationContract = [
  { path: "/agents", label: "Agent 目录" },
  { path: "/models", label: "模型" },
  { path: "/skills", label: "Skills" },
  { path: "/mcp", label: "MCP" },
  { path: "/prompts", label: "提示词" },
  { path: "/memory", label: "记忆" },
] as const;

const toolNames = ["Search", "Settings", "Avatar"] as const;
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

describe("FyAgent V2 routing", () => {
  it.each(["/", "/route-that-does-not-exist"])(
    "redirects %s to the models route",
    async (initialEntry) => {
      const router = renderRoute(initialEntry);

      await expectPath(router, "/models");
      expect(
        screen.getByRole("link", { name: "模型", current: "page" }),
      ).toHaveAttribute("href", "/models");
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
    },
  );

  it("renders the local-Agent Prompt workspace at the prompts route", async () => {
    const router = renderRoute("/prompts");

    await expectPath(router, "/prompts");
    expect(screen.getByTestId("prompts-page")).toBeVisible();
    expect(screen.getByTestId("prompt-library")).toBeVisible();
    expect(screen.getByTestId("prompt-editor")).toBeVisible();
    expect(screen.getByTestId("prompt-inspector")).toBeVisible();
  });

  it("renders the local-Agent Memory workspace at the memory route", async () => {
    const router = renderRoute("/memory");

    await expectPath(router, "/memory");
    expect(screen.getByTestId("memory-page")).toBeVisible();
    expect(screen.getByTestId("memory-library")).toBeVisible();
    expect(screen.getByTestId("memory-editor")).toBeVisible();
    expect(screen.getByTestId("memory-inspector")).toBeVisible();
  });

  it("keeps the frameless shell available when a child route fails", async () => {
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

    expect(await screen.findByRole("alert")).toHaveTextContent("Route failed");
    expect(screen.getByTestId("top-bar")).toBeVisible();
    expect(screen.getByTestId("titlebar-drag-region")).toBeInTheDocument();
    for (const name of windowControlNames) {
      expect(screen.getByRole("button", { name })).toBeVisible();
    }
  });
});

describe("FyAgent V2 shell accessibility", () => {
  it("exposes the frozen labels and landmarks in the primary tab order", async () => {
    const user = userEvent.setup();
    renderRoute("/models");

    const brand = screen.getByTestId("brand");
    const navigation = screen.getByRole("navigation", { name: "主导航" });
    const routeLinks = within(navigation).getAllByRole("link");
    const toolButtons = toolNames.map((name) =>
      screen.getByRole("button", { name }),
    );
    const windowButtons = windowControlNames.map((name) =>
      screen.getByRole("button", { name }),
    );

    expect(brand).toHaveAccessibleName("FyAgent 品牌");
    expect(screen.getByRole("main", { name: "内容承载区" })).toBeVisible();
    expect(routeLinks.map((link) => link.textContent?.trim())).toEqual(
      navigationContract.map(({ label }) => label),
    );

    const expectedTabOrder = [...routeLinks, ...toolButtons, ...windowButtons];
    for (const control of expectedTabOrder) {
      await user.tab();
      expect(control).toHaveFocus();
    }
  });

  it("keeps inert tools and browser window controls safely clickable", () => {
    renderRoute("/models");

    const buttons = [...toolNames, ...windowControlNames].map((name) =>
      screen.getByRole("button", { name }),
    );

    for (const button of buttons) {
      expect(button).toBeEnabled();
      expect(button).toHaveAccessibleName();
      expect(() => fireEvent.click(button)).not.toThrow();
    }
  });
});
