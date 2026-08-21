#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import process from "node:process";
import { pathToFileURL } from "node:url";

export const CONVENTIONAL_COMMIT_TYPES = Object.freeze([
  "feat",
  "fix",
  "docs",
  "style",
  "refactor",
  "test",
  "ci",
  "chore",
]);

export const CONVENTIONAL_SUBJECT_PATTERN =
  /^(feat|fix|docs|style|refactor|test|ci|chore)(\([a-z0-9./-]+\))?: .+$/u;

export const MERGE_COMMIT_SUBJECT_PATTERNS = Object.freeze([
  /^Merge pull request #\d+ from /u,
  /^Merge branch /u,
  /^Merge remote-tracking branch /u,
]);

export const REVERT_COMMIT_SUBJECT_PATTERN = /^Revert "/u;

export const GITHUB_SQUASH_PR_SUFFIX_PATTERN = /\s\(#\d+\)$/u;

const SUBJECT_MAX_LENGTH = 72;

function git(args) {
  const result = spawnSync("git", args, {
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  });
  if (result.status !== 0) {
    const detail = (result.stderr || result.stdout || "").trim();
    throw new Error(
      detail ? `git ${args.join(" ")} failed: ${detail}` : `git ${args.join(" ")} failed`,
    );
  }
  return result.stdout;
}

function assertCommitSha(value, label) {
  if (typeof value !== "string" || !/^[0-9a-f]{40}$/u.test(value)) {
    throw new Error(`${label} must be a full 40-character commit SHA`);
  }
  git(["cat-file", "-e", `${value}^{commit}`]);
  return value;
}

export function stripGithubSquashSuffix(subject) {
  return subject.replace(GITHUB_SQUASH_PR_SUFFIX_PATTERN, "");
}

export function isMergeCommitSubject(subject) {
  return MERGE_COMMIT_SUBJECT_PATTERNS.some((pattern) => pattern.test(subject));
}

export function isRevertCommitSubject(subject) {
  return REVERT_COMMIT_SUBJECT_PATTERN.test(subject);
}

export function isConventionalCommitSubject(subject) {
  const trimmed = subject.trim();
  if (trimmed.length === 0 || trimmed.length > SUBJECT_MAX_LENGTH) {
    return false;
  }
  if (isMergeCommitSubject(trimmed) || isRevertCommitSubject(trimmed)) {
    return true;
  }
  const normalized = stripGithubSquashSuffix(trimmed);
  if (normalized.length === 0 || normalized.length > SUBJECT_MAX_LENGTH) {
    return false;
  }
  return CONVENTIONAL_SUBJECT_PATTERN.test(normalized);
}

export function validateCommitSubject(subject) {
  const trimmed = subject.trim();
  if (trimmed.length === 0) {
    return "commit subject must not be empty";
  }
  if (trimmed.length > SUBJECT_MAX_LENGTH) {
    return `commit subject exceeds ${SUBJECT_MAX_LENGTH} characters`;
  }
  if (isMergeCommitSubject(trimmed) || isRevertCommitSubject(trimmed)) {
    return null;
  }
  const normalized = stripGithubSquashSuffix(trimmed);
  if (normalized.length === 0) {
    return "commit subject must not be empty after removing GitHub PR suffix";
  }
  if (normalized.length > SUBJECT_MAX_LENGTH) {
    return `commit subject exceeds ${SUBJECT_MAX_LENGTH} characters after removing GitHub PR suffix`;
  }
  if (!CONVENTIONAL_SUBJECT_PATTERN.test(normalized)) {
    return [
      "commit subject must follow Conventional Commits:",
      "  type(scope): description",
      `  allowed types: ${CONVENTIONAL_COMMIT_TYPES.join(", ")}`,
      "  examples: fix(ci): align contracts, chore(deps): update dependencies",
    ].join("\n");
  }
  return null;
}

export function listCommitSubjectsInRange(baseSha, headSha) {
  assertCommitSha(baseSha, "base");
  assertCommitSha(headSha, "head");
  if (baseSha === headSha) {
    return [{ sha: headSha, subject: git(["show", "-s", "--format=%s", headSha]).trim() }];
  }
  const output = git([
    "log",
    "--format=%H%x09%s",
    `${baseSha}..${headSha}`,
  ]).trim();
  if (output.length === 0) {
    return [];
  }
  return output.split("\n").map((line) => {
    const separator = line.indexOf("\t");
    if (separator <= 0) {
      throw new Error(`malformed git log output: ${line}`);
    }
    return {
      sha: line.slice(0, separator),
      subject: line.slice(separator + 1),
    };
  });
}

export function verifyCommitMessages({
  baseSha,
  headSha,
  prTitle = null,
}) {
  const errors = [];
  const commits = listCommitSubjectsInRange(baseSha, headSha);
  for (const commit of commits) {
    const violation = validateCommitSubject(commit.subject);
    if (violation) {
      errors.push(`${commit.sha.slice(0, 12)} ${commit.subject}\n  ${violation}`);
    }
  }
  if (typeof prTitle === "string" && prTitle.trim().length > 0) {
    const violation = validateCommitSubject(prTitle);
    if (violation) {
      errors.push(`pull request title ${JSON.stringify(prTitle)}\n  ${violation}`);
    }
  }
  return {
    ok: errors.length === 0,
    commitCount: commits.length,
    errors,
  };
}

function argumentValue(argv, name) {
  const index = argv.indexOf(name);
  if (index === -1) return null;
  const value = argv[index + 1];
  if (!value || value.startsWith("--")) {
    throw new Error(`${name} requires a value`);
  }
  return value;
}

export function runVerifyCommitMessagesCli(argv = process.argv.slice(2)) {
  try {
    const baseSha = argumentValue(argv, "--base");
    const headSha = argumentValue(argv, "--head");
    if (!baseSha || !headSha) {
      throw new Error("--base and --head are required");
    }
    const prTitle = argumentValue(argv, "--pr-title");
    const report = verifyCommitMessages({ baseSha, headSha, prTitle });
    console.log(JSON.stringify(report, null, 2));
    if (!report.ok) {
      for (const error of report.errors) {
        console.error(`Commit convention failed:\n${error}`);
      }
      process.exitCode = 1;
    }
    return report;
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
    return {
      ok: false,
      commitCount: 0,
      errors: [error instanceof Error ? error.message : String(error)],
    };
  }
}

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(process.argv[1]).href
) {
  runVerifyCommitMessagesCli();
}
