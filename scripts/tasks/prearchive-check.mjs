#!/usr/bin/env node

import process from "node:process";
import { fail, isMain, run, usageValue } from "./lib.mjs";
import { validateActiveTaskExclusion } from "./supported-platform-check.mjs";

const ACTIVE_TASK_ENV = "FYAGENT_SUPPORTED_PLATFORM_ACTIVE_TASK";

export function resolvePrearchiveTarget(mode) {
  if (mode === "full") return "check";
  if (mode === "contracts") return "check:contracts";
  throw new Error("Prearchive check mode must be full or contracts");
}

export function prearchiveEnvironment(
  activeTask,
  environment = process.env,
  validator = validateActiveTaskExclusion,
) {
  if (!activeTask) {
    throw new Error("--exclude-active-task is required before archival");
  }
  if (environment[ACTIVE_TASK_ENV]) {
    throw new Error(`${ACTIVE_TASK_ENV} must not be set by the caller`);
  }
  const validated = validator(activeTask);
  return {
    ...environment,
    // The public mise usage value belongs to this wrapper. Clear it before the
    // nested composite so the leaf receives exactly one private input source.
    usage_exclude_active_task: "",
    [ACTIVE_TASK_ENV]: validated,
  };
}

export function runPrearchiveCheck(mode, options = {}) {
  const target = resolvePrearchiveTarget(mode);
  const activeTask = options.activeTask ?? usageValue("exclude_active_task");
  const environment = prearchiveEnvironment(
    activeTask,
    options.environment,
    options.validator,
  );
  const runner = options.runner ?? run;
  return runner("mise", ["run", target], { env: environment });
}

if (isMain(import.meta.url)) {
  try {
    if (process.argv.length !== 3) {
      throw new Error("Usage: prearchive-check.mjs <full|contracts>");
    }
    runPrearchiveCheck(process.argv[2]);
  } catch (error) {
    fail(error);
  }
}
