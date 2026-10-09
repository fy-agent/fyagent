#!/usr/bin/env node

import { createHash } from "node:crypto";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import path from "node:path";

const SCANNER_VERSION = 1;
const MAX_GIT_OUTPUT_BYTES = 768 * 1024 * 1024;
const MAX_BLOB_BYTES = 512 * 1024 * 1024;
const TARGET_BATCH_BYTES = 64 * 1024 * 1024;
const REDACTED_PATH = "<redacted-path>";
// Skills update tests use this self-signed synthetic certificate private key.
// Issuer: FyAgent local review fixture CA. Local loopback TLS tests only;
// this is not a release signing key. Both exact Git path and bytes must match.
const TEST_KEY_PATH =
  "src-tauri/src/services/skill/update/fixtures/server-key.pem";
const TEST_KEY_SHA256 =
  "9eb178442a8aea2b23b54ca8b85ddfb8b78272ac6f3a2f5a2e106dab17fbed2f";

function isTestKeyPath(rawPath) {
  return rawPath !== null && rawPath.equals(Buffer.from(TEST_KEY_PATH));
}

const SECRET_PATTERNS = Object.freeze([
  {
    category: "openai-api-key",
    pattern: /\bsk-(?:proj-|svcacct-)?[A-Za-z0-9_-]{20,}\b/gu,
  },
  {
    category: "github-token",
    pattern: /\bgh[opusr]_[A-Za-z0-9]{20,}\b/gu,
  },
  {
    category: "aws-access-key",
    pattern: /\b(?:AKIA|ASIA)[A-Z0-9]{16}\b/gu,
  },
  {
    category: "private-key",
    pattern: /-----BEGIN (?:RSA |EC |OPENSSH |DSA )?PRIVATE KEY-----/gu,
  },
]);

class SafeScanFailure extends Error {
  constructor(category) {
    super(category);
    this.name = "SafeScanFailure";
    this.category = category;
  }
}

function fail(category) {
  throw new SafeScanFailure(category);
}

function runGit(args, input, failureCategory) {
  const result = spawnSync("git", args, {
    cwd: process.cwd(),
    encoding: null,
    input,
    maxBuffer: MAX_GIT_OUTPUT_BYTES,
    shell: false,
    windowsHide: true,
    env: {
      ...process.env,
      GIT_OPTIONAL_LOCKS: "0",
      LC_ALL: "C",
      LANG: "C",
    },
  });

  if (
    result.error !== undefined ||
    result.signal !== null ||
    result.status !== 0 ||
    !Buffer.isBuffer(result.stdout) ||
    !Buffer.isBuffer(result.stderr)
  ) {
    fail(failureCategory);
  }
  return result.stdout;
}

function splitNul(buffer, failureCategory) {
  if (buffer.length === 0) return [];
  if (buffer[buffer.length - 1] !== 0) fail(failureCategory);
  const records = [];
  let start = 0;
  for (let cursor = 0; cursor < buffer.length; cursor += 1) {
    if (buffer[cursor] !== 0) continue;
    records.push(buffer.subarray(start, cursor));
    start = cursor + 1;
  }
  return records;
}

function parseAscii(buffer, pattern, failureCategory) {
  const value = buffer.toString("ascii");
  if (!pattern.test(value) || !Buffer.from(value, "ascii").equals(buffer)) {
    fail(failureCategory);
  }
  return value;
}

function objectFormat() {
  const output = runGit(
    ["rev-parse", "--show-object-format"],
    undefined,
    "repository-format-failed",
  );
  const value = output.toString("ascii").trim();
  if (value === "sha1") return { length: 40, pattern: /^[0-9a-f]{40}$/u };
  if (value === "sha256") return { length: 64, pattern: /^[0-9a-f]{64}$/u };
  fail("repository-format-invalid");
}

function countMatches(text, pattern) {
  pattern.lastIndex = 0;
  let count = 0;
  while (pattern.exec(text) !== null) count += 1;
  pattern.lastIndex = 0;
  return count;
}

export function classifications(bytes, rawPath = null, allPathsAllowed = true) {
  const text = bytes.toString("latin1");
  const results = [];
  for (const { category, pattern } of SECRET_PATTERNS) {
    if (
      category === "private-key" &&
      allPathsAllowed &&
      isTestKeyPath(rawPath) &&
      createHash("sha256").update(bytes).digest("hex") === TEST_KEY_SHA256
    ) {
      continue;
    }
    const count = countMatches(text, pattern);
    if (count > 0) results.push({ category, count });
  }
  return results;
}

