#!/usr/bin/env node

import { createHash } from "node:crypto";
import { cpSync, existsSync, readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  CI_WORKFLOW_PATH,
  EXPECTED_REPOSITORY,
  EXPECTED_REPOSITORY_ID,
  PREFLIGHT_WORKFLOW_BRANCH,
  PRODUCT_NAME,
  RELEASE_WORKFLOW_PATH,
  expectedInstallerNames,
} from "./release-contract.mjs";
import { readOfflineSigningPolicy } from "./windows-offline-handoff.mjs";

export const FORMAL_OFFLINE_ADMISSION_SCHEMA =
  "fyagent-windows-formal-offline-admission/v1";
export const FORMAL_OFFLINE_ADMISSION_PROOF_SCHEMA =
  "fyagent-windows-formal-offline-admission-proof/v1";
export const FORMAL_OFFLINE_ADMISSION_MANIFEST =
  "docs/release/windows-formal-offline-admission.json";
export const OFFLINE_RETURN_WORKFLOW_NAME = "Windows offline sign return";
export const OFFLINE_RETURN_WORKFLOW_PATH =
  ".github/workflows/windows-offline-sign-return.yml";

// Files that may differ between the Windows artifact source and the formal tag.
// Native build jobs do not execute these paths. release.yml is further pinned:
// its product build jobs must stay byte-identical except the admission gate.
export const WORKFLOW_ONLY_PATHS = Object.freeze([
  ".github/workflows/release.yml",
  ".trellis/spec/backend/github-release-workflow.md",
  "docs/fyagent/development/ci-release/release.md",
  "docs/release-notes/v0.4.15-en.md",
  "docs/release-notes/v0.4.15-zh.md",
  "docs/release/release-identity.md",
  "docs/release/windows-formal-offline-admission.json",
  "scripts/release/generate-build-metadata.mjs",
  "scripts/release/windows-formal-offline-admission.d.mts",
  "scripts/release/windows-formal-offline-admission.mjs",
  "scripts/tasks/release-check.mjs",
  "scripts/tasks/supported-platform-structure-assets.json",
  "tests/releaseWorkflow.test.ts",
  "tests/windowsFormalOfflineAdmission.test.ts",
]);

const PRODUCT_INPUT_EXACT = new Set([
  "CHANGELOG.md",
  "package.json",
  "pnpm-lock.yaml",
  "docs/release/windows-offline-signing-policy.json",
  "scripts/release/create-macos-dmg.sh",
  "scripts/release/macos-developer-id.sh",
  "scripts/release/windows-signing.mjs",
  "scripts/release/write-platform-metadata.mjs",
  "scripts/version.mjs",
  "src-tauri/Cargo.lock",
  "src-tauri/Cargo.toml",
  "src-tauri/tauri.conf.json",
]);
const PRODUCT_INPUT_PREFIXES = Object.freeze(["src/", "src-tauri/"]);
const PRODUCT_JOBS = Object.freeze([
  "build-windows",
  "prove-windows-preflight",
  "sign-windows-formal",
  "seal-windows-formal",
  "build-macos",
]);
const BUILD_WINDOWS_SKIP_IF =
  "    if: needs.eligibility.outputs.windows_byte_source != 'offline-sealed'\n";
const FORMAL_REBUILD_IF =
  "    if: needs.eligibility.outputs.release_mode == 'formal' && (github.event_name == 'push' || github.event_name == 'workflow_dispatch')\n";
const FORMAL_REBUILD_IF_WITH_GATE =
  "    if: \"${{ !cancelled() && needs.eligibility.result == 'success' && needs['pin-release-build-inputs'].result == 'success' && needs.eligibility.outputs.release_mode == 'formal' && needs.eligibility.outputs.windows_byte_source != 'offline-sealed' && (github.event_name == 'push' || github.event_name == 'workflow_dispatch') }}\"\n";
const SHA256_PATTERN = /^[0-9a-f]{64}$/u;
const SOURCE_SHA_PATTERN = /^[0-9a-f]{40}$/u;
const RUN_ID_PATTERN = /^[1-9][0-9]{0,18}$/u;
const ARCHITECTURES = Object.freeze(["x64", "arm64"]);
const RELEASE_RUN_NAME = "Release";
const RELEASE_RUN_EVENTS = new Set(["push", "workflow_dispatch"]);
const MAX_TEXT_BYTES = 1024 * 1024;
const MAX_INSTALLER_BYTES = 64 * 1024 * 1024;
const IDENTITY_FIELDS = Object.freeze([
  "productVersion",
  "tag",
  "sourceSha",
  "repository",
  "repositoryId",
  "workflowPath",
  "workflowRef",
  "workflowSha",
  "runId",
  "runAttempt",
  "event",
  "mode",
  "ciWorkflowPath",
  "ciRunId",
  "ciRunAttempt",
]);

