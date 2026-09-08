import {
  comparePlatformVersions,
  isInstalledLocalStatus,
  type InstallerErrorDto,
  type InstallerPrimaryAction,
  type InstallerViewState,
  type JobSnapshot,
  type LocalInstallStatus,
  type RemoteReleaseStatus,
} from "./types";
import {
  blocksInstallOrUpdate,
  canRetryRemoteVersion,
  type LocalVersionState,
  type RemoteVersionState,
} from "./versionState";

export interface InstallerViewStateOptions {
  localPending: boolean;
  remotePending: boolean;
  localFailed: boolean;
  remoteFailed: boolean;
  job: JobSnapshot | null | undefined;
}

export function deriveInstallerViewState(
  local: LocalInstallStatus | undefined,
  remote: RemoteReleaseStatus | undefined,
  options: InstallerViewStateOptions,
): InstallerViewState {
  if (local?.state === "unsupported") {
    return local.reason === "platform" ? "hidden" : "unsupported_architecture";
  }

  if (local?.state === "ambiguous") {
    return "ambiguous";
  }

  const job = options.job;
  if (job) {
    switch (job.stage) {
      case "checking":
        return "job_checking";
      case "preflight":
        return "job_preflight";
      case "downloading":
        return "job_downloading";
      case "installing":
        return "job_installing";
      case "verifying_installation":
        return "job_verifying_installation";
      case "succeeded":
        return "succeeded";
      case "failed":
        return "failed";
      case "cancelled":
        return "cancelled";
    }
  }

  if (options.localPending || options.remotePending || !local) {
    return "checking";
  }

  // A background remote failure retains the previously validated descriptor.
  // Its dedicated version state disables install/update while the known local
  // application remains launchable; it must not collapse into unavailable.
  if (options.localFailed || !remote) {
    return isInstalledLocalStatus(local)
      ? "remote_unavailable_installed"
      : "remote_unavailable";
  }

  if (!isInstalledLocalStatus(local)) {
    return "ready_install";
  }

  const comparison = comparePlatformVersions(
    local.application.platformVersion,
    remote.platformVersion,
  );
  if (comparison === -1) return "ready_update";
  if (comparison === 0) return "ready_launch";
  if (comparison === 1) return "local_newer";

  return "remote_unavailable_installed";
}

export function deriveInstallerPrimaryAction(
  state: InstallerViewState,
  local: LocalInstallStatus | undefined,
  remote: RemoteReleaseStatus | undefined,
  error: InstallerErrorDto | null,
): InstallerPrimaryAction {
  switch (state) {
    case "ready_install":
      return "install";
    case "ready_update":
      return "update";
    case "ready_launch":
    case "local_newer":
    case "remote_unavailable_installed":
    case "succeeded":
      return "launch";
    case "checking":
      return isInstalledLocalStatus(local) ? "launch" : null;
    case "remote_unavailable":
      return "retry";
    case "failed":
      if (error?.suggestedAction === "refresh") return "refresh";
      return error?.retryable ? "retry" : null;
    case "cancelled":
      if (!remote) return "retry";
      if (!isInstalledLocalStatus(local)) return "install";
      return (
        deriveInstallerPrimaryAction(
          deriveInstallerViewState(local, remote, {
            localPending: false,
            remotePending: false,
            localFailed: false,
            remoteFailed: false,
            job: null,
          }),
          local,
          remote,
          null,
        ) ?? "retry"
      );
    default:
      return null;
  }
}

export interface InstallerActionState {
  canInstall: boolean;
  canUpdate: boolean;
  canLaunch: boolean;
  canRetryRemote: boolean;
  primaryAction: InstallerPrimaryAction;
  primaryDisabled: boolean;
}

