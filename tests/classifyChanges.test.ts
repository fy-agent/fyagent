import { spawnSync, type SpawnSyncReturns } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { afterAll, describe, expect, it } from "vitest";
import {
  CHANGE_DOMAINS,
  changedPathsBetweenCommits,
  classifyChangedPaths,
  parseNameStatusZ,
  type ChangeClassification,
} from "../scripts/ci/classify-changes.mjs";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const CLASSIFIER = path.join(ROOT, "scripts", "ci", "classify-changes.mjs");
const temporaryRoots: string[] = [];

function domains(...enabled: (typeof CHANGE_DOMAINS)[number][]) {
  return Object.fromEntries(
    CHANGE_DOMAINS.map((domain) => [domain, enabled.includes(domain)]),
  );
}

function git(cwd: string, ...args: string[]): string {
  const result = spawnSync("git", args, { cwd, encoding: "utf8" });
  if (result.status !== 0) {
    throw new Error(result.stderr || `git ${args.join(" ")} failed`);
  }
  return result.stdout.trim();
}

function write(root: string, relativePath: string, contents: string): void {
  const destination = path.join(root, relativePath);
  fs.mkdirSync(path.dirname(destination), { recursive: true });
  fs.writeFileSync(destination, contents);
}

function commit(root: string, message: string): string {
  git(root, "add", "-A");
  git(root, "commit", "-m", message);
  return git(root, "rev-parse", "HEAD");
}

function temporaryRepository(): string {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "fyagent-classifier-"));
  temporaryRoots.push(root);
  git(root, "init", "--quiet");
  git(root, "config", "user.name", "FyAgent Tests");
  git(root, "config", "user.email", "tests@fyagent.invalid");
  return root;
}

function runClassifier(cwd: string, args: string[]): SpawnSyncReturns<string> {
  return spawnSync(process.execPath, [CLASSIFIER, ...args], {
    cwd,
    encoding: "utf8",
  });
}

afterAll(() => {
  for (const root of temporaryRoots) {
    fs.rmSync(root, { recursive: true, force: true });
  }
});

