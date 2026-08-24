import { invoke } from "@tauri-apps/api/core";

import type {
  ProviderQuickSetupRequest,
  WorkBuddySaveModelsRequest,
} from "../../../features/models";
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

function assertUpsertRequest(
  request: ProviderQuickSetupRequest,
): ProviderQuickSetupRequest {
  if (
    typeof request !== "object" ||
    request === null ||
    Array.isArray(request)
  ) {
    throw new Error("Change Plan upsert request is invalid");
  }
  const keys = Object.keys(request).sort();
  const allowed = new Set([
    "apiKey",
    "baseUrl",
    "codexFeatures",
    "modelId",
    "name",
  ]);
  if (
    keys.some((key) => !allowed.has(key)) ||
    typeof request.name !== "string" ||
    !request.name ||
    typeof request.baseUrl !== "string" ||
    !request.baseUrl ||
    typeof request.apiKey !== "string" ||
    !request.apiKey ||
    typeof request.modelId !== "string" ||
    !request.modelId
  ) {
    throw new Error("Change Plan upsert request is invalid");
  }
  return request;
}

function assertWorkBuddySaveRequest(
  request: WorkBuddySaveModelsRequest,
): WorkBuddySaveModelsRequest {
  if (
    typeof request !== "object" ||
    request === null ||
    Array.isArray(request)
  ) {
    throw new Error("Change Plan WorkBuddy save request is invalid");
  }
  const keys = Object.keys(request);
  const allowed = new Set([
    "allowNoApiKey",
    "apiKey",
    "baseUrl",
    "clearExistingApiKeys",
    "expectedRevision",
    "manualModelIds",
    "removedModelIds",
    "selectedModelIds",
  ]);
  if (
    keys.some((key) => !allowed.has(key)) ||
    typeof request.baseUrl !== "string" ||
    !request.baseUrl ||
    typeof request.apiKey !== "string" ||
    typeof request.allowNoApiKey !== "boolean" ||
    !Array.isArray(request.selectedModelIds) ||
    !Array.isArray(request.manualModelIds) ||
    (request.removedModelIds !== undefined &&
      !Array.isArray(request.removedModelIds)) ||
    typeof request.clearExistingApiKeys !== "boolean" ||
    !(
      request.expectedRevision === null ||
      typeof request.expectedRevision === "string"
    )
  ) {
    throw new Error("Change Plan WorkBuddy save request is invalid");
  }
  return request;
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
    createCodexProviderUpsertPlan: async (request) =>
      parseChangePlan(
        await invoke<unknown>("create_codex_provider_upsert_plan", {
          request: assertUpsertRequest(request),
        }),
      ),
    createWorkBuddySavePlan: async (request) =>
      parseChangePlan(
        await invoke<unknown>("create_workbuddy_save_plan", {
          request: assertWorkBuddySaveRequest(request),
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
