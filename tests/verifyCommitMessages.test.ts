import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { afterAll, describe, expect, it } from "vitest";
// @ts-expect-error The workflow executes this dependency-free JavaScript helper directly.
import * as commitMessages from "../scripts/ci/verify-commit-messages.mjs";

const ROOT = path.resolve(__dirname, "..");
const VERIFY_COMMIT_MESSAGES = path.join(
  ROOT,
  "scripts",
  "ci",
  "verify-commit-messages.mjs",
);
const temporaryRoots: string[] = [];

function git(cwd: string, ...args: string[]): string {
  const result = spawnSync("git", args, { cwd, encoding: "utf8" });
  if (result.status !== 0) {
    throw new Error(result.stderr || `git ${args.join(" ")} failed`);
  }
  return result.stdout.trim();
}

const {
  CONVENTIONAL_COMMIT_TYPES,
  isConventionalCommitSubject,
  stripGithubSquashSuffix,
  validateCommitSubject,
} = commitMessages;

describe("commit message convention", () => {
  it("accepts repository Conventional Commit examples", () => {
    for (const subject of [
      "feat(provider): add support for new provider",
      "fix(tray): resolve menu not updating after switch",
      "docs(readme): update installation instructions",
      "ci: add format check workflow",
      "chore(deps): update dependencies",
      "chore(task): archive 08-21-v2-model-probe",
      "style(v2): format model probe frontend files for Prettier",
    ]) {
      expect(isConventionalCommitSubject(subject), subject).toBe(true);
      expect(validateCommitSubject(subject), subject).toBeNull();
    }
  });

  it("accepts merge, revert, and GitHub squash suffix subjects", () => {
    const mergeQueueSubject =
      "Merge pull request #146 from fy-agent/dev/change-plan-typed-executor-final";
    expect(mergeQueueSubject.length).toBeGreaterThan(72);
    expect(isConventionalCommitSubject(mergeQueueSubject)).toBe(true);
    expect(validateCommitSubject(mergeQueueSubject)).toBeNull();
    expect(
      isConventionalCommitSubject(
        "Merge pull request #19 from fy-agent/codex/fix-v0.3.4-release",
      ),
    ).toBe(true);
    expect(
      isConventionalCommitSubject('Revert "fix(ci): align contracts"'),
    ).toBe(true);
    expect(isConventionalCommitSubject("fix(ci): align contracts (#120)")).toBe(
      true,
    );
    expect(stripGithubSquashSuffix("fix(ci): align contracts (#120)")).toBe(
      "fix(ci): align contracts",
    );
  });

  it("rejects empty and non-conventional subjects without imposing a max length", () => {
    expect(validateCommitSubject("")).toContain("must not be empty");
    expect(validateCommitSubject("Update files")).toContain(
      "Conventional Commits",
    );
    expect(validateCommitSubject("feat: ")).toContain("Conventional Commits");
    const longSubject = `fix(ci): ${"x".repeat(240)}`;
    expect(longSubject.length).toBeGreaterThan(200);
    expect(isConventionalCommitSubject(longSubject)).toBe(true);
    expect(validateCommitSubject(longSubject)).toBeNull();
    expect(validateCommitSubject(`Update files ${"x".repeat(240)}`)).toContain(
      "Conventional Commits",
    );
  });

  it("documents the allowed Conventional Commit types", () => {
    expect(CONVENTIONAL_COMMIT_TYPES).toEqual([
      "feat",
      "fix",
      "docs",
      "style",
      "refactor",
      "test",
      "ci",
      "chore",
    ]);
  });

  it("fails closed on malformed CLI input", () => {
    const result = spawnSync(
      process.execPath,
      ["scripts/ci/verify-commit-messages.mjs", "--base", "abc"],
      { cwd: ROOT, encoding: "utf8" },
    );
    expect(result.status).toBe(1);
    expect(result.stderr).toContain("--head are required");
  });
});

describe("commit message range verification", () => {
  afterAll(() => {
    for (const root of temporaryRoots) {
      fs.rmSync(root, { recursive: true, force: true });
    }
  });

  it("validates a conventional HEAD subject in an empty comparison", () => {
    const root = fs.mkdtempSync(
      path.join(os.tmpdir(), "fyagent-commit-messages-"),
    );
    temporaryRoots.push(root);
    git(root, "init", "--quiet");
    git(root, "config", "user.name", "FyAgent Tests");
    git(root, "config", "user.email", "tests@fyagent.invalid");
    fs.writeFileSync(path.join(root, "README"), "fixture\n");
    git(root, "add", "README");
    git(root, "commit", "-m", "ci: empty comparison fixture");
    const head = git(root, "rev-parse", "HEAD");
    const result = spawnSync(
      process.execPath,
      [VERIFY_COMMIT_MESSAGES, "--base", head, "--head", head],
      { cwd: root, encoding: "utf8" },
    );
    expect(result.status).toBe(0);
    const report = JSON.parse(result.stdout) as {
      ok: boolean;
      commitCount: number;
    };
    expect(report.ok).toBe(true);
    expect(report.commitCount).toBe(1);
  });

  it("accepts a long conventional pull request title", () => {
    const root = fs.mkdtempSync(
      path.join(os.tmpdir(), "fyagent-commit-messages-"),
    );
    temporaryRoots.push(root);
    git(root, "init", "--quiet");
    git(root, "config", "user.name", "FyAgent Tests");
    git(root, "config", "user.email", "tests@fyagent.invalid");
    fs.writeFileSync(path.join(root, "README"), "fixture\n");
    git(root, "add", "README");
    git(root, "commit", "-m", "ci: long title fixture");
    const head = git(root, "rev-parse", "HEAD");
    const prTitle = `ci(release): ${"formal retry contract ".repeat(12).trim()}`;
    expect(prTitle.length).toBeGreaterThan(200);

    const result = spawnSync(
      process.execPath,
      [
        VERIFY_COMMIT_MESSAGES,
        "--base",
        head,
        "--head",
        head,
        "--pr-title",
        prTitle,
      ],
      { cwd: root, encoding: "utf8" },
    );
    expect(result.status).toBe(0);
    const report = JSON.parse(result.stdout) as {
      ok: boolean;
      commitCount: number;
      errors: string[];
    };
    expect(report).toEqual({ ok: true, commitCount: 1, errors: [] });
  });
});