function sanitizePath(rawPath) {
  if (rawPath === null) return null;
  const decoded = rawPath.toString("utf8");
  if (!Buffer.from(decoded, "utf8").equals(rawPath)) return REDACTED_PATH;
  if (decoded.length === 0 || /[\p{Cc}\p{Cf}]/u.test(decoded)) {
    return REDACTED_PATH;
  }
  if (
    /(?:^|[^A-Za-z0-9])(?:[A-Za-z]:[\\/])?Users[\\/](?!<)[^\\/\s]+/iu.test(
      decoded,
    ) ||
    /(?:^|[\\/])(?:home|Users)[\\/](?!<)[^\\/\s]+/u.test(decoded) ||
    classifications(Buffer.from(decoded, "utf8")).length > 0
  ) {
    return REDACTED_PATH;
  }
  return decoded;
}

function parseCurrentTree(treeOid, oidFormat) {
  const output = runGit(
    ["ls-tree", "-r", "-z", "--full-tree", treeOid],
    undefined,
    "current-enumeration-failed",
  );
  const objects = new Map();

  for (const record of splitNul(output, "current-enumeration-invalid")) {
    const tab = record.indexOf(0x09);
    if (tab < 0) fail("current-enumeration-invalid");
    const metadata = parseAscii(
      record.subarray(0, tab),
      /^(?:100644|100755|120000) blob [0-9a-f]{40,64}$/u,
      "current-enumeration-invalid",
    );
    const [mode, type, oid] = metadata.split(" ");
    if (
      !["100644", "100755", "120000"].includes(mode) ||
      type !== "blob" ||
      !oidFormat.pattern.test(oid)
    ) {
      fail("current-enumeration-invalid");
    }
    const rawPath = record.subarray(tab + 1);
    if (rawPath.length === 0) fail("current-enumeration-invalid");
    const existing = objects.get(oid);
    if (existing === undefined) {
      objects.set(oid, {
        oid,
        rawPath,
        pathCount: 1,
        allPathsAllowed: isTestKeyPath(rawPath),
      });
    } else {
      existing.pathCount += 1;
      existing.allPathsAllowed &&= isTestKeyPath(rawPath);
    }
  }
  return objects;
}

export function parseHistoryObjectIds(oidOutput, oidFormat) {
  // OIDs cannot contain LF. This authority works with Git versions that do
  // not implement rev-list's newer NUL/name-metadata protocol.
  if (oidOutput.length > 0 && oidOutput[oidOutput.length - 1] !== 0x0a) {
    fail("history-enumeration-invalid");
  }
  const records = [];
  let start = 0;
  for (let cursor = 0; cursor < oidOutput.length; cursor += 1) {
    if (oidOutput[cursor] !== 0x0a) continue;
    records.push(oidOutput.subarray(start, cursor));
    start = cursor + 1;
  }
  const orderedOids = records.map((record) =>
    parseAscii(record, oidFormat.pattern, "history-enumeration-invalid"),
  );
  if (new Set(orderedOids).size !== orderedOids.length) {
    fail("history-enumeration-invalid");
  }
  return orderedOids;
}

function parseHistory(oidFormat) {
  const oidOutput = runGit(
    ["rev-list", "--objects", "--all", "--no-object-names"],
    undefined,
    "history-enumeration-failed",
  );
  const orderedOids = parseHistoryObjectIds(oidOutput, oidFormat);

  return new Map(
    orderedOids.map((oid) => [oid, { oid, rawPath: null, pathCount: 0 }]),
  );
}

function batchCheck(objects, oidFormat) {
  const ordered = [...objects.values()].sort((left, right) =>
    left.oid.localeCompare(right.oid),
  );
  if (ordered.length === 0) return [];
  const input = Buffer.from(
    `${ordered.map(({ oid }) => oid).join("\n")}\n`,
    "ascii",
  );
  const output = runGit(
    ["cat-file", "--batch-check=%(objectname) %(objecttype) %(objectsize)"],
    input,
    "object-metadata-failed",
  );
  const lines = output.toString("ascii").split("\n");
  if (lines.pop() !== "" || lines.length !== ordered.length) {
    fail("object-metadata-invalid");
  }

  const metadata = [];
  for (let index = 0; index < ordered.length; index += 1) {
    const match = lines[index].match(
      /^([0-9a-f]{40,64}) (blob|tree|commit|tag) ([0-9]+)$/u,
    );
    if (match === null || match[1] !== ordered[index].oid) {
      fail("object-metadata-invalid");
    }
    const [, oid, type, rawSize] = match;
    if (!oidFormat.pattern.test(oid)) fail("object-metadata-invalid");
    const size = Number(rawSize);
    if (!Number.isSafeInteger(size) || size < 0) {
      fail("object-metadata-invalid");
    }
    if ((type === "blob" || type === "tree") && size > MAX_BLOB_BYTES) {
      fail("object-size-unsupported");
    }
    metadata.push({ ...ordered[index], type, size });
  }
  return metadata;
}

