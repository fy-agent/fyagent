#!/usr/bin/env node

import { writeFileSync } from "node:fs";
import {
  CI_WORKFLOW_PATH,
  RELEASE_WORKFLOW_PATH,
  buildBuildMetadata,
} from "./release-contract.mjs";

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

try {
  const metadata = buildBuildMetadata({
    metadataDirectory,
    identity: {
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
    },
    generatedAt,
  });
  writeFileSync(output, `${JSON.stringify(metadata, null, 2)}\n`, {
    flag: "wx",
  });
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error));
  process.exit(1);
}
