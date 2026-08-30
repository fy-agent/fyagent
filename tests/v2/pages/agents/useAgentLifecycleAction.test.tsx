import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type {
  AgentActionJobSnapshot,
  AgentActionJobStage,
  AgentActionResult,
  AgentInstallationInventory,
  AgentInstallationTarget,
  AgentInstallReadiness,
  AgentInstallReadinessPort,
  AgentInstallState,
  AgentReasonCode,
} from "@/v2/shared/features/agent-install-readiness";
import {
  AGENT_LIFECYCLE_INCOMPLETE_COPY,
  AGENT_LIFECYCLE_SUCCEEDED_COPY,
  AGENT_LIFECYCLE_TIMEOUT_COPY,
  deriveAgentLifecyclePrimaryAction,
  isTerminalAgentJobStage,
  jobStageCopy,
  reasonCopy,
  useAgentLifecycleAction,
} from "@/v2/pages/agents/useAgentLifecycleAction";

function readiness(
  overrides: Partial<AgentInstallReadiness> = {},
): AgentInstallReadiness {
  return {
    contractVersion: 3,
    agentId: "qoderwork",
    reviewedAt: "2026-08-29",
    installState: "not_installed",
    inventoryState: "not_observed",
    requiresTargetSelection: false,
    updateState: "latest_unknown",
    releaseId: `v1:${"a".repeat(64)}`,
    localVersion: null,
    remoteVersion: null,
    authOwnership: "agent_owned",
    authState: "unknown",
    sourceKind: "managed_desktop",
    allowedActions: ["install"],
    reasonCodes: ["auth_state_unknown"],
    ...overrides,
  };
}

function jobSnapshot(
  stage: AgentActionJobStage,
  overrides: Partial<AgentActionJobSnapshot> = {},
): AgentActionJobSnapshot {
  return {
    contractVersion: 2,
    jobId: "job-1",
    agentId: "qoderwork",
    action: "install",
    stage,
    cancellable:
      stage === "checking" || stage === "downloading" || stage === "staging",
    reasonCode: null,
    ...overrides,
  };
}

function actionResult(
  overrides: Partial<AgentActionResult> = {},
): AgentActionResult {
  return {
    contractVersion: 2,
    agentId: "qoderwork",
    action: "install",
    jobId: "job-1",
    stage: "checking",
    reasonCode: null,
    ...overrides,
  };
}

function installationInventory(): AgentInstallationInventory {
  return {
    contractVersion: 1,
    inventoryId: `i1:${"a".repeat(32)}`,
    agentId: "qoderwork",
    state: "not_observed",
    candidates: [],
    freshDestinations: [],
    reasonCodes: [],
  };
}

function lifecycleTarget(
  action: "install" | "update" = "install",
): AgentInstallationTarget {
  return {
    kind: action === "install" ? "fresh_destination" : "candidate",
    inventoryId: `i1:${"a".repeat(32)}`,
    targetId: `${action === "install" ? "d1" : "c1"}:${"b".repeat(32)}`,
    expectedTargetRevision: `r1:${"c".repeat(64)}`,
    label: "测试目标",
    scope: "current_user",
    eligibleActions: [action],
    reasonCodes: [],
  };
}

function createPort(
  overrides: Partial<AgentInstallReadinessPort> = {},
): AgentInstallReadinessPort {
  return {
    get: vi.fn(async () => readiness()),
    getInventory: vi.fn(async () => installationInventory()),
    startAction: vi.fn(),
    cancelAction: vi.fn(),
    getActionJob: vi.fn(),
    ...overrides,
  };
}

