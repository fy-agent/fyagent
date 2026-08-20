import { spawnSync } from "node:child_process";
import path from "node:path";
import { describe, expect, it } from "vitest";
// @ts-expect-error The workflow executes this dependency-free JavaScript helper directly.
import * as requiredGateModule from "../scripts/ci/required-gate.mjs";

type Plan = {
  domains: Record<string, boolean>;
  unknownPaths: string[];
  forceFull: boolean;
};

const ROOT = path.resolve(__dirname, "..");
const REQUIRED_JOB_IDS =
  requiredGateModule.REQUIRED_CI_JOBS as readonly string[];
const DEPENDENCY_JOB_IDS =
  requiredGateModule.REQUIRED_CI_DEPENDENCIES as readonly string[];
const evaluateRequiredCiResults =
  requiredGateModule.evaluateRequiredCiResults as (
    needs: unknown,
    plan: unknown,
    jobs: unknown,
  ) => {
    ok: boolean;
    requestedJobs: Record<string, boolean>;
    results: Record<string, string>;
    conclusions: Record<string, string | null>;
    errors: string[];
  };

const DISPLAY_NAMES: Record<string, string[]> = {
  changes: ["Classify Changes"],
  contracts: ["Repository Contracts"],
  frontend: ["Frontend Checks"],
  "desktop-acceptance-contract": ["Desktop Acceptance Contract"],
  "backend-windows": ["Backend Checks (Windows)"],
  "windows-native-contracts": [
    "Windows Native Contracts (X64)",
    "Windows Native Contracts (ARM64)",
  ],
  "backend-macos": ["Backend Checks (macOS)"],
};

function fullPlan(): Plan {
  return {
    domains: {
      contracts: true,
      frontend: true,
      desktop: true,
      backend: true,
      windowsNative: true,
      docsSpec: true,
    },
    unknownPaths: [],
    forceFull: true,
  };
}

function docsPlan(): Plan {
  return {
    domains: {
      contracts: false,
      frontend: false,
      desktop: false,
      backend: false,
      windowsNative: false,
      docsSpec: true,
    },
    unknownPaths: [],
    forceFull: false,
  };
}

function requestedJobs(plan: Plan): Record<string, boolean> {
  return requiredGateModule.requestedJobsForPlan(plan) as Record<
    string,
    boolean
  >;
}

function needs(plan: Plan): Record<string, { result: string }> {
  const requested = requestedJobs(plan);
  return Object.fromEntries(
    DEPENDENCY_JOB_IDS.map((job) => [
      job,
      { result: job === "changes" || requested[job] ? "success" : "skipped" },
    ]),
  );
}

function attemptJobs(plan: Plan): {
  total_count: number;
  jobs: Array<{ name: string; conclusion: string }>;
} {
  const input = needs(plan);
  const jobs = DEPENDENCY_JOB_IDS.flatMap((job) =>
    DISPLAY_NAMES[job].map((name) => ({
      name,
      conclusion: input[job].result,
    })),
  );
  return { total_count: jobs.length, jobs };
}

function setConclusion(
  jobs: ReturnType<typeof attemptJobs>,
  name: string,
  conclusion: string,
): void {
  const job = jobs.jobs.find((entry) => entry.name === name);
  expect(job).toBeDefined();
  job!.conclusion = conclusion;
}

