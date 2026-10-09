import { readFileSync } from "node:fs";
import path from "node:path";

import { describe, expect, it } from "vitest";

const pages = path.resolve(process.cwd(), "src", "pages");
const promptCss = readFileSync(path.join(pages, "prompts", "page.css"), "utf8");
const memoryCss = readFileSync(path.join(pages, "memory", "page.css"), "utf8");

describe("Prompt and Memory editor reading geometry", () => {
  it("keeps readable body space and scroll access without a tall forced min-height", () => {
    for (const css of [promptCss, memoryCss]) {
      expect(css).not.toMatch(/min-height:\s*(220|330|450)px/);
      expect(css).toMatch(/min-height:\s*0;/);
    }
    expect(promptCss).toMatch(
      /\.fy-prompts-editor-content-field\s*\{[^}]*flex:\s*1 0 160px;[^}]*min-height:\s*160px;/,
    );
    // Prompt metadata/actions keep their space; the pane scrolls at short heights.
    expect(promptCss).toMatch(
      /\.fy-prompts-editor-form\s*\{[^}]*flex:\s*1 0 auto;[^}]*min-height:\s*0;/,
    );
    expect(promptCss).toMatch(
      /\.fy-prompts-editor-pane\s*\{[^}]*min-height:\s*0;[^}]*height:\s*100%;[^}]*overflow:\s*auto;/,
    );
    // Editable/read-only bodies still expand and own long-content scrolling.
    expect(promptCss).toMatch(
      /\.fy-prompts-editor-content,\s*\.fy-prompts-live-content\s*\{[^}]*flex:\s*1 1 auto;[^}]*min-height:\s*0;[^}]*overflow:\s*auto;[^}]*overscroll-behavior:\s*contain;/,
    );
    expect(memoryCss).toMatch(
      /\.fy-memory-editor-field\s*\{[^}]*flex:\s*1 1 auto;[^}]*min-height:\s*0;/,
    );
    expect(memoryCss).toMatch(
      /\.fy-memory-editor-textarea\s*\{[^}]*flex:\s*1 1 auto;[^}]*min-height:\s*0;[^}]*overflow:\s*auto;/,
    );
  });
});