export function deriveInstallerActionState(options: {
  state: InstallerViewState;
  local: LocalInstallStatus | undefined;
  remote: RemoteReleaseStatus | undefined;
  localVersion: LocalVersionState;
  remoteVersion: RemoteVersionState;
  error: InstallerErrorDto | null;
  isActing: boolean;
}): InstallerActionState {
  const canInstall =
    options.localVersion.kind === "not_installed" &&
    options.remoteVersion.kind === "available";
  const canUpdate =
    isInstalledLocalStatus(options.local) &&
    options.remoteVersion.kind === "available" &&
    options.remote !== undefined &&
    comparePlatformVersions(
      options.local.application.platformVersion,
      options.remote.platformVersion,
    ) === -1;
  const canLaunch = options.localVersion.kind === "installed";
  const canRetryRemote = canRetryRemoteVersion(options.remoteVersion);
  const defaultPrimaryAction = deriveInstallerPrimaryAction(
    options.state,
    options.local,
    options.remote,
    options.error,
  );
  const primaryAction =
    options.state === "remote_unavailable" && !canRetryRemote
      ? null
      : defaultPrimaryAction;
  const primaryDisabled =
    !primaryAction ||
    options.isActing ||
    ((primaryAction === "install" || primaryAction === "update") &&
      (blocksInstallOrUpdate(options.remoteVersion) ||
        (primaryAction === "install" && !canInstall) ||
        (primaryAction === "update" && !canUpdate)));

  return {
    canInstall,
    canUpdate,
    canLaunch,
    canRetryRemote,
    primaryAction,
    primaryDisabled,
  };
}

export const installerStateMessageKeys: Readonly<
  Record<InstallerViewState, string>
> = {
  hidden: "codexDesktop.state.hidden",
  checking: "codexDesktop.state.checking",
  unsupported_architecture: "codexDesktop.state.unsupportedArchitecture",
  ambiguous: "codexDesktop.state.ambiguous",
  ready_install: "codexDesktop.state.readyInstall",
  ready_update: "codexDesktop.state.updateAvailable",
  ready_launch: "codexDesktop.state.upToDate",
  local_newer: "codexDesktop.state.localNewer",
  remote_unavailable: "codexDesktop.state.remoteUnavailable",
  remote_unavailable_installed: "codexDesktop.state.remoteUnavailableInstalled",
  job_checking: "codexDesktop.state.checking",
  job_preflight: "codexDesktop.state.preflight",
  job_downloading: "codexDesktop.state.downloading",
  job_installing: "codexDesktop.state.installing",
  job_verifying_installation: "codexDesktop.state.verifyingInstallation",
  succeeded: "codexDesktop.state.succeeded",
  failed: "codexDesktop.state.failed",
  cancelled: "codexDesktop.state.cancelled",
};

export function installerStatusMessageKey(
  state: InstallerViewState,
  localVersion: LocalVersionState,
  remoteVersion: RemoteVersionState,
): string {
  if (
    state.startsWith("job_") ||
    state === "succeeded" ||
    state === "failed" ||
    state === "cancelled" ||
    state === "hidden" ||
    state === "unsupported_architecture" ||
    state === "ambiguous"
  ) {
    return installerStateMessageKeys[state];
  }

  switch (remoteVersion.kind) {
    case "refreshing":
      return "codexDesktop.version.refreshing";
    case "refetch_error":
      return "codexDesktop.version.refreshNetworkFailed";
    case "initial_network_error":
      return "codexDesktop.version.fetchFailed";
    case "platform_unavailable":
      return "codexDesktop.version.platformUnavailable";
    case "metadata_error":
      return "codexDesktop.version.metadataInvalid";
    case "loading":
      return localVersion.kind === "loading"
        ? "codexDesktop.version.localLoading"
        : "codexDesktop.version.remoteLoading";
    case "available":
      if (localVersion.kind === "loading") {
        return "codexDesktop.version.localLoading";
      }
      if (localVersion.kind === "error") {
        return "codexDesktop.version.localError";
      }
      return installerStateMessageKeys[state];
  }
}
