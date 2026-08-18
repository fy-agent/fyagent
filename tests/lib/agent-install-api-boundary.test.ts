import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

describe("agent-install API boundary", () => {
  const source = readFileSync(
    resolve(process.cwd(), "src/lib/api/agent-install.ts"),
    "utf8",
  );

  it("start_install sends only snapshotId", () => {
    expect(source).toContain("request: { snapshotId }");
    expect(source).not.toMatch(/expectedReleaseId|downloadUrl|packagePath/);
  });

  it("has no url path hash script fields on the facade", () => {
    expect(source).not.toMatch(/\b(url|path|hash|script)\s*:/);
  });
});
