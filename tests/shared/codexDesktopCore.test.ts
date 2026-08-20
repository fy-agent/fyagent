import fs from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";
import {
  comparePlatformVersions as compareLegacyPlatformVersions,
  displayPlatformVersion as displayLegacyPlatformVersion,
} from "@/types/codexDesktop";
import {
  deriveLocalVersionState as deriveLegacyLocalVersionState,
  deriveRemoteVersionState as deriveLegacyRemoteVersionState,
} from "@/components/codex/versionState";
import {
  comparePlatformVersions,
  createDownloadSpeedState,
  deriveInstallerActionState,
  deriveInstallerViewState,
  deriveLocalVersionState,
  deriveRemoteVersionState,
  displayPlatformVersion,
  parseJobSnapshot,
  projectInstallerProgress,
  shouldAcceptJobSnapshot,
  updateDownloadSpeedState,
  type JobSnapshot,
  type LocalInstallStatus,
  type RemoteReleaseStatus,
} from "@/shared/codex-desktop";

const remote: RemoteReleaseStatus = {
  releaseId: `v1:${"a".repeat(64)}`,
  displayVersion: "26.1.0",
  platformVersion: {
    kind: "windows_msix",
    major: 26,
    minor: 1,
    build: 0,
    revision: 0,
  },
  downloadSizeHint: 8 * 1024 * 1024,
  checkedAt: "2026-08-14T00:00:00.000Z",
};

const notInstalled: LocalInstallStatus = {
  state: "not_installed",
  platform: "windows",
  architecture: "x86_64",
};

function makeJob(
  stage: JobSnapshot["stage"],
  sequence: number,
  overrides: Partial<JobSnapshot> = {},
): JobSnapshot {
  return {
    jobId: "job-1",
    sequence,
    stage,
    release: remote,
    startedAt: "2026-08-14T00:00:00.000Z",
    updatedAt: "2026-08-14T00:00:00.000Z",
    progress: null,
    cancellable: false,
    result: null,
    error: null,
    ...overrides,
  };
}

function makeDownloadJob(
  sequence: number,
  completedBytes: number,
  updatedAt: string,
  overrides: Partial<JobSnapshot> = {},
): JobSnapshot {
  return makeJob("downloading", sequence, {
    updatedAt,
    progress: {
      phase: "download",
      completedBytes,
      totalBytes: 8 * 1024 * 1024,
      percent: 50,
    },
    cancellable: true,
    ...overrides,
  });
}

