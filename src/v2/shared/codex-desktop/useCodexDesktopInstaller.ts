import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import {
  asInstallerError,
  createDownloadSpeedState,
  deriveInstallerActionState,
  deriveInstallerViewState,
  deriveLocalVersionState,
  deriveRemoteVersionState,
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
  type LocalInstallStatus,
  type LocalVersionState,
  type RemoteReleaseStatus,
  type RemoteVersionState,
} from "@/shared/codex-desktop";
import { useFeatures } from "../features/provider";

export interface CodexDesktopInstallerViewModel {
  state: InstallerViewState;
  localVersion: LocalVersionState;
  remoteVersion: RemoteVersionState;
  progress: CodexDesktopProgress | undefined;
  error: InstallerErrorDto | null;
  primaryAction: InstallerPrimaryAction;
  primaryDisabled: boolean;
  canCancel: boolean;
  canOpenLogs: boolean;
  isActing: boolean;
  isRefreshing: boolean;
  authorityUnavailable: boolean;
  liveUpdatesUnavailable: boolean;
  operationFailed: boolean;
  refresh: () => Promise<void>;
  runPrimaryAction: () => Promise<void>;
  cancel: () => Promise<void>;
  openLogs: () => Promise<void>;
}

function terminalIdentity(job: JobSnapshot): string {
  return `${job.jobId}:${job.sequence}:${job.stage}`;
}

function terminalOutcomeIdentity(job: JobSnapshot): string {
  return `${job.jobId}:${job.stage}`;
}

function isWorkingState(state: InstallerViewState): boolean {
  return state.startsWith("job_");
}

