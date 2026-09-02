import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import {
  AGENT_ACTION_CONTRACT_VERSION,
  AGENT_INSTALL_READINESS_CONTRACT_VERSION,
  type AgentActionJobSnapshot,
  type AgentActionJobStage,
  type AgentActionResult,
  type AgentInstallationInventory,
  type AgentInstallationTarget,
  type AgentInstallReadiness,
  type AgentInstallReadinessPort,
  type AgentInstallState,
  type AgentReasonCode,
} from "@/v2/shared/features/agent-install-readiness";
import {
  AGENT_LIFECYCLE_INCOMPLETE_COPY,
  AGENT_LIFECYCLE_SUCCEEDED_COPY,
  AGENT_LIFECYCLE_VENDOR_HANDOFF_COPY,
  AGENT_LIFECYCLE_TIMEOUT_COPY,
  deriveAgentLifecyclePrimaryAction,
  isTerminalAgentJobStage,
  jobStageCopy,
  reasonCopy,
  bindLiveInventoryTarget,
  useAgentLifecycleAction,
} from "@/v2/pages/agents/useAgentLifecycleAction";

function readiness(
  overrides: Partial<AgentInstallReadiness> = {},
): AgentInstallReadiness {
  return {
    contractVersion: AGENT_INSTALL_READINESS_CONTRACT_VERSION,
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
    contractVersion: AGENT_ACTION_CONTRACT_VERSION,
    jobId: "job-1",
    agentId: "qoderwork",
    action: "install",
    stage,
    cancellable:
      stage === "checking" || stage === "downloading" || stage === "staging",
    reasonCode: null,
    transfer: null,
    ...overrides,
  };
}

function actionResult(
  overrides: Partial<AgentActionResult> = {},
): AgentActionResult {
  return {
    contractVersion: AGENT_ACTION_CONTRACT_VERSION,
    agentId: "qoderwork",
    action: "install",
    jobId: "job-1",
    stage: "checking",
    reasonCode: null,
    ...overrides,
  };
}

function installationInventory(
  agentId: AgentInstallReadiness["agentId"] = "qoderwork",
): AgentInstallationInventory {
  return {
    contractVersion: 1,
    inventoryId: `i1:${"a".repeat(32)}`,
    agentId,
    state: "not_observed",
    candidates: [
      {
        candidateId: `c1:${"b".repeat(32)}`,
        candidateRevision: `r1:${"c".repeat(64)}`,
        agentId,
        scope: "current_user",
        owner: "unknown",
        packageKind: "unknown",
        localVersion: "1.0.0",
        launchEligible: true,
        installEligible: false,
        updateEligible: true,
        reasonCodes: [],
        evidenceCodes: ["path_lookup"],
        locationLabel: "测试目标",
      },
    ],
    freshDestinations: [
      {
        destinationId: `d1:${"b".repeat(32)}`,
        destinationRevision: `r1:${"c".repeat(64)}`,
        scope: "current_user",
        owner: "unknown",
        packageKind: "exe",
        requiresElevation: false,
        writable: true,
        eligible: true,
        reasonCodes: [],
        locationLabel: "测试目标",
      },
    ],
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
    getInventory: vi.fn(async (agentId) => installationInventory(agentId)),
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
        "qoderwork",
        readiness({
          installState: "not_installed",
          allowedActions: ["install"],
        }),
      ),
    ).toBe("install");
    expect(
      deriveAgentLifecyclePrimaryAction(
        "qoderwork",
        readiness({
          installState: "not_installed",
          allowedActions: ["launch"],
        }),
      ),
    ).toBeNull();
    expect(
      deriveAgentLifecyclePrimaryAction(
        "qoderwork",
        readiness({ installState: "unknown", allowedActions: ["install"] }),
      ),
    ).toBeNull();
    expect(
      deriveAgentLifecyclePrimaryAction(
        "qoderwork",
        readiness({
          installState: "unavailable",
          allowedActions: ["install"],
        }),
      ),
    ).toBeNull();
  });

  it("offers update only when the product allows it, installed, update_available, and backend allows it", () => {
    expect(
      deriveAgentLifecyclePrimaryAction(
        "opencode",
        readiness({
          agentId: "opencode",
          installState: "installed",
          updateState: "update_available",
          allowedActions: ["update", "launch"],
        }),
      ),
    ).toBe("update");
    expect(
      deriveAgentLifecyclePrimaryAction(
        "claude-code",
        readiness({
          agentId: "claude-code",
          installState: "installed_not_runnable",
          updateState: "update_available",
          allowedActions: ["update"],
        }),
      ),
    ).toBe("update");
    expect(
      deriveAgentLifecyclePrimaryAction(
        "opencode",
        readiness({
          agentId: "opencode",
          installState: "installed",
          updateState: "up_to_date",
          allowedActions: ["install", "launch"],
        }),
      ),
    ).toBeNull();
    expect(
      deriveAgentLifecyclePrimaryAction(
        "opencode",
        readiness({
          agentId: "opencode",
          installState: "installed",
          updateState: "update_available",
          allowedActions: ["launch"],
        }),
      ),
    ).toBeNull();
    expect(
      deriveAgentLifecyclePrimaryAction(
        "qoderwork",
        readiness({
          installState: "installed",
          updateState: "update_available",
          allowedActions: ["update", "launch"],
        }),
      ),
    ).toBeNull();
    expect(
      deriveAgentLifecyclePrimaryAction(
        "workbuddy",
        readiness({
          agentId: "workbuddy",
          installState: "installed",
          updateState: "update_available",
          allowedActions: ["update"],
        }),
      ),
    ).toBeNull();
  });
});

