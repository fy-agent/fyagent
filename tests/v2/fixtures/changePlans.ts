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

export const changePlanWorkBuddyWire: ChangePlan = {
  ...changePlanWire,
  operation: "workbuddy_models_save",
  targetProviderId: "fyagent-v2-workbuddy-models",
  targetProviderName: "https://api.example.test/v1",
  currentProviderCode: "object_root",
  targetProviderCode: "object_root",
  restartExpectation: "not_required",
  adapter: {
    ...changePlanWire.adapter,
    adapterId: "workbuddy_models_save",
    operationType: "workbuddy_models_save",
    readSet: ["work_buddy_models_config", "work_buddy_backup"],
    writeSet: ["work_buddy_models_config", "work_buddy_backup"],
  },
  risks: [
    { code: "local_configuration_write", severity: "notice" },
    { code: "existing_model_ids_will_be_updated", severity: "warning" },
  ],
};

export const changeJobWorkBuddyWire: ChangeJobSnapshot = {
  ...changeJobWire,
  targetProviderId: "fyagent-v2-workbuddy-models",
  resources: [
    {
      kind: "work_buddy_models_config",
      status: "pending",
      code: "pending",
    },
    { kind: "work_buddy_backup", status: "pending", code: "pending" },
  ],
};
