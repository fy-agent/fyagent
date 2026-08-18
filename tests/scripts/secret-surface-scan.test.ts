import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { afterEach, describe, expect, it } from "vitest";
// @ts-expect-error ESM scanner is untyped JavaScript.
import {
  FORBIDDEN_SOURCE_SPELLINGS,
  plantAndAssertCanary,
  scan,
  scanCodexFeatureRuntime,
  scanContractSchema,
  scanRepositoryRuntimeGlobal,
  scanRepositoryStaticInventory,
} from "../../scripts/tasks/secret-surface-scan.mjs";

const HERE = path.dirname(fileURLToPath(import.meta.url));
const SCANNER = path.resolve(HERE, "../../scripts/tasks/secret-surface-scan.mjs");

type ScanReport = {
  schemaVersion: number;
  levels: {
    contract_schema: string;
    codex_feature_runtime: string;
    repository_static_inventory: string;
    repository_runtime_global: string;
  };
  findings: Array<{
    level: string;
    code: string;
    path?: string;
    key?: string;
    canonical?: string;
    entry?: string;
    claim?: string;
  }>;
  requiredRuntimeEntries: string[];
  adjacentDebt: Array<{
    id: string;
    level: string;
    domains: string[];
    waiver: boolean;
  }>;
};

const fixtures: string[] = [];

function tempDir(label: string): string {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), `secret-surface-${label}-`));
  fixtures.push(root);
  return root;
}

function writeFile(absolute: string, contents: string): void {
  fs.mkdirSync(path.dirname(absolute), { recursive: true });
  fs.writeFileSync(absolute, contents);
}

function writeJson(absolute: string, value: unknown): void {
  writeFile(absolute, `${JSON.stringify(value, null, 2)}\n`);
}

function sha256Text(text: string): string {
  return createHash("sha256").update(text, "utf8").digest("hex");
}

function adjacentDebt() {
  return [
    {
      id: "codexMcpEnvOrHeaderCredential",
      level: "repository_static_inventory",
      domains: ["db", "live", "export", "sync"],
      waiver: false,
    },
  ];
}

function writeBaseline(
  root: string,
  extras: {
    baselinedFiles?: string[];
    fileDigests?: Record<string, string>;
    adjacentDebt?: unknown;
  } = {},
): string {
  const baselinePath = path.join(root, "secret-surface-baseline.json");
  writeJson(baselinePath, {
    schemaVersion: 1,
    levels: {
      contract_schema: "enforced",
      codex_feature_runtime: "enforced",
      repository_static_inventory: "enforced",
      repository_runtime_global: "NOT_CLAIMED",
    },
    adjacentDebt: extras.adjacentDebt ?? adjacentDebt(),
    baselinedFiles: extras.baselinedFiles ?? [],
    fileDigests: extras.fileDigests ?? {},
    requiredRuntimeEntries: [
      "state.json",
      "journal/**",
      "audit/**",
      "durable-replace-temp",
      ".retired-*",
    ],
  });
  return baselinePath;
}

function writeRuntimeRoot(root: string): string {
  const runtimeRoot = path.join(root, "runtime");
  writeJson(path.join(runtimeRoot, "state.json"), { schemaVersion: 1 });
  fs.mkdirSync(path.join(runtimeRoot, "journal"), { recursive: true });
  fs.mkdirSync(path.join(runtimeRoot, "audit"), { recursive: true });
  return runtimeRoot;
}

function writeAllowlistedSchema(root: string): void {
  writeJson(path.join(root, "schema.json"), { secretRef: "sec_demo" });
}

afterEach(() => {
  while (fixtures.length > 0) {
    const root = fixtures.pop();
    if (!root) continue;
    if (root.startsWith(os.tmpdir()) && path.basename(root).startsWith("secret-surface-")) {
      fs.rmSync(root, { recursive: true, force: true });
    }
  }
});

describe("secret-surface-scan contract constants", () => {
  it("keeps 25 source spellings that collapse to 24 canonical keys", () => {
    expect(FORBIDDEN_SOURCE_SPELLINGS).toHaveLength(25);
    const canonical = new Set(
      FORBIDDEN_SOURCE_SPELLINGS.map((spelling: string) =>
        spelling.toLowerCase().replace(/[^a-z0-9]/g, ""),
      ),
    );
    expect(canonical.size).toBe(24);
    expect(canonical.has("apikey")).toBe(true);
  });
});

describe("contract_schema", () => {
  it("fails temp JSON with apiKey and api_key object keys", () => {
    const root = tempDir("l1-neg");
    writeJson(path.join(root, "api-key.json"), { apiKey: "x" });
    writeJson(path.join(root, "api_key.json"), { api_key: "x" });
    const result = scanContractSchema({ root });
    expect(result.status).toBe("FAIL");
    const keys = result.findings.map((item: { key?: string }) => item.key);
    expect(keys).toContain("apiKey");
    expect(keys).toContain("api_key");
    expect(
      result.findings.every(
        (item: { code: string; canonical?: string }) =>
          item.code === "forbidden_semantic_key" && item.canonical === "apikey",
      ),
    ).toBe(true);
  });

  it("passes temp JSON that only uses allowlisted keys", () => {
    const root = tempDir("l1-pos");
    writeJson(path.join(root, "allowlisted.json"), { secretRef: "sec_demo" });
    const result = scanContractSchema({ root });
    expect(result.status).toBe("PASS");
    expect(result.findings).toEqual([]);
  });
});

