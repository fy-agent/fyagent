import { describe, it, expect } from "vitest";
import fixture from "../../fixtures/deliveryKitContract.v1.json";
import {
  parseKitView,
  parseKitDemo,
  parseKitPreview,
} from "@/domain/delivery-kits";
import { safeKitError } from "@/shared/features/delivery-kits";

describe("delivery kit strict boundary", () => {
  it("accepts the native v1 golden contract and rejects nested extras", () => {
    expect(parseKitView(fixture).manifest.title).toBe("经营周报");
    expect(() => parseKitView({ ...fixture, secretRef: "CANARY" })).toThrow();
    expect(() =>
      parseKitView({
        ...fixture,
        manifest: { ...fixture.manifest, script: "CANARY" },
      }),
    ).toThrow();
    const altered = structuredClone(fixture);
    Object.assign(altered.manifest.connections[0], {
      env: { TOKEN: "CANARY" },
    });
    expect(() => parseKitView(altered)).toThrow();
    expect(() =>
      parseKitView({ ...fixture, connectionsChecked: true }),
    ).toThrow();
    expect(() =>
      parseKitView({ ...fixture, builtin: false, exportable: true }),
    ).toThrow();
    expect(() =>
      parseKitView({
        ...fixture,
        identity: { ...fixture.identity, kitId: "wrong" },
      }),
    ).toThrow();
  });
  it("validates preview kinds and closed native result identities", () => {
    expect(() =>
      parseKitPreview({
        previewId: "path",
        kind: "import",
        kit: fixture,
        conflict: false,
      }),
    ).toThrow();
    const result = {
      ...fixture.identity,
      sourceClass: "local_fixture",
      validatorVersion: "weekly-report/v1",
      checkedAt: "2026-09-19T12:00:00+00:00",
      cases: [
        {
          fixtureId: "baseline",
          inputDigest: "a".repeat(64),
          code: "ok",
          metrics: {
            currentMinor: 18000000,
            previousMinor: 15000000,
            growthBps: 2000,
            targetBps: 9000,
          },
          sourceRowIds: ["row-1"],
          matchesExpectation: true,
        },
      ],
    };
    expect(
      parseKitDemo(result, fixture.identity).cases[0].metrics?.currentMinor,
    ).toBe(18000000);
    expect(() =>
      parseKitDemo({ ...result, sourceClass: "customer" }, fixture.identity),
    ).toThrow();
    expect(() =>
      parseKitDemo(
        { ...result, manifestDigest: "b".repeat(64) },
        fixture.identity,
      ),
    ).toThrow();
    expect(() =>
      parseKitDemo(
        { ...result, cases: [{ ...result.cases[0], metrics: null }] },
        fixture.identity,
      ),
    ).toThrow();
  });
  it("never displays arbitrary error material", () => {
    expect(safeKitError("unsafe_content").code).toBe("unsafe_content");
    expect(
      safeKitError(new Error("SECRET-CANARY /Users/private")).message,
    ).not.toContain("CANARY");
    expect(
      safeKitError({ code: "unsafe_content", secret: "CANARY" }).code,
    ).toBe("library_unavailable");
  });
});
