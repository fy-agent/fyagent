import { describe, expect, it } from "vitest";
import {
  inspectRecoverableReleaseDraft,
  verifyRecoverableReleaseDraft,
} from "../scripts/release/verify-release-draft-ownership.mjs";

const SOURCE_SHA = "a".repeat(40);
const OTHER_SHA = "b".repeat(40);
const RELEASE_TAG = "v0.4.3";
const RELEASE_ID = 24680;
const RUN_ID = 13579;
const RUN_ATTEMPT = 2;

function releaseFixture() {
  return {
    id: RELEASE_ID,
    draft: true,
    prerelease: false,
    tag_name: RELEASE_TAG,
    name: `FyAgent ${RELEASE_TAG}`,
    target_commitish: SOURCE_SHA,
    created_at: "2026-08-25T01:00:30.000Z",
    published_at: null as string | null,
    body: `Release notes\n\n<!-- fyagent-release-transaction:run=${RUN_ID};attempt=${RUN_ATTEMPT};source=${SOURCE_SHA} -->`,
  };
}

function runAttemptFixture(event: "push" | "workflow_dispatch" = "push") {
  return {
    id: RUN_ID,
    run_attempt: RUN_ATTEMPT,
    name: "Release",
    path: `.github/workflows/release.yml@${RELEASE_TAG}`,
    event,
    head_sha: SOURCE_SHA,
    status: "completed",
    conclusion: "failure",
    repository: {
      id: 1_313_497_021,
      full_name: "fy-agent/fyagent",
    },
    head_repository: {
      id: 1_313_497_021,
      full_name: "fy-agent/fyagent",
    },
  };
}

function jobsFixture() {
  return {
    total_count: 1,
    jobs: [
      {
        run_id: RUN_ID,
        name: "Publish stable GitHub Release",
        workflow_name: "Release",
        head_sha: SOURCE_SHA,
        status: "completed",
        conclusion: "failure",
        steps: [
          {
            name: "Stage, re-download, and publish one stable Release transaction",
            status: "completed",
            conclusion: "failure",
            started_at: "2026-08-25T01:00:00.000Z",
            completed_at: "2026-08-25T01:01:00.000Z",
          },
        ],
      },
    ],
  };
}

describe("release draft ownership recovery", () => {
  it("extracts only the exact failed FyAgent transaction marker", () => {
    expect(
      inspectRecoverableReleaseDraft(releaseFixture(), RELEASE_TAG, SOURCE_SHA),
    ).toEqual({
      releaseId: String(RELEASE_ID),
      runId: String(RUN_ID),
      runAttempt: String(RUN_ATTEMPT),
      sourceSha: SOURCE_SHA,
      createdAt: "2026-08-25T01:00:30.000Z",
    });
  });

  it.each(["push", "workflow_dispatch"] as const)(
    "accepts an owned failed draft created by a formal %s run",
    (event) => {
      expect(
        verifyRecoverableReleaseDraft(
          releaseFixture(),
          runAttemptFixture(event),
          jobsFixture(),
          RELEASE_TAG,
          SOURCE_SHA,
        ),
      ).toMatchObject({
        releaseId: String(RELEASE_ID),
        runId: String(RUN_ID),
        runAttempt: String(RUN_ATTEMPT),
      });
    },
  );

  it.each([
    [
      "published release",
      (release: ReturnType<typeof releaseFixture>) => {
        release.draft = false;
      },
      /must still be a draft/,
    ],
    [
      "different source",
      (release: ReturnType<typeof releaseFixture>) => {
        release.body = release.body.replace(SOURCE_SHA, OTHER_SHA);
      },
      /marker source/,
    ],
    [
      "different target commit",
      (release: ReturnType<typeof releaseFixture>) => {
        release.target_commitish = OTHER_SHA;
      },
      /target_commitish/,
    ],
    [
      "published timestamp",
      (release: ReturnType<typeof releaseFixture>) => {
        release.published_at = "2026-08-25T01:00:45.000Z";
      },
      /published_at must be null/,
    ],
    [
      "duplicate marker",
      (release: ReturnType<typeof releaseFixture>) => {
        release.body += `\n<!-- fyagent-release-transaction:run=${RUN_ID};attempt=${RUN_ATTEMPT};source=${SOURCE_SHA} -->`;
      },
      /exactly one/,
    ],
    [
      "marker not final",
      (release: ReturnType<typeof releaseFixture>) => {
        release.body += "\nforged suffix";
      },
      /final draft body suffix/,
    ],
  ])(
    "rejects a %s before querying Actions provenance",
    (_label, mutate, error) => {
      const release = releaseFixture();
      mutate(release);
      expect(() =>
        inspectRecoverableReleaseDraft(release, RELEASE_TAG, SOURCE_SHA),
      ).toThrow(error);
    },
  );

  it.each([
    [
      "wrong repository",
      (run: ReturnType<typeof runAttemptFixture>) => {
        run.repository.full_name = "other/repository";
      },
      /repository\.full_name/,
    ],
    [
      "wrong workflow",
      (run: ReturnType<typeof runAttemptFixture>) => {
        run.path = ".github/workflows/other.yml@v0.4.3";
      },
      /workflow run attempt\.path/,
    ],
    [
      "successful origin run",
      (run: ReturnType<typeof runAttemptFixture>) => {
        run.conclusion = "success";
      },
      /recoverable failure/,
    ],
    [
      "different origin SHA",
      (run: ReturnType<typeof runAttemptFixture>) => {
        run.head_sha = OTHER_SHA;
      },
      /head_sha/,
    ],
  ])("rejects %s provenance", (_label, mutate, error) => {
    const run = runAttemptFixture();
    mutate(run);
    expect(() =>
      verifyRecoverableReleaseDraft(
        releaseFixture(),
        run,
        jobsFixture(),
        RELEASE_TAG,
        SOURCE_SHA,
      ),
    ).toThrow(error);
  });

  it.each([
    [
      "missing publish job",
      (jobs: ReturnType<typeof jobsFixture>) => {
        jobs.jobs[0].name = "Other job";
      },
      /exactly one FyAgent publish job/,
    ],
    [
      "successful publish job",
      (jobs: ReturnType<typeof jobsFixture>) => {
        jobs.jobs[0].conclusion = "success";
      },
      /publish job conclusion/,
    ],
    [
      "successful transaction step",
      (jobs: ReturnType<typeof jobsFixture>) => {
        jobs.jobs[0].steps[0].conclusion = "success";
      },
      /transaction step did not end/,
    ],
    [
      "incomplete job page",
      (jobs: ReturnType<typeof jobsFixture>) => {
        jobs.total_count = 2;
      },
      /complete bounded attempt job set/,
    ],
    [
      "draft created outside transaction step",
      (jobs: ReturnType<typeof jobsFixture>) => {
        jobs.jobs[0].steps[0].started_at = "2026-08-25T01:00:31.000Z";
      },
      /draft creation time/,
    ],
  ])("fails closed for %s", (_label, mutate, error) => {
    const jobs = jobsFixture();
    mutate(jobs);
    expect(() =>
      verifyRecoverableReleaseDraft(
        releaseFixture(),
        runAttemptFixture(),
        jobs,
        RELEASE_TAG,
        SOURCE_SHA,
      ),
    ).toThrow(error);
  });
});