describe("neutral Codex Desktop core", () => {
  it("keeps legacy imports as compatibility re-exports", () => {
    expect(compareLegacyPlatformVersions).toBe(comparePlatformVersions);
    expect(displayLegacyPlatformVersion).toBe(displayPlatformVersion);
    expect(deriveLegacyLocalVersionState).toBe(deriveLocalVersionState);
    expect(deriveLegacyRemoteVersionState).toBe(deriveRemoteVersionState);
  });

  it("contains no Tauri, React, UI, i18n, toast, or platform imports", () => {
    const root = path.resolve("src/shared/codex-desktop");
    const sources = fs
      .readdirSync(root)
      .filter((file) => file.endsWith(".ts"))
      .map((file) => fs.readFileSync(path.join(root, file), "utf8"))
      .join("\n");

    expect(sources).not.toMatch(
      /@tauri-apps|from ["']react|react-i18next|sonner|@\/(?:components|hooks|lib|v2)\//,
    );
  });

  it("derives the existing install action and refresh-disable rule", () => {
    const localVersion = deriveLocalVersionState(notInstalled, {
      isLoading: false,
      isError: false,
    });
    const remoteVersion = deriveRemoteVersionState(remote, {
      isLoading: false,
      isError: false,
      isRefetching: false,
      isRefetchError: false,
      errorCode: null,
    });
    const state = deriveInstallerViewState(notInstalled, remote, {
      localPending: false,
      remotePending: false,
      localFailed: false,
      remoteFailed: false,
      job: null,
    });

    expect(
      deriveInstallerActionState({
        state,
        local: notInstalled,
        remote,
        localVersion,
        remoteVersion,
        error: null,
        isActing: false,
      }),
    ).toMatchObject({
      canInstall: true,
      primaryAction: "install",
      primaryDisabled: false,
    });

    expect(
      deriveInstallerActionState({
        state,
        local: notInstalled,
        remote,
        localVersion,
        remoteVersion: { kind: "refreshing", version: "26.1.0" },
        error: null,
        isActing: false,
      }).primaryDisabled,
    ).toBe(true);
  });

  it("accepts only monotonic snapshots and orders distinct jobs by start time", () => {
    const current = makeJob("downloading", 3);
    expect(shouldAcceptJobSnapshot(current, makeJob("checking", 3))).toBe(
      false,
    );
    expect(shouldAcceptJobSnapshot(current, makeJob("downloading", 4))).toBe(
      true,
    );
    expect(
      shouldAcceptJobSnapshot(
        current,
        makeJob("checking", 0, {
          jobId: "job-2",
          startedAt: "2026-08-14T00:01:00.000Z",
        }),
      ),
    ).toBe(true);
    expect(
      shouldAcceptJobSnapshot(
        current,
        makeJob("succeeded", 99, {
          jobId: "older-job",
          startedAt: "2026-08-13T23:59:00.000Z",
        }),
      ),
    ).toBe(false);
  });

  it("binds download speed to the job-lifetime average", () => {
    const mebibyte = 1024 * 1024;
    const first = makeDownloadJob(1, mebibyte, "2026-08-14T00:00:01.000Z");
    const second = makeDownloadJob(2, 5 * mebibyte, "2026-08-14T00:00:03.000Z");
    let state = updateDownloadSpeedState(createDownloadSpeedState(), first);
    expect(projectInstallerProgress(first, state)?.bytesPerSecond).toBeNull();

    state = updateDownloadSpeedState(state, second);
    expect(projectInstallerProgress(second, state)).toMatchObject({
      current: 5 * mebibyte,
      total: 8 * mebibyte,
      percent: 50,
      bytesPerSecond: 2 * mebibyte,
    });
    expect(updateDownloadSpeedState(state, second)).toBe(state);

    const otherJob = makeDownloadJob(
      1,
      7 * mebibyte,
      "2026-08-14T00:01:01.000Z",
      { jobId: "job-2", startedAt: "2026-08-14T00:01:00.000Z" },
    );
    state = updateDownloadSpeedState(state, otherJob);
    expect(
      projectInstallerProgress(otherJob, state)?.bytesPerSecond,
    ).toBeNull();
  });

  it("keeps a stable average when later hops slow down", () => {
    const mebibyte = 1024 * 1024;
    let state = updateDownloadSpeedState(
      createDownloadSpeedState(),
      makeDownloadJob(1, mebibyte, "2026-08-14T00:00:01.000Z"),
    );
    state = updateDownloadSpeedState(
      state,
      makeDownloadJob(2, 5 * mebibyte, "2026-08-14T00:00:02.000Z"),
    );
    expect(
      projectInstallerProgress(
        makeDownloadJob(2, 5 * mebibyte, "2026-08-14T00:00:02.000Z"),
        state,
      )?.bytesPerSecond,
    ).toBe(4 * mebibyte);

    const slowed = makeDownloadJob(3, 6 * mebibyte, "2026-08-14T00:00:06.000Z");
    state = updateDownloadSpeedState(state, slowed);
    expect(projectInstallerProgress(slowed, state)?.bytesPerSecond).toBe(
      mebibyte,
    );
  });

  it("accepts progress after actual bytes exceed a remote size hint", () => {
    const job = makeDownloadJob(
      2,
      9 * 1024 * 1024,
      "2026-08-14T00:00:02.000Z",
      {
        progress: {
          phase: "download",
          completedBytes: 9 * 1024 * 1024,
          totalBytes: 8 * 1024 * 1024,
          percent: 100,
        },
      },
    );

    expect(parseJobSnapshot(job).progress).toEqual(job.progress);
  });

  it("clears speed outside download and keeps the last average while stalled", () => {
    const mebibyte = 1024 * 1024;
    let state = updateDownloadSpeedState(
      createDownloadSpeedState(),
      makeDownloadJob(1, mebibyte, "2026-08-14T00:00:01.000Z"),
    );
    state = updateDownloadSpeedState(
      state,
      makeDownloadJob(2, 3 * mebibyte, "2026-08-14T00:00:02.000Z"),
    );
    const stalled = makeDownloadJob(
      3,
      3 * mebibyte,
      "2026-08-14T00:00:03.000Z",
    );
    state = updateDownloadSpeedState(state, stalled);
    expect(projectInstallerProgress(stalled, state)?.bytesPerSecond).toBe(
      2 * mebibyte,
    );

    const recovered = makeDownloadJob(
      4,
      5 * mebibyte,
      "2026-08-14T00:00:04.000Z",
    );
    state = updateDownloadSpeedState(state, recovered);
    expect(projectInstallerProgress(recovered, state)?.bytesPerSecond).toBe(
      (4 * mebibyte) / 3,
    );

    const installing = makeJob("installing", 5, {
      progress: {
        phase: "installation",
        completedBytes: 50,
        totalBytes: 100,
        percent: 50,
      },
    });
    state = updateDownloadSpeedState(state, installing);
    expect(projectInstallerProgress(installing, state)).toEqual({
      current: 50,
      total: 100,
      percent: 50,
      bytesPerSecond: null,
    });
  });
});
