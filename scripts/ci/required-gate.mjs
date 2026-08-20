#!/usr/bin/env node

import process from "node:process";
import { pathToFileURL } from "node:url";

export const CI_DOMAINS = Object.freeze([
  "contracts",
  "frontend",
  "desktop",
  "backend",
  "windowsNative",
  "docsSpec",
]);

export const REQUIRED_CI_JOBS = Object.freeze([
  "contracts",
  "frontend",
  "desktop-acceptance-contract",
  "backend-windows",
  "windows-native-contracts",
  "backend-macos",
]);

export const REQUIRED_CI_DEPENDENCIES = Object.freeze([
  "changes",
  ...REQUIRED_CI_JOBS,
]);

const KNOWN_NEEDS_RESULTS = new Set([
  "success",
  "failure",
  "cancelled",
  "skipped",
]);
const KNOWN_JOB_CONCLUSIONS = new Set([
  "success",
  "failure",
  "cancelled",
  "skipped",
  "timed_out",
  "action_required",
  "neutral",
  "stale",
  "startup_failure",
]);

const DISPLAY_NAME_MATCHERS = Object.freeze({
  changes: (name) => name === "Classify Changes",
  contracts: (name) => name === "Repository Contracts",
  frontend: (name) => name === "Frontend Checks",
  "desktop-acceptance-contract": (name) =>
    name === "Desktop Acceptance Contract",
  "backend-windows": (name) => name === "Backend Checks (Windows)",
  "windows-native-contracts": (name) =>
    /^Windows Native Contracts(?: \((?:X64|ARM64)\))?$/.test(name),
  "backend-macos": (name) => name === "Backend Checks (macOS)",
});

function isPlainObject(value) {
  return (
    value !== null &&
    typeof value === "object" &&
    !Array.isArray(value) &&
    (Object.getPrototypeOf(value) === Object.prototype ||
      Object.getPrototypeOf(value) === null)
  );
}

function assertExactKeys(value, expected, label, errors) {
  if (!isPlainObject(value)) {
    errors.push(`${label} must be a JSON object`);
    return false;
  }
  const actual = Object.keys(value).sort();
  const wanted = [...expected].sort();
  if (JSON.stringify(actual) !== JSON.stringify(wanted)) {
    errors.push(
      `${label} keys must be exactly ${wanted.join(", ")}; received ${actual.join(", ")}`,
    );
    return false;
  }
  return true;
}

export function requestedJobsForPlan(plan) {
  const domains = plan.domains;
  return Object.freeze({
    contracts: domains.contracts || domains.docsSpec,
    frontend: domains.frontend,
    "desktop-acceptance-contract": domains.desktop,
    "backend-windows": domains.backend || domains.windowsNative,
    "windows-native-contracts": domains.windowsNative,
    "backend-macos": domains.backend,
  });
}

function validatePlan(value, errors) {
  if (
    !assertExactKeys(
      value,
      ["domains", "unknownPaths", "forceFull"],
      "classification plan",
      errors,
    )
  ) {
    return null;
  }
  if (
    !assertExactKeys(
      value.domains,
      CI_DOMAINS,
      "classification domains",
      errors,
    )
  ) {
    return null;
  }
  for (const domain of CI_DOMAINS) {
    if (typeof value.domains[domain] !== "boolean") {
      errors.push(`classification domain ${domain} must be boolean`);
    }
  }
  if (!Array.isArray(value.unknownPaths)) {
    errors.push("classification unknownPaths must be an array");
  } else {
    const normalized = value.unknownPaths.filter(
      (entry) => typeof entry === "string" && entry.length > 0,
    );
    if (normalized.length !== value.unknownPaths.length) {
      errors.push("classification unknownPaths must contain non-empty strings");
    }
    if (
      normalized.length !== new Set(normalized).size ||
      JSON.stringify(normalized) !== JSON.stringify([...normalized].sort())
    ) {
      errors.push("classification unknownPaths must be unique and sorted");
    }
    if (normalized.length > 0) {
      errors.push(`unclassified paths: ${normalized.join(", ")}`);
    }
  }
  if (typeof value.forceFull !== "boolean") {
    errors.push("classification forceFull must be boolean");
  } else if (
    value.forceFull &&
    CI_DOMAINS.some((domain) => value.domains[domain] !== true)
  ) {
    errors.push("forceFull classification must request every domain");
  }
  return errors.length === 0 ? value : null;
}

function normalizeAttemptJobs(value, errors) {
  if (!isPlainObject(value)) {
    errors.push("current run-attempt jobs must be a JSON object");
    return {};
  }
  if (!Number.isInteger(value.total_count) || value.total_count < 0) {
    errors.push(
      "current run-attempt total_count must be a non-negative integer",
    );
    return {};
  }
  if (!Array.isArray(value.jobs)) {
    errors.push("current run-attempt jobs must contain a jobs array");
    return {};
  }
  if (value.total_count !== value.jobs.length) {
    errors.push(
      `current run-attempt jobs are incomplete: expected ${value.total_count}, received ${value.jobs.length}`,
    );
  }

  const conclusions = Object.fromEntries(
    REQUIRED_CI_DEPENDENCIES.map((job) => [job, []]),
  );
  for (const [index, entry] of value.jobs.entries()) {
    if (!isPlainObject(entry) || typeof entry.name !== "string") {
      errors.push(`current run-attempt job ${index} has no name`);
      continue;
    }
    const matched = REQUIRED_CI_DEPENDENCIES.filter((job) =>
      DISPLAY_NAME_MATCHERS[job](entry.name),
    );
    if (matched.length === 0) continue;
    if (matched.length !== 1) {
      errors.push(`ambiguous current run-attempt job name: ${entry.name}`);
      continue;
    }
    if (
      typeof entry.conclusion !== "string" ||
      !KNOWN_JOB_CONCLUSIONS.has(entry.conclusion)
    ) {
      errors.push(
        `unknown current run-attempt conclusion for ${entry.name}: ${String(entry.conclusion)}`,
      );
      continue;
    }
    conclusions[matched[0]].push(entry.conclusion);
  }
  return conclusions;
}

