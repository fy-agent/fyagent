import { beforeEach, describe, expect, it, vi } from "vitest";
import { changeJobSnapshotSchema, changePlanApi } from "@/lib/api/change-plan";
import {
  CHANGE_JOB_POLL_INTERVAL_MS,
  changeJobRefetchInterval,
  shouldInvalidateChangeJobEvent,
} from "@/lib/query/change-plan";

const invokeMock = vi.hoisted(() => vi.fn());

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(vi.fn()),
}));

export const changeJobFixture = {
  jobId: "job-1",
  planId: "plan-1",
  targetProviderId: "provider-2",
  revision: 3,
  eventSeq: 3,
  status: "succeeded",
  resultCode: "applied",
  steps: [
    { kind: "precheck", status: "succeeded", code: "baseline_matched" },
    { kind: "apply", status: "succeeded", code: "writer_returned" },
    { kind: "readback", status: "succeeded", code: "target_matched" },
    { kind: "reconcile", status: "pending", code: "pending" },
  ],
  resources: [
    { kind: "provider_db_current", status: "matched", code: "target_current" },
    { kind: "device_current", status: "matched", code: "target_current" },
    {
      kind: "target_definition",
      status: "matched",
      code: "definition_matched",
    },
    { kind: "codex_live_projection", status: "matched", code: "live_matched" },
  ],
  restartRequirement: "not_required",
  usageEvidence: "not_observed",
  recoveryState: "not_needed",
  diagnosticCode: "target_readback_matched",
  liveConfigChanged: false,
  createdAt: 100,
  updatedAt: 101,
} as const;

beforeEach(() => invokeMock.mockReset());

describe("change-plan API and decoder", () => {
  it("uses the fixed command surface and decodes the safe projection", async () => {
    const plan = {
      planId: "plan-1",
      operation: "codex_provider_switch",
      targetProviderId: "provider-2",
      targetProviderName: "Provider 2",
      planDigest: "digest",
      baselineDigest: "baseline",
      createdAt: 100,
      expiresAt: 1000,
      status: "ready",
      currentProviderCode: "current_configured",
      targetProviderCode: "existing_provider",
      restartExpectation: "recommended",
      risks: [{ code: "local_configuration_write", severity: "notice" }],
      evidenceNote: "usage_not_observed",
    };
    invokeMock.mockResolvedValueOnce(plan);
    await expect(
      changePlanApi.createCodexProviderSwitchPlan("provider-2"),
    ).resolves.toEqual(plan);
    expect(invokeMock).toHaveBeenCalledWith(
      "create_codex_provider_switch_plan",
      { targetProviderId: "provider-2" },
    );

    invokeMock.mockResolvedValueOnce({
      kind: "admitted",
      job: changeJobFixture,
    });
    await changePlanApi.apply("plan-1", "digest");
    expect(invokeMock).toHaveBeenLastCalledWith("apply_change_plan", {
      planId: "plan-1",
      planDigest: "digest",
    });
  });

  it("fails closed on missing fields and renders future codes as unknown", () => {
    expect(() => changeJobSnapshotSchema.parse({ jobId: "only" })).toThrow();
    const parsed = changeJobSnapshotSchema.parse({
      ...changeJobFixture,
      resultCode: "future_result_code",
      resources: [
        { kind: "future_resource", status: "future_status", code: "safe" },
      ],
    });
    expect(parsed.resultCode).toBe("unknown");
    expect(parsed.resources[0]).toMatchObject({
      kind: "unknown",
      status: "unknown",
    });
  });

  it("polls nonterminal jobs, stops on terminal snapshots, and deduplicates events", () => {
    expect(
      changeJobRefetchInterval({
        state: { data: { ...changeJobFixture, status: "running" } },
      } as never),
    ).toBe(CHANGE_JOB_POLL_INTERVAL_MS);
    expect(
      changeJobRefetchInterval({ state: { data: changeJobFixture } } as never),
    ).toBe(false);
    expect(
      changeJobRefetchInterval({
        state: { data: { ...changeJobFixture, status: "unknown" } },
      } as never),
    ).toBe(false);
    expect(
      shouldInvalidateChangeJobEvent(
        { jobId: "job-1", eventSeq: 4 },
        "job-1",
        3,
      ),
    ).toBe(true);
    expect(
      shouldInvalidateChangeJobEvent(
        { jobId: "job-1", eventSeq: 3 },
        "job-1",
        3,
      ),
    ).toBe(false);
    expect(
      shouldInvalidateChangeJobEvent(
        { jobId: "other", eventSeq: 9 },
        "job-1",
        3,
      ),
    ).toBe(false);
  });
});
