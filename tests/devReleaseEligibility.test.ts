import { execFileSync, execSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import {
  DEV_RELEASE_ELIGIBILITY_INPUT_SCHEMA,
  FORMAL_REPAIR_PATHS,
  FORMAL_REPAIR_SOURCE_SHA,
  assertFormalRepairDifference,
  evaluateDevReleaseEligibility,
  type DevReleaseEligibilityInput,
} from "../scripts/release/dev-release-eligibility.mjs";
import { WORKFLOW_ONLY_PATHS } from "../scripts/release/windows-formal-offline-admission.mjs";

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
        repair: false,
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

const REPAIR_WORKFLOW_SHA = "e".repeat(40);

function productWorkflow(macosRunner: string) {
  return [
    "\n  build-windows:\n    runs-on: windows-2025\n",
    "\n  prove-windows-preflight:\n    runs-on: windows-2025\n",
    "\n  sign-windows-formal:\n    runs-on: windows-2025\n",
    "\n  seal-windows-formal:\n    runs-on: windows-2025\n",
    `\n  build-macos:\n    runs-on: ${macosRunner}\n`,
  ].join("");
}

function repairInput(): MutableRecord {
  return {
    schema: DEV_RELEASE_ELIGIBILITY_INPUT_SCHEMA,
    repository: { ...REPOSITORY },
    event: {
      dispatchMode: "formal-repair",
      dispatchSourceSha: FORMAL_REPAIR_SOURCE_SHA,
      name: "workflow_dispatch",
      ref: "refs/heads/main",
      refName: "main",
      refType: "branch",
      sha: REPAIR_WORKFLOW_SHA,
    },
    workflow: {
      name: "Release",
      path: ".github/workflows/release.yml",
      ref: "fy-agent/fyagent/.github/workflows/release.yml@refs/heads/main",
      sha: REPAIR_WORKFLOW_SHA,
    },
    candidate: {
      canonicalVersion: "0.4.15",
      releaseTag: "v0.4.15",
      sourceSha: FORMAL_REPAIR_SOURCE_SHA,
    },
    remoteDev: {
      name: "main",
      ref: "refs/heads/main",
      headSha: REPAIR_WORKFLOW_SHA,
    },
    remoteTag: {
      ref: "refs/tags/v0.4.15",
      refObject: { type: "commit", sha: FORMAL_REPAIR_SOURCE_SHA },
      tagObject: null,
    },
  };
}

function gitSucceeds(args: string[]) {
  try {
    execFileSync("git", args, {
      stdio: "ignore",
      env: { ...process.env, GIT_TERMINAL_PROMPT: "0" },
    });
    return true;
  } catch {
    return false;
  }
}

function gitFetch(args: string[]) {
  execFileSync(
    "git",
    ["-c", "protocol.version=2", "fetch", "--no-tags", ...args],
    {
      stdio: "pipe",
      env: { ...process.env, GIT_TERMINAL_PROMPT: "0" },
    },
  );
}

function ensureRepairTagHistory(sha: string) {
  if (!gitSucceeds(["cat-file", "-e", `${sha}^{commit}`])) {
    gitFetch(["--depth=1", "origin", sha]);
  }
  if (gitSucceeds(["merge-base", "--is-ancestor", sha, "HEAD"])) return;
  const shallow = execFileSync("git", ["rev-parse", "--is-shallow-repository"], {
    encoding: "utf8",
  }).trim();
  if (shallow !== "true") {
    throw new Error(`${sha} is not an ancestor of HEAD`);
  }
  gitFetch(["--deepen=128"]);
  execFileSync("git", ["merge-base", "--is-ancestor", sha, "HEAD"], {
    stdio: "ignore",
  });
}

describe("v0.4.15 formal repair", () => {
  it("keeps ordinary formal closed when the workflow SHA splits from the tag", () => {
    const input = mutableInput("formal");
    input.event.sha = WORKFLOW_SHA;
    input.workflow.sha = WORKFLOW_SHA;
    expect(() =>
      evaluateDevReleaseEligibility(input as DevReleaseEligibilityInput),
    ).toThrow(/event\.sha must be "a{40}"/);
    expect(WORKFLOW_ONLY_PATHS).not.toContain(
      "scripts/release/dev-release-eligibility.mjs",
    );
    expect(WORKFLOW_ONLY_PATHS).not.toContain(
      "scripts/release/release-contract.mjs",
    );
  });

  it("accepts the narrow main repair and records it separately from formal mode", () => {
    expect(
      evaluateDevReleaseEligibility(repairInput() as DevReleaseEligibilityInput),
    ).toEqual({
      appVersion: "0.4.15",
      releaseTag: "v0.4.15",
      sourceSha: FORMAL_REPAIR_SOURCE_SHA,
      workflowSha: REPAIR_WORKFLOW_SHA,
      ciRunId: null,
      ciRunAttempt: null,
      mode: "formal",
      repair: true,
    });
  });

  it("rejects a repair whose remote tag peels to another commit", () => {
    const input = repairInput();
    input.remoteTag.refObject.sha = OTHER_SHA;
    expect(() =>
      evaluateDevReleaseEligibility(input as DevReleaseEligibilityInput),
    ).toThrow(/remoteTag\.refObject\.sha/);
  });

  it("rejects a repair that is not dispatched from main", () => {
    const input = repairInput();
    input.event.ref = "refs/heads/feature/repair";
    input.event.refName = "feature/repair";
    input.workflow.ref =
      "fy-agent/fyagent/.github/workflows/release.yml@refs/heads/feature/repair";
    expect(() =>
      evaluateDevReleaseEligibility(input as DevReleaseEligibilityInput),
    ).toThrow(/event\.ref/);
  });

  it("rejects a repair that forges the workflow identity as the tag", () => {
    const sameCommit = repairInput();
    sameCommit.event.sha = FORMAL_REPAIR_SOURCE_SHA;
    sameCommit.workflow.sha = FORMAL_REPAIR_SOURCE_SHA;
    expect(() =>
      evaluateDevReleaseEligibility(sameCommit as DevReleaseEligibilityInput),
    ).toThrow(/workflow\.sha must differ from the tag commit/);
    const forgedRef = repairInput();
    forgedRef.workflow.ref =
      "fy-agent/fyagent/.github/workflows/release.yml@refs/tags/v0.4.15";
    expect(() =>
      evaluateDevReleaseEligibility(forgedRef as DevReleaseEligibilityInput),
    ).toThrow(/workflow\.ref/);
  });

  it("rejects a repair diff outside the path table, a forbidden admission manifest, or a non-descendant", () => {
    const diff = FORMAL_REPAIR_PATHS.map((candidate) => `M\t${candidate}`).join(
      "\n",
    );
    const workflows = "  build-windows:\n    runs-on: ubuntu-24.04\n";
    expect(() =>
      assertFormalRepairDifference({
        sourceSha: FORMAL_REPAIR_SOURCE_SHA,
        workflowSha: REPAIR_WORKFLOW_SHA,
        ancestor: true,
        diffStatus: `${diff}\nM\tREADME.md\n`,
        tagWorkflow: workflows,
        executingWorkflow: workflows,
      }),
    ).toThrow(/outside the repair table: README.md/);
    expect(() =>
      assertFormalRepairDifference({
        sourceSha: FORMAL_REPAIR_SOURCE_SHA,
        workflowSha: REPAIR_WORKFLOW_SHA,
        ancestor: true,
        diffStatus: `${diff}\nM\tdocs/release/windows-formal-offline-admission.json\n`,
        tagWorkflow: workflows,
        executingWorkflow: workflows,
      }),
    ).toThrow(/forbidden release input: docs\/release\/windows-formal-offline-admission.json/);
    expect(() =>
      assertFormalRepairDifference({
        sourceSha: FORMAL_REPAIR_SOURCE_SHA,
        workflowSha: REPAIR_WORKFLOW_SHA,
        ancestor: false,
        diffStatus: `${diff}\n`,
        tagWorkflow: workflows,
        executingWorkflow: workflows,
      }),
    ).toThrow(/not a descendant of the tag commit/);
  });

  it("rejects a product-job body change in the executing workflow", () => {
    const diff = `${FORMAL_REPAIR_PATHS.map((candidate) => `M\t${candidate}`).join("\n")}\n`;
    const tagWorkflow = productWorkflow("macos-15");
    expect(() =>
      assertFormalRepairDifference({
        sourceSha: FORMAL_REPAIR_SOURCE_SHA,
        workflowSha: REPAIR_WORKFLOW_SHA,
        ancestor: true,
        diffStatus: diff,
        tagWorkflow,
        executingWorkflow: productWorkflow("macos-14"),
      }),
    ).toThrow(/Product build job build-macos changed/);
  });

  it("accepts exactly the repair path table for the v0.4.15 tag", () => {
    ensureRepairTagHistory(FORMAL_REPAIR_SOURCE_SHA);
    const diffStatus = execSync(
      `git -c core.quotePath=false diff --name-status --find-renames ${FORMAL_REPAIR_SOURCE_SHA}`,
      { encoding: "utf8" },
    );
    execSync(`git merge-base --is-ancestor ${FORMAL_REPAIR_SOURCE_SHA} HEAD`);
    const tagWorkflow = execSync(
      `git show ${FORMAL_REPAIR_SOURCE_SHA}:.github/workflows/release.yml`,
      { encoding: "utf8" },
    );
    const workflowSha = execSync("git rev-parse HEAD", {
      encoding: "utf8",
    }).trim();
    assertFormalRepairDifference({
      sourceSha: FORMAL_REPAIR_SOURCE_SHA,
      workflowSha,
      ancestor: true,
      diffStatus,
      tagWorkflow,
      executingWorkflow: readFileSync(".github/workflows/release.yml", "utf8"),
    });
    expect(workflowSha).not.toBe(FORMAL_REPAIR_SOURCE_SHA);
  });
});
