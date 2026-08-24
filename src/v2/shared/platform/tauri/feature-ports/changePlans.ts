import { invoke } from "@tauri-apps/api/core";

import {
  parseApplyChangePlanOutcome,
  parseCancelChangeJobOutcome,
  parseChangeJobSnapshot,
  parseChangePlan,
  parseRecoverableChangeJobs,
  type ChangePlansPort,
} from "../../../features/change-plans";

const OPAQUE_ID = /^[A-Za-z0-9_][A-Za-z0-9_.-]{0,127}$/u;
const DIGEST = /^[0-9a-f]{64}$/u;

function assertOpaqueId(value: string, label: string): string {
  if (!OPAQUE_ID.test(value)) throw new Error(`${label} is invalid`);
  return value;
}

export function createChangePlansPort(): ChangePlansPort {
  return {
    createCodexProviderSwitchPlan: async (targetProviderId) =>
      parseChangePlan(
        await invoke<unknown>("create_codex_provider_switch_plan", {
          targetProviderId: assertOpaqueId(
            targetProviderId,
            "Change Plan target",
          ),
        }),
      ),
    applyChangePlan: async (input) => {
      if (
        typeof input !== "object" ||
        input === null ||
        Array.isArray(input) ||
        Object.keys(input).sort().join(",") !== "planDigest,planId" ||
        !DIGEST.test(input.planDigest)
      ) {
        throw new Error("Change Plan Apply request is invalid");
      }
      return parseApplyChangePlanOutcome(
        await invoke<unknown>("apply_change_plan", {
          planId: assertOpaqueId(input.planId, "Change Plan ID"),
          planDigest: input.planDigest,
        }),
      );
    },
    cancelChangeJob: async (jobId) =>
      parseCancelChangeJobOutcome(
        await invoke<unknown>("cancel_change_job", {
          jobId: assertOpaqueId(jobId, "Change Job ID"),
        }),
      ),
    getChangeJob: async (jobId) =>
      parseChangeJobSnapshot(
        await invoke<unknown>("get_change_job", {
          jobId: assertOpaqueId(jobId, "Change Job ID"),
        }),
      ),
    listRecoverableChangeJobs: async () =>
      parseRecoverableChangeJobs(
        await invoke<unknown>("list_recoverable_change_jobs"),
      ),
  };
}
