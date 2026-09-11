import { render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { CatalogList } from "@/shared/ui/catalog";
import { SelectionLens } from "@/shared/ui/SelectionLens";

afterEach(() => vi.restoreAllMocks());

describe("CatalogList", () => {
  it("remeasures a reordered selection without replacing its focused button or lens", async () => {
    const nativeBox = HTMLElement.prototype.getBoundingClientRect;
    vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(
      function (this: HTMLElement) {
        if (this.matches(".fy-catalog-list"))
          return new DOMRect(0, 0, 200, 160);
        if (this.matches("button")) {
          const index = Array.from(
            this.parentElement!.querySelectorAll<HTMLElement>("button"),
          ).indexOf(this);
          return new DOMRect(8, 12 + index * 60, 184, 40);
        }
        return nativeBox.call(this);
      },
    );
    function List({ order }: { order: string[] }) {
      return (
        <CatalogList layoutKey={order.join(",")}>
          {order.map((id) => (
            <button key={id} type="button">
              <SelectionLens active={id === "Codex"} />
              {id}
            </button>
          ))}
        </CatalogList>
      );
    }
    const { rerender } = render(<List order={["QoderWork", "Codex"]} />);
    const selected = screen.getByRole("button", { name: "Codex" });
    const lens = screen.getByTestId("selection-lens");
    await waitFor(() =>
      expect(lens.style.transform).toContain("translateY(72px)"),
    );
    selected.focus();

    // Equal row and list sizes do not notify ResizeObserver. Only the
    // explicit order identity changes while the selected Agent stays fixed.
    rerender(<List order={["Codex", "QoderWork"]} />);
    await waitFor(() =>
      expect(lens.style.transform).toContain("translateY(12px)"),
    );
    expect(screen.getByRole("button", { name: "Codex" })).toBe(selected);
    expect(screen.getByTestId("selection-lens")).toBe(lens);
    expect(selected).toHaveFocus();
  });
});
