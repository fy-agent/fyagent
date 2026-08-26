import { readFileSync } from "node:fs";
import path from "node:path";

import { describe, expect, it } from "vitest";

const pageCss = readFileSync(
  path.resolve(process.cwd(), "src", "v2", "pages", "models", "Page.css"),
  "utf8",
);
const pageSource = readFileSync(
  path.resolve(process.cwd(), "src", "v2", "pages", "models", "Page.tsx"),
  "utf8",
);

describe("V2 Models management layout", () => {
  it("keeps the management title above the application catalog", () => {
    expect(pageSource).toMatch(
      /<header className="fy-models-page-heading">\s*<h1>模型管理<\/h1>\s*<\/header>\s*<CatalogMasterDetail>/,
    );
    expect(pageCss).toMatch(
      /\.fy-models-page-heading\s*\{[^}]*flex:\s*0\s+0\s+auto;[^}]*margin-bottom:\s*14px;/s,
    );
    expect(pageCss).toMatch(
      /\.fy-models-page\s*>\s*\.fy-catalog-master-detail\s*\{[^}]*flex:\s*1\s+1\s+auto;[^}]*min-height:\s*0;/s,
    );
  });
});
