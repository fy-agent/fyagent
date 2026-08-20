import { describe, expect, it } from "vitest";
import {
  collectDevReleaseRemoteEvidence,
  contextFromEnvironment,
  verifyDevReleaseRemote,
  type DevReleaseRemoteContext,
} from "../scripts/release/verify-dev-release-remote.mjs";

const SOURCE_SHA = "a".repeat(40);
const OTHER_SHA = "b".repeat(40);
const TAG_OBJECT_SHA = "c".repeat(40);
const PRE_TRANSFER_REPOSITORY = ["NongHua123", "fyagent"].join("/");
const REPOSITORY = {
  id: 1_313_497_021,
  full_name: "fy-agent/fyagent",
};
const WORKFLOW_ID = 314_159;
const OLD_RUN_ID = 9_000;
const RUN_ID = 9_001;
const RUN_ATTEMPT = 2;
const CHECK_SUITE_ID = 6_001;
const CONTRACTS_JOB_ID = 7_000;
const REQUIRED_JOB_ID = 7_001;
const CONTRACTS_CHECK_ID = 8_000;
const REQUIRED_CHECK_ID = 8_001;
const TOKEN = "never-print-this-token";

type Mode = "preflight" | "formal";
type FixtureOptions = {
  branchSha?: string;
  failPath?: string;
  headRepository?: typeof REPOSITORY;
  incompleteRuns?: boolean;
  mode?: Mode;
  repository?: typeof REPOSITORY;
  rerunDuringCollection?: boolean;
  runSha?: string;
  tagType?: "commit" | "tag";
  tagTargetSha?: string;
};

function context(mode: Mode = "preflight"): DevReleaseRemoteContext {
  const releaseTag = "v0.3.1";
  const authorityBranch =
    mode === "preflight" ? "dev/laiyongjie" : "main";
  const ref =
    mode === "preflight"
      ? `refs/heads/${authorityBranch}`
      : `refs/tags/${releaseTag}`;
  return {
    token: TOKEN,
    apiBase: "https://api.github.com",
    repository: "fy-agent/fyagent",
    repositoryId: "1313497021",
    eventName: mode === "preflight" ? "workflow_dispatch" : "push",
    ref,
    refName: mode === "preflight" ? authorityBranch : releaseTag,
    refType: mode === "preflight" ? "branch" : "tag",
    eventSha: SOURCE_SHA,
    workflowName: "Release",
    workflowRef: `fy-agent/fyagent/.github/workflows/release.yml@${ref}`,
    workflowSha: SOURCE_SHA,
    appVersion: "0.3.1",
    releaseTag,
    sourceSha: SOURCE_SHA,
    dispatchSourceSha: mode === "preflight" ? SOURCE_SHA : null,
  };
}

function rawRun({
  id = RUN_ID,
  runNumber = 42,
  runAttempt = RUN_ATTEMPT,
  checkSuiteId = CHECK_SUITE_ID,
  headBranch = "dev/laiyongjie",
  headSha = SOURCE_SHA,
  repository = REPOSITORY,
  headRepository = REPOSITORY,
}: {
  id?: number;
  runNumber?: number;
  runAttempt?: number;
  checkSuiteId?: number;
  headBranch?: string;
  headSha?: string;
  repository?: typeof REPOSITORY;
  headRepository?: typeof REPOSITORY;
} = {}) {
  return {
    id,
    run_number: runNumber,
    run_attempt: runAttempt,
    check_suite_id: checkSuiteId,
    repository,
    head_repository: headRepository,
    workflow_id: WORKFLOW_ID,
    name: "CI",
    path: ".github/workflows/ci.yml",
    event: "push",
    head_branch: headBranch,
    head_sha: headSha,
    status: "completed",
    conclusion: "success",
  };
}

function rawJob({
  id,
  name,
  checkId,
}: {
  id: number;
  name: string;
  checkId: number;
}) {
  return {
    id,
    name,
    run_id: RUN_ID,
    run_attempt: RUN_ATTEMPT,
    status: "completed",
    conclusion: "success",
    check_run_url: `https://api.github.com/repos/fy-agent/fyagent/check-runs/${checkId}`,
    html_url: `https://github.com/fy-agent/fyagent/actions/runs/${RUN_ID}/job/${id}`,
  };
}