describe("deriveAgentLifecyclePrimaryAction", () => {
  it("offers install only when scan confirmed not_installed and backend allows it", () => {
    expect(
      deriveAgentLifecyclePrimaryAction(
        readiness({
          installState: "not_installed",
          allowedActions: ["install"],
        }),
      ),
    ).toBe("install");
    expect(
      deriveAgentLifecyclePrimaryAction(
        readiness({
          installState: "not_installed",
          allowedActions: ["launch"],
        }),
      ),
    ).toBeNull();
    expect(
      deriveAgentLifecyclePrimaryAction(
        readiness({ installState: "unknown", allowedActions: ["install"] }),
      ),
    ).toBeNull();
    expect(
      deriveAgentLifecyclePrimaryAction(
        readiness({
          installState: "unavailable",
          allowedActions: ["install"],
        }),
      ),
    ).toBeNull();
  });

  it("offers update only when installed, update_available, and backend allows it", () => {
    expect(
      deriveAgentLifecyclePrimaryAction(
        readiness({
          installState: "installed",
          updateState: "update_available",
          allowedActions: ["update", "launch"],
        }),
      ),
    ).toBe("update");
    expect(
      deriveAgentLifecyclePrimaryAction(
        readiness({
          installState: "installed_not_runnable",
          updateState: "update_available",
          allowedActions: ["update"],
        }),
      ),
    ).toBe("update");
    expect(
      deriveAgentLifecyclePrimaryAction(
        readiness({
          installState: "installed",
          updateState: "up_to_date",
          allowedActions: ["install", "launch"],
        }),
      ),
    ).toBeNull();
    expect(
      deriveAgentLifecyclePrimaryAction(
        readiness({
          installState: "installed",
          updateState: "update_available",
          allowedActions: ["launch"],
        }),
      ),
    ).toBeNull();
  });
});

describe("macOS lifecycle state copy", () => {
  it("distinguishes staging, authorization, restored rollback, and unknown recovery", () => {
    expect(jobStageCopy("staging")).toBe("正在准备安装包");
    expect(reasonCopy("authorization_required")).toContain("管理员授权");
    expect(reasonCopy("authorization_required")).toContain("不会改装到其他目录");
    expect(reasonCopy("rollback_restored")).toContain("已恢复之前的应用");
    expect(reasonCopy("recovery_required")).toContain("停止重试");
  });
});

describe("Windows external-installer state copy", () => {
  it("distinguishes launch, user interaction, incomplete observation, and terminal reasons", () => {
    expect(jobStageCopy("launching_installer")).toContain("打开 Windows 安装向导");
    expect(jobStageCopy("awaiting_user")).toContain("请在 Windows 中完成安装");
    expect(jobStageCopy("incomplete")).toBe("安装结果待检查");
    expect(reasonCopy("installer_user_cancelled")).toContain("取消");
    expect(reasonCopy("installer_artifact_unavailable")).toContain("磁盘空间");
    expect(reasonCopy("installer_process_unobservable")).toContain("无法读取其进度");
    expect(reasonCopy("installer_timed_out")).toContain("完成或关闭向导");
    expect(reasonCopy("installer_exited_nonzero")).toContain("未能完成");
    expect(isTerminalAgentJobStage("incomplete")).toBe(true);
    expect(isTerminalAgentJobStage("awaiting_user")).toBe(false);
  });
});

