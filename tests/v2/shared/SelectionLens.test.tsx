import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { describe, expect, it } from "vitest";

import {
  SelectionLens,
  SelectionLensGroup,
  SelectionLensTrack,
  selectionLensCollapsedOrigin,
  selectionLensTransition,
} from "@/v2/shared/ui/SelectionLens";

describe("SelectionLens", () => {
  it("keeps the source L1 control spring", () => {
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
