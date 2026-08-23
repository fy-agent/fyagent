import { beforeEach, describe, expect, it, vi } from "vitest";

import { changeJobWire, changePlanWire } from "../fixtures/changePlans";
import { createChangePlansPort } from "@/v2/shared/platform/tauri/feature-ports/changePlans";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

describe("Tauri Change Plans port", () => {
  beforeEach(() => invoke.mockReset());

  it("uses exactly the four native commands and bounded payloads", async () => {
    invoke
      .mockResolvedValueOnce(changePlanWire)
      .mockResolvedValueOnce({ kind: "admitted", job: changeJobWire })
      .mockResolvedValueOnce(changeJobWire)
      .mockResolvedValueOnce([changeJobWire]);
    const port = createChangePlansPort();

    await port.createCodexProviderSwitchPlan("provider-1");
    await port.applyChangePlan({
      planId: "plan-1",
      planDigest: "a".repeat(64),
    });
    await port.getChangeJob("job-1");
    await port.listRecoverableChangeJobs();

    expect(invoke.mock.calls).toEqual([
      ["create_codex_provider_switch_plan", { targetProviderId: "provider-1" }],
      ["apply_change_plan", { planId: "plan-1", planDigest: "a".repeat(64) }],
      ["get_change_job", { jobId: "job-1" }],
      ["list_recoverable_change_jobs"],
    ]);
  });

  it("rejects invalid requests and excess responses before product use", async () => {
    const port = createChangePlansPort();
    await expect(
      port.createCodexProviderSwitchPlan("../provider"),
    ).rejects.toThrow("Change Plan target is invalid");
    expect(invoke).not.toHaveBeenCalled();

    invoke.mockResolvedValue({ ...changePlanWire, apiKey: "sentinel" });
    await expect(
      port.createCodexProviderSwitchPlan("provider-1"),
    ).rejects.toThrow("Change Plan is unavailable");
  });
});
