import { beforeEach, describe, expect, it, vi } from "vitest";

import fixtureJson from "../../fixtures/changePlanDtoContract.v1.json";
import {
  createBrowserFeaturePorts,
  NATIVE_ONLY_ERROR,
} from "@/v2/shared/platform/browser/features";

const invoke = vi.fn();
const listen = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen }));

describe("V2 Change Plan port", () => {
  beforeEach(() => {
    invoke.mockReset();
    listen.mockReset();
  });

  it("parses the frozen plan, job, event, cancellation, and recovery commands", async () => {
    const unlisten = vi.fn();
    let eventHandler: ((event: { payload: unknown }) => void) | undefined;
    listen.mockImplementation(async (_name, handler) => {
      eventHandler = handler;
      return unlisten;
    });
    invoke.mockImplementation(async (command: string) => {
      switch (command) {
        case "create_codex_provider_switch_plan":
          return structuredClone(fixtureJson.plan);
        case "apply_change_plan":
          return structuredClone(fixtureJson.applyOutcome);
        case "get_change_job":
          return structuredClone(fixtureJson.applyOutcome.job);
        case "list_recoverable_change_jobs":
          return [structuredClone(fixtureJson.applyOutcome.job)];
        case "cancel_change_job":
          return structuredClone(fixtureJson.cancelOutcome);
        default:
          throw new Error(`unexpected command ${command}`);
      }
    });

    const { createChangePlanPort } = await import(
      "@/v2/shared/platform/tauri/feature-ports/changePlan"
    );
    const port = createChangePlanPort();
    await expect(
      port.createCodexProviderSwitchPlan("provider-target"),
    ).resolves.toMatchObject({
      targetProviderName: "Target Provider",
      adapter: {
        phases: [
          "precheck",
          "snapshot",
          "managed_write",
          "readback",
          "finalize",
        ],
      },
    });
    await expect(port.apply("plan-contract", "plan-digest")).resolves.toEqual(
      fixtureJson.applyOutcome,
    );
    await expect(port.getJob("job-contract")).resolves.toEqual(
      fixtureJson.applyOutcome.job,
    );
    await expect(port.listRecoverableJobs()).resolves.toEqual([
      fixtureJson.applyOutcome.job,
    ]);
    await expect(port.cancelJob("job-contract")).resolves.toEqual(
      fixtureJson.cancelOutcome,
    );

    const onEvent = vi.fn();
    await expect(port.subscribeJobUpdates(onEvent)).resolves.toBe(unlisten);
    expect(listen).toHaveBeenCalledWith(
      "change-job://updated",
      expect.any(Function),
    );
    eventHandler?.({ payload: fixtureJson.event });
    expect(onEvent).toHaveBeenCalledWith(fixtureJson.event);

    expect(invoke).toHaveBeenCalledWith("create_codex_provider_switch_plan", {
      targetProviderId: "provider-target",
    });
    expect(invoke).toHaveBeenCalledWith("apply_change_plan", {
      planId: "plan-contract",
      planDigest: "plan-digest",
    });
  });

  it("fails closed on excess wire fields and ignores malformed event hints", async () => {
    let eventHandler: ((event: { payload: unknown }) => void) | undefined;
    listen.mockImplementation(async (_name, handler) => {
      eventHandler = handler;
      return vi.fn();
    });
    const { createChangePlanPort } = await import(
      "@/v2/shared/platform/tauri/feature-ports/changePlan"
    );
    const port = createChangePlanPort();
    invoke.mockResolvedValueOnce({
      ...structuredClone(fixtureJson.plan),
      secret: "SECRET-CANARY-MUST-NOT-PARSE",
    });
    await expect(
      port.createCodexProviderSwitchPlan("provider-target"),
    ).rejects.toThrow("Change Plan is unavailable");

    invoke.mockResolvedValueOnce({
      ...structuredClone(fixtureJson.applyOutcome.job),
      planDigest: "must-not-cross-job-wire",
    });
    await expect(port.getJob("job-contract")).rejects.toThrow(
      "Change job is unavailable",
    );

    const onEvent = vi.fn();
    await port.subscribeJobUpdates(onEvent);
    expect(() =>
      eventHandler?.({
        payload: { ...fixtureJson.event, extra: "not-allowed" },
      }),
    ).not.toThrow();
    expect(onEvent).not.toHaveBeenCalled();
  });

  it("keeps every browser Change Plan operation native-only", async () => {
    const port = createBrowserFeaturePorts().changePlan;
    await expect(
      port.createCodexProviderSwitchPlan("provider-target"),
    ).rejects.toThrow(NATIVE_ONLY_ERROR);
    await expect(port.apply("plan-contract", "plan-digest")).rejects.toThrow(
      NATIVE_ONLY_ERROR,
    );
    await expect(port.getJob("job-contract")).rejects.toThrow(
      NATIVE_ONLY_ERROR,
    );
    await expect(port.listRecoverableJobs()).rejects.toThrow(NATIVE_ONLY_ERROR);
    await expect(port.cancelJob("job-contract")).rejects.toThrow(
      NATIVE_ONLY_ERROR,
    );
    await expect(port.subscribeJobUpdates(vi.fn())).rejects.toThrow(
      NATIVE_ONLY_ERROR,
    );
  });
});
