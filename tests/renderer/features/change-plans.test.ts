import { describe, expect, it } from "vitest";

import {
  parseApplyChangePlanOutcome,
  parseCancelChangeJobOutcome,
  parseChangeJobSnapshot,
  parseChangePlan,
} from "@/shared/features/change-plans";
import {
  changeJobWire,
  changePlanUpsertWire,
  changePlanWire,
  changePlanWorkBuddyWire,
  changeJobWorkBuddyWire,
} from "../fixtures/changePlans";

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

  it("allows adding, removing, omitting, or reordering faultPoints without blocking preview while rejecting other errors", () => {
    const originalFaultPoints = changePlanWire.adapter.faultPoints ?? [];

    // 1. Added fault point does not block preview
    const withAddedFault = {
      ...changePlanWire,
      adapter: {
        ...changePlanWire.adapter,
        faultPoints: [
          ...originalFaultPoints,
          "unexpected_fault_injection",
        ],
      },
    };
    expect(parseChangePlan(withAddedFault).planId).toBe(changePlanWire.planId);

    // 2. Reordered fault points do not block preview
    const withReorderedFault = {
      ...changePlanWire,
      adapter: {
        ...changePlanWire.adapter,
        faultPoints: [...originalFaultPoints].reverse(),
      },
    };
    expect(parseChangePlan(withReorderedFault).planId).toBe(
      changePlanWire.planId,
    );

    // 3. Removed fault point does not block preview
    const withRemovedFault = {
      ...changePlanWire,
      adapter: {
        ...changePlanWire.adapter,
        faultPoints: originalFaultPoints.slice(0, 1),
      },
    };
    expect(parseChangePlan(withRemovedFault).planId).toBe(
      changePlanWire.planId,
    );

    // 4. Omitted faultPoints completely does not block preview
    const { faultPoints, ...adapterWithoutFaultPoints } =
      changePlanWire.adapter;
    expect(faultPoints).toBeDefined();
    const withOmittedFault = {
      ...changePlanWire,
      adapter: adapterWithoutFaultPoints,
    };
    expect(parseChangePlan(withOmittedFault).planId).toBe(
      changePlanWire.planId,
    );

    // 5. Version error must still be rejected
    expect(() =>
      parseChangePlan({
        ...changePlanWire,
        adapter: {
          ...changePlanWire.adapter,
          adapterVersion: "2",
        },
      }),
    ).toThrow("Change Plan is unavailable");

    // 6. Operation identity error must still be rejected
    expect(() =>
      parseChangePlan({
        ...changePlanWire,
        adapter: {
          ...changePlanWire.adapter,
          operationType: "workbuddy_models_save",
        },
      }),
    ).toThrow("Change Plan is unavailable");

    // 7. Real read/write set errors must still be rejected
    expect(() =>
      parseChangePlan({
        ...changePlanWire,
        adapter: {
          ...changePlanWire.adapter,
          writeSet: ["arbitrary_file"],
        },
      }),
    ).toThrow("Change Plan is unavailable");

    // 8. Digest error must still be rejected
    expect(() =>
      parseChangePlan({
        ...changePlanWire,
        planDigest: "not-a-valid-sha256-hex-digest",
      }),
    ).toThrow("Change Plan is unavailable");
  });
});


