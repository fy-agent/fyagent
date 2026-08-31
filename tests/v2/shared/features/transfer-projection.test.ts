import { describe, expect, it } from "vitest";

import {
  AGENT_ACTION_CONTRACT_VERSION,
  type AgentActionJobSnapshot,
} from "@/v2/shared/features/agent-install-readiness";
import {
  agentJobToSpeedSample,
  createDownloadSpeedState,
  projectAgentJobTransfer,
  updateDownloadSpeedFromSample,
} from "@/v2/shared/features/transfer-projection";

function snapshot(
  overrides: Partial<AgentActionJobSnapshot> = {},
): AgentActionJobSnapshot {
  return {
    contractVersion: AGENT_ACTION_CONTRACT_VERSION,
    jobId: "job-1",
    agentId: "qoderwork",
    action: "install",
    stage: "downloading",
    cancellable: true,
    reasonCode: null,
    transfer: {
      phase: "download",
      completedBytes: 3744,
      totalBytes: 10_000,
      attempt: 1,
      maxAttempts: 3,
      sequence: 1,
      observedAt: "2026-08-14T00:00:01.000Z",
    },
    ...overrides,
  };
}

describe("projectAgentJobTransfer", () => {
  it("projects one-decimal percent from job transfer bytes", () => {
    const view = projectAgentJobTransfer(
      "downloading",
      snapshot(),
      createDownloadSpeedState(),
    );
    expect(view.percent).toBeCloseTo(37.44, 5);
    expect(view.percentLabel).toBe("37.4%");
    expect(view.downloadLine).toBe("下载中 37.4%");
    expect(view.speedLabel).toBeNull();
  });

  it("shows transferred bytes without inventing percent when total is unknown", () => {
    const view = projectAgentJobTransfer(
      "downloading",
      snapshot({
        transfer: {
          phase: "download",
          completedBytes: 126 * 1024 * 1024,
          totalBytes: null,
          attempt: 1,
          maxAttempts: 3,
          sequence: 1,
          observedAt: "2026-08-14T00:00:01.000Z",
        },
      }),
      createDownloadSpeedState(),
    );
    expect(view.percent).toBeNull();
    expect(view.indeterminate).toBe(true);
    expect(view.downloadLine).toBe("已下载 126 MB");
  });

  it("hides speed on terminal snapshots even if a previous measurement exists", () => {
    let speed = createDownloadSpeedState();
    const first = snapshot({
      transfer: {
        phase: "download",
        completedBytes: 1024,
        totalBytes: 4096,
        attempt: 1,
        maxAttempts: 3,
        sequence: 1,
        observedAt: "2026-08-14T00:00:01.000Z",
      },
    });
    const second = snapshot({
      transfer: {
        phase: "download",
        completedBytes: 2048,
        totalBytes: 4096,
        attempt: 1,
        maxAttempts: 3,
        sequence: 2,
        observedAt: "2026-08-14T00:00:02.000Z",
      },
    });
    speed = updateDownloadSpeedFromSample(speed, agentJobToSpeedSample(first));
    speed = updateDownloadSpeedFromSample(speed, agentJobToSpeedSample(second));
    const terminal = projectAgentJobTransfer(
      "succeeded",
      snapshot({
        stage: "succeeded",
        transfer: second.transfer,
      }),
      speed,
    );
    expect(terminal.speedLabel).toBeNull();
    expect(terminal.downloadLine).toBeNull();
    expect(terminal.percent).toBeNull();
  });
});
