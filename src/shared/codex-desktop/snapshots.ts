import { isTerminalJobStage, type JobSnapshot } from "./types";

export interface CodexDesktopProgress {
  current: number | null;
  total: number | null;
  percent: number | null;
  bytesPerSecond: number | null;
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
  job: JobSnapshot,
): DownloadSpeedMeasurement | null {
  return measurement && measurement.jobId === job.jobId
    ? { ...measurement, sequence: job.sequence }
    : null;
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
export function updateDownloadSpeedState(
  current: DownloadSpeedState,
  job: JobSnapshot | null | undefined,
): DownloadSpeedState {
  const identity = job ? { jobId: job.jobId, sequence: job.sequence } : null;
  if (
    identity &&
    current.snapshot &&
    identity.jobId === current.snapshot.jobId &&
    identity.sequence === current.snapshot.sequence
  ) {
    return current;
  }

  const completedBytes = job?.progress?.completedBytes;
  const updatedAtMs = job ? Date.parse(job.updatedAt) : Number.NaN;
  if (
    job?.stage !== "downloading" ||
    job.progress?.phase !== "download" ||
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

  const sample = { jobId: job.jobId, completedBytes, updatedAtMs };
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
  if (elapsedFromPrevious === 0 && deltaFromPrevious > 0) {
    return {
      snapshot: identity,
      origin: null,
      sample: null,
      measurement: null,
    };
  }
  if (deltaFromPrevious === 0) {
    return {
      snapshot: identity,
      origin: current.origin,
      sample,
      measurement: retainMeasurement(current.measurement, job),
    };
  }

  const elapsedFromOrigin = sample.updatedAtMs - current.origin.updatedAtMs;
  const deltaFromOrigin = sample.completedBytes - current.origin.completedBytes;
  if (elapsedFromOrigin < MIN_AVERAGE_ELAPSED_MS || deltaFromOrigin <= 0) {
    return {
      snapshot: identity,
      origin: current.origin,
      sample,
      measurement: retainMeasurement(current.measurement, job),
    };
  }

  const bytesPerSecond = (deltaFromOrigin * 1000) / elapsedFromOrigin;
  return {
    snapshot: identity,
    origin: current.origin,
    sample,
    measurement:
      Number.isFinite(bytesPerSecond) && bytesPerSecond >= 0
        ? { jobId: job.jobId, sequence: job.sequence, bytesPerSecond }
        : null,
  };
}

export function selectDownloadBytesPerSecond(
  state: DownloadSpeedState,
  job: JobSnapshot | null | undefined,
): number | null {
  return job?.stage === "downloading" &&
    job.progress?.phase === "download" &&
    state.measurement?.jobId === job.jobId &&
    state.measurement.sequence === job.sequence
    ? state.measurement.bytesPerSecond
    : null;
}

export function projectInstallerProgress(
  job: JobSnapshot | null | undefined,
  downloadSpeed: DownloadSpeedState,
): CodexDesktopProgress | undefined {
  if (!job?.progress) return undefined;
  return {
    current: job.progress.completedBytes,
    total: job.progress.totalBytes,
    percent: job.progress.percent,
    bytesPerSecond: selectDownloadBytesPerSecond(downloadSpeed, job),
  };
}
