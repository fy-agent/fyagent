import { readFileSync } from "node:fs";
import path from "node:path";

import { describe, expect, it } from "vitest";

const pageCss = readFileSync(
  path.resolve(process.cwd(), "src", "pages", "skills", "page.css"),
  "utf8",
);
const pageSource = readFileSync(
  path.resolve(process.cwd(), "src", "pages", "skills", "Page.tsx"),
  "utf8",
);

describe("Skills discovery page scroll", () => {
  it("uses bounded shared workspace tabs instead of a discovery-only overflow override", () => {
    expect(pageSource.match(/layout="workspace"/g)).toHaveLength(2);
    expect(pageCss).not.toContain("fy-skills-page-discovery");
  });
});

describe("Skills management layout", () => {
  it("keeps the title above tabs/actions and balances the desktop three panes", () => {
    expect(pageSource).toMatch(
      /<header className="fy-feature-header">\s*<h1 className="fy-skills-page-title">Skills 管理<\/h1>\s*<FeatureTabs[\s\S]*?<div className="fy-feature-actions">/,
    );
    expect(pageCss).toMatch(
      /\.fy-skills-page\s*>\s*\.fy-feature-header\s*\{[^}]*display:\s*grid;[^}]*grid-template-columns:\s*minmax\(0,\s*1fr\)\s*auto;/s,
    );
    expect(pageSource).toMatch(/<SplitPanes\s+\{\.\.\.DETAIL_PANE_SIZING\}/);
    expect(pageSource).toContain("<BulkAssignmentPanel");
    expect(pageCss).not.toContain("--fy-split-pane-");
  });
});
