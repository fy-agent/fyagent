import { readFileSync } from "node:fs";
import { pathToFileURL } from "node:url";
import { assertProductJobsPreserved } from "./windows-formal-offline-admission.mjs";

export const DEV_RELEASE_ELIGIBILITY_INPUT_SCHEMA =
  "fyagent-dev-release-eligibility-input/v2";
export const EXPECTED_REPOSITORY = "fy-agent/fyagent";
export const EXPECTED_REPOSITORY_ID = "1313497021";
export const RELEASE_WORKFLOW_NAME = "Release";
export const RELEASE_WORKFLOW_PATH = ".github/workflows/release.yml";
export const CI_WORKFLOW_NAME = "CI";
export const CI_WORKFLOW_PATH = ".github/workflows/ci.yml";
export const PREFLIGHT_WORKFLOW_BRANCH = "main";
export const PREFLIGHT_WORKFLOW_REF = `refs/heads/${PREFLIGHT_WORKFLOW_BRANCH}`;
export const FORMAL_BRANCH = "main";
export const FORMAL_REF = `refs/heads/${FORMAL_BRANCH}`;
export const REQUIRED_JOB_NAME = "CI / Required";
export const FORMAL_REPAIR_RELEASE_TAG = "v0.4.15";
export const FORMAL_REPAIR_SOURCE_SHA =
  "b581222135e3db66e0e6e3fdba908662bc60dcda";

// Paths that may differ between the v0.4.15 tag and this repair workflow commit.
// This table is not WORKFLOW_ONLY_PATHS. Admission manifests, signing policy,
// version manifests, release notes, installers, and signer implementations
// stay forbidden here even when the global workflow allowlist names them.
export const FORMAL_REPAIR_PATHS = Object.freeze([
  ".github/workflows/release.yml",
  ".trellis/spec/backend/github-release-workflow.md",
  "scripts/release/dev-release-eligibility.d.mts",
  "scripts/release/dev-release-eligibility.mjs",
  "scripts/release/generate-build-metadata.mjs",
  "scripts/release/release-contract.d.mts",
  "scripts/release/release-contract.mjs",
  "scripts/release/verify-dev-release-remote.d.mts",
  "scripts/release/verify-dev-release-remote.mjs",
  "scripts/release/windows-formal-offline-admission.mjs",
  "tests/devReleaseEligibility.test.ts",
  "tests/devReleaseRemote.test.ts",
  "tests/releaseWorkflow.test.ts",
  "tests/windowsFormalOfflineAdmission.test.ts",
]);

const FORMAL_REPAIR_FORBIDDEN_EXACT = new Set([
  "docs/release/windows-formal-offline-admission.json",
  "docs/release/windows-offline-signing-policy.json",
  "package.json",
  "pnpm-lock.yaml",
  "scripts/release/create-macos-dmg.sh",
  "scripts/release/macos-developer-id.sh",
  "scripts/release/windows-signing.mjs",
  "scripts/version.mjs",
  "src-tauri/Cargo.lock",
  "src-tauri/Cargo.toml",
  "src-tauri/tauri.conf.json",
  "src-tauri/user-helper/Cargo.toml",
]);

const SHA_PATTERN = /^[0-9a-f]{40}$/;
const DECIMAL_PATTERN = /^(0|[1-9]\d*)$/;
const POSITIVE_DECIMAL_PATTERN = /^[1-9]\d*$/;
const STABLE_VERSION_PATTERN = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/;
const CARGO_VERSION_COMPONENT_MAX = 18_446_744_073_709_551_615n;

function fail(message) {
  throw new Error(`Dev release eligibility rejected: ${message}`);
}

function isRecord(value) {
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    return false;
  }
  const prototype = Object.getPrototypeOf(value);
  return prototype === Object.prototype || prototype === null;
}

function expectRecord(value, label) {
  if (!isRecord(value)) fail(`${label} must be a plain object`);
  return value;
}

