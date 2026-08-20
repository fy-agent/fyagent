import { readFileSync } from "node:fs";
import { pathToFileURL } from "node:url";

export const DEV_RELEASE_ELIGIBILITY_INPUT_SCHEMA =
  "fyagent-dev-release-eligibility-input/v1";
export const EXPECTED_REPOSITORY = "fy-agent/fyagent";
export const EXPECTED_REPOSITORY_ID = "1313497021";
export const RELEASE_WORKFLOW_NAME = "Release";
export const RELEASE_WORKFLOW_PATH = ".github/workflows/release.yml";
export const CI_WORKFLOW_NAME = "CI";
export const CI_WORKFLOW_PATH = ".github/workflows/ci.yml";
export const DEV_BRANCH = "dev/laiyongjie";
export const DEV_REF = `refs/heads/${DEV_BRANCH}`;
export const FORMAL_BRANCH = "main";
export const FORMAL_REF = `refs/heads/${FORMAL_BRANCH}`;
export const REQUIRED_JOB_NAME = "CI / Required";

const SHA_PATTERN = /^[0-9a-f]{40}$/;
const DECIMAL_PATTERN = /^(0|[1-9]\d*)$/;
const POSITIVE_DECIMAL_PATTERN = /^[1-9]\d*$/;
const STABLE_VERSION_PATTERN = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/;
const CARGO_VERSION_COMPONENT_MAX = 18_446_744_073_709_551_615n;

const RUN_STATUSES = new Set([
  "completed",
  "in_progress",
  "pending",
  "queued",
  "requested",
  "waiting",
]);
const CONCLUSIONS = new Set([
  "action_required",
  "cancelled",
  "failure",
  "neutral",
  "skipped",
  "stale",
  "startup_failure",
  "success",
  "timed_out",
]);

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

function expectDecimal(value, label, { positive = false } = {}) {
  const pattern = positive ? POSITIVE_DECIMAL_PATTERN : DECIMAL_PATTERN;
  if (typeof value !== "string" || !pattern.test(value)) {
    fail(
      `${label} must be a canonical${positive ? " positive" : ""} decimal string`,
    );
  }
  return value;
}

function expectArray(value, label) {
  if (!Array.isArray(value)) fail(`${label} must be an array`);
  return value;
}

function expectStatus(status, conclusion, label) {
  if (typeof status !== "string" || !RUN_STATUSES.has(status)) {
    fail(`${label}.status is not a recognized GitHub Actions status`);
  }
  if (conclusion !== null && !CONCLUSIONS.has(conclusion)) {
    fail(`${label}.conclusion is not a recognized GitHub Actions conclusion`);
  }
  if (status === "completed" && conclusion === null) {
    fail(`${label}.conclusion cannot be null when status is completed`);
  }
  if (status !== "completed" && conclusion !== null) {
    fail(`${label}.conclusion must be null before status is completed`);
  }
}

