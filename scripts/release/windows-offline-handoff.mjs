#!/usr/bin/env node

import { createHash } from "node:crypto";
import { appendFileSync, readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  EXPECTED_REPOSITORY_ID,
  PRODUCT_NAME,
  RELEASE_WORKFLOW_PATH,
  WINDOWS_SIGNING_STATUS_NAME,
  assertWindowsBundleVersion,
  expectedInstallerNames,
} from "./release-contract.mjs";

export const OFFLINE_RETURN_SCHEMA =
  "fyagent-windows-offline-signing-return/v1";
export const OFFLINE_POLICY_SCHEMA =
  "fyagent-windows-offline-signing-policy/v1";
export const OFFLINE_BINDING_SCHEMA =
  "fyagent-windows-offline-sealed-binding/v1";
export const OFFLINE_RETURN_MANIFEST_NAME = "signing-return.json";
export const RAW_WINDOWS_ARTIFACTS = Object.freeze({
  x64: "raw-windows-x64",
  arm64: "raw-windows-arm64",
});
export const MAX_OFFLINE_INSTALLER_BYTES = 64 * 1024 * 1024;

const SHA256_PATTERN = /^[0-9a-f]{64}$/u;
const RUN_ID_PATTERN = /^[1-9][0-9]{0,18}$/u;
const SOURCE_SHA_PATTERN = /^[0-9a-f]{40}$/u;
const ARCHITECTURES = Object.freeze(["x64", "arm64"]);
const RELEASE_RUN_NAME = "Release";
const RELEASE_RUN_EVENTS = new Set(["push", "workflow_dispatch"]);

function fail(message) {
  throw new Error(message);
}

function assertExactKeys(value, keys, label) {
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    fail(`${label} must be a JSON object`);
  }
  const actual = Object.keys(value).sort();
  const expected = [...keys].sort();
  if (
    actual.length !== expected.length ||
    actual.some((key, index) => key !== expected[index])
  ) {
    fail(`${label} keys must be exactly ${expected.join(", ")}`);
  }
}

