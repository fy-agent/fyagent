import { describe, expect, it } from "vitest";
import fixture from "../fixtures/configPackDtoContract.v1.json";
import {
  CONFIG_PACK_MAX_BYTES,
  parseConfigPack,
  parseImportPreview,
  parseImportResult,
  importRequest,
} from "../../src/domain/config-pack";

describe("portable configuration pack", () => {
  it("uses the native fixture and only admits portable fields", () => {
    expect(parseConfigPack(JSON.stringify(fixture))).toEqual(fixture);
    for (const key of [
      "apiKey",
      "secretRef",
      "credentialRef",
      "auth",
      "env",
      "command",
      "path",
      "activate",
      "__proto__",
    ]) {
      const changed = structuredClone(fixture);
      Object.defineProperty(changed.providers[0], key, {
        enumerable: true,
        value: "SECRET-CANARY",
      });
      expect(() => parseConfigPack(JSON.stringify(changed))).toThrow();
    }
  });
  it("rejects versions, byte overflow, duplicate names and unsafe portable values", () => {
    expect(() =>
      parseConfigPack(" ".repeat(CONFIG_PACK_MAX_BYTES + 1)),
    ).toThrow("too_large");
    expect(() =>
      parseConfigPack(
        JSON.stringify({ ...fixture, format: "fyagent-config-pack/v99" }),
      ),
    ).toThrow();
    expect(() =>
      parseConfigPack(
        JSON.stringify({
          ...fixture,
          providers: [{ ...fixture.providers[0], wireApi: "custom" }],
        }),
      ),
    ).toThrow();
    expect(() =>
      parseConfigPack(
        JSON.stringify({
          ...fixture,
          providers: [fixture.providers[0], fixture.providers[0]],
        }),
      ),
    ).toThrow();
    for (const endpoint of [
      "file:///Users/me/config",
      "https://user:password@example.com",
      "https://api.example.com/?key=value",
      "https://api.example.com/%2e%2e/a",
    ]) {
      expect(() =>
        parseConfigPack(
          JSON.stringify({
            ...fixture,
            providers: [{ ...fixture.providers[0], endpoint }],
          }),
        ),
      ).toThrow();
    }
    expect(() =>
      importRequest(JSON.stringify(fixture), [
        { action: "rename", name: "../escape" },
      ]),
    ).toThrow();
    expect(() =>
      parseConfigPack(
        JSON.stringify({
          ...fixture,
          providers: [{ ...fixture.providers[0], model: "./local-model" }],
        }),
      ),
    ).toThrow();
  });
  it("requires credential-pending previews and exact saved-field readback", () => {
    const preview = parseImportPreview({
      previewId: "11111111-1111-4111-8111-111111111111",
      digest: "a".repeat(64),
      entries: fixture.providers.map((provider) => ({
        provider,
        action: "add",
        conflict: false,
        canOverwrite: false,
        existing: null,
        credentialsRequired: true,
      })),
    });
    expect(
      parseImportResult({ providers: fixture.providers, skipped: 0 }, preview)
        .providers,
    ).toHaveLength(2);
    expect(() =>
      parseImportResult({ providers: [], skipped: 2 }, preview),
    ).toThrow("readback_failed");
    expect(() =>
      parseImportPreview({
        ...preview,
        entries: [{ ...preview.entries[0], credentialsRequired: false }],
      }),
    ).toThrow();
    expect(() =>
      parseImportPreview({
        ...preview,
        entries: [{ ...preview.entries[0], action: "overwrite" }],
      }),
    ).toThrow();
  });
});
