#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import fs from "node:fs";
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

const GLOBAL_TOOLING_PREFIXES = Object.freeze([".mise/"]);

const DOCUMENTATION_CONTROL_PREFIXES = Object.freeze([
  ".agents/",
  ".codebuddy/",
  ".codex/",
  ".cursor/",
  ".trellis/agents/",
  ".trellis/scripts/",
  "scripts/audit/",
]);

const RELEASE_CONTROL_PREFIXES = Object.freeze(["scripts/release/"]);

const GLOBAL_TOOLING_FILES = new Set([
  ".node-version",
  ".python-version",
  "mise.lock",
  "mise.toml",
  "pyproject.toml",
  "rust-toolchain.toml",
  "uv.lock",
]);

const CI_AUTHORITY_FILES = new Set([
  ".github/workflows/ci.yml",
  "scripts/ci/classify-changes.d.mts",
  "scripts/ci/classify-changes.mjs",
  "scripts/ci/evaluate-step-outcomes.mjs",
  "scripts/ci/required-gate.mjs",
  "scripts/ci/verify-toolchain.mjs",
]);

const CI_POLICY_FILES = new Set([
  ".github/workflows/commit-convention-push.yml",
  "scripts/ci/verify-commit-messages.mjs",
]);

const RELEASE_CONTROL_FILES = new Set([
  ".github/workflows/release.yml",
  "scripts/generate-download-manifest.mjs",
  "scripts/tasks/release-check.mjs",
  "scripts/version.mjs",
]);

const GITHUB_CONTRACT_FILES = new Set([
  ".github/labeler.yml",
  ".github/workflows/labeler.yml",
  ".github/workflows/star-history.yml",
]);

const GITHUB_DOCUMENTATION_FILES = new Set([
  ".github/CODEOWNERS",
  ".github/FUNDING.yml",
  ".github/dependabot.yml",
  ".github/pull_request_template.md",
]);

const TRELLIS_CONTROL_FILES = new Set([
  ".trellis/.gitignore",
  ".trellis/.template-hashes.json",
  ".trellis/.version",
  ".trellis/config.yaml",
  ".trellis/workflow.md",
  "AGENTS.md",
]);

const TASK_CONTRACT_FILES = new Set([
  "scripts/tasks/clean.mjs",
  "scripts/tasks/dep0040-check.mjs",
  "scripts/tasks/format-files.mjs",
  "scripts/tasks/lockfile-check.mjs",
  "scripts/tasks/maintenance.mjs",
  "scripts/tasks/python.mjs",
  "scripts/tasks/system-check.mjs",
  "scripts/tasks/task-contract-check.mjs",
  "scripts/tasks/upstream.mjs",
]);

const TASK_DOCUMENTATION_FILES = new Set([
  "scripts/tasks/docs-contract-check.mjs",
  "scripts/tasks/prearchive-check.mjs",
  "scripts/tasks/task-docs.mjs",
]);

const TASK_FRONTEND_FILES = new Set([
  "scripts/tasks/frontend.d.mts",
  "scripts/tasks/frontend.mjs",
]);

const TASK_BACKEND_FILES = new Set([
  "scripts/tasks/rust.mjs",
  "scripts/tasks/windows-msvc-env.mjs",
]);

const TASK_HOST_NATIVE_FILES = new Set([
  "scripts/tasks/host-native.mjs",
  "scripts/tasks/macos-signed-dev-cargo.mjs",
  "scripts/tasks/macos-signed-dev.mjs",
]);

const TASK_GLOBAL_AUTHORITY_FILES = new Set([
  "scripts/tasks/lib.mjs",
  "scripts/tasks/platform.mjs",
  "scripts/tasks/supported-platform-check.mjs",
  "scripts/tasks/toolchain-check.mjs",
]);

const TASK_PLATFORM_INVENTORY_FILES = new Set([
  "scripts/tasks/supported-platform-raster-assets.json",
  "scripts/tasks/supported-platform-structure-assets.json",
]);

const RELEASE_AND_CI_CONTRACT_TEST =
  /^tests\/(?:ci|classifyChanges|githubWorkflow|localBuildBoundary|miseTaskContract|requiredCiGate|release|systemCheck|taskDocs|verifyCommitMessages|version|windowsSigningAdapter|writePlatformMetadata|downloadManifest)/u;

