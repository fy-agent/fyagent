#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import process from "node:process";
import { pathToFileURL } from "node:url";

export const CHANGE_DOMAINS = Object.freeze([
  "contracts",
  "frontend",
  "desktop",
  "backend",
  "windowsNative",
  "docsSpec",
]);

const EMPTY_DOMAINS = Object.freeze(
  Object.fromEntries(CHANGE_DOMAINS.map((domain) => [domain, false])),
);

const ALL_DOMAINS = Object.freeze(
  Object.fromEntries(CHANGE_DOMAINS.map((domain) => [domain, true])),
);

const CONTROL_PLANE_PREFIXES = Object.freeze([
  ".agents/",
  ".codex/",
  ".github/",
  ".mise/",
  ".trellis/agents/",
  ".trellis/scripts/",
  "scripts/ci/",
  "scripts/release/",
  "scripts/tasks/",
  "scripts/trellis/",
]);

const CONTROL_PLANE_FILES = new Set([
  ".node-version",
  ".python-version",
  ".trellis/.gitignore",
  ".trellis/.template-hashes.json",
  ".trellis/.version",
  ".trellis/config.yaml",
  ".trellis/workflow.md",
  "AGENTS.md",
  "mise.lock",
  "mise.toml",
  "pyproject.toml",
  "rust-toolchain.toml",
  "scripts/generate-download-manifest.mjs",
  "scripts/version.mjs",
  "uv.lock",
]);

const RELEASE_AND_CI_CONTRACT_TEST =
  /^tests\/(?:ci|classifyChanges|githubWorkflow|localBuildBoundary|miseTaskContract|requiredCiGate|release|systemCheck|taskDocs|trellisOverlay|version|windowsSigningAdapter|writePlatformMetadata|downloadManifest)/u;

const WINDOWS_NATIVE_TEST =
  /^tests\/(?:codexDesktopDtoContract|codexWindowsUserScopeContract|desktopSecurityBoundary|windowsNsisContract|fixtures\/windows-nsis)/u;

const FRONTEND_TEST_PREFIXES = Object.freeze([
  "tests/components/",
  "tests/config/",
  "tests/hooks/",
  "tests/integration/",
  "tests/lib/",
  "tests/msw/",
  "tests/utils/",
]);

const FRONTEND_ROOT_FILES = new Set([
  "components.json",
  "deplink.html",
  "postcss.config.cjs",
  "tailwind.config.cjs",
  "tsconfig.json",
  "tsconfig.node.json",
  "vite.config.ts",
  "vitest.config.ts",
]);

const DOCUMENTATION_ROOT_FILES = new Set([
  "CHANGELOG.md",
  "CODE_OF_CONDUCT.md",
  "COMMERCIAL-LICENSE.md",
  "CONTRIBUTING.md",
  "LICENSE",
  "LICENSING.md",
  "MEMORY.md",
  "README.md",
  "README_JA.md",
  "README_ZH.md",
  "SECURITY.md",
  "SUPPORT.md",
  "THIRD_PARTY_NOTICES.md",
  "docs/fyagent/history/session-manager-prd.md",
]);

const CODEX_WINDOWS_PREFIXES = Object.freeze([
  "src-tauri/src/codex_desktop/",
  "src-tauri/src/platform/windows/",
  "src-tauri/src/services/codex_desktop/",
  "src-tauri/src/windows_runtime/",
  "src-tauri/tests/fixtures/codex_desktop/",
]);

const CODEX_WINDOWS_FILES = new Set([
  "src-tauri/src/codex_desktop_runtime.rs",
  "src-tauri/src/commands/codex_desktop.rs",
  "src-tauri/tests/codex_desktop_domain.rs",
]);

function hasPrefix(path, prefixes) {
  return prefixes.some((prefix) => path.startsWith(prefix));
}

function addDomains(target, domains) {
  for (const domain of domains) target[domain] = true;
}

function isRepositoryPath(path) {
  if (
    typeof path !== "string" ||
    path.length === 0 ||
    path.includes("\0") ||
    path.includes("\\") ||
    path.startsWith("/")
  ) {
    return false;
  }
  const segments = path.split("/");
  return segments.every(
    (segment) => segment.length > 0 && segment !== "." && segment !== "..",
  );
}

/**
 * The repository's path ownership authority. Keep policy about GitHub event
 * types in the workflow; this function classifies paths only.
 */
