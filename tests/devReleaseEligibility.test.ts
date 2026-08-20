import { describe, expect, it } from "vitest";
import {
  DEV_RELEASE_ELIGIBILITY_INPUT_SCHEMA,
  evaluateDevReleaseEligibility,
  type DevReleaseEligibilityInput,
} from "../scripts/release/dev-release-eligibility.mjs";

const SOURCE_SHA = "a".repeat(40);
const OTHER_SHA = "c".repeat(40);
const TAG_OBJECT_SHA = "b".repeat(40);
const PRE_TRANSFER_REPOSITORY = ["NongHua123", "fyagent"].join("/");
const REPOSITORY = {
  nameWithOwner: "fy-agent/fyagent",
  id: "1313497021",
} as const;
const CI_WORKFLOW_ID = "314159";
const CI_RUN_ID = "9001";
const CI_RUN_ATTEMPT = "1";
const CHECK_SUITE_ID = "6001";
const REQUIRED_JOB_ID = "7001";
const REQUIRED_CHECK_ID = "8001";

type MutableRecord = Record<string, any>;

function requiredJob(runId = CI_RUN_ID, runAttempt = CI_RUN_ATTEMPT) {
  return {
    id: REQUIRED_JOB_ID,
    name: "CI / Required",
    runId,
    runAttempt,
    status: "completed" as const,
    conclusion: "success" as const,
    checkRunUrl: `https://api.github.com/repos/fy-agent/fyagent/check-runs/${REQUIRED_CHECK_ID}`,
    htmlUrl: `https://github.com/fy-agent/fyagent/actions/runs/${runId}/job/${REQUIRED_JOB_ID}`,
  };
}

function requiredCheck(runId = CI_RUN_ID, runAttempt = CI_RUN_ATTEMPT) {
  return {
    id: REQUIRED_CHECK_ID,
    name: "CI / Required",
    runId,
    runAttempt,
    checkSuiteId: CHECK_SUITE_ID,
    appSlug: "github-actions",
    headSha: SOURCE_SHA,
    status: "completed" as const,
    conclusion: "success" as const,
    url: `https://api.github.com/repos/fy-agent/fyagent/check-runs/${REQUIRED_CHECK_ID}`,
    detailsUrl: `https://github.com/fy-agent/fyagent/actions/runs/${runId}/job/${REQUIRED_JOB_ID}`,
  };
}

function ciRun(
  overrides: MutableRecord = {},
): DevReleaseEligibilityInput["ciRuns"][number] {
  return {
    id: CI_RUN_ID,
    runNumber: "42",
    runAttempt: CI_RUN_ATTEMPT,
    checkSuiteId: CHECK_SUITE_ID,
    repository: { ...REPOSITORY },
    headRepository: { ...REPOSITORY },
    workflow: {
      id: CI_WORKFLOW_ID,
      name: "CI",
      path: ".github/workflows/ci.yml",
    },
    event: "push",
    headBranch: "dev/laiyongjie",
    headSha: SOURCE_SHA,
    status: "completed",
    conclusion: "success",
    ...overrides,
  } as DevReleaseEligibilityInput["ciRuns"][number];
}

