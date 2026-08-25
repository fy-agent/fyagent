import { readFileSync } from "node:fs";
import { pathToFileURL } from "node:url";
import {
  EXPECTED_REPOSITORY,
  EXPECTED_REPOSITORY_ID,
  RELEASE_WORKFLOW_NAME,
  RELEASE_WORKFLOW_PATH,
} from "./dev-release-eligibility.mjs";

const SHA_PATTERN = /^[0-9a-f]{40}$/u;
const STABLE_TAG_PATTERN = /^v(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/u;
const POSITIVE_DECIMAL_PATTERN = /^[1-9]\d*$/u;
const TRANSACTION_MARKER_PATTERN =
  /<!-- fyagent-release-transaction:run=([1-9]\d*);attempt=([1-9]\d*);source=([0-9a-f]{40}) -->/gu;
const RECOVERABLE_RUN_CONCLUSIONS = new Set([
  "cancelled",
  "failure",
  "timed_out",
]);
const RECOVERABLE_STEP_CONCLUSIONS = new Set(["cancelled", "failure"]);
const PUBLISH_JOB_NAME = "Publish stable GitHub Release";
const TRANSACTION_STEP_NAME =
  "Stage, re-download, and publish one stable Release transaction";

function fail(message) {
  throw new Error(`Release draft ownership rejected: ${message}`);
}

function expectRecord(value, label) {
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    fail(`${label} must be an object`);
  }
  return value;
}

function expectString(value, label) {
  if (typeof value !== "string" || value.length === 0) {
    fail(`${label} must be a non-empty string`);
  }
  return value;
}

function expectPositiveDecimal(value, label) {
  let normalized;
  if (typeof value === "number") {
    if (!Number.isSafeInteger(value) || value <= 0) {
      fail(`${label} must be a positive safe integer`);
    }
    normalized = String(value);
  } else {
    normalized = value;
  }
  if (
    typeof normalized !== "string" ||
    !POSITIVE_DECIMAL_PATTERN.test(normalized)
  ) {
    fail(`${label} must be a canonical positive decimal`);
  }
  return normalized;
}

function expectSha(value, label) {
  if (typeof value !== "string" || !SHA_PATTERN.test(value)) {
    fail(`${label} must be a lowercase full 40-character commit SHA`);
  }
  return value;
}

function expectInstant(value, label) {
  const instant = expectString(value, label);
  const milliseconds = Date.parse(instant);
  if (
    !/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d{1,9})?(?:Z|[+-]\d{2}:\d{2})$/u.test(
      instant,
    ) ||
    !Number.isFinite(milliseconds)
  ) {
    fail(`${label} must be an ISO-8601 instant`);
  }
  return { instant, milliseconds };
}

function validateExpectedIdentity(expectedTag, expectedSourceSha) {
  if (!STABLE_TAG_PATTERN.test(expectedTag)) {
    fail("expected tag must be a stable vX.Y.Z release tag");
  }
  expectSha(expectedSourceSha, "expected source SHA");
}

function validateRepository(value, label) {
  const repository = expectRecord(value, label);
  if (repository.full_name !== EXPECTED_REPOSITORY) {
    fail(`${label}.full_name must be ${EXPECTED_REPOSITORY}`);
  }
  if (
    expectPositiveDecimal(repository.id, `${label}.id`) !==
    EXPECTED_REPOSITORY_ID
  ) {
    fail(`${label}.id must be ${EXPECTED_REPOSITORY_ID}`);
  }
}