describe("macOS lifecycle state copy", () => {
  it("distinguishes staging, authorization, restored rollback, and unknown recovery", () => {
    expect(jobStageCopy("staging")).toBe("正在准备安装包");
    expect(reasonCopy("authorization_required")).toContain("不可用于一键安装");
    expect(reasonCopy("authorization_required")).toContain(
      "不会改装到其他目录",
    );
    expect(reasonCopy("rollback_restored")).toContain("已恢复之前的应用");
    expect(reasonCopy("recovery_required")).toContain("停止重试");
  });

  it("projects helper-specific reasons without paths or internal names", () => {
    const codes: AgentReasonCode[] = [
      "helper_not_packaged",
      "helper_signature_invalid",
      "helper_install_authorization_cancelled",
      "helper_install_failed",
      "helper_update_required",
      "helper_downgrade_rejected",
      "helper_protocol_incompatible",
      "helper_peer_rejected",
      "operation_authorization_cancelled",
      "operation_authorization_invalid",
      "source_capability_invalid",
      "source_changed",
      "target_slot_invalid",
      "helper_removal_failed",
    ];
    for (const code of codes) {
      const copy = reasonCopy(code);
      expect(copy).toBeTruthy();
      expect(copy).not.toMatch(/\/Applications/u);
      expect(copy).not.toContain("~/");
      expect(copy?.toLowerCase()).not.toContain("helper");
      expect(copy).not.toContain("SMJobBless");
      expect(copy).not.toContain("XPC");
      expect(copy).not.toContain("com.fyagent.desktop.system-commit-helper");
    }
    expect(reasonCopy("helper_not_packaged")).toContain("系统文件夹");
    expect(reasonCopy("helper_not_packaged")).toContain("不会改到其他目录");
    expect(reasonCopy("operation_authorization_cancelled")).toContain(
      "取消了管理员授权",
    );
    expect(reasonCopy("authorization_required")).toContain(
      "系统应用程序文件夹",
    );
    expect(reasonCopy("action_not_supported")).toBe("当前产品不支持此操作。");
    expect(reasonCopy("action_not_supported")).not.toMatch(/https?:\/\//u);
    expect(reasonCopy("action_not_supported")).not.toContain("~/");
  });
});

describe("Windows external-installer state copy", () => {
  it("distinguishes launch, user interaction, incomplete observation, and terminal reasons", () => {
    expect(jobStageCopy("launching_installer")).toContain("官方安装窗口");
    expect(jobStageCopy("awaiting_user")).toContain(
      "请在弹出的官方安装窗口中完成安装",
    );
    expect(jobStageCopy("incomplete")).toBe("安装结果待检查");
    expect(reasonCopy("installer_user_cancelled")).toContain("取消");
    expect(reasonCopy("installer_artifact_unavailable")).toContain("磁盘空间");
    expect(reasonCopy("installer_process_unobservable")).toContain(
      "无法读取其进度",
    );
    expect(reasonCopy("installer_timed_out")).toContain("完成或关闭向导");
    expect(reasonCopy("installer_exited_nonzero")).toContain("未能完成");
    expect(isTerminalAgentJobStage("incomplete")).toBe(true);
    expect(isTerminalAgentJobStage("awaiting_user")).toBe(false);
  });
});

describe("bindLiveInventoryTarget", () => {
  it("prefers the unique eligible destination over a stale triplet", () => {
    const stale = lifecycleTarget();
    const live = installationInventory("qoderwork");
    live.inventoryId = `i1:${"f".repeat(32)}`;
    live.freshDestinations[0] = {
      ...live.freshDestinations[0],
      destinationId: `d1:${"e".repeat(32)}`,
      destinationRevision: `r1:${"d".repeat(64)}`,
    };
    expect(bindLiveInventoryTarget(live, "install", stale)).toEqual({
      kind: "fresh_destination",
      inventoryId: `i1:${"f".repeat(32)}`,
      targetId: `d1:${"e".repeat(32)}`,
      expectedTargetRevision: `r1:${"d".repeat(64)}`,
      label: "测试目标",
      scope: "current_user",
      eligibleActions: ["install"],
      reasonCodes: [],
    });
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

  it("uses vendor-window copy after launching the official installer", async () => {
    const stages: AgentActionJobStage[] = [
      "launching_installer",
      "awaiting_user",
      "succeeded",
    ];
    const after = readiness();
    const port = createPort({
      get: vi.fn(async () => after),
      startAction: vi.fn(async () =>
        actionResult({ stage: "launching_installer" }),
      ),
      getActionJob: vi.fn(async () =>
        jobSnapshot(stages.shift() ?? "succeeded"),
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

    expect(result.current.success).toBe(AGENT_LIFECYCLE_VENDOR_HANDOFF_COPY);
    expect(result.current.error).toBeNull();
    expect(result.current.busy).toBe(false);
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

    expect(result.current.error).toBe("另一个安装任务正在进行，请完成后再试。");
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
      agentId: "opencode",
      installState: "installed",
      updateState: "update_available",
      allowedActions: ["update"],
    });
    const port = createPort({
      get: vi.fn(async () => installed),
      startAction: vi.fn(async () =>
        actionResult({
          agentId: "opencode",
          action: "update",
          jobId: null,
          stage: "succeeded",
        }),
      ),
    });
    const { result } = renderHook(() =>
      useAgentLifecycleAction({
        agentId: "opencode",
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
      agentId: "opencode",
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

  it("keeps percent null through generic job stages without transfer telemetry", async () => {
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

  it("projects one-decimal download progress from job transfer telemetry", async () => {
    let release = false;
    const downloading = jobSnapshot("downloading", {
      transfer: {
        phase: "download",
        completedBytes: 3744,
        totalBytes: 10_000,
        attempt: 1,
        maxAttempts: 3,
        sequence: 1,
        observedAt: "2026-08-14T00:00:01.000Z",
      },
    });
    const port = createPort({
      get: vi.fn(async () =>
        readiness({
          installState: "installed",
          updateState: "up_to_date",
          allowedActions: ["launch"],
        }),
      ),
      startAction: vi.fn(async () => actionResult()),
      getActionJob: vi.fn(async () =>
        release ? jobSnapshot("succeeded") : downloading,
      ),
    });
    const { result } = renderHook(() =>
      useAgentLifecycleAction({
        agentId: "qoderwork",
        port,
        readiness: readiness({
          allowedActions: ["install"],
        }),
        target: lifecycleTarget(),
        pollIntervalMs: 5,
        maxPolls: 20,
      }),
    );

    let finished: Promise<void> | undefined;
    act(() => {
      finished = result.current.run("install");
    });
    await waitFor(() => {
      expect(result.current.stage).toBe("downloading");
      expect(result.current.percent).toBeCloseTo(37.44, 5);
      expect(result.current.progressLabel).toBe("下载中 37.4%");
    });
    release = true;
    await act(async () => {
      await finished;
    });
    expect(result.current.percent).toBeNull();
    expect(result.current.progressLabel).toBeNull();
    expect(result.current.success).toBe(AGENT_LIFECYCLE_SUCCEEDED_COPY);
  });

  it("uses OpenCode desktop allowedActions without a dual CLI surface", async () => {
    const desktopRelease = `v1:${"d".repeat(64)}`;
    const current = readiness({
      agentId: "opencode",
      sourceKind: "managed_desktop",
      authOwnership: "provider_owned",
      allowedActions: ["launch"],
      releaseId: desktopRelease,
      installState: "installed",
      inventoryState: "single",
      updateState: "up_to_date",
      localVersion: "1.18.19",
      remoteVersion: "1.18.19",
    });
    const port = createPort({
      get: vi.fn(async () => current),
      getInventory: vi.fn(async () => ({
        ...installationInventory(),
        agentId: "opencode" as const,
      })),
      startAction: vi.fn(async () =>
        actionResult({
          agentId: "opencode",
          action: "launch",
          jobId: null,
          stage: "succeeded",
        }),
      ),
    });
    const { result } = renderHook(() =>
      useAgentLifecycleAction({
        agentId: "opencode",
        port,
        readiness: current,
      }),
    );

    await act(async () => {
      await result.current.run("launch");
    });

    expect(port.startAction).toHaveBeenCalledWith({
      agentId: "opencode",
      action: "launch",
      expectedReleaseId: desktopRelease,
    });
    expect(port.startAction).not.toHaveBeenCalledWith(
      expect.objectContaining({ surface: "cli" }),
    );
    expect(port.getInventory).toHaveBeenCalledWith("opencode", undefined);
    expect(result.current.success).toBe(AGENT_LIFECYCLE_SUCCEEDED_COPY);
  });

  it("binds a live CLI destination before install", async () => {
    const current = readiness({
      agentId: "grokbuild",
      sourceKind: "cli_tooling",
      allowedActions: ["install"],
    });
    const port = createPort({
      get: vi.fn(async () => current),
      startAction: vi.fn(async () =>
        actionResult({
          agentId: "grokbuild",
          action: "install",
          jobId: null,
          stage: "succeeded",
        }),
      ),
    });
    const { result } = renderHook(() =>
      useAgentLifecycleAction({
        agentId: "grokbuild",
        port,
        readiness: current,
      }),
    );

    await act(async () => {
      await result.current.run("install");
    });

    expect(port.getInventory).toHaveBeenCalledWith("grokbuild", undefined);
    expect(port.startAction).toHaveBeenCalledWith({
      agentId: "grokbuild",
      action: "install",
      expectedReleaseId: current.releaseId,
      inventoryId: `i1:${"a".repeat(32)}`,
      targetId: `d1:${"b".repeat(32)}`,
      expectedTargetRevision: `r1:${"c".repeat(64)}`,
    });
    expect(result.current.success).toBe(AGENT_LIFECYCLE_SUCCEEDED_COPY);
  });

  it("replaces a stale inventory triplet before startAction", async () => {
    const live = installationInventory("qoderwork");
    live.inventoryId = `i1:${"f".repeat(32)}`;
    live.freshDestinations[0] = {
      ...live.freshDestinations[0],
      destinationId: `d1:${"e".repeat(32)}`,
      destinationRevision: `r1:${"d".repeat(64)}`,
    };
    const port = createPort({
      get: vi.fn(async () => readiness()),
      getInventory: vi.fn(async () => live),
      startAction: vi.fn(async () =>
        actionResult({ jobId: null, stage: "succeeded" }),
      ),
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

    expect(port.startAction).toHaveBeenCalledWith({
      agentId: "qoderwork",
      action: "install",
      expectedReleaseId: readiness().releaseId,
      inventoryId: `i1:${"f".repeat(32)}`,
      targetId: `d1:${"e".repeat(32)}`,
      expectedTargetRevision: `r1:${"d".repeat(64)}`,
    });
    expect(result.current.success).toBe(AGENT_LIFECYCLE_SUCCEEDED_COPY);
  });

  it("does not invent percent when Content-Length is unknown", async () => {
    const port = createPort({
      startAction: vi.fn(async () => actionResult()),
      getActionJob: vi.fn(async () =>
        jobSnapshot("downloading", {
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
      ),
    });
    const { result } = renderHook(() =>
      useAgentLifecycleAction({
        agentId: "qoderwork",
        port,
        readiness: readiness(),
        target: lifecycleTarget(),
        pollIntervalMs: 5,
        maxPolls: 20,
      }),
    );

    act(() => {
      void result.current.run("install");
    });
    await waitFor(() => {
      expect(result.current.stage).toBe("downloading");
      expect(result.current.progressLabel).toBe("已下载 126 MB");
      expect(result.current.percent).toBeNull();
    });
  });
});
