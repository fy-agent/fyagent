import { describe, expect, it } from "vitest";

import {
  parseApplyChangePlanOutcome,
  parseCancelChangeJobOutcome,
  parseChangeJobSnapshot,
  parseChangePlan,
} from "@/v2/shared/features/change-plans";
import { changeJobWire, changePlanUpsertWire, changePlanWire, changePlanWorkBuddyWire, changeJobWorkBuddyWire } from "../fixtures/changePlans";

describe("Change Plan strict wire parsers", () => {
  it("accepts the exact nullable plan and job contracts", () => {
    expect(parseChangePlan(changePlanWire)).toEqual(changePlanWire);
    expect(parseChangePlan(changePlanUpsertWire)).toEqual(changePlanUpsertWire);
    expect(parseChangePlan(changePlanWorkBuddyWire)).toEqual(
      changePlanWorkBuddyWire,
    );
    expect(parseChangeJobSnapshot(changeJobWire)).toEqual(changeJobWire);
    expect(parseChangeJobSnapshot(changeJobWorkBuddyWire)).toEqual(
      changeJobWorkBuddyWire,
    );
    expect(
      parseApplyChangePlanOutcome({ kind: "admitted", job: changeJobWire }),
    ).toEqual({ kind: "admitted", job: changeJobWire });
    expect(
      parseApplyChangePlanOutcome({
        kind: "idempotent_replay",
        job: changeJobWire,
      }),
    ).toEqual({ kind: "idempotent_replay", job: changeJobWire });
    expect(
      parseCancelChangeJobOutcome({
        accepted: true,
        code: "accepted",
        jobId: "job-1",
      }),
    ).toEqual({ accepted: true, code: "accepted", jobId: "job-1" });
  });

  it("fails closed on excess keys and unknown enums", () => {
    expect(() =>
      parseChangePlan({ ...changePlanWire, rawConfig: "secret" }),
    ).toThrow("Change Plan is unavailable");
    expect(() =>
      parseChangeJobSnapshot({ ...changeJobWire, status: "future" }),
    ).toThrow("Change Job is unavailable");
    expect(() =>
      parseChangePlan({
        ...changePlanWorkBuddyWire,
        adapter: changePlanWire.adapter,
      }),
    ).toThrow("Change Plan is unavailable");
    expect(() =>
      parseChangeJobSnapshot({
        ...changeJobWorkBuddyWire,
        resources: [
          ...changeJobWire.resources,
          ...changeJobWorkBuddyWire.resources,
        ],
      }),
    ).toThrow("Change Job is unavailable");
    expect(() =>
      parseChangePlan({
        ...changePlanWire,
        adapter: { ...changePlanWire.adapter, writeSet: ["arbitrary_file"] },
      }),
    ).toThrow("Change Plan is unavailable");
    expect(() =>
      parseChangeJobSnapshot({
        ...changeJobWire,
        executionId: "different-job",
      }),
    ).toThrow("Change Job is unavailable");
    expect(() =>
      parseChangeJobSnapshot({
        ...changeJobWire,
        eventSeq: 3,
      }),
    ).toThrow("Change Job is unavailable");
    expect(() =>
      parseApplyChangePlanOutcome({ kind: "rejected", errorCode: "future" }),
    ).toThrow("Change Plan Apply is unavailable");
    expect(() =>
      parseCancelChangeJobOutcome({
        accepted: true,
        code: "commit_point_passed",
        jobId: "job-1",
      }),
    ).toThrow("Change Job Cancel is unavailable");
  });
});
