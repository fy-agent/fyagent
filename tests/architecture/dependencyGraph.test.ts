import { execFileSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { describe, expect, it } from "vitest";

const root = process.cwd();
const cli = path.join(
  root,
  "node_modules/dependency-cruiser/bin/dependency-cruise.mjs",
);
const config = path.join(root, ".dependency-cruiser.cjs");

function scan(cwd: string, directories: string[]) {
  const args = [
    cli,
    "--config",
    config,
    "--output-type",
    "json",
    ...directories,
  ];
  let output: string;
  try {
    output = execFileSync(process.execPath, args, {
      cwd,
      encoding: "utf8",
      maxBuffer: 16 * 1024 * 1024,
    });
  } catch (error) {
    const result = error as { stdout?: string; status?: number };
    if (result.status !== 1 || !result.stdout) throw error;
    output = result.stdout;
  }
  return JSON.parse(output) as {
    modules: { source: string }[];
    summary: {
      violations: { rule: { name: string }; from: string; to: string }[];
      environment: { issues?: unknown[] };
    };
  };
}

describe("executable runtime dependency graph", () => {
  it("resolves real TypeScript and keeps all runtime boundaries acyclic", () => {
    const result = scan(root, ["src", "scripts"]);
    expect(result.summary.environment.issues ?? []).toEqual([]);
    const paths = result.modules.map((module) => module.source);
    for (const required of [
      "src/App.tsx",
      "src/v2/shared/features/queries.ts",
      "src/v2/pages/models/apply/useChangeJob.ts",
      "scripts/build-v2-preview.mjs",
    ])
      expect(paths).toContain(required);
    expect(paths.length).toBeGreaterThan(500);
    expect(result.summary.violations).toEqual([]);
  }, 30_000);

  it("fails on cycles, unresolved imports and an upward V2 dependency", () => {
    const temporary = fs.mkdtempSync(path.join(os.tmpdir(), "fyagent-graph-"));
    try {
      fs.mkdirSync(path.join(temporary, "src/v2/shared/ui"), {
        recursive: true,
      });
      fs.mkdirSync(path.join(temporary, "src/v2/pages"), { recursive: true });
      fs.writeFileSync(
        path.join(temporary, "tsconfig.json"),
        '{"compilerOptions":{"module":"esnext"}}',
      );
      fs.writeFileSync(
        path.join(temporary, "src/v2/shared/ui/a.ts"),
        'import "../../pages/b"; import "./missing";',
      );
      fs.writeFileSync(
        path.join(temporary, "src/v2/pages/b.ts"),
        'import "../shared/ui/a";',
      );
      const rules = scan(temporary, ["src"]).summary.violations.map(
        (item) => item.rule.name,
      );
      expect(rules).toContain("no-runtime-cycle");
      expect(rules).toContain("no-unresolved-runtime-import");
      expect(rules).toContain("v2-shared-does-not-import-pages-or-widgets");
    } finally {
      fs.rmSync(temporary, { recursive: true, force: true });
    }
  }, 30_000);
});