export function inspectRecoverableReleaseDraft(
  releaseValue,
  expectedTag,
  expectedSourceSha,
) {
  validateExpectedIdentity(expectedTag, expectedSourceSha);
  const release = expectRecord(releaseValue, "release");
  const releaseId = expectPositiveDecimal(release.id, "release.id");
  if (release.draft !== true) fail("release must still be a draft");
  if (release.prerelease !== false) fail("release must not be a prerelease");
  if (release.tag_name !== expectedTag) {
    fail(`release.tag_name must be ${expectedTag}`);
  }
  if (release.name !== `FyAgent ${expectedTag}`) {
    fail(`release.name must be FyAgent ${expectedTag}`);
  }
  if (release.target_commitish !== expectedSourceSha) {
    fail("release.target_commitish does not match the frozen source SHA");
  }
  if (release.published_at !== null) {
    fail("draft release.published_at must be null");
  }
  const createdAt = expectInstant(release.created_at, "release.created_at");
  const body = expectString(release.body, "release.body");
  const markers = [...body.matchAll(TRANSACTION_MARKER_PATTERN)];
  if (markers.length !== 1) {
    fail("release body must contain exactly one FyAgent transaction marker");
  }
  const [, runId, runAttempt, sourceSha] = markers[0];
  if (sourceSha !== expectedSourceSha) {
    fail("transaction marker source does not match the frozen source SHA");
  }
  const expectedMarker = `<!-- fyagent-release-transaction:run=${runId};attempt=${runAttempt};source=${sourceSha} -->`;
  if (!body.endsWith(expectedMarker)) {
    fail("transaction marker must be the final draft body suffix");
  }
  return Object.freeze({
    releaseId,
    runId,
    runAttempt,
    sourceSha,
    createdAt: createdAt.instant,
  });
}

function validateRunAttempt(runValue, marker, expectedSourceSha) {
  const run = expectRecord(runValue, "workflow run attempt");
  if (
    expectPositiveDecimal(run.id, "workflow run attempt.id") !== marker.runId
  ) {
    fail("workflow run attempt.id does not match the transaction marker");
  }
  if (
    expectPositiveDecimal(
      run.run_attempt,
      "workflow run attempt.run_attempt",
    ) !== marker.runAttempt
  ) {
    fail("workflow run attempt number does not match the transaction marker");
  }
  if (run.name !== RELEASE_WORKFLOW_NAME) {
    fail(`workflow run attempt.name must be ${RELEASE_WORKFLOW_NAME}`);
  }
  const path = expectString(run.path, "workflow run attempt.path");
  if (
    path !== RELEASE_WORKFLOW_PATH &&
    !path.startsWith(`${RELEASE_WORKFLOW_PATH}@`)
  ) {
    fail(`workflow run attempt.path must identify ${RELEASE_WORKFLOW_PATH}`);
  }
  if (!new Set(["push", "workflow_dispatch"]).has(run.event)) {
    fail("workflow run attempt.event must be push or workflow_dispatch");
  }
  if (
    expectSha(run.head_sha, "workflow run attempt.head_sha") !==
    expectedSourceSha
  ) {
    fail("workflow run attempt.head_sha does not match the frozen source SHA");
  }
  validateRepository(run.repository, "workflow run attempt.repository");
  validateRepository(
    run.head_repository,
    "workflow run attempt.head_repository",
  );
  if (run.status !== "completed") {
    fail("originating workflow run attempt must be completed before recovery");
  }
  if (!RECOVERABLE_RUN_CONCLUSIONS.has(run.conclusion)) {
    fail(
      "originating workflow run attempt conclusion is not a recoverable failure",
    );
  }
}

