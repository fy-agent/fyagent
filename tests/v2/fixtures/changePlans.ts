import fixture from "../../fixtures/changePlanDtoContract.v2.json";

import type {
  CancelChangeJobOutcome,
  ChangeJobSnapshot,
  ChangePlan,
} from "@/v2/shared/features/change-plans";

export const changePlanWire = fixture.plan as ChangePlan;
export const changeJobWire = fixture.job as ChangeJobSnapshot;
export const cancelChangeJobWire = fixture.cancelOutcome as CancelChangeJobOutcome;

export const changePlanUpsertWire: ChangePlan = {
  ...changePlanWire,
  operation: "codex_provider_upsert_and_switch",
  targetProviderId: "fyagent-v2-quick-setup-codex",
  targetProviderName: "FyAgent Codex",
  targetProviderCode: "quick_setup_create",
  adapter: {
    ...changePlanWire.adapter,
    adapterId: "codex_provider_upsert_and_switch",
    operationType: "codex_provider_upsert_and_switch",
  },
  risks: [
    { code: "local_configuration_write", severity: "notice" },
    { code: "save_provider_then_set_current", severity: "notice" },
  ],
};
