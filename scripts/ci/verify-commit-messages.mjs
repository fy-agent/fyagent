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

const UPSTREAM_REPOSITORY = "https://github.com/farion1231/cc-switch.git";
const UPSTREAM_MERGE_SUBJECT = /^merge\(upstream\): \S.*$/u;

function git(args) {
  const result = spawnSync("git", args, {
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  });
  if (result.status !== 0) {
    const detail = (result.stderr || result.stdout || "").trim();
    throw new Error(
      detail
        ? `git ${args.join(" ")} failed: ${detail}`
        : `git ${args.join(" ")} failed`,
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
  if (trimmed.length === 0) {
    return false;
  }
  if (isMergeCommitSubject(trimmed) || isRevertCommitSubject(trimmed)) {
    return true;
  }
  const normalized = stripGithubSquashSuffix(trimmed);
  if (normalized.length === 0) {
    return false;
  }
  return CONVENTIONAL_SUBJECT_PATTERN.test(normalized);
}

export function validateCommitSubject(subject) {
  const trimmed = subject.trim();
  if (trimmed.length === 0) {
    return "commit subject must not be empty";
  }
  if (isMergeCommitSubject(trimmed) || isRevertCommitSubject(trimmed)) {
    return null;
  }
  const normalized = stripGithubSquashSuffix(trimmed);
  if (normalized.length === 0) {
    return "commit subject must not be empty after removing GitHub PR suffix";
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
  const format = "--format=%H%x09%P%x09%s";
  const output = git(
    baseSha === headSha
      ? ["show", "-s", format, headSha]
      : ["log", format, `${baseSha}..${headSha}`],
  ).trim();
  if (output.length === 0) {
    return [];
  }
  return output.split("\n").map((line) => {
    const separator = line.indexOf("\t");
    const subjectStart = line.indexOf("\t", separator + 1);
    const parents = line
      .slice(separator + 1, subjectStart)
      .split(" ")
      .filter(Boolean);
    if (
      separator <= 0 ||
      subjectStart < 0 ||
      !/^[0-9a-f]{40}$/u.test(line.slice(0, separator)) ||
      parents.some((parent) => !/^[0-9a-f]{40}$/u.test(parent))
    ) {
      throw new Error(`malformed git log output: ${line}`);
    }
    return {
      sha: line.slice(0, separator),
      parents,
      subject: line.slice(subjectStart + 1),
    };
  });
}

function verifiedUpstreamHistory(commits, baseSha, headSha) {
  const merges = commits.filter(
    ({ parents, subject }) =>
      parents.length === 2 &&
      parents[0] !== parents[1] &&
      UPSTREAM_MERGE_SUBJECT.test(subject),
  );
  const sources = [];
  const mergeShas = new Set();
  const upstreamShas = new Set();
  if (merges.length === 0) return { sources, mergeShas, upstreamShas };

  const paths = git([
    "ls-tree",
    "-r",
    "--name-only",
    headSha,
    "--",
    "docs/upstream",
  ])
    .trim()
    .split("\n");
  for (const path of paths) {
    if (!/^docs\/upstream\/cc-switch-v\d+\.\d+\.\d+\.md$/u.test(path)) continue;
    const ledger = git(["show", `${headSha}:${path}`]);
    const rows = new Map(
      [...ledger.matchAll(/^\|\s*([^|]+?)\s*\|\s*`([^`]+)`[^|]*\|/gmu)].map(
        (match) => [match[1].trim(), match[2]],
      ),
    );
    const commit =
      rows.get("Full peeled commit SHA") ?? rows.get("Peeled commit");
    const matchingMerges = merges.filter(
      ({ parents }) => parents[1] === commit,
    );
    if (matchingMerges.length === 0) continue;
    const tag = rows.get("Annotated tag");
    const tagObject = rows.get("Full tag-object SHA") ?? rows.get("Tag object");
    if (
      rows.get("Upstream repository") !== UPSTREAM_REPOSITORY ||
      !/^v\d+\.\d+\.\d+$/u.test(tag ?? "") ||
      path !== `docs/upstream/cc-switch-${tag}.md` ||
      !/^[0-9a-f]{40}$/u.test(tagObject ?? "")
    ) {
      throw new Error(`invalid upstream source identity in ${path}`);
    }
    // Source identities are verified when the upstream ledger is maintained.
    // Title checks use that pinned commit graph without fetching tags or refs.
    sources.push({ repository: UPSTREAM_REPOSITORY, tag, tagObject, commit });
    for (const merge of matchingMerges) mergeShas.add(merge.sha);
    for (const sha of git(["rev-list", commit, "--not", baseSha])
      .trim()
      .split("\n")) {
      if (sha) upstreamShas.add(sha);
    }
  }
  return { sources, mergeShas, upstreamShas };
}

export function verifyCommitMessages({ baseSha, headSha, prTitle = null }) {
  const errors = [];
  const commits = listCommitSubjectsInRange(baseSha, headSha);
  const upstream = verifiedUpstreamHistory(commits, baseSha, headSha);
  for (const commit of commits) {
    if (
      upstream.upstreamShas.has(commit.sha) ||
      upstream.mergeShas.has(commit.sha)
    )
      continue;
    // Custom integration subjects are meaningful only on real merge objects.
    // Ordinary side commits and PR titles remain strict.
    if (
      new Set(commit.parents).size >= 2 &&
      /^merge: \S.*$/u.test(commit.subject)
    )
      continue;
    const violation = validateCommitSubject(commit.subject);
    if (violation) {
      errors.push(
        `${commit.sha.slice(0, 12)} ${commit.subject}\n  ${violation}`,
      );
    }
  }
  if (typeof prTitle === "string" && prTitle.trim().length > 0) {
    const violation = validateCommitSubject(prTitle);
    if (violation) {
      errors.push(
        `pull request title ${JSON.stringify(prTitle)}\n  ${violation}`,
      );
    }
  }
  return {
    ok: errors.length === 0,
    commitCount: commits.length,
    upstreamCommitCount: commits.filter(({ sha }) =>
      upstream.upstreamShas.has(sha),
    ).length,
    upstreamSources: upstream.sources,
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