function classifyPath(path, domains) {
  if (
    CONTROL_PLANE_FILES.has(path) ||
    hasPrefix(path, CONTROL_PLANE_PREFIXES) ||
    RELEASE_AND_CI_CONTRACT_TEST.test(path)
  ) {
    addDomains(domains, CHANGE_DOMAINS);
    return { matched: true, forceFull: true };
  }

  if (
    path.startsWith(".trellis/spec/") ||
    path.startsWith(".trellis/tasks/") ||
    path.startsWith(".trellis/workspace/")
  ) {
    addDomains(domains, ["contracts", "docsSpec"]);
    return { matched: true, forceFull: false };
  }

  if (path === ".gitattributes" || path === ".gitignore") {
    addDomains(domains, ["contracts"]);
    return { matched: true, forceFull: false };
  }

  if (
    path === "package.json" ||
    path === "pnpm-lock.yaml" ||
    path === "pnpm-workspace.yaml"
  ) {
    addDomains(domains, ["contracts", "frontend", "desktop"]);
    return { matched: true, forceFull: false };
  }

  if (path === "src-tauri/Cargo.toml" || path === "src-tauri/Cargo.lock") {
    addDomains(domains, ["contracts", "backend", "windowsNative"]);
    return { matched: true, forceFull: false };
  }

  if (
    path === "src-tauri/tauri.windows.conf.json" ||
    path.startsWith("src-tauri/nsis/") ||
    path.startsWith("src-tauri/windows/")
  ) {
    addDomains(domains, ["contracts", "windowsNative"]);
    return { matched: true, forceFull: false };
  }

  if (
    CODEX_WINDOWS_FILES.has(path) ||
    hasPrefix(path, CODEX_WINDOWS_PREFIXES)
  ) {
    addDomains(domains, ["contracts", "backend", "windowsNative"]);
    return { matched: true, forceFull: false };
  }

  if (path.startsWith("src-tauri/")) {
    addDomains(domains, ["contracts", "backend"]);
    return { matched: true, forceFull: false };
  }

  if (path.startsWith("src/")) {
    addDomains(domains, ["frontend"]);
    return { matched: true, forceFull: false };
  }

  if (FRONTEND_ROOT_FILES.has(path)) {
    addDomains(domains, ["contracts", "frontend"]);
    return { matched: true, forceFull: false };
  }

  if (path.startsWith("tests/desktop-acceptance/")) {
    addDomains(domains, ["contracts", "desktop"]);
    return { matched: true, forceFull: false };
  }

  if (path.startsWith("scripts/desktop-acceptance/")) {
    addDomains(domains, ["contracts", "desktop"]);
    return { matched: true, forceFull: false };
  }

  if (path.startsWith("tests/e2e/")) {
    addDomains(domains, ["frontend", "desktop"]);
    return { matched: true, forceFull: false };
  }

  if (WINDOWS_NATIVE_TEST.test(path)) {
    addDomains(domains, ["contracts", "windowsNative"]);
    return { matched: true, forceFull: false };
  }

  if (hasPrefix(path, FRONTEND_TEST_PREFIXES)) {
    addDomains(domains, ["frontend"]);
    return { matched: true, forceFull: false };
  }

  if (path.startsWith("tests/")) {
    addDomains(domains, ["contracts"]);
    return { matched: true, forceFull: false };
  }

  if (
    path.startsWith("docs/") ||
    path.startsWith("LICENSES/") ||
    path.startsWith("memory/") ||
    path.startsWith(".omo/")
  ) {
    addDomains(domains, ["contracts", "docsSpec"]);
    return { matched: true, forceFull: false };
  }

  if (DOCUMENTATION_ROOT_FILES.has(path)) {
    addDomains(domains, ["contracts", "docsSpec"]);
    return { matched: true, forceFull: false };
  }

  if (path.startsWith("flatpak/")) {
    addDomains(domains, ["backend", "docsSpec"]);
    return { matched: true, forceFull: false };
  }

  if (path.startsWith("assets/")) {
    addDomains(domains, ["frontend", "backend", "docsSpec"]);
    return { matched: true, forceFull: false };
  }

  return { matched: false, forceFull: false };
}