function rawCheck({
  id,
  name,
  jobId,
}: {
  id: number;
  name: string;
  jobId: number;
}) {
  return {
    id,
    name,
    check_suite: { id: CHECK_SUITE_ID },
    app: { slug: "github-actions" },
    head_sha: SOURCE_SHA,
    status: "completed",
    conclusion: "success",
    url: `https://api.github.com/repos/fy-agent/fyagent/check-runs/${id}`,
    details_url: `https://github.com/fy-agent/fyagent/actions/runs/${RUN_ID}/job/${jobId}`,
  };
}

function jsonResponse(
  body: unknown,
  { link, status = 200 }: { link?: string; status?: number } = {},
) {
  const headers = new Headers({ "content-type": "application/json" });
  if (link !== undefined) headers.set("link", link);
  return new Response(JSON.stringify(body), { headers, status });
}

function fixtureFetch(options: FixtureOptions = {}) {
  const mode = options.mode ?? "preflight";
  const authorityBranch = mode === "preflight" ? "dev/laiyongjie" : "main";
  const requests: Array<{ authorization: string | null; url: URL }> = [];
  let runListSnapshots = 0;
  const fetchImpl: typeof fetch = async (input, init) => {
    const url = new URL(String(input));
    requests.push({
      authorization: new Headers(init?.headers).get("authorization"),
      url,
    });
    const path = decodeURIComponent(url.pathname);
    if (options.failPath !== undefined && path.includes(options.failPath)) {
      return jsonResponse({ message: TOKEN }, { status: 503 });
    }

    if (path === "/repos/fy-agent/fyagent") {
      return jsonResponse(options.repository ?? REPOSITORY);
    }
    if (
      path ===
      `/repos/fy-agent/fyagent/git/ref/heads/${authorityBranch}`
    ) {
      return jsonResponse({
        ref: `refs/heads/${authorityBranch}`,
        object: { type: "commit", sha: options.branchSha ?? SOURCE_SHA },
      });
    }
    if (path === "/repos/fy-agent/fyagent/git/ref/tags/v0.3.1") {
      if (mode !== "formal") throw new Error("preflight fetched a tag");
      return jsonResponse({
        ref: "refs/tags/v0.3.1",
        object: {
          type: options.tagType ?? "tag",
          sha: options.tagType === "commit" ? SOURCE_SHA : TAG_OBJECT_SHA,
        },
      });
    }
    if (path === `/repos/fy-agent/fyagent/git/tags/${TAG_OBJECT_SHA}`) {
      return jsonResponse({
        sha: TAG_OBJECT_SHA,
        tag: "v0.3.1",
        object: {
          type: "commit",
          sha: options.tagTargetSha ?? SOURCE_SHA,
        },
      });
    }
    if (path === "/repos/fy-agent/fyagent/actions/workflows/ci.yml") {
      return jsonResponse({
        id: WORKFLOW_ID,
        name: "CI",
        path: ".github/workflows/ci.yml",
        state: "active",
      });
    }
    if (
      path === `/repos/fy-agent/fyagent/actions/workflows/${WORKFLOW_ID}/runs`
    ) {
      expect(url.searchParams.get("branch")).toBe(authorityBranch);
      expect(url.searchParams.get("event")).toBe("push");
      expect(url.searchParams.get("head_sha")).toBe(SOURCE_SHA);
      if (url.searchParams.get("page") === "2") {
        return jsonResponse({
          total_count: 2,
          workflow_runs: [
            rawRun({
              headBranch: authorityBranch,
              headSha: options.runSha ?? SOURCE_SHA,
              headRepository: options.headRepository ?? REPOSITORY,
            }),
          ],
        });
      }
      runListSnapshots += 1;
      if (options.rerunDuringCollection && runListSnapshots > 1) {
        return jsonResponse({
          total_count: 1,
          workflow_runs: [
            rawRun({
              headBranch: authorityBranch,
              runAttempt: RUN_ATTEMPT + 1,
            }),
          ],
        });
      }
      const nextUrl = new URL(
        "https://api.github.com/repos/fy-agent/fyagent/actions/workflows/314159/runs",
      );
      nextUrl.searchParams.set("branch", authorityBranch);
      nextUrl.searchParams.set("event", "push");
      nextUrl.searchParams.set("head_sha", SOURCE_SHA);
      nextUrl.searchParams.set("per_page", "100");
      nextUrl.searchParams.set("page", "2");
      const next = `<${nextUrl.href}>; rel="next"`;
      return jsonResponse(
        {
          total_count: 2,
          workflow_runs: [
            rawRun({
              headBranch: authorityBranch,
              id: OLD_RUN_ID,
              runNumber: 41,
              runAttempt: 1,
              checkSuiteId: 6_000,
              headSha: options.runSha ?? SOURCE_SHA,
              repository: options.repository ?? REPOSITORY,
              headRepository: options.headRepository ?? REPOSITORY,
            }),
          ],
        },
        options.incompleteRuns ? undefined : { link: next },
      );
    }
    if (
      path ===
      `/repos/fy-agent/fyagent/actions/runs/${RUN_ID}/attempts/${RUN_ATTEMPT}/jobs`
    ) {
      if (url.searchParams.get("page") === "2") {
        return jsonResponse({
          total_count: 2,
          jobs: [
            rawJob({
              id: REQUIRED_JOB_ID,
              name: "CI / Required",
              checkId: REQUIRED_CHECK_ID,
            }),
          ],
        });
      }
      return jsonResponse(
        {
          total_count: 2,
          jobs: [
            rawJob({
              id: CONTRACTS_JOB_ID,
              name: "Contracts",
              checkId: CONTRACTS_CHECK_ID,
            }),
          ],
        },
        {
          link: `<https://api.github.com${url.pathname}?per_page=100&page=2>; rel="next"`,
        },
      );
    }
    if (
      path ===
      `/repos/fy-agent/fyagent/check-suites/${CHECK_SUITE_ID}/check-runs`
    ) {
      return jsonResponse({
        total_count: 3,
        check_runs: [
          {
            ...rawCheck({ id: 7_999, name: "Old attempt", jobId: 6_999 }),
            url: "https://api.github.com/repos/fy-agent/fyagent/check-runs/7999",
          },
          rawCheck({
            id: CONTRACTS_CHECK_ID,
            name: "Contracts",
            jobId: CONTRACTS_JOB_ID,
          }),
          rawCheck({
            id: REQUIRED_CHECK_ID,
            name: "CI / Required",
            jobId: REQUIRED_JOB_ID,
          }),
        ],
      });
    }
    throw new Error(`Unexpected fixture request: ${url.toString()}`);
  };
  return { fetchImpl, requests };
}

