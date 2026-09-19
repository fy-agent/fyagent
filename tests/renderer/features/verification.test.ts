import { describe, it, expect } from "vitest";
import {
  snapshotSchema,
  manualSchema,
  evidenceLabel,
} from "@/domain/verification";
import {
  verificationFixture,
  evidenceFixture,
  PROJECT,
} from "../fixtures/verification";
describe("verification evidence boundary", () => {
  it("rejects native credentials, unknown fields and customer machine outcomes", () => {
    const snapshot = verificationFixture();
    snapshot.evidence = [evidenceFixture()];
    expect(snapshotSchema.safeParse(snapshot).success).toBe(true);
    expect(
      snapshotSchema.safeParse({ ...snapshot, secretRef: "private" }).success,
    ).toBe(false);
    snapshot.evidence[0].stage = "customer_accepted";
    expect(snapshotSchema.safeParse(snapshot).success).toBe(false);
  });
  it("never displays expired, future or stale evidence as passed", () => {
    const e = evidenceFixture();
    expect(evidenceLabel(e)).toBe("通过");
    e.expiresAt = new Date(Date.now() - 1).toISOString();
    expect(evidenceLabel(e)).toBe("待复核");
    e.expiresAt = null;
    e.validity = "stale";
    expect(evidenceLabel(e)).toBe("待复核");
    e.validity = "current";
    e.recordedAt = new Date(Date.now() + 60000).toISOString();
    expect(evidenceLabel(e)).toBe("待复核");
  });
  it("requires real scoped manual details and rejects raw path/secret fields", () => {
    const r = {
      projectId: PROJECT,
      expectedRevision: 1,
      stage: "customer_accepted",
      outcome: "passed",
      person: "张三",
      role: "客户负责人",
      scope: "周报试点",
      observedAt: new Date().toISOString(),
      externalBasis: { reference: "UAT-9", issuer: "客户业务部" },
      basisEvidenceIds: [],
    };
    expect(manualSchema.safeParse(r).success).toBe(true);
    expect(manualSchema.safeParse({ ...r, person: "" }).success).toBe(false);
    expect(manualSchema.safeParse({ ...r, externalBasis: null }).success).toBe(
      false,
    );
    expect(
      manualSchema.safeParse({ ...r, scope: "/Users/private" }).success,
    ).toBe(false);
    expect(
      manualSchema.safeParse({ ...r, scope: "sk-secret-canary" }).success,
    ).toBe(false);
  });
});