function expectExactKeys(value, keys, label) {
  const record = expectRecord(value, label);
  const actual = Object.keys(record).sort();
  const expected = [...keys].sort();
  if (
    actual.length !== expected.length ||
    actual.some((key, index) => key !== expected[index])
  ) {
    fail(
      `${label} must contain exactly ${expected.join(", ")}; received ${actual.join(", ")}`,
    );
  }
  return record;
}

function expectString(value, label) {
  if (typeof value !== "string" || value.length === 0) {
    fail(`${label} must be a non-empty string`);
  }
  return value;
}

function expectEqual(actual, expected, label) {
  if (actual !== expected) {
    fail(
      `${label} must be ${JSON.stringify(expected)}; received ${JSON.stringify(actual)}`,
    );
  }
}

function expectSha(value, label) {
  if (typeof value !== "string" || !SHA_PATTERN.test(value)) {
    fail(`${label} must be a lowercase full 40-character Git commit SHA`);
  }
  return value;
}

function assertRepairPathTable() {
  const seen = new Set();
  for (const candidate of FORMAL_REPAIR_PATHS) {
    if (seen.has(candidate)) {
      fail(`formal repair path table repeats ${candidate}`);
    }
    seen.add(candidate);
    if (
      FORMAL_REPAIR_FORBIDDEN_EXACT.has(candidate) ||
      candidate.startsWith("docs/release-notes/") ||
      candidate.startsWith("src/") ||
      candidate.startsWith("src-tauri/")
    ) {
      fail(`formal repair path table includes a forbidden release input: ${candidate}`);
    }
  }
}

assertRepairPathTable();

function parseRepairDiffPaths(diffStatusText) {
  if (typeof diffStatusText !== "string") {
    fail("formal repair diff must be text");
  }
  const paths = [];
  const lines = diffStatusText.endsWith("\n")
    ? diffStatusText.slice(0, -1).split("\n")
    : diffStatusText.split("\n");
  if (lines.length === 1 && lines[0] === "") return [];
  for (const line of lines) {
    if (line.length === 0) fail("formal repair diff contains a blank line");
    const parts = line.split("\t");
    const status = parts[0];
    if (!/^(?:[AMD]|R\d{3}|C\d{3})$/u.test(status) || parts.length < 2) {
      fail(`git diff status is unsupported: ${line}`);
    }
    const changed = parts.slice(1);
    const expectedCount =
      status.startsWith("R") || status.startsWith("C") ? 2 : 1;
    if (changed.length !== expectedCount) {
      fail(`git diff status is unsupported: ${line}`);
    }
    for (const candidate of changed) {
      if (candidate.startsWith('"') || candidate.includes("\\")) {
        fail("git diff path must be unquoted; pass core.quotePath=false");
      }
      if (
        FORMAL_REPAIR_FORBIDDEN_EXACT.has(candidate) ||
        candidate.startsWith("docs/release-notes/") ||
        candidate.startsWith("src/") ||
        candidate.startsWith("src-tauri/")
      ) {
        fail(
          `Formal repair diff includes a forbidden release input: ${candidate}`,
        );
      }
      if (!FORMAL_REPAIR_PATHS.includes(candidate)) {
        fail(
          `Formal repair diff includes a path outside the repair table: ${candidate}`,
        );
      }
      paths.push(candidate);
    }
  }
  return [...new Set(paths)].sort((left, right) =>
    left.localeCompare(right, "en"),
  );
}

