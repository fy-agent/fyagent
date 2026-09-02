import { isTerminalJobStage, type JobSnapshot } from "./types";

export interface CodexDesktopProgress {
  current: number | null;
  total: number | null;
  percent: number | null;
  bytesPerSecond: number | null;
}

export interface TransferSpeedSampleInput {
  jobId: string;
  sequence: number;
  downloading: boolean;
  downloadPhase: boolean;
  completedBytes: number | null;
  updatedAtMs: number;
}

export interface TransferPresentation {
  percent: number | null;
  percentLabel: string | null;
  transferredLabel: string | null;
  speedLabel: string | null;
  indeterminate: boolean;
  downloadLine: string | null;
}

export interface DownloadSpeedSample {
  jobId: string;
  completedBytes: number;
  updatedAtMs: number;
}

export interface DownloadSpeedMeasurement {
  jobId: string;
  sequence: number;
  bytesPerSecond: number;
}

export interface DownloadSpeedSnapshotIdentity {
  jobId: string;
  sequence: number;
}

export interface DownloadSpeedState {
  snapshot: DownloadSpeedSnapshotIdentity | null;
  origin: DownloadSpeedSample | null;
  sample: DownloadSpeedSample | null;
  measurement: DownloadSpeedMeasurement | null;
}

export function createDownloadSpeedState(): DownloadSpeedState {
  return { snapshot: null, origin: null, sample: null, measurement: null };
}

const MIN_AVERAGE_ELAPSED_MS = 1000;

function retainMeasurement(
  measurement: DownloadSpeedMeasurement | null,
  jobId: string,
  sequence: number,
): DownloadSpeedMeasurement | null {
  return measurement && measurement.jobId === jobId
    ? { ...measurement, sequence }
    : null;
}

function jobToSpeedSample(
  job: JobSnapshot | null | undefined,
): TransferSpeedSampleInput | null {
  if (!job) return null;
  return {
    jobId: job.jobId,
    sequence: job.sequence,
    downloading: job.stage === "downloading",
    downloadPhase: job.progress?.phase === "download",
    completedBytes: job.progress?.completedBytes ?? null,
    updatedAtMs: Date.parse(job.updatedAt),
  };
}

/**
 * Accepts only a later snapshot for the same job. Distinct job IDs are ordered
 * by backend-issued start time so a delayed event for an older terminal job
 * cannot overwrite a newly started installation.
 */
export function shouldAcceptJobSnapshot(
  current: JobSnapshot | null | undefined,
  incoming: JobSnapshot,
): boolean {
  if (!current) return true;

  if (current.jobId === incoming.jobId) {
    return incoming.sequence > current.sequence;
  }

  const currentStartedAt = Date.parse(current.startedAt);
  const incomingStartedAt = Date.parse(incoming.startedAt);
  if (Number.isFinite(currentStartedAt) && Number.isFinite(incomingStartedAt)) {
    if (incomingStartedAt !== currentStartedAt) {
      return incomingStartedAt > currentStartedAt;
    }
  }

  return (
    isTerminalJobStage(current.stage) && !isTerminalJobStage(incoming.stage)
  );
}

/**
 * Records one already-accepted snapshot. Speed is the job-lifetime average
 * from the first valid sample, so 100ms progress hops do not flicker.
 */
export function updateDownloadSpeedFromSample(
  current: DownloadSpeedState,
  input: TransferSpeedSampleInput | null | undefined,
): DownloadSpeedState {
  const identity = input
    ? { jobId: input.jobId, sequence: input.sequence }
    : null;
  if (
    identity &&
    current.snapshot &&
    identity.jobId === current.snapshot.jobId &&
    identity.sequence === current.snapshot.sequence
  ) {
    return current;
  }

  const completedBytes = input?.completedBytes;
  const updatedAtMs = input?.updatedAtMs ?? Number.NaN;
  if (
    !input?.downloading ||
    !input.downloadPhase ||
    completedBytes == null ||
    !Number.isFinite(completedBytes) ||
    completedBytes < 0 ||
    !Number.isFinite(updatedAtMs)
  ) {
    return {
      snapshot: identity,
      origin: null,
      sample: null,
      measurement: null,
    };
  }

  const sample = { jobId: input.jobId, completedBytes, updatedAtMs };
  if (!current.origin || current.origin.jobId !== sample.jobId) {
    return { snapshot: identity, origin: sample, sample, measurement: null };
  }

  const previous = current.sample;
  if (!previous || previous.jobId !== sample.jobId) {
    return { snapshot: identity, origin: sample, sample, measurement: null };
  }

  const elapsedFromPrevious = sample.updatedAtMs - previous.updatedAtMs;
  const deltaFromPrevious = sample.completedBytes - previous.completedBytes;
  if (elapsedFromPrevious < 0 || deltaFromPrevious < 0) {
    return {
      snapshot: identity,
      origin: null,
      sample: null,
      measurement: null,
    };
  }
  if (elapsedFromPrevious === 0 || deltaFromPrevious === 0) {
    return {
      snapshot: identity,
      origin: current.origin,
      sample,
      measurement: retainMeasurement(
        current.measurement,
        input.jobId,
        input.sequence,
      ),
    };
  }

  const elapsedFromOrigin = sample.updatedAtMs - current.origin.updatedAtMs;
  const deltaFromOrigin = sample.completedBytes - current.origin.completedBytes;
  if (elapsedFromOrigin < MIN_AVERAGE_ELAPSED_MS || deltaFromOrigin <= 0) {
    return {
      snapshot: identity,
      origin: current.origin,
      sample,
      measurement: retainMeasurement(
        current.measurement,
        input.jobId,
        input.sequence,
      ),
    };
  }

  const bytesPerSecond = (deltaFromOrigin * 1000) / elapsedFromOrigin;
  return {
    snapshot: identity,
    origin: current.origin,
    sample,
    measurement:
      Number.isFinite(bytesPerSecond) && bytesPerSecond >= 0
        ? { jobId: input.jobId, sequence: input.sequence, bytesPerSecond }
        : null,
  };
}

