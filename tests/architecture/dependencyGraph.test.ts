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
const config = path.join(root, "config/dependency-cruiser.cjs");

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
      "src/main.tsx",
      "src/shared/features/queries.ts",
      "src/shared/features/change-plans-ui/useChangeJob.ts",
      "src/domain/codex-desktop/parsers.ts",
      "scripts/verify-route-chunks.mjs",
    ])
      expect(paths).toContain(required);
    // Required roots plus positive coverage prevent an empty old-tree scanner
    // from passing after the explicitly retired renderer is removed.
    expect(
      paths.filter((source) => source.startsWith("src/")).length,
    ).toBeGreaterThan(200);
    expect(result.summary.violations).toEqual([]);
  }, 30_000);

  it("fails on cycles, unresolved imports and an upward shared dependency", () => {
    const temporary = fs.mkdtempSync(path.join(os.tmpdir(), "fyagent-graph-"));
    try {
      fs.mkdirSync(path.join(temporary, "src/shared/ui"), {
        recursive: true,
      });
      fs.mkdirSync(path.join(temporary, "src/pages"), { recursive: true });
      fs.writeFileSync(
        path.join(temporary, "tsconfig.json"),
        '{"compilerOptions":{"module":"esnext"}}',
      );
      fs.writeFileSync(
        path.join(temporary, "src/shared/ui/a.ts"),
        'import "../../pages/b"; import "./missing";',
      );
      fs.writeFileSync(
        path.join(temporary, "src/pages/b.ts"),
        'import "../shared/ui/a";',
      );
      const rules = scan(temporary, ["src"]).summary.violations.map(
        (item) => item.rule.name,
      );
      expect(rules).toContain("no-runtime-cycle");
      expect(rules).toContain("no-unresolved-runtime-import");
      expect(rules).toContain("shared-does-not-import-pages-widgets-or-app");
    } finally {
      fs.rmSync(temporary, { recursive: true, force: true });
    }
  }, 30_000);
});
