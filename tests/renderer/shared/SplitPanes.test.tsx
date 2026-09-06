import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { SplitPanes } from "@/shared/ui/split";

// Pointer/keyboard constraints and resize interruption require real layout;
// those former mocked-pixel cases live in browser/layout-integrity.spec.ts.
describe("SplitPanes adapter", () => {
  it("keeps content and drafts usable without an observer rather than mounting an unsupported resize engine", () => {
    vi.stubGlobal("ResizeObserver", undefined);
    try {
      const { rerender, unmount } = render(
        <SplitPanes>
          <section>list</section>
          <input aria-label="fallback draft" />
        </SplitPanes>,
      );
      const input = screen.getByRole("textbox", { name: "fallback draft" });
      fireEvent.change(input, { target: { value: "not discarded" } });
      expect(
        document.querySelector("[data-resize-fallback='static']"),
      ).not.toBeNull();
      expect(screen.queryByRole("separator")).not.toBeInTheDocument();
      rerender(
        <SplitPanes minWidths={[240, 420]}>
          <section>list</section>
          <input aria-label="fallback draft" />
        </SplitPanes>,
      );
      expect(screen.getByRole("textbox")).toBe(input);
      expect(input).toHaveValue("not discarded");
      unmount();
    } finally {
      vi.unstubAllGlobals();
    }
  });
  it("delegates semantics and maintains explicit accessible separator labels", () => {
    render(
      <SplitPanes separatorLabels={["调整两栏宽度"]}>
        <section>left</section>
        <section>right</section>
      </SplitPanes>,
    );
    expect(
      screen.getByRole("separator", { name: "调整两栏宽度" }),
    ).toBeInTheDocument();
    expect(document.querySelectorAll("[data-panel]")).toHaveLength(2);
    expect(screen.getByText("right")).toBeVisible();
  });
  it("retains an editor instance when dimensional constraints change", () => {
    const content = <input aria-label="draft" defaultValue="" />;
    const { rerender } = render(
      <SplitPanes minWidths={[220, 360]}>
        <section>left</section>
        {content}
      </SplitPanes>,
    );
    const editor = screen.getByRole("textbox");
    // JSDOM has no panel bounds, so pointer hit testing belongs to the browser
    // regression. This case verifies the editor lifetime, not drag admission.
    fireEvent.change(editor, { target: { value: "unsaved value" } });
    expect(editor).toHaveValue("unsaved value");
    rerender(
      <SplitPanes minWidths={[170, 260]}>
        <section>left</section>
        {content}
      </SplitPanes>,
    );
    expect(screen.getByRole("textbox")).toBe(editor);
    expect(editor).toHaveValue("unsaved value");
  });
  it("keeps nested group IDs unique and uses the requested three-pane labels", () => {
    render(
      <SplitPanes separatorLabels={["第一分隔", "第二分隔"]}>
        <section>list</section>
        <SplitPanes>
          <section>nested one</section>
          <section>nested two</section>
        </SplitPanes>
        <section>assignment</section>
      </SplitPanes>,
    );
    expect(
      screen.getByRole("separator", { name: "第一分隔" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("separator", { name: "第二分隔" }),
    ).toBeInTheDocument();
    const ids = [...document.querySelectorAll("[data-panel]")].map(
      (node) => node.id,
    );
    expect(new Set(ids).size).toBe(ids.length);
  });
});
