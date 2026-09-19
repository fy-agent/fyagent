import type { Evidence, VerificationSnapshot } from "@/domain/verification";
export const PROJECT = "00000000-0000-4000-8000-000000000001";
export const OTHER_PROJECT = "00000000-0000-4000-8000-000000000002";
export function verificationFixture(projectId = PROJECT): VerificationSnapshot {
  return {
    schemaVersion: 1,
    projectId,
    projectRevision: 1,
    available: true,
    checkedAt: new Date().toISOString(),
    evidence: [],
    handoff: { items: [], rollback: null },
    handoffRevision: 0,
  };
}
export function evidenceFixture(): Evidence {
  return {
    id: "00000000-0000-4000-8000-000000000003",
    projectRevision: 1,
    kit: null,
    stage: "configuration_saved",
    outcome: "passed",
    validity: "current",
    sourceClass: "native_local",
    checkerId: "saved_configuration_readback",
    fixture: null,
    sample: null,
    checkerVersion: 1,
    appVersion: "0.4.5",
    observedAt: new Date().toISOString(),
    recordedAt: new Date().toISOString(),
    expiresAt: null,
    reasonCode: "saved_projection_unavailable",
    basisEvidenceIds: [],
    manual: null,
  };
}