export function useCodexDesktopInstaller(): CodexDesktopInstallerViewModel {
  const { ports } = useFeatures();
  const port = ports.codexDesktop;
  const aliveRef = useRef(false);
  const operationLockRef = useRef(false);
  const localRequestRef = useRef(0);
  const remoteRequestRef = useRef(0);
  const remoteRef = useRef<RemoteReleaseStatus>();
  const acceptedJobRef = useRef<JobSnapshot | null>(null);
  const speedRef = useRef<DownloadSpeedState>(createDownloadSpeedState());
  const terminalRefreshRef = useRef<string | null>(null);
  const subscriptionBarrierRef = useRef<Promise<void>>(Promise.resolve());

  const [local, setLocal] = useState<LocalInstallStatus>();
  const [remote, setRemote] = useState<RemoteReleaseStatus>();
  const [job, setJob] = useState<JobSnapshot | null>(null);
  const [downloadSpeed, setDownloadSpeed] = useState<DownloadSpeedState>(
    createDownloadSpeedState,
  );
  const [dismissedTerminal, setDismissedTerminal] = useState<string | null>(
    null,
  );
  const [localPending, setLocalPending] = useState(true);
  const [remotePending, setRemotePending] = useState(true);
  const [remoteRefetching, setRemoteRefetching] = useState(false);
  const [localFailure, setLocalFailure] = useState<unknown>(null);
  const [remoteFailure, setRemoteFailure] = useState<unknown>(null);
  const [remoteRefetchFailed, setRemoteRefetchFailed] = useState(false);
  const [jobReadFailure, setJobReadFailure] = useState<unknown>(null);
  const [subscriptionFailure, setSubscriptionFailure] = useState<unknown>(null);
  const [actionFailure, setActionFailure] = useState<unknown>(null);
  const [operationFailed, setOperationFailed] = useState(false);
  const [isActing, setIsActing] = useState(false);

  useEffect(() => {
    aliveRef.current = true;
    return () => {
      aliveRef.current = false;
    };
  }, []);

  const acceptSnapshot = useCallback((incoming: JobSnapshot): boolean => {
    const accepted = shouldAcceptJobSnapshot(acceptedJobRef.current, incoming);
    if (!accepted) {
      return false;
    }

    acceptedJobRef.current = incoming;
    const nextSpeed = updateDownloadSpeedState(speedRef.current, incoming);
    speedRef.current = nextSpeed;
    if (aliveRef.current) {
      setJob(incoming);
      setDownloadSpeed(nextSpeed);
      setDismissedTerminal((current) =>
        current === terminalIdentity(incoming) ? current : null,
      );
      setJobReadFailure(null);
    }
    return true;
  }, []);

  const readLocal = useCallback(async (): Promise<LocalInstallStatus> => {
    const request = localRequestRef.current + 1;
    localRequestRef.current = request;
    if (aliveRef.current) setLocalPending(true);

    try {
      const next = await port.getLocalStatus();
      if (aliveRef.current && request === localRequestRef.current) {
        setLocal(next);
        setLocalFailure(null);
        setLocalPending(false);
      }
      return next;
    } catch (error) {
      if (aliveRef.current && request === localRequestRef.current) {
        setLocalFailure(error);
        setLocalPending(false);
      }
      throw error;
    }
  }, [port]);

  const readRemote = useCallback(
    async (force: boolean): Promise<RemoteReleaseStatus> => {
      const request = remoteRequestRef.current + 1;
      remoteRequestRef.current = request;
      if (aliveRef.current) {
        if (remoteRef.current) setRemoteRefetching(true);
        else setRemotePending(true);
      }

      try {
        const next = await port.checkLatest(force);
        if (aliveRef.current && request === remoteRequestRef.current) {
          remoteRef.current = next;
          setRemote(next);
          setRemoteFailure(null);
          setRemoteRefetchFailed(false);
          setRemotePending(false);
          setRemoteRefetching(false);
        }
        return next;
      } catch (error) {
        if (aliveRef.current && request === remoteRequestRef.current) {
          setRemoteFailure(error);
          setRemoteRefetchFailed(remoteRef.current !== undefined);
          setRemotePending(false);
          setRemoteRefetching(false);
        }
        throw error;
      }
    },
    [port],
  );

  useEffect(() => {
    void readLocal().catch(() => undefined);
    void readRemote(false).catch(() => undefined);
  }, [readLocal, readRemote]);

  useEffect(() => {
    let disposed = false;
    let unsubscribe: (() => void) | undefined;
    const previousSubscription = subscriptionBarrierRef.current;

    const establish = async () => {
      await previousSubscription;
      if (disposed) return;
      try {
        const cleanup = await port.subscribeJobUpdates((snapshot) => {
          if (!disposed) acceptSnapshot(snapshot);
        });
        if (disposed) {
          cleanup();
          return;
        }
        unsubscribe = cleanup;
        setSubscriptionFailure(null);

        try {
          const recovered = await port.getJob();
          if (!disposed) {
            setJobReadFailure(null);
            if (recovered) acceptSnapshot(recovered);
          }
        } catch (error) {
          if (!disposed) setJobReadFailure(error);
        }
      } catch (error) {
        if (!disposed) setSubscriptionFailure(error);
      }
    };

    const lifecycle = establish();
    subscriptionBarrierRef.current = lifecycle.catch(() => undefined);
    return () => {
      disposed = true;
      unsubscribe?.();
    };
  }, [acceptSnapshot, port]);

  const effectiveJob =
    job && dismissedTerminal === terminalIdentity(job) ? null : job;

  useEffect(() => {
    if (!effectiveJob || !isTerminalJobStage(effectiveJob.stage)) return;
    const identity = terminalOutcomeIdentity(effectiveJob);
    if (terminalRefreshRef.current === identity) return;
    terminalRefreshRef.current = identity;

    void readLocal().catch(() => undefined);
    void readRemote(false).catch(() => undefined);
    void port
      .getJob()
      .then((snapshot) => {
        if (snapshot) acceptSnapshot(snapshot);
      })
      .catch((error: unknown) => {
        if (aliveRef.current) setJobReadFailure(error);
      });
  }, [acceptSnapshot, effectiveJob, port, readLocal, readRemote]);

  const localVersion = deriveLocalVersionState(local, {
    isLoading: localPending,
    isError: localFailure !== null,
  });
  const remoteVersion = deriveRemoteVersionState(remote, {
    isLoading: remotePending,
    isError: remoteFailure !== null,
    isRefetching: remoteRefetching,
    isRefetchError: remoteRefetchFailed,
    errorCode: asInstallerError(remoteFailure)?.code ?? null,
  });
  const state = deriveInstallerViewState(local, remote, {
    localPending,
    remotePending,
    localFailed: localFailure !== null,
    remoteFailed: remoteFailure !== null,
    job: effectiveJob,
  });
  const error = latestKnownInstallerError(local, effectiveJob, [
    actionFailure,
    jobReadFailure,
    remoteFailure,
    localFailure,
    subscriptionFailure,
  ]);
  const actionState = deriveInstallerActionState({
    state,
    local,
    remote,
    localVersion,
    remoteVersion,
    error,
    isActing,
  });
  const progress = useMemo(
    () => projectInstallerProgress(effectiveJob, downloadSpeed),
    [downloadSpeed, effectiveJob],
  );
  const authorityUnavailable =
    !local && !remote && localFailure !== null && remoteFailure !== null;
  const working = isWorkingState(state);

  const performLocked = useCallback(
    async (operation: () => Promise<void>): Promise<void> => {
      if (operationLockRef.current) return;
      operationLockRef.current = true;
      if (aliveRef.current) {
        setIsActing(true);
        setActionFailure(null);
        setOperationFailed(false);
      }
      try {
        await operation();
      } catch (error) {
        if (aliveRef.current) {
          setActionFailure(error);
          setOperationFailed(true);
        }
      } finally {
        operationLockRef.current = false;
        if (aliveRef.current) setIsActing(false);
      }
    },
    [],
  );

  const refresh = useCallback(
    async () =>
      performLocked(async () => {
        await readRemote(true);
        if (
          effectiveJob?.stage === "failed" &&
          effectiveJob.error?.suggestedAction === "refresh"
        ) {
          setDismissedTerminal(terminalIdentity(effectiveJob));
        }
      }),
    [effectiveJob, performLocked, readRemote],
  );

  const runPrimaryAction = useCallback(async () => {
    return performLocked(async () => {
      switch (actionState.primaryAction) {
        case "install":
        case "update": {
          if (!remote) {
            return;
          }
          const snapshot = await port.startInstall(remote.releaseId);
          acceptSnapshot(snapshot);
          return;
        }
        case "launch":
          await port.launch();
          return;
        case "refresh":
          await readRemote(true);
          if (effectiveJob?.stage === "failed") {
            setDismissedTerminal(terminalIdentity(effectiveJob));
          }
          return;
        case "retry":
          if (state === "failed" && remote) {
            const snapshot = await port.startInstall(remote.releaseId);
            acceptSnapshot(snapshot);
          } else {
            await readRemote(true);
          }
          return;
        case null:
          return;
      }
    });
  }, [
    acceptSnapshot,
    actionState.primaryAction,
    effectiveJob,
    performLocked,
    port,
    readRemote,
    remote,
    state,
  ]);

  const cancel = useCallback(
    async () =>
      performLocked(async () => {
        if (!effectiveJob?.cancellable) return;
        const snapshot = await port.cancelInstall(effectiveJob.jobId);
        acceptSnapshot(snapshot);
      }),
    [acceptSnapshot, effectiveJob, performLocked, port],
  );

  const openLogs = useCallback(async () => {
    return performLocked(async () => {
      await port.openLogDirectory();
    });
  }, [performLocked, port]);

  return {
    state,
    localVersion,
    remoteVersion,
    progress,
    error,
    primaryAction: actionState.primaryAction,
    primaryDisabled: actionState.primaryDisabled || working,
    canCancel: Boolean(effectiveJob?.cancellable) && !isActing,
    canOpenLogs: !isActing && (state === "failed" || operationFailed),
    isActing,
    isRefreshing: remoteRefetching,
    authorityUnavailable,
    liveUpdatesUnavailable: subscriptionFailure !== null,
    operationFailed,
    refresh,
    runPrimaryAction,
    cancel,
    openLogs,
  };
}