function validInput(
  mode: "preflight" | "formal" = "preflight",
): DevReleaseEligibilityInput {
  const releaseTag = "v0.3.1";
  const authorityBranch =
    mode === "preflight" ? "dev/laiyongjie" : "main";
  const ref =
    mode === "preflight"
      ? `refs/heads/${authorityBranch}`
      : `refs/tags/${releaseTag}`;
  return {
    schema: DEV_RELEASE_ELIGIBILITY_INPUT_SCHEMA,
    repository: { ...REPOSITORY },
    event: {
      dispatchSourceSha: mode === "preflight" ? SOURCE_SHA : null,
      name: mode === "preflight" ? "workflow_dispatch" : "push",
      ref,
      refName: mode === "preflight" ? authorityBranch : releaseTag,
      refType: mode === "preflight" ? "branch" : "tag",
      sha: SOURCE_SHA,
    },
    workflow: {
      name: "Release",
      path: ".github/workflows/release.yml",
      ref: `fy-agent/fyagent/.github/workflows/release.yml@${ref}`,
      sha: SOURCE_SHA,
    },
    candidate: {
      canonicalVersion: "0.3.1",
      releaseTag,
      sourceSha: SOURCE_SHA,
    },
    remoteDev: {
      name: authorityBranch,
      ref: `refs/heads/${authorityBranch}`,
      headSha: SOURCE_SHA,
    },
    remoteTag:
      mode === "preflight"
        ? null
        : {
            ref: `refs/tags/${releaseTag}`,
            refObject: {
              type: "tag",
              sha: TAG_OBJECT_SHA,
            },
            tagObject: {
              sha: TAG_OBJECT_SHA,
              name: releaseTag,
              target: {
                type: "commit",
                sha: SOURCE_SHA,
              },
            },
          },
    ciWorkflow: {
      id: CI_WORKFLOW_ID,
      name: "CI",
      path: ".github/workflows/ci.yml",
      state: "active",
      repository: { ...REPOSITORY },
    },
    ciRuns: [ciRun({ headBranch: authorityBranch })],
    ciEvidence: {
      runId: CI_RUN_ID,
      runAttempt: CI_RUN_ATTEMPT,
      checkSuiteId: CHECK_SUITE_ID,
      jobs: [requiredJob()],
      checkRuns: [requiredCheck()],
    },
  };
}

function mutableInput(
  mode: "preflight" | "formal" = "preflight",
): MutableRecord {
  return structuredClone(validInput(mode)) as MutableRecord;
}

function expectRejected(
  mutate: (input: MutableRecord) => void,
  expected?: RegExp,
  mode: "preflight" | "formal" = "preflight",
) {
  const input = mutableInput(mode);
  mutate(input);
  expect(() =>
    evaluateDevReleaseEligibility(input as DevReleaseEligibilityInput),
  ).toThrow(expected ?? /Dev release eligibility rejected/);
}

function bindEvidenceToRun(input: MutableRecord, run: MutableRecord) {
  input.ciEvidence.runId = run.id;
  input.ciEvidence.runAttempt = run.runAttempt;
  input.ciEvidence.checkSuiteId = run.checkSuiteId;
  input.ciEvidence.jobs = [requiredJob(run.id, run.runAttempt)];
  input.ciEvidence.checkRuns = [requiredCheck(run.id, run.runAttempt)];
}

