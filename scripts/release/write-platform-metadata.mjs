#!/usr/bin/env node

import { writeFileSync } from "node:fs";
import {
  CI_WORKFLOW_PATH,
  EXPECTED_TARGETS,
  GITHUB_RUNNER_ARCHITECTURES,
  RELEASE_WORKFLOW_PATH,
} from "./release-contract.mjs";

const [output] = process.argv.slice(2);
if (!output) {
  console.error(
    "Usage: node scripts/release/write-platform-metadata.mjs <output>",
  );
  process.exit(1);
}

function required(name) {
  const value = process.env[name];
  if (!value?.trim())
    throw new Error(`Required environment variable is missing: ${name}`);
  return value.trim();
}

function requiredPositiveInteger(name) {
  const value = required(name);
  if (!/^[1-9]\d*$/.test(value)) {
    throw new Error(`${name} must be a positive decimal integer`);
  }
  return value;
}

function requireExpected(name, actual, expected, targetGroup) {
  if (actual !== expected) {
    throw new Error(
      `${name} must be ${expected} for ${targetGroup}; received ${actual}`,
    );
  }
}

try {
  const targetGroup = required("TARGET_GROUP");
  const expected = EXPECTED_TARGETS.find(
    (candidate) => candidate.targetGroup === targetGroup,
  );
  if (!expected) {
    throw new Error(`Unsupported target group: ${targetGroup}`);
  }

  const platform = required("TARGET_PLATFORM");
  const architecture = required("TARGET_ARCHITECTURE");
  const requestedRunnerLabel = required("REQUESTED_RUNNER_LABEL");
  const runnerOs = required("ACTUAL_RUNNER_OS");
  const runnerArch = required("ACTUAL_RUNNER_ARCH");
  requireExpected("TARGET_PLATFORM", platform, expected.platform, targetGroup);
  requireExpected(
    "TARGET_ARCHITECTURE",
    architecture,
    expected.architecture,
    targetGroup,
  );
  requireExpected(
    "REQUESTED_RUNNER_LABEL",
    requestedRunnerLabel,
    expected.requestedRunnerLabel,
    targetGroup,
  );
  requireExpected(
    "ACTUAL_RUNNER_OS",
    runnerOs,
    expected.expectedRunnerOs,
    targetGroup,
  );
  if (!GITHUB_RUNNER_ARCHITECTURES.includes(runnerArch)) {
    throw new Error(
      `ACTUAL_RUNNER_ARCH is not a documented GitHub runner architecture: ${runnerArch}`,
    );
  }
  requireExpected(
    "ACTUAL_RUNNER_ARCH",
    runnerArch,
    expected.expectedRunnerArch,
    targetGroup,
  );

  const mode = required("RELEASE_MODE");
  if (!(mode === "preflight" || mode === "formal")) {
    throw new Error(`Unsupported release mode: ${mode}`);
  }
  const ciRunId = requiredPositiveInteger("EXPECTED_CI_RUN_ID");
  const ciRunAttempt = requiredPositiveInteger("EXPECTED_CI_RUN_ATTEMPT");
  const metadata = {
    schema: "fyagent-platform-build/v2",
    targetGroup,
    platform,
    architecture,
    runner: {
      requestedLabel: requestedRunnerLabel,
      context: {
        os: runnerOs,
        arch: runnerArch,
      },
    },
    toolchain: {
      node: required("ACTUAL_NODE_VERSION"),
      pnpm: required("ACTUAL_PNPM_VERSION"),
      rustc: required("ACTUAL_RUST_VERSION"),
    },
    identity: {
      productVersion: required("APP_VERSION"),
      tag: required("RELEASE_TAG"),
      sourceSha: required("SOURCE_SHA"),
      repository: required("GITHUB_REPOSITORY"),
      repositoryId: required("GITHUB_REPOSITORY_ID"),
      workflowPath: RELEASE_WORKFLOW_PATH,
      workflowRef: required("GITHUB_WORKFLOW_REF"),
      workflowSha: required("GITHUB_WORKFLOW_SHA"),
      runId: required("GITHUB_RUN_ID"),
      runAttempt: required("GITHUB_RUN_ATTEMPT"),
      event: required("GITHUB_EVENT_NAME"),
      mode,
      ciWorkflowPath: CI_WORKFLOW_PATH,
      ciRunId,
      ciRunAttempt,
    },
  };
  writeFileSync(output, `${JSON.stringify(metadata, null, 2)}\n`, {
    flag: "wx",
  });
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error));
  process.exit(1);
}
