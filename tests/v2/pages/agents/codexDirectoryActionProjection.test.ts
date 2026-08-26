import { describe, expect, it } from "vitest";

import type { InstallerErrorDto } from "@/shared/codex-desktop";

import {
  projectCodexDirectoryAction,
  type CodexDirectoryActionSource,
} from "@/v2/pages/agents/codexDirectoryActionProjection";

function source(
  overrides: Partial<CodexDirectoryActionSource> = {},
): CodexDirectoryActionSource {
  return {
    primaryAction: null,
    primaryDisabled: true,
    isActing: false,
    state: "checking",
    progress: undefined,
    error: null,
    canCancel: false,
    operationFailed: false,
    ...overrides,
  };
}

const sampleError: InstallerErrorDto = {
  code: "DOWNLOAD_FAILED",
  stage: "downloading",
  messageKey: "codexDesktop.error.network",
  retryable: true,
  suggestedAction: "retry",
  details: {
    endpointKind: null,
    attempt: 1,
    maxAttempts: 3,
    httpStatus: null,
    platformErrorCode: null,
    redactedMessage: null,
    context: {},
  },
};

describe("projectCodexDirectoryAction", () => {
  it("copies percent from the existing view model and never invents one", () => {
    expect(projectCodexDirectoryAction(source()).percent).toBeNull();
    expect(
      projectCodexDirectoryAction(
        source({
          progress: {
            current: 50,
            total: 100,
            percent: null,
            bytesPerSecond: 12,
          },
        }),
      ).percent,
    ).toBeNull();
    expect(
      projectCodexDirectoryAction(
        source({
          progress: {
            current: 512,
            total: 1024,
            percent: 41,
            bytesPerSecond: 8,
          },
        }),
      ).percent,
    ).toBe(41);
    expect(
      projectCodexDirectoryAction(
        source({
          progress: {
            current: 0,
            total: 100,
            percent: 0,
            bytesPerSecond: null,
          },
        }),
      ).percent,
    ).toBe(0);
  });

  it("derives install/update only from the view model primaryAction", () => {
    expect(
      projectCodexDirectoryAction(
        source({
          primaryAction: "install",
          primaryDisabled: false,
          state: "ready_install",
        }),
      ).primaryAction,
    ).toBe("install");
    expect(
      projectCodexDirectoryAction(
        source({
          primaryAction: "update",
          primaryDisabled: false,
          state: "ready_update",
        }),
      ).primaryAction,
    ).toBe("update");
    expect(
      projectCodexDirectoryAction(
        source({
          primaryAction: "launch",
          primaryDisabled: false,
          state: "ready_launch",
        }),
      ).primaryAction,
    ).toBeNull();
    expect(
      projectCodexDirectoryAction(
        source({
          primaryAction: "retry",
          state: "failed",
          operationFailed: true,
        }),
      ),
    ).toMatchObject({
      primaryAction: null,
      canRetry: true,
    });
    expect(
      projectCodexDirectoryAction(
        source({
          primaryAction: "refresh",
          state: "remote_unavailable",
        }),
      ).primaryAction,
    ).toBeNull();
  });

  it("projects busy, cancel, and error from the existing owner", () => {
    expect(
      projectCodexDirectoryAction(
        source({ isActing: true, state: "ready_install" }),
      ).busy,
    ).toBe(true);
    expect(
      projectCodexDirectoryAction(
        source({
          state: "job_downloading",
          progress: {
            current: 10,
            total: 40,
            percent: 25,
            bytesPerSecond: 3,
          },
          canCancel: true,
        }),
      ),
    ).toMatchObject({
      busy: true,
      percent: 25,
      canCancel: true,
      state: "job_downloading",
    });
    expect(
      projectCodexDirectoryAction(
        source({
          state: "failed",
          error: sampleError,
          primaryAction: "retry",
        }),
      ),
    ).toMatchObject({
      busy: false,
      error: sampleError,
      canRetry: true,
      percent: null,
    });
  });

  it("does not enable run when the view model disabled the primary action", () => {
    expect(
      projectCodexDirectoryAction(
        source({
          primaryAction: "install",
          primaryDisabled: true,
          state: "ready_install",
        }),
      ).canRun,
    ).toBe(false);
    expect(
      projectCodexDirectoryAction(
        source({
          primaryAction: "install",
          primaryDisabled: false,
          state: "ready_install",
        }),
      ).canRun,
    ).toBe(true);
  });
});
