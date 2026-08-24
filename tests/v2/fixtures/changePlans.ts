import fixture from "../../fixtures/changePlanDtoContract.v2.json";

import type {
  CancelChangeJobOutcome,
  ChangeJobSnapshot,
  ChangePlan,
} from "@/v2/shared/features/change-plans";

export const changePlanWire = fixture.plan as ChangePlan;
export const changeJobWire = fixture.job as ChangeJobSnapshot;
export const cancelChangeJobWire = fixture.cancelOutcome as CancelChangeJobOutcome;
