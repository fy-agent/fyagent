import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { useQueryClient } from "@tanstack/react-query";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import {
  asInstallerError,
  createDownloadSpeedState,
  deriveInstallerActionState,
  deriveInstallerViewState,
  deriveLocalVersionState,
  deriveRemoteVersionState,
  installerErrorDetailsForCopy,
  installerStatusMessageKey,
  isTerminalJobStage,
  latestKnownInstallerError,
  projectInstallerProgress,
  shouldAcceptJobSnapshot,
  updateDownloadSpeedState,
  type CodexDesktopProgress,
  type DownloadSpeedState,
  type InstallerErrorDto,
  type InstallerPrimaryAction,
  type InstallerViewState,
  type JobSnapshot,
  type LocalVersionState,
  type RemoteVersionState,
} from "@/shared/codex-desktop";
import { codexDesktopApi } from "@/lib/api/codex-desktop";
import {
  codexDesktopKeys,
  useCodexDesktopJob,
  useCodexDesktopLatestRelease,
  useCodexDesktopLocalStatus,
} from "@/lib/query/codex-desktop";

export {
  deriveInstallerViewState,
  shouldAcceptJobSnapshot,
} from "@/shared/codex-desktop";
export type { CodexDesktopProgress } from "@/shared/codex-desktop";

const JOB_UPDATED_EVENT = "codex-desktop-installer://job-updated";
const successToastJobIds = new Set<string>();

export interface CodexDesktopInstallerViewModel {
  state: InstallerViewState;
  localVersion: LocalVersionState;
  remoteVersion: RemoteVersionState;
  canInstall: boolean;
  canUpdate: boolean;
  canLaunch: boolean;
  canRetryRemote: boolean;
  statusMessageKey: string;
  progress?: CodexDesktopProgress;
  primaryAction: InstallerPrimaryAction;
  primaryDisabled: boolean;
  canCancel: boolean;
  error: InstallerErrorDto | null;
  isActing: boolean;
  isRefreshing: boolean;
  refresh(): Promise<void>;
  runPrimaryAction(): Promise<void>;
  launch(): Promise<void>;
  cancel(): Promise<void>;
  copyErrorDetails(): Promise<void>;
  openLogs(): Promise<void>;
}

