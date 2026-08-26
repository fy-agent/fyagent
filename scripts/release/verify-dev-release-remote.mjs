import { readFileSync, writeFileSync } from "node:fs";
import { pathToFileURL } from "node:url";
import {
  PREFLIGHT_WORKFLOW_BRANCH,
  PREFLIGHT_WORKFLOW_REF,
  FORMAL_BRANCH,
  FORMAL_REF,
  DEV_RELEASE_ELIGIBILITY_INPUT_SCHEMA,
  EXPECTED_REPOSITORY,
  EXPECTED_REPOSITORY_ID,
  evaluateDevReleaseEligibility,
} from "./dev-release-eligibility.mjs";

const DEFAULT_API_URL = "https://api.github.com";
const API_VERSION = "2022-11-28";
const SHA_PATTERN = /^[0-9a-f]{40}$/;
const POSITIVE_DECIMAL_PATTERN = /^[1-9]\d*$/;

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

function authorityBranchForEvent(eventName, dispatchMode) {
  if (eventName === "workflow_dispatch" && dispatchMode === "preflight") {
    return {
      branch: PREFLIGHT_WORKFLOW_BRANCH,
      ref: PREFLIGHT_WORKFLOW_REF,
    };
  }
  if (
    eventName === "push" ||
    (eventName === "workflow_dispatch" && dispatchMode === "formal")
  ) {
    return { branch: FORMAL_BRANCH, ref: FORMAL_REF };
  }
  fail(
    `unsupported release event/mode ${JSON.stringify(eventName)} / ${JSON.stringify(dispatchMode)}`,
  );
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

async function collectRemoteTag(client, repoPath, formal, candidate) {
  if (!formal) return null;
  const expectedTagRef = `refs/tags/${candidate.releaseTag}`;
  const refResponse = await client.get(
    `/repos/${repoPath}/git/ref/tags/${encodeURIComponent(candidate.releaseTag)}`,
  );
  const tagRefObject = normalizeRef(
    refResponse.body,
    expectedTagRef,
    "release tag ref",
  );
  if (tagRefObject.type === "commit") {
    return {
      ref: expectedTagRef,
      refObject: tagRefObject,
      tagObject: null,
    };
  }
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

function createEventAndCandidate(context) {
  const repository = expectString(context.repository, "context.repository");
  const repositoryId = expectString(
    context.repositoryId,
    "context.repositoryId",
  );
  expectEqual(repository, EXPECTED_REPOSITORY, "context.repository");
  expectEqual(repositoryId, EXPECTED_REPOSITORY_ID, "context.repositoryId");

  const eventName = expectString(context.eventName, "context.eventName");
  const rawDispatchMode = context.dispatchMode;
  const rawDispatchSourceSha = context.dispatchSourceSha;
  let dispatchMode = null;
  let dispatchSourceSha = null;
  if (eventName === "workflow_dispatch") {
    dispatchMode = expectString(rawDispatchMode, "context.dispatchMode");
    if (dispatchMode === "preflight") {
      dispatchSourceSha = expectSha(
        rawDispatchSourceSha,
        "context.dispatchSourceSha",
      );
    } else if (dispatchMode === "formal") {
      if (
        rawDispatchSourceSha !== null &&
        rawDispatchSourceSha !== undefined &&
        rawDispatchSourceSha !== ""
      ) {
        fail(
          "context.dispatchSourceSha must be empty for formal workflow_dispatch",
        );
      }
    } else {
      fail(
        `context.dispatchMode must be \"preflight\" or \"formal\" for workflow_dispatch; received ${JSON.stringify(dispatchMode)}`,
      );
    }
  } else {
    if (
      rawDispatchMode !== null &&
      rawDispatchMode !== undefined &&
      rawDispatchMode !== ""
    ) {
      fail("context.dispatchMode must be empty outside workflow_dispatch");
    }
    if (
      rawDispatchSourceSha !== null &&
      rawDispatchSourceSha !== undefined &&
      rawDispatchSourceSha !== ""
    ) {
      fail("context.dispatchSourceSha must be empty outside workflow_dispatch");
    }
  }

  return {
    repository: { nameWithOwner: repository, id: repositoryId },
    event: {
      dispatchMode,
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
  const authority = authorityBranchForEvent(
    identity.event.name,
    identity.event.dispatchMode,
  );
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

  const remoteTag = await collectRemoteTag(
    client,
    repoPath,
    identity.event.name === "push" || identity.event.dispatchMode === "formal",
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
    dispatchMode: env.RELEASE_DISPATCH_MODE ?? null,
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