function blobBatches(blobs) {
  const batches = [];
  let batch = [];
  let bytes = 0;
  for (const blob of blobs) {
    if (batch.length > 0 && bytes + blob.size > TARGET_BATCH_BYTES) {
      batches.push(batch);
      batch = [];
      bytes = 0;
    }
    batch.push(blob);
    bytes += blob.size;
  }
  if (batch.length > 0) batches.push(batch);
  return batches;
}

function readObjectBatch(batch, oidFormat, visit) {
  const input = Buffer.from(
    `${batch.map(({ oid }) => oid).join("\n")}\n`,
    "ascii",
  );
  const output = runGit(["cat-file", "--batch"], input, "object-read-failed");
  let cursor = 0;

  for (const expected of batch) {
    const lineEnd = output.indexOf(0x0a, cursor);
    if (lineEnd < 0) fail("object-read-invalid");
    const header = parseAscii(
      output.subarray(cursor, lineEnd),
      /^[0-9a-f]{40,64} (?:blob|tree) [0-9]+$/u,
      "object-read-invalid",
    );
    const [oid, type, rawSize] = header.split(" ");
    const size = Number(rawSize);
    if (
      !oidFormat.pattern.test(oid) ||
      oid !== expected.oid ||
      type !== expected.type ||
      size !== expected.size
    ) {
      fail("object-read-invalid");
    }
    const bodyStart = lineEnd + 1;
    const bodyEnd = bodyStart + size;
    if (bodyEnd >= output.length || output[bodyEnd] !== 0x0a) {
      fail("object-read-invalid");
    }
    visit(expected, output.subarray(bodyStart, bodyEnd));
    cursor = bodyEnd + 1;
  }
  if (cursor !== output.length) fail("object-read-invalid");
}

function attachHistoryPaths(metadata, oidFormat, orderedOids) {
  const objects = new Map(metadata.map((object) => [object.oid, object]));
  const trees = new Map();
  const childTrees = new Set();
  const oidBytes = oidFormat.length / 2;

  // Tree bodies are binary: mode SP name NUL raw-OID. Unlike rev-list names,
  // they preserve filenames containing LF or OID-shaped lines unambiguously.
  for (const batch of blobBatches(
    metadata.filter(({ type }) => type === "tree"),
  )) {
    readObjectBatch(batch, oidFormat, (tree, bytes) => {
      const entries = [];
      let cursor = 0;
      while (cursor < bytes.length) {
        const space = bytes.indexOf(0x20, cursor);
        const nul = bytes.indexOf(0, space + 1);
        if (
          space < cursor ||
          nul <= space + 1 ||
          nul + 1 + oidBytes > bytes.length
        ) {
          fail("history-path-enumeration-invalid");
        }
        const mode = parseAscii(
          bytes.subarray(cursor, space),
          /^(?:40000|100644|100755|120000|160000)$/u,
          "history-path-enumeration-invalid",
        );
        const name = bytes.subarray(space + 1, nul);
        if (
          name.includes(0x2f) ||
          name.equals(Buffer.from(".")) ||
          name.equals(Buffer.from(".."))
        ) {
          fail("history-path-enumeration-invalid");
        }
        const oid = bytes.subarray(nul + 1, nul + 1 + oidBytes).toString("hex");
        cursor = nul + 1 + oidBytes;
        // A Gitlink is an external commit, not an object reachable in this repo.
        if (mode === "160000") continue;
        const object = objects.get(oid);
        if (
          object === undefined ||
          object.type !== (mode === "40000" ? "tree" : "blob")
        ) {
          fail("history-path-enumeration-invalid");
        }
        if (object.type === "tree") childTrees.add(oid);
        entries.push({ oid, name });
      }
      trees.set(tree.oid, entries);
    });
  }

  const pending = orderedOids
    .filter((oid) => trees.has(oid) && !childTrees.has(oid))
    .reverse()
    .map((oid) => ({ oid, prefix: Buffer.alloc(0) }));
  const visited = new Set();
  const visitedTrees = new Set();
  while (pending.length > 0) {
    const { oid, prefix } = pending.pop();
    // Revisit shared trees when their prefix can reach the exact exception.
    // All other prefixes share one non-allowlisted state per tree.
    const context = Buffer.from(TEST_KEY_PATH)
      .subarray(0, prefix.length)
      .equals(prefix)
      ? prefix.toString("hex")
      : "other";
    const visitKey = `${oid}:${context}`;
    if (visited.has(visitKey)) continue;
    visited.add(visitKey);
    visitedTrees.add(oid);
    for (const entry of trees.get(oid)) {
      const rawPath = Buffer.concat([prefix, entry.name]);
      const object = objects.get(entry.oid);
      if (object.type === "tree") {
        pending.push({
          oid: entry.oid,
          prefix: Buffer.concat([rawPath, Buffer.from("/")]),
        });
      } else {
        if (object.rawPath === null) {
          object.rawPath = rawPath;
          object.pathCount = 1;
          object.allPathsAllowed = isTestKeyPath(rawPath);
        } else {
          object.allPathsAllowed &&= isTestKeyPath(rawPath);
        }
      }
    }
  }
  if (visitedTrees.size !== trees.size)
    fail("history-path-enumeration-invalid");
}