function aggregateConclusions(entries) {
  if (entries.length === 0) return null;
  if (entries.includes("timed_out")) return "timed_out";
  if (entries.includes("cancelled")) return "cancelled";
  if (
    entries.some((entry) =>
      [
        "failure",
        "action_required",
        "neutral",
        "stale",
        "startup_failure",
      ].includes(entry),
    )
  ) {
    return "failure";
  }
  if (entries.every((entry) => entry === "success")) return "success";
  if (entries.every((entry) => entry === "skipped")) return "skipped";
  return "mixed";
}

export function evaluateRequiredCiResults(
  needsValue,
  planValue,
  attemptJobsValue,
) {
  const errors = [];
  const results = {};
  const conclusions = {};

  const plan = validatePlan(planValue, errors);
  const requestedJobs = plan
    ? requestedJobsForPlan(plan)
    : Object.fromEntries(REQUIRED_CI_JOBS.map((job) => [job, false]));

  if (assertExactKeys(needsValue, REQUIRED_CI_DEPENDENCIES, "needs", errors)) {
    for (const job of REQUIRED_CI_DEPENDENCIES) {
      const entry = needsValue[job];
      if (!isPlainObject(entry) || typeof entry.result !== "string") {
        errors.push(`missing result for dependency job: ${job}`);
        continue;
      }
      results[job] = entry.result;
      if (!KNOWN_NEEDS_RESULTS.has(entry.result)) {
        errors.push(`unknown result for ${job}: ${entry.result}`);
      }
    }
  }

  const attemptConclusions = normalizeAttemptJobs(attemptJobsValue, errors);
  for (const job of REQUIRED_CI_DEPENDENCIES) {
    conclusions[job] = aggregateConclusions(attemptConclusions[job] ?? []);
  }

  if (results.changes && results.changes !== "success") {
    const observed = conclusions.changes ?? results.changes;
    errors.push(`change classifier finished with ${observed}`);
  } else if (results.changes === "success" && conclusions.changes === null) {
    errors.push("change classifier is missing from current run-attempt jobs");
  }

  for (const job of REQUIRED_CI_JOBS) {
    const result = results[job];
    if (!result || !KNOWN_NEEDS_RESULTS.has(result)) continue;
    if (requestedJobs[job]) {
      if (result !== "success") {
        const observed = conclusions[job] ?? result;
        errors.push(`required job ${job} finished with ${observed}`);
      } else if (conclusions[job] === null) {
        errors.push(
          `required job ${job} is missing from current run-attempt jobs`,
        );
      } else if (conclusions[job] !== null && conclusions[job] !== "success") {
        errors.push(
          `required job ${job} result/conclusion mismatch: success/${conclusions[job]}`,
        );
      }
    } else if (result !== "skipped") {
      const observed = conclusions[job] ?? result;
      errors.push(`non-requested job ${job} finished with ${observed}`);
    } else if (conclusions[job] !== null && conclusions[job] !== "skipped") {
      errors.push(
        `non-requested job ${job} result/conclusion mismatch: skipped/${conclusions[job]}`,
      );
    }
  }

  return {
    ok: errors.length === 0,
    requestedJobs,
    results,
    conclusions,
    errors,
  };
}

function argumentValue(argv, name) {
  const indexes = argv
    .map((argument, index) => (argument === name ? index : -1))
    .filter((index) => index >= 0);
  if (indexes.length > 1) throw new Error(`${name} may be provided only once`);
  if (indexes.length === 0) return null;
  const value = argv[indexes[0] + 1];
  if (!value || value.startsWith("--")) {
    throw new Error(`${name} requires a JSON value`);
  }
  return value;
}

function parseJson(label, serialized) {
  if (!serialized) throw new Error(`Missing ${label} JSON input`);
  try {
    return JSON.parse(serialized);
  } catch (error) {
    throw new Error(
      `${label} is not valid JSON: ${error instanceof Error ? error.message : String(error)}`,
    );
  }
}

export function runRequiredGateCli(
  argv = process.argv.slice(2),
  env = process.env,
) {
  let report;
  try {
    const knownArguments = new Set([
      "--results-json",
      "--plan-json",
      "--jobs-json",
    ]);
    for (let index = 0; index < argv.length; index += 2) {
      if (!knownArguments.has(argv[index])) {
        throw new Error(`Unknown argument: ${argv[index]}`);
      }
    }
    const needs = parseJson(
      "required results",
      argumentValue(argv, "--results-json") ?? env.REQUIRED_RESULTS,
    );
    const plan = parseJson(
      "classification plan",
      argumentValue(argv, "--plan-json") ?? env.CI_CLASSIFICATION_PLAN,
    );
    const jobs = parseJson(
      "current run-attempt jobs",
      argumentValue(argv, "--jobs-json") ?? env.CURRENT_RUN_JOBS,
    );
    report = evaluateRequiredCiResults(needs, plan, jobs);
  } catch (error) {
    report = {
      ok: false,
      requestedJobs: {},
      results: {},
      conclusions: {},
      errors: [error instanceof Error ? error.message : String(error)],
    };
  }
  console.log(JSON.stringify(report, null, 2));
  if (!report.ok) process.exitCode = 1;
  return report;
}

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(process.argv[1]).href
) {
  runRequiredGateCli();
}