describe("useAgentLifecycleAction", () => {
  it("shows real job stages and only applies the reread readiness", async () => {
    const stages: AgentActionJobStage[] = [
      "downloading",
      "installing",
      "succeeded",
    ];
    const after: AgentInstallReadiness = readiness({
      installState: "installed",
      updateState: "up_to_date",
      allowedActions: ["launch"],
    });
    const port = createPort({
      get: vi.fn(async () => after),
      startAction: vi.fn(async () => actionResult()),
      getActionJob: vi.fn(async () =>
        jobSnapshot(stages.shift() ?? "succeeded"),
      ),
    });
    const onReadinessChange = vi.fn();
    const { result } = renderHook(() =>
      useAgentLifecycleAction({
        agentId: "qoderwork",
        port,
        readiness: readiness(),
        target: lifecycleTarget(),
        onReadinessChange,
        pollIntervalMs: 5,
      }),
    );

    expect(result.current.primaryAction).toBe("install");
    expect(result.current.percent).toBeNull();

    await act(async () => {
      await result.current.run("install");
    });

    expect(port.startAction).toHaveBeenCalledWith({
      agentId: "qoderwork",
      action: "install",
      expectedReleaseId: readiness().releaseId,
      inventoryId: `i1:${"a".repeat(32)}`,
      targetId: `d1:${"b".repeat(32)}`,
      expectedTargetRevision: `r1:${"c".repeat(64)}`,
    });
    expect(port.getActionJob).toHaveBeenCalled();
    expect(port.get).toHaveBeenCalledWith("qoderwork");
    expect(onReadinessChange).toHaveBeenCalledWith(after);
    expect(result.current.success).toBe(AGENT_LIFECYCLE_SUCCEEDED_COPY);
    expect(result.current.busy).toBe(false);
    expect(result.current.stage).toBeNull();
    expect(result.current.percent).toBeNull();
    expect(result.current.primaryAction).toBe("install");
  });

  it("does not set an optimistic installed flag before authoritative get", async () => {
    const stillMissing = readiness({ installState: "not_installed" });
    const observed: AgentInstallState[] = [];
    const port = createPort({
      get: vi.fn(async () => stillMissing),
      startAction: vi.fn(async () =>
        actionResult({ jobId: "job-1", stage: "succeeded" }),
      ),
      getActionJob: vi.fn(async () => jobSnapshot("succeeded")),
    });
    const { result } = renderHook(() =>
      useAgentLifecycleAction({
        agentId: "qoderwork",
        port,
        readiness: readiness(),
        target: lifecycleTarget(),
        onReadinessChange: (data) => {
          observed.push(data.installState);
        },
        pollIntervalMs: 5,
      }),
    );

    await act(async () => {
      await result.current.run("install");
    });

    expect(observed).toEqual(["not_installed"]);
    expect(result.current.percent).toBeNull();
    expect(port.get).toHaveBeenCalled();
  });

  it("runs an immediate CLI action then rereads without polling a job", async () => {
    const after = readiness({
      installState: "installed",
      allowedActions: ["launch"],
    });
    const port = createPort({
      get: vi.fn(async () => after),
      startAction: vi.fn(async () =>
        actionResult({ jobId: null, stage: "succeeded", action: "install" }),
      ),
    });
    const { result } = renderHook(() =>
      useAgentLifecycleAction({
        agentId: "qoderwork",
        port,
        readiness: readiness(),
        target: lifecycleTarget(),
        pollIntervalMs: 5,
      }),
    );

    await act(async () => {
      await result.current.runPrimary();
    });

    expect(port.getActionJob).not.toHaveBeenCalled();
    expect(port.get).toHaveBeenCalledWith("qoderwork");
    expect(result.current.success).toBe(AGENT_LIFECYCLE_SUCCEEDED_COPY);
    expect(result.current.percent).toBeNull();
  });

  it("surfaces operation_conflict and still rereads", async () => {
    const port = createPort({
      get: vi.fn(async () => readiness()),
      startAction: vi.fn(async () => {
        const error = new Error("busy") as Error & {
          reasonCode: AgentReasonCode;
        };
        error.reasonCode = "operation_conflict";
        throw error;
      }),
    });
    const { result } = renderHook(() =>
      useAgentLifecycleAction({
        agentId: "qoderwork",
        port,
        readiness: readiness(),
        target: lifecycleTarget(),
      }),
    );

    await act(async () => {
      await result.current.run("install");
    });

    expect(result.current.error).toBe(
      "另一个安装任务正在进行，请完成后再试。",
    );
    expect(result.current.reasonCode).toBe("operation_conflict");
    expect(result.current.success).toBeNull();
    expect(port.get).toHaveBeenCalledWith("qoderwork");
    expect(result.current.canRetry).toBe(true);
  });

  it("surfaces refresh_required from a failed job snapshot", async () => {
    const port = createPort({
      get: vi.fn(async () => readiness()),
      startAction: vi.fn(async () => actionResult()),
      getActionJob: vi.fn(async () =>
        jobSnapshot("failed", {
          reasonCode: "refresh_required",
          cancellable: false,
        }),
      ),
    });
    const { result } = renderHook(() =>
      useAgentLifecycleAction({
        agentId: "qoderwork",
        port,
        readiness: readiness(),
        target: lifecycleTarget(),
        pollIntervalMs: 5,
      }),
    );

    await act(async () => {
      await result.current.run("install");
    });

    expect(result.current.error).toBe("下载信息已更新，请刷新后重试。");
    expect(result.current.reasonCode).toBe("refresh_required");
    expect(result.current.success).toBeNull();
    expect(port.get).toHaveBeenCalled();
  });

  it("times out without treating the job as success or failure", async () => {
    const port = createPort({
      get: vi.fn(async () => readiness()),
      startAction: vi.fn(async () => actionResult()),
      getActionJob: vi.fn(async () => jobSnapshot("downloading")),
    });
    const { result } = renderHook(() =>
      useAgentLifecycleAction({
        agentId: "qoderwork",
        port,
        readiness: readiness(),
        target: lifecycleTarget(),
        pollIntervalMs: 1,
        maxPolls: 1,
      }),
    );

    await act(async () => {
      await result.current.run("install");
    });

    expect(result.current.error).toBe(AGENT_LIFECYCLE_TIMEOUT_COPY);
    expect(result.current.success).toBeNull();
    expect(result.current.reasonCode).toBeNull();
    expect(port.get).toHaveBeenCalledWith("qoderwork");
    expect(result.current.percent).toBeNull();
  });

  it("does not claim success when the authoritative reread fails", async () => {
    const port = createPort({
      get: vi.fn(async () => {
        throw new Error("offline");
      }),
      startAction: vi.fn(async () =>
        actionResult({ jobId: null, stage: "succeeded" }),
      ),
    });
    const onReadinessChange = vi.fn();
    const { result } = renderHook(() =>
      useAgentLifecycleAction({
        agentId: "qoderwork",
        port,
        readiness: readiness(),
        target: lifecycleTarget(),
        onReadinessChange,
      }),
    );

    await act(async () => {
      await result.current.run("install");
    });

    expect(onReadinessChange).not.toHaveBeenCalled();
    expect(result.current.success).toBeNull();
    expect(result.current.error).toBe(AGENT_LIFECYCLE_INCOMPLETE_COPY);
  });

  it("ignores actions the backend omitted from allowedActions", async () => {
    const port = createPort();
    const { result } = renderHook(() =>
      useAgentLifecycleAction({
        agentId: "qoderwork",
        port,
        readiness: readiness({ allowedActions: ["launch"] }),
        target: lifecycleTarget(),
      }),
    );

    expect(result.current.primaryAction).toBeNull();
    await act(async () => {
      await result.current.run("install");
      await result.current.run("update");
    });
    expect(port.startAction).not.toHaveBeenCalled();
  });

  it("runs update when that is the derived primary action", async () => {
    const installed = readiness({
      installState: "installed",
      updateState: "update_available",
      allowedActions: ["update"],
    });
    const port = createPort({
      get: vi.fn(async () => installed),
      startAction: vi.fn(async () =>
        actionResult({ action: "update", jobId: null, stage: "succeeded" }),
      ),
    });
    const { result } = renderHook(() =>
      useAgentLifecycleAction({
        agentId: "qoderwork",
        port,
        readiness: installed,
        target: lifecycleTarget("update"),
      }),
    );

    expect(result.current.primaryAction).toBe("update");
    await act(async () => {
      await result.current.runPrimary();
    });
    expect(port.startAction).toHaveBeenCalledWith({
      agentId: "qoderwork",
      action: "update",
      expectedReleaseId: installed.releaseId,
      inventoryId: `i1:${"a".repeat(32)}`,
      targetId: `c1:${"b".repeat(32)}`,
      expectedTargetRevision: `r1:${"c".repeat(64)}`,
    });
  });

  it("retries the last allowed action after a failure", async () => {
    const startAction = vi
      .fn<AgentInstallReadinessPort["startAction"]>()
      .mockRejectedValueOnce(
        Object.assign(new Error("conflict"), {
          reasonCode: "operation_conflict",
        }),
      )
      .mockResolvedValueOnce(actionResult({ jobId: null, stage: "succeeded" }));
    const port = createPort({
      get: vi.fn(async () => readiness()),
      startAction,
    });
    const { result } = renderHook(() =>
      useAgentLifecycleAction({
        agentId: "qoderwork",
        port,
        readiness: readiness(),
        target: lifecycleTarget(),
      }),
    );

    await act(async () => {
      await result.current.run("install");
    });
    expect(result.current.canRetry).toBe(true);

    await act(async () => {
      await result.current.retry();
    });
    expect(startAction).toHaveBeenCalledTimes(2);
    expect(result.current.success).toBe(AGENT_LIFECYCLE_SUCCEEDED_COPY);
  });

  it("cancels a cancellable job and rereads after the poll observes it", async () => {
    let cancelled = false;
    const port = createPort({
      get: vi.fn(async () => readiness()),
      startAction: vi.fn(async () => actionResult()),
      getActionJob: vi.fn(async () =>
        cancelled
          ? jobSnapshot("cancelled", {
              reasonCode: "cancelled",
              cancellable: false,
            })
          : jobSnapshot("downloading"),
      ),
      cancelAction: vi.fn(async () => {
        cancelled = true;
        return jobSnapshot("cancelled", {
          reasonCode: "cancelled",
          cancellable: false,
        });
      }),
    });
    const { result } = renderHook(() =>
      useAgentLifecycleAction({
        agentId: "qoderwork",
        port,
        readiness: readiness(),
        target: lifecycleTarget(),
        pollIntervalMs: 10,
        maxPolls: 20,
      }),
    );

    let finished: Promise<void> | undefined;
    act(() => {
      finished = result.current.run("install");
    });

    await waitFor(() => {
      expect(result.current.stage).toBe("downloading");
      expect(result.current.canCancel).toBe(true);
      expect(result.current.percent).toBeNull();
    });

    await act(async () => {
      await result.current.cancel();
    });
    await act(async () => {
      await finished;
    });

    expect(port.cancelAction).toHaveBeenCalledWith("job-1");
    expect(result.current.error).toBe("操作已取消。");
    expect(result.current.reasonCode).toBe("cancelled");
    expect(result.current.success).toBeNull();
    expect(port.get).toHaveBeenCalled();
  });

  it("keeps percent null through every generic job stage", async () => {
    const seen: Array<number | null> = [];
    const stages: AgentActionJobStage[] = [
      "checking",
      "downloading",
      "staging",
      "installing",
      "verifying_installation",
      "succeeded",
    ];
    const port = createPort({
      get: vi.fn(async () => readiness()),
      startAction: vi.fn(async () => actionResult()),
      getActionJob: vi.fn(async () =>
        jobSnapshot(stages.shift() ?? "succeeded"),
      ),
    });
    const { result } = renderHook(() => {
      const view = useAgentLifecycleAction({
        agentId: "qoderwork",
        port,
        readiness: readiness(),
        target: lifecycleTarget(),
        pollIntervalMs: 5,
      });
      seen.push(view.percent);
      return view;
    });

    await act(async () => {
      await result.current.run("install");
    });

    expect(seen.length).toBeGreaterThan(1);
    expect(seen.every((value) => value === null)).toBe(true);
  });
});
