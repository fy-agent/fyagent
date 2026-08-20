#!/usr/bin/env node

import { fail, isMain, run } from "./lib.mjs";

const CI_SAFE_TESTS = Object.freeze([
  "tests/classifyChanges.test.ts",
  "tests/devReleaseEligibility.test.ts",
  "tests/devReleaseRemote.test.ts",
  "tests/releaseWorkflow.test.ts",
  "tests/hdiutilRetry.test.ts",
  "tests/downloadManifest.test.ts",
  "tests/releaseAssets.test.ts",
  "tests/windowsNsisContract.test.ts",
  "tests/windowsSetupIcon.test.ts",
  "tests/windowsSigningAdapter.test.ts",
  "tests/writePlatformMetadata.test.ts",
  "tests/githubWorkflowTriggers.test.ts",
  "tests/ciWorkflow.test.ts",
  "tests/ciStepOutcomes.test.ts",
  "tests/currentDocsContract.test.ts",
  "tests/repositoryGovernanceScan.test.ts",
  "tests/codexWindowsUserScopeContract.test.ts",
  "tests/formatFiles.test.ts",
  "tests/taskAtomicWriter.test.ts",
  "tests/requiredCiGate.test.ts",
  "tests/ciToolchainContract.test.ts",
  "tests/dep0040Contract.test.ts",
  "tests/localBuildBoundary.test.ts",
  "tests/releaseCheckAggregation.test.ts",
]);

const LOCAL_MISE_TESTS = Object.freeze([
  "tests/developmentEnvironment.test.ts",
  "tests/miseTaskContract.test.ts",
  "tests/taskDocs.test.ts",
  "tests/systemCheck.test.ts",
]);

export function parseReleaseCheckMode(args) {
  const ciMode = args.length === 1 && args[0] === "--ci";
  if (args.length > 0 && !ciMode) {
    throw new Error("Usage: release-check.mjs [--ci]");
  }
  return ciMode;
}

export function releaseCheckPlan(ciMode) {
  const plan = [
    ["version", "pnpm", ["run", "version:check"]],
    ["lockfile", "node", ["scripts/tasks/lockfile-check.mjs"]],
    ["dep0040", "node", ["scripts/tasks/dep0040-check.mjs"]],
  ];
  if (!ciMode) {
    plan.push([
      "task-contract",
      "node",
      ["scripts/tasks/task-contract-check.mjs"],
    ]);
  }
  plan.push(
    ["task-docs", "node", ["scripts/tasks/task-docs.mjs", "check"]],
    [
      "windows-nsis-contract",
      "node",
      ["scripts/release/verify-windows-nsis-contract.mjs"],
    ],
    [
      "supported-platform",
      "node",
      ["scripts/tasks/supported-platform-check.mjs"],
    ],
    [
      "contract-tests",
      "pnpm",
      [
        "run",
        "test:unit",
        ...CI_SAFE_TESTS,
        ...(ciMode ? [] : LOCAL_MISE_TESTS),
      ],
    ],
  );
  if (!ciMode) {
    plan.push(["native-fetch", "pnpm", ["run", "test:native-fetch"]]);
  }
  return plan;
}

export function runReleaseChecks(ciMode, execute = run) {
  const plan = releaseCheckPlan(ciMode);
  const failures = [];

  for (const [id, command, args] of plan) {
    console.log(`[release-check] running ${id}`);
    try {
      execute(command, args);
    } catch (error) {
      const detail = error instanceof Error ? error.message : String(error);
      console.error(`[release-check] ${id} failed: ${detail}`);
      failures.push(new Error(`${id}: ${detail}`, { cause: error }));
    }
  }

  if (failures.length > 0) {
    const failedIds = failures.map((error) => error.message.split(":", 1)[0]);
    throw new AggregateError(
      failures,
      `${failures.length} release diagnostic(s) failed: ${failedIds.join(", ")}`,
    );
  }
}

if (isMain(import.meta.url)) {
  try {
    runReleaseChecks(parseReleaseCheckMode(process.argv.slice(2)));
  } catch (error) {
    fail(error);
  }
}