export function assertFormalRepairDifference({
  sourceSha,
  workflowSha,
  ancestor,
  diffStatus,
  tagWorkflow,
  executingWorkflow,
}) {
  const source = expectSha(sourceSha, "formal repair source");
  const workflow = expectSha(workflowSha, "formal repair workflow");
  if (source !== FORMAL_REPAIR_SOURCE_SHA) {
    fail("Formal repair source must be the v0.4.15 tag commit");
  }
  if (source === workflow) {
    fail("formal repair workflow.sha must differ from the tag commit");
  }
  if (ancestor !== true) {
    fail(
      "Formal repair workflow commit is not a descendant of the tag commit",
    );
  }
  const paths = parseRepairDiffPaths(diffStatus);
  const expected = [...FORMAL_REPAIR_PATHS].sort((left, right) =>
    left.localeCompare(right, "en"),
  );
  if (
    paths.length !== expected.length ||
    paths.some((candidate, index) => candidate !== expected[index])
  ) {
    fail(
      `Formal repair diff must be exactly ${expected.join(", ")}; received ${paths.join(", ") || "<empty>"}`,
    );
  }
  if (typeof tagWorkflow !== "string" || typeof executingWorkflow !== "string") {
    fail("formal repair workflows must be text");
  }
  assertProductJobsPreserved(tagWorkflow, executingWorkflow);
}

function expectDecimal(value, label, { positive = false } = {}) {
  const pattern = positive ? POSITIVE_DECIMAL_PATTERN : DECIMAL_PATTERN;
  if (typeof value !== "string" || !pattern.test(value)) {
    fail(
      `${label} must be a canonical${positive ? " positive" : ""} decimal string`,
    );
  }
  return value;
}

function validateRepository(value, label) {
  const repository = expectExactKeys(value, ["id", "nameWithOwner"], label);
  expectEqual(
    expectString(repository.nameWithOwner, `${label}.nameWithOwner`),
    EXPECTED_REPOSITORY,
    `${label}.nameWithOwner`,
  );
  expectEqual(
    expectDecimal(repository.id, `${label}.id`, { positive: true }),
    EXPECTED_REPOSITORY_ID,
    `${label}.id`,
  );
}

