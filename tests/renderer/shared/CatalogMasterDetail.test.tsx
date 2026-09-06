import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { getAgentBrand } from "@/shared/assets/agents";
import {
  BrandIconFrame,
  CatalogDetail,
  CatalogList,
  CatalogListItem,
  CatalogMasterDetail,
  CatalogRail,
} from "@/shared/ui/catalog";

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

  it("uses the shared resizable group rather than a second catalog drag implementation", () => {
    renderCatalog();
    expect(
      document.querySelector(".fy-catalog-master-detail [data-group]"),
    ).not.toBeNull();
    expect(
      document.querySelectorAll(".fy-catalog-master-detail [data-panel]"),
    ).toHaveLength(2);
  });
});
