import { createEvent, fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { SplitPanes } from "@/v2/shared/ui/split";

function dispatchPointer(
  element: Element,
  type: "pointerdown" | "pointermove" | "pointerup",
  clientX: number,
) {
  const event =
    type === "pointerdown"
      ? createEvent.pointerDown(element, { button: 0 })
      : type === "pointermove"
        ? createEvent.pointerMove(element, { button: 0 })
        : createEvent.pointerUp(element, { button: 0 });
  Object.defineProperties(event, {
    clientX: { configurable: true, get: () => clientX },
    clientY: { configurable: true, get: () => 16 },
  });
  fireEvent(element, event);
}

function mockBox(
  element: Element,
  box: { width: number; left?: number; height?: number },
) {
  vi.spyOn(element, "getBoundingClientRect").mockReturnValue({
    x: box.left ?? 0,
    y: 0,
    top: 0,
    left: box.left ?? 0,
    bottom: box.height ?? 400,
    right: (box.left ?? 0) + box.width,
    width: box.width,
    height: box.height ?? 400,
    toJSON() {
      return {};
    },
  } as DOMRect);
}

function stubMatchMedia(matches: boolean) {
  window.matchMedia = (query: string) =>
    ({
      matches,
      media: query,
      onchange: null,
      addEventListener() {},
      removeEventListener() {},
      addListener() {},
      removeListener() {},
      dispatchEvent() {
        return false;
      },
    }) as MediaQueryList;
}

describe("SplitPanes", () => {
  it("resizes two panes from the separator and restores the default on double-click", async () => {
    const user = userEvent.setup();
    stubMatchMedia(false);
    render(
      <SplitPanes
        minWidths={[220, 360]}
        maxWidths={[420]}
        separatorLabels={["调整两栏宽度"]}
      >
        <section>left</section>
        <section>right</section>
      </SplitPanes>,
    );

    const root = document.querySelector(".fy-split-panes") as HTMLElement;
    const pane0 = root.querySelector(
      '.fy-split-pane[data-index="0"]',
    ) as HTMLElement;
    mockBox(root, { width: 900, left: 0 });
    mockBox(pane0, { width: 240, left: 0 });

    const handle = screen.getByRole("separator", { name: "调整两栏宽度" });
    dispatchPointer(handle, "pointerdown", 240);
    dispatchPointer(handle, "pointermove", 300);
    dispatchPointer(handle, "pointerup", 300);
    expect(root.getAttribute("style")).toContain("--fy-split-pane-0: 300px");
    expect(handle).toHaveAttribute("aria-valuenow", "300");

    fireEvent.doubleClick(handle);
    expect(root.getAttribute("style") || "").not.toContain("--fy-split-pane-0");

    await user.click(handle);
    await user.keyboard("{ArrowRight}");
    expect(root.getAttribute("style")).toContain("--fy-split-pane-0: 256px");
  });

  it("resizes the middle pane of a three-column layout", () => {
    stubMatchMedia(false);
    render(
      <SplitPanes
        minWidths={[220, 330, 220]}
        maxWidths={[420]}
        separatorLabels={["调整列表与详情的宽度", "调整详情与侧栏的宽度"]}
      >
        <section>list</section>
        <section>detail</section>
        <section>side</section>
      </SplitPanes>,
    );

    const root = document.querySelector(".fy-split-panes") as HTMLElement;
    const pane0 = root.querySelector(
      '.fy-split-pane[data-index="0"]',
    ) as HTMLElement;
    const pane1 = root.querySelector(
      '.fy-split-pane[data-index="1"]',
    ) as HTMLElement;
    mockBox(root, { width: 1280, left: 0 });
    mockBox(pane0, { width: 240, left: 0 });
    mockBox(pane1, { width: 400, left: 254 });

    expect(root).toHaveAttribute("data-panes", "3");
    const detailHandle = screen.getByRole("separator", {
      name: "调整详情与侧栏的宽度",
    });
    dispatchPointer(detailHandle, "pointerdown", 654);
    dispatchPointer(detailHandle, "pointermove", 704);
    dispatchPointer(detailHandle, "pointerup", 704);
    expect(root.getAttribute("style")).toContain("--fy-split-pane-1: 450px");
    expect(detailHandle).toHaveAttribute("aria-valuenow", "450");
  });

  it("ignores pointer resize when the layout is stacked", () => {
    stubMatchMedia(true);
    render(
      <SplitPanes separatorLabels={["调整两栏宽度"]}>
        <section>left</section>
        <section>right</section>
      </SplitPanes>,
    );
    const root = document.querySelector(".fy-split-panes") as HTMLElement;
    const pane0 = root.querySelector(
      '.fy-split-pane[data-index="0"]',
    ) as HTMLElement;
    mockBox(root, { width: 600, left: 0 });
    mockBox(pane0, { width: 600, left: 0 });
    const handle = screen.getByRole("separator", { name: "调整两栏宽度" });
    dispatchPointer(handle, "pointerdown", 300);
    dispatchPointer(handle, "pointermove", 380);
    dispatchPointer(handle, "pointerup", 380);
    expect(root.getAttribute("style")).toBeNull();
  });
});
