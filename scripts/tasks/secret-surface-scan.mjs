#!/usr/bin/env node

import { createHash, randomBytes } from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { pathToFileURL } from "node:url";

export const SCANNER_VERSION = 1;
export const SCHEMA_VERSION = 1;

export const LEVELS = Object.freeze([
  "contract_schema",
  "codex_feature_runtime",
  "repository_static_inventory",
  "repository_runtime_global",
]);

/** Contract §12.3 sole source-spelling list: 25 rows, 24 canonical (apiKey/api_key collapse). */
export const FORBIDDEN_SOURCE_SPELLINGS = Object.freeze([
  "secret",
  "secretValue",
  "value",
  "apiKey",
  "api_key",
  "openaiApiKey",
  "experimentalBearerToken",
  "token",
  "accessToken",
  "refreshToken",
  "accessKey",
  "secretKey",
  "password",
  "authorization",
  "credential",
  "privateKey",
  "credentialBlob",
  "backendLocator",
  "rawError",
  "rawMessage",
  "rawConfig",
  "providerSettings",
  "liveSettings",
  "absolutePath",
  "materialDigest",
]);

export const ALLOWLISTED_KEYS = Object.freeze([
  "secretRef",
  "secretRefDisplay",
  "secretState",
  "secretSummary",
  "secretCount",
  "secretBackend",
  "secretPurpose",
  "secretOperation",
  "secretCandidate",
  "secretProjection",
  "lastValidatedAt",
]);

export const REQUIRED_ADJACENT_DEBT_ID = "codexMcpEnvOrHeaderCredential";
export const REQUIRED_ADJACENT_DEBT_DOMAINS = Object.freeze([
  "db",
  "live",
  "export",
  "sync",
]);

export const REQUIRED_RUNTIME_ENTRIES = Object.freeze([
  "state.json",
  "journal/**",
  "audit/**",
  "durable-replace-temp",
  ".retired-*",
]);

export const DEFAULT_BASELINE_RELATIVE =
  ".trellis/tasks/08-14-issue-35-secret-backend/research/secret-surface-baseline.json";

const SKIP_DIR_NAMES = new Set([".git", "node_modules", "target", "dist"]);
const SCHEMA_EXTENSIONS = new Set([
  ".json",
  ".ts",
  ".tsx",
  ".js",
  ".mjs",
  ".rs",
]);

const IDENT_KEY_RE =
  /(?:^|[^A-Za-z0-9_$])([A-Za-z_][A-Za-z0-9_]*)\s*:/gm;