describe("split preflight and formal release identity", () => {
  it.each(["preflight", "formal"] as const)(
    "freezes the exact eligible %s identity and successful authority-branch CI attempt",
    (mode) => {
      const output = evaluateDevReleaseEligibility(validInput(mode));

      expect(output).toEqual({
        appVersion: "0.3.1",
        releaseTag: "v0.3.1",
        sourceSha: SOURCE_SHA,
        workflowSha: SOURCE_SHA,
        ciRunId: CI_RUN_ID,
        ciRunAttempt: CI_RUN_ATTEMPT,
        mode,
      });
      expect(Object.isFrozen(output)).toBe(true);
      expect(evaluateDevReleaseEligibility(validInput(mode), output)).toEqual(
        output,
      );
    },
  );

  it("accepts only strict stable tags equal to the canonical version", () => {
    for (const [version, tag] of [
      ["1.2.3", "v1.2.3"],
      ["0.0.0", "v0.0.0"],
      ["18446744073709551615.2.3", "v18446744073709551615.2.3"],
    ]) {
      const input = mutableInput();
      input.candidate.canonicalVersion = version;
      input.candidate.releaseTag = tag;
      expect(
        evaluateDevReleaseEligibility(input as DevReleaseEligibilityInput),
      ).toMatchObject({
        appVersion: version,
        releaseTag: tag,
      });
    }
  });

  it.each([
    ["0.3", "v0.3"],
    ["01.3.1", "v01.3.1"],
    ["0.3.1-beta.1", "v0.3.1-beta.1"],
    ["0.3.1+build", "v0.3.1+build"],
    ["0.3.1", "0.3.1"],
    ["0.3.1", "v0.3.2"],
    ["18446744073709551616.2.3", "v18446744073709551616.2.3"],
  ])("rejects non-formal version/tag pair %s / %s", (version, tag) => {
    expectRejected((input) => {
      input.candidate.canonicalVersion = version;
      input.candidate.releaseTag = tag;
    });
  });

  it.each([
    [
      "repository name",
      (input: MutableRecord) =>
        (input.repository.nameWithOwner = PRE_TRANSFER_REPOSITORY),
    ],
    ["repository id", (input: MutableRecord) => (input.repository.id = "99")],
    [
      "workflow name",
      (input: MutableRecord) => (input.workflow.name = "Other"),
    ],
    [
      "workflow path",
      (input: MutableRecord) =>
        (input.workflow.path = ".github/workflows/other.yml"),
    ],
    [
      "workflow SHA",
      (input: MutableRecord) => (input.workflow.sha = OTHER_SHA),
    ],
    ["event SHA", (input: MutableRecord) => (input.event.sha = OTHER_SHA)],
    [
      "dispatch source SHA",
      (input: MutableRecord) => (input.event.dispatchSourceSha = OTHER_SHA),
    ],
    [
      "candidate SHA",
      (input: MutableRecord) => (input.candidate.sourceSha = OTHER_SHA),
    ],
    [
      "remote authority branch",
      (input: MutableRecord) => (input.remoteDev.name = "main"),
    ],
    [
      "remote authority ref",
      (input: MutableRecord) =>
        (input.remoteDev.ref = "refs/heads/main"),
    ],
    [
      "moved remote dev HEAD",
      (input: MutableRecord) => (input.remoteDev.headSha = OTHER_SHA),
    ],
  ])("rejects wrong %s identity", (_label, mutate) => {
    expectRejected(mutate);
  });

  it.each([
    [
      "event name",
      (input: MutableRecord) => (input.event.name = "pull_request"),
    ],
    [
      "branch ref",
      (input: MutableRecord) => (input.event.ref = "refs/heads/main"),
    ],
    [
      "branch ref type",
      (input: MutableRecord) => (input.event.refType = "tag"),
    ],
    [
      "branch ref name",
      (input: MutableRecord) => (input.event.refName = "main"),
    ],
    [
      "workflow ref",
      (input: MutableRecord) => (input.workflow.ref += "-moved"),
    ],
    [
      "tag evidence",
      (input: MutableRecord) => (input.remoteTag = { unexpected: true }),
    ],
  ])("rejects malformed preflight %s", (_label, mutate) => {
    expectRejected(mutate);
  });

  it.each([
    [
      "event name",
      (input: MutableRecord) => (input.event.name = "workflow_dispatch"),
    ],
    [
      "dispatch source SHA",
      (input: MutableRecord) => (input.event.dispatchSourceSha = SOURCE_SHA),
    ],
    [
      "tag ref",
      (input: MutableRecord) => (input.event.ref = "refs/tags/v0.3.2"),
    ],
    [
      "tag ref type",
      (input: MutableRecord) => (input.event.refType = "branch"),
    ],
    [
      "tag ref name",
      (input: MutableRecord) => (input.event.refName = "v0.3.2"),
    ],
    [
      "workflow ref",
      (input: MutableRecord) =>
        (input.workflow.ref =
          "fy-agent/fyagent/.github/workflows/release.yml@refs/heads/main"),
    ],
    [
      "missing annotated tag",
      (input: MutableRecord) => (input.remoteTag = null),
    ],
    [
      "lightweight tag",
      (input: MutableRecord) => (input.remoteTag.refObject.type = "commit"),
    ],
    [
      "tag object SHA",
      (input: MutableRecord) => (input.remoteTag.tagObject.sha = OTHER_SHA),
    ],
    [
      "tag object name",
      (input: MutableRecord) => (input.remoteTag.tagObject.name = "v0.3.2"),
    ],
    [
      "tag target type",
      (input: MutableRecord) => (input.remoteTag.tagObject.target.type = "tag"),
    ],
    [
      "tag target SHA",
      (input: MutableRecord) =>
        (input.remoteTag.tagObject.target.sha = OTHER_SHA),
    ],
  ])("rejects malformed formal %s evidence", (_label, mutate) => {
    expectRejected(mutate, undefined, "formal");
  });
});