function resolveTreeish(treeish, oidFormat) {
  if (
    typeof treeish !== "string" ||
    treeish.length === 0 ||
    treeish.includes("\0")
  ) {
    fail("invalid-arguments");
  }
  const output = runGit(
    ["rev-parse", "--verify", "--end-of-options", `${treeish}^{tree}`],
    undefined,
    "treeish-resolution-failed",
  );
  const oid = output.toString("ascii").trim();
  if (
    !oidFormat.pattern.test(oid) ||
    output.toString("ascii").trimEnd() !== oid
  ) {
    fail("treeish-resolution-invalid");
  }
  return oid;
}

function parseArguments(args) {
  if (args.length === 3 && args[0] === "current" && args[1] === "--treeish") {
    return { mode: "current", treeish: args[2] };
  }
  if (args.length === 1 && args[0] === "history") {
    return { mode: "history", treeish: null };
  }
  fail("invalid-arguments");
}

function emptyReport(mode = "invalid") {
  return {
    scannerVersion: SCANNER_VERSION,
    mode,
    sourceOid: null,
    counts: { objects: 0, blobs: 0, paths: 0, findings: 0 },
    findings: [],
    sizes: [],
    failures: [],
  };
}

export function scanRepository(args) {
  const parsed = parseArguments(args);
  const report = emptyReport(parsed.mode);
  const oidFormat = objectFormat();
  let objects;

  if (parsed.mode === "current") {
    report.sourceOid = resolveTreeish(parsed.treeish, oidFormat);
    objects = parseCurrentTree(report.sourceOid, oidFormat);
  } else {
    objects = parseHistory(oidFormat);
  }

  const metadata = batchCheck(objects, oidFormat);
  if (parsed.mode === "history") {
    attachHistoryPaths(metadata, oidFormat, [...objects.keys()]);
  }
  const blobs = metadata.filter(({ type }) => type === "blob");
  report.counts.objects = objects.size;
  report.counts.blobs = blobs.length;
  report.counts.paths = blobs.reduce((sum, blob) => sum + blob.pathCount, 0);

  for (const batch of blobBatches(blobs)) {
    readObjectBatch(batch, oidFormat, (blob, bytes) => {
      const safePath = sanitizePath(blob.rawPath);
      report.sizes.push({
        category: "blob-size",
        path: safePath,
        oid: blob.oid,
        count: blob.pathCount,
        size: blob.size,
      });
      for (const finding of classifications(
        bytes,
        blob.rawPath,
        blob.allPathsAllowed,
      )) {
        report.findings.push({
          category: finding.category,
          path: safePath,
          oid: blob.oid,
          count: finding.count,
          size: blob.size,
        });
      }
    });
  }

  report.findings.sort((left, right) =>
    `${left.oid}:${left.category}`.localeCompare(
      `${right.oid}:${right.category}`,
    ),
  );
  report.sizes.sort((left, right) => left.oid.localeCompare(right.oid));
  report.counts.findings = report.findings.reduce(
    (sum, finding) => sum + finding.count,
    0,
  );
  return report;
}

function safeFailureReport(error) {
  const report = emptyReport();
  report.failures.push({
    category:
      error instanceof SafeScanFailure ? error.category : "internal-failure",
    path: null,
    oid: null,
    count: 1,
    size: 0,
  });
  return report;
}

function main() {
  try {
    process.stdout.write(
      `${JSON.stringify(scanRepository(process.argv.slice(2)))}\n`,
    );
  } catch (error) {
    process.stderr.write(`${JSON.stringify(safeFailureReport(error))}\n`);
    process.exitCode = 1;
  }
}

if (
  process.argv[1] !== undefined &&
  path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)
) {
  main();
}