function sha256(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

function assertSha256(value, label) {
  if (typeof value !== "string" || !SHA256_PATTERN.test(value)) {
    fail(`${label} must be a lowercase SHA-256 digest`);
  }
}

function windowsInstallerName(version, architecture) {
  const names = expectedInstallerNames(version).filter((name) =>
    name.endsWith(`-Windows-${architecture}-setup.exe`),
  );
  if (names.length !== 1) {
    fail(`Release contract must name one Windows ${architecture} installer`);
  }
  return names[0];
}

export function expectedOfflineReturnPaths(version) {
  assertWindowsBundleVersion(version);
  return [
    OFFLINE_RETURN_MANIFEST_NAME,
    windowsInstallerName(version, "x64"),
    windowsInstallerName(version, "arm64"),
  ].sort((left, right) => left.localeCompare(right, "en"));
}

export function readOfflineSigningPolicy(value) {
  assertExactKeys(
    value,
    ["schema", "publisher", "certificateSha256"],
    "Windows offline signing policy",
  );
  if (value.schema !== OFFLINE_POLICY_SCHEMA) {
    fail("Windows offline signing policy schema is unsupported");
  }
  if (
    typeof value.publisher !== "string" ||
    value.publisher.length === 0 ||
    value.publisher.length > 256 ||
    value.publisher !== value.publisher.trim() ||
    /[\r\n\u0000]/u.test(value.publisher)
  ) {
    fail("Windows offline signing policy publisher is invalid");
  }
  assertSha256(
    value.certificateSha256,
    "Windows offline signing policy certificate",
  );
  return {
    schema: OFFLINE_POLICY_SCHEMA,
    publisher: value.publisher,
    certificateSha256: value.certificateSha256,
  };
}

function assertAsset(asset, version) {
  assertExactKeys(
    asset,
    ["architecture", "name", "rawSha256", "signedSha256"],
    "Windows offline return asset",
  );
  if (!ARCHITECTURES.includes(asset.architecture)) {
    fail("Windows offline return architecture is unsupported");
  }
  if (asset.name !== windowsInstallerName(version, asset.architecture)) {
    fail(
      `Windows offline return name must be ${windowsInstallerName(version, asset.architecture)}`,
    );
  }
  assertSha256(asset.rawSha256, `${asset.architecture} raw installer`);
  assertSha256(asset.signedSha256, `${asset.architecture} signed installer`);
  if (asset.rawSha256 === asset.signedSha256) {
    fail(
      `${asset.architecture} signed installer is byte-identical to the raw installer; unsigned output cannot complete offline signing`,
    );
  }
}

export function readOfflineReturnManifest(value, version) {
  assertWindowsBundleVersion(version);
  assertExactKeys(
    value,
    [
      "schema",
      "sourceRunId",
      "sourceRunAttempt",
      "sourceSha",
      "version",
      "repositoryId",
      "assets",
    ],
    "Windows offline return",
  );
  if (value.schema !== OFFLINE_RETURN_SCHEMA) {
    fail("Windows offline return schema is unsupported");
  }
  if (!RUN_ID_PATTERN.test(value.sourceRunId)) {
    fail("Windows offline return source run id is invalid");
  }
  if (!RUN_ID_PATTERN.test(value.sourceRunAttempt)) {
    fail("Windows offline return source run attempt is invalid");
  }
  if (
    typeof value.sourceSha !== "string" ||
    !SOURCE_SHA_PATTERN.test(value.sourceSha)
  ) {
    fail("Windows offline return source SHA is invalid");
  }
  if (value.version !== version) {
    fail("Windows offline return version does not match the trusted version");
  }
  if (value.repositoryId !== EXPECTED_REPOSITORY_ID) {
    fail("Windows offline return repository id is invalid");
  }
  if (!Array.isArray(value.assets) || value.assets.length !== 2) {
    fail("Windows offline return must name x64 and arm64 exactly once");
  }
  const assets = value.assets.map((asset) => {
    assertAsset(asset, version);
    return { ...asset };
  });
  const architectures = assets.map((asset) => asset.architecture).sort();
  if (architectures.join(",") !== "arm64,x64") {
    fail("Windows offline return must name x64 and arm64 exactly once");
  }
  if (assets[0].rawSha256 === assets[1].rawSha256) {
    fail("Windows x64 and arm64 raw installers must be different bytes");
  }
  if (assets[0].signedSha256 === assets[1].signedSha256) {
    fail("Windows x64 and arm64 signed installers must be different bytes");
  }
  const serialized = JSON.stringify(value);
  if (
    serialized.includes("releases/download") ||
    serialized.includes("browser_download_url") ||
    serialized.includes("https://") ||
    serialized.includes("http://")
  ) {
    fail("Windows offline return must not carry a release or network location");
  }
  return {
    schema: OFFLINE_RETURN_SCHEMA,
    sourceRunId: value.sourceRunId,
    sourceRunAttempt: value.sourceRunAttempt,
    sourceSha: value.sourceSha,
    version,
    repositoryId: EXPECTED_REPOSITORY_ID,
    assets,
  };
}

export function assertOfflineReturnTree(paths, version) {
  if (!Array.isArray(paths)) {
    fail("Windows offline return tree must be a path list");
  }
  const normalized = paths.map((entry) => {
    if (typeof entry !== "string" || entry.length === 0) {
      fail("Windows offline return tree contains an empty path");
    }
    if (
      entry.includes("\\") ||
      entry.startsWith("/") ||
      entry.split("/").some((segment) => segment === "" || segment === "." || segment === "..")
    ) {
      fail(`Windows offline return tree path is not canonical: ${entry}`);
    }
    return entry;
  });
  const actual = [...normalized].sort((left, right) =>
    left.localeCompare(right, "en"),
  );
  const expected = expectedOfflineReturnPaths(version);
  if (
    actual.length !== expected.length ||
    actual.some((entry, index) => entry !== expected[index]) ||
    new Set(actual).size !== actual.length
  ) {
    fail(
      `Windows offline return tree must be exactly ${expected.join(", ")}`,
    );
  }
}

export function normalizeReleaseRun(run) {
  if (run === null || typeof run !== "object" || Array.isArray(run)) {
    fail("Release run evidence must be a JSON object");
  }
  const repository = run.repository;
  if (
    repository === null ||
    typeof repository !== "object" ||
    Array.isArray(repository)
  ) {
    fail("Release run evidence is missing its repository");
  }
  return {
    id: String(run.id),
    runAttempt: String(run.run_attempt),
    headSha: typeof run.head_sha === "string" ? run.head_sha : "",
    status: run.status,
    conclusion: run.conclusion,
    event: run.event,
    name: run.name,
    path: run.path,
    repositoryId: String(repository.id),
  };
}

function assertBytes(bytes, label) {
  if (!Buffer.isBuffer(bytes) || bytes.length === 0) {
    fail(`${label} must be non-empty bytes`);
  }
  if (bytes.length > MAX_OFFLINE_INSTALLER_BYTES) {
    fail(`${label} exceeds the offline installer size limit`);
  }
}

export function admitOfflineReturn({
  policy,
  returnManifest,
  treePaths,
  run,
  rawBytesByArchitecture,
  signedBytesByArchitecture,
  version,
  sourceSha,
  sourceRunId,
  sourceRunAttempt,
}) {
  const trustedPolicy = readOfflineSigningPolicy(policy);
  const manifest = readOfflineReturnManifest(returnManifest, version);
  assertOfflineReturnTree(treePaths, version);
  if (!RUN_ID_PATTERN.test(sourceRunId) || sourceRunId !== manifest.sourceRunId) {
    fail("Offline return run id does not match the requested Release run");
  }
  if (
    !RUN_ID_PATTERN.test(sourceRunAttempt) ||
    sourceRunAttempt !== manifest.sourceRunAttempt
  ) {
    fail("Offline return run attempt does not match the requested Release run");
  }
  if (!SOURCE_SHA_PATTERN.test(sourceSha) || sourceSha !== manifest.sourceSha) {
    fail("Offline return commit does not match the requested Release commit");
  }
  const observedRun = normalizeReleaseRun(run);
  if (observedRun.id !== sourceRunId) {
    fail("GitHub Release run id does not match the offline return");
  }
  if (observedRun.runAttempt !== sourceRunAttempt) {
    fail("GitHub Release run attempt does not match the offline return");
  }
  if (observedRun.headSha !== sourceSha) {
    fail("GitHub Release run commit does not match the offline return");
  }
  if (observedRun.status !== "completed" || observedRun.conclusion !== "success") {
    fail("Offline signing requires a completed successful Release run");
  }
  if (!RELEASE_RUN_EVENTS.has(observedRun.event)) {
    fail("Offline signing raw artifacts must come from a Release push or dispatch");
  }
  if (
    observedRun.name !== RELEASE_RUN_NAME ||
    observedRun.path !== RELEASE_WORKFLOW_PATH
  ) {
    fail("Offline signing raw artifacts must come from the Release workflow");
  }
  if (observedRun.repositoryId !== EXPECTED_REPOSITORY_ID) {
    fail("Offline signing Release run is for a different repository");
  }

  const assets = [];
  for (const architecture of ARCHITECTURES) {
    const declared = manifest.assets.find(
      (asset) => asset.architecture === architecture,
    );
    const rawBytes = rawBytesByArchitecture[architecture];
    const signedBytes = signedBytesByArchitecture[architecture];
    assertBytes(rawBytes, `${architecture} raw installer`);
    assertBytes(signedBytes, `${architecture} signed installer`);
    const rawSha256 = sha256(rawBytes);
    const signedSha256 = sha256(signedBytes);
    if (rawSha256 !== declared.rawSha256) {
      fail(
        `${architecture} raw installer SHA-256 does not match the Release artifact`,
      );
    }
    if (signedSha256 !== declared.signedSha256) {
      fail(
        `${architecture} signed installer SHA-256 does not match the offline return`,
      );
    }
    const other = architecture === "x64" ? "arm64" : "x64";
    const otherDeclared = manifest.assets.find(
      (asset) => asset.architecture === other,
    );
    if (
      rawSha256 === otherDeclared.rawSha256 ||
      signedSha256 === otherDeclared.signedSha256 ||
      rawSha256 === otherDeclared.signedSha256
    ) {
      fail("Windows offline return architectures are crossed");
    }
    assets.push({
      architecture,
      name: declared.name,
      rawSha256,
      signedSha256,
    });
  }

  return {
    schema: OFFLINE_BINDING_SCHEMA,
    product: PRODUCT_NAME,
    sourceRunId,
    sourceRunAttempt,
    sourceSha,
    version,
    repositoryId: EXPECTED_REPOSITORY_ID,
    workflow: RELEASE_WORKFLOW_PATH,
    publisher: trustedPolicy.publisher,
    certificateSha256: trustedPolicy.certificateSha256,
    assets,
  };
}

export function assertOfflinePublicationGate(status, binding) {
  assertExactKeys(
    binding,
    [
      "schema",
      "product",
      "sourceRunId",
      "sourceRunAttempt",
      "sourceSha",
      "version",
      "repositoryId",
      "workflow",
      "publisher",
      "certificateSha256",
      "assets",
    ],
    "Windows offline sealed binding",
  );
  if (binding.schema !== OFFLINE_BINDING_SCHEMA) {
    fail("Windows offline sealed binding schema is unsupported");
  }
  if (status === null || typeof status !== "object" || Array.isArray(status)) {
    fail("Windows signing status must be a JSON object");
  }
  if (status.mode !== "signed") {
    fail(
      "Unsigned Windows signing status cannot complete offline signing",
    );
  }
  if (status.product !== PRODUCT_NAME || status.version !== binding.version) {
    fail("Windows signing status identity does not match the offline binding");
  }
  if (status.sourceSha !== binding.sourceSha) {
    fail("Windows signing status commit does not match the offline binding");
  }
  if (!Array.isArray(status.assets) || status.assets.length !== 2) {
    fail("Windows signing status must contain x64 and arm64");
  }
  for (const architecture of ARCHITECTURES) {
    const sealed = status.assets.find(
      (asset) => asset.architecture === architecture,
    );
    const bound = binding.assets.find(
      (asset) => asset.architecture === architecture,
    );
    if (!sealed || !bound) {
      fail(`Windows signing status is missing ${architecture}`);
    }
    if (sealed.name !== bound.name || sealed.sha256 !== bound.signedSha256) {
      fail(
        `${architecture} sealed installer does not match the offline binding`,
      );
    }
    if (sealed.signature?.status !== "Valid") {
      fail(`${architecture} sealed installer is not Authenticode Valid`);
    }
    if (sealed.signature?.publisher !== binding.publisher) {
      fail(`${architecture} sealed publisher does not match the trusted policy`);
    }
    if (
      sealed.signature?.signerCertificate?.sha256 !== binding.certificateSha256
    ) {
      fail(
        `${architecture} sealed certificate SHA-256 does not match the trusted policy`,
      );
    }
  }
  return true;
}

function readJsonFile(filePath, label) {
  try {
    return JSON.parse(readFileSync(filePath, "utf8"));
  } catch (error) {
    fail(
      `${label} is not JSON: ${error instanceof Error ? error.message : String(error)}`,
    );
  }
}

function optionValue(options, name) {
  const value = options.get(name);
  if (value === undefined) fail(`Missing ${name}`);
  return value;
}

function parseOptions(args) {
  const options = new Map();
  for (let index = 0; index < args.length; index += 2) {
    const name = args[index];
    const value = args[index + 1];
    if (typeof name !== "string" || !name.startsWith("--") || value === undefined) {
      fail("Windows offline handoff options must be --name value pairs");
    }
    if (options.has(name)) fail(`Duplicate Windows offline handoff option: ${name}`);
    options.set(name, value);
  }
  return options;
}

function writeGithubOutput(filePath, values) {
  const lines = Object.entries(values).map(([key, value]) => `${key}=${value}`);
  appendFileSync(filePath, `${lines.join("\n")}\n`);
}

export function validateOfflineDispatch(input) {
  if (!RUN_ID_PATTERN.test(input.sourceRunId)) {
    fail("source_run_id must be a positive decimal integer");
  }
  if (!RUN_ID_PATTERN.test(input.sourceRunAttempt)) {
    fail("source_run_attempt must be a positive decimal integer");
  }
  if (!SOURCE_SHA_PATTERN.test(input.sourceSha)) {
    fail("source_sha must be a lowercase full commit SHA");
  }
  if (!SOURCE_SHA_PATTERN.test(input.returnSha)) {
    fail("return_sha must be a lowercase full commit SHA");
  }
  if (input.returnSha === input.sourceSha) {
    fail("return_sha must be a separate commit from the Release source");
  }
  return {
    source_run_id: input.sourceRunId,
    source_run_attempt: input.sourceRunAttempt,
    source_sha: input.sourceSha,
    return_sha: input.returnSha,
  };
}

function runCli(args = process.argv.slice(2)) {
  const [command, ...rest] = args;
  const options = parseOptions(rest);
  if (command === "validate-dispatch") {
    const validated = validateOfflineDispatch({
      sourceRunId: process.env.SOURCE_RUN_ID ?? "",
      sourceRunAttempt: process.env.SOURCE_RUN_ATTEMPT ?? "",
      sourceSha: process.env.SOURCE_SHA ?? "",
      returnSha: process.env.RETURN_SHA ?? "",
    });
    writeGithubOutput(optionValue(options, "--github-output"), validated);
    return;
  }
  if (command === "admit") {
    const version = optionValue(options, "--version");
    const binding = admitOfflineReturn({
      policy: readJsonFile(optionValue(options, "--policy"), "policy"),
      returnManifest: readJsonFile(
        optionValue(options, "--return"),
        "return manifest",
      ),
      treePaths: readFileSync(optionValue(options, "--tree"), "utf8")
        .split(/\r?\n/u)
        .filter((line) => line.length > 0),
      run: readJsonFile(optionValue(options, "--run"), "Release run"),
      rawBytesByArchitecture: {
        x64: readFileSync(optionValue(options, "--raw-x64")),
        arm64: readFileSync(optionValue(options, "--raw-arm64")),
      },
      signedBytesByArchitecture: {
        x64: readFileSync(optionValue(options, "--signed-x64")),
        arm64: readFileSync(optionValue(options, "--signed-arm64")),
      },
      version,
      sourceSha: optionValue(options, "--source-sha"),
      sourceRunId: optionValue(options, "--source-run-id"),
      sourceRunAttempt: optionValue(options, "--source-run-attempt"),
    });
    writeFileSync(
      optionValue(options, "--output"),
      `${JSON.stringify(binding, null, 2)}\n`,
      { flag: "wx" },
    );
    return;
  }
  if (command === "assert-status") {
    assertOfflinePublicationGate(
      readJsonFile(optionValue(options, "--status"), WINDOWS_SIGNING_STATUS_NAME),
      readJsonFile(optionValue(options, "--binding"), "binding"),
    );
    return;
  }
  fail(
    "Usage: windows-offline-handoff.mjs validate-dispatch --github-output <file> | admit ... | assert-status ...",
  );
}

if (
  process.argv[1] &&
  path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)
) {
  try {
    runCli();
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