function compareDecimal(left, right) {
  const leftValue = BigInt(left);
  const rightValue = BigInt(right);
  return leftValue < rightValue ? -1 : leftValue > rightValue ? 1 : 0;
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
    ["dispatchSourceSha", "name", "ref", "refName", "refType", "sha"],
    "event",
  );
  return {
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
  expectEqual(
    expectString(refObject.type, "remoteTag.refObject.type"),
    "tag",
    "remoteTag.refObject.type",
  );
  const tagObjectSha = expectSha(refObject.sha, "remoteTag.refObject.sha");

  const tagObject = expectExactKeys(
    remoteTag.tagObject,
    ["name", "sha", "target"],
    "remoteTag.tagObject",
  );
  expectEqual(
    expectSha(tagObject.sha, "remoteTag.tagObject.sha"),
    tagObjectSha,
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

function validateCiWorkflow(value) {
  const workflow = expectExactKeys(
    value,
    ["id", "name", "path", "repository", "state"],
    "ciWorkflow",
  );
  validateRepository(workflow.repository, "ciWorkflow.repository");
  expectEqual(
    expectString(workflow.name, "ciWorkflow.name"),
    CI_WORKFLOW_NAME,
    "ciWorkflow.name",
  );
  expectEqual(
    expectString(workflow.path, "ciWorkflow.path"),
    CI_WORKFLOW_PATH,
    "ciWorkflow.path",
  );
  expectEqual(
    expectString(workflow.state, "ciWorkflow.state"),
    "active",
    "ciWorkflow.state",
  );
  return expectDecimal(workflow.id, "ciWorkflow.id", { positive: true });
}

function validateCiRun(value, index, ciWorkflowId, authorityBranch) {
  const label = `ciRuns[${index}]`;
  const run = expectExactKeys(
    value,
    [
      "checkSuiteId",
      "conclusion",
      "event",
      "headBranch",
      "headRepository",
      "headSha",
      "id",
      "repository",
      "runAttempt",
      "runNumber",
      "status",
      "workflow",
    ],
    label,
  );
  validateRepository(run.repository, `${label}.repository`);
  validateRepository(run.headRepository, `${label}.headRepository`);
  const workflow = expectExactKeys(
    run.workflow,
    ["id", "name", "path"],
    `${label}.workflow`,
  );
  expectEqual(
    expectDecimal(workflow.id, `${label}.workflow.id`, { positive: true }),
    ciWorkflowId,
    `${label}.workflow.id`,
  );
  expectEqual(
    expectString(workflow.name, `${label}.workflow.name`),
    CI_WORKFLOW_NAME,
    `${label}.workflow.name`,
  );
  expectEqual(
    expectString(workflow.path, `${label}.workflow.path`),
    CI_WORKFLOW_PATH,
    `${label}.workflow.path`,
  );
  expectEqual(
    expectString(run.event, `${label}.event`),
    "push",
    `${label}.event`,
  );
  expectEqual(
    expectString(run.headBranch, `${label}.headBranch`),
    authorityBranch,
    `${label}.headBranch`,
  );
  expectStatus(run.status, run.conclusion, label);
  return {
    checkSuiteId: expectDecimal(run.checkSuiteId, `${label}.checkSuiteId`, {
      positive: true,
    }),
    conclusion: run.conclusion,
    headSha: expectSha(run.headSha, `${label}.headSha`),
    id: expectDecimal(run.id, `${label}.id`, { positive: true }),
    runAttempt: expectDecimal(run.runAttempt, `${label}.runAttempt`, {
      positive: true,
    }),
    runNumber: expectDecimal(run.runNumber, `${label}.runNumber`, {
      positive: true,
    }),
    status: run.status,
  };
}

function validateCiEvidence(value, selectedRun, sourceSha) {
  const evidence = expectExactKeys(
    value,
    ["checkRuns", "checkSuiteId", "jobs", "runAttempt", "runId"],
    "ciEvidence",
  );
  expectEqual(
    expectDecimal(evidence.runId, "ciEvidence.runId", { positive: true }),
    selectedRun.id,
    "ciEvidence.runId",
  );
  expectEqual(
    expectDecimal(evidence.runAttempt, "ciEvidence.runAttempt", {
      positive: true,
    }),
    selectedRun.runAttempt,
    "ciEvidence.runAttempt",
  );
  expectEqual(
    expectDecimal(evidence.checkSuiteId, "ciEvidence.checkSuiteId", {
      positive: true,
    }),
    selectedRun.checkSuiteId,
    "ciEvidence.checkSuiteId",
  );

  const jobs = expectArray(evidence.jobs, "ciEvidence.jobs").map(
    (jobValue, index) => {
      const label = `ciEvidence.jobs[${index}]`;
      const job = expectExactKeys(
        jobValue,
        [
          "checkRunUrl",
          "conclusion",
          "htmlUrl",
          "id",
          "name",
          "runAttempt",
          "runId",
          "status",
        ],
        label,
      );
      expectEqual(
        expectDecimal(job.runId, `${label}.runId`, { positive: true }),
        selectedRun.id,
        `${label}.runId`,
      );
      expectEqual(
        expectDecimal(job.runAttempt, `${label}.runAttempt`, {
          positive: true,
        }),
        selectedRun.runAttempt,
        `${label}.runAttempt`,
      );
      expectStatus(job.status, job.conclusion, label);
      return {
        checkRunUrl: expectString(job.checkRunUrl, `${label}.checkRunUrl`),
        conclusion: job.conclusion,
        htmlUrl: expectString(job.htmlUrl, `${label}.htmlUrl`),
        id: expectDecimal(job.id, `${label}.id`, { positive: true }),
        name: expectString(job.name, `${label}.name`),
        status: job.status,
      };
    },
  );

  const checkRuns = expectArray(evidence.checkRuns, "ciEvidence.checkRuns").map(
    (checkValue, index) => {
      const label = `ciEvidence.checkRuns[${index}]`;
      const check = expectExactKeys(
        checkValue,
        [
          "appSlug",
          "checkSuiteId",
          "conclusion",
          "detailsUrl",
          "headSha",
          "id",
          "name",
          "runAttempt",
          "runId",
          "status",
          "url",
        ],
        label,
      );
      expectEqual(
        expectDecimal(check.runId, `${label}.runId`, { positive: true }),
        selectedRun.id,
        `${label}.runId`,
      );
      expectEqual(
        expectDecimal(check.runAttempt, `${label}.runAttempt`, {
          positive: true,
        }),
        selectedRun.runAttempt,
        `${label}.runAttempt`,
      );
      expectEqual(
        expectDecimal(check.checkSuiteId, `${label}.checkSuiteId`, {
          positive: true,
        }),
        selectedRun.checkSuiteId,
        `${label}.checkSuiteId`,
      );
      expectStatus(check.status, check.conclusion, label);
      return {
        appSlug: expectString(check.appSlug, `${label}.appSlug`),
        conclusion: check.conclusion,
        detailsUrl: expectString(check.detailsUrl, `${label}.detailsUrl`),
        headSha: expectSha(check.headSha, `${label}.headSha`),
        id: expectDecimal(check.id, `${label}.id`, { positive: true }),
        name: expectString(check.name, `${label}.name`),
        status: check.status,
        url: expectString(check.url, `${label}.url`),
      };
    },
  );

  const requiredJobs = jobs.filter(({ name }) => name === REQUIRED_JOB_NAME);
  if (requiredJobs.length !== 1) {
    fail(
      `selected CI attempt must contain exactly one ${REQUIRED_JOB_NAME} job`,
    );
  }
  const requiredChecks = checkRuns.filter(
    ({ name }) => name === REQUIRED_JOB_NAME,
  );
  if (requiredChecks.length !== 1) {
    fail(
      `selected CI attempt must contain exactly one ${REQUIRED_JOB_NAME} check-run`,
    );
  }

  const job = requiredJobs[0];
  const check = requiredChecks[0];
  if (job.status !== "completed" || job.conclusion !== "success") {
    fail(`${REQUIRED_JOB_NAME} job must be completed successfully`);
  }
  if (check.status !== "completed" || check.conclusion !== "success") {
    fail(`${REQUIRED_JOB_NAME} check-run must be completed successfully`);
  }
  expectEqual(check.appSlug, "github-actions", `${REQUIRED_JOB_NAME} app slug`);
  expectEqual(check.headSha, sourceSha, `${REQUIRED_JOB_NAME} head SHA`);

  const expectedCheckUrl = `https://api.github.com/repos/${EXPECTED_REPOSITORY}/check-runs/${check.id}`;
  const expectedDetailsUrl = `https://github.com/${EXPECTED_REPOSITORY}/actions/runs/${selectedRun.id}/job/${job.id}`;
  expectEqual(
    job.checkRunUrl,
    expectedCheckUrl,
    `${REQUIRED_JOB_NAME} job check URL`,
  );
  expectEqual(check.url, expectedCheckUrl, `${REQUIRED_JOB_NAME} check URL`);
  expectEqual(job.htmlUrl, expectedDetailsUrl, `${REQUIRED_JOB_NAME} job URL`);
  expectEqual(
    check.detailsUrl,
    expectedDetailsUrl,
    `${REQUIRED_JOB_NAME} details URL`,
  );
}

function selectLatestSuccessfulCi(input, sourceSha, authorityBranch) {
  const ciWorkflowId = validateCiWorkflow(input.ciWorkflow);
  const runs = expectArray(input.ciRuns, "ciRuns").map((run, index) =>
    validateCiRun(run, index, ciWorkflowId, authorityBranch),
  );
  const seenAttempts = new Set();
  for (const run of runs) {
    const key = `${run.id}:${run.runAttempt}`;
    if (seenAttempts.has(key)) {
      fail(`ciRuns contains duplicate run attempt ${key}`);
    }
    seenAttempts.add(key);
  }

  const matching = runs
    .filter((run) => run.headSha === sourceSha)
    .sort((left, right) => {
      const runOrder = compareDecimal(left.runNumber, right.runNumber);
      return runOrder === 0
        ? compareDecimal(left.runAttempt, right.runAttempt)
        : runOrder;
    });
  if (matching.length === 0) {
    fail(`no ${authorityBranch} push CI run exists for source ${sourceSha}`);
  }
  const selectedRun = matching.at(-1);
  if (
    selectedRun.status !== "completed" ||
    selectedRun.conclusion !== "success"
  ) {
    fail(
      `latest exact-source ${authorityBranch} push CI run/attempt must be completed successfully`,
    );
  }
  validateCiEvidence(input.ciEvidence, selectedRun, sourceSha);
  return selectedRun;
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
      "ciEvidence",
      "ciRuns",
      "ciWorkflow",
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
  expectEqual(event.sha, candidate.sourceSha, "event.sha");
  expectEqual(workflow.sha, candidate.sourceSha, "workflow.sha");

  const expectedWorkflowRefPrefix = `${EXPECTED_REPOSITORY}/${RELEASE_WORKFLOW_PATH}@`;
  let mode;
  let authorityBranch;
  if (event.name === "workflow_dispatch") {
    mode = "preflight";
    authorityBranch = DEV_BRANCH;
    expectEqual(
      expectSha(event.dispatchSourceSha, "event.dispatchSourceSha"),
      candidate.sourceSha,
      "event.dispatchSourceSha",
    );
    expectEqual(event.ref, DEV_REF, "event.ref");
    expectEqual(event.refType, "branch", "event.refType");
    expectEqual(event.refName, DEV_BRANCH, "event.refName");
    expectEqual(
      workflow.ref,
      `${expectedWorkflowRefPrefix}${DEV_REF}`,
      "workflow.ref",
    );
    if (input.remoteTag !== null) {
      fail("remoteTag must be null for workflow_dispatch preflight");
    }
  } else if (event.name === "push") {
    mode = "formal";
    authorityBranch = FORMAL_BRANCH;
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
      fail("formal release requires annotated remoteTag evidence");
    }
    validateRemoteTag(
      input.remoteTag,
      candidate.releaseTag,
      candidate.sourceSha,
    );
  } else {
    fail(`unsupported event.name ${JSON.stringify(event.name)}`);
  }

  const remoteDevHeadSha = validateRemoteDev(
    input.remoteDev,
    authorityBranch,
  );
  expectEqual(remoteDevHeadSha, candidate.sourceSha, "remoteDev.headSha");
  const selectedRun = selectLatestSuccessfulCi(
    input,
    candidate.sourceSha,
    authorityBranch,
  );
  const output = Object.freeze({
    appVersion: candidate.canonicalVersion,
    releaseTag: candidate.releaseTag,
    sourceSha: candidate.sourceSha,
    workflowSha: workflow.sha,
    ciRunId: selectedRun.id,
    ciRunAttempt: selectedRun.runAttempt,
    mode,
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

function runCli() {
  try {
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
