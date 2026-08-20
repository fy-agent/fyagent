import { readFileSync } from "node:fs";
import path from "node:path";

import { describe, expect, it } from "vitest";

const pageCss = readFileSync(
  path.resolve(process.cwd(), "src", "v2", "pages", "skills", "page.css"),
  "utf8",
);

describe("V2 Skills discovery page scroll", () => {
  it("lets the Skills feature page scroll as a whole on discovery", () => {
    expect(pageCss).toMatch(
      /\.fy-skills-page\.fy-skills-page-discovery\.fy-feature-page:has\(\s*\.fy-feature-workspace\s*\)\s*\{[^}]*overflow:\s*auto;/s,
    );
    expect(pageCss).toMatch(
      /\.fy-skills-page\.fy-skills-page-discovery\s+\.fy-feature-discovery-scroll\s*\{[^}]*overflow:\s*visible;/s,
    );
  });
});
