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
    expect(validateCommitSubject("merge: ordinary subject")).toContain(
      "Conventional Commits",
    );
    for (const subject of ["perf: local change", "build: local change"]) {
      expect(validateCommitSubject(subject)).toContain("Conventional Commits");
    }
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

  function mergedFixture(
    sideSubject = "feat: side branch change",
    mergeSubject = "merge: integrate tested changes",
  ) {
    const root = fs.mkdtempSync(
      path.join(os.tmpdir(), "fyagent-commit-topology-"),
    );
    temporaryRoots.push(root);
    git(root, "init", "--quiet");
    git(root, "config", "user.name", "FyAgent Tests");
    git(root, "config", "user.email", "tests@fyagent.invalid");
    git(root, "commit", "--allow-empty", "-m", "chore: baseline");
    const base = git(root, "rev-parse", "HEAD");
    const branch = git(root, "branch", "--show-current");
    git(root, "checkout", "-b", "topic");
    git(root, "commit", "--allow-empty", "-m", sideSubject);
    const side = git(root, "rev-parse", "HEAD");
    git(root, "checkout", branch);
    git(root, "commit", "--allow-empty", "-m", "chore: primary branch change");
    git(root, "merge", "--no-ff", "topic", "-m", mergeSubject);
    const head = git(root, "rev-parse", "HEAD");
    return { root, base, head, side };
  }

  function upstreamFixture(subject = "Original upstream release title") {
    const fixture = mergedFixture(
      subject,
      "merge(upstream): import CC Switch v4.0.4",
    );
    const { root, side } = fixture;
    git(root, "tag", "-a", "v4.0.4", side, "-m", "upstream release");
    const tagObject = git(root, "rev-parse", "v4.0.4");
    const ledger = path.join(root, "docs/upstream/cc-switch-v4.0.4.md");
    fs.mkdirSync(path.dirname(ledger), { recursive: true });
    fs.writeFileSync(
      ledger,
      [
        "| Item | Identity |",
        "| --- | --- |",
        "| Upstream repository | `https://github.com/farion1231/cc-switch.git` |",
        "| Annotated tag | `v4.0.4` |",
        `| Full tag-object SHA | \`${tagObject}\` |`,
        `| Full peeled commit SHA | \`${side}\` |`,
        "",
      ].join("\n"),
    );
    git(root, "add", "docs/upstream");
    git(root, "commit", "-m", "docs(upstream): record verified source");
    // CI needs no upstream remote or fetched tag ref.
    git(root, "tag", "-d", "v4.0.4");
    return {
      ...fixture,
      head: git(root, "rev-parse", "HEAD"),
      ledger,
      tagObject,
    };
  }

  function verifyRange(
    root: string,
    base: string,
    head: string,
    prTitle?: string,
  ) {
    return spawnSync(
      process.execPath,
      [
        VERIFY_COMMIT_MESSAGES,
        "--base",
        base,
        "--head",
        head,
        ...(prTitle ? ["--pr-title", prTitle] : []),
      ],
      { cwd: root, encoding: "utf8" },
    );
  }

  it("accepts an explicit integration subject only with real merge parents, including HEAD-only checks", () => {
    const { root, base, head } = mergedFixture();
    expect(
      git(root, "show", "-s", "--format=%P", head).split(" "),
    ).toHaveLength(2);
    for (const [from, count] of [
      [base, 3],
      [head, 1],
    ] as const) {
      const result = verifyRange(root, from, head);
      expect(result.status, result.stderr).toBe(0);
      expect(JSON.parse(result.stdout)).toEqual({
        ok: true,
        commitCount: count,
        upstreamCommitCount: 0,
        upstreamSources: [],
        errors: [],
      });
    }
    const title = verifyRange(
      root,
      base,
      head,
      "merge: not a pull request type",
    );
    expect(title.status).toBe(1);
    expect(title.stderr).toContain("pull request title");
  });

  it("rejects a single-parent commit pretending to be an integration", () => {
    const { root, head: base } = mergedFixture();
    git(root, "commit", "--allow-empty", "-m", "merge: pretend integration");
    const head = git(root, "rev-parse", "HEAD");
    const result = verifyRange(root, base, head);
    expect(result.status).toBe(1);
    expect(result.stderr).toContain("merge: pretend integration");
  });

  it("still inspects invalid side-branch commits behind a valid merge", () => {
    const { root, base, head } = mergedFixture("invalid side change");
    const result = verifyRange(root, base, head);
    expect(result.status).toBe(1);
    const report = JSON.parse(result.stdout) as {
      commitCount: number;
      errors: string[];
    };
    expect(report.commitCount).toBe(3);
    expect(report.errors).toHaveLength(1);
    expect(report.errors[0]).toContain("invalid side change");
  });

  it("does not exempt arbitrary or empty integration subjects even on merge objects", () => {
    for (const subject of ["untyped integration", "merge: "]) {
      const { root, base, head } = mergedFixture(undefined, subject);
      const result = verifyRange(root, base, head);
      expect(result.status).toBe(1);
      expect(result.stderr).toContain("Conventional Commits");
    }
  });

  it.each([
    "Original upstream release title",
    "perf: improve requests",
    "build: update toolchain",
  ])(
    "preserves pinned upstream history without applying FyAgent subject rules: %s",
    (subject) => {
      const { root, base, head, side, tagObject } = upstreamFixture(subject);
      const result = verifyRange(
        root,
        base,
        head,
        "feat: integrate upstream capabilities",
      );
      expect(result.status, result.stderr).toBe(0);
      expect(JSON.parse(result.stdout)).toEqual({
        ok: true,
        commitCount: 4,
        upstreamCommitCount: 1,
        upstreamSources: [
          {
            repository: "https://github.com/farion1231/cc-switch.git",
            tag: "v4.0.4",
            tagObject,
            commit: side,
          },
        ],
        errors: [],
      });
      expect(
        verifyRange(root, base, head, "merge(upstream): invalid PR title")
          .status,
      ).toBe(1);
    },
  );

  it("keeps local commits above the upstream anchor and first-parent commits strict", () => {
    const { root, base, side } = upstreamFixture();
    const branch = git(root, "branch", "--show-current");
    git(root, "checkout", "-b", "local-above-upstream", side);
    git(root, "commit", "--allow-empty", "-m", "invalid local side change");
    git(root, "checkout", branch);
    git(root, "commit", "--allow-empty", "-m", "invalid local main change");
    git(
      root,
      "merge",
      "--no-ff",
      "local-above-upstream",
      "-m",
      "merge: integrate local work",
    );
    const result = verifyRange(root, base, git(root, "rev-parse", "HEAD"));
    expect(result.status).toBe(1);
    const report = JSON.parse(result.stdout);
    expect(report.upstreamCommitCount).toBe(1);
    expect(report.errors).toHaveLength(2);
    expect(result.stderr).toContain("invalid local side change");
    expect(result.stderr).toContain("invalid local main change");
  });

  it("does not grant upstream exemptions from the subject alone or a mismatched anchor", () => {
    const missing = mergedFixture(
      "upstream prose",
      "merge(upstream): unrecorded import",
    );
    const missingResult = verifyRange(missing.root, missing.base, missing.head);
    expect(missingResult.status).toBe(1);
    expect(JSON.parse(missingResult.stdout).upstreamCommitCount).toBe(0);

    const { root, base, side, ledger } = upstreamFixture();
    fs.writeFileSync(
      ledger,
      fs.readFileSync(ledger, "utf8").replace(side, base),
    );
    git(root, "add", "docs/upstream");
    git(root, "commit", "-m", "docs: wrong upstream anchor");
    const mismatch = verifyRange(root, base, git(root, "rev-parse", "HEAD"));
    expect(mismatch.status).toBe(1);
    expect(JSON.parse(mismatch.stdout).upstreamCommitCount).toBe(0);
    expect(mismatch.stderr).toContain("Original upstream release title");
  });

  it("rejects an upstream-style subject on a single-parent commit", () => {
    const { root, head: base } = upstreamFixture();
    git(
      root,
      "commit",
      "--allow-empty",
      "-m",
      "merge(upstream): pretend import",
    );
    const result = verifyRange(root, base, git(root, "rev-parse", "HEAD"));
    expect(result.status).toBe(1);
    expect(result.stderr).toContain("merge(upstream): pretend import");
  });

  it("reads the established provenance ledger field names and rejects a foreign source", () => {
    const { root, base, ledger } = upstreamFixture();
    fs.writeFileSync(
      ledger,
      fs
        .readFileSync(ledger, "utf8")
        .replace("Full tag-object SHA", "Tag object")
        .replace("Full peeled commit SHA", "Peeled commit"),
    );
    git(root, "add", "docs/upstream");
    git(root, "commit", "-m", "docs: use existing provenance field names");
    expect(verifyRange(root, base, git(root, "rev-parse", "HEAD")).status).toBe(
      0,
    );

    fs.writeFileSync(
      ledger,
      fs
        .readFileSync(ledger, "utf8")
        .replace("farion1231/cc-switch", "someone/another-repository"),
    );
    git(root, "add", "docs/upstream");
    git(root, "commit", "-m", "docs: point at a different source");
    const result = verifyRange(root, base, git(root, "rev-parse", "HEAD"));
    expect(result.status).toBe(1);
    expect(result.stderr).toContain("invalid upstream source identity");
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
    expect(report).toEqual({
      ok: true,
      commitCount: 1,
      upstreamCommitCount: 0,
      upstreamSources: [],
      errors: [],
    });
  });
});
