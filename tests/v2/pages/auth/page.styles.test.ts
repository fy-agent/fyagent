import { readFileSync } from "node:fs";
import path from "node:path";

import { describe, expect, it } from "vitest";

const pageCss = readFileSync(
  path.resolve(process.cwd(), "src", "v2", "pages", "auth", "page.css"),
  "utf8",
);

describe("Auth page styles", () => {
  it("disables non-essential animation under reduced motion", () => {
    expect(pageCss).toMatch(
      /@media\s*\(prefers-reduced-motion:\s*reduce\)[\s\S]*?animation:\s*none;/,
    );
    expect(pageCss).toMatch(
      /@media\s*\(prefers-reduced-motion:\s*reduce\)[\s\S]*?transition:\s*none;/,
    );
  });

  it("uses one-pane mobile detail navigation without forcing a second column", () => {
    expect(pageCss).toMatch(
      /\.fy-auth-page\[data-mobile-detail="false"\][\s\S]*?display:\s*none;/,
    );
    expect(pageCss).toMatch(
      /\.fy-auth-page\[data-mobile-detail="true"\][\s\S]*?\.fy-auth-mobile-back[\s\S]*?display:\s*inline-flex;/,
    );
  });
});