describe("CI / Required gate", () => {
  it("accepts the complete requested full-CI dependency set", () => {
    const plan = fullPlan();
    const report = evaluateRequiredCiResults(
      needs(plan),
      plan,
      attemptJobs(plan),
    );
    expect(report.ok).toBe(true);
    expect(report.requestedJobs).toEqual(
      Object.fromEntries(REQUIRED_JOB_IDS.map((job) => [job, true])),
    );
    expect(report.errors).toEqual([]);
  });

  it("accepts only classifier-authorized skips for docs/spec-only changes", () => {
    const plan = docsPlan();
    const report = evaluateRequiredCiResults(
      needs(plan),
      plan,
      attemptJobs(plan),
    );
    expect(report.ok).toBe(true);
    expect(report.requestedJobs).toEqual({
      contracts: true,
      frontend: false,
      "desktop-acceptance-contract": false,
      "backend-windows": false,
      "windows-native-contracts": false,
      "backend-macos": false,
    });
  });

  it.each([
    ["failure", "failure"],
    ["cancelled", "cancelled"],
    ["failure", "timed_out"],
  ])(
    "rejects a requested needs=%s / API=%s conclusion",
    (result, conclusion) => {
      const plan = fullPlan();
      const input = needs(plan);
      input.frontend.result = result;
      const jobs = attemptJobs(plan);
      setConclusion(jobs, "Frontend Checks", conclusion);
      const report = evaluateRequiredCiResults(input, plan, jobs);
      expect(report.ok).toBe(false);
      expect(report.errors).toContain(
        `required job frontend finished with ${conclusion}`,
      );
    },
  );

  it("rejects a required skip and an unrequested job that runs", () => {
    const full = fullPlan();
    const fullNeeds = needs(full);
    fullNeeds.frontend.result = "skipped";
    const fullJobs = attemptJobs(full);
    setConclusion(fullJobs, "Frontend Checks", "skipped");
    expect(
      evaluateRequiredCiResults(fullNeeds, full, fullJobs).errors,
    ).toContain("required job frontend finished with skipped");

    const docs = docsPlan();
    const docsNeeds = needs(docs);
    docsNeeds.frontend.result = "success";
    const docsJobs = attemptJobs(docs);
    setConclusion(docsJobs, "Frontend Checks", "success");
    expect(
      evaluateRequiredCiResults(docsNeeds, docs, docsJobs).errors,
    ).toContain("non-requested job frontend finished with success");
  });

  it("fails closed for unknown paths, malformed force-full plans, and classifier failure", () => {
    const unknown = docsPlan();
    unknown.unknownPaths = ["new-area/file.txt"];
    expect(
      evaluateRequiredCiResults(needs(unknown), unknown, attemptJobs(unknown))
        .errors,
    ).toContain("unclassified paths: new-area/file.txt");

    const malformed = docsPlan();
    malformed.forceFull = true;
    expect(
      evaluateRequiredCiResults(
        needs(malformed),
        malformed,
        attemptJobs(malformed),
      ).errors,
    ).toContain("forceFull classification must request every domain");

    const plan = fullPlan();
    const failedNeeds = needs(plan);
    failedNeeds.changes.result = "failure";
    const failedJobs = attemptJobs(plan);
    setConclusion(failedJobs, "Classify Changes", "timed_out");
    expect(
      evaluateRequiredCiResults(failedNeeds, plan, failedJobs).errors,
    ).toContain("change classifier finished with timed_out");
  });

  it("rejects incomplete attempt pagination and result/conclusion drift", () => {
    const plan = fullPlan();
    const incomplete = attemptJobs(plan);
    incomplete.total_count += 1;
    expect(
      evaluateRequiredCiResults(needs(plan), plan, incomplete).errors,
    ).toContain(
      `current run-attempt jobs are incomplete: expected ${incomplete.total_count}, received ${incomplete.jobs.length}`,
    );

    const drift = attemptJobs(plan);
    setConclusion(drift, "Frontend Checks", "failure");
    expect(
      evaluateRequiredCiResults(needs(plan), plan, drift).errors,
    ).toContain(
      "required job frontend result/conclusion mismatch: success/failure",
    );

    const missingClassifier = attemptJobs(plan);
    missingClassifier.jobs = missingClassifier.jobs.filter(
      ({ name }) => name !== "Classify Changes",
    );
    missingClassifier.total_count = missingClassifier.jobs.length;
    expect(
      evaluateRequiredCiResults(needs(plan), plan, missingClassifier).errors,
    ).toContain("change classifier is missing from current run-attempt jobs");

    const missingRequested = attemptJobs(plan);
    missingRequested.jobs = missingRequested.jobs.filter(
      ({ name }) => name !== "Frontend Checks",
    );
    missingRequested.total_count = missingRequested.jobs.length;
    expect(
      evaluateRequiredCiResults(needs(plan), plan, missingRequested).errors,
    ).toContain(
      "required job frontend is missing from current run-attempt jobs",
    );
  });

  it("rejects missing, extra, unknown-result, and malformed plan keys", () => {
    const plan = fullPlan();
    const missing = needs(plan);
    delete missing["backend-macos"];
    expect(
      evaluateRequiredCiResults(missing, plan, attemptJobs(plan)).errors[0],
    ).toContain("needs keys must be exactly");

    const extra = { ...needs(plan), optional: { result: "success" } };
    expect(
      evaluateRequiredCiResults(extra, plan, attemptJobs(plan)).errors[0],
    ).toContain("needs keys must be exactly");

    const unknown = needs(plan);
    unknown["backend-macos"].result = "neutral";
    expect(
      evaluateRequiredCiResults(unknown, plan, attemptJobs(plan)).errors,
    ).toContain("unknown result for backend-macos: neutral");

    const extraPlan = { ...plan, event: "pull_request" };
    expect(
      evaluateRequiredCiResults(needs(plan), extraPlan, attemptJobs(plan))
        .errors[0],
    ).toContain("classification plan keys must be exactly");
  });

  it("fails closed on malformed CLI input and emits a machine-readable report", () => {
    const result = spawnSync(
      process.execPath,
      [
        "scripts/ci/required-gate.mjs",
        "--results-json",
        "{",
        "--plan-json",
        JSON.stringify(fullPlan()),
        "--jobs-json",
        JSON.stringify(attemptJobs(fullPlan())),
      ],
      { cwd: ROOT, encoding: "utf8" },
    );
    expect(result.status).toBe(1);
    const report = JSON.parse(result.stdout) as {
      ok: boolean;
      errors: string[];
    };
    expect(report.ok).toBe(false);
    expect(report.errors[0]).toContain("not valid JSON");
  });
});
