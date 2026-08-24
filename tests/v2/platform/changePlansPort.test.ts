import { beforeEach, describe, expect, it, vi } from "vitest";

import { changeJobWire, changePlanWire, changePlanWorkBuddyWire } from "../fixtures/changePlans";
import { createChangePlansPort } from "@/v2/shared/platform/tauri/feature-ports/changePlans";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

describe("Tauri Change Plans port", () => {
  beforeEach(() => invoke.mockReset());

  it("uses the seven native commands and bounded payloads", async () => {
    invoke
      .mockResolvedValueOnce(changePlanWire)
      .mockResolvedValueOnce(changePlanWire)
      .mockResolvedValueOnce(changePlanWorkBuddyWire)
      .mockResolvedValueOnce({ kind: "admitted", job: changeJobWire })
      .mockResolvedValueOnce({ accepted: false, code: "commit_point_passed", jobId: "job-1" })
      .mockResolvedValueOnce(changeJobWire)
      .mockResolvedValueOnce([changeJobWire]);
    const port = createChangePlansPort();

    await port.createCodexProviderSwitchPlan("provider-1");
    await port.createCodexProviderUpsertPlan({
      name: "Gateway",
      baseUrl: "https://codex.example/v1",
      apiKey: "secret",
      modelId: "gpt-5",
    });
    await port.createWorkBuddySavePlan({
      baseUrl: "https://api.example.test/v1",
      apiKey: "secret",
      allowNoApiKey: false,
      selectedModelIds: ["model-a"],
      manualModelIds: [],
      removedModelIds: [],
      clearExistingApiKeys: false,
      expectedRevision: null,
    });
    await port.applyChangePlan({
      planId: "plan-1",
      planDigest: "a".repeat(64),
    });
    await port.cancelChangeJob("job-1");
    await port.getChangeJob("job-1");
    await port.listRecoverableChangeJobs();

    expect(invoke.mock.calls).toEqual([
      ["create_codex_provider_switch_plan", { targetProviderId: "provider-1" }],
      [
        "create_codex_provider_upsert_plan",
        {
          request: {
            name: "Gateway",
            baseUrl: "https://codex.example/v1",
            apiKey: "secret",
            modelId: "gpt-5",
          },
        },
      ],
      [
        "create_workbuddy_save_plan",
        {
          request: {
            baseUrl: "https://api.example.test/v1",
            apiKey: "secret",
            allowNoApiKey: false,
            selectedModelIds: ["model-a"],
            manualModelIds: [],
            removedModelIds: [],
            clearExistingApiKeys: false,
            expectedRevision: null,
          },
        },
      ],
      ["apply_change_plan", { planId: "plan-1", planDigest: "a".repeat(64) }],
      ["cancel_change_job", { jobId: "job-1" }],
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

    await expect(
      port.createWorkBuddySavePlan({
        baseUrl: "https://api.example.test/v1",
        apiKey: "secret",
        allowNoApiKey: false,
        selectedModelIds: ["model-a"],
        manualModelIds: [],
        overwriteToken: "leaked",
        clearExistingApiKeys: false,
        expectedRevision: null,
      } as never),
    ).rejects.toThrow("Change Plan WorkBuddy save request is invalid");
  });
});