describe("repository change classifier", () => {
  it("uses the stable public domain shape for an empty diff", () => {
    expect(CHANGE_DOMAINS).toEqual([
      "contracts",
      "frontend",
      "desktop",
      "backend",
      "windowsNative",
      "docsSpec",
    ]);
    expect(classifyChangedPaths([])).toEqual({
      domains: domains(),
      unknownPaths: [],
      forceFull: false,
    });
  });

  it.each([
    [
      "docs/spec",
      ["docs/fyagent/development/ci-release/ci.md"],
      domains("contracts", "docsSpec"),
    ],
    ["frontend", ["src/components/App.tsx"], domains("frontend")],
    [
      "V2 renderer tooling",
      [
        "eslint.v2.config.mjs",
        "playwright.v2.config.ts",
        "scripts/build-v2-preview.mjs",
        "scripts/verify-v2-route-chunks.d.mts",
        "scripts/verify-v2-route-chunks.mjs",
        "tsconfig.v2.json",
        "vitest.v2.config.ts",
      ],
      domains("contracts", "frontend"),
    ],
    [
      "retired generated standalone preview",
      ["FyAgent-前端交互预览.html"],
      domains("contracts", "frontend"),
    ],
    [
      "retired session memory",
      ["memory/2026-08-10.md", ".omo/plans/docs-restructure-v0.3.0.md"],
      domains("contracts", "docsSpec"),
    ],
    [
      "retired sandbox packaging",
      [["flat", "pak/com.fyagent.desktop.yml"].join("")],
      domains("contracts", "docsSpec"),
    ],
    [
      "backend",
      ["src-tauri/src/proxy/server.rs"],
      domains("contracts", "backend"),
    ],
    [
      "Windows installer",
      ["src-tauri/nsis/validate-install-dir.nsh"],
      domains("contracts", "windowsNative"),
    ],
    [
      "Codex Windows",
      ["src-tauri/src/codex_desktop/platform/windows.rs"],
      domains("contracts", "backend", "windowsNative"),
    ],
    [
      "Codex Windows user helper",
      ["src-tauri/user-helper/src/protocol.rs"],
      domains("contracts", "backend", "windowsNative"),
    ],
    [
      "Codex Windows user helper contract",
      ["tests/codexUserHelperContract.test.ts"],
      domains("contracts", "windowsNative"),
    ],
    [
      "Cargo dependency root",
      ["src-tauri/Cargo.lock"],
      domains("contracts", "backend", "windowsNative"),
    ],
    [
      "Tauri bundle and capability boundary",
      [
        "src-tauri/tauri.conf.json",
        "src-tauri/build.rs",
        "src-tauri/capabilities/default.json",
      ],
      domains("contracts", "backend"),
    ],
    [
      "pnpm dependency root",
      ["pnpm-lock.yaml"],
      domains("contracts", "frontend", "desktop"),
    ],
    [
      "V2 frontend toolchain roots",
      [
        "eslint.v2.config.mjs",
        "playwright.v2.config.ts",
        "tsconfig.v2.json",
        "vitest.v2.config.ts",
      ],
      domains("contracts", "frontend"),
    ],
  ])("classifies the %s fixture", (_name, paths, expectedDomains) => {
    expect(classifyChangedPaths(paths as string[])).toEqual({
      domains: expectedDomains,
      unknownPaths: [],
      forceFull: false,
    });
  });

  it.each([
    ".github/workflows/ci.yml",
    "scripts/ci/classify-changes.mjs",
    "scripts/ci/required-gate.mjs",
    "scripts/ci/evaluate-step-outcomes.mjs",
    "scripts/ci/verify-toolchain.mjs",
    "scripts/tasks/lib.mjs",
    "scripts/tasks/platform.mjs",
    "scripts/tasks/supported-platform-check.mjs",
    "scripts/tasks/toolchain-check.mjs",
    "rust-toolchain.toml",
  ])("forces every domain for global authority path %s", (changedPath) => {
    expect(classifyChangedPaths([changedPath])).toEqual({
      domains: domains(...CHANGE_DOMAINS),
      unknownPaths: [],
      forceFull: true,
    });
  });

  it.each([
    [
      "release authority",
      [
        ".github/workflows/release.yml",
        "scripts/release/release-contract.mjs",
        "scripts/tasks/release-check.mjs",
        "tests/releaseWorkflow.test.ts",
      ],
      domains("contracts"),
    ],
    [
      "commit policy authority",
      [
        ".github/workflows/commit-convention-push.yml",
        "scripts/ci/verify-commit-messages.mjs",
        "tests/verifyCommitMessages.test.ts",
      ],
      domains("contracts"),
    ],
    [
      "GitHub repository automation",
      [
        ".github/labeler.yml",
        ".github/workflows/labeler.yml",
        ".github/workflows/star-history.yml",
      ],
      domains("contracts"),
    ],
    [
      "developer governance",
      [
        ".codex/hooks.json",
        ".cursor/skills/trellis-check/SKILL.md",
        ".codebuddy/settings.json",
        "scripts/audit/repository-governance-scan.mjs",
      ],
      domains("contracts", "docsSpec"),
    ],
    [
      "frontend repository task",
      ["scripts/tasks/frontend.mjs"],
      domains("contracts", "frontend"),
    ],
    [
      "backend repository task",
      ["scripts/tasks/rust.mjs"],
      domains("contracts", "backend", "windowsNative"),
    ],
    [
      "Windows helper preparation",
      ["scripts/prepare-windows-user-helper.mjs"],
      domains("contracts", "backend", "windowsNative"),
    ],
    [
      "supported-platform digest inventory",
      [
        "scripts/tasks/supported-platform-structure-assets.json",
        "scripts/tasks/supported-platform-raster-assets.json",
      ],
      domains("contracts"),
    ],
  ])("classifies typed control plane: %s", (_name, paths, expectedDomains) => {
    expect(classifyChangedPaths(paths as string[])).toEqual({
      domains: expectedDomains,
      unknownPaths: [],
      forceFull: false,
    });
  });

  it("keeps a PR-151-style release change out of unrelated product domains", () => {
    expect(
      classifyChangedPaths([
        ".github/workflows/release.yml",
        "scripts/release/dev-release-eligibility.mjs",
        "scripts/release/verify-release-draft-ownership.mjs",
        "scripts/tasks/release-check.mjs",
        "scripts/tasks/supported-platform-structure-assets.json",
        "tests/releaseDraftOwnership.test.ts",
        "tests/releaseWorkflow.test.ts",
        ".trellis/spec/backend/github-release-workflow.md",
      ]),
    ).toEqual({
      domains: domains("contracts", "docsSpec"),
      unknownPaths: [],
      forceFull: false,
    });
  });

  it("keeps a PR-147-style frontend change off Full CI", () => {
    expect(
      classifyChangedPaths([
        "src/v2/pages/models/apply/ChangePlanWorkspace.tsx",
        "tests/v2/pages/models/apply/ChangePlanWorkspace.test.tsx",
        ".trellis/spec/frontend/v2-agent-models.md",
      ]),
    ).toEqual({
      domains: domains("contracts", "frontend", "docsSpec"),
      unknownPaths: [],
      forceFull: false,
    });
  });

  it("keeps a PR-148-style frontend and backend product change off Full CI", () => {
    expect(
      classifyChangedPaths([
        "src/v2/pages/models/apply/CodexSavePlanWorkspace.tsx",
        "src-tauri/src/services/change_plan/service.rs",
        "tests/v2/pages/models/apply/CodexSavePlanWorkspace.test.tsx",
        ".trellis/spec/backend/change-plan-executor.md",
      ]),
    ).toEqual({
      domains: domains("contracts", "frontend", "backend", "docsSpec"),
      unknownPaths: [],
      forceFull: false,
    });
  });

  it("keeps a PR-150-style spec-only change on contracts and docs", () => {
    expect(
      classifyChangedPaths([
        ".trellis/spec/backend/github-release-workflow.md",
        ".trellis/spec/frontend/index.md",
      ]),
    ).toEqual({
      domains: domains("contracts", "docsSpec"),
      unknownPaths: [],
      forceFull: false,
    });
  });

  it("keeps a PR-144-style CI authority change on Full CI", () => {
    expect(
      classifyChangedPaths([
        ".github/workflows/ci.yml",
        "scripts/ci/classify-changes.mjs",
        "scripts/ci/required-gate.mjs",
        ".trellis/spec/backend/github-ci-workflow.md",
      ]),
    ).toEqual({
      domains: domains(...CHANGE_DOMAINS),
      unknownPaths: [],
      forceFull: true,
    });
  });

  it("unions release authority with product changes without forcing every domain", () => {
    expect(
      classifyChangedPaths([
        ".github/workflows/release.yml",
        "scripts/release/release-contract.mjs",
        "src/v2/App.tsx",
        "src-tauri/src/proxy/server.rs",
      ]),
    ).toEqual({
      domains: domains("contracts", "frontend", "backend"),
      unknownPaths: [],
      forceFull: false,
    });
  });

  it("unions multiple affected domains without converting them to full CI", () => {
    expect(
      classifyChangedPaths([
        "src/components/App.tsx",
        "src-tauri/src/proxy/server.rs",
        "docs/fyagent/development/ci-release/ci.md",
        "src/components/App.tsx",
      ]),
    ).toEqual({
      domains: domains("contracts", "frontend", "backend", "docsSpec"),
      unknownPaths: [],
      forceFull: false,
    });
  });

  it("reports unknown and unsafe paths deterministically instead of guessing", () => {
    expect(
      classifyChangedPaths([
        "unknown/new-file.txt",
        "../outside.txt",
        "another-unknown.txt",
        "unknown/new-file.txt",
      ]),
    ).toEqual({
      domains: domains(),
      unknownPaths: [
        "../outside.txt",
        "another-unknown.txt",
        "unknown/new-file.txt",
      ],
      forceFull: false,
    });
  });

  it("keeps every currently tracked repository path owned", () => {
    const tracked = git(ROOT, "ls-files", "-z")
      .split("\0")
      .filter(Boolean)
      .filter((file) => fs.existsSync(path.join(ROOT, file)));
    expect(classifyChangedPaths(tracked).unknownPaths).toEqual([]);
  });

  it("parses both sides of rename/copy records and rejects truncated data", () => {
    expect(
      parseNameStatusZ(
        "M\0src/App.tsx\0R100\0docs/old.md\0src/new.ts\0C75\0README.md\0docs/copy.md\0",
      ),
    ).toEqual([
      "src/App.tsx",
      "docs/old.md",
      "src/new.ts",
      "README.md",
      "docs/copy.md",
    ]);
    expect(() => parseNameStatusZ("R100\0docs/old.md\0")).toThrow(
      "is missing a path",
    );
    expect(() => parseNameStatusZ("M\0src/App.tsx")).toThrow(
      "missing its final NUL byte",
    );
  });

  it("classifies a real Git rename from both its old and new path", () => {
    const root = temporaryRepository();
    write(root, "docs/old.md", "same contents\n");
    const base = commit(root, "base");
    fs.mkdirSync(path.join(root, "src"), { recursive: true });
    fs.renameSync(
      path.join(root, "docs", "old.md"),
      path.join(root, "src", "new.ts"),
    );
    const head = commit(root, "rename");

    expect(changedPathsBetweenCommits(base, head, root)).toEqual([
      "docs/old.md",
      "src/new.ts",
    ]);

    const result = runClassifier(root, [
      "--base",
      base,
      "--head",
      head,
      "--json",
    ]);
    expect(result.status).toBe(0);
    expect(JSON.parse(result.stdout) as ChangeClassification).toEqual({
      domains: domains("contracts", "frontend", "docsSpec"),
      unknownPaths: [],
      forceFull: false,
    });
  });

  it("keeps deleted and renamed legacy documentation paths owned", () => {
    const root = temporaryRepository();
    write(root, "README_DE.md", "retired locale readme\n");
    write(root, "session-manager.md", "session manager history\n");
    const base = commit(root, "base");

    fs.rmSync(path.join(root, "README_DE.md"));
    write(
      root,
      "docs/fyagent/history/session-manager-prd.md",
      "session manager history\n",
    );
    fs.rmSync(path.join(root, "session-manager.md"));
    const head = commit(root, "restructure documentation");

    const changedPaths = changedPathsBetweenCommits(base, head, root);
    expect(changedPaths).toHaveLength(3);
    expect(changedPaths).toEqual(
      expect.arrayContaining([
        "README_DE.md",
        "session-manager.md",
        "docs/fyagent/history/session-manager-prd.md",
      ]),
    );

    const result = runClassifier(root, [
      "--base",
      base,
      "--head",
      head,
      "--json",
    ]);
    expect(result.status).toBe(0);
    expect(JSON.parse(result.stdout) as ChangeClassification).toEqual({
      domains: domains("contracts", "docsSpec"),
      unknownPaths: [],
      forceFull: false,
    });
  });

  it("does not broadly classify an arbitrary root Markdown file", () => {
    expect(classifyChangedPaths(["UNOWNED_ROOT_DOC.md"])).toEqual({
      domains: domains(),
      unknownPaths: ["UNOWNED_ROOT_DOC.md"],
      forceFull: false,
    });
  });

  it("emits JSON and exits nonzero for an unclassified Git path", () => {
    const root = temporaryRepository();
    write(root, "README.md", "base\n");
    const base = commit(root, "base");
    write(root, "infra/pipeline.yml", "unknown: true\n");
    const head = commit(root, "unknown path");

    const result = runClassifier(root, [
      "--base",
      base,
      "--head",
      head,
      "--json",
    ]);
    expect(result.status).toBe(1);
    expect(JSON.parse(result.stdout) as ChangeClassification).toEqual({
      domains: domains(),
      unknownPaths: ["infra/pipeline.yml"],
      forceFull: false,
    });
    expect(result.stderr).toContain("Unclassified repository paths");
  });

  it("writes path ownership diagnostics without changing the stable JSON plan", () => {
    const root = temporaryRepository();
    write(root, "README.md", "base\n");
    const base = commit(root, "base");
    write(root, ".github/workflows/release.yml", "name: Release\n");
    const head = commit(root, "release workflow");
    const summary = path.join(root, "classification-summary.md");

    const result = runClassifier(root, [
      "--base",
      base,
      "--head",
      head,
      "--json",
      "--summary-file",
      summary,
    ]);
    expect(result.status).toBe(0);
    expect(JSON.parse(result.stdout) as ChangeClassification).toEqual({
      domains: domains("contracts"),
      unknownPaths: [],
      forceFull: false,
    });
    const prose = fs.readFileSync(summary, "utf8");
    expect(prose).toContain("### Change classification");
    expect(prose).toContain("`release-authority`");
    expect(prose).toContain("Path-derived forceFull: `false`");
    expect(prose).toContain("Selected domains: `contracts`");
    expect(prose).toContain("Full CI reason: none");
  });

  it("fails closed for malformed, injected, missing, and non-commit revisions", () => {
    const root = temporaryRepository();
    write(root, "README.md", "base\n");
    const commitSha = commit(root, "base");
    const missingSha = "f".repeat(40);
    const blobSha = git(root, "hash-object", "-w", "README.md");

    for (const args of [
      ["--base", "HEAD", "--head", commitSha, "--json"],
      ["--base", "--help", "--head", commitSha, "--json"],
      ["--base", missingSha, "--head", commitSha, "--json"],
      ["--base", blobSha, "--head", commitSha, "--json"],
      ["--base", commitSha, "--head", commitSha, "--json", "--evil"],
    ]) {
      const result = runClassifier(root, args);
      expect(result.status, args.join(" ")).toBe(1);
      expect(result.stdout, args.join(" ")).toBe("");
      expect(result.stderr.length, args.join(" ")).toBeGreaterThan(0);
    }
  });

  it("returns the stable empty report when base and head are identical", () => {
    const root = temporaryRepository();
    write(root, "README.md", "base\n");
    const sha = commit(root, "base");
    const result = runClassifier(root, [
      "--base",
      sha,
      "--head",
      sha,
      "--json",
    ]);
    expect(result.status).toBe(0);
    expect(JSON.parse(result.stdout) as ChangeClassification).toEqual({
      domains: domains(),
      unknownPaths: [],
      forceFull: false,
    });
  });
});
