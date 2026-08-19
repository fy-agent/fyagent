import { fireEvent, render, screen } from "@testing-library/react";
import { readFileSync } from "node:fs";
import path from "node:path";
import { useState } from "react";
import { describe, expect, it, vi } from "vitest";

import { FeatureList, FeatureListItem } from "@/v2/shared/ui/FeatureList";
import { FeaturePagination } from "@/v2/shared/ui/FeaturePagination";
import { FeatureSearch } from "@/v2/shared/ui/FeatureSearch";
import { FeatureTabs } from "@/v2/shared/ui/FeatureTabs";

describe("FeatureTabs", () => {
  it("keeps one selected tab and reports the next id", async () => {
    const onChange = vi.fn();
    render(
      <FeatureTabs
        id="demo-tabs"
        label="演示页签"
        value="one"
        onChange={onChange}
        options={[
          { id: "one", label: "已安装" },
          { id: "two", label: "发现" },
        ]}
      />,
    );

    expect(screen.getByRole("tab", { name: "已安装" })).toHaveAttribute(
      "aria-selected",
      "true",
    );
    fireEvent.click(screen.getByRole("tab", { name: "发现" }));
    expect(onChange).toHaveBeenCalledWith("two");
  });
});

describe("FeatureSearch", () => {
  it("reports input changes with an accessible search field", () => {
    const onValueChange = vi.fn();
    render(
      <FeatureSearch
        value=""
        onValueChange={onValueChange}
        placeholder="搜索名称"
        ariaLabel="搜索项目"
        clearLabel="清除搜索"
      />,
    );

    const input = screen.getByRole("searchbox", { name: "搜索项目" });
    expect(input).toHaveAttribute("placeholder", "搜索名称");
    fireEvent.change(input, { target: { value: "alpha" } });
    expect(onValueChange).toHaveBeenCalledWith("alpha");
    expect(
      screen.queryByRole("button", { name: "清除搜索" }),
    ).not.toBeInTheDocument();
  });

  it("clears a non-empty search from the button or Escape", () => {
    const onValueChange = vi.fn();
    render(
      <FeatureSearch
        value="alpha"
        onValueChange={onValueChange}
        placeholder="搜索名称"
        ariaLabel="搜索项目"
        clearLabel="清除搜索"
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "清除搜索" }));
    expect(onValueChange).toHaveBeenCalledWith("");

    fireEvent.keyDown(screen.getByRole("searchbox", { name: "搜索项目" }), {
      key: "Escape",
    });
    expect(onValueChange).toHaveBeenLastCalledWith("");
  });
});

describe("FeatureList", () => {
  it("marks the selected row and keeps one overlay pill", () => {
    function List() {
      const [current, setCurrent] = useState("one");
      return (
        <FeatureList id="demo-list">
          <FeatureListItem
            selected={current === "one"}
            title="One"
            onSelect={() => setCurrent("one")}
          >
            <span>first</span>
          </FeatureListItem>
          <FeatureListItem
            selected={current === "two"}
            title="Two"
            onSelect={() => setCurrent("two")}
          >
            <span>second</span>
          </FeatureListItem>
        </FeatureList>
      );
    }

    render(<List />);
    expect(screen.getByRole("button", { name: /One/ })).toHaveAttribute(
      "aria-current",
      "true",
    );
    expect(screen.getAllByTestId("selection-lens")).toHaveLength(1);
    fireEvent.click(screen.getByRole("button", { name: /Two/ }));
    expect(screen.getByRole("button", { name: /Two/ })).toHaveAttribute(
      "aria-current",
      "true",
    );
  });

  it("uses a column flex track so the overlay lens is not a grid item", () => {
    const featuresCss = readFileSync(
      path.resolve(process.cwd(), "src", "v2", "app", "styles", "features.css"),
      "utf8",
    );
    expect(featuresCss).toMatch(
      /\.fy-feature-list\s*\{[^}]*display:\s*flex;[^}]*flex-direction:\s*column;/s,
    );
    expect(featuresCss).not.toMatch(
      /\.fy-feature-list\s*\{[^}]*display:\s*grid;/s,
    );
  });
});

describe("FeaturePagination", () => {
  it("keeps the current page selected and reports the next page", () => {
    function Pager() {
      const [page, setPage] = useState(5);
      return (
        <FeaturePagination
          page={page}
          totalPages={8}
          ariaLabel="演示分页"
          onPageChange={setPage}
        />
      );
    }

    render(<Pager />);
    const current = screen.getByRole("button", { name: "5" });
    expect(current).toHaveAttribute("aria-current", "page");
    expect(screen.getByRole("button", { name: "3" })).toBeVisible();
    expect(screen.getByRole("button", { name: "7" })).toBeVisible();
    expect(screen.queryByRole("button", { name: "2" })).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "6" }));
    expect(screen.getByRole("button", { name: "6" })).toHaveAttribute(
      "aria-current",
      "page",
    );
  });

  it("hides when there is only one page", () => {
    render(
      <FeaturePagination
        page={1}
        totalPages={1}
        ariaLabel="演示分页"
        onPageChange={() => undefined}
      />,
    );
    expect(screen.queryByRole("navigation", { name: "演示分页" })).toBeNull();
  });

  it("gives discovery cards an independent scroller and leaves detail scroll without overflow", () => {
    const featuresCss = readFileSync(
      path.resolve(process.cwd(), "src", "v2", "app", "styles", "features.css"),
      "utf8",
    );
    expect(featuresCss).toMatch(
      /\.fy-feature-discovery-scroll\s*\{[^}]*overflow:\s*auto;/s,
    );
    const detailBlock =
      featuresCss.match(/\.fy-feature-detail-scroll\s*\{[^}]*\}/s)?.[0] ?? "";
    expect(detailBlock).toContain(".fy-feature-detail-scroll");
    expect(detailBlock).not.toMatch(/overflow\s*:/);
  });
});
