import { describe, it, expect } from "vitest";
import {
  snapshotSchema,
  manualSchema,
  handoffSchema,
  evidenceLabel,
} from "@/domain/verification";
import {
  verificationFixture,
  evidenceFixture,
  PROJECT,
} from "../fixtures/verification";
describe("verification evidence boundary", () => {
  it("keeps bounded business values and rejects unknown sample content", () => {
    const snapshot = verificationFixture();
    const sample = {
      inputDigest: "a".repeat(64),
      code: "ok" as const,
      matchesExpectation: true,
      validator: "weekly-report/v1" as const,
      metrics: {
        currentMinor: 18000000,
        previousMinor: 15000000,
        growthBps: 2000,
        targetBps: 9000,
      },
      sourceRowIds: ["row-1" as const],
    };
    snapshot.evidence = [
      {
        ...evidenceFixture(),
        stage: "sample_passed",
        checkerId: "kit_validator",
        sourceClass: "local_fixture",
        fixture: "baseline",
        sample,
      },
    ];
    expect(
      snapshotSchema.parse(snapshot).evidence[0].sample?.metrics?.growthBps,
    ).toBe(2000);
    snapshot.evidence[0].sample = {
      ...sample,
      metrics: { ...sample.metrics, growthBps: Number.MAX_SAFE_INTEGER + 1 },
    };
    expect(snapshotSchema.safeParse(snapshot).success).toBe(false);
    snapshot.evidence[0].sample = {
      ...sample,
      code: "invalid_input",
      metrics: null,
      sourceRowIds: [],
    };
    expect(snapshotSchema.safeParse(snapshot).success).toBe(true);
    expect(
      snapshotSchema.safeParse({
        ...snapshot,
        evidence: [
          {
            ...snapshot.evidence[0],
            sample: { ...sample, sourceRowIds: ["customer-private-row"] },
          },
        ],
      }).success,
    ).toBe(false);
  });

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
    for (const scope of [
      "本机演练：经营周报样例，不是真实客户验收",
      "阶段一；口径确认。张三·负责人（只读）！“周报”‘演练’？",
    ]) {
      expect(manualSchema.safeParse({ ...r, scope }).success).toBe(true);
      expect(
        handoffSchema.safeParse({
          items: [{ title: scope, owner: "交付·张三", completed: false }],
          rollback: "manual_only",
        }).success,
      ).toBe(true);
    }
    for (const scope of [
      "https://example.com/doc",
      "/tmp/report",
      "C:\\reports\\a",
      "../report",
      "api_key：private",
      "Bearer private",
      "secretRef：private",
      "token：private",
      "javascript:alert(1)",
      "资料\n秘密",
      " 前置空格",
    ]) {
      expect(manualSchema.safeParse({ ...r, scope }).success).toBe(false);
      expect(
        handoffSchema.safeParse({
          items: [{ title: scope, owner: null, completed: false }],
          rollback: "manual_only",
        }).success,
      ).toBe(false);
    }
    expect(
      manualSchema.safeParse({
        ...r,
        externalBasis: {
          ...r.externalBasis,
          reference: "https://example.feishu.cn/docx/Abc123",
        },
      }).success,
    ).toBe(true);
    for (const reference of [
      "javascript:alert(1)",
      "file:///tmp/report",
      "https://user:password@example.com/doc",
      "https://example.com/doc?token=private",
      "https://example.com/doc#secret",
      "https://example.com/%73k-private",
    ]) {
      expect(
        manualSchema.safeParse({
          ...r,
          externalBasis: { ...r.externalBasis, reference },
        }).success,
      ).toBe(false);
    }

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
