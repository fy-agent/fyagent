import {
  createEvent,
  fireEvent,
  render,
  screen,
  within,
} from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { getAgentBrand } from "@/v2/shared/assets/agents";
import {
  BrandIconFrame,
  CatalogDetail,
  CatalogList,
  CatalogListItem,
  CatalogMasterDetail,
  CatalogRail,
} from "@/v2/shared/ui/catalog";

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

function renderCatalog() {
  const select = vi.fn();
  const brand = getAgentBrand("qoderwork");
  const view = render(
    <CatalogMasterDetail>
      <CatalogRail
        as="aside"
        ariaLabel="目录目标"
        title="选择 Agent"
        meta="目录元数据"
      >
        <CatalogList>
          <CatalogListItem
            asset={brand}
            label="QoderWork CN"
            summary="能力待验证"
            selected
            onSelect={select}
          />
        </CatalogList>
      </CatalogRail>
      <CatalogDetail ariaLabel="QoderWork CN 详情">
        <h2>QoderWork CN</h2>
        <BrandIconFrame asset={brand} size="detail" />
      </CatalogDetail>
    </CatalogMasterDetail>,
  );
  return { ...view, select, brand };
}

describe("CatalogMasterDetail", () => {
  it("keeps semantic rail selection, decorative artwork, and detail identity together", async () => {
    const user = userEvent.setup();
    const { select, brand } = renderCatalog();

    const rail = screen.getByRole("complementary", { name: "目录目标" });
    expect(
      within(rail).getByRole("heading", { name: "选择 Agent" }),
    ).toBeVisible();
    expect(within(rail).getByText("目录元数据")).toBeVisible();
    const item = within(rail).getByRole("button", {
      name: "QoderWork CN 能力待验证",
    });
    expect(item).toHaveAttribute("aria-current", "true");
    expect(within(rail).getByTestId("selection-lens")).toBeVisible();
    expect(item.querySelector('[data-size="list"]')).toHaveAttribute(
      "data-background",
      brand.list.background,
    );
    const listImage = item.querySelector("img");
    expect(listImage).toHaveAttribute("alt", "");
    expect(listImage).toHaveAttribute("aria-hidden", "true");

    const detail = screen.getByRole("region", {
      name: "QoderWork CN 详情",
    });
    expect(detail.querySelector('[data-size="detail"]')).toHaveAttribute(
      "data-corner",
      brand.detail.corner,
    );
    expect(detail.querySelector("img")).toHaveAttribute("aria-hidden", "true");
    expect(detail.parentElement).toHaveClass("fy-catalog-pane");
    expect(
      screen.getByRole("separator", { name: "调整目录与详情的宽度" }),
    ).toBeVisible();

    await user.click(item);
    expect(select).toHaveBeenCalledTimes(1);
  });

  it("resizes the rail from the separator and clamps width", async () => {
    const user = userEvent.setup();
    window.matchMedia = (query: string) =>
      ({
        matches: false,
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
    renderCatalog();
    const root = document.querySelector(
      ".fy-catalog-master-detail",
    ) as HTMLElement;
    const pane0 = root.querySelector(
      '.fy-split-pane[data-index="0"]',
    ) as HTMLElement;
    mockBox(root, { width: 900, left: 0 });
    mockBox(pane0, { width: 240, left: 0 });

    const handle = screen.getByRole("separator", {
      name: "调整目录与详情的宽度",
    });
    dispatchPointer(handle, "pointerdown", 240);
    dispatchPointer(handle, "pointermove", 300);
    dispatchPointer(handle, "pointerup", 300);
    expect(root.getAttribute("style")).toContain(
      "--fy-catalog-rail-width: 300px",
    );
    expect(handle).toHaveAttribute("aria-valuenow", "300");

    dispatchPointer(handle, "pointerdown", 300);
    dispatchPointer(handle, "pointermove", 800);
    dispatchPointer(handle, "pointerup", 800);
    expect(root.getAttribute("style")).toContain(
      "--fy-catalog-rail-width: 420px",
    );

    dispatchPointer(handle, "pointerdown", 420);
    dispatchPointer(handle, "pointermove", 0);
    dispatchPointer(handle, "pointerup", 0);
    expect(root.getAttribute("style")).toContain(
      "--fy-catalog-rail-width: 220px",
    );

    fireEvent.doubleClick(handle);
    expect(root.getAttribute("style")).not.toContain("--fy-catalog-rail-width");

    await user.click(handle);
    await user.keyboard("{ArrowRight}");
    expect(root.getAttribute("style")).toContain(
      "--fy-catalog-rail-width: 256px",
    );
  });
});
