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
const WORKFLOW_SHA = "d".repeat(40);
const PRE_TRANSFER_REPOSITORY = ["NongHua123", "fyagent"].join("/");
const REPOSITORY = {
  id: 1_313_497_021,
  full_name: "fy-agent/fyagent",
};
const TOKEN = "never-print-this-token";

type Mode = "preflight" | "formal";
type FixtureOptions = {
  branchSha?: string;
  failPath?: string;
  mode?: Mode;
  repository?: typeof REPOSITORY;
  tagType?: "commit" | "tag";
  tagTargetSha?: string;
};

function context(
  mode: Mode = "preflight",
  eventName: "push" | "workflow_dispatch" = mode === "preflight"
    ? "workflow_dispatch"
    : "push",
): DevReleaseRemoteContext {
  const releaseTag = "v0.3.1";
  const authorityBranch = "main";
  const ref =
    mode === "preflight"
      ? `refs/heads/${authorityBranch}`
      : `refs/tags/${releaseTag}`;
  const workflowSha = mode === "preflight" ? WORKFLOW_SHA : SOURCE_SHA;
  return {
    token: TOKEN,
    apiBase: "https://api.github.com",
    repository: "fy-agent/fyagent",
    repositoryId: "1313497021",
    eventName,
    ref,
    refName: mode === "preflight" ? authorityBranch : releaseTag,
    refType: mode === "preflight" ? "branch" : "tag",
    eventSha: workflowSha,
    workflowName: "Release",
    workflowRef: `fy-agent/fyagent/.github/workflows/release.yml@${ref}`,
    workflowSha,
    appVersion: "0.3.1",
    releaseTag,
    sourceSha: SOURCE_SHA,
    dispatchMode: eventName === "workflow_dispatch" ? mode : null,
    dispatchSourceSha:
      eventName === "workflow_dispatch" && mode === "preflight"
        ? SOURCE_SHA
        : null,
  };
}

function jsonResponse(
  body: unknown,
  { status = 200 }: { status?: number } = {},
) {
  const headers = new Headers({ "content-type": "application/json" });
  return new Response(JSON.stringify(body), { headers, status });
}