describe("codex_feature_runtime", () => {
  it("fails a missing runtime root and a missing state.json", () => {
    const missingRoot = scanCodexFeatureRuntime({
      runtimeRoot: path.join(os.tmpdir(), "secret-surface-missing-runtime"),
    });
    expect(missingRoot.status).toBe("FAIL");
    expect(
      missingRoot.findings.some(
        (item: { code: string }) => item.code === "runtime_root_absent",
      ),
    ).toBe(true);

    const root = tempDir("l2-neg");
    const runtimeRoot = writeRuntimeRoot(root);
    fs.rmSync(path.join(runtimeRoot, "state.json"));
    const missingState = scanCodexFeatureRuntime({ runtimeRoot });
    expect(missingState.status).toBe("FAIL");
    expect(
      missingState.findings.some(
        (item: { code: string; entry?: string }) =>
          item.code === "required_entry_absent" && item.entry === "state.json",
      ),
    ).toBe(true);
  });

  it("passes a temp root with state.json, journal, and audit, and plantAndAssertCanary writes, finds, then cleans", () => {
    const root = tempDir("l2-pos");
    const runtimeRoot = writeRuntimeRoot(root);
    const result = scanCodexFeatureRuntime({ runtimeRoot });
    expect(result.status).toBe("PASS");
    expect(result.findings).toEqual([]);
    expect(result.requiredRuntimeEntries).toEqual([
      "state.json",
      "journal/**",
      "audit/**",
      "durable-replace-temp",
      ".retired-*",
    ]);

    const sink = path.join(runtimeRoot, "canary-sink.txt");
    const canary = plantAndAssertCanary(sink);
    expect(canary).toEqual({ planted: true, found: true, cleaned: true });
    expect(fs.existsSync(sink)).toBe(false);
    expect(JSON.stringify(canary)).not.toMatch(/sk-|ghp_|bearer /i);
  });
});

describe("repository_static_inventory", () => {
  it("fails when a baselined file gains a new password: literal", () => {
    const root = tempDir("l3-neg");
    const relative = "src/known.ts";
    const original = 'export const cfg = { secretRef: "sec_demo" };\n';
    writeFile(path.join(root, relative), original);
    const baseline = writeBaseline(root, {
      baselinedFiles: [relative],
      fileDigests: { [relative]: sha256Text(original) },
    });
    writeFile(path.join(root, relative), "export const cfg = { password: \"x\" };\n");
    const result = scanRepositoryStaticInventory({ root, baseline });
    expect(result.status).toBe("FAIL");
    expect(
      result.findings.some(
        (item: { code: string; key?: string }) =>
          item.code === "new_forbidden_literal" && item.key === "password",
      ),
    ).toBe(true);
  });

  it("passes when baseline lists codexMcpEnvOrHeaderCredential as visible debt, not a Level-2 waiver", () => {
    const root = tempDir("l3-pos");
    const baseline = writeBaseline(root);
    const result = scanRepositoryStaticInventory({ root, baseline });
    expect(result.status).toBe("PASS");
    expect(result.findings).toEqual([]);
    expect(result.adjacentDebt).toEqual([
      {
        id: "codexMcpEnvOrHeaderCredential",
        level: "repository_static_inventory",
        domains: ["db", "live", "export", "sync"],
        waiver: false,
      },
    ]);
  });
});

describe("repository_runtime_global", () => {
  it("fails a claim of PASS with illegal_global_claim", () => {
    const result = scanRepositoryRuntimeGlobal({ claim: "PASS" });
    expect(result.status).toBe("NOT_CLAIMED");
    expect(
      result.findings.some(
        (item: { code: string; claim?: string }) =>
          item.code === "illegal_global_claim" && item.claim === "PASS",
      ),
    ).toBe(true);
  });

  it("passes when claim is omitted or NOT_CLAIMED and report.levels.repository_runtime_global === NOT_CLAIMED", () => {
    const omitted = scanRepositoryRuntimeGlobal({});
    expect(omitted.status).toBe("NOT_CLAIMED");
    expect(omitted.findings).toEqual([]);

    const explicit = scanRepositoryRuntimeGlobal({ claim: "NOT_CLAIMED" });
    expect(explicit.status).toBe("NOT_CLAIMED");
    expect(explicit.findings).toEqual([]);

    const root = tempDir("l4-pos");
    writeAllowlistedSchema(root);
    const runtimeRoot = writeRuntimeRoot(root);
    const baseline = writeBaseline(root);
    const report = scan({ root, runtimeRoot, baseline }) as ScanReport;
    expect(report.levels.repository_runtime_global).toBe("NOT_CLAIMED");
    expect(report.levels.contract_schema).toBe("PASS");
    expect(report.levels.codex_feature_runtime).toBe("PASS");
    expect(report.levels.repository_static_inventory).toBe("PASS");
    expect(report.findings).toEqual([]);
  });
});

describe("CLI", () => {
  it("prints JSON and exits 1 when findings exist", () => {
    const root = tempDir("cli");
    writeJson(path.join(root, "bad.json"), { apiKey: "x" });
    const runtimeRoot = writeRuntimeRoot(root);
    const baseline = writeBaseline(root);
    const result = spawnSync(process.execPath, [
      SCANNER,
      "--root",
      root,
      "--runtime-root",
      runtimeRoot,
      "--baseline",
      baseline,
      "--claim",
      "PASS",
      "--json",
    ], {
      encoding: "utf8",
      windowsHide: true,
    });
    expect(result.status).toBe(1);
    const report = JSON.parse(result.stdout) as ScanReport;
    expect(report.levels.repository_runtime_global).toBe("NOT_CLAIMED");
    expect(report.levels.contract_schema).toBe("FAIL");
    expect(report.findings.some((item) => item.code === "illegal_global_claim")).toBe(
      true,
    );
    expect(report.findings.some((item) => item.key === "apiKey")).toBe(true);
    expect(result.stdout).not.toMatch(/sk-|ghp_|bearer /i);
    expect(result.stderr).not.toMatch(/sk-|ghp_|bearer /i);
  });
});
