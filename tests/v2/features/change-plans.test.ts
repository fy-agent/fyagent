import { describe, expect, it } from "vitest";

import {
  parseApplyChangePlanOutcome,
  parseChangeJobSnapshot,
  parseChangePlan,
} from "@/v2/shared/features/change-plans";
import { changeJobWire, changePlanWire } from "../fixtures/changePlans";

describe("Change Plan strict wire parsers", () => {
  it("accepts the exact nullable plan and job contracts", () => {
    expect(parseChangePlan(changePlanWire)).toEqual(changePlanWire);
    expect(parseChangeJobSnapshot(changeJobWire)).toEqual(changeJobWire);
    expect(
      parseApplyChangePlanOutcome({ kind: "admitted", job: changeJobWire }),
    ).toEqual({ kind: "admitted", job: changeJobWire });
  });

  it("fails closed on excess keys and unknown enums", () => {
    expect(() =>
      parseChangePlan({ ...changePlanWire, rawConfig: "secret" }),
    ).toThrow("Change Plan is unavailable");
    expect(() =>
      parseChangeJobSnapshot({ ...changeJobWire, status: "cancelled" }),
    ).toThrow("Change Job is unavailable");
    expect(() =>
      parseApplyChangePlanOutcome({ kind: "rejected", errorCode: "future" }),
    ).toThrow("Change Plan Apply is unavailable");
  });
});