function fixtureFetch(options: FixtureOptions = {}) {
  const mode = options.mode ?? "preflight";
  const authorityBranch = "main";
  const requests: Array<{ authorization: string | null; url: URL }> = [];
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
    if (path === `/repos/fy-agent/fyagent/git/ref/heads/${authorityBranch}`) {
      return jsonResponse({
        ref: `refs/heads/${authorityBranch}`,
        object: {
          type: "commit",
          sha:
            options.branchSha ??
            (mode === "preflight" ? WORKFLOW_SHA : SOURCE_SHA),
        },
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
    throw new Error(`Unexpected fixture request: ${url.toString()}`);
  };
  return { fetchImpl, requests };
}

describe("dev release remote evidence", () => {
  it("collects repository and branch evidence without Required CI", async () => {
    const fixture = fixtureFetch();
    const { evidence, result } = await verifyDevReleaseRemote(context(), {
      fetchImpl: fixture.fetchImpl,
    });

    expect(result).toEqual({
      appVersion: "0.3.1",
      releaseTag: "v0.3.1",
      sourceSha: SOURCE_SHA,
      workflowSha: WORKFLOW_SHA,
      ciRunId: null,
      ciRunAttempt: null,
      mode: "preflight",
    });
    expect(evidence).not.toHaveProperty("ciRuns");
    expect(evidence).not.toHaveProperty("ciEvidence");
    expect(
      fixture.requests.every(
        ({ authorization }) => authorization === `Bearer ${TOKEN}`,
      ),
    ).toBe(true);
    expect(
      fixture.requests.some(({ url }) =>
        decodeURIComponent(url.pathname).includes("/actions/"),
      ),
    ).toBe(false);
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

  it("binds workflow_dispatch formal to the same remote tag identity", async () => {
    const fixture = fixtureFetch({ mode: "formal" });
    const { evidence, result } = await verifyDevReleaseRemote(
      context("formal", "workflow_dispatch"),
      { fetchImpl: fixture.fetchImpl },
    );

    expect(result).toMatchObject({
      sourceSha: SOURCE_SHA,
      workflowSha: SOURCE_SHA,
      releaseTag: "v0.3.1",
      mode: "formal",
    });
    expect(evidence.event).toMatchObject({
      name: "workflow_dispatch",
      dispatchMode: "formal",
      dispatchSourceSha: null,
      ref: "refs/tags/v0.3.1",
      refType: "tag",
    });
    expect(evidence.remoteTag).not.toBeNull();
  });

  it("accepts a lightweight formal tag without reading an annotated tag object", async () => {
    const fixture = fixtureFetch({ mode: "formal", tagType: "commit" });
    const { evidence, result } = await verifyDevReleaseRemote(
      context("formal"),
      { fetchImpl: fixture.fetchImpl },
    );

    expect(result.mode).toBe("formal");
    expect(evidence.remoteTag).toEqual({
      ref: "refs/tags/v0.3.1",
      refObject: { type: "commit", sha: SOURCE_SHA },
      tagObject: null,
    });
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
      workflowSha: WORKFLOW_SHA,
      ciRunId: null,
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

  it("accepts preflight when main moves after the trusted workflow commit was frozen", async () => {
    const fixture = fixtureFetch({ branchSha: OTHER_SHA });
    const { result } = await verifyDevReleaseRemote(context(), {
      fetchImpl: fixture.fetchImpl,
    });
    expect(result).toMatchObject({
      sourceSha: SOURCE_SHA,
      workflowSha: WORKFLOW_SHA,
      mode: "preflight",
    });
  });

  it("accepts a formal tag after live main has moved", async () => {
    const fixture = fixtureFetch({ mode: "formal", branchSha: OTHER_SHA });
    const { result } = await verifyDevReleaseRemote(context("formal"), {
      fetchImpl: fixture.fetchImpl,
    });
    expect(result).toMatchObject({
      sourceSha: SOURCE_SHA,
      mode: "formal",
      ciRunId: null,
    });
  });

  it("rejects a wrong repository identity", async () => {
    const fixture = fixtureFetch({
      repository: {
        id: REPOSITORY.id,
        full_name: PRE_TRANSFER_REPOSITORY,
      },
    });

    await expect(
      collectDevReleaseRemoteEvidence(context(), {
        fetchImpl: fixture.fetchImpl,
      }),
    ).rejects.toThrow(/must be "fy-agent\/fyagent"/);
  });

  it("reports an HTTP failure without exposing the token or body", async () => {
    const fixture = fixtureFetch({ failPath: "/repos/fy-agent/fyagent" });
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
});

describe("remote verifier environment", () => {
  it("builds the workflow context without persisting its token", () => {
    const ctx = contextFromEnvironment({
      GITHUB_TOKEN: TOKEN,
      GITHUB_API_URL: "https://api.github.com",
      GITHUB_REPOSITORY: "fy-agent/fyagent",
      GITHUB_REPOSITORY_ID: "1313497021",
      GITHUB_EVENT_NAME: "workflow_dispatch",
      GITHUB_REF: "refs/heads/main",
      GITHUB_REF_NAME: "main",
      GITHUB_REF_TYPE: "branch",
      GITHUB_SHA: WORKFLOW_SHA,
      GITHUB_WORKFLOW: "Release",
      GITHUB_WORKFLOW_REF:
        "fy-agent/fyagent/.github/workflows/release.yml@refs/heads/main",
      GITHUB_WORKFLOW_SHA: WORKFLOW_SHA,
      RELEASE_APP_VERSION: "0.3.1",
      RELEASE_TAG: "v0.3.1",
      RELEASE_SOURCE_SHA: SOURCE_SHA,
      RELEASE_DISPATCH_MODE: "preflight",
      RELEASE_DISPATCH_SOURCE_SHA: SOURCE_SHA,
    });

    expect(ctx).toMatchObject({
      token: TOKEN,
      sourceSha: SOURCE_SHA,
      dispatchMode: "preflight",
      dispatchSourceSha: SOURCE_SHA,
    });
    expect(JSON.stringify({ ...ctx, token: undefined })).not.toContain(TOKEN);
  });

  it("normalizes an empty formal dispatch input to null evidence", async () => {
    const formal = context("formal", "workflow_dispatch");
    const fixture = fixtureFetch({ mode: "formal" });
    const { evidence } = await verifyDevReleaseRemote(
      { ...formal, dispatchSourceSha: "" },
      { fetchImpl: fixture.fetchImpl },
    );

    expect(evidence.event.dispatchMode).toBe("formal");
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
