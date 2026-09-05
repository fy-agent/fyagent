import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { describe, expect, it, vi } from "vitest";

import { fySpringTransition } from "@/v2/shared/ui/motion";
import {
  SelectionLens,
  SelectionLensGroup,
  SelectionLensTrack,
  selectionLensCollapsedOrigin,
  selectionLensTransition,
} from "@/v2/shared/ui/SelectionLens";

describe("SelectionLens", () => {
  it("keeps the source L1 control spring", () => {
    expect(selectionLensTransition).toBe(fySpringTransition);
    expect(selectionLensTransition).toEqual({
      type: "spring",
      stiffness: 520,
      damping: 42,
      mass: 0.62,
    });
  });

  it("collapses appear origin to the active host top-left, not the track origin", () => {
    expect(selectionLensCollapsedOrigin({ x: 24, y: 88 })).toEqual({
      x: 24,
      y: 88,
      width: 0,
      height: 0,
    });
    expect(selectionLensCollapsedOrigin({ x: 24, y: 88 })).not.toEqual({
      x: 0,
      y: 0,
      width: 0,
      height: 0,
    });
  });

  it("renders one track pill for the active option", () => {
    render(
      <SelectionLensTrack id="demo-track" role="list">
        <button type="button">
          <SelectionLens active={false} />
          Idle
        </button>
        <button type="button" aria-current="true">
          <SelectionLens active />
          Current
        </button>
      </SelectionLensTrack>,
    );

    expect(screen.getAllByTestId("selection-lens")).toHaveLength(1);
    expect(screen.getByTestId("selection-lens")).toHaveAttribute(
      "aria-hidden",
      "true",
    );
    expect(screen.getByTestId("selection-lens")).toHaveAttribute(
      "data-selection-lens-geometry",
      "size-and-position",
    );
  });

  it("exposes the position-only geometry used by size-stable navigation tracks", () => {
    render(
      <SelectionLensTrack id="position-track" geometry="position">
        <button type="button">
          <SelectionLens active />
          Current
        </button>
      </SelectionLensTrack>,
    );

    expect(screen.getByTestId("selection-lens")).toHaveAttribute(
      "data-selection-lens-geometry",
      "position",
    );
  });

  it("does not render outside a group", () => {
    const { container } = render(<SelectionLens active />);

    expect(
      container.querySelector("[data-testid='selection-lens']"),
    ).toBeNull();
  });

  it("keeps one shared pill inside the group", () => {
    render(
      <SelectionLensGroup id="shared-track">
        <SelectionLens active />
      </SelectionLensGroup>,
    );

    expect(screen.getByTestId("selection-lens")).toBeVisible();
  });

  it("replays the appear spring after a hidden ancestor is shown again", async () => {
    function Track({ hide }: { hide: boolean }) {
      return (
        <div hidden={hide ? true : undefined}>
          <SelectionLensGroup id="hidden-track">
            <button type="button">
              <SelectionLens active />
              Current
            </button>
          </SelectionLensGroup>
        </div>
      );
    }

    const { rerender } = render(<Track hide={false} />);
    expect(screen.getByTestId("selection-lens")).toHaveAttribute(
      "data-selection-lens-reveal",
      "0",
    );

    rerender(<Track hide />);
    rerender(<Track hide={false} />);
    await waitFor(() => {
      expect(screen.getByTestId("selection-lens")).toHaveAttribute(
        "data-selection-lens-reveal",
        "1",
      );
    });
  });

  it("observes only the active host and track instead of the layout subtree", () => {
    const observed = new Set<Element>();

    class RecordingResizeObserver {
      observe(element: Element) {
        observed.add(element);
      }
      unobserve(element: Element) {
        observed.delete(element);
      }
      disconnect() {}
    }

    vi.stubGlobal("ResizeObserver", RecordingResizeObserver);

    try {
      render(
        <SelectionLensGroup id="reflow-track" data-testid="reflow-scope">
          <div data-testid="reflow-spacer" />
          <button type="button" data-testid="reflow-host">
            <SelectionLens active />
            Current
          </button>
        </SelectionLensGroup>,
      );

      expect(observed.has(screen.getByTestId("reflow-scope"))).toBe(true);
      expect(observed.has(screen.getByTestId("reflow-spacer"))).toBe(false);
      expect(observed.has(screen.getByTestId("reflow-host"))).toBe(true);
      expect(observed.has(screen.getByTestId("selection-lens"))).toBe(false);
    } finally {
      vi.unstubAllGlobals();
    }
  });

  it("retargets the pill when a sibling resizes and the host translates", async () => {
    const callbacks = new Set<ResizeObserverCallback>();

    class FlushableResizeObserver {
      constructor(callback: ResizeObserverCallback) {
        callbacks.add(callback);
      }
      observe() {}
      unobserve() {}
      disconnect() {
        callbacks.clear();
      }
    }

    vi.stubGlobal("ResizeObserver", FlushableResizeObserver);

    const mockBox = (
      element: Element,
      box: { x: number; y: number; width: number; height: number },
    ) => {
      vi.spyOn(element, "getBoundingClientRect").mockReturnValue({
        x: box.x,
        y: box.y,
        top: box.y,
        left: box.x,
        bottom: box.y + box.height,
        right: box.x + box.width,
        width: box.width,
        height: box.height,
        toJSON() {
          return {};
        },
      } as DOMRect);
    };

    try {
      render(
        <SelectionLensGroup id="translate-track" data-testid="translate-scope">
          <div data-testid="translate-spacer" />
          <button type="button" data-testid="translate-host">
            <SelectionLens active />
            Current
          </button>
        </SelectionLensGroup>,
      );

      const scope = screen.getByTestId("translate-scope");
      const host = screen.getByTestId("translate-host");
      const lens = screen.getByTestId("selection-lens");

      mockBox(scope, { x: 0, y: 0, width: 200, height: 240 });
      mockBox(host, { x: 8, y: 40, width: 184, height: 36 });
      act(() => {
        for (const callback of callbacks) {
          callback([], {} as ResizeObserver);
        }
      });

      await waitFor(() => {
        expect(lens.style.transform).toContain("translateY(40px)");
      });

      mockBox(host, { x: 8, y: 120, width: 184, height: 36 });
      act(() => {
        for (const callback of callbacks) {
          callback([], {} as ResizeObserver);
        }
      });

      await waitFor(() => {
        expect(lens.style.transform).toContain("translateY(120px)");
      });
    } finally {
      vi.unstubAllGlobals();
    }
  });

  it("keeps the same overlay node when the active option changes", async () => {
    const user = userEvent.setup();
    function Track() {
      const [current, setCurrent] = useState("one");
      return (
        <SelectionLensTrack id="interrupt-track">
          <button type="button" onClick={() => setCurrent("one")}>
            <SelectionLens active={current === "one"} />
            One
          </button>
          <button type="button" onClick={() => setCurrent("two")}>
            <SelectionLens active={current === "two"} />
            Two
          </button>
        </SelectionLensTrack>
      );
    }

    render(<Track />);
    const pill = screen.getByTestId("selection-lens");
    await user.click(screen.getByRole("button", { name: "Two" }));
    expect(screen.getByTestId("selection-lens")).toBe(pill);
  });
});
