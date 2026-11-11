#!/usr/bin/env node

import { mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import {
  CI_WORKFLOW_PATH,
  RELEASE_WORKFLOW_PATH,
  buildBuildMetadata,
} from "./release-contract.mjs";
import { stageMetadataForAggregate } from "./windows-formal-offline-admission.mjs";

const [
  metadataDirectory,
  version,
  tag,
  sourceSha,
  repository,
  repositoryId,
  runId,
  runAttempt,
  event,
  mode,
  workflowRef,
  workflowSha,
  ciRunId,
  ciRunAttempt,
  generatedAt,
  output = "build-metadata.json",
] = process.argv.slice(2);

if (
  !metadataDirectory ||
  !version ||
  !tag ||
  !sourceSha ||
  !repository ||
  !repositoryId ||
  !runId ||
  !runAttempt ||
  !event ||
  !mode ||
  !workflowRef ||
  !workflowSha ||
  ciRunId === undefined ||
  ciRunAttempt === undefined ||
  !generatedAt
) {
  console.error(
    "Usage: node scripts/release/generate-build-metadata.mjs <metadata-dir> <version> <tag> <source-sha> <repository> <repository-id> <run-id> <run-attempt> <event> <mode> <workflow-ref> <workflow-sha> <ci-run-id> <ci-run-attempt> <generated-at> [output]",
  );
  process.exit(1);
}

function optionalCiId(value) {
  if (value === "" || value === "null") return null;
  return value;
}

function formalRepairRequested() {
  const value = process.env.FYAGENT_FORMAL_REPAIR;
  if (value === undefined) return false;
  if (value === "true") return true;
  if (value === "false") return false;
  throw new Error('FYAGENT_FORMAL_REPAIR must be "true" or "false"');
}

try {
  const identity = {
    productVersion: version,
    tag,
    sourceSha,
    repository,
    repositoryId,
    workflowPath: RELEASE_WORKFLOW_PATH,
    workflowRef,
    workflowSha,
    runId,
    runAttempt,
    event,
    mode,
    ciWorkflowPath: CI_WORKFLOW_PATH,
    ciRunId: optionalCiId(ciRunId),
    ciRunAttempt: optionalCiId(ciRunAttempt),
  };
  const proofPath = process.env.FYAGENT_WINDOWS_OFFLINE_PROOF;
  let aggregateDirectory = metadataDirectory;
  let offlineAdmission;
  if (proofPath) {
    const proof = JSON.parse(readFileSync(proofPath, "utf8"));
    // This directory already exists and is empty. The stager accepts that slot.
    const staged = mkdtempSync(path.join(tmpdir(), "fyagent-admitted-metadata-"));
    stageMetadataForAggregate({
      sourceDirectory: metadataDirectory,
      destinationDirectory: staged,
      proof,
      releaseIdentity: identity,
    });
    aggregateDirectory = staged;
    offlineAdmission = proof;
  }
  const metadata = {
    ...buildBuildMetadata({
      metadataDirectory: aggregateDirectory,
      identity,
      generatedAt,
      repair: formalRepairRequested(),
    }),
    ...(offlineAdmission ? { offlineAdmission } : {}),
  };
  writeFileSync(output, `${JSON.stringify(metadata, null, 2)}\n`, {
    flag: "wx",
  });
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error));
  process.exit(1);
}
