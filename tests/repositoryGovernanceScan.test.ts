import { spawnSync, type SpawnSyncReturns } from "node:child_process";
import { randomBytes } from "node:crypto";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { afterAll, beforeAll, describe, expect, it } from "vitest";

const ROOT = path.resolve(__dirname, "..");
const SCANNER = path.join(ROOT, "scripts/audit/repository-governance-scan.mjs");

type SafeEntry = {
  category: string;
  path: string | null;
  oid: string | null;
  count: number;
  size: number;
};

type ScanReport = {
  scannerVersion: number;
  mode: string;
  sourceOid: string | null;
  counts: {
    objects: number;
    blobs: number;
    paths: number;
    findings: number;
  };
  findings: SafeEntry[];
  sizes: SafeEntry[];
  failures: SafeEntry[];
};

function runGit(cwd: string, args: string[], input?: string | Buffer): Buffer {
  const result = spawnSync("git", args, {
    cwd,
    input,
    encoding: null,
    shell: false,
    windowsHide: true,
  });
  if (result.status !== 0 || result.error !== undefined) {
    throw new Error("synthetic Git fixture command failed");
  }
  return result.stdout;
}

function gitText(cwd: string, args: string[], input?: string | Buffer): string {
  return runGit(cwd, args, input).toString("utf8").trim();
}

function runScanner(cwd: string, args: string[]): SpawnSyncReturns<string> {
  return spawnSync(process.execPath, [SCANNER, ...args], {
    cwd,
    encoding: "utf8",
    maxBuffer: 64 * 1024 * 1024,
    shell: false,
    windowsHide: true,
  });
}

function successReport(result: SpawnSyncReturns<string>): ScanReport {
  expect(result.status, "scanner exit status").toBe(0);
  expect(result.stderr, "scanner stderr length").toHaveLength(0);
  return JSON.parse(result.stdout) as ScanReport;
}

function assertCandidateSuppressed(
  candidate: string,
  result: SpawnSyncReturns<string>,
  report?: ScanReport,
): void {
  const surfaces = [
    result.stdout,
    result.stderr,
    result.error?.message ?? "",
    report === undefined ? "" : JSON.stringify(report),
  ];
  for (const [index, surface] of surfaces.entries()) {
    expect(
      surface.includes(candidate),
      `candidate suppression surface ${index}`,
    ).toBe(false);
  }
}

function assertSafeShape(report: ScanReport): void {
  expect(Object.keys(report).sort()).toEqual([
    "counts",
    "failures",
    "findings",
    "mode",
    "scannerVersion",
    "sizes",
    "sourceOid",
  ]);
  for (const entry of [
    ...report.findings,
    ...report.sizes,
    ...report.failures,
  ]) {
    expect(Object.keys(entry).sort()).toEqual([
      "category",
      "count",
      "oid",
      "path",
      "size",
    ]);
  }
}

