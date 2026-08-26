import { describe, expect, it } from "vitest";
import {
  DEV_RELEASE_ELIGIBILITY_INPUT_SCHEMA,
  evaluateDevReleaseEligibility,
  type DevReleaseEligibilityInput,
} from "../scripts/release/dev-release-eligibility.mjs";

const SOURCE_SHA = "a".repeat(40);
const OTHER_SHA = "c".repeat(40);
const WORKFLOW_SHA = "d".repeat(40);
const TAG_OBJECT_SHA = "b".repeat(40);
const PRE_TRANSFER_REPOSITORY = ["NongHua123", "fyagent"].join("/");
const REPOSITORY = {
  nameWithOwner: "fy-agent/fyagent",
  id: "1313497021",
} as const;

type MutableRecord = Record<string, any>;

function validInput(
  mode: "preflight" | "formal" = "preflight",
): DevReleaseEligibilityInput {
  const releaseTag = "v0.3.1";
  const authorityBranch = "main";
  const ref =
    mode === "preflight"
      ? `refs/heads/${authorityBranch}`
      : `refs/tags/${releaseTag}`;
  const workflowSha = mode === "preflight" ? WORKFLOW_SHA : SOURCE_SHA;
  return {
    schema: DEV_RELEASE_ELIGIBILITY_INPUT_SCHEMA,
    repository: { ...REPOSITORY },
    event: {
      dispatchMode: mode === "preflight" ? "preflight" : null,
      dispatchSourceSha: mode === "preflight" ? SOURCE_SHA : null,
      name: mode === "preflight" ? "workflow_dispatch" : "push",
      ref,
      refName: mode === "preflight" ? authorityBranch : releaseTag,
      refType: mode === "preflight" ? "branch" : "tag",
      sha: workflowSha,
    },
    workflow: {
      name: "Release",
      path: ".github/workflows/release.yml",
      ref: `fy-agent/fyagent/.github/workflows/release.yml@${ref}`,
      sha: workflowSha,
    },
    candidate: {
      canonicalVersion: "0.3.1",
      releaseTag,
      sourceSha: SOURCE_SHA,
    },
    remoteDev: {
      name: authorityBranch,
      ref: `refs/heads/${authorityBranch}`,
      headSha: workflowSha,
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

describe("split preflight and formal release identity", () => {
  it.each(["preflight", "formal"] as const)(
    "freezes the exact eligible %s identity without Required CI",
    (mode) => {
      const output = evaluateDevReleaseEligibility(validInput(mode));

      expect(output).toEqual({
        appVersion: "0.3.1",
        releaseTag: "v0.3.1",
        sourceSha: SOURCE_SHA,
        workflowSha: mode === "preflight" ? WORKFLOW_SHA : SOURCE_SHA,
        ciRunId: null,
        ciRunAttempt: null,
        mode,
      });
      expect(Object.isFrozen(output)).toBe(true);
      expect(evaluateDevReleaseEligibility(validInput(mode), output)).toEqual(
        output,
      );
    },
  );

  it("accepts workflow_dispatch formal when the selected ref is the exact release tag", () => {
    const input = mutableInput("formal");
    input.event.name = "workflow_dispatch";
    input.event.dispatchMode = "formal";
    expect(
      evaluateDevReleaseEligibility(input as DevReleaseEligibilityInput),
    ).toMatchObject({
      sourceSha: SOURCE_SHA,
      workflowSha: SOURCE_SHA,
      releaseTag: "v0.3.1",
      mode: "formal",
    });
  });

  it("allows preflight candidate SHA to differ from the trusted main workflow SHA", () => {
    expect(
      evaluateDevReleaseEligibility(validInput("preflight")),
    ).toMatchObject({
      sourceSha: SOURCE_SHA,
      workflowSha: WORKFLOW_SHA,
      mode: "preflight",
    });
  });

  it("rejects workflow_dispatch formal when source_sha is supplied instead of trusting the tag ref", () => {
    const input = mutableInput("formal");
    input.event.name = "workflow_dispatch";
    input.event.dispatchMode = "formal";
    input.event.dispatchSourceSha = SOURCE_SHA;
    expect(() =>
      evaluateDevReleaseEligibility(input as DevReleaseEligibilityInput),
    ).toThrow(/event\.dispatchSourceSha/);
  });

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
      "dispatch mode",
      (input: MutableRecord) => (input.event.dispatchMode = "formal"),
    ],
    [
      "candidate SHA",
      (input: MutableRecord) => (input.candidate.sourceSha = OTHER_SHA),
    ],
    [
      "remote authority branch",
      (input: MutableRecord) => (input.remoteDev.name = "other"),
    ],
    [
      "remote authority ref",
      (input: MutableRecord) => (input.remoteDev.ref = "refs/heads/other"),
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
      (input: MutableRecord) => (input.event.ref = "refs/heads/feature/test"),
    ],
    [
      "branch ref type",
      (input: MutableRecord) => (input.event.refType = "tag"),
    ],
    [
      "branch ref name",
      (input: MutableRecord) => (input.event.refName = "feature/test"),
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
      "dispatch mode",
      (input: MutableRecord) => (input.event.dispatchMode = "formal"),
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
    ["missing tag", (input: MutableRecord) => (input.remoteTag = null)],
    [
      "lightweight tag object",
      (input: MutableRecord) => {
        input.remoteTag.refObject.type = "commit";
        input.remoteTag.refObject.sha = SOURCE_SHA;
        input.remoteTag.tagObject = { name: "v0.3.1" };
      },
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

  it("accepts a lightweight formal tag whose commit SHA is the frozen source", () => {
    const input = mutableInput("formal");
    input.remoteTag.refObject = { type: "commit", sha: SOURCE_SHA };
    input.remoteTag.tagObject = null;
    expect(
      evaluateDevReleaseEligibility(input as DevReleaseEligibilityInput),
    ).toMatchObject({
      sourceSha: SOURCE_SHA,
      ciRunId: null,
      mode: "formal",
    });
  });

  it("rejects a lightweight formal tag that points at another commit", () => {
    expectRejected(
      (input) => {
        input.remoteTag.refObject = { type: "commit", sha: OTHER_SHA };
        input.remoteTag.tagObject = null;
      },
      /remoteTag\.refObject\.sha/,
      "formal",
    );
  });

  it("accepts a formal tag after live main has moved past the frozen source", () => {
    const input = mutableInput("formal");
    input.remoteDev.headSha = OTHER_SHA;
    expect(
      evaluateDevReleaseEligibility(input as DevReleaseEligibilityInput),
    ).toMatchObject({
      sourceSha: SOURCE_SHA,
      mode: "formal",
    });
  });

  it("accepts a preflight after live main moves past the trusted workflow commit", () => {
    const input = mutableInput("preflight");
    input.remoteDev.headSha = OTHER_SHA;
    expect(
      evaluateDevReleaseEligibility(input as DevReleaseEligibilityInput),
    ).toMatchObject({
      sourceSha: SOURCE_SHA,
      workflowSha: WORKFLOW_SHA,
      mode: "preflight",
    });
  });
});

describe("frozen output recheck", () => {
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
          ciRunAttempt: null,
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
  ])("rejects %s", (_label, mutate) => {
    expectRejected(mutate);
  });
});