export function updateDownloadSpeedState(
  current: DownloadSpeedState,
  job: JobSnapshot | null | undefined,
): DownloadSpeedState {
  return updateDownloadSpeedFromSample(current, jobToSpeedSample(job));
}

export function selectDownloadBytesPerSecondFromSample(
  state: DownloadSpeedState,
  input: TransferSpeedSampleInput | null | undefined,
): number | null {
  if (
    !input?.downloading ||
    !input.downloadPhase ||
    state.measurement?.jobId !== input.jobId ||
    state.measurement.sequence !== input.sequence
  ) {
    return null;
  }
  const bytesPerSecond = state.measurement.bytesPerSecond;
  return Number.isFinite(bytesPerSecond) && bytesPerSecond > 0
    ? bytesPerSecond
    : null;
}

export function selectDownloadBytesPerSecond(
  state: DownloadSpeedState,
  job: JobSnapshot | null | undefined,
): number | null {
  return selectDownloadBytesPerSecondFromSample(state, jobToSpeedSample(job));
}

export function clampTransferPercent(value: number): number {
  if (!Number.isFinite(value)) return 0;
  return Math.min(100, Math.max(0, value));
}

export function computeTransferPercent(
  completedBytes: number | null | undefined,
  totalBytes: number | null | undefined,
  backendPercent?: number | null,
): number | null {
  if (typeof backendPercent === "number" && Number.isFinite(backendPercent)) {
    return clampTransferPercent(backendPercent);
  }
  if (
    typeof completedBytes !== "number" ||
    typeof totalBytes !== "number" ||
    !Number.isFinite(completedBytes) ||
    !Number.isFinite(totalBytes) ||
    totalBytes <= 0
  ) {
    return null;
  }
  return clampTransferPercent((completedBytes / totalBytes) * 100);
}

export function formatTransferPercent(percent: number): string {
  const clamped = clampTransferPercent(percent);
  const rounded = Math.round(clamped * 10) / 10;
  return Number.isInteger(rounded) ? `${rounded}%` : `${rounded.toFixed(1)}%`;
}

function formatTransferAmount(
  value: number | null | undefined,
  keepWholeTenth: boolean,
): string | null {
  if (value == null || !Number.isFinite(value) || value < 0) return null;
  const units = ["B", "KB", "MB", "GB", "TB"] as const;
  let amount = value;
  let unit = 0;
  while (amount >= 1024 && unit < units.length - 1) {
    amount /= 1024;
    unit += 1;
  }
  if (unit === 0) return `${Math.round(amount)} B`;
  const digits = amount >= 100 ? 0 : 1;
  let label = amount.toFixed(digits);
  if (!keepWholeTenth) {
    label = label.replace(/\.0$/, "");
  }
  return `${label} ${units[unit]}`;
}

export function formatTransferBytes(
  value: number | null | undefined,
): string | null {
  return formatTransferAmount(value, false);
}

export function formatTransferSpeed(
  bytesPerSecond: number | null | undefined,
): string | null {
  if (
    bytesPerSecond == null ||
    !Number.isFinite(bytesPerSecond) ||
    bytesPerSecond <= 0
  ) {
    return null;
  }
  const bytes = formatTransferAmount(bytesPerSecond, true);
  return bytes ? `${bytes}/s` : null;
}

export function projectTransferPresentation({
  downloading,
  terminal,
  completedBytes,
  totalBytes,
  percent,
  bytesPerSecond,
}: {
  downloading: boolean;
  terminal: boolean;
  completedBytes: number | null;
  totalBytes: number | null;
  percent: number | null;
  bytesPerSecond: number | null;
}): TransferPresentation {
  const resolvedPercent = computeTransferPercent(
    completedBytes,
    totalBytes,
    percent,
  );
  const indeterminate = resolvedPercent === null;
  const transferredLabel = formatTransferBytes(completedBytes);
  const speedLabel =
    downloading && !terminal ? formatTransferSpeed(bytesPerSecond) : null;
  const percentLabel =
    downloading && !indeterminate && resolvedPercent !== null
      ? formatTransferPercent(resolvedPercent)
      : null;
  let downloadLine: string | null = null;
  if (downloading) {
    if (percentLabel) {
      downloadLine = speedLabel
        ? `下载中 ${percentLabel} · ${speedLabel}`
        : `下载中 ${percentLabel}`;
    } else if (transferredLabel) {
      downloadLine = speedLabel
        ? `已下载 ${transferredLabel} · ${speedLabel}`
        : `已下载 ${transferredLabel}`;
    }
  }
  return {
    percent: downloading ? resolvedPercent : null,
    percentLabel: downloading ? percentLabel : null,
    transferredLabel: downloading ? transferredLabel : null,
    speedLabel,
    indeterminate: downloading && indeterminate,
    downloadLine,
  };
}

export function projectInstallerProgress(
  job: JobSnapshot | null | undefined,
  downloadSpeed: DownloadSpeedState,
): CodexDesktopProgress | undefined {
  if (!job?.progress) return undefined;
  return {
    current: job.progress.completedBytes,
    total: job.progress.totalBytes,
    percent: computeTransferPercent(
      job.progress.completedBytes,
      job.progress.totalBytes,
      job.progress.percent,
    ),
    bytesPerSecond: selectDownloadBytesPerSecond(downloadSpeed, job),
  };
}