describe("exact authority-branch push CI admission", () => {
  it("selects the newest exact-source run and rerun attempt", () => {
    const input = mutableInput();
    const oldOtherSource = ciRun({
      id: "8999",
      runNumber: "41",
      headSha: OTHER_SHA,
    });
    const rerun = ciRun({ runAttempt: "2" });
    input.ciRuns = [oldOtherSource, ciRun(), rerun];
    bindEvidenceToRun(input, rerun);

    expect(
      evaluateDevReleaseEligibility(input as DevReleaseEligibilityInput),
    ).toMatchObject({
      ciRunId: CI_RUN_ID,
      ciRunAttempt: "2",
    });
  });

  it.each([
    [
      "inactive workflow",
      (input: MutableRecord) => (input.ciWorkflow.state = "disabled_manually"),
    ],
    [
      "wrong workflow repository",
      (input: MutableRecord) =>
        (input.ciWorkflow.repository.nameWithOwner = "fork/fyagent"),
    ],
    [
      "wrong workflow repository id",
      (input: MutableRecord) => (input.ciWorkflow.repository.id = "9"),
    ],
    [
      "wrong workflow id",
      (input: MutableRecord) => (input.ciWorkflow.id = "999"),
    ],
    [
      "wrong workflow name",
      (input: MutableRecord) => (input.ciWorkflow.name = "Other"),
    ],
    [
      "wrong workflow path",
      (input: MutableRecord) =>
        (input.ciWorkflow.path = ".github/workflows/other.yml"),
    ],
    [
      "wrong run repository",
      (input: MutableRecord) =>
        (input.ciRuns[0].repository.nameWithOwner = "fork/fyagent"),
    ],
    [
      "wrong run repository id",
      (input: MutableRecord) => (input.ciRuns[0].repository.id = "9"),
    ],
    [
      "wrong head repository",
      (input: MutableRecord) =>
        (input.ciRuns[0].headRepository.nameWithOwner = "fork/fyagent"),
    ],
    [
      "wrong head repository id",
      (input: MutableRecord) => (input.ciRuns[0].headRepository.id = "9"),
    ],
    [
      "wrong run workflow id",
      (input: MutableRecord) => (input.ciRuns[0].workflow.id = "999"),
    ],
    [
      "wrong run workflow name",
      (input: MutableRecord) => (input.ciRuns[0].workflow.name = "Other"),
    ],
    [
      "wrong run workflow path",
      (input: MutableRecord) =>
        (input.ciRuns[0].workflow.path = ".github/workflows/other.yml"),
    ],
    [
      "wrong run event",
      (input: MutableRecord) => (input.ciRuns[0].event = "workflow_dispatch"),
    ],
    [
      "wrong run branch",
      (input: MutableRecord) => (input.ciRuns[0].headBranch = "main"),
    ],
    [
      "wrong run SHA",
      (input: MutableRecord) => (input.ciRuns[0].headSha = OTHER_SHA),
    ],
    ["missing run", (input: MutableRecord) => (input.ciRuns = [])],
  ])("rejects %s", (_label, mutate) => {
    expectRejected(mutate);
  });

  it("does not accept an old green commit after the dev branch moves", () => {
    expectRejected((input) => {
      input.event.sha = OTHER_SHA;
      input.event.dispatchSourceSha = OTHER_SHA;
      input.workflow.sha = OTHER_SHA;
      input.candidate.sourceSha = OTHER_SHA;
      input.remoteDev.headSha = OTHER_SHA;
    }, /no dev\/laiyongjie push CI run exists/);
  });

  it.each([
    ["failure", "completed", "failure"],
    ["cancellation", "completed", "cancelled"],
    ["timeout", "completed", "timed_out"],
    ["in-progress attempt", "in_progress", null],
  ])(
    "rejects a newer %s after an older green attempt",
    (_label, status, conclusion) => {
      expectRejected((input) => {
        input.ciRuns.push(
          ciRun({
            runAttempt: "2",
            status,
            conclusion,
          }),
        );
      }, /latest exact-source dev\/laiyongjie push CI run\/attempt must be completed successfully/);
    },
  );

  it.each([
    ["run id", (input: MutableRecord) => (input.ciEvidence.runId = "999")],
    ["attempt", (input: MutableRecord) => (input.ciEvidence.runAttempt = "2")],
    [
      "check suite",
      (input: MutableRecord) => (input.ciEvidence.checkSuiteId = "999"),
    ],
    ["missing job", (input: MutableRecord) => (input.ciEvidence.jobs = [])],
    [
      "missing check",
      (input: MutableRecord) => (input.ciEvidence.checkRuns = []),
    ],
    [
      "duplicate job",
      (input: MutableRecord) =>
        input.ciEvidence.jobs.push(structuredClone(input.ciEvidence.jobs[0])),
    ],
    [
      "duplicate check",
      (input: MutableRecord) =>
        input.ciEvidence.checkRuns.push(
          structuredClone(input.ciEvidence.checkRuns[0]),
        ),
    ],
    [
      "job failure",
      (input: MutableRecord) =>
        (input.ciEvidence.jobs[0].conclusion = "failure"),
    ],
    [
      "check failure",
      (input: MutableRecord) =>
        (input.ciEvidence.checkRuns[0].conclusion = "failure"),
    ],
    [
      "wrong app",
      (input: MutableRecord) =>
        (input.ciEvidence.checkRuns[0].appSlug = "external"),
    ],
    [
      "wrong check head",
      (input: MutableRecord) =>
        (input.ciEvidence.checkRuns[0].headSha = OTHER_SHA),
    ],
    [
      "wrong job check URL",
      (input: MutableRecord) =>
        (input.ciEvidence.jobs[0].checkRunUrl += "?other=1"),
    ],
    [
      "wrong check API URL",
      (input: MutableRecord) =>
        (input.ciEvidence.checkRuns[0].url += "?other=1"),
    ],
    [
      "wrong job URL",
      (input: MutableRecord) =>
        (input.ciEvidence.jobs[0].htmlUrl += "?other=1"),
    ],
    [
      "wrong details URL",
      (input: MutableRecord) =>
        (input.ciEvidence.checkRuns[0].detailsUrl += "?other=1"),
    ],
  ])("rejects %s evidence", (_label, mutate) => {
    expectRejected(mutate);
  });

  it("rejects any drift from a previously frozen eligibility output", () => {
    const input = mutableInput();
    const expected = {
      ...evaluateDevReleaseEligibility(input as DevReleaseEligibilityInput),
      ciRunAttempt: "2",
    };
    expect(() =>
      evaluateDevReleaseEligibility(
        input as DevReleaseEligibilityInput,
        expected,
      ),
    ).toThrow(/expectedFrozen\.ciRunAttempt/);
    expect(() =>
      evaluateDevReleaseEligibility(
        input as DevReleaseEligibilityInput,
        {
          ...expected,
          ciRunAttempt: "1",
          unexpected: true,
        } as never,
      ),
    ).toThrow(/expectedFrozen must contain exactly/);
  });
});