const WINDOWS_NATIVE_TEST =
  /^tests\/(?:codexDesktopDtoContract|codexUserHelperContract|codexWindowsUserScopeContract|desktopSecurityBoundary|windowsNsisContract|fixtures\/windows-nsis)/u;

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
  "eslint.v2.config.mjs",
  "playwright.v2.config.ts",
  "postcss.config.cjs",
  "scripts/build-v2-preview.mjs",
  "scripts/verify-v2-route-chunks.d.mts",
  "scripts/verify-v2-route-chunks.mjs",
  "tailwind.config.cjs",
  "tsconfig.json",
  "tsconfig.node.json",
  "tsconfig.v2.json",
  "vite.config.ts",
  "vitest.config.ts",
  "vitest.v2.config.ts",
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
  "README_EN.md",
  "README_JA.md",
  "SECURITY.md",
  "SUPPORT.md",
  "THIRD_PARTY_NOTICES.md",
  "docs/fyagent/history/session-manager-prd.md",
]);

// Name-status diffs include deleted paths and the old side of renames. Keep
// retired root-document names owned so history comparisons remain classifiable.
const LEGACY_DOCUMENTATION_ROOT_FILES = new Set([
  "README_DE.md",
  "README_ZH.md",
  "session-manager.md",
]);

// Name-status diffs include deleted paths. Keep the retired generated
// standalone preview owned so untracking it is not an unknown path.
const LEGACY_FRONTEND_ROOT_FILES = new Set(["FyAgent-前端交互预览.html"]);

// These trees are gone from this branch but still exist on older main
// history. Name-status diffs include the deleted side, so they must stay
// owned or PR classification against main fails closed.
const RETIRED_SESSION_MEMORY_PREFIXES = Object.freeze(["memory/", ".omo/"]);
const RETIRED_SANDBOX_PACKAGE_PREFIX = ["flat", "pak/"].join("");

