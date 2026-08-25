import { readFileSync } from "node:fs";
import path from "node:path";

import { describe, expect, it } from "vitest";

const pageCss = readFileSync(
  path.resolve(process.cwd(), "src", "v2", "pages", "mcp", "page.css"),
  "utf8",
);
const pageSource = readFileSync(
  path.resolve(process.cwd(), "src", "v2", "pages", "mcp", "Page.tsx"),
  "utf8",
);

describe("V2 MCP management layout", () => {
  it("keeps the title above tabs/actions and balances the desktop three panes", () => {
    expect(pageSource).toMatch(
      /<header className="fy-feature-header">\s*<h1 className="fy-mcp-page-title">MCP 管理<\/h1>\s*<FeatureTabs[\s\S]*?<div className="fy-feature-actions">/,
    );
    expect(pageCss).toMatch(
      /\.fy-mcp-page\s*>\s*\.fy-feature-header\s*\{[^}]*display:\s*grid;[^}]*grid-template-columns:\s*minmax\(0,\s*1fr\)\s*auto;/s,
    );
    expect(pageCss).toMatch(
      /\.fy-mcp-page\s+\.fy-split-panes\[data-panes="3"\]\s*\{[^}]*--fy-split-pane-0:\s*clamp\(230px,\s*22vw,\s*292px\);[^}]*--fy-split-pane-1:\s*clamp\(350px,\s*31vw,\s*470px\);/s,
    );
  });
});