const QUOTED_KEY_RE = /(["'])([^"'\n]+)\1\s*:/gm;

function isAscii(value) {
  for (const ch of value) {
    if ((ch.codePointAt(0) ?? 0) > 0x7f) return false;
  }
  return true;
}

function asciiLower(value) {
  let out = "";
  for (const ch of value) {
    const cp = ch.codePointAt(0) ?? 0;
    out += cp >= 0x41 && cp <= 0x5a ? String.fromCodePoint(cp + 0x20) : ch;
  }
  return out;
}

export function canonicalizeKey(value) {
  if (typeof value !== "string" || value.length === 0) return null;
  if (!isAscii(value)) return null;
  const kept = [...value].filter((ch) => /[A-Za-z0-9]/u.test(ch)).join("");
  if (kept.length === 0) return null;
  return asciiLower(kept);
}

const FORBIDDEN_CANONICAL = new Set(
  FORBIDDEN_SOURCE_SPELLINGS.map((spelling) => canonicalizeKey(spelling)).filter(
    Boolean,
  ),
);

const ALLOWLIST_CANONICAL = new Set(
  ALLOWLISTED_KEYS.map((key) => canonicalizeKey(key)).filter(Boolean),
);

export function isAllowlistedKey(value) {
  const canonical = canonicalizeKey(value);
  return canonical !== null && ALLOWLIST_CANONICAL.has(canonical);
}

export function isForbiddenKey(value) {
  if (isAllowlistedKey(value)) return false;
  const canonical = canonicalizeKey(value);
  return canonical !== null && FORBIDDEN_CANONICAL.has(canonical);
}

function finding(level, code, extra = {}) {
  const item = { level, code };
  if (extra.path !== undefined) item.path = extra.path;
  if (extra.key !== undefined) item.key = extra.key;
  if (extra.canonical !== undefined) item.canonical = extra.canonical;
  if (extra.entry !== undefined) item.entry = extra.entry;
  if (extra.claim !== undefined) item.claim = extra.claim;
  return item;
}

function posixRelative(from, to) {
  return path.relative(from, to).split(path.sep).join("/");
}

function safeRelative(root, absolute) {
  const relative = posixRelative(root, absolute);
  if (!relative || relative.startsWith("..") || path.isAbsolute(relative)) {
    return path.basename(absolute);
  }
  return relative;
}

function readText(absolute) {
  return fs.readFileSync(absolute, "utf8").replace(/\r\n/g, "\n");
}

function sha256Text(text) {
  return createHash("sha256").update(text, "utf8").digest("hex");
}

function extractJsonKeys(value, keys) {
  if (Array.isArray(value)) {
    for (const item of value) extractJsonKeys(item, keys);
    return;
  }
  if (value === null || typeof value !== "object") return;
  for (const [key, child] of Object.entries(value)) {
    keys.push(key);
    extractJsonKeys(child, keys);
  }
}

function extractIdentKeys(text) {
  const keys = [];
  IDENT_KEY_RE.lastIndex = 0;
  let match;
  while ((match = IDENT_KEY_RE.exec(text)) !== null) {
    keys.push(match[1]);
  }
  QUOTED_KEY_RE.lastIndex = 0;
  while ((match = QUOTED_KEY_RE.exec(text)) !== null) {
    keys.push(match[2]);
  }
  return keys;
}

export function extractObjectKeys(text, { json = false } = {}) {
  const keys = [];
  if (json) {
    try {
      extractJsonKeys(JSON.parse(text), keys);
      return keys;
    } catch {
      // Fall through to ident:/quoted-key extraction for invalid JSON.
    }
  }
  keys.push(...extractIdentKeys(text));
  return keys;
}

function walkSchemaFiles(root) {
  if (!fs.existsSync(root)) return [];
  const stat = fs.statSync(root);
  if (stat.isFile()) {
    return SCHEMA_EXTENSIONS.has(path.extname(root)) ? [path.resolve(root)] : [];
  }
  if (!stat.isDirectory()) return [];
  const files = [];
  const stack = [path.resolve(root)];
  while (stack.length > 0) {
    const current = stack.pop();
    let entries;
    try {
      entries = fs.readdirSync(current, { withFileTypes: true });
    } catch {
      continue;
    }
    for (const entry of entries) {
      if (SKIP_DIR_NAMES.has(entry.name)) continue;
      const absolute = path.join(current, entry.name);
      if (entry.isDirectory()) {
        stack.push(absolute);
        continue;
      }
      if (entry.isFile() && SCHEMA_EXTENSIONS.has(path.extname(entry.name))) {
        files.push(absolute);
      }
    }
  }
  return files.sort((a, b) => a.localeCompare(b));
}

function collectSchemaTargets(options) {
  const root = path.resolve(options.root ?? process.cwd());
  const seen = new Set();
  const files = [];
  for (const absolute of walkSchemaFiles(root)) {
    if (seen.has(absolute)) continue;
    seen.add(absolute);
    files.push(absolute);
  }
  for (const extra of options.schemaFiles ?? []) {
    const absolute = path.resolve(extra);
    if (seen.has(absolute)) continue;
    seen.add(absolute);
    files.push(absolute);
  }
  return { root, files };
}

export function scanContractSchema(options = {}) {
  const findings = [];
  const { root, files } = collectSchemaTargets(options);
  for (const absolute of files) {
    const relative = safeRelative(root, absolute);
    let text;
    try {
      text = readText(absolute);
    } catch {
      findings.push(
        finding("contract_schema", "schema_file_unreadable", { path: relative }),
      );
      continue;
    }
    const json = path.extname(absolute) === ".json";
    const keys = extractObjectKeys(text, { json });
    const seen = new Set();
    for (const key of keys) {
      if (seen.has(key)) continue;
      seen.add(key);
      if (!isForbiddenKey(key)) continue;
      findings.push(
        finding("contract_schema", "forbidden_semantic_key", {
          path: relative,
          key,
          canonical: canonicalizeKey(key),
        }),
      );
    }
  }
  return {
    status: findings.length === 0 ? "PASS" : "FAIL",
    findings,
  };
}

function assertReadableEntry(absolute, kind) {
  try {
    const stat = fs.statSync(absolute);
    if (kind === "file" && !stat.isFile()) return "required_entry_absent";
    if (kind === "dir" && !stat.isDirectory()) return "required_entry_absent";
    fs.accessSync(absolute, fs.constants.R_OK);
    if (kind === "file") fs.readFileSync(absolute, { flag: "r" });
    if (kind === "dir") fs.readdirSync(absolute);
    return null;
  } catch (error) {
    if (error && (error.code === "ENOENT" || error.code === "ENOTDIR")) {
      return "required_entry_absent";
    }
    return "required_entry_unreadable";
  }
}

export function scanCodexFeatureRuntime(options = {}) {
  const findings = [];
  const runtimeRoot = options.runtimeRoot;
  if (typeof runtimeRoot !== "string" || runtimeRoot.length === 0) {
    findings.push(
      finding("codex_feature_runtime", "runtime_root_absent", {
        entry: "runtime-root",
      }),
    );
    return {
      status: "FAIL",
      findings,
      requiredRuntimeEntries: [...REQUIRED_RUNTIME_ENTRIES],
    };
  }
  const resolved = path.resolve(runtimeRoot);
  if (!fs.existsSync(resolved)) {
    findings.push(
      finding("codex_feature_runtime", "runtime_root_absent", {
        path: resolved,
        entry: "runtime-root",
      }),
    );
    return {
      status: "FAIL",
      findings,
      requiredRuntimeEntries: [...REQUIRED_RUNTIME_ENTRIES],
    };
  }
  try {
    const stat = fs.statSync(resolved);
    if (!stat.isDirectory()) {
      findings.push(
        finding("codex_feature_runtime", "runtime_root_absent", {
          entry: "runtime-root",
        }),
      );
    } else {
      fs.accessSync(resolved, fs.constants.R_OK);
      fs.readdirSync(resolved);
    }
  } catch {
    findings.push(
      finding("codex_feature_runtime", "runtime_root_unreadable", {
        entry: "runtime-root",
      }),
    );
    return {
      status: "FAIL",
      findings,
      requiredRuntimeEntries: [...REQUIRED_RUNTIME_ENTRIES],
    };
  }

  const required = [
    ["state.json", path.join(resolved, "state.json"), "file"],
    ["journal", path.join(resolved, "journal"), "dir"],
    ["audit", path.join(resolved, "audit"), "dir"],
  ];
  for (const [entry, absolute, kind] of required) {
    const code = assertReadableEntry(absolute, kind);
    if (code) {
      findings.push(
        finding("codex_feature_runtime", code, { entry, path: entry }),
      );
    }
  }

  return {
    status: findings.length === 0 ? "PASS" : "FAIL",
    findings,
    requiredRuntimeEntries: [...REQUIRED_RUNTIME_ENTRIES],
  };
}

/**
 * Write a random canary to `sink`, read it back, then delete it.
 * Assert-then-clean: the sink is never allowlisted.
 * The canary value is not returned so callers cannot print it.
 */
export function plantAndAssertCanary(sink) {
  if (typeof sink !== "string" || sink.length === 0) {
    throw new Error("plantAndAssertCanary requires a sink path");
  }
  const resolved = path.resolve(sink);
  let target = resolved;
  if (fs.existsSync(resolved) && fs.statSync(resolved).isDirectory()) {
    target = path.join(resolved, ".secret-surface-canary");
  } else {
    fs.mkdirSync(path.dirname(resolved), { recursive: true });
  }
  const canary = `canary_${randomBytes(16).toString("hex")}`;
  fs.writeFileSync(target, canary, { encoding: "utf8", flag: "w" });
  let found = false;
  try {
    const readBack = fs.readFileSync(target, "utf8");
    found = readBack === canary;
    if (!found) {
      throw new Error("canary assert failed: planted value was not read back");
    }
  } finally {
    fs.rmSync(target, { force: true });
  }
  const cleaned = !fs.existsSync(target);
  if (!cleaned) {
    throw new Error("canary assert failed: sink was not cleaned");
  }
  return { planted: true, found, cleaned };
}

function loadBaseline(baselinePath) {
  try {
    return {
      ok: true,
      value: JSON.parse(readText(baselinePath)),
    };
  } catch {
    return { ok: false, value: null };
  }
}

function adjacentDebtEntry(baseline) {
  const rows = Array.isArray(baseline?.adjacentDebt) ? baseline.adjacentDebt : [];
  return rows.find((row) => row && row.id === REQUIRED_ADJACENT_DEBT_ID) ?? null;
}

function domainsMatch(domains) {
  if (!Array.isArray(domains)) return false;
  return REQUIRED_ADJACENT_DEBT_DOMAINS.every((name) => domains.includes(name));
}

export function scanRepositoryStaticInventory(options = {}) {
  const findings = [];
  const root = path.resolve(options.root ?? process.cwd());
  const baselinePath = path.resolve(
    options.baseline ?? path.join(root, DEFAULT_BASELINE_RELATIVE),
  );
  const loaded = loadBaseline(baselinePath);
  if (!loaded.ok || !loaded.value || typeof loaded.value !== "object") {
    findings.push(
      finding("repository_static_inventory", "baseline_unreadable", {
        path: path.basename(baselinePath),
      }),
    );
    return {
      status: "FAIL",
      findings,
      adjacentDebt: [],
    };
  }
  const baseline = loaded.value;
  const debt = adjacentDebtEntry(baseline);
  if (!debt) {
    findings.push(
      finding("repository_static_inventory", "adjacent_debt_missing", {
        key: REQUIRED_ADJACENT_DEBT_ID,
      }),
    );
  } else {
    if (!domainsMatch(debt.domains)) {
      findings.push(
        finding("repository_static_inventory", "adjacent_debt_domains", {
          key: REQUIRED_ADJACENT_DEBT_ID,
        }),
      );
    }
    if (debt.waiver === true) {
      findings.push(
        finding("repository_static_inventory", "adjacent_debt_waiver", {
          key: REQUIRED_ADJACENT_DEBT_ID,
        }),
      );
    }
  }

  const baselinedFiles = Array.isArray(baseline.baselinedFiles)
    ? baseline.baselinedFiles.filter((value) => typeof value === "string")
    : [];
  const fileDigests =
    baseline.fileDigests && typeof baseline.fileDigests === "object"
      ? baseline.fileDigests
      : {};

  for (const relative of baselinedFiles) {
    const absolute = path.join(root, ...relative.split("/"));
    let text;
    try {
      text = readText(absolute);
    } catch {
      findings.push(
        finding("repository_static_inventory", "baselined_file_unreadable", {
          path: relative,
        }),
      );
      continue;
    }
    const digest = sha256Text(text);
    const expected = fileDigests[relative];
    if (typeof expected === "string" && expected === digest) {
      continue;
    }
    const json = path.extname(absolute) === ".json";
    const keys = extractObjectKeys(text, { json });
    const seen = new Set();
    for (const key of keys) {
      if (seen.has(key) || !isForbiddenKey(key)) continue;
      seen.add(key);
      findings.push(
        finding("repository_static_inventory", "new_forbidden_literal", {
          path: relative,
          key,
          canonical: canonicalizeKey(key),
        }),
      );
    }
  }

  const visibleDebt = debt
    ? [
        {
          id: debt.id,
          level: debt.level ?? "repository_static_inventory",
          domains: Array.isArray(debt.domains) ? [...debt.domains] : [],
          waiver: debt.waiver === true,
        },
      ]
    : [];

  return {
    status: findings.length === 0 ? "PASS" : "FAIL",
    findings,
    adjacentDebt: visibleDebt,
  };
}

export function scanRepositoryRuntimeGlobal(options = {}) {
  const claim = options.claim;
  const findings = [];
  if (claim !== undefined && claim !== null && claim !== "" && claim !== "NOT_CLAIMED") {
    findings.push(
      finding("repository_runtime_global", "illegal_global_claim", { claim }),
    );
  }
  return {
    status: "NOT_CLAIMED",
    findings,
  };
}

export function parseArgs(argv) {
  const options = {
    root: process.cwd(),
    baseline: null,
    runtimeRoot: null,
    claim: undefined,
    schemaFiles: [],
    json: false,
  };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    const next = () => argv[++index];
    if (arg === "--root") options.root = path.resolve(next() ?? "");
    else if (arg === "--baseline") options.baseline = path.resolve(next() ?? "");
    else if (arg === "--runtime-root") options.runtimeRoot = path.resolve(next() ?? "");
    else if (arg === "--claim") options.claim = next() ?? "";
    else if (arg === "--schema-file") options.schemaFiles.push(path.resolve(next() ?? ""));
    else if (arg === "--json") options.json = true;
    else if (arg.startsWith("-")) {
      throw new Error(`Unknown flag: ${arg}`);
    }
  }
  if (!options.baseline) {
    options.baseline = path.join(options.root, DEFAULT_BASELINE_RELATIVE);
  }
  return options;
}

export function scan(options = {}) {
  const contract = scanContractSchema(options);
  const runtime = scanCodexFeatureRuntime(options);
  const inventory = scanRepositoryStaticInventory(options);
  const global = scanRepositoryRuntimeGlobal(options);
  const findings = [
    ...contract.findings,
    ...runtime.findings,
    ...inventory.findings,
    ...global.findings,
  ];
  return {
    schemaVersion: SCHEMA_VERSION,
    scannerVersion: SCANNER_VERSION,
    levels: {
      contract_schema: contract.status,
      codex_feature_runtime: runtime.status,
      repository_static_inventory: inventory.status,
      repository_runtime_global: global.status,
    },
    findings,
    requiredRuntimeEntries: [...REQUIRED_RUNTIME_ENTRIES],
    adjacentDebt: inventory.adjacentDebt,
  };
}

function isMain(importMetaUrl) {
  if (!process.argv[1]) return false;
  return pathToFileURL(path.resolve(process.argv[1])).href === importMetaUrl;
}

function main() {
  const options = parseArgs(process.argv.slice(2));
  const report = scan(options);
  process.stdout.write(`${JSON.stringify(report, null, 2)}\n`);
  if (report.findings.length > 0) process.exitCode = 1;
}

if (isMain(import.meta.url)) {
  try {
    main();
  } catch (error) {
    process.stderr.write(
      `${error instanceof Error ? error.message : String(error)}\n`,
    );
    process.exitCode = 1;
  }
}