function fail(message) {
  throw new Error(message);
}

function assert(condition, message) {
  if (!condition) fail(message);
}

function sha256(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

function assertExactKeys(value, keys, label) {
  assert(
    value !== null && typeof value === "object" && !Array.isArray(value),
    `${label} must be a JSON object`,
  );
  const actual = Object.keys(value).sort();
  const expected = [...keys].sort();
  assert(
    actual.length === expected.length &&
      actual.every((key, index) => key === expected[index]),
    `${label} keys must be exactly ${expected.join(", ")}`,
  );
}

function assertSha256(value, label) {
  assert(
    typeof value === "string" && SHA256_PATTERN.test(value),
    `${label} must be a lowercase SHA-256 digest`,
  );
}

function assertSourceSha(value, label) {
  assert(
    typeof value === "string" && SOURCE_SHA_PATTERN.test(value),
    `${label} must be a lowercase full commit SHA`,
  );
}

function assertRunId(value, label) {
  assert(
    typeof value === "string" && RUN_ID_PATTERN.test(value),
    `${label} must be a positive decimal integer`,
  );
}

function assertAllowlist() {
  const allowed = new Set(WORKFLOW_ONLY_PATHS);
  assert(
    allowed.size === WORKFLOW_ONLY_PATHS.length,
    "Workflow-only admission allowlist contains a duplicate path",
  );
  for (const candidate of WORKFLOW_ONLY_PATHS) {
    assert(
      !PRODUCT_INPUT_EXACT.has(candidate) &&
        !PRODUCT_INPUT_PREFIXES.some((prefix) => candidate.startsWith(prefix)),
      `Admission allowlist includes a product input: ${candidate}`,
    );
  }
}

assertAllowlist();

function installerName(version, architecture) {
  const names = expectedInstallerNames(version).filter((name) =>
    name.endsWith(`-Windows-${architecture}-setup.exe`),
  );
  assert(
    names.length === 1,
    `Release contract must name one Windows ${architecture} installer`,
  );
  return names[0];
}

export function readFormalOfflineAdmissionManifest(value) {
  assertExactKeys(
    value,
    [
      "schema",
      "version",
      "artifactSourceSha",
      "productTreeHash",
      "preflightRunId",
      "preflightRunAttempt",
      "offlineRunId",
      "offlineRunAttempt",
      "assets",
    ],
    "Windows formal offline admission",
  );
  assert(
    value.schema === FORMAL_OFFLINE_ADMISSION_SCHEMA,
    "Windows formal offline admission schema is unsupported",
  );
  expectedInstallerNames(value.version);
  assertSourceSha(value.artifactSourceSha, "Admission artifact source");
  assertSha256(value.productTreeHash, "Admission product tree");
  assertRunId(value.preflightRunId, "Admission preflight run");
  assertRunId(value.preflightRunAttempt, "Admission preflight attempt");
  assertRunId(value.offlineRunId, "Admission offline run");
  assertRunId(value.offlineRunAttempt, "Admission offline attempt");
  assert(
    value.preflightRunId !== value.offlineRunId,
    "Offline seal run must be distinct from the preflight run",
  );
  assert(
    Array.isArray(value.assets) && value.assets.length === 2,
    "Windows formal offline admission must name x64 and arm64 exactly once",
  );
  const assets = value.assets.map((asset) => {
    assertExactKeys(
      asset,
      ["architecture", "name", "rawSha256", "signedSha256"],
      "Windows formal offline admission asset",
    );
    assert(
      ARCHITECTURES.includes(asset.architecture),
      "Windows formal offline admission architecture is unsupported",
    );
    assert(
      asset.name === installerName(value.version, asset.architecture),
      `Windows formal offline admission name must be ${installerName(value.version, asset.architecture)}`,
    );
    assertSha256(asset.rawSha256, `${asset.architecture} raw installer`);
    assertSha256(asset.signedSha256, `${asset.architecture} signed installer`);
    assert(
      asset.rawSha256 !== asset.signedSha256,
      `${asset.architecture} signed installer is byte-identical to the raw installer; unsigned output cannot enter a formal release`,
    );
    return { ...asset };
  });
  const architectures = assets.map((asset) => asset.architecture).sort();
  assert(
    architectures.join(",") === "arm64,x64",
    "Windows formal offline admission must name x64 and arm64 exactly once",
  );
  assert(
    assets[0].rawSha256 !== assets[1].rawSha256 &&
      assets[0].signedSha256 !== assets[1].signedSha256,
    "Windows x64 and arm64 installers must be different bytes",
  );
  return {
    schema: FORMAL_OFFLINE_ADMISSION_SCHEMA,
    version: value.version,
    artifactSourceSha: value.artifactSourceSha,
    productTreeHash: value.productTreeHash,
    preflightRunId: value.preflightRunId,
    preflightRunAttempt: value.preflightRunAttempt,
    offlineRunId: value.offlineRunId,
    offlineRunAttempt: value.offlineRunAttempt,
    assets,
  };
}

export function classifyWindowsByteSource({ mode, version, manifest }) {
  expectedInstallerNames(version);
  assert(mode === "preflight" || mode === "formal", `Unsupported release mode: ${mode}`);
  if (manifest === null) return "rebuild";
  const admitted = readFormalOfflineAdmissionManifest(manifest);
  assert(
    admitted.version === version,
    "Windows formal offline admission version does not match the release version",
  );
  if (mode === "preflight") return "rebuild";
  return "offline-sealed";
}

function splitLines(text, label) {
  assert(typeof text === "string", `${label} must be text`);
  assert(
    Buffer.byteLength(text) <= MAX_TEXT_BYTES,
    `${label} exceeds the admission text limit`,
  );
  return text
    .split(/\r?\n/u)
    .map((line) => line.trimEnd())
    .filter((line) => line.length > 0);
}

export function productTreeHash(lsTreeText) {
  const rows = [];
  for (const line of splitLines(lsTreeText, "git ls-tree")) {
    const tab = line.indexOf("\t");
    assert(tab > 0, "git ls-tree output is not canonical");
    const meta = line.slice(0, tab);
    const entryPath = line.slice(tab + 1);
    assert(
      !entryPath.startsWith('"') && !entryPath.includes("\\"),
      "git ls-tree path must be unquoted; pass core.quotePath=false",
    );
    const parts = meta.split(" ");
    assert(parts.length === 3, "git ls-tree output is not canonical");
    const [mode, type, oid] = parts;
    assert(type === "blob", `git ls-tree entry is not a blob: ${entryPath}`);
    assert(
      SOURCE_SHA_PATTERN.test(oid),
      `git ls-tree object id is invalid: ${entryPath}`,
    );
    assert(
      mode === "100644" || mode === "100755",
      `git ls-tree mode is unsupported: ${entryPath}`,
    );
    assert(
      !entryPath.startsWith("/") &&
        !entryPath.split("/").some((segment) => segment === "" || segment === "." || segment === ".."),
      `git ls-tree path is not canonical: ${entryPath}`,
    );
    if (WORKFLOW_ONLY_PATHS.includes(entryPath)) continue;
    rows.push(`${mode} ${type} ${oid} ${entryPath}`);
  }
  rows.sort((left, right) => left.localeCompare(right, "en"));
  return sha256(`${rows.join("\n")}\n`);
}

function assertWorkflowOnlyDiff(diffStatusText) {
  const paths = [];
  for (const line of splitLines(diffStatusText, "git diff")) {
    const parts = line.split("\t");
    const status = parts[0];
    assert(
      /^(?:[AMD]|R\d{3}|C\d{3})$/u.test(status) && parts.length >= 2,
      `git diff status is unsupported: ${line}`,
    );
    const changed = parts.slice(1);
    assert(
      changed.length === (status.startsWith("R") || status.startsWith("C") ? 2 : 1),
      `git diff status is unsupported: ${line}`,
    );
    for (const candidate of changed) {
      assert(
        WORKFLOW_ONLY_PATHS.includes(candidate),
        `Release diff includes a non-workflow path: ${candidate}`,
      );
      paths.push(candidate);
    }
  }
  return [...new Set(paths)].sort((left, right) => left.localeCompare(right, "en"));
}

export function extractWorkflowJob(workflowText, name) {
  assert(typeof workflowText === "string", "Release workflow must be text");
  const source = workflowText.endsWith("\n")
    ? workflowText
    : `${workflowText}\n`;
  const marker = `\n  ${name}:\n`;
  const start = source.indexOf(marker);
  assert(start >= 0, `Release workflow is missing ${name}`);
  assert(
    source.indexOf(marker, start + marker.length) < 0,
    `Release workflow contains more than one ${name} job`,
  );
  const after = source.slice(start + marker.length);
  const next = after.search(/\n  [A-Za-z0-9_-]+:\n/u);
  const body = next < 0 ? after : after.slice(0, next);
  return `  ${name}:\n${body}`;
}

function normalizeProductJob(name, block) {
  if (name === "build-windows") {
    assert(
      block.split(BUILD_WINDOWS_SKIP_IF).length <= 2,
      "build-windows admission gate is duplicated",
    );
    return block.replace(BUILD_WINDOWS_SKIP_IF, "");
  }
  if (name === "sign-windows-formal" || name === "seal-windows-formal") {
    assert(
      block.split(FORMAL_REBUILD_IF_WITH_GATE).length <= 2,
      `${name} admission gate is duplicated`,
    );
    return block.replace(FORMAL_REBUILD_IF_WITH_GATE, FORMAL_REBUILD_IF);
  }
  return block;
}

export function productJobFingerprint(workflowText) {
  return sha256(
    PRODUCT_JOBS.map((name) =>
      normalizeProductJob(name, extractWorkflowJob(workflowText, name)),
    ).join("\n---\n"),
  );
}

export function assertProductJobsPreserved(artifactWorkflow, releaseWorkflow) {
  for (const name of PRODUCT_JOBS) {
    const artifactJob = normalizeProductJob(
      name,
      extractWorkflowJob(artifactWorkflow, name),
    );
    const releaseJob = normalizeProductJob(
      name,
      extractWorkflowJob(releaseWorkflow, name),
    );
    assert(
      artifactJob === releaseJob,
      `Product build job ${name} changed outside the offline admission gate`,
    );
  }
}

function normalizeRun(run, label) {
  assert(
    run !== null && typeof run === "object" && !Array.isArray(run),
    `${label} evidence must be a JSON object`,
  );
  const repository = run.repository;
  assert(
    repository !== null &&
      typeof repository === "object" &&
      !Array.isArray(repository),
    `${label} evidence is missing its repository`,
  );
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

function assertSuccessfulRun(run, label) {
  assert(
    run.status === "completed" && run.conclusion === "success",
    `${label} must be a completed successful run`,
  );
  assert(
    run.repositoryId === EXPECTED_REPOSITORY_ID,
    `${label} belongs to a different repository`,
  );
}

function assertBytes(bytes, label) {
  assert(Buffer.isBuffer(bytes) && bytes.length > 0, `${label} must be non-empty bytes`);
  assert(bytes.length <= MAX_INSTALLER_BYTES, `${label} exceeds the installer size limit`);
}

function assetByArchitecture(assets, architecture) {
  const matches = assets.filter((asset) => asset.architecture === architecture);
  assert(matches.length === 1, `Admission evidence is missing ${architecture}`);
  return matches[0];
}

function assertBinding(binding, manifest, policy) {
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
  assert(
    binding.schema === "fyagent-windows-offline-sealed-binding/v1",
    "Windows offline sealed binding schema is unsupported",
  );
  assert(binding.product === PRODUCT_NAME, "Offline binding product drifted");
  assert(
    binding.sourceRunId === manifest.preflightRunId &&
      binding.sourceRunAttempt === manifest.preflightRunAttempt &&
      binding.sourceSha === manifest.artifactSourceSha &&
      binding.version === manifest.version &&
      binding.repositoryId === EXPECTED_REPOSITORY_ID &&
      binding.workflow === RELEASE_WORKFLOW_PATH,
    "Offline binding does not match the formal admission manifest",
  );
  assert(
    binding.publisher === policy.publisher &&
      binding.certificateSha256 === policy.certificateSha256,
    "Offline binding publisher or certificate does not match the public policy",
  );
  assert(
    Array.isArray(binding.assets) && binding.assets.length === 2,
    "Offline binding must name x64 and arm64",
  );
  for (const architecture of ARCHITECTURES) {
    const bound = assetByArchitecture(binding.assets, architecture);
    const declared = assetByArchitecture(manifest.assets, architecture);
    assertExactKeys(
      bound,
      ["architecture", "name", "rawSha256", "signedSha256"],
      `${architecture} offline binding asset`,
    );
    assert(
      bound.name === declared.name &&
        bound.rawSha256 === declared.rawSha256 &&
        bound.signedSha256 === declared.signedSha256,
      `${architecture} offline binding does not match the admission manifest`,
    );
  }
}

export function proveFormalOfflineAdmission({
  manifest,
  policy,
  releaseSha,
  ancestor,
  artifactLsTree,
  releaseLsTree,
  diffStatus,
  artifactWorkflow,
  releaseWorkflow,
  preflightRun,
  offlineRun,
  binding,
  rawBytesByArchitecture,
  signedBytesByArchitecture,
}) {
  const admitted = readFormalOfflineAdmissionManifest(manifest);
  const trustedPolicy = readOfflineSigningPolicy(policy);
  assertSourceSha(releaseSha, "Formal release source");
  assert(
    ancestor === true,
    "Windows artifact source is not an ancestor of the formal release commit",
  );
  const artifactHash = productTreeHash(artifactLsTree);
  const releaseHash = productTreeHash(releaseLsTree);
  assert(
    artifactHash === releaseHash && artifactHash === admitted.productTreeHash,
    "Application bytes differ from the Windows artifact source",
  );
  const diffPaths = assertWorkflowOnlyDiff(diffStatus);
  assertProductJobsPreserved(artifactWorkflow, releaseWorkflow);
  assertBinding(binding, admitted, trustedPolicy);

  const preflight = normalizeRun(preflightRun, "Preflight run");
  assertSuccessfulRun(preflight, "Preflight run");
  assert(
    preflight.id === admitted.preflightRunId &&
      preflight.runAttempt === admitted.preflightRunAttempt &&
      preflight.headSha === admitted.artifactSourceSha &&
      preflight.name === RELEASE_RUN_NAME &&
      preflight.path === RELEASE_WORKFLOW_PATH &&
      RELEASE_RUN_EVENTS.has(preflight.event),
    "Preflight run does not match the formal admission manifest",
  );
  const offline = normalizeRun(offlineRun, "Offline seal run");
  assertSuccessfulRun(offline, "Offline seal run");
  assert(
    offline.id === admitted.offlineRunId &&
      offline.runAttempt === admitted.offlineRunAttempt &&
      offline.headSha === admitted.artifactSourceSha &&
      offline.name === OFFLINE_RETURN_WORKFLOW_NAME &&
      offline.path === OFFLINE_RETURN_WORKFLOW_PATH &&
      offline.event === "workflow_dispatch",
    "Offline seal run does not match the formal admission manifest",
  );

  const assets = [];
  for (const architecture of ARCHITECTURES) {
    const declared = assetByArchitecture(admitted.assets, architecture);
    const rawBytes = rawBytesByArchitecture[architecture];
    const signedBytes = signedBytesByArchitecture[architecture];
    assertBytes(rawBytes, `${architecture} raw installer`);
    assertBytes(signedBytes, `${architecture} signed installer`);
    assert(
      sha256(rawBytes) === declared.rawSha256,
      `${architecture} raw installer SHA-256 does not match the admission manifest`,
    );
    assert(
      sha256(signedBytes) === declared.signedSha256,
      `${architecture} signed installer SHA-256 does not match the admission manifest`,
    );
    assets.push({ ...declared });
  }

  return {
    schema: FORMAL_OFFLINE_ADMISSION_PROOF_SCHEMA,
    version: admitted.version,
    releaseSha,
    artifactSourceSha: admitted.artifactSourceSha,
    productTreeHash: artifactHash,
    diffPaths,
    preflightRunId: admitted.preflightRunId,
    preflightRunAttempt: admitted.preflightRunAttempt,
    offlineRunId: admitted.offlineRunId,
    offlineRunAttempt: admitted.offlineRunAttempt,
    publisher: trustedPolicy.publisher,
    certificateSha256: trustedPolicy.certificateSha256,
    assets,
  };
}

function readMetadata(directory, targetGroup) {
  const filePath = path.join(directory, `${targetGroup}.json`);
  let value;
  try {
    value = JSON.parse(readFileSync(filePath, "utf8"));
  } catch (error) {
    fail(
      `${targetGroup} platform metadata is missing or not JSON: ${error instanceof Error ? error.message : String(error)}`,
    );
  }
  assert(
    value !== null && typeof value === "object" && !Array.isArray(value),
    `${targetGroup} platform metadata must be a JSON object`,
  );
  assert(
    value.identity !== null &&
      typeof value.identity === "object" &&
      !Array.isArray(value.identity),
    `${targetGroup} platform metadata is missing its identity`,
  );
  return { filePath, value };
}

function sameIdentity(actual, expected) {
  return IDENTITY_FIELDS.every((key) => actual[key] === expected[key]);
}

export function preflightWindowsIdentity(proof) {
  assert(
    proof?.schema === FORMAL_OFFLINE_ADMISSION_PROOF_SCHEMA,
    "Windows offline admission proof schema is unsupported",
  );
  return {
    productVersion: proof.version,
    tag: `v${proof.version}`,
    sourceSha: proof.artifactSourceSha,
    repository: EXPECTED_REPOSITORY,
    repositoryId: EXPECTED_REPOSITORY_ID,
    workflowPath: RELEASE_WORKFLOW_PATH,
    workflowRef: `${EXPECTED_REPOSITORY}/${RELEASE_WORKFLOW_PATH}@refs/heads/${PREFLIGHT_WORKFLOW_BRANCH}`,
    workflowSha: proof.artifactSourceSha,
    runId: proof.preflightRunId,
    runAttempt: proof.preflightRunAttempt,
    event: "workflow_dispatch",
    mode: "preflight",
    ciWorkflowPath: CI_WORKFLOW_PATH,
    ciRunId: null,
    ciRunAttempt: null,
  };
}

export function assertAdmittedWindowsMetadata(sourceDirectory, proof) {
  const expectedWindows = preflightWindowsIdentity(proof);
  for (const targetGroup of ["windows-x64", "windows-arm64"]) {
    const { value } = readMetadata(sourceDirectory, targetGroup);
    assert(
      sameIdentity(value.identity, expectedWindows),
      `${targetGroup} metadata does not belong to the admitted preflight run`,
    );
  }
}

export function stageMetadataForAggregate({
  sourceDirectory,
  destinationDirectory,
  proof,
  releaseIdentity,
}) {
  assertExactKeys(releaseIdentity, IDENTITY_FIELDS, "Formal release identity");
  assert(
    releaseIdentity.sourceSha === proof.releaseSha &&
      releaseIdentity.productVersion === proof.version &&
      releaseIdentity.tag === `v${proof.version}` &&
      releaseIdentity.mode === "formal",
    "Formal release identity does not match the offline admission proof",
  );
  assertAdmittedWindowsMetadata(sourceDirectory, proof);
  assert(
    !existsSync(destinationDirectory),
    "Staged metadata directory already exists",
  );
  cpSync(sourceDirectory, destinationDirectory, {
    recursive: true,
    dereference: false,
  });
  for (const targetGroup of ["windows-x64", "windows-arm64"]) {
    const { value } = readMetadata(destinationDirectory, targetGroup);
    value.identity = { ...releaseIdentity };
    writeFileSync(
      path.join(destinationDirectory, `${targetGroup}.json`),
      `${JSON.stringify(value, null, 2)}\n`,
    );
  }
  return destinationDirectory;
}

export function bindFormalSigningStatus(status, proof) {
  assert(
    proof?.schema === FORMAL_OFFLINE_ADMISSION_PROOF_SCHEMA,
    "Windows offline admission proof schema is unsupported",
  );
  assert(
    status !== null && typeof status === "object" && !Array.isArray(status),
    "Windows signing status must be a JSON object",
  );
  assert(
    status.schema === "fyagent-windows-signing-status/v1" &&
      status.product === PRODUCT_NAME &&
      status.version === proof.version &&
      status.sourceSha === proof.artifactSourceSha,
    "Windows signing status does not match the admitted artifact source",
  );
  assert(
    status.mode === "signed",
    "Unsigned Windows signing status cannot enter a formal offline release",
  );
  assert(!Object.hasOwn(status, "offlineAdmission"), "Windows signing status already carries an admission proof");
  assert(
    Array.isArray(status.assets) && status.assets.length === 2,
    "Windows signing status must contain x64 and arm64",
  );
  const assets = status.assets.map((asset) => {
    const declared = assetByArchitecture(proof.assets, asset.architecture);
    assert(
      asset.name === declared.name &&
        asset.sha256 === declared.signedSha256 &&
        asset.sourceSha === proof.artifactSourceSha &&
        asset.signature?.status === "Valid" &&
        asset.signature?.publisher === proof.publisher &&
        asset.signature?.signerCertificate?.sha256 === proof.certificateSha256,
      `${asset.architecture} sealed installer does not match the admission proof`,
    );
    return asset;
  });
  return {
    schema: status.schema,
    product: status.product,
    version: status.version,
    sourceSha: proof.releaseSha,
    mode: "signed",
    offlineAdmission: proof,
    assets,
  };
}

export function assertPublishedAdmission({
  proof,
  signingStatus,
  buildMetadata,
  signedBytesByArchitecture,
}) {
  assert(
    signingStatus !== null &&
      typeof signingStatus === "object" &&
      !Array.isArray(signingStatus) &&
      Object.hasOwn(signingStatus, "offlineAdmission"),
    "Published Windows signing status is missing its admission proof",
  );
  const { offlineAdmission: _publishedProof, ...withoutProof } = signingStatus;
  const bound = bindFormalSigningStatus(
    { ...withoutProof, sourceSha: proof.artifactSourceSha },
    proof,
  );
  assert(
    JSON.stringify(signingStatus) === JSON.stringify(bound),
    "Published Windows signing status does not match the recomputed admission proof",
  );
  assert(
    buildMetadata?.offlineAdmission &&
      JSON.stringify(buildMetadata.offlineAdmission) === JSON.stringify(proof),
    "Published build metadata does not match the recomputed admission proof",
  );
  for (const architecture of ARCHITECTURES) {
    const declared = assetByArchitecture(proof.assets, architecture);
    const bytes = signedBytesByArchitecture[architecture];
    assertBytes(bytes, `${architecture} published installer`);
    assert(
      sha256(bytes) === declared.signedSha256,
      `${architecture} published installer SHA-256 does not match the admission proof`,
    );
  }
  return true;
}

function readJsonFile(filePath, label) {
  try {
    const text = readFileSync(filePath, "utf8");
    assert(Buffer.byteLength(text) <= MAX_TEXT_BYTES, `${label} exceeds the admission text limit`);
    return JSON.parse(text);
  } catch (error) {
    fail(`${label} is not JSON: ${error instanceof Error ? error.message : String(error)}`);
  }
}

function readTextFile(filePath, label) {
  try {
    return readFileSync(filePath, "utf8");
  } catch (error) {
    fail(`${label} is missing: ${error instanceof Error ? error.message : String(error)}`);
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
    assert(
      typeof name === "string" && name.startsWith("--") && value !== undefined,
      "Windows formal offline admission options must be --name value pairs",
    );
    assert(!options.has(name), `Duplicate admission option: ${name}`);
    options.set(name, value);
  }
  return options;
}

function writeGithubOutput(filePath, values) {
  const lines = Object.entries(values).map(([key, value]) => {
    assert(/^[a-z0-9_-]+$/u.test(key), "GitHub output name is invalid");
    assert(
      typeof value === "string" && /^[A-Za-z0-9._-]+$/u.test(value),
      "GitHub output value is invalid",
    );
    return `${key}=${value}`;
  });
  writeFileSync(filePath, `${lines.join("\n")}\n`, { flag: "a" });
}

function runCli(args = process.argv.slice(2)) {
  const [command, ...rest] = args;
  const options = parseOptions(rest);
  if (command === "classify") {
    const manifestPath = options.get("--manifest");
    const absent = options.get("--manifest-absent");
    assert(
      (manifestPath === undefined) !== (absent === undefined) &&
        (absent === undefined || absent === "yes"),
      "classify requires exactly one of --manifest or --manifest-absent yes",
    );
    const source = classifyWindowsByteSource({
      mode: optionValue(options, "--mode"),
      version: optionValue(options, "--version"),
      manifest:
        manifestPath === undefined ? null : readJsonFile(manifestPath, "admission manifest"),
    });
    writeGithubOutput(optionValue(options, "--github-output"), {
      windows_byte_source: source,
    });
    return;
  }
  if (command === "print-dispatch") {
    const manifest = readFormalOfflineAdmissionManifest(
      readJsonFile(optionValue(options, "--manifest"), "admission manifest"),
    );
    writeGithubOutput(optionValue(options, "--github-output"), {
      artifact_source_sha: manifest.artifactSourceSha,
      preflight_run_id: manifest.preflightRunId,
      preflight_run_attempt: manifest.preflightRunAttempt,
      offline_run_id: manifest.offlineRunId,
      offline_run_attempt: manifest.offlineRunAttempt,
    });
    return;
  }
  if (command === "prove") {
    const proof = proveFormalOfflineAdmission({
      manifest: readJsonFile(optionValue(options, "--manifest"), "admission manifest"),
      policy: readJsonFile(optionValue(options, "--policy"), "signing policy"),
      releaseSha: optionValue(options, "--release-sha"),
      ancestor: optionValue(options, "--ancestor") === "true",
      artifactLsTree: readTextFile(optionValue(options, "--artifact-tree"), "artifact tree"),
      releaseLsTree: readTextFile(optionValue(options, "--release-tree"), "release tree"),
      diffStatus: readTextFile(optionValue(options, "--diff"), "release diff"),
      artifactWorkflow: readTextFile(
        optionValue(options, "--artifact-workflow"),
        "artifact workflow",
      ),
      releaseWorkflow: readTextFile(
        optionValue(options, "--release-workflow"),
        "release workflow",
      ),
      preflightRun: readJsonFile(optionValue(options, "--preflight-run"), "preflight run"),
      offlineRun: readJsonFile(optionValue(options, "--offline-run"), "offline run"),
      binding: readJsonFile(optionValue(options, "--binding"), "offline binding"),
      rawBytesByArchitecture: {
        x64: readFileSync(optionValue(options, "--raw-x64")),
        arm64: readFileSync(optionValue(options, "--raw-arm64")),
      },
      signedBytesByArchitecture: {
        x64: readFileSync(optionValue(options, "--signed-x64")),
        arm64: readFileSync(optionValue(options, "--signed-arm64")),
      },
    });
    writeFileSync(optionValue(options, "--output"), `${JSON.stringify(proof, null, 2)}\n`, {
      flag: "wx",
    });
    return;
  }
  if (command === "assert-metadata") {
    assertAdmittedWindowsMetadata(
      optionValue(options, "--metadata"),
      readJsonFile(optionValue(options, "--proof"), "admission proof"),
    );
    return;
  }
  if (command === "bind-status") {
    const proof = readJsonFile(optionValue(options, "--proof"), "admission proof");
    const bound = bindFormalSigningStatus(
      readJsonFile(optionValue(options, "--status"), "signing status"),
      proof,
    );
    writeFileSync(optionValue(options, "--output"), `${JSON.stringify(bound, null, 2)}\n`, {
      flag: "wx",
    });
    return;
  }
  if (command === "assert-publish") {
    const proof = proveFormalOfflineAdmission({
      manifest: readJsonFile(optionValue(options, "--manifest"), "admission manifest"),
      policy: readJsonFile(optionValue(options, "--policy"), "signing policy"),
      releaseSha: optionValue(options, "--release-sha"),
      ancestor: optionValue(options, "--ancestor") === "true",
      artifactLsTree: readTextFile(optionValue(options, "--artifact-tree"), "artifact tree"),
      releaseLsTree: readTextFile(optionValue(options, "--release-tree"), "release tree"),
      diffStatus: readTextFile(optionValue(options, "--diff"), "release diff"),
      artifactWorkflow: readTextFile(
        optionValue(options, "--artifact-workflow"),
        "artifact workflow",
      ),
      releaseWorkflow: readTextFile(
        optionValue(options, "--release-workflow"),
        "release workflow",
      ),
      preflightRun: readJsonFile(optionValue(options, "--preflight-run"), "preflight run"),
      offlineRun: readJsonFile(optionValue(options, "--offline-run"), "offline run"),
      binding: readJsonFile(optionValue(options, "--binding"), "offline binding"),
      rawBytesByArchitecture: {
        x64: readFileSync(optionValue(options, "--raw-x64")),
        arm64: readFileSync(optionValue(options, "--raw-arm64")),
      },
      signedBytesByArchitecture: {
        x64: readFileSync(optionValue(options, "--signed-x64")),
        arm64: readFileSync(optionValue(options, "--signed-arm64")),
      },
    });
    assertPublishedAdmission({
      proof,
      signingStatus: readJsonFile(optionValue(options, "--status"), "signing status"),
      buildMetadata: readJsonFile(optionValue(options, "--build-metadata"), "build metadata"),
      signedBytesByArchitecture: {
        x64: readFileSync(optionValue(options, "--signed-x64")),
        arm64: readFileSync(optionValue(options, "--signed-arm64")),
      },
    });
    return;
  }
  fail(
    "Usage: windows-formal-offline-admission.mjs classify | print-dispatch | prove | assert-metadata | bind-status | assert-publish",
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