export function useCodexDesktopInstaller(): CodexDesktopInstallerViewModel {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const localQuery = useCodexDesktopLocalStatus();
  const remoteQuery = useCodexDesktopLatestRelease();
  const jobQuery = useCodexDesktopJob();
  const [actionError, setActionError] = useState<unknown>(null);
  const [isActing, setIsActing] = useState(false);
  const downloadSpeedStateRef = useRef<DownloadSpeedState>(
    createDownloadSpeedState(),
  );
  const [downloadSpeedState, setDownloadSpeedState] =
    useState<DownloadSpeedState>(() => downloadSpeedStateRef.current);
  const [acknowledgedMetadataChangeJobId, setAcknowledgedMetadataChangeJobId] =
    useState<string | null>(null);

  // Native events can coalesce before React renders; retain every accepted
  // snapshot so the next accepted event still has its adjacent baseline.
  const recordDownloadSpeedSnapshot = useCallback(
    (snapshot: JobSnapshot | null | undefined) => {
      const next = updateDownloadSpeedState(
        downloadSpeedStateRef.current,
        snapshot,
      );
      if (next === downloadSpeedStateRef.current) return;
      downloadSpeedStateRef.current = next;
      setDownloadSpeedState(next);
    },
    [],
  );

  const mergeJobSnapshot = useCallback(
    (incoming: JobSnapshot) => {
      let accepted = false;
      queryClient.setQueryData<JobSnapshot | null>(
        codexDesktopKeys.job(),
        (current) => {
          accepted = shouldAcceptJobSnapshot(current, incoming);
          return accepted ? incoming : current;
        },
      );
      if (accepted) recordDownloadSpeedSnapshot(incoming);
    },
    [queryClient, recordDownloadSpeedSnapshot],
  );

  useEffect(() => {
    let disposed = false;
    let unlisten: UnlistenFn | undefined;

    void (async () => {
      try {
        const dispose = await listen<JobSnapshot>(
          JOB_UPDATED_EVENT,
          (event) => {
            mergeJobSnapshot(event.payload);
          },
        );
        if (disposed) {
          dispose();
          return;
        }
        unlisten = dispose;
        const snapshot = await codexDesktopApi.getJob();
        if (!disposed && snapshot) {
          mergeJobSnapshot(snapshot);
        }
      } catch (error) {
        if (!disposed) {
          console.warn("Failed to recover Codex desktop installer job", error);
        }
      }
    })();

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [mergeJobSnapshot]);

  useEffect(() => {
    let disposed = false;

    const recoverOnFocus = () => {
      void codexDesktopApi
        .getJob()
        .then((snapshot) => {
          if (!disposed && snapshot) mergeJobSnapshot(snapshot);
        })
        .catch((error) => {
          if (!disposed) {
            console.warn(
              "Failed to refresh Codex desktop installer job",
              error,
            );
          }
        });
    };
    window.addEventListener("focus", recoverOnFocus);
    return () => {
      disposed = true;
      window.removeEventListener("focus", recoverOnFocus);
    };
  }, [mergeJobSnapshot]);

  const local = localQuery.data;
  const remote = remoteQuery.data;
  const job = jobQuery.data;
  const localVersion = deriveLocalVersionState(local, {
    isLoading: localQuery.isLoading,
    isError: localQuery.isError,
  });
  const remoteVersion = deriveRemoteVersionState(remote, {
    isLoading: remoteQuery.isLoading,
    isError: remoteQuery.isError,
    isRefetching: remoteQuery.isRefetching,
    isRefetchError: remoteQuery.isRefetchError,
    errorCode: asInstallerError(remoteQuery.error)?.code ?? null,
  });

  useLayoutEffect(() => {
    recordDownloadSpeedSnapshot(job);
  }, [job, recordDownloadSpeedSnapshot]);

  const isAcknowledgedMetadataChange =
    job?.stage === "failed" && job.jobId === acknowledgedMetadataChangeJobId;
  // JobStore intentionally retains terminal successes. Once a refresh reports
  // another release, local and remote versions determine the next action.
  const isSucceededJobSupersededByRemote =
    job?.stage === "succeeded" &&
    remote !== undefined &&
    job.release.releaseId !== remote.releaseId;
  const displayJob =
    isAcknowledgedMetadataChange || isSucceededJobSupersededByRemote
      ? null
      : job;
  const state = deriveInstallerViewState(local, remote, {
    localPending: localQuery.isLoading,
    remotePending: remoteQuery.isLoading,
    localFailed: localQuery.isError,
    remoteFailed: remoteQuery.isError,
    job: displayJob,
  });
  const error = latestKnownInstallerError(local, displayJob, [
    actionError,
    localQuery.error,
    remoteQuery.error,
  ]);
  const {
    canInstall,
    canUpdate,
    canLaunch,
    canRetryRemote,
    primaryAction,
    primaryDisabled,
  } = deriveInstallerActionState({
    state,
    local,
    remote,
    localVersion,
    remoteVersion,
    error,
    isActing,
  });
  const statusMessageKey = installerStatusMessageKey(
    state,
    localVersion,
    remoteVersion,
  );

  useEffect(() => {
    if (!job || !isTerminalJobStage(job.stage)) return;
    void queryClient.invalidateQueries({ queryKey: codexDesktopKeys.local() });
    void queryClient.invalidateQueries({ queryKey: codexDesktopKeys.remote() });
  }, [job?.jobId, job?.stage, queryClient]);

  useEffect(() => {
    if (job?.stage !== "succeeded" || successToastJobIds.has(job.jobId)) {
      return;
    }
    successToastJobIds.add(job.jobId);
    toast.success(t("codexDesktop.toast.installed"));
  }, [job?.jobId, job?.stage, t]);

  const refreshLatest = useCallback(async (): Promise<boolean> => {
    setActionError(null);
    try {
      const latest = await queryClient.fetchQuery({
        queryKey: codexDesktopKeys.remote(),
        queryFn: () => codexDesktopApi.checkLatest(true),
        staleTime: 0,
      });
      queryClient.setQueryData(codexDesktopKeys.remote(), latest);
      return true;
    } catch (error) {
      setActionError(error);
      return false;
    }
  }, [queryClient]);

  const refresh = useCallback(async () => {
    const refreshed = await refreshLatest();
    if (
      refreshed &&
      job?.stage === "failed" &&
      job.error?.suggestedAction === "refresh"
    ) {
      // A metadata mismatch is deliberately a two-step action: refreshing
      // reveals the newly checked release, while a separate primary action is
      // required before any installation can start.
      setAcknowledgedMetadataChangeJobId(job.jobId);
    }
  }, [job, refreshLatest]);

  const startWithKnownRelease = useCallback(async () => {
    const expectedReleaseId = remote?.releaseId ?? job?.release.releaseId;
    if (!expectedReleaseId) {
      await refresh();
      return;
    }

    const snapshot = await codexDesktopApi.startInstall(expectedReleaseId);
    mergeJobSnapshot(snapshot);
  }, [job?.release.releaseId, mergeJobSnapshot, refresh, remote?.releaseId]);

  const launch = useCallback(async () => {
    if (isActing) return;
    setActionError(null);
    setIsActing(true);
    try {
      await codexDesktopApi.launch();
    } catch (error) {
      setActionError(error);
    } finally {
      setIsActing(false);
    }
  }, [isActing]);

  const runPrimaryAction = useCallback(async () => {
    if (!primaryAction || isActing) return;
    setActionError(null);
    setIsActing(true);
    try {
      if (primaryAction === "launch") {
        await codexDesktopApi.launch();
      } else if (
        primaryAction === "refresh" ||
        (primaryAction === "retry" && state === "remote_unavailable")
      ) {
        await refresh();
      } else {
        await startWithKnownRelease();
      }
    } catch (error) {
      setActionError(error);
    } finally {
      setIsActing(false);
    }
  }, [isActing, primaryAction, refresh, startWithKnownRelease, state]);

  const cancel = useCallback(async () => {
    if (!job?.cancellable || isActing) return;
    setActionError(null);
    setIsActing(true);
    try {
      const snapshot = await codexDesktopApi.cancelInstall(job.jobId);
      mergeJobSnapshot(snapshot);
    } catch (error) {
      setActionError(error);
    } finally {
      setIsActing(false);
    }
  }, [isActing, job?.cancellable, job?.jobId, mergeJobSnapshot]);

  const copyErrorDetails = useCallback(async () => {
    const details = installerErrorDetailsForCopy(error);
    if (!details) return;

    try {
      if (!navigator.clipboard?.writeText) {
        throw new Error("clipboard API is unavailable");
      }
      await navigator.clipboard.writeText(details);
      toast.success(t("codexDesktop.toast.copied"));
    } catch (clipboardError) {
      console.warn(
        "Failed to copy Codex desktop installer diagnostics",
        clipboardError,
      );
      toast.error(t("codexDesktop.toast.copyFailed"));
    }
  }, [error, t]);

  const openLogs = useCallback(async () => {
    setActionError(null);
    setIsActing(true);
    try {
      await codexDesktopApi.openLogDirectory();
    } catch (openLogsError) {
      setActionError(openLogsError);
    } finally {
      setIsActing(false);
    }
  }, []);

  const progress = useMemo<CodexDesktopProgress | undefined>(() => {
    return projectInstallerProgress(job, downloadSpeedState);
  }, [downloadSpeedState, job]);

  return {
    state,
    localVersion,
    remoteVersion,
    canInstall,
    canUpdate,
    canLaunch,
    canRetryRemote,
    statusMessageKey,
    progress,
    primaryAction,
    primaryDisabled,
    canCancel: Boolean(job?.cancellable) && !isActing,
    error,
    isActing,
    isRefreshing: remoteQuery.isFetching,
    refresh,
    runPrimaryAction,
    launch,
    cancel,
    copyErrorDetails,
    openLogs,
  };
}
