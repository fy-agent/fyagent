import { readFileSync, writeFileSync } from "node:fs";
import { pathToFileURL } from "node:url";
import {
  CI_WORKFLOW_NAME,
  CI_WORKFLOW_PATH,
  DEV_BRANCH,
  DEV_REF,
  FORMAL_BRANCH,
  FORMAL_REF,
  DEV_RELEASE_ELIGIBILITY_INPUT_SCHEMA,
  EXPECTED_REPOSITORY,
  EXPECTED_REPOSITORY_ID,
  evaluateDevReleaseEligibility,
} from "./dev-release-eligibility.mjs";

const DEFAULT_API_URL = "https://api.github.com";
const API_VERSION = "2022-11-28";
const CI_WORKFLOW_FILE = "ci.yml";
const SHA_PATTERN = /^[0-9a-f]{40}$/;
const POSITIVE_DECIMAL_PATTERN = /^[1-9]\d*$/;
const MAX_PAGES = 1_000;

function fail(message) {
  throw new Error(`Dev release remote verification failed: ${message}`);
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

function expectNullableString(value, label) {
  if (value !== null && typeof value !== "string") {
    fail(`${label} must be a string or null`);
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

function decimalString(value, label, { positive = true } = {}) {
  let normalized;
  if (typeof value === "number") {
    if (!Number.isSafeInteger(value)) {
      fail(`${label} must be a safely representable integer`);
    }
    normalized = String(value);
  } else if (typeof value === "string") {
    normalized = value;
  } else {
    fail(`${label} must be an integer or canonical decimal string`);
  }
  const pattern = positive ? POSITIVE_DECIMAL_PATTERN : /^(0|[1-9]\d*)$/;
  if (!pattern.test(normalized)) {
    fail(`${label} must be a canonical${positive ? " positive" : ""} decimal`);
  }
  return normalized;
}

function expectArray(value, label) {
  if (!Array.isArray(value)) fail(`${label} must be an array`);
  return value;
}

function repositoryPath(nameWithOwner) {
  const parts = nameWithOwner.split("/");
  if (parts.length !== 2 || parts.some((part) => part.length === 0)) {
    fail("repository must have owner/name form");
  }
  return parts.map(encodeURIComponent).join("/");
}

function normalizeRepository(value, label) {
  const repository = expectRecord(value, label);
  const nameWithOwner = expectString(
    repository.full_name,
    `${label}.full_name`,
  );
  const id = decimalString(repository.id, `${label}.id`);
  expectEqual(nameWithOwner, EXPECTED_REPOSITORY, `${label}.full_name`);
  expectEqual(id, EXPECTED_REPOSITORY_ID, `${label}.id`);
  return { nameWithOwner, id };
}

function normalizeStatus(value, label) {
  return expectString(value, label);
}

function normalizeConclusion(value, label) {
  return expectNullableString(value, label);
}

function normalizeApiBase(value) {
  let url;
  try {
    url = new URL(expectString(value, "GitHub API URL"));
  } catch {
    fail("GitHub API URL must be an absolute URL");
  }
  if (url.username || url.password || url.search || url.hash) {
    fail("GitHub API URL must not contain credentials, query, or fragment");
  }
  if (url.protocol !== "https:") {
    fail("GitHub API URL must use HTTPS");
  }
  url.pathname = url.pathname.replace(/\/+$/, "");
  return url.toString().replace(/\/$/, "");
}

function createApiClient({
  apiBase,
  fetchImpl,
  repositoryId,
  repositoryPath,
  token,
}) {
  const normalizedBase = normalizeApiBase(apiBase);
  expectEqual(normalizedBase, DEFAULT_API_URL, "context.apiBase");
  const baseUrl = new URL(`${normalizedBase}/`);
  const authorization = `Bearer ${expectString(token, "GitHub token")}`;

  async function get(pathOrUrl, query) {
    const url = new URL(pathOrUrl, baseUrl);
    if (url.origin !== baseUrl.origin) {
      fail("GitHub pagination URL changed origin");
    }
    if (query !== undefined) {
      for (const [key, value] of Object.entries(query)) {
        url.searchParams.set(key, String(value));
      }
    }

    let response;
    try {
      response = await fetchImpl(url, {
        method: "GET",
        redirect: "error",
        headers: {
          Accept: "application/vnd.github+json",
          Authorization: authorization,
          "User-Agent": "fyagent-release-eligibility",
          "X-GitHub-Api-Version": API_VERSION,
        },
      });
    } catch (error) {
      const rawReason = error instanceof Error ? error.message : String(error);
      const reason = rawReason.replaceAll(token, "[REDACTED]");
      fail(`GET ${url.pathname} could not be completed: ${reason}`);
    }
    if (!response.ok) {
      fail(`GET ${url.pathname} returned HTTP ${response.status}`);
    }

    let body;
    try {
      body = await response.json();
    } catch {
      fail(`GET ${url.pathname} did not return valid JSON`);
    }
    return { body, headers: response.headers, url };
  }

  return { baseUrl, get, repositoryId, repositoryPath };
}

function nextPageUrl(headers, currentUrl, client) {
  const link = headers.get("link");
  if (link === null || link.trim().length === 0) return null;
  const next = [];
  for (const entry of link.split(",")) {
    const match = entry.trim().match(/^<([^>]+)>\s*;\s*rel="([^"]+)"$/);
    if (match === null) fail("GitHub pagination Link header is malformed");
    if (match[2].split(/\s+/).includes("next")) next.push(match[1]);
  }
  if (next.length > 1) fail("GitHub pagination exposed multiple next pages");
  if (next.length === 0) return null;

  const url = new URL(next[0], currentUrl);
  if (url.origin !== client.baseUrl.origin) {
    fail("GitHub pagination next link changed origin");
  }
  const decodedPath = decodeURIComponent(url.pathname);
  const decodedCurrentPath = decodeURIComponent(currentUrl.pathname);
  const namePrefix = `/repos/${client.repositoryPath}/`;
  const idPrefix = `/repositories/${client.repositoryId}/`;
  if (
    !decodedPath.startsWith(namePrefix) &&
    !decodedPath.startsWith(idPrefix)
  ) {
    fail("GitHub pagination next link changed repository");
  }
  const normalizeResourcePath = (path) =>
    path.startsWith(namePrefix)
      ? path.slice(namePrefix.length)
      : path.slice(idPrefix.length);
  if (
    normalizeResourcePath(decodedPath) !==
    normalizeResourcePath(decodedCurrentPath)
  ) {
    fail("GitHub pagination next link changed resource");
  }
  return url.toString();
}

async function collectAllPages(client, path, query, property, label) {
  let next = path;
  let nextQuery = query;
  let expectedTotal = null;
  const seen = new Set();
  const collected = [];

  for (let page = 1; next !== null; page += 1) {
    if (page > MAX_PAGES) fail(`${label} exceeded ${MAX_PAGES} pages`);
    const response = await client.get(next, nextQuery);
    nextQuery = undefined;
    const pageKey = response.url.toString();
    if (seen.has(pageKey)) fail(`${label} pagination loop detected`);
    seen.add(pageKey);

    const body = expectRecord(response.body, `${label} response`);
    const total = decimalString(body.total_count, `${label}.total_count`, {
      positive: false,
    });
    if (expectedTotal === null) expectedTotal = total;
    expectEqual(total, expectedTotal, `${label}.total_count across pages`);
    collected.push(...expectArray(body[property], `${label}.${property}`));

    const following = nextPageUrl(response.headers, response.url, client);
    if (following === null) {
      if (BigInt(collected.length) !== BigInt(expectedTotal)) {
        fail(
          `${label} pagination is incomplete: expected ${expectedTotal}, received ${collected.length}`,
        );
      }
    } else if (BigInt(collected.length) >= BigInt(expectedTotal)) {
      fail(`${label} pagination has an unexpected next page`);
    }
    next = following;
  }
  return collected;
}

function normalizeRef(value, expectedRef, label) {
  const ref = expectRecord(value, label);
  expectEqual(
    expectString(ref.ref, `${label}.ref`),
    expectedRef,
    `${label}.ref`,
  );
  const object = expectRecord(ref.object, `${label}.object`);
  return {
    type: expectString(object.type, `${label}.object.type`),
    sha: expectSha(object.sha, `${label}.object.sha`),
  };
}

function authorityBranchForEvent(eventName) {
  if (eventName === "workflow_dispatch") {
    return { branch: DEV_BRANCH, ref: DEV_REF };
  }
  if (eventName === "push") {
    return { branch: FORMAL_BRANCH, ref: FORMAL_REF };
  }
  fail(`unsupported release event ${JSON.stringify(eventName)}`);
}

async function collectRemoteDev(client, repoPath, authority) {
  const branchResponse = await client.get(
    `/repos/${repoPath}/git/ref/heads/${encodeURIComponent(authority.branch)}`,
  );
  const branchObject = normalizeRef(
    branchResponse.body,
    authority.ref,
    `${authority.branch} branch ref`,
  );
  expectEqual(
    branchObject.type,
    "commit",
    `${authority.branch} branch ref object type`,
  );
  return {
    name: authority.branch,
    ref: authority.ref,
    headSha: branchObject.sha,
  };
}

async function collectRemoteTag(client, repoPath, eventName, candidate) {
  if (eventName !== "push") return null;
  const expectedTagRef = `refs/tags/${candidate.releaseTag}`;
  const refResponse = await client.get(
    `/repos/${repoPath}/git/ref/tags/${encodeURIComponent(candidate.releaseTag)}`,
  );
  const tagRefObject = normalizeRef(
    refResponse.body,
    expectedTagRef,
    "release tag ref",
  );
  expectEqual(tagRefObject.type, "tag", "release tag ref object type");
  const tagResponse = await client.get(
    `/repos/${repoPath}/git/tags/${tagRefObject.sha}`,
  );
  const tag = expectRecord(tagResponse.body, "annotated tag object");
  expectEqual(
    expectSha(tag.sha, "annotated tag object.sha"),
    tagRefObject.sha,
    "annotated tag object.sha",
  );
  const target = expectRecord(tag.object, "annotated tag object.object");
  return {
    ref: expectedTagRef,
    refObject: tagRefObject,
    tagObject: {
      sha: expectSha(tag.sha, "annotated tag object.sha"),
      name: expectString(tag.tag, "annotated tag object.tag"),
      target: {
        type: expectString(target.type, "annotated tag target.type"),
        sha: expectSha(target.sha, "annotated tag target.sha"),
      },
    },
  };
}

function compareRuns(left, right) {
  const numberOrder =
    BigInt(left.runNumber) < BigInt(right.runNumber)
      ? -1
      : BigInt(left.runNumber) > BigInt(right.runNumber)
        ? 1
        : 0;
  if (numberOrder !== 0) return numberOrder;
  return BigInt(left.runAttempt) < BigInt(right.runAttempt)
    ? -1
    : BigInt(left.runAttempt) > BigInt(right.runAttempt)
      ? 1
      : 0;
}

function requireUnique(items, keyFor, label) {
  const seen = new Set();
  for (const item of items) {
    const key = keyFor(item);
    if (seen.has(key)) fail(`${label} contains duplicate identity ${key}`);
    seen.add(key);
  }
}

function normalizeRun(
  value,
  index,
  repository,
  workflow,
  sourceSha,
  authorityBranch,
) {
  const label = `workflow runs[${index}]`;
  const run = expectRecord(value, label);
  const runRepository = normalizeRepository(
    run.repository,
    `${label}.repository`,
  );
  const headRepository = normalizeRepository(
    run.head_repository,
    `${label}.head_repository`,
  );
  expectEqual(runRepository.id, repository.id, `${label}.repository.id`);
  expectEqual(headRepository.id, repository.id, `${label}.head_repository.id`);
  expectEqual(
    decimalString(run.workflow_id, `${label}.workflow_id`),
    workflow.id,
    `${label}.workflow_id`,
  );
  expectEqual(
    expectString(run.name, `${label}.name`),
    workflow.name,
    `${label}.name`,
  );
  expectEqual(
    expectString(run.path, `${label}.path`),
    workflow.path,
    `${label}.path`,
  );
  expectEqual(
    expectString(run.event, `${label}.event`),
    "push",
    `${label}.event`,
  );
  expectEqual(
    expectString(run.head_branch, `${label}.head_branch`),
    authorityBranch,
    `${label}.head_branch`,
  );
  expectEqual(
    expectSha(run.head_sha, `${label}.head_sha`),
    sourceSha,
    `${label}.head_sha`,
  );

  return {
    id: decimalString(run.id, `${label}.id`),
    runNumber: decimalString(run.run_number, `${label}.run_number`),
    runAttempt: decimalString(run.run_attempt, `${label}.run_attempt`),
    checkSuiteId: decimalString(run.check_suite_id, `${label}.check_suite_id`),
    repository: runRepository,
    headRepository,
    workflow: {
      id: workflow.id,
      name: expectString(run.name, `${label}.name`),
      path: expectString(run.path, `${label}.path`),
    },
    event: expectString(run.event, `${label}.event`),
    headBranch: expectString(run.head_branch, `${label}.head_branch`),
    headSha: expectSha(run.head_sha, `${label}.head_sha`),
    status: normalizeStatus(run.status, `${label}.status`),
    conclusion: normalizeConclusion(run.conclusion, `${label}.conclusion`),
  };
}

async function collectCiRuns(
  client,
  repoPath,
  repository,
  workflow,
  sourceSha,
  authorityBranch,
) {
  const rawRuns = await collectAllPages(
    client,
    `/repos/${repoPath}/actions/workflows/${workflow.id}/runs`,
    {
      branch: authorityBranch,
      event: "push",
      head_sha: sourceSha,
      per_page: "100",
    },
    "workflow_runs",
    "CI workflow runs",
  );
  const runs = rawRuns.map((run, index) =>
    normalizeRun(
      run,
      index,
      repository,
      workflow,
      sourceSha,
      authorityBranch,
    ),
  );
  if (runs.length === 0) {
    fail(`no exact ${authorityBranch} push CI run was returned`);
  }
  requireUnique(
    runs,
    (run) => `${run.id}:${run.runAttempt}`,
    "CI workflow runs",
  );
  requireUnique(
    runs,
    (run) => `${run.runNumber}:${run.runAttempt}`,
    "CI workflow run ordering",
  );
  return runs;
}

function normalizeJob(value, index, selectedRun) {
  const label = `jobs[${index}]`;
  const job = expectRecord(value, label);
  expectEqual(
    decimalString(job.run_id, `${label}.run_id`),
    selectedRun.id,
    `${label}.run_id`,
  );
  expectEqual(
    decimalString(job.run_attempt, `${label}.run_attempt`),
    selectedRun.runAttempt,
    `${label}.run_attempt`,
  );
  return {
    id: decimalString(job.id, `${label}.id`),
    name: expectString(job.name, `${label}.name`),
    runId: selectedRun.id,
    runAttempt: selectedRun.runAttempt,
    status: normalizeStatus(job.status, `${label}.status`),
    conclusion: normalizeConclusion(job.conclusion, `${label}.conclusion`),
    checkRunUrl: expectString(job.check_run_url, `${label}.check_run_url`),
    htmlUrl: expectString(job.html_url, `${label}.html_url`),
  };
}

function normalizeSelectedCheckRuns(rawChecks, jobs, selectedRun) {
  const jobsByCheckUrl = new Map();
  for (const job of jobs) {
    if (jobsByCheckUrl.has(job.checkRunUrl)) {
      fail(`selected attempt jobs reuse check-run URL ${job.checkRunUrl}`);
    }
    jobsByCheckUrl.set(job.checkRunUrl, job);
  }

  const selected = [];
  const seenUrls = new Set();
  rawChecks.forEach((value, index) => {
    const label = `check-runs[${index}]`;
    const check = expectRecord(value, label);
    const url = expectString(check.url, `${label}.url`);
    const job = jobsByCheckUrl.get(url);
    if (job === undefined) return;
    if (seenUrls.has(url))
      fail(`selected attempt has duplicate check-run URL ${url}`);
    seenUrls.add(url);

    const suite = expectRecord(check.check_suite, `${label}.check_suite`);
    expectEqual(
      decimalString(suite.id, `${label}.check_suite.id`),
      selectedRun.checkSuiteId,
      `${label}.check_suite.id`,
    );
    const detailsUrl = expectString(check.details_url, `${label}.details_url`);
    expectEqual(detailsUrl, job.htmlUrl, `${label}.details_url`);
    const app = expectRecord(check.app, `${label}.app`);
    selected.push({
      id: decimalString(check.id, `${label}.id`),
      name: expectString(check.name, `${label}.name`),
      runId: selectedRun.id,
      runAttempt: selectedRun.runAttempt,
      checkSuiteId: selectedRun.checkSuiteId,
      appSlug: expectString(app.slug, `${label}.app.slug`),
      headSha: expectSha(check.head_sha, `${label}.head_sha`),
      status: normalizeStatus(check.status, `${label}.status`),
      conclusion: normalizeConclusion(check.conclusion, `${label}.conclusion`),
      url,
      detailsUrl,
    });
  });

  if (seenUrls.size !== jobs.length) {
    const missing = jobs
      .filter((job) => !seenUrls.has(job.checkRunUrl))
      .map((job) => job.name)
      .join(", ");
    fail(`selected attempt check-runs are incomplete for jobs: ${missing}`);
  }
  return selected;
}

function createEventAndCandidate(context) {
  const repository = expectString(context.repository, "context.repository");
  const repositoryId = expectString(
    context.repositoryId,
    "context.repositoryId",
  );
  expectEqual(repository, EXPECTED_REPOSITORY, "context.repository");
  expectEqual(repositoryId, EXPECTED_REPOSITORY_ID, "context.repositoryId");

  const eventName = expectString(context.eventName, "context.eventName");
  const dispatchSourceSha =
    eventName === "workflow_dispatch"
      ? expectSha(context.dispatchSourceSha, "context.dispatchSourceSha")
      : null;
  if (
    eventName !== "workflow_dispatch" &&
    context.dispatchSourceSha !== null &&
    context.dispatchSourceSha !== undefined &&
    context.dispatchSourceSha !== ""
  ) {
    fail("context.dispatchSourceSha must be empty outside workflow_dispatch");
  }

  return {
    repository: { nameWithOwner: repository, id: repositoryId },
    event: {
      dispatchSourceSha,
      name: eventName,
      ref: expectString(context.ref, "context.ref"),
      refName: expectString(context.refName, "context.refName"),
      refType: expectString(context.refType, "context.refType"),
      sha: expectSha(context.eventSha, "context.eventSha"),
    },
    workflow: {
      name: expectString(context.workflowName, "context.workflowName"),
      path: ".github/workflows/release.yml",
      ref: expectString(context.workflowRef, "context.workflowRef"),
      sha: expectSha(context.workflowSha, "context.workflowSha"),
    },
    candidate: {
      canonicalVersion: expectString(context.appVersion, "context.appVersion"),
      releaseTag: expectString(context.releaseTag, "context.releaseTag"),
      sourceSha: expectSha(context.sourceSha, "context.sourceSha"),
    },
  };
}

export async function collectDevReleaseRemoteEvidence(
  context,
  { fetchImpl = globalThis.fetch } = {},
) {
  if (typeof fetchImpl !== "function")
    fail("fetch implementation is unavailable");
  const identity = createEventAndCandidate(context);
  const authority = authorityBranchForEvent(identity.event.name);
  const repoPath = repositoryPath(identity.repository.nameWithOwner);
  const client = createApiClient({
    apiBase: expectString(context.apiBase, "context.apiBase"),
    fetchImpl,
    repositoryId: identity.repository.id,
    repositoryPath: repoPath,
    token: context.token,
  });

  const repositoryResponse = await client.get(`/repos/${repoPath}`);
  const repository = normalizeRepository(
    repositoryResponse.body,
    "repository API",
  );
  expectEqual(repository.id, identity.repository.id, "repository API identity");

  const workflowResponse = await client.get(
    `/repos/${repoPath}/actions/workflows/${CI_WORKFLOW_FILE}`,
  );
  const workflowBody = expectRecord(workflowResponse.body, "CI workflow API");
  const ciWorkflow = {
    id: decimalString(workflowBody.id, "CI workflow API.id"),
    name: expectString(workflowBody.name, "CI workflow API.name"),
    path: expectString(workflowBody.path, "CI workflow API.path"),
    state: expectString(workflowBody.state, "CI workflow API.state"),
    repository,
  };
  expectEqual(ciWorkflow.name, CI_WORKFLOW_NAME, "CI workflow API.name");
  expectEqual(ciWorkflow.path, CI_WORKFLOW_PATH, "CI workflow API.path");

  const initialCiRuns = await collectCiRuns(
    client,
    repoPath,
    repository,
    ciWorkflow,
    identity.candidate.sourceSha,
    authority.branch,
  );
  const selectedRun = [...initialCiRuns].sort(compareRuns).at(-1);

  const rawJobs = await collectAllPages(
    client,
    `/repos/${repoPath}/actions/runs/${selectedRun.id}/attempts/${selectedRun.runAttempt}/jobs`,
    { per_page: "100" },
    "jobs",
    "selected CI attempt jobs",
  );
  const jobs = rawJobs.map((job, index) =>
    normalizeJob(job, index, selectedRun),
  );
  if (jobs.length === 0) fail("selected CI attempt has no jobs");
  requireUnique(jobs, (job) => job.id, "selected CI attempt jobs");

  const rawChecks = await collectAllPages(
    client,
    `/repos/${repoPath}/check-suites/${selectedRun.checkSuiteId}/check-runs`,
    { per_page: "100" },
    "check_runs",
    "selected CI check suite check-runs",
  );
  const checkRuns = normalizeSelectedCheckRuns(rawChecks, jobs, selectedRun);

  // A rerun can start while attempt evidence is being collected. Re-list the
  // exact-SHA runs, then read the tag and branch as the final remote facts.
  // The pure evaluator rejects the earlier evidence if the selected attempt
  // changed between these observations.
  const ciRuns = await collectCiRuns(
    client,
    repoPath,
    repository,
    ciWorkflow,
    identity.candidate.sourceSha,
    authority.branch,
  );
  const remoteTag = await collectRemoteTag(
    client,
    repoPath,
    identity.event.name,
    identity.candidate,
  );
  const remoteDev = await collectRemoteDev(client, repoPath, authority);

  return {
    schema: DEV_RELEASE_ELIGIBILITY_INPUT_SCHEMA,
    repository,
    event: identity.event,
    workflow: identity.workflow,
    candidate: identity.candidate,
    remoteDev,
    remoteTag,
    ciWorkflow,
    ciRuns,
    ciEvidence: {
      runId: selectedRun.id,
      runAttempt: selectedRun.runAttempt,
      checkSuiteId: selectedRun.checkSuiteId,
      jobs,
      checkRuns,
    },
  };
}

export async function verifyDevReleaseRemote(
  context,
  { expectedFrozen, fetchImpl = globalThis.fetch } = {},
) {
  const evidence = await collectDevReleaseRemoteEvidence(context, {
    fetchImpl,
  });
  return {
    evidence,
    result: evaluateDevReleaseEligibility(evidence, expectedFrozen),
  };
}

function requiredEnvironment(env, name) {
  return expectString(env[name], `environment ${name}`);
}

export function contextFromEnvironment(env = process.env) {
  const githubToken = env.GITHUB_TOKEN;
  const ghToken = env.GH_TOKEN;
  if (
    typeof githubToken === "string" &&
    githubToken.length > 0 &&
    typeof ghToken === "string" &&
    ghToken.length > 0 &&
    githubToken !== ghToken
  ) {
    fail("GITHUB_TOKEN and GH_TOKEN disagree");
  }
  const token = githubToken || ghToken;
  const apiBase = env.GITHUB_API_URL || DEFAULT_API_URL;
  expectEqual(
    normalizeApiBase(apiBase),
    DEFAULT_API_URL,
    "environment GITHUB_API_URL",
  );

  return {
    token: expectString(token, "environment GITHUB_TOKEN or GH_TOKEN"),
    apiBase,
    repository: requiredEnvironment(env, "GITHUB_REPOSITORY"),
    repositoryId: requiredEnvironment(env, "GITHUB_REPOSITORY_ID"),
    eventName: requiredEnvironment(env, "GITHUB_EVENT_NAME"),
    ref: requiredEnvironment(env, "GITHUB_REF"),
    refName: requiredEnvironment(env, "GITHUB_REF_NAME"),
    refType: requiredEnvironment(env, "GITHUB_REF_TYPE"),
    eventSha: requiredEnvironment(env, "GITHUB_SHA"),
    workflowName: requiredEnvironment(env, "GITHUB_WORKFLOW"),
    workflowRef: requiredEnvironment(env, "GITHUB_WORKFLOW_REF"),
    workflowSha: requiredEnvironment(env, "GITHUB_WORKFLOW_SHA"),
    appVersion: requiredEnvironment(env, "RELEASE_APP_VERSION"),
    releaseTag: requiredEnvironment(env, "RELEASE_TAG"),
    sourceSha: requiredEnvironment(env, "RELEASE_SOURCE_SHA"),
    dispatchSourceSha: env.RELEASE_DISPATCH_SOURCE_SHA ?? null,
  };
}

function parseExpected(value) {
  const trimmed = expectString(value, "--expected value").trim();
  if (trimmed.startsWith("{")) return JSON.parse(trimmed);
  return JSON.parse(readFileSync(trimmed, "utf8"));
}

function parseArguments(argv) {
  let json = false;
  let evidencePath = null;
  let expected = undefined;
  for (let index = 0; index < argv.length; index += 1) {
    const argument = argv[index];
    if (argument === "--json" && !json) {
      json = true;
    } else if (argument === "--evidence" && evidencePath === null) {
      evidencePath = argv[index + 1] ?? null;
      index += 1;
    } else if (argument === "--expected" && expected === undefined) {
      const value = argv[index + 1];
      if (value === undefined)
        fail("--expected requires a JSON path or object");
      expected = parseExpected(value);
      index += 1;
    } else {
      fail(`unknown or duplicate CLI argument ${JSON.stringify(argument)}`);
    }
  }
  if (!json) {
    fail(
      "usage: node verify-dev-release-remote.mjs --json [--evidence <path>] [--expected <path|json>]",
    );
  }
  if (evidencePath !== null && evidencePath.length === 0) {
    fail("--evidence requires a path");
  }
  return { evidencePath, expected };
}

async function runCli() {
  try {
    const { evidencePath, expected } = parseArguments(process.argv.slice(2));
    const { evidence, result } = await verifyDevReleaseRemote(
      contextFromEnvironment(),
      { expectedFrozen: expected },
    );
    if (evidencePath !== null) {
      writeFileSync(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`, {
        encoding: "utf8",
        mode: 0o600,
      });
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
  await runCli();
}