function validateCandidate(value) {
  const candidate = expectExactKeys(
    value,
    ["canonicalVersion", "releaseTag", "sourceSha"],
    "candidate",
  );
  const canonicalVersion = expectString(
    candidate.canonicalVersion,
    "candidate.canonicalVersion",
  );
  if (!STABLE_VERSION_PATTERN.test(canonicalVersion)) {
    fail(
      "candidate.canonicalVersion must be stable SemVer X.Y.Z without leading zeros",
    );
  }
  if (
    canonicalVersion
      .split(".")
      .some((component) => BigInt(component) > CARGO_VERSION_COMPONENT_MAX)
  ) {
    fail(
      `candidate.canonicalVersion components must not exceed ${CARGO_VERSION_COMPONENT_MAX}`,
    );
  }
  const releaseTag = expectString(candidate.releaseTag, "candidate.releaseTag");
  if (!/^v(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.test(releaseTag)) {
    fail("candidate.releaseTag must be a strict stable tag vX.Y.Z");
  }
  expectEqual(releaseTag, `v${canonicalVersion}`, "candidate.releaseTag");
  return {
    canonicalVersion,
    releaseTag,
    sourceSha: expectSha(candidate.sourceSha, "candidate.sourceSha"),
  };
}

function validateEvent(value) {
  const event = expectExactKeys(
    value,
    [
      "dispatchMode",
      "dispatchSourceSha",
      "name",
      "ref",
      "refName",
      "refType",
      "sha",
    ],
    "event",
  );
  return {
    dispatchMode:
      event.dispatchMode === null
        ? null
        : expectString(event.dispatchMode, "event.dispatchMode"),
    dispatchSourceSha: event.dispatchSourceSha,
    name: expectString(event.name, "event.name"),
    ref: expectString(event.ref, "event.ref"),
    refName: expectString(event.refName, "event.refName"),
    refType: expectString(event.refType, "event.refType"),
    sha: expectSha(event.sha, "event.sha"),
  };
}

function validateWorkflow(value) {
  const workflow = expectExactKeys(
    value,
    ["name", "path", "ref", "sha"],
    "workflow",
  );
  expectEqual(
    expectString(workflow.name, "workflow.name"),
    RELEASE_WORKFLOW_NAME,
    "workflow.name",
  );
  expectEqual(
    expectString(workflow.path, "workflow.path"),
    RELEASE_WORKFLOW_PATH,
    "workflow.path",
  );
  return {
    ref: expectString(workflow.ref, "workflow.ref"),
    sha: expectSha(workflow.sha, "workflow.sha"),
  };
}

function validateRemoteDev(value, authorityBranch) {
  const remoteDev = expectExactKeys(
    value,
    ["headSha", "name", "ref"],
    "remoteDev",
  );
  expectEqual(
    expectString(remoteDev.name, "remoteDev.name"),
    authorityBranch,
    "remoteDev.name",
  );
  expectEqual(
    expectString(remoteDev.ref, "remoteDev.ref"),
    `refs/heads/${authorityBranch}`,
    "remoteDev.ref",
  );
  return expectSha(remoteDev.headSha, "remoteDev.headSha");
}

function validateRemoteTag(value, releaseTag, sourceSha) {
  const remoteTag = expectExactKeys(
    value,
    ["ref", "refObject", "tagObject"],
    "remoteTag",
  );
  expectEqual(
    expectString(remoteTag.ref, "remoteTag.ref"),
    `refs/tags/${releaseTag}`,
    "remoteTag.ref",
  );

  const refObject = expectExactKeys(
    remoteTag.refObject,
    ["sha", "type"],
    "remoteTag.refObject",
  );
  const refType = expectString(refObject.type, "remoteTag.refObject.type");
  const refObjectSha = expectSha(refObject.sha, "remoteTag.refObject.sha");

  if (refType === "commit") {
    if (remoteTag.tagObject !== null) {
      fail("lightweight remoteTag.tagObject must be null");
    }
    expectEqual(refObjectSha, sourceSha, "remoteTag.refObject.sha");
    return;
  }

  expectEqual(refType, "tag", "remoteTag.refObject.type");

  const tagObject = expectExactKeys(
    remoteTag.tagObject,
    ["name", "sha", "target"],
    "remoteTag.tagObject",
  );
  expectEqual(
    expectSha(tagObject.sha, "remoteTag.tagObject.sha"),
    refObjectSha,
    "remoteTag.tagObject.sha",
  );
  expectEqual(
    expectString(tagObject.name, "remoteTag.tagObject.name"),
    releaseTag,
    "remoteTag.tagObject.name",
  );
  const target = expectExactKeys(
    tagObject.target,
    ["sha", "type"],
    "remoteTag.tagObject.target",
  );
  expectEqual(
    expectString(target.type, "remoteTag.tagObject.target.type"),
    "commit",
    "remoteTag.tagObject.target.type",
  );
  expectEqual(
    expectSha(target.sha, "remoteTag.tagObject.target.sha"),
    sourceSha,
    "remoteTag.tagObject.target.sha",
  );
}

function assertExpectedFrozenOutput(output, expectedValue) {
  const expected = expectExactKeys(
    expectedValue,
    [
      "appVersion",
      "ciRunAttempt",
      "ciRunId",
      "mode",
      "releaseTag",
      "repair",
      "sourceSha",
      "workflowSha",
    ],
    "expectedFrozen",
  );
  for (const key of Object.keys(output)) {
    expectEqual(expected[key], output[key], `expectedFrozen.${key}`);
  }
}

export function evaluateDevReleaseEligibility(inputValue, expectedFrozen) {
  const input = expectExactKeys(
    inputValue,
    [
      "candidate",
      "event",
      "remoteDev",
      "remoteTag",
      "repository",
      "schema",
      "workflow",
    ],
    "input",
  );
  expectEqual(
    expectString(input.schema, "input.schema"),
    DEV_RELEASE_ELIGIBILITY_INPUT_SCHEMA,
    "input.schema",
  );
  validateRepository(input.repository, "repository");
  const candidate = validateCandidate(input.candidate);
  const event = validateEvent(input.event);
  const workflow = validateWorkflow(input.workflow);
  expectEqual(event.sha, workflow.sha, "event.sha");

  const expectedWorkflowRefPrefix = `${EXPECTED_REPOSITORY}/${RELEASE_WORKFLOW_PATH}@`;
  let mode;
  let repair = false;
  let authorityBranch;
  const validateFormalIdentity = () => {
    mode = "formal";
    repair = false;
    authorityBranch = FORMAL_BRANCH;
    expectEqual(event.sha, candidate.sourceSha, "event.sha");
    expectEqual(workflow.sha, candidate.sourceSha, "workflow.sha");
    expectEqual(event.dispatchSourceSha, null, "event.dispatchSourceSha");
    const tagRef = `refs/tags/${candidate.releaseTag}`;
    expectEqual(event.ref, tagRef, "event.ref");
    expectEqual(event.refType, "tag", "event.refType");
    expectEqual(event.refName, candidate.releaseTag, "event.refName");
    expectEqual(
      workflow.ref,
      `${expectedWorkflowRefPrefix}${tagRef}`,
      "workflow.ref",
    );
    if (input.remoteTag === null) {
      fail("formal release requires remoteTag evidence");
    }
    validateRemoteTag(
      input.remoteTag,
      candidate.releaseTag,
      candidate.sourceSha,
    );
  };
  const validateFormalRepairIdentity = () => {
    mode = "formal";
    repair = true;
    authorityBranch = FORMAL_BRANCH;
    expectEqual(
      candidate.canonicalVersion,
      "0.4.15",
      "candidate.canonicalVersion",
    );
    expectEqual(
      candidate.releaseTag,
      FORMAL_REPAIR_RELEASE_TAG,
      "candidate.releaseTag",
    );
    expectEqual(
      candidate.sourceSha,
      FORMAL_REPAIR_SOURCE_SHA,
      "candidate.sourceSha",
    );
    expectEqual(
      expectSha(event.dispatchSourceSha, "event.dispatchSourceSha"),
      candidate.sourceSha,
      "event.dispatchSourceSha",
    );
    expectEqual(event.ref, FORMAL_REF, "event.ref");
    expectEqual(event.refType, "branch", "event.refType");
    expectEqual(event.refName, FORMAL_BRANCH, "event.refName");
    expectEqual(
      workflow.ref,
      `${expectedWorkflowRefPrefix}${FORMAL_REF}`,
      "workflow.ref",
    );
    if (workflow.sha === candidate.sourceSha) {
      fail("formal repair workflow.sha must differ from the tag commit");
    }
    if (input.remoteTag === null) {
      fail("formal repair requires remoteTag evidence");
    }
    validateRemoteTag(
      input.remoteTag,
      candidate.releaseTag,
      candidate.sourceSha,
    );
  };
  if (event.name === "workflow_dispatch") {
    if (event.dispatchMode === "preflight") {
      mode = "preflight";
      authorityBranch = PREFLIGHT_WORKFLOW_BRANCH;
      expectEqual(
        expectSha(event.dispatchSourceSha, "event.dispatchSourceSha"),
        candidate.sourceSha,
        "event.dispatchSourceSha",
      );
      expectEqual(event.ref, PREFLIGHT_WORKFLOW_REF, "event.ref");
      expectEqual(event.refType, "branch", "event.refType");
      expectEqual(event.refName, PREFLIGHT_WORKFLOW_BRANCH, "event.refName");
      expectEqual(
        workflow.ref,
        `${expectedWorkflowRefPrefix}${PREFLIGHT_WORKFLOW_REF}`,
        "workflow.ref",
      );
      if (input.remoteTag !== null) {
        fail("remoteTag must be null for workflow_dispatch preflight");
      }
    } else if (event.dispatchMode === "formal") {
      validateFormalIdentity();
    } else if (event.dispatchMode === "formal-repair") {
      validateFormalRepairIdentity();
    } else {
      fail(
        `workflow_dispatch event.dispatchMode must be \"preflight\", \"formal\", or \"formal-repair\"; received ${JSON.stringify(event.dispatchMode)}`,
      );
    }
  } else if (event.name === "push") {
    expectEqual(event.dispatchMode, null, "event.dispatchMode");
    expectEqual(event.dispatchSourceSha, null, "event.dispatchSourceSha");
    validateFormalIdentity();
  } else {
    fail(`unsupported event.name ${JSON.stringify(event.name)}`);
  }

  validateRemoteDev(input.remoteDev, authorityBranch);
  const output = Object.freeze({
    appVersion: candidate.canonicalVersion,
    releaseTag: candidate.releaseTag,
    sourceSha: candidate.sourceSha,
    workflowSha: workflow.sha,
    ciRunId: null,
    ciRunAttempt: null,
    mode,
    repair,
  });
  if (expectedFrozen !== undefined) {
    assertExpectedFrozenOutput(output, expectedFrozen);
  }
  return output;
}

function parseCliArguments(argv) {
  let inputPath = null;
  let expectedPath = null;
  let json = false;
  for (let index = 0; index < argv.length; index += 1) {
    const argument = argv[index];
    if (argument === "--input" && inputPath === null) {
      inputPath = argv[index + 1] ?? null;
      index += 1;
    } else if (argument === "--expected" && expectedPath === null) {
      expectedPath = argv[index + 1] ?? null;
      index += 1;
    } else if (argument === "--json" && !json) {
      json = true;
    } else {
      fail(`unknown or duplicate CLI argument ${JSON.stringify(argument)}`);
    }
  }
  if (inputPath === null || inputPath.length === 0 || !json) {
    fail(
      "usage: node dev-release-eligibility.mjs --input <path|-> [--expected <path>] --json",
    );
  }
  if (expectedPath !== null && expectedPath.length === 0) {
    fail("--expected requires a JSON file path");
  }
  return { expectedPath, inputPath };
}

function parseRepairDiffArguments(argv) {
  const values = new Map();
  for (let index = 0; index < argv.length; index += 2) {
    const name = argv[index];
    const value = argv[index + 1];
    if (
      typeof name !== "string" ||
      !name.startsWith("--") ||
      value === undefined ||
      values.has(name)
    ) {
      fail(
        "usage: node dev-release-eligibility.mjs assert-repair-diff --source-sha <sha> --workflow-sha <sha> --ancestor <true|false> --diff <path> --tag-workflow <path> --executing-workflow <path>",
      );
    }
    values.set(name, value);
  }
  const required = [
    "--source-sha",
    "--workflow-sha",
    "--ancestor",
    "--diff",
    "--tag-workflow",
    "--executing-workflow",
  ];
  for (const name of required) {
    if (!values.has(name)) {
      fail(`formal repair diff is missing ${name}`);
    }
  }
  if (values.size !== required.length) {
    fail("formal repair diff received an unknown argument");
  }
  const ancestor = values.get("--ancestor");
  if (ancestor !== "true" && ancestor !== "false") {
    fail("formal repair --ancestor must be true or false");
  }
  return {
    sourceSha: values.get("--source-sha"),
    workflowSha: values.get("--workflow-sha"),
    ancestor: ancestor === "true",
    diffStatus: readFileSync(values.get("--diff"), "utf8"),
    tagWorkflow: readFileSync(values.get("--tag-workflow"), "utf8"),
    executingWorkflow: readFileSync(values.get("--executing-workflow"), "utf8"),
  };
}

function runCli() {
  try {
    if (process.argv[2] === "assert-repair-diff") {
      assertFormalRepairDifference(
        parseRepairDiffArguments(process.argv.slice(3)),
      );
      return;
    }
    const { expectedPath, inputPath } = parseCliArguments(
      process.argv.slice(2),
    );
    const contents = readFileSync(inputPath === "-" ? 0 : inputPath, "utf8");
    const input = JSON.parse(contents);
    const expected =
      expectedPath === null
        ? undefined
        : JSON.parse(readFileSync(expectedPath, "utf8"));
    process.stdout.write(
      `${JSON.stringify(evaluateDevReleaseEligibility(input, expected))}\n`,
    );
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    process.stderr.write(`${message}\n`);
    process.exitCode = 1;
  }
}

if (
  process.argv[1] &&
  pathToFileURL(process.argv[1]).href === import.meta.url
) {
  runCli();
}
