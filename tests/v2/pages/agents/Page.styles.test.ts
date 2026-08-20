import { readFileSync } from "node:fs";
import path from "node:path";

import { describe, expect, it } from "vitest";

const repositoryRoot = process.cwd();
const catalogCss = readFileSync(
  path.resolve(
    repositoryRoot,
    "src",
    "v2",
    "shared",
    "ui",
    "catalog",
    "catalog.css",
  ),
  "utf8",
);
const splitCss = readFileSync(
  path.resolve(
    repositoryRoot,
    "src",
    "v2",
    "shared",
    "ui",
    "split",
    "split.css",
  ),
  "utf8",
);
const featuresCss = readFileSync(
  path.resolve(repositoryRoot, "src", "v2", "app", "styles", "features.css"),
  "utf8",
);
const pageCss = ["agents", "models"]
  .map((page) =>
    readFileSync(
      path.resolve(repositoryRoot, "src", "v2", "pages", page, "Page.css"),
      "utf8",
    ),
  )
  .join("\n");

function rule(css: string, selector: string): string {
  const escaped = selector.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const match = css.match(new RegExp(`${escaped}\\s*\\{([^}]*)\\}`));
  expect(match, `missing CSS rule for ${selector}`).not.toBeNull();
  return match?.[1] ?? "";
}

describe("shared V2 catalog presentation styles", () => {
  it("owns the only catalog rail geometry and independently scrolling panes", () => {
    const tokens = rule(catalogCss, ".fy-catalog-master-detail");
    expect(tokens).toMatch(
      /--fy-catalog-rail-width:\s*clamp\(220px,\s*24vw,\s*268px\);/,
    );
    expect(tokens).toMatch(/--fy-catalog-gap:\s*14px;/);
    expect(tokens).toMatch(
      /--fy-split-pane-0:\s*var\(--fy-catalog-rail-width\);/,
    );
    expect(tokens).not.toMatch(/grid-template-columns:/);
    expect(rule(splitCss, ".fy-split-panes")).toMatch(
      /--fy-split-gap:\s*14px;/,
    );
    expect(rule(splitCss, '.fy-split-panes[data-panes="2"]')).toMatch(
      /grid-template-columns:\s*var\(--fy-split-pane-0\)\s+var\(--fy-split-gap\)\s+minmax\(0,\s*1fr\);/,
    );
    expect(splitCss).toMatch(
      /@media\s*\(max-width:\s*760px\)[\s\S]*?\.fy-split-panes\[data-panes="2"\][\s\S]*?grid-template-columns:\s*minmax\(0,\s*1fr\);/,
    );
    expect(splitCss).toMatch(
      /@media\s*\(max-width:\s*760px\)[\s\S]*?\.fy-split-resize-handle[\s\S]*?display:\s*none;/,
    );
    expect(rule(catalogCss, ".fy-feature-page.fy-catalog-page")).toMatch(
      /gap:\s*0;/,
    );
    expect(rule(splitCss, ".fy-feature-page.fy-split-page")).toMatch(
      /gap:\s*0;/,
    );
    expect(pageCss).not.toMatch(
      /\.fy-(?:agents|models)-page\s*\{[^}]*(?:gap|padding-top)\s*:/s,
    );
    expect(
      rule(
        splitCss,
        ".fy-content-viewport:has(> :not([hidden]) .fy-split-page)",
      ),
    ).toMatch(/overflow:\s*hidden;/);
    expect(
      rule(
        splitCss,
        ".fy-content-viewport:has(> :not([hidden]) .fy-split-page)",
      ),
    ).toMatch(/scrollbar-gutter:\s*stable;/);
    expect(catalogCss).toMatch(
      /\.fy-catalog-rail,\s*\.fy-catalog-pane\s*\{[^}]*overflow:\s*auto;/s,
    );
    expect(splitCss).toMatch(/\.fy-split-pane\s*\{[^}]*overflow:\s*auto;/s);
    expect(splitCss).toMatch(
      /\.fy-split-pane\s*>\s*\*\s*\{[^}]*overflow:\s*auto;/s,
    );
    expect(splitCss).toMatch(
      /\.fy-split-pane\s*>\s*\*\s*\{[^}]*min-height:\s*100%;/s,
    );
    expect(catalogCss).toMatch(
      /\.fy-catalog-pane\s*>\s*\*\s*\{[^}]*min-height:\s*100%;/s,
    );
    expect(catalogCss).not.toMatch(
      /\.fy-catalog-pane\s*\{[^}]*display:\s*grid/s,
    );
  });

  it("keeps split-pane children scrolling and assignment rows wrapping", () => {
    const child = rule(splitCss, ".fy-split-pane > *");
    expect(child).toMatch(/overflow:\s*auto;/);
    expect(child).toMatch(/min-height:\s*100%;/);
    expect(child).toMatch(/height:\s*100%;/);
    const assignment = rule(featuresCss, ".fy-feature-assignment");
    expect(assignment).toMatch(/flex-wrap:\s*wrap;/);
    expect(assignment).toMatch(/min-width:\s*0;/);
  });

  it("freezes shared row, frame, and artwork geometry", () => {
    const layout = rule(catalogCss, ".fy-catalog-master-detail");
    expect(layout).toMatch(/--fy-catalog-row-min-height:\s*56px;/);
    expect(layout).toMatch(/--fy-catalog-list-frame-size:\s*36px;/);
    expect(layout).toMatch(/--fy-catalog-list-artwork-size:\s*28px;/);
    expect(layout).toMatch(/--fy-catalog-detail-frame-size:\s*64px;/);
    expect(layout).toMatch(/--fy-catalog-detail-artwork-size:\s*48px;/);
    expect(rule(catalogCss, ".fy-catalog-list-item")).toMatch(
      /min-height:\s*var\(--fy-catalog-row-min-height\);/,
    );
    expect(catalogCss).not.toMatch(/transition:\s*all/);
  });

  it("keeps catalog geometry, brand exceptions, and motion out of page CSS", () => {
    expect(pageCss).not.toMatch(
      /\.fy-(?:agent-layout|models-layout|agent-selector|models-target(?:-list|-icon)?)(?:\s|,|\{|:|\[)/,
    );
    expect(pageCss).not.toMatch(/qoderwork|trae-work|workbuddy|claude-code/i);
    expect(catalogCss).toMatch(
      /@media\s*\(prefers-reduced-motion:\s*reduce\)[\s\S]*?transition:\s*none;/,
    );
    const withoutReducedMotion = catalogCss.replace(
      /@media\s*\(prefers-reduced-motion:\s*reduce\)\s*\{[\s\S]*\}\s*$/,
      "",
    );
    expect(withoutReducedMotion).not.toMatch(
      /\.fy-catalog-(?:master-detail|rail|brand-frame|list-copy)[^{]*\{[^}]*(?:animation|transition)\s*:/s,
    );
  });
});
