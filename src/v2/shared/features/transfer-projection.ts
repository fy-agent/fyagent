import {
  createDownloadSpeedState,
  formatTransferBytes,
  formatTransferPercent,
  formatTransferSpeed,
  projectTransferPresentation,
  selectDownloadBytesPerSecondFromSample,
  updateDownloadSpeedFromSample,
  type DownloadSpeedState,
  type TransferPresentation,
  type TransferSpeedSampleInput,
} from "@/shared/codex-desktop";

import type {
  AgentActionJobSnapshot,
  AgentActionJobStage,
} from "./agent-install-readiness";

export {
  createDownloadSpeedState,
  formatTransferBytes,
  formatTransferPercent,
  formatTransferSpeed,
  projectTransferPresentation,
  updateDownloadSpeedFromSample,
  type DownloadSpeedState,
  type TransferPresentation,
};

export function agentJobToSpeedSample(
  snapshot: AgentActionJobSnapshot | null | undefined,
): TransferSpeedSampleInput | null {
  if (!snapshot?.transfer) return null;
  const observedAtMs = Date.parse(snapshot.transfer.observedAt);
  return {
    jobId: `${snapshot.jobId}:${snapshot.transfer.attempt}`,
    sequence: snapshot.transfer.sequence,
    downloading:
      snapshot.stage === "downloading" &&
      snapshot.transfer.phase === "download",
    downloadPhase: snapshot.transfer.phase === "download",
    completedBytes: snapshot.transfer.completedBytes,
    updatedAtMs: Number.isFinite(observedAtMs) ? observedAtMs : Number.NaN,
  };
}

export function projectAgentJobTransfer(
  stage: AgentActionJobStage | null,
  snapshot: AgentActionJobSnapshot | null | undefined,
  speed: DownloadSpeedState,
): TransferPresentation {
  const downloading = stage === "downloading";
  const terminal =
    stage === "succeeded" ||
    stage === "failed" ||
    stage === "cancelled" ||
    stage === "incomplete";
  const transfer = snapshot?.transfer ?? null;
  const sample = agentJobToSpeedSample(snapshot);
  return projectTransferPresentation({
    downloading,
    terminal,
    completedBytes: transfer?.completedBytes ?? null,
    totalBytes: transfer?.totalBytes ?? null,
    percent: null,
    bytesPerSecond: selectDownloadBytesPerSecondFromSample(speed, sample),
  });
}
