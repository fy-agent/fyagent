import { readFileSync } from "node:fs";
import path from "node:path";

import { describe, expect, it } from "vitest";

const pages = path.resolve(process.cwd(), "src", "v2", "pages");
const promptCss = readFileSync(path.join(pages, "prompts", "page.css"), "utf8");
const memoryCss = readFileSync(path.join(pages, "memory", "page.css"), "utf8");

describe("Prompt and Memory editor reading geometry", () => {
  it("lets the body fill the pane instead of forcing a tall min-height", () => {
    for (const css of [promptCss, memoryCss]) {
      expect(css).not.toMatch(/min-height:\s*(220|330|450)px/);
      expect(css).toMatch(/min-height:\s*0;/);
    }
    expect(promptCss).toMatch(
      /\.fy-prompts-editor-content-field\s*\{[^}]*flex:\s*1 1 auto;/,
    );
    expect(memoryCss).toMatch(
      /\.fy-memory-editor-field\s*\{[^}]*flex:\s*1 1 auto;/,
    );
  });
});