describe("dev release remote evidence", () => {
  it("follows complete pagination and binds the latest CI attempt", async () => {
    const fixture = fixtureFetch();
    const { evidence, result } = await verifyDevReleaseRemote(context(), {
      fetchImpl: fixture.fetchImpl,
    });

    expect(result).toEqual({
      appVersion: "0.3.1",
      releaseTag: "v0.3.1",
      sourceSha: SOURCE_SHA,
      workflowSha: SOURCE_SHA,
      ciRunId: String(RUN_ID),
      ciRunAttempt: String(RUN_ATTEMPT),
      mode: "preflight",
    });
    expect(evidence.ciRuns.map((run) => run.id)).toEqual([
      String(OLD_RUN_ID),
      String(RUN_ID),
    ]);
    expect(evidence.ciEvidence.jobs.map((job) => job.name)).toEqual([
      "Contracts",
      "CI / Required",
    ]);
    expect(evidence.ciEvidence.checkRuns.map((check) => check.name)).toEqual([
      "Contracts",
      "CI / Required",
    ]);
    expect(
      fixture.requests.every(
        ({ authorization }) => authorization === `Bearer ${TOKEN}`,
      ),
    ).toBe(true);
    const secondPages = fixture.requests.filter(
      ({ url }) => url.searchParams.get("page") === "2",
    );
    expect(
      secondPages.filter(({ url }) =>
        decodeURIComponent(url.pathname).endsWith(
          `/workflows/${WORKFLOW_ID}/runs`,
        ),
      ),
    ).toHaveLength(2);
    expect(
      secondPages.filter(({ url }) =>
        decodeURIComponent(url.pathname).endsWith(
          `/runs/${RUN_ID}/attempts/${RUN_ATTEMPT}/jobs`,
        ),
      ),
    ).toHaveLength(1);
    expect(JSON.stringify(evidence)).not.toContain(TOKEN);
  });

  it("dereferences an annotated formal tag to its commit", async () => {
    const fixture = fixtureFetch({ mode: "formal" });
    const { evidence, result } = await verifyDevReleaseRemote(
      context("formal"),
      { fetchImpl: fixture.fetchImpl },
    );

    expect(result.mode).toBe("formal");
    expect(evidence.remoteTag).toEqual({
      ref: "refs/tags/v0.3.1",
      refObject: { type: "tag", sha: TAG_OBJECT_SHA },
      tagObject: {
        sha: TAG_OBJECT_SHA,
        name: "v0.3.1",
        target: { type: "commit", sha: SOURCE_SHA },
      },
    });
  });

  it("rejects a lightweight formal tag before collecting CI evidence", async () => {
    const fixture = fixtureFetch({ mode: "formal", tagType: "commit" });

    await expect(
      verifyDevReleaseRemote(context("formal"), {
        fetchImpl: fixture.fetchImpl,
      }),
    ).rejects.toThrow(/release tag ref object type must be "tag"/);
    expect(
      fixture.requests.some(({ url }) =>
        decodeURIComponent(url.pathname).includes("/git/tags/"),
      ),
    ).toBe(false);
  });

  it("rejects an annotated tag that targets another commit", async () => {
    const fixture = fixtureFetch({ mode: "formal", tagTargetSha: OTHER_SHA });

    await expect(
      verifyDevReleaseRemote(context("formal"), {
        fetchImpl: fixture.fetchImpl,
      }),
    ).rejects.toThrow(/remoteTag\.tagObject\.target\.sha/);
  });

  it("rejects a frozen result mismatch on a live recheck", async () => {
    const fixture = fixtureFetch();
    const expectedFrozen = {
      appVersion: "0.3.1",
      releaseTag: "v0.3.1",
      sourceSha: SOURCE_SHA,
      workflowSha: SOURCE_SHA,
      ciRunId: String(RUN_ID),
      ciRunAttempt: "1",
      mode: "preflight" as const,
    };

    await expect(
      verifyDevReleaseRemote(context(), {
        expectedFrozen,
        fetchImpl: fixture.fetchImpl,
      }),
    ).rejects.toThrow(/expectedFrozen\.ciRunAttempt/);
  });

  it("rejects a rerun that starts while attempt evidence is collected", async () => {
    const fixture = fixtureFetch({ rerunDuringCollection: true });

    await expect(
      verifyDevReleaseRemote(context(), { fetchImpl: fixture.fetchImpl }),
    ).rejects.toThrow(/ciEvidence\.runAttempt/);
  });

  it("rejects when the dev branch moves", async () => {
    const fixture = fixtureFetch({ branchSha: OTHER_SHA });

    await expect(
      verifyDevReleaseRemote(context(), { fetchImpl: fixture.fetchImpl }),
    ).rejects.toThrow(/remoteDev\.headSha/);
  });

  it("rejects a CI response whose run SHA ignores the exact filter", async () => {
    const fixture = fixtureFetch({ runSha: OTHER_SHA });

    await expect(
      verifyDevReleaseRemote(context(), { fetchImpl: fixture.fetchImpl }),
    ).rejects.toThrow(/workflow runs\[0\]\.head_sha/);
  });

  it.each([
    [
      "repository",
      {
        repository: {
          id: REPOSITORY.id,
          full_name: PRE_TRANSFER_REPOSITORY,
        },
      },
    ],
    [
      "head repository",
      { headRepository: { id: 99, full_name: "fork/fyagent" } },
    ],
  ])("rejects a wrong %s identity", async (_label, options) => {
    const fixture = fixtureFetch(options);

    await expect(
      collectDevReleaseRemoteEvidence(context(), {
        fetchImpl: fixture.fetchImpl,
      }),
    ).rejects.toThrow(/must be "fy-agent\/fyagent"/);
  });

  it("hard fails an incomplete paginated response", async () => {
    const fixture = fixtureFetch({ incompleteRuns: true });

    await expect(
      collectDevReleaseRemoteEvidence(context(), {
        fetchImpl: fixture.fetchImpl,
      }),
    ).rejects.toThrow(/pagination is incomplete/);
  });

  it("reports an HTTP failure without exposing the token or body", async () => {
    const fixture = fixtureFetch({ failPath: "/actions/workflows/" });
    let message = "";
    try {
      await collectDevReleaseRemoteEvidence(context(), {
        fetchImpl: fixture.fetchImpl,
      });
    } catch (error) {
      message = error instanceof Error ? error.message : String(error);
    }

    expect(message).toMatch(/returned HTTP 503/);
    expect(message).not.toContain(TOKEN);
  });

  it("redacts a token repeated by the transport failure", async () => {
    const fetchImpl: typeof fetch = async () => {
      throw new Error(`transport rejected Bearer ${TOKEN}`);
    };
    let message = "";
    try {
      await collectDevReleaseRemoteEvidence(context(), { fetchImpl });
    } catch (error) {
      message = error instanceof Error ? error.message : String(error);
    }

    expect(message).toContain("[REDACTED]");
    expect(message).not.toContain(TOKEN);
  });

  it("rejects a pagination link that changes repository", async () => {
    const base = fixtureFetch();
    const fetchImpl: typeof fetch = async (input, init) => {
      const response = await base.fetchImpl(input, init);
      const url = new URL(String(input));
      if (
        decodeURIComponent(url.pathname).endsWith(
          `/workflows/${WORKFLOW_ID}/runs`,
        )
      ) {
        const headers = new Headers(response.headers);
        headers.set(
          "link",
          '<https://api.github.com/repos/other/project/actions/workflows/314159/runs?page=2>; rel="next"',
        );
        return new Response(await response.text(), {
          headers,
          status: response.status,
        });
      }
      return response;
    };

    await expect(
      collectDevReleaseRemoteEvidence(context(), { fetchImpl }),
    ).rejects.toThrow(/pagination next link changed repository/);
  });

  it("rejects a pagination link that changes resource", async () => {
    const base = fixtureFetch();
    const fetchImpl: typeof fetch = async (input, init) => {
      const response = await base.fetchImpl(input, init);
      const url = new URL(String(input));
      if (
        decodeURIComponent(url.pathname).endsWith(
          `/workflows/${WORKFLOW_ID}/runs`,
        )
      ) {
        const headers = new Headers(response.headers);
        headers.set(
          "link",
          '<https://api.github.com/repos/fy-agent/fyagent/actions/runs?page=2>; rel="next"',
        );
        return new Response(await response.text(), {
          headers,
          status: response.status,
        });
      }
      return response;
    };

    await expect(
      collectDevReleaseRemoteEvidence(context(), { fetchImpl }),
    ).rejects.toThrow(/pagination next link changed resource/);
  });
});