const CODEX_WINDOWS_PREFIXES = Object.freeze([
  "src-tauri/src/codex_desktop/",
  "src-tauri/src/platform/windows/",
  "src-tauri/src/services/codex_desktop/",
  "src-tauri/src/windows_runtime/",
  "src-tauri/tests/fixtures/codex_desktop/",
  "src-tauri/user-helper/",
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

function matchDomains(domains, enabledDomains, reason, forceFull = false) {
  addDomains(domains, enabledDomains);
  return {
    matched: true,
    forceFull,
    reason,
    domains: [...enabledDomains],
  };
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
    CI_AUTHORITY_FILES.has(path) ||
    GLOBAL_TOOLING_FILES.has(path) ||
    TASK_GLOBAL_AUTHORITY_FILES.has(path) ||
    hasPrefix(path, GLOBAL_TOOLING_PREFIXES)
  ) {
    return matchDomains(
      domains,
      CHANGE_DOMAINS,
      CI_AUTHORITY_FILES.has(path)
        ? "ci-authority"
        : "global-tooling-authority",
      true,
    );
  }

  if (CI_POLICY_FILES.has(path)) {
    return matchDomains(domains, ["contracts"], "ci-policy");
  }

  if (TASK_PLATFORM_INVENTORY_FILES.has(path)) {
    return matchDomains(domains, ["contracts"], "supported-platform-inventory");
  }

  if (
    RELEASE_CONTROL_FILES.has(path) ||
    hasPrefix(path, RELEASE_CONTROL_PREFIXES)
  ) {
    return matchDomains(domains, ["contracts"], "release-authority");
  }

  if (RELEASE_AND_CI_CONTRACT_TEST.test(path)) {
    return matchDomains(domains, ["contracts"], "ci-release-contract-test");
  }

  if (GITHUB_CONTRACT_FILES.has(path)) {
    return matchDomains(domains, ["contracts"], "github-contract");
  }

  if (
    GITHUB_DOCUMENTATION_FILES.has(path) ||
    path.startsWith(".github/ISSUE_TEMPLATE/") ||
    path.startsWith(".github/DISCUSSION_TEMPLATE/")
  ) {
    return matchDomains(
      domains,
      ["contracts", "docsSpec"],
      "github-governance-docs",
    );
  }

  if (
    TRELLIS_CONTROL_FILES.has(path) ||
    hasPrefix(path, DOCUMENTATION_CONTROL_PREFIXES)
  ) {
    return matchDomains(
      domains,
      ["contracts", "docsSpec"],
      "developer-governance",
    );
  }

  if (TASK_CONTRACT_FILES.has(path)) {
    return matchDomains(domains, ["contracts"], "repository-task-contract");
  }

  if (TASK_DOCUMENTATION_FILES.has(path)) {
    return matchDomains(
      domains,
      ["contracts", "docsSpec"],
      "repository-task-docs",
    );
  }

  if (TASK_FRONTEND_FILES.has(path)) {
    return matchDomains(
      domains,
      ["contracts", "frontend"],
      "repository-task-frontend",
    );
  }

  if (TASK_HOST_NATIVE_FILES.has(path)) {
    return matchDomains(
      domains,
      ["contracts", "frontend", "backend", "windowsNative"],
      "repository-task-host-native",
    );
  }

  if (TASK_BACKEND_FILES.has(path)) {
    return matchDomains(
      domains,
      ["contracts", "backend", "windowsNative"],
      "repository-task-backend",
    );
  }

  if (path === "scripts/prepare-windows-user-helper.mjs") {
    return matchDomains(
      domains,
      ["contracts", "backend", "windowsNative"],
      "windows-build-preparation",
    );
  }

  if (
    path.startsWith(".trellis/spec/") ||
    path.startsWith(".trellis/tasks/") ||
    path.startsWith(".trellis/workspace/")
  ) {
    return matchDomains(domains, ["contracts", "docsSpec"], "trellis-content");
  }

  if (path === ".gitattributes" || path === ".gitignore") {
    return matchDomains(domains, ["contracts"], "repository-metadata");
  }

  if (
    path === "package.json" ||
    path === "pnpm-lock.yaml" ||
    path === "pnpm-workspace.yaml"
  ) {
    return matchDomains(
      domains,
      ["contracts", "frontend", "desktop"],
      "frontend-dependency-root",
    );
  }

  if (path === "src-tauri/Cargo.toml" || path === "src-tauri/Cargo.lock") {
    return matchDomains(
      domains,
      ["contracts", "backend", "windowsNative"],
      "cargo-dependency-root",
    );
  }

  if (
    path === "src-tauri/tauri.windows.conf.json" ||
    path.startsWith("src-tauri/nsis/") ||
    path.startsWith("src-tauri/windows/")
  ) {
    return matchDomains(
      domains,
      ["contracts", "windowsNative"],
      "windows-packaging",
    );
  }

  if (
    CODEX_WINDOWS_FILES.has(path) ||
    hasPrefix(path, CODEX_WINDOWS_PREFIXES)
  ) {
    return matchDomains(
      domains,
      ["contracts", "backend", "windowsNative"],
      "windows-backend",
    );
  }

  if (path.startsWith("src-tauri/")) {
    return matchDomains(domains, ["contracts", "backend"], "backend");
  }

  if (path.startsWith("src/")) {
    return matchDomains(domains, ["frontend"], "frontend");
  }

  if (FRONTEND_ROOT_FILES.has(path) || LEGACY_FRONTEND_ROOT_FILES.has(path)) {
    return matchDomains(domains, ["contracts", "frontend"], "frontend-tooling");
  }

  if (path.startsWith("tests/desktop-acceptance/")) {
    return matchDomains(
      domains,
      ["contracts", "desktop"],
      "desktop-acceptance",
    );
  }

  if (path.startsWith("scripts/desktop-acceptance/")) {
    return matchDomains(
      domains,
      ["contracts", "desktop"],
      "desktop-acceptance",
    );
  }

  if (path.startsWith("tests/e2e/")) {
    return matchDomains(domains, ["frontend", "desktop"], "desktop-e2e");
  }

  if (WINDOWS_NATIVE_TEST.test(path)) {
    return matchDomains(
      domains,
      ["contracts", "windowsNative"],
      "windows-native-contract",
    );
  }

  if (hasPrefix(path, FRONTEND_TEST_PREFIXES)) {
    return matchDomains(domains, ["frontend"], "frontend-test");
  }

  if (path.startsWith("tests/")) {
    return matchDomains(domains, ["contracts"], "repository-contract-test");
  }

  if (
    path.startsWith("docs/") ||
    path.startsWith("LICENSES/") ||
    hasPrefix(path, RETIRED_SESSION_MEMORY_PREFIXES)
  ) {
    return matchDomains(domains, ["contracts", "docsSpec"], "documentation");
  }

  if (
    DOCUMENTATION_ROOT_FILES.has(path) ||
    LEGACY_DOCUMENTATION_ROOT_FILES.has(path)
  ) {
    return matchDomains(domains, ["contracts", "docsSpec"], "documentation");
  }

  if (path.startsWith(RETIRED_SANDBOX_PACKAGE_PREFIX)) {
    return matchDomains(
      domains,
      ["contracts", "docsSpec"],
      "retired-documentation",
    );
  }

  if (path.startsWith("assets/")) {
    return matchDomains(
      domains,
      ["frontend", "backend", "docsSpec"],
      "shared-assets",
    );
  }

  return { matched: false, forceFull: false, reason: null, domains: [] };
}

function classifyChangedPathsDetailed(paths) {
  if (!Array.isArray(paths)) {
    throw new TypeError("changed paths must be an array");
  }

  const domains = { ...EMPTY_DOMAINS };
  const uniquePaths = [...new Set(paths)].sort();
  const unknownPaths = [];
  const entries = [];
  let forceFull = false;

  for (const path of uniquePaths) {
    if (!isRepositoryPath(path)) {
      unknownPaths.push(String(path));
      entries.push({
        path: String(path),
        reason: "invalid-or-unsafe-path",
        domains: [],
        forceFull: false,
      });
      continue;
    }
    const classification = classifyPath(path, domains);
    if (!classification.matched) unknownPaths.push(path);
    if (classification.forceFull) forceFull = true;
    entries.push({
      path,
      reason: classification.reason ?? "unclassified",
      domains: classification.domains,
      forceFull: classification.forceFull,
    });
  }

  return {
    report: {
      domains: forceFull ? { ...ALL_DOMAINS } : domains,
      unknownPaths,
      forceFull,
    },
    entries,
  };
}

export function classifyChangedPaths(paths) {
  return classifyChangedPathsDetailed(paths).report;
}

function markdownCode(value) {
  return `\`${String(value).replaceAll("`", "\\`")}\``;
}

function renderClassificationSummary(details) {
  const lines = [
    "### Change classification",
    "",
    "| Path | Owner | Requested domains | Full CI |",
    "| --- | --- | --- | --- |",
  ];

  if (details.entries.length === 0) {
    lines.push("| _(empty comparison)_ | — | — | no |");
  } else {
    for (const entry of details.entries) {
      lines.push(
        `| ${markdownCode(entry.path)} | ${markdownCode(entry.reason)} | ${
          entry.domains.length > 0
            ? entry.domains.map(markdownCode).join(", ")
            : "—"
        } | ${entry.forceFull ? "yes" : "no"} |`,
      );
    }
  }

  const selected = CHANGE_DOMAINS.filter(
    (domain) => details.report.domains[domain],
  );
  const forceReasons = details.entries
    .filter((entry) => entry.forceFull)
    .map((entry) => `${entry.path} (${entry.reason})`);

  lines.push(
    "",
    `- Path-derived forceFull: ${markdownCode(details.report.forceFull)}`,
    `- Selected domains: ${
      selected.length > 0 ? selected.map(markdownCode).join(", ") : "none"
    }`,
    `- Full CI reason: ${
      forceReasons.length > 0
        ? forceReasons.map(markdownCode).join(", ")
        : "none"
    }`,
  );
  if (details.report.unknownPaths.length > 0) {
    lines.push(
      `- Unknown paths: ${details.report.unknownPaths.map(markdownCode).join(", ")}`,
    );
  }
  lines.push("");
  return `${lines.join("\n")}\n`;
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
    if (
      argument !== "--base" &&
      argument !== "--head" &&
      argument !== "--summary-file"
    ) {
      throw new Error(`unknown argument: ${argument}`);
    }
    if (values.has(argument)) {
      throw new Error(`${argument} may be provided only once`);
    }
    const value = argv[index + 1];
    if (!value || value.startsWith("-")) {
      throw new Error(`${argument} requires a value`);
    }
    values.set(argument, value);
    index += 1;
  }

  if (!json) throw new Error("--json is required");
  if (!values.has("--base")) throw new Error("--base is required");
  if (!values.has("--head")) throw new Error("--head is required");
  return {
    base: values.get("--base"),
    head: values.get("--head"),
    summaryFile: values.get("--summary-file") ?? null,
  };
}

export function runChangeClassifierCli(
  argv = process.argv.slice(2),
  cwd = process.cwd(),
) {
  try {
    const { base, head, summaryFile } = parseArguments(argv);
    const details = classifyChangedPathsDetailed(
      changedPathsBetweenCommits(base, head, cwd),
    );
    const report = details.report;
    if (summaryFile) {
      fs.appendFileSync(
        summaryFile,
        renderClassificationSummary(details),
        "utf8",
      );
    }
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
