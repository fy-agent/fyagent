import { render, screen } from "@testing-library/react";
import { createMemoryRouter, RouterProvider } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";

const overlayState = vi.hoisted(() => ({ show: false }));

vi.mock("@/v2/shared/platform", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/v2/shared/platform")>();
  return {
    ...actual,
    shouldShowMacOverlayDragStrip: () => overlayState.show,
  };
});

vi.mock("@samasante/liquid-glass", () => ({
  Glass: ({ children }: { children?: React.ReactNode }) => (
    <span>{children}</span>
  ),
}));

import { TooltipProvider } from "@/v2/shared/ui/primitives";
import { TopBar } from "@/v2/widgets/app-shell/TopBar";

function renderTopBar() {
  const router = createMemoryRouter(
    [
      {
        path: "/",
        element: (
          <TooltipProvider delayDuration={250} skipDelayDuration={100}>
            <TopBar />
          </TooltipProvider>
        ),
      },
    ],
    { initialEntries: ["/"] },
  );
  return render(<RouterProvider router={router} />);
}

describe("TopBar macOS Overlay drag strip", () => {
  beforeEach(() => {
    overlayState.show = false;
  });

  it("keeps the browser shell free of a drag region", () => {
    overlayState.show = false;
    renderTopBar();

    expect(screen.queryByTestId("titlebar-drag-region")).not.toBeInTheDocument();
    expect(document.querySelector("[data-tauri-drag-region]")).toBeNull();
    expect(screen.getByTestId("brand")).toBeVisible();
    expect(screen.getByTestId("primary-navigation")).toBeVisible();
    expect(screen.getByTestId("tool-cluster")).toBeVisible();
  });

  it("places an inert drag strip above the chrome row on native macOS", () => {
    overlayState.show = true;
    renderTopBar();

    const topBar = screen.getByTestId("top-bar");
    const dragStrip = screen.getByTestId("titlebar-drag-region");
    const chrome = topBar.querySelector(".fy-top-bar-chrome");
    const dragSurface = document.querySelector("[data-tauri-drag-region]");

    expect(dragStrip).toBeVisible();
    expect(dragSurface).not.toBeNull();
    expect(chrome).not.toBeNull();
    expect(dragStrip.compareDocumentPosition(chrome!)).toBe(
      Node.DOCUMENT_POSITION_FOLLOWING,
    );
    expect(screen.queryByRole("button", { name: "关闭" })).not.toBeInTheDocument();
  });
});
