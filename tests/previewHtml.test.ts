import { describe, expect, it } from "vitest";
import {
  htmlAttribute,
  htmlElements,
  scriptContent,
} from "../scripts/preview-html.mjs";
import { parseDistributionEntryAssets } from "../scripts/build-v2-preview.mjs";

describe("browser-compatible preview HTML parsing", () => {
  it("ignores comments, template and raw-text lookalikes", () => {
    const fake = '<script type="module" src="/fake.js"></script>';
    const source = `<!doctype html><html><head><!--${fake}--><script type="module" src="/real.js"></script></head><body><template>${fake}</template><textarea>${fake}</textarea></body></html>`;
    expect(
      parseDistributionEntryAssets(source).map((entry) =>
        entry.kind === "vite-bootstrap" ? entry.scriptSource : entry.source,
      ),
    ).toEqual(["/real.js"]);
  });

  it("handles quoted angle brackets, entity attributes and HTML end-tag syntax", () => {
    const source =
      '<head><SCRIPT data-note=">" TYPE="module" SRC="/entry.js?x=1&amp;y=2">const x=1;</SCRIPT\t\n ignored><link rel="stylesheet" href="/style.css"></head>';
    const entries = parseDistributionEntryAssets(source);
    expect(
      entries.map((entry) =>
        entry.kind === "vite-bootstrap" ? entry.scriptSource : entry.source,
      ),
    ).toEqual(["/entry.js?x=1&y=2", "/style.css"]);
    const element = htmlElements(source, ["script"])[0];
    expect(htmlAttribute(element, "data-note")).toBe(">");
    expect(scriptContent(source, element)).toBe("const x=1;");
    expect(
      source
        .slice(entries[0].start, entries[0].end)
        .endsWith("</SCRIPT\t\n ignored>"),
    ).toBe(true);
  });

  it("rejects an unclosed executable entry rather than silently consuming the page", () => {
    expect(() =>
      parseDistributionEntryAssets(
        '<head><script type="module" src="/entry.js">',
      ),
    ).toThrow("Unclosed script");
  });
});
