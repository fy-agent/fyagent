import { spawnSync } from "node:child_process";
import path from "node:path";
import { describe, expect, it } from "vitest";
// @ts-expect-error The workflow executes this dependency-free JavaScript helper directly.
import * as commitMessages from "../scripts/ci/verify-commit-messages.mjs";

const ROOT = path.resolve(__dirname, "..");
const {
  CONVENTIONAL_COMMIT_TYPES,
  isConventionalCommitSubject,
  stripGithubSquashSuffix,
  validateCommitSubject,
  verifyCommitMessages,
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
    expect(
      isConventionalCommitSubject(
        'Merge pull request #19 from fy-agent/codex/fix-v0.3.4-release',
      ),
    ).toBe(true);
    expect(isConventionalCommitSubject('Revert "fix(ci): align contracts"')).toBe(
      true,
    );
    expect(
      isConventionalCommitSubject("fix(ci): align contracts (#120)"),
    ).toBe(true);
    expect(stripGithubSquashSuffix("fix(ci): align contracts (#120)")).toBe(
      "fix(ci): align contracts",
    );
  });

  it("rejects empty, overlong, and non-conventional subjects", () => {
    expect(validateCommitSubject("")).toContain("must not be empty");
    expect(validateCommitSubject("Update files")).toContain(
      "Conventional Commits",
    );
    expect(validateCommitSubject("feat: ")).toContain("Conventional Commits");
    expect(
      validateCommitSubject(`fix(ci): ${"x".repeat(80)}`),
    ).toContain("exceeds 72");
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
  it("validates the current HEAD subject in an empty comparison", () => {
    const head = spawnSync("git", ["rev-parse", "HEAD"], {
      cwd: ROOT,
      encoding: "utf8",
    }).stdout.trim();
    const report = verifyCommitMessages({ baseSha: head, headSha: head });
    expect(report.ok).toBe(true);
    expect(report.commitCount).toBe(1);
  });
});