describe("repository governance scanner", () => {
  let repository = "";
  let nonRepository = "";
  let candidate = "";
  let firstCommit = "";
  let headCommit = "";
  let deletedOid = "";
  let pathlessOid = "";
  let currentDuplicateOid = "";
  let binaryOid = "";
  let unusualCommit = "";
  let unusualOid = "";
  let nestedOid = "";

  beforeAll(() => {
    repository = fs.mkdtempSync(path.join(os.tmpdir(), "fyagent-audit-repo-"));
    nonRepository = fs.mkdtempSync(
      path.join(os.tmpdir(), "fyagent-audit-empty-"),
    );
    candidate = ["ghp", randomBytes(24).toString("hex")].join("_");

    gitText(repository, ["init", "--quiet"]);
    gitText(repository, ["config", "user.name", "Audit Fixture"]);
    gitText(repository, ["config", "user.email", "audit@example.invalid"]);

    const deletedContent = `deleted:${candidate}`;
    fs.writeFileSync(
      path.join(repository, "deleted-secret.txt"),
      deletedContent,
    );
    fs.writeFileSync(
      path.join(repository, "rename-source.txt"),
      "rename fixture",
    );
    gitText(repository, ["add", "--all"]);
    gitText(repository, ["commit", "--quiet", "-m", "fixture: initial"]);
    firstCommit = gitText(repository, ["rev-parse", "HEAD"]);
    deletedOid = gitText(repository, ["hash-object", "deleted-secret.txt"]);

    fs.rmSync(path.join(repository, "deleted-secret.txt"));
    fs.renameSync(
      path.join(repository, "rename-source.txt"),
      path.join(repository, "renamed-current.txt"),
    );
    const duplicateContent = `current:${candidate}`;
    fs.writeFileSync(path.join(repository, "current-a.txt"), duplicateContent);
    fs.writeFileSync(path.join(repository, "current-b.txt"), duplicateContent);
    fs.writeFileSync(
      path.join(repository, "binary-secret.bin"),
      Buffer.concat([
        Buffer.from([0, 255, 1, 254]),
        Buffer.from(candidate, "ascii"),
        Buffer.from([0, 2, 0, 3]),
      ]),
    );
    fs.writeFileSync(
      path.join(repository, `${candidate}.txt`),
      `protected-path:${candidate}`,
    );
    fs.mkdirSync(path.join(repository, "nested"));
    fs.writeFileSync(
      path.join(repository, "nested", "path=hint.txt"),
      "nested fixture",
    );
    gitText(repository, ["add", "--all"]);
    gitText(repository, ["commit", "--quiet", "-m", "fixture: current"]);
    headCommit = gitText(repository, ["rev-parse", "HEAD"]);
    currentDuplicateOid = gitText(repository, ["hash-object", "current-a.txt"]);
    binaryOid = gitText(repository, ["hash-object", "binary-secret.bin"]);
    nestedOid = gitText(repository, ["hash-object", "nested/path=hint.txt"]);

    pathlessOid = gitText(
      repository,
      ["hash-object", "-w", "--stdin"],
      `pathless:${candidate}`,
    );
    gitText(repository, [
      "update-ref",
      "refs/tags/pathless-audit-fixture",
      pathlessOid,
    ]);

    unusualOid = gitText(
      repository,
      ["hash-object", "-w", "--stdin"],
      `unusual:${candidate}`,
    );
    const unusualTree = gitText(
      repository,
      ["mktree", "-z"],
      Buffer.concat([
        Buffer.from(
          `100644 blob ${unusualOid}\tline\n${deletedOid}\npath=break.bin`,
          "utf8",
        ),
        Buffer.from([0]),
      ]),
    );
    unusualCommit = gitText(repository, [
      "commit-tree",
      unusualTree,
      "-m",
      "fixture: unusual path",
    ]);
    gitText(repository, [
      "update-ref",
      "refs/tags/unusual-audit-fixture",
      unusualCommit,
    ]);
  });

  afterAll(() => {
    for (const temporary of [repository, nonRepository]) {
      if (
        temporary.startsWith(os.tmpdir()) &&
        path.basename(temporary).startsWith("fyagent-audit-")
      ) {
        fs.rmSync(temporary, { recursive: true, force: true });
      }
    }
  });

  it("scans an explicit current tree and deduplicates blobs", () => {
    const result = runScanner(repository, ["current", "--treeish", headCommit]);
    const report = successReport(result);

    expect(report.mode).toBe("current");
    expect(report.sourceOid).toMatch(/^[0-9a-f]{40,64}$/u);
    expect(report.failures).toEqual([]);
    expect(
      report.sizes.filter(({ oid }) => oid === currentDuplicateOid),
    ).toEqual([expect.objectContaining({ count: 2, path: "current-a.txt" })]);
    expect(report.findings.some(({ oid }) => oid === binaryOid)).toBe(true);
    expect(report.sizes.some(({ oid }) => oid === deletedOid)).toBe(false);
    expect(report.sizes.some(({ oid }) => oid === pathlessOid)).toBe(false);
    expect(
      report.sizes.some(({ path: safePath }) => safePath === "<redacted-path>"),
    ).toBe(true);
    assertSafeShape(report);
    assertCandidateSuppressed(candidate, result, report);
  });

  it("binds current scanning to the requested historical treeish", () => {
    const result = runScanner(repository, [
      "current",
      "--treeish",
      firstCommit,
    ]);
    const report = successReport(result);

    expect(report.sizes.some(({ oid }) => oid === deletedOid)).toBe(true);
    expect(report.sizes.some(({ oid }) => oid === currentDuplicateOid)).toBe(
      false,
    );
    assertCandidateSuppressed(candidate, result, report);
  });

  it("scans every unique reachable history blob, including deleted and pathless objects", () => {
    const result = runScanner(repository, ["history"]);
    const report = successReport(result);

    expect(report.mode).toBe("history");
    expect(report.sourceOid).toBeNull();
    expect(report.sizes.some(({ oid }) => oid === deletedOid)).toBe(true);
    expect(report.sizes.filter(({ oid }) => oid === pathlessOid)).toEqual([
      expect.objectContaining({ count: 0, path: null }),
    ]);
    expect(
      report.sizes.some(
        ({ path: safePath }) => safePath === "renamed-current.txt",
      ),
    ).toBe(true);
    expect(
      report.sizes.some(({ path: safePath }) => safePath?.startsWith("path=")),
    ).toBe(false);
    expect(new Set(report.sizes.map(({ oid }) => oid)).size).toBe(
      report.sizes.length,
    );
    expect(report.findings.some(({ oid }) => oid === pathlessOid)).toBe(true);
    expect(report.sizes.filter(({ oid }) => oid === nestedOid)).toEqual([
      expect.objectContaining({ count: 1, path: "nested/path=hint.txt" }),
    ]);
    expect(report.sizes.filter(({ oid }) => oid === unusualOid)).toEqual([
      expect.objectContaining({ count: 1, path: "<redacted-path>" }),
    ]);
    const reachableObjects = runGit(repository, [
      "rev-list",
      "--objects",
      "--all",
      "--no-object-names",
    ]);
    const reachableBlobs = runGit(
      repository,
      ["cat-file", "--batch-check=%(objectname) %(objecttype)"],
      reachableObjects,
    )
      .toString("ascii")
      .trim()
      .split("\n")
      .filter((record) => record.endsWith(" blob"))
      .map((record) => record.split(" ")[0]);
    expect(report.sizes.map(({ oid }) => oid).sort()).toEqual(
      reachableBlobs.sort(),
    );
    assertSafeShape(report);
    assertCandidateSuppressed(candidate, result, report);
  });

  it("parses NUL-delimited unusual paths without returning control bytes", () => {
    const result = runScanner(repository, [
      "current",
      "--treeish",
      unusualCommit,
    ]);
    const report = successReport(result);

    expect(report.sizes).toHaveLength(1);
    expect(report.sizes[0].path).toBe("<redacted-path>");
    expect(report.findings).toHaveLength(1);
    assertCandidateSuppressed(candidate, result, report);
  });

  it("fails closed with fixed JSON when a candidate-shaped treeish is invalid", () => {
    const result = runScanner(repository, ["current", "--treeish", candidate]);

    expect(result.status).toBe(1);
    expect(result.stdout).toHaveLength(0);
    const report = JSON.parse(result.stderr) as ScanReport;
    expect(report.failures).toEqual([
      {
        category: "treeish-resolution-failed",
        path: null,
        oid: null,
        count: 1,
        size: 0,
      },
    ]);
    assertSafeShape(report);
    assertCandidateSuppressed(candidate, result, report);
  });

  it("fails closed outside a repository without forwarding Git diagnostics", () => {
    const result = runScanner(nonRepository, ["history"]);

    expect(result.status).toBe(1);
    expect(result.stdout).toHaveLength(0);
    const report = JSON.parse(result.stderr) as ScanReport;
    expect(report.failures).toEqual([
      expect.objectContaining({ category: "repository-format-failed" }),
    ]);
    assertSafeShape(report);
    assertCandidateSuppressed(candidate, result, report);
  });

  it("rejects malformed history OID records without normalizing high-bit bytes", async () => {
    const scanner = await import(/* @vite-ignore */ SCANNER);
    const format = { length: 40, pattern: /^[0-9a-f]{40}$/u };
    const valid = Buffer.from(`${"a".repeat(40)}\n`, "ascii");
    expect(scanner.parseHistoryObjectIds(valid, format)).toEqual([
      "a".repeat(40),
    ]);
    expect(scanner.parseHistoryObjectIds(Buffer.alloc(0), format)).toEqual([]);
    const highBit = Buffer.from(valid);
    highBit[0] |= 0x80;
    for (const invalid of [
      highBit,
      valid.subarray(0, -1),
      Buffer.concat([valid, valid]),
      Buffer.from("\n"),
    ]) {
      expect(() => scanner.parseHistoryObjectIds(invalid, format)).toThrow(
        "history-enumeration-invalid",
      );
    }
  });

  it("fails closed on a reachable tree with an unsupported entry mode", () => {
    const malformedTree = gitText(
      repository,
      ["hash-object", "-w", "-t", "tree", "--literally", "--stdin"],
      Buffer.concat([
        Buffer.from("100600 invalid-mode.txt\0", "ascii"),
        Buffer.from(deletedOid, "hex"),
      ]),
    );
    const ref = "refs/tags/malformed-audit-fixture";
    gitText(repository, ["update-ref", ref, malformedTree]);
    try {
      const result = runScanner(repository, ["history"]);
      expect(result.status).toBe(1);
      expect(result.stdout).toHaveLength(0);
      const report = JSON.parse(result.stderr) as ScanReport;
      expect(report.failures).toEqual([
        expect.objectContaining({
          category: "history-path-enumeration-invalid",
        }),
      ]);
      assertSafeShape(report);
      assertCandidateSuppressed(candidate, result, report);
    } finally {
      gitText(repository, ["update-ref", "-d", ref]);
    }
  });

  it("preserves complete SHA-256 history and binary tree object identities", () => {
    const root = fs.mkdtempSync(
      path.join(os.tmpdir(), "fyagent-audit-sha256-"),
    );
    try {
      gitText(root, ["init", "--quiet", "--object-format=sha256"]);
      const blob = gitText(
        root,
        ["hash-object", "-w", "--stdin"],
        `sha256:${candidate}`,
      );
      const childTree = gitText(
        root,
        ["mktree"],
        `100644 blob ${blob}\tchild.txt\n`,
      );
      const tree = gitText(
        root,
        ["mktree"],
        `040000 tree ${childTree}\tnested\n`,
      );
      gitText(root, ["update-ref", "refs/tags/sha256-tree", tree]);
      const pathless = gitText(
        root,
        ["hash-object", "-w", "--stdin"],
        `pathless-sha256:${candidate}`,
      );
      gitText(root, ["update-ref", "refs/tags/sha256-pathless", pathless]);
      const result = runScanner(root, ["history"]);
      const report = successReport(result);
      expect(report.sizes.map(({ oid }) => oid).sort()).toEqual(
        [blob, pathless].sort(),
      );
      expect(report.sizes.find(({ oid }) => oid === blob)).toEqual(
        expect.objectContaining({ path: "nested/child.txt", count: 1 }),
      );
      expect(report.sizes.find(({ oid }) => oid === pathless)).toEqual(
        expect.objectContaining({ path: null, count: 0 }),
      );
      expect(
        report.sizes.every(({ oid }) => /^[0-9a-f]{64}$/u.test(oid ?? "")),
      ).toBe(true);
      assertCandidateSuppressed(candidate, result, report);
    } finally {
      if (
        path.dirname(root) === path.resolve(os.tmpdir()) &&
        path.basename(root).startsWith("fyagent-audit-sha256-")
      ) {
        fs.rmSync(root, { recursive: true, force: true });
      }
    }
  });
});

describe("repository governance private-key allowlist without Git writes", () => {
  it("requires the exact fixture path and pinned digest", async () => {
    const scanner = await import(/* @vite-ignore */ SCANNER);
    const fixturePath =
      "src-tauri/src/services/skill/update/fixtures/server-key.pem";
    const bytes = fs.readFileSync(path.join(ROOT, fixturePath));
    const rawPath = Buffer.from(fixturePath);
    expect(scanner.classifications(bytes, rawPath)).toEqual([]);
    const finding = [{ category: "private-key", count: 1 }];
    expect(
      scanner.classifications(
        Buffer.concat([bytes, Buffer.from("\n")]),
        rawPath,
      ),
    ).toEqual(finding);
    for (const otherPath of [
      "server-key.pem",
      `other/${fixturePath}`,
      fixturePath.replace("server-key", "other-key"),
    ]) {
      expect(scanner.classifications(bytes, Buffer.from(otherPath))).toEqual(
        finding,
      );
    }
    expect(scanner.classifications(bytes, rawPath, false)).toEqual(finding);
    expect(scanner.classifications(bytes)).toEqual(finding);
  });
});