describe("fail-closed input shape", () => {
  it.each([
    ["top-level extras", (input: MutableRecord) => (input.unexpected = true)],
    [
      "repository extras",
      (input: MutableRecord) => (input.repository.unexpected = true),
    ],
    ["event extras", (input: MutableRecord) => (input.event.unexpected = true)],
    [
      "workflow extras",
      (input: MutableRecord) => (input.workflow.unexpected = true),
    ],
    [
      "candidate extras",
      (input: MutableRecord) => (input.candidate.unexpected = true),
    ],
    [
      "remote branch extras",
      (input: MutableRecord) => (input.remoteDev.unexpected = true),
    ],
    [
      "CI workflow extras",
      (input: MutableRecord) => (input.ciWorkflow.unexpected = true),
    ],
    [
      "CI run extras",
      (input: MutableRecord) => (input.ciRuns[0].unexpected = true),
    ],
    [
      "CI job extras",
      (input: MutableRecord) => (input.ciEvidence.jobs[0].unexpected = true),
    ],
    [
      "CI check extras",
      (input: MutableRecord) =>
        (input.ciEvidence.checkRuns[0].unexpected = true),
    ],
    [
      "non-canonical run id",
      (input: MutableRecord) => (input.ciRuns[0].id = "09001"),
    ],
    [
      "unknown run status",
      (input: MutableRecord) => (input.ciRuns[0].status = "mystery"),
    ],
    [
      "premature conclusion",
      (input: MutableRecord) => {
        input.ciRuns[0].status = "in_progress";
        input.ciRuns[0].conclusion = "success";
      },
    ],
  ])("rejects %s", (_label, mutate) => {
    expectRejected(mutate);
  });
});
