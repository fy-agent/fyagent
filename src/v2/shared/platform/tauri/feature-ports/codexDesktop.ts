import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

import {
  assertExpectedReleaseId,
  parseJobSnapshot,
  parseLocalInstallStatus,
  parseOptionalJobSnapshot,
  parseRemoteReleaseStatus,
} from "@/shared/codex-desktop";

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
    startInstall: async (expectedReleaseId) =>
      parseJobSnapshot(
        await invoke<unknown>("codex_desktop_start_install", {
          request: {
            expectedReleaseId: assertExpectedReleaseId(expectedReleaseId),
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
