import { readFileSync } from "node:fs";
import path from "node:path";

import { describe, expect, it } from "vitest";

const pageCss = readFileSync(
  path.resolve(process.cwd(), "src", "pages", "mcp", "page.css"),
  "utf8",
);
const pageSource = readFileSync(
  path.resolve(process.cwd(), "src", "pages", "mcp", "Page.tsx"),
  "utf8",
);

describe("MCP management layout", () => {
  it("keeps the title above tabs/actions and balances the desktop three panes", () => {
    expect(pageSource).toMatch(
      /<header className="fy-feature-header">\s*<h1 className="fy-mcp-page-title">MCP 管理<\/h1>\s*<FeatureTabs[\s\S]*?<div className="fy-feature-actions">/,
    );
    expect(pageCss).toMatch(
      /\.fy-mcp-page\s*>\s*\.fy-feature-header\s*\{[^}]*display:\s*grid;[^}]*grid-template-columns:\s*minmax\(0,\s*1fr\)\s*auto;/s,
    );
    expect(pageSource).toMatch(/<SplitPanes\s+\{\.\.\.DETAIL_PANE_SIZING\}/);
    expect(pageSource).toContain("<BulkAssignmentDialog");
    expect(pageSource).not.toContain("<BulkAssignmentPanel");
    expect(pageSource).toMatch(
      /<header className="fy-feature-header">[\s\S]*?onClick=\{\(\) => setBulkOpen\(true\)\}[\s\S]*?批量分配[\s\S]*?<\/header>/,
    );
    expect(pageCss).not.toContain("--fy-split-pane-");
  });
});