describe("remote verifier environment", () => {
  it("builds the workflow context without persisting its token", () => {
    const ctx = contextFromEnvironment({
      GITHUB_TOKEN: TOKEN,
      GITHUB_API_URL: "https://api.github.com",
      GITHUB_REPOSITORY: "fy-agent/fyagent",
      GITHUB_REPOSITORY_ID: "1313497021",
      GITHUB_EVENT_NAME: "workflow_dispatch",
      GITHUB_REF: "refs/heads/dev/laiyongjie",
      GITHUB_REF_NAME: "dev/laiyongjie",
      GITHUB_REF_TYPE: "branch",
      GITHUB_SHA: SOURCE_SHA,
      GITHUB_WORKFLOW: "Release",
      GITHUB_WORKFLOW_REF:
        "fy-agent/fyagent/.github/workflows/release.yml@refs/heads/dev/laiyongjie",
      GITHUB_WORKFLOW_SHA: SOURCE_SHA,
      RELEASE_APP_VERSION: "0.3.1",
      RELEASE_TAG: "v0.3.1",
      RELEASE_SOURCE_SHA: SOURCE_SHA,
      RELEASE_DISPATCH_SOURCE_SHA: SOURCE_SHA,
    });

    expect(ctx).toMatchObject({
      token: TOKEN,
      sourceSha: SOURCE_SHA,
      dispatchSourceSha: SOURCE_SHA,
    });
    expect(JSON.stringify({ ...ctx, token: undefined })).not.toContain(TOKEN);
  });

  it("normalizes an empty formal dispatch input to null evidence", async () => {
    const formal = context("formal");
    const fixture = fixtureFetch({ mode: "formal" });
    const { evidence } = await verifyDevReleaseRemote(
      { ...formal, dispatchSourceSha: "" },
      { fetchImpl: fixture.fetchImpl },
    );

    expect(evidence.event.dispatchSourceSha).toBeNull();
  });

  it("rejects ambiguous token sources and non-canonical API origins", () => {
    expect(() =>
      contextFromEnvironment({
        GITHUB_TOKEN: "one",
        GH_TOKEN: "two",
        GITHUB_API_URL: "https://api.github.com",
      }),
    ).toThrow(/GITHUB_TOKEN and GH_TOKEN disagree/);
    expect(() =>
      contextFromEnvironment({
        GITHUB_TOKEN: TOKEN,
        GITHUB_API_URL: "https://example.com",
      }),
    ).toThrow(/GITHUB_API_URL/);
  });
});
