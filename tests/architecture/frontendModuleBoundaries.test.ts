import fs from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";

const repositoryRoot = path.resolve(process.cwd());

function read(relativePath: string): string {
  return fs.readFileSync(path.join(repositoryRoot, relativePath), "utf8");
}

describe("frontend modular architecture boundaries", () => {
  it("keeps provider config compatibility exports separate from implementations", () => {
    const facade = read("src/utils/providerConfigUtils.ts");
    const json = read("src/utils/providerConfigJsonUtils.ts");
    const codex = read("src/utils/codexConfigUtils.ts");
    const structural = read("src/utils/providerConfigStructural.ts");

    expect(facade).toContain('from "@/utils/providerConfigJsonUtils"');
    expect(facade).toContain('from "@/utils/codexConfigUtils"');
    expect(facade).not.toContain("JSON.parse(");
    expect(facade).not.toContain("parseToml(");
    expect(facade).not.toContain("TOML_SECTION_HEADER_PATTERN");

    expect(json).toContain("updateCommonConfigSnippet");
    expect(json).toContain("applyTemplateValues");
    expect(codex).toContain("extractCodexBaseUrl");
    expect(codex).toContain("setCodexModelName");
    expect(structural).toContain("FORBIDDEN_MERGE_KEYS");
    expect(structural).toContain("sanitizeSnippet");
  });
});