function validatePublishJob(jobsValue, marker, expectedSourceSha) {
  const response = expectRecord(jobsValue, "workflow jobs response");
  if (!Array.isArray(response.jobs)) {
    fail("workflow jobs response.jobs must be an array");
  }
  const totalCount = expectPositiveDecimal(
    response.total_count,
    "workflow jobs response.total_count",
  );
  if (
    totalCount !== String(response.jobs.length) ||
    BigInt(totalCount) > 100n
  ) {
    fail(
      "workflow jobs response must contain the complete bounded attempt job set",
    );
  }
  const publishJobs = response.jobs.filter(
    (job) =>
      job !== null && typeof job === "object" && job.name === PUBLISH_JOB_NAME,
  );
  if (publishJobs.length !== 1) {
    fail("originating attempt must contain exactly one FyAgent publish job");
  }
  const publishJob = expectRecord(publishJobs[0], "publish job");
  if (
    expectPositiveDecimal(publishJob.run_id, "publish job.run_id") !==
    marker.runId
  ) {
    fail("publish job.run_id does not match the transaction marker");
  }
  if (publishJob.workflow_name !== RELEASE_WORKFLOW_NAME) {
    fail(`publish job.workflow_name must be ${RELEASE_WORKFLOW_NAME}`);
  }
  if (
    expectSha(publishJob.head_sha, "publish job.head_sha") !== expectedSourceSha
  ) {
    fail("publish job.head_sha does not match the frozen source SHA");
  }
  if (publishJob.status !== "completed") {
    fail("originating publish job must be completed before recovery");
  }
  if (!RECOVERABLE_RUN_CONCLUSIONS.has(publishJob.conclusion)) {
    fail("originating publish job conclusion is not a recoverable failure");
  }
  if (!Array.isArray(publishJob.steps)) {
    fail("publish job.steps must be an array");
  }
  const transactionSteps = publishJob.steps.filter(
    (step) =>
      step !== null &&
      typeof step === "object" &&
      step.name === TRANSACTION_STEP_NAME,
  );
  if (transactionSteps.length !== 1) {
    fail("originating publish job must contain the FyAgent transaction step");
  }
  const transactionStep = expectRecord(
    transactionSteps[0],
    "publish transaction step",
  );
  if (transactionStep.status !== "completed") {
    fail("originating publish transaction step must be completed");
  }
  if (!RECOVERABLE_STEP_CONCLUSIONS.has(transactionStep.conclusion)) {
    fail(
      "originating publish transaction step did not end in a recoverable failure",
    );
  }
  const stepStartedAt = expectInstant(
    transactionStep.started_at,
    "publish transaction step.started_at",
  );
  const stepCompletedAt = expectInstant(
    transactionStep.completed_at,
    "publish transaction step.completed_at",
  );
  const releaseCreatedAt = expectInstant(
    marker.createdAt,
    "release.created_at",
  );
  if (
    stepStartedAt.milliseconds > stepCompletedAt.milliseconds ||
    releaseCreatedAt.milliseconds < stepStartedAt.milliseconds ||
    releaseCreatedAt.milliseconds > stepCompletedAt.milliseconds
  ) {
    fail(
      "draft creation time must fall inside the originating publish transaction step",
    );
  }
}

export function verifyRecoverableReleaseDraft(
  releaseValue,
  runAttemptValue,
  jobsValue,
  expectedTag,
  expectedSourceSha,
) {
  const marker = inspectRecoverableReleaseDraft(
    releaseValue,
    expectedTag,
    expectedSourceSha,
  );
  validateRunAttempt(runAttemptValue, marker, expectedSourceSha);
  validatePublishJob(jobsValue, marker, expectedSourceSha);
  return marker;
}

function readJson(path, label) {
  try {
    return JSON.parse(readFileSync(path, "utf8"));
  } catch (error) {
    const reason = error instanceof Error ? error.message : String(error);
    fail(`${label} is not readable JSON: ${reason}`);
  }
}

function runCli() {
  try {
    const [command, ...args] = process.argv.slice(2);
    let result;
    if (command === "inspect" && args.length === 3) {
      const [releasePath, expectedTag, expectedSourceSha] = args;
      result = inspectRecoverableReleaseDraft(
        readJson(releasePath, "release file"),
        expectedTag,
        expectedSourceSha,
      );
    } else if (command === "verify" && args.length === 5) {
      const [
        releasePath,
        runAttemptPath,
        jobsPath,
        expectedTag,
        expectedSourceSha,
      ] = args;
      result = verifyRecoverableReleaseDraft(
        readJson(releasePath, "release file"),
        readJson(runAttemptPath, "workflow run attempt file"),
        readJson(jobsPath, "workflow jobs file"),
        expectedTag,
        expectedSourceSha,
      );
    } else {
      fail(
        "usage: verify-release-draft-ownership.mjs inspect <release.json> <tag> <source-sha> | verify <release.json> <run-attempt.json> <jobs.json> <tag> <source-sha>",
      );
    }
    process.stdout.write(`${JSON.stringify(result)}\n`);
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