export function classifyChangedPaths(paths) {
  if (!Array.isArray(paths)) {
    throw new TypeError("changed paths must be an array");
  }

  const domains = { ...EMPTY_DOMAINS };
  const uniquePaths = [...new Set(paths)].sort();
  const unknownPaths = [];
  let forceFull = false;

  for (const path of uniquePaths) {
    if (!isRepositoryPath(path)) {
      unknownPaths.push(String(path));
      continue;
    }
    const classification = classifyPath(path, domains);
    if (!classification.matched) unknownPaths.push(path);
    if (classification.forceFull) forceFull = true;
  }

  return {
    domains: forceFull ? { ...ALL_DOMAINS } : domains,
    unknownPaths,
    forceFull,
  };
}

export function parseNameStatusZ(output) {
  if (typeof output !== "string") {
    throw new TypeError("git name-status output must be a string");
  }
  if (output.length === 0) return [];

  const fields = output.split("\0");
  if (fields.at(-1) !== "") {
    throw new Error("git name-status output is missing its final NUL byte");
  }
  fields.pop();

  const paths = [];
  for (let index = 0; index < fields.length; ) {
    const status = fields[index++];
    if (!/^[ACDMRTUXB][0-9]*$/u.test(status)) {
      throw new Error(`unexpected git diff status: ${status || "<empty>"}`);
    }
    const pathCount = /^[RC]/u.test(status) ? 2 : 1;
    if (index + pathCount > fields.length) {
      throw new Error(`git diff status ${status} is missing a path`);
    }
    for (let offset = 0; offset < pathCount; offset += 1) {
      paths.push(fields[index++]);
    }
  }
  return paths;
}

function runGit(args, cwd) {
  const result = spawnSync("git", args, {
    cwd,
    encoding: "utf8",
    windowsHide: true,
  });
  if (result.error) {
    throw new Error(`failed to execute git: ${result.error.message}`);
  }
  if (result.status !== 0) {
    const detail = result.stderr.trim() || `exit status ${result.status}`;
    throw new Error(`git ${args[0]} failed: ${detail}`);
  }
  return result.stdout;
}

function assertCommit(sha, name, cwd) {
  if (typeof sha !== "string" || !/^[0-9a-fA-F]{40}$/u.test(sha)) {
    throw new Error(
      `${name} must be a full 40-character hexadecimal commit SHA`,
    );
  }
  const objectType = runGit(["cat-file", "-t", sha], cwd).trim();
  if (objectType !== "commit") {
    throw new Error(`${name} does not identify a commit object`);
  }
}

export function changedPathsBetweenCommits(base, head, cwd = process.cwd()) {
  assertCommit(base, "--base", cwd);
  assertCommit(head, "--head", cwd);
  const output = runGit(
    [
      "diff",
      "--name-status",
      "-z",
      "--find-renames=50%",
      "--no-ext-diff",
      "--no-textconv",
      base,
      head,
      "--",
    ],
    cwd,
  );
  return parseNameStatusZ(output);
}

function parseArguments(argv) {
  const values = new Map();
  let json = false;

  for (let index = 0; index < argv.length; index += 1) {
    const argument = argv[index];
    if (argument === "--json") {
      if (json) throw new Error("--json may be provided only once");
      json = true;
      continue;
    }
    if (argument !== "--base" && argument !== "--head") {
      throw new Error(`unknown argument: ${argument}`);
    }
    if (values.has(argument)) {
      throw new Error(`${argument} may be provided only once`);
    }
    const value = argv[index + 1];
    if (!value || value.startsWith("-")) {
      throw new Error(`${argument} requires a commit SHA value`);
    }
    values.set(argument, value);
    index += 1;
  }

  if (!json) throw new Error("--json is required");
  if (!values.has("--base")) throw new Error("--base is required");
  if (!values.has("--head")) throw new Error("--head is required");
  return { base: values.get("--base"), head: values.get("--head") };
}

export function runChangeClassifierCli(
  argv = process.argv.slice(2),
  cwd = process.cwd(),
) {
  try {
    const { base, head } = parseArguments(argv);
    const report = classifyChangedPaths(
      changedPathsBetweenCommits(base, head, cwd),
    );
    console.log(JSON.stringify(report));
    if (report.unknownPaths.length > 0) {
      console.error(
        `Unclassified repository paths: ${report.unknownPaths.join(", ")}`,
      );
      process.exitCode = 1;
    }
    return report;
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
    return null;
  }
}

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(process.argv[1]).href
) {
  runChangeClassifierCli();
}
