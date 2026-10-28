import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

import {
  assertExpectedReleaseId,
  assertInstallConfirmationId,
  assertStagingCapabilityId,
  parseCodexInstallPreflight,
  parseCodexStagingSelection,
  parseJobSnapshot,
  parseLocalInstallStatus,
  parseOptionalJobSnapshot,
  parseRemoteReleaseStatus,
} from "@/domain/codex-desktop";

import type { FeaturePorts } from "../../../features/ports";

const JOB_UPDATED_EVENT = "codex-desktop-installer://job-updated";

function assertJobId(jobId: string): string {
  if (jobId.trim().length === 0 || jobId.trim() !== jobId)
    throw new Error("Codex desktop installer request is invalid");
  return jobId;
}

export function createCodexDesktopPort(): FeaturePorts["codexDesktop"] {
  return {
    getLocalStatus: async () =>
      parseLocalInstallStatus(
        await invoke<unknown>("codex_desktop_get_local_status"),
      ),
    checkLatest: async (force) => {
      if (typeof force !== "boolean")
        throw new Error("Codex desktop installer request is invalid");
      return parseRemoteReleaseStatus(
        await invoke<unknown>("codex_desktop_check_latest", { force }),
      );
    },
    getJob: async () =>
      parseOptionalJobSnapshot(await invoke<unknown>("codex_desktop_get_job")),
    pickStagingDirectory: async () =>
      parseCodexStagingSelection(
        await invoke<unknown>("codex_desktop_pick_staging_directory"),
      ),
    prepareInstall: async (expectedReleaseId, stagingCapabilityId) =>
      parseCodexInstallPreflight(
        await invoke<unknown>("codex_desktop_prepare_install", {
          request: {
            expectedReleaseId: assertExpectedReleaseId(expectedReleaseId),
            stagingCapabilityId:
              stagingCapabilityId === undefined
                ? null
                : assertStagingCapabilityId(stagingCapabilityId),
          },
        }),
        expectedReleaseId,
      ),
    discardInstallConfirmation: async (confirmationId) => {
      await invoke("codex_desktop_discard_install_confirmation", {
        confirmationId: assertInstallConfirmationId(confirmationId),
      });
    },
    startInstall: async (expectedReleaseId, confirmationId) =>
      parseJobSnapshot(
        await invoke<unknown>("codex_desktop_start_install", {
          request: {
            expectedReleaseId: assertExpectedReleaseId(expectedReleaseId),
            confirmationId: assertInstallConfirmationId(confirmationId),
          },
        }),
      ),
    cancelInstall: async (jobId) =>
      parseJobSnapshot(
        await invoke<unknown>("codex_desktop_cancel_install", {
          jobId: assertJobId(jobId),
        }),
      ),
    launch: async () => {
      await invoke("codex_desktop_launch");
    },
    openLogDirectory: async () => {
      await invoke("codex_desktop_open_log_directory");
    },
    subscribeJobUpdates: async (onSnapshot) =>
      listen<unknown>(JOB_UPDATED_EVENT, (event) => {
        onSnapshot(parseJobSnapshot(event.payload));
      }),
  };
}
